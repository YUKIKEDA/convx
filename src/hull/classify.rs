//! Distance-zero classification and the index partition (design §3, §5).
//!
//! After insertion and the coplanar merge, every representative that is not
//! a vertex of the simplicial complex is classified by the exact sign of its
//! distance to each logical facet: all negative is interior, and a zero on
//! some facet puts it on that facet's boundary set.
//!
//! The facet's points (its simplicial vertices and the zero-distance points)
//! all lie on the facet's supporting hyperplane. Their extreme points are the
//! facet's vertices. They are found exactly: the hyperplane is projected onto
//! D - 1 coordinates by dropping an axis along which it is not vertical (an
//! affine bijection of the hyperplane, so extremeness is preserved), and the
//! hull of the projected points is built recursively down to D = 1.
//!
//! This decides, with the same exact signs, the cases §3 lists: a
//! zero-distance point outside `conv(V)` becomes a vertex, one inside stays a
//! non-vertex boundary point. It also decides a case §3 does not name: a
//! simplicial vertex that was extreme when it was inserted and later fell on
//! the relative interior of a face or an edge is not extreme, so it moves to
//! `coplanar_points`. `vertices` is then exactly the set of extreme points.
//!
//! Every facet other than a single simplex with extreme vertices is then
//! triangulated by placing its extreme points in index order (see [`place`]).
//! §3 asks for a re-triangulation only of a facet that gained a vertex, from
//! the lexicographically minimum basis; but two facets that share a lower
//! face must split it the same way, or the boundary does not close. A facet
//! kept as built and a re-triangulated neighbor can disagree there, and so
//! can two neighbors triangulated basis-first. The placing triangulation
//! restricts to the placing triangulation of every face in the same order, so
//! all facets agree. Simplex neighbors are then linked across the updated
//! simplices; face neighbors are those of the merged groups.

use std::collections::HashMap;

use super::input::{accept, Input};
use super::merge::merge;
use super::ridge::{fingerprint, pair_equal_keys};
use super::simplicial::{Execution, SimplicialHull};
use super::ConvexHullError;
use crate::arena::{FacetId, SlotMarks};
use crate::predicates::{orient, orient_direction, Sign};
use crate::small::Small;

/// A simplex of the boundary complex.
#[derive(Clone)]
pub(crate) struct ComplexSimplex {
    /// D vertices.
    pub(crate) vertices: Small<u32, 8>,
    /// The face that contains this simplex.
    pub(crate) face: u32,
    /// `neighbors[i]` is the simplex across the ridge opposite
    /// `vertices[i]`. Empty for D = 1.
    pub(crate) neighbors: Small<u32, 8>,
}

/// A logical facet with its extreme points.
#[derive(Clone)]
pub(crate) struct Face {
    /// Extreme points, ascending.
    pub(crate) vertices: Vec<u32>,
    /// Neighboring faces, ascending.
    pub(crate) neighbors: Vec<u32>,
}

/// The hull after classification. Faces are in construction order; the
/// public numbering is fixed when the hull is published.
pub(crate) struct Classified<'a> {
    pub(crate) input: Input<'a>,
    pub(crate) faces: Vec<Face>,
    pub(crate) simplices: Vec<ComplexSimplex>,
    pub(crate) vertices: Vec<u32>,
    pub(crate) coplanar_points: Vec<u32>,
    pub(crate) interior_points: Vec<u32>,
}

/// The chain edge `point` lies on, or `None` when it is strictly inside.
///
/// `cycle` is the extreme vertices counterclockwise. The fan from
/// `cycle[0]` names the triangle, and one orientation against that
/// triangle's outer edge separates an interior point from a boundary point.
fn polygon_edge(
    input: &Input<'_>,
    cycle: &[u32],
    point: u32,
) -> Result<Option<usize>, ConvexHullError> {
    let n = cycle.len();
    debug_assert!(n >= 3, "a polygon has at least three vertices");
    let turn = |a: usize, b: usize| input.orient(&[cycle[a], cycle[b], point]);
    if turn(0, 1)? == Sign::Zero {
        return Ok(Some(0));
    }
    debug_assert_ne!(
        turn(0, 1)?,
        Sign::Negative,
        "a point is outside the polygon"
    );
    if turn(0, n - 1)? == Sign::Zero {
        return Ok(Some(n - 1));
    }
    let mut lo = 1usize;
    let mut hi = n - 1;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if turn(0, mid)? == Sign::Positive {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let side = input.orient(&[cycle[lo], cycle[(lo + 1) % n], point])?;
    debug_assert_ne!(side, Sign::Negative, "a point is outside the polygon");
    Ok(if side == Sign::Zero { Some(lo) } else { None })
}

/// Builds and classifies the hull of an accepted input.
pub(crate) fn classify(
    input: Input<'_>,
    execution: Execution,
) -> Result<Classified<'_>, ConvexHullError> {
    let built = SimplicialHull::build(input, execution)?;
    classify_built(built, execution)
}

