//! Hull construction, kept simplicial (design §4, §6).
//!
//! D = 1 is the two endpoints. D = 2 is the chain of strict turns, after the
//! points proved inside the polygon of the directional extremes are set
//! aside; it is not stored as one simplex per edge. D >= 3 is Quickhull.
//!
//! Every facet is a (D-1)-simplex of D vertices, kept in a [`FacetStore`].
//! `neighbors[i]` is the facet across the ridge opposite `vertices[i]`. A
//! point is outside a facet when the orientation of the facet's vertices
//! followed by the point has the facet's `outward` sign; for D >= 2 the
//! vertex order is chosen so that this sign is always [`Sign::Positive`],
//! and only the two endpoints of D = 1 need a stored sign. Visibility is
//! decided by that exact orientation alone. No facets are merged during
//! insertion.
//!
//! The sequential build inserts one point at a time and changes the hull in
//! place (design §6): it takes a candidate of the facets with outside
//! points from the farthest bucket of working distances ([`Candidates`]),
//! plans its insertion against the hull as it is into reusable flat lists,
//! and applies it at once. There is no round and no reservation.
//!
//! It is the only build (design §6, ADR 0004).

use std::collections::VecDeque;

use super::input::Input;
use super::ridge::{fingerprint, pair_equal_keys_into};
use super::store::{Facet, FacetStore};
use super::ConvexHullError;
use crate::arena::FacetId;
use crate::cull::CullPlane;
use crate::normal::{facet_cofactors, facet_cofactors_in_lanes, working_normal};
use crate::predicates::{Sign, COFACTOR_LANES};
use crate::small::Small;

/// The simplicial hull after every point has been absorbed.
pub(crate) struct SimplicialHull<'a> {
    pub(crate) input: Input<'a>,
    pub(crate) facets: FacetStore,
    /// Points construction dropped with every sign it tested strictly
    /// negative: strictly inside the hull at that step, so in the interior
    /// of the final hull (design §3). Unordered.
    pub(crate) proved_interior: Vec<u32>,
    /// The edges are a strict polygon: adjacent edges are not collinear, so
    /// the coplanar merge does not need to test them.
    pub(crate) strict_edges: bool,
    /// Extreme vertices counterclockwise, when [`Self::strict_edges`].
    pub(crate) polygon: Vec<u32>,
}

/// The most representatives [`SimplicialHull::discarding_pays`] samples.
const DISCARD_SAMPLE: usize = 1024;

/// Discarding goes ahead when at least one sampled point in this many is
/// proved inside. Timed on a unit circle with a share of its points moved
/// inside (10^5 and 10^6 points, #232): with one point in 64 inside,
/// discarding was 1 to 7% slower than not; with one in 32, the two were
/// equal; with one in 16, discarding was 3 to 8% faster.
const DISCARD_ONE_IN: usize = 32;

/// What decides the side of a point against a simplex: its vertices, its
/// outward sign, and its certified cull plane, when it has one. A facet of
/// the store and a simplex of a plan not yet applied are both read this
/// way.
struct Geometry<'a> {
    vertices: &'a [u32],
    outward: Sign,
    cull: Option<CullPlane<&'a [f64]>>,
    normal: Option<&'a [f64]>,
}

impl<'a> From<Facet<'a>> for Geometry<'a> {
    fn from(facet: Facet<'a>) -> Self {
        Self {
            vertices: facet.vertices(),
            outward: facet.outward(),
            cull: facet.cull(),
            normal: facet.normal(),
        }
    }
}

/// The working planes of simplices, one entry per simplex: the working unit
/// normal (D entries) and, when certified, the cull threshold.
#[derive(Default)]
struct Planes {
    normals: Vec<f64>,
    /// Whether the simplex has a working normal, and its cull threshold.
    kinds: Vec<(bool, Option<(f64, f64)>)>,
}

impl Planes {
    fn clear(&mut self) {
        self.normals.clear();
        self.kinds.clear();
    }

    /// The geometry of simplex `k`, whose vertices are `vertices`, with
    /// the outward sign `outward`.
    fn geometry<'a>(&'a self, k: usize, vertices: &'a [u32], outward: Sign) -> Geometry<'a> {
        let d = vertices.len();
        let normal = &self.normals[k * d..(k + 1) * d];
        let (has_normal, cull) = self.kinds[k];
        Geometry {
            vertices,
            outward,
            cull: cull.map(|(slope, floor)| CullPlane::from_parts(normal, slope, floor)),
            normal: has_normal.then_some(normal),
        }
    }

    /// Writes simplex `k`'s plane into the store's facet `slot`.
    fn store(&self, k: usize, d: usize, facets: &mut FacetStore, slot: u32) {
        let (has_normal, cull) = self.kinds[k];
        let normal = has_normal.then(|| &self.normals[k * d..(k + 1) * d]);
        facets.set_plane(slot, normal, cull);
    }
}

impl<'a> SimplicialHull<'a> {
    /// Builds the simplicial hull of an accepted input.
    pub(crate) fn build(input: Input<'a>) -> Result<Self, ConvexHullError> {
        let dim = input.dim();
        let mut hull = Self {
            input,
            facets: FacetStore::new(dim),
            proved_interior: Vec::new(),
            strict_edges: false,
            polygon: Vec::new(),
        };
        if dim == 1 {
            hull.build_segment()?;
        } else if dim == 2 {
            hull.build_polygon()?;
        } else {
            let initial = hull.initial_simplex()?;
            let candidates: Vec<u32> = hull
                .input
                .representatives
                .iter()
                .copied()
                .filter(|p| !hull.input.spanning_points.contains(p))
                .collect();
            hull.assign(candidates, &initial)?;
            hull.absorb_in_place()?;
        }
        Ok(hull)
    }

    /// Engine-space coordinates of `vertices` for working normals.
    fn coords_of(&self, vertices: &[u32]) -> Small<&[f64], 10> {
        vertices.iter().map(|&v| self.input.point(v)).collect()
    }

    /// The exact side of `point` relative to `facet`: [`Sign::Positive`] is
    /// strictly outside, [`Sign::Zero`] on the supporting hyperplane.
    pub(crate) fn side(&self, facet: Facet<'_>, point: u32) -> Result<Sign, ConvexHullError> {
        side(&self.input, &facet.into(), point)
    }

    /// Evaluates the working normal and the cull plane of each simplex of
    /// `vertices` (D per simplex), all with the outward sign `outward`,
    /// into `planes`, cleared first. The cofactors of four simplices at a
    /// time share one elimination in lanes, bit for bit those of each alone.
    fn plan_planes(
        &self,
        vertices: &[u32],
        outward: &[Sign],
        planes: &mut Planes,
    ) -> Result<(), ConvexHullError> {
        let d = self.input.dim();
        planes.clear();
        let count = outward.len();
        let simplex = |k: usize| &vertices[k * d..(k + 1) * d];
        let full = count - count % COFACTOR_LANES;
        for first in (0..full).step_by(COFACTOR_LANES) {
            let points: [Small<&[f64], 10>; COFACTOR_LANES] =
                core::array::from_fn(|lane| self.coords_of(simplex(first + lane)));
            // Input coordinates are finite (design §3), so every facet's
            // cofactors can be evaluated.
            let cofactors = facet_cofactors_in_lanes(points.each_ref().map(|p| &**p));
            for (lane, (points, cofactors)) in points.iter().zip(&cofactors).enumerate() {
                Self::push_plane(points, outward[first + lane], cofactors.as_deref(), planes)?;
            }
        }
        for (k, &outward) in outward.iter().enumerate().skip(full) {
            let points = self.coords_of(simplex(k));
            let cofactors = facet_cofactors(&points);
            Self::push_plane(&points, outward, cofactors.as_deref(), planes)?;
        }
        Ok(())
    }

