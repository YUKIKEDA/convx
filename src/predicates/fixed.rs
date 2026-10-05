//! Exact determinant over fixed-size integers on the stack (#173).
//!
//! The matrix of [`super::exact`] has integer entries once every value is
//! divided by the lowest power of two among their set bits; dividing every
//! entry of a column by the same power of two keeps the sign. On coplanar
//! input, whose coordinates are small integers or share a few exponents,
//! those integers are short, and so is the determinant. This stage
//! evaluates it over one `i128` ([`Wide`]) or over sign-magnitude integers
//! of 4, 8, or 16 64-bit limbs ([`Fixed`]), in arrays of the matrix size
//! `K`, so it reserves nothing on the heap: by cofactor expansion for
//! `K <= 4`, and by the Berkowitz polynomial of [`super::exact`] above.
//! Every operation checks that its result fits; when one does not, the
//! stage gives up with [`Overflow`], and the caller tries the next width,
//! then the heap integers. No value is rounded, so a result is always the
//! exact one.

use super::exact::decompose;
use super::{Row, Rows, Sign};

/// A result did not fit in the limbs of the stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Overflow;

/// The largest matrix this stage holds on the stack. Larger ones go to the
/// heap integers.
pub(super) const MAX_SIZE: usize = 10;

/// A signed integer of `N` little-endian 64-bit limbs, sign and magnitude.
/// Limbs at and above `len` are zero, and `limbs[len - 1]` is not. Zero has
/// `len == 0` and is never negative.
#[derive(Clone, Copy, Debug)]
pub(super) struct Fixed<const N: usize> {
    negative: bool,
    len: usize,
    limbs: [u64; N],
}

impl<const N: usize> Default for Fixed<N> {
    fn default() -> Self {
        Self {
            negative: false,
            len: 0,
            limbs: [0; N],
        }
    }
}

impl<const N: usize> Fixed<N> {
    fn one() -> Self {
        let mut limbs = [0; N];
        limbs[0] = 1;
        Self {
            negative: false,
            len: 1,
            limbs,
        }
    }

    /// `x * 2^(1074 - base)`, an integer when `base` is at most the
    /// position of the lowest set bit of `x` (see [`lowest_bit`]).
    fn from_f64_scaled(x: f64, base: u64) -> Result<Self, Overflow> {
        let (mantissa, shift) = decompose(x);
        if mantissa == 0 {
            return Ok(Self::default());
        }
        debug_assert!(lowest_bit(x) >= Some(base), "scale base above a set bit");
        // |x| = mantissa * 2^(shift - 1074); the scaled magnitude is
        // mantissa * 2^(shift - base), whose low bits may lie inside the
        // trailing zeros of the mantissa.
        let wide = if shift >= base {
            let shift = shift - base;
            (u128::from(mantissa) << (shift % 64), (shift / 64) as usize)
        } else {
            (u128::from(mantissa >> (base - shift)), 0)
        };
        let ((wide, limb), negative) = (wide, x.to_bits() >> 63 == 1);
        let (low, high) = (wide as u64, (wide >> 64) as u64);
        let len = if high == 0 { limb + 1 } else { limb + 2 };
        if len > N {
            return Err(Overflow);
        }
        let mut limbs = [0; N];
        limbs[limb] = low;
        if high != 0 {
            limbs[limb + 1] = high;
        }
        Ok(Self {
            negative,
            len,
            limbs,
        })
    }

    pub(super) fn sign(&self) -> Sign {
        if self.len == 0 {
            Sign::Zero
        } else if self.negative {
            Sign::Negative
        } else {
            Sign::Positive
        }
    }

    fn negated(mut self) -> Self {
        self.negative = !self.negative && self.len != 0;
        self
    }

    /// Number of significant bits of the magnitude.
    pub(super) fn bit_length(&self) -> u64 {
        match self.len {
            0 => 0,
            len => (len as u64 - 1) * 64 + u64::from(64 - self.limbs[len - 1].leading_zeros()),
        }
    }

