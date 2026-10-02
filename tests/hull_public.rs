//! The published convex hull, through the public API only.

use convx::{ConvexHullBuilder, ConvexHullError};

fn cube() -> Vec<f64> {
    (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1)))
        .collect()
}

#[test]
fn square_facets_planes_and_cycles() {
    let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.0];
    let hull = ConvexHullBuilder::new(2, &points).build().unwrap();
    assert_eq!(hull.vertices, vec![0, 1, 2, 3]);
    assert_eq!(hull.coplanar_points, vec![4]);
    let lists: Vec<Vec<u32>> = hull.facets.iter().map(|f| f.vertices.clone()).collect();
    assert_eq!(lists, vec![vec![0, 1], vec![0, 3], vec![1, 2], vec![2, 3]]);
    // Facet [0, 1] is y = 0 with outward normal (0, -1).
    assert_eq!(hull.facets[0].plane.normal, vec![0.0, -1.0]);
    assert_eq!(hull.facets[0].plane.offset, 0.0);
    assert_eq!(hull.facets[0].neighbors, vec![1, 2]);
    assert_eq!(hull.boundary_cycle(0), Some(vec![0, 1]));
    assert_eq!(hull.boundary_cycle(4), None);
    assert_eq!(hull.volume(), 1.0);
}

#[test]
fn cube_volume_cycles_and_outward_triangles() {
    let hull = ConvexHullBuilder::new(3, &cube()).build().unwrap();
    assert_eq!(hull.facets.len(), 6);
    // Each term is a rounded sixth, so the sum is within a few ulps of 1.
    assert!((hull.volume() - 1.0).abs() < 4.0 * f64::EPSILON);
    let triangulation = hull.triangulation();
    assert_eq!(triangulation.len(), 12);
    assert!(!triangulation.is_empty());
    assert!(triangulation.get(12).is_none());
    // Lexicographic order of the ascending lists before the swap.
    let sorted: Vec<Vec<u32>> = triangulation
        .iter()
        .map(|s| {
            let mut v = s.vertices.to_vec();
            v.sort_unstable();
            v
        })
        .collect();
    let mut expected = sorted.clone();
    expected.sort();
    assert_eq!(sorted, expected);
    for simplex in triangulation.iter() {
        let facet = &hull.facets[simplex.facet as usize];
        // The triangle's right-hand normal points along the facet normal.
        let p = |v: u32| -> [f64; 3] {
            let i = v as usize * 3;
            let c = cube();
            [c[i], c[i + 1], c[i + 2]]
        };
        let (a, b, c) = (
            p(simplex.vertices[0]),
            p(simplex.vertices[1]),
            p(simplex.vertices[2]),
        );
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let dot: f64 = cross
            .iter()
            .zip(&facet.plane.normal)
            .map(|(x, n)| x * n)
            .sum();
        assert!(dot > 0.0);
    }
    // Face z = 0 is [0, 1, 2, 3]; seen from below (outside) it runs 0, 2, 3, 1.
    let bottom = hull
        .facets
        .iter()
        .position(|f| f.vertices == vec![0, 1, 2, 3])
        .unwrap();
    assert_eq!(hull.boundary_cycle(bottom as u32), Some(vec![0, 2, 3, 1]));
    assert_eq!(hull.facets[bottom].plane.normal, vec![0.0, 0.0, -1.0]);
}

#[test]
fn segment_in_one_dimension() {
    let points = [2.0, -1.0, 0.5, -1.0, 5.0];
    let hull = ConvexHullBuilder::new(1, &points).build().unwrap();
    assert_eq!(hull.representative, vec![0, 1, 2, 1, 4]);
    assert_eq!(hull.vertices, vec![1, 4]);
    assert_eq!(hull.interior_points, vec![0, 2]);
    assert_eq!(hull.facets.len(), 2);
    assert!(hull.facets.iter().all(|f| f.neighbors.is_empty()));
    assert_eq!(hull.facets[0].plane.normal, vec![-1.0]);
    assert_eq!(hull.facets[1].plane.normal, vec![1.0]);
    assert_eq!(hull.boundary_cycle(1), Some(vec![4]));
    assert_eq!(hull.volume(), 6.0);
    assert_eq!(hull.triangulation().len(), 2);
}

#[test]
fn tetrahedron_and_four_dimensions() {
    let points = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let hull = ConvexHullBuilder::new(3, &points).build().unwrap();
    assert!((hull.volume() - 1.0 / 6.0).abs() < 1e-15);
    let simplex4: Vec<f64> = (0..5)
        .flat_map(|i| (0..4).map(move |a| if i == a + 1 { 1.0 } else { 0.0 }))
        .collect();
    let hull4 = ConvexHullBuilder::new(4, &simplex4).build().unwrap();
    assert_eq!(hull4.facets.len(), 5);
    assert_eq!(hull4.boundary_cycle(0), None);
    assert!((hull4.volume() - 1.0 / 24.0).abs() < 1e-15);
}

#[test]
fn planes_contain_their_vertices() {
    let mut state = 17_u64;
    let mut unit = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1_u64 << 53) as f64 - 0.5
    };
    let points: Vec<f64> = (0..300).map(|_| unit()).collect();
    let hull = ConvexHullBuilder::new(3, &points).build().unwrap();
    for facet in &hull.facets {
        let length: f64 = facet.plane.normal.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((length - 1.0).abs() < 1e-14);
        for &v in &facet.vertices {
            let x = &points[v as usize * 3..v as usize * 3 + 3];
            let value: f64 = x
                .iter()
                .zip(&facet.plane.normal)
                .map(|(a, n)| a * n)
                .sum::<f64>()
                + facet.plane.offset;
            assert!(value.abs() < 1e-14);
        }
        for &v in &hull.vertices {
            let x = &points[v as usize * 3..v as usize * 3 + 3];
            let value: f64 = x
                .iter()
                .zip(&facet.plane.normal)
                .map(|(a, n)| a * n)
                .sum::<f64>()
                + facet.plane.offset;
            assert!(value < 1e-14);
        }
    }
}

#[test]
fn input_errors_through_the_builder() {
    assert_eq!(
        ConvexHullBuilder::new(0, &[]).build().err(),
        Some(ConvexHullError::NonPositiveDimension)
    );
    assert_eq!(
        ConvexHullBuilder::new(2, &[0.0, 0.0, 1.0, 0.0, 2.0, 0.0])
            .build()
            .err(),
        Some(ConvexHullError::DegenerateDimension {
            actual_dim: 1,
            spanning_points: vec![0, 1]
        })
    );
}

#[test]
fn offset_overflow_is_a_non_finite_plane() {
    let m = f64::MAX;
    let points = [m, m / 2.0, m / 2.0, m, 0.0, 0.0];
    assert_eq!(
        ConvexHullBuilder::new(2, &points).build().err(),
        Some(ConvexHullError::NonFiniteFacetPlane)
    );
}
