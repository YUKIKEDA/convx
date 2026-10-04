//! Sequential Quickhull through insertion, kept simplicial (design §4, §6).
//!
//! Every facet is a (D-1)-simplex of D vertices. `neighbors[i]` is the facet
//! across the ridge opposite `vertices[i]`. A point is outside a facet when
//! the orientation of the facet's vertices followed by the point has the
//! facet's `outward` sign; for D >= 2 the vertex order is chosen so that this
//! sign is always [`Sign::Positive`], and only the two endpoints of D = 1
//! need a stored sign. Visibility is decided by that exact orientation alone.
//! No facets are merged during insertion.
//!
//! Points are absorbed in rounds by the batch extraction of design §6, which
//! the sequential and the parallel builds share:
//!
//! 1. Each facet with outside points proposes its farthest one.
//! 2. Candidates are packed by working distance, largest first, ties by the
//!    smaller index. A candidate is taken when its facets T = V ∪ N (visible
//!    facets and the facets across its horizon) and its horizon ridges H are
//!    not yet reserved, and when neither it nor a taken candidate is strictly
//!    outside a prospective simplex (a horizon ridge joined with the point)
//!    of the other. Otherwise it waits for the next round in its outside
//!    set.
//! 3. The batch is applied in ascending input index.

#[cfg(any(test, debug_assertions))]
use core::convert::Infallible;
#[cfg(debug_assertions)]
use std::collections::HashSet;

use rayon::prelude::*;

use super::input::Input;
use super::ridge::{fingerprint, pair_equal_keys};
use super::ConvexHullError;
use crate::arena::{Arena, ArenaFull, FacetId, SlotMarks};
use crate::cull::CullPlane;
use crate::normal::{facet_cofactors, lifted_facet_cofactors, working_normal};
use crate::predicates::Sign;

/// A simplicial facet during construction.
pub(crate) struct Simplex {
    pub(crate) vertices: Vec<u32>,
    pub(crate) neighbors: Vec<FacetId>,
    /// The sign of `orient(vertices, q)` for a point `q` outside.
    pub(crate) outward: Sign,
    /// Working unit normal, when one could be certified but no cull plane,
    /// which otherwise carries it (see [`Simplex::normal`]).
    normal: Option<Vec<f64>>,
    cull: Option<CullPlane>,
    /// Points assigned to this facet that are strictly outside it.
    outside: Vec<u32>,
    /// The candidate of `outside` for a round, fixed with it (#120): see
    /// [`farthest`].
    farthest: Option<(u32, Option<f64>)>,
}

impl Simplex {
    /// Working unit normal, when one could be certified.
    fn normal(&self) -> Option<&[f64]> {
        match &self.cull {
            Some(cull) => Some(cull.normal()),
            None => self.normal.as_deref(),
        }
    }

    /// The cull plane of this simplex, when one could be certified.
    pub(crate) fn cull(&self) -> Option<&CullPlane> {
        self.cull.as_ref()
    }
}

/// The simplicial hull after every point has been absorbed.
pub(crate) struct SimplicialHull<'a> {
    pub(crate) input: Input<'a>,
    pub(crate) facets: Arena<Simplex>,
    /// Points construction dropped with every sign it tested strictly
    /// negative: strictly inside the hull at that step, so in the interior
    /// of the final hull (design §3). Unordered.
    pub(crate) proved_interior: Vec<u32>,
}

/// An index that no longer fits in `u32` is an exhaustion of the index
/// space; like an ordinary allocation failure, it aborts (design §3).
fn insert_or_abort(arena: &mut Arena<Simplex>, simplex: Simplex) -> FacetId {
    match arena.insert(simplex) {
        Ok(id) => id,
        Err(ArenaFull) => std::process::abort(),
    }
}

impl<'a> SimplicialHull<'a> {
    /// Builds the simplicial hull of an accepted input.
    pub(crate) fn build(input: Input<'a>, execution: Execution) -> Result<Self, ConvexHullError> {
        let mut hull = Self {
            input,
            facets: Arena::new(),
            proved_interior: Vec::new(),
        };
        if hull.input.engine_dim() == 1 {
            hull.build_segment()?;
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
            hull.absorb(execution)?;
        }
        Ok(hull)
    }

    /// Engine-space coordinates of `vertices` for working normals.
    fn coords_of(&self, vertices: &[u32]) -> Vec<&[f64]> {
        vertices.iter().map(|&v| self.input.coords(v)).collect()
    }

    /// The exact side of `point` relative to `facet`: [`Sign::Positive`] is
    /// strictly outside, [`Sign::Zero`] on the supporting hyperplane.
    pub(crate) fn side(&self, facet: &Simplex, point: u32) -> Result<Sign, ConvexHullError> {
        side(&self.input, facet, point)
    }

    fn make_simplex(
        &self,
        vertices: Vec<u32>,
        neighbors: Vec<FacetId>,
        outward: Sign,
    ) -> Result<Simplex, ConvexHullError> {
        let points = self.coords_of(&vertices);
        // Rounded lifted coordinates can be infinite; that facet then has no
        // working normal, and the farthest point falls back to index order.
        let finite = points.iter().all(|p| p.iter().all(|x| x.is_finite()));
        // The cofactors certify both the working normal and the cull plane;
        // they are evaluated once (#86).
        let cofactors = if finite {
            facet_cofactors(&points)
        } else {
            None
        };
        // The working normal is the certified cofactor direction. Published
        // planes still run Householder QR (design §1).
        let normal = if finite {
            working_normal(&points, outward, cofactors.as_deref())?
        } else {
            None
        };
        // A lifted facet's points are rounded, so its cull plane is certified
        // against the exact lift: the cofactors carry each height's bound,
        // and the threshold adds the origin's and the query's (#109).
        let cull = normal.as_deref().and_then(|n| {
            if self.input.is_lifted() {
                let bounds: Vec<f64> = vertices
                    .iter()
                    .map(|&v| self.input.height_bound(v))
                    .collect();
                let widened = lifted_facet_cofactors(&points, cofactors.as_deref()?, &bounds);
                CullPlane::with_lifted_cofactors(&points, n, outward, widened.as_deref(), bounds[0])
            } else {
                CullPlane::with_cofactors(&points, n, outward, cofactors.as_deref())
            }
        });
        // The cull plane carries the normal; it is kept apart only without
        // one.
        let normal = if cull.is_some() { None } else { normal };
        Ok(Simplex {
            vertices,
            neighbors,
            outward,
            normal,
            cull,
            outside: Vec::new(),
            farthest: None,
        })
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
        let low_facet = self.make_simplex(vec![low], Vec::new(), Sign::Negative)?;
        let high_facet = self.make_simplex(vec![high], Vec::new(), Sign::Positive)?;
        insert_or_abort(&mut self.facets, low_facet);
        insert_or_abort(&mut self.facets, high_facet);
        Ok(())
    }

