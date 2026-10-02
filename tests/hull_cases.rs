//! Phase 2 completion inputs (design §10). Expected results come from the
//! statement of each case. Debug builds also run the §10 invariant checker
//! inside every `build`.

// Helpers outside #[test] functions unwrap results of inputs that are valid
// by construction.
#![allow(clippy::unwrap_used)]

use convx::{ConvexHull, ConvexHullBuilder, ConvexHullError};

fn build(dim: usize, points: &[f64]) -> ConvexHull {
    ConvexHullBuilder::new(dim, points).build().unwrap()
}

fn facet_sets(hull: &ConvexHull) -> Vec<Vec<u32>> {
    hull.facets.iter().map(|f| f.vertices.clone()).collect()
}

fn cube() -> Vec<f64> {
    (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1)))
        .collect()
}

// D = 1

#[test]
fn one_dimension_two_points() {
    let hull = build(1, &[3.0, -2.0]);
    assert_eq!(hull.vertices, vec![0, 1]);
    assert_eq!(hull.facets.len(), 2);
    assert!(hull.facets.iter().all(|f| f.neighbors.is_empty()));
    // Facets are ordered by vertex list: [0] (x = 3), then [1] (x = -2).
    assert_eq!(hull.boundary_cycle(0), Some(vec![0]));
    assert_eq!(hull.boundary_cycle(1), Some(vec![1]));
    assert_eq!(hull.facets[0].plane.normal, vec![1.0]);
}

#[test]
fn one_dimension_duplicates_and_signed_zero() {
    let hull = build(1, &[0.0, 1.0, -0.0, 1.0, 0.5]);
    assert_eq!(hull.representative, vec![0, 1, 0, 1, 4]);
    assert_eq!(hull.vertices, vec![0, 1]);
    assert_eq!(hull.interior_points, vec![4]);
    assert!(hull.facets.iter().all(|f| f.neighbors.is_empty()));
}

// D = 2

#[test]
fn triangle() {
    let hull = build(2, &[0.0, 0.0, 4.0, 0.0, 0.0, 3.0]);
    assert_eq!(hull.vertices, vec![0, 1, 2]);
    assert_eq!(facet_sets(&hull), vec![vec![0, 1], vec![0, 2], vec![1, 2]]);
    assert_eq!(hull.volume(), 6.0);
}

#[test]
fn square_with_edge_point_and_interior_point() {
    let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 1.0, 0.25, 0.75];
    let hull = build(2, &points);
    // The fourth corner is extreme; the edge midpoint is not.
    assert_eq!(hull.vertices, vec![0, 1, 2, 3]);
    assert_eq!(hull.coplanar_points, vec![4]);
    assert_eq!(hull.interior_points, vec![5]);
}

#[test]
fn many_duplicates() {
    let mut points = Vec::new();
    for _ in 0..20 {
        points.extend([0.0, 0.0, 2.0, 0.0, 0.0, 2.0, 0.5, 0.5]);
    }
    let hull = build(2, &points);
    assert_eq!(hull.vertices, vec![0, 1, 2]);
    assert_eq!(hull.interior_points, vec![3]);
    for (i, &r) in hull.representative.iter().enumerate() {
        assert_eq!(r as usize, i % 4);
    }
}

#[test]
fn collinear_fails() {
    assert_eq!(
        ConvexHullBuilder::new(2, &[0.0, 0.0, 1.0, 1.0, 3.0, 3.0, 2.0, 2.0])
            .build()
            .err(),
        Some(ConvexHullError::DegenerateDimension {
            actual_dim: 1,
            spanning_points: vec![0, 1]
        })
    );
}

// D = 3

#[test]
fn tetrahedron() {
    let hull = build(
        3,
        &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    );
    assert_eq!(hull.vertices, vec![0, 1, 2, 3]);
    assert_eq!(hull.facets.len(), 4);
    assert!(hull.facets.iter().all(|f| f.neighbors.len() == 3));
}