    /// Appends the plane of one simplex at `points` to `planes`, with the
    /// [`facet_cofactors`] of those points.
    fn push_plane(
        points: &[&[f64]],
        outward: Sign,
        cofactors: Option<&[(f64, f64)]>,
        planes: &mut Planes,
    ) -> Result<(), ConvexHullError> {
        let d = points.len();
        // The cofactors certify both the working normal and the cull plane;
        // they are evaluated once (#86). The working normal is the certified
        // cofactor direction, the same direction a published plane takes
        // from its own basis (design §1).
        let normal = working_normal(points, outward, cofactors)?;
        match normal {
            Some(normal) => {
                let cull = CullPlane::with_cofactors(points, &normal, outward, cofactors)
                    .map(|c| (c.slope(), c.floor()));
                planes.normals.extend_from_slice(&normal);
                planes.kinds.push((true, cull));
            }
            None => {
                planes.normals.extend(core::iter::repeat_n(0.0, d));
                planes.kinds.push((false, None));
            }
        }
        Ok(())
    }

    /// Adds simplices on `vertices` (D per simplex) with the outward signs
    /// `outward` and their planes, and returns their slots. Their neighbors
    /// are left for the caller.
    fn add_simplices(
        &mut self,
        vertices: &[u32],
        outward: &[Sign],
    ) -> Result<Vec<u32>, ConvexHullError> {
        let d = self.input.dim();
        let mut planes = Planes::default();
        self.plan_planes(vertices, outward, &mut planes)?;
        let mut slots = Vec::with_capacity(outward.len());
        for (k, (vertices, &outward)) in vertices.chunks_exact(d).zip(outward).enumerate() {
            let slot = self.facets.alloc(vertices, outward);
            planes.store(k, d, &mut self.facets, slot);
            slots.push(slot);
        }
        Ok(slots)
    }

    /// D = 1: the hull is the two extreme representatives.
    fn build_segment(&mut self) -> Result<(), ConvexHullError> {
        let reps = &self.input.representatives;
        let coordinate = |i: u32| self.input.point(i)[0];
        let mut low = reps[0];
        let mut high = reps[0];
        for &r in &reps[1..] {
            if coordinate(r) < coordinate(low) {
                low = r;
            }
            if coordinate(r) > coordinate(high) {
                high = r;
            }
        }
        self.add_simplices(&[low, high], &[Sign::Negative, Sign::Positive])?;
        Ok(())
    }

    /// The polygon that discards points before the D = 2 chain (design §6):
    /// the strict chain of the farthest representative in each of eight
    /// directions, counterclockwise. A tie keeps the smaller index. Fewer
    /// than three vertices mean no polygon, and the list is empty.
    ///
    /// The directions are compared as rounded sums and differences. Any
    /// input points make a valid polygon, so the rounding only changes
    /// which ones it takes.
    fn discard_polygon(&self) -> Result<Vec<u32>, ConvexHullError> {
        let mut best = [f64::NEG_INFINITY; 8];
        let mut farthest = [None; 8];
        for &r in &self.input.representatives {
            let p = self.input.point(r);
            let (x, y) = (p[0], p[1]);
            let keys = [x, -x, y, -y, x + y, x - y, y - x, -x - y];
            for ((key, best), farthest) in keys.into_iter().zip(&mut best).zip(&mut farthest) {
                // Representatives ascend, so a strict test keeps the
                // smaller index on a tie.
                if key > *best {
                    *best = key;
                    *farthest = Some(r);
                }
            }
        }
        let mut extremes: Vec<u32> = farthest.into_iter().flatten().collect();
        extremes.sort_unstable();
        extremes.dedup();
        let cycle = self.strict_cycle(extremes)?;
        Ok(if cycle.len() < 3 { Vec::new() } else { cycle })
    }

    /// `point` is a strict left turn of every edge of the counterclockwise
    /// convex `cycle`, by the certified filter: strictly inside it.
    fn clearly_inside(&self, cycle: &[u32], point: u32) -> bool {
        let n = cycle.len();
        let c = self.input.point(point);
        for i in 0..n {
            let a = self.input.point(cycle[i]);
            let b = self.input.point(cycle[(i + 1) % n]);
            if crate::predicates::orient2_filter(a, b, c) != Some(Sign::Positive) {
                return false;
            }
        }
        true
    }

    /// Whether discarding by `polygon` pays (design §6): of a stride of at
    /// most [`DISCARD_SAMPLE`] representatives in input order, at least one
    /// in [`DISCARD_ONE_IN`] is proved strictly inside.
    ///
    /// A point that is not discarded costs its place in the sort and its
    /// classification; one that is tested and not discarded costs the tests
    /// as well. Where every point is extreme, as on a circle, the pass
    /// found nothing and cost 4 to 6% of the build (#232).
    fn discarding_pays(&self, polygon: &[u32]) -> bool {
        let reps = &self.input.representatives;
        let step = (reps.len() / DISCARD_SAMPLE).max(1);
        let (mut seen, mut inside) = (0usize, 0usize);
        for &r in reps.iter().step_by(step).take(DISCARD_SAMPLE) {
            seen += 1;
            inside += usize::from(self.clearly_inside(polygon, r));
        }
        inside * DISCARD_ONE_IN >= seen
    }

    /// D = 2: the cycle of strict left turns, counterclockwise (design §6).
    ///
    /// A representative proved strictly inside [`Self::discard_polygon`] is
    /// strictly inside the hull, because the polygon's vertices are input
    /// points. It is recorded as proved interior and takes no further part.
    /// Where that would not pay ([`Self::discarding_pays`]), no point is
    /// tested, and the chain and classification handle them all.
    /// Of the others, a representative stays on the chain only while it
    /// makes a strict turn under the exact orientation. Points on an edge
    /// or inside the polygon are left for classification, the same split
    /// [`Self::build_segment`] uses.
    fn build_polygon(&mut self) -> Result<(), ConvexHullError> {
        self.strict_edges = true;
        let discard = self.discard_polygon()?;
        let mut kept = Vec::new();
        if discard.is_empty() || !self.discarding_pays(&discard) {
            kept.clone_from(&self.input.representatives);
        } else {
            for &r in &self.input.representatives {
                if self.clearly_inside(&discard, r) {
                    self.proved_interior.push(r);
                } else {
                    kept.push(r);
                }
            }
        }
        let cycle = self.strict_cycle(kept)?;
        debug_assert!(
            cycle.len() >= 3,
            "a full-dimensional set has at least three extremes"
        );
        self.polygon = cycle;
        Ok(())
    }

    /// The cycle of strict left turns of `points`, counterclockwise: the
    /// lower chain, then the upper chain without the two endpoints the
    /// lower one already lists. Fewer than three points come back as they
    /// are, sorted.
    fn strict_cycle(&self, mut points: Vec<u32>) -> Result<Vec<u32>, ConvexHullError> {
        points.sort_by(|&a, &b| {
            let pa = self.input.point(a);
            let pb = self.input.point(b);
            pa[0]
                .total_cmp(&pb[0])
                .then(pa[1].total_cmp(&pb[1]))
                .then(a.cmp(&b))
        });
        let mut lower = Vec::new();
        for &point in &points {
            self.pop_until_left(&mut lower, point)?;
            lower.push(point);
        }
        let mut upper = Vec::new();
        for &point in points.iter().rev() {
            self.pop_until_left(&mut upper, point)?;
            upper.push(point);
        }
        let mut cycle = lower;
        let upper_middle = upper.len().saturating_sub(2);
        cycle.extend(upper.into_iter().skip(1).take(upper_middle));
        Ok(cycle)
    }

    /// Drops the tail while `point` is not a strict left turn from it.
    fn pop_until_left(&self, chain: &mut Vec<u32>, point: u32) -> Result<(), ConvexHullError> {
        while chain.len() >= 2 {
            let a = chain[chain.len() - 2];
            let b = chain[chain.len() - 1];
            let (pa, pb, pc) = (
                self.input.point(a),
                self.input.point(b),
                self.input.point(point),
            );
            let sign = if let Some(sign) = crate::predicates::orient2_filter(pa, pb, pc) {
                #[cfg(debug_assertions)]
                debug_assert_eq!(sign, self.input.orient(&[a, b, point])?);
                sign
            } else {
                self.input.orient(&[a, b, point])?
            };
            if sign == Sign::Positive {
                break;
            }
            chain.pop();
        }
        Ok(())
    }

