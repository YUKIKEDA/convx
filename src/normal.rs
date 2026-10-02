//! Unit normal of a hyperplane through D points of dimension D.
//!
//! The direction comes from the Householder QR factorization (`faer`) of the
//! D x (D - 1) matrix of edge vectors: the last column of the full Q is
//! orthogonal to the column space of the edges. Which of the two opposite
//! directions is returned is decided only by the exact sign of
//! [`orient_direction`] against the computed vector, never by a
//! floating-point comparison.
//!
//! QR can lose a direction: when an edge is nearly parallel to the span of
//! the others, a Householder step whose remaining column norm is at rounding
//! level is skipped, and the last column of Q is orthogonal to the edges only
//! up to that rounding, possibly far from the true normal while still on the
//! correct side. So the QR result is checked against the unit direction of
//! the facet's cofactor vector, which is certified by the predicate filter or
//! computed exactly. When the two differ by more than the certified error
//! plus `QR_TOLERANCE`, the cofactor direction is returned instead.
//!
//! Coordinates are first multiplied by one power of two so that the largest
//! magnitude lies in [1, 2). The scaling is exact except for components that
//! underflow, and it keeps the edge vectors finite for any finite input.

// The public builder (P2-4, #11) is the first caller outside tests.
#![cfg_attr(not(test), allow(dead_code))]

use faer::Mat;

use crate::predicates::{cofactor_direction, orient_direction, ExactEvaluationExhausted, Sign};

/// `2^k` for `-1022 <= k <= 1023`.
fn power_of_two(k: i32) -> f64 {
    debug_assert!(
        (-1022..=1023).contains(&k),
        "exponent outside the normal range"
    );
    f64::from_bits(((k + 1023) as u64) << 52)
}

/// `x * 2^k` for `|k| <= 2 * 1022`, in two exact steps.
pub(crate) fn scale_by_power_of_two(x: f64, k: i32) -> f64 {
    let half = k / 2;
    x * power_of_two(half) * power_of_two(k - half)
}

/// The exponent `e` with `2^e <= |x| < 2^(e + 1)` for a nonzero finite `x`.
pub(crate) fn binary_exponent(x: f64) -> i32 {
    let bits = x.abs().to_bits();
    let biased = (bits >> 52) as i32;
    if biased == 0 {
        // Subnormal: the value is fraction * 2^-1074.
        let fraction = bits & ((1_u64 << 52) - 1);
        63 - fraction.leading_zeros() as i32 - 1074
    } else {
        biased - 1023
    }
}

/// Unit normal of the hyperplane through `facet`: D affinely independent
/// points of dimension D.
///
/// The returned vector `n` satisfies `orient_direction(facet, n) == outward`.
/// `outward` must be [`Sign::Positive`] or [`Sign::Negative`]. Returns
/// `Ok(None)` when no finite vector with that certified orientation could be
/// formed, for example when the points are not affinely independent.
pub(crate) fn unit_normal(
    facet: &[&[f64]],
    outward: Sign,
) -> Result<Option<Vec<f64>>, ExactEvaluationExhausted> {
    let d = facet.len();
    debug_assert!(d >= 1, "a hyperplane needs at least one point");
    debug_assert!(
        facet.iter().all(|p| p.len() == d),
        "facet needs D points of dimension D"
    );
    debug_assert!(
        outward != Sign::Zero,
        "the outward side must be a nonzero sign"
    );
    if outward == Sign::Zero {
        return Ok(None);
    }

    let candidate = if d == 1 {
        vec![1.0]
    } else {
        // The certified direction of the cofactor vector checks the QR
        // result (see the module documentation).
        let scaled = exact_unit_scaling(facet);
        let reference = match &scaled {
            Some(points) => {
                let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
                cofactor_direction(&refs)?
            }
            None => cofactor_direction(facet)?,
        };
        let Some((direction, err)) = reference else {
            // Every cofactor is zero: the points are affinely dependent.
            return Ok(None);
        };
        match qr_normal(facet) {
            Some(n) if distance_up_to_sign(&n, &direction) <= err + QR_TOLERANCE => n,
            _ => direction,
        }
    };

    let sign = orient_direction(facet, &candidate)?;
    Ok(if sign == outward {
        Some(candidate)
    } else if sign == outward.reversed() {
        Some(candidate.into_iter().map(|x| -x).collect())
    } else {
        None
    })
}

