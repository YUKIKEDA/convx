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

#[derive(Default)]
pub(crate) struct LogicalFacets {
    pub(crate) groups: Lists<FacetId>,
    /// Per group, the union of the member vertices, ascending.
    pub(crate) vertices: Lists<u32>,
    /// Per group, the indices of the neighboring groups, ascending.
    pub(crate) neighbors: Lists<u32>,
}

struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    /// `n` sets of one member each, in the storage of `parent`.
    fn new(n: usize, mut parent: Vec<u32>) -> Self {
        parent.clear();
        parent.extend(0..n as u32);
        Self { parent }
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
    let mut facets = LogicalFacets::default();
    merge_into(hull, &mut facets, &mut MergeWork::default())?;
    Ok(facets)
}

/// The working lists of [`merge_into`], and the groups of an earlier merge
/// for the next to fill: a merge per face allocated each of them per face
/// (#531).
#[derive(Default)]
pub(crate) struct MergeWork {
    ids: Vec<FacetId>,
    by_slot: Vec<u32>,
    parent: Vec<u32>,
    number_of_root: Vec<u32>,
    group_of: SlotMarks<u32>,
    scratch: Vec<u32>,
    /// The group of each simplex by dense index, where each group's
    /// simplices start, and the simplices by group.
    number: Vec<u32>,
    offsets: Vec<u32>,
    members: Vec<FacetId>,
}

/// [`merge`] into `facets`, emptied first, with the working lists of
/// `work`.
pub(crate) fn merge_into(
    hull: &SimplicialHull<'_>,
    facets: &mut LogicalFacets,
    work: &mut MergeWork,
) -> Result<(), ConvexHullError> {
    facets.groups.clear();
    facets.vertices.clear();
    facets.neighbors.clear();
    if hull.strict_edges {
        *facets = polygon_groups(hull);
        return Ok(());
    }
    merge_except(hull, &SlotMarks::default(), facets, work)
}

