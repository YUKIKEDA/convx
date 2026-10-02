//! Sequential Quickhull through insertion, kept simplicial (design §4, §6).
//!
//! Every facet is a (D-1)-simplex of D vertices. `neighbors[i]` is the facet
//! across the ridge opposite `vertices[i]`. A point is outside a facet when
//! the orientation of the facet's vertices followed by the point has the
//! facet's `outward` sign; for D >= 2 the vertex order is chosen so that this
//! sign is always [`Sign::Positive`], and only the two endpoints of D = 1
//! need a stored sign. Visibility is decided by that exact orientation alone.
//! No facets are merged during insertion.

use std::collections::{HashMap, VecDeque};

use super::input::Input;
use super::ConvexHullError;
use crate::arena::{Arena, ArenaFull, FacetId};
use crate::cull::CullPlane;
use crate::normal::unit_normal;
use crate::predicates::{orient, Sign};

/// A simplicial facet during construction.
pub(crate) struct Simplex {
    pub(crate) vertices: Vec<u32>,
    pub(crate) neighbors: Vec<FacetId>,
    /// The sign of `orient(vertices, q)` for a point `q` outside.
    pub(crate) outward: Sign,
    /// Working unit normal, when one could be certified.
    normal: Option<Vec<f64>>,
    cull: Option<CullPlane>,
    /// Points assigned to this facet that are strictly outside it.
    outside: Vec<u32>,
}

impl Simplex {
    /// The cull plane of this simplex, when one could be certified.
    pub(crate) fn cull(&self) -> Option<&CullPlane> {
        self.cull.as_ref()
    }
}