    /// The D + 1 facets of the simplex on `spanning_points`, each ordered so
    /// that the opposite vertex is on the negative side.
    fn initial_simplex(&mut self) -> Result<Vec<FacetId>, ConvexHullError> {
        let simplex = self.input.spanning_points.clone();
        let d = self.input.engine_dim();
        let mut ordered = Vec::with_capacity(d + 1);
        for (i, &apex) in simplex.iter().enumerate() {
            let mut vertices: Vec<u32> = simplex.iter().copied().filter(|&v| v != apex).collect();
            let mut indices = vertices.clone();
            indices.push(apex);
            if self.input.orient(&indices)? == Sign::Positive {
                vertices.swap(0, 1);
            }
            ordered.push((i, vertices));
        }
        // Reserve ids first so neighbors can refer to them.
        let mut ids = Vec::with_capacity(d + 1);
        for (_, vertices) in &ordered {
            let simplex = self.make_simplex(vertices.clone(), Vec::new(), Sign::Positive)?;
            ids.push(insert_or_abort(&mut self.facets, simplex));
        }
        // The facet omitting simplex[i] has id ids[i]; across the ridge
        // opposite vertex v lies the facet omitting v.
        for (i, vertices) in &ordered {
            let neighbors: Vec<FacetId> = vertices
                .iter()
                .map(|v| ids[simplex.iter().position(|s| s == v).unwrap_or(0)])
                .collect();
            if let Some(facet) = self.facets.get_mut(ids[*i]) {
                facet.neighbors = neighbors;
            }
        }
        Ok(ids)
    }

