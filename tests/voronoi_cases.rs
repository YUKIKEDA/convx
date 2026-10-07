//! Phase 4 completion inputs for Voronoi (design §10): the vertices are
//! exactly the lower logical facets of the lift, checked by the exact
//! oracle, and the square's single vertex does not depend on the diagonal.

#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::oracle::{check_delaunay, check_voronoi};
use convx::{
    ConvexHullError, DelaunayBuilder, VoronoiBuilder, VoronoiCells, VoronoiDiagram,
    VoronoiInterfaces, VoronoiRays, VoronoiVertices,
};

fn build(dim: usize, points: &[f64]) -> VoronoiDiagram {
    let v = VoronoiBuilder::new(dim, points).build().unwrap();
    let t = DelaunayBuilder::new(dim, points).build().unwrap();
    check_delaunay(&t, points);
    check_voronoi(&v, &t, points);
    v
}

#[test]
fn square_has_one_vertex_whatever_the_diagonal() {
    // Two input orders pull the square from different corners, so the
    // Delaunay diagonals differ; the Voronoi vertex is the same, and no
    // interface joins the ends of either diagonal.
    let a = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
    let b = [2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 0.0];
    let da = DelaunayBuilder::new(2, &a).build().unwrap();
    let db = DelaunayBuilder::new(2, &b).build().unwrap();
    // In a, the diagonal is (0,0)-(2,2): sites 0 and 2. In b, site 0 is
    // (2,0), so the diagonal is (2,0)-(0,2): again sites 0 and 2 by index,
    // the other diagonal by position.
    assert!(da.simplices().iter().all(|s| s.vertices().contains(&0)));
    assert!(db.simplices().iter().all(|s| s.vertices().contains(&0)));
    for (points, name) in [(&a, "a"), (&b, "b")] {
        let v = build(2, points);
        assert_eq!(v.vertices().len(), 1, "{name}");
        assert_eq!(v.vertices().get(0_u32).unwrap().sites(), vec![0, 1, 2, 3]);
        assert!(v
            .vertices()
            .get(0_u32)
            .unwrap()
            .coords()
            .iter()
            .all(|x| (x - 1.0).abs() < 1e-12));
        let pairs: Vec<[u32; 2]> = v.interfaces().iter().map(|f| f.sites()).collect();
        assert!(
            !pairs.contains(&[0, 2]) && !pairs.contains(&[1, 3]),
            "{name}: {pairs:?}"
        );
        assert_eq!(pairs.len(), 4);
    }
}

#[test]
fn hand_inputs_match_the_oracle() {
    let cube: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    let cases: Vec<(usize, Vec<f64>, usize)> = vec![
        (1, vec![5.0, -2.0], 1),
        (2, vec![0.0, 0.0, 4.0, 0.0, 1.0, 3.0], 1),
        (2, vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0], 1),
        (
            2,
            vec![0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0, 5.0, 1.0],
            // The square (one group) and three triangles.
            4,
        ),
        (
            3,
            vec![0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 3.0],
            1,
        ),
        (3, cube, 1),
    ];
    for (dim, points, vertices) in cases {
        let v = build(dim, &points);
        assert_eq!(v.vertices().len(), vertices, "D = {dim}, {points:?}");
    }
}

#[test]
fn random_and_grid_inputs_match_the_oracle() {
    let mut state = 0x1234_5678_9abc_def1_u64;
    let mut next = move |range: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % range) as f64
    };
    for (dim, count, range) in [
        (2, 40, 1000),
        (2, 40, 5),
        (3, 30, 1000),
        (3, 30, 3),
        (4, 16, 100),
    ] {
        let points: Vec<f64> = (0..dim * count).map(|_| next(range)).collect();
        build(dim, &points);
    }
}

#[test]
fn sites_not_spanning_d_fail() {
    let line = [0.0, 0.0, 1.0, 1.0, 3.0, 3.0];
    assert!(matches!(
        VoronoiBuilder::new(2, &line).build(),
        Err(ConvexHullError::DegenerateDimension { actual_dim: 1, .. })
    ));
}

#[test]
fn overflowing_lift_does_not_fail() {
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0, 5.0, 1.0];
    let huge: Vec<f64> = points.iter().map(|x| x * 2f64.powi(600)).collect();
    let small = build(2, &points);
    let big = VoronoiBuilder::new(2, &huge).build().unwrap();
    let sites = |v: &VoronoiDiagram| {
        v.vertices()
            .iter()
            .map(|x| x.sites().to_vec())
            .collect::<Vec<_>>()
    };
    assert_eq!(sites(&big), sites(&small));
    assert_eq!(big.interfaces().len(), small.interfaces().len());
    // The circumcenters are the small ones scaled by 2^600.
    for (b, s) in big.vertices().iter().zip(small.vertices().iter()) {
        for (x, y) in b.coords().iter().zip(s.coords()) {
            assert!((x / 2f64.powi(600) - y).abs() <= 1e-12 * y.abs().max(1.0));
        }
    }
}

/// The four collection views are public names: a caller can write them in
/// a signature.
#[test]
fn collection_views_can_be_named() {
    fn counts(
        vertices: VoronoiVertices<'_>,
        rays: VoronoiRays<'_>,
        cells: VoronoiCells<'_>,
        interfaces: VoronoiInterfaces<'_>,
    ) -> [usize; 4] {
        [vertices.len(), rays.len(), cells.len(), interfaces.len()]
    }
    // A unit square: one vertex, four rays, four cells, four interfaces.
    let v = build(2, &[0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0]);
    assert_eq!(
        counts(v.vertices(), v.rays(), v.cells(), v.interfaces()),
        [1, 4, 4, 4]
    );
}