    /// `self / 2^shift`, truncated to the 64 bits from position `shift` and
    /// then rounded to `f64`, as `to_f64_shifted` of the heap integers does.
    pub(super) fn shifted_f64(&self, shift: u64) -> f64 {
        let limb = |i: u64| {
            usize::try_from(i)
                .ok()
                .and_then(|i| self.limbs.get(i))
                .copied()
                .unwrap_or(0)
        };
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

    /// Adds the magnitude `other` (its first `other_len` limbs) with sign
    /// `negative`.
    fn accumulate(
        &mut self,
        other: &[u64; N],
        other_len: usize,
        negative: bool,
    ) -> Result<(), Overflow> {
        if other_len == 0 {
            return Ok(());
        }
        if self.len == 0 || self.negative == negative {
            if self.len == 0 {
                self.negative = negative;
            }
            let len = self.len.max(other_len);
            let mut carry = false;
            for (slot, &b) in self.limbs[..len].iter_mut().zip(&other[..len]) {
                let (sum, c1) = slot.overflowing_add(b);
                let (sum, c2) = sum.overflowing_add(u64::from(carry));
                *slot = sum;
                carry = c1 || c2;
            }
            self.len = len;
            if carry {
                if len == N {
                    return Err(Overflow);
                }
                self.limbs[len] = 1;
                self.len = len + 1;
            }
            return Ok(());
        }
        // Opposite signs: subtract the smaller magnitude from the larger.
        let order = self.len.cmp(&other_len).then_with(|| {
            self.limbs[..self.len]
                .iter()
                .rev()
                .cmp(other[..other_len].iter().rev())
        });
        let len = self.len.max(other_len);
        let mut borrow = false;
        match order {
            core::cmp::Ordering::Equal => {
                *self = Self::default();
                return Ok(());
            }
            core::cmp::Ordering::Greater => {
                for (slot, &b) in self.limbs[..len].iter_mut().zip(&other[..len]) {
                    let (diff, b1) = slot.overflowing_sub(b);
                    let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
                    *slot = diff;
                    borrow = b1 || b2;
                }
            }
            core::cmp::Ordering::Less => {
                for (slot, &b) in self.limbs[..len].iter_mut().zip(&other[..len]) {
                    let (diff, b1) = b.overflowing_sub(*slot);
                    let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
                    *slot = diff;
                    borrow = b1 || b2;
                }
                self.negative = negative;
            }
        }
        debug_assert!(!borrow, "the larger magnitude was subtracted from");
        self.len = len;
        self.trim();
        Ok(())
    }

    fn trim(&mut self) {
        while self.len > 0 && self.limbs[self.len - 1] == 0 {
            self.len -= 1;
        }
    }

    /// `self += sign * x * y`, where `sign` is -1 when `negate`.
    fn add_product(&mut self, x: &Self, y: &Self, negate: bool) -> Result<(), Overflow> {
        if x.len == 0 || y.len == 0 {
            return Ok(());
        }
        // The product has x.len + y.len - 1 or x.len + y.len limbs.
        if x.len + y.len - 1 > N {
            return Err(Overflow);
        }
        let mut product = [0_u64; N];
        let mut top = 0_u64;
        for (i, &a) in x.limbs[..x.len].iter().enumerate() {
            let mut carry = 0_u128;
            for (j, &b) in y.limbs[..y.len].iter().enumerate() {
                let slot = &mut product[i + j];
                let cur = u128::from(*slot) + u128::from(a) * u128::from(b) + carry;
                *slot = cur as u64;
                carry = cur >> 64;
            }
            let end = i + y.len;
            if end < N {
                product[end] = carry as u64;
            } else if carry != 0 {
                // Only the last row reaches past the limbs (end == N).
                top = carry as u64;
            }
        }
        if top != 0 {
            return Err(Overflow);
        }
        let mut len = (x.len + y.len).min(N);
        while len > 0 && product[len - 1] == 0 {
            len -= 1;
        }
        self.accumulate(&product, len, (x.negative != y.negative) != negate)
    }
}

/// The integer operations the determinant needs, for every width.
pub(super) trait Int: Copy + Default {
    fn one() -> Self;
    /// `x * 2^(1074 - base)`, an integer when `base` is at most the
    /// position of the lowest set bit of `x` (see [`lowest_bit`]).
    fn from_f64_scaled(x: f64, base: u64) -> Result<Self, Overflow>;
    fn negated(self) -> Self;
    fn sign(&self) -> Sign;
    /// `self -= other`.
    fn sub_assign(&mut self, other: &Self) -> Result<(), Overflow>;
    /// `self += sign * x * y`, where `sign` is -1 when `negate`.
    fn add_product(&mut self, x: &Self, y: &Self, negate: bool) -> Result<(), Overflow>;
    /// Number of significant bits of the magnitude.
    fn bit_length(&self) -> u64;
    /// `self / 2^shift`, truncated to the 64 bits from position `shift` and
    /// then rounded to `f64`, as `to_f64_shifted` of the heap integers does.
    fn shifted_f64(&self, shift: u64) -> f64;
}

impl<const N: usize> Int for Fixed<N> {
    fn one() -> Self {
        Fixed::one()
    }

