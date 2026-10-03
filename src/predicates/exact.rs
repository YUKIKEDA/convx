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

/// Grows `v` so that it can hold `total` elements without reallocating.
fn try_reserve_total<T>(v: &mut Vec<T>, total: usize) -> Result<(), ExactEvaluationExhausted> {
    if total > v.capacity() {
        v.try_reserve(total - v.len())
            .map_err(|_| ExactEvaluationExhausted)?;
    }
    Ok(())
}

/// A signed integer of arbitrary size.
///
/// The magnitude is stored as little-endian 64-bit limbs without high zero
/// limbs. Zero has an empty magnitude and is never negative.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct BigInt {
    negative: bool,
    magnitude: Vec<u64>,
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

    fn from_parts(negative: bool, mut magnitude: Vec<u64>) -> Self {
        trim(&mut magnitude);
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
        let limb_shift = (shift / 64) as usize;
        let bit_shift = (shift % 64) as u32;
        let wide = u128::from(mantissa) << bit_shift;
        let mut magnitude = try_vec(limb_shift + 2)?;
        magnitude.resize(limb_shift, 0);
        magnitude.push(wide as u64);
        magnitude.push((wide >> 64) as u64);
        Ok(Self::from_parts(negative, magnitude))
    }

    /// Number of significant bits of the magnitude.
    fn bit_length(&self) -> u64 {
        match self.magnitude.last() {
            Some(&top) => {
                (self.magnitude.len() as u64 - 1) * 64 + u64::from(64 - top.leading_zeros())
            }
            None => 0,
        }
    }

    /// `self / 2^shift`, truncated to the 64 bits from position `shift` and
    /// then rounded to `f64`. Exact enough for a direction when `shift` is
    /// the bit length of the largest value of a vector minus 64.
    fn to_f64_shifted(&self, shift: u64) -> f64 {
        let limb = |i: u64| self.magnitude.get(i as usize).copied().unwrap_or(0);
        let (index, offset) = (shift / 64, (shift % 64) as u32);
        let bits = if offset == 0 {
            limb(index)
        } else {
            (limb(index) >> offset) | (limb(index + 1) << (64 - offset))
        };
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

    /// Adds the magnitude `other` with sign `negative` in place.
    fn accumulate(
        &mut self,
        other: &[u64],
        negative: bool,
    ) -> Result<(), ExactEvaluationExhausted> {
        if other.is_empty() {
            return Ok(());
        }
        if self.magnitude.is_empty() || self.negative == negative {
            if self.magnitude.is_empty() {
                self.negative = negative;
            }
            return add_assign_magnitude(&mut self.magnitude, other);
        }
        match compare_magnitude(&self.magnitude, other) {
            Ordering::Equal => {
                self.magnitude.clear();
                self.negative = false;
            }
            Ordering::Greater => sub_assign_magnitude(&mut self.magnitude, other),
            Ordering::Less => {
                reverse_sub_assign_magnitude(&mut self.magnitude, other)?;
                self.negative = negative;
            }
        }
        Ok(())
    }

    /// `self += sign * x * y`, where `sign` is -1 when `negate`. `scratch` is
    /// reused across calls, so a dot product allocates only as its
    /// accumulator grows.
    fn add_product(
        &mut self,
        x: &Self,
        y: &Self,
        negate: bool,
        scratch: &mut Vec<u64>,
    ) -> Result<(), ExactEvaluationExhausted> {
        if x.magnitude.is_empty() || y.magnitude.is_empty() {
            return Ok(());
        }
        mul_into(scratch, &x.magnitude, &y.magnitude)?;
        self.accumulate(scratch, (x.negative != y.negative) != negate)
    }

    #[cfg(test)]
    pub(super) fn add(&self, other: &Self) -> Result<Self, ExactEvaluationExhausted> {
        let mut sum = self.try_clone()?;
        sum.accumulate(&other.magnitude, other.negative)?;
        Ok(sum)
    }

    pub(super) fn sub(&self, other: &Self) -> Result<Self, ExactEvaluationExhausted> {
        let mut difference = self.try_clone()?;
        difference.accumulate(&other.magnitude, !other.negative)?;
        Ok(difference)
    }

    #[cfg(test)]
    pub(super) fn mul(&self, other: &Self) -> Result<Self, ExactEvaluationExhausted> {
        let mut product = Self::zero();
        let mut scratch = Vec::new();
        product.add_product(self, other, false, &mut scratch)?;
        Ok(product)
    }
}

fn trim(magnitude: &mut Vec<u64>) {
    while magnitude.last() == Some(&0) {
        magnitude.pop();
    }
}

