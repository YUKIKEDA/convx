//! Incremental insertion of the Delaunay sites (design §7, #189).
//!
//! The complex is kept as the projection of the lower hull of the lift of
//! the sites inserted so far. The outside of the site hull is covered by
//! outside simplices: a facet of the site hull with [`INFINITE`] in place of
//! one vertex. A finite simplex is stored in positive orientation. An
//! outside simplex is stored so that replacing [`INFINITE`] by a point
//! strictly beyond its facet gives positive orientation.
//!
//! A new site `q` is located by a visibility walk, then every simplex in
//! conflict with it is removed (the cavity) and each face on the cavity's
//! boundary is joined to `q`, in the orientation of the removed simplex
//! with `q` in place of the vertex opposite that face. Conflict is decided
//! by exact signs only:
//!
//! - A finite simplex conflicts when `q` is strictly inside its
//!   circumsphere: the lifted orientation of its vertices followed by `q`
//!   has the sign it has for a point interior to the simplex.
//! - An outside simplex conflicts when `q` is strictly beyond its facet.
//!   When `q` is on the facet's hyperplane, it conflicts when it is strictly
//!   inside the facet's circumsphere within that hyperplane. For a point on
//!   the hyperplane, the power with respect to every sphere through the
//!   facet is the same (the centers lie on the line normal to the
//!   hyperplane through the facet's circumcenter, and the normal part
//!   cancels), so this is the test against the finite simplex across the
//!   facet.
//!
//! With these rules a cavity's boundary face is never on a hyperplane
//! through `q`: if `q` were on the hyperplane of a face between a
//! conflicting simplex and one that does not conflict, it would have the
//! same power with respect to both, so both or neither would conflict. So
//! every new finite simplex has positive orientation, and debug builds
//! check it.
//!
//! The new simplices are linked from the cavity, with no search by vertex
//! set (#255). A new simplex shares each face through `q` with the new
//! simplex of the boundary face next to its own around the ridge they have
//! in common, and turning around that ridge through the cavity reaches it.
//!
//! The insertion also records, for each face it links to a finite simplex
//! outside the cavity, whether the two simplices are cospherical: the
//! conflict test already evaluated that lifted orientation. Two new
//! simplices whose boundary faces lie in one finite cavity simplex are not
//! cospherical, since together they hold its sites and `q`, which is
//! strictly inside its circumsphere. Only the other faces between new
//! simplices have no recorded answer.
//!
//! The insertion is one procedure over a [`Shape`], compiled once for
//! D = 2 ([`Plane`]), once for D = 3 ([`Space`]), and once for every other
//! dimension ([`Any`]) (design §7, ADR 0005). A shape gives the number of
//! vertices of a simplex, a constant for the first two, and the two
//! predicates. Every shape runs the same steps on the same exact signs, so
//! all publish the same triangulation.

use crate::hull::input::Input;
use crate::morton::{spread2, spread3};
use crate::predicates::{
    first_stage_fixed, orient, orient_from, orient_lifted_from, orient_lifted_with, LiftedHeight,
    Sign, Start,
};
use crate::small::Small;
use crate::Error;

/// The vertex at infinity of an outside simplex. Site numbers are below
/// `u32::MAX`, so it names no site.
pub(super) const INFINITE: u32 = u32::MAX;

/// No simplex: a free slot's neighbor before linking.
const NONE: u32 = u32::MAX;

/// Up to this many points, a predicate gathers its rows on the stack.
const INLINE: usize = 18;

/// Whether the two simplices across a face are cospherical, as the
/// insertion left it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Across {
    /// Not decided: a face between two simplices created together from
    /// boundary faces of different cavity simplices, or one with a simplex
    /// at infinity. The pass after the insertion decides the first kind by
    /// an orientation, and by the lifted orientation only when that is zero
    /// (design §7, #451).
    Unknown,
    /// The far vertex of either is not on the circumsphere of the other.
    Distinct,
    /// The two simplices lie on one sphere.
    Cospherical,
    /// A face between two simplices created together whose sites other than
    /// the newest are affinely independent: whether or not the two are
    /// cospherical, the face joins nothing after the insertion, since
    /// recorded faces join them when they are (design §7, #667).
    Independent,
}

/// What the insertion needs to know of the dimension: the number of
/// vertices of a simplex, and the predicates on sites.
///
/// [`Plane`] and [`Space`] make `k` a constant, so the loops over the
/// vertices of a simplex have a fixed length. They read each site as an
/// array of D coordinates and call the first stage of a predicate on those
/// arrays, and build the ids of a predicate in arrays of a fixed size
/// (ADR 0008, #446). [`Any`] reads `k` from a field and uses the predicates
/// of [`Sites`]. A predicate returns the same exact sign under every shape.
pub(super) trait Shape: Copy {
    /// Whether the insertion flips edges (D = 2, ADR 0007) rather than
    /// digging a cavity.
    const FLIPS: bool;

    /// `k = D + 1`: the vertices, and the neighbors, of a simplex.
    fn k(self) -> usize;

    /// Orientation of the sites `ids` (D + 1 of them).
    fn orient(self, sites: &Sites, ids: &[u32]) -> Result<Sign, Error>;

    /// Lifted orientation of the sites `ids` (D + 2 of them).
    fn lifted(self, sites: &Sites, ids: &[u32]) -> Result<Sign, Error>;

    /// The lifted orientation of the simplex `vertices` (D + 1 sites)
    /// followed by `q`.
    fn sphere_sign(self, sites: &Sites, vertices: &[u32], q: u32) -> Result<Sign, Error>;

    /// The orientation of the simplex `vertices` with vertex `slot` replaced
    /// by `q`.
    fn replaced(self, sites: &Sites, vertices: &[u32], slot: usize, q: u32) -> Result<Sign, Error>;
}

/// Any dimension: `k` is a value.
#[derive(Clone, Copy)]
pub(super) struct Any {
    k: usize,
}

impl Any {
    /// The shape of dimension `dim`.
    pub(super) fn of(dim: usize) -> Self {
        Self { k: dim + 1 }
    }
}

impl Shape for Any {
    const FLIPS: bool = false;

    #[inline(always)]
    fn k(self) -> usize {
        self.k
    }

    #[inline(always)]
    fn orient(self, sites: &Sites, ids: &[u32]) -> Result<Sign, Error> {
        sites.orient(ids, Start::FirstStage)
    }

    #[inline(always)]
    fn lifted(self, sites: &Sites, ids: &[u32]) -> Result<Sign, Error> {
        sites.lifted(ids, Start::FirstStage)
    }

    fn sphere_sign(self, sites: &Sites, vertices: &[u32], q: u32) -> Result<Sign, Error> {
        let k = self.k;
        if k < INLINE {
            let mut ids = [0_u32; INLINE];
            ids[..k].copy_from_slice(vertices);
            ids[k] = q;
            self.lifted(sites, &ids[..=k])
        } else {
            let mut ids = vertices.to_vec();
            ids.push(q);
            self.lifted(sites, &ids)
        }
    }

    fn replaced(self, sites: &Sites, vertices: &[u32], slot: usize, q: u32) -> Result<Sign, Error> {
        let k = self.k;
        if k <= INLINE {
            let mut ids = [0_u32; INLINE];
            ids[..k].copy_from_slice(vertices);
            ids[slot] = q;
            self.orient(sites, &ids[..k])
        } else {
            let mut ids = vertices.to_vec();
            ids[slot] = q;
            self.orient(sites, &ids)
        }
    }
}

