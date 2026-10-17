//! The insertion of D = 2 on half-edges (#667): a triangle holds its three
//! vertices and, for each of its edges, the half-edge across it, so a flip or
//! a split links the triangles around it without looking for the slot that
//! points back. The insertion is that of [`super::insert::Mesh`] by edge
//! flips (design §7, ADR 0007): the same order, walk, predicates, and
//! candidates for cospherical faces; only the storage differs.
//!
//! The half-edge `3 t + i` is the edge of triangle `t` opposite its vertex
//! `i`, from vertex `i + 1` to vertex `i + 2` (indices mod 3). A triangle is
//! positive, so the triangle across an edge holds it reversed. A triangle is
//! never removed: a split and a flip rewrite the triangles they change and
//! add the ones they need.
//!
//! A triangle also holds the coordinates of its vertices, bit for bit the
//! sites', so the first stage of a predicate reads the triangle and the new
//! site and no row of another site; what that stage leaves open goes to the
//! predicate of [`Sites`] on the vertices' ids, as [`Plane`] does.

use super::insert::{inside_sign, Across, Built, Plane, Shape, Sites, INFINITE};
use crate::predicates::{first_stage_fixed, Sign, Start};
use crate::Error;

/// No half-edge: an edge not linked yet.
const NONE: u32 = u32::MAX;

/// A point of the plane.
type Point = [f64; 2];

/// The coordinates held for the vertex at infinity, which no predicate
/// reads: a triangle with that vertex is decided by its finite edge.
const NOWHERE: Point = [0.0; 2];

/// A triangle: its vertices, positive, their coordinates, and the
/// half-edge across each edge.
#[derive(Clone, Copy)]
struct Triangle {
    points: [Point; 3],
    vertices: [u32; 3],
    across: [u32; 3],
}

/// The triangulation of D = 2 being built on half-edges.
pub(super) struct PlaneMesh<'a> {
    sites: &'a Sites,
    triangles: Vec<Triangle>,
    /// The lifted orientation's sign for a point strictly inside the
    /// circumcircle of a positive triangle.
    inside: Sign,
    /// A triangle the last insertion made, where the next walk starts.
    last: u32,
    /// The half-edges opposite the new site still to test.
    stack: Vec<u32>,
    /// Whether a face was ever found cospherical, and the two triangles of
    /// each such face, tested again after the insertion.
    cospherical: bool,
    candidates: Vec<(u32, u32)>,
}

impl<'a> PlaneMesh<'a> {
    /// The triangulation of the sites `order` after the initial triangle
    /// `first` (three affinely independent sites), as
    /// [`super::insert::Mesh::build`] makes it by flips.
    pub(super) fn build(sites: &'a Sites, first: &[u32], order: &[u32]) -> Result<Self, Error> {
        let mut mesh = Self {
            sites,
            triangles: Vec::with_capacity(2 * (order.len() + first.len()) + 4),
            inside: inside_sign(2)?,
            last: 0,
            stack: Vec::with_capacity(32),
            cospherical: false,
            candidates: Vec::new(),
        };
        let mut abc = [first[0], first[1], first[2]];
        match Plane.orient(sites, &abc)? {
            Sign::Negative => abc.swap(0, 1),
            Sign::Positive => {}
            Sign::Zero => {
                debug_assert!(false, "the initial triangle spans the plane");
            }
        }
        let s = mesh.push(abc);
        // The three triangles at infinity, one across each edge of `abc`:
        // the edge reversed and the vertex at infinity, positive.
        let mut cells = [s; 4];
        for j in 0..3 {
            let mut outside = abc;
            outside[j] = INFINITE;
            outside.swap(j, (j + 1) % 3);
            cells[j + 1] = mesh.push(outside);
        }
        mesh.link_initial(&cells);
        mesh.last = s;
        for &q in order {
            mesh.insert(q)?;
        }
        debug_assert!(mesh.linked(), "half-edges are symmetric and share an edge");
        Ok(mesh)
    }

    /// Adds the triangle `vertices`, its edges not linked yet.
    fn push(&mut self, vertices: [u32; 3]) -> u32 {
        let points = vertices.map(|v| self.point(v));
        self.triangles.push(Triangle {
            points,
            vertices,
            across: [NONE; 3],
        });
        (self.triangles.len() - 1) as u32
    }

    /// Adds a triangle a split fills next.
    fn blank(&mut self) -> u32 {
        self.push([INFINITE; 3])
    }

    /// The coordinates of site `v`; [`NOWHERE`] for the vertex at infinity.
    #[inline(always)]
    fn point(&self, v: u32) -> Point {
        if v == INFINITE {
            NOWHERE
        } else {
            self.sites.fixed(v)
        }
    }

