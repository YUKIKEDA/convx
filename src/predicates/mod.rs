//! Geometric predicates. They decide every topological sign.
//!
//! The orientation of k + 1 points of dimension k is the sign of the
//! determinant whose rows are `p_i - p_0` for `i` in `1..=k`. The geometric
//! degree k is independent of the hull dimension. Input coordinates are taken
//! as the exact values their bit patterns name; nothing is translated or
//! scaled first.
//!
//! Each predicate first evaluates in `f64` with a running absolute error
//! bound (see [`filter`]). When the bound certifies the sign, that sign is
//! returned. Otherwise the sign of the same polynomial is computed exactly
//! (see [`exact`]). Degree 1 compares the two coordinates directly; degrees 2
//! to 4 use dedicated expansions; larger degrees use a filtered determinant.

// The hull build (P2-1, #8) is the first caller outside tests.
#![cfg_attr(not(test), allow(dead_code))]

mod exact;
mod filter;

use filter::Approx;

/// The sign of a predicate.
///
/// Predicates return a sign, never a floating-point value. Coplanar means an
/// orientation of [`Sign::Zero`]. A point is outside a facet when the
/// orientation of the facet's points, in outward order, followed by the point
/// is [`Sign::Positive`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sign {
    /// The determinant is negative.
    Negative,
    /// The determinant is exactly zero.
    Zero,
    /// The determinant is positive.
    Positive,
}

impl Sign {
    /// The opposite sign. [`Sign::Zero`] stays zero.
    #[must_use]
    pub fn reversed(self) -> Self {
        match self {
            Self::Negative => Self::Positive,
            Self::Zero => Self::Zero,
            Self::Positive => Self::Negative,
        }
    }
}

/// The work space for exact evaluation could not be reserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExactEvaluationExhausted;

/// Orientation of `points`: k + 1 points, each of dimension k, with k >= 1.
///
/// Every coordinate must be finite. The result is the exact sign of the
/// determinant of `points[i] - points[0]`, `i = 1..=k`.
pub(crate) fn orient(points: &[&[f64]]) -> Result<Sign, ExactEvaluationExhausted> {
    debug_assert!(points.len() >= 2, "orientation needs at least two points");
    sign_of(Rows {
        origin: points[0],
        points: &points[1..],
        direction: None,
    })
}

/// Side of `query` relative to the hyperplane through `facet` (k points of
/// dimension k, in outward order): [`Sign::Positive`] is outside.
pub(crate) fn distance_sign(
    facet: &[&[f64]],
    query: &[f64],
) -> Result<Sign, ExactEvaluationExhausted> {
    let mut points: Vec<&[f64]> = Vec::with_capacity(facet.len() + 1);
    points.extend_from_slice(facet);
    points.push(query);
    orient(&points)
}

/// Whether `points` (k + 1 points of dimension k) lie on one hyperplane.
pub(crate) fn is_coplanar(points: &[&[f64]]) -> Result<bool, ExactEvaluationExhausted> {
    Ok(orient(points)? == Sign::Zero)
}

/// Orientation of a direction against a hyperplane: the exact sign of the
/// determinant of `facet[i] - facet[0]`, `i = 1..k`, followed by the row
/// `direction` as it is. `facet` holds k points of dimension k.
///
/// A direction pointing to the side where [`distance_sign`] is positive has
/// a positive sign here.
pub(crate) fn orient_direction(
    facet: &[&[f64]],
    direction: &[f64],
) -> Result<Sign, ExactEvaluationExhausted> {
    debug_assert!(!facet.is_empty(), "a hyperplane needs at least one point");
    sign_of(Rows {
        origin: facet[0],
        points: &facet[1..],
        direction: Some(direction),
    })
}

/// The rows of an orientation determinant: `p - origin` for each point,
/// then, when present, a direction row taken as it is.
#[derive(Clone, Copy)]
struct Rows<'a> {
    origin: &'a [f64],
    points: &'a [&'a [f64]],
    direction: Option<&'a [f64]>,
}

enum Row<'a> {
    Difference(&'a [f64]),
    Direction(&'a [f64]),
}

impl<'a> Rows<'a> {
    /// Size of the square determinant.
    fn k(self) -> usize {
        self.origin.len()
    }

    fn row(self, i: usize) -> Row<'a> {
        match self.points.get(i) {
            Some(p) => Row::Difference(p),
            None => Row::Direction(self.direction.unwrap_or(self.origin)),
        }
    }

    fn values(self) -> impl Iterator<Item = f64> + 'a {
        self.origin
            .iter()
            .chain(self.points.iter().flat_map(|p| p.iter()))
            .chain(self.direction.into_iter().flatten())
            .copied()
    }
}

