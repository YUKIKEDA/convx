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
        // factor (1 + u), and the product by (1 - 4u) undoes it.
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
        5 => Arch::new().dispatch(Square(Stack::<5, 2>::load(5, 5, entry))),
        6 => Arch::new().dispatch(Square(Stack::<6, 2>::load(6, 6, entry))),
        7 => Arch::new().dispatch(Square(Stack::<7, 2>::load(7, 7, entry))),
        8 => Arch::new().dispatch(Square(Stack::<8, 2>::load(8, 8, entry))),
        9 => Arch::new().dispatch(Square(Stack::<9, 3>::load(9, 9, entry))),
        _ => Arch::new().dispatch(Square(Heap::load(n, n, entry))),
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
        5 => Arch::new().dispatch(Shared(Stack::<4, 2>::load(4, 5, entry))),
        6 => Arch::new().dispatch(Shared(Stack::<5, 2>::load(5, 6, entry))),
        7 => Arch::new().dispatch(Shared(Stack::<6, 2>::load(6, 7, entry))),
        8 => Arch::new().dispatch(Shared(Stack::<7, 2>::load(7, 8, entry))),
        9 => Arch::new().dispatch(Shared(Stack::<8, 3>::load(8, 9, entry))),
        _ => Arch::new().dispatch(Shared(Heap::load(k - 1, k, entry))),
    }
}

/// Lanes of one block of a row.
const LANES: usize = 4;

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

/// [`determinant`] under the widest instruction set the CPU runs.
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

/// [`cofactors`] under the widest instruction set the CPU runs.
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