/// A shape of one dimension `$d`: `k` is the constant `$k = $d + 1`, and a
/// predicate first tries the semi-static stage on its sites read as arrays
/// of `$d` coordinates, with no cached height and no dispatch on the size
/// ([`first_stage_fixed`]). What that stage does not certify goes to the
/// predicate of [`Sites`], which starts after that stage
/// ([`Start::AfterFirstStage`]), so the first stage runs once and the sign
/// is the one [`Any`] returns; debug builds assert it, and that predicate
/// checks its own certified signs against the exact one.
macro_rules! fixed_shape {
    ($(#[$doc:meta])* $name:ident, $d:literal, $k:literal, $lifted:literal, $flips:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy)]
        pub(super) struct $name;

        impl Shape for $name {
            const FLIPS: bool = $flips;

            #[inline(always)]
            fn k(self) -> usize {
                $k
            }

            #[inline(always)]
            fn orient(self, sites: &Sites, ids: &[u32]) -> Result<Sign, Error> {
                debug_assert_eq!(ids.len(), $k);
                debug_assert_eq!(sites.dim() + 1, $k);
                let origin: [f64; $d] = sites.fixed(ids[0]);
                let rows: [[f64; $d]; $d] = core::array::from_fn(|i| sites.fixed(ids[i + 1]));
                match first_stage_fixed(&origin, &rows) {
                    Some(sign) => {
                        debug_assert_eq!(Ok(sign), sites.orient(ids, Start::FirstStage));
                        Ok(sign)
                    }
                    None => sites.orient(ids, Start::AfterFirstStage),
                }
            }

            #[inline(always)]
            fn lifted(self, sites: &Sites, ids: &[u32]) -> Result<Sign, Error> {
                debug_assert_eq!(ids.len(), $lifted);
                debug_assert_eq!(sites.dim() + 2, $lifted);
                let origin: [f64; $d] = sites.fixed(ids[0]);
                let rows: [[f64; $d]; $k] = core::array::from_fn(|i| sites.fixed(ids[i + 1]));
                match first_stage_fixed(&origin, &rows) {
                    Some(sign) => {
                        debug_assert_eq!(Ok(sign), sites.lifted(ids, Start::FirstStage));
                        Ok(sign)
                    }
                    None => sites.lifted(ids, Start::AfterFirstStage),
                }
            }

            #[inline(always)]
            fn sphere_sign(
                self,
                sites: &Sites,
                vertices: &[u32],
                q: u32,
            ) -> Result<Sign, Error> {
                let ids: [u32; $lifted] =
                    core::array::from_fn(|i| if i < $k { vertices[i] } else { q });
                self.lifted(sites, &ids)
            }

            #[inline(always)]
            fn replaced(
                self,
                sites: &Sites,
                vertices: &[u32],
                slot: usize,
                q: u32,
            ) -> Result<Sign, Error> {
                let mut ids: [u32; $k] = core::array::from_fn(|i| vertices[i]);
                ids[slot] = q;
                self.orient(sites, &ids)
            }
        }
    };
}

fixed_shape!(
    /// D = 2: triangles, inserted by edge flips (ADR 0007).
    Plane,
    2,
    3,
    4,
    true
);
fixed_shape!(
    /// D = 3: tetrahedra.
    Space,
    3,
    4,
    5,
    false
);

/// What the pass after the insertion reads of a triangulation: the
/// mesh of every dimension and the half-edges of D = 2 (#667).
pub(super) trait Built {
    /// Whether the insertion was by flips, keeping candidates for
    /// cospherical faces instead of a record per face.
    const FLIPS: bool;
    fn slots(&self) -> usize;
    fn finite_cells(&self) -> impl Iterator<Item = u32> + '_;
    fn neighbor(&self, c: u32, slot: usize) -> u32;
    fn across(&self, c: u32, slot: usize) -> Across;
    fn is_finite(&self, c: u32) -> bool;
    fn vertices_of(&self, c: u32) -> &[u32];
    fn lifted_ids(&self, ids: &[u32]) -> Result<Sign, Error>;
    fn orient_ids(&self, ids: &[u32]) -> Result<Sign, Error>;
    fn candidates(&self) -> &[(u32, u32)];
    fn meet(&self, c: u32, n: u32) -> bool;
    fn recorded_cospherical(&self) -> bool;
    fn left_undecided(&self) -> bool;
}

impl<S: Shape> Built for Mesh<'_, S> {
    const FLIPS: bool = S::FLIPS;
    fn slots(&self) -> usize {
        Mesh::slots(self)
    }
    fn finite_cells(&self) -> impl Iterator<Item = u32> + '_ {
        Mesh::finite_cells(self)
    }
    fn neighbor(&self, c: u32, slot: usize) -> u32 {
        Mesh::neighbor(self, c, slot)
    }
    fn across(&self, c: u32, slot: usize) -> Across {
        Mesh::across(self, c, slot)
    }
    fn is_finite(&self, c: u32) -> bool {
        Mesh::is_finite(self, c)
    }
    fn vertices_of(&self, c: u32) -> &[u32] {
        Mesh::vertices_of(self, c)
    }
    fn lifted_ids(&self, ids: &[u32]) -> Result<Sign, Error> {
        Mesh::lifted_ids(self, ids)
    }
    fn orient_ids(&self, ids: &[u32]) -> Result<Sign, Error> {
        Mesh::orient_ids(self, ids)
    }
    fn candidates(&self) -> &[(u32, u32)] {
        Mesh::candidates(self)
    }
    fn meet(&self, c: u32, n: u32) -> bool {
        Mesh::meet(self, c, n)
    }
    fn recorded_cospherical(&self) -> bool {
        Mesh::recorded_cospherical(self)
    }
    fn left_undecided(&self) -> bool {
        Mesh::left_undecided(self)
    }
}

/// The triangulation being built: `k = D + 1` vertices and neighbors per
/// simplex, in flat arrays with stride `k`. `neighbors[c * k + i]` is the
/// simplex across the face opposite vertex `i` of simplex `c`.
pub(super) struct Mesh<'a, S: Shape> {
    shape: S,
    sites: &'a Sites,
    vertices: Vec<u32>,
    neighbors: Vec<u32>,
    /// Per face, as `neighbors`: what the insertion knows of the two
    /// simplices across it.
    across: Vec<Across>,
    /// Whether any face was ever recorded [`Across::Cospherical`]. When none
    /// was, no two finite simplices of D = 2 are joined after the insertion
    /// (#667).
    cospherical: bool,
    /// The insertion by flips (D = 2) keeps no record per face: the two
    /// triangles of every face its in-circle test found cospherical, which
    /// the pass after the insertion tests again where they still meet
    /// (#667).
    candidates: Vec<(u32, u32)>,
    /// Whether the insertion of D = 3 left a face between two finite
    /// simplices [`Across::Unknown`]: four coplanar sites besides the newest.
    /// When it left none and recorded no face cospherical, no face joins
    /// anything after the insertion (#667).
    undecided: bool,
    alive: Vec<bool>,
    mark: Vec<u32>,
    free: Vec<u32>,
    epoch: u32,
    /// The lifted orientation's sign for a point strictly inside the
    /// circumsphere of a positive simplex.
    inside: Sign,
    /// A simplex created by the last insertion, where the next walk starts.
    last: u32,
    /// Work space reused by every insertion.
    stack: Vec<u32>,
    cavity: Vec<u32>,
    boundary: Vec<(u32, usize, Across)>,
    created: Vec<u32>,
    /// Work space of [`Self::link_by_keys`]: an open-addressing table of
    /// (key, entry, slot), [`EMPTY_FACE`] where free, and the positions
    /// used by the current insertion, which are freed after it.
    faces: Vec<(u64, u32, u32)>,
    used: Vec<u32>,
}

/// A free position of [`Mesh::faces`]: no key, since two vertices of a
/// simplex differ.
const EMPTY_FACE: u64 = u64::MAX;

/// Every input site with its filtered lifted height, by index: one row
/// `(p, |p|^2, bound)` of length D + 2, so a predicate reads a site's
/// coordinates and its cached height from one place (design §7 allows the
/// cache; the exact stage never reads it).
pub(super) struct Sites {
    dim: usize,
    rows: Vec<f64>,
    /// The input index of each row: `0..n` for the input's own order, and
    /// the order of insertion for the copy the mesh is built on.
    input: Vec<u32>,
}

impl Sites {
    pub(super) fn of(input: &Input<'_>) -> Self {
        let dim = input.dim();
        let n = input.representative.len();
        let mut rows = Vec::with_capacity(n * (dim + 2));
        for i in 0..n as u32 {
            let p = input.point(i);
            let height = LiftedHeight::of(p);
            rows.extend_from_slice(p);
            rows.push(height.value());
            rows.push(height.error());
        }
        Self {
            dim,
            rows,
            input: (0..n as u32).collect(),
        }
    }

    /// The rows of `ids`, in that order: site `i` of the copy is site
    /// `ids[i]` of `self`.
    pub(super) fn permuted(&self, ids: &[u32]) -> Self {
        let stride = self.dim + 2;
        let mut rows = Vec::with_capacity(ids.len() * stride);
        for &i in ids {
            let at = i as usize * stride;
            rows.extend_from_slice(&self.rows[at..at + stride]);
        }
        Self {
            dim: self.dim,
            rows,
            input: ids.iter().map(|&i| self.input[i as usize]).collect(),
        }
    }

    /// The input index of site `index`.
    pub(super) fn input_index(&self, index: u32) -> u32 {
        self.input[index as usize]
    }

    pub(super) fn dim(&self) -> usize {
        self.dim
    }

    fn row(&self, index: u32) -> &[f64] {
        let stride = self.dim + 2;
        let start = index as usize * stride;
        &self.rows[start..start + stride]
    }

