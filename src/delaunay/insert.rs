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

use crate::hull::input::Input;
use crate::hull::ridge::{fingerprint, pair_equal_keys};
use crate::hull::ConvexHullError;
use crate::predicates::{orient, orient_lifted_with, LiftedHeight, Sign};

/// The vertex at infinity of an outside simplex. Site numbers are below
/// `u32::MAX`, so it names no site.
pub(super) const INFINITE: u32 = u32::MAX;

/// No simplex: a free slot's neighbor before linking.
const NONE: u32 = u32::MAX;

/// Up to this many points, a predicate gathers its rows on the stack.
const INLINE: usize = 18;

/// The triangulation being built: `k = D + 1` vertices and neighbors per
/// simplex, in flat arrays with stride `k`. `neighbors[c * k + i]` is the
/// simplex across the face opposite vertex `i` of simplex `c`.
pub(super) struct Mesh<'a> {
    sites: &'a Sites,
    k: usize,
    vertices: Vec<u32>,
    neighbors: Vec<u32>,
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
    boundary: Vec<(u32, usize)>,
    created: Vec<u32>,
    keys: Vec<u32>,
    owners: Vec<(u32, usize)>,
}

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

    /// Orientation of the sites `ids` (D + 1 of them).
    fn orient(&self, ids: &[u32]) -> Result<Sign, ConvexHullError> {
        let n = ids.len();
        if n <= INLINE {
            let mut points: [&[f64]; INLINE] = [&[]; INLINE];
            for (slot, &i) in points.iter_mut().zip(ids) {
                *slot = self.point(i);
            }
            Ok(orient(&points[..n])?)
        } else {
            let points: Vec<&[f64]> = ids.iter().map(|&i| self.point(i)).collect();
            Ok(orient(&points)?)
        }
    }

    /// Lifted orientation of the sites `ids` (D + 2 of them).
    pub(super) fn lifted(&self, ids: &[u32]) -> Result<Sign, ConvexHullError> {
        let n = ids.len();
        if n <= INLINE {
            let mut points: [&[f64]; INLINE] = [&[]; INLINE];
            let mut heights = [LiftedHeight::of(&[]); INLINE];
            for ((slot, height), &i) in points.iter_mut().zip(&mut heights).zip(ids) {
                *slot = self.point(i);
                *height = self.height(i);
            }
            Ok(orient_lifted_with(&points[..n], &heights[..n])?)
        } else {
            let points: Vec<&[f64]> = ids.iter().map(|&i| self.point(i)).collect();
            let heights: Vec<LiftedHeight> = ids.iter().map(|&i| self.height(i)).collect();
            Ok(orient_lifted_with(&points, &heights)?)
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

impl<'a> Mesh<'a> {
    pub(super) fn vertices_of(&self, c: u32) -> &[u32] {
        let k = self.k;
        &self.vertices[c as usize * k..(c as usize + 1) * k]
    }

    pub(super) fn neighbor(&self, c: u32, slot: usize) -> u32 {
        self.neighbors[c as usize * self.k + slot]
    }

    fn set_neighbor(&mut self, c: u32, slot: usize, n: u32) {
        let k = self.k;
        self.neighbors[c as usize * k + slot] = n;
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
        self.sites.orient(ids)
    }

    /// Lifted orientation of the sites `ids` (D + 2 of them).
    pub(super) fn lifted_ids(&self, ids: &[u32]) -> Result<Sign, ConvexHullError> {
        self.sites.lifted(ids)
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
        let k = self.k;
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
        Ok(sign == self.inside)
    }

    /// The orientation of simplex `c` with vertex `slot` replaced by `q`.
    fn replaced(&self, c: u32, slot: usize, q: u32) -> Result<Sign, ConvexHullError> {
        let k = self.k;
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
        match self.vertices_of(c).iter().position(|&v| v == INFINITE) {
            None => self.in_sphere(c, q),
            Some(slot) => match self.replaced(c, slot, q)? {
                Sign::Positive => Ok(true),
                Sign::Negative => Ok(false),
                Sign::Zero => self.in_sphere(self.neighbor(c, slot), q),
            },
        }
    }

    fn alloc(&mut self, vertices: &[u32]) -> u32 {
        let k = self.k;
        if let Some(c) = self.free.pop() {
            let at = c as usize * k;
            self.vertices[at..at + k].copy_from_slice(vertices);
            self.neighbors[at..at + k].fill(NONE);
            self.alive[c as usize] = true;
            self.mark[c as usize] = 0;
            c
        } else {
            self.vertices.extend_from_slice(vertices);
            self.neighbors.extend(core::iter::repeat_n(NONE, k));
            self.alive.push(true);
            self.mark.push(0);
            (self.alive.len() - 1) as u32
        }
    }

    /// Links the faces of `cells` that are still unlinked, by their vertex
    /// sets: two simplices that share a face are neighbors across it. Every
    /// unlinked face of `cells` lies in exactly two of them.
    fn link(&mut self, cells: &[u32]) {
        let k = self.k;
        let mut keys = core::mem::take(&mut self.keys);
        let mut owners = core::mem::take(&mut self.owners);
        keys.clear();
        owners.clear();
        for &c in cells {
            for slot in 0..k {
                if self.neighbor(c, slot) != NONE {
                    continue;
                }
                let start = keys.len();
                keys.extend(
                    self.vertices_of(c)
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != slot)
                        .map(|(_, &v)| v),
                );
                keys[start..].sort_unstable();
                owners.push((c, slot));
            }
        }
        for (a, b) in pair_equal_keys(&keys, owners.len(), fingerprint) {
            let ((ca, sa), (cb, sb)) = (owners[a], owners[b]);
            self.set_neighbor(ca, sa, cb);
            self.set_neighbor(cb, sb, ca);
        }
        self.keys = keys;
        self.owners = owners;
    }

    /// The triangulation of the sites `order` after the initial simplex
    /// `first` (D + 1 affinely independent sites).
    pub(super) fn build(
        sites: &'a Sites,
        first: &[u32],
        order: &[u32],
    ) -> Result<Self, ConvexHullError> {
        let d = sites.dim();
        let k = d + 1;
        debug_assert_eq!(first.len(), k, "the initial simplex has D + 1 sites");
        let mut mesh = Self {
            sites,
            k,
            vertices: Vec::with_capacity(order.len() * k * (2 * d)),
            neighbors: Vec::with_capacity(order.len() * k * (2 * d)),
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
            keys: Vec::new(),
            owners: Vec::new(),
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
        mesh.link(&cells);
        mesh.last = s;
        for &q in order {
            mesh.insert(q)?;
        }
        Ok(mesh)
    }

    /// The simplex where `q` is located: one that conflicts with it. A
    /// visibility walk from the last created simplex; the face to cross is
    /// tried from a position that depends on `q`, which keeps the walk from
    /// cycling.
    fn locate(&self, q: u32) -> Result<u32, ConvexHullError> {
        let k = self.k;
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

    /// Inserts site `q`.
    fn insert(&mut self, q: u32) -> Result<(), ConvexHullError> {
        let k = self.k;
        let start = self.locate(q)?;
        debug_assert!(self.conflicts(start, q)?, "the located simplex conflicts");
        self.epoch = self.epoch.wrapping_add(1);
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
                if self.conflicts(n, q)? {
                    self.mark[n as usize] = epoch;
                    stack.push(n);
                    cavity.push(n);
                } else {
                    boundary.push((c, slot));
                }
            }
        }
        let mut vertices = [0_u32; INLINE];
        let mut spill = Vec::new();
        for &(c, slot) in &boundary {
            let row: &mut [u32] = if k <= INLINE {
                &mut vertices[..k]
            } else {
                spill.resize(k, 0);
                &mut spill
            };
            row.copy_from_slice(self.vertices_of(c));
            row[slot] = q;
            let outside = self.neighbor(c, slot);
            let new = if k <= INLINE {
                self.alloc(&vertices[..k])
            } else {
                self.alloc(&spill)
            };
            self.set_neighbor(new, slot, outside);
            let back = (0..k).find(|&s| self.neighbor(outside, s) == c);
            debug_assert!(back.is_some(), "neighbors are symmetric");
            if let Some(back) = back {
                self.set_neighbor(outside, back, new);
            }
            debug_assert!(
                !self.is_finite(new) || self.orient_ids(self.vertices_of(new))? == Sign::Positive,
                "a new finite simplex is positive"
            );
            created.push(new);
        }
        self.link(&created);
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
                                rows.lifted(&indices).unwrap(),
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
