//! Filtered `f64` evaluation of the orientation determinant.
//!
//! Each intermediate value carries an absolute error bound for the operations
//! actually performed (running error analysis). For a rounded operation
//! `c = fl(a op b)` with inputs known to within `e_a` and `e_b`:
//!
//! - add and subtract: `|err| <= e_a + e_b + u |c| + eta`
//! - multiply: `|err| <= |a| e_b + |b| e_a + e_a e_b + u |c| + eta`
//! - divide, when `|b| > e_b`: `|err| <= (e_a + |c| e_b) / (|b| - e_b) + u |c| + eta`
//!
//! `u = 2^-53` is the unit roundoff. `eta = 2^-1073` covers the absolute error
//! of a result that lands in the subnormal range, where the relative bound
//! does not hold, and of a bound term that itself underflows. The bound is
//! computed in `f64` from non-negative terms; each such evaluation loses at
//! most a factor `(1 - u)` per operation, so the result is multiplied by
//! `1 + 16 u`, which exceeds the loss of the at most eight roundings in one
//! update. Every operation adds at least `eta`, so a nonzero bound is always
//! present and a value that underflowed to zero is never taken as zero.
//!
//! No step uses `mul_add`, and Rust does not contract `a * b + c` into a fused
//! multiply-add on its own, so the bound does not need to cover FMA rounding.
//!
//! The filter returns a sign only when `|value| > bound` and both are finite.
//! Otherwise the caller falls back to the exact sign.
//!
//! The cofactors of a facet of two to four points are the exception: they
//! are evaluated in plain `f64` with one bound per cofactor from a constant
//! per size, when the edges lie in a range where that bound holds (see
//! [`small_cofactors`], #174).

use core::cmp::Ordering;

use pulp::{Arch, Simd, WithSimd};

use super::Sign;
use crate::small::Small;

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;
/// Two units in the last place of the smallest subnormal (2^-1073).
const ETA: f64 = f64::from_bits(2);
const GROW: f64 = 1.0 + 16.0 * UNIT_ROUNDOFF;

/// A computed `f64` value with an absolute bound on its error.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Approx {
    value: f64,
    error: f64,
}

impl Approx {
    pub(super) fn exact(value: f64) -> Self {
        Self { value, error: 0.0 }
    }

    /// A value and bound computed earlier and stored.
    pub(super) fn stored(value: f64, error: f64) -> Self {
        Self { value, error }
    }

    pub(super) fn add(self, other: Self) -> Self {
        let value = self.value + other.value;
        let error = (self.error + other.error + value.abs() * UNIT_ROUNDOFF + ETA) * GROW;
        Self { value, error }
    }

    pub(super) fn sub(self, other: Self) -> Self {
        let value = self.value - other.value;
        let error = (self.error + other.error + value.abs() * UNIT_ROUNDOFF + ETA) * GROW;
        Self { value, error }
    }

    pub(super) fn mul(self, other: Self) -> Self {
        let value = self.value * other.value;
        let error = (self.value.abs() * other.error
            + other.value.abs() * self.error
            + self.error * other.error
            + value.abs() * UNIT_ROUNDOFF
            + ETA)
            * GROW;
        Self { value, error }
    }

    /// Returns `None` when the divisor's sign is not certain.
    pub(super) fn div(self, other: Self) -> Option<Self> {
        let divisor_certain = other.value.abs() > other.error;
        if !divisor_certain {
            return None;
        }
        // A lower bound on |b| - e_b: the subtraction rounds by at most a
        // factor (1 + u), and the product by (1 - 4u) undoes it. The lanes
        // of [`edge_cofactors_in_lanes`] repeat these steps (see `Ops::div`).
        let denominator = (other.value.abs() - other.error) * (1.0 - 4.0 * UNIT_ROUNDOFF);
        let denominator_positive = denominator > 0.0;
        if !denominator_positive {
            return None;
        }
        let value = self.value / other.value;
        let error = ((self.error + value.abs() * other.error) / denominator
            + value.abs() * UNIT_ROUNDOFF
            + ETA)
            * GROW;
        Some(Self { value, error })
    }

    /// The value negated, with the same bound (negation is exact).
    pub(super) fn negated(self) -> Self {
        Self {
            value: -self.value,
            error: self.error,
        }
    }

    pub(super) fn value(self) -> f64 {
        self.value
    }

    pub(super) fn error(self) -> f64 {
        self.error
    }

    /// The sign of the value when the bound certifies it.
    pub(super) fn certified_sign(self) -> Option<Sign> {
        if !self.value.is_finite() || !self.error.is_finite() {
            return None;
        }
        if self.value.abs() > self.error {
            Some(if self.value > 0.0 {
                Sign::Positive
            } else {
                Sign::Negative
            })
        } else {
            None
        }
    }
}

fn det2(a: Approx, b: Approx, c: Approx, d: Approx) -> Approx {
    a.mul(d).sub(b.mul(c))
}