/// How far the QR normal may lie from the certified cofactor direction,
/// beyond that direction's own error bound, before it is rejected.
const QR_TOLERANCE: f64 = 1e-8;

/// `min(|a - b|, |a + b|)`.
fn distance_up_to_sign(a: &[f64], b: &[f64]) -> f64 {
    let minus: f64 = a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum();
    let plus: f64 = a.iter().zip(b).map(|(x, y)| (x + y) * (x + y)).sum();
    minus.min(plus).sqrt()
}

/// The facet scaled by the power of two that brings its largest magnitude
/// into [1, 2), when that scaling is exact for every coordinate.
pub(crate) fn exact_unit_scaling(facet: &[&[f64]]) -> Option<Vec<Vec<f64>>> {
    let largest = facet
        .iter()
        .flat_map(|p| p.iter())
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    if largest == 0.0 {
        return None;
    }
    let shift = -binary_exponent(largest);
    let scaled: Vec<Vec<f64>> = facet
        .iter()
        .map(|p| p.iter().map(|&x| scale_by_power_of_two(x, shift)).collect())
        .collect();
    // Bit equality after the round trip is the test for an exact scaling; it
    // does not decide a geometric sign.
    let exact = facet.iter().zip(&scaled).all(|(p, q)| {
        p.iter()
            .zip(q)
            .all(|(&x, &y)| scale_by_power_of_two(y, -shift) == x)
    });
    exact.then_some(scaled)
}

