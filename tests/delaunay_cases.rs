//! Phase 4 completion inputs for Delaunay (design §10), checked against the
//! brute-force exact oracle and the statement of each case. Every case also
//! builds the public hull, so the debug hull invariants run on it.

#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::oracle::{check_delaunay, integer_sites, orientation};
use convx::{ConvexHullBuilder, ConvexHullError, DelaunayBuilder, DelaunayTriangulation};

fn build(dim: usize, points: &[f64]) -> DelaunayTriangulation {
    let t = DelaunayBuilder::new(dim, points).build().unwrap();
    check_delaunay(&t, points);
    t
}

/// The ascending vertex lists of the simplices, sorted: the published
/// order is the construction's (design §7), which these tests do not fix.
fn cells(t: &DelaunayTriangulation) -> Vec<Vec<u32>> {
    let mut cells: Vec<Vec<u32>> = t
        .simplices()
        .iter()
        .map(|s| {
            let mut v = s.vertices().to_vec();
            v.sort_unstable();
            v
        })
        .collect();
    cells.sort();
    cells
}

#[test]
fn one_dimension_two_points() {
    let t = build(1, &[5.0, -2.0]);
    assert_eq!(t.simplices().len(), 1);
    assert_eq!(t.simplices().get(0_u32).unwrap().vertices(), vec![1, 0]);
    assert_eq!(
        t.simplices().get(0_u32).unwrap().neighbors(),
        vec![u32::MAX, u32::MAX]
    );
}

#[test]
fn two_dimensions_three_points() {
    let t = build(2, &[0.0, 0.0, 4.0, 0.0, 1.0, 3.0]);
    assert_eq!(cells(&t), vec![vec![0, 1, 2]]);
}

#[test]
fn square_entirely_cocircular() {
    let t = build(2, &[0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0]);
    assert_eq!(t.simplices().len(), 2);
}

#[test]
fn partly_cocircular() {
    // The square's corners are cocircular; (1, -3) and (5, 1) are not on
    // that circle.
    let t = build(
        2,
        &[0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0, 5.0, 1.0],
    );
    assert_eq!(t.simplices().len(), 5);
}

#[test]
fn three_dimensions_four_points() {
    let t = build(
        3,
        &[0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 3.0],
    );
    assert_eq!(cells(&t), vec![vec![0, 1, 2, 3]]);
}

#[test]
fn entirely_cospherical() {
    // The cube's corners, and the octahedron's.
    let cube: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    assert_eq!(build(3, &cube).simplices().len(), 6);
    let octahedron = [
        1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0,
    ];
    assert_eq!(build(3, &octahedron).simplices().len(), 4);
}

#[test]
fn exactly_d_plus_one_points() {
    for dim in 1..=6 {
        let points: Vec<f64> = (0..=dim)
            .flat_map(|i| (0..dim).map(move |a| if i == a + 1 { 2.0 } else { 0.0 }))
            .collect();
        let t = build(dim, &points);
        assert_eq!(t.simplices().len(), 1, "D = {dim}");
    }
}

#[test]
fn sites_not_spanning_d_fail() {
    let line = [0.0, 0.0, 1.0, 1.0, 3.0, 3.0];
    let expected = Some(ConvexHullError::DegenerateDimension {
        actual_dim: 1,
        spanning_points: vec![0, 1],
    });
    assert_eq!(DelaunayBuilder::new(2, &line).build().err(), expected);
    let plane = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
    assert!(matches!(
        DelaunayBuilder::new(3, &plane).build(),
        Err(ConvexHullError::DegenerateDimension { actual_dim: 2, .. })
    ));
}

#[test]
fn overflowing_lift_does_not_fail() {
    // Scaling by 2^600 overflows every |p|^2 in f64; the triangulation of the
    // exact lift is the same.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0, 5.0, 1.0];
    let huge: Vec<f64> = points.iter().map(|x| x * 2f64.powi(600)).collect();
    let t = DelaunayBuilder::new(2, &huge).build().unwrap();
    assert_eq!(t, build(2, &points));
}

// Transforms (exact: integer translations, powers of two, axis swaps).

/// A map of one point's coordinates.
type Map = Box<dyn Fn(&[f64]) -> Vec<f64>>;