    /// Links the four initial triangles: two share an edge when they hold
    /// its two vertices.
    fn link_initial(&mut self, cells: &[u32; 4]) {
        for (x, &a) in cells.iter().enumerate() {
            for &b in &cells[x + 1..] {
                for i in 0..3 {
                    let (p, q) = self.edge(a, i);
                    for j in 0..3 {
                        if self.edge(b, j) == (q, p) {
                            self.link(3 * a + i as u32, 3 * b + j as u32);
                        }
                    }
                }
            }
        }
    }

    /// The edge opposite vertex `i` of triangle `t`, from vertex `i + 1`.
    fn edge(&self, t: u32, i: usize) -> (u32, u32) {
        let v = self.triangles[t as usize].vertices;
        (v[(i + 1) % 3], v[(i + 2) % 3])
    }

    /// Makes `e` and `f` across from each other.
    #[inline(always)]
    fn link(&mut self, e: u32, f: u32) {
        self.triangles[(e / 3) as usize].across[(e % 3) as usize] = f;
        self.triangles[(f / 3) as usize].across[(f % 3) as usize] = e;
    }

    #[inline(always)]
    fn across_of(&self, e: u32) -> u32 {
        self.triangles[(e / 3) as usize].across[(e % 3) as usize]
    }

    /// Sets the vertices of `t`, with their coordinates `points`, and links
    /// each of its edges to `across[i]`.
    #[inline(always)]
    fn set(&mut self, t: u32, vertices: [u32; 3], points: [Point; 3], across: [u32; 3]) {
        debug_assert!(
            vertices
                .iter()
                .zip(&points)
                .all(|(&v, p)| self.point(v) == *p),
            "a triangle holds its vertices' coordinates"
        );
        let triangle = &mut self.triangles[t as usize];
        triangle.vertices = vertices;
        triangle.points = points;
        for (i, &f) in across.iter().enumerate() {
            self.link(3 * t + i as u32, f);
        }
    }

    /// The lifted orientation of finite triangle `t` followed by `q`, at
    /// `pq`: [`Plane`]'s, on the coordinates the triangle holds.
    #[inline(always)]
    fn sphere_sign(&self, t: u32, q: u32, pq: Point) -> Result<Sign, Error> {
        let triangle = &self.triangles[t as usize];
        let [a, b, c] = triangle.points;
        let [x, y, z] = triangle.vertices;
        match first_stage_fixed(&a, &[b, c, pq]) {
            Some(sign) => {
                debug_assert_eq!(Ok(sign), Plane.sphere_sign(self.sites, &[x, y, z], q));
                Ok(sign)
            }
            None => self.sites.lifted(&[x, y, z, q], Start::AfterFirstStage),
        }
    }

    /// Whether finite triangle `t` has `q`, at `pq`, strictly inside its
    /// circumcircle.
    fn in_circle(&self, t: u32, q: u32, pq: Point) -> Result<bool, Error> {
        Ok(self.sphere_sign(t, q, pq)? == self.inside)
    }

    /// The orientation of triangle `t` with vertex `slot` replaced by `q`,
    /// at `pq`: [`Plane`]'s, on the coordinates the triangle holds.
    #[inline(always)]
    fn replaced(&self, t: u32, slot: usize, q: u32, pq: Point) -> Result<Sign, Error> {
        let triangle = &self.triangles[t as usize];
        let mut points = triangle.points;
        let mut ids = triangle.vertices;
        points[slot] = pq;
        ids[slot] = q;
        match first_stage_fixed(&points[0], &[points[1], points[2]]) {
            Some(sign) => {
                debug_assert_eq!(
                    Ok(sign),
                    Plane.replaced(self.sites, &triangle.vertices, slot, q)
                );
                Ok(sign)
            }
            None => self.sites.orient(&ids, Start::AfterFirstStage),
        }
    }

    /// Whether triangle `t` conflicts with `q`, at `pq`, and for a finite
    /// `t`, whether `t` and `q` are cocircular ([`super::insert::Mesh`]'s
    /// conflict).
    fn conflict(&self, t: u32, q: u32, pq: Point) -> Result<(bool, Across), Error> {
        let v = self.triangles[t as usize].vertices;
        match v.iter().position(|&x| x == INFINITE) {
            None => {
                let sign = self.sphere_sign(t, q, pq)?;
                let across = if sign == Sign::Zero {
                    Across::Cospherical
                } else {
                    Across::Distinct
                };
                Ok((sign == self.inside, across))
            }
            Some(slot) => {
                let conflict = match self.replaced(t, slot, q, pq)? {
                    Sign::Positive => true,
                    Sign::Negative => false,
                    Sign::Zero => self.in_circle(self.across_of(3 * t + slot as u32) / 3, q, pq)?,
                };
                Ok((conflict, Across::Unknown))
            }
        }
    }

