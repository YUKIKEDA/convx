//! Exact sign of a determinant whose entries are differences of `f64` values.
//!
//! Every finite `f64` is an integer multiple of 2^-1074, so scaling every
//! coordinate by 2^1074 turns it into an integer without rounding. The
//! determinant of the scaled differences is then evaluated over the integers
//! with the division-free Berkowitz algorithm, so only addition, subtraction,
//! and multiplication of big integers are needed. Scaling every entry by the
//! same positive power of two does not change the sign of the determinant.
//!
//! Every buffer is reserved with `try_reserve` or `try_reserve_exact`. A
//! failed reservation is reported as [`ExactEvaluationExhausted`] instead of
//! aborting.
//!
//! The buffers are those of a workspace kept per thread ([`HeapWork`]): the
//! scaled coordinates, the matrix, the slots of the determinant, and the
//! limbs of each integer are reserved when a call first needs them and
//! filled again by every later call. Reserved for each call, they were 10
//! to 1,250 allocations per point of a hull whose coordinates have
//! exponents far apart (`docs/bench.md`, #185).
//!
//! [`sign_exact`] and [`cofactor_direction_exact`] first try the integers on
//! the stack of [`super::fixed`], which hold most determinants without
//! reserving memory (#173); the heap integers here take the rest.

use core::cell::RefCell;
use core::cmp::Ordering;

use super::{ExactEvaluationExhausted, Row, Rows, Sign};

