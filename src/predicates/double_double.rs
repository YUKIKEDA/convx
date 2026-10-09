//! The second stage of the orientations and lifted orientations with
//! k ≤ 6 (#349, #352): the determinant in double-double arithmetic,
//! certified by a bound derived once per size, as the first stage
//! ([`super::semi_static`]) is.
//!
//! It runs on what the `f64` stage leaves open: after the first stage for
//! k ≤ 5, after the running filter for k = 6 (design §1). When it does not
//! certify the sign either, the exact stage decides, so the sign returned is
//! unchanged.
//!
//! # Double-double values
//!
//! A value is a pair `(hi, lo)` standing for `hi + lo`, with
//! `hi = fl(hi + lo)`, so `|lo| ≤ u |hi|` and `|hi| ≤ |hi + lo| / (1 - u)`,
//! with `u = 2^-53`. Every pair here comes out of [`two_sum`] or
//! [`two_diff`], which give that form exactly (Knuth's TwoSum).
//! [`two_product`] gives `a b` exactly as a pair by Dekker's method, with
//! Veltkamp's split and no FMA, when no operation in it underflows or
//! overflows (Dekker 1971; Shewchuk 1997, Theorem 18).
//!
//! # Where the stage runs
//!
//! Only when every nonzero input coordinate lies in `[2^-90, 2^100]`.
//! Otherwise it returns `None` and the exact stage decides.
//!
//! - **No underflow, no subnormal value.** A coordinate of magnitude at
//!   least `2^-90` is a multiple of `2^-142`. Give a value computed here the
//!   weight `w` when it is a multiple of `2^(-142 w)`: a coordinate and a
//!   difference of coordinates weigh 1, a sum or difference weighs the
//!   larger weight of its operands, a product the sum of theirs, and
//!   rounding to nearest keeps the weight (a rounded multiple of `2^-q` is a
//!   multiple of its own ulp, or exact). The split halves of a value and the
//!   partial products of [`two_product`] keep the weights of what they
//!   split. The determinant has degree k ≤ 6 in the differences, and the
//!   lifted column degree 2, so no weight exceeds 7: every exact result of
//!   an operation is a multiple of `2^-994`, and nonzero ones are at least
//!   `2^-994`, above the subnormal range. Rounding is then that of an
//!   unbounded exponent range, with no underflow term, and Dekker's product
//!   is exact.
//! - **No overflow.** A difference is at most `2^101` and a lifted entry
//!   `5 · 2^202`, so a minor's permanent is at most `6! 2^505 · 5 · 2^202`,
//!   below `2^720`, and every value here, the splitter's product
//!   `(2^27 + 1) x` included, stays below `2^760`.
//!
//! # One operation
//!
//! Let `x = (xh, xl)` and `y = (yh, yl)` be pairs of the form above.
//!
//! [`add`]: `(sh, sl) = TwoSum(xh, yh)`, `v = fl(xl + yl)`,
//! `w = fl(sl + v)`, `z = TwoSum(sh, w)`. Then
//! `x + y - z = (xl + yl - v) + (sl + v - w)`. With `S = |xh| + |yh|`:
//! `|xl + yl| ≤ u S`, so the first rounding is at most `u^2 S`;
//! `|sl| ≤ u S` and `|v| ≤ u (1 + u) S`, so the second is at most
//! `u^2 (2 + u) S`. As `S ≤ (|x| + |y|) / (1 - u)`,
//! `|z - (x + y)| ≤ α (|x| + |y|)` with `α = 3 u^2 (1 + 2u)`. The last step
//! is TwoSum, not Fast2Sum, because `|w|` may exceed `|sh|` when `xh` and
//! `yh` cancel.
//!
//! [`mul`]: `(ch, c1) = TwoProduct(xh, yh)`, `t1 = fl(xh yl)`,
//! `t2 = fl(xl yh)`, `c2 = fl(t1 + t2)`, `c3 = fl(c1 + c2)`,
//! `z = TwoSum(ch, c3)`. Then
//! `x y - z = xl yl + (xh yl - t1) + (xl yh - t2) + (t1 + t2 - c2) + (c1 + c2 - c3)`.
//! With `Q = |xh| |yh|`, the five terms are at most `u^2 Q`, `u^2 Q`,
//! `u^2 Q`, `2 u^2 (1 + u) Q` (as `|t1 + t2| ≤ 2 u (1 + u) Q`), and
//! `u^2 (3 + 4u + 2u^2) Q` (as `|c1| ≤ u Q` and `|c2| ≤ 2u (1 + u)^2 Q`).
//! As `Q ≤ |x| |y| / (1 - u)^2`, `|z - x y| ≤ μ |x| |y|` with
//! `μ = 8 u^2 (1 + 3u)`.
//!
//! Negation is exact.
//!
//! # The bound
//!
//! Every entry of a coordinate column is a difference `d = p_i - p_0`,
//! exact as a pair. A lifted entry is `sum_c d_c^2` (the polynomial of
//! [`super::semi_static`], with the same sign), each square by [`mul`] and
//! the sum pairwise. The determinant is expanded along its last column over
//! the minors of the leading columns, one subset of rows at a time, the `m`
//! terms of a minor of size `m` added pairwise.
//!
//! Let `P` be a node's permanent: the same expression with every difference
//! replaced by its absolute value and every subtraction by an addition, in
//! exact arithmetic. If the computed operands of a node are within
//! `θ_x P_x` and `θ_y P_y` of their exact values, the lemmas give a sum
//! within `((1 + α)(1 + θ) - 1)(P_x + P_y)` and a product within
//! `((1 + μ)(1 + θ_x)(1 + θ_y) - 1) P_x P_y`, with `θ` the larger of the
//! two, since `|x| ≤ (1 + θ_x) P_x`. So the determinant is within `θ P`,
//! where `1 + θ` is the product of `(1 + μ)` over the `n_m` products of a
//! monomial and of `(1 + α)` over its `n_a` levels of sums:
//!
//! - a minor of size `m` adds one product and `ceil(log2 m)` levels, so
//!   `k - 1` products and `A_k = sum_(m = 2..k) ceil(log2 m)` levels
//!   (`A_2 .. A_6` = 1, 3, 5, 8, 11);
//! - a lifted entry of `D` coordinates adds one product and
//!   `ceil(log2 D)` levels.
//!
//! Then `θ ≤ exp(n_m μ + n_a α) - 1 ≤ N u^2 (1 + 4u)` with
//! `N = 8 n_m + 3 n_a`, at most 90 here ([`constant`]).
//!
//! `P` is not computed. Every monomial takes one entry from each column,
//! so `P ≤ prod_c S_c`, with `S_c` the sum of the column's absolute values.
//! An exact entry is at most `(1 + 2u)` times the absolute value of its
//! `hi`. That holds for a lifted entry too, whose computed pair is within
//! `θ` of the exact sum of squares. `P̂` is the `f64` product of the column
//! sums of `|hi|`, with `k - 1` roundings per sum and `k - 1` for the
//! product, all on nonnegative values. So `P ≤ (1 + 2u)^k (1 - u)^(-(k^2 - 1)) P̂ ≤ (1 + 50 u) P̂`.
//!
//! The sign is certified when `|zh| 2^106 > fl((N + 1) P̂)`, with `zh` the
//! determinant's `hi`. Both sides are finite and the left one is exact.
//! Then `|z| ≥ (1 - u) |zh| > (1 - u)^2 (N + 1) u^2 P̂`, which exceeds
//! `N (1 + 4u)(1 + 50u) u^2 P̂ ≥ θ P` for `N ≤ 90`, so the exact
//! determinant has the sign of `zh`.
//!
//! An exact zero is never certified, since its bound is never below
//! `|zh|`. The exact stage decides it.