    /// The `D` coordinates of site `index` as an array, for the shapes of a
    /// fixed dimension; `D` is the sites' dimension.
    #[inline(always)]
    fn fixed<const D: usize>(&self, index: u32) -> [f64; D] {
        debug_assert_eq!(D, self.dim, "the shape is that of the sites' dimension");
        let start = index as usize * (D + 2);
        let row = &self.rows[start..start + D];
        core::array::from_fn(|j| row[j])
    }

    /// The coordinates of site `index`, bit for bit the input's.
    pub(super) fn point(&self, index: u32) -> &[f64] {
        &self.row(index)[..self.dim]
    }

    fn height(&self, index: u32) -> LiftedHeight {
        let row = self.row(index);
        LiftedHeight::stored(row[self.dim], row[self.dim + 1])
    }

    /// Orientation of the sites `ids` (D + 1 of them), starting at `start`.
    fn orient(&self, ids: &[u32], start: Start) -> Result<Sign, Error> {
        let n = ids.len();
        if n <= INLINE {
            let mut points: [&[f64]; INLINE] = [&[]; INLINE];
            for (slot, &i) in points.iter_mut().zip(ids) {
                *slot = self.point(i);
            }
            Ok(orient_from(&points[..n], start)?)
        } else {
            let points: Vec<&[f64]> = ids.iter().map(|&i| self.point(i)).collect();
            Ok(orient_from(&points, start)?)
        }
    }

    /// Lifted orientation of the sites `ids` (D + 2 of them), starting at
    /// `start`.
    pub(super) fn lifted(&self, ids: &[u32], start: Start) -> Result<Sign, Error> {
        let n = ids.len();
        if n <= INLINE {
            let mut points: [&[f64]; INLINE] = [&[]; INLINE];
            let mut heights = [LiftedHeight::of(&[]); INLINE];
            for ((slot, height), &i) in points.iter_mut().zip(&mut heights).zip(ids) {
                *slot = self.point(i);
                *height = self.height(i);
            }
            Ok(orient_lifted_from(&points[..n], &heights[..n], start)?)
        } else {
            let points: Vec<&[f64]> = ids.iter().map(|&i| self.point(i)).collect();
            let heights: Vec<LiftedHeight> = ids.iter().map(|&i| self.height(i)).collect();
            Ok(orient_lifted_from(&points, &heights, start)?)
        }
    }
}

/// The sign of the lifted orientation of a positive simplex followed by a
/// point strictly inside its circumsphere: the standard simplex and its
/// centroid, in dimension `d`.
pub(super) fn inside_sign(d: usize) -> Result<Sign, Error> {
    // It depends on `d` alone: computed once a dimension in a process, for
    // dimensions below the cache's size (#667).
    static CACHE: [std::sync::OnceLock<Sign>; 17] = [const { std::sync::OnceLock::new() }; 17];
    if let Some(sign) = CACHE.get(d).and_then(std::sync::OnceLock::get) {
        return Ok(*sign);
    }
    let sign = inside_sign_of(d)?;
    if let Some(cell) = CACHE.get(d) {
        let _ = cell.set(sign);
    }
    Ok(sign)
}

/// [`inside_sign`], computed: the lifted orientation of the standard simplex
/// with its centroid.
fn inside_sign_of(d: usize) -> Result<Sign, Error> {
    let mut rows: Vec<Vec<f64>> = (0..=d)
        .map(|i| (0..d).map(|j| if i == j + 1 { 1.0 } else { 0.0 }).collect())
        .collect();
    rows.push(vec![1.0 / (d as f64 + 1.0); d]);
    let points: Vec<&[f64]> = rows.iter().map(Vec::as_slice).collect();
    debug_assert_eq!(
        orient(&points[..=d])?,
        Sign::Positive,
        "the standard simplex is positive"
    );
    let heights: Vec<LiftedHeight> = points.iter().map(|p| LiftedHeight::of(p)).collect();
    let sign = orient_lifted_with(&points, &heights)?;
    debug_assert_ne!(sign, Sign::Zero, "the centroid is inside the circumsphere");
    Ok(sign)
}

impl<'a, S: Shape> Mesh<'a, S> {
    pub(super) fn vertices_of(&self, c: u32) -> &[u32] {
        let k = self.shape.k();
        &self.vertices[c as usize * k..(c as usize + 1) * k]
    }

    pub(super) fn neighbor(&self, c: u32, slot: usize) -> u32 {
        self.neighbors[c as usize * self.shape.k() + slot]
    }

    fn set_neighbor(&mut self, c: u32, slot: usize, n: u32) {
        let k = self.shape.k();
        self.neighbors[c as usize * k + slot] = n;
    }

    /// What the insertion knows of simplex `c` and its neighbor across the
    /// face opposite vertex `slot`.
    /// What the insertion knows of the face of `c` opposite `slot`. The
    /// insertion by flips keeps no record per face, and every face is
    /// [`Across::Unknown`] to it.
    pub(super) fn across(&self, c: u32, slot: usize) -> Across {
        self.across
            .get(c as usize * self.shape.k() + slot)
            .copied()
            .unwrap_or(Across::Unknown)
    }

    /// The slot of simplex `c` whose neighbor is `n`.
    fn back(&self, c: u32, n: u32) -> Option<usize> {
        (0..self.shape.k()).find(|&s| self.neighbor(c, s) == n)
    }

    /// The slot of vertex `v` in simplex `c`. Every caller read `v` from the
    /// vertices of `c`, so the slot exists; debug builds assert it.
    fn slot_of(&self, c: u32, v: u32) -> usize {
        let slot = self.vertices_of(c).iter().position(|&x| x == v);
        debug_assert!(slot.is_some(), "the vertex is in the simplex");
        slot.unwrap_or(0)
    }

    /// Whether the insertion ever recorded a face cospherical.
    pub(super) fn recorded_cospherical(&self) -> bool {
        self.cospherical
    }

    /// Whether a face between two finite simplices was left undecided
    /// ([`Self::undecided`]).
    pub(super) fn left_undecided(&self) -> bool {
        self.undecided
    }

    /// The pairs of triangles whose shared face the insertion by flips found
    /// cospherical, in the order found; a pair may no longer meet, or no
    /// longer exist.
    pub(super) fn candidates(&self) -> &[(u32, u32)] {
        &self.candidates
    }

    /// Whether `c` and `n` are live simplices across one face from each
    /// other.
    pub(super) fn meet(&self, c: u32, n: u32) -> bool {
        (c as usize) < self.alive.len()
            && (n as usize) < self.alive.len()
            && self.alive[c as usize]
            && self.alive[n as usize]
            && self.back(c, n).is_some()
    }

    pub(super) fn is_finite(&self, c: u32) -> bool {
        !self.vertices_of(c).contains(&INFINITE)
    }