/// Dedicated formula for k = 2: the 2 x 2 determinant.
pub(super) fn orient2(m: &[[Approx; 2]; 2]) -> Approx {
    det2(m[0][0], m[0][1], m[1][0], m[1][1])
}

/// Dedicated formula for k = 3: cofactor expansion along the first row.
pub(super) fn orient3(m: &[[Approx; 3]; 3]) -> Approx {
    let c0 = det2(m[1][1], m[1][2], m[2][1], m[2][2]);
    let c1 = det2(m[1][0], m[1][2], m[2][0], m[2][2]);
    let c2 = det2(m[1][0], m[1][1], m[2][0], m[2][1]);
    m[0][0].mul(c0).sub(m[0][1].mul(c1)).add(m[0][2].mul(c2))
}

/// Dedicated formula for k = 4: Laplace expansion along the first two rows,
/// pairing each 2 x 2 minor of rows 0 and 1 with the complementary minor of
/// rows 2 and 3.
pub(super) fn orient4(m: &[[Approx; 4]; 4]) -> Approx {
    let top = |i: usize, j: usize| det2(m[0][i], m[0][j], m[1][i], m[1][j]);
    let bottom = |i: usize, j: usize| det2(m[2][i], m[2][j], m[3][i], m[3][j]);
    top(0, 1)
        .mul(bottom(2, 3))
        .sub(top(0, 2).mul(bottom(1, 3)))
        .add(top(0, 3).mul(bottom(1, 2)))
        .add(top(1, 2).mul(bottom(0, 3)))
        .sub(top(1, 3).mul(bottom(0, 2)))
        .add(top(2, 3).mul(bottom(0, 1)))
}

/// Filtered determinant of the `n` x `n` matrix with entries `entry(i, j)`,
/// for any size, by Gaussian elimination with partial pivoting. Returns
/// `None` when a pivot's sign is not certain.
///
/// Sizes 5 to 9 (every orientation and lifted orientation of the static
/// range) keep the matrix on the stack, so a call does not allocate. Larger
/// sizes use one vector. Both run [`eliminate`], so the value and the bound
/// do not depend on the storage.
pub(super) fn determinant(n: usize, entry: impl Fn(usize, usize) -> Approx) -> Option<Approx> {
    match n {
        5 => in_blocks(Square(Stack::<5, 2>::load(5, 5, entry))),
        6 => in_blocks(Square(Stack::<6, 2>::load(6, 6, entry))),
        7 => in_blocks(Square(Stack::<7, 2>::load(7, 7, entry))),
        8 => in_blocks(Square(Stack::<8, 2>::load(8, 8, entry))),
        9 => in_blocks(Square(Stack::<9, 3>::load(9, 9, entry))),
        _ => in_blocks(Square(Heap::load(n, n, entry))),
    }
}

/// Every cofactor `c_j` of the `(k - 1) x k` matrix `entry`, that is the
/// determinant of its rows followed by the unit row `e_j`, from one
/// elimination (#111, design §1).
///
/// Gaussian elimination with partial pivoting over the first `k - 1`
/// columns gives `[T | u]`, `T` upper triangular, after `s` row swaps. Row
/// additions leave every maximal minor unchanged and a swap flips its sign,
/// so with `T x = u` the cofactor vector is `(-1)^s det T (-x, 1)`. Every
/// operation carries its running bound, so each returned value bounds its
/// own cofactor. Returns `None` when a divisor's sign is not certified.
pub(super) fn cofactors(
    k: usize,
    entry: impl Fn(usize, usize) -> Approx,
) -> Option<Small<Approx, 10>> {
    // The k - 1 rows sit on the stack for the orders `determinant` keeps
    // there.
    match k {
        5 => in_blocks(Shared(Stack::<4, 2>::load(4, 5, entry))),
        6 => in_blocks(Shared(Stack::<5, 2>::load(5, 6, entry))),
        7 => in_blocks(Shared(Stack::<6, 2>::load(6, 7, entry))),
        8 => in_blocks(Shared(Stack::<7, 2>::load(7, 8, entry))),
        9 => in_blocks(Shared(Stack::<8, 3>::load(8, 9, entry))),
        _ => in_blocks(Shared(Heap::load(k - 1, k, entry))),
    }
}

/// The magnitudes inside which [`small_cofactors`] takes a facet's edges:
/// every nonzero edge entry lies in `[2^-250, 2^250]`.
const SMALL_LOW: f64 = f64::from_bits((1023 - 250) << 52); // 2^-250
const SMALL_HIGH: f64 = f64::from_bits((1023 + 250) << 52); // 2^250