    /// The triangle where `q`, at `pq`, is located, by the visibility walk
    /// of [`super::insert::Mesh`] from the last triangle made.
    fn locate(&self, q: u32, pq: Point) -> Result<u32, Error> {
        let mut state =
            u64::from(self.sites.input_index(q)).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
        let mut c = self.last;
        let mut came_from = NONE;
        loop {
            let v = self.triangles[c as usize].vertices;
            if let Some(slot) = v.iter().position(|&x| x == INFINITE) {
                if self.conflict(c, q, pq)?.0 {
                    return Ok(c);
                }
                came_from = c;
                c = self.across_of(3 * c + slot as u32) / 3;
                continue;
            }
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let offset = (state % 3) as usize;
            let mut next = None;
            for t in 0..3 {
                let slot = (t + offset) % 3;
                let n = self.across_of(3 * c + slot as u32) / 3;
                if n == came_from {
                    continue;
                }
                if self.replaced(c, slot, q, pq)? == Sign::Negative {
                    next = Some(n);
                    break;
                }
            }
            match next {
                Some(n) => {
                    came_from = c;
                    c = n;
                }
                None => return Ok(c),
            }
        }
    }

    /// Inserts `q` by edge flips: the located triangle split in three, or
    /// with the triangle across in four when `q` is on an edge, and every
    /// edge opposite `q` whose far triangle conflicts flipped.
    fn insert(&mut self, q: u32) -> Result<(), Error> {
        let pq = self.point(q);
        let t = self.locate(q, pq)?;
        let v = self.triangles[t as usize].vertices;
        let on_edge = match v.iter().position(|&x| x == INFINITE) {
            Some(slot) => (self.replaced(t, slot, q, pq)? == Sign::Zero).then_some(slot),
            None => {
                let mut edge = None;
                for slot in 0..3 {
                    if self.replaced(t, slot, q, pq)? == Sign::Zero {
                        edge = Some(slot);
                    }
                }
                edge
            }
        };
        let mut stack = core::mem::take(&mut self.stack);
        stack.clear();
        match on_edge {
            Some(slot) => self.split_edge(t, slot, q, pq, &mut stack),
            None => self.split(t, q, pq, &mut stack),
        }
        while let Some(e) = stack.pop() {
            let c = e / 3;
            let f = self.across_of(e);
            let n = f / 3;
            let (conflict, across) = self.conflict(n, q, pq)?;
            if conflict {
                let (a, b) = self.flip(e, f, q, pq);
                stack.push(a);
                stack.push(b);
            } else if across == Across::Cospherical {
                self.cospherical = true;
                self.candidates.push((c, n));
            }
        }
        self.stack = stack;
        self.last = t;
        Ok(())
    }

    /// Splits `t` into three through `q`; pushes the half-edges opposite `q`.
    fn split(&mut self, t: u32, q: u32, pq: Point, stack: &mut Vec<u32>) {
        let Triangle {
            points: [pa, pb, pc],
            vertices: [a, b, c],
            across: [oa, ob, oc],
        } = self.triangles[t as usize];
        let t2 = self.blank();
        let t3 = self.blank();
        // t = (a, b, q), t2 = (b, c, q), t3 = (c, a, q): q at slot 2 of each.
        self.set(t, [a, b, q], [pa, pb, pq], [3 * t2 + 1, 3 * t3, oc]);
        self.set(t2, [b, c, q], [pb, pc, pq], [3 * t3 + 1, 3 * t, oa]);
        self.set(t3, [c, a, q], [pc, pa, pq], [3 * t + 1, 3 * t2, ob]);
        stack.extend([3 * t + 2, 3 * t2 + 2, 3 * t3 + 2]);
    }

