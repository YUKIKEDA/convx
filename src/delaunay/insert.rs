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
use crate::hull::ConvexHullError;
use crate::predicates::{
    first_stage, orient, orient_from, orient_lifted_from, orient_lifted_with, LiftedHeight, Sign,
    Start,
};
use crate::small::Small;

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
    /// at infinity.
    Unknown,
    /// The far vertex of either is not on the circumsphere of the other.
    Distinct,
    /// The two simplices lie on one sphere.
    Cospherical,
}

/// What the insertion needs to know of the dimension: the number of
/// vertices of a simplex, and the two predicates on sites.
///
/// [`Plane`] and [`Space`] make `k` a constant, so the loops over the
/// vertices of a simplex have a fixed length, and call the first stage of a
/// predicate on rows gathered into an array of that size. [`Any`] reads `k`
/// from a field and uses the predicates of [`Sites`]. A predicate returns
/// the same exact sign under every shape.
pub(super) trait Shape: Copy {
    /// `k = D + 1`: the vertices, and the neighbors, of a simplex.
    fn k(self) -> usize;

    /// Orientation of the sites `ids` (D + 1 of them).
    fn orient(self, sites: &Sites, ids: &[u32]) -> Result<Sign, ConvexHullError>;

    /// Lifted orientation of the sites `ids` (D + 2 of them).
    fn lifted(self, sites: &Sites, ids: &[u32]) -> Result<Sign, ConvexHullError>;
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
    #[inline(always)]
    fn k(self) -> usize {
        self.k
    }

    #[inline(always)]
    fn orient(self, sites: &Sites, ids: &[u32]) -> Result<Sign, ConvexHullError> {
        sites.orient(ids, Start::FirstStage)
    }

    #[inline(always)]
    fn lifted(self, sites: &Sites, ids: &[u32]) -> Result<Sign, ConvexHullError> {
        sites.lifted(ids, Start::FirstStage)
    }
}