/// One group per edge, in chain order. Adjacent edges are not collinear.
fn polygon_groups(hull: &SimplicialHull<'_>) -> LogicalFacets {
    let cycle = &hull.polygon;
    let n = cycle.len();
    let mut groups = Lists::with_capacity(n, 0);
    let mut vertices = Lists::with_capacity(n, 2 * n);
    let mut neighbors = Lists::with_capacity(n, 2 * n);
    for i in 0..n {
        let (start, end) = (cycle[i], cycle[(i + 1) % n]);
        vertices.push(&[start.min(end), start.max(end)]);
        let prev = if i == 0 { n - 1 } else { i - 1 } as u32;
        let next = ((i + 1) % n) as u32;
        neighbors.push(&[prev.min(next), prev.max(next)]);
        groups.push(&[]);
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
fn merge_except(
    hull: &SimplicialHull<'_>,
    cut: &SlotMarks<()>,
    facets: &mut LogicalFacets,
    work: &mut MergeWork,
) -> Result<(), ConvexHullError> {
    // The arena's iterator cannot say how many entries are live, so the
    // lists below are sized from its count: grown by doubling, each would be
    // copied a dozen times (#209).
    let mut ids = core::mem::take(&mut work.ids);
    ids.clear();
    ids.reserve(hull.facets.len());
    ids.extend(hull.facets.iter().map(|(id, _)| id));
    let dense = Dense::of(&ids, core::mem::take(&mut work.by_slot));
    let mut sets = UnionFind::new(ids.len(), core::mem::take(&mut work.parent));
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

    if joined {
        grouped(hull, &ids, &mut sets, facets, work);
    } else {
        singletons(hull, &ids, &dense, facets, work);
        #[cfg(debug_assertions)]
        {
            let mut general = LogicalFacets::default();
            grouped(
                hull,
                &ids,
                &mut sets,
                &mut general,
                &mut MergeWork::default(),
            );
            debug_assert!(
                same_groups(facets, &general),
                "the groups of one simplex each are those of the general path"
            );
        }
    }
    work.ids = ids;
    work.by_slot = dense.by_slot;
    work.parent = sets.parent;
    Ok(())
}

/// The dense index of each live simplex, by its slot: the neighbors of a
/// live simplex are live, so a slot names one of them, and an array by slot
/// is a quarter of the reads of [`SlotMarks`] in the ridge loop (#485).
struct Dense {
    by_slot: Vec<u32>,
}

impl Dense {
    /// The dense index of `ids`, in the storage of `by_slot`.
    fn of(ids: &[FacetId], mut by_slot: Vec<u32>) -> Self {
        let slots = ids
            .iter()
            .map(|id| id.index() as usize + 1)
            .max()
            .unwrap_or(0);
        by_slot.clear();
        by_slot.resize(slots, u32::MAX);
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
/// live simplices `ids` in arena order, into `facets`, which is empty.
fn grouped(
    hull: &SimplicialHull<'_>,
    ids: &[FacetId],
    sets: &mut UnionFind,
    facets: &mut LogicalFacets,
    work: &mut MergeWork,
) {
    let LogicalFacets {
        groups,
        vertices,
        neighbors,
    } = facets;
    let MergeWork {
        number_of_root,
        group_of,
        scratch,
        number,
        offsets,
        members,
        ..
    } = work;
    // Number groups by their smallest member, in arena order.
    // A root is a dense index; a group is numbered when its root is first
    // met, and `u32::MAX` marks a root not met yet.
    number_of_root.clear();
    number_of_root.resize(ids.len(), u32::MAX);
    group_of.clear();
    number.clear();
    let mut count = 0_usize;
    for (i, &id) in ids.iter().enumerate() {
        let root = sets.find(i as u32) as usize;
        if number_of_root[root] == u32::MAX {
            number_of_root[root] = count as u32;
            count += 1;
        }
        number.push(number_of_root[root]);
        group_of.insert(id, number_of_root[root]);
    }
    // A group's simplices in arena order, by a counting sort over the
    // groups. A list on the heap per group of three simplices or more
    // allocated per facet (#531).
    offsets.clear();
    offsets.resize(count + 2, 0);
    for &g in number.iter() {
        offsets[g as usize + 2] += 1;
    }
    for g in 2..offsets.len() {
        offsets[g] += offsets[g - 1];
    }
    members.clear();
    members.resize(ids.len(), FacetId::default());
    for (&g, &id) in number.iter().zip(ids) {
        let slot = &mut offsets[g as usize + 1];
        members[*slot as usize] = id;
        *slot += 1;
    }
    groups.reserve(count, ids.len());
    for g in 0..count {
        groups.push(&members[offsets[g] as usize..offsets[g + 1] as usize]);
    }

    // Sized once: a simplex has D vertices and D neighbors.
    let d = hull.input.dim();
    vertices.reserve(groups.len(), ids.len() * d);
    neighbors.reserve(groups.len(), ids.len() * d);
    for (number, group) in groups.iter().enumerate() {
        let members = || group.iter().filter_map(|id| hull.facets.get(*id));
        scratch.clear();
        scratch.extend(members().flat_map(|facet| facet.vertices().iter().copied()));
        scratch.sort_unstable();
        scratch.dedup();
        vertices.push(scratch);
        scratch.clear();
        scratch.extend(
            members()
                .flat_map(|facet| facet.neighbors())
                .filter_map(|n| group_of.get(n))
                .filter(|&g| g as usize != number),
        );
        scratch.sort_unstable();
        scratch.dedup();
        neighbors.push(scratch);
    }
}

/// The groups of [`merge_except`] when no ridge joined two simplices: each
/// simplex is a group of its own, numbered by its dense index, which is the
/// numbering by smallest member in arena order. Its vertices and neighbors
/// are written straight into lists sized once, without the per-group
/// gathering, sort, and dedup of the general case (#485): the vertices of a
/// simplex are distinct, and so are its neighbors, which are other groups.
fn singletons(
    hull: &SimplicialHull<'_>,
    ids: &[FacetId],
    dense: &Dense,
    facets: &mut LogicalFacets,
    work: &mut MergeWork,
) {
    let LogicalFacets {
        groups,
        vertices,
        neighbors,
    } = facets;
    let d = hull.input.dim();
    groups.reserve(ids.len(), ids.len());
    vertices.reserve(ids.len(), ids.len() * d);
    neighbors.reserve(ids.len(), ids.len() * d);
    let row = &mut work.scratch;
    for &id in ids {
        groups.push(&[id]);
        let Some(facet) = hull.facets.get(id) else {
            vertices.push(&[]);
            neighbors.push(&[]);
            continue;
        };
        row.clear();
        row.extend(facet.vertices().iter().copied());
        row.sort_unstable();
        vertices.push(row);
        row.clear();
        row.extend(facet.neighbors().filter_map(|n| dense.get(ids, n)));
        row.sort_unstable();
        neighbors.push(row);
    }
}

/// Whether `a` and `b` have the same groups, member for member, and the same
/// vertex and neighbor lists. A debug check of [`singletons`].
fn same_groups(a: &LogicalFacets, b: &LogicalFacets) -> bool {
    a.groups.iter().eq(b.groups.iter())
        && a.vertices.iter().eq(b.vertices.iter())
        && a.neighbors.iter().eq(b.neighbors.iter())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hull::input::accept;
    use crate::hull::simplicial::tests::drawn;
    use crate::hull::simplicial::tests::Rng;

    /// A merge into the lists of an earlier one, with its working lists
    /// (#531), gives the groups of a merge from nothing, on hulls of D = 2
    /// to 5 whose facets are simplices, pairs, and larger groups.
    #[test]
    fn a_merge_into_the_lists_of_another_is_the_merge_from_nothing() {
        let mut rng = Rng(531);
        let (mut facets, mut work) = (LogicalFacets::default(), MergeWork::default());
        let (mut merged, mut large) = (0, 0);
        for round in 0..3 {
            for (dim, count, grid) in [
                (4, 400, true),
                (3, 60, false),
                (5, 300, true),
                (2, 500, false),
                (3, 900, true),
                (4, 90, false),
                (3, 12, true),
            ] {
                let points = drawn(&mut rng, dim, count, grid);
                let Ok(input) = accept(dim, &points) else {
                    continue;
                };
                let hull = SimplicialHull::build(input).unwrap();
                let fresh = merge(&hull).unwrap();
                merge_into(&hull, &mut facets, &mut work).unwrap();
                assert!(
                    same_groups(&facets, &fresh),
                    "D = {dim}, {count} points, round {round}"
                );
                merged += 1;
                large += fresh.groups.iter().filter(|group| group.len() > 2).count();
            }
        }
        assert!(
            merged >= 19 && large > 20,
            "{merged} merges, {large} large groups"
        );
    }

    fn groups_of(dim: usize, points: &[f64]) -> LogicalFacets {
        let hull = SimplicialHull::build(accept(dim, points).unwrap()).unwrap();
        let facets = merge(&hull).unwrap();
        // Every simplex belongs to exactly one group.
        let mut members: Vec<FacetId> = facets.groups.iter().flat_map(|g| g.to_vec()).collect();
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
                    assert_eq!(group.len(), count, "dim {dim}");
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
                assert!(facets.groups.iter().all(|g| g.is_empty()), "dim {dim}");
                assert!(facets.vertices.iter().all(|v| v.len() == 2), "dim {dim}");
            } else {
                assert!(facets.groups.iter().all(|g| g.len() == 1), "dim {dim}");
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