/// Classifies a strict polygon from its extreme cycle.
///
/// The cycle vertices are the extreme points. A representative that missed
/// the chain is on one edge or strictly inside; it is not a new vertex.
fn classify_chain(hull: SimplicialHull<'_>) -> Result<Classified<'_>, ConvexHullError> {
    let n = hull.polygon.len();
    // The lists below are filtered from the representatives, in their
    // order, and published ascending without a sort.
    debug_assert!(
        hull.input.representatives.is_sorted(),
        "the representatives are ascending"
    );
    let mut on_cycle = vec![false; hull.input.representative.len()];
    for &v in &hull.polygon {
        on_cycle[v as usize] = true;
    }
    let others: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| !on_cycle[p as usize])
        .collect();
    let mut on_boundary = vec![false; others.len()];
    for (k, &point) in others.iter().enumerate() {
        if polygon_edge(&hull.input, &hull.polygon, point)?.is_some() {
            on_boundary[k] = true;
        }
    }

    let mut vertices = hull.polygon.clone();
    vertices.sort_unstable();
    // `others` is ascending, so both lists are too.
    let coplanar_points: Vec<u32> = others
        .iter()
        .zip(&on_boundary)
        .filter(|&(_, &b)| b)
        .map(|(&p, _)| p)
        .collect();
    let interior_points: Vec<u32> = others
        .iter()
        .zip(&on_boundary)
        .filter(|&(_, &b)| !b)
        .map(|(&p, _)| p)
        .collect();

    let groups = merge(&hull)?.groups;
    let mut faces = Vec::with_capacity(n);
    let mut simplices = Vec::with_capacity(n);
    for (i, group) in groups.into_iter().enumerate() {
        let start = hull.polygon[i];
        let end = hull.polygon[(i + 1) % n];
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;
        faces.push(Face {
            vertices: group.vertices,
            neighbors: group.neighbors,
        });
        // `neighbors[i]` is the simplex across the ridge opposite `vertices[i]`.
        // The tip is first, so slot 0 faces the previous edge and slot 1 the next.
        simplices.push(ComplexSimplex {
            vertices: [end, start].as_slice().into(),
            face: i as u32,
            neighbors: [prev as u32, next as u32].as_slice().into(),
        });
    }
    Ok(Classified {
        input: hull.input,
        faces,
        simplices,
        vertices,
        coplanar_points,
        interior_points,
    })
}