/// The simplicial hull after every point has been absorbed.
pub(crate) struct SimplicialHull<'a> {
    pub(crate) input: Input<'a>,
    pub(crate) facets: Arena<Simplex>,
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
    pub(crate) fn build(input: Input<'a>) -> Result<Self, ConvexHullError> {
        let mut hull = Self {
            input,
            facets: Arena::new(),
        };
        if hull.input.dim() == 1 {
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
            hull.absorb(initial)?;
        }
        Ok(hull)
    }

    fn points_of(&self, vertices: &[u32]) -> Vec<&'a [f64]> {
        vertices.iter().map(|&v| self.input.point(v)).collect()
    }

    /// The exact side of `point` relative to `facet`: [`Sign::Positive`] is
    /// strictly outside, [`Sign::Zero`] on the supporting hyperplane.
    pub(crate) fn side(&self, facet: &Simplex, point: u32) -> Result<Sign, ConvexHullError> {
        let mut points = self.points_of(&facet.vertices);
        points.push(self.input.point(point));
        let sign = orient(&points)?;
        Ok(if facet.outward == Sign::Positive {
            sign
        } else {
            sign.reversed()
        })
    }

    fn make_simplex(
        &self,
        vertices: Vec<u32>,
        neighbors: Vec<FacetId>,
        outward: Sign,
    ) -> Result<Simplex, ConvexHullError> {
        let points = self.points_of(&vertices);
        let normal = unit_normal(&points, outward)?;
        let cull = normal
            .as_deref()
            .and_then(|n| CullPlane::new(&points, n, outward));
        Ok(Simplex {
            vertices,
            neighbors,
            outward,
            normal,
            cull,
            outside: Vec::new(),
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
        let d = self.input.dim();
        let mut ordered = Vec::with_capacity(d + 1);
        for (i, &apex) in simplex.iter().enumerate() {
            let mut vertices: Vec<u32> = simplex.iter().copied().filter(|&v| v != apex).collect();
            let mut points = self.points_of(&vertices);
            points.push(self.input.point(apex));
            if orient(&points)? == Sign::Positive {
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

    /// Assigns each candidate to the first facet in `facets` that it is
    /// strictly outside. Candidates outside none are dropped: they are inside
    /// the current hull or on its boundary.
    fn assign(
        &mut self,
        mut remaining: Vec<u32>,
        facets: &[FacetId],
    ) -> Result<(), ConvexHullError> {
        let points = self.input.points();
        let mut inside = Vec::new();
        for &id in facets {
            if remaining.is_empty() {
                break;
            }
            let Some(facet) = self.facets.get(id) else {
                continue;
            };
            inside.clear();
            inside.resize(remaining.len(), false);
            if let Some(cull) = &facet.cull {
                cull.mark_inside(points, &remaining, &mut inside);
            }
            let mut kept = Vec::with_capacity(remaining.len());
            let mut outside = Vec::new();
            for (&p, &culled) in remaining.iter().zip(&inside) {
                if !culled && self.side(facet, p)? == Sign::Positive {
                    outside.push(p);
                } else {
                    kept.push(p);
                }
            }
            if let Some(facet) = self.facets.get_mut(id) {
                facet.outside = outside;
            }
            remaining = kept;
        }
        Ok(())
    }

    /// The point of `facet.outside` farthest by working distance; ties and a
    /// missing working normal fall back to the smallest index.
    fn farthest(&self, facet: &Simplex) -> Option<u32> {
        let Some(normal) = &facet.normal else {
            return facet.outside.iter().copied().min();
        };
        let origin = self.input.point(facet.vertices[0]);
        let distance = |p: u32| -> f64 {
            self.input
                .point(p)
                .iter()
                .zip(origin)
                .zip(normal)
                .map(|((x, o), n)| (x - o) * n)
                .sum()
        };
        facet
            .outside
            .iter()
            .copied()
            .fold(None, |best: Option<(u32, f64)>, p| {
                let d = distance(p);
                match best {
                    Some((b, bd)) if bd > d || (bd == d && b < p) || d.is_nan() => Some((b, bd)),
                    _ => Some((p, d)),
                }
            })
            .map(|(p, _)| p)
    }

    /// Absorbs every outside point, one point per step.
    fn absorb(&mut self, initial: Vec<FacetId>) -> Result<(), ConvexHullError> {
        let mut queue: VecDeque<FacetId> = initial.into_iter().collect();
        while let Some(id) = queue.pop_front() {
            let Some(facet) = self.facets.get(id) else {
                continue;
            };
            let Some(apex) = self.farthest(facet) else {
                continue;
            };
            let new_facets = self.insert_point(id, apex)?;
            queue.extend(new_facets);
        }
        Ok(())
    }

    /// Replaces the facets visible from `apex`, starting at `start`, with the
    /// cone from `apex` over the horizon. Returns the new facets that have
    /// outside points.
    fn insert_point(&mut self, start: FacetId, apex: u32) -> Result<Vec<FacetId>, ConvexHullError> {
        // Visible region by breadth-first search over neighbors.
        let mut visible = vec![start];
        let mut is_visible: HashMap<FacetId, bool> = HashMap::from([(start, true)]);
        let mut horizon: Vec<(FacetId, usize)> = Vec::new();
        let mut cursor = 0;
        while cursor < visible.len() {
            let id = visible[cursor];
            cursor += 1;
            let neighbors = match self.facets.get(id) {
                Some(f) => f.neighbors.clone(),
                None => continue,
            };
            for (slot, neighbor) in neighbors.into_iter().enumerate() {
                let seen = match is_visible.get(&neighbor) {
                    Some(&v) => v,
                    None => {
                        let v = match self.facets.get(neighbor) {
                            Some(n) => self.side(n, apex)? == Sign::Positive,
                            None => false,
                        };
                        is_visible.insert(neighbor, v);
                        if v {
                            visible.push(neighbor);
                        }
                        v
                    }
                };
                if !seen {
                    horizon.push((id, slot));
                }
            }
        }

        // One new facet per horizon ridge: the visible facet's vertex order
        // with the vertex opposite the ridge replaced by the apex keeps the
        // outward orientation.
        let mut created = Vec::with_capacity(horizon.len());
        let mut ridges: HashMap<Vec<u32>, (FacetId, usize)> = HashMap::new();
        for &(visible_id, slot) in &horizon {
            let Some(old) = self.facets.get(visible_id) else {
                continue;
            };
            let across = old.neighbors[slot];
            let mut vertices = old.vertices.clone();
            vertices[slot] = apex;
            let d = vertices.len();
            let mut neighbors = vec![across; d];
            neighbors[slot] = across;
            let simplex = self.make_simplex(vertices.clone(), neighbors, Sign::Positive)?;
            let id = insert_or_abort(&mut self.facets, simplex);
            if let Some(n) = self.facets.get_mut(across) {
                if let Some(back) = n.neighbors.iter_mut().find(|f| **f == visible_id) {
                    *back = id;
                }
            }
            for other in (0..d).filter(|&m| m != slot) {
                let mut key: Vec<u32> = vertices
                    .iter()
                    .enumerate()
                    .filter(|&(m, _)| m != other)
                    .map(|(_, &v)| v)
                    .collect();
                key.sort_unstable();
                if let Some((twin, twin_slot)) = ridges.remove(&key) {
                    if let Some(f) = self.facets.get_mut(id) {
                        f.neighbors[other] = twin;
                    }
                    if let Some(f) = self.facets.get_mut(twin) {
                        f.neighbors[twin_slot] = id;
                    }
                } else {
                    ridges.insert(key, (id, other));
                }
            }
            created.push(id);
        }
        debug_assert!(
            ridges.is_empty(),
            "every new ridge is shared by two new facets"
        );

        // Reassign the outside points of the deleted facets.
        let mut orphans = Vec::new();
        for id in visible {
            if let Some(old) = self.facets.remove(id) {
                orphans.extend(old.outside.into_iter().filter(|&p| p != apex));
            }
        }
        orphans.sort_unstable();
        self.assign(orphans, &created)?;
        Ok(created
            .into_iter()
            .filter(|&id| self.facets.get(id).is_some_and(|f| !f.outside.is_empty()))
            .collect())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hull::input::accept;

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
