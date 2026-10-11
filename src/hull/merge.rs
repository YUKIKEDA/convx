//! Merge of exactly coplanar simplices into logical facets (design §5).
//!
//! After every point has been inserted, two simplices that share a ridge are
//! in one logical facet exactly when the orientation of one simplex's
//! vertices and the other simplex's vertex across the ridge is zero. Groups
//! are the connected components of that relation, found with Union-Find over
//! the ridges. One supporting plane of a convex polytope cuts one face, and
//! the simplices of that face are connected through ridges, so pairwise ridge
//! tests are enough.

use super::simplicial::SimplicialHull;
use super::ConvexHullError;
use crate::arena::{FacetId, SlotMarks};
use crate::lists::Lists;
use crate::predicates::Sign;
use crate::small::Small;

/// A logical facet during construction. `GroupId` in the design is the index
/// of the group in [`LogicalFacets::groups`]; it exists only here.
pub(crate) struct Group {
    /// Member simplices in store order. Inline for the usual group of one
    /// simplex, so a hull in general position allocates no list per facet.
    pub(crate) simplices: Small<FacetId, 2>,
}

/// The groups, with their vertex and neighbor lists kept flat (#254), so
/// that no group allocates a list.
pub(crate) struct LogicalFacets {
    pub(crate) groups: Vec<Group>,
    /// Per group, the union of the member vertices, ascending.
    pub(crate) vertices: Lists<u32>,
    /// Per group, the indices of the neighboring groups, ascending.
    pub(crate) neighbors: Lists<u32>,
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
    if hull.strict_edges {
        return Ok(polygon_groups(hull));
    }
    merge_except(hull, &SlotMarks::default())
}

/// One group per edge, in chain order. Adjacent edges are not collinear.
fn polygon_groups(hull: &SimplicialHull<'_>) -> LogicalFacets {
    let cycle = &hull.polygon;
    let n = cycle.len();
    let mut groups = Vec::with_capacity(n);
    let mut vertices = Lists::with_capacity(n, 2 * n);
    let mut neighbors = Lists::with_capacity(n, 2 * n);
    for i in 0..n {
        let (start, end) = (cycle[i], cycle[(i + 1) % n]);
        vertices.push(&[start.min(end), start.max(end)]);
        let prev = if i == 0 { n - 1 } else { i - 1 } as u32;
        let next = ((i + 1) % n) as u32;
        neighbors.push(&[prev.min(next), prev.max(next)]);
        groups.push(Group {
            simplices: Small::new(),
        });
    }
    LogicalFacets {
        groups,
        vertices,
        neighbors,
    }
}

/// [`merge`] without testing a ridge of a simplex marked in `cut`: each cut
/// simplex stays a group of its own. A caller cuts only simplices whose
/// group it has already decided to drop, and a cut simplex is coplanar with
/// no simplex it keeps, so the groups of the kept simplices are those of
/// [`merge`].
pub(crate) fn merge_except(
    hull: &SimplicialHull<'_>,
    cut: &SlotMarks<()>,
) -> Result<LogicalFacets, ConvexHullError> {
    // The arena's iterator cannot say how many entries are live, so the
    // lists below are sized from its count: grown by doubling, each would be
    // copied a dozen times (#209).
    let mut ids: Vec<FacetId> = Vec::with_capacity(hull.facets.len());
    ids.extend(hull.facets.iter().map(|(id, _)| id));
    let dense = Dense::of(&ids);
    let mut sets = UnionFind::new(ids.len());
    let mut joined = false;

    for (i, &id) in ids.iter().enumerate() {
        let Some(facet) = hull.facets.get(id) else {
            continue;
        };
        if cut.contains(id) {
            continue;
        }
        for neighbor in facet.neighbors() {
            let Some(j) = dense.get(&ids, neighbor) else {
                continue;
            };
            if cut.contains(neighbor) {
                continue;
            }
            // Each ridge is tested once, from the lower dense index.
            if (j as usize) < i {
                continue;
            }
            let Some(other) = hull.facets.get(neighbor) else {
                continue;
            };
            let Some(&across) = other
                .vertices()
                .iter()
                .find(|v| !facet.vertices().contains(v))
            else {
                continue;
            };
            // The side of `across` is the orientation of the facet's
            // vertices and that point, up to the outward sign; the certified
            // cull usually proves it nonzero first.
            if hull.side(facet, across)? == Sign::Zero {
                sets.union(i as u32, j);
                joined = true;
            }
        }
    }

    if !joined {
        let fast = singletons(hull, &ids, &dense);
        debug_assert!(
            same_groups(&fast, &grouped(hull, &ids, &mut sets)),
            "the groups of one simplex each are those of the general path"
        );
        return Ok(fast);
    }
    Ok(grouped(hull, &ids, &mut sets))
}

