//! Unit normal of a hyperplane through D points of dimension D.
//!
//! Both the published normal and the working normal used by distance scans
//! are the unit direction of the facet's cofactor vector (design §1, #210).
//! The predicate filter certifies that direction with an error bound below
//! 10^-10; otherwise it is computed exactly and rounded once. Which of the two
//! opposite directions is returned is proved from the certified error, and
//! the orientation determinant is not evaluated: a vector within distance 1
//! of the unit direction of the exact cofactors has a positive
//! [`orient_direction`] sign (#120).
//!
//! The published normal was a Householder QR of the edge matrix until #212.
//! It carried no bound of its own and was checked against this direction,
//! which was the more accurate of the two on every measured set
//! (docs/adr/0002-publish-the-certified-cofactor-direction.md).
//!
//! When the exact cofactors are needed, the coordinates are first multiplied
//! by one power of two so that the largest magnitude lies in [1, 2), which
//! keeps them finite for any finite input. The scaling is exact except for
//! components that underflow, and then the facet is taken as it is.

use crate::predicates::{
    certified_cofactor_direction, cofactor_direction_from, direction_cofactors, orient_direction,
    scaled_direction_cofactors_in_lanes, Cofactors, Direction, ExactEvaluationExhausted, Sign,
    COFACTOR_LANES,
};

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;
/// 2^-1073.
const ETA: f64 = f64::from_bits(2);
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

/// [`unit_normal_with`] evaluating the cofactors itself.
#[cfg(test)]
pub(crate) fn unit_normal(
    facet: &[&[f64]],
    outward: Sign,
) -> Result<Option<Vec<f64>>, ExactEvaluationExhausted> {
    unit_normal_with(facet, outward, facet_cofactors(facet).as_deref())
}

/// The filtered cofactors of `facet` (k points of dimension k) as the
/// certification of its normal reads them: of the facet scaled by one power
/// of two when that is exact for every coordinate (see
/// [`unit_scaling_shift`]), otherwise of the facet itself. [`unit_normal_with`]
/// and [`crate::cull::CullPlane::with_cofactors`] both certify against these, so a
/// caller that needs both evaluates them once and passes them on (#86).
pub(crate) fn facet_cofactors(facet: &[&[f64]]) -> Option<Cofactors> {
    with_unit_scaling(facet, direction_cofactors)
}

/// [`facet_cofactors`] of `COFACTOR_LANES` facets of one size, result
/// `lane` bit for bit that of facet `lane`. Facets of more than four points
/// share one elimination in lanes. The scaling is exact, so one product by
/// `2^shift`, when that is a normal number, gives the scaled copy's
/// coordinates; a facet without a shift is evaluated unscaled, times 1.
/// The coordinates must not be NaN.
pub(crate) fn facet_cofactors_in_lanes(
    facets: [&[&[f64]]; COFACTOR_LANES],
) -> [Option<Cofactors>; COFACTOR_LANES] {
    let k = facets[0].len();
    if k > 4 && facets.iter().all(|f| f.len() == k) {
        let factors = |largest: [f64; COFACTOR_LANES]| {
            let mut factors = [1.0; COFACTOR_LANES];
            for ((factor, largest), facet) in factors.iter_mut().zip(largest).zip(facets) {
                if let Some(shift) = scaling_shift_of(facet, largest) {
                    if !(-1022..=1023).contains(&shift) {
                        return None;
                    }
                    *factor = power_of_two(shift);
                }
            }
            Some(factors)
        };
        if let Some(cofactors) = scaled_direction_cofactors_in_lanes(facets, factors) {
            return cofactors;
        }
    }
    facets.map(facet_cofactors)
}

/// Unit normal of the hyperplane through `facet`: D affinely independent
/// points of dimension D. `cofactors` are the [`facet_cofactors`] of
/// `facet`. It is the [`working_normal`], the certified cofactor direction,
/// for any number of points (design §1).
///
/// The returned vector `n` satisfies `orient_direction(facet, n) == outward`.
/// `outward` must be [`Sign::Positive`] or [`Sign::Negative`]. Returns
/// `Ok(None)` when no finite vector with that certified orientation could be
/// formed, for example when the points are not affinely independent.
#[cfg(test)]
pub(crate) fn unit_normal_with(
    facet: &[&[f64]],
    outward: Sign,
    cofactors: Option<&[(f64, f64)]>,
) -> Result<Option<Vec<f64>>, ExactEvaluationExhausted> {
    debug_assert!(
        facet.iter().all(|p| p.len() == facet.len()),
        "facet needs D points of dimension D"
    );
    Ok(working_normal(facet, outward, cofactors)?.map(|n| n.to_vec()))
}