/// A shape of one dimension: `k` is the constant `$k`, and a predicate first
/// tries the semi-static stage on `$k` (or `$k + 1`) rows in an array, with
/// no cached height and no dispatch on the size. What that stage does not
/// certify goes to the predicate of [`Sites`], which starts after that stage
/// ([`Start::AfterFirstStage`]), so the first stage runs once and the sign
/// is the one [`Any`] returns; debug builds assert it, and that predicate
/// checks its own certified signs against the exact one.
macro_rules! fixed_shape {
    ($(#[$doc:meta])* $name:ident, $k:literal, $lifted:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy)]
        pub(super) struct $name;

        impl Shape for $name {
            #[inline(always)]
            fn k(self) -> usize {
                $k
            }

            #[inline(always)]
            fn orient(self, sites: &Sites, ids: &[u32]) -> Result<Sign, ConvexHullError> {
                debug_assert_eq!(ids.len(), $k);
                debug_assert_eq!(sites.dim() + 1, $k);
                let rows: [&[f64]; $k] = core::array::from_fn(|i| sites.point(ids[i]));
                match first_stage(rows[0], &rows[1..], false) {
                    Some(sign) => {
                        debug_assert_eq!(Ok(sign), sites.orient(ids, Start::FirstStage));
                        Ok(sign)
                    }
                    None => sites.orient(ids, Start::AfterFirstStage),
                }
            }

            #[inline(always)]
            fn lifted(self, sites: &Sites, ids: &[u32]) -> Result<Sign, ConvexHullError> {
                debug_assert_eq!(ids.len(), $lifted);
                debug_assert_eq!(sites.dim() + 2, $lifted);
                let rows: [&[f64]; $lifted] = core::array::from_fn(|i| sites.point(ids[i]));
                match first_stage(rows[0], &rows[1..], true) {
                    Some(sign) => {
                        debug_assert_eq!(Ok(sign), sites.lifted(ids, Start::FirstStage));
                        Ok(sign)
                    }
                    None => sites.lifted(ids, Start::AfterFirstStage),
                }
            }
        }
    };
}

fixed_shape!(
    /// D = 2: triangles.
    Plane,
    3,
    4
);
fixed_shape!(
    /// D = 3: tetrahedra.
    Space,
    4,
    5
);

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
        Self { dim, rows }
    }

    pub(super) fn dim(&self) -> usize {
        self.dim
    }

    fn row(&self, index: u32) -> &[f64] {
        let stride = self.dim + 2;
        let start = index as usize * stride;
        &self.rows[start..start + stride]
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
    fn orient(&self, ids: &[u32], start: Start) -> Result<Sign, ConvexHullError> {
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
    pub(super) fn lifted(&self, ids: &[u32], start: Start) -> Result<Sign, ConvexHullError> {
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
fn inside_sign(d: usize) -> Result<Sign, ConvexHullError> {
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
    pub(super) fn across(&self, c: u32, slot: usize) -> Across {
        self.across[c as usize * self.shape.k() + slot]
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

    pub(super) fn is_finite(&self, c: u32) -> bool {
        !self.vertices_of(c).contains(&INFINITE)
    }

    /// The live finite simplices.
    pub(super) fn finite_cells(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.alive.len() as u32).filter(|&c| self.alive[c as usize] && self.is_finite(c))
    }

    /// Orientation of the sites `ids` (D + 1 of them).
    fn orient_ids(&self, ids: &[u32]) -> Result<Sign, ConvexHullError> {
        self.shape.orient(self.sites, ids)
    }

    /// Lifted orientation of the sites `ids` (D + 2 of them).
    pub(super) fn lifted_ids(&self, ids: &[u32]) -> Result<Sign, ConvexHullError> {
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
    fn in_sphere(&self, c: u32, q: u32) -> Result<bool, ConvexHullError> {
        Ok(self.sphere_sign(c, q)? == self.inside)
    }

    /// The lifted orientation of finite simplex `c` followed by `q`.
    fn sphere_sign(&self, c: u32, q: u32) -> Result<Sign, ConvexHullError> {
        let k = self.shape.k();
        let sign = if k < INLINE {
            let mut ids = [0_u32; INLINE];
            ids[..k].copy_from_slice(self.vertices_of(c));
            ids[k] = q;
            self.lifted_ids(&ids[..=k])?
        } else {
            let mut ids = self.vertices_of(c).to_vec();
            ids.push(q);
            self.lifted_ids(&ids)?
        };
        Ok(sign)
    }

    /// The orientation of simplex `c` with vertex `slot` replaced by `q`.
    fn replaced(&self, c: u32, slot: usize, q: u32) -> Result<Sign, ConvexHullError> {
        let k = self.shape.k();
        if k <= INLINE {
            let mut ids = [0_u32; INLINE];
            ids[..k].copy_from_slice(self.vertices_of(c));
            ids[slot] = q;
            self.orient_ids(&ids[..k])
        } else {
            let mut ids = self.vertices_of(c).to_vec();
            ids[slot] = q;
            self.orient_ids(&ids)
        }
    }

    /// Whether simplex `c` conflicts with `q` (module docs).
    fn conflicts(&self, c: u32, q: u32) -> Result<bool, ConvexHullError> {
        Ok(self.conflict(c, q)?.0)
    }

    /// Whether simplex `c` conflicts with `q`, and for a finite `c`, whether
    /// `c` and `q` are cospherical.
    fn conflict(&self, c: u32, q: u32) -> Result<(bool, Across), ConvexHullError> {
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
            self.across[at..at + k].fill(Across::Unknown);
            self.alive[c as usize] = true;
            self.mark[c as usize] = 0;
            c
        } else {
            self.vertices.extend_from_slice(vertices);
            self.neighbors.extend(core::iter::repeat_n(NONE, k));
            self.across.extend(core::iter::repeat_n(Across::Unknown, k));
            self.alive.push(true);
            self.mark.push(0);
            (self.alive.len() - 1) as u32
        }
    }

    /// Links the initial simplex and its D + 1 outside simplices: two of
    /// them are neighbors across the face they share. There are D + 2, so
    /// comparing every pair of faces costs nothing that grows with the input.
    fn link_initial(&mut self, cells: &[u32]) {
        let k = self.shape.k();
        let face = |mesh: &Self, c: u32, slot: usize| -> Vec<u32> {
            let mut face: Vec<u32> = mesh
                .vertices_of(c)
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != slot)
                .map(|(_, &v)| v)
                .collect();
            face.sort_unstable();
            face
        };
        for (i, &a) in cells.iter().enumerate() {
            for &b in &cells[i + 1..] {
                for sa in 0..k {
                    for sb in 0..k {
                        if face(self, a, sa) == face(self, b, sb) {
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
        order: &[u32],
    ) -> Result<Self, ConvexHullError> {
        let d = sites.dim();
        let k = shape.k();
        debug_assert_eq!(k, d + 1, "the shape is that of the sites' dimension");
        debug_assert_eq!(first.len(), k, "the initial simplex has D + 1 sites");
        let mut mesh = Self {
            shape,
            sites,
            vertices: Vec::with_capacity(order.len() * k * (2 * d)),
            neighbors: Vec::with_capacity(order.len() * k * (2 * d)),
            across: Vec::with_capacity(order.len() * k * (2 * d)),
            alive: Vec::new(),
            mark: Vec::new(),
            free: Vec::new(),
            epoch: 0,
            inside: inside_sign(d)?,
            last: 0,
            stack: Vec::new(),
            cavity: Vec::new(),
            boundary: Vec::new(),
            created: Vec::new(),
            faces: Vec::new(),
            used: Vec::new(),
        };
        let mut simplex = first.to_vec();
        match mesh.orient_ids(&simplex)? {
            Sign::Negative => simplex.swap(0, 1),
            Sign::Positive => {}
            Sign::Zero => {
                debug_assert!(false, "the initial simplex spans dimension D");
            }
        }
        let s = mesh.alloc(&simplex);
        let mut cells = vec![s];
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
        for &q in order {
            mesh.insert(q)?;
        }
        debug_assert!(mesh.linked(), "neighbors are symmetric and share a face");
        Ok(mesh)
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
    fn locate(&self, q: u32) -> Result<u32, ConvexHullError> {
        let k = self.shape.k();
        let mut state = u64::from(q).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
        let mut c = self.last;
        loop {
            if let Some(slot) = self.vertices_of(c).iter().position(|&v| v == INFINITE) {
                if self.conflicts(c, q)? {
                    return Ok(c);
                }
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
                if self.replaced(c, slot, q)? == Sign::Negative {
                    next = Some(self.neighbor(c, slot));
                    break;
                }
            }
            match next {
                Some(n) => c = n,
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
    fn link_by_keys(&mut self, boundary: &[(u32, usize, Across)], created: &[u32]) {
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
                        if other_c == c && self.is_finite(c) {
                            self.across[new as usize * 4 + i] = Across::Distinct;
                            self.across[other_new as usize * 4 + other_slot] = Across::Distinct;
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
    }

    /// Inserts site `q`.
    fn insert(&mut self, q: u32) -> Result<(), ConvexHullError> {
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
            self.link_by_keys(&boundary, &created);
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

/// The insertion order of `sites` (dimension `d`): biased randomized
/// insertion order (BRIO) over rounds of geometric sizes, each round in
/// Morton order of the coordinates quantized over the sites' bounding box.
/// Deterministic: the round of a site comes from a hash of its index, and
/// ties go to the smaller index.
pub(super) fn brio(rows: &Sites, sites: &[u32]) -> Vec<u32> {
    let d = rows.dim();
    let mut low = vec![f64::INFINITY; d];
    let mut high = vec![f64::NEG_INFINITY; d];
    for &s in sites {
        for (j, &x) in rows.point(s).iter().enumerate() {
            low[j] = low[j].min(x);
            high[j] = high[j].max(x);
        }
    }
    let bits = (63 / d.max(1)).min(21) as u32;
    let scale = if bits == 0 {
        0.0
    } else {
        ((1_u64 << bits) - 1) as f64
    };
    let morton = |s: u32| -> u64 {
        let p = rows.point(s);
        // On the stack for D <= 16, so the order allocates nothing per site.
        let cell: Small<u64, 16> = (0..d)
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
    let round = |s: u32| -> u32 {
        // SplitMix64 of the index; about half the sites get 0 (the last
        // round), a quarter 1, and so on.
        let mut z = u64::from(s).wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        z.trailing_zeros().min(32)
    };
    let mut keyed: Vec<(u32, u64, u32)> = sites.iter().map(|&s| (round(s), morton(s), s)).collect();
    keyed.sort_unstable_by_key(|&(r, m, s)| (core::cmp::Reverse(r), m, s));
    keyed.into_iter().map(|(_, _, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hull::input::accept;
    use core::cmp::Ordering;

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
            let input = accept(2, &points).unwrap();
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