/// Cofactors of one edge `tip - origin`, or `None` when an entry lies
/// outside the range [`small_cofactors`] accepts.
pub(crate) fn two_point_cofactors(origin: &[f64], tip: &[f64]) -> Option<[(f64, f64); 2]> {
    let (ex, ey) = (tip[0] - origin[0], tip[1] - origin[1]);
    let inside = |d: f64| d == 0.0 || (SMALL_LOW..=SMALL_HIGH).contains(&d.abs());
    if !inside(ex) || !inside(ey) {
        return None;
    }
    let u = UNIT_ROUNDOFF;
    Some([(-ey, 2.0 * u * ey.abs()), (ex, 2.0 * u * ex.abs())])
}

/// Every cofactor of a facet of `k = 2, 3, 4` points of dimension `k`, with
/// one error bound per cofactor from a constant per `k` (#174).
///
/// The cofactor `c_j` is the determinant of the edges `E_i = p_i - p_0`
/// followed by the unit row `e_j`; expanding along that row, it is
/// `(-1)^(k-1+j)` times the minor `M_j` of the edges without column `j`.
/// The edges are rounded once, `e = fl(E) = E(1 + d)`, and the minors are
/// evaluated in plain `f64`, the 2 x 2 minors of the last two edges shared
/// by every cofactor. With unit roundoff `u`, each term of `M_j` passes
/// through at most `n` roundings: `n = 1` for k = 2 (the edge), 4 for k = 3
/// (two edges, the product, the difference), 8 for k = 4 (three edges, two
/// products, the difference of the minor, and two more sums). So
/// `|fl(M_j) - M_j| <= gamma_n P_j`, with `gamma_n = n u / (1 - n u)` and
/// `P_j` the same expansion over `|E|` (Higham, Accuracy and Stability of
/// Numerical Algorithms, §3.1). `P_j` is evaluated over `|e|` in the same
/// order: every step rounds a sum of non-negative terms, and `|E| <= |e| /
/// (1 - u)`, so `P_j <= p / (1 - u)^n` for the computed `p`. The bound is
/// `fl(c u p)` with `c = 2, 5, 9`, which exceeds `gamma_n / (1 - u)^n`
/// after its own rounding.
///
/// The model has no underflow or overflow when every nonzero edge entry
/// lies in `[2^-250, 2^250]`: a product of two entries is at least 2^-500,
/// a nonzero 2 x 2 minor is a multiple of 2^-552, a product with a third
/// entry at least 2^-802, every value at most 2^753, and a sum that lands
/// in the subnormal range is exact. Otherwise, or when a value is not
/// finite, this returns `None` and the caller takes the running bound of
/// [`Approx`]. Plain `f64` operations in a fixed order, so every
/// instruction set gives the same bits.
pub(super) fn small_cofactors(facet: &[&[f64]]) -> Option<[(f64, f64); 4]> {
    let k = facet.len();
    debug_assert!(
        (2..=4).contains(&k),
        "small cofactors are for 2 to 4 points"
    );
    let mut e = [[0.0_f64; 4]; 3];
    for (row, p) in e.iter_mut().zip(&facet[1..]) {
        for ((slot, &x), &o) in row.iter_mut().zip(p.iter()).zip(facet[0]) {
            let d = x - o;
            let magnitude = d.abs();
            // Also rejects NaN and infinities.
            let inside = magnitude == 0.0 || (SMALL_LOW..=SMALL_HIGH).contains(&magnitude);
            if !inside {
                return None;
            }
            *slot = d;
        }
    }
    let u = UNIT_ROUNDOFF;
    let mut out = [(0.0, 0.0); 4];
    match k {
        2 => {
            // c_0 = -E_1, c_1 = E_0.
            let pair = two_point_cofactors(facet[0], facet[1])?;
            out[0] = pair[0];
            out[1] = pair[1];
        }
        3 => {
            // c_j = (-1)^j M_j, M_j the 2 x 2 minor without column j.
            let (a, b) = (e[0], e[1]);
            let minor = |x: usize, y: usize| {
                (
                    a[x] * b[y] - a[y] * b[x],
                    a[x].abs() * b[y].abs() + a[y].abs() * b[x].abs(),
                )
            };
            let columns = [(1, 2), (0, 2), (0, 1)];
            for (j, &(x, y)) in columns.iter().enumerate() {
                let (m, p) = minor(x, y);
                let value = if j % 2 == 0 { m } else { -m };
                out[j] = (value, 5.0 * u * p);
            }
        }
        _ => {
            // c_j = (-1)^(j + 1) M_j, M_j the 3 x 3 minor without column j,
            // expanded along the first edge over the shared 2 x 2 minors of
            // the last two edges.
            let (a, b, c) = (e[0], e[1], e[2]);
            let mut minor = [[(0.0, 0.0); 4]; 4];
            for x in 0..4 {
                for y in x + 1..4 {
                    minor[x][y] = (
                        b[x] * c[y] - b[y] * c[x],
                        b[x].abs() * c[y].abs() + b[y].abs() * c[x].abs(),
                    );
                }
            }
            let columns = [[1, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]];
            for (j, &[x, y, z]) in columns.iter().enumerate() {
                let m = a[x] * minor[y][z].0 - a[y] * minor[x][z].0 + a[z] * minor[x][y].0;
                let p = a[x].abs() * minor[y][z].1
                    + a[y].abs() * minor[x][z].1
                    + a[z].abs() * minor[x][y].1;
                let value = if j % 2 == 1 { m } else { -m };
                out[j] = (value, 9.0 * u * p);
            }
        }
    }
    Some(out)
}