/// Working normal of the hyperplane through `facet`: the certified cofactor
/// direction, oriented so that `orient_direction(facet, n) == outward`.
///
/// Distance scans use this vector, and [`unit_normal_with`] publishes it.
/// `cofactors` are the [`facet_cofactors`] of `facet`.
pub(crate) fn working_normal(
    facet: &[&[f64]],
    outward: Sign,
    cofactors: Option<&[(f64, f64)]>,
) -> Result<Option<Direction>, ExactEvaluationExhausted> {
    let Some((direction, _)) = cofactor_reference(facet, cofactors)? else {
        return Ok(None);
    };
    Ok(orient_by_proof(facet, direction, outward))
}

/// The certified cofactor direction of `facet`, before it is oriented, with
/// its error bound, which is below 1. `None` when every cofactor is zero.
/// The facet is scaled only when the exact cofactors are needed.
fn cofactor_reference(
    facet: &[&[f64]],
    cofactors: Option<&[(f64, f64)]>,
) -> Result<Option<(Direction, f64)>, ExactEvaluationExhausted> {
    let d = facet.len();
    debug_assert!(d >= 1, "a hyperplane needs at least one point");
    debug_assert!(
        facet.iter().all(|p| p.len() == d),
        "facet needs D points of dimension D"
    );
    if d == 1 {
        return Ok(Some(([1.0].as_slice().into(), 0.0)));
    }
    if let Some(certified) = certified_cofactor_direction(d, cofactors) {
        return Ok(Some(certified));
    }
    with_unit_scaling(facet, |points| cofactor_direction_from(points, None))
}

/// `candidate`, which lies within distance 1 of the unit direction of the
/// exact cofactor vector `c` of `facet`, flipped to the `outward` side.
///
/// `orient_direction(facet, v)` is the sign of `v . c`. For `u = c / |c|`
/// and `|v - u| = e < 1`, `|v| >= 1 - e` and
/// `v . u = (|v|^2 + 1 - e^2) / 2 >= 1 - e > 0`, so the sign of the
/// candidate is positive without evaluating the determinant. Scaling the
/// facet by a power of two multiplies `c` by a positive number and keeps
/// that sign.
fn orient_by_proof(facet: &[&[f64]], candidate: Direction, outward: Sign) -> Option<Direction> {
    debug_assert!(
        outward != Sign::Zero,
        "the outward side must be a nonzero sign"
    );
    // An exhausted exact evaluation proves nothing either way; release
    // builds never evaluate it.
    if cfg!(debug_assertions) {
        if let Ok(sign) = orient_direction(facet, &candidate) {
            debug_assert_eq!(
                sign,
                Sign::Positive,
                "a certified direction lies on the positive side"
            );
        }
    }
    match outward {
        Sign::Positive => Some(candidate),
        Sign::Negative => Some(candidate.iter().map(|x| -x).collect()),
        Sign::Zero => None,
    }
}