    /// Assigns each candidate to the first facet in `facets`, every facet of
    /// the initial simplex, that it is strictly outside. Candidates outside
    /// none are dropped: they are inside the current hull or on its
    /// boundary. Those strictly inside every facet are recorded in
    /// [`Self::proved_interior`].
    fn assign(
        &mut self,
        mut remaining: Vec<u32>,
        facets: &[FacetId],
    ) -> Result<(), ConvexHullError> {
        let mut strict = vec![true; remaining.len()];
        let mut inside = Vec::new();
        for &id in facets {
            if remaining.is_empty() {
                break;
            }
            let Some(facet) = self.facets.get_mut(id) else {
                continue;
            };
            take_outside(&self.input, &mut remaining, &mut strict, &mut inside, facet)?;
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

    /// The working distance of `point` from `facet`, or `None` without a
    /// certified working normal.
    #[cfg(test)]
    fn working_distance(&self, facet: &Simplex, point: u32) -> Option<f64> {
        working_distance(&self.input, facet, point)
    }

    /// One candidate per facet with outside points, in packing order.
    ///
    /// `pending` holds the candidates of every facet with outside points,
    /// possibly with entries of removed facets, which are dropped here; it
    /// is filled from the whole arena when `seeded` is false. Commits add
    /// the candidates of the facets they insert (see [`Self::absorb`]), so
    /// a round does not read every facet (#120).
    fn candidates(
        &self,
        pending: &mut Vec<(u32, FacetId, Option<f64>)>,
        seeded: &mut bool,
    ) -> Vec<(u32, FacetId, Option<f64>)> {
        if *seeded {
            pending.retain(|&(_, id, _)| self.facets.get(id).is_some());
        } else {
            pending.clear();
            pending.extend(
                self.facets
                    .iter()
                    .filter_map(|(id, facet)| facet.farthest.map(|(p, d)| (p, id, d))),
            );
            *seeded = true;
        }
        // Outside sets are disjoint, so a point is the candidate of at most
        // one facet, and the order does not depend on the order of
        // `pending`.
        pending.sort_by(|&(p, _, dp), &(q, _, dq)| {
            if packs_before((p, dp), (q, dq)) {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Greater
            }
        });
        #[cfg(debug_assertions)]
        {
            let mut whole: Vec<(u32, FacetId)> = self
                .facets
                .iter()
                .filter_map(|(id, facet)| facet.farthest.map(|(p, _)| (p, id)))
                .collect();
            let mut kept: Vec<(u32, FacetId)> = pending.iter().map(|&(p, id, _)| (p, id)).collect();
            whole.sort_unstable();
            kept.sort_unstable();
            debug_assert!(
                kept == whole,
                "the pending candidates differ from the arena's"
            );
        }
        pending.clone()
    }

    /// [`Self::candidates`] read from the whole arena, for tests.
    #[cfg(test)]
    fn all_candidates(&self) -> Vec<(u32, FacetId, Option<f64>)> {
        self.candidates(&mut Vec::new(), &mut false)
    }

    /// Absorbs every outside point in rounds of batches (design §6). Each
    /// point of a batch is planned against the hull before the round, on one
    /// thread or on rayon's pool, and the plans are committed in ascending
    /// input index. Both executions run the same plans and the same commits.
    fn absorb(&mut self, execution: Execution) -> Result<(), ConvexHullError> {
        let mut scratch = WalkScratch::default();
        loop {
            let batch = self.next_batch_with(&mut scratch)?;
            if batch.is_empty() {
                return Ok(());
            }
            let (keys, regions): (Vec<(u32, FacetId)>, Vec<Region>) = batch
                .into_iter()
                .map(|(point, start, region)| ((point, start), region))
                .unzip();
            let plans: Vec<Plan> = match execution {
                Execution::Sequential => keys
                    .iter()
                    .zip(regions)
                    .map(|(&(point, _), region)| self.plan_region(point, region))
                    .collect::<Result<_, _>>()?,
                Execution::Parallel => keys
                    .par_iter()
                    .zip(regions)
                    .map(|(&(point, _), region)| self.plan_region(point, region))
                    .collect::<Result<_, _>>()?,
            };
            for (plan, &(point, start)) in plans.into_iter().zip(&keys) {
                // Debug check of §6: applying the batch sequentially in index
                // order, planning each point against the hull as the earlier
                // commits left it, gives the same mutation as the plan made
                // before the round, so the same topology.
                #[cfg(debug_assertions)]
                if execution == Execution::Parallel {
                    let again = self.plan(start, point)?;
                    debug_assert!(
                        again.shape() == plan.shape(),
                        "point {point}: the parallel plan differs from sequential application"
                    );
                }
                #[cfg(not(debug_assertions))]
                let _ = (point, start);
                for id in self.commit(plan) {
                    if let Some((p, d)) = self.facets.get(id).and_then(|f| f.farthest) {
                        scratch.pending.push((p, id, d));
                    }
                }
            }
        }
    }

    /// The next batch, as (point, facet it is outside) in ascending input
    /// index (design §6). The first candidate always fits, so a round with
    /// candidates is never empty.
    ///
    /// Candidates are packed in [`packs_before`] order. A candidate is taken
    /// when none of its facets T = V ∪ N and none of its horizon ridges H is
    /// already reserved. A debug build also asserts that neither it nor a
    /// taken candidate is strictly outside a prospective simplex of the
    /// other; that never holds once T and H are free (see
    /// [`Self::conflicts`]), so it skips no candidate. A skipped candidate
    /// stays in its outside set for a later round. This wraps
    /// [`Self::next_batch_with`] with fresh scratch for tests.
    #[cfg(test)]
    fn next_batch(&self) -> Result<Vec<(u32, FacetId)>, ConvexHullError> {
        Ok(self
            .next_batch_with(&mut WalkScratch::default())?
            .into_iter()
            .map(|(p, f, _)| (p, f))
            .collect())
    }

    /// The batch of the next round, with each point's visible region, in
    /// ascending point index.
    ///
    /// A horizon ridge lies in exactly two facets, one of V and one of N,
    /// so two regions with disjoint T share no horizon ridge: the H test of
    /// §6 never rejects a candidate T admits, and runs only as a debug
    /// assertion. Nor does the prospective-simplex conflict test (see
    /// [`Self::conflicts`]), like the sequential replay of a parallel round.
    fn next_batch_with(
        &self,
        scratch: &mut WalkScratch,
    ) -> Result<Vec<(u32, FacetId, Region)>, ConvexHullError> {
        let WalkScratch {
            visited,
            taken,
            pending,
            seeded,
        } = scratch;
        taken.clear();
        #[cfg(debug_assertions)]
        let mut ridges: HashSet<Vec<u32>> = HashSet::new();
        let mut batch: Vec<(u32, FacetId, Region)> = Vec::new();
        #[cfg(debug_assertions)]
        let mut taken_prospective: Vec<(u32, Vec<Vec<u32>>)> = Vec::new();
        for (point, start, _) in self.candidates(pending, seeded) {
            // A candidate whose V or N meets a taken facet is rejected; once
            // that is certain, its walk stops (#110). Its start facet is in
            // its V, so a taken start needs no walk at all.
            if taken.contains(start) {
                continue;
            }
            let stop = |id: FacetId| if taken.contains(id) { Err(()) } else { Ok(()) };
            let Ok(region) = self.walk_region(start, point, visited, stop)? else {
                continue;
            };
            #[cfg(debug_assertions)]
            {
                let horizon = self.horizon_ridges(&region);
                debug_assert!(
                    horizon.iter().all(|r| !ridges.contains(r)),
                    "point {point} shares a horizon ridge although T is free"
                );
                ridges.extend(horizon);
                let prospective = self.prospective(&region, point);
                for (other, other_prospective) in &taken_prospective {
                    debug_assert!(
                        !self.conflicts((point, &prospective), (*other, other_prospective))?,
                        "points {point} and {other} conflict although T and H are free"
                    );
                }
                taken_prospective.push((point, prospective));
            }
            for id in region.touched() {
                taken.insert(id, ());
            }
            batch.push((point, start, region));
        }
        batch.sort_unstable_by_key(|&(point, _, _)| point);
        Ok(batch)
    }

    /// The horizon ridges H of `region`, each as its sorted vertex list.
    #[cfg(any(test, debug_assertions))]
    fn horizon_ridges(&self, region: &Region) -> Vec<Vec<u32>> {
        region
            .horizon
            .iter()
            .filter_map(|&(id, slot, _)| {
                let facet = self.facets.get(id)?;
                let mut ridge: Vec<u32> = facet
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(m, _)| m != slot)
                    .map(|(_, &v)| v)
                    .collect();
                ridge.sort_unstable();
                Some(ridge)
            })
            .collect()
    }

    /// The prospective simplices of `apex`: each horizon ridge joined with
    /// the apex, in the outward vertex order the insertion will give them.
    #[cfg(any(test, debug_assertions))]
    fn prospective(&self, region: &Region, apex: u32) -> Vec<Vec<u32>> {
        region
            .horizon
            .iter()
            .filter_map(|&(id, slot, _)| {
                let mut vertices = self.facets.get(id)?.vertices.clone();
                vertices[slot] = apex;
                Some(vertices)
            })
            .collect()
    }

    /// Whether two candidates conflict on their prospective simplices: one
    /// is strictly outside a prospective simplex of the other.
    ///
    /// After the T and H test this never holds in exact arithmetic. A
    /// prospective simplex of P sits on a horizon ridge, which lies on
    /// exactly two facets: a visible one and one of N(P), both supporting
    /// hyperplanes of the convex hull. The strict outer side of the
    /// prospective simplex is covered by the strict outer sides of those two,
    /// and a point strictly outside a facet's hyperplane sees that facet. So
    /// a Q strictly outside a prospective simplex of P sees a facet of T(P),
    /// and the reverse holds the same way. Connectivity of the visible region
    /// is what puts every facet across the horizon into N(P). §6 keeps the
    /// test as a debug assertion, which [`Self::next_batch_with`] checks.
    #[cfg(any(test, debug_assertions))]
    fn conflicts(
        &self,
        (p, p_prospective): (u32, &[Vec<u32>]),
        (q, q_prospective): (u32, &[Vec<u32>]),
    ) -> Result<bool, ConvexHullError> {
        Ok(self.outside_any(p_prospective, q)? || self.outside_any(q_prospective, p)?)
    }

    /// Whether `point` is strictly outside any of `simplices`, each in
    /// outward order (orientation positive outside).
    #[cfg(any(test, debug_assertions))]
    fn outside_any(&self, simplices: &[Vec<u32>], point: u32) -> Result<bool, ConvexHullError> {
        for vertices in simplices {
            let mut indices = vertices.clone();
            indices.push(point);
            if self.input.orient(&indices)? == Sign::Positive {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The facets visible from `apex`, found by breadth-first search over
    /// neighbors from `start`, and the horizon ridges as (visible facet,
    /// slot) pairs.
    #[cfg(any(test, debug_assertions))]
    fn visible_region(&self, start: FacetId, apex: u32) -> Result<Region, ConvexHullError> {
        let mut visited = SlotMarks::default();
        match self.walk_region(start, apex, &mut visited, |_| Ok::<(), Infallible>(()))? {
            Ok(region) => Ok(region),
            Err(never) => match never {},
        }
    }

    /// The walk of [`Self::visible_region`]. `stop` sees every facet of V and
    /// N as the walk reaches it; an `Err` from it ends the walk. `visited`
    /// is scratch, cleared here.
    fn walk_region<E>(
        &self,
        start: FacetId,
        apex: u32,
        visited: &mut SlotMarks<bool>,
        stop: impl Fn(FacetId) -> Result<(), E>,
    ) -> Result<Result<Region, E>, ConvexHullError> {
        if let Err(e) = stop(start) {
            return Ok(Err(e));
        }
        visited.clear();
        let mut visible = vec![start];
        visited.insert(start, true);
        let mut horizon: Vec<(FacetId, usize, FacetId)> = Vec::new();
        let mut cursor = 0;
        while cursor < visible.len() {
            let id = visible[cursor];
            cursor += 1;
            let Some(facet) = self.facets.get(id) else {
                continue;
            };
            for (slot, &neighbor) in facet.neighbors.iter().enumerate() {
                // Every neighbor of a facet of V is in V or N.
                if let Err(e) = stop(neighbor) {
                    return Ok(Err(e));
                }
                let seen = match visited.get(neighbor) {
                    Some(v) => v,
                    None => {
                        let v = match self.facets.get(neighbor) {
                            Some(n) => self.side(n, apex)? == Sign::Positive,
                            None => false,
                        };
                        visited.insert(neighbor, v);
                        if v {
                            visible.push(neighbor);
                        }
                        v
                    }
                };
                if !seen {
                    horizon.push((id, slot, neighbor));
                }
            }
        }
        Ok(Ok(Region { visible, horizon }))
    }

    /// Inserts one point now: [`Self::plan`] then [`Self::commit`].
    #[cfg(test)]
    fn insert_point(&mut self, start: FacetId, apex: u32) -> Result<(), ConvexHullError> {
        let plan = self.plan(start, apex)?;
        self.commit(plan);
        Ok(())
    }

    /// Prepares the insertion of `apex`, outside `start`, without changing
    /// the hull: the cone from `apex` over the horizon, with links between
    /// the new simplices by local number, and the outside points of the
    /// visible facets reassigned to the new simplices. Runs on a worker.
    #[cfg(any(test, debug_assertions))]
    fn plan(&self, start: FacetId, apex: u32) -> Result<Plan, ConvexHullError> {
        self.plan_region(apex, self.visible_region(start, apex)?)
    }

    /// [`Self::plan`] with the visible region already found.
    fn plan_region(&self, apex: u32, region: Region) -> Result<Plan, ConvexHullError> {
        let Region { visible, horizon } = region;

        // One new simplex per horizon ridge: the visible facet's vertex order
        // with the vertex opposite the ridge replaced by the apex keeps the
        // outward orientation.
        let mut created: Vec<Planned> = Vec::with_capacity(horizon.len());
        // Each ridge between two new simplices holds the apex and D - 2
        // vertices of a horizon ridge. Its key is those D - 2 vertices,
        // sorted, in one flat buffer; sorting the keys pairs the two
        // simplices that share each ridge, with no hashing and no
        // allocation per key.
        let width = self.input.engine_dim().saturating_sub(2);
        let mut keys: Vec<u32> = Vec::new();
        let mut owners: Vec<(usize, usize)> = Vec::new();
        for &(visible_id, slot, across) in &horizon {
            let Some(old) = self.facets.get(visible_id) else {
                continue;
            };
            let mut vertices = old.vertices.clone();
            vertices[slot] = apex;
            let d = vertices.len();
            debug_assert_eq!(d, width + 2, "a simplex has D vertices");
            let k = created.len();
            for other in (0..d).filter(|&m| m != slot) {
                let start = keys.len();
                keys.extend(
                    vertices
                        .iter()
                        .enumerate()
                        .filter(|&(m, _)| m != other && m != slot)
                        .map(|(_, &v)| v),
                );
                keys[start..].sort_unstable();
                owners.push((k, other));
            }
            created.push(Planned {
                simplex: self.make_simplex(vertices, Vec::new(), Sign::Positive)?,
                links: vec![Link::Old(across); d],
                across,
                replaces: visible_id,
            });
        }
        for (first, second) in pair_equal_keys(&keys, owners.len(), fingerprint) {
            let (a, a_slot) = owners[first];
            let (b, b_slot) = owners[second];
            created[a].links[a_slot] = Link::New(b);
            created[b].links[b_slot] = Link::New(a);
        }

        // Reassign the outside points of the visible facets.
        let mut orphans: Vec<u32> = visible
            .iter()
            .filter_map(|&id| self.facets.get(id))
            .flat_map(|f| f.outside.iter().copied())
            .filter(|&p| p != apex)
            .collect();
        // A vertex of only visible facets stops being a vertex. It is
        // proved interior on the same terms as an orphan (design §3).
        let mut lost: Vec<u32> = visible
            .iter()
            .filter_map(|&id| self.facets.get(id))
            .flat_map(|f| f.vertices.iter().copied())
            .collect();
        lost.sort_unstable();
        lost.dedup();
        let mut kept: Vec<u32> = created
            .iter()
            .flat_map(|p| p.simplex.vertices.iter().copied())
            .collect();
        kept.sort_unstable();
        kept.dedup();
        lost.retain(|v| kept.binary_search(v).is_err());
        orphans.extend(lost);
        orphans.sort_unstable();
        let mut strict = vec![true; orphans.len()];
        let mut inside = Vec::new();
        for planned in &mut created {
            if orphans.is_empty() {
                break;
            }
            take_outside(
                &self.input,
                &mut orphans,
                &mut strict,
                &mut inside,
                &mut planned.simplex,
            )?;
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
        let interior = orphans
            .iter()
            .zip(&strict)
            .filter(|&(_, &s)| s)
            .map(|(&p, _)| p)
            .collect();
        Ok(Plan {
            visible,
            created,
            interior,
        })
    }

    /// Applies a plan: inserts the new simplices, turns local links into
    /// arena ids, points each facet across the horizon at its new neighbor,
    /// and removes the visible facets.
    fn commit(&mut self, plan: Plan) -> Vec<FacetId> {
        let Plan {
            visible,
            created,
            interior,
        } = plan;
        self.proved_interior.extend(interior);
        let mut ids = Vec::with_capacity(created.len());
        let mut wiring = Vec::with_capacity(created.len());
        for planned in created {
            ids.push(insert_or_abort(&mut self.facets, planned.simplex));
            wiring.push((planned.links, planned.across, planned.replaces));
        }
        for (&id, (links, across, replaces)) in ids.iter().zip(wiring) {
            let neighbors: Vec<FacetId> = links
                .iter()
                .map(|link| match *link {
                    Link::Old(old) => old,
                    Link::New(k) => ids[k],
                })
                .collect();
            if let Some(f) = self.facets.get_mut(id) {
                f.neighbors = neighbors;
            }
            if let Some(n) = self.facets.get_mut(across) {
                if let Some(back) = n.neighbors.iter_mut().find(|f| **f == replaces) {
                    *back = id;
                }
            }
        }
        for id in visible {
            self.facets.remove(id);
        }
        ids
    }
}

/// How the points of a batch are planned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Execution {
    /// On the calling thread.
    Sequential,
    /// On rayon's global pool.
    Parallel,
}

/// The insertion of one point, prepared against the hull before its round.
struct Plan {
    /// Facets to remove.
    visible: Vec<FacetId>,
    /// New simplices, numbered locally by position.
    created: Vec<Planned>,
    /// Outside points of the visible facets that are strictly inside every
    /// new simplex, for [`SimplicialHull::proved_interior`].
    interior: Vec<u32>,
}

impl Plan {
    /// Everything a commit writes, without the derived planes: removed
    /// facets, and per new simplex its vertices, links, the facet across,
    /// and its outside points.
    #[cfg(debug_assertions)]
    fn shape(&self) -> PlanShape {
        (
            self.visible.clone(),
            self.created
                .iter()
                .map(|c| {
                    (
                        c.simplex.vertices.clone(),
                        c.links.clone(),
                        c.across,
                        c.replaces,
                        c.simplex.outside.clone(),
                    )
                })
                .collect(),
        )
    }
}

/// See [`Plan::shape`].
#[cfg(debug_assertions)]
type PlanShape = (
    Vec<FacetId>,
    Vec<(Vec<u32>, Vec<Link>, FacetId, FacetId, Vec<u32>)>,
);

/// A new simplex of a plan.
struct Planned {
    /// The simplex, with its outside points; neighbors are set at commit.
    simplex: Simplex,
    /// Neighbor per slot, by arena id or by local number.
    links: Vec<Link>,
    /// The facet across the horizon ridge this simplex is built on.
    across: FacetId,
    /// The visible facet that `across` points at until the commit.
    replaces: FacetId,
}

/// A neighbor reference inside a plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    /// A facet of the hull before the round (across the horizon).
    Old(FacetId),
    /// The new simplex with this local number.
    New(usize),
}

/// The exact side of `point` relative to `facet`: [`Sign::Positive`] is
/// strictly outside, [`Sign::Zero`] on the supporting hyperplane.
///
/// A facet with a certified working normal first tries to prove the strict
/// side from the working distance (design §1); the orientation decides
/// everything else. Debug builds check every proved side against the
/// orientation.
fn side(input: &Input<'_>, facet: &Simplex, point: u32) -> Result<Sign, ConvexHullError> {
    if let Some(proved) = facet
        .cull
        .as_ref()
        .and_then(|cull| cull.proved_side(input.coords(point), input.height_bound(point)))
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

/// The side of `point` relative to `facet` by orientation alone.
fn oriented_side(input: &Input<'_>, facet: &Simplex, point: u32) -> Result<Sign, ConvexHullError> {
    let mut indices = facet.vertices.clone();
    indices.push(point);
    let sign = input.orient(&indices)?;
    Ok(if facet.outward == Sign::Positive {
        sign
    } else {
        sign.reversed()
    })
}

/// The working distance of `point` from `facet`, or `None` without a
/// certified working normal.
fn working_distance(input: &Input<'_>, facet: &Simplex, point: u32) -> Option<f64> {
    let normal = facet.normal()?;
    let origin = input.coords(facet.vertices[0]);
    Some(
        input
            .coords(point)
            .iter()
            .zip(origin)
            .zip(normal)
            .map(|((x, o), n)| (x - o) * n)
            .sum(),
    )
}

/// The point of `facet.outside` farthest by working distance, ties by
/// the smaller index, with its distance. Without a working normal (or
/// with a NaN distance) the smallest index is taken, and its distance is
/// `None`, which packs after every finite distance.
fn farthest(input: &Input<'_>, facet: &Simplex) -> Option<(u32, Option<f64>)> {
    let mut best: Option<(u32, Option<f64>)> = None;
    for &p in &facet.outside {
        let d = working_distance(input, facet, p).filter(|d| !d.is_nan());
        best = match best {
            Some((b, bd)) if !packs_before((p, d), (b, bd)) => Some((b, bd)),
            _ => Some((p, d)),
        };
    }
    best
}

/// Moves the points of `remaining` strictly outside `facet` into its outside
/// set, keeping the order of both lists. `strict[i]` belongs to
/// `remaining[i]` and is cleared when that point is on the supporting
/// hyperplane of `facet`; a culled point is proved strictly inside.
/// `inside` is scratch for the scan.
fn take_outside(
    input: &Input<'_>,
    remaining: &mut Vec<u32>,
    strict: &mut Vec<bool>,
    inside: &mut Vec<bool>,
    facet: &mut Simplex,
) -> Result<(), ConvexHullError> {
    inside.clear();
    inside.resize(remaining.len(), false);
    if let Some(cull) = &facet.cull {
        let (rows, stride) = input.engine_rows();
        cull.mark_inside(rows, stride, remaining, inside);
    }
    // Kept points move down in place, in order.
    let mut kept = 0;
    for k in 0..remaining.len() {
        let p = remaining[k];
        let sign = if inside[k] {
            // A culled point is dropped without `side`, so debug builds check
            // the scan's proof here, as `side` checks `proved_side`.
            #[cfg(debug_assertions)]
            debug_assert_eq!(
                oriented_side(input, facet, p)?,
                Sign::Negative,
                "the scan culled point {p}, which is not strictly inside"
            );
            Sign::Negative
        } else {
            side(input, facet, p)?
        };
        if sign == Sign::Positive {
            facet.outside.push(p);
        } else {
            remaining[kept] = p;
            strict[kept] = strict[k] && sign == Sign::Negative;
            kept += 1;
        }
    }
    remaining.truncate(kept);
    strict.truncate(kept);
    // The outside set is final: points leave it only with the facet.
    facet.farthest = farthest(input, facet);
    Ok(())
}

/// The region a point would replace: its visible facets V and its horizon
/// ridges, each with the facet across it (N).
#[derive(Clone, Debug, PartialEq)]
struct Region {
    visible: Vec<FacetId>,
    /// (visible facet, slot of the ridge in it, facet across the ridge).
    horizon: Vec<(FacetId, usize, FacetId)>,
}

impl Region {
    /// T = V ∪ N, with repeats.
    fn touched(&self) -> impl Iterator<Item = FacetId> + '_ {
        self.visible
            .iter()
            .copied()
            .chain(self.horizon.iter().map(|&(_, _, n)| n))
    }
}

/// Per-search marks reused across walks and rounds, so neither hashes nor
/// allocates per facet (#120).
#[derive(Default)]
struct WalkScratch {
    /// Facets the current walk has decided: visible or not.
    visited: SlotMarks<bool>,
    /// T of the candidates taken in the current round.
    taken: SlotMarks<()>,
    /// Candidates of the facets with outside points; see
    /// [`SimplicialHull::candidates`].
    pending: Vec<(u32, FacetId, Option<f64>)>,
    /// Whether `pending` has been filled from the arena.
    seeded: bool,
}

/// Packing order of design §6: a larger working distance first, ties by the
/// smaller index; a missing distance packs after every present one.
fn packs_before(a: (u32, Option<f64>), b: (u32, Option<f64>)) -> bool {
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
    use std::collections::HashSet;

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
        SimplicialHull::build(accept(dim, points).unwrap(), Execution::Sequential).unwrap()
    }

    /// No representative strictly outside any facet, and neighbor links are
    /// symmetric across the same ridge.
    pub(crate) fn check_invariants(hull: &SimplicialHull<'_>) {
        for (id, facet) in hull.facets.iter() {
            for &p in &hull.input.representatives {
                assert_ne!(
                    hull.side(facet, p).unwrap(),
                    Sign::Positive,
                    "point {p} outside"
                );
            }
            for (slot, &n) in facet.neighbors.iter().enumerate() {
                let other = hull.facets.get(n).expect("neighbor is live");
                let back = other
                    .neighbors
                    .iter()
                    .position(|&b| b == id)
                    .expect("link back");
                let mut a: Vec<u32> = facet
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v)
                    .collect();
                let mut b: Vec<u32> = other
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != back)
                    .map(|(_, &v)| v)
                    .collect();
                a.sort_unstable();
                b.sort_unstable();
                assert_eq!(a, b, "neighbors share the ridge");
            }
            assert!(facet.outside.is_empty());
        }
    }