fn sample(dim: usize, count: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..dim * count)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 64) as f64
        })
        .collect()
}

/// The original simplices renormalized on `moved`: ascending, the last two
/// swapped when the orientation on the moved sites is negative.
fn renormalized(t: &DelaunayTriangulation, moved: &[f64]) -> Vec<Vec<u32>> {
    let d = t.dim();
    let sites = integer_sites(d, moved);
    let mut out: Vec<Vec<u32>> = cells(t)
        .into_iter()
        .map(|mut v| {
            if orientation(&sites, &v) < 0 {
                v.swap(d - 1, d);
            }
            v
        })
        .collect();
    out.sort_by(|a, b| {
        let (mut x, mut y) = (a.clone(), b.clone());
        x.sort_unstable();
        y.sort_unstable();
        x.cmp(&y)
    });
    out
}

/// The oriented vertex lists, sorted by their ascending lists, as
/// [`renormalized`] returns them.
fn oriented(t: &DelaunayTriangulation) -> Vec<Vec<u32>> {
    let mut out: Vec<Vec<u32>> = t
        .simplices()
        .iter()
        .map(|s| s.vertices().to_vec())
        .collect();
    out.sort_by_key(|v| {
        let mut x = v.clone();
        x.sort_unstable();
        x
    });
    out
}

#[test]
fn transforms_keep_simplices_after_renormalizing() {
    for dim in 2..=3 {
        // A grid corner region adds exact cospherical groups.
        let mut points = sample(dim, 30, 41 + dim as u64);
        points.extend(
            (0..1 << dim).flat_map(|i: u32| (0..dim).map(move |a| f64::from((i >> a) & 1) * 64.0)),
        );
        let original = build(dim, &points);
        let maps: Vec<Map> = vec![
            Box::new(|p| p.iter().map(|x| x + 1000.0).collect()),
            Box::new(|p| p.iter().map(|x| x * 8.0).collect()),
            Box::new(|p| p.iter().map(|x| -x * 2.0).collect()),
            Box::new(move |p| {
                let mut q = p.to_vec();
                q.swap(0, dim - 1);
                q
            }),
        ];
        for (k, map) in maps.iter().enumerate() {
            let moved: Vec<f64> = points.chunks_exact(dim).flat_map(map).collect();
            let t = build(dim, &moved);
            assert_eq!(
                oriented(&t),
                renormalized(&original, &moved),
                "D = {dim}, map {k}"
            );
        }
    }
}

#[test]
fn input_permutation_keeps_general_position_simplices() {
    // Without cospherical groups the triangulation is unique, so after
    // mapping indices the simplices agree; cospherical diagonals need not.
    let dim = 2;
    let points = sample(dim, 25, 7);
    let count = points.len() / dim;
    let permutation: Vec<usize> = (0..count).map(|i| (i * 11 + 3) % count).collect();
    let shuffled: Vec<f64> = permutation
        .iter()
        .flat_map(|&i| points[i * dim..(i + 1) * dim].iter().copied())
        .collect();
    let original = build(dim, &points);
    let moved = build(dim, &shuffled);
    let to_original = |j: u32| original.representative()[permutation[j as usize]];
    let mut mapped: Vec<Vec<u32>> = cells(&moved)
        .into_iter()
        .map(|c| {
            let mut v: Vec<u32> = c.into_iter().map(to_original).collect();
            v.sort_unstable();
            v
        })
        .collect();
    mapped.sort();
    // This sample has no cospherical group: one Voronoi vertex per simplex.
    let voronoi = convx::VoronoiBuilder::new(dim, &points).build().unwrap();
    assert_eq!(voronoi.vertices().len(), original.simplices().len());
    assert_eq!(mapped, cells(&original));
}

#[test]
fn hull_invariants_run_on_every_case() {
    // The debug hull invariants run inside every public hull build; the
    // oracle's coverage check builds one for each case above. This case
    // checks that a degenerate input fails the same way for both.
    let line = [0.0, 0.0, 1.0, 1.0, 3.0, 3.0];
    assert_eq!(
        ConvexHullBuilder::new(2, &line).build().err(),
        DelaunayBuilder::new(2, &line).build().err()
    );
}