/// Facets per call of [`edge_cofactors_in_lanes`].
pub(super) const FACET_LANES: usize = 4;

/// The cofactors of each lane's facet, `None` where they are not certified.
pub(super) type LaneCofactors = [Option<Small<(f64, f64), 10>>; FACET_LANES];

/// The finite [`cofactors`] of the edge rows of `FACET_LANES` facets of
/// `k` points of dimension `k`, facet `lane` with every coordinate
/// multiplied by `factors(largest)[lane]`, where `largest[lane]` is the
/// largest coordinate magnitude of that facet. Result `lane` is bit for bit
/// what [`cofactors`] returns for the rows
/// `exact(p_(i+1) * f).sub(exact(p_0 * f))` of that facet, `None` when that
/// is `None` or a value or bound is not finite.
///
/// One elimination is a chain of dependent operations, column after
/// column. With four `f64` lanes per vector, each lane runs that chain for
/// its own facet, with the same operations in the same order, so the
/// chains of the four facets take the time of one. The coordinates are
/// gathered, scaled, and differenced in the same vectors. A lane whose
/// divisor is not certain keeps computing values that are not read.
///
/// Returns `None` without such vectors, for `k` outside 5 to 9, and when
/// `factors` returns `None`; the caller then evaluates each facet alone.
/// The coordinates must not be NaN.
pub(super) fn edge_cofactors_in_lanes(
    facets: [&[&[f64]]; FACET_LANES],
    factors: impl Fn([f64; FACET_LANES]) -> Option<[f64; FACET_LANES]>,
) -> Option<LaneCofactors> {
    debug_assert!(
        facets.iter().all(|f| f
            .iter()
            .all(|p| p.len() == f.len() && p.iter().all(|x| !x.is_nan()))),
        "each facet has k points of dimension k, none NaN"
    );
    #[cfg(target_arch = "x86_64")]
    if let Some(simd) = pulp::x86::V3::try_new() {
        let factors = &factors;
        return match facets[0].len() {
            5 => Simd::vectorize(simd, lanes::Edges::<4, 5, _>(simd, facets, factors)),
            6 => Simd::vectorize(simd, lanes::Edges::<5, 6, _>(simd, facets, factors)),
            7 => Simd::vectorize(simd, lanes::Edges::<6, 7, _>(simd, facets, factors)),
            8 => Simd::vectorize(simd, lanes::Edges::<7, 8, _>(simd, facets, factors)),
            9 => Simd::vectorize(simd, lanes::Edges::<8, 9, _>(simd, facets, factors)),
            _ => None,
        };
    }
    #[cfg(not(target_arch = "x86_64"))]
    let _ = (facets, factors);
    None
}

/// The lane elimination of [`edge_cofactors_in_lanes`] on AVX2 vectors of
/// four `f64`, written with `V3`'s own operations, each inlined into the
/// one vectorized call.
#[cfg(target_arch = "x86_64")]
mod lanes {
    use pulp::x86::V3;
    use pulp::{cast, f64x4, i64x4, m64x4, Simd, WithSimd};

    use super::{LaneCofactors, ETA, FACET_LANES, GROW, UNIT_ROUNDOFF};

    /// One entry of every lane in two vectors.
    #[derive(Clone, Copy)]
    struct Pair {
        value: f64x4,
        error: f64x4,
    }

    /// The [`super::Approx`] operations on vectors of lanes. Each lane
    /// rounds as the scalar operation does: the same terms, added in the
    /// same order.
    #[derive(Clone, Copy)]
    struct Ops(V3);

    impl Ops {
        #[inline(always)]
        fn splat(self, x: f64) -> f64x4 {
            self.0.splat_f64x4(x)
        }

        #[inline(always)]
        fn exact(self, value: f64x4) -> Pair {
            Pair {
                value,
                error: self.splat(0.0),
            }
        }

        #[inline(always)]
        fn sub(self, a: Pair, b: Pair) -> Pair {
            let s = self.0;
            let value = s.sub_f64x4(a.value, b.value);
            let terms = s.add_f64x4(a.error, b.error);
            let terms = s.add_f64x4(
                terms,
                s.mul_f64x4(s.abs_f64x4(value), self.splat(UNIT_ROUNDOFF)),
            );
            let error = s.mul_f64x4(s.add_f64x4(terms, self.splat(ETA)), self.splat(GROW));
            Pair { value, error }
        }

