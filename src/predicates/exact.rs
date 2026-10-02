//! Exact sign of a determinant whose entries are differences of `f64` values.
//!
//! Every finite `f64` is an integer multiple of 2^-1074, so scaling every
//! coordinate by 2^1074 turns it into an integer without rounding. The
//! determinant of the scaled differences is then evaluated over the integers
//! with the division-free Berkowitz algorithm, so only addition, subtraction,
//! and multiplication of big integers are needed. Scaling every entry by the
//! same positive power of two does not change the sign of the determinant.
//!
//! Every buffer is reserved with `try_reserve_exact`. A failed reservation is
//! reported as [`ExactEvaluationExhausted`] instead of aborting.

use core::cmp::Ordering;

use super::{ExactEvaluationExhausted, Row, Rows, Sign};

/// Reserves an empty vector with room for `capacity` elements.
fn try_vec<T>(capacity: usize) -> Result<Vec<T>, ExactEvaluationExhausted> {
    let mut v = Vec::new();
    v.try_reserve_exact(capacity)
        .map_err(|_| ExactEvaluationExhausted)?;
    Ok(v)
}

/// Splits a finite `x` into `(mantissa, shift)` with
/// `|x| = mantissa * 2^(shift - 1074)`.
///
/// A normal value is (2^52 + fraction) * 2^(biased - 1075), so its shift is
/// biased - 1. A subnormal value is fraction * 2^-1074, so its shift is 0.
fn decompose(x: f64) -> (u64, u64) {
    let bits = x.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let fraction = bits & ((1_u64 << 52) - 1);
    if biased == 0 {
        (fraction, 0)
    } else {
        (fraction | (1_u64 << 52), biased - 1)
    }
}

/// The exponent shift of `x` (see [`decompose`]), or `None` for zero.
pub(super) fn exponent_shift(x: f64) -> Option<u64> {
    let (mantissa, shift) = decompose(x);
    (mantissa != 0).then_some(shift)
}

/// A signed integer of arbitrary size.
///
/// The magnitude is stored as little-endian 32-bit limbs without high zero
/// limbs. Zero has an empty magnitude and is never negative.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct BigInt {
    negative: bool,
    magnitude: Vec<u32>,
}

impl BigInt {
    fn zero() -> Self {
        Self {
            negative: false,
            magnitude: Vec::new(),
        }
    }

    fn one() -> Result<Self, ExactEvaluationExhausted> {
        let mut magnitude = try_vec(1)?;
        magnitude.push(1);
        Ok(Self {
            negative: false,
            magnitude,
        })
    }

    fn from_parts(negative: bool, mut magnitude: Vec<u32>) -> Self {
        while magnitude.last() == Some(&0) {
            magnitude.pop();
        }
        let negative = negative && !magnitude.is_empty();
        Self {
            negative,
            magnitude,
        }
    }

    /// Returns `x * 2^(1074 - base)` as an integer. `x` must be finite and
    /// `base` must not exceed [`exponent_shift`] of any nonzero `x` converted
    /// with it.
    pub(super) fn from_f64_scaled(x: f64, base: u64) -> Result<Self, ExactEvaluationExhausted> {
        debug_assert!(x.is_finite(), "predicate input must be finite");
        let negative = x.to_bits() >> 63 == 1;
        let (mantissa, shift) = decompose(x);
        if mantissa == 0 {
            return Ok(Self::zero());
        }
        debug_assert!(shift >= base, "scale base above an input exponent");
        let shift = shift - base;
        let limb_shift = (shift / 32) as usize;
        let bit_shift = (shift % 32) as u32;
        let wide = u128::from(mantissa) << bit_shift;
        let mut magnitude = try_vec(limb_shift + 3)?;
        magnitude.resize(limb_shift, 0);
        magnitude.push(wide as u32);
        magnitude.push((wide >> 32) as u32);
        magnitude.push((wide >> 64) as u32);
        Ok(Self::from_parts(negative, magnitude))
    }

    pub(super) fn sign(&self) -> Sign {
        if self.magnitude.is_empty() {
            Sign::Zero
        } else if self.negative {
            Sign::Negative
        } else {
            Sign::Positive
        }
    }

    fn try_clone(&self) -> Result<Self, ExactEvaluationExhausted> {
        let mut magnitude = try_vec(self.magnitude.len())?;
        magnitude.extend_from_slice(&self.magnitude);
        Ok(Self {
            negative: self.negative,
            magnitude,
        })
    }

    fn negated(mut self) -> Self {
        self.negative = !self.negative && !self.magnitude.is_empty();
        self
    }