    fn from_f64_scaled(x: f64, base: u64) -> Result<Self, Overflow> {
        Fixed::from_f64_scaled(x, base)
    }

    fn negated(self) -> Self {
        Fixed::negated(self)
    }

    fn sign(&self) -> Sign {
        Fixed::sign(self)
    }

    fn sub_assign(&mut self, other: &Self) -> Result<(), Overflow> {
        self.accumulate(&other.limbs, other.len, !other.negative)
    }

    fn add_product(&mut self, x: &Self, y: &Self, negate: bool) -> Result<(), Overflow> {
        Fixed::add_product(self, x, y, negate)
    }

    fn bit_length(&self) -> u64 {
        Fixed::bit_length(self)
    }

    fn shifted_f64(&self, shift: u64) -> f64 {
        Fixed::shifted_f64(self, shift)
    }
}

/// A signed integer in one `i128`, every operation checked. Small integer
/// input, such as a grid, stays in this width, where an operation is a few
/// instructions.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Wide(i128);

impl Int for Wide {
    fn one() -> Self {
        Self(1)
    }

    fn from_f64_scaled(x: f64, base: u64) -> Result<Self, Overflow> {
        let (mantissa, shift) = decompose(x);
        if mantissa == 0 {
            return Ok(Self(0));
        }
        debug_assert!(lowest_bit(x) >= Some(base), "scale base above a set bit");
        let magnitude = if shift >= base {
            let shift = shift - base;
            // The mantissa has at most 53 bits; 2^74 times it still fits.
            if shift > 74 {
                return Err(Overflow);
            }
            i128::from(mantissa) << shift
        } else {
            i128::from(mantissa >> (base - shift))
        };
        Ok(Self(if x.to_bits() >> 63 == 1 {
            -magnitude
        } else {
            magnitude
        }))
    }

    fn negated(self) -> Self {
        // Every value is the result of a checked operation, so it is above
        // i128::MIN.
        Self(-self.0)
    }

    fn sign(&self) -> Sign {
        match self.0.cmp(&0) {
            core::cmp::Ordering::Less => Sign::Negative,
            core::cmp::Ordering::Equal => Sign::Zero,
            core::cmp::Ordering::Greater => Sign::Positive,
        }
    }

    fn sub_assign(&mut self, other: &Self) -> Result<(), Overflow> {
        self.0 = self
            .0
            .checked_sub(other.0)
            .filter(|&v| v != i128::MIN)
            .ok_or(Overflow)?;
        Ok(())
    }

    fn add_product(&mut self, x: &Self, y: &Self, negate: bool) -> Result<(), Overflow> {
        // Two factors of 64 bits cannot overflow, and their product is a
        // single widening multiply; `checked_mul` of two i128 is a call.
        let product = match (i64::try_from(x.0), i64::try_from(y.0)) {
            (Ok(a), Ok(b)) => i128::from(a) * i128::from(b),
            _ => x.0.checked_mul(y.0).ok_or(Overflow)?,
        };
        let sum = if negate {
            self.0.checked_sub(product)
        } else {
            self.0.checked_add(product)
        };
        self.0 = sum.filter(|&v| v != i128::MIN).ok_or(Overflow)?;
        Ok(())
    }

    fn bit_length(&self) -> u64 {
        u64::from(128 - self.0.unsigned_abs().leading_zeros())
    }