    /// Every side a lifted facet's cull plane proves, by `proved_side` or by
    /// the scan, is the exact lifted orientation sign. Returns the number of
    /// cull planes, of proved sides, and of sides left to the orientation.
    fn check_lifted_proofs(dim: usize, points: &[f64]) -> (usize, usize, usize) {
        let Ok(input) = accept(dim, points).unwrap().lift().unwrap() else {
            panic!("the sites are not all cospherical, so the lift is not flat");
        };
        let hull = SimplicialHull::build(input, Execution::Sequential).unwrap();
        let input = &hull.input;
        let (rows, stride) = input.engine_rows();
        let sites = &input.representatives;
        let (mut planes, mut proved, mut open) = (0, 0, 0);
        for (_, facet) in hull.facets.iter() {
            let Some(cull) = facet.cull() else {
                continue;
            };
            planes += 1;
            let mut inside = vec![false; sites.len()];
            cull.mark_inside(rows, stride, sites, &mut inside);
            let mut scalar = vec![false; sites.len()];
            cull.mark_inside_scalar(rows, stride, sites, &mut scalar);
            assert_eq!(
                inside, scalar,
                "the vector and scalar scans disagree on lifted rows"
            );
            for (&p, &culled) in sites.iter().zip(&inside) {
                let exact = oriented_side(input, facet, p).unwrap();
                match cull.proved_side(input.coords(p), input.height_bound(p)) {
                    Some(sign) => {
                        assert_eq!(sign, exact, "facet {:?}, site {p}", facet.vertices);
                        proved += 1;
                    }
                    None => open += 1,
                }
                if culled {
                    assert_eq!(exact, Sign::Negative, "culled site {p}");
                }
            }
        }
        (planes, proved, open)
    }