/// The sign of `orient(facet, x)` (the facet's points followed by `x`)
/// proved from `cofactors`, the [`facet_cofactors`] of `facet`, or `None`
/// when their bounds do not decide it (#120).
///
/// The orientation determinant is linear in its last row, so it equals
/// `c . (x - p0)` for the cofactors `c` of the facet's edge rows; the
/// filtered cofactors are those of the facet scaled by a power of two,
/// which multiplies `c` by a positive number. With `d^ = fl(x - p0)`,
/// `|d - d^| <= 2u |d^|`, `|c - c^| <= e`, and the recursive sum of `k`
/// products within `gamma_k sum |c^ d^|` of the exact sum,
///
/// `|c . d - fl(c^ . d^)| <= (1 + 2u) sum e |d^| + (2u + gamma_k) sum |c^ d^|`,
///
/// plus one underflow `eta` per product. The bound is rounded up.
pub(crate) fn certified_side(
    facet: &[&[f64]],
    cofactors: &[(f64, f64)],
    x: &[f64],
) -> Option<Sign> {
    let k = facet.len();
    debug_assert!(cofactors.len() == k && x.len() == k);
    let mut sum = 0.0;
    let mut magnitude = 0.0;
    let mut spread = 0.0;
    for ((&xi, &oi), &(c, e)) in x.iter().zip(facet[0]).zip(cofactors) {
        let d = xi - oi;
        sum += c * d;
        magnitude += (c * d).abs();
        spread += e * d.abs();
    }
    let n = k as f64;
    let gamma = n * UNIT_ROUNDOFF / (1.0 - n * UNIT_ROUNDOFF);
    let bound = ((1.0 + 2.0 * UNIT_ROUNDOFF) * spread
        + (2.0 * UNIT_ROUNDOFF + gamma) * magnitude
        + 2.0 * n * ETA)
        * (1.0 + 4.0 * (n + 2.0) * UNIT_ROUNDOFF);
    if !(sum.is_finite() && bound.is_finite()) {
        return None;
    }
    if sum > bound {
        Some(Sign::Positive)
    } else if sum < -bound {
        Some(Sign::Negative)
    } else {
        None
    }
}

/// The power of two that brings the largest magnitude of `facet` into
/// [1, 2), when multiplying every coordinate by it is exact. Nothing is
/// allocated.
pub(crate) fn unit_scaling_shift(facet: &[&[f64]]) -> Option<i32> {
    let largest = facet
        .iter()
        .flat_map(|p| p.iter())
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    scaling_shift_of(facet, largest)
}

/// [`unit_scaling_shift`] of `facet`, whose largest coordinate magnitude is
/// `largest`.
fn scaling_shift_of(facet: &[&[f64]], largest: f64) -> Option<i32> {
    if largest == 0.0 {
        return None;
    }
    let shift = -binary_exponent(largest);
    // Scaling up never rounds: the largest magnitude lands below 2. Scaling
    // down rounds only a value that lands below the normal range; bit
    // equality after the round trip is the test there, and it does not
    // decide a geometric sign.
    let exact = shift >= 0
        || facet
            .iter()
            .flat_map(|p| p.iter())
            .all(|&x| scale_by_power_of_two(scale_by_power_of_two(x, shift), -shift) == x);
    exact.then_some(shift)
}

/// Largest number of points per facet whose scaled copy lives on the stack.
const STACK_POINTS: usize = 10;