    fn signed_add(
        &self,
        other: &Self,
        other_negative: bool,
    ) -> Result<Self, ExactEvaluationExhausted> {
        if self.negative == other_negative {
            let magnitude = add_magnitude(&self.magnitude, &other.magnitude)?;
            return Ok(Self::from_parts(self.negative, magnitude));
        }
        match compare_magnitude(&self.magnitude, &other.magnitude) {
            Ordering::Equal => Ok(Self::zero()),
            Ordering::Greater => {
                let magnitude = sub_magnitude(&self.magnitude, &other.magnitude)?;
                Ok(Self::from_parts(self.negative, magnitude))
            }
            Ordering::Less => {
                let magnitude = sub_magnitude(&other.magnitude, &self.magnitude)?;
                Ok(Self::from_parts(other_negative, magnitude))
            }
        }
    }

    pub(super) fn add(&self, other: &Self) -> Result<Self, ExactEvaluationExhausted> {
        self.signed_add(other, other.negative)
    }

    pub(super) fn sub(&self, other: &Self) -> Result<Self, ExactEvaluationExhausted> {
        let other_negative = !other.negative && !other.magnitude.is_empty();
        self.signed_add(other, other_negative)
    }

    pub(super) fn mul(&self, other: &Self) -> Result<Self, ExactEvaluationExhausted> {
        if self.magnitude.is_empty() || other.magnitude.is_empty() {
            return Ok(Self::zero());
        }
        let magnitude = mul_magnitude(&self.magnitude, &other.magnitude)?;
        Ok(Self::from_parts(self.negative != other.negative, magnitude))
    }
}

fn compare_magnitude(a: &[u32], b: &[u32]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_magnitude(a: &[u32], b: &[u32]) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = try_vec(long.len() + 1)?;
    let mut carry = 0_u64;
    for (i, &limb) in long.iter().enumerate() {
        let sum = u64::from(limb) + u64::from(short.get(i).copied().unwrap_or(0)) + carry;
        out.push(sum as u32);
        carry = sum >> 32;
    }
    if carry != 0 {
        out.push(carry as u32);
    }
    Ok(out)
}

/// `a - b` for `a >= b`.
fn sub_magnitude(a: &[u32], b: &[u32]) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let mut out = try_vec(a.len())?;
    let mut borrow = 0_i64;
    for (i, &limb) in a.iter().enumerate() {
        let mut diff = i64::from(limb) - i64::from(b.get(i).copied().unwrap_or(0)) - borrow;
        borrow = 0;
        if diff < 0 {
            diff += 1 << 32;
            borrow = 1;
        }
        out.push(diff as u32);
    }
    debug_assert_eq!(borrow, 0, "sub_magnitude requires a >= b");
    Ok(out)
}

fn mul_magnitude(a: &[u32], b: &[u32]) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let mut out = try_vec(a.len() + b.len())?;
    out.resize(a.len() + b.len(), 0);
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0_u64;
        for (j, &y) in b.iter().enumerate() {
            let cur = u64::from(out[i + j]) + u64::from(x) * u64::from(y) + carry;
            out[i + j] = cur as u32;
            carry = cur >> 32;
        }
        out[i + b.len()] = carry as u32;
    }
    Ok(out)
}

/// Exact sign of the determinant described by `rows`.
pub(super) fn sign_exact(rows: Rows<'_>) -> Result<Sign, ExactEvaluationExhausted> {
    let k = rows.k();
    // Every value is a multiple of 2^(base - 1074). Dividing all of them by
    // that power of two keeps the integers small and multiplies the
    // determinant by a positive factor, so the sign is unchanged.
    let base = rows.values().filter_map(exponent_shift).min().unwrap_or(0);
    let mut scaled_origin = try_vec(k)?;
    for &x in rows.origin {
        scaled_origin.push(BigInt::from_f64_scaled(x, base)?);
    }
    let mut matrix: Vec<Vec<BigInt>> = try_vec(k)?;
    for i in 0..k {
        let mut row = try_vec(k)?;
        match rows.row(i) {
            Row::Difference(point) => {
                for (&x, o) in point.iter().zip(&scaled_origin) {
                    row.push(BigInt::from_f64_scaled(x, base)?.sub(o)?);
                }
            }
            Row::Direction(direction) => {
                for &x in direction {
                    row.push(BigInt::from_f64_scaled(x, base)?);
                }
            }
        }
        matrix.push(row);
    }
    Ok(determinant(&matrix)?.sign())
}