/// The dense index of each live simplex, by its slot: the neighbors of a
/// live simplex are live, so a slot names one of them, and an array by slot
/// is a quarter of the reads of [`SlotMarks`] in the ridge loop (#485).
struct Dense {
    by_slot: Vec<u32>,
}

impl Dense {
    fn of(ids: &[FacetId]) -> Self {
        let slots = ids
            .iter()
            .map(|id| id.index() as usize + 1)
            .max()
            .unwrap_or(0);
        let mut by_slot = vec![u32::MAX; slots];
        for (i, id) in ids.iter().enumerate() {
            by_slot[id.index() as usize] = i as u32;
        }
        Self { by_slot }
    }

    /// The dense index of `id`, a neighbor of a live simplex.
    fn get(&self, ids: &[FacetId], id: FacetId) -> Option<u32> {
        let j = *self.by_slot.get(id.index() as usize)?;
        if j == u32::MAX {
            return None;
        }
        debug_assert!(
            ids[j as usize] == id,
            "a neighbor of a live simplex is live"
        );
        Some(j)
    }
}

/// The groups of [`merge_except`] from the joined ridges in `sets`, over the
/// live simplices `ids` in arena order.
fn grouped(hull: &SimplicialHull<'_>, ids: &[FacetId], sets: &mut UnionFind) -> LogicalFacets {
    // Number groups by their smallest member, in arena order.
    // A root is a dense index; a group is numbered when its root is first
    // met, and `u32::MAX` marks a root not met yet.
    let mut number_of_root = vec![u32::MAX; ids.len()];
    let roots = (0..ids.len() as u32).filter(|&i| sets.find(i) == i).count();
    let mut groups: Vec<Group> = Vec::with_capacity(roots);
    let mut group_of: SlotMarks<u32> = SlotMarks::default();
    for (i, &id) in ids.iter().enumerate() {
        let root = sets.find(i as u32) as usize;
        if number_of_root[root] == u32::MAX {
            number_of_root[root] = groups.len() as u32;
            groups.push(Group {
                simplices: Small::new(),
            });
        }
        let number = number_of_root[root];
        groups[number as usize].simplices.push(id);
        group_of.insert(id, number);
    }

    // Sized once: a simplex has D vertices and D neighbors.
    let d = hull.input.dim();
    let mut vertices = Lists::with_capacity(groups.len(), ids.len() * d);
    let mut neighbors = Lists::with_capacity(groups.len(), ids.len() * d);
    let mut scratch: Vec<u32> = Vec::new();
    for (number, group) in groups.iter().enumerate() {
        let members = || group.simplices.iter().filter_map(|id| hull.facets.get(*id));
        scratch.clear();
        scratch.extend(members().flat_map(|facet| facet.vertices().iter().copied()));
        scratch.sort_unstable();
        scratch.dedup();
        vertices.push(&scratch);
        scratch.clear();
        scratch.extend(
            members()
                .flat_map(|facet| facet.neighbors())
                .filter_map(|n| group_of.get(n))
                .filter(|&g| g as usize != number),
        );
        scratch.sort_unstable();
        scratch.dedup();
        neighbors.push(&scratch);
    }

    LogicalFacets {
        groups,
        vertices,
        neighbors,
    }
}