/// `f` of the facet scaled by [`unit_scaling_shift`] when that is exact,
/// otherwise of the facet itself. Facets of up to `STACK_POINTS` points
/// are scaled into stack buffers.
pub(crate) fn with_unit_scaling<R>(facet: &[&[f64]], f: impl FnOnce(&[&[f64]]) -> R) -> R {
    let Some(shift) = unit_scaling_shift(facet) else {
        return f(facet);
    };
    let k = facet.len();
    let scale = |x: f64| scale_by_power_of_two(x, shift);
    if k <= STACK_POINTS && facet.iter().all(|p| p.len() <= STACK_POINTS) {
        let mut values = [[0.0; STACK_POINTS]; STACK_POINTS];
        for (row, p) in values.iter_mut().zip(facet) {
            for (y, &x) in row.iter_mut().zip(p.iter()) {
                *y = scale(x);
            }
        }
        let mut refs: [&[f64]; STACK_POINTS] = [&[]; STACK_POINTS];
        for ((r, row), p) in refs.iter_mut().zip(&values).zip(facet) {
            *r = &row[..p.len()];
        }
        f(&refs[..k])
    } else {
        let values: Vec<Vec<f64>> = facet
            .iter()
            .map(|p| p.iter().map(|&x| scale(x)).collect())
            .collect();
        let refs: Vec<&[f64]> = values.iter().map(Vec::as_slice).collect();
        f(&refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `|a - sign * b|^2`.
    fn squared_distance(a: &[f64], b: &[f64], sign: f64) -> f64 {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - sign * y) * (x - sign * y))
            .sum()
    }

    /// `min(|a - b|, |a + b|)`.
    fn distance_up_to_sign(a: &[f64], b: &[f64]) -> f64 {
        squared_distance(a, b, 1.0)
            .min(squared_distance(a, b, -1.0))
            .sqrt()
    }

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
    fn cofactors_in_lanes_match_each_facet_bit_for_bit() {
        // Sizes with and without lanes; scales that take one product, that
        // need none (all zero), and whose shift leaves the normal range
        // (subnormal coordinates), so a group also falls back; and
        // repeated points, whose divisor is not certain.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut unit = move || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            ((z ^ (z >> 31)) >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        };
        let scales = [
            1.0,
            3.7,
            1e-200,
            1e200,
            0.0,
            1e-310,
            1.0,
            2.0f64.powi(-1000),
        ];
        let (mut in_lanes, mut alone, mut uncertain) = (0, 0, 0);
        for k in 4..=10 {
            for group in 0..24 {
                let facets: Vec<Vec<Vec<f64>>> = (0..COFACTOR_LANES)
                    .map(|lane| {
                        let scale = scales[(group + 3 * lane) % scales.len()];
                        let mut facet: Vec<Vec<f64>> = (0..k)
                            .map(|_| (0..k).map(|_| unit() * scale).collect())
                            .collect();
                        // One coordinate of one point alone sets the
                        // largest magnitude, so a scan that misses it
                        // scales by another power of two.
                        if (group + lane) % 3 == 1 {
                            facet[(group + lane) % k][lane % k] = 4.0 * scale;
                        }
                        if (group + lane) % 5 == 0 {
                            facet[k - 1] = facet[0].clone();
                        }
                        facet
                    })
                    .collect();
                let refs: Vec<Vec<&[f64]>> = facets
                    .iter()
                    .map(|f| f.iter().map(Vec::as_slice).collect())
                    .collect();
                let four: [&[&[f64]]; COFACTOR_LANES] = core::array::from_fn(|l| &*refs[l]);
                let lanes_used = k > 4
                    && k <= 9
                    && four
                        .iter()
                        .all(|f| unit_scaling_shift(f).is_none_or(|s| (-1022..=1023).contains(&s)));
                if lanes_used {
                    in_lanes += 1;
                } else {
                    alone += 1;
                }
                let bits = |c: Option<Cofactors>| {
                    c.map(|c| {
                        c.iter()
                            .map(|&(v, e)| (v.to_bits(), e.to_bits()))
                            .collect::<Vec<_>>()
                    })
                };
                for (lane, got) in facet_cofactors_in_lanes(four).into_iter().enumerate() {
                    let expected = facet_cofactors(four[lane]);
                    uncertain += usize::from(lanes_used && expected.is_none());
                    assert_eq!(
                        bits(got),
                        bits(expected),
                        "k = {k}, group {group}, lane {lane}"
                    );
                }
            }
        }
        assert!(in_lanes > 0 && alone > 0, "both paths run");
        assert!(uncertain > 0, "some lane has no certified cofactors");
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
        // 2 + t are exact; `faer`'s QR, which published the normal then,
        // returned (-1, 0, 0) for t = 2^-48 .. 2^-51. Below that the rounded
        // points have other normals, which the exact cofactors give.
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
    fn working_normal_of_nearly_parallel_edges_proves_the_inside() {
        // t = 2^-50. The true normal is the diagonal. `faer`'s QR, before
        // #135, returned a vector near (-1, 0, 0), about 0.77 from it. The
        // working normal is the cofactor direction, so it stays on the
        // diagonal, and the cull plane it certifies proves a point inside.
        let t = 2f64.powi(-50);
        let points = vec![
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 4.0],
            vec![1.0 + t, 2.0 + t, 4.0],
        ];
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        let n = working_normal(&refs, Sign::Positive, facet_cofactors(&refs).as_deref())
            .unwrap()
            .expect("the three points are affinely independent");
        let diagonal = [-0.5_f64.sqrt(), 0.5_f64.sqrt(), 0.0];
        assert!(distance_up_to_sign(&n, &diagonal) <= 1e-8, "{n:?}");
        let inside = [
            points[0][0] - diagonal[0],
            points[0][1] - diagonal[1],
            points[0][2] - diagonal[2],
        ];
        let plane = crate::cull::CullPlane::with_cofactors(
            &refs,
            &n,
            Sign::Positive,
            facet_cofactors(&refs).as_deref(),
        )
        .expect("the cofactor direction certifies a plane");
        assert_eq!(plane.proved_side(&points[0], &inside), Some(Sign::Negative));
        let mut with_point = points.clone();
        with_point.push(inside.to_vec());
        let rows: Vec<&[f64]> = with_point.iter().map(Vec::as_slice).collect();
        assert_eq!(
            crate::predicates::orient(&rows).unwrap(),
            Sign::Negative,
            "a point the plane proves inside is strictly inside"
        );
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
                let (Some(n), Some((direction, _))) = (n.clone(), reference.clone()) else {
                    assert!(n.is_none() && reference.is_none(), "d = {d}, trial {trial}");
                    continue;
                };
                // Both are within 1e-10 of the exact unit direction (design
                // §5), so within 2e-10 of each other.
                assert!(
                    distance_up_to_sign(&n, &direction) <= 2e-10,
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
                    if c.is_multiple_of(2) {
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
    fn integer_facets_match_an_independent_reference_within_the_published_bound() {
        // Large integer coordinates, the reference normal from i128 Laplace
        // cofactors. Every other facet is thin, with one edge of length
        // about 1, so its cofactors cancel and the filter's bound decides
        // whether the direction is certified or computed exactly. Design §5
        // bounds the published normal by 1e-10; the reference itself is
        // within a few ulps of the exact unit direction.
        let mut state = 0x0f0f_1234_5678_9abc_u64;
        let mut next = |bound: i64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % (2 * bound as u64 + 1)) as i64 - bound
        };
        for d in 3..=6 {
            for trial in 0..60 {
                // As large as the i128 Laplace expansion allows, so that a
                // thin facet is badly conditioned.
                let bound: i64 = match d {
                    3 => 1 << 40,
                    4 => 1 << 38,
                    5 => 1 << 30,
                    _ => 1 << 23,
                };
                let mut points: Vec<Vec<i64>> = (0..d)
                    .map(|_| (0..d).map(|_| next(bound)).collect())
                    .collect();
                if trial % 2 == 1 {
                    let step: Vec<i64> = (0..d).map(|_| next(1)).collect();
                    let last: Vec<i64> = points[d - 2]
                        .iter()
                        .zip(&step)
                        .map(|(a, b)| a + b)
                        .collect();
                    points[d - 1] = last;
                }
                let cofactors = integer_cofactors(&points);
                let floats: Vec<Vec<f64>> = points
                    .iter()
                    .map(|p| p.iter().map(|&x| x as f64).collect())
                    .collect();
                let refs: Vec<&[f64]> = floats.iter().map(Vec::as_slice).collect();
                let cofactors_f = facet_cofactors(&refs);
                let length = cofactors
                    .iter()
                    .map(|&c| (c as f64) * (c as f64))
                    .sum::<f64>()
                    .sqrt();
                // `orient_direction(facet, v)` is the sign of `v . c`, so the
                // outward normal lies near `sign * c / |c|`, not only up to
                // sign: the side is checked as well as the direction (#120).
                for (outward, sign) in [(Sign::Positive, 1.0), (Sign::Negative, -1.0)] {
                    let published = unit_normal(&refs, outward).unwrap();
                    let working = working_normal(&refs, outward, cofactors_f.as_deref()).unwrap();
                    if cofactors.iter().all(|&c| c == 0) {
                        assert!(published.is_none() && working.is_none());
                        continue;
                    }
                    let reference: Vec<f64> = cofactors
                        .iter()
                        .map(|&c| sign * c as f64 / length)
                        .collect();
                    for n in [published.unwrap(), working.unwrap().to_vec()] {
                        assert!(
                            squared_distance(&n, &reference, 1.0).sqrt() <= 1e-10,
                            "d = {d}, trial {trial}: {n:?} vs {reference:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn unit_scaling_is_the_same_on_the_stack_and_the_heap() {
        // 3 points use the stack buffers; 11 points exceed them.
        for k in [3, STACK_POINTS + 1] {
            let points: Vec<Vec<f64>> = (0..k)
                .map(|i| (0..k).map(|j| (i * k + j) as f64 * 1e5 - 3.0).collect())
                .collect();
            let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
            let largest = points.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
            // Every coordinate is an integer below 2^53, so the scaling is exact.
            let shift = unit_scaling_shift(&refs).unwrap();
            assert_eq!(shift, -binary_exponent(largest));
            let scaled =
                with_unit_scaling(&refs, |f| f.iter().map(|p| p.to_vec()).collect::<Vec<_>>());
            for (p, q) in points.iter().zip(&scaled) {
                assert_eq!(p.len(), q.len());
                for (&x, &y) in p.iter().zip(q) {
                    assert_eq!(y, x * 2f64.powi(shift), "k = {k}");
                }
            }
        }
        // The smallest subnormal does not survive the scaling of 2^1000:
        // the facet is passed on as it is.
        let points = [vec![2f64.powi(1000), ETA / 2.0], vec![1.0, 1.0]];
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        assert_eq!(unit_scaling_shift(&refs), None);
        let same = with_unit_scaling(&refs, |f| f[0][1]);
        assert_eq!(same, ETA / 2.0);
    }

    #[test]
    fn certified_side_matches_an_independent_exact_sign() {
        // Integer facets and query points, the reference sign from the i128
        // Laplace cofactors: c . (x - p0) is the orientation determinant.
        let mut state = 0x5eed_0120_u64;
        let mut next = |bound: i64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % (2 * bound as u64 + 1)) as i64 - bound
        };
        let (mut decided, mut coplanar) = (0, 0);
        for d in 2..=6 {
            let bound: i64 = match d {
                2 | 3 => 1 << 30,
                4 => 1 << 25,
                5 => 1 << 20,
                _ => 1 << 16,
            };
            for trial in 0..400 {
                let mut points: Vec<Vec<i64>> = (0..d)
                    .map(|_| (0..d).map(|_| next(bound)).collect())
                    .collect();
                // Half the facets are thin: the last point is one step from
                // the one before, so the cofactors cancel and their filter
                // bounds matter.
                if trial % 2 == 1 {
                    let step: Vec<i64> = (0..d).map(|_| next(1)).collect();
                    points[d - 1] = points[d - 2]
                        .iter()
                        .zip(&step)
                        .map(|(a, b)| a + b)
                        .collect();
                }
                let c = integer_cofactors(&points);
                if c.iter().all(|&v| v == 0) {
                    continue;
                }
                // Every third query lies on the plane: an affine combination
                // with integer weights; the others are near it or anywhere.
                let x: Vec<i64> = match (trial / 2) % 3 {
                    0 => (0..d)
                        .map(|j| points[0][j] + (points[1][j] - points[0][j]) * 2)
                        .collect(),
                    1 => (0..d)
                        .map(|j| points[0][j] + (points[1][j] - points[0][j]) * 2 + next(1))
                        .collect(),
                    _ => (0..d).map(|_| next(bound)).collect(),
                };
                let exact: i128 = c
                    .iter()
                    .zip(&x)
                    .zip(&points[0])
                    .map(|((&cj, &xj), &oj)| cj * i128::from(xj - oj))
                    .sum();
                let expected = match exact.signum() {
                    1 => Sign::Positive,
                    -1 => Sign::Negative,
                    _ => Sign::Zero,
                };
                let floats: Vec<Vec<f64>> = points
                    .iter()
                    .map(|p| p.iter().map(|&v| v as f64).collect())
                    .collect();
                let refs: Vec<&[f64]> = floats.iter().map(Vec::as_slice).collect();
                let query: Vec<f64> = x.iter().map(|&v| v as f64).collect();
                let Some(cofactors) = facet_cofactors(&refs) else {
                    continue;
                };
                if let Some(sign) = certified_side(&refs, &cofactors, &query) {
                    assert_eq!(sign, expected, "d = {d}, trial {trial}");
                    decided += 1;
                }
                if expected == Sign::Zero {
                    coplanar += 1;
                    assert_eq!(certified_side(&refs, &cofactors, &query), None);
                }
            }
        }
        // The proof is not vacuous, and the exact zeros were exercised.
        assert!(decided > 1000, "decided {decided}");
        assert!(coplanar > 400, "coplanar {coplanar}");
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