        #[inline(always)]
        fn mul(self, a: Pair, b: Pair) -> Pair {
            let s = self.0;
            let value = s.mul_f64x4(a.value, b.value);
            let terms = s.mul_f64x4(s.abs_f64x4(a.value), b.error);
            let terms = s.add_f64x4(terms, s.mul_f64x4(s.abs_f64x4(b.value), a.error));
            let terms = s.add_f64x4(terms, s.mul_f64x4(a.error, b.error));
            let terms = s.add_f64x4(
                terms,
                s.mul_f64x4(s.abs_f64x4(value), self.splat(UNIT_ROUNDOFF)),
            );
            let error = s.mul_f64x4(s.add_f64x4(terms, self.splat(ETA)), self.splat(GROW));
            Pair { value, error }
        }

        /// [`super::Approx::div`]; `certain` is cleared in the lanes whose
        /// divisor's sign is not certain.
        #[inline(always)]
        fn div(self, a: Pair, b: Pair, certain: &mut m64x4) -> Pair {
            let s = self.0;
            let divisor = s.abs_f64x4(b.value);
            let divisor_certain = s.cmp_gt_f64x4(divisor, b.error);
            let denominator = s.mul_f64x4(
                s.sub_f64x4(divisor, b.error),
                self.splat(1.0 - 4.0 * UNIT_ROUNDOFF),
            );
            let denominator_positive = s.cmp_gt_f64x4(denominator, self.splat(0.0));
            *certain = s.and_m64x4(*certain, s.and_m64x4(divisor_certain, denominator_positive));
            let value = s.div_f64x4(a.value, b.value);
            let magnitude = s.abs_f64x4(value);
            let spread = s.add_f64x4(a.error, s.mul_f64x4(magnitude, b.error));
            let terms = s.add_f64x4(
                s.div_f64x4(spread, denominator),
                s.mul_f64x4(magnitude, self.splat(UNIT_ROUNDOFF)),
            );
            let error = s.mul_f64x4(s.add_f64x4(terms, self.splat(ETA)), self.splat(GROW));
            Pair { value, error }
        }

        /// `-value`, by flipping the sign bit as the scalar negation does.
        #[inline(always)]
        fn negated(self, value: f64x4) -> f64x4 {
            self.0.xor_f64x4(value, self.splat(-0.0))
        }

        #[inline(always)]
        fn select(self, mask: m64x4, if_true: Pair, if_false: Pair) -> Pair {
            Pair {
                value: self.0.select_f64x4(mask, if_true.value, if_false.value),
                error: self.0.select_f64x4(mask, if_true.error, if_false.error),
            }
        }

        /// The bits of `|value|`. Their sign bit is clear, so they order as
        /// `f64::total_cmp` orders the magnitudes.
        #[inline(always)]
        fn magnitude_bits(self, value: f64x4) -> i64x4 {
            cast(self.0.abs_f64x4(value))
        }

        #[inline(always)]
        fn index(self, i: usize) -> i64x4 {
            self.0.splat_i64x4(i as i64)
        }
    }

