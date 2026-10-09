//! Geometric predicates. They decide every topological sign.
//!
//! The orientation of k + 1 points of dimension k is the sign of the
//! determinant whose rows are `p_i - p_0` for `i` in `1..=k`. The geometric
//! degree k is independent of the hull dimension. Input coordinates are taken
//! as the exact values their bit patterns name; nothing is translated or
//! scaled first.
//!
//! Degree 1 compares the two coordinates directly. An orientation or
//! lifted orientation with 2 ≤ k ≤ 6 is decided in stages (design §1):
//!
//! 1. in `f64`: for k ≤ 5 a dedicated formula with a bound derived once for
//!    it (see [`semi_static`]); for k = 6 the determinant with a running
//!    absolute error bound (see [`filter`]);
//! 2. in double-double, with a bound derived once per size (see
//!    [`double_double`]);
//! 3. exactly.
//!
//! Before stage 2, and before the running bound wherever it runs, a matrix
//! with a coordinate column of zeros decides zero ([`has_zero_column`]).
//!
//! Every other predicate, and every degree above 6, evaluates in `f64` with
//! the running bound and then exactly. When a bound certifies the sign,
//! that sign is returned. The exact sign is that of the same polynomial,
//! over integers on the stack when they hold it (see [`fixed`]), and over
//! heap integers otherwise (see [`exact`]).

mod double_double;
mod exact;
mod filter;
mod fixed;
mod semi_static;

pub(crate) use filter::two_point_cofactors;

use filter::Approx;

use crate::small::Small;

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
    orient_from(points, Start::FirstStage)
}

/// Where a predicate starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Start {
    /// At the first stage, for the sizes it covers.
    FirstStage,
    /// After it: the caller ran [`first_stage`] on the same rows, and it
    /// left the sign open. Only for the sizes that stage covers.
    AfterFirstStage,
}

/// [`orient`], starting at `start`.
pub(crate) fn orient_from(
    points: &[&[f64]],
    start: Start,
) -> Result<Sign, ExactEvaluationExhausted> {
    debug_assert!(points.len() >= 2, "orientation needs at least two points");
    sign_of(
        Rows {
            origin: points[0],
            points: &points[1..],
            direction: None,
            lifted: None,
        },
        start,
    )
}

/// The first stage alone of [`orient`] (`lifted` false) or of the lifted
/// orientation (`lifted` true) of `origin` followed by `points`: the sign
/// when the semi-static bound certifies it, for the sizes that stage covers.
/// `None` means the caller must use the full predicate, which returns the
/// same sign whenever this does. For a caller that holds its rows in arrays
/// of a fixed size and wants no gathering of rows on the way (#294).
#[inline(always)]
pub(crate) fn first_stage(origin: &[f64], points: &[&[f64]], lifted: bool) -> Option<Sign> {
    semi_static::sign(origin, points, lifted)
}

/// The sign of the planar orientation of `a`, `b`, `c`, when Shewchuk's
/// stage-A bound certifies it. `None` means the caller must use [`orient`].
///
/// The determinant is `(a - c) × (b - c)`. When the two products have
/// opposite signs there is no cancellation, and the sign is immediate.
/// Otherwise it is certified only when it exceeds
/// `(3 + 16 ε) ε (|left| + |right|)`.
#[inline(always)]
pub(crate) fn orient2_filter(a: &[f64], b: &[f64], c: &[f64]) -> Option<Sign> {
    debug_assert!(a.len() >= 2 && b.len() >= 2 && c.len() >= 2);
    let det_left = (a[0] - c[0]) * (b[1] - c[1]);
    let det_right = (a[1] - c[1]) * (b[0] - c[0]);
    let det = det_left - det_right;
    let det_sum = if det_left > 0.0 {
        if det_right <= 0.0 {
            return Some(Sign::Positive);
        }
        det_left + det_right
    } else if det_left < 0.0 {
        if det_right >= 0.0 {
            return Some(Sign::Negative);
        }
        -det_left - det_right
    } else {
        return None;
    };
    let err = (3.0 + 16.0 * f64::EPSILON) * f64::EPSILON * det_sum;
    if det > err {
        Some(Sign::Positive)
    } else if -det > err {
        Some(Sign::Negative)
    } else {
        None
    }
}

/// The filtered lifted height `|p|^2` of one site, with its error bound
/// (design §7). A cache of these is bit for bit what the filter computed from
/// the coordinates on every call, so caching changes no sign. The exact stage
/// never reads it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LiftedHeight(Approx);

impl LiftedHeight {
    /// The height of site `p`: the sum of its squares, left to right, with
    /// the running bound of [`filter`]. A non-finite value leaves every
    /// filter that reads it uncertified.
    pub(crate) fn of(p: &[f64]) -> Self {
        Self(p.iter().fold(Approx::exact(0.0), |sum, &x| {
            sum.add(Approx::exact(x).mul(Approx::exact(x)))
        }))
    }

    /// The rounded height.
    pub(crate) fn value(self) -> f64 {
        self.0.value()
    }

    /// The error bound of [`Self::value`].
    pub(crate) fn error(self) -> f64 {
        self.0.error()
    }

    /// A height stored as its [`Self::value`] and [`Self::error`], read back
    /// from a cache. Debug builds check every height a predicate reads
    /// against [`Self::of`] its point.
    pub(crate) fn stored(value: f64, error: f64) -> Self {
        Self(Approx::stored(value, error))
    }

    /// The value and bound as bits, to check a cache against its source.
    fn bits(self) -> (u64, u64) {
        (self.0.value().to_bits(), self.0.error().to_bits())
    }
}

/// [`orient_lifted_with`] evaluating the heights itself.
#[cfg(test)]
pub(crate) fn orient_lifted(points: &[&[f64]]) -> Result<Sign, ExactEvaluationExhausted> {
    let heights: Vec<LiftedHeight> = points.iter().map(|p| LiftedHeight::of(p)).collect();
    orient_lifted_with(points, &heights)
}