fn sign_of(rows: Rows<'_>) -> Result<Sign, ExactEvaluationExhausted> {
    let k = rows.k();
    debug_assert!(k >= 1, "orientation needs dimension at least 1");
    debug_assert_eq!(
        rows.points.len() + usize::from(rows.direction.is_some()),
        k,
        "a determinant of size k needs k rows"
    );
    debug_assert!(
        rows.points.iter().all(|p| p.len() == k) && rows.direction.is_none_or(|d| d.len() == k),
        "every row needs dimension k"
    );
    debug_assert!(
        rows.values().all(f64::is_finite),
        "predicate input must be finite"
    );
    if let Some(sign) = filtered(rows) {
        return Ok(sign);
    }
    exact::sign_exact(rows)
}

fn from_ordering(ordering: core::cmp::Ordering) -> Sign {
    match ordering {
        core::cmp::Ordering::Less => Sign::Negative,
        core::cmp::Ordering::Equal => Sign::Zero,
        core::cmp::Ordering::Greater => Sign::Positive,
    }
}

fn filtered(rows: Rows<'_>) -> Option<Sign> {
    let k = rows.k();
    if k == 1 {
        // Degree 1: the sign of b - a is the order of two finite values.
        return match rows.row(0) {
            Row::Difference(p) => p[0].partial_cmp(&rows.origin[0]).map(from_ordering),
            Row::Direction(d) => d[0].partial_cmp(&0.0).map(from_ordering),
        };
    }
    filtered_value(rows)?.certified_sign()
}

/// The filtered determinant with its error bound, for k >= 2.
fn filtered_value(rows: Rows<'_>) -> Option<Approx> {
    let k = rows.k();
    let origin = rows.origin;
    let entry = |i: usize, j: usize| match rows.row(i) {
        Row::Difference(p) => Approx::exact(p[j]).sub(Approx::exact(origin[j])),
        Row::Direction(d) => Approx::exact(d[j]),
    };
    Some(match k {
        2 => filter::orient2(&core::array::from_fn(|i| {
            core::array::from_fn(|j| entry(i, j))
        })),
        3 => filter::orient3(&core::array::from_fn(|i| {
            core::array::from_fn(|j| entry(i, j))
        })),
        4 => filter::orient4(&core::array::from_fn(|i| {
            core::array::from_fn(|j| entry(i, j))
        })),
        _ => filter::determinant(
            (0..k)
                .map(|i| (0..k).map(|j| entry(i, j)).collect())
                .collect(),
        )?,
    })
}

/// The cofactor vector `c` of the hyperplane through `facet` (k points of
/// dimension k): `c_j` is the determinant of the edges `facet[i] - facet[0]`
/// followed by the unit row `e_j`, so `orient_direction(facet, v)` is the sign
/// of `c . v`. Each entry is returned as `(value, bound)` with
/// `|c_j - value| <= bound`.
///
/// Returns `None` when a bound is not finite or a filtered elimination could
/// not certify a pivot.
pub(crate) fn direction_cofactors(facet: &[&[f64]]) -> Option<Vec<(f64, f64)>> {
    let k = facet.len();
    debug_assert!(k >= 1, "a hyperplane needs at least one point");
    if k == 1 {
        return Some(vec![(1.0, 0.0)]);
    }
    let mut unit = vec![0.0; k];
    let mut cofactors = Vec::with_capacity(k);
    for j in 0..k {
        unit[j] = 1.0;
        let value = filtered_value(Rows {
            origin: facet[0],
            points: &facet[1..],
            direction: Some(&unit),
        });
        unit[j] = 0.0;
        let value = value?;
        if !value.value().is_finite() || !value.error().is_finite() {
            return None;
        }
        cofactors.push((value.value(), value.error()));
    }
    Some(cofactors)
}

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;

/// The largest error bound at which a filtered cofactor direction is used.
const FILTERED_DIRECTION_LIMIT: f64 = 1e-10;