use super::semi_static::subsets_by_size;
use super::Sign;

/// `2^106 = u^-2`.
const INVERSE_U_SQUARED: f64 = f64::from_bits((1023 + 106) << 52);

/// The smallest nonzero coordinate magnitude the stage takes: `2^-90`.
const SMALLEST: f64 = f64::from_bits((1023 - 90) << 52);

/// The largest coordinate magnitude the stage takes: `2^100`.
const LARGEST: f64 = f64::from_bits((1023 + 100) << 52);

/// `2^27 + 1`, Veltkamp's splitter for 53-bit significands.
const SPLITTER: f64 = 134_217_729.0;

/// `hi + lo`, with `hi = fl(hi + lo)` (module docs).
#[derive(Clone, Copy)]
struct Pair {
    hi: f64,
    lo: f64,
}

/// Knuth's TwoSum: `a + b` exactly, for any order of magnitudes.
#[inline(always)]
fn two_sum(a: f64, b: f64) -> Pair {
    let hi = a + b;
    let b_virtual = hi - a;
    let a_virtual = hi - b_virtual;
    Pair {
        hi,
        lo: (a - a_virtual) + (b - b_virtual),
    }
}

/// `a - b` exactly: TwoSum of `a` and `-b`.
#[inline(always)]
fn two_diff(a: f64, b: f64) -> Pair {
    two_sum(a, -b)
}