    /// The D + 1 facets of the simplex on `spanning_points`, each ordered so
    /// that the opposite vertex is on the negative side, as slots.
    fn initial_simplex(&mut self) -> Result<Vec<u32>, ConvexHullError> {
        let simplex = self.input.spanning_points.clone();
        let mut ordered: Vec<Vec<u32>> = Vec::with_capacity(simplex.len());
        for &apex in &simplex {
            let mut vertices: Vec<u32> = simplex.iter().copied().filter(|&v| v != apex).collect();
            let mut indices = vertices.clone();
            indices.push(apex);
            if self.input.orient(&indices)? == Sign::Positive {
                vertices.swap(0, 1);
            }
            ordered.push(vertices);
        }
        let flat: Vec<u32> = ordered.iter().flatten().copied().collect();
        let slots = self.add_simplices(&flat, &vec![Sign::Positive; ordered.len()])?;
        // The facet omitting simplex[i] is slots[i]; across the ridge
        // opposite vertex v lies the facet omitting v.
        for (i, vertices) in ordered.iter().enumerate() {
            for (slot, v) in vertices.iter().enumerate() {
                let omitted = simplex.iter().position(|s| s == v);
                debug_assert!(omitted.is_some(), "a facet's vertex is in the simplex");
                let across = slots[omitted.unwrap_or(0)];
                self.facets.neighbors_mut(slots[i])[slot] = across;
            }
        }
        Ok(slots)
    }

    /// Assigns each candidate to the first facet in `facets`, every facet of
    /// the initial simplex, that it is strictly outside. Candidates outside
    /// none are dropped: they are inside the current hull or on its
    /// boundary. Those strictly inside every facet are recorded in
    /// [`Self::proved_interior`].
    fn assign(&mut self, mut remaining: Vec<u32>, facets: &[u32]) -> Result<(), ConvexHullError> {
        let mut strict = vec![true; remaining.len()];
        let mut sides = Vec::new();
        let mut outside = Vec::new();
        for &slot in facets {
            if remaining.is_empty() {
                break;
            }
            outside.clear();
            let farthest = take_outside(
                &self.input,
                &mut remaining,
                &mut strict,
                &mut sides,
                None,
                &self.facets.facet(slot).into(),
                &mut outside,
            )?;
            self.facets.set_outside(slot, &outside, farthest);
        }
        self.proved_interior.extend(
            remaining
                .iter()
                .zip(&strict)
                .filter(|&(_, &s)| s)
                .map(|(&p, _)| p),
        );
        Ok(())
    }

    /// The candidate of every facet with outside points, for the queue of
    /// [`Self::absorb_in_place`].
    fn all_candidates(&self) -> Candidates {
        let mut pending = Candidates::default();
        for (id, facet) in self.facets.iter() {
            if let Some((point, distance)) = facet.farthest() {
                pending.push(Pending {
                    point,
                    facet: id,
                    distance,
                });
            }
        }
        pending
    }

    /// The sequential build (design §6): while a facet has outside points,
    /// inserts the next candidate of the queue, against the hull as it is,
    /// and applies the insertion at once.
    ///
    /// `pending` holds the candidate of every facet with outside points,
    /// possibly with entries of removed facets, which are dropped when they
    /// are taken. A live facet's candidate never changes, because its
    /// outside set is final (see [`take_outside`]).
    fn absorb_in_place(&mut self) -> Result<(), ConvexHullError> {
        let mut pending = self.all_candidates();
        let mut visited = Marks::default();
        let mut cone = Cone::default();
        let mut slots = Vec::new();
        while let Some(candidate) = next_candidate(&mut pending) {
            let Some(start) = self.facets.slot_of(candidate.facet) else {
                continue;
            };
            self.walk_region(start, candidate.point, &mut visited, &mut cone.region)?;
            self.plan_region(candidate.point, &mut cone)?;
            self.commit(&mut cone, &mut slots);
            self.push_candidates(&slots, &mut pending);
        }
        Ok(())
    }

    /// Adds the candidates of the facets in `slots` to `pending`.
    fn push_candidates(&self, slots: &[u32], pending: &mut Candidates) {
        for &slot in slots {
            let facet = self.facets.facet(slot);
            if let Some((point, distance)) = facet.farthest() {
                pending.push(Pending {
                    point,
                    facet: facet.id(),
                    distance,
                });
            }
        }
    }

    /// The facets visible from `apex`, found by breadth-first search over
    /// neighbors from `start`, and the horizon, written to `region`, cleared
    /// here. `visited` is scratch, cleared here.
    fn walk_region(
        &self,
        start: u32,
        apex: u32,
        visited: &mut Marks,
        region: &mut Region,
    ) -> Result<(), ConvexHullError> {
        visited.clear(self.facets.slots());
        let Region { visible, horizon } = region;
        visible.clear();
        horizon.clear();
        visible.push(start);
        visited.set(start, true);
        let mut cursor = 0;
        while cursor < visible.len() {
            let slot = visible[cursor];
            cursor += 1;
            let facet = self.facets.facet(slot);
            for (m, &neighbor) in facet.neighbor_slots().iter().enumerate() {
                let seen = match visited.get(neighbor) {
                    Some(v) => v,
                    None => {
                        let v = self.side(self.facets.facet(neighbor), apex)? == Sign::Positive;
                        visited.set(neighbor, v);
                        if v {
                            visible.push(neighbor);
                        }
                        v
                    }
                };
                if !seen {
                    let back = self
                        .facets
                        .facet(neighbor)
                        .neighbor_slots()
                        .iter()
                        .position(|&b| b == slot);
                    debug_assert!(back.is_some(), "the facet across points back");
                    let back = back.unwrap_or(0);
                    horizon.push(Horizon {
                        visible: slot,
                        slot: m as u32,
                        across: neighbor,
                        back: back as u32,
                    });
                }
            }
        }
        Ok(())
    }

    /// Plans the insertion of `apex` whose visible region is in
    /// `created.region`, without changing the hull: the cone from `apex`
    /// over the horizon, with links between the new simplices by local
    /// number, their planes, and the outside points of the visible facets
    /// reassigned to them. Every list of `created` is cleared first and
    /// keeps its storage, so a plan allocates only when one of them grows
    /// (#209).
    fn plan_region(&self, apex: u32, created: &mut Cone) -> Result<(), ConvexHullError> {
        let Cone {
            region: Region { visible, horizon },
            scratch:
                PlanScratch {
                    keys,
                    owners,
                    table,
                    pairs,
                    orphans,
                    lost,
                    strict,
                    sides,
                    kept,
                    copied,
                    outward,
                },
            vertices,
            links,
            planes,
            outside,
            ranges,
            farthest,
            interior,
        } = created;
        let (visible, horizon) = (&*visible, &*horizon);
        let dim = self.input.dim();

        // One new simplex per horizon ridge: the visible facet's vertex order
        // with the vertex opposite the ridge replaced by the apex keeps the
        // outward orientation.
        vertices.clear();
        links.clear();
        keys.clear();
        owners.clear();
        // Each ridge between two new simplices holds the apex and D - 2
        // vertices of a horizon ridge. Its key is those D - 2 vertices,
        // sorted, in one flat buffer; sorting the keys pairs the two
        // simplices that share each ridge, with no hashing and no
        // allocation per key.
        for (k, h) in horizon.iter().enumerate() {
            let start = vertices.len();
            vertices.extend_from_slice(self.facets.facet(h.visible).vertices());
            vertices[start + h.slot as usize] = apex;
            // The horizon ridge, sorted once with each vertex's slot; the
            // key of the ridge opposite one of them is the rest, still
            // sorted.
            let mut ridge: Small<(u32, usize), 8> = vertices[start..]
                .iter()
                .enumerate()
                .filter(|&(m, _)| m != h.slot as usize)
                .map(|(m, &v)| (v, m))
                .collect();
            ridge.sort_unstable();
            for (skip, &(_, other)) in ridge.iter().enumerate() {
                keys.extend(
                    ridge
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != skip)
                        .map(|(_, &(v, _))| v),
                );
                owners.push((k, other));
            }
            links.extend((0..dim).map(|_| Link::Old(h.across)));
        }
        let count = horizon.len();
        outward.clear();
        outward.resize(count, Sign::Positive);
        self.plan_planes(vertices, outward, planes)?;
        pair_equal_keys_into(keys, owners.len(), fingerprint, table, pairs);
        for &(first, second) in pairs.iter() {
            let (a, a_slot) = owners[first];
            let (b, b_slot) = owners[second];
            links[a * dim + a_slot] = Link::New(b as u32);
            links[b * dim + b_slot] = Link::New(a as u32);
        }