fn compare_magnitude(a: &[u64], b: &[u64]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

/// `acc += b`.
fn add_assign_magnitude(acc: &mut Vec<u64>, b: &[u64]) -> Result<(), ExactEvaluationExhausted> {
    let len = acc.len().max(b.len());
    try_reserve_total(acc, len + 1)?;
    acc.resize(len, 0);
    let mut carry = false;
    for (i, slot) in acc.iter_mut().enumerate() {
        let (sum, c1) = slot.overflowing_add(b.get(i).copied().unwrap_or(0));
        let (sum, c2) = sum.overflowing_add(u64::from(carry));
        *slot = sum;
        carry = c1 || c2;
    }
    if carry {
        acc.push(1);
    }
    Ok(())
}

/// `acc -= b` for `acc > b`.
fn sub_assign_magnitude(acc: &mut Vec<u64>, b: &[u64]) {
    let mut borrow = false;
    for (i, slot) in acc.iter_mut().enumerate() {
        let (diff, b1) = slot.overflowing_sub(b.get(i).copied().unwrap_or(0));
        let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
        *slot = diff;
        borrow = b1 || b2;
    }
    debug_assert!(!borrow, "sub_assign_magnitude requires acc > b");
    trim(acc);
}

/// `acc = b - acc` for `b > acc`.
fn reverse_sub_assign_magnitude(
    acc: &mut Vec<u64>,
    b: &[u64],
) -> Result<(), ExactEvaluationExhausted> {
    try_reserve_total(acc, b.len())?;
    acc.resize(b.len(), 0);
    let mut borrow = false;
    for (slot, &limb) in acc.iter_mut().zip(b) {
        let (diff, b1) = limb.overflowing_sub(*slot);
        let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
        *slot = diff;
        borrow = b1 || b2;
    }
    debug_assert!(!borrow, "reverse_sub_assign_magnitude requires b > acc");
    trim(acc);
    Ok(())
}

/// `out = a * b`, reusing the buffer of `out`.
fn mul_into(out: &mut Vec<u64>, a: &[u64], b: &[u64]) -> Result<(), ExactEvaluationExhausted> {
    out.clear();
    try_reserve_total(out, a.len() + b.len())?;
    out.resize(a.len() + b.len(), 0);
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0_u128;
        for (j, &y) in b.iter().enumerate() {
            let cur = u128::from(out[i + j]) + u128::from(x) * u128::from(y) + carry;
            out[i + j] = cur as u64;
            carry = cur >> 64;
        }
        out[i + b.len()] = carry as u64;
    }
    trim(out);
    Ok(())
}

/// Exact sign of the determinant described by `rows`.
pub(super) fn sign_exact(rows: Rows<'_>) -> Result<Sign, ExactEvaluationExhausted> {
    Ok(determinant_of(rows)?.sign())
}