/// Reserves an empty vector with room for `capacity` elements.
#[cfg(test)]
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
pub(super) fn decompose(x: f64) -> (u64, u64) {
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
#[derive(Debug, Default, PartialEq, Eq)]
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

    #[cfg(test)]
    fn one() -> Result<Self, ExactEvaluationExhausted> {
        let mut magnitude = try_vec(1)?;
        magnitude.push(1);
        Ok(Self {
            negative: false,
            magnitude,
        })
    }

    /// Returns `x * 2^(1074 - base)` as an integer. `x` must be finite and
    /// `base` must not exceed [`exponent_shift`] of any nonzero `x` converted
    /// with it.
    #[cfg(test)]
    pub(super) fn from_f64_scaled(x: f64, base: u64) -> Result<Self, ExactEvaluationExhausted> {
        let mut value = Self::zero();
        value.set_f64_scaled(x, base)?;
        Ok(value)
    }

    /// Sets the value to `x * 2^(1074 - base)`, keeping the limb buffer. `x`
    /// must be finite and `base` must not exceed [`exponent_shift`] of any
    /// nonzero `x` converted with it.
    fn set_f64_scaled(&mut self, x: f64, base: u64) -> Result<(), ExactEvaluationExhausted> {
        debug_assert!(x.is_finite(), "predicate input must be finite");
        self.clear();
        let (mantissa, shift) = decompose(x);
        if mantissa == 0 {
            return Ok(());
        }
        debug_assert!(shift >= base, "scale base above an input exponent");
        let shift = shift - base;
        let limb_shift = (shift / 64) as usize;
        let bit_shift = (shift % 64) as u32;
        let wide = u128::from(mantissa) << bit_shift;
        try_reserve_total(&mut self.magnitude, limb_shift + 2)?;
        self.magnitude.resize(limb_shift, 0);
        self.magnitude.push(wide as u64);
        self.magnitude.push((wide >> 64) as u64);
        trim(&mut self.magnitude);
        self.negative = x.to_bits() >> 63 == 1 && !self.magnitude.is_empty();
        Ok(())
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

    /// The sign and the little-endian limbs of the magnitude, for tests.
    #[cfg(test)]
    pub(super) fn parts(&self) -> (bool, Vec<u64>) {
        (self.negative, self.magnitude.clone())
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

    #[cfg(test)]
    fn try_clone(&self) -> Result<Self, ExactEvaluationExhausted> {
        let mut magnitude = try_vec(self.magnitude.len())?;
        magnitude.extend_from_slice(&self.magnitude);
        Ok(Self {
            negative: self.negative,
            magnitude,
        })
    }

    /// Sets the value to zero, keeping the limb buffer.
    fn clear(&mut self) {
        self.magnitude.clear();
        self.negative = false;
    }

    /// Sets the value to one, keeping the limb buffer.
    fn set_one(&mut self) -> Result<(), ExactEvaluationExhausted> {
        self.clear();
        try_reserve_total(&mut self.magnitude, 1)?;
        self.magnitude.push(1);
        Ok(())
    }

    /// Copies `other` into this value's limb buffer.
    fn assign(&mut self, other: &Self) -> Result<(), ExactEvaluationExhausted> {
        self.magnitude.clear();
        try_reserve_total(&mut self.magnitude, other.magnitude.len())?;
        self.magnitude.extend_from_slice(&other.magnitude);
        self.negative = other.negative;
        Ok(())
    }

    /// Copies `-other` into this value's limb buffer.
    fn assign_negated(&mut self, other: &Self) -> Result<(), ExactEvaluationExhausted> {
        self.assign(other)?;
        self.negative = !self.negative && !self.magnitude.is_empty();
        Ok(())
    }

    #[cfg(test)]
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

    #[cfg(test)]
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
///
/// The fixed-size integers of [`super::fixed`] are tried first. They hold
/// most determinants without reserving memory (#173); the heap integers
/// below take the rest.
pub(super) fn sign_exact(rows: Rows<'_>) -> Result<Sign, ExactEvaluationExhausted> {
    if let Some(sign) = super::fixed::sign(rows) {
        return Ok(sign);
    }
    sign_exact_on_heap(rows)
}

/// The buffers of the heap stage, kept from one call to the next on a
/// thread: every list of integers keeps its slots, and every integer its
/// limbs.
#[derive(Default)]
struct HeapWork {
    /// The scaled origin and the scaled point of one row.
    origin: Vec<BigInt>,
    point: Vec<BigInt>,
    origin_norm: BigInt,
    /// The k x k matrix, row-major.
    matrix: Vec<BigInt>,
    /// The product of one multiplication.
    scratch: Vec<u64>,
    /// The slots of [`determinant_in`]; its result is left in `poly`.
    poly: Vec<BigInt>,
    next_poly: Vec<BigInt>,
    column: Vec<BigInt>,
    v: Vec<BigInt>,
    next: Vec<BigInt>,
    /// The cofactors of [`cofactor_direction_on_heap`] and its unit row.
    cofactors: Vec<BigInt>,
    unit: Vec<f64>,
}

thread_local! {
    /// The workspace of this thread. A build runs on one thread (design
    /// §6), and two threads do not share one.
    static WORK: RefCell<HeapWork> = RefCell::new(HeapWork::default());
}

/// Runs `f` with this thread's workspace. A workspace of its own serves a
/// call made while the thread's is in use, or while the thread ends.
fn with_work<T>(
    f: impl FnOnce(&mut HeapWork) -> Result<T, ExactEvaluationExhausted>,
) -> Result<T, ExactEvaluationExhausted> {
    let mut f = Some(f);
    let kept = WORK
        .try_with(|cell| {
            let mut work = cell.try_borrow_mut().ok()?;
            f.take().map(|f| f(&mut work))
        })
        .ok()
        .flatten();
    kept.or_else(|| f.take().map(|f| f(&mut HeapWork::default())))
        // `f` is taken only where its result is returned, so one of the
        // two ran; no path reaches this.
        .unwrap_or(Err(ExactEvaluationExhausted))
}

/// At least `len` slots in `slots`, the new ones zero. Slots are never
/// dropped, so their limb buffers stay.
fn fit(slots: &mut Vec<BigInt>, len: usize) -> Result<(), ExactEvaluationExhausted> {
    if slots.len() < len {
        try_reserve_total(slots, len)?;
        slots.resize_with(len, BigInt::zero);
    }
    Ok(())
}

/// [`sign_exact`] over the heap integers alone.
pub(super) fn sign_exact_on_heap(rows: Rows<'_>) -> Result<Sign, ExactEvaluationExhausted> {
    with_work(|work| {
        let negated = determinant_of_in(rows, work)?;
        let sign = work.poly[rows.k()].sign();
        Ok(if negated { flipped(sign) } else { sign })
    })
}

/// `sign` of the value negated.
fn flipped(sign: Sign) -> Sign {
    match sign {
        Sign::Positive => Sign::Negative,
        Sign::Negative => Sign::Positive,
        Sign::Zero => Sign::Zero,
    }
}

/// The determinant described by `rows`, scaled by a positive power of two
/// that depends only on the multiset of values in `rows`.
#[cfg(test)]
pub(super) fn determinant_of(rows: Rows<'_>) -> Result<BigInt, ExactEvaluationExhausted> {
    let mut work = HeapWork::default();
    let negated = determinant_of_in(rows, &mut work)?;
    let value = core::mem::take(&mut work.poly[rows.k()]);
    Ok(if negated { value.negated() } else { value })
}

/// [`determinant_of`] in the buffers of `work`: the value is
/// `work.poly[rows.k()]`, negated when this returns `true`.
fn determinant_of_in(
    rows: Rows<'_>,
    work: &mut HeapWork,
) -> Result<bool, ExactEvaluationExhausted> {
    let k = rows.k();
    let HeapWork {
        origin,
        point: scaled,
        origin_norm,
        matrix,
        scratch,
        ..
    } = work;
    // Every value is a multiple of 2^(base - 1074). Dividing all of them by
    // that power of two keeps the integers small and multiplies the
    // determinant by a positive factor, so the sign is unchanged.
    let base = rows.values().filter_map(exponent_shift).min().unwrap_or(0);
    fit(origin, rows.origin.len())?;
    for (slot, &x) in origin.iter_mut().zip(rows.origin) {
        slot.set_f64_scaled(x, base)?;
    }
    let origin = &origin[..rows.origin.len()];
    // The lifted column holds sum X^2 - sum O^2 over the scaled integers:
    // the true difference times 2^(2 (base - 1074)), another positive factor
    // that applies to the whole column.
    let lifted = rows.lifted.is_some();
    if lifted {
        origin_norm.clear();
        for x in origin {
            origin_norm.add_product(x, x, false, scratch)?;
        }
    }
    fit(matrix, k * k)?;
    for i in 0..k {
        let row = &mut matrix[i * k..(i + 1) * k];
        match rows.row(i) {
            Row::Difference(point) => {
                if lifted {
                    fit(scaled, point.len())?;
                    let scaled = &mut scaled[..point.len()];
                    for (slot, &x) in scaled.iter_mut().zip(point) {
                        slot.set_f64_scaled(x, base)?;
                    }
                    // The last entry of the row: the norm of the point less
                    // that of the origin.
                    let norm = &mut row[point.len()];
                    norm.clear();
                    for x in scaled.iter() {
                        norm.add_product(x, x, false, scratch)?;
                    }
                    norm.accumulate(&origin_norm.magnitude, !origin_norm.negative)?;
                    for ((entry, x), o) in row.iter_mut().zip(scaled.iter()).zip(origin) {
                        entry.assign(x)?;
                        entry.accumulate(&o.magnitude, !o.negative)?;
                    }
                } else {
                    for ((entry, &x), o) in row.iter_mut().zip(point).zip(origin) {
                        entry.set_f64_scaled(x, base)?;
                        entry.accumulate(&o.magnitude, !o.negative)?;
                    }
                }
            }
            Row::Direction(direction) => {
                for (entry, &x) in row.iter_mut().zip(direction) {
                    entry.set_f64_scaled(x, base)?;
                }
            }
        }
    }
    determinant_in(k, work)
}

/// The unit direction of the exact cofactor vector of the hyperplane through
/// `facet` (k points of dimension k), rounded to `f64`: within `(k/2 + 5) u`
/// (Euclidean, `u = 2^-53`) of the exact unit vector, as
/// [`super::cofactor_direction_from`] derives. `None` when every cofactor is
/// zero, that is, when the points are affinely dependent.
pub(super) fn cofactor_direction_exact(
    facet: &[&[f64]],
) -> Result<Option<super::Direction>, ExactEvaluationExhausted> {
    if let Some(direction) = super::fixed::cofactor_direction(facet) {
        return Ok(direction);
    }
    cofactor_direction_on_heap(facet)
}

/// [`cofactor_direction_exact`] over the heap integers alone.
pub(super) fn cofactor_direction_on_heap(
    facet: &[&[f64]],
) -> Result<Option<super::Direction>, ExactEvaluationExhausted> {
    let k = facet.len();
    with_work(|work| {
        fit(&mut work.cofactors, k)?;
        let mut unit = core::mem::take(&mut work.unit);
        unit.clear();
        unit.try_reserve(k).map_err(|_| ExactEvaluationExhausted)?;
        unit.resize(k, 0.0);
        for j in 0..k {
            unit[j] = 1.0;
            // Every row set holds the same values (the facet, one 1, and
            // zeros), so every cofactor carries the same power-of-two scale.
            let negated = determinant_of_in(
                Rows {
                    origin: facet[0],
                    points: &facet[1..],
                    direction: Some(&unit),
                    lifted: None,
                },
                work,
            )?;
            unit[j] = 0.0;
            let HeapWork {
                cofactors, poly, ..
            } = &mut *work;
            if negated {
                cofactors[j].assign_negated(&poly[k])?;
            } else {
                cofactors[j].assign(&poly[k])?;
            }
        }
        work.unit = unit;
        let cofactors = &work.cofactors[..k];
        let longest = cofactors.iter().map(BigInt::bit_length).max().unwrap_or(0);
        if longest == 0 {
            return Ok(None);
        }
        let shift = longest.saturating_sub(64);
        let values: super::Direction = cofactors.iter().map(|c| c.to_f64_shifted(shift)).collect();
        let norm = values.iter().map(|x| x * x).sum::<f64>().sqrt();
        Ok(Some(values.iter().map(|x| x / norm).collect()))
    })
}

/// Determinant of a square integer matrix by the Berkowitz algorithm.
#[cfg(test)]
pub(super) fn determinant(a: &[Vec<BigInt>]) -> Result<BigInt, ExactEvaluationExhausted> {
    let n = a.len();
    let mut work = HeapWork::default();
    fit(&mut work.matrix, n * n)?;
    for (slot, entry) in work.matrix.iter_mut().zip(a.iter().flatten()) {
        slot.assign(entry)?;
    }
    let negated = determinant_in(n, &mut work)?;
    let value = core::mem::take(&mut work.poly[n]);
    Ok(if negated { value.negated() } else { value })
}

/// Determinant of the `n` x `n` matrix in `work.matrix` (row-major) by the
/// Berkowitz algorithm: the value is `work.poly[n]`, negated when this
/// returns `true`.
///
/// The algorithm builds the characteristic polynomial det(xI - A) of each
/// leading principal submatrix from the previous one with a Toeplitz matrix,
/// using O(n^4) ring operations and no division. The constant term of the
/// polynomial of the full matrix is (-1)^n det(A).
fn determinant_in(n: usize, work: &mut HeapWork) -> Result<bool, ExactEvaluationExhausted> {
    let HeapWork {
        matrix,
        scratch,
        poly,
        next_poly,
        column,
        v,
        next,
        ..
    } = work;
    let a = |i: usize, j: usize| &matrix[i * n + j];
    // Every intermediate lives in a slot kept by the workspace and
    // overwritten in place, so limb buffers are reused across steps and
    // across calls.
    // Coefficients of det(xI - A_r), highest power first, for the leading
    // r x r submatrix A_r: r + 1 of them. Starts with r = 1.
    fit(poly, n + 1)?;
    fit(next_poly, n + 1)?;
    fit(column, n + 1)?;
    fit(v, n)?;
    fit(next, n)?;
    poly[0].set_one()?;
    if n == 0 {
        return Ok(false);
    }
    poly[1].assign_negated(a(0, 0))?;

    for r in 1..n {
        // First column of the Toeplitz matrix: 1, -a_rr, -R C, -R A_r C, ...,
        // -R A_r^(r-1) C, where R is row r and C is column r restricted to
        // the leading r entries.
        column[0].set_one()?;
        column[1].assign_negated(a(r, r))?;
        for (i, slot) in v.iter_mut().enumerate().take(r) {
            slot.assign(a(i, r))?;
        }
        for step in 0..r {
            // -(R . v), accumulated with the sign folded into each product.
            let dot = &mut column[step + 2];
            dot.clear();
            for (j, y) in v.iter().enumerate().take(r) {
                dot.add_product(a(r, j), y, true, scratch)?;
            }
            if step + 1 < r {
                for (i, sum) in next.iter_mut().enumerate().take(r) {
                    sum.clear();
                    for (j, y) in v.iter().enumerate().take(r) {
                        sum.add_product(a(i, j), y, false, scratch)?;
                    }
                }
                core::mem::swap(v, next);
            }
        }
        // poly <- T * poly, T lower-triangular Toeplitz of size (r+2) x (r+1).
        for (i, sum) in next_poly.iter_mut().enumerate().take(r + 2) {
            sum.clear();
            for (j, p) in poly.iter().enumerate().take((i + 1).min(r + 1)) {
                sum.add_product(&column[i - j], p, false, scratch)?;
            }
        }
        core::mem::swap(poly, next_poly);
    }
    Ok(n % 2 == 1)
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

    /// A determinant in a workspace that earlier calls have used (#185) is
    /// the determinant in a new one, limb for limb: sizes 1 to 7 in turn,
    /// larger after smaller and smaller after larger, with and without the
    /// lifted row and the direction row, on coordinates whose exponents are
    /// far apart, so the integers are long and of very different lengths.
    /// The direction of a facet's cofactors agrees in the same way.
    #[test]
    fn a_workspace_used_before_gives_the_values_of_a_new_one() {
        let mut state = 0x0185_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut coordinate = |wide: bool| -> f64 {
            let sign = if next().is_multiple_of(2) { 1.0 } else { -1.0 };
            match next() % 6 {
                0 => 0.0,
                1 => (next() % 7) as f64 - 3.0,
                _ => {
                    let span = if wide { 1800 } else { 60 };
                    let exponent = (next() % span) as i32 - span as i32 / 2;
                    sign * ((next() >> 11) as f64 / (1_u64 << 53) as f64 + 0.5)
                        * 2f64.powi(exponent)
                }
            }
        };
        let read = |work: &HeapWork, k: usize, negated: bool| {
            let value = &work.poly[k];
            (
                negated != value.negative && !value.magnitude.is_empty(),
                value.magnitude.clone(),
            )
        };
        let mut used = HeapWork::default();
        let (mut cases, mut long, mut zero) = (0, 0, 0);
        for round in 0..40 {
            for d in [3, 7, 1, 5, 2, 6, 4] {
                let wide = round % 2 == 0;
                let mut rows: Vec<Vec<f64>> = (0..d + 2)
                    .map(|_| (0..d).map(|_| coordinate(wide)).collect())
                    .collect();
                if round % 5 == 0 && d >= 2 {
                    rows[d] = rows[1].clone();
                }
                let refs: Vec<&[f64]> = rows.iter().map(Vec::as_slice).collect();
                // The orientation of d + 1 points, the lifted orientation
                // of d + 2, and d points with a direction.
                let shapes = [
                    Rows {
                        origin: refs[0],
                        points: &refs[1..=d],
                        direction: None,
                        lifted: None,
                    },
                    Rows {
                        origin: refs[0],
                        points: &refs[1..=d + 1],
                        direction: None,
                        lifted: Some(&[]),
                    },
                    Rows {
                        origin: refs[0],
                        points: &refs[1..d],
                        direction: Some(refs[d]),
                        lifted: None,
                    },
                ];
                for shape in shapes {
                    let k = shape.k();
                    let negated = determinant_of_in(shape, &mut used).unwrap();
                    let mut new = HeapWork::default();
                    let want = determinant_of_in(shape, &mut new).unwrap();
                    assert_eq!(
                        read(&used, k, negated),
                        read(&new, k, want),
                        "round {round}, D = {d}"
                    );
                    cases += 1;
                    long += usize::from(new.poly[k].magnitude.len() > 16);
                    zero += usize::from(new.poly[k].magnitude.is_empty());
                }
                // The direction: on this thread's workspace, then again
                // after it was used for another size.
                let facet = &refs[..d];
                let first = cofactor_direction_on_heap(facet).unwrap();
                let _ = sign_exact_on_heap(shapes[1]).unwrap();
                let again = cofactor_direction_on_heap(facet).unwrap();
                let bits = |v: &Option<crate::predicates::Direction>| {
                    v.as_ref()
                        .map(|v| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>())
                };
                assert_eq!(bits(&first), bits(&again), "round {round}, D = {d}");
                if let Some(fixed) = crate::predicates::fixed::cofactor_direction(facet) {
                    assert_eq!(bits(&first), bits(&fixed), "round {round}, D = {d}");
                }
            }
        }
        assert_eq!(cases, 840);
        // The slots of the largest size stay through the smaller ones, with
        // their limbs: the lifted orientation of D = 7 has k = 8.
        assert_eq!((used.matrix.len(), used.poly.len()), (64, 9));
        assert!(used.matrix[63].magnitude.capacity() > 0);
        assert!(long > 200 && zero > 20, "{long} long values, {zero} zeros");
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
