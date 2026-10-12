//! Inputs whose coordinates are far from 1 (#589), through the public API.
//!
//! A build runs on a copy of such an input scaled by one power of two,
//! exactly. Every result is compared with the result of the same points at
//! scale 1, which runs on the points themselves: what is published as
//! indices is equal, and what is published as coordinates is that of scale
//! 1 times the scale, because a power of two scales every step of those
//! computations without a rounding.

mod common;

use common::generator::Family;
use convx::{ConvexHullBuilder, DelaunayBuilder, VoronoiBuilder};

/// `2^e`, for an `e` whose halves are both normal powers.
fn two(e: i32) -> f64 {
    2f64.powi(e / 2) * 2f64.powi(e - e / 2)
}

/// The scales: beyond the hull's and the sites' thresholds on both sides,
/// between the two thresholds, and at the ends of the range.
const EXPONENTS: [i32; 8] = [-1000, -300, -70, -20, 20, 70, 300, 900];

fn scaled(points: &[f64], e: i32) -> Vec<f64> {
    let s = two(e);
    let out: Vec<f64> = points.iter().map(|x| x * s).collect();
    assert!(
        out.iter().zip(points).all(|(y, x)| y / s == *x),
        "the test's own scaling by 2^{e} is exact"
    );
    out
}

fn sets() -> Vec<(usize, Vec<f64>)> {
    let mut sets = Vec::new();
    for dim in 2..=4 {
        let n = [0, 0, 150, 100, 50][dim];
        // General position, an integer lattice (coplanar and cospherical
        // points, duplicates), and points on the faces of a cube.
        for family in [Family::Cube, Family::Lattice, Family::CubeSurf] {
            sets.push((dim, family.points(dim, n, 589)));
        }
    }
    sets
}

#[test]
fn a_hull_of_scaled_points_is_the_hull_at_scale_one() {
    for (dim, points) in sets() {
        let base = ConvexHullBuilder::new(dim, &points).build().unwrap();
        let base_planes = base.planes().unwrap();
        for e in EXPONENTS {
            let s = two(e);
            let hull = ConvexHullBuilder::new(dim, &scaled(&points, e))
                .build()
                .unwrap();
            let what = format!("D = {dim}, 2^{e}");
            assert_eq!(hull.vertices(), base.vertices(), "{what}");
            assert_eq!(hull.coplanar_points(), base.coplanar_points(), "{what}");
            assert_eq!(hull.interior_points(), base.interior_points(), "{what}");
            assert_eq!(hull.representative(), base.representative(), "{what}");
            assert_eq!(hull.facets().len(), base.facets().len(), "{what}");
            for (a, b) in hull.facets().iter().zip(base.facets().iter()) {
                assert_eq!(a.vertices(), b.vertices(), "{what}");
                assert_eq!(a.neighbors(), b.neighbors(), "{what}");
            }
            // The planes come from the input's own coordinates: the same
            // unit normal, and the offset of scale 1 times the scale.
            let planes = hull.planes().unwrap();
            for (a, b) in planes.iter().zip(base_planes.iter()) {
                assert_eq!(a.normal(), b.normal(), "{what}");
                assert_eq!(a.offset().to_bits(), (b.offset() * s).to_bits(), "{what}");
            }
        }
    }
}

#[test]
fn a_triangulation_of_scaled_sites_is_the_one_at_scale_one() {
    for (dim, points) in sets() {
        let base = DelaunayBuilder::new(dim, &points).build().unwrap();
        for e in EXPONENTS {
            let scaled = DelaunayBuilder::new(dim, &scaled(&points, e))
                .build()
                .unwrap();
            assert_eq!(scaled, base, "D = {dim}, 2^{e}");
        }
    }
}

#[test]
fn a_diagram_of_scaled_sites_is_the_one_at_scale_one_times_the_scale() {
    for (dim, points) in sets() {
        let base = VoronoiBuilder::new(dim, &points).build().unwrap();
        for e in EXPONENTS {
            let s = two(e);
            let diagram = VoronoiBuilder::new(dim, &scaled(&points, e))
                .build()
                .unwrap();
            let what = format!("D = {dim}, 2^{e}");
            assert_eq!(diagram.representative(), base.representative(), "{what}");
            assert_eq!(diagram.vertices().len(), base.vertices().len(), "{what}");
            for (a, b) in diagram.vertices().iter().zip(base.vertices().iter()) {
                assert_eq!(a.sites(), b.sites(), "{what}");
                // The coordinates are those of the sites as given: the
                // values of scale 1 times the scale. A coordinate that
                // underflows in that product is a zero of either sign.
                let expected: Vec<f64> = b.coords().iter().map(|x| x * s).collect();
                assert_eq!(a.coords(), expected, "{what}");
            }
            assert_eq!(diagram.rays().len(), base.rays().len(), "{what}");
            for (a, b) in diagram.rays().iter().zip(base.rays().iter()) {
                assert_eq!(a.apex(), b.apex(), "{what}");
                assert_eq!(a.hull_facet(), b.hull_facet(), "{what}");
                assert_eq!(a.direction(), b.direction(), "{what}");
            }
            assert_eq!(diagram.cells().len(), base.cells().len(), "{what}");
            for (a, b) in diagram.cells().iter().zip(base.cells().iter()) {
                assert_eq!(a.site(), b.site(), "{what}");
                assert_eq!(a.vertices(), b.vertices(), "{what}");
                assert_eq!(a.ray_numbers(), b.ray_numbers(), "{what}");
            }
            assert_eq!(
                diagram.interfaces().len(),
                base.interfaces().len(),
                "{what}"
            );
            for (a, b) in diagram.interfaces().iter().zip(base.interfaces().iter()) {
                assert_eq!(a.sites(), b.sites(), "{what}");
                assert_eq!(a.vertices(), b.vertices(), "{what}");
                assert_eq!(a.ray_numbers(), b.ray_numbers(), "{what}");
            }
        }
    }
}

/// A scale that would round a coordinate is not taken. `c` is above the
/// line from `a` to `b` by the smallest subnormal, so it is a vertex. One
/// scale that brings `2^1000` to 1 sends that coordinate to zero, and `c`
/// would lie on the edge.
#[test]
fn an_input_that_one_scale_would_round_is_built_as_it_is() {
    let h = two(1000);
    let tiny = f64::from_bits(1);
    let points = [0.0, 0.0, h, 0.0, h / 2.0, -tiny, h / 2.0, -h];
    let hull = ConvexHullBuilder::new(2, &points).build().unwrap();
    assert_eq!(hull.vertices(), [0, 1, 3]);
    assert_eq!(hull.interior_points(), [2]);
    let above = [0.0, 0.0, h, 0.0, h / 2.0, tiny, h / 2.0, -h];
    let hull = ConvexHullBuilder::new(2, &above).build().unwrap();
    assert_eq!(hull.vertices(), [0, 1, 2, 3]);
    assert!(hull.coplanar_points().is_empty());
    // The same for sites: with `c` on the segment the four are not in
    // convex position, and above it they are.
    let below = DelaunayBuilder::new(2, &points).build().unwrap();
    let over = DelaunayBuilder::new(2, &above).build().unwrap();
    assert_eq!(below.simplices().len(), 3);
    assert_eq!(over.simplices().len(), 2);
}