    /// The live finite simplices.
    pub(super) fn finite_cells(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.alive.len() as u32).filter(|&c| self.alive[c as usize] && self.is_finite(c))
    }

    /// Orientation of the sites `ids` (D + 1 of them).
    pub(super) fn orient_ids(&self, ids: &[u32]) -> Result<Sign, Error> {
        self.shape.orient(self.sites, ids)
    }

    /// Lifted orientation of the sites `ids` (D + 2 of them).
    pub(super) fn lifted_ids(&self, ids: &[u32]) -> Result<Sign, Error> {
        self.shape.lifted(self.sites, ids)
    }

    /// One past the largest simplex number.
    pub(super) fn slots(&self) -> usize {
        self.alive.len()
    }

    /// Whether `q` is strictly inside the circumsphere of finite simplex `c`.
    ///
    /// A cospherical `q` (zero) is no conflict, as design §7 states. Counting
    /// it as one would also give a Delaunay triangulation, with larger
    /// cavities; the published split does not depend on it either way,
    /// because each cospherical group is split again by placing (see
    /// `super::inserted`).
    fn in_sphere(&self, c: u32, q: u32) -> Result<bool, Error> {
        Ok(self.sphere_sign(c, q)? == self.inside)
    }

    /// The lifted orientation of finite simplex `c` followed by `q`.
    fn sphere_sign(&self, c: u32, q: u32) -> Result<Sign, Error> {
        self.shape.sphere_sign(self.sites, self.vertices_of(c), q)
    }

    /// The orientation of simplex `c` with vertex `slot` replaced by `q`.
    fn replaced(&self, c: u32, slot: usize, q: u32) -> Result<Sign, Error> {
        self.shape
            .replaced(self.sites, self.vertices_of(c), slot, q)
    }

    /// Whether simplex `c` conflicts with `q` (module docs).
    fn conflicts(&self, c: u32, q: u32) -> Result<bool, Error> {
        Ok(self.conflict(c, q)?.0)
    }

    /// Whether simplex `c` conflicts with `q`, and for a finite `c`, whether
    /// `c` and `q` are cospherical.
    fn conflict(&self, c: u32, q: u32) -> Result<(bool, Across), Error> {
        match self.vertices_of(c).iter().position(|&v| v == INFINITE) {
            None => {
                let sign = self.sphere_sign(c, q)?;
                let across = if sign == Sign::Zero {
                    Across::Cospherical
                } else {
                    Across::Distinct
                };
                Ok((sign == self.inside, across))
            }
            Some(slot) => {
                let conflict = match self.replaced(c, slot, q)? {
                    Sign::Positive => true,
                    Sign::Negative => false,
                    Sign::Zero => self.in_sphere(self.neighbor(c, slot), q)?,
                };
                Ok((conflict, Across::Unknown))
            }
        }
    }

    fn alloc(&mut self, vertices: &[u32]) -> u32 {
        let k = self.shape.k();
        if let Some(c) = self.free.pop() {
            let at = c as usize * k;
            self.vertices[at..at + k].copy_from_slice(vertices);
            self.neighbors[at..at + k].fill(NONE);
            if !S::FLIPS {
                self.across[at..at + k].fill(Across::Unknown);
            }
            self.alive[c as usize] = true;
            self.mark[c as usize] = 0;
            c
        } else {
            self.vertices.extend_from_slice(vertices);
            self.neighbors.extend(core::iter::repeat_n(NONE, k));
            if !S::FLIPS {
                self.across.extend(core::iter::repeat_n(Across::Unknown, k));
            }
            self.alive.push(true);
            self.mark.push(0);
            (self.alive.len() - 1) as u32
        }
    }

    /// Links the initial simplex and its D + 1 outside simplices: two of
    /// them are neighbors across the face they share. There are D + 2, so
    /// comparing every pair of faces costs nothing that grows with the input.
    ///
    /// Each face is sorted once into one list, D sites a face. A sorted
    /// list built for both faces of every comparison was (D + 2)(D + 1)^3
    /// allocations, 33,000 in D = 9 and nearly all of a build's
    /// (`docs/bench.md`, #562).
    fn link_initial(&mut self, cells: &[u32]) {
        let k = self.shape.k();
        let d = k - 1;
        let mut faces: Vec<u32> = Vec::with_capacity(cells.len() * k * d);
        for &c in cells {
            for slot in 0..k {
                let start = faces.len();
                faces.extend(
                    self.vertices_of(c)
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != slot)
                        .map(|(_, &v)| v),
                );
                faces[start..].sort_unstable();
            }
        }
        // The face of the cell at place `i` in `cells` without its vertex
        // at `slot`.
        let face = |i: usize, slot: usize| &faces[(i * k + slot) * d..(i * k + slot + 1) * d];
        for (i, &a) in cells.iter().enumerate() {
            for (j, &b) in cells.iter().enumerate().skip(i + 1) {
                for sa in 0..k {
                    for sb in 0..k {
                        if face(i, sa) == face(j, sb) {
                            self.set_neighbor(a, sa, b);
                            self.set_neighbor(b, sb, a);
                        }
                    }
                }
            }
        }
        debug_assert!(
            cells
                .iter()
                .all(|&c| (0..k).all(|s| self.neighbor(c, s) != NONE)),
            "every face of the initial simplices is shared"
        );
    }

    /// The triangulation of the sites `order` after the initial simplex
    /// `first` (D + 1 affinely independent sites).
    pub(super) fn build(
        shape: S,
        sites: &'a Sites,
        first: &[u32],
        order: impl ExactSizeIterator<Item = u32>,
    ) -> Result<Self, Error> {
        let d = sites.dim();
        let k = shape.k();
        debug_assert_eq!(k, d + 1, "the shape is that of the sites' dimension");
        debug_assert_eq!(first.len(), k, "the initial simplex has D + 1 sites");
        let mut mesh = Self {
            shape,
            sites,
            vertices: Vec::with_capacity(order.len() * k * (2 * d)),
            neighbors: Vec::with_capacity(order.len() * k * (2 * d)),
            across: Vec::with_capacity(if S::FLIPS {
                0
            } else {
                order.len() * k * (2 * d)
            }),
            cospherical: false,
            candidates: Vec::new(),
            undecided: false,
            // The lists that grow with the simplices are sized with them, and
            // the work space of one insertion for a few dozen simplices, so
            // a small input reallocates none of them (#667).
            alive: Vec::with_capacity(order.len() * (2 * d)),
            mark: Vec::with_capacity(order.len() * (2 * d)),
            free: Vec::with_capacity(4 * k),
            epoch: 0,
            inside: inside_sign(d)?,
            last: 0,
            // The insertion by flips uses the stack alone.
            stack: Vec::with_capacity(8 * k),
            cavity: Vec::with_capacity(if S::FLIPS { 0 } else { 8 * k }),
            boundary: Vec::with_capacity(if S::FLIPS { 0 } else { 8 * k }),
            created: Vec::with_capacity(if S::FLIPS { 0 } else { 8 * k }),
            faces: Vec::new(),
            used: Vec::new(),
        };
        // The initial simplex and its outside ones on the stack (#667).
        let mut simplex: Small<u32, 12> = first.iter().copied().collect();
        match mesh.orient_ids(&simplex)? {
            Sign::Negative => simplex.swap(0, 1),
            Sign::Positive => {}
            Sign::Zero => {
                debug_assert!(false, "the initial simplex spans dimension D");
            }
        }
        let s = mesh.alloc(&simplex);
        let mut cells: Small<u32, 13> = Small::new();
        cells.push(s);
        for j in 0..k {
            // The facet opposite vertex j, with the vertex at infinity on
            // the side away from it: swapping two entries flips the sign.
            let mut outside = simplex.clone();
            outside[j] = INFINITE;
            outside.swap(j, (j + 1) % k);
            cells.push(mesh.alloc(&outside));
        }
        mesh.link_initial(&cells);
        mesh.last = s;
        for q in order {
            if S::FLIPS {
                mesh.insert_by_flips(q)?;
            } else {
                mesh.insert(q)?;
            }
        }
        debug_assert!(mesh.linked(), "neighbors are symmetric and share a face");
        Ok(mesh)
    }

    /// The three vertices of triangle `c`.
    fn triangle(&self, c: u32) -> [u32; 3] {
        let v = self.vertices_of(c);
        [v[0], v[1], v[2]]
    }

    /// Sets the vertices and the neighbors of triangle `c`.
    fn set_triangle(&mut self, c: u32, vertices: [u32; 3], neighbors: [u32; 3]) {
        let at = c as usize * 3;
        self.vertices[at..at + 3].copy_from_slice(&vertices);
        self.neighbors[at..at + 3].copy_from_slice(&neighbors);
    }

    /// Points the slot of `n` that held `old` at `new`.
    fn repoint(&mut self, n: u32, old: u32, new: u32) {
        let back = self.back(n, old);
        debug_assert!(back.is_some(), "neighbors are symmetric");
        if let Some(back) = back {
            self.set_neighbor(n, back, new);
        }
    }

    /// Keeps `c` and `n` as a candidate pair when their face is cospherical.
    /// Nothing is recorded per face: a face of D = 2 joins two triangles only
    /// when cospherical, and the candidates are tested again after the
    /// insertion (#667).
    fn record(&mut self, c: u32, n: u32, across: Across) {
        if across == Across::Cospherical {
            self.cospherical = true;
            self.candidates.push((c, n));
        }
    }

    /// Inserts site `q` by edge flips (D = 2, ADR 0007, design §7). The
    /// located triangle contains `q`, or, at infinity, sees it beyond its
    /// hull edge or inside it. It splits into three triangles through `q`,
    /// or, with the triangle across, into four when `q` is on an edge. Then
    /// every edge opposite `q` whose far triangle conflicts with `q` is
    /// flipped, which leaves the triangles around `q` Delaunay. An edge not
    /// flipped records what its in-circle test said.
    fn insert_by_flips(&mut self, q: u32) -> Result<(), Error> {
        debug_assert_eq!(self.shape.k(), 3, "edge flips are for triangles");
        let t = self.locate(q)?;
        debug_assert!(self.conflicts(t, q)?, "the located triangle conflicts");
        // The edge `q` lies on, by the slot of the vertex opposite it.
        let on_edge = match self.vertices_of(t).iter().position(|&v| v == INFINITE) {
            Some(slot) => (self.replaced(t, slot, q)? == Sign::Zero).then_some(slot),
            None => {
                let mut edge = None;
                for slot in 0..3 {
                    if self.replaced(t, slot, q)? == Sign::Zero {
                        debug_assert!(edge.is_none(), "a site is no vertex of the triangle");
                        edge = Some(slot);
                    }
                }
                edge
            }
        };
        let mut stack = core::mem::take(&mut self.stack);
        stack.clear();
        match on_edge {
            Some(slot) => self.split_edge(t, slot, q, &mut stack),
            None => self.split(t, q, &mut stack),
        }
        while let Some(c) = stack.pop() {
            let slot = self.slot_of(c, q);
            let n = self.neighbor(c, slot);
            let (conflict, across) = self.conflict(n, q)?;
            if conflict {
                self.flip(c, n, q);
                stack.push(c);
                stack.push(n);
            } else if across != Across::Unknown {
                // `n` is finite and `q` is on its circumcircle or strictly
                // outside it.
                self.record(c, n, across);
            }
        }
        self.stack = stack;
        self.last = t;
        Ok(())
    }

    /// Splits triangle `t` into three through `q`. Each new triangle is `t`
    /// with one vertex replaced by `q`, so it keeps the orientation of `t`.
    fn split(&mut self, t: u32, q: u32, stack: &mut Vec<u32>) {
        let [a, b, c] = self.triangle(t);
        let [na, nb, nc] = [
            self.neighbor(t, 0),
            self.neighbor(t, 1),
            self.neighbor(t, 2),
        ];
        let t2 = self.alloc(&[b, c, q]);
        let t3 = self.alloc(&[c, a, q]);
        self.set_triangle(t, [a, b, q], [t2, t3, nc]);
        self.set_triangle(t2, [b, c, q], [t3, t, na]);
        self.set_triangle(t3, [c, a, q], [t, t2, nb]);
        self.repoint(na, t, t2);
        self.repoint(nb, t, t3);
        stack.extend([t, t2, t3]);
    }

    /// Splits triangle `t` and the triangle across its edge opposite `slot`
    /// into four through `q`, which lies inside that edge.
    fn split_edge(&mut self, t: u32, slot: usize, q: u32, stack: &mut Vec<u32>) {
        let v = self.triangle(t);
        let (x, p, r) = (v[slot], v[(slot + 1) % 3], v[(slot + 2) % 3]);
        // `t` is (x, p, r); `u` across the edge (p, r) is (y, r, p).
        let t_np = self.neighbor(t, (slot + 1) % 3);
        let t_nr = self.neighbor(t, (slot + 2) % 3);
        let u = self.neighbor(t, slot);
        let y = self
            .triangle(u)
            .into_iter()
            .find(|&z| z != p && z != r)
            .unwrap_or(INFINITE);
        let u_np = self.neighbor(u, self.slot_of(u, p));
        let u_nr = self.neighbor(u, self.slot_of(u, r));
        let t2 = self.alloc(&[x, q, r]);
        let t4 = self.alloc(&[y, q, p]);
        self.set_triangle(t, [x, p, q], [t4, t2, t_nr]);
        self.set_triangle(t2, [x, q, r], [u, t_np, t]);
        self.set_triangle(u, [y, r, q], [t2, t4, u_np]);
        self.set_triangle(t4, [y, q, p], [t, u_nr, u]);
        self.repoint(t_np, t, t2);
        self.repoint(u_nr, u, t4);
        stack.extend([t, t2, u, t4]);
    }

    /// Flips the edge opposite `q` in triangle `t`, shared with `n`:
    /// `(q, a, b)` and `(b, a, d)` become `(q, a, d)` and `(q, d, b)`.
    fn flip(&mut self, t: u32, n: u32, q: u32) {
        let at_q = self.slot_of(t, q);
        let v = self.triangle(t);
        let (a, b) = (v[(at_q + 1) % 3], v[(at_q + 2) % 3]);
        let t_na = self.neighbor(t, (at_q + 2) % 3);
        let t_nb = self.neighbor(t, (at_q + 1) % 3);
        let d = self
            .triangle(n)
            .into_iter()
            .find(|&z| z != a && z != b)
            .unwrap_or(INFINITE);
        let n_ad = self.neighbor(n, self.slot_of(n, b));
        let n_db = self.neighbor(n, self.slot_of(n, a));
        self.set_triangle(t, [q, a, d], [n_ad, n, t_na]);
        self.set_triangle(n, [q, d, b], [n_db, t_nb, t]);
        self.repoint(n_ad, n, t);
        self.repoint(t_nb, t, n);
    }

    /// Whether every live simplex's neighbor across each face is live,
    /// points back, and holds that face. A debug check of the linking.
    fn linked(&self) -> bool {
        let k = self.shape.k();
        (0..self.alive.len() as u32)
            .filter(|&c| self.alive[c as usize])
            .all(|c| {
                (0..k).all(|slot| {
                    let n = self.neighbor(c, slot);
                    if n == NONE || !self.alive[n as usize] {
                        return false;
                    }
                    let Some(back) = self.back(n, c) else {
                        return false;
                    };
                    self.across(c, slot) == self.across(n, back)
                        && self
                            .vertices_of(c)
                            .iter()
                            .enumerate()
                            .filter(|&(i, _)| i != slot)
                            .all(|(_, v)| self.vertices_of(n).contains(v))
                })
            })
    }

    /// The simplex where `q` is located: one that conflicts with it. A
    /// visibility walk from the last created simplex; the face to cross is
    /// tried from a position that depends on `q`, which keeps the walk from
    /// cycling.
    ///
    /// The face to the simplex the walk came from is not tested (ADR 0007).
    /// From a finite simplex the walk crossed it because `q` is strictly
    /// beyond it, and from a simplex at infinity because `q` is not strictly
    /// beyond its hull facet; either way the orientation with `q` in place of
    /// the vertex opposite that face is not negative, so the walk takes the
    /// same steps and ends where it did.
    fn locate(&self, q: u32) -> Result<u32, Error> {
        let k = self.shape.k();
        // Seeded by the input index, so that the walk, and with it the order
        // the simplices are created and published in, does not depend on
        // the order the sites are stored in.
        let mut state =
            u64::from(self.sites.input_index(q)).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
        let mut c = self.last;
        let mut came_from = NONE;
        loop {
            if let Some(slot) = self.vertices_of(c).iter().position(|&v| v == INFINITE) {
                if self.conflicts(c, q)? {
                    return Ok(c);
                }
                came_from = c;
                c = self.neighbor(c, slot);
                continue;
            }
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let offset = (state % k as u64) as usize;
            let mut next = None;
            for t in 0..k {
                let slot = (t + offset) % k;
                if self.neighbor(c, slot) == came_from {
                    debug_assert_ne!(
                        self.replaced(c, slot, q)?,
                        Sign::Negative,
                        "the face the walk came through is not negative"
                    );
                    continue;
                }
                if self.replaced(c, slot, q)? == Sign::Negative {
                    next = Some(self.neighbor(c, slot));
                    break;
                }
            }
            match next {
                Some(n) => {
                    came_from = c;
                    c = n;
                }
                // q is in the closed simplex and is no vertex of it, so it
                // is strictly inside the circumsphere.
                None => return Ok(c),
            }
        }
    }

    /// Links the faces through the new site of the new tetrahedra, for
    /// D = 3, without turning around ridges.
    ///
    /// The new tetrahedron of boundary face `(c, slot)` has the new site at
    /// `slot`. Its face opposite vertex `i` (`i != slot`) holds the new site
    /// and the two vertices other than `slot` and `i`. That face is shared
    /// with exactly one other new tetrahedron, the one whose boundary face
    /// meets this one in those two vertices, so the two, ascending, name
    /// it. The faces are paired by that key in an open-addressing table of
    /// at least twice their number; only the positions this insertion used
    /// are freed after it. As in the turn of [`Self::insert`], two new
    /// tetrahedra whose boundary faces lie in one finite cavity simplex are
    /// not cospherical.
    ///
    /// A face between two finite new tetrahedra made from boundary faces of
    /// different cavity simplices is decided here when it can be: the sites
    /// of the two without the new one, which is the newest, are four, and
    /// when they are affinely independent the face joins nothing after the
    /// insertion (design §7), so it is recorded [`Across::Independent`]. That
    /// orientation was the pass after the insertion's, where it read the
    /// two tetrahedra again (#667). Four coplanar sites leave it
    /// [`Across::Unknown`], for the lifted test.
    fn link_by_keys(
        &mut self,
        q: u32,
        boundary: &[(u32, usize, Across)],
        created: &[u32],
    ) -> Result<(), Error> {
        let size = (2 * created.len() * 3).next_power_of_two().max(4);
        let mask = size - 1;
        let mut faces = core::mem::take(&mut self.faces);
        let mut used = core::mem::take(&mut self.used);
        if faces.len() < size {
            faces.resize(size, (EMPTY_FACE, 0, 0));
        }
        used.clear();
        for (entry, (&(c, slot, _), &new)) in boundary.iter().zip(created).enumerate() {
            for i in (0..4).filter(|&i| i != slot) {
                let v = self.vertices_of(new);
                let mut rest = (0..4).filter(|&m| m != slot && m != i).map(|m| v[m]);
                let (a, b) = (rest.next().unwrap_or(0), rest.next().unwrap_or(0));
                let key = u64::from(a.min(b)) << 32 | u64::from(a.max(b));
                let mut at = (key.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 32) as usize & mask;
                // At most half of the first `size` positions is used, so a
                // probe ends.
                let mut probes = 0_usize;
                loop {
                    probes += 1;
                    debug_assert!(probes <= size, "the table has room");
                    let (stored, other, other_slot) = faces[at];
                    if stored == EMPTY_FACE {
                        faces[at] = (key, entry as u32, i as u32);
                        used.push(at as u32);
                        break;
                    }
                    if stored == key {
                        let (other_c, _, _) = boundary[other as usize];
                        let other_new = created[other as usize];
                        let other_slot = other_slot as usize;
                        debug_assert_eq!(self.neighbor(new, i), NONE, "linked once");
                        debug_assert_eq!(self.neighbor(other_new, other_slot), NONE, "linked once");
                        self.set_neighbor(new, i, other_new);
                        self.set_neighbor(other_new, other_slot, new);
                        let distinct = if other_c == c {
                            self.is_finite(c)
                        } else if self.is_finite(new) && self.is_finite(other_new) {
                            let far = self.vertices_of(other_new)[other_slot];
                            let v = self.vertices_of(new);
                            let mut sites = [0_u32; 4];
                            let mut at = 0;
                            for (m, &s) in v.iter().enumerate() {
                                if m == i || s != q {
                                    sites[at] = s;
                                    at += 1;
                                }
                            }
                            debug_assert_eq!(at, 3, "the new site is a vertex of the face");
                            sites[3] = far;
                            let independent = self.orient_ids(&sites)? != Sign::Zero;
                            self.undecided |= !independent;
                            independent
                        } else {
                            false
                        };
                        if distinct {
                            let across = if other_c == c {
                                Across::Distinct
                            } else {
                                Across::Independent
                            };
                            self.across[new as usize * 4 + i] = across;
                            self.across[other_new as usize * 4 + other_slot] = across;
                        }
                        break;
                    }
                    at = (at + 1) & mask;
                }
            }
        }
        for &at in &used {
            faces[at as usize].0 = EMPTY_FACE;
        }
        debug_assert!(
            created
                .iter()
                .all(|&new| (0..4).all(|s| self.neighbor(new, s) != NONE)),
            "every face of a new tetrahedron is linked"
        );
        self.faces = faces;
        self.used = used;
        Ok(())
    }

    /// Inserts site `q`.
    fn insert(&mut self, q: u32) -> Result<(), Error> {
        let k = self.shape.k();
        let start = self.locate(q)?;
        debug_assert!(self.conflicts(start, q)?, "the located simplex conflicts");
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            // A simplex outside every cavity so far carries 0; after a wrap
            // no old mark may equal the new epoch.
            self.mark.fill(0);
            self.epoch = 1;
        }
        let epoch = self.epoch;
        self.mark[start as usize] = epoch;
        let mut stack = core::mem::take(&mut self.stack);
        let mut cavity = core::mem::take(&mut self.cavity);
        let mut boundary = core::mem::take(&mut self.boundary);
        let mut created = core::mem::take(&mut self.created);
        stack.clear();
        cavity.clear();
        boundary.clear();
        created.clear();
        stack.push(start);
        cavity.push(start);
        while let Some(c) = stack.pop() {
            for slot in 0..k {
                let n = self.neighbor(c, slot);
                if self.mark[n as usize] == epoch {
                    continue;
                }
                let (conflict, across) = self.conflict(n, q)?;
                if conflict {
                    self.mark[n as usize] = epoch;
                    stack.push(n);
                    cavity.push(n);
                } else {
                    boundary.push((c, slot, across));
                }
            }
        }

        // One new simplex per boundary face, linked to the simplex outside
        // it. The cavity simplex's slot is pointed at the new simplex too,
        // so that the turns below end on it.
        let mut row = [0_u32; INLINE];
        let mut spill = Vec::new();
        for &(c, slot, across) in &boundary {
            let vertices: &mut [u32] = if k <= INLINE {
                &mut row[..k]
            } else {
                spill.resize(k, 0);
                &mut spill
            };
            vertices.copy_from_slice(self.vertices_of(c));
            vertices[slot] = q;
            let outside = self.neighbor(c, slot);
            let new = if k <= INLINE {
                self.alloc(&row[..k])
            } else {
                self.alloc(&spill)
            };
            let at = new as usize * k + slot;
            self.neighbors[at] = outside;
            self.across[at] = across;
            self.cospherical |= across == Across::Cospherical;
            let back = self.back(outside, c);
            debug_assert!(back.is_some(), "neighbors are symmetric");
            if let Some(back) = back {
                let at = outside as usize * k + back;
                self.neighbors[at] = new;
                self.across[at] = across;
            }
            self.set_neighbor(c, slot, new);
            debug_assert!(
                !self.is_finite(new) || self.orient_ids(self.vertices_of(new))? == Sign::Positive,
                "a new finite simplex is positive"
            );
            created.push(new);
        }

        if k == 4 {
            self.link_by_keys(q, &boundary, &created)?;
        } else {
            // The face of a new simplex opposite vertex `i` holds `q` and the
            // ridge of its boundary face without that vertex. Turning around
            // that ridge through the cavity, from the boundary face, ends at the
            // next boundary face around it, whose new simplex shares the face.
            for (&(c, slot, _), &new) in boundary.iter().zip(&created) {
                for i in 0..k {
                    if i == slot || self.neighbor(new, i) != NONE {
                        continue;
                    }
                    // `cur` holds the ridge and `behind` and `ahead`; the face
                    // opposite `behind` was crossed, the one opposite `ahead` is
                    // next.
                    let (mut cur, mut behind, mut ahead) =
                        (c, self.vertices_of(c)[slot], self.vertices_of(c)[i]);
                    // The turn passes each cavity simplex around the ridge at
                    // most once, so it ends within the size of the cavity.
                    let mut turned = 0_usize;
                    loop {
                        turned += 1;
                        debug_assert!(turned <= cavity.len(), "the turn leaves the cavity");
                        let next = self.neighbor(cur, self.slot_of(cur, ahead));
                        if self.mark[next as usize] != epoch {
                            // `next` is the new simplex of the boundary face
                            // opposite `ahead` in `cur`; it has `cur`'s layout,
                            // so the shared face is opposite `behind`'s slot.
                            let slot = self.slot_of(cur, behind);
                            debug_assert_eq!(self.neighbor(next, slot), NONE, "linked once");
                            self.set_neighbor(new, i, next);
                            self.set_neighbor(next, slot, new);
                            // Both boundary faces in `c`: the two new simplices
                            // hold the sites of `c` and `q`, and `q` is strictly
                            // inside the circumsphere of a finite simplex of the
                            // cavity, so the two are not cospherical.
                            if cur == c && self.is_finite(c) {
                                self.across[new as usize * k + i] = Across::Distinct;
                                self.across[next as usize * k + slot] = Across::Distinct;
                            }
                            break;
                        }
                        let entry = self.back(next, cur);
                        debug_assert!(entry.is_some(), "neighbors are symmetric");
                        let far = self.vertices_of(next)[entry.unwrap_or(0)];
                        (cur, behind, ahead) = (next, far, behind);
                    }
                }
            }
        }

        for &c in &cavity {
            self.alive[c as usize] = false;
            self.free.push(c);
        }
        if let Some(&finite) = created.iter().find(|&&c| self.is_finite(c)) {
            self.last = finite;
        } else if let Some(&any) = created.first() {
            self.last = any;
        }
        self.stack = stack;
        self.cavity = cavity;
        self.boundary = boundary;
        self.created = created;
        Ok(())
    }
}