/// Classifies a built simplicial hull. Its `proved_interior` points go to
/// `interior_points` without a scan.
fn classify_built(
    hull: SimplicialHull<'_>,
    execution: Execution,
) -> Result<Classified<'_>, ConvexHullError> {
    if hull.strict_edges {
        return classify_chain(hull);
    }
    let mut groups = merge(&hull)?;
    let d = hull.input.dim();
    // A group's member simplices, in outward order.
    let members = |g: usize| {
        groups.groups[g]
            .simplices
            .iter()
            .filter_map(|id| hull.facets.get(*id))
            .map(|s| &s.vertices[..])
    };

    // Simplicial vertices.
    let mut on_complex = vec![false; hull.input.representative.len()];
    for g in 0..groups.groups.len() {
        for vertices in members(g) {
            for &v in vertices {
                on_complex[v as usize] = true;
            }
        }
    }

    // Distance signs of the other representatives against every group.
    // Points construction proved strictly inside are interior already
    // (design §3) and skip the scan.
    let mut skipped = vec![false; hull.input.representative.len()];
    for &p in &hull.proved_interior {
        debug_assert!(!on_complex[p as usize], "a dropped point is not a vertex");
        // A point leaves every outside set when it is proved interior, and
        // a lost vertex is in none, so no later plan meets it again.
        debug_assert!(!skipped[p as usize], "point {p} was proved interior twice");
        skipped[p as usize] = true;
    }
    let others: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| !on_complex[p as usize] && !skipped[p as usize])
        .collect();
    let mut on_boundary = vec![false; others.len()];
    let mut zero_points: Vec<Vec<u32>> = vec![Vec::new(); groups.groups.len()];
    if !others.is_empty() {
        let mut sides = vec![None; others.len()];
        for (g, group) in groups.groups.iter().enumerate() {
            let Some(simplex) = group.simplices.first().and_then(|id| hull.facets.get(*id)) else {
                continue;
            };
            sides.fill(None);
            if let Some(cull) = simplex.cull() {
                let (rows, stride) = hull.input.rows();
                let origin = hull.input.point(simplex.vertices[0]);
                cull.mark_sides(origin, rows, stride, &others, &mut sides);
            }
            for (k, &p) in others.iter().enumerate() {
                let side = match sides[k] {
                    Some(proved) => {
                        // `side` checks its own proof against the
                        // orientation in debug builds, so this checks the
                        // scan's.
                        #[cfg(debug_assertions)]
                        debug_assert_eq!(
                            hull.side(simplex, p)?,
                            proved,
                            "the scan proved the wrong side of point {p}"
                        );
                        proved
                    }
                    None => hull.side(simplex, p)?,
                };
                debug_assert!(
                    side != Sign::Positive,
                    "a point is outside the finished hull"
                );
                if side == Sign::Zero {
                    on_boundary[k] = true;
                    zero_points[g].push(p);
                }
            }
        }
    }

    // Extreme points of each group; `None` when they are the group's
    // vertices, a single simplex with no other point on its plane.
    let mut extremes: Vec<Option<Vec<u32>>> = Vec::with_capacity(groups.groups.len());
    for (g, group) in groups.groups.iter().enumerate() {
        if group.simplices.len() == 1 && zero_points[g].is_empty() {
            extremes.push(None);
            continue;
        }
        let mut candidates = group.vertices.clone();
        candidates.extend_from_slice(&zero_points[g]);
        candidates.sort_unstable();
        candidates.dedup();
        let plane = members(g).next().unwrap_or(&[]);
        extremes.push(Some(face_extremes(
            &hull.input,
            plane,
            &candidates,
            execution,
        )?));
    }

    let mut is_vertex = vec![false; hull.input.representative.len()];
    for (g, e) in extremes.iter().enumerate() {
        for &v in e.as_deref().unwrap_or(&groups.groups[g].vertices) {
            is_vertex[v as usize] = true;
        }
    }
    let vertices: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| is_vertex[p as usize])
        .collect();
    // Every representative is a vertex, on the complex, on the boundary
    // by the scan, or interior: either proved during construction or
    // scanned strictly inside. One pass over the ascending representatives
    // lists the points of each class in order, without sorting the
    // interior points, which are almost all of a large input (#213).
    debug_assert!(
        hull.input.representatives.is_sorted(),
        "the representatives are ascending"
    );
    let mut boundary = vec![false; hull.input.representative.len()];
    for (&p, &b) in others.iter().zip(&on_boundary) {
        boundary[p as usize] = b;
    }
    let mut coplanar_points: Vec<u32> = Vec::new();
    let mut interior_points: Vec<u32> = Vec::new();
    for &p in &hull.input.representatives {
        let i = p as usize;
        if on_complex[i] || boundary[i] {
            if !is_vertex[i] {
                coplanar_points.push(p);
            }
        } else {
            debug_assert!(!is_vertex[i], "vertex {p} is on no facet");
            interior_points.push(p);
        }
    }
    debug_assert_eq!(
        interior_points.len(),
        hull.proved_interior.len() + on_boundary.iter().filter(|&&b| !b).count(),
        "an interior point is proved during construction or scanned inside"
    );

    // Boundary simplices: kept as built, or re-triangulated by placing.
    // At least one simplex per face, and exactly one in general position.
    let mut simplices: Vec<ComplexSimplex> = Vec::with_capacity(extremes.len());
    let mut faces: Vec<Face> = Vec::with_capacity(extremes.len());
    // Simplices kept as built, by their index here and their construction id.
    let mut kept: Vec<(u32, FacetId)> = Vec::with_capacity(extremes.len());
    let unlinked = if d == 1 { 0 } else { d };
    for (g, extreme) in extremes.into_iter().enumerate() {
        let group = &mut groups.groups[g];
        let first = simplices.len() as u32;
        let mut push = |vertices: Small<u32, 8>| {
            simplices.push(ComplexSimplex {
                vertices,
                face: g as u32,
                neighbors: core::iter::repeat_n(UNLINKED, unlinked).collect(),
            });
        };
        // A single simplex whose vertices are all extreme is kept; every
        // other facet is re-triangulated by placing, so facets that share a
        // lower face split it the same way.
        let single = group.simplices.len() == 1;
        let extreme = match extreme {
            Some(extreme) if !(single && extreme == group.vertices) => {
                let q = vertices
                    .iter()
                    .copied()
                    .find(|v| extreme.binary_search(v).is_err())
                    .unwrap_or(extreme[0]);
                for members in place(&hull.input, &extreme, q)? {
                    push(members.into());
                }
                extreme
            }
            _ => {
                if let Some(&id) = group.simplices.first() {
                    if let Some(s) = hull.facets.get(id) {
                        kept.push((first, id));
                        push(s.vertices.clone());
                    }
                }
                core::mem::take(&mut group.vertices)
            }
        };
        faces.push(Face {
            vertices: extreme,
            neighbors: core::mem::take(&mut group.neighbors),
        });
    }
    // A kept simplex has its construction vertex order, so its neighbor
    // across slot i during construction, when kept too, is its neighbor
    // across slot i here: each ridge of the closed complex has two sides.
    let mut index_of: SlotMarks<u32> = SlotMarks::default();
    for &(s, id) in &kept {
        index_of.insert(id, s);
    }
    for &(s, id) in &kept {
        if let Some(facet) = hull.facets.get(id) {
            for (slot, &neighbor) in facet.neighbors.iter().enumerate() {
                if let Some(other) = index_of.get(neighbor) {
                    simplices[s as usize].neighbors[slot] = other;
                }
            }
        }
    }
    #[cfg(debug_assertions)]
    let mut paired_all = (simplices.clone(), faces.clone());
    link_neighbors(d, &mut simplices, &faces);
    #[cfg(debug_assertions)]
    {
        for simplex in &mut paired_all.0 {
            simplex.neighbors.fill(UNLINKED);
        }
        link_neighbors(d, &mut paired_all.0, &paired_all.1);
        debug_assert!(
            simplices
                .iter()
                .zip(&paired_all.0)
                .all(|(a, b)| a.neighbors == b.neighbors),
            "kept construction links differ from pairing every ridge"
        );
    }

    Ok(Classified {
        input: hull.input,
        faces,
        simplices,
        vertices,
        coplanar_points,
        interior_points,
    })
}