/// The unit direction of the cofactor vector of the hyperplane through
/// `facet` (k points of dimension k), with a bound `err` such that the
/// returned vector is within `err` (Euclidean) of the exact unit direction
/// `c / |c|`. Its sign follows `c`: `orient_direction(facet, v)` is the sign
/// of `v . c`.
///
/// The filtered cofactors are used when their bounds certify the direction;
/// otherwise the cofactors are computed exactly. Returns `None` only when
/// every cofactor is exactly zero, that is, when the points are affinely
/// dependent.
pub(crate) fn cofactor_direction(
    facet: &[&[f64]],
) -> Result<Option<(Vec<f64>, f64)>, ExactEvaluationExhausted> {
    let k = facet.len() as f64;
    if let Some(cofactors) = direction_cofactors(facet) {
        let bound: f64 = cofactors.iter().map(|&(_, e)| e).sum::<f64>() * (1.0 + k * UNIT_ROUNDOFF);
        let length = cofactors.iter().map(|&(c, _)| c * c).sum::<f64>().sqrt();
        let length_low = length * (1.0 - (k + 3.0) * UNIT_ROUNDOFF);
        if length_low.is_finite() && length_low > 0.0 {
            // |c/|c| - c^/|c^|| <= 2|c - c^| / |c^|, plus the rounding of
            // the normalization.
            let err = (2.0 * bound / length_low + 4.0 * (k + 4.0) * UNIT_ROUNDOFF)
                * (1.0 + 8.0 * UNIT_ROUNDOFF);
            // A loose certificate is not a useful reference; the exact
            // cofactors are.
            if err <= FILTERED_DIRECTION_LIMIT {
                let direction = cofactors.iter().map(|&(c, _)| c / length).collect();
                return Ok(Some((direction, err)));
            }
        }
    }
    let err = k * 2f64.powi(-49);
    Ok(exact::cofactor_direction_exact(facet)?.map(|d| (d, err)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orient_of(points: &[Vec<f64>]) -> Sign {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        orient(&refs).unwrap()
    }

    fn exact_of(points: &[Vec<f64>]) -> Sign {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        exact::sign_exact(Rows {
            origin: refs[0],
            points: &refs[1..],
            direction: None,
        })
        .unwrap()
    }

    /// The standard simplex: origin and the unit vectors, positively oriented.
    fn unit_simplex(k: usize) -> Vec<Vec<f64>> {
        let mut points = vec![vec![0.0; k]];
        for i in 0..k {
            let mut p = vec![0.0; k];
            p[i] = 1.0;
            points.push(p);
        }
        points
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn unit(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        }
    }

    #[test]
    fn unit_simplex_is_positive_for_every_degree() {
        for k in 1..=8 {
            assert_eq!(orient_of(&unit_simplex(k)), Sign::Positive, "k = {k}");
        }
    }

    #[test]
    fn swapping_two_points_reverses_the_sign() {
        for k in 1..=7 {
            let mut points = unit_simplex(k);
            let s = orient_of(&points);
            points.swap(k - 1, k);
            assert_eq!(orient_of(&points), s.reversed(), "k = {k}");
        }
    }

    #[test]
    fn repeated_point_is_zero() {
        for k in 1..=7 {
            let mut points = unit_simplex(k);
            points[k] = points[0].clone();
            assert_eq!(orient_of(&points), Sign::Zero, "k = {k}");
        }
    }

    #[test]
    fn overflowing_filter_falls_back_to_exact() {
        for k in 2..=6 {
            let mut points = unit_simplex(k);
            for p in &mut points {
                for x in p.iter_mut() {
                    *x *= f64::MAX / 2.0;
                }
            }
            points[0] = vec![-f64::MAX / 2.0; k];
            assert_eq!(orient_of(&points), exact_of(&points), "k = {k}");
            assert_ne!(orient_of(&points), Sign::Zero, "k = {k}");
        }
    }

    #[test]
    fn collinear_and_one_ulp_off() {
        let a = vec![0.1, 0.1];
        let b = vec![0.3, 0.3];
        let c = vec![0.7, 0.7];
        assert_eq!(orient_of(&[a.clone(), b.clone(), c.clone()]), Sign::Zero);
        let up = vec![0.7, f64::from_bits(0.7_f64.to_bits() + 1)];
        let down = vec![0.7, f64::from_bits(0.7_f64.to_bits() - 1)];
        assert_eq!(orient_of(&[a.clone(), b.clone(), up]), Sign::Positive);
        assert_eq!(orient_of(&[a, b, down]), Sign::Negative);
    }

    #[test]
    fn subnormal_product_is_not_taken_as_zero() {
        // det = (2^-600)(2^-600) - 0, which underflows to zero in f64.
        let tiny = 2.0_f64.powi(-600);
        let points = [vec![0.0, 0.0], vec![tiny, 0.0], vec![0.0, tiny]];
        assert_eq!(orient_of(&points), Sign::Positive);
    }

    #[test]
    fn filter_agrees_with_exact_on_random_and_nearly_flat_inputs() {
        let mut rng = Rng(7);
        for k in 2..=7 {
            for trial in 0..300 {
                let mut points: Vec<Vec<f64>> = (0..=k)
                    .map(|_| (0..k).map(|_| rng.unit()).collect())
                    .collect();
                if trial % 2 == 0 {
                    // Put the last point on the affine hull of the first k
                    // points (up to rounding), so many cases are near zero.
                    let weights: Vec<f64> = (0..k).map(|_| rng.unit()).collect();
                    let sum: f64 = weights.iter().sum();
                    let last: Vec<f64> = (0..k)
                        .map(|j| (0..k).map(|i| weights[i] / sum * points[i][j]).sum())
                        .collect();
                    points[k] = last;
                }
                assert_eq!(
                    orient_of(&points),
                    exact_of(&points),
                    "k = {k}, trial = {trial}"
                );
            }
        }
    }

    #[test]
    fn filter_certifies_general_position() {
        let mut rng = Rng(11);
        for k in 2..=7 {
            let certified = (0..200)
                .filter(|_| {
                    let points: Vec<Vec<f64>> = (0..=k)
                        .map(|_| (0..k).map(|_| rng.unit()).collect())
                        .collect();
                    let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
                    filtered(Rows {
                        origin: refs[0],
                        points: &refs[1..],
                        direction: None,
                    })
                    .is_some()
                })
                .count();
            assert!(
                certified >= 195,
                "k = {k}: only {certified} of 200 certified"
            );
        }
    }

    #[test]
    fn integer_lattice_coplanar_is_zero() {
        // Points on the plane x + y + z = 3 with integer coordinates.
        let points = [
            vec![1.0, 1.0, 1.0],
            vec![3.0, 0.0, 0.0],
            vec![0.0, 3.0, 0.0],
            vec![2.0, 2.0, -1.0],
        ];
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        assert!(is_coplanar(&refs).unwrap());
    }

    #[test]
    fn distance_sign_follows_the_outward_convention() {
        // Facet x = 1 in 2D, oriented so the origin side is inside.
        let a = [1.0, 0.0];
        let b = [1.0, 1.0];
        let facet: [&[f64]; 2] = [&b, &a];
        assert_eq!(distance_sign(&facet, &[0.0, 0.5]).unwrap(), Sign::Negative);
        assert_eq!(distance_sign(&facet, &[2.0, 0.5]).unwrap(), Sign::Positive);
        assert_eq!(distance_sign(&facet, &[1.0, 7.0]).unwrap(), Sign::Zero);
    }

    #[test]
    fn direction_agrees_with_the_distance_sign() {
        let a = [1.0, 0.0];
        let b = [1.0, 1.0];
        let facet: [&[f64]; 2] = [&b, &a];
        assert_eq!(
            orient_direction(&facet, &[1.0, 0.0]).unwrap(),
            Sign::Positive
        );
        assert_eq!(
            orient_direction(&facet, &[-1.0, 3.0]).unwrap(),
            Sign::Negative
        );
        assert_eq!(orient_direction(&facet, &[0.0, 5.0]).unwrap(), Sign::Zero);
        let point: [&[f64]; 1] = [&[4.0]];
        assert_eq!(orient_direction(&point, &[-2.0]).unwrap(), Sign::Negative);
        // A tiny direction whose filtered determinant underflows.
        let tiny = f64::from_bits(1);
        assert_eq!(
            orient_direction(&facet, &[tiny, 0.0]).unwrap(),
            Sign::Positive
        );
    }

    #[test]
    fn huge_and_tiny_mixed() {
        let big = 2.0_f64.powi(1000);
        let small = 2.0_f64.powi(-1000);
        let points = [
            vec![0.0, 0.0, 0.0],
            vec![big, 0.0, 0.0],
            vec![0.0, small, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        assert_eq!(orient_of(&points), Sign::Positive);
        let flat = [
            vec![0.0, 0.0, 0.0],
            vec![big, 0.0, 0.0],
            vec![0.0, small, 0.0],
            vec![big, small, 0.0],
        ];
        assert_eq!(orient_of(&flat), Sign::Zero);
    }
}
