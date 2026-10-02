//! Merge of exactly coplanar simplices into logical facets (design §5).
//!
//! After every point has been inserted, two simplices that share a ridge are
//! in one logical facet exactly when the orientation of one simplex's
//! vertices and the other simplex's vertex across the ridge is zero. Groups
//! are the connected components of that relation, found with Union-Find over
//! the ridges. One supporting plane of a convex polytope cuts one face, and
//! the simplices of that face are connected through ridges, so pairwise ridge
//! tests are enough.

use std::collections::HashMap;

use super::simplicial::SimplicialHull;
use super::ConvexHullError;
use crate::arena::FacetId;
use crate::predicates::{orient, Sign};

/// A logical facet during construction. `GroupId` in the design is the index
/// of the group in [`LogicalFacets::groups`]; it exists only here.
pub(crate) struct Group {
    /// Member simplices in arena order.
    pub(crate) simplices: Vec<FacetId>,
    /// Union of the member vertices, ascending.
    pub(crate) vertices: Vec<u32>,
    /// Indices of the neighboring groups, ascending.
    pub(crate) neighbors: Vec<u32>,
}

pub(crate) struct LogicalFacets {
    pub(crate) groups: Vec<Group>,
}

struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n as u32).collect(),
        }
    }

    fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let grand = self.parent[self.parent[x as usize] as usize];
            self.parent[x as usize] = grand;
            x = grand;
        }
        x
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        // The smaller root wins, so the result does not depend on the order
        // of the unions.
        if ra < rb {
            self.parent[rb as usize] = ra;
        } else if rb < ra {
            self.parent[ra as usize] = rb;
        }
    }
}

/// Groups the simplices of `hull` into logical facets.
pub(crate) fn merge(hull: &SimplicialHull<'_>) -> Result<LogicalFacets, ConvexHullError> {
    let ids: Vec<FacetId> = hull.facets.iter().map(|(id, _)| id).collect();
    let dense: HashMap<FacetId, u32> = ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i as u32))
        .collect();
    let mut sets = UnionFind::new(ids.len());

    for (i, &id) in ids.iter().enumerate() {
        let Some(facet) = hull.facets.get(id) else {
            continue;
        };
        for &neighbor in &facet.neighbors {
            let Some(&j) = dense.get(&neighbor) else {
                continue;
            };
            // Each ridge is tested once, from the lower dense index.
            if (j as usize) < i {
                continue;
            }
            let Some(other) = hull.facets.get(neighbor) else {
                continue;
            };
            let Some(&across) = other.vertices.iter().find(|v| !facet.vertices.contains(v)) else {
                continue;
            };
            let mut points: Vec<&[f64]> = facet
                .vertices
                .iter()
                .map(|&v| hull.input.point(v))
                .collect();
            points.push(hull.input.point(across));
            if orient(&points)? == Sign::Zero {
                sets.union(i as u32, j);
            }
        }
    }

    // Number groups by their smallest member, in arena order.
    let mut number_of_root: HashMap<u32, u32> = HashMap::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut group_of = HashMap::with_capacity(ids.len());
    for (i, &id) in ids.iter().enumerate() {
        let root = sets.find(i as u32);
        let number = *number_of_root.entry(root).or_insert_with(|| {
            groups.push(Group {
                simplices: Vec::new(),
                vertices: Vec::new(),
                neighbors: Vec::new(),
            });
            groups.len() as u32 - 1
        });
        groups[number as usize].simplices.push(id);
        group_of.insert(id, number);
    }

    for group in &mut groups {
        for id in &group.simplices {
            if let Some(facet) = hull.facets.get(*id) {
                group.vertices.extend_from_slice(&facet.vertices);
            }
        }
        group.vertices.sort_unstable();
        group.vertices.dedup();
    }
    let neighbor_sets: Vec<Vec<u32>> = groups
        .iter()
        .enumerate()
        .map(|(number, group)| {
            let mut neighbors: Vec<u32> = group
                .simplices
                .iter()
                .filter_map(|id| hull.facets.get(*id))
                .flat_map(|facet| facet.neighbors.iter())
                .filter_map(|n| group_of.get(n).copied())
                .filter(|&g| g as usize != number)
                .collect();
            neighbors.sort_unstable();
            neighbors.dedup();
            neighbors
        })
        .collect();
    for (group, neighbors) in groups.iter_mut().zip(neighbor_sets) {
        group.neighbors = neighbors;
    }

    Ok(LogicalFacets { groups })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hull::input::accept;
    use crate::hull::simplicial::tests::Rng;

    fn groups_of(dim: usize, points: &[f64]) -> LogicalFacets {
        let hull = SimplicialHull::build(accept(dim, points).unwrap()).unwrap();
        let facets = merge(&hull).unwrap();
        // Every simplex belongs to exactly one group.
        let mut members: Vec<FacetId> = facets
            .groups
            .iter()
            .flat_map(|g| g.simplices.clone())
            .collect();
        let total = members.len();
        members.sort_unstable();
        members.dedup();
        assert_eq!(members.len(), total);
        assert_eq!(total, hull.facets.len());
        // Neighbor sets are symmetric.
        for (g, group) in facets.groups.iter().enumerate() {
            for &n in &group.neighbors {
                assert!(facets.groups[n as usize].neighbors.contains(&(g as u32)));
            }
        }
        facets
    }

    fn cube(dim: usize) -> Vec<f64> {
        (0..1_usize << dim)
            .flat_map(|i| (0..dim).map(move |a| ((i >> a) & 1) as f64))
            .collect()
    }

    #[test]
    fn cubes_merge_into_their_faces() {
        for (dim, faces, simplices_per_face) in [(2, 4, Some(1)), (3, 6, Some(2)), (4, 8, None)] {
            let facets = groups_of(dim, &cube(dim));
            assert_eq!(facets.groups.len(), faces, "dim {dim}");
            for group in &facets.groups {
                assert_eq!(group.vertices.len(), 1 << (dim - 1), "dim {dim}");
                // A 3-cube splits into 5 or 6 tetrahedra depending on the
                // diagonals, so only the lower dimensions fix the count.
                if let Some(count) = simplices_per_face {
                    assert_eq!(group.simplices.len(), count, "dim {dim}");
                }
                // Every face of a cube meets every other face except the
                // opposite one.
                assert_eq!(group.neighbors.len(), faces - 2, "dim {dim}");
            }
        }
    }

    #[test]
    fn segment_has_two_groups() {
        let facets = groups_of(1, &[0.0, 2.0, 1.0]);
        assert_eq!(facets.groups.len(), 2);
        assert!(facets.groups.iter().all(|g| g.neighbors.is_empty()));
    }

    #[test]
    fn random_simplicial_input_keeps_one_simplex_per_group() {
        let mut rng = Rng(21);
        for dim in 2..=4 {
            let points: Vec<f64> = (0..60 * dim).map(|_| rng.unit()).collect();
            let facets = groups_of(dim, &points);
            assert!(
                facets.groups.iter().all(|g| g.simplices.len() == 1),
                "dim {dim}"
            );
        }
    }

    #[test]
    fn integer_grid_has_six_faces() {
        let mut grid = Vec::new();
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    grid.extend([i as f64, j as f64, k as f64]);
                }
            }
        }
        let facets = groups_of(3, &grid);
        assert_eq!(facets.groups.len(), 6);
    }

    #[test]
    fn octahedron_faces_stay_separate() {
        let points = [
            1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            -1.0,
        ];
        let facets = groups_of(3, &points);
        assert_eq!(facets.groups.len(), 8);
    }
}
