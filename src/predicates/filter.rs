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

use super::Sign;

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;
/// Two units in the last place of the smallest subnormal (2^-1073).
const ETA: f64 = f64::from_bits(2);
const GROW: f64 = 1.0 + 16.0 * UNIT_ROUNDOFF;

/// A computed `f64` value with an absolute bound on its error.
#[derive(Clone, Copy, Debug)]
pub(super) struct Approx {
    value: f64,
    error: f64,
}

impl Approx {
    pub(super) fn exact(value: f64) -> Self {
        Self { value, error: 0.0 }
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

/// Filtered determinant for any size by Gaussian elimination with partial
/// pivoting. Returns `None` when a pivot's sign is not certain.
pub(super) fn determinant(mut m: Vec<Vec<Approx>>) -> Option<Approx> {
    let n = m.len();
    let mut det = Approx::exact(1.0);
    for col in 0..n {
        let best =
            (col..n).max_by(|&a, &b| m[a][col].value.abs().total_cmp(&m[b][col].value.abs()))?;
        if best != col {
            m.swap(best, col);
            det = Approx {
                value: -det.value,
                error: det.error,
            };
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