/// Extreme points of `candidates`, which lie on the hyperplane through the
/// D affinely independent points `plane`.
fn face_extremes(
    input: &Input<'_>,
    plane: &[u32],
    candidates: &[u32],
    execution: Execution,
) -> Result<Vec<u32>, ConvexHullError> {
    let d = input.dim();
    if d == 1 {
        return Ok(candidates.to_vec());
    }
    // An axis along which the hyperplane is not vertical: its cofactor is
    // nonzero, so dropping that coordinate is a bijection of the hyperplane.
    let plane_points: Vec<&[f64]> = plane.iter().map(|&v| input.point(v)).collect();
    let mut unit = vec![0.0; d];
    let mut axis = None;
    for j in 0..d {
        unit[j] = 1.0;
        let sign = orient_direction(&plane_points, &unit)?;
        unit[j] = 0.0;
        if sign != Sign::Zero {
            axis = Some(j);
            break;
        }
    }
    let Some(axis) = axis else {
        debug_assert!(false, "a hyperplane has a nonzero cofactor");
        return Ok(candidates.to_vec());
    };
    let projected: Vec<f64> = candidates
        .iter()
        .flat_map(|&p| {
            input
                .point(p)
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != axis)
                .map(|(_, &x)| x)
        })
        .collect();
    let sub = classify(accept(d - 1, &projected)?, execution)?;
    Ok(sub
        .vertices
        .iter()
        .map(|&i| candidates[i as usize])
        .collect())
}

/// Placing triangulation of the extreme points `extreme` (ascending) of one
/// facet, oriented so that `q`, a hull vertex off the facet, is inside.
///
/// Points are placed one at a time in index order. A point that raises the
/// affine dimension of the points placed so far is joined to every current
/// simplex. Otherwise it lies outside their hull, since it is extreme, and it
/// is joined to every boundary ridge it is beyond: the ridge's other vertex
/// and the point are strictly on opposite sides of the ridge, judged by exact
/// orientation within the current affine span.
///
/// The placing triangulation restricts to the placing triangulation of each
/// face in the same order, so two facets that share a lower face split it
/// the same way, and the boundary complex closes up.
fn place(input: &Input<'_>, extreme: &[u32], q: u32) -> Result<Vec<Vec<u32>>, ConvexHullError> {
    let d = input.dim();
    let simplices = placing(d, |i| input.point(i), extreme)?;
    debug_assert!(
        simplices.iter().all(|s| s.len() == d),
        "a facet spans D - 1 dimensions"
    );
    simplices
        .into_iter()
        .map(|s| oriented(input, s, q))
        .collect()
}