    fn shifted_f64(&self, shift: u64) -> f64 {
        let bits = u32::try_from(shift)
            .ok()
            .and_then(|shift| self.0.unsigned_abs().checked_shr(shift))
            .unwrap_or(0) as u64;
        let value = bits as f64;
        if self.0 < 0 {
            -value
        } else {
            value
        }
    }
}

/// The position of the lowest set bit of `x`, in units of 2^-1074, or
/// `None` for zero. Every value of a matrix is a multiple of the smallest
/// such power of two, so dividing by it leaves integers, and an input of
/// small integers stays small (#173).
fn lowest_bit(x: f64) -> Option<u64> {
    let (mantissa, shift) = decompose(x);
    (mantissa != 0).then(|| shift + u64::from(mantissa.trailing_zeros()))
}

/// The scale base of `rows` and a guess of the bits its determinant needs:
/// k entries, each with the bits of the largest value after scaling, plus
/// the growth of the sums. A wrong guess only costs a retry, because every
/// operation is checked.
fn plan(rows: Rows<'_>) -> (u64, u64) {
    // One pass: the lowest set bit and the highest bit of every value.
    let (mut base, mut top) = (u64::MAX, 0);
    for x in rows.values() {
        let (mantissa, shift) = decompose(x);
        if mantissa != 0 {
            base = base.min(shift + u64::from(mantissa.trailing_zeros()));
            top = top.max(shift + u64::from(64 - mantissa.leading_zeros()));
        }
    }
    if base == u64::MAX {
        return (0, 0);
    }
    // A difference has one bit more; a lifted height squares it and sums
    // fewer than 2^64 terms.
    let mut bits = top - base + 1;
    if rows.lifted.is_some() {
        bits = 2 * bits + 64;
    }
    (base, rows.k() as u64 * (bits + 4) + 2)
}

/// Runs `f` with the narrowest integer type whose width exceeds
/// `estimate` bits, then with each wider one when a result overflows.
/// `None` when even the widest overflows.
fn widening<R>(estimate: u64, mut f: impl FnMut(Width) -> Result<R, Overflow>) -> Option<R> {
    let widths = [
        (Width::I128, 127),
        (Width::Limbs4, 64 * 4),
        (Width::Limbs8, 64 * 8),
        (Width::Limbs16, u64::MAX),
    ];
    widths
        .iter()
        .filter(|&&(_, bits)| estimate < bits)
        .find_map(|&(width, _)| f(width).ok())
}

/// The integer types of this stage, narrowest first.
#[derive(Clone, Copy)]
enum Width {
    I128,
    Limbs4,
    Limbs8,
    Limbs16,
}

/// The sign of the determinant described by `rows`, or `None` when this
/// stage does not hold it: the matrix is larger than [`MAX_SIZE`], or its
/// values need more limbs than the stage has.
pub(super) fn sign(rows: Rows<'_>) -> Option<Sign> {
    let k = rows.k();
    if k == 0 || k > MAX_SIZE {
        return None;
    }
    let (base, estimate) = plan(rows);
    widening(estimate, |width| match width {
        Width::I128 => determinant_n::<Wide>(rows, base).map(|d| d.sign()),
        Width::Limbs4 => determinant_n::<Fixed<4>>(rows, base).map(|d| d.sign()),
        Width::Limbs8 => determinant_n::<Fixed<8>>(rows, base).map(|d| d.sign()),
        Width::Limbs16 => determinant_n::<Fixed<16>>(rows, base).map(|d| d.sign()),
    })
}

/// The unit direction of the cofactors of the hyperplane through `facet`
/// (k points of dimension k), rounded as
/// [`super::exact::cofactor_direction_exact`] rounds it: `Some(None)` when
/// every cofactor is zero, and `None` when this stage does not hold them.
pub(super) fn cofactor_direction(facet: &[&[f64]]) -> Option<Option<Vec<f64>>> {
    let k = facet.len();
    if k == 0 || k > MAX_SIZE {
        return None;
    }
    let mut unit = [0.0; MAX_SIZE];
    unit[0] = 1.0;
    let (base, estimate) = plan(Rows {
        origin: facet[0],
        points: &facet[1..],
        direction: Some(&unit[..k]),
        lifted: None,
    });
    widening(estimate, |width| match width {
        Width::I128 => cofactor_direction_n::<Wide>(facet, base),
        Width::Limbs4 => cofactor_direction_n::<Fixed<4>>(facet, base),
        Width::Limbs8 => cofactor_direction_n::<Fixed<8>>(facet, base),
        Width::Limbs16 => cofactor_direction_n::<Fixed<16>>(facet, base),
    })
}

fn cofactor_direction_n<T: Int>(facet: &[&[f64]], base: u64) -> Result<Option<Vec<f64>>, Overflow> {
    let k = facet.len();
    let mut unit = [0.0; MAX_SIZE];
    let mut cofactors = [T::default(); MAX_SIZE];
    // Every row set holds the same values (the facet, one 1, and zeros),
    // so every cofactor carries the same power-of-two scale, and `base` is
    // the same for each.
    for (j, cofactor) in cofactors.iter_mut().enumerate().take(k) {
        unit[j] = 1.0;
        *cofactor = determinant_n::<T>(
            Rows {
                origin: facet[0],
                points: &facet[1..],
                direction: Some(&unit[..k]),
                lifted: None,
            },
            base,
        )?;
        unit[j] = 0.0;
    }
    let cofactors = &cofactors[..k];
    let longest = cofactors.iter().map(Int::bit_length).max().unwrap_or(0);
    if longest == 0 {
        return Ok(None);
    }
    let shift = longest.saturating_sub(64);
    let values: Vec<f64> = cofactors.iter().map(|c| c.shifted_f64(shift)).collect();
    let norm = values.iter().map(|x| x * x).sum::<f64>().sqrt();
    Ok(Some(values.iter().map(|x| x / norm).collect()))
}

/// The determinant of `rows` over `T`, for k up to [`MAX_SIZE`].
fn determinant_n<T: Int>(rows: Rows<'_>, base: u64) -> Result<T, Overflow> {
    match rows.k() {
        1 => determinant_of::<1, T>(rows, base),
        2 => determinant_of::<2, T>(rows, base),
        3 => determinant_of::<3, T>(rows, base),
        4 => determinant_of::<4, T>(rows, base),
        5 => determinant_of::<5, T>(rows, base),
        6 => determinant_of::<6, T>(rows, base),
        7 => determinant_of::<7, T>(rows, base),
        8 => determinant_of::<8, T>(rows, base),
        9 => determinant_of::<9, T>(rows, base),
        10 => determinant_of::<10, T>(rows, base),
        _ => Err(Overflow),
    }
}

/// The matrix of `rows` over the scaled integers, as in
/// [`super::exact`], then its determinant.
fn determinant_of<const K: usize, T: Int>(rows: Rows<'_>, base: u64) -> Result<T, Overflow> {
    debug_assert_eq!(rows.k(), K, "matrix size");
    let dim = rows.origin.len();
    let mut origin = [T::default(); K];
    for (slot, &x) in origin.iter_mut().zip(rows.origin) {
        *slot = T::from_f64_scaled(x, base)?;
    }
    // The lifted column is sum X^2 - sum O^2 over the scaled integers.
    let squared_norm = |scaled: &[T]| -> Result<T, Overflow> {
        let mut sum = T::default();
        for x in scaled {
            sum.add_product(x, x, false)?;
        }
        Ok(sum)
    };
    let origin_norm = match rows.lifted {
        Some(_) => Some(squared_norm(&origin[..dim])?),
        None => None,
    };
    let mut a = [[T::default(); K]; K];
    for (i, row) in a.iter_mut().enumerate() {
        match rows.row(i) {
            Row::Difference(point) => {
                for (j, &x) in point.iter().enumerate() {
                    row[j] = T::from_f64_scaled(x, base)?;
                }
                if let Some(origin_norm) = &origin_norm {
                    let mut norm = squared_norm(&row[..dim])?;
                    norm.sub_assign(origin_norm)?;
                    row[dim] = norm;
                }
                for (x, o) in row[..dim].iter_mut().zip(&origin) {
                    x.sub_assign(o)?;
                }
            }
            Row::Direction(direction) => {
                for (slot, &x) in row.iter_mut().zip(direction) {
                    *slot = T::from_f64_scaled(x, base)?;
                }
            }
        }
    }
    if K <= 4 {
        expansion(&a)
    } else {
        determinant(&a)
    }
}

/// Determinant for K <= 4 by cofactor expansion along the first row, with
/// the 2 x 2 minors of the last two rows computed once: 9 products for
/// K = 3 and 28 for K = 4, against about 25 and 70 for Berkowitz.
fn expansion<const K: usize, T: Int>(a: &[[T; K]; K]) -> Result<T, Overflow> {
    debug_assert!(K <= 4, "expansion is for K <= 4");
    // x * y - z * w
    let det2 = |x: &T, y: &T, z: &T, w: &T| {
        let mut d = T::default();
        d.add_product(x, y, false)?;
        d.add_product(z, w, true)?;
        Ok(d)
    };
    match K {
        1 => Ok(a[0][0]),
        2 => det2(&a[0][0], &a[1][1], &a[0][1], &a[1][0]),
        _ => {
            let (r, s) = (K - 2, K - 1);
            // minor[c][d], c < d: the 2 x 2 minor of rows r, s and columns
            // c, d.
            let mut minor = [[T::default(); 4]; 4];
            for c in 0..K {
                for d in c + 1..K {
                    minor[c][d] = det2(&a[r][c], &a[s][d], &a[r][d], &a[s][c])?;
                }
            }
            // The 3 x 3 determinant of the last three rows over the columns
            // `cols`, expanded along the first of those rows.
            let det3 = |cols: [usize; 3]| -> Result<T, Overflow> {
                let row = &a[K - 3];
                let mut d = T::default();
                d.add_product(&row[cols[0]], &minor[cols[1]][cols[2]], false)?;
                d.add_product(&row[cols[1]], &minor[cols[0]][cols[2]], true)?;
                d.add_product(&row[cols[2]], &minor[cols[0]][cols[1]], false)?;
                Ok(d)
            };
            if K == 3 {
                return det3([0, 1, 2]);
            }
            let others = [[1, 2, 3], [0, 2, 3], [0, 1, 3], [0, 1, 2]];
            let mut d = T::default();
            for (c, cols) in others.iter().enumerate() {
                d.add_product(&a[0][c], &det3(*cols)?, c % 2 == 1)?;
            }
            Ok(d)
        }
    }
}

/// Determinant by the Berkowitz algorithm, as
/// [`super::exact::determinant`], with the leading coefficient 1 of every
/// polynomial and of every Toeplitz column left implicit, so `K` slots hold
/// them.
fn determinant<const K: usize, T: Int>(a: &[[T; K]; K]) -> Result<T, Overflow> {
    // poly[i - 1] is the coefficient of x^(r - i) in det(xI - A_r), for
    // i = 1..=r; the coefficient of x^r is 1.
    let mut poly = [T::default(); K];
    let mut next_poly = [T::default(); K];
    // column[i - 1] is entry i of the Toeplitz column, i = 1..=r+1; entry 0
    // is 1.
    let mut column = [T::default(); K];
    let mut v = [T::default(); K];
    let mut next = [T::default(); K];
    poly[0] = a[0][0].negated();
    for r in 1..K {
        column[0] = a[r][r].negated();
        for (slot, row) in v.iter_mut().zip(a).take(r) {
            *slot = row[r];
        }
        for step in 0..r {
            // -(R . v), with the sign folded into each product.
            let mut dot = T::default();
            for (x, y) in a[r][..r].iter().zip(&v[..r]) {
                dot.add_product(x, y, true)?;
            }
            column[step + 1] = dot;
            if step + 1 < r {
                for (sum, row) in next.iter_mut().zip(a).take(r) {
                    *sum = T::default();
                    for (x, y) in row[..r].iter().zip(&v[..r]) {
                        sum.add_product(x, y, false)?;
                    }
                }
                core::mem::swap(&mut v, &mut next);
            }
        }
        // next_poly_i = sum_{j=0}^{min(i, r)} column_{i-j} poly_j for
        // i = 1..=r+1, with column_0 = poly_0 = 1.
        let one = T::one();
        for i in 1..=r + 1 {
            let mut sum = T::default();
            for j in 0..=i.min(r) {
                let c = if i == j { &one } else { &column[i - j - 1] };
                let p = if j == 0 { &one } else { &poly[j - 1] };
                sum.add_product(c, p, false)?;
            }
            next_poly[i - 1] = sum;
        }
        core::mem::swap(&mut poly, &mut next_poly);
    }
    let constant = poly[K - 1];
    Ok(if K % 2 == 1 {
        constant.negated()
    } else {
        constant
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::exact::{
        cofactor_direction_on_heap, determinant_of as heap_determinant, sign_exact_on_heap,
    };
    use crate::predicates::LiftedHeight;

    impl<const N: usize> Fixed<N> {
        fn parts(&self) -> (bool, Vec<u64>) {
            (self.negative, self.limbs[..self.len].to_vec())
        }
    }

    impl Wide {
        fn parts(&self) -> (bool, Vec<u64>) {
            let m = self.0.unsigned_abs();
            let mut limbs = vec![m as u64, (m >> 64) as u64];
            while limbs.last() == Some(&0) {
                limbs.pop();
            }
            (self.0 < 0, limbs)
        }
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        /// Coordinates of five kinds: small integers (exact coplanarities
        /// and zero determinants), values in [-1, 1) (several exponents),
        /// mixed exponents in [2^-60, 2^60], one huge or tiny exponent
        /// (past every limb count), and the extremes of `f64`.
        fn coordinate(&mut self, kind: u64) -> f64 {
            let sign = if self.next().is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            match kind {
                0 => (self.next() % 5) as f64 - 2.0,
                1 => (self.next() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0,
                2 => {
                    sign * (self.next() >> 11) as f64 * 2f64.powi((self.next() % 120) as i32 - 113)
                }
                3 => match self.next() % 4 {
                    0 => sign * 2f64.powi(900),
                    1 => sign * f64::from_bits(1),
                    _ => (self.next() % 5) as f64 - 2.0,
                },
                _ => match self.next() % 3 {
                    0 => sign * f64::MAX,
                    1 => sign * f64::MIN_POSITIVE,
                    _ => 0.0,
                },
            }
        }
    }

    /// Points of dimension `dim`, of one coordinate kind. A repeated point
    /// now and then makes the determinant exactly zero.
    fn points(rng: &mut Rng, count: usize, dim: usize, kind: u64) -> Vec<Vec<f64>> {
        let mut points: Vec<Vec<f64>> = (0..count)
            .map(|_| (0..dim).map(|_| rng.coordinate(kind)).collect())
            .collect();
        if count >= 3 && rng.next().is_multiple_of(4) {
            points[count - 1] = points[1].clone();
        }
        points
    }

    /// `magnitude * 2^shift`, little-endian limbs without high zeros.
    fn shifted_left(magnitude: &[u64], shift: u64) -> Vec<u64> {
        if magnitude.is_empty() {
            return Vec::new();
        }
        let (limbs, bits) = ((shift / 64) as usize, (shift % 64) as u32);
        let mut out = vec![0; limbs];
        let mut carry = 0;
        for &m in magnitude {
            out.push((m << bits) | carry);
            carry = if bits == 0 { 0 } else { m >> (64 - bits) };
        }
        out.push(carry);
        while out.last() == Some(&0) {
            out.pop();
        }
        out
    }

    /// Every limb count either gives the heap integers' exact value or
    /// reports [`Overflow`]; it never returns a wrong value. The heap
    /// integers divide by the smallest exponent, this stage by the lowest
    /// set bit, so the values differ by 2^(delta per entry) in every
    /// column, twice in the lifted one.
    fn check_every_limb_count(rows: Rows<'_>, held: &mut [usize; 4], signs: &mut [usize; 3]) {
        let want = heap_determinant(rows).unwrap();
        let base = rows.values().filter_map(lowest_bit).min().unwrap_or(0);
        let heap_base = rows
            .values()
            .filter_map(crate::predicates::exact::exponent_shift)
            .min()
            .unwrap_or(0);
        let columns = rows.k() as u64 + u64::from(rows.lifted.is_some());
        let shift = (base - heap_base) * columns;
        let results = [
            determinant_n::<Wide>(rows, base).map(|d| d.parts()),
            determinant_n::<Fixed<4>>(rows, base).map(|d| d.parts()),
            determinant_n::<Fixed<8>>(rows, base).map(|d| d.parts()),
            determinant_n::<Fixed<16>>(rows, base).map(|d| d.parts()),
        ];
        for (n, result) in results.iter().enumerate() {
            if let Ok((negative, magnitude)) = result {
                assert_eq!(
                    (*negative, shifted_left(magnitude, shift)),
                    want.parts(),
                    "limb set {n}"
                );
                held[n] += 1;
            }
        }
        // The stage's own choice agrees with the heap path whenever it
        // holds the value.
        if let Some(sign) = sign(rows) {
            assert_eq!(sign, sign_exact_on_heap(rows).unwrap());
        }
        signs[want.sign() as usize] += 1;
    }

    #[test]
    fn fixed_determinants_equal_the_heap_integers() {
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        for kind in 0..5 {
            let (mut held, mut signs) = ([0usize; 4], [0usize; 3]);
            // The heap integers of kinds 3 and 4 hold thousands of bits; a
            // debug build takes seconds per size past 6.
            let largest = if kind >= 3 { 6 } else { MAX_SIZE };
            for k in 1..=largest {
                for _ in 0..12 {
                    // Orientation: k + 1 points of dimension k.
                    let p = points(&mut rng, k + 1, k, kind);
                    let refs: Vec<&[f64]> = p.iter().map(Vec::as_slice).collect();
                    let rows = Rows {
                        origin: refs[0],
                        points: &refs[1..],
                        direction: None,
                        lifted: None,
                    };
                    check_every_limb_count(rows, &mut held, &mut signs);
                    // A direction row after k - 1 edges.
                    let direction: Vec<f64> = (0..k).map(|_| rng.coordinate(kind)).collect();
                    let rows = Rows {
                        origin: refs[0],
                        points: &refs[1..k],
                        direction: Some(&direction),
                        lifted: None,
                    };
                    check_every_limb_count(rows, &mut held, &mut signs);
                    // Lifted: k + 1 points of dimension k - 1 stand for
                    // points of dimension k.
                    if k >= 2 {
                        let p = points(&mut rng, k + 1, k - 1, kind);
                        let refs: Vec<&[f64]> = p.iter().map(Vec::as_slice).collect();
                        let heights: Vec<LiftedHeight> =
                            refs.iter().map(|p| LiftedHeight::of(p)).collect();
                        let rows = Rows {
                            origin: refs[0],
                            points: &refs[1..],
                            direction: None,
                            lifted: Some(&heights),
                        };
                        check_every_limb_count(rows, &mut held, &mut signs);
                    }
                }
            }
            // Each kind reaches the cases it is built for. Per kind there
            // are 12 cases of each of the three row sets per size, and no
            // lifted set at k = 1.
            let total = 12 * (3 * largest - 1);
            // held[0] counts the i128 width, then 4, 8, and 16 limbs.
            match kind {
                // Small integers stay small: one i128 holds every one.
                0 => assert_eq!(held[0], total, "kind 0: {held:?}"),
                // Coordinates in [-1, 1) span about 120 bits: sixteen limbs
                // hold every one, and the i128 some.
                1 => assert!(held[3] == total && held[0] > 0, "kind 1: {held:?}"),
                // Exponents 2^900 and 2^-1074 together pass every width,
                // so the heap integers take most of them.
                3 | 4 => assert!(held[3] < total / 2, "kind {kind}: {held:?}"),
                _ => {}
            }
            if kind == 0 {
                assert!(
                    signs.iter().all(|&c| c > 0),
                    "kind 0: every sign: {signs:?}"
                );
            }
        }
    }

    #[test]
    fn fixed_cofactor_direction_equals_the_heap_one() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let (mut held, mut zero) = (0, 0);
        for kind in 0..5 {
            // The heap integers of kinds 3 and 4 hold thousands of bits; a
            // debug build takes seconds per size past 6.
            let largest = if kind >= 3 { 6 } else { MAX_SIZE };
            for k in 1..=largest {
                for _ in 0..8 {
                    let p = points(&mut rng, k, k, kind);
                    let refs: Vec<&[f64]> = p.iter().map(Vec::as_slice).collect();
                    let want = cofactor_direction_on_heap(&refs).unwrap();
                    if let Some(got) = cofactor_direction(&refs) {
                        // Bit for bit: the same integers, shifted and
                        // rounded the same way.
                        let bits = |d: &Option<Vec<f64>>| {
                            d.as_ref()
                                .map(|v| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>())
                        };
                        assert_eq!(bits(&got), bits(&want), "{p:?}");
                        held += 1;
                        zero += usize::from(got.is_none());
                    }
                }
            }
        }
        assert!(held > 0 && zero > 0, "held {held}, all zero {zero}");
    }

    #[test]
    fn a_product_past_the_limbs_overflows() {
        // 2^255 needs all four limbs; its square needs eight.
        let x = Fixed::<4>::from_f64_scaled(2f64.powi(255 - 1074), 0).unwrap();
        assert_eq!(x.bit_length(), 256);
        let mut sum = Fixed::<4>::default();
        assert_eq!(sum.add_product(&x, &x, false), Err(Overflow));
        // A carry out of the top limb overflows too.
        let mut sum = x;
        assert_eq!(sum.accumulate(&x.limbs, x.len, false), Err(Overflow));
        // The same values fit in eight limbs.
        let y = Fixed::<8>::from_f64_scaled(2f64.powi(255 - 1074), 0).unwrap();
        let mut sum = Fixed::<8>::default();
        assert!(sum.add_product(&y, &y, false).is_ok());
        assert_eq!(sum.bit_length(), 511);
        // A value past the limbs does not convert.
        assert_eq!(
            Fixed::<4>::from_f64_scaled(2f64.powi(256 - 1074), 0).map(|f| f.len),
            Err(Overflow)
        );
    }
}