    #[test]
    fn lifted_proved_sides_are_the_exact_sides() {
        // x1^2 + y1^2 = x2^2 + y2^2 = N (two products of sums of two squares,
        // Brahmagupta-Fibonacci), N about 2^57.1. Their rounded heights (the
        // sum of the rounded squares) differ by 32, so without the height bounds a side would
        // be proved for exactly cocircular sites. Sites one unit off the
        // circle, inside it so the cocircular sites keep their upper facets,
        // are provable. The small circle has exact heights; at 2^-20
        // and 2^20 it checks the scaling.
        let (x1, y1) = (8_362_900.0, 392_702_530.0);
        let (x2, y2) = (377_454_220.0, 108_690_050.0);
        let mut big: Vec<[f64; 2]> = Vec::new();
        for (x, y) in [(x1, y1), (y1, x1), (x2, y2), (y2, x2)] {
            big.extend([[x, y], [-x, y], [x, -y], [-x, -y]]);
        }
        big.extend([[x1 - 1.0, y1], [x2, y2 - 1.0], [0.0, 0.0]]);
        let small: Vec<[f64; 2]> = vec![
            [5.0, 0.0],
            [0.0, 5.0],
            [-5.0, 0.0],
            [0.0, -5.0],
            [3.0, 4.0],
            [4.0, 3.0],
            [-3.0, 4.0],
            [4.0, -3.0],
            [3.0, 5.0],
            [5.0, 1.0],
            [0.0, 0.0],
        ];
        let mut sphere: Vec<[f64; 3]> = Vec::new();
        for p in [
            [x1, y1, 0.0],
            [x2, y2, 0.0],
            [y1, 0.0, x1],
            [0.0, x2, y2],
            [0.0, y1, x1],
            [y2, 0.0, x2],
        ] {
            for signs in 0..4 {
                let mut q = p;
                if signs & 1 == 1 {
                    q[0] = -q[0];
                }
                if signs & 2 == 2 {
                    q[1] = -q[1];
                }
                sphere.push(q);
            }
        }
        sphere.extend([[x1 - 1.0, y1, 0.0], [0.0, x2, y2 - 1.0], [0.0, 0.0, 0.0]]);
        let cases: Vec<(String, usize, Vec<f64>)> = vec![
            (
                "big circle".into(),
                2,
                big.iter().flatten().copied().collect(),
            ),
            (
                "big sphere".into(),
                3,
                sphere.iter().flatten().copied().collect(),
            ),
            (
                "small circle".into(),
                2,
                small.iter().flatten().copied().collect(),
            ),
            (
                "small circle 2^-20".into(),
                2,
                small.iter().flatten().map(|x| x * 2f64.powi(-20)).collect(),
            ),
            (
                "small circle 2^20".into(),
                2,
                small.iter().flatten().map(|x| x * 2f64.powi(20)).collect(),
            ),
        ];
        for (name, dim, points) in &cases {
            let (planes, proved, open) = check_lifted_proofs(*dim, points);
            assert!(planes > 0 && proved > 0, "{name}: nothing proved");
            assert!(open > 0, "{name}: every cospherical side was proved");
        }
        // Heights that overflow to infinity give no working normal, so no
        // cull plane, and every side goes to the orientation.
        let huge: Vec<f64> = small.iter().flatten().map(|x| x * 2f64.powi(520)).collect();
        assert_eq!(check_lifted_proofs(2, &huge), (0, 0, 0));
    }

