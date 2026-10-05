//! Phase 1 completion inputs (design §10, §11).
//!
//! Every expected sign here is derived from how the case is constructed, never
//! from the library's own output.
//!
//! # The hyperplane family
//!
//! For degree k >= 2 the points `p_0 .. p_{k-1}` satisfy `x_k = x_1` exactly
//! (the last coordinate is a copy of the first), and their projections onto
//! the first k - 1 coordinates form a positively oriented simplex: `p_0' = c`
//! and `p_i' = c + h_i e_i` with every `h_i > 0` much larger than the rounding
//! of `c + h_i`. Subtracting column 1 from column k leaves a last column that
//! is zero except for `f(p_k) = p_k[k-1] - p_k[0]`, so
//!
//! `orient(p_0, ..., p_k) = sign(f(p_k)) * orient'(projection) = sign(f(p_k))`.
//!
//! Choosing `p_k[k-1]` equal to `p_k[0]`, or one ulp above or below it, gives
//! an exact zero and the two nearest nonzero signs.

use crate::cull::CullPlane;
use crate::normal::unit_normal;
use crate::predicates::{distance_sign, orient, orient_direction, Sign};

/// A change of one coordinate.
type Adjust = fn(f64) -> f64;

fn next_up(x: f64) -> f64 {
    if x == 0.0 {
        f64::from_bits(1)
    } else if x > 0.0 {
        f64::from_bits(x.to_bits() + 1)
    } else {
        f64::from_bits(x.to_bits() - 1)
    }
}

fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

fn sign_of(points: &[Vec<f64>]) -> Sign {
    let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
    orient(&refs).unwrap()
}

/// A scale family: base offset `c`, the steps `h_i`, and the free
/// coordinates of the query point.
struct Family {
    name: &'static str,
    offset: f64,
    steps: fn(usize) -> f64,
    query: f64,
}

fn families() -> Vec<Family> {
    vec![
        Family {
            name: "unit",
            offset: 0.1,
            steps: |_| 0.75,
            query: 0.3,
        },
        Family {
            name: "huge",
            offset: 1.1 * 2f64.powi(1000),
            steps: |_| 0.75 * 2f64.powi(1000),
            query: 1.3 * 2f64.powi(1000),
        },
        Family {
            name: "near max exponent",
            offset: 2f64.powi(1021),
            steps: |_| 2f64.powi(1022),
            query: 1.5 * 2f64.powi(1022),
        },
        Family {
            name: "tiny",
            offset: 1.1 * 2f64.powi(-1000),
            steps: |_| 0.75 * 2f64.powi(-1000),
            query: 1.3 * 2f64.powi(-1000),
        },
        Family {
            name: "min normal",
            offset: 2f64.powi(-1022),
            steps: |_| 2f64.powi(-1021),
            query: 3.0 * 2f64.powi(-1022),
        },
        Family {
            name: "subnormal",
            offset: f64::from_bits(1 << 20),
            steps: |_| f64::from_bits(1 << 30),
            query: f64::from_bits(3 << 28),
        },
        Family {
            name: "huge and tiny mixed",
            offset: 0.0,
            // The first axis is huge, the others tiny.
            steps: |i| {
                if i == 1 {
                    2f64.powi(900)
                } else {
                    2f64.powi(-900)
                }
            },
            query: 2f64.powi(-950),
        },
    ]
}

/// `p_0 .. p_{k-1}` on the hyperplane `x_k = x_1`.
fn hyperplane_points(k: usize, family: &Family) -> Vec<Vec<f64>> {
    (0..k)
        .map(|i| {
            let mut p: Vec<f64> = (1..k).map(|_| family.offset).collect();
            if i > 0 {
                p[i - 1] += (family.steps)(i);
            }
            p.push(p[0]);
            p
        })
        .collect()
}

/// The query point with `x_k - x_1` set by `last`.
fn query_point(k: usize, family: &Family, last: Adjust) -> Vec<f64> {
    let mut q: Vec<f64> = (1..k).map(|_| family.query).collect();
    q.push(last(q[0]));
    q
}