    /// The lanes of [`super::edge_cofactors_in_lanes`] for facets of `C`
    /// points, whose edges make `R = C - 1` rows. A method, not a closure:
    /// `vectorize` inlines `with_simd`, and a closure is not always
    /// inlined, which leaves every `V3` operation a call.
    pub(super) struct Edges<'a, const R: usize, const C: usize, F>(
        pub(super) V3,
        pub(super) [&'a [&'a [f64]]; FACET_LANES],
        pub(super) &'a F,
    );

    impl<const R: usize, const C: usize, F> WithSimd for Edges<'_, R, C, F>
    where
        F: Fn([f64; FACET_LANES]) -> Option<[f64; FACET_LANES]>,
    {
        type Output = Option<LaneCofactors>;

        #[inline(always)]
        fn with_simd<S: Simd>(self, _: S) -> Self::Output {
            let Self(simd, facets, factors) = self;
            edges::<R, C>(simd, facets, factors)
        }
    }

    #[inline(always)]
    fn edges<const R: usize, const C: usize>(
        simd: V3,
        facets: [&[&[f64]]; FACET_LANES],
        factors: &impl Fn([f64; FACET_LANES]) -> Option<[f64; FACET_LANES]>,
    ) -> Option<LaneCofactors> {
        if facets
            .iter()
            .any(|f| f.len() != C || f.iter().any(|p| p.len() != C))
        {
            return None;
        }
        let ops = Ops(simd);
        let zero = ops.splat(0.0);
        let mut x = [[zero; C]; C];
        // The maximum of magnitudes that are not NaN does not depend on the
        // order, so it equals the scalar fold of each facet.
        let mut largest = zero;
        for (i, row) in x.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = cast([
                    facets[0][i][j],
                    facets[1][i][j],
                    facets[2][i][j],
                    facets[3][i][j],
                ]);
                largest = simd.max_f64x4(largest, simd.abs_f64x4(*v));
            }
        }
        let scale: f64x4 = cast(factors(cast(largest))?);
        let mut m = [[ops.exact(zero); C]; R];
        for (row, point) in m.iter_mut().zip(&x[1..]) {
            for ((entry, &p), &o) in row.iter_mut().zip(point).zip(&x[0]) {
                *entry = ops.sub(
                    ops.exact(simd.mul_f64x4(p, scale)),
                    ops.exact(simd.mul_f64x4(o, scale)),
                );
            }
        }
        let all = simd.cmp_eq_i64x4(ops.index(0), ops.index(0));
        let mut certain = all;
        let mut odd = simd.xor_m64x4(all, all);
        for col in 0..R {
            // The pivot of each lane: the last row of largest magnitude.
            let mut best = ops.index(col);
            let mut most = ops.magnitude_bits(m[col][col].value);
            for (r, row) in m.iter().enumerate().skip(col + 1) {
                let bits = ops.magnitude_bits(row[col].value);
                let take = simd.cmp_ge_i64x4(bits, most);
                most = simd.select_i64x4(take, bits, most);
                best = simd.select_i64x4(take, ops.index(r), best);
            }
            // `best` starts at `col` and only moves down.
            let swapped = simd.cmp_gt_i64x4(best, ops.index(col));
            odd = simd.xor_m64x4(odd, swapped);
            // Each lane swaps its pivot row in, by selection.
            let (above, below) = m.split_at_mut(col + 1);
            let pivot_row = &mut above[col];
            for (offset, row) in below.iter_mut().enumerate() {
                let swap = simd.cmp_eq_i64x4(best, ops.index(col + 1 + offset));
                for (a, b) in pivot_row.iter_mut().zip(row.iter_mut()) {
                    let (old_a, old_b) = (*a, *b);
                    *a = ops.select(swap, old_b, old_a);
                    *b = ops.select(swap, old_a, old_b);
                }
            }
            let pivot_row = &*pivot_row;
            let pivot = pivot_row[col];
            for row in below.iter_mut() {
                let factor = ops.div(row[col], pivot, &mut certain);
                // The whole row, a loop of fixed length; the entries at and
                // left of the pivot column are never read again.
                for (entry, &up) in row.iter_mut().zip(pivot_row) {
                    *entry = ops.sub(*entry, ops.mul(factor, up));
                }
            }
        }
        let mut det = ops.exact(ops.splat(1.0));
        for (i, row) in m.iter().enumerate() {
            det = ops.mul(det, row[i]);
        }
        det.value = simd.select_f64x4(odd, ops.negated(det.value), det.value);
        // Back substitution for T x = u, u the last column.
        let mut out = [ops.exact(zero); C];
        for i in (0..R).rev() {
            let mut sum = m[i][R];
            for j in i + 1..R {
                sum = ops.sub(sum, ops.mul(m[i][j], out[j]));
            }
            out[i] = ops.div(sum, m[i][i], &mut certain);
        }
        for value in out.iter_mut().take(R) {
            let product = ops.mul(det, *value);
            value.value = ops.negated(product.value);
            value.error = product.error;
        }
        out[R] = det;
        // A value or bound is finite when its magnitude is below infinity;
        // NaN compares false.
        let infinity = ops.splat(f64::INFINITY);
        for p in &out {
            let finite = simd.and_m64x4(
                simd.cmp_lt_f64x4(simd.abs_f64x4(p.value), infinity),
                simd.cmp_lt_f64x4(simd.abs_f64x4(p.error), infinity),
            );
            certain = simd.and_m64x4(certain, finite);
        }
        let sure: [u64; FACET_LANES] = cast(certain);
        let values: [[f64; FACET_LANES]; C] = out.map(|p| cast(p.value));
        let errors: [[f64; FACET_LANES]; C] = out.map(|p| cast(p.error));
        Some(core::array::from_fn(|lane| {
            (sure[lane] != 0).then(|| {
                values
                    .iter()
                    .zip(&errors)
                    .map(|(v, e)| (v[lane], e[lane]))
                    .collect()
            })
        }))
    }
}

/// Lanes of one block of a row.
const LANES: usize = 4;

/// Runs an elimination over blocks of [`LANES`] values under `V3` (AVX2)
/// when the CPU has it, otherwise under [`Arch`]'s choice.
///
/// A block is one 256-bit register, so a wider instruction set does not
/// shorten a row update. Under `V4` (AVX-512) the 9 x 9 determinant on
/// the stack took about 24 times as long as under `V3`, and the heap
/// 10 x 10 about 1.3 times; `V3` was no slower at any size from 5 to 13
/// (#195). The code path and every operation are the same, so the value
/// and the bound do not depend on the instruction set.
#[inline(always)]
fn in_blocks<W: WithSimd>(op: W) -> W::Output {
    #[cfg(target_arch = "x86_64")]
    if let Some(v3) = pulp::x86::V3::try_new() {
        return Simd::vectorize(v3, op);
    }
    Arch::new().dispatch(op)
}