    #[test]
    fn lifted_working_distance_reads_the_lifted_coordinate() {
        // On the lift, the working distance is the dot product over all
        // D + 1 engine coordinates; dropping the lifted one changes it.
        let mut rng = Rng(7);
        let points: Vec<f64> = (0..2 * 40).map(|_| rng.unit()).collect();
        let Ok(input) = accept(2, &points).unwrap().lift().unwrap() else {
            panic!("random sites lift to a full-dimensional set");
        };
        let hull = SimplicialHull::build(input, Execution::Sequential).unwrap();
        let mut checked = 0;
        for (_, facet) in hull.facets.iter() {
            let Some(normal) = facet.normal() else {
                continue;
            };
            assert_eq!(normal.len(), 3);
            let origin = hull.input.coords(facet.vertices[0]);
            for &p in &hull.input.representatives {
                let full: f64 = hull
                    .input
                    .coords(p)
                    .iter()
                    .zip(origin)
                    .zip(normal)
                    .map(|((x, o), n)| (x - o) * n)
                    .sum();
                assert_eq!(hull.working_distance(facet, p), Some(full));
                checked += 1;
            }
        }
        assert!(checked > 0);
    }

    fn vertex_set(hull: &SimplicialHull<'_>) -> Vec<u32> {
        let mut v: Vec<u32> = hull
            .facets
            .iter()
            .flat_map(|(_, f)| f.vertices.clone())
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
            facets: Arena::new(),
            proved_interior: Vec::new(),
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
    fn packing_order_is_distance_then_index() {
        assert!(packs_before((7, Some(2.0)), (3, Some(1.0))));
        assert!(!packs_before((3, Some(1.0)), (7, Some(2.0))));
        // Ties by the smaller index.
        assert!(packs_before((3, Some(1.0)), (7, Some(1.0))));
        assert!(!packs_before((7, Some(1.0)), (3, Some(1.0))));
        // A missing distance packs after any present one, then by index.
        assert!(packs_before((9, Some(-1.0)), (2, None)));
        assert!(packs_before((2, None), (9, None)));
    }

    #[test]
    fn candidate_is_the_farthest_with_ties_by_index() {
        // Triangle (0,0), (4,0), (0,4). Facet y = 0 sees 3 (0.5, -1),
        // 4 (2, -3), and 5 (3, -3); 4 and 5 tie at distance 3, so 4 wins.
        let points = [
            0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 0.5, -1.0, 2.0, -3.0, 3.0, -3.0,
        ];
        let hull = initial(2, &points);
        let candidates = hull.all_candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, 4);
        assert_eq!(candidates[0].2, Some(3.0));
    }