#[test]
fn degree_one_zero_and_one_ulp() {
    for a in [
        0.0,
        0.1,
        -3.5,
        2f64.powi(1023),
        2f64.powi(-1022),
        f64::from_bits(1),
        -(2f64.powi(1023)),
    ] {
        assert_eq!(sign_of(&[vec![a], vec![a]]), Sign::Zero, "{a}");
        assert_eq!(sign_of(&[vec![a], vec![next_up(a)]]), Sign::Positive, "{a}");
        assert_eq!(
            sign_of(&[vec![a], vec![next_down(a)]]),
            Sign::Negative,
            "{a}"
        );
    }
}

#[test]
fn degree_one_huge_and_tiny_in_one_call() {
    // k = 1 has no cancelling determinant: the sign is the order of the two
    // coordinates, here 2^1000 against 2^-1000 in the same orientation.
    let huge = 2f64.powi(1000);
    let tiny = 2f64.powi(-1000);
    assert_eq!(sign_of(&[vec![huge], vec![tiny]]), Sign::Negative);
    assert_eq!(sign_of(&[vec![tiny], vec![huge]]), Sign::Positive);
    assert_eq!(sign_of(&[vec![-huge], vec![tiny]]), Sign::Positive);
    assert_eq!(sign_of(&[vec![tiny], vec![-tiny]]), Sign::Negative);
}

#[test]
fn degree_one_exact_translation_keeps_the_sign() {
    // Integers below 2^52 plus an integer translation stay exact.
    for (a, b) in [(3.0, 5.0), (-7.0, -8.0), (12345.0, 12345.0)] {
        let expected = sign_of(&[vec![a], vec![b]]);
        for t in [2f64.powi(40), -(2f64.powi(45)), 1.0] {
            assert_eq!(
                sign_of(&[vec![a + t], vec![b + t]]),
                expected,
                "{a}, {b}, t = {t}"
            );
        }
    }
}

#[test]
fn exact_zero_and_one_ulp_for_every_family() {
    for k in 2..=6 {
        for family in families() {
            let base = hyperplane_points(k, &family);
            let cases: [(Adjust, Sign); 3] = [
                (|x| x, Sign::Zero),
                (next_up, Sign::Positive),
                (next_down, Sign::Negative),
            ];
            for (last, expected) in cases {
                let mut points = base.clone();
                points.push(query_point(k, &family, last));
                assert_eq!(sign_of(&points), expected, "k = {k}, {}", family.name);
            }
        }
    }
}

#[test]
fn swap_reverses_repeat_zeroes() {
    for k in 2..=6 {
        for family in families() {
            let mut points = hyperplane_points(k, &family);
            points.push(query_point(k, &family, next_up));
            for i in 0..k {
                let mut swapped = points.clone();
                swapped.swap(i, i + 1);
                assert_eq!(
                    sign_of(&swapped),
                    Sign::Negative,
                    "k = {k}, {}, swap {i}",
                    family.name
                );
            }
            for i in 1..=k {
                let mut repeated = points.clone();
                repeated[i] = repeated[i - 1].clone();
                assert_eq!(
                    sign_of(&repeated),
                    Sign::Zero,
                    "k = {k}, {}, repeat {i}",
                    family.name
                );
            }
        }
    }
}

/// Rows `[[a+1, a], [a, a-1]]` padded with unit rows: the determinant is -1
/// while each product is near `a^2`, so the filtered value cancels.
fn cancelling(k: usize, a: f64) -> Vec<Vec<f64>> {
    let mut points = vec![vec![0.0; k]];
    for i in 0..k {
        let mut p = vec![0.0; k];
        match i {
            0 => {
                p[0] = a + 1.0;
                p[1] = a;
            }
            1 => {
                p[0] = a;
                p[1] = a - 1.0;
            }
            _ => p[i] = 1.0,
        }
        points.push(p);
    }
    points
}