/// Four adjacent entries of one row.
type Block = [f64; LANES];

/// The bound a padding entry starts with. Padding is never read; a bound
/// of a few `ETA` there would make each update's products subnormal, and
/// every subnormal operation costs a microcode assist.
const PAD_ERROR: f64 = 1.0;

/// The rows of an elimination: values and bounds apart, each row padded
/// with zero entries to whole blocks, so that one row update is a loop of
/// fixed-width block operations (see [`update`]).
trait Rows {
    fn rows(&self) -> usize;
    fn blocks(&self) -> usize;
    fn parts(&mut self) -> (&mut [Block], &mut [Block]);
}

/// `R` rows of `B` blocks on the stack.
struct Stack<const R: usize, const B: usize> {
    value: [[Block; B]; R],
    error: [[Block; B]; R],
}

impl<const R: usize, const B: usize> Stack<R, B> {
    fn load(rows: usize, cols: usize, entry: impl Fn(usize, usize) -> Approx) -> Self {
        debug_assert!(
            rows == R && cols.div_ceil(LANES) == B,
            "the matrix fills the array"
        );
        let mut m = Self {
            value: [[[0.0; LANES]; B]; R],
            error: [[[PAD_ERROR; LANES]; B]; R],
        };
        for i in 0..R {
            for j in 0..cols {
                let a = entry(i, j);
                m.value[i][j / LANES][j % LANES] = a.value;
                m.error[i][j / LANES][j % LANES] = a.error;
            }
        }
        m
    }
}

impl<const R: usize, const B: usize> Rows for Stack<R, B> {
    fn rows(&self) -> usize {
        R
    }

    fn blocks(&self) -> usize {
        B
    }

    fn parts(&mut self) -> (&mut [Block], &mut [Block]) {
        (self.value.as_flattened_mut(), self.error.as_flattened_mut())
    }
}

/// Rows of any width in one vector each for values and bounds.
struct Heap {
    rows: usize,
    blocks: usize,
    value: Vec<Block>,
    error: Vec<Block>,
}

impl Heap {
    fn load(rows: usize, cols: usize, entry: impl Fn(usize, usize) -> Approx) -> Self {
        let blocks = cols.div_ceil(LANES);
        let mut value = vec![[0.0; LANES]; rows * blocks];
        let mut error = vec![[PAD_ERROR; LANES]; rows * blocks];
        for i in 0..rows {
            for j in 0..cols {
                let a = entry(i, j);
                value[i * blocks + j / LANES][j % LANES] = a.value;
                error[i * blocks + j / LANES][j % LANES] = a.error;
            }
        }
        Self {
            rows,
            blocks,
            value,
            error,
        }
    }
}

impl Rows for Heap {
    fn rows(&self) -> usize {
        self.rows
    }

    fn blocks(&self) -> usize {
        self.blocks
    }

    fn parts(&mut self) -> (&mut [Block], &mut [Block]) {
        (&mut self.value, &mut self.error)
    }
}

/// Row swaps on the stack up to this many rows, on the heap beyond.
const STACK_SWAPS: usize = 16;

/// Per column of an elimination, whether a row swap brought its pivot in.
enum Swaps {
    Stack([bool; STACK_SWAPS], usize),
    Heap(Vec<bool>),
}

impl Swaps {
    fn new(rows: usize) -> Self {
        if rows <= STACK_SWAPS {
            Self::Stack([false; STACK_SWAPS], rows)
        } else {
            Self::Heap(vec![false; rows])
        }
    }

    fn as_mut(&mut self) -> &mut [bool] {
        match self {
            Self::Stack(swaps, rows) => &mut swaps[..*rows],
            Self::Heap(swaps) => swaps,
        }
    }
}

/// [`determinant`], run by [`in_blocks`].
struct Square<M>(M);

impl<M: Rows> WithSimd for Square<M> {
    type Output = Option<Approx>;

    #[inline(always)]
    fn with_simd<S: Simd>(mut self, _simd: S) -> Self::Output {
        let n = self.0.rows();
        let b = self.0.blocks();
        let mut swaps = Swaps::new(n);
        let swaps = swaps.as_mut();
        let (value, error) = self.0.parts();
        eliminate(value, error, n, b, swaps)?;
        // Each pivot is final once its column is eliminated, so the
        // product in column order repeats the entry-wise elimination's.
        let mut det = Approx::exact(1.0);
        for (col, &swapped) in swaps.iter().enumerate() {
            if swapped {
                det = det.negated();
            }
            det = det.mul(at(value, error, b, col, col));
        }
        Some(det)
    }
}

/// [`cofactors`], run by [`in_blocks`].
struct Shared<M>(M);

impl<M: Rows> WithSimd for Shared<M> {
    type Output = Option<Small<Approx, 10>>;

