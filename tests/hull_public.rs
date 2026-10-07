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
    assert_eq!(hull.vertices(), vec![0, 1, 2, 3]);
    assert_eq!(hull.coplanar_points(), vec![4]);
    let lists: Vec<Vec<u32>> = hull
        .facets()
        .iter()
        .map(|f| f.vertices().to_vec())
        .collect();
    assert_eq!(lists, vec![vec![0, 1], vec![0, 3], vec![1, 2], vec![2, 3]]);
    // Facet [0, 1] is y = 0 with outward normal (0, -1).
    assert_eq!(hull.facets().get(0_u32).unwrap().normal(), vec![0.0, -1.0]);
    assert_eq!(hull.facets().get(0_u32).unwrap().offset(), 0.0);
    assert_eq!(hull.facets().get(0_u32).unwrap().neighbors(), vec![1, 2]);
    assert_eq!(hull.boundary_cycle(0), Some(vec![0, 1]));
    assert_eq!(hull.boundary_cycle(4), None);
    assert_eq!(hull.volume(), 1.0);
}

#[test]
fn cube_volume_cycles_and_outward_triangles() {
    let hull = ConvexHullBuilder::new(3, &cube()).build().unwrap();
    assert_eq!(hull.facets().len(), 6);
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
        let facet = &hull.facets().get((simplex.facet as usize) as u32).unwrap();
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
        let dot: f64 = cross.iter().zip(facet.normal()).map(|(x, n)| x * n).sum();
        assert!(dot > 0.0);
    }
    // Face z = 0 is [0, 1, 2, 3]; seen from below (outside) it runs 0, 2, 3, 1.
    let bottom = hull
        .facets()
        .iter()
        .position(|f| f.vertices() == vec![0, 1, 2, 3])
        .unwrap();
    assert_eq!(hull.boundary_cycle(bottom as u32), Some(vec![0, 2, 3, 1]));
    assert_eq!(
        hull.facets().get((bottom) as u32).unwrap().normal(),
        vec![0.0, 0.0, -1.0]
    );
}