/// Veltkamp's split of `a` into two halves of at most 26 significant bits
/// each, `a = high + low` exactly.
#[inline(always)]
fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let high = c - (c - a);
    (high, a - high)
}

/// Dekker's product: `a b` exactly, under the module's conditions.
#[inline(always)]
fn two_product(a: f64, b: f64) -> Pair {
    let hi = a * b;
    let (a_high, a_low) = split(a);
    let (b_high, b_low) = split(b);
    let err1 = hi - a_high * b_high;
    let err2 = err1 - a_low * b_high;
    let err3 = err2 - a_high * b_low;
    Pair {
        hi,
        lo: a_low * b_low - err3,
    }
}

/// `x + y`, within `α (|x| + |y|)` (module docs).
#[inline(always)]
fn add(x: Pair, y: Pair) -> Pair {
    let s = two_sum(x.hi, y.hi);
    let v = x.lo + y.lo;
    let w = s.lo + v;
    two_sum(s.hi, w)
}

/// `x y`, within `μ |x| |y|` (module docs).
#[inline(always)]
fn mul(x: Pair, y: Pair) -> Pair {
    let c = two_product(x.hi, y.hi);
    let t1 = x.hi * y.lo;
    let t2 = x.lo * y.hi;
    let c2 = t1 + t2;
    let c3 = c.lo + c2;
    two_sum(c.hi, c3)
}

/// `-x`, exact.
#[inline(always)]
fn neg(x: Pair) -> Pair {
    Pair {
        hi: -x.hi,
        lo: -x.lo,
    }
}

/// The sum of `terms` (at most 6), added pairwise in a tree of depth
/// `ceil(log2 n)`.
#[inline(always)]
fn pairwise_sum(terms: &[Pair]) -> Pair {
    match terms.len() {
        1 => terms[0],
        2 => add(terms[0], terms[1]),
        3 => add(add(terms[0], terms[1]), terms[2]),
        4 => add(add(terms[0], terms[1]), add(terms[2], terms[3])),
        5 => add(
            add(add(terms[0], terms[1]), add(terms[2], terms[3])),
            terms[4],
        ),
        _ => add(
            add(add(terms[0], terms[1]), add(terms[2], terms[3])),
            add(terms[4], terms[5]),
        ),
    }
}

/// `ceil(log2 m)` for `m ≥ 1`.
const fn ceil_log2(m: usize) -> u32 {
    usize::BITS - (m - 1).leading_zeros()
}

/// `N + 1`, with `N = 8 n_m + 3 n_a` for a determinant of size `k`, its
/// last column lifted from `lifted` coordinates when given (module docs).
const fn constant(k: usize, lifted: Option<usize>) -> f64 {
    let mut products = k - 1;
    let mut levels = 0;
    let mut m = 2;
    while m <= k {
        levels += ceil_log2(m) as usize;
        m += 1;
    }
    if let Some(dim) = lifted {
        products += 1;
        levels += ceil_log2(dim) as usize;
    }
    (8 * products + 3 * levels + 1) as f64
}

/// A determinant evaluated in double-double and its bound in units of
/// `u^2`: `fl((N + 1) P̂)` (module docs).
#[derive(Clone, Copy)]
struct Estimate {
    det: Pair,
    bound: f64,
}

impl Estimate {
    /// The sign of the determinant when `|hi| 2^106` exceeds the bound.
    #[inline(always)]
    fn sign(self) -> Option<Sign> {
        let hi = self.det.hi;
        if hi.abs() * INVERSE_U_SQUARED > self.bound {
            Some(if hi > 0.0 {
                Sign::Positive
            } else {
                Sign::Negative
            })
        } else {
            None
        }
    }
}