/// The determinant described by `rows`, scaled by a positive power of two
/// that depends only on the multiset of values in `rows`.
fn determinant_of(rows: Rows<'_>) -> Result<BigInt, ExactEvaluationExhausted> {
    let k = rows.k();
    // Every value is a multiple of 2^(base - 1074). Dividing all of them by
    // that power of two keeps the integers small and multiplies the
    // determinant by a positive factor, so the sign is unchanged.
    let base = rows.values().filter_map(exponent_shift).min().unwrap_or(0);
    let mut scaled_origin = try_vec(rows.origin.len())?;
    for &x in rows.origin {
        scaled_origin.push(BigInt::from_f64_scaled(x, base)?);
    }
    // The lifted column holds sum X^2 - sum O^2 over the scaled integers:
    // the true difference times 2^(2 (base - 1074)), another positive factor
    // that applies to the whole column.
    let mut scratch = Vec::new();
    let mut squared_norm = |scaled: &[BigInt]| -> Result<BigInt, ExactEvaluationExhausted> {
        let mut sum = BigInt::zero();
        for x in scaled {
            sum.add_product(x, x, false, &mut scratch)?;
        }
        Ok(sum)
    };
    let origin_norm = if rows.lifted.is_some() {
        Some(squared_norm(&scaled_origin)?)
    } else {
        None
    };
    let mut matrix: Vec<Vec<BigInt>> = try_vec(k)?;
    for i in 0..k {
        let mut row = try_vec(k)?;
        match rows.row(i) {
            Row::Difference(point) => {
                let mut scaled = try_vec(point.len())?;
                for &x in point {
                    scaled.push(BigInt::from_f64_scaled(x, base)?);
                }
                for (x, o) in scaled.iter().zip(&scaled_origin) {
                    row.push(x.sub(o)?);
                }
                if let Some(origin_norm) = &origin_norm {
                    row.push(squared_norm(&scaled)?.sub(origin_norm)?);
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
    determinant(&matrix)
}

/// The unit direction of the exact cofactor vector of the hyperplane through
/// `facet` (k points of dimension k), rounded to `f64`: each component is
/// within `2^-50` of the exact unit vector's. `None` when every cofactor is
/// zero, that is, when the points are affinely dependent.
pub(super) fn cofactor_direction_exact(
    facet: &[&[f64]],
) -> Result<Option<Vec<f64>>, ExactEvaluationExhausted> {
    let k = facet.len();
    let mut unit = vec![0.0; k];
    let mut cofactors = try_vec(k)?;
    for j in 0..k {
        unit[j] = 1.0;
        // Every row set holds the same values (the facet, one 1, and zeros),
        // so every cofactor carries the same power-of-two scale.
        cofactors.push(determinant_of(Rows {
            origin: facet[0],
            points: &facet[1..],
            direction: Some(&unit),
            lifted: None,
        })?);
        unit[j] = 0.0;
    }
    let longest = cofactors.iter().map(BigInt::bit_length).max().unwrap_or(0);
    if longest == 0 {
        return Ok(None);
    }
    let shift = longest.saturating_sub(64);
    let values: Vec<f64> = cofactors.iter().map(|c| c.to_f64_shifted(shift)).collect();
    let norm = values.iter().map(|x| x * x).sum::<f64>().sqrt();
    Ok(Some(values.iter().map(|x| x / norm).collect()))
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
    // One product buffer for the whole determinant.
    let mut scratch: Vec<u64> = Vec::new();
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
            // -(R . v), accumulated with the sign folded into each product.
            let mut dot = BigInt::zero();
            for (x, y) in a[r][..r].iter().zip(&v) {
                dot.add_product(x, y, true, &mut scratch)?;
            }
            column.push(dot);
            if step + 1 < r {
                let mut next: Vec<BigInt> = try_vec(r)?;
                for row in a.iter().take(r) {
                    let mut sum = BigInt::zero();
                    for (x, y) in row[..r].iter().zip(&v) {
                        sum.add_product(x, y, false, &mut scratch)?;
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
                sum.add_product(&column[i - j], p, false, &mut scratch)?;
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
#[path = "exact_reference.rs"]
mod reference;

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
            total += if c.is_multiple_of(2) { term } else { -term };
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

    /// Random `f64` entries: mixed exponents, zeros, and repeated rows, so
    /// that values span many limbs and some determinants are exactly zero.
    fn random_matrix(n: usize, next: &mut impl FnMut() -> u64) -> Vec<Vec<f64>> {
        let mut m: Vec<Vec<f64>> = (0..n)
            .map(|_| {
                (0..n)
                    .map(|_| match next() % 8 {
                        0 => 0.0,
                        1 => (next() % 7) as f64 - 3.0,
                        _ => {
                            let mantissa = (next() >> 11) as f64;
                            let exponent = (next() % 400) as i32 - 200;
                            let sign = if next().is_multiple_of(2) { 1.0 } else { -1.0 };
                            sign * mantissa * 2f64.powi(exponent)
                        }
                    })
                    .collect()
            })
            .collect();
        if n >= 2 && next().is_multiple_of(4) {
            m[n - 1] = m[0].clone();
        }
        m
    }

    #[test]
    fn determinant_matches_the_previous_exact_stage() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut signs = [0usize; 3];
        for n in 1..=9 {
            for _ in 0..60 {
                let m = random_matrix(n, &mut next);
                let base = m
                    .iter()
                    .flatten()
                    .copied()
                    .filter_map(exponent_shift)
                    .min()
                    .unwrap_or(0);
                let ours: Vec<Vec<BigInt>> = m
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|&x| BigInt::from_f64_scaled(x, base).unwrap())
                            .collect()
                    })
                    .collect();
                let theirs: Vec<Vec<reference::RefInt>> = m
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|&x| reference::RefInt::from_f64_scaled(x, base).unwrap())
                            .collect()
                    })
                    .collect();
                let got = determinant(&ours).unwrap();
                let want = reference::determinant(&theirs).unwrap().parts();
                assert_eq!((got.negative, got.magnitude.clone()), want, "{m:?}");
                signs[got.sign() as usize] += 1;
            }
        }
        assert!(signs.iter().all(|&c| c > 0), "every sign occurs: {signs:?}");
    }
}