        // Reassign the outside points of the visible facets.
        orphans.clear();
        for &slot in visible {
            orphans.extend(
                self.facets
                    .facet(slot)
                    .outside()
                    .iter()
                    .copied()
                    .filter(|&p| p != apex),
            );
        }
        // A vertex of only visible facets stops being a vertex. It is
        // proved interior on the same terms as an orphan (design §3).
        // Size the set from the vertices this plan inserts.
        kept.fill(vertices.iter().copied(), vertices.len());
        lost.clear();
        for &slot in visible {
            lost.extend(
                self.facets
                    .facet(slot)
                    .vertices()
                    .iter()
                    .copied()
                    .filter(|&v| !kept.contains(v)),
            );
        }
        lost.sort_unstable();
        lost.dedup();
        orphans.extend_from_slice(lost);
        orphans.sort_unstable();
        strict.clear();
        strict.resize(orphans.len(), true);
        // The orphans are scanned against the new simplices one by one. With
        // many simplices each orphan is scanned many times, and gathering its
        // row from the input every time costs more than copying the rows
        // once into consecutive memory (#214).
        let copy = !orphans.is_empty() && copy_orphans(&self.input, count);
        if copy {
            #[cfg(test)]
            tests::COPIES.with(|c| c.set(c.get() + 1));
            copied.fill(&self.input, orphans);
        }
        outside.clear();
        ranges.clear();
        farthest.clear();
        for k in 0..count {
            let start = outside.len();
            let found = if orphans.is_empty() {
                None
            } else {
                let rows = if copy { Some(&mut *copied) } else { None };
                let geometry =
                    planes.geometry(k, &vertices[k * dim..(k + 1) * dim], Sign::Positive);
                take_outside(
                    &self.input,
                    orphans,
                    strict,
                    sides,
                    rows,
                    &geometry,
                    outside,
                )?
            };
            ranges.push((start as u32, (outside.len() - start) as u32));
            farthest.push(found);
        }
        // An orphan strictly inside every new simplex lies in the open cone
        // from the apex over the hull, before the visible facet it was
        // outside, so on the open segment from the apex to the hull. It is
        // strictly inside every kept facet too: on one only if the apex is,
        // and then a new simplex shares that plane. So it is interior to the
        // hull with the apex, and to the final hull (design §3).
        // A lost vertex is in the hull and on no kept facet's plane: the
        // simplices of that plane around it would share the kept facet's
        // sign against the apex and keep it a vertex. Strictly inside every
        // new simplex, it is interior too.
        interior.clear();
        interior.extend(
            orphans
                .iter()
                .zip(strict.iter())
                .filter(|&(_, &s)| s)
                .map(|(&p, _)| p),
        );
        Ok(())
    }

    /// Applies a plan: removes the visible facets, adds the new simplices
    /// with their planes and outside sets, turns local links into slots,
    /// and points each facet across the horizon at its new neighbor. Writes
    /// the slots of the new simplices into `slots`, cleared first. The
    /// cone's lists keep their storage for the next plan.
    fn commit(&mut self, created: &mut Cone, slots: &mut Vec<u32>) {
        self.proved_interior.extend_from_slice(&created.interior);
        let d = self.input.dim();
        // Every neighbor of a visible facet is visible or across the
        // horizon, and the commit rewrites the latter's link below, so the
        // visible slots can be reused at once.
        for &slot in &created.region.visible {
            self.facets.remove(slot);
        }
        slots.clear();
        for vertices in created.vertices.chunks_exact(d) {
            slots.push(self.facets.alloc(vertices, Sign::Positive));
        }
        for (k, h) in created.region.horizon.iter().enumerate() {
            let slot = slots[k];
            let row = self.facets.neighbors_mut(slot);
            for (link, neighbor) in created.links[k * d..(k + 1) * d].iter().zip(row) {
                *neighbor = match *link {
                    Link::Old(old) => old,
                    Link::New(j) => slots[j as usize],
                };
            }
            self.facets.neighbors_mut(h.across)[h.back as usize] = slot;
            created.planes.store(k, d, &mut self.facets, slot);
            let (start, len) = created.ranges[k];
            let outside = &created.outside[start as usize..(start + len) as usize];
            self.facets.set_outside(slot, outside, created.farthest[k]);
        }
    }
}

/// The insertion of one point, prepared against the hull: the region it
/// replaces and the new simplices, numbered locally by position, in flat
/// lists. A committed cone is planned again for a later point, and every
/// list keeps its storage.
#[derive(Default)]
struct Cone {
    /// The visible facets, which the commit removes, and the horizon.
    region: Region,
    /// Buffers of [`SimplicialHull::plan_region`].
    scratch: PlanScratch,
    /// D vertices per new simplex, in outward order.
    vertices: Vec<u32>,
    /// D neighbor links per new simplex, slot by slot.
    links: Vec<Link>,
    /// The working plane of each new simplex.
    planes: Planes,
    /// The outside points of every new simplex, each one's in a run.
    outside: Vec<u32>,
    /// Per new simplex, its run in `outside`.
    ranges: Vec<(u32, u32)>,
    /// Per new simplex, the candidate of its outside set.
    farthest: Vec<Option<(u32, Option<f64>)>>,
    /// Outside points of the visible facets that are strictly inside every
    /// new simplex, for [`SimplicialHull::proved_interior`].
    interior: Vec<u32>,
}

/// The working lists of one plan, kept with its [`Cone`] so that planning
/// a point allocates none of them again (#209). Each is cleared where it is
/// filled.
#[derive(Default)]
struct PlanScratch {
    /// Sorted ridge keys of the new simplices, packed.
    keys: Vec<u32>,
    /// The (simplex, slot) of each key.
    owners: Vec<(usize, usize)>,
    /// Scratch and result of [`pair_equal_keys_into`].
    table: Vec<usize>,
    pairs: Vec<(usize, usize)>,
    /// Outside points of the visible facets, then those no new simplex took.
    orphans: Vec<u32>,
    /// Vertices of the visible facets that no new simplex keeps.
    lost: Vec<u32>,
    /// Per orphan, whether every new simplex so far has it strictly inside.
    strict: Vec<bool>,
    /// Scratch of [`take_outside`].
    sides: Vec<Option<Sign>>,
    /// The vertices of the new simplices.
    kept: VertexSet,
    /// The orphans' coordinates, when the plan scans them from a copy.
    copied: CopiedRows,
    /// The outward sign of each new simplex, all positive.
    outward: Vec<Sign>,
}

/// A neighbor reference inside a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    /// A facet of the hull before the insertion (across the horizon), by
    /// slot.
    Old(u32),
    /// The new simplex with this local number.
    New(u32),
}

/// The exact side of `point` relative to a simplex: [`Sign::Positive`] is
/// strictly outside, [`Sign::Zero`] on the supporting hyperplane.
///
/// A simplex with a certified working normal first tries to prove the
/// strict side from the working distance (design §1); the orientation
/// decides everything else. Debug builds check every proved side against
/// the orientation.
fn side(input: &Input<'_>, facet: &Geometry<'_>, point: u32) -> Result<Sign, ConvexHullError> {
    if let Some(proved) = facet
        .cull
        .as_ref()
        .and_then(|cull| cull.proved_side(input.point(facet.vertices[0]), input.point(point)))
    {
        debug_assert_eq!(
            proved,
            oriented_side(input, facet, point)?,
            "the working distance proved the wrong side of point {point}"
        );
        return Ok(proved);
    }
    oriented_side(input, facet, point)
}

/// The side of `point` relative to a simplex by orientation alone.
fn oriented_side(
    input: &Input<'_>,
    facet: &Geometry<'_>,
    point: u32,
) -> Result<Sign, ConvexHullError> {
    let mut indices: Small<u32, 10> = facet.vertices.iter().copied().collect();
    indices.push(point);
    let sign = input.orient(&indices)?;
    Ok(if facet.outward == Sign::Positive {
        sign
    } else {
        sign.reversed()
    })
}