/// The sign of the orientation of `origin` followed by `points`, or of the
/// lifted orientation when `lifted`, for 2 ≤ k ≤ 6. `None` when the bound
/// does not certify it, when a coordinate is out of the stage's range, or
/// when the size is not covered.
///
/// `points` holds k rows of the dimension of `origin`: one per coordinate,
/// and one more when `lifted`.
#[inline(never)]
pub(super) fn sign(origin: &[f64], points: &[&[f64]], lifted: bool) -> Option<Sign> {
    estimate(origin, points, lifted)?.sign()
}

/// The determinant [`sign`] decides and its bound, or `None` when a
/// coordinate is out of range or the size is not covered.
fn estimate(origin: &[f64], points: &[&[f64]], lifted: bool) -> Option<Estimate> {
    let dim = origin.len();
    debug_assert_eq!(
        points.len(),
        dim + usize::from(lifted),
        "a determinant of size k needs k rows"
    );
    debug_assert!(
        points.iter().all(|p| p.len() == dim),
        "every row needs the dimension of the origin"
    );
    let in_range = |&c: &f64| c == 0.0 || (SMALLEST..=LARGEST).contains(&c.abs());
    if !(origin.iter().all(in_range) && points.iter().all(|p| p.iter().all(in_range))) {
        return None;
    }
    match dim + usize::from(lifted) {
        2 => Some(determinant::<2>(origin, points, lifted)),
        3 => Some(determinant::<3>(origin, points, lifted)),
        4 => Some(determinant::<4>(origin, points, lifted)),
        5 => Some(determinant::<5>(origin, points, lifted)),
        6 => Some(determinant::<6>(origin, points, lifted)),
        _ => None,
    }
}

/// The determinant of size `K` of the rows `points[i] - origin`, with the
/// lifted entries `|points[i] - origin|^2` as the last column when
/// `lifted`, and its bound (module docs).
fn determinant<const K: usize>(origin: &[f64], points: &[&[f64]], lifted: bool) -> Estimate {
    let dim = origin.len();
    let zero = Pair { hi: 0.0, lo: 0.0 };
    let mut rows = [[zero; K]; K];
    for (row, point) in rows.iter_mut().zip(points) {
        for c in 0..dim {
            row[c] = two_diff(point[c], origin[c]);
        }
        if lifted {
            let squares: [Pair; K] = core::array::from_fn(|c| mul(row[c], row[c]));
            row[dim] = pairwise_sum(&squares[..dim]);
        }
    }
    let (order, start) = const { subsets_by_size::<K>() };
    // Minors over row subsets, as bit masks of the K rows; those of size m
    // are over the columns 0 .. m.
    let mut det = [zero; 64];
    for (r, row) in rows.iter().enumerate() {
        det[1 << r] = row[0];
    }
    for m in 2..=K {
        let column = m - 1;
        for &subset in &order[start[m]..start[m + 1]] {
            let subset = usize::from(subset);
            let mut terms = [zero; 6];
            let mut bits = subset;
            let mut i = 0;
            while bits != 0 {
                let r = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                let term = mul(rows[r][column], det[subset & !(1 << r)]);
                terms[i] = if (i + column) % 2 == 0 {
                    term
                } else {
                    neg(term)
                };
                i += 1;
            }
            det[subset] = pairwise_sum(&terms[..m]);
        }
    }
    let column_sum = |c: usize| -> f64 { rows.iter().map(|row| row[c].hi.abs()).sum() };
    let permanent = (0..K).fold(1.0, |product, c| product * column_sum(c));
    Estimate {
        det: det[(1 << K) - 1],
        bound: constant(K, if lifted { Some(dim) } else { None }) * permanent,
    }
}

#[cfg(test)]
mod tests {
    use super::super::exact::{exponent_shift, BigInt};
    use super::super::semi_static::tests::{
        cofactor_expansion, count, exact_of, int, near_degenerate, pow2, two_to, Rng,
    };
    use super::super::{filtered_value, semi_static, LiftedHeight, Rows};
    use super::*;

    /// Every size: (dimension of a point, lifted).
    const FORMULAS: [(usize, bool); 10] = [
        (2, false),
        (3, false),
        (4, false),
        (5, false),
        (6, false),
        (1, true),
        (2, true),
        (3, true),
        (4, true),
        (5, true),
    ];

    fn refs(points: &[Vec<f64>]) -> Vec<&[f64]> {
        points.iter().map(Vec::as_slice).collect()
    }