/// Orientation of `points` lifted to the paraboloid (design §7): k + 2
/// points, each of dimension k >= 1, standing for `(p, |p|^2)` in dimension
/// k + 1. Cospherical means [`Sign::Zero`].
///
/// `heights[i]` is [`LiftedHeight::of`] `points[i]`; the filter reads it in
/// place of the squares. The exact stage takes the lifted coordinate as the
/// polynomial `sum_i p_i^2` of the input coordinates, never a rounded `f64`;
/// an intermediate that overflows only sends the evaluation to the exact
/// sign.
pub(crate) fn orient_lifted_with(
    points: &[&[f64]],
    heights: &[LiftedHeight],
) -> Result<Sign, ExactEvaluationExhausted> {
    orient_lifted_from(points, heights, Start::FirstStage)
}

/// [`orient_lifted_with`], starting at `start`.
pub(crate) fn orient_lifted_from(
    points: &[&[f64]],
    heights: &[LiftedHeight],
    start: Start,
) -> Result<Sign, ExactEvaluationExhausted> {
    debug_assert!(points.len() >= 3, "a lifted orientation needs k + 2 points");
    debug_assert_eq!(heights.len(), points.len(), "one height per point");
    sign_of(
        Rows {
            origin: points[0],
            points: &points[1..],
            direction: None,
            lifted: Some(heights),
        },
        start,
    )
}

/// Side of `query` relative to the hyperplane through `facet` (k points of
/// dimension k, in outward order): [`Sign::Positive`] is outside.
#[cfg(test)]
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
#[cfg(test)]
pub(crate) fn is_coplanar(points: &[&[f64]]) -> Result<bool, ExactEvaluationExhausted> {
    Ok(orient(points)? == Sign::Zero)
}

/// Orientation of a direction against a hyperplane: the exact sign of the
/// determinant of `facet[i] - facet[0]`, `i = 1..k`, followed by the row
/// `direction` as it is. `facet` holds k points of dimension k.
///
/// A direction pointing to the side where [`orient`] of `facet` followed by
/// a point is positive has a positive sign here.
pub(crate) fn orient_direction(
    facet: &[&[f64]],
    direction: &[f64],
) -> Result<Sign, ExactEvaluationExhausted> {
    debug_assert!(!facet.is_empty(), "a hyperplane needs at least one point");
    sign_of(
        Rows {
            origin: facet[0],
            points: &facet[1..],
            direction: Some(direction),
            lifted: None,
        },
        Start::FirstStage,
    )
}

/// The rows of an orientation determinant: `p - origin` for each point,
/// then, when present, a direction row taken as it is.
///
/// When `lifted`, each point `p` of dimension D stands for the point of
/// dimension D + 1 whose last coordinate is the polynomial `sum_i p_i^2`
/// (design §7). The array is not extended: the last column of a difference
/// row is `|p|^2 - |origin|^2`, evaluated by the filter from the heights
/// (`origin` first, then `points` in order) or exactly from the coordinates.
#[derive(Clone, Copy)]
struct Rows<'a> {
    origin: &'a [f64],
    points: &'a [&'a [f64]],
    direction: Option<&'a [f64]>,
    lifted: Option<&'a [LiftedHeight]>,
}