/// The most sites of one round a cell of the insertion order's grid keeps
/// in the order of their indices. A cell with more is ordered again on the
/// bounding box of its own sites ([`brio`]).
const CROWDED: usize = 16;

/// The insertion order of `sites` (dimension `d`): biased randomized
/// insertion order (BRIO) over rounds of geometric sizes, each round in
/// Morton order of the coordinates quantized over the sites' bounding box.
/// Deterministic: the round of a site comes from a hash of its index, and
/// ties go to the smaller index.
///
/// A cell that holds more than [`CROWDED`] sites of a round is ordered
/// again, by the Morton code on the bounding box of those sites, and so on
/// while a cell is crowded (#620). One grid over the whole box does not
/// follow the density: sites dense about a point with a long tail fall into
/// a few cells, consecutive insertions are then far apart, and the walk
/// between them grows with the sites (105 steps a site for 5.5 on 10^6
/// Kuzmin sites, #618). Where no cell is crowded the order is that of the
/// one grid.
///
/// It ends: a cell is ordered again only when it is a proper part of the
/// sites ordered with it, so every range handed on is shorter than the one
/// it came from. Distinct sites on a box whose widths are finite do part,
/// since the least and the greatest of a coordinate take the first and the
/// last cell.
pub(super) fn brio(rows: &Sites, sites: &[u32]) -> Vec<u32> {
    // D = 2 orders `u64` keys on a grid of 13 bits, which sort in less time
    // than `u128` ones; a crowded cell of that coarser grid is ordered again
    // like any other (#667).
    if rows.dim() == 2 {
        ordered::<u64>(rows, sites)
    } else {
        ordered::<u128>(rows, sites)
    }
}