/// Determinant of a square integer matrix by the Berkowitz algorithm.
///
/// The algorithm builds the characteristic polynomial det(xI - A) of each
/// leading principal submatrix from the previous one with a Toeplitz matrix,
/// using O(n^4) ring operations and no division. The constant term of the
/// polynomial of the full matrix is (-1)^n det(A).
pub(super) fn determinant(a: &[Vec<BigInt>]) -> Result<BigInt, ExactEvaluationExhausted> {
    let n = a.len();
    if n == 0 {
        return BigInt::one();
    }
    // Coefficients of det(xI - A_r), highest power first, for the leading
    // r x r submatrix A_r. Starts with r = 1.
    let mut poly: Vec<BigInt> = try_vec(2)?;
    poly.push(BigInt::one()?);
    poly.push(a[0][0].try_clone()?.negated());

    for r in 1..n {
        // First column of the Toeplitz matrix: 1, -a_rr, -R C, -R A_r C, ...,
        // -R A_r^(r-1) C, where R is row r and C is column r restricted to
        // the leading r entries.
        let mut column: Vec<BigInt> = try_vec(r + 2)?;
        column.push(BigInt::one()?);
        column.push(a[r][r].try_clone()?.negated());
        let mut v: Vec<BigInt> = try_vec(r)?;
        for row in a.iter().take(r) {
            v.push(row[r].try_clone()?);
        }
        for step in 0..r {
            let mut dot = BigInt::zero();
            for (x, y) in a[r][..r].iter().zip(&v) {
                dot = dot.add(&x.mul(y)?)?;
            }
            column.push(dot.negated());
            if step + 1 < r {
                let mut next: Vec<BigInt> = try_vec(r)?;
                for row in a.iter().take(r) {
                    let mut sum = BigInt::zero();
                    for (x, y) in row[..r].iter().zip(&v) {
                        sum = sum.add(&x.mul(y)?)?;
                    }
                    next.push(sum);
                }
                v = next;
            }
        }
        // poly <- T * poly, T lower-triangular Toeplitz of size (r+2) x (r+1).
        let mut next_poly: Vec<BigInt> = try_vec(r + 2)?;
        for i in 0..r + 2 {
            let mut sum = BigInt::zero();
            for (j, p) in poly.iter().enumerate().take(i + 1) {
                sum = sum.add(&column[i - j].mul(p)?)?;
            }
            next_poly.push(sum);
        }
        poly = next_poly;
    }

    let constant = poly.pop().unwrap_or_else(BigInt::zero);
    Ok(if n % 2 == 1 {
        constant.negated()
    } else {
        constant
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(x: i64) -> BigInt {
        // Small integers are exact in f64; undo nothing, the scale is shared.
        BigInt::from_f64_scaled(x as f64, 0).unwrap()
    }

    #[test]
    fn scaled_conversion_keeps_order_and_sign() {
        let values = [
            -f64::MAX,
            -1.5,
            -f64::MIN_POSITIVE,
            -f64::from_bits(1),
            0.0,
            -0.0,
            f64::from_bits(1),
            f64::from_bits(2),
            f64::MIN_POSITIVE,
            1.0,
            3.0,
            f64::MAX,
        ];
        for w in values.windows(2) {
            let a = BigInt::from_f64_scaled(w[0], 0).unwrap();
            let b = BigInt::from_f64_scaled(w[1], 0).unwrap();
            let diff = b.sub(&a).unwrap().sign();
            let expected = if w[0] == w[1] {
                Sign::Zero
            } else {
                Sign::Positive
            };
            assert_eq!(diff, expected, "{} -> {}", w[0], w[1]);
        }
    }

    #[test]
    fn smallest_subnormal_is_one() {
        let one = BigInt::from_f64_scaled(f64::from_bits(1), 0).unwrap();
        assert_eq!(one, BigInt::one().unwrap());
    }

    #[test]
    fn arithmetic_matches_i128() {
        let samples = [-7_i64, -3, -1, 0, 1, 2, 5, 1 << 20, -(1 << 25)];
        for &x in &samples {
            for &y in &samples {
                let (bx, by) = (int(x), int(y));
                let scale = BigInt::from_f64_scaled(1.0, 0).unwrap();
                assert_eq!(bx.add(&by).unwrap(), int(x + y));
                assert_eq!(bx.sub(&by).unwrap(), int(x - y));
                // (x s)(y s) = (x y s) s
                assert_eq!(
                    bx.mul(&by).unwrap(),
                    int(x * y).mul(&scale).unwrap(),
                    "{x} * {y}"
                );
            }
        }
    }

    fn det_i128(m: &[Vec<i128>]) -> i128 {
        let n = m.len();
        if n == 0 {
            return 1;
        }
        let mut total = 0;
        for c in 0..n {
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
            let term = m[0][c] * det_i128(&minor);
            total += if c % 2 == 0 { term } else { -term };
        }
        total
    }

    #[test]
    fn berkowitz_matches_cofactor_expansion() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 11) as i128 - 5
        };
        for n in 1..=6 {
            for _ in 0..40 {
                let m: Vec<Vec<i128>> = (0..n).map(|_| (0..n).map(|_| next()).collect()).collect();
                let big: Vec<Vec<BigInt>> = m
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|&v| BigInt::from_f64_scaled(v as f64, 0).unwrap())
                            .collect()
                    })
                    .collect();
                let expected = det_i128(&m);
                let got = determinant(&big).unwrap().sign();
                let want = match expected.cmp(&0) {
                    Ordering::Less => Sign::Negative,
                    Ordering::Equal => Sign::Zero,
                    Ordering::Greater => Sign::Positive,
                };
                assert_eq!(got, want, "{m:?}");
            }
        }
    }
}