    #[test]
    fn overlapping_regions_wait_for_a_later_round() {
        // In a triangle every region T is the whole hull. Point 3 is far
        // below the bottom edge, point 4 just left of the left edge, so 3
        // packs first and 4 waits in its outside set.
        let points = [0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 2.0, -10.0, -1.0, 2.0];
        let mut hull = initial(2, &points);
        let batch = hull.next_batch().unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].0, 3);
        hull.insert_point(batch[0].1, 3).unwrap();
        let next: Vec<u32> = hull.all_candidates().iter().map(|c| c.0).collect();
        assert_eq!(next, vec![4], "the deferred point is proposed again");
    }

    /// A taken candidate in the replay: point, T, H, prospective simplices.
    type Taken = (u32, HashSet<FacetId>, HashSet<Vec<u32>>, Vec<Vec<u32>>);

    /// Statistics of the rounds of one build.
    #[derive(Default)]
    struct Rounds {
        /// The largest batch.
        largest: usize,
        /// Candidates whose T and H were free but that were dropped for a
        /// prospective-simplex conflict.
        prospective_drops: usize,
    }

    /// Replays every round against an independent statement of the §6
    /// packing: in packing order, a candidate is taken exactly when its T
    /// and H miss every taken candidate's T and H and neither it nor a taken
    /// candidate is strictly outside a prospective simplex of the other.
    /// Batches are in ascending index, and the final hull is valid.
    fn check_rounds(dim: usize, points: &[f64]) -> Rounds {
        let mut hull = initial(dim, points);
        let mut stats = Rounds::default();
        loop {
            let candidates = hull.all_candidates();
            let batch = hull.next_batch().unwrap();
            if batch.is_empty() {
                assert!(candidates.is_empty());
                break;
            }
            stats.largest = stats.largest.max(batch.len());
            assert!(batch.windows(2).all(|w| w[0].0 < w[1].0));
            let mut taken: Vec<Taken> = Vec::new();
            for &(p, f, _) in &candidates {
                let region = hull.visible_region(f, p).unwrap();
                let t: HashSet<FacetId> = region.touched().collect();
                let h: HashSet<Vec<u32>> = hull.horizon_ridges(&region).into_iter().collect();
                let prospective = hull.prospective(&region, p);
                let free = taken
                    .iter()
                    .all(|(_, tt, th, _)| tt.is_disjoint(&t) && th.is_disjoint(&h));
                let conflict = taken.iter().any(|(q, _, _, qp)| {
                    hull.outside_any(qp, p).unwrap() || hull.outside_any(&prospective, *q).unwrap()
                });
                let expected = free && !conflict;
                assert_eq!(batch.contains(&(p, f)), expected, "candidate {p}");
                if free && conflict {
                    stats.prospective_drops += 1;
                }
                if expected {
                    taken.push((p, t, h, prospective));
                }
            }
            for (p, f) in batch {
                hull.insert_point(f, p).unwrap();
            }
        }
        check_invariants(&hull);
        stats
    }

    #[test]
    fn rounds_follow_the_reservation_and_the_prospective_test() {
        let mut rng = Rng(21);
        let mut largest = 0;
        let mut drops = 0;
        for (dim, count) in [(2, 400), (3, 300), (4, 120)] {
            for _ in 0..3 {
                let points: Vec<f64> = (0..count * dim).map(|_| rng.unit()).collect();
                let stats = check_rounds(dim, &points);
                largest = largest.max(stats.largest);
                drops += stats.prospective_drops;
            }
        }
        assert!(largest >= 2, "some round packs more than one point");
        // With T and H free, a prospective conflict cannot occur (see
        // `conflicts`); the replay above would have counted one.
        assert_eq!(drops, 0);
    }

    #[test]
    fn prospective_conflicts_in_either_direction() {
        // Triangle (0,0), (4,0), (0,4). P = 3 = (2, -10) sees the bottom
        // edge; its prospective edges join (0,0) and (4,0) to P. Q = 4 =
        // (30, -5) is outside the edge from P to (4, 0). R = 5 = (1, 1) is
        // inside the triangle and outside nothing.
        let points = [
            0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 2.0, -10.0, 30.0, -5.0, 1.0, 1.0,
        ];
        let hull = initial(2, &points);
        let start = hull
            .facets
            .iter()
            .find(|(_, f)| f.outside.contains(&3))
            .map(|(id, _)| id)
            .unwrap();
        let p = hull.prospective(&hull.visible_region(start, 3).unwrap(), 3);
        assert_eq!(p.len(), 2);
        let none: Vec<Vec<u32>> = Vec::new();
        // Q outside a prospective simplex of P, in either argument order.
        assert!(hull.conflicts((3, &p), (4, &none)).unwrap());
        assert!(hull.conflicts((4, &none), (3, &p)).unwrap());
        // R outside none of them.
        assert!(!hull.conflicts((3, &p), (5, &none)).unwrap());
        assert!(!hull.conflicts((5, &none), (3, &p)).unwrap());
    }

    #[test]
    fn far_apart_points_share_a_batch() {
        // An octagon, then 8 and 9 just outside two opposite edges. Once the
        // octagon is built, their regions (one edge and its two neighbors)
        // are disjoint and neither sees the other's new edges.
        let points = [
            10.0, 0.0, 7.0, 7.0, 0.0, 10.0, -7.0, 7.0, -10.0, 0.0, -7.0, -7.0, 0.0, -10.0, 7.0,
            -7.0, 8.6, 3.55, -8.6, -3.55,
        ];
        let mut hull = initial(2, &points);
        let mut seen = false;
        loop {
            let candidates: Vec<u32> = hull.all_candidates().iter().map(|c| c.0).collect();
            let batch = hull.next_batch().unwrap();
            if batch.is_empty() {
                break;
            }
            if candidates == vec![8, 9] || candidates == vec![9, 8] {
                let taken: Vec<u32> = batch.iter().map(|b| b.0).collect();
                assert_eq!(taken, vec![8, 9]);
                seen = true;
            }
            for (p, f) in batch {
                hull.insert_point(f, p).unwrap();
            }
        }
        assert!(seen, "a round proposed exactly 8 and 9");
        check_invariants(&hull);
    }

    #[test]
    fn a_shared_horizon_ridge_keeps_points_apart() {
        // Square, then 4 and 5 just outside the two edges at corner (1, 1):
        // both horizons contain the ridge {2} (the corner), so they never
        // share a batch.
        let points = [
            0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0001, 0.5, 0.5, 1.0001,
        ];
        let mut hull = initial(2, &points);
        loop {
            let batch = hull.next_batch().unwrap();
            if batch.is_empty() {
                break;
            }
            let taken: Vec<u32> = batch.iter().map(|b| b.0).collect();
            assert!(!(taken.contains(&4) && taken.contains(&5)), "{taken:?}");
            for (p, f) in batch {
                hull.insert_point(f, p).unwrap();
            }
        }
        check_invariants(&hull);
    }

    /// The arena as (id, vertices, neighbors, outward, outside), in id order.
    type Snapshot = Vec<(FacetId, Vec<u32>, Vec<FacetId>, Sign, Vec<u32>)>;

    fn snapshot(hull: &SimplicialHull<'_>) -> Snapshot {
        hull.facets
            .iter()
            .map(|(id, f)| {
                (
                    id,
                    f.vertices.clone(),
                    f.neighbors.clone(),
                    f.outward,
                    f.outside.clone(),
                )
            })
            .collect()
    }

    #[test]
    fn parallel_planning_commits_the_same_arena() {
        let mut rng = Rng(34);
        for (dim, count) in [(2, 500), (3, 400), (4, 150), (5, 60)] {
            let points: Vec<f64> = (0..count * dim).map(|_| rng.unit()).collect();
            let sequential =
                SimplicialHull::build(accept(dim, &points).unwrap(), Execution::Sequential)
                    .unwrap();
            let parallel =
                SimplicialHull::build(accept(dim, &points).unwrap(), Execution::Parallel).unwrap();
            assert_eq!(snapshot(&sequential), snapshot(&parallel), "dim {dim}");
            check_invariants(&parallel);
        }
    }

    #[test]
    fn segment_in_one_dimension() {
        let points = [3.0, -1.0, 2.0, -1.0, 7.0, 0.5];
        let hull = build(1, &points);
        assert_eq!(hull.facets.len(), 2);
        assert_eq!(vertex_set(&hull), vec![1, 4]);
        for (_, f) in hull.facets.iter() {
            assert!(f.neighbors.is_empty());
        }
        check_invariants(&hull);
    }

    #[test]
    fn square_with_interior_and_edge_points() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5, 0.5, 0.0];
        let hull = build(2, &points);
        check_invariants(&hull);
        assert_eq!(vertex_set(&hull), vec![0, 1, 2, 3]);
        assert_eq!(hull.facets.len(), 4);
    }

    #[test]
    fn a_basis_point_on_a_collinear_edge_stays_a_simplicial_vertex() {
        // Review of #36: (1, 0) belongs to the minimum basis; (2, 0) is on
        // the line of the edge (0,0)-(1,0), not strictly outside it, so that
        // facet is not visible and (1, 0) stays a vertex of the simplicial
        // complex. Insertion does not merge (§5); P2-3 (#10) classifies
        // (1, 0) as a non-extreme boundary point after the merge.
        let points = [0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let hull = build(2, &points);
        check_invariants(&hull);
        assert_eq!(hull.input.spanning_points, vec![0, 1, 3]);
        assert_eq!(vertex_set(&hull), vec![0, 1, 2, 3, 4]);
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