/// The placing triangulation of `points` (ascending), coordinates of
/// dimension `d` given by `point`: simplices of `r + 1` vertices for the
/// affine dimension `r` of the points, unoriented. See [`place`].
pub(crate) fn placing<'p>(
    d: usize,
    point: impl Fn(u32) -> &'p [f64],
    extreme: &[u32],
) -> Result<Vec<Vec<u32>>, ConvexHullError> {
    let Some((&first, rest)) = extreme.split_first() else {
        return Ok(Vec::new());
    };
    let mut simplices: Vec<Vec<u32>> = vec![vec![first]];
    // Coordinates on which the points placed so far project to an affinely
    // independent basis; their count is the current affine dimension.
    let mut axes: Vec<usize> = Vec::with_capacity(d);
    let mut basis: Vec<u32> = vec![first];
    let project = |v: u32, axes: &[usize], extra: Option<usize>| -> Vec<f64> {
        let x = point(v);
        axes.iter()
            .map(|&a| x[a])
            .chain(extra.map(|j| x[j]))
            .collect()
    };
    for &p in rest {
        // Does p raise the affine dimension?
        let mut raised = None;
        for j in (0..d).filter(|j| !axes.contains(j)) {
            let projected: Vec<Vec<f64>> = basis
                .iter()
                .chain(core::iter::once(&p))
                .map(|&v| project(v, &axes, Some(j)))
                .collect();
            let refs: Vec<&[f64]> = projected.iter().map(Vec::as_slice).collect();
            if orient(&refs)? != Sign::Zero {
                raised = Some(j);
                break;
            }
        }
        if let Some(j) = raised {
            axes.push(j);
            basis.push(p);
            for simplex in &mut simplices {
                simplex.push(p);
            }
            continue;
        }
        // Beyond which boundary ridges of the current complex is p?
        let mut sides: HashMap<Vec<u32>, (usize, usize, usize)> = HashMap::new();
        for (s, simplex) in simplices.iter().enumerate() {
            for slot in 0..simplex.len() {
                let mut ridge: Vec<u32> = simplex
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v)
                    .collect();
                ridge.sort_unstable();
                let entry = sides.entry(ridge).or_insert((0, s, slot));
                entry.0 += 1;
            }
        }
        let mut added = Vec::new();
        for (ridge, (count, s, slot)) in sides {
            if count != 1 {
                continue;
            }
            let a = simplices[s][slot];
            let side_of = |v: u32| -> Result<Sign, ConvexHullError> {
                let projected: Vec<Vec<f64>> = ridge
                    .iter()
                    .chain(core::iter::once(&v))
                    .map(|&u| project(u, &axes, None))
                    .collect();
                let refs: Vec<&[f64]> = projected.iter().map(Vec::as_slice).collect();
                Ok(orient(&refs)?)
            };
            let (sp, sa) = (side_of(p)?, side_of(a)?);
            if sp != Sign::Zero && sa != Sign::Zero && sp != sa {
                let mut vertices = ridge;
                vertices.push(p);
                added.push(vertices);
            }
        }
        added.sort_unstable();
        simplices.extend(added);
    }
    Ok(simplices)
}

/// Orders `vertices` so that `q` is on the negative side.
fn oriented(
    input: &Input<'_>,
    mut vertices: Vec<u32>,
    q: u32,
) -> Result<Vec<u32>, ConvexHullError> {
    let mut points: Vec<&[f64]> = vertices.iter().map(|&v| input.point(v)).collect();
    points.push(input.point(q));
    if orient(&points)? == Sign::Positive && vertices.len() >= 2 {
        vertices.swap(0, 1);
    }
    Ok(vertices)
}

/// A neighbor slot of a [`ComplexSimplex`] not linked yet.
const UNLINKED: u32 = u32::MAX;