    #[inline(always)]
    fn with_simd<S: Simd>(mut self, _simd: S) -> Self::Output {
        let m = self.0.rows();
        let b = self.0.blocks();
        let mut swaps = Swaps::new(m);
        let swaps = swaps.as_mut();
        let (value, error) = self.0.parts();
        eliminate(value, error, m, b, swaps)?;
        let entry = |i: usize, j: usize| at(value, error, b, i, j);
        let mut det = Approx::exact(1.0);
        for i in 0..m {
            det = det.mul(entry(i, i));
        }
        if swaps.iter().filter(|&&s| s).count() % 2 == 1 {
            det = det.negated();
        }
        // Back substitution for T x = u, u the last column, written into the
        // first m entries of the result.
        let mut out: Small<Approx, 10> = (0..=m).map(|_| Approx::exact(0.0)).collect();
        for i in (0..m).rev() {
            let mut sum = entry(i, m);
            for (j, &xj) in out.iter().enumerate().take(m).skip(i + 1) {
                sum = sum.sub(entry(i, j).mul(xj));
            }
            out[i] = sum.div(entry(i, i))?;
        }
        for value in out.iter_mut().take(m) {
            *value = det.mul(*value).negated();
        }
        out[m] = det;
        Some(out)
    }
}

/// Entry `(i, j)` of rows of `b` blocks.
#[inline(always)]
fn at(value: &[Block], error: &[Block], b: usize, i: usize, j: usize) -> Approx {
    let block = i * b + j / LANES;
    Approx {
        value: value[block][j % LANES],
        error: error[block][j % LANES],
    }
}

/// Gaussian elimination with partial pivoting over the first `rows`
/// columns of `rows` rows of `b` blocks. `swaps[col]` records whether
/// column `col` swapped its pivot row in. Returns `None` when a divisor's
/// sign is not certain.
///
/// Every value and bound that is read later is the one the entry-wise
/// elimination with [`Approx::mul`] and [`Approx::sub`] computes, bit for
/// bit: the pivot is the last row of largest magnitude, and each update is
/// [`update`]. A row update starts at the block of the pivot column, so it
/// also rewrites the entries before that column in the block and the zero
/// padding; those entries are never read again.
#[inline(always)]
fn eliminate(
    value: &mut [Block],
    error: &mut [Block],
    rows: usize,
    b: usize,
    swaps: &mut [bool],
) -> Option<()> {
    for col in 0..rows {
        let mut best = col;
        for r in col + 1..rows {
            let magnitude = at(value, error, b, r, col).value.abs();
            if magnitude.total_cmp(&at(value, error, b, best, col).value.abs()) != Ordering::Less {
                best = r;
            }
        }
        if best != col {
            swap_rows(value, b, best, col);
            swap_rows(error, b, best, col);
            swaps[col] = true;
        }
        let pivot = at(value, error, b, col, col);
        let first = col / LANES;
        let (value_above, value_below) = value.split_at_mut((col + 1) * b);
        let (error_above, error_below) = error.split_at_mut((col + 1) * b);
        let pivot_value = &value_above[col * b + first..];
        let pivot_error = &error_above[col * b + first..];
        for (row_value, row_error) in value_below
            .chunks_exact_mut(b)
            .zip(error_below.chunks_exact_mut(b))
        {
            let factor = Approx {
                value: row_value[first][col % LANES],
                error: row_error[first][col % LANES],
            }
            .div(pivot)?;
            update(
                &mut row_value[first..],
                &mut row_error[first..],
                factor,
                pivot_value,
                pivot_error,
            );
        }
    }
    Some(())
}

fn swap_rows(m: &mut [Block], b: usize, r: usize, s: usize) {
    let (low, high) = (r.min(s), r.max(s));
    let (head, tail) = m.split_at_mut(high * b);
    head[low * b..(low + 1) * b].swap_with_slice(&mut tail[..b]);
}

/// `row - factor * above`, entry by entry: [`Approx::mul`] of the factor
/// and the entry above, then [`Approx::sub`] from the entry, in the same
/// order of operations.
#[inline(always)]
fn update(
    row_value: &mut [Block],
    row_error: &mut [Block],
    factor: Approx,
    above_value: &[Block],
    above_error: &[Block],
) {
    let magnitude = factor.value.abs();
    for (((v, e), av), ae) in row_value
        .iter_mut()
        .zip(row_error.iter_mut())
        .zip(above_value)
        .zip(above_error)
    {
        for lane in 0..LANES {
            let product = factor.value * av[lane];
            let product_error = (magnitude * ae[lane]
                + av[lane].abs() * factor.error
                + factor.error * ae[lane]
                + product.abs() * UNIT_ROUNDOFF
                + ETA)
                * GROW;
            let difference = v[lane] - product;
            e[lane] = (e[lane] + product_error + difference.abs() * UNIT_ROUNDOFF + ETA) * GROW;
            v[lane] = difference;
        }
    }
}