#[test]
fn segment_in_one_dimension() {
    let points = [2.0, -1.0, 0.5, -1.0, 5.0];
    let hull = ConvexHullBuilder::new(1, &points).build().unwrap();
    assert_eq!(hull.representative(), vec![0, 1, 2, 1, 4]);
    assert_eq!(hull.vertices(), vec![1, 4]);
    assert_eq!(hull.interior_points(), vec![0, 2]);
    assert_eq!(hull.facets().len(), 2);
    assert!(hull.facets().iter().all(|f| f.neighbors().is_empty()));
    assert_eq!(hull.facets().get(0_u32).unwrap().normal(), vec![-1.0]);
    assert_eq!(hull.facets().get(1_u32).unwrap().normal(), vec![1.0]);
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
    assert_eq!(hull4.facets().len(), 5);
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
    let centroid: Vec<f64> = (0..3)
        .map(|a| {
            hull.vertices()
                .iter()
                .map(|&v| points[v as usize * 3 + a])
                .sum::<f64>()
                / hull.vertices().len() as f64
        })
        .collect();
    for facet in hull.facets().iter() {
        // A residual check alone passes a wrong normal (review of #39); the
        // centroid of the vertices must be strictly inside every plane.
        let inside: f64 = centroid
            .iter()
            .zip(facet.normal())
            .map(|(a, n)| a * n)
            .sum::<f64>()
            + facet.offset();
        assert!(inside < 0.0);
        let length: f64 = facet.normal().iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!((length - 1.0).abs() < 1e-14);
        for &v in facet.vertices().iter() {
            let x = &points[v as usize * 3..v as usize * 3 + 3];
            let value: f64 = x
                .iter()
                .zip(facet.normal())
                .map(|(a, n)| a * n)
                .sum::<f64>()
                + facet.offset();
            assert!(value.abs() < 1e-14);
        }
        for &v in hull.vertices().iter() {
            let x = &points[v as usize * 3..v as usize * 3 + 3];
            let value: f64 = x
                .iter()
                .zip(facet.normal())
                .map(|(a, n)| a * n)
                .sum::<f64>()
                + facet.offset();
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

#[test]
fn nearly_parallel_edges_keep_the_true_public_plane() {
    // Review of #39: the facet [0, 1, 2] has edges (0, 0, 1) and
    // (2^-48, 2^-48, 1). Its plane contains the direction (1, 1, 0), so the
    // normal is +-(1, -1, 0) / sqrt(2), pointing away from point 3. QR alone
    // gave x = 1, which puts the centroid outside.
    let t = 2f64.powi(-48);
    let points = [
        1.0,
        2.0,
        3.0,
        1.0,
        2.0,
        4.0,
        1.0 + t,
        2.0 + t,
        4.0,
        1.0,
        3.0,
        3.5,
    ];
    let hull = ConvexHullBuilder::new(3, &points).build().unwrap();
    let facet = hull
        .facets()
        .iter()
        .find(|f| f.vertices() == vec![0, 1, 2])
        .unwrap();
    let expected = [0.5_f64.sqrt(), -(0.5_f64.sqrt()), 0.0];
    for (n, e) in facet.normal().iter().zip(expected) {
        assert!((n - e).abs() < 1e-12, "{:?}", facet.normal());
    }
    let value = |p: &[f64]| -> f64 {
        p.iter()
            .zip(facet.normal())
            .map(|(a, n)| a * n)
            .sum::<f64>()
            + facet.offset()
    };
    let centroid: Vec<f64> = (0..3)
        .map(|a| (0..4).map(|i| points[i * 3 + a]).sum::<f64>() / 4.0)
        .collect();
    assert!(value(&centroid) < -0.1, "the centroid must be inside");
    for i in 0..3 {
        assert!(value(&points[i * 3..i * 3 + 3]).abs() < 1e-14);
    }
}

// The tilt bound of the public normal (design §5, #50): within
// 1e-8 + 2e-10 (Euclidean) of the exact unit cofactor direction of the
// facet, oriented so that the inside is negative. The reference is computed
// in i128 from the input coordinates, independently of the library.

/// The promised bound, written here as a fixed number.
const TILT_BOUND: f64 = 1e-8 + 2e-10;

fn exact_determinant(m: &[Vec<i128>]) -> i128 {
    if m.len() == 1 {
        return m[0][0];
    }
    (0..m.len())
        .map(|j| {
            let minor: Vec<Vec<i128>> = m[1..]
                .iter()
                .map(|row| {
                    row.iter()
                        .enumerate()
                        .filter(|&(c, _)| c != j)
                        .map(|(_, &x)| x)
                        .collect()
                })
                .collect();
            let term = m[0][j] * exact_determinant(&minor);
            if j.is_multiple_of(2) {
                term
            } else {
                -term
            }
        })
        .sum()
}

/// `x * 2^shift` as an integer; panics if that is not exact.
fn as_integer(x: f64, shift: i32) -> i128 {
    let scaled = x * 2f64.powi(shift);
    assert_eq!(scaled.fract(), 0.0, "{x} is not a multiple of 2^-{shift}");
    scaled as i128
}

/// The exact cofactor vector of the hyperplane through `facet` (D points),
/// entry j being the determinant of the edges followed by the unit row e_j.
fn exact_cofactors(facet: &[Vec<i128>]) -> Vec<i128> {
    let d = facet.len();
    let edges: Vec<Vec<i128>> = facet[1..]
        .iter()
        .map(|p| p.iter().zip(&facet[0]).map(|(x, o)| x - o).collect())
        .collect();
    (0..d)
        .map(|j| {
            let mut m = edges.clone();
            m.push((0..d).map(|i| i128::from(i == j)).collect());
            exact_determinant(&m)
        })
        .collect()
}

/// Checks every facet of the hull of `points` against the exact direction.
/// Coordinates must be multiples of 2^-shift. Returns the largest distance.
// The inputs are valid by construction, so a failed build is a test failure.
#[allow(clippy::unwrap_used, clippy::expect_used)]
fn check_tilt(dim: usize, points: &[f64], shift: i32) -> f64 {
    let hull = ConvexHullBuilder::new(dim, points).build().unwrap();
    let at = |v: u32| -> Vec<i128> {
        points[v as usize * dim..(v as usize + 1) * dim]
            .iter()
            .map(|&x| as_integer(x, shift))
            .collect()
    };
    let mut worst = 0.0_f64;
    for facet in hull.facets().iter() {
        // Any D affinely independent facet vertices span the same
        // hyperplane; take the first D-subset with a nonzero cofactor vector.
        let vertices = &facet.vertices();
        let mut cofactors = None;
        let mut chosen = (0..dim).collect::<Vec<usize>>();
        loop {
            let pts: Vec<Vec<i128>> = chosen.iter().map(|&i| at(vertices[i])).collect();
            let c = exact_cofactors(&pts);
            if c.iter().any(|&x| x != 0) {
                cofactors = Some((c, pts[0].clone()));
                break;
            }
            // Next combination in lexicographic order.
            let n = vertices.len();
            let Some(i) = (0..dim).rev().find(|&i| chosen[i] < n - dim + i) else {
                break;
            };
            chosen[i] += 1;
            for k in i + 1..dim {
                chosen[k] = chosen[k - 1] + 1;
            }
        }
        let (mut c, origin) = cofactors.expect("a facet spans a hyperplane");
        // Divide by the gcd: same direction, and the products below stay in
        // range.
        let gcd = c.iter().fold(0_i128, |a, &b| {
            let (mut x, mut y) = (a.abs(), b.abs());
            while y != 0 {
                (x, y) = (y, x % y);
            }
            x
        });
        c.iter_mut().for_each(|x| *x /= gcd);
        // Orient: a hull vertex off the facet is on the negative side.
        let inner = *hull
            .vertices()
            .iter()
            .find(|v| vertices.binary_search(v).is_err())
            .unwrap();
        let side: i128 = at(inner)
            .iter()
            .zip(&origin)
            .zip(&c)
            .map(|((x, o), ci)| (x - o) * ci)
            .sum();
        assert_ne!(side, 0);
        if side > 0 {
            c.iter_mut().for_each(|x| *x = -*x);
        }
        let length = c
            .iter()
            .map(|&x| (x as f64) * (x as f64))
            .sum::<f64>()
            .sqrt();
        let distance = facet
            .normal()
            .iter()
            .zip(&c)
            .map(|(n, &x)| (n - x as f64 / length).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            distance <= TILT_BOUND,
            "facet {vertices:?}: normal {:?} is {distance:e} from the exact direction",
            facet.normal()
        );
        worst = worst.max(distance);
    }
    worst
}

#[test]
fn nearly_parallel_edges_keep_the_tilt_bound() {
    // The #33 facet with the short offset t swept from 2^-12 to 2^-52. QR
    // stays at rounding level until t = 2^-48, then loses the direction
    // outright; the cofactor direction is published from there on.
    for e in 12..=52 {
        let t = 2f64.powi(-e);
        let points = [
            1.0,
            2.0,
            3.0,
            1.0,
            2.0,
            4.0,
            1.0 + t,
            2.0 + t,
            4.0,
            1.0,
            3.0,
            3.5,
        ];
        check_tilt(3, &points, e);
    }
}

#[test]
fn random_facets_keep_the_tilt_bound() {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for dim in 2..=4 {
        for _ in 0..20 {
            let points: Vec<f64> = (0..dim * 12)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    ((state >> 40) % 256) as f64 / 8.0 - 16.0
                })
                .collect();
            check_tilt(dim, &points, 3);
        }
    }
}

#[test]
fn nearly_collinear_edges_keep_the_tilt_bound() {
    // A generic facet whose second edge is twice the first plus t times a
    // skew direction. QR loses accuracy gradually as t shrinks (about
    // 1e-8 near t = 2^-23, more below), so the validation must replace
    // every QR normal beyond the bound with the cofactor direction.
    for e in 8..=50 {
        let t = 2f64.powi(-e);
        let points = [
            1.0,
            2.0,
            3.0,
            2.0,
            4.25,
            5.5,
            3.0 + 0.375 * t,
            6.5 - 0.625 * t,
            8.0 + 0.5 * t,
            0.0,
            5.0,
            1.0,
        ];
        check_tilt(3, &points, e + 3);
    }
}