/// The null direction of the edge matrix, unit length, of either sign.
fn qr_normal(facet: &[&[f64]]) -> Option<Vec<f64>> {
    let d = facet.len();
    let largest = facet
        .iter()
        .flat_map(|p| p.iter())
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    if largest == 0.0 {
        return None;
    }
    let shift = -binary_exponent(largest);
    let scaled: Vec<Vec<f64>> = facet
        .iter()
        .map(|p| p.iter().map(|&x| scale_by_power_of_two(x, shift)).collect())
        .collect();
    let origin = &scaled[0];
    let edges = Mat::from_fn(d, d - 1, |i, j| scaled[j + 1][i] - origin[i]);
    let q = edges.qr().compute_Q();
    let column: Vec<f64> = (0..d).map(|i| q[(i, d - 1)]).collect();
    let norm = column.iter().map(|x| x * x).sum::<f64>().sqrt();
    let normal: Vec<f64> = column.iter().map(|x| x / norm).collect();
    normal.iter().all(|x| x.is_finite()).then_some(normal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal_of(points: &[Vec<f64>], outward: Sign) -> Option<Vec<f64>> {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        unit_normal(&refs, outward).unwrap()
    }

    fn assert_close(a: &[f64], b: &[f64]) {
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() < 1e-14, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn binary_exponent_brackets_the_value() {
        for &x in &[
            1.0,
            1.5,
            2.0,
            3.0,
            0.75,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::from_bits(3),
        ] {
            let e = binary_exponent(x);
            let low = scale_by_power_of_two(1.0, e);
            let high = scale_by_power_of_two(1.0, e + 1);
            assert!(low <= x && (x < high || e + 1 > 1023), "{x}: e = {e}");
        }
    }

    #[test]
    fn one_dimension_points_to_the_requested_side() {
        assert_eq!(normal_of(&[vec![3.0]], Sign::Positive), Some(vec![1.0]));
        assert_eq!(normal_of(&[vec![3.0]], Sign::Negative), Some(vec![-1.0]));
    }

    #[test]
    fn axis_aligned_facets_give_axis_normals() {
        // Facet x_last = 1 through the unit points of the other axes.
        for d in 2..=5 {
            let points: Vec<Vec<f64>> = (0..d)
                .map(|i| {
                    let mut p = vec![0.0; d];
                    if i > 0 {
                        p[i - 1] = 1.0;
                    }
                    p[d - 1] = 1.0;
                    p
                })
                .collect();
            let n = normal_of(&points, Sign::Positive).unwrap();
            let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
            assert_eq!(orient_direction(&refs, &n).unwrap(), Sign::Positive);
            let mut axis = vec![0.0; d];
            axis[d - 1] = n[d - 1].signum();
            assert_close(&n, &axis);
        }
    }

    #[test]
    fn flipping_the_side_flips_the_normal() {
        let points = vec![
            vec![0.2, 0.1, 0.0],
            vec![1.0, 0.3, 0.2],
            vec![0.4, 1.1, 0.5],
        ];
        let up = normal_of(&points, Sign::Positive).unwrap();
        let down = normal_of(&points, Sign::Negative).unwrap();
        let negated: Vec<f64> = up.iter().map(|x| -x).collect();
        assert_eq!(down, negated);
    }

    #[test]
    fn random_facets_are_unit_and_orthogonal() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut unit = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        };
        for d in 2..=8 {
            for _ in 0..20 {
                let points: Vec<Vec<f64>> =
                    (0..d).map(|_| (0..d).map(|_| unit()).collect()).collect();
                let n = normal_of(&points, Sign::Positive).unwrap();
                let length: f64 = n.iter().map(|x| x * x).sum::<f64>().sqrt();
                assert!((length - 1.0).abs() < 1e-14);
                for p in &points[1..] {
                    let dot: f64 = p
                        .iter()
                        .zip(&points[0])
                        .zip(&n)
                        .map(|((a, b), c)| (a - b) * c)
                        .sum();
                    assert!(dot.abs() < 1e-12, "d = {d}: dot = {dot}");
                }
            }
        }
    }

    #[test]
    fn huge_and_tiny_coordinates_stay_finite() {
        for scale in [1e300, 1e-300, f64::MAX / 4.0, f64::from_bits(1 << 40)] {
            let points = vec![vec![-scale, 0.0], vec![scale, scale]];
            let n = normal_of(&points, Sign::Negative).unwrap();
            assert!(n.iter().all(|x| x.is_finite()));
            let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
            assert_eq!(orient_direction(&refs, &n).unwrap(), Sign::Negative);
        }
    }

    #[test]
    fn nearly_parallel_edges_keep_the_true_normal() {
        // Review of #33: edges (0, 0, 1) and (t, t, 1). The normal is
        // (-1, 1, 0) / sqrt(2) for every t > 0 for which both 1 + t and
        // 2 + t are exact; QR alone returned (-1, 0, 0) for t = 2^-48 ..
        // 2^-51. Below that the rounded points have other normals, which the
        // exact cofactors give.
        let diagonal = [-0.5_f64.sqrt(), 0.5_f64.sqrt(), 0.0];
        for e in 40..=60 {
            let t = 2f64.powi(-e);
            let points = vec![
                vec![1.0, 2.0, 3.0],
                vec![1.0, 2.0, 4.0],
                vec![1.0 + t, 2.0 + t, 4.0],
            ];
            let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
            let exact = crate::predicates::cofactor_direction(&refs).unwrap();
            let Some(n) = normal_of(&points, Sign::Positive) else {
                assert!(
                    exact.is_none(),
                    "t = 2^-{e}: dependent only if every cofactor is zero"
                );
                continue;
            };
            if e <= 51 {
                assert!(
                    distance_up_to_sign(&n, &diagonal) < 1e-12,
                    "t = 2^-{e}: {n:?}"
                );
            }
            let (direction, _) = exact.unwrap();
            assert!(
                distance_up_to_sign(&n, &direction) < 1e-12,
                "t = 2^-{e}: {n:?}"
            );
            assert_eq!(orient_direction(&refs, &n).unwrap(), Sign::Positive);
            // The point A + (1, 2, 0) is on the side the normal points to
            // exactly when its exact orientation is positive.
            let dot = n[0] * 1.0 + n[1] * 2.0;
            let side = orient_direction(&refs, &[1.0, 2.0, 0.0]).unwrap();
            assert_eq!(dot > 0.0, side == Sign::Positive, "t = 2^-{e}");
        }
    }

    #[test]
    fn random_near_degenerate_facets_match_the_exact_cofactors() {
        let mut state = 0x1234_5678_9abc_def0_u64;
        let mut unit = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        };
        for d in 3..=6 {
            for trial in 0..40 {
                let mut points: Vec<Vec<f64>> =
                    (0..d).map(|_| (0..d).map(|_| unit()).collect()).collect();
                // Make the last point nearly a copy of the previous one plus
                // a tiny step, so one direction is short.
                let tiny = 2f64.powi(-30 - (trial % 20));
                let last: Vec<f64> = points[d - 2].iter().map(|x| x + tiny * unit()).collect();
                points[d - 1] = last;
                let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
                let reference = crate::predicates::cofactor_direction(&refs).unwrap();
                let n = normal_of(&points, Sign::Positive);
                // None only for dependent points: every cofactor zero.
                let (Some(n), Some((direction, err))) = (n.clone(), reference.clone()) else {
                    assert!(n.is_none() && reference.is_none(), "d = {d}, trial {trial}");
                    continue;
                };
                // The contract: within the certified error plus the check's
                // tolerance of the cofactor direction.
                assert!(
                    distance_up_to_sign(&n, &direction) <= err + QR_TOLERANCE,
                    "d = {d}, trial {trial}"
                );
            }
        }
    }

    /// Cofactors of an integer edge matrix by Laplace expansion in `i128`, an
    /// implementation independent of the predicate module.
    fn integer_cofactors(points: &[Vec<i64>]) -> Vec<i128> {
        fn det(m: &[Vec<i128>]) -> i128 {
            if m.is_empty() {
                return 1;
            }
            (0..m.len())
                .map(|c| {
                    let minor: Vec<Vec<i128>> = m[1..]
                        .iter()
                        .map(|row| {
                            row.iter()
                                .enumerate()
                                .filter(|&(j, _)| j != c)
                                .map(|(_, &v)| v)
                                .collect()
                        })
                        .collect();
                    let term = m[0][c] * det(&minor);
                    if c % 2 == 0 {
                        term
                    } else {
                        -term
                    }
                })
                .sum()
        }
        let d = points.len();
        let edges: Vec<Vec<i128>> = points[1..]
            .iter()
            .map(|p| {
                p.iter()
                    .zip(&points[0])
                    .map(|(a, b)| i128::from(a - b))
                    .collect()
            })
            .collect();
        (0..d)
            .map(|j| {
                let mut m = edges.clone();
                let mut unit = vec![0_i128; d];
                unit[j] = 1;
                m.push(unit);
                det(&m)
            })
            .collect()
    }

    #[test]
    fn near_degenerate_integer_facets_match_an_independent_reference() {
        // Large integer coordinates with one edge of length about 1: the
        // reference normal comes from i128 Laplace cofactors.
        let mut state = 0x0f0f_1234_5678_9abc_u64;
        let mut next = |bound: i64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % (2 * bound as u64 + 1)) as i64 - bound
        };
        for d in 3..=6 {
            for _ in 0..30 {
                // As large as the i128 Laplace expansion allows, so that the
                // facet is badly conditioned and QR alone loses accuracy.
                let bound: i64 = match d {
                    3 => 1 << 40,
                    4 => 1 << 38,
                    5 => 1 << 30,
                    _ => 1 << 23,
                };
                let mut points: Vec<Vec<i64>> = (0..d)
                    .map(|_| (0..d).map(|_| next(bound)).collect())
                    .collect();
                let step: Vec<i64> = (0..d).map(|_| next(1)).collect();
                let last: Vec<i64> = points[d - 2]
                    .iter()
                    .zip(&step)
                    .map(|(a, b)| a + b)
                    .collect();
                points[d - 1] = last;
                let cofactors = integer_cofactors(&points);
                let floats: Vec<Vec<f64>> = points
                    .iter()
                    .map(|p| p.iter().map(|&x| x as f64).collect())
                    .collect();
                let n = normal_of(&floats, Sign::Positive);
                if cofactors.iter().all(|&c| c == 0) {
                    assert!(n.is_none());
                    continue;
                }
                let n = n.unwrap();
                let length = cofactors
                    .iter()
                    .map(|&c| (c as f64) * (c as f64))
                    .sum::<f64>()
                    .sqrt();
                let reference: Vec<f64> = cofactors.iter().map(|&c| c as f64 / length).collect();
                assert!(
                    distance_up_to_sign(&n, &reference) <= 2e-8,
                    "d = {d}: {n:?} vs {reference:?}"
                );
            }
        }
    }

    #[test]
    fn dependent_points_give_none() {
        let points = vec![
            vec![1.0, 1.0, 1.0],
            vec![2.0, 2.0, 2.0],
            vec![3.0, 3.0, 3.0],
        ];
        assert_eq!(normal_of(&points, Sign::Positive), None);
    }
}
