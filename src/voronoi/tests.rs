//! Expectations come from each case's statement, or from properties of the
//! Voronoi diagram checked directly on the sites.

use super::*;
use crate::{ConvexHullBuilder, DelaunayBuilder};

fn diagram(dim: usize, points: &[f64]) -> VoronoiDiagram {
    let v = VoronoiBuilder::new(dim, points).build().unwrap();
    let parallel = VoronoiBuilder::new(dim, points)
        .parallel(true)
        .build()
        .unwrap();
    assert_eq!(v, parallel, "the parallel build differs");
    check(&v, points);
    v
}

/// Circumcenters are solved in f64 by QR: compared at a fixed 1e-12.
fn close(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 1e-12)
}

fn pairs(v: &VoronoiDiagram) -> Vec<[u32; 2]> {
    v.interfaces().iter().map(|f| f.sites()).collect()
}

/// Structural checks of §8: orders, lengths, incidence between vertices,
/// cells, interfaces, and rays; and each vertex equidistant from its sites
/// with no site strictly closer.
fn check(v: &VoronoiDiagram, points: &[f64]) {
    let d = v.dim();
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    // Distances are measured in coordinates divided by a power of two near
    // the largest magnitude, so squares neither overflow nor underflow.
    let largest = points.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    let unit = if largest > 0.0 {
        2f64.powi(largest.log2().floor() as i32)
    } else {
        1.0
    };
    let distance2 = |c: &[f64], i: u32| -> f64 {
        point(i)
            .iter()
            .zip(c)
            .map(|(x, y)| (x / unit - y / unit) * (x / unit - y / unit))
            .sum()
    };
    assert!(v
        .vertices()
        .iter()
        .collect::<Vec<_>>()
        .windows(2)
        .all(|w| w[0].sites() < w[1].sites()));
    assert!(v
        .interfaces()
        .iter()
        .collect::<Vec<_>>()
        .windows(2)
        .all(|w| w[0].sites() < w[1].sites()));
    assert!(v
        .cells()
        .iter()
        .collect::<Vec<_>>()
        .windows(2)
        .all(|w| w[0].site() < w[1].site()));
    let n = v.representative().len() as u32;
    for vertex in v.vertices().iter() {
        assert!(vertex.sites().len() > d);
        assert!(vertex.coords().iter().all(|x| x.is_finite()));
        let r = distance2(vertex.coords(), vertex.sites()[0]);
        for &s in vertex.sites().iter() {
            assert!((distance2(vertex.coords(), s) - r).abs() <= 1e-9 * r.max(1.0));
        }
        for q in (0..n).filter(|&q| v.representative()[q as usize] == q) {
            assert!(
                distance2(vertex.coords(), q) >= r * (1.0 - 1e-9),
                "site {q} inside"
            );
        }
    }
    let ordered = |rays: &[VoronoiRay]| {
        rays.windows(2)
            .all(|w| (w[0].apex(), &w[0].hull_facet()) < (w[1].apex(), &w[1].hull_facet()))
    };
    for cell in v.cells().iter() {
        assert!(ordered(&cell.rays().collect::<Vec<_>>()));
        for &x in cell.vertices().iter() {
            assert!(v
                .vertices()
                .get((x as usize) as u32)
                .unwrap()
                .sites()
                .contains(&cell.site()));
        }
        for ray in cell.rays() {
            assert!(cell.vertices().contains(&ray.apex()));
            let norm: f64 = ray.direction().iter().map(|x| x * x).sum();
            assert!((norm - 1.0).abs() < 1e-12);
        }
    }
    for face in v.interfaces().iter() {
        assert!(face.sites()[0] < face.sites()[1]);
        assert!(!face.vertices().is_empty());
        assert!(ordered(&face.rays().collect::<Vec<_>>()));
        for &x in face.vertices().iter() {
            let sites = &v.vertices().get((x as usize) as u32).unwrap().sites();
            assert!(sites.contains(&face.sites()[0]) && sites.contains(&face.sites()[1]));
        }
    }
}