#[test]
fn cube_with_face_and_edge_points() {
    let mut points = cube();
    points.extend([0.5, 0.5, 1.0, 1.0, 0.5, 0.0, 0.5, 0.5, 0.5]);
    let hull = build(3, &points);
    assert_eq!(hull.vertices, (0..8).collect::<Vec<u32>>());
    assert_eq!(hull.coplanar_points, vec![8, 9]);
    assert_eq!(hull.interior_points, vec![10]);
    assert_eq!(hull.facets.len(), 6);
}

#[test]
fn near_coplanar_input_succeeds() {
    // A square with one corner lifted by one ulp.
    let lift = f64::from_bits(1.0_f64.to_bits() + 1) - 1.0;
    let points = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, lift, 0.5, 0.5, -1.0,
    ];
    let hull = build(3, &points);
    assert_eq!(hull.vertices, vec![0, 1, 2, 3, 4]);
    // The top is two triangles, not one quadrilateral.
    assert!(hull.facets.iter().all(|f| f.vertices.len() == 3));
}

#[test]
fn coplanar_input_fails() {
    let points = [0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 3.0, 5.0, 1.0];
    assert_eq!(
        ConvexHullBuilder::new(3, &points).build().err(),
        Some(ConvexHullError::DegenerateDimension {
            actual_dim: 2,
            spanning_points: vec![0, 1, 2]
        })
    );
}

// Transforms. Coordinates are multiples of 1/8 below 16, so translations by
// integers, scales by powers of two, and negation are exact.

fn sample(dim: usize, count: usize, seed: u64) -> Vec<f64> {
    let mut state = seed;
    (0..dim * count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 33) % 128) as f64 / 8.0 - 8.0
        })
        .collect()
}

fn check_transform(dim: usize, points: &[f64], map: impl Fn(&[f64]) -> Vec<f64>) {
    let original = build(dim, points);
    let moved: Vec<f64> = points.chunks_exact(dim).flat_map(map).collect();
    let transformed = build(dim, &moved);
    assert_eq!(facet_sets(&original), facet_sets(&transformed));
    assert_eq!(original.vertices, transformed.vertices);
    assert_eq!(original.coplanar_points, transformed.coplanar_points);
    assert_eq!(original.interior_points, transformed.interior_points);
}

#[test]
fn translation_and_scale_keep_facets() {
    for dim in 2..=4 {
        let mut points = sample(dim, 40, dim as u64);
        // Include a grid corner region for exact coplanarities.
        points.extend(
            (0..1 << dim).flat_map(|i: u32| (0..dim).map(move |a| f64::from((i >> a) & 1) * 4.0)),
        );
        check_transform(dim, &points, |p| p.iter().map(|x| x + 1024.0).collect());
        check_transform(dim, &points, |p| p.iter().map(|x| x * 0.25).collect());
        check_transform(dim, &points, |p| p.iter().map(|x| x * 64.0).collect());
        check_transform(dim, &points, |p| p.iter().map(|x| -x * 2.0).collect());
    }
}

#[test]
fn axis_swap_keeps_facets() {
    for dim in 2..=4 {
        let points = sample(dim, 40, 100 + dim as u64);
        check_transform(dim, &points, |p| {
            let mut q = p.to_vec();
            q.swap(0, dim - 1);
            q
        });
    }
}

#[test]
fn input_permutation_keeps_facets_after_mapping() {
    let dim = 3;
    let mut points = sample(dim, 50, 7);
    points.extend(cube().iter().map(|x| x * 8.0));
    let count = points.len() / dim;
    let permutation: Vec<usize> = (0..count).map(|i| (i * 37 + 11) % count).collect();
    let shuffled: Vec<f64> = permutation
        .iter()
        .flat_map(|&i| points[i * dim..(i + 1) * dim].iter().copied())
        .collect();
    let original = build(dim, &points);
    let moved = build(dim, &shuffled);
    // Map each shuffled index to the original index, then to its
    // representative in the original hull.
    let to_original = |j: u32| original.representative[permutation[j as usize]];
    let mut mapped: Vec<Vec<u32>> = moved
        .facets
        .iter()
        .map(|f| {
            let mut v: Vec<u32> = f.vertices.iter().map(|&j| to_original(j)).collect();
            v.sort_unstable();
            v
        })
        .collect();
    mapped.sort();
    assert_eq!(mapped, facet_sets(&original));
}