/// The groups of [`merge_except`] when no ridge joined two simplices: each
/// simplex is a group of its own, numbered by its dense index, which is the
/// numbering by smallest member in arena order. Its vertices and neighbors
/// are written straight into lists sized once, without the per-group
/// gathering, sort, and dedup of the general case (#485): the vertices of a
/// simplex are distinct, and so are its neighbors, which are other groups.
fn singletons(hull: &SimplicialHull<'_>, ids: &[FacetId], dense: &Dense) -> LogicalFacets {
    let d = hull.input.dim();
    let mut groups: Vec<Group> = Vec::with_capacity(ids.len());
    let mut vertices = Lists::with_capacity(ids.len(), ids.len() * d);
    let mut neighbors = Lists::with_capacity(ids.len(), ids.len() * d);
    let mut row: Vec<u32> = Vec::with_capacity(d);
    for &id in ids {
        let mut simplices = Small::new();
        simplices.push(id);
        groups.push(Group { simplices });
        let Some(facet) = hull.facets.get(id) else {
            vertices.push(&[]);
            neighbors.push(&[]);
            continue;
        };
        row.clear();
        row.extend(facet.vertices().iter().copied());
        row.sort_unstable();
        vertices.push(&row);
        row.clear();
        row.extend(facet.neighbors().filter_map(|n| dense.get(ids, n)));
        row.sort_unstable();
        neighbors.push(&row);
    }
    LogicalFacets {
        groups,
        vertices,
        neighbors,
    }
}

/// Whether `a` and `b` have the same groups, member for member, and the same
/// vertex and neighbor lists. A debug check of [`singletons`].
fn same_groups(a: &LogicalFacets, b: &LogicalFacets) -> bool {
    a.groups.len() == b.groups.len()
        && a.groups
            .iter()
            .zip(&b.groups)
            .all(|(x, y)| x.simplices[..] == y.simplices[..])
        && a.vertices.iter().eq(b.vertices.iter())
        && a.neighbors.iter().eq(b.neighbors.iter())
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
            .flat_map(|g| g.simplices.to_vec())
            .collect();
        let total = members.len();
        members.sort_unstable();
        members.dedup();
        assert_eq!(members.len(), total);
        assert_eq!(total, hull.facets.len());
        // Neighbor sets are symmetric.
        for (g, neighbors) in facets.neighbors.iter().enumerate() {
            for &n in neighbors {
                assert!(facets.neighbors.get(n as usize).contains(&(g as u32)));
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
        // D = 2 is the extreme chain: a face is the edge, with no arena simplex.
        for (dim, faces, simplices_per_face) in [(2, 4, Some(0)), (3, 6, Some(2)), (4, 8, None)] {
            let facets = groups_of(dim, &cube(dim));
            assert_eq!(facets.groups.len(), faces, "dim {dim}");
            for (g, group) in facets.groups.iter().enumerate() {
                assert_eq!(facets.vertices.get(g).len(), 1 << (dim - 1), "dim {dim}");
                // A 3-cube splits into 5 or 6 tetrahedra depending on the
                // diagonals, so only the lower dimensions fix the count.
                if let Some(count) = simplices_per_face {
                    assert_eq!(group.simplices.len(), count, "dim {dim}");
                }
                // Every face of a cube meets every other face except the
                // opposite one.
                assert_eq!(facets.neighbors.get(g).len(), faces - 2, "dim {dim}");
            }
        }
    }

    #[test]
    fn segment_has_two_groups() {
        let facets = groups_of(1, &[0.0, 2.0, 1.0]);
        assert_eq!(facets.groups.len(), 2);
        assert!(facets.neighbors.iter().all(<[u32]>::is_empty));
    }

    #[test]
    fn random_simplicial_input_keeps_one_simplex_per_group() {
        let mut rng = Rng(21);
        for dim in 2..=4 {
            let points: Vec<f64> = (0..60 * dim).map(|_| rng.unit()).collect();
            let facets = groups_of(dim, &points);
            if dim == 2 {
                // The extreme chain stores the edge as the group.
                assert!(
                    facets.groups.iter().all(|g| g.simplices.is_empty()),
                    "dim {dim}"
                );
                assert!(facets.vertices.iter().all(|v| v.len() == 2), "dim {dim}");
            } else {
                assert!(
                    facets.groups.iter().all(|g| g.simplices.len() == 1),
                    "dim {dim}"
                );
            }
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
