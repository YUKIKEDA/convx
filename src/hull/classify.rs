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
//! all facets agree. Neighbors are then recomputed from the updated simplices.

use std::collections::HashMap;

use super::input::{accept, Input};
use super::merge::merge;
use super::ridge::{fingerprint, pair_equal_keys};
use super::simplicial::{Execution, SimplicialHull};
use super::ConvexHullError;
use crate::predicates::{orient, orient_direction, Sign};

/// A simplex of the boundary complex.
pub(crate) struct ComplexSimplex {
    /// D vertices.
    pub(crate) vertices: Vec<u32>,
    /// The face that contains this simplex.
    pub(crate) face: u32,
    /// `neighbors[i]` is the simplex across the ridge opposite
    /// `vertices[i]`. Empty for D = 1.
    pub(crate) neighbors: Vec<u32>,
}

/// A logical facet with its extreme points.
pub(crate) struct Face {
    /// Extreme points, ascending.
    pub(crate) vertices: Vec<u32>,
    /// Neighboring faces, ascending.
    pub(crate) neighbors: Vec<u32>,
    /// Indices into [`Classified::simplices`].
    pub(crate) simplices: Vec<u32>,
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

/// Builds and classifies the hull of an accepted input.
pub(crate) fn classify(
    input: Input<'_>,
    execution: Execution,
) -> Result<Classified<'_>, ConvexHullError> {
    classify_built(SimplicialHull::build(input, execution)?, execution)
}