#[test]
fn square_has_one_vertex_and_no_diagonal_interface() {
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
    let v = diagram(2, &points);
    assert_eq!(v.vertices().len(), 1);
    assert!(close(
        v.vertices().get(0_u32).unwrap().coords(),
        &[1.0, 1.0]
    ));
    assert_eq!(v.vertices().get(0_u32).unwrap().sites(), vec![0, 1, 2, 3]);
    assert_eq!(pairs(&v), vec![[0, 1], [0, 3], [1, 2], [2, 3]]);
    for face in v.interfaces().iter() {
        assert_eq!(face.vertices(), vec![0]);
        assert_eq!(face.rays().len(), 1);
        assert_eq!(
            face.rays().next().unwrap().hull_facet(),
            face.sites().to_vec()
        );
    }
    // The ray between cells 0 and 1 runs down, along the edge y = 0.
    assert_eq!(
        v.interfaces()
            .get(0_u32)
            .unwrap()
            .rays()
            .next()
            .unwrap()
            .direction(),
        vec![0.0, -1.0]
    );
    assert!(v
        .cells()
        .iter()
        .all(|c| c.vertices() == vec![0] && c.rays().len() == 2));
}

#[test]
fn partly_cocircular_square_and_a_far_site() {
    // Vertex 0: the square's four cocircular sites, center (1, 1). Vertex 1:
    // sites 0, 1, 4, center (1, -4/3). Edge {0, 1} is interior; the four
    // other square edges and {0, 4}, {1, 4} are on the site hull.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0];
    let v = diagram(2, &points);
    assert_eq!(v.vertices().get(0_u32).unwrap().sites(), vec![0, 1, 2, 3]);
    assert!(close(
        v.vertices().get(0_u32).unwrap().coords(),
        &[1.0, 1.0]
    ));
    assert_eq!(v.vertices().get(1_u32).unwrap().sites(), vec![0, 1, 4]);
    assert!(close(
        v.vertices().get(1_u32).unwrap().coords(),
        &[1.0, -4.0 / 3.0]
    ));
    assert_eq!(
        pairs(&v),
        vec![[0, 1], [0, 3], [0, 4], [1, 2], [1, 4], [2, 3]]
    );
    let face = |a: u32, b: u32| v.interfaces().iter().find(|f| f.sites() == [a, b]).unwrap();
    assert_eq!(face(0, 1).vertices(), vec![0, 1]);
    assert!(face(0, 1).rays().len() == 0);
    assert_eq!(face(0, 4).vertices(), vec![1]);
    assert_eq!(face(0, 4).rays().next().unwrap().hull_facet(), vec![0, 4]);
    // Outward normal of the edge (0,0)-(1,-3): (-3, -1) / sqrt(10).
    let n = face(0, 4).rays().next().unwrap().direction();
    assert!((n[0] + 3.0 / 10f64.sqrt()).abs() < 1e-15 && (n[1] + 1.0 / 10f64.sqrt()).abs() < 1e-15);
    assert_eq!(v.cells().get(4_u32).unwrap().vertices(), vec![1]);
    assert_eq!(v.cells().get(4_u32).unwrap().rays().len(), 2);
    let rays0: Vec<(u32, Vec<u32>)> = v
        .cells()
        .get(0_u32)
        .unwrap()
        .rays()
        .map(|r| (r.apex(), r.hull_facet().to_vec()))
        .collect();
    assert_eq!(rays0, vec![(0, vec![0, 3]), (1, vec![0, 4])]);
}

#[test]
fn cube_has_one_vertex_and_only_edge_interfaces() {
    // All eight corners are cospherical: one vertex at the center, one ray
    // per face, and interfaces only across the 12 cube edges (no face or
    // space diagonals).
    let points: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    let v = diagram(3, &points);
    assert_eq!(v.vertices().len(), 1);
    assert!(close(
        v.vertices().get(0_u32).unwrap().coords(),
        &[1.0, 1.0, 1.0]
    ));
    let edges: Vec<[u32; 2]> = (0..8_u32)
        .flat_map(|a| (a + 1..8).map(move |b| [a, b]))
        .filter(|&[a, b]| (a ^ b).count_ones() == 1)
        .collect();
    assert_eq!(pairs(&v), edges);
    // Each edge lies on two cube faces: two rays.
    assert!(v.interfaces().iter().all(|f| f.rays().len() == 2));
    assert!(v.cells().iter().all(|c| c.rays().len() == 3));
}