enum Row<'a> {
    Difference(&'a [f64]),
    Direction(&'a [f64]),
}

impl<'a> Rows<'a> {
    /// Size of the square determinant.
    fn k(self) -> usize {
        self.origin.len() + usize::from(self.lifted.is_some())
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

fn sign_of(rows: Rows<'_>, start: Start) -> Result<Sign, ExactEvaluationExhausted> {
    let k = rows.k();
    debug_assert!(k >= 1, "orientation needs dimension at least 1");
    debug_assert_eq!(
        rows.points.len() + usize::from(rows.direction.is_some()),
        k,
        "a determinant of size k needs k rows"
    );
    debug_assert!(
        rows.points.iter().all(|p| p.len() == rows.origin.len())
            && rows.direction.is_none_or(|d| d.len() == k),
        "every row needs dimension k"
    );
    debug_assert!(
        rows.values().all(f64::is_finite),
        "predicate input must be finite"
    );
    debug_assert!(
        rows.lifted
            .is_none_or(|heights| heights.len() == rows.points.len() + 1
                && core::iter::once(rows.origin)
                    .chain(rows.points.iter().copied())
                    .zip(heights)
                    .all(|(p, &h)| h.bits() == LiftedHeight::of(p).bits())),
        "a cached height must be the height of its point"
    );
    if let Some(sign) = filtered(rows, start) {
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

/// The largest size the first stage covers ([`semi_static`]).
const FIRST_STAGE_UP_TO: usize = 5;

/// The largest size the double-double stage covers ([`double_double`]).
const DOUBLE_DOUBLE_UP_TO: usize = 6;

/// The sign when a stage before the exact one certifies it.
fn filtered(rows: Rows<'_>, start: Start) -> Option<Sign> {
    let k = rows.k();
    debug_assert!(
        start == Start::FirstStage || (rows.direction.is_none() && k <= FIRST_STAGE_UP_TO),
        "only a size of the first stage starts after it"
    );
    if k == 1 {
        // Degree 1: the sign of b - a is the order of two finite values.
        return match rows.row(0) {
            Row::Difference(p) => p[0].partial_cmp(&rows.origin[0]).map(from_ordering),
            Row::Direction(d) => d[0].partial_cmp(&0.0).map(from_ordering),
        };
    }
    if rows.direction.is_none() && k <= DOUBLE_DOUBLE_UP_TO {
        let lifted = rows.lifted.is_some();
        if k <= FIRST_STAGE_UP_TO {
            if start == Start::FirstStage {
                if let Some(sign) = semi_static::sign(rows.origin, rows.points, lifted) {
                    debug_assert_eq!(
                        Ok(sign),
                        exact::sign_exact(rows),
                        "the semi-static bound certified the wrong sign"
                    );
                    return Some(sign);
                }
            } else {
                debug_assert!(
                    semi_static::sign(rows.origin, rows.points, lifted).is_none(),
                    "the caller's first stage left the sign open"
                );
            }
        }
        if has_zero_column(rows) {
            debug_assert_eq!(exact::sign_exact(rows), Ok(Sign::Zero), "a zero column");
            return Some(Sign::Zero);
        }
        if k > FIRST_STAGE_UP_TO {
            if let Some(sign) = filtered_value(rows).and_then(|v| v.certified_sign()) {
                return Some(sign);
            }
        }
        let sign = double_double::sign(rows.origin, rows.points, lifted)?;
        debug_assert_eq!(
            Ok(sign),
            exact::sign_exact(rows),
            "the double-double bound certified the wrong sign"
        );
        return Some(sign);
    }
    if has_zero_column(rows) {
        debug_assert_eq!(exact::sign_exact(rows), Ok(Sign::Zero), "a zero column");
        return Some(Sign::Zero);
    }
    filtered_value(rows)?.certified_sign()
}

/// Whether a coordinate column of the matrix is zero: every point has the
/// origin's coordinate there, and a direction row, if any, is zero there.
/// The determinant is then exactly zero. `a - b` is zero in `f64` exactly
/// when `a == b` (`-0.0` and `+0.0` included), so the test is exact. On
/// inputs on hyperplanes parallel to the axes, most zero signs are of this
/// kind, and no bound can certify a zero (#368).
fn has_zero_column(rows: Rows<'_>) -> bool {
    (0..rows.origin.len()).any(|c| {
        rows.points.iter().all(|p| p[c] == rows.origin[c])
            && rows.direction.is_none_or(|d| d[c] == 0.0)
    })
}

/// The filtered determinant with its error bound, for k >= 2.
fn filtered_value(rows: Rows<'_>) -> Option<Approx> {
    let k = rows.k();
    let origin = rows.origin;
    let heights = rows.lifted.unwrap_or_default();
    let entry = |i: usize, j: usize| match rows.row(i) {
        // The lifted column; a non-finite height leaves the filter
        // uncertified, and the exact path decides (design §7).
        Row::Difference(_) if j == origin.len() => heights[i + 1].0.sub(heights[0].0),
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
        _ => filter::determinant(k, entry)?,
    })
}

/// The cofactor vector `c` of the hyperplane through `facet` (k points of
/// dimension k): `c_j` is the determinant of the edges `facet[i] - facet[0]`
/// followed by the unit row `e_j`, so `orient_direction(facet, v)` is the sign
/// of `c . v`. Each entry is returned as `(value, bound)` with
/// `|c_j - value| <= bound`.
///
/// Facets of two to four points take [`filter::small_cofactors`] when their
/// edges allow it, otherwise one filtered determinant per cofactor; larger
/// facets share one filtered elimination.
///
/// Returns `None` when a bound is not finite or a filtered elimination could
/// not certify a pivot.
pub(crate) fn direction_cofactors(facet: &[&[f64]]) -> Option<Cofactors> {
    let k = facet.len();
    debug_assert!(k >= 1, "a hyperplane needs at least one point");
    if k == 1 {
        return Some([(1.0, 0.0)].as_slice().into());
    }
    if k > 4 {
        // One elimination shared by every cofactor (#111).
        let entry =
            |i: usize, j: usize| Approx::exact(facet[i + 1][j]).sub(Approx::exact(facet[0][j]));
        return finite_cofactors(filter::cofactors(k, entry)?);
    }
    // Shared minors and one bound per cofactor from a constant (#174).
    if let Some(cofactors) = filter::small_cofactors(facet) {
        return Some(cofactors[..k].into());
    }
    let mut unit = [0.0; 4];
    let unit = &mut unit[..k];
    let mut cofactors = [(0.0, 0.0); 4];
    for (j, cofactor) in cofactors.iter_mut().enumerate().take(k) {
        unit[j] = 1.0;
        let value = filtered_value(Rows {
            origin: facet[0],
            points: &facet[1..],
            direction: Some(unit),
            lifted: None,
        });
        unit[j] = 0.0;
        let value = value?;
        if !value.value().is_finite() || !value.error().is_finite() {
            return None;
        }
        *cofactor = (value.value(), value.error());
    }
    Some(cofactors[..k].into())
}

/// Facets per call of [`scaled_direction_cofactors_in_lanes`].
pub(crate) const COFACTOR_LANES: usize = filter::FACET_LANES;

/// [`direction_cofactors`] of `COFACTOR_LANES` facets of the same size
/// `k > 4`, facet `lane` with every coordinate multiplied by
/// `factors(largest)[lane]`, where `largest[lane]` is that facet's largest
/// coordinate magnitude. Result `lane` is bit for bit
/// [`direction_cofactors`] of that facet scaled, when every product is
/// exact.
///
/// Returns `None` when the lanes do not take the facets (no AVX2, or `k`
/// above 9) or `factors` returns `None`. The coordinates must not be NaN.
pub(crate) fn scaled_direction_cofactors_in_lanes(
    facets: [&[&[f64]]; COFACTOR_LANES],
    factors: impl Fn([f64; COFACTOR_LANES]) -> Option<[f64; COFACTOR_LANES]>,
) -> Option<[Option<Cofactors>; COFACTOR_LANES]> {
    debug_assert!(
        facets
            .iter()
            .all(|f| f.len() == facets[0].len() && f.len() > 4),
        "one size above four per call"
    );
    filter::edge_cofactors_in_lanes(facets, factors)
}

/// The filtered cofactors, when every value and bound is finite.
fn finite_cofactors(values: Small<Approx, 10>) -> Option<Cofactors> {
    values
        .into_iter()
        .map(|c| (c.value().is_finite() && c.error().is_finite()).then(|| (c.value(), c.error())))
        .collect()
}

/// The cofactors of one hyperplane, inline up to ten points (#143).
pub(crate) type Cofactors = Small<(f64, f64), 10>;

/// A unit direction of one hyperplane, inline up to dimension ten (#143).
pub(crate) type Direction = Small<f64, 10>;

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;

/// The largest error bound at which a filtered cofactor direction is used.
const FILTERED_DIRECTION_LIMIT: f64 = 1e-10;

/// [`cofactor_direction_from`] evaluating the cofactors itself.
#[cfg(test)]
pub(crate) fn cofactor_direction(
    facet: &[&[f64]],
) -> Result<Option<(Direction, f64)>, ExactEvaluationExhausted> {
    cofactor_direction_from(facet, direction_cofactors(facet).as_deref())
}

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
///
/// `cofactors` are the filtered cofactors of `facet` already evaluated by
/// [`direction_cofactors`], so a caller that needs them twice evaluates them
/// once (#86).
pub(crate) fn cofactor_direction_from(
    facet: &[&[f64]],
    cofactors: Option<&[(f64, f64)]>,
) -> Result<Option<(Direction, f64)>, ExactEvaluationExhausted> {
    if let Some(certified) = certified_cofactor_direction(facet.len(), cofactors) {
        return Ok(Some(certified));
    }
    let err = facet.len() as f64 * 2f64.powi(-49);
    Ok(exact::cofactor_direction_exact(facet)?.map(|d| (d.into(), err)))
}

/// The filtered half of [`cofactor_direction_from`]: the unit direction of
/// `cofactors` (of k points) with its certified error, or `None` when the
/// cofactors are missing or their certificate is looser than the limit at
/// which the exact direction is used instead.
pub(crate) fn certified_cofactor_direction(
    k: usize,
    cofactors: Option<&[(f64, f64)]>,
) -> Option<(Direction, f64)> {
    let k = k as f64;
    let cofactors = cofactors?;
    let bound: f64 = cofactors.iter().map(|&(_, e)| e).sum::<f64>() * (1.0 + k * UNIT_ROUNDOFF);
    let length = cofactors.iter().map(|&(c, _)| c * c).sum::<f64>().sqrt();
    let length_low = length * (1.0 - (k + 3.0) * UNIT_ROUNDOFF);
    if !(length_low.is_finite() && length_low > 0.0) {
        return None;
    }
    // |c/|c| - c^/|c^|| <= 2|c - c^| / |c^|, plus the rounding of the
    // normalization.
    let err =
        (2.0 * bound / length_low + 4.0 * (k + 4.0) * UNIT_ROUNDOFF) * (1.0 + 8.0 * UNIT_ROUNDOFF);
    // A loose certificate is not a useful reference; the exact cofactors are.
    (err <= FILTERED_DIRECTION_LIMIT)
        .then(|| (cofactors.iter().map(|&(c, _)| c / length).collect(), err))
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
            lifted: None,
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

    /// The elimination over a matrix of row vectors of `Approx`, entry by
    /// entry: the reference every size must match bit for bit.
    fn determinant_of_rows(mut m: Vec<Vec<Approx>>) -> Option<Approx> {
        let n = m.len();
        let mut det = Approx::exact(1.0);
        for col in 0..n {
            let best = (col..n)
                .max_by(|&a, &b| m[a][col].value().abs().total_cmp(&m[b][col].value().abs()))?;
            if best != col {
                m.swap(best, col);
                det = det.negated();
            }
            let pivot = m[col][col];
            det = det.mul(pivot);
            let (upper, lower) = m.split_at_mut(col + 1);
            let pivot_row = &upper[col];
            for row in lower {
                let factor = row[col].div(pivot)?;
                for (entry, &above) in row[col + 1..].iter_mut().zip(&pivot_row[col + 1..]) {
                    *entry = entry.sub(factor.mul(above));
                }
            }
        }
        Some(det)
    }

    #[test]
    fn determinant_matches_the_row_elimination_bit_for_bit() {
        // Sizes on the stack (5..=9) and in row vectors (10..=18); magnitudes
        // far from 1; a last row of exact entries near the subnormal range,
        // which pivoting keeps for last, so its rounding terms are subnormal,
        // eta shows in the bits (an input bound would hide it), and the
        // product of the pivots stays normal; two rows tied
        // in the first pivot column; repeated rows and columns, so some pivots
        // are not certain and both sides must return None.
        let mut rng = Rng(72);
        let mut uncertain = 0;
        for n in 5..=18 {
            for trial in 0..40 {
                let scale = [1.0, 1e-120, 1e120, 3.0][trial % 4];
                let mut m: Vec<Vec<f64>> = (0..n)
                    .map(|_| (0..n).map(|_| rng.unit() * scale).collect())
                    .collect();
                if trial % 3 == 0 {
                    m[1][0] = -m[0][0];
                }
                if trial % 8 == 4 {
                    m[n - 1].iter_mut().for_each(|x| *x *= 1e-300);
                }
                if trial % 5 == 0 {
                    m[n - 1] = m[0].clone();
                }
                if trial % 7 == 0 {
                    // Equal first two columns: after the first step the
                    // second pivot is not certain, and rows below divide by it.
                    for row in &mut m {
                        row[1] = row[0];
                    }
                }
                let entry = |i: usize, j: usize| input_entry(&m, trial, i, j);
                let reference = determinant_of_rows(
                    (0..n)
                        .map(|i| (0..n).map(|j| entry(i, j)).collect())
                        .collect(),
                );
                let stored = filter::determinant(n, entry);
                let bits =
                    |a: Option<Approx>| a.map(|a| (a.value().to_bits(), a.error().to_bits()));
                assert_eq!(bits(stored), bits(reference), "n = {n}, trial {trial}");
                uncertain += usize::from(reference.is_none());
            }
        }
        assert!(uncertain > 0, "some pivot is uncertain");
    }

    /// Whether [`filter::edge_cofactors_in_lanes`] has vectors here.
    fn lanes_on_this_cpu() -> bool {
        #[cfg(target_arch = "x86_64")]
        return pulp::x86::V3::try_new().is_some();
        #[cfg(not(target_arch = "x86_64"))]
        return false;
    }

    /// Entry `(i, j)` of a test matrix; odd trials carry an input bound, so
    /// every term of the running bound is nonzero.
    fn input_entry(m: &[Vec<f64>], trial: usize, i: usize, j: usize) -> Approx {
        let x = m[i][j];
        if trial % 2 == 1 {
            Approx::stored(x, x.abs() * 2f64.powi(-50) + f64::MIN_POSITIVE)
        } else {
            Approx::exact(x)
        }
    }

    /// The cofactor elimination over row vectors of `Approx`, entry by
    /// entry: the reference every size must match bit for bit.
    fn cofactors_of_rows(mut rows: Vec<Vec<Approx>>) -> Option<Vec<Approx>> {
        let m = rows.len();
        let k = m + 1;
        let mut swapped = false;
        for col in 0..m {
            let best = (col..m).max_by(|&a, &b| {
                rows[a][col]
                    .value()
                    .abs()
                    .total_cmp(&rows[b][col].value().abs())
            })?;
            if best != col {
                rows.swap(best, col);
                swapped = !swapped;
            }
            let (upper, lower) = rows.split_at_mut(col + 1);
            let pivot_row = &upper[col];
            let pivot = pivot_row[col];
            for row in lower {
                let factor = row[col].div(pivot)?;
                for (entry, &above) in row[col + 1..k].iter_mut().zip(&pivot_row[col + 1..k]) {
                    *entry = entry.sub(factor.mul(above));
                }
            }
        }
        let mut det = Approx::exact(1.0);
        for (i, row) in rows.iter().enumerate() {
            det = det.mul(row[i]);
        }
        if swapped {
            det = det.negated();
        }
        let mut out = vec![Approx::exact(0.0); k];
        for i in (0..m).rev() {
            let row = &rows[i];
            let mut sum = row[m];
            for (j, &xj) in out.iter().enumerate().take(m).skip(i + 1) {
                sum = sum.sub(row[j].mul(xj));
            }
            out[i] = sum.div(row[i])?;
        }
        for value in out.iter_mut().take(m) {
            *value = det.mul(*value).negated();
        }
        out[m] = det;
        Some(out)
    }

    #[test]
    fn cofactors_match_the_row_elimination_bit_for_bit() {
        // Orders on the stack (5..=9) and on the heap (10..=18), with the
        // same magnitudes, ties, and repeated rows and columns as the
        // determinant.
        // The same matrices, four trials at a time, also go through the
        // lanes as facet edges: each lane must equal the reference of its
        // own facet, and none where a value or bound is not finite.
        let mut rng = Rng(73);
        let mut uncertain = 0;
        let mut mixed = 0;
        let mut not_finite = 0;
        let lanes_available = lanes_on_this_cpu();
        let bits = |a: Option<Vec<Approx>>| {
            a.map(|a| {
                a.iter()
                    .map(|a| (a.value().to_bits(), a.error().to_bits()))
                    .collect::<Vec<_>>()
            })
        };
        for k in 5..=18 {
            let mut cases = Vec::new();
            for trial in 0..40 {
                let scale = [1.0, 1e-120, 1e120, 3.0][trial % 4];
                let mut m: Vec<Vec<f64>> = (0..k - 1)
                    .map(|_| (0..k).map(|_| rng.unit() * scale).collect())
                    .collect();
                if trial % 3 == 0 {
                    m[1][0] = -m[0][0];
                }
                if trial % 8 == 4 {
                    m[k - 2].iter_mut().for_each(|x| *x *= 1e-300);
                }
                if trial % 5 == 0 {
                    m[k - 2] = m[0].clone();
                }
                // Dependent up to rounding: the last pivot is a small
                // residue whose sign is not certain, and only the back
                // substitution divides by it.
                if trial % 6 == 2 {
                    m[k - 2] = (0..k).map(|j| 0.3 * m[0][j] + 0.7 * m[1][j]).collect();
                }
                if trial % 7 == 0 {
                    for row in &mut m {
                        row[1] = row[0];
                    }
                }
                let entry = |i: usize, j: usize| input_entry(&m, trial, i, j);
                let reference = cofactors_of_rows(
                    (0..k - 1)
                        .map(|i| (0..k).map(|j| entry(i, j)).collect())
                        .collect(),
                );
                let stored = filter::cofactors(k, entry);
                uncertain += usize::from(reference.is_none());
                let reference = bits(reference);
                assert_eq!(
                    bits(stored.map(|s| s.to_vec())),
                    reference,
                    "k = {k}, trial {trial}"
                );
                cases.push(m);
            }
            // The rows as the edges of a facet from the origin, unscaled;
            // the 1e120 rows overflow, so some lanes are not finite.
            for (group, lanes) in cases.chunks_exact(filter::FACET_LANES).enumerate() {
                let facets: Vec<Vec<Vec<f64>>> = lanes
                    .iter()
                    .map(|m| {
                        core::iter::once(vec![0.0; k])
                            .chain(m.iter().cloned())
                            .collect()
                    })
                    .collect();
                let refs: Vec<Vec<&[f64]>> = facets
                    .iter()
                    .map(|f| f.iter().map(Vec::as_slice).collect())
                    .collect();
                let four: [&[&[f64]]; filter::FACET_LANES] = core::array::from_fn(|l| &*refs[l]);
                let mut expected = Vec::new();
                for f in four {
                    let rows = (0..k - 1)
                        .map(|i| {
                            (0..k)
                                .map(|j| Approx::exact(f[i + 1][j]).sub(Approx::exact(f[0][j])))
                                .collect()
                        })
                        .collect();
                    let reference = bits(cofactors_of_rows(rows));
                    let finite = reference.clone().filter(|c| {
                        c.iter().all(|&(v, e)| {
                            f64::from_bits(v).is_finite() && f64::from_bits(e).is_finite()
                        })
                    });
                    not_finite += usize::from(reference.is_some() && finite.is_none());
                    expected.push(finite);
                }
                let got =
                    filter::edge_cofactors_in_lanes(four, |_| Some([1.0; filter::FACET_LANES]));
                if !(lanes_available && (5..=9).contains(&k)) {
                    assert!(got.is_none(), "k = {k}: no lanes");
                    continue;
                }
                let got = got.expect("the lanes take k = 5 to 9");
                let failed = expected.iter().filter(|e| e.is_none()).count();
                mixed += usize::from(failed > 0 && failed < filter::FACET_LANES);
                for (lane, values) in got.into_iter().enumerate() {
                    let values = values.map(|c| {
                        c.iter()
                            .map(|&(v, e)| (v.to_bits(), e.to_bits()))
                            .collect::<Vec<_>>()
                    });
                    assert_eq!(
                        values, expected[lane],
                        "k = {k}, group {group}, lane {lane}"
                    );
                }
            }
        }
        assert!(uncertain > 0, "some divisor is uncertain");
        assert!(
            !lanes_available || (mixed > 0 && not_finite > 0),
            "some group has a lane that fails beside one that does not, \
             and some lane fails only on a bound that is not finite"
        );
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
                    filtered(
                        Rows {
                            origin: refs[0],
                            points: &refs[1..],
                            direction: None,
                            lifted: None,
                        },
                        Start::FirstStage,
                    )
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

    // Lifted orientation (design §7).

    fn lifted_of(points: &[Vec<f64>]) -> Sign {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        orient_lifted(&refs).unwrap()
    }

    /// Independent reference: the lifted determinant of integer points in
    /// `i128`, by cofactor expansion.
    fn lifted_reference(points: &[Vec<i64>]) -> Sign {
        fn det(m: &[Vec<i128>]) -> i128 {
            if m.len() == 1 {
                return m[0][0];
            }
            (0..m.len())
                .map(|j| {
                    let minor: Vec<Vec<i128>> = m[1..]
                        .iter()
                        .map(|r| {
                            r.iter()
                                .enumerate()
                                .filter(|&(c, _)| c != j)
                                .map(|(_, &x)| x)
                                .collect()
                        })
                        .collect();
                    let t = m[0][j] * det(&minor);
                    if j.is_multiple_of(2) {
                        t
                    } else {
                        -t
                    }
                })
                .sum()
        }
        let norm = |p: &[i64]| {
            p.iter()
                .map(|&x| i128::from(x) * i128::from(x))
                .sum::<i128>()
        };
        let o = &points[0];
        let rows: Vec<Vec<i128>> = points[1..]
            .iter()
            .map(|p| {
                p.iter()
                    .zip(o)
                    .map(|(&x, &y)| i128::from(x - y))
                    .chain(core::iter::once(norm(p) - norm(o)))
                    .collect()
            })
            .collect();
        match det(&rows).cmp(&0) {
            core::cmp::Ordering::Less => Sign::Negative,
            core::cmp::Ordering::Equal => Sign::Zero,
            core::cmp::Ordering::Greater => Sign::Positive,
        }
    }

    /// Whether the exact cofactor `j` of `facet` lies in `[value - bound,
    /// value + bound]`, compared exactly: every number is scaled to an
    /// integer by 2^1074 per row.
    fn cofactor_within(facet: &[Vec<f64>], j: usize, value: f64, bound: f64) -> bool {
        use exact::BigInt;
        let k = facet.len();
        let int = |x: f64| BigInt::from_f64_scaled(x, 0).unwrap();
        let mut rows: Vec<Vec<BigInt>> = facet[1..]
            .iter()
            .map(|p| {
                p.iter()
                    .zip(&facet[0])
                    .map(|(&x, &o)| int(x).sub(&int(o)).unwrap())
                    .collect()
            })
            .collect();
        rows.push(
            (0..k)
                .map(|m| int(if m == j { 1.0 } else { 0.0 }))
                .collect(),
        );
        let exact = exact::determinant(&rows).unwrap();
        // value and bound times 2^(1074 k): one factor from the conversion,
        // k - 1 more from the scale.
        let mut scale = int(1.0);
        for _ in 1..k - 1 {
            scale = scale.mul(&int(1.0)).unwrap();
        }
        let high = int(value).add(&int(bound)).unwrap().mul(&scale).unwrap();
        let low = int(value).sub(&int(bound)).unwrap().mul(&scale).unwrap();
        exact.sub(&high).unwrap().sign() != Sign::Positive
            && exact.sub(&low).unwrap().sign() != Sign::Negative
    }

    #[test]
    fn shared_elimination_bounds_every_cofactor() {
        // k = 5..9 points of dimension k, at three scales, in general
        // position and with the last point a hair off the affine span of
        // the others, where the cofactors cancel and the bounds matter.
        // Named exception: at 2^300 every cofactor of k >= 5 points is a
        // product of at least four entries near 2^300, beyond the f64 range,
        // so no set is certified and the direction falls back to the exact
        // stage. Every other case is certified, and every cofactor lies in
        // its interval.
        let mut rng = Rng(111);
        let mut tight = 0;
        for k in 5..=9 {
            for scale in [1.0, 2f64.powi(-300), 2f64.powi(300)] {
                for near in [false, true] {
                    for _ in 0..6 {
                        let mut facet: Vec<Vec<f64>> = (0..k)
                            .map(|_| (0..k).map(|_| rng.unit() * scale).collect())
                            .collect();
                        if near {
                            let others = k - 1;
                            let last: Vec<f64> = (0..k)
                                .map(|m| {
                                    let mean: f64 =
                                        facet[..others].iter().map(|p| p[m]).sum::<f64>()
                                            / others as f64;
                                    mean + rng.unit() * scale * 1e-9
                                })
                                .collect();
                            facet[k - 1] = last;
                        }
                        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
                        let result = direction_cofactors(&refs);
                        if scale > 1.0 {
                            assert!(
                                result.is_none(),
                                "k {k}, 2^300, near {near}: certified past f64 range"
                            );
                            continue;
                        }
                        let Some(cofactors) = result else {
                            panic!("k {k}, scale {scale:e}, near {near}: not certified");
                        };
                        assert_eq!(cofactors.len(), k, "one cofactor per column");
                        for (j, &(value, bound)) in cofactors.iter().enumerate() {
                            assert!(
                                cofactor_within(&facet, j, value, bound),
                                "k {k}, scale {scale:e}, near {near}, cofactor {j}: {value:e} +- {bound:e}"
                            );
                            if bound > value.abs() * 1e-12 {
                                tight += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(tight > 0, "no case where the bound is far above rounding");
    }

    #[test]
    fn small_facet_bounds_every_cofactor() {
        // k = 2..4 points of dimension k (#174): in general position, with
        // the last point a hair off the affine span of the others, where
        // the cofactors cancel and the bound is far above their rounding,
        // with integer coordinates, where a cofactor can be exactly zero,
        // and with spread exponents, where the edges round. At 1 and at 2^-200 every edge lies inside [2^-250, 2^250],
        // so the shared minors with the constant bound take them; at 2^-300
        // and 2^300 the edges lie outside, so the running bound does.
        // Every cofactor lies in its interval either way.
        let mut rng = Rng(174);
        let (mut small, mut fallback, mut tight, mut zero) = (0, 0, 0, 0);
        for k in 2..=4 {
            for scale in [1.0, 2f64.powi(-200), 2f64.powi(-300), 2f64.powi(300)] {
                for shape in 0..4 {
                    for _ in 0..40 {
                        let mut facet: Vec<Vec<f64>> = (0..k)
                            .map(|_| {
                                (0..k)
                                    .map(|_| match shape {
                                        2 => (rng.next() % 5) as f64 - 2.0,
                                        // Exponents spread over 2^-40..1, so
                                        // the edges themselves round.
                                        3 => rng.unit() * 2f64.powi(-((rng.next() % 41) as i32)),
                                        _ => rng.unit(),
                                    })
                                    .collect()
                            })
                            .collect();
                        if shape == 1 {
                            let others = k - 1;
                            for m in 0..k {
                                let mean: f64 = facet[..others].iter().map(|p| p[m]).sum::<f64>()
                                    / others as f64;
                                facet[k - 1][m] = mean + rng.unit() * 1e-9;
                            }
                        }
                        for p in &mut facet {
                            for x in p.iter_mut() {
                                *x *= scale;
                            }
                        }
                        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
                        // Coincident integer points have zero edges, which
                        // the small path takes at every scale.
                        let low = 2f64.powi(-250)..=2f64.powi(250);
                        let inside = facet[1..].iter().all(|p| {
                            p.iter()
                                .zip(&facet[0])
                                .all(|(x, o)| x == o || low.contains(&(x - o).abs()))
                        });
                        assert!(
                            inside == (scale == 1.0 || scale == 2f64.powi(-200))
                                || facet.iter().all(|p| *p == facet[0])
                                || shape == 2,
                            "k {k}, scale {scale:e}: an edge outside the scale's range"
                        );
                        let direct = filter::small_cofactors(&refs);
                        assert_eq!(direct.is_some(), inside, "k {k}, scale {scale:e}");
                        let Some(cofactors) = direction_cofactors(&refs) else {
                            panic!("k {k}, scale {scale:e}, shape {shape}: not certified");
                        };
                        if let Some(direct) = direct {
                            assert_eq!(&cofactors[..], &direct[..k], "the small path is used");
                            small += 1;
                        } else {
                            fallback += 1;
                        }
                        assert_eq!(cofactors.len(), k, "one cofactor per column");
                        for (j, &(value, bound)) in cofactors.iter().enumerate() {
                            assert!(
                                cofactor_within(&facet, j, value, bound),
                                "k {k}, scale {scale:e}, shape {shape}, cofactor {j}: \
                                 {value:e} +- {bound:e}"
                            );
                            tight += usize::from(bound > value.abs() * 1e-12);
                            zero += usize::from(value == 0.0 && bound == 0.0);
                        }
                    }
                }
            }
        }
        assert!(
            small > 0 && fallback > 0,
            "small {small}, fallback {fallback}"
        );
        assert!(tight > 0 && zero > 0, "tight {tight}, exact zero {zero}");
    }

    #[test]
    fn lifted_matches_an_independent_reference() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 9) as i64 - 4
        };
        let mut zeros = 0;
        for k in 2..=3 {
            for _ in 0..400 {
                let points: Vec<Vec<i64>> = (0..k + 2)
                    .map(|_| (0..k).map(|_| next()).collect())
                    .collect();
                let as_f64: Vec<Vec<f64>> = points
                    .iter()
                    .map(|p| p.iter().map(|&x| x as f64).collect())
                    .collect();
                let expected = lifted_reference(&points);
                zeros += usize::from(expected == Sign::Zero);
                assert_eq!(lifted_of(&as_f64), expected, "{points:?}");
            }
        }
        assert!(zeros > 0, "the small grid gives exact cospherical cases");
    }

    #[test]
    fn lifted_cocircular_and_inside() {
        // (0,0), (1,0), (0,1) counterclockwise; (1,1) is on their circle,
        // (0.5, 0.5) inside it, (2, 2) outside.
        let base = vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]];
        let with = |q: [f64; 2]| {
            let mut p = base.clone();
            p.push(q.to_vec());
            lifted_of(&p)
        };
        assert_eq!(with([1.0, 1.0]), Sign::Zero);
        assert_eq!(with([0.5, 0.5]), Sign::Negative);
        assert_eq!(with([2.0, 2.0]), Sign::Positive);
    }

    #[test]
    fn lifted_survives_overflowing_and_underflowing_squares() {
        // Scaling by 2^e multiplies the coordinate columns by 2^e and the
        // lifted column by 2^2e, so the sign is unchanged; at 2^600 every
        // square overflows f64, at 2^-600 every square underflows.
        let cases: [([f64; 2], Sign); 3] = [
            ([1.0, 1.0], Sign::Zero),
            ([0.5, 0.5], Sign::Negative),
            ([2.0, 2.0], Sign::Positive),
        ];
        for e in [600, -600, 1000, -1000] {
            let s = 2f64.powi(e);
            for (q, expected) in cases {
                let points = vec![
                    vec![0.0, 0.0],
                    vec![s, 0.0],
                    vec![0.0, s],
                    vec![q[0] * s, q[1] * s],
                ];
                assert_eq!(lifted_of(&points), expected, "scale 2^{e}, {q:?}");
            }
        }
    }

    #[test]
    fn lifted_translation_keeps_the_sign() {
        // |p + t|^2 - |o + t|^2 = |p|^2 - |o|^2 + 2 t . (p - o): a column
        // operation, so the determinant is unchanged. Integer translations
        // below 2^26 keep every coordinate exact.
        let points = [
            vec![0.0, 0.0],
            vec![3.0, 0.0],
            vec![0.0, 3.0],
            vec![3.0, 3.0],
        ];
        let near = [
            vec![0.0, 0.0],
            vec![3.0, 0.0],
            vec![0.0, 3.0],
            vec![3.0, 2.0],
        ];
        for t in [1.0, 1024.0, -(2f64.powi(25))] {
            let moved = |ps: &[Vec<f64>]| -> Vec<Vec<f64>> {
                ps.iter()
                    .map(|p| p.iter().map(|x| x + t).collect())
                    .collect()
            };
            assert_eq!(lifted_of(&moved(&points)), Sign::Zero, "t = {t}");
            assert_eq!(lifted_of(&moved(&near)), lifted_of(&near), "t = {t}");
        }
        assert_eq!(lifted_of(&near), Sign::Negative);
    }

    /// The exact sign of `points` (k + 1 rows of dimension k, or k + 2 when
    /// `lifted`), or of `points` followed by `direction`, from the exact
    /// stage alone: the reference of the zero-column test.
    fn exact_sign(points: &[Vec<f64>], lifted: bool, direction: Option<&[f64]>) -> Sign {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        let heights: Vec<LiftedHeight> = refs.iter().map(|p| LiftedHeight::of(p)).collect();
        exact::sign_exact(Rows {
            origin: refs[0],
            points: &refs[1..],
            direction,
            lifted: lifted.then_some(&heights[..]),
        })
        .unwrap()
    }

    /// A column of zero differences decides zero (#384), and a column one
    /// ulp away from zero is left to the stages. Every size of the plain
    /// orientation (k = 2 to 7), the lifted one (D = 2 to 5; at D = 1 a
    /// shared column makes the points equal), and an orientation against a
    /// direction, each column in turn.
    #[test]
    fn a_zero_column_is_zero_and_one_ulp_off_is_not() {
        let mut rng = Rng(384);
        for (dim, extra, direction) in (2..=7)
            .map(|k| (k, 0, false))
            .chain((2..=5).map(|d| (d, 1, false)))
            .chain((2..=6).map(|k| (k, 0, true)))
        {
            let lifted = extra == 1;
            let count = if direction { dim } else { dim + 1 + extra };
            for column in 0..dim {
                let shared = rng.unit();
                let mut points: Vec<Vec<f64>> = (0..count)
                    .map(|_| (0..dim).map(|_| rng.unit()).collect())
                    .collect();
                for p in &mut points {
                    p[column] = shared;
                }
                let mut toward: Vec<f64> = (0..dim).map(|_| rng.unit()).collect();
                toward[column] = 0.0;
                let dir = direction.then_some(&toward[..]);
                let sign = |points: &[Vec<f64>], dir: Option<&[f64]>| {
                    let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
                    match (lifted, dir) {
                        (true, _) => orient_lifted(&refs).unwrap(),
                        (false, Some(d)) => orient_direction(&refs, d).unwrap(),
                        (false, None) => orient(&refs).unwrap(),
                    }
                };
                let what =
                    format!("dim {dim}, lifted {lifted}, direction {direction}, column {column}");
                assert_eq!(exact_sign(&points, lifted, dir), Sign::Zero, "{what}");
                assert_eq!(sign(&points, dir), Sign::Zero, "{what}");
                // One ulp off, at the last row: the column is no longer zero.
                if direction {
                    toward[column] = f64::from_bits(1);
                } else {
                    let last = points.len() - 1;
                    points[last][column] = shared.next_up();
                }
                let dir = direction.then_some(&toward[..]);
                let expected = exact_sign(&points, lifted, dir);
                assert_ne!(expected, Sign::Zero, "{what}: one ulp off");
                assert_eq!(sign(&points, dir), expected, "{what}: one ulp off");
            }
        }
    }
}
