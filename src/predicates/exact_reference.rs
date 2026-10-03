//! The exact stage before #96, with 32-bit limbs and a new `Vec` per
//! operation: an independent reference for the tests of the current one.
#![allow(dead_code)]

use core::cmp::Ordering;

use super::super::{ExactEvaluationExhausted, Sign};
use super::{decompose, try_vec};

/// A signed integer of arbitrary size.
///
/// The magnitude is stored as little-endian 32-bit limbs without high zero
/// limbs. Zero has an empty magnitude and is never negative.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RefInt {
    negative: bool,
    magnitude: Vec<u32>,
}

impl RefInt {
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

    /// Number of significant bits of the magnitude.
    fn bit_length(&self) -> u64 {
        match self.magnitude.last() {
            Some(&top) => {
                (self.magnitude.len() as u64 - 1) * 32 + u64::from(32 - top.leading_zeros())
            }
            None => 0,
        }
    }

    /// `self / 2^shift`, truncated to the 64 bits from position `shift` and
    /// then rounded to `f64`. Exact enough for a direction when `shift` is
    /// the bit length of the largest value of a vector minus 64.
    fn to_f64_shifted(&self, shift: u64) -> f64 {
        let mut bits = 0_u64;
        for (offset, slot) in (0..64_u64).enumerate() {
            let position = shift + slot;
            let limb = self
                .magnitude
                .get((position / 32) as usize)
                .copied()
                .unwrap_or(0);
            if (limb >> (position % 32)) & 1 == 1 {
                bits |= 1 << offset;
            }
        }
        let value = bits as f64;
        if self.negative {
            -value
        } else {
            value
        }
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

/// Determinant of a square integer matrix by the Berkowitz algorithm.
///
/// The algorithm builds the characteristic polynomial det(xI - A) of each
/// leading principal submatrix from the previous one with a Toeplitz matrix,
/// using O(n^4) ring operations and no division. The constant term of the
/// polynomial of the full matrix is (-1)^n det(A).
pub(super) fn determinant(a: &[Vec<RefInt>]) -> Result<RefInt, ExactEvaluationExhausted> {
    let n = a.len();
    if n == 0 {
        return RefInt::one();
    }
    // Coefficients of det(xI - A_r), highest power first, for the leading
    // r x r submatrix A_r. Starts with r = 1.
    let mut poly: Vec<RefInt> = try_vec(2)?;
    poly.push(RefInt::one()?);
    poly.push(a[0][0].try_clone()?.negated());

    for r in 1..n {
        // First column of the Toeplitz matrix: 1, -a_rr, -R C, -R A_r C, ...,
        // -R A_r^(r-1) C, where R is row r and C is column r restricted to
        // the leading r entries.
        let mut column: Vec<RefInt> = try_vec(r + 2)?;
        column.push(RefInt::one()?);
        column.push(a[r][r].try_clone()?.negated());
        let mut v: Vec<RefInt> = try_vec(r)?;
        for row in a.iter().take(r) {
            v.push(row[r].try_clone()?);
        }
        for step in 0..r {
            let mut dot = RefInt::zero();
            for (x, y) in a[r][..r].iter().zip(&v) {
                dot = dot.add(&x.mul(y)?)?;
            }
            column.push(dot.negated());
            if step + 1 < r {
                let mut next: Vec<RefInt> = try_vec(r)?;
                for row in a.iter().take(r) {
                    let mut sum = RefInt::zero();
                    for (x, y) in row[..r].iter().zip(&v) {
                        sum = sum.add(&x.mul(y)?)?;
                    }
                    next.push(sum);
                }
                v = next;
            }
        }
        // poly <- T * poly, T lower-triangular Toeplitz of size (r+2) x (r+1).
        let mut next_poly: Vec<RefInt> = try_vec(r + 2)?;
        for i in 0..r + 2 {
            let mut sum = RefInt::zero();
            for (j, p) in poly.iter().enumerate().take(i + 1) {
                sum = sum.add(&column[i - j].mul(p)?)?;
            }
            next_poly.push(sum);
        }
        poly = next_poly;
    }

    let constant = poly.pop().unwrap_or_else(RefInt::zero);
    Ok(if n % 2 == 1 {
        constant.negated()
    } else {
        constant
    })
}

impl RefInt {
    /// The magnitude as 64-bit limbs, and the sign, to compare values.
    pub(super) fn parts(&self) -> (bool, Vec<u64>) {
        let mut limbs: Vec<u64> = self
            .magnitude
            .chunks(2)
            .map(|c| u64::from(c[0]) | (u64::from(c.get(1).copied().unwrap_or(0)) << 32))
            .collect();
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        (self.negative, limbs)
    }
}