    fn double_double_of(points: &[Vec<f64>], lifted: bool) -> Option<Sign> {
        let refs = refs(points);
        sign(refs[0], &refs[1..], lifted)
    }

    /// The sign the `f64` stage certifies: the first stage for k ≤ 5, the
    /// running filter for k = 6 (design §1).
    fn f64_stage_of(points: &[Vec<f64>], lifted: bool) -> Option<Sign> {
        let refs = refs(points);
        if points[0].len() + usize::from(lifted) <= 5 {
            return semi_static::sign(refs[0], &refs[1..], lifted);
        }
        let heights: Vec<LiftedHeight> = refs.iter().map(|p| LiftedHeight::of(p)).collect();
        filtered_value(Rows {
            origin: refs[0],
            points: &refs[1..],
            direction: None,
            lifted: lifted.then_some(&heights[..]),
        })
        .and_then(|value| value.certified_sign())
    }

    /// Certified, left open, and exactly zero, over one family.
    #[derive(Debug, Default)]
    struct Tally {
        certified: usize,
        open: usize,
        zero: usize,
    }

    /// Checks every sign the stage certifies against the exact sign and
    /// tallies the cases. A zero is never certified.
    fn check(points: &[Vec<f64>], lifted: bool, tally: &mut Tally, what: &str) {
        let expected = exact_of(points, lifted);
        tally.zero += usize::from(expected == Sign::Zero);
        match double_double_of(points, lifted) {
            Some(sign) => {
                assert_eq!(sign, expected, "{what}: {points:?}");
                tally.certified += 1;
            }
            None => tally.open += 1,
        }
    }

    /// The constants `N + 1` of the module docs: `n_m = k - 1` products and
    /// `A_k` levels, plus one product and `ceil(log2 D)` levels for a lifted
    /// column.
    #[test]
    fn the_constants_count_products_and_levels() {
        assert_eq!(
            [2, 3, 4, 5, 6].map(|k| constant(k, None)),
            [12.0, 26.0, 40.0, 57.0, 74.0]
        );
        assert_eq!(
            [1, 2, 3, 4, 5].map(|dim| constant(dim + 1, Some(dim))),
            [20.0, 37.0, 54.0, 71.0, 91.0]
        );
    }