#[test]
fn cancelling_determinant() {
    for k in 2..=6 {
        for a in [2f64.powi(26), 2f64.powi(40), 2f64.powi(52)] {
            assert_eq!(
                sign_of(&cancelling(k, a)),
                Sign::Negative,
                "k = {k}, a = {a}"
            );
        }
    }
}

#[test]
fn exact_translation_keeps_the_sign() {
    // Integer coordinates below 2^52 plus an integer translation stay exact.
    for k in 2..=6 {
        let points = cancelling(k, 2f64.powi(26));
        for t in [2f64.powi(40), -(2f64.powi(45)), 12345.0] {
            let moved: Vec<Vec<f64>> = points
                .iter()
                .map(|p| p.iter().map(|x| x + t).collect())
                .collect();
            assert_eq!(sign_of(&moved), Sign::Negative, "k = {k}, t = {t}");
        }
    }
}

#[test]
fn overflowing_intermediate_does_not_fail() {
    // Coordinates near 2^1022: every product overflows f64.
    for k in 2..=6 {
        let family = &families()[2];
        let mut points = hyperplane_points(k, family);
        points.push(query_point(k, family, next_up));
        assert_eq!(sign_of(&points), Sign::Positive, "k = {k}");
    }
}

#[test]
fn normal_and_cull_on_the_same_extreme_inputs() {
    for k in 2..=6 {
        for family in families() {
            let facet = hyperplane_points(k, &family);
            let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
            for outward in [Sign::Positive, Sign::Negative] {
                let case = format!("k = {k}, {}, outward {outward:?}", family.name);
                let normal = unit_normal(&refs, outward)
                    .unwrap()
                    .unwrap_or_else(|| panic!("{case}: no normal"));
                assert_eq!(orient_direction(&refs, &normal).unwrap(), outward, "{case}");

                // The mixed family in k >= 4: its cofactors cannot be
                // certified (the filter's bound is too wide and the facet
                // does not scale exactly), so no cull plane is built. Every
                // other facet has one, on both sides.
                let mixed = family.name == "huge and tiny mixed";
                let plane = CullPlane::new(&refs, &normal, outward);
                if mixed && k >= 4 {
                    assert!(plane.is_none(), "{case}: expected no plane");
                    continue;
                }
                let plane = plane.unwrap_or_else(|| panic!("{case}: no plane"));

                // orient = sign(f), f = x_k - x_1 of the query. With the
                // outward sign positive, f > 0 is outside; with it negative,
                // f < 0 is outside.
                let on = query_point(k, &family, |x| x);
                let (just_outside, deep_inside) = if outward == Sign::Positive {
                    (
                        query_point(k, &family, next_up),
                        query_point(k, &family, |x| x * 0.5),
                    )
                } else {
                    (
                        query_point(k, &family, next_down),
                        query_point(k, &family, |x| x * 1.25),
                    )
                };
                let queries = [on, just_outside, deep_inside];
                let flat: Vec<f64> = queries.iter().flatten().copied().collect();
                let mut inside = [false; 3];
                plane.mark_inside(&facet[0], &flat, k, &[0, 1, 2], &mut inside);
                assert!(!inside[0], "{case}: culled a point on the plane");
                assert!(!inside[1], "{case}: culled a point 1 ulp outside");
                // Near the largest exponent the L1 distance l sums to
                // infinity for k >= 5, and for k = 4 on the side whose deep
                // point lies above the facet, so no finite threshold exists:
                // the plane is built but culls nothing.
                let overflowing = family.name == "near max exponent"
                    && (k >= 5 || (k == 4 && outward == Sign::Negative));
                assert_eq!(inside[2], !overflowing, "{case}: deep inside point");
                assert_eq!(
                    distance_sign(&refs, &queries[2]).unwrap(),
                    outward.reversed(),
                    "{case}"
                );
            }
        }
    }
}