/// The working distance of `point` from a simplex, or `None` without a
/// certified working normal.
fn working_distance(input: &Input<'_>, facet: &Geometry<'_>, point: u32) -> Option<f64> {
    let normal = facet.normal?;
    let origin = input.point(facet.vertices[0]);
    Some(
        input
            .point(point)
            .iter()
            .zip(origin)
            .zip(normal)
            .map(|((x, o), n)| (x - o) * n)
            .sum(),
    )
}

/// The point of `outside` farthest from a simplex by working distance, ties
/// by the smaller index, with its distance. Without a working normal (or
/// with a NaN distance) the smallest index is taken, and its distance is
/// `None`, which comes after every finite distance.
fn farthest(
    input: &Input<'_>,
    facet: &Geometry<'_>,
    outside: &[u32],
) -> Option<(u32, Option<f64>)> {
    let mut best: Option<(u32, Option<f64>)> = None;
    for &p in outside {
        let d = working_distance(input, facet, p).filter(|d| !d.is_nan());
        best = match best {
            Some((b, bd)) if !candidate_before((p, d), (b, bd)) => Some((b, bd)),
            _ => Some((p, d)),
        };
    }
    best
}

/// Moves the points of `remaining` strictly outside a simplex to the end of
/// `outside`, keeping the order of both lists, and returns the candidate of
/// the points moved. `strict[i]` belongs to `remaining[i]` and is cleared
/// when that point is on the supporting hyperplane. `sides` is scratch for
/// the scan, which proves most points strictly inside or strictly outside;
/// only the points it leaves undecided reach the orientation (#214). With
/// `copied`, the scan reads the points from those rows, which follow
/// `remaining` as it shrinks.
///
/// A simplex's outside set is final once taken: points leave it only with
/// the simplex.
fn take_outside(
    input: &Input<'_>,
    remaining: &mut Vec<u32>,
    strict: &mut Vec<bool>,
    sides: &mut Vec<Option<Sign>>,
    mut copied: Option<&mut CopiedRows>,
    facet: &Geometry<'_>,
    outside: &mut Vec<u32>,
) -> Result<Option<(u32, Option<f64>)>, ConvexHullError> {
    let first = outside.len();
    sides.clear();
    sides.resize(remaining.len(), None);
    if let Some(cull) = &facet.cull {
        let origin = input.point(facet.vertices[0]);
        match copied.as_deref() {
            Some(c) => {
                debug_assert_eq!(c.rows.len(), remaining.len() * input.dim());
                let order = &c.order[..remaining.len()];
                cull.mark_sides(origin, &c.rows, input.dim(), order, sides);
            }
            None => {
                let (rows, stride) = input.rows();
                cull.mark_sides(origin, rows, stride, remaining, sides);
            }
        }
    }
    // Kept points move down in place, in order.
    let mut kept = 0;
    for k in 0..remaining.len() {
        let p = remaining[k];
        let sign = match sides[k] {
            Some(proved) => {
                // A proved point skips `side`, so debug builds check the
                // scan's proof here, as `side` checks `proved_side`.
                #[cfg(debug_assertions)]
                debug_assert_eq!(
                    oriented_side(input, facet, p)?,
                    proved,
                    "the scan proved the wrong side of point {p}"
                );
                proved
            }
            None => side(input, facet, p)?,
        };
        if sign == Sign::Positive {
            outside.push(p);
        } else {
            remaining[kept] = p;
            strict[kept] = strict[k] && sign == Sign::Negative;
            if let Some(c) = copied.as_deref_mut() {
                let d = input.dim();
                c.rows.copy_within(k * d..(k + 1) * d, kept * d);
            }
            kept += 1;
        }
    }
    remaining.truncate(kept);
    strict.truncate(kept);
    if let Some(c) = copied {
        c.rows.truncate(kept * input.dim());
    }
    Ok(farthest(input, facet, &outside[first..]))
}

/// A plan with at least this many new simplices may scan its orphans from
/// [`CopiedRows`]. On hull `cube` D = 5, 10^6 points, such plans hold
/// about 90% of the orphan scans, at 10 to 450 scans per orphan. Below it,
/// at 3 to 5 scans per orphan (`cube` D = 3, 10^6), the copy cost what the
/// scans saved (#214).
const COPY_ORPHANS_FROM: usize = 32;

/// Input coordinates, in bytes, below which no plan copies its orphans.
/// Rows of a smaller input stay in cache, so gathering them again is
/// cheap and the copy is pure cost: on `cube` D = 5, 10^4 points (400 KB)
/// copying made the build 5 to 7% slower (#214).
const COPY_ORPHANS_OVER_BYTES: usize = 4 << 20;

/// Whether a plan of `created` new simplices scans its orphans from
/// [`CopiedRows`]: [`COPY_ORPHANS_FROM`] and [`COPY_ORPHANS_OVER_BYTES`],
/// or a test's override on this thread, which ignores the input size.
fn copy_orphans(input: &Input<'_>, created: usize) -> bool {
    #[cfg(test)]
    if let Some(from) = tests::COPY_FROM.with(core::cell::Cell::get) {
        return created >= from;
    }
    created >= COPY_ORPHANS_FROM
        && core::mem::size_of_val(input.rows().0) >= COPY_ORPHANS_OVER_BYTES
}

/// The coordinates of a list of points in consecutive rows, in the list's
/// order: row `i` is point `i` of the list. [`take_outside`] keeps the rows
/// in step as the list shrinks. Both lists keep their storage from one
/// plan to the next.
#[derive(Default)]
struct CopiedRows {
    rows: Vec<f64>,
    /// `0, 1, 2, ...`, at least as long as the list: the scan's indices
    /// into `rows`.
    order: Vec<u32>,
}

impl CopiedRows {
    /// Replaces the rows by those of `points`.
    fn fill(&mut self, input: &Input<'_>, points: &[u32]) {
        self.rows.clear();
        self.rows.reserve(points.len() * input.dim());
        for &p in points {
            self.rows.extend_from_slice(input.point(p));
        }
        let known = self.order.len() as u32;
        self.order.extend(known..points.len() as u32);
    }
}

/// A horizon ridge: the slot of the ridge in its visible facet, and the
/// facet across it (in N) with the slot of the ridge there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Horizon {
    visible: u32,
    slot: u32,
    across: u32,
    back: u32,
}

/// The region a point would replace: its visible facets V and its horizon
/// ridges, each with the facet across it (N). Facets are slots.
#[derive(Debug, Default)]
struct Region {
    visible: Vec<u32>,
    horizon: Vec<Horizon>,
}

/// A flag per slot for one search at a time, cleared without
/// touching the entries: an entry counts only when it carries the current
/// epoch.
#[derive(Default)]
struct Marks {
    entries: Vec<(u32, bool)>,
    epoch: u32,
}

impl Marks {
    /// Forgets every flag, for a store of `slots` slots.
    fn clear(&mut self, slots: usize) {
        if self.entries.len() < slots {
            self.entries.resize(slots, (0, false));
        }
        if self.epoch == u32::MAX {
            self.entries.fill((0, false));
            self.epoch = 0;
        }
        self.epoch += 1;
    }

    fn get(&self, slot: u32) -> Option<bool> {
        match self.entries.get(slot as usize) {
            Some(&(epoch, value)) if epoch == self.epoch => Some(value),
            _ => None,
        }
    }

    fn set(&mut self, slot: u32, value: bool) {
        self.entries[slot as usize] = (self.epoch, value);
    }
}

/// A set of point numbers for one plan, filled once and then queried, so
/// that neither list is sorted. Open addressing with linear probing over
/// at least twice as many slots as insertions, so a probe always ends.
/// `u32::MAX` marks an empty slot; it numbers no point (design §3).
#[derive(Default)]
struct VertexSet {
    slots: Vec<u32>,
    bits: u32,
}

impl VertexSet {
    const EMPTY: u32 = u32::MAX;