    #[test]
    fn general_position_is_certified_and_exact() {
        let mut rng = Rng(31);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for _ in 0..500 {
                let points: Vec<Vec<f64>> = (0..count(dim, lifted))
                    .map(|_| (0..dim).map(|_| rng.unit()).collect())
                    .collect();
                check(&points, lifted, &mut tally, "random");
            }
            assert_eq!(
                tally.certified, 500,
                "dim {dim}, lifted {lifted}: {tally:?}"
            );
        }
    }

    #[test]
    fn exact_zeros_are_left_open() {
        // Small integer grids: every nonzero determinant is at least 1 and
        // is certified; every zero stays open.
        let mut rng = Rng(37);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for _ in 0..1000 {
                let points: Vec<Vec<f64>> = (0..count(dim, lifted))
                    .map(|_| (0..dim).map(|_| rng.below(5) as f64 - 2.0).collect())
                    .collect();
                check(&points, lifted, &mut tally, "grid");
            }
            assert!(tally.zero > 0, "dim {dim}, lifted {lifted}: {tally:?}");
            assert_eq!(
                (tally.certified, tally.open),
                (1000 - tally.zero, tally.zero),
                "dim {dim}, lifted {lifted}"
            );
        }
    }

    /// The near-degenerate inputs of the first stage's tests: the ones the
    /// `f64` stage leaves open, all of them nonzero, are certified here.
    #[test]
    fn what_the_f64_stage_leaves_open_is_certified() {
        let mut rng = Rng(41);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for trial in 0..3000 {
                let offset = [0.0, 1.0, 1e3, 1e6][trial % 4];
                let points = near_degenerate(&mut rng, dim, lifted, offset);
                if f64_stage_of(&points, lifted).is_none() {
                    check(&points, lifted, &mut tally, "near degenerate");
                }
            }
            assert!(tally.certified > 0, "dim {dim}, lifted {lifted}: {tally:?}");
            assert_eq!(
                tally.open, tally.zero,
                "dim {dim}, lifted {lifted}: only the exact zeros stay open"
            );
        }
    }

    #[test]
    fn every_exponent_in_range_and_none_outside() {
        let mut rng = Rng(43);
        let in_range = |points: &[Vec<f64>]| {
            points
                .iter()
                .flatten()
                .all(|&c| c == 0.0 || (two_to(-90)..=two_to(100)).contains(&c.abs()))
        };
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for e in (-1074..=1023).step_by(23).chain([-1074, -90, 100, 1023]) {
                for _ in 0..4 {
                    let points: Vec<Vec<f64>> = (0..count(dim, lifted))
                        .map(|_| (0..dim).map(|_| rng.unit() * two_to(e)).collect())
                        .collect();
                    if in_range(&points) {
                        check(&points, lifted, &mut tally, "scaled");
                    } else {
                        assert_eq!(double_double_of(&points, lifted), None, "{points:?}");
                    }
                    // A scale per coordinate, from the whole range.
                    let mixed: Vec<Vec<f64>> = (0..count(dim, lifted))
                        .map(|_| {
                            (0..dim)
                                .map(|_| rng.unit() * two_to(rng.below(191) as i32 - 90))
                                .collect()
                        })
                        .collect();
                    if in_range(&mixed) {
                        check(&mixed, lifted, &mut tally, "mixed");
                    }
                }
            }
            // Coordinates of very different magnitudes can make the
            // determinant tiny against the product of the column sums, so a
            // nonzero one may stay open here.
            assert!(tally.certified > 0, "dim {dim}, lifted {lifted}: {tally:?}");
        }
    }

    /// The ends of the range are in it, and the next values out are not.
    /// Entries drawn from `0`, `±1`, `±2^-90`, and `±2^100` reach the
    /// largest and the smallest products the module docs allow.
    #[test]
    fn the_range_ends() {
        let mut rng = Rng(47);
        let values = [
            0.0,
            1.0,
            -1.0,
            two_to(-90),
            -two_to(-90),
            two_to(100),
            -two_to(100),
        ];
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for _ in 0..300 {
                let mut points: Vec<Vec<f64>> = (0..count(dim, lifted))
                    .map(|_| {
                        (0..dim)
                            .map(|_| values[rng.below(values.len() as u64) as usize])
                            .collect()
                    })
                    .collect();
                check(&points, lifted, &mut tally, "ends");
                let last = points.len() - 1;
                for out in [f64::next_down(two_to(-90)), f64::next_up(two_to(100))] {
                    points[last][0] = out;
                    assert_eq!(double_double_of(&points, lifted), None, "{points:?}");
                }
            }
            // As in `every_exponent_in_range_and_none_outside`, a nonzero
            // determinant of mixed magnitudes may stay open.
            assert!(tally.certified > 0, "dim {dim}, lifted {lifted}: {tally:?}");
        }
    }

    #[test]
    fn one_ulp_from_zero() {
        let unit = |dim: usize, axis: usize, sign: f64| -> Vec<f64> {
            let mut v = vec![0.0; dim];
            v[axis] = sign;
            v
        };
        for (dim, lifted) in FORMULAS {
            // Plain: points on the hyperplane `x_(dim-1) = 1`. Lifted: points
            // on the unit sphere. The moved coordinate is nonzero, so it
            // stays in range.
            let (points, moved): (Vec<Vec<f64>>, usize) = if lifted && dim == 1 {
                // Three distinct points of D = 1 are never on one sphere.
                (vec![vec![0.0], vec![1.0], vec![2.0]], 0)
            } else if lifted {
                let points = [unit(dim, 0, 1.0), unit(dim, 0, -1.0)]
                    .into_iter()
                    .chain((1..dim).map(|a| unit(dim, a, 1.0)))
                    .chain([unit(dim, 1, -1.0)])
                    .collect();
                (points, 1)
            } else {
                let lift = |mut p: Vec<f64>| {
                    p[dim - 1] = 1.0;
                    p
                };
                let points = [lift(vec![0.0; dim])]
                    .into_iter()
                    .chain((0..dim - 1).map(|a| lift(unit(dim, a, 1.0))))
                    .chain([lift(vec![0.25; dim])])
                    .collect();
                (points, dim - 1)
            };
            assert_eq!(
                exact_of(&points, lifted) == Sign::Zero,
                !(lifted && dim == 1),
                "dim {dim}, lifted {lifted}"
            );
            let last = points.len() - 1;
            for step in [f64::next_down, f64::next_up] {
                let mut points = points.clone();
                points[last][moved] = step(points[last][moved]);
                let mut tally = Tally::default();
                check(&points, lifted, &mut tally, "one ulp");
                assert_eq!(tally.certified, 1, "dim {dim}, lifted {lifted}: {points:?}");
            }
        }
    }

    /// The error `z - E` of the determinant the stage computes and its
    /// bound, both times one positive power of two. `E` is the determinant
    /// of the exact differences, with the exact `|d_i|^2` as the lifted
    /// column, expanded over integers here.
    fn error_and_bound(points: &[Vec<f64>], lifted: bool) -> (BigInt, BigInt) {
        let refs = refs(points);
        let Estimate { det, bound } = estimate(refs[0], &refs[1..], lifted).unwrap();
        // Every coordinate is a multiple of 2^-shift.
        let base = points
            .iter()
            .flatten()
            .filter_map(|&x| exponent_shift(x))
            .min()
            .unwrap_or(0)
            .min(1074);
        let shift = 1074 - base;
        let rows: Vec<Vec<BigInt>> = points[1..]
            .iter()
            .map(|p| {
                let mut row: Vec<BigInt> = p
                    .iter()
                    .zip(&points[0])
                    .map(|(&x, &o)| {
                        let scaled = |x| BigInt::from_f64_scaled(x, base).unwrap();
                        scaled(x).sub(&scaled(o)).unwrap()
                    })
                    .collect();
                if lifted {
                    let norm = row
                        .iter()
                        .fold(int(0.0), |sum, d| sum.add(&d.mul(d).unwrap()).unwrap());
                    row.push(norm);
                }
                row
            })
            .collect();
        let columns: Vec<usize> = (0..rows.len()).collect();
        let exact = cofactor_expansion(&rows, &columns);
        assert_eq!(exact.sign(), exact_of(points, lifted), "{points:?}");
        // `exact` is E 2^(shift degree) and `int(v)` is v 2^1074. The bound
        // is in units of u^2 = 2^-106, so the error is brought to
        // 2^(shift degree + 1074 + 106) and the bound to
        // 2^(shift degree + 1074).
        let degree = (points[0].len() + 2 * usize::from(lifted)) as u64;
        let lift = pow2(shift * degree);
        let computed = int(det.hi).add(&int(det.lo)).unwrap();
        let error = computed
            .mul(&lift)
            .unwrap()
            .sub(&exact.mul(&pow2(1074)).unwrap())
            .unwrap()
            .mul(&pow2(106))
            .unwrap();
        (error, int(bound).mul(&lift).unwrap())
    }

    /// Whether `bound >= factor |error|`.
    fn covers(bound: &BigInt, error: &BigInt, factor: u32) -> bool {
        // `int(factor)` is factor 2^1074.
        let scaled = error.mul(&int(f64::from(factor))).unwrap();
        let bound = bound.mul(&pow2(1074)).unwrap();
        bound.sub(&scaled).unwrap().sign() != Sign::Negative
            && bound.add(&scaled).unwrap().sign() != Sign::Negative
    }

    /// The bound is derived as a worst case, which no input here reaches,
    /// so a smaller constant can pass every test of a sign. This test pins
    /// the distance instead: over the near-degenerate inputs, the bound
    /// stays a fixed factor above the exact error of the computed
    /// determinant, and some input comes within twice that factor, so a
    /// constant cut in half fails the first assertion.
    #[test]
    fn the_bound_keeps_its_measured_margin() {
        /// Per size, in the order of `FORMULAS`. The product of the column
        /// sums stands for the permanent, so the margin grows with k.
        const MARGIN: [u32; 10] = [70, 80, 500, 1700, 13000, 30, 90, 450, 1900, 11000];
        let mut rng = Rng(53);
        for ((dim, lifted), margin) in FORMULAS.into_iter().zip(MARGIN) {
            let mut near = 0;
            for trial in 0..1000 {
                let offset = [0.0, 1.0, 1e3, 1e6][trial % 4];
                let points = near_degenerate(&mut rng, dim, lifted, offset);
                let (error, bound) = error_and_bound(&points, lifted);
                assert!(
                    covers(&bound, &error, margin),
                    "dim {dim}, lifted {lifted}: {points:?}"
                );
                near += usize::from(!covers(&bound, &error, 2 * margin));
            }
            assert!(near > 0, "dim {dim}, lifted {lifted}");
        }
    }
}