/// The bits a coordinate of the grid of [`brio`] in dimension `d`: 13 for
/// D = 2, so that its key is a `u64` (#667); otherwise as many as a code of
/// 63 bits holds, at most 21.
fn grid_bits(d: usize) -> u32 {
    if d == 2 {
        13
    } else {
        (63 / d.max(1)).min(21) as u32
    }
}

/// A key of [`brio`]: it orders as `(Reverse(round), code, site)`, the
/// round in its top bits (at most 32, so 6 bits), the Morton code from bit
/// 32, the site in the low 32 bits.
trait OrderKey: Copy + Ord {
    /// The key of `site` in round `round`, its code zero.
    fn of(round: u32, site: u32) -> Self;
    /// The key with `code` as its code.
    fn with_code(self, code: u64) -> Self;
    /// The round and the code: equal for the sites of one cell of a round.
    fn cell(self) -> u128;
    /// The site.
    fn site(self) -> u32;
}

impl OrderKey for u128 {
    fn of(round: u32, site: u32) -> Self {
        u128::from(32 - round) << 96 | u128::from(site)
    }
    fn with_code(self, code: u64) -> Self {
        const CODE: u128 = (u64::MAX as u128) << 32;
        self & !CODE | u128::from(code) << 32
    }
    fn cell(self) -> u128 {
        self >> 32
    }
    fn site(self) -> u32 {
        self as u32
    }
}