    /// The set of `points`, of which there are at most `count`.
    #[cfg(test)]
    fn of(points: impl IntoIterator<Item = u32>, count: usize) -> Self {
        let mut set = Self::default();
        set.fill(points, count);
        set
    }

    /// Replaces the set by `points`, of which there are at most `count`,
    /// keeping the storage of the slots.
    fn fill(&mut self, points: impl IntoIterator<Item = u32>, count: usize) {
        let size = (2 * count).next_power_of_two().max(2);
        self.slots.clear();
        self.slots.resize(size, Self::EMPTY);
        self.bits = size.trailing_zeros();
        for point in points {
            debug_assert_ne!(point, Self::EMPTY, "no point is numbered u32::MAX");
            let mut slot = self.home(point);
            loop {
                match self.slots[slot] {
                    Self::EMPTY => {
                        self.slots[slot] = point;
                        break;
                    }
                    other if other == point => break,
                    _ => slot = (slot + 1) & (size - 1),
                }
            }
        }
    }

    /// The first slot probed for `point`: the top bits of a multiplicative
    /// hash.
    fn home(&self, point: u32) -> usize {
        (u64::from(point).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> (64 - self.bits)) as usize
    }

    fn contains(&self, point: u32) -> bool {
        let mask = self.slots.len() - 1;
        let mut slot = self.home(point);
        loop {
            match self.slots[slot] {
                Self::EMPTY => return false,
                other if other == point => return true,
                _ => slot = (slot + 1) & mask,
            }
        }
    }
}

/// The next candidate to insert: the queue's next, or under a test's
/// override on this thread the one from its other end.
fn next_candidate(pending: &mut Candidates) -> Option<Pending> {
    #[cfg(test)]
    if tests::NEAREST_FIRST.with(core::cell::Cell::get) {
        return pending.pop_nearest();
    }
    pending.pop()
}

/// The candidate of a facet with outside points: its farthest outside point
/// and that point's working distance, when one was computed.
#[derive(Clone, Copy, Debug)]
struct Pending {
    point: u32,
    facet: FacetId,
    distance: Option<f64>,
}

/// Mantissa bits of a distance that select its bucket, beside the exponent:
/// a bucket holds distances within a factor of `2^(1/4)` of each other.
/// Timed against 0, 4, and 8 bits in `docs/bench.md` (#286).
const BUCKET_MANTISSA_BITS: u32 = 2;

/// The candidates waiting for insertion, in buckets by working distance.
///
/// [`Self::pop`] takes from the bucket of the largest distances, and inside
/// a bucket the candidate pushed last. That is farthest first up to the
/// width of a bucket, which keeps the number of created facets near that of
/// an exact order, without the comparisons and the scattered memory of a
/// heap over every candidate. In `docs/bench.md` (the order of candidates,
/// #286) an exact heap was 1.06 to 1.14 times slower on the large `sphere`
/// sets, and last in first out over all candidates up to 9 times slower on
/// `cube`; the queue costs about 3% on `cube` D = 6 with 10^4 points.
///
/// The order depends only on the distances and on the order of the pushes,
/// so it is decided by the values and the order of the input (design §6).
/// The buckets cover only the range of keys pushed so far.
#[derive(Default)]
struct Candidates {
    /// Key of `buckets[0]`.
    low: u64,
    /// Key of the highest bucket that may hold a candidate.
    top: u64,
    buckets: VecDeque<Vec<Pending>>,
    /// Candidates without a positive finite distance, taken last.
    rest: Vec<Pending>,
}

impl Candidates {
    /// The bucket key of a positive finite distance: its exponent and its
    /// top mantissa bits, which order as the distances do.
    fn key(distance: Option<f64>) -> Option<u64> {
        distance
            .filter(|d| *d > 0.0 && d.is_finite())
            .map(|d| d.to_bits() >> (52 - BUCKET_MANTISSA_BITS))
    }

    fn push(&mut self, candidate: Pending) {
        let Some(key) = Self::key(candidate.distance) else {
            self.rest.push(candidate);
            return;
        };
        if self.buckets.is_empty() {
            self.low = key;
            self.top = key;
        }
        for _ in key..self.low {
            self.buckets.push_front(Vec::new());
        }
        self.low = self.low.min(key);
        let at = (key - self.low) as usize;
        while self.buckets.len() <= at {
            self.buckets.push_back(Vec::new());
        }
        self.buckets[at].push(candidate);
        self.top = self.top.max(key);
    }

    fn pop(&mut self) -> Option<Pending> {
        while !self.buckets.is_empty() {
            if let Some(candidate) = self.buckets[(self.top - self.low) as usize].pop() {
                return Some(candidate);
            }
            if self.top == self.low {
                break;
            }
            self.top -= 1;
        }
        self.rest.pop()
    }

    /// The other end of the queue: a candidate without a distance first,
    /// then the one pushed first into the bucket of the smallest distances.
    #[cfg(test)]
    fn pop_nearest(&mut self) -> Option<Pending> {
        if !self.rest.is_empty() {
            return Some(self.rest.remove(0));
        }
        let bucket = self.buckets.iter_mut().find(|b| !b.is_empty())?;
        Some(bucket.remove(0))
    }

    /// Every candidate, in no promised order.
    #[cfg(test)]
    fn into_vec(self) -> Vec<Pending> {
        self.buckets
            .into_iter()
            .flatten()
            .chain(self.rest)
            .collect()
    }
}