    /// Splits `t` and the triangle across its edge opposite `slot` into four
    /// through `q`, which lies inside that edge.
    fn split_edge(&mut self, t: u32, slot: usize, q: u32, pq: Point, stack: &mut Vec<u32>) {
        let tv = self.triangles[t as usize].vertices;
        let tp = self.triangles[t as usize].points;
        let ta = self.triangles[t as usize].across;
        let (x, p, r) = (tv[slot], tv[(slot + 1) % 3], tv[(slot + 2) % 3]);
        let (px, pp, pr) = (tp[slot], tp[(slot + 1) % 3], tp[(slot + 2) % 3]);
        let t_np = ta[(slot + 1) % 3];
        let t_nr = ta[(slot + 2) % 3];
        let h = ta[slot];
        let (u, su) = (h / 3, (h % 3) as usize);
        // u holds the edge reversed: (y, r, p) from its slot su.
        let uv = self.triangles[u as usize].vertices;
        let ua = self.triangles[u as usize].across;
        let y = uv[su];
        let py = self.triangles[u as usize].points[su];
        let u_np = ua[(su + 2) % 3];
        let u_nr = ua[(su + 1) % 3];
        let t2 = self.blank();
        let t4 = self.blank();
        // t = (x, p, q), t2 = (x, q, r), u = (y, r, q), t4 = (y, q, p).
        self.set(t, [x, p, q], [px, pp, pq], [3 * t4, 3 * t2 + 2, t_nr]);
        self.set(t2, [x, q, r], [px, pq, pr], [3 * u, t_np, 3 * t + 1]);
        self.set(u, [y, r, q], [py, pr, pq], [3 * t2, 3 * t4 + 2, u_np]);
        self.set(t4, [y, q, p], [py, pq, pp], [3 * t, u_nr, 3 * u + 1]);
        stack.extend([3 * t + 2, 3 * t2 + 1, 3 * u + 2, 3 * t4 + 1]);
    }

    /// Flips the edge `e` of the triangle holding `q` opposite it, shared
    /// with `f`: `(q, a, b)` and `(b, a, d)` become `(q, a, d)` and
    /// `(q, d, b)`. Returns their half-edges opposite `q`.
    fn flip(&mut self, e: u32, f: u32, q: u32, pq: Point) -> (u32, u32) {
        let (t, at_q) = (e / 3, (e % 3) as usize);
        let (n, at_d) = (f / 3, (f % 3) as usize);
        let tv = self.triangles[t as usize].vertices;
        let tp = self.triangles[t as usize].points;
        let ta = self.triangles[t as usize].across;
        let nv = self.triangles[n as usize].vertices;
        let na = self.triangles[n as usize].across;
        debug_assert_eq!(tv[at_q], q, "the edge is opposite q");
        let (a, b) = (tv[(at_q + 1) % 3], tv[(at_q + 2) % 3]);
        let (pa, pb) = (tp[(at_q + 1) % 3], tp[(at_q + 2) % 3]);
        let d = nv[at_d];
        let pd = self.triangles[n as usize].points[at_d];
        let t_na = ta[(at_q + 2) % 3];
        let t_nb = ta[(at_q + 1) % 3];
        // n is (d, b, a) from at_d.
        let n_ad = na[(at_d + 1) % 3];
        let n_db = na[(at_d + 2) % 3];
        self.set(t, [q, a, d], [pq, pa, pd], [n_ad, 3 * n + 2, t_na]);
        self.set(n, [q, d, b], [pq, pd, pb], [n_db, t_nb, 3 * t + 1]);
        (3 * t, 3 * n)
    }

    /// Whether every half-edge's across points back and holds the edge
    /// reversed. A debug check.
    fn linked(&self) -> bool {
        (0..self.triangles.len() as u32).all(|t| {
            (0..3).all(|i| {
                let e = 3 * t + i;
                let f = self.across_of(e);
                f != NONE
                    && self.across_of(f) == e
                    && self.edge(t, i as usize) == {
                        let (p, q) = self.edge(f / 3, (f % 3) as usize);
                        (q, p)
                    }
            })
        })
    }
}

impl Built for PlaneMesh<'_> {
    const FLIPS: bool = true;

    fn slots(&self) -> usize {
        self.triangles.len()
    }

    fn finite_cells(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.triangles.len() as u32).filter(|&t| self.is_finite(t))
    }

    fn neighbor(&self, c: u32, slot: usize) -> u32 {
        self.triangles[c as usize].across[slot] / 3
    }

    fn across(&self, _c: u32, _slot: usize) -> Across {
        Across::Unknown
    }

    fn is_finite(&self, c: u32) -> bool {
        !self.triangles[c as usize].vertices.contains(&INFINITE)
    }

    fn vertices_of(&self, c: u32) -> &[u32] {
        &self.triangles[c as usize].vertices
    }

    fn lifted_ids(&self, ids: &[u32]) -> Result<Sign, Error> {
        Plane.lifted(self.sites, ids)
    }

    fn orient_ids(&self, ids: &[u32]) -> Result<Sign, Error> {
        Plane.orient(self.sites, ids)
    }

    fn candidates(&self) -> &[(u32, u32)] {
        &self.candidates
    }

    fn meet(&self, c: u32, n: u32) -> bool {
        (c as usize) < self.triangles.len()
            && (n as usize) < self.triangles.len()
            && self.triangles[c as usize]
                .across
                .iter()
                .any(|&f| f / 3 == n)
    }

    fn recorded_cospherical(&self) -> bool {
        self.cospherical
    }

    fn left_undecided(&self) -> bool {
        false
    }
}