impl OrderKey for u64 {
    fn of(round: u32, site: u32) -> Self {
        u64::from(32 - round) << 58 | u64::from(site)
    }
    fn with_code(self, code: u64) -> Self {
        const CODE: u64 = ((1 << 26) - 1) << 32;
        self & !CODE | (code & ((1 << 26) - 1)) << 32
    }
    fn cell(self) -> u128 {
        u128::from(self >> 32)
    }
    fn site(self) -> u32 {
        self as u32
    }
}

/// [`brio`] on keys `K`.
fn ordered<K: OrderKey>(rows: &Sites, sites: &[u32]) -> Vec<u32> {
    let d = rows.dim();
    let bits = grid_bits(d);
    let round = |s: u32| -> u32 {
        // SplitMix64 of the index; about half the sites get 0 (the last
        // round), a quarter 1, and so on.
        let mut z = u64::from(s).wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        z.trailing_zeros().min(32)
    };
    // One key per site (#423); the code is written by `in_morton_order`.
    let mut keys: Vec<K> = sites.iter().map(|&s| K::of(round(s), s)).collect();
    // The box of the range being ordered, allocated once, and the ranges
    // still to order after the whole: none where no cell is crowded, and
    // then nothing is allocated for them.
    let mut low = vec![0.0; d];
    let mut high = vec![0.0; d];
    let mut crowded: Vec<(usize, usize)> = Vec::new();
    let mut whole = Some((0, keys.len()));
    while let Some((from, to)) = whole.take().or_else(|| crowded.pop()) {
        let range = &mut keys[from..to];
        in_morton_order(rows, bits, range, &mut low, &mut high);
        // The cells of one round with more than `CROWDED` sites.
        let mut start = 0;
        for end in 1..=range.len() {
            if end < range.len() && range[end].cell() == range[start].cell() {
                continue;
            }
            if end - start > CROWDED && end - start < range.len() {
                crowded.push((from + start, from + end));
            }
            start = end;
        }
    }
    keys.into_iter().map(OrderKey::site).collect()
}