/// Classifies a built simplicial hull. Its `proved_interior` points go to
/// `interior_points` without a scan.
fn classify_built(
    hull: SimplicialHull<'_>,
    execution: Execution,
) -> Result<Classified<'_>, ConvexHullError> {
    let groups = merge(&hull)?;
    let d = hull.input.dim();

    // Simplicial vertices, and each group's member simplices in outward order.
    let mut on_complex = vec![false; hull.input.representative.len()];
    let mut group_simplices: Vec<Vec<Vec<u32>>> = Vec::with_capacity(groups.groups.len());
    for group in &groups.groups {
        let mut members = Vec::with_capacity(group.simplices.len());
        for id in &group.simplices {
            if let Some(s) = hull.facets.get(*id) {
                for &v in &s.vertices {
                    on_complex[v as usize] = true;
                }
                members.push(s.vertices.clone());
            }
        }
        group_simplices.push(members);
    }

    // Distance signs of the other representatives against every group.
    // Points construction proved strictly inside are interior already
    // (design §3) and skip the scan.
    let mut skipped = vec![false; hull.input.representative.len()];
    for &p in &hull.proved_interior {
        debug_assert!(!on_complex[p as usize], "a dropped point is not a vertex");
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
    let mut inside = vec![false; others.len()];
    for (g, group) in groups.groups.iter().enumerate() {
        let Some(simplex) = group.simplices.first().and_then(|id| hull.facets.get(*id)) else {
            continue;
        };
        inside.iter_mut().for_each(|x| *x = false);
        if let Some(cull) = simplex.cull() {
            let (rows, stride) = hull.input.engine_rows();
            cull.mark_inside(rows, stride, &others, &mut inside);
        }
        for (k, &p) in others.iter().enumerate() {
            if inside[k] {
                continue;
            }
            let side = hull.side(simplex, p)?;
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

    // Extreme points of each group.
    let mut extremes: Vec<Vec<u32>> = Vec::with_capacity(groups.groups.len());
    for (g, group) in groups.groups.iter().enumerate() {
        if group.simplices.len() == 1 && zero_points[g].is_empty() {
            // A single simplex: every vertex is extreme.
            extremes.push(group.vertices.clone());
            continue;
        }
        let mut candidates = group.vertices.clone();
        candidates.extend_from_slice(&zero_points[g]);
        candidates.sort_unstable();
        candidates.dedup();
        let plane = &group_simplices[g][0];
        extremes.push(face_extremes(&hull.input, plane, &candidates, execution)?);
    }

    let mut is_vertex = vec![false; hull.input.representative.len()];
    for e in &extremes {
        for &v in e {
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
    let mut coplanar_points: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| on_complex[p as usize] && !is_vertex[p as usize])
        .chain(
            others
                .iter()
                .zip(&on_boundary)
                .filter(|&(&p, &b)| b && !is_vertex[p as usize])
                .map(|(&p, _)| p),
        )
        .collect();
    coplanar_points.sort_unstable();
    let mut interior_points: Vec<u32> = others
        .iter()
        .zip(&on_boundary)
        .filter(|&(_, &b)| !b)
        .map(|(&p, _)| p)
        .chain(hull.proved_interior.iter().copied())
        .collect();
    interior_points.sort_unstable();

    // Boundary simplices: kept as built, or re-triangulated by placing.
    let mut simplices: Vec<ComplexSimplex> = Vec::new();
    let mut faces: Vec<Face> = Vec::with_capacity(extremes.len());
    for (g, extreme) in extremes.into_iter().enumerate() {
        let original = &groups.groups[g].vertices;
        // A single simplex whose vertices are all extreme is kept; every
        // other facet is re-triangulated by placing, so facets that share a
        // lower face split it the same way.
        let members: Vec<Vec<u32>> = if &extreme == original && group_simplices[g].len() == 1 {
            group_simplices[g].clone()
        } else {
            let q = vertices
                .iter()
                .copied()
                .find(|v| extreme.binary_search(v).is_err())
                .unwrap_or(extreme[0]);
            place(&hull.input, &extreme, q)?
        };
        let first = simplices.len() as u32;
        for vertices in members {
            simplices.push(ComplexSimplex {
                vertices,
                face: g as u32,
                neighbors: Vec::new(),
            });
        }
        faces.push(Face {
            vertices: extreme,
            neighbors: Vec::new(),
            simplices: (first..simplices.len() as u32).collect(),
        });
    }
    link_neighbors(d, &mut simplices, &mut faces);

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

/// Recomputes simplex neighbors from shared ridges, then face neighbors.
fn link_neighbors(d: usize, simplices: &mut [ComplexSimplex], faces: &mut [Face]) {
    if d == 1 {
        return;
    }
    // Ridges as sorted vertex lists packed in one buffer, with their owner.
    let mut keys: Vec<u32> = Vec::with_capacity(simplices.len() * d * (d - 1));
    let mut owners: Vec<(u32, usize)> = Vec::with_capacity(simplices.len() * d);
    for (s, simplex) in simplices.iter().enumerate() {
        for slot in 0..d {
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
    for simplex in simplices.iter_mut() {
        simplex.neighbors = vec![u32::MAX; d];
    }
    // Every ridge of a closed boundary has two sides.
    for (first, second) in pair_equal_keys(&keys, owners.len(), fingerprint) {
        let (a, slot_a) = owners[first];
        let (b, slot_b) = owners[second];
        simplices[a as usize].neighbors[slot_a] = b;
        simplices[b as usize].neighbors[slot_b] = a;
    }
    for face in faces.iter_mut() {
        let mut neighbors: Vec<u32> = face
            .simplices
            .iter()
            .flat_map(|&s| simplices[s as usize].neighbors.iter())
            .filter_map(|&n| simplices.get(n as usize).map(|t| t.face))
            .collect();
        neighbors.sort_unstable();
        neighbors.dedup();
        face.neighbors = neighbors;
    }
    for (f, face) in faces.iter_mut().enumerate() {
        face.neighbors.retain(|&n| n as usize != f);
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
            assert!(c.faces[simplex.face as usize]
                .simplices
                .contains(&(s as u32)));
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

    /// The partition with the proved points skipped, and without: the
    /// reference scans every representative against every group.
    fn with_and_without_reuse(dim: usize, points: &[f64]) -> (usize, Classified<'_>) {
        let built =
            |execution| SimplicialHull::build(accept(dim, points).unwrap(), execution).unwrap();
        let sequential = built(Execution::Sequential);
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
        (proved.len(), reused)
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
            let (proved, c) = with_and_without_reuse(*dim, points);
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