/// Links every [`UNLINKED`] simplex neighbor through its shared ridge.
///
/// The face neighbors are those of the merged groups. Two simplices of
/// different facets meet in a ridge of dimension D - 2 on both facets'
/// planes, so the facets share a (D - 2)-face; and a (D - 2)-face of a
/// polytope lies on exactly two facets, so the simplex across any ridge on
/// it belongs to the other. Groups and faces are therefore adjacent alike,
/// however the faces are triangulated; debug builds check it.
fn link_neighbors(d: usize, simplices: &mut [ComplexSimplex], faces: &[Face]) {
    if d == 1 {
        // A segment has no ridge. Each simplex's neighbor list is empty
        // (`unlinked` is 0), so there is no triangulation to compare with
        // the group neighbors. `segment_has_two_groups` checks that those
        // neighbors are empty.
        return;
    }
    // Unlinked ridges as sorted vertex lists packed in one buffer, with
    // their owner. A linked ridge has both of its sides linked.
    let mut keys: Vec<u32> = Vec::new();
    let mut owners: Vec<(u32, usize)> = Vec::new();
    for (s, simplex) in simplices.iter().enumerate() {
        for slot in 0..d {
            if simplex.neighbors[slot] != UNLINKED {
                continue;
            }
            let start = keys.len();
            keys.extend(
                simplex
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v),
            );
            keys[start..].sort_unstable();
            owners.push((s as u32, slot));
        }
    }
    // Every ridge of a closed boundary has two sides.
    for (first, second) in pair_equal_keys(&keys, owners.len(), fingerprint) {
        let (a, slot_a) = owners[first];
        let (b, slot_b) = owners[second];
        simplices[a as usize].neighbors[slot_a] = b;
        simplices[b as usize].neighbors[slot_b] = a;
    }
    #[cfg(not(debug_assertions))]
    let _ = faces;
    #[cfg(debug_assertions)]
    {
        let mut across: Vec<Vec<u32>> = vec![Vec::new(); faces.len()];
        for simplex in simplices.iter() {
            across[simplex.face as usize].extend(
                simplex
                    .neighbors
                    .iter()
                    .filter_map(|&n| simplices.get(n as usize).map(|t| t.face))
                    .filter(|&n| n != simplex.face),
            );
        }
        for (f, (face, mut neighbors)) in faces.iter().zip(across).enumerate() {
            neighbors.sort_unstable();
            neighbors.dedup();
            debug_assert_eq!(
                face.neighbors, neighbors,
                "face {f}: the group neighbors differ from the triangulation's"
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hull::simplicial::tests::Rng;

    pub(crate) fn classified(dim: usize, points: &[f64]) -> Classified<'_> {
        let c = classify(accept(dim, points).unwrap(), Execution::Sequential).unwrap();
        check(&c);
        c
    }

    /// Partition, extreme vertex sets, and symmetric neighbors.
    pub(crate) fn check(c: &Classified<'_>) {
        let mut all: Vec<u32> = c
            .vertices
            .iter()
            .chain(&c.coplanar_points)
            .chain(&c.interior_points)
            .copied()
            .collect();
        all.sort_unstable();
        assert_eq!(
            all, c.input.representatives,
            "the three lists partition the representatives"
        );
        let mut face_vertices: Vec<u32> = c.faces.iter().flat_map(|f| f.vertices.clone()).collect();
        face_vertices.sort_unstable();
        face_vertices.dedup();
        assert_eq!(face_vertices, c.vertices);
        for (s, simplex) in c.simplices.iter().enumerate() {
            for (slot, &n) in simplex.neighbors.iter().enumerate() {
                let other = &c.simplices[n as usize];
                let back = other.neighbors.iter().position(|&b| b == s as u32).unwrap();
                let mut a: Vec<u32> = simplex
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
                assert_eq!(a, b);
            }
            // The simplex spans a supporting hyperplane: no two
            // representatives lie strictly on opposite sides.
            let signs: Vec<Sign> = c
                .input
                .representatives
                .iter()
                .map(|&p| {
                    let mut points: Vec<&[f64]> =
                        simplex.vertices.iter().map(|&v| c.input.point(v)).collect();
                    points.push(c.input.point(p));
                    orient(&points).unwrap()
                })
                .collect();
            assert!(!(signs.contains(&Sign::Positive) && signs.contains(&Sign::Negative)));
            for v in &simplex.vertices {
                assert!(c.faces[simplex.face as usize].vertices.contains(v));
            }
        }
        for (f, face) in c.faces.iter().enumerate() {
            for &n in &face.neighbors {
                assert!(c.faces[n as usize].neighbors.contains(&(f as u32)));
            }
        }
    }

    #[test]
    fn square_with_an_edge_midpoint_and_a_center() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.0, 0.5, 0.5];
        let c = classified(2, &points);
        assert_eq!(c.vertices, vec![0, 1, 2, 3]);
        assert_eq!(c.coplanar_points, vec![4]);
        assert_eq!(c.interior_points, vec![5]);
        assert_eq!(c.faces.len(), 4);
    }

    #[test]
    fn cube_with_face_center_and_edge_midpoint() {
        let mut points = Vec::new();
        for i in 0..8 {
            points.extend([(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64]);
        }
        points.extend([0.5, 0.5, 0.0, 0.5, 0.0, 0.0, 0.5, 0.5, 0.5]);
        let c = classified(3, &points);
        assert_eq!(c.vertices, (0..8).collect::<Vec<u32>>());
        assert_eq!(c.coplanar_points, vec![8, 9]);
        assert_eq!(c.interior_points, vec![10]);
        assert_eq!(c.faces.len(), 6);
        assert!(c
            .faces
            .iter()
            .all(|f| f.vertices.len() == 4 && f.neighbors.len() == 4));
    }

    #[test]
    fn duplicate_of_a_corner_stays_a_duplicate() {
        let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        let c = classified(2, &points);
        assert_eq!(c.input.representative, vec![0, 1, 2, 1]);
        assert_eq!(c.vertices, vec![0, 1, 2]);
        assert!(c.coplanar_points.is_empty());
    }

    #[test]
    fn segment_endpoints() {
        let c = classified(1, &[2.0, -1.0, 0.5, 3.0]);
        assert_eq!(c.vertices, vec![1, 3]);
        assert_eq!(c.interior_points, vec![0, 2]);
    }

    #[test]
    fn collinear_vertex_inserted_early_is_demoted() {
        // (1, 0) is extreme for the first triangle, then (2, 0) puts it on
        // the bottom edge.
        let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 2.0, 0.0];
        let c = classified(2, &points);
        assert_eq!(c.vertices, vec![0, 2, 3]);
        assert_eq!(c.coplanar_points, vec![1]);
        assert_eq!(c.faces.len(), 3);
    }

    #[test]
    fn integer_grids_keep_only_corners() {
        for dim in 2..=4 {
            let side = 3_usize;
            let count = side.pow(dim as u32);
            let points: Vec<f64> = (0..count)
                .flat_map(|i| (0..dim).map(move |a| ((i / side.pow(a as u32)) % side) as f64))
                .collect();
            let c = classified(dim, &points);
            assert_eq!(c.vertices.len(), 1 << dim, "dim {dim}");
            assert_eq!(c.faces.len(), 2 * dim, "dim {dim}");
            assert_eq!(
                c.interior_points.len(),
                (side - 2).pow(dim as u32),
                "dim {dim}"
            );
        }
    }

    #[test]
    fn random_inputs_partition() {
        let mut rng = Rng(31);
        for dim in 2..=4 {
            let points: Vec<f64> = (0..40 * dim).map(|_| rng.unit()).collect();
            let c = classified(dim, &points);
            assert!(c.coplanar_points.is_empty());
        }
    }

    #[test]
    fn rounded_grid_on_sphere_shell() {
        // Points of an integer grid inside a ball, many cocircular boundary
        // points, but exact coordinates.
        let mut points = Vec::new();
        for i in -3_i32..=3 {
            for j in -3_i32..=3 {
                for k in -3_i32..=3 {
                    if i * i + j * j + k * k <= 9 {
                        points.extend([f64::from(i), f64::from(j), f64::from(k)]);
                    }
                }
            }
        }
        classified(3, &points);
    }

    /// Andrew's chain, from the exact orientation, as a reference that does
    /// not call the hull.
    fn monotone_cycle(points: &[f64]) -> Vec<u32> {
        let n = points.len() / 2;
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by(|&a, &b| {
            let (pa, pb) = (
                &points[a as usize * 2..a as usize * 2 + 2],
                &points[b as usize * 2..b as usize * 2 + 2],
            );
            pa[0]
                .total_cmp(&pb[0])
                .then(pa[1].total_cmp(&pb[1]))
                .then(a.cmp(&b))
        });
        let p = |i: u32| &points[i as usize * 2..i as usize * 2 + 2];
        let mut chain = Vec::new();
        for pass in [order.clone(), order.iter().rev().copied().collect()] {
            let start = chain.len();
            for &i in &pass {
                while chain.len() >= start + 2 {
                    let sign =
                        orient(&[p(chain[chain.len() - 2]), p(chain[chain.len() - 1]), p(i)])
                            .unwrap();
                    if sign == Sign::Positive {
                        break;
                    }
                    chain.pop();
                }
                chain.push(i);
            }
            chain.pop();
        }
        chain
    }

    /// 1024 points is the first size that may choose insertion. A filled
    /// square does, and edge points stay coplanar.
    #[test]
    fn large_square_inserts_and_keeps_edge_points_coplanar() {
        let side = 32usize;
        let mut points = Vec::with_capacity(side * side * 2);
        for y in 0..side {
            for x in 0..side {
                points.push(x as f64);
                points.push(y as f64);
            }
        }
        let at = |x: usize, y: usize| (y * side + x) as u32;
        let mut vertices = vec![
            at(0, 0),
            at(side - 1, 0),
            at(0, side - 1),
            at(side - 1, side - 1),
        ];
        vertices.sort_unstable();
        let mut coplanar = Vec::new();
        let mut interior = Vec::new();
        for y in 0..side {
            for x in 0..side {
                let edge = x == 0 || y == 0 || x + 1 == side || y + 1 == side;
                let corner = (x == 0 || x + 1 == side) && (y == 0 || y + 1 == side);
                if corner {
                    continue;
                } else if edge {
                    coplanar.push(at(x, y));
                } else {
                    interior.push(at(x, y));
                }
            }
        }
        let cycle = monotone_cycle(&points);
        let mut by_chain = cycle.clone();
        by_chain.sort_unstable();
        assert_eq!(by_chain, vertices, "the square's extremes are its corners");
        for execution in [Execution::Sequential, Execution::Parallel] {
            let hull = SimplicialHull::build(accept(2, &points).unwrap(), execution).unwrap();
            assert!(
                !hull.strict_edges,
                "a filled square of 1024 points stays on insertion"
            );
            let c = classify_built(hull, execution).unwrap();
            check(&c);
            assert_eq!(c.vertices, vertices);
            assert_eq!(c.coplanar_points, coplanar);
            assert_eq!(c.interior_points, interior);
        }
    }

    /// The same size on a circle takes the chain, and every site is extreme.
    #[test]
    fn large_circle_uses_the_chain() {
        let n = 1100usize;
        let points: Vec<f64> = (0..n)
            .flat_map(|i| {
                let t = (i as f64) * core::f64::consts::TAU / (n as f64);
                [t.cos(), t.sin()]
            })
            .collect();
        let cycle = monotone_cycle(&points);
        let mut vertices = cycle.clone();
        vertices.sort_unstable();
        for execution in [Execution::Sequential, Execution::Parallel] {
            let hull = SimplicialHull::build(accept(2, &points).unwrap(), execution).unwrap();
            assert!(hull.strict_edges, "a circle takes the chain");
            let c = classify_built(hull, execution).unwrap();
            check(&c);
            assert_eq!(c.vertices, vertices);
            assert!(c.coplanar_points.is_empty());
            assert!(c.interior_points.is_empty());
        }
    }

    /// The partition with the proved points skipped, and without: the
    /// reference scans every representative against every group.
    fn with_and_without_reuse(dim: usize, points: &[f64]) -> (usize, Classified<'_>, bool) {
        let built =
            |execution| SimplicialHull::build(accept(dim, points).unwrap(), execution).unwrap();
        let sequential = built(Execution::Sequential);
        let chain = sequential.strict_edges;
        let mut proved = sequential.proved_interior.clone();
        let mut parallel = built(Execution::Parallel).proved_interior;
        proved.sort_unstable();
        parallel.sort_unstable();
        assert_eq!(
            proved, parallel,
            "sequential and parallel prove the same points"
        );

        let reused = classify_built(sequential, Execution::Sequential).unwrap();
        let mut reference = built(Execution::Sequential);
        reference.proved_interior.clear();
        let reference = classify_built(reference, Execution::Sequential).unwrap();
        check(&reused);
        assert_eq!(reused.vertices, reference.vertices);
        assert_eq!(reused.coplanar_points, reference.coplanar_points);
        assert_eq!(reused.interior_points, reference.interior_points);
        for p in &proved {
            assert!(
                reference.interior_points.binary_search(p).is_ok(),
                "{p} is interior"
            );
        }
        (proved.len(), reused, chain)
    }

    #[test]
    fn reused_interior_proofs_change_no_partition() {
        let mut rng = Rng(73);
        let mut cases: Vec<(String, usize, Vec<f64>)> = Vec::new();
        for dim in 2..=4 {
            // Full integer grids: most points lie on facets, ridges, and
            // edges, where construction sees a zero sign.
            let side = 5_usize;
            let count = side.pow(dim as u32);
            let grid: Vec<f64> = (0..count)
                .flat_map(|i| (0..dim).map(move |a| ((i / side.pow(a as u32)) % side) as f64))
                .collect();
            cases.push((format!("grid {dim}"), dim, grid));
            // Random points of a small grid: duplicates and coplanar sets.
            let coarse: Vec<f64> = (0..60 * dim).map(|_| (rng.next() % 4) as f64).collect();
            cases.push((format!("coarse {dim}"), dim, coarse));
            // General position.
            let general: Vec<f64> = (0..200 * dim).map(|_| rng.unit()).collect();
            cases.push((format!("general {dim}"), dim, general));
        }
        let mut ball = Vec::new();
        for i in -3_i32..=3 {
            for j in -3_i32..=3 {
                for k in -3_i32..=3 {
                    if i * i + j * j + k * k <= 9 {
                        ball.extend([f64::from(i), f64::from(j), f64::from(k)]);
                    }
                }
            }
        }
        cases.push(("ball".to_string(), 3, ball));
        for (name, dim, points) in &cases {
            let (proved, c, chain) = with_and_without_reuse(*dim, points);
            if chain {
                assert_eq!(proved, 0, "{name}: the chain inserts no interior proof");
                continue;
            }
            let non_vertices = c.interior_points.len() + c.coplanar_points.len();
            assert!(proved > 0, "{name}: no point was proved interior");
            if name.starts_with("general") {
                // No zero sign: every non-vertex is proved, including the
                // vertices a later insertion swallowed (design §3).
                assert_eq!(proved, non_vertices, "{name}");
            } else {
                // Zero signs leave points to the scan.
                assert!(proved < non_vertices, "{name}: no point was scanned");
            }
        }
    }
}
#[cfg(test)]
mod grid_regression {
    use super::tests::classified;
    use crate::hull::simplicial::tests::Rng;

    #[test]
    fn random_integer_grids_in_high_dimensions() {
        for dim in 4..=6 {
            // Seed 3 in 6D was the first failure: two facets split a shared
            // lower face differently before every facet used placing.
            for seed in [0, 3] {
                let mut rng = Rng(seed * 31 + dim as u64);
                let points: Vec<f64> = (0..35 * dim).map(|_| (rng.next() % 4) as f64).collect();
                classified(dim, &points);
            }
        }
    }
}