/// Which of two outside points of one facet is its candidate: the larger
/// working distance, ties by the smaller index; a missing distance comes
/// after every present one.
fn candidate_before(a: (u32, Option<f64>), b: (u32, Option<f64>)) -> bool {
    match (a.1, b.1) {
        (Some(x), Some(y)) if x != y => x > y,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        _ => a.0 < b.0,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hull::input::accept;
    use crate::predicates::orient;

    pub(crate) struct Rng(pub(crate) u64);

    impl Rng {
        pub(crate) fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        pub(crate) fn unit(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        }
    }

    fn build(dim: usize, points: &[f64]) -> SimplicialHull<'_> {
        SimplicialHull::build(accept(dim, points).unwrap()).unwrap()
    }

    /// No representative strictly outside any facet, and neighbor links are
    /// symmetric across the same ridge.
    pub(crate) fn check_invariants(hull: &SimplicialHull<'_>) {
        if hull.strict_edges {
            let cycle = &hull.polygon;
            assert!(cycle.len() >= 3, "a polygon has at least three vertices");
            let n = cycle.len();
            let mut on_cycle = vec![false; hull.input.representative.len()];
            for i in 0..n {
                let sign = hull
                    .input
                    .orient(&[cycle[i], cycle[(i + 1) % n], cycle[(i + 2) % n]])
                    .unwrap();
                assert_eq!(sign, Sign::Positive, "the chain turns left");
                on_cycle[cycle[i] as usize] = true;
            }
            for &p in &hull.input.representatives {
                if on_cycle[p as usize] {
                    continue;
                }
                for i in 0..n {
                    let sign = hull
                        .input
                        .orient(&[cycle[i], cycle[(i + 1) % n], p])
                        .unwrap();
                    assert_ne!(sign, Sign::Negative, "point {p} outside");
                }
            }
            return;
        }
        for (id, facet) in hull.facets.iter() {
            for &p in &hull.input.representatives {
                assert_ne!(
                    hull.side(facet, p).unwrap(),
                    Sign::Positive,
                    "point {p} outside"
                );
            }
            for (slot, n) in facet.neighbors().enumerate() {
                let other = hull.facets.get(n).expect("neighbor is live");
                let back = other.neighbors().position(|b| b == id).expect("link back");
                let mut a: Vec<u32> = facet
                    .vertices()
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v)
                    .collect();
                let mut b: Vec<u32> = other
                    .vertices()
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != back)
                    .map(|(_, &v)| v)
                    .collect();
                a.sort_unstable();
                b.sort_unstable();
                assert_eq!(a, b, "neighbors share the ridge");
            }
            assert!(facet.outside().is_empty());
        }
    }

    fn vertex_set(hull: &SimplicialHull<'_>) -> Vec<u32> {
        if hull.strict_edges {
            let mut v = hull.polygon.clone();
            v.sort_unstable();
            return v;
        }
        let mut v: Vec<u32> = hull
            .facets
            .iter()
            .flat_map(|(_, f)| f.vertices().to_vec())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The hull after the initial simplex and the first assignment, before
    /// any point is absorbed.
    fn initial(dim: usize, points: &[f64]) -> SimplicialHull<'_> {
        let mut hull = SimplicialHull {
            input: accept(dim, points).unwrap(),
            facets: FacetStore::new(dim),
            proved_interior: Vec::new(),
            strict_edges: false,
            polygon: Vec::new(),
        };
        let initial = hull.initial_simplex().unwrap();
        let candidates: Vec<u32> = hull
            .input
            .representatives
            .iter()
            .copied()
            .filter(|p| !hull.input.spanning_points.contains(p))
            .collect();
        hull.assign(candidates, &initial).unwrap();
        hull
    }

    #[test]
    fn candidates_come_farthest_bucket_first() {
        // A bucket is an exponent and two mantissa bits: 8.0 and 9.0 share
        // one, 10.0 is in the next, and 1.0 and 3.0e-5 are far below.
        let candidate = |point: u32, distance: Option<f64>| Pending {
            point,
            facet: FacetId::new(point, 0),
            distance,
        };
        let pushed = [
            (0, Some(1.0)),
            (1, Some(8.0)),
            (2, None),
            (3, Some(9.0)),
            (4, Some(10.0)),
            (5, Some(3.0e-5)),
            (6, Some(0.0)),
            (7, Some(f64::INFINITY)),
        ];
        let fill = || {
            let mut queue = Candidates::default();
            for (point, distance) in pushed {
                queue.push(candidate(point, distance));
            }
            queue
        };
        // Farthest bucket first; inside a bucket and among the candidates
        // without a positive finite distance, the last pushed first.
        let mut queue = fill();
        let order: Vec<u32> = core::iter::from_fn(|| queue.pop())
            .map(|c| c.point)
            .collect();
        assert_eq!(order, [4, 3, 1, 0, 5, 7, 6, 2]);
        // The test override takes from the other end.
        let mut queue = fill();
        let order: Vec<u32> = core::iter::from_fn(|| queue.pop_nearest())
            .map(|c| c.point)
            .collect();
        assert_eq!(order, [2, 6, 7, 5, 0, 1, 3, 4]);
        // Keys follow the distances, so a larger distance is never in a
        // lower bucket.
        let keys: Vec<u64> = [3.0e-5, 1.0, 8.0, 9.0, 10.0]
            .iter()
            .map(|&d| Candidates::key(Some(d)).unwrap())
            .collect();
        assert!(keys.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(keys[2], keys[3]);
        assert!(keys[3] < keys[4]);
        assert_eq!(Candidates::key(Some(-1.0)), None);
        assert_eq!(Candidates::key(Some(f64::NAN)), None);
    }

    #[test]
    fn a_facets_candidate_is_by_distance_then_index() {
        assert!(candidate_before((7, Some(2.0)), (3, Some(1.0))));
        assert!(!candidate_before((3, Some(1.0)), (7, Some(2.0))));
        // Ties by the smaller index.
        assert!(candidate_before((3, Some(1.0)), (7, Some(1.0))));
        assert!(!candidate_before((7, Some(1.0)), (3, Some(1.0))));
        // A missing distance comes after any present one, then by index.
        assert!(candidate_before((9, Some(-1.0)), (2, None)));
        assert!(candidate_before((2, None), (9, None)));
    }

    #[test]
    fn a_vertex_set_holds_its_members_and_not_the_empty_marker() {
        let empty = VertexSet::of(core::iter::empty(), 0);
        assert!(!empty.contains(0));
        assert!(!empty.contains(u32::MAX - 1));
        assert!(!empty.contains(u32::MAX));

        // The same `count` fixes the table, so the homes match the set below.
        let count = 8;
        let sized = VertexSet::of(core::iter::empty(), count);
        let mut other = 1u32;
        while sized.home(0) != sized.home(other) {
            other += 1;
            assert!(other < 100_000, "two points share a home slot");
        }
        let set = VertexSet::of([0, other, u32::MAX - 1], count);
        assert!(set.contains(0));
        assert!(set.contains(other));
        assert!(set.contains(u32::MAX - 1));
        assert!(!set.contains(u32::MAX));
        let mut missing = 1u32;
        while missing == other || missing == u32::MAX - 1 {
            missing += 1;
        }
        assert!(!set.contains(missing));
    }

    #[test]
    fn candidate_is_the_farthest_with_ties_by_index() {
        // Triangle (0,0), (4,0), (0,4). Facet y = 0 sees 3 (0.5, -1),
        // 4 (2, -3), and 5 (3, -3); 4 and 5 tie at distance 3, so 4 wins.
        let points = [
            0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 0.5, -1.0, 2.0, -3.0, 3.0, -3.0,
        ];
        let hull = initial(2, &points);
        let candidates = hull.all_candidates().into_vec();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].point, 4);
        assert_eq!(candidates[0].distance, Some(3.0));
    }

    thread_local! {
        /// Overrides [`copy_orphans`] for sequential builds on this thread:
        /// copy from this many new simplices, at any input size.
        pub(super) static COPY_FROM: core::cell::Cell<Option<usize>> =
            const { core::cell::Cell::new(None) };
        /// Plans on this thread that scanned copied rows.
        pub(super) static COPIES: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
        /// Makes [`next_candidate`] take from the other end of the queue
        /// ([`Candidates::pop_nearest`]), so that a build on this thread
        /// inserts in another order.
        pub(super) static NEAREST_FIRST: core::cell::Cell<bool> =
            const { core::cell::Cell::new(false) };
    }

    /// The published hull of `points`, taking candidates from the other
    /// end of the queue when `nearest_first`.
    fn published(dim: usize, points: &[f64], nearest_first: bool) -> crate::ConvexHull {
        NEAREST_FIRST.with(|c| c.set(nearest_first));
        let hull = crate::ConvexHullBuilder::new(dim, points).build();
        NEAREST_FIRST.with(|c| c.set(false));
        hull.unwrap()
    }

    /// The published hull does not depend on the order of insertion (design
    /// §6, ADR 0003): taking candidates from the other end of the queue
    /// publishes the same hull, in general position and on inputs with
    /// coplanar points and faces that are not simplices.
    #[test]
    fn the_published_hull_does_not_depend_on_the_insertion_order() {
        let mut rng = Rng(271);
        for (dim, count, family) in [
            (3, 200, "sphere"),
            (4, 80, "sphere"),
            (5, 40, "sphere"),
            (3, 300, "cube"),
            (4, 150, "cube"),
            (3, 300, "grid"),
            (4, 300, "grid"),
            (5, 200, "grid"),
            (3, 300, "surface"),
            (4, 200, "surface"),
        ] {
            let mut points = Vec::new();
            for _ in 0..count {
                let mut v: Vec<f64> = (0..dim).map(|_| rng.unit()).collect();
                match family {
                    "sphere" => {
                        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
                        v.iter_mut().for_each(|x| *x /= norm);
                    }
                    // Integer points of [0, 3]^D: coplanar points, faces
                    // that are not simplices, and duplicates.
                    "grid" => v.iter_mut().for_each(|x| *x = (rng.next() % 4) as f64),
                    // Integer points on the surface of [0, 4]^D.
                    "surface" => {
                        v.iter_mut().for_each(|x| *x = (rng.next() % 5) as f64);
                        let axis = (rng.next() % dim as u64) as usize;
                        v[axis] = (rng.next() % 2 * 4) as f64;
                    }
                    _ => {}
                }
                points.extend(v);
            }
            let farthest = published(dim, &points, false);
            let nearest = published(dim, &points, true);
            assert_eq!(farthest, nearest, "D = {dim}, {family}");
            // Equality does not look at the planes, and the claim covers
            // them, the split of faces that are not simplices, and the
            // order in which `volume()` adds: each is compared by itself.
            let planes = |hull: &crate::ConvexHull| -> Vec<(Vec<f64>, f64)> {
                let planes = hull.planes().unwrap();
                planes
                    .iter()
                    .map(|p| (p.normal().to_vec(), p.offset()))
                    .collect()
            };
            assert_eq!(planes(&farthest), planes(&nearest), "D = {dim}, {family}");
            assert!(
                farthest
                    .triangulation()
                    .iter()
                    .eq(nearest.triangulation().iter()),
                "D = {dim}, {family}"
            );
            assert_eq!(
                farthest.volume().to_bits(),
                nearest.volume().to_bits(),
                "D = {dim}, {family}"
            );
        }
    }

    #[test]
    fn taking_the_nearest_candidate_changes_the_insertion_order() {
        // The override bites: on a sphere the two orders build different
        // simplicial hulls slot by slot, so the test above compares two
        // constructions and not one.
        let mut rng = Rng(272);
        let mut points = Vec::new();
        for _ in 0..200 {
            let v: Vec<f64> = (0..3).map(|_| rng.unit()).collect();
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            points.extend(v.iter().map(|x| x / norm));
        }
        let farthest = build(3, &points);
        NEAREST_FIRST.with(|c| c.set(true));
        let nearest = build(3, &points);
        NEAREST_FIRST.with(|c| c.set(false));
        assert_ne!(snapshot(&farthest), snapshot(&nearest));
    }

    /// The sequential build of `points` with orphans copied from plans of
    /// `from` new simplices, and the number of plans that copied.
    fn build_copying_from(dim: usize, points: &[f64], from: usize) -> (SimplicialHull<'_>, usize) {
        COPY_FROM.with(|c| c.set(Some(from)));
        COPIES.with(|c| c.set(0));
        let hull = SimplicialHull::build(accept(dim, points).unwrap()).unwrap();
        COPY_FROM.with(|c| c.set(None));
        (hull, COPIES.with(core::cell::Cell::get))
    }

    #[test]
    fn copied_orphan_rows_change_no_plan() {
        // Every plan copies (from 1) or none does (from usize::MAX): the
        // arena, the outside sets, and the proved interior points agree.
        // Points on a sphere keep most orphans outside some new simplex;
        // points in a cube are mostly proved interior.
        let mut rng = Rng(214);
        for (dim, count, on_sphere) in [
            (3, 300, true),
            (4, 120, true),
            (5, 60, true),
            (3, 800, false),
            (4, 400, false),
        ] {
            let mut points = Vec::new();
            for _ in 0..count {
                let v: Vec<f64> = (0..dim).map(|_| rng.unit()).collect();
                let norm = if on_sphere {
                    v.iter().map(|x| x * x).sum::<f64>().sqrt()
                } else {
                    1.0
                };
                points.extend(v.iter().map(|x| x / norm));
            }
            let (copying, copies) = build_copying_from(dim, &points, 1);
            let (gathering, none) = build_copying_from(dim, &points, usize::MAX);
            assert!(copies > 0, "dim {dim}: no plan copied");
            assert_eq!(none, 0, "dim {dim}: a plan copied past the bound");
            assert_eq!(snapshot(&copying), snapshot(&gathering), "dim {dim}");
            let mut a = copying.proved_interior.clone();
            let mut b = gathering.proved_interior.clone();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b, "dim {dim}");
            check_invariants(&copying);
        }
    }

    /// The store as (id, vertices, neighbors, outward, outside), in slot
    /// order.
    type Snapshot = Vec<(FacetId, Vec<u32>, Vec<FacetId>, Sign, Vec<u32>)>;

    fn snapshot(hull: &SimplicialHull<'_>) -> Snapshot {
        hull.facets
            .iter()
            .map(|(id, f)| {
                (
                    id,
                    f.vertices().to_vec(),
                    f.neighbors().collect(),
                    f.outward(),
                    f.outside().to_vec(),
                )
            })
            .collect()
    }

    #[test]
    fn segment_in_one_dimension() {
        let points = [3.0, -1.0, 2.0, -1.0, 7.0, 0.5];
        let hull = build(1, &points);
        assert_eq!(hull.facets.len(), 2);
        assert_eq!(vertex_set(&hull), vec![1, 4]);
        for (_, f) in hull.facets.iter() {
            assert_eq!(f.neighbors().len(), 0);
        }
        check_invariants(&hull);
    }

    #[test]
    fn square_with_interior_and_edge_points() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5, 0.5, 0.0];
        let hull = build(2, &points);
        check_invariants(&hull);
        assert_eq!(vertex_set(&hull), vec![0, 1, 2, 3]);
        assert_eq!(hull.polygon.len(), 4);
    }

    #[test]
    fn a_collinear_basis_point_is_not_an_extreme_edge_vertex() {
        // (1, 0) lies on the segment (0, 0)-(2, 0). The polygon keeps a
        // point only while it makes a strict turn, so (1, 0) is not a
        // simplicial vertex. Classification still reports it as coplanar.
        let points = [0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let hull = build(2, &points);
        check_invariants(&hull);
        assert_eq!(hull.input.spanning_points, vec![0, 1, 3]);
        assert_eq!(vertex_set(&hull), vec![0, 2, 3, 4]);
        assert_eq!(reference_2d(&points), vec![0, 2, 3, 4]);
    }

    #[test]
    fn cube_is_closed() {
        let mut points = Vec::new();
        for i in 0..8 {
            points.extend([(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64]);
        }
        points.extend([0.5, 0.5, 0.5, 0.5, 0.5, 0.0]);
        let hull = build(3, &points);
        check_invariants(&hull);
        assert_eq!(vertex_set(&hull), (0..8).collect::<Vec<u32>>());
        assert_eq!(hull.facets.len(), 12);
    }

    /// Andrew's monotone chain with exact orientation, as an independent
    /// reference for strictly convex vertices in 2D.
    fn reference_2d(points: &[f64]) -> Vec<u32> {
        let n = points.len() / 2;
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by(|&a, &b| {
            let (pa, pb) = (
                &points[a as usize * 2..a as usize * 2 + 2],
                &points[b as usize * 2..b as usize * 2 + 2],
            );
            pa[0].total_cmp(&pb[0]).then(pa[1].total_cmp(&pb[1]))
        });
        let p = |i: u32| &points[i as usize * 2..i as usize * 2 + 2];
        let mut chain: Vec<u32> = Vec::new();
        for pass in [order.clone(), order.iter().rev().copied().collect()] {
            let start = chain.len();
            for &i in &pass {
                while chain.len() >= start + 2 {
                    let s = orient(&[p(chain[chain.len() - 2]), p(chain[chain.len() - 1]), p(i)])
                        .unwrap();
                    if s == Sign::Positive {
                        break;
                    }
                    chain.pop();
                }
                chain.push(i);
            }
            chain.pop();
        }
        chain.sort_unstable();
        chain.dedup();
        chain
    }

    #[test]
    fn random_2d_matches_monotone_chain() {
        let mut rng = Rng(5);
        for trial in 0..50 {
            let n = 10 + trial * 7;
            let points: Vec<f64> = (0..n * 2).map(|_| rng.unit()).collect();
            let hull = build(2, &points);
            check_invariants(&hull);
            assert_eq!(vertex_set(&hull), reference_2d(&points), "trial {trial}");
        }
    }

    #[test]
    fn random_points_in_three_to_five_dimensions() {
        let mut rng = Rng(8);
        // The invariant check is O(facets x points); sizes stay small so
        // debug runs stay fast.
        for (dim, count) in [(3, 150), (4, 80), (5, 50)] {
            for _ in 0..3 {
                let points: Vec<f64> = (0..count * dim).map(|_| rng.unit()).collect();
                let hull = build(dim, &points);
                check_invariants(&hull);
            }
        }
    }

    #[test]
    fn points_on_a_sphere_and_a_grid() {
        let mut rng = Rng(13);
        let mut sphere = Vec::new();
        for _ in 0..300 {
            let v: Vec<f64> = (0..3).map(|_| rng.unit()).collect();
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            sphere.extend(v.iter().map(|x| x / norm));
        }
        check_invariants(&build(3, &sphere));
        let mut grid = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                for k in 0..5 {
                    grid.extend([i as f64, j as f64, k as f64]);
                }
            }
        }
        let hull = build(3, &grid);
        check_invariants(&hull);
    }
}