#[test]
fn one_dimension() {
    // Sites 0, 3, 1, 7: vertices at the midpoints of consecutive sites.
    let points = [0.0, 3.0, 1.0, 7.0];
    let v = diagram(1, &points);
    let sites: Vec<Vec<u32>> = v.vertices().iter().map(|x| x.sites().to_vec()).collect();
    assert_eq!(sites, vec![vec![0, 2], vec![1, 2], vec![1, 3]]);
    for (vertex, mid) in v.vertices().iter().zip([0.5, 2.0, 5.0]) {
        assert!(close(vertex.coords(), &[mid]));
    }
    assert_eq!(pairs(&v), vec![[0, 2], [1, 2], [1, 3]]);
    // A finite boundary face in D = 1 is one vertex, with no ray.
    assert!(v
        .interfaces()
        .iter()
        .all(|f| f.vertices().len() == 1 && f.rays().len() == 0));
    assert_eq!(v.cells().get(0_u32).unwrap().rays().len(), 1);
    assert_eq!(
        v.cells()
            .get(0_u32)
            .unwrap()
            .rays()
            .next()
            .unwrap()
            .direction(),
        vec![-1.0]
    );
    assert_eq!(
        v.cells()
            .get(3_u32)
            .unwrap()
            .rays()
            .next()
            .unwrap()
            .direction(),
        vec![1.0]
    );
    assert!(
        v.cells().get(1_u32).unwrap().rays().len() == 0
            && v.cells().get(2_u32).unwrap().rays().len() == 0
    );
}

#[test]
fn interior_sites_have_no_rays_and_boundary_sites_have_some() {
    let mut state = 0x853c_49e6_748f_ea9b_u64;
    let mut next = move |range: u64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % range) as f64
    };
    for (dim, count, range) in [(2, 60, 50), (2, 40, 6), (3, 40, 1000), (3, 30, 4)] {
        let points: Vec<f64> = (0..dim * count).map(|_| next(range)).collect();
        let v = diagram(dim, &points);
        let hull = ConvexHullBuilder::new(dim, &points).build().unwrap();
        for cell in v.cells().iter() {
            let interior = hull.interior_points().contains(&cell.site());
            assert_eq!(
                cell.rays().len() == 0,
                interior,
                "D = {dim}, site {}",
                cell.site()
            );
        }
        // Without cospherical groups, one vertex per Delaunay simplex.
        let delaunay = DelaunayBuilder::new(dim, &points).build().unwrap();
        assert!(v.vertices().len() <= delaunay.simplices().len());
    }
}

#[test]
fn a_circumcenter_that_overflows_fails() {
    // The only triangle is nearly flat with huge coordinates; its
    // circumcenter is far beyond f64.
    let points = [0.0, 0.0, 1e308, 0.0, 5e307, 1e-300];
    assert_eq!(
        VoronoiBuilder::new(2, &points).build().err(),
        Some(ConvexHullError::NonFiniteCircumcenter)
    );
    // The Delaunay triangulation of the same sites succeeds.
    assert!(DelaunayBuilder::new(2, &points).build().is_ok());
}

#[test]
fn huge_and_tiny_coordinates_keep_finite_circumcenters() {
    // The circumcenter of sites scaled by 2^±600 is representable, though
    // the squared edges of the unscaled solve would overflow or underflow;
    // the solve scales the translated simplex exactly and back.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0];
    let base = VoronoiBuilder::new(2, &points).build().unwrap();
    for e in [600, -600] {
        let s = 2f64.powi(e);
        let scaled: Vec<f64> = points.iter().map(|x| x * s).collect();
        let v = diagram(2, &scaled);
        assert_eq!(v.vertices().len(), base.vertices().len());
        for (a, b) in v.vertices().iter().zip(base.vertices().iter()) {
            assert_eq!(a.sites(), b.sites());
            let back: Vec<f64> = a.coords().iter().map(|x| x / s).collect();
            assert!(
                close(&back, b.coords()),
                "2^{e}: {:?} vs {:?}",
                back,
                b.coords()
            );
        }
    }
}