/// Writes into each key of `keys` the Morton code of its site on a grid of
/// `bits` bits a coordinate over the bounding box of the sites of `keys`,
/// and sorts them. `low` and `high` are scratch for the box.
fn in_morton_order<K: OrderKey>(
    rows: &Sites,
    bits: u32,
    keys: &mut [K],
    low: &mut [f64],
    high: &mut [f64],
) {
    let d = rows.dim();
    low.fill(f64::INFINITY);
    high.fill(f64::NEG_INFINITY);
    for &key in keys.iter() {
        for (j, &x) in rows.point(key.site()).iter().enumerate() {
            low[j] = low[j].min(x);
            high[j] = high[j].max(x);
        }
    }
    let scale = if bits == 0 {
        0.0
    } else {
        ((1_u64 << bits) - 1) as f64
    };
    // Coordinate `j` of site `s` on the grid of `bits` bits. The mask keeps
    // the cell within those bits, as the loop below keeps only them, so the
    // masks of D = 2 and 3 and the loop agree for any cell (review of #434).
    let mask = (1_u64 << bits) - 1;
    let cell = |p: &[f64], j: usize| -> u64 {
        let width = high[j] - low[j];
        let t = if width > 0.0 {
            (p[j] - low[j]) / width
        } else {
            0.0
        };
        // NaN and out-of-range values saturate; only the order is affected.
        (t * scale) as u64 & mask
    };
    // The bits of the cells interleaved from the top, coordinate 0 first
    // within each level: bit `b` of coordinate `j` lands at `b d + d - 1 - j`.
    // D = 2 and 3 spread each cell's bits with masks (#423); the loop below
    // gives the same code for every dimension.
    let morton = |s: u32| -> u64 {
        let p = rows.point(s);
        match d {
            2 => spread2(cell(p, 0)) << 1 | spread2(cell(p, 1)),
            3 => spread3(cell(p, 0)) << 2 | spread3(cell(p, 1)) << 1 | spread3(cell(p, 2)),
            _ => {
                // On the stack for D <= 16, so the order allocates nothing
                // per site.
                let cells: Small<u64, 16> = (0..d).map(|j| cell(p, j)).collect();
                let mut code = 0_u64;
                for b in (0..bits).rev() {
                    for &c in &cells {
                        code = (code << 1) | ((c >> b) & 1);
                    }
                }
                code
            }
        }
    };
    for key in keys.iter_mut() {
        *key = key.with_code(morton(key.site()));
    }
    keys.sort_unstable();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hull::input::accept_sites;
    use core::cmp::Ordering;

    /// The Morton code of each of `sites` on the grid over their bounding
    /// box, the bits interleaved one at a time (the code before #423).
    fn codes_by_loop(rows: &Sites, sites: &[u32]) -> Vec<u64> {
        let d = rows.dim();
        let mut low = vec![f64::INFINITY; d];
        let mut high = vec![f64::NEG_INFINITY; d];
        for &s in sites {
            for (j, &x) in rows.point(s).iter().enumerate() {
                low[j] = low[j].min(x);
                high[j] = high[j].max(x);
            }
        }
        let bits = grid_bits(d);
        let scale = if bits == 0 {
            0.0
        } else {
            ((1_u64 << bits) - 1) as f64
        };
        let morton = |s: u32| -> u64 {
            let p = rows.point(s);
            let cell: Vec<u64> = (0..d)
                .map(|j| {
                    let width = high[j] - low[j];
                    let t = if width > 0.0 {
                        (p[j] - low[j]) / width
                    } else {
                        0.0
                    };
                    // NaN and out-of-range values saturate; only the order is
                    // affected.
                    (t * scale) as u64
                })
                .collect();
            let mut code = 0_u64;
            for b in (0..bits).rev() {
                for &c in &cell {
                    code = (code << 1) | ((c >> b) & 1);
                }
            }
            code
        };
        sites.iter().map(|&s| morton(s)).collect()
    }

    /// The round of site `s`: SplitMix64 of the index; about half the sites
    /// get 0 (the last round), a quarter 1, and so on.
    fn round_of(s: u32) -> u32 {
        let mut z = u64::from(s).wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        z.trailing_zeros().min(32)
    }

    /// The BRIO order on one grid, as it was before #620: a sort of tuples.
    /// The reference of [`brio`] where no cell is crowded.
    fn brio_by_tuples(rows: &Sites, sites: &[u32]) -> Vec<u32> {
        let codes = codes_by_loop(rows, sites);
        let mut keyed: Vec<(u32, u64, u32)> = sites
            .iter()
            .zip(codes)
            .map(|(&s, m)| (round_of(s), m, s))
            .collect();
        keyed.sort_unstable_by_key(|&(r, m, s)| (core::cmp::Reverse(r), m, s));
        keyed.into_iter().map(|(_, _, s)| s).collect()
    }

    /// The sites of one cell (ascending) in their order: as they are when
    /// they are at most 16, and otherwise by the cells of the grid over
    /// their own box, each in its order; as they are when that grid does not
    /// part them.
    fn refined(rows: &Sites, cell: &[u32], out: &mut Vec<u32>) {
        if cell.len() <= 16 {
            out.extend_from_slice(cell);
            return;
        }
        let mut parts: std::collections::BTreeMap<u64, Vec<u32>> = Default::default();
        for (&s, code) in cell.iter().zip(codes_by_loop(rows, cell)) {
            parts.entry(code).or_default().push(s);
        }
        if parts.len() == 1 {
            out.extend_from_slice(cell);
            return;
        }
        for part in parts.values() {
            refined(rows, part, out);
        }
    }

    /// The reference of [`brio`]: the rounds from the last hash bit down,
    /// each by the cells of the one grid, and a crowded cell by recursion.
    fn brio_by_recursion(rows: &Sites, sites: &[u32]) -> Vec<u32> {
        let mut sorted = sites.to_vec();
        sorted.sort_unstable();
        let codes = codes_by_loop(rows, &sorted);
        let mut cells: std::collections::BTreeMap<(core::cmp::Reverse<u32>, u64), Vec<u32>> =
            Default::default();
        for (&s, code) in sorted.iter().zip(codes) {
            cells
                .entry((core::cmp::Reverse(round_of(s)), code))
                .or_default()
                .push(s);
        }
        let mut out = Vec::new();
        let whole = cells.len() == 1;
        for cell in cells.values() {
            if whole {
                out.extend_from_slice(cell);
            } else {
                refined(rows, cell, &mut out);
            }
        }
        out
    }

    #[test]
    fn brio_keeps_the_order_of_the_tuples() {
        // Sites in D = 1 to 5 (the masks of D = 2 and 3, the loop of the
        // others): random ones, a grid with repeated coordinates, a set with
        // two values in one coordinate, and D + 1 random sites (the fewest
        // an accepted input has). The order is the reference's, site for
        // site.
        let mut state = 0x4230_0001_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut cases = 0;
        for d in 1..=5 {
            for case in 0..4 {
                let n = if case == 3 { d + 1 } else { 2000 };
                let points: Vec<f64> = (0..n * d)
                    .map(|i| match case {
                        0 => (next() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0,
                        1 => (next() % 7) as f64,
                        2 if i % d == 0 => 3.5 + (next() % 2) as f64,
                        _ => (next() >> 11) as f64 / (1_u64 << 53) as f64,
                    })
                    .collect();
                let input = accept_sites(d, &points).unwrap();
                let rows = Sites::of(&input);
                let all: Vec<u32> = input.representatives.clone();
                assert_eq!(
                    brio(&rows, &all),
                    brio_by_tuples(&rows, &all),
                    "D = {d}, case {case}"
                );
                cases += 1;
            }
        }
        assert_eq!(cases, 20);
    }

    /// A crowded cell is ordered again on its own box (#620), against the
    /// recursive reference: sites dense about a point with a long tail (a
    /// radius of `(1 - u)^-6`), sites at scales that halve a
    /// hundred times, and both at once, in D = 2 to 5. On each the order
    /// is a permutation and is not the order of the one grid.
    #[test]
    fn brio_orders_a_crowded_cell_again() {
        let mut state = 0x6200_0001_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1_u64 << 53) as f64
        };
        let mut cases = 0;
        for d in 2..=5 {
            for case in 0..3 {
                let n = 6000;
                let mut points = Vec::with_capacity(n * d);
                for i in 0..n {
                    let radius = match case {
                        0 => (1.0 - next()).powi(-6),
                        1 => 0.5_f64.powi((i % 100) as i32),
                        _ => 0.5_f64.powi((i % 40) as i32) / (1.0 - next()),
                    };
                    points.extend((0..d).map(|_| radius * (2.0 * next() - 1.0)));
                }
                let input = accept_sites(d, &points).unwrap();
                let rows = Sites::of(&input);
                let all: Vec<u32> = input.representatives.clone();
                let order = brio(&rows, &all);
                let what = format!("D = {d}, case {case}");
                assert_eq!(order, brio_by_recursion(&rows, &all), "{what}");
                assert!(order != brio_by_tuples(&rows, &all), "{what}");
                let mut sorted = order.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, all, "{what}");
                cases += 1;
            }
        }
        assert_eq!(cases, 12);
    }

    /// Sites the grid cannot part end the order: the widths of a box that
    /// spans the whole range of `f64` are infinite, every site takes the
    /// first cell, and the cell is left in the order of the indices.
    #[test]
    fn brio_ends_where_the_grid_parts_nothing() {
        let top = f64::MAX;
        let mut points = vec![-top, -top, top, top];
        points.extend((0..200).map(|i| f64::from(i) * 1e300));
        let input = accept_sites(2, &points).unwrap();
        let rows = Sites::of(&input);
        let all: Vec<u32> = input.representatives.clone();
        let order = brio(&rows, &all);
        assert_eq!(order, brio_by_tuples(&rows, &all));
        assert_eq!(order, brio_by_recursion(&rows, &all));
    }

    /// Exact lifted orientation of four integer sites of dimension 2: the
    /// sign of the 3 x 3 determinant of `(p - o, |p|^2 - |o|^2)` in `i128`.
    fn lifted_sign_2d(points: &[[i64; 2]]) -> Sign {
        let norm = |p: [i64; 2]| i128::from(p[0]).pow(2) + i128::from(p[1]).pow(2);
        let o = points[0];
        let m: Vec<[i128; 3]> = points[1..]
            .iter()
            .map(|&p| {
                [
                    i128::from(p[0] - o[0]),
                    i128::from(p[1] - o[1]),
                    norm(p) - norm(o),
                ]
            })
            .collect();
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        match det.cmp(&0) {
            Ordering::Less => Sign::Negative,
            Ordering::Equal => Sign::Zero,
            Ordering::Greater => Sign::Positive,
        }
    }

    #[test]
    fn cached_heights_give_the_exact_lifted_sign() {
        // Eight sites on the circle of radius 5, two just off it, and the
        // centre: many quadruples are exactly cocircular, and the others are
        // one unit from it. Scaling by 2^e multiplies the lifted determinant
        // by a positive power of two, and a translation keeps it, so the
        // integer sign is the expected sign of every case. At 2^-540 the
        // squares underflow and at 2^520 they overflow, so the cached heights
        // cannot certify and the exact stage decides; at 2^40 with the shift
        // the filter fails on cancellation.
        let sites: [[i64; 2]; 11] = [
            [5, 0],
            [0, 5],
            [-5, 0],
            [0, -5],
            [3, 4],
            [4, 3],
            [-3, 4],
            [4, -3],
            [3, 5],
            [5, 1],
            [0, 0],
        ];
        let cases: [(i32, f64); 6] = [
            (0, 0.0),
            (-540, 0.0),
            (520, 0.0),
            (40, 0.0),
            (40, 1.0e15),
            (0, 1.0e15),
        ];
        let mut decided = [0usize; 3];
        for (e, shift) in cases {
            let scale = 2f64.powi(e);
            let points: Vec<f64> = sites
                .iter()
                .flat_map(|p| p.map(|x| x as f64 * scale + shift))
                .collect();
            let input = accept_sites(2, &points).unwrap();
            let rows = Sites::of(&input);
            let n = sites.len() as u32;
            for a in 0..n {
                for b in a + 1..n {
                    for c in b + 1..n {
                        for d in c + 1..n {
                            let indices = [a, b, c, d];
                            let expected = lifted_sign_2d(&indices.map(|i| sites[i as usize]));
                            assert_eq!(
                                rows.lifted(&indices, Start::FirstStage).unwrap(),
                                expected,
                                "2^{e}, shift {shift}, {indices:?}"
                            );
                            decided[expected as usize] += 1;
                        }
                    }
                }
            }
        }
        assert!(
            decided.iter().all(|&c| c > 0),
            "every sign occurs: {decided:?}"
        );
    }
}
