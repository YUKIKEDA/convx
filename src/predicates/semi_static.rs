//! The first stage of the orientations and lifted orientations with k ≤ 5
//! (#249, #350): the determinant in `f64`, certified by a constant times
//! the permanent of the same expression, both evaluated in the same call.
//!
//! The running bound of [`super::filter`] updates a bound on every
//! operation. Here the bound is derived once per formula, so a call costs
//! about two evaluations of the expression instead of five or more. When it
//! does not certify the sign, the running bound is tried next, then the
//! exact stage, so the sign returned is unchanged.
//!
//! # The bound
//!
//! Every formula takes the difference rows `d_i = p_i - p_0` and, for the
//! lifted orientation, the column `l_i = |d_i|^2` in place of
//! `|p_i|^2 - |p_0|^2`. Since `|p_i|^2 - |p_0|^2 = |d_i|^2 + 2 p_0 . d_i`,
//! the two columns differ by a combination of the coordinate columns, and
//! the determinant is the same polynomial in the input coordinates
//! (design §7 allows another expression with the same sign).
//!
//! Rounding is to nearest. With `u = 2^-53` and `η = 2^-1074`:
//!
//! - `fl(x ± y) = (x ± y)(1 + δ)`, `|δ| ≤ u`. An addition has no underflow
//!   error: a subnormal sum is exact.
//! - `fl(x y) = x y (1 + δ) + ε`, `|δ| ≤ u`, `|ε| ≤ η / 2`.
//!
//! Leave `ε` aside first. Expanding a formula gives a sum of monomials in
//! the exact `d_i`, each carrying one factor `(1 + δ)` per rounding on its
//! way to the root: one for the subtraction that made each of its leaves,
//! one per product and per sum above them. When no monomial carries more
//! than `n` factors, `|fl(E) - E| ≤ γ_n P` with `γ_n = n u / (1 - n u)` and
//! `P` the sum of the monomials' absolute values (Higham, Lemma 3.1). `P` is
//! the same expression with every leaf replaced by its absolute value and
//! every subtraction by an addition. Its computed value `P̂` uses the same
//! tree, whose operations only add nonnegative values, so
//! `P̂ ≥ (1 - u)^n P` up to underflow. Then
//! `γ_n P ≤ γ_n (1 - u)^-n P̂ ≤ n u (1 + (2n + 1) u) P̂`, and `(n + 1) u`
//! covers that and the two roundings of the bound itself, as
//! `n (2n + 2) u < 1` for every `n` here.
//!
//! An underflowing product adds at most `η / 2`, which the products above
//! it scale by the absolute values of their other factors. Each formula
//! states a sum `X ≥ 1` of absolute values it computes anyway that bounds
//! those scaled errors by `η X (1 + u)^n`. The same holds for `P̂`, scaled
//! by `γ_n`. `4 η X`, rounded, covers both and the rounding of the bound.
//!
//! The sign is certified when `|det| > (n + 1) u P̂ + 4 η X`, computed in
//! `f64`. A non-finite value or bound certifies nothing: an overflow in the
//! determinant overflows the permanent too, and a NaN fails both tests.
//!
//! # Size five
//!
//! For k = 5 (the plain orientation of D = 5, the lifted one of D = 4) the
//! determinant is expanded along its last column, over the minors of the
//! leading columns, one subset of rows at a time: the minor of a subset `S`
//! of `m` rows over columns `0 .. m` is
//! `sum_i (-1)^(i + m - 1) a[r_i][m - 1] M(S - r_i)`, the rows `r_i` of `S`
//! in increasing order. The `m` terms are added pairwise, in a tree of depth
//! `ceil(log2 m)`. Every minor is computed once and shared.
//!
//! A monomial of a minor of `m` columns carries the roundings of its child
//! minor, those of its last-column entry, one for the product, and one per
//! level of the sum: `n_m = n_(m-1) + e + 1 + ceil(log2 m)`, with `n_1 = 1`
//! (the difference that made a leaf). A coordinate entry carries `e = 1`. A
//! lifted entry `sum_c d_c^2` of `D` coordinates, added pairwise, carries two
//! roundings of its leaf, one for the square, and `ceil(log2 D)` for the
//! sum: `e = 3 + ceil(log2 D)`. This recursion gives the constants of the
//! formulas above (4 for k = 2, 8 for k = 3, 12 for k = 4, 11 for the lifted
//! D = 2, 16 for the lifted D = 3), and here `n = 17` (plain) and `n = 21`
//! (lifted).
//!
//! The permanent `P` of the absolute values is not computed. Every monomial
//! takes one entry from each column, so `P <= prod_c s_c`, with
//! `s_c = sum_r |a[r][c]|` the sums of the columns' absolute values, and
//! `P̂ = fl(prod_c s_c)` stands for it. Each of its `2k - 2` roundings only
//! adds nonnegative values or multiplies them, so `P̂ >= (1 - u)^(2k - 2) P`,
//! which `(n + 1) u` covers as it covers `(1 - u)^n` above.
//!
//! The underflow term uses the same sums. Each product of a minor underflows
//! by at most `η / 2`, scaled by the entries above it. With `x_1 = 0` and
//! `x_m = s_(m-1) x_(m-1) + m / 2`, the scaled underflow of every minor of
//! `m` coordinate columns is at most `η x_m (1 + u)^n`. A lifted last column
//! adds, for each of its `m` products, the underflow of its entry, at most
//! `D η / 2`, scaled by the child minor, whose absolute value is at most
//! `p_(m-1) = s_0 s_1 ... s_(m-2)`: so
//! `x_m = s_l x_(m-1) + m p_(m-1) D / 2 + m / 2`, with `s_l` the sum of the
//! lifted entries. `X = max(1, x_k)`, and the bound is that of every formula,
//! `(n + 1) u P̂ + 4 η X`.
//!
//! Size six is left to the running filter. The same expansion took 296 ns a
//! test at k = 6, against 270 ns for the running filter, and made Delaunay
//! `cube` D5 10^4 6% slower; at k = 5 it took 143 ns against 208 (#350).
//!
//! # The bound without a subnormal product
//!
//! `4 η = 2^-1072` is subnormal, and a product with a subnormal factor is
//! slow on common processors: on an Intel Core i5-13400F it took about
//! 28 ns, against about 6 ns for the rest of an in-circle test (#311). So
//! the bound is computed as `s (1 + 2^-50)` with `s = fl((n + 1) u P̂)`,
//! when `s ≥ 2^-960` and `X ≤ 2^60`, and as `fl(s + fl(4 η X))` otherwise.
//! The first is never smaller than the second, so it certifies a subset of
//! what the proof above allows:
//!
//! - `4 η X ≤ 2^-1012`, and `fl(4 η X)` is at most that plus `η / 2`, so
//!   `t = fl(4 η X) ≤ 2^-1011 ≤ 2^-51 s`.
//! - `fl(s + t) ≤ (s + t)(1 + u) ≤ s (1 + 2^-51)(1 + 2^-53)`.
//! - `s (1 + 2^-50)` is a normal number or infinity (`1 + 2^-50` is
//!   exact), so its rounding is at least `s (1 + 2^-50)(1 - 2^-53)`, and
//!   `(1 + 2^-50)(1 - 2^-53) - (1 + 2^-51)(1 + 2^-53) > 2^-53 > 0`.
//!
//! A NaN `s` or `X` fails the condition and takes the second form, as
//! before. The second form is out of line and marked cold: written in
//! place, the compiler evaluated both forms and selected one, and the
//! product was made on every call (#311).

use super::Sign;

const U: f64 = f64::EPSILON / 2.0;

/// `4 η`, four times the smallest subnormal.
const FOUR_ETA: f64 = f64::from_bits(4);

/// The smallest `(n + 1) u P̂` for which the bound needs no product with
/// `4 η`: `2^-960`.
const PLAIN_FROM: f64 = f64::from_bits((1023 - 960) << 52);

/// The largest `X` for which the bound needs no product with `4 η`: `2^60`.
const PLAIN_UP_TO: f64 = f64::from_bits((1023 + 60) << 52);

/// `1 + 2^-50`, the margin that covers `4 η X` below [`PLAIN_UP_TO`].
const MARGIN: f64 = 1.0 + f64::from_bits((1023 - 50) << 52);

/// The relative constant `(n + 1) u` of a formula whose monomials carry at
/// most `n` roundings.
const fn relative(n: u32) -> f64 {
    (n + 1) as f64 * U
}

/// A determinant evaluated in `f64` and the bound on its error.
#[derive(Clone, Copy)]
struct Estimate {
    det: f64,
    bound: f64,
}

/// The bound `relative * permanent + 4 η x`, computed as the module docs
/// prove (The bound without a subnormal product).
#[inline(always)]
fn bound(relative: f64, permanent: f64, x: f64) -> f64 {
    let s = relative * permanent;
    if s >= PLAIN_FROM && x <= PLAIN_UP_TO {
        s * MARGIN
    } else {
        with_underflow_term(s, x)
    }
}

/// `s + 4 η x`, rounded: the bound for a small `s` or a large `x`.
#[cold]
#[inline(never)]
fn with_underflow_term(s: f64, x: f64) -> f64 {
    s + FOUR_ETA * x
}

impl Estimate {
    /// `det` with the bound `relative * permanent + 4 η x`.
    #[inline(always)]
    fn new(det: f64, relative: f64, permanent: f64, x: f64) -> Self {
        Self {
            det,
            bound: bound(relative, permanent, x),
        }
    }

    /// The sign of the determinant when its absolute value exceeds the
    /// bound.
    #[inline(always)]
    fn sign(self) -> Option<Sign> {
        if self.det > self.bound {
            Some(Sign::Positive)
        } else if -self.det > self.bound {
            Some(Sign::Negative)
        } else {
            None
        }
    }
}

/// The sign of the orientation of `origin` followed by `points`, or of the
/// lifted orientation when `lifted`, for k ≤ 4. `None` when the bound does
/// not certify it, or when no formula here covers the size.
///
/// `points` holds k rows of the dimension of `origin`: one per coordinate,
/// and one more when `lifted`.
#[inline]
pub(super) fn sign(origin: &[f64], points: &[&[f64]], lifted: bool) -> Option<Sign> {
    estimate(origin, points, lifted)?.sign()
}

/// The determinant [`sign`] decides and its bound, or `None` when no formula
/// here covers the size.
#[inline(always)]
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
    let diff = |i: usize, j: usize| points[i][j] - origin[j];
    match (dim, lifted) {
        (2, false) => Some(orient2([diff(0, 0), diff(0, 1)], [diff(1, 0), diff(1, 1)])),
        (3, false) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1), diff(i, 2)];
            Some(orient3(row(0), row(1), row(2)))
        }
        (4, false) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1), diff(i, 2), diff(i, 3)];
            Some(orient4(row(0), row(1), row(2), row(3)))
        }
        (1, true) => Some(lifted1(diff(0, 0), diff(1, 0))),
        (2, true) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1)];
            Some(lifted2(row(0), row(1), row(2)))
        }
        (3, true) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1), diff(i, 2)];
            Some(lifted3(row(0), row(1), row(2), row(3)))
        }
        // Out of line, so that the formulas above stay small where they
        // are inlined.
        (5, false) | (4, true) => Some(large::<5>(origin, points, lifted)),
        _ => None,
    }
}

/// `x0 y1 - x1 y0` and its permanent. Each monomial carries 4 roundings:
/// two leaves, the product, and the difference.
#[inline(always)]
fn minor2(x: [f64; 2], y: [f64; 2]) -> (f64, f64) {
    let (p, q) = (x[0] * y[1], x[1] * y[0]);
    (p - q, p.abs() + q.abs())
}

/// The 3 x 3 determinant of rows `x`, `y`, `z`, expanded along column 2
/// over the minors of columns 0 and 1, with its permanent: `n = 8` (a
/// minor's 4, the product, and two sums).
#[inline(always)]
fn minor3(
    (x, y, z): ([f64; 3], [f64; 3], [f64; 3]),
    (yz, xz, xy): ((f64, f64), (f64, f64), (f64, f64)),
) -> (f64, f64) {
    let det = (x[2] * yz.0 - y[2] * xz.0) + z[2] * xy.0;
    let permanent = (x[2].abs() * yz.1 + y[2].abs() * xz.1) + z[2].abs() * xy.1;
    (det, permanent)
}

fn first2(row: [f64; 3]) -> [f64; 2] {
    [row[0], row[1]]
}

fn first3(row: [f64; 4]) -> [f64; 3] {
    [row[0], row[1], row[2]]
}

/// Orientation, k = 2: `n = 4`. Each of the two products underflows by at
/// most `η / 2`, so `X = 1`.
fn orient2(a: [f64; 2], b: [f64; 2]) -> Estimate {
    let (det, permanent) = minor2(a, b);
    Estimate::new(det, relative(4), permanent, 1.0)
}

/// Orientation, k = 3: [`minor3`], `n = 8`. Each of the six products of the
/// minors underflows by at most `η / 2`, scaled by the column-2 entry it is
/// multiplied with; each of the three outer products by `η / 2`. So
/// `X = |a2| + |b2| + |c2| + 2`.
fn orient3(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Estimate {
    let (a2, b2, c2) = (first2(a), first2(b), first2(c));
    let (det, permanent) = minor3((a, b, c), (minor2(b2, c2), minor2(a2, c2), minor2(a2, b2)));
    let x = (a[2].abs() + b[2].abs()) + c[2].abs() + 2.0;
    Estimate::new(det, relative(8), permanent, x)
}

/// The six minors of columns 0 and 1 of four rows, in the order
/// `ab, ac, ad, bc, bd, cd`.
#[inline(always)]
fn minors2_of_four(rows: [[f64; 2]; 4]) -> [(f64, f64); 6] {
    let [a, b, c, d] = rows;
    [
        minor2(a, b),
        minor2(a, c),
        minor2(a, d),
        minor2(b, c),
        minor2(b, d),
        minor2(c, d),
    ]
}

/// The 4 x 4 determinant of rows `a, b, c, d`, expanded along column 3 over
/// the 3 x 3 minors of columns 0 to 2 ([`minor3`]):
/// `(b3 M_acd - a3 M_bcd) + (d3 M_abc - c3 M_abd)`. Returns it, its
/// permanent, and each minor's permanent, in the order of the rows left
/// out. A monomial carries the 8 roundings of its minor, the roundings of
/// its column-3 entry, the product, and two sums.
#[inline(always)]
fn expand4(rows: [[f64; 3]; 4], column: [f64; 4]) -> (f64, f64, [f64; 4]) {
    let [a, b, c, d] = rows;
    let [ab, ac, ad, bc, bd, cd] = minors2_of_four(rows.map(first2));
    let m_bcd = minor3((b, c, d), (cd, bd, bc));
    let m_acd = minor3((a, c, d), (cd, ad, ac));
    let m_abd = minor3((a, b, d), (bd, ad, ab));
    let m_abc = minor3((a, b, c), (bc, ac, ab));
    let [a3, b3, c3, d3] = column;
    let det = (b3 * m_acd.0 - a3 * m_bcd.0) + (d3 * m_abc.0 - c3 * m_abd.0);
    let permanent =
        (b3.abs() * m_acd.1 + a3.abs() * m_bcd.1) + (d3.abs() * m_abc.1 + c3.abs() * m_abd.1);
    (det, permanent, [m_bcd.1, m_acd.1, m_abd.1, m_abc.1])
}

/// Orientation, k = 4: [`expand4`] with column 3 a leaf, `n = 12`. Each
/// minor's underflow is at most `η (Z + 3/2)` ([`orient3`]), with `Z` the sum
/// of its column-2 entries, at most `Z = |a2| + |b2| + |c2| + |d2|`, and
/// is scaled by the column-3 entry; each of the four outer products adds
/// `η / 2`. So `X = (|a3| + |b3| + |c3| + |d3|) (Z + 2) + 2`.
fn orient4(a: [f64; 4], b: [f64; 4], c: [f64; 4], d: [f64; 4]) -> Estimate {
    let rows = [a, b, c, d];
    let (det, permanent, _) = expand4(rows.map(first3), rows.map(|r| r[3]));
    let z = (a[2].abs() + b[2].abs()) + (c[2].abs() + d[2].abs());
    let s = (a[3].abs() + b[3].abs()) + (c[3].abs() + d[3].abs());
    Estimate::new(det, relative(12), permanent, s * (z + 2.0) + 2.0)
}

/// Lifted orientation, D = 1: rows `(a, a^2)` and `(b, b^2)`,
/// `det = a b^2 - a^2 b`. A square carries 3 roundings, a product 5, the
/// difference 6. Each square underflows by at most `η / 2`, scaled by the
/// other coordinate; each product by `η / 2`. So `X = |a| + |b| + 1`.
fn lifted1(a: f64, b: f64) -> Estimate {
    let (la, lb) = (a * a, b * b);
    let (p, q) = (a * lb, la * b);
    let x = a.abs() + b.abs() + 1.0;
    Estimate::new(p - q, relative(6), p.abs() + q.abs(), x)
}

/// Lifted orientation, D = 2: rows `(x, y, x^2 + y^2)`, expanded along the
/// lifted column over the minors of the coordinates (the in-circle test).
/// A lifted entry carries 4 roundings, a minor 4, a term 9, and two sums
/// make `n = 11`. A lifted entry underflows by at most `η`, scaled by its
/// minor; a minor by `η`, scaled by its lifted entry; each outer product
/// by `η / 2`. So `X` is the sum of the minors' permanents, of the lifted
/// entries, and 2.
fn lifted2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Estimate {
    let lift = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let (la, lb, lc) = (lift(a), lift(b), lift(c));
    let (bc, ac, ab) = (minor2(b, c), minor2(a, c), minor2(a, b));
    let det = (la * bc.0 - lb * ac.0) + lc * ab.0;
    let permanent = (la * bc.1 + lb * ac.1) + lc * ab.1;
    let x = ((bc.1 + ac.1) + ab.1) + ((la + lb) + lc) + 2.0;
    Estimate::new(det, relative(11), permanent, x)
}

/// Lifted orientation, D = 3: [`expand4`] with column 3 the lifted entries
/// `(x^2 + y^2) + z^2` (the in-sphere test). A lifted entry carries 5
/// roundings, a term 14, and two sums make `n = 16`. A lifted entry
/// underflows by at most `3 η / 2`, scaled by its minor's permanent; a minor
/// as in [`orient4`], scaled by its lifted entry; each outer product by
/// `η / 2`. So `X = 2 (sum of the minors' permanents) + (sum of the lifted
/// entries) (Z + 2) + 2`.
fn lifted3(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> Estimate {
    let rows = [a, b, c, d];
    let lifts = rows.map(|p| (p[0] * p[0] + p[1] * p[1]) + p[2] * p[2]);
    let (det, permanent, minors) = expand4(rows, lifts);
    let z = (a[2].abs() + b[2].abs()) + (c[2].abs() + d[2].abs());
    let l = (lifts[0] + lifts[1]) + (lifts[2] + lifts[3]);
    let m = (minors[0] + minors[1]) + (minors[2] + minors[3]);
    Estimate::new(det, relative(16), permanent, 2.0 * m + l * (z + 2.0) + 2.0)
}

/// The sum of `terms` (at most 5), added pairwise in a tree of depth
/// `ceil(log2 n)` (module docs, Size five).
#[inline(always)]
fn pairwise_sum(terms: &[f64]) -> f64 {
    match terms.len() {
        0 => 0.0,
        1 => terms[0],
        2 => terms[0] + terms[1],
        3 => (terms[0] + terms[1]) + terms[2],
        4 => (terms[0] + terms[1]) + (terms[2] + terms[3]),
        _ => ((terms[0] + terms[1]) + (terms[2] + terms[3])) + terms[4],
    }
}

/// The rounding count `n` of size five (module docs): 17 for the plain
/// k = 5 and 21 for the lifted D = 4. The test
/// `the_rounding_recursion_gives_every_constant` derives them, and the
/// constants of k <= 4, from the recursion.
fn large_roundings(k: usize, lifted: Option<usize>) -> u32 {
    match (k, lifted) {
        (5, None) => 17,
        (5, Some(4)) => 21,
        _ => {
            debug_assert!(false, "no large formula of size {k}, lifted {lifted:?}");
            u32::MAX
        }
    }
}

/// The masks of the nonempty subsets of `K` rows, by size: the subsets of
/// size `m` are `order[start[m]..start[m + 1]]`.
pub(super) const fn subsets_by_size<const K: usize>() -> ([u8; 64], [usize; 8]) {
    let mut order = [0_u8; 64];
    let mut start = [0_usize; 8];
    let mut n = 0;
    let mut m = 1;
    while m <= K {
        start[m] = n;
        let mut subset = 1;
        while subset < 1 << K {
            if (subset as u32).count_ones() as usize == m {
                order[n] = subset as u8;
                n += 1;
            }
            subset += 1;
        }
        m += 1;
    }
    start[K + 1] = n;
    (order, start)
}

/// The determinant of size `K` (5) of the rows `points[i] - origin`, with
/// the lifted entries `|points[i] - origin|^2` as the last column when
/// `lifted`, and its bound (module docs, Size five).
#[inline(never)]
fn large<const K: usize>(origin: &[f64], points: &[&[f64]], lifted: bool) -> Estimate {
    let dim = origin.len();
    debug_assert_eq!(dim + usize::from(lifted), K, "a determinant of size K");
    let mut rows = [[0.0; K]; K];
    for (row, point) in rows.iter_mut().zip(points) {
        let mut squares = [0.0; 5];
        for c in 0..dim {
            row[c] = point[c] - origin[c];
            squares[c] = row[c] * row[c];
        }
        if lifted {
            row[dim] = pairwise_sum(&squares[..dim]);
        }
    }
    let (order, start) = const { subsets_by_size::<K>() };
    // Minors over row subsets, as bit masks of the K rows; those of size m
    // are over the columns 0 .. m.
    let mut det = [0.0_f64; 64];
    for (r, row) in rows.iter().enumerate() {
        det[1 << r] = row[0];
    }
    for m in 2..=K {
        let column = m - 1;
        for &subset in &order[start[m]..start[m + 1]] {
            let subset = usize::from(subset);
            let mut terms = [0.0; 5];
            let mut bits = subset;
            let mut i = 0;
            while bits != 0 {
                let r = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                let child = subset & !(1 << r);
                let entry = rows[r][column];
                let term = entry * det[child];
                terms[i] = if (i + column) % 2 == 0 { term } else { -term };
                i += 1;
            }
            det[subset] = pairwise_sum(&terms[..m]);
        }
    }
    // The underflow bound from the column sums (module docs).
    let column_sum = |c: usize| -> f64 { rows.iter().map(|row| row[c].abs()).sum() };
    let mut x = 0.0;
    let mut leading = 1.0;
    for m in 2..=K {
        let s_last = column_sum(m - 1);
        let half = m as f64 / 2.0;
        x = if lifted && m == K {
            s_last * x + m as f64 * leading * dim as f64 / 2.0 + half
        } else {
            s_last * x + half
        };
        leading *= column_sum(m - 2);
    }
    // The permanent of the absolute values is at most the product of the
    // column sums (module docs, Size five).
    let columns = (0..K).fold(1.0, |product, c| product * column_sum(c));
    let full = (1_usize << K) - 1;
    Estimate::new(
        det[full],
        relative(large_roundings(K, lifted.then_some(dim))),
        columns,
        x.max(1.0),
    )
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::exact::{exponent_shift, BigInt};
    use super::super::{exact, LiftedHeight, Rows};
    use super::*;

    /// The exact sign of the orientation (or lifted orientation) of
    /// `points`, from the exact stage.
    pub(in crate::predicates) fn exact_of(points: &[Vec<f64>], lifted: bool) -> Sign {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        let heights: Vec<LiftedHeight> = refs.iter().map(|p| LiftedHeight::of(p)).collect();
        exact::sign_exact(Rows {
            origin: refs[0],
            points: &refs[1..],
            direction: None,
            lifted: lifted.then_some(&heights[..]),
        })
        .unwrap()
    }

    fn semi_of(points: &[Vec<f64>], lifted: bool) -> Option<Sign> {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        sign(refs[0], &refs[1..], lifted)
    }

    pub(in crate::predicates) struct Rng(pub(in crate::predicates) u64);

    impl Rng {
        pub(in crate::predicates) fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        pub(in crate::predicates) fn unit(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        }

        pub(in crate::predicates) fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// Every formula: (dimension of a point, lifted).
    const FORMULAS: [(usize, bool); 8] = [
        (2, false),
        (3, false),
        (4, false),
        (1, true),
        (2, true),
        (3, true),
        (5, false),
        (4, true),
    ];

    /// The number of points of a formula: k + 1, with k = dim (+ 1 when
    /// lifted).
    pub(in crate::predicates) fn count(dim: usize, lifted: bool) -> usize {
        dim + 1 + usize::from(lifted)
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
        match semi_of(points, lifted) {
            Some(sign) => {
                assert_eq!(sign, expected, "{what}: {points:?}");
                tally.certified += 1;
            }
            None => tally.open += 1,
        }
    }

    /// The rounding counts of every formula from the recursion of the
    /// module docs (Size five): `n_1 = 1`,
    /// `n_m = n_(m-1) + e + 1 + ceil(log2 m)`, with `e = 1` for a
    /// coordinate column and `e = 3 + ceil(log2 D)` for a lifted one.
    #[test]
    fn the_rounding_recursion_gives_every_constant() {
        let ceil_log2 = |m: usize| usize::BITS - (m - 1).leading_zeros();
        let plain = |k: usize| (2..=k).fold(1, |n, m| n + 1 + 1 + ceil_log2(m));
        let lifted = |dim: usize| plain(dim) + (3 + ceil_log2(dim)) + 1 + ceil_log2(dim + 1);
        // The formulas of k <= 4 state these in their docs.
        assert_eq!([plain(2), plain(3), plain(4)], [4, 8, 12]);
        assert_eq!([lifted(2), lifted(3)], [11, 16]);
        assert_eq!(plain(5), large_roundings(5, None));
        assert_eq!(lifted(4), large_roundings(5, Some(4)));
    }

    #[test]
    fn general_position_is_certified_and_exact() {
        let mut rng = Rng(3);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for _ in 0..500 {
                let points: Vec<Vec<f64>> = (0..count(dim, lifted))
                    .map(|_| (0..dim).map(|_| rng.unit()).collect())
                    .collect();
                check(&points, lifted, &mut tally, "random");
            }
            assert!(
                tally.certified >= 490,
                "dim {dim}, lifted {lifted}: {tally:?}"
            );
        }
    }

    #[test]
    fn exact_zeros_are_left_open() {
        // Small integer grids: many exact zeros, every one computed
        // without rounding, so the determinant is 0 and stays open.
        let mut rng = Rng(5);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for _ in 0..2000 {
                let points: Vec<Vec<f64>> = (0..count(dim, lifted))
                    .map(|_| (0..dim).map(|_| rng.below(5) as f64 - 2.0).collect())
                    .collect();
                check(&points, lifted, &mut tally, "grid");
                if exact_of(&points, lifted) == Sign::Zero {
                    assert_eq!(semi_of(&points, lifted), None, "{points:?}");
                }
            }
            assert!(
                tally.zero > 0 && tally.certified > 0,
                "dim {dim}, lifted {lifted}: {tally:?}"
            );
        }
    }

    /// Points on a lower-dimensional affine hull (or, lifted, a sphere) up
    /// to rounding, moved far from the origin so that every difference
    /// rounds: the determinant cancels to near zero.
    pub(in crate::predicates) fn near_degenerate(
        rng: &mut Rng,
        dim: usize,
        lifted: bool,
        offset: f64,
    ) -> Vec<Vec<f64>> {
        let n = count(dim, lifted);
        let mut points: Vec<Vec<f64>> = (0..n)
            .map(|_| (0..dim).map(|_| rng.unit()).collect())
            .collect();
        if lifted && dim == 1 {
            // Two points of D = 1 are a sphere; a third close to one of
            // them makes the determinant `d1 d2 (d2 - d1)` cancel.
            let tiny = (rng.below(64) as f64 + 1.0) * 2f64.powi(-50);
            points[2][0] = points[1][0] * (1.0 + tiny);
        } else if lifted {
            // On the unit sphere through the rounding of a normalization.
            for p in &mut points {
                let r = p.iter().map(|x| x * x).sum::<f64>().sqrt();
                p.iter_mut().for_each(|x| *x /= r);
            }
        } else {
            let weights: Vec<f64> = (0..n - 1).map(|_| rng.unit()).collect();
            let sum: f64 = weights.iter().sum();
            let last: Vec<f64> = (0..dim)
                .map(|j| (0..n - 1).map(|i| weights[i] / sum * points[i][j]).sum())
                .collect();
            points[n - 1] = last;
        }
        for p in &mut points {
            p.iter_mut().for_each(|x| *x += offset);
        }
        points
    }

    #[test]
    fn cancelling_determinants_are_exact_when_certified() {
        let mut rng = Rng(9);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for trial in 0..3000 {
                let offset = [0.0, 1.0, 1e3, 1e6][trial % 4];
                let points = near_degenerate(&mut rng, dim, lifted, offset);
                check(&points, lifted, &mut tally, "near degenerate");
            }
            // Both outcomes occur: the family reaches the bound.
            assert!(
                tally.certified > 0 && tally.open > 0,
                "dim {dim}, lifted {lifted}: {tally:?}"
            );
        }
    }

    #[test]
    fn every_exponent_and_mixed_magnitudes() {
        let mut rng = Rng(13);
        for (dim, lifted) in FORMULAS {
            let mut tally = Tally::default();
            for e in (-1074..=1023).step_by(37).chain([-1074, -1022, 1023]) {
                let scale = two_to(e);
                for _ in 0..4 {
                    // One scale for every coordinate; at the ends the
                    // differences or the products overflow or underflow.
                    let points: Vec<Vec<f64>> = (0..count(dim, lifted))
                        .map(|_| (0..dim).map(|_| rng.unit() * scale).collect())
                        .collect();
                    check(&points, lifted, &mut tally, "scaled");
                    // A scale per coordinate, from the whole range.
                    let mixed: Vec<Vec<f64>> = (0..count(dim, lifted))
                        .map(|_| {
                            (0..dim)
                                .map(|_| {
                                    let e = rng.below(2097) as i32 - 1074;
                                    rng.unit() * two_to(e)
                                })
                                .collect()
                        })
                        .collect();
                    check(&mixed, lifted, &mut tally, "mixed");
                }
            }
            assert!(
                tally.certified > 0 && tally.open > 0,
                "dim {dim}, lifted {lifted}: {tally:?}"
            );
        }
    }

    #[test]
    fn one_ulp_from_zero() {
        // An exact zero, then its last coordinate moved one ulp each way.
        let unit = |dim: usize, axis: usize, sign: f64| -> Vec<f64> {
            let mut v = vec![0.0; dim];
            v[axis] = sign;
            v
        };
        let base: [(usize, bool, Vec<Vec<f64>>); 8] = [
            (
                2,
                false,
                vec![vec![0.1, 0.1], vec![0.3, 0.3], vec![0.7, 0.7]],
            ),
            (
                3,
                false,
                vec![
                    vec![1.0, 1.0, 1.0],
                    vec![3.0, 0.0, 0.0],
                    vec![0.0, 3.0, 0.0],
                    vec![2.0, 2.0, -1.0],
                ],
            ),
            (
                4,
                false,
                vec![
                    vec![0.0; 4],
                    vec![1.0, 0.0, 0.0, 0.0],
                    vec![0.0, 1.0, 0.0, 0.0],
                    vec![0.0, 0.0, 1.0, 0.0],
                    vec![0.25, 0.25, 0.5, 0.0],
                ],
            ),
            (1, true, vec![vec![0.0], vec![1.0], vec![2.0]]),
            (
                2,
                true,
                vec![
                    vec![0.0, 0.0],
                    vec![1.0, 0.0],
                    vec![0.0, 1.0],
                    vec![1.0, 1.0],
                ],
            ),
            (
                3,
                true,
                vec![
                    vec![1.0, 0.0, 0.0],
                    vec![-1.0, 0.0, 0.0],
                    vec![0.0, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0],
                    vec![0.0, -1.0, 0.0],
                ],
            ),
            // The origin, four unit vectors, and a point in their span.
            (
                5,
                false,
                [vec![0.0; 5]]
                    .into_iter()
                    .chain((0..4).map(|a| unit(5, a, 1.0)))
                    .chain([vec![0.25, 0.25, 0.25, 0.25, 0.0]])
                    .collect(),
            ),
            // Unit vectors of both signs: every one on the unit sphere.
            (
                4,
                true,
                [unit(4, 0, 1.0), unit(4, 0, -1.0)]
                    .into_iter()
                    .chain((1..4).map(|a| unit(4, a, 1.0)))
                    .chain([unit(4, 1, -1.0)])
                    .collect(),
            ),
        ];
        for (dim, lifted, points) in base {
            if lifted && dim == 1 {
                // Three distinct points on a line are never on one
                // "sphere" of D = 1 (two points); skip the zero case.
                assert_ne!(exact_of(&points, lifted), Sign::Zero);
            } else {
                assert_eq!(exact_of(&points, lifted), Sign::Zero, "dim {dim}");
            }
            let last = points.len() - 1;
            for step in [f64::next_down, f64::next_up] {
                let mut moved = points.clone();
                let x = &mut moved[last][dim - 1];
                *x = step(*x);
                let mut tally = Tally::default();
                check(&moved, lifted, &mut tally, "one ulp");
                assert_ne!(exact_of(&moved, lifted), Sign::Zero, "dim {dim}");
            }
        }
    }

    /// The underflow of a minor's product, scaled by a huge entry of
    /// column 2, flips the computed sign: only the absolute term `4 η X`
    /// keeps it open. True determinant `H (0.4 - 0.3) η > 0`; computed
    /// `-0.3 H η`, because `x y = 0.4 η` rounds to zero.
    #[test]
    fn a_scaled_underflow_is_left_open() {
        let x = 2f64.powi(-537);
        let y = 0.4 * 2f64.powi(-537);
        let h = 2f64.powi(1000);
        let points = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 2f64.powi(-200), h],
            vec![x, 0.0, 0.0],
            vec![0.0, y, 0.3 * 2f64.powi(663)],
        ];
        assert_eq!(exact_of(&points, false), Sign::Positive);
        let computed = {
            let d = &points[1..];
            let m_bc = d[1][0] * d[2][1] - d[1][1] * d[2][0];
            let m_ab = d[0][0] * d[1][1] - d[0][1] * d[1][0];
            d[0][2] * m_bc + d[2][2] * m_ab
        };
        assert!(computed < 0.0, "the case flips the computed sign");
        assert_eq!(semi_of(&points, false), None);
        // Embedded in k = 4 with a unit fourth axis, the same flip.
        let embedded: Vec<Vec<f64>> = points
            .iter()
            .map(|p| {
                let mut p = p.clone();
                p.push(0.0);
                p
            })
            .chain([vec![0.0, 0.0, 0.0, 1.0]])
            .collect();
        assert_eq!(
            exact_of(&embedded, false),
            exact_of(&points, false),
            "the fourth axis keeps the sign"
        );
        assert_eq!(semi_of(&embedded, false), None);
    }

    /// `2^e` for `-1074 <= e <= 1023`, from its bits. `f64::powi` is not
    /// used: on x86_64 Linux it divides one by `2^-e` for a negative `e`,
    /// and every `2^-e` above `2^1023` overflows, so each power below
    /// `2^-1023` came out as 0 there (#344).
    pub(in crate::predicates) fn two_to(e: i32) -> f64 {
        assert!((-1074..=1023).contains(&e), "2^{e} is not a finite f64");
        if e >= -1022 {
            f64::from_bits(((e + 1023) as u64) << 52)
        } else {
            f64::from_bits(1_u64 << (e + 1074))
        }
    }

    #[test]
    fn two_to_is_exact_at_both_ends() {
        assert_eq!(two_to(-1074), f64::from_bits(1));
        assert_eq!(two_to(-1073), f64::from_bits(2));
        assert_eq!(two_to(-1023), f64::MIN_POSITIVE / 2.0);
        assert_eq!(two_to(-1022), f64::MIN_POSITIVE);
        assert_eq!(two_to(0), 1.0);
        assert_eq!(two_to(52), 4_503_599_627_370_496.0);
        assert_eq!(two_to(1023), f64::MAX / (2.0 - f64::EPSILON));
        for e in -1074..1023 {
            assert_eq!(two_to(e + 1), two_to(e) * 2.0, "2^{e}");
        }
    }

    /// `x 2^1074`, an integer for every finite `x`.
    pub(in crate::predicates) fn int(x: f64) -> BigInt {
        BigInt::from_f64_scaled(x, 0).unwrap()
    }

    /// `2^k`.
    pub(in crate::predicates) fn pow2(k: u64) -> BigInt {
        let mut power = int(two_to((k % 1074) as i32 - 1074));
        for _ in 0..k / 1074 {
            power = power.mul(&int(1.0)).unwrap();
        }
        power
    }

    /// The determinant of the square matrix `rows` over `columns`, by
    /// cofactor expansion along the first row.
    pub(in crate::predicates) fn cofactor_expansion(
        rows: &[Vec<BigInt>],
        columns: &[usize],
    ) -> BigInt {
        let Some((first, below)) = rows.split_first() else {
            return pow2(0);
        };
        let mut sum = int(0.0);
        for (i, &column) in columns.iter().enumerate() {
            let rest: Vec<usize> = columns.iter().copied().filter(|&c| c != column).collect();
            let term = first[column]
                .mul(&cofactor_expansion(below, &rest))
                .unwrap();
            sum = if i % 2 == 0 {
                sum.add(&term)
            } else {
                sum.sub(&term)
            }
            .unwrap();
        }
        sum
    }

    /// The error `fl(E) - E` of the determinant the stage computes and its
    /// bound, both times one positive power of two. `E` is the determinant
    /// of the exact differences, with the exact `|d_i|^2` as the lifted
    /// column, expanded over integers here.
    fn error_and_bound(points: &[Vec<f64>], lifted: bool) -> (BigInt, BigInt) {
        let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
        let Estimate { det, bound } = estimate(refs[0], &refs[1..], lifted).unwrap();
        assert!(det.is_finite() && bound.is_finite(), "{points:?}");
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
        // `exact` is E 2^(shift degree) and `int(v)` is v 2^1074, so both
        // sides are brought to 2^(shift degree + 1074).
        let degree = (points[0].len() + 2 * usize::from(lifted)) as u64;
        let lift = pow2(shift * degree);
        let error = int(det)
            .mul(&lift)
            .unwrap()
            .sub(&exact.mul(&pow2(1074)).unwrap())
            .unwrap();
        (error, int(bound).mul(&lift).unwrap())
    }

    /// Whether `bound >= factor |error|`.
    pub(in crate::predicates) fn covers(bound: &BigInt, error: &BigInt, factor: u32) -> bool {
        let scaled = (0..factor).fold(int(0.0), |sum, _| sum.add(error).unwrap());
        bound.sub(&scaled).unwrap().sign() != Sign::Negative
            && bound.add(&scaled).unwrap().sign() != Sign::Negative
    }

    /// Every coordinate near `2^(-1074 / degree)`, where the products of a
    /// monomial underflow.
    fn underflowing(rng: &mut Rng, dim: usize, lifted: bool, trial: usize) -> Vec<Vec<f64>> {
        let degree = (dim + 2 * usize::from(lifted)) as i32;
        let scale = 2f64.powi(-1074 / degree - 60 + (trial % 80) as i32);
        (0..count(dim, lifted))
            .map(|_| (0..dim).map(|_| rng.unit() * scale).collect())
            .collect()
    }

    /// The bound of the common path is never below `fl(s + fl(4 η x))`,
    /// the bound of the proof, on the edges of its condition and on random
    /// values. Without the margin, or without the condition, it is.
    #[test]
    fn the_plain_bound_covers_the_underflow_term() {
        let proof = |s: f64, x: f64| s + FOUR_ETA * x;
        let up = f64::next_up;
        let down = f64::next_down;
        let s_edges = [
            PLAIN_FROM,
            up(PLAIN_FROM),
            down(2.0 * PLAIN_FROM),
            2.0 * PLAIN_FROM,
            1.0,
            down(2.0),
            2f64.powi(1000),
            f64::MAX,
        ];
        let x_edges = [1.0, 2.0, down(PLAIN_UP_TO), PLAIN_UP_TO, 1e10, 3.0e17];
        let mut rng = Rng(316);
        let mut checked = 0;
        let mut strictly_above = 0;
        for &s in &s_edges {
            for &x in &x_edges {
                let b = bound(1.0, s, x);
                assert!(b >= proof(s, x), "s {s:e}, x {x:e}");
                checked += 1;
                strictly_above += usize::from(b > proof(s, x));
            }
        }
        for _ in 0..100_000 {
            let s = rng.unit().abs() * 2f64.powi(rng.below(2000) as i32 - 960);
            let x = 1.0 + rng.unit().abs() * 2f64.powi(rng.below(61) as i32);
            if s >= PLAIN_FROM && x <= PLAIN_UP_TO {
                assert!(bound(1.0, s, x) >= proof(s, x), "s {s:e}, x {x:e}");
                checked += 1;
            }
        }
        assert!(
            checked > 10_000 && strictly_above > 0,
            "{checked} {strictly_above}"
        );
        // Outside the condition the bound is the proof's, bit for bit.
        for (s, x) in [
            (down(PLAIN_FROM), 1.0),
            (1e-300, up(PLAIN_UP_TO)),
            (0.0, 1.0),
            (f64::NAN, 1.0),
            (1.0, f64::NAN),
        ] {
            let (b, p) = (bound(1.0, s, x), proof(s, x));
            assert!(b.to_bits() == p.to_bits(), "s {s:e}, x {x:e}");
        }
        // The two mutations: no margin, and no condition. Each gives a
        // bound below the proof's on some input above.
        assert!(PLAIN_FROM * 1.0 < proof(PLAIN_FROM, PLAIN_UP_TO));
        let tiny = two_to(-1060);
        assert!(tiny * MARGIN < proof(tiny, PLAIN_UP_TO));
    }

    /// The bound is derived as a worst case, which no input here reaches,
    /// so a smaller constant can pass every test of a sign. This test pins
    /// the distance instead. Over each family, the bound stays a fixed
    /// factor above the exact error of the computed determinant, and some
    /// input comes within twice that factor, so a constant cut in half
    /// fails the first assertion.
    #[test]
    fn the_bound_keeps_its_measured_margin() {
        /// Per formula, in the order of `FORMULAS`: the bound is at least
        /// this many times the error over the cancelling inputs, and over
        /// the underflowing ones. Size five bounds the permanent and the
        /// underflow from column sums (module docs), so its margins over the
        /// cancelling inputs are wider.
        const MARGIN: [[u32; 2]; 8] = [
            [3, 3],
            [3, 3],
            [5, 4],
            [3, 3],
            [4, 3],
            [4, 3],
            [500, 4],
            [500, 3],
        ];
        assert_eq!(MARGIN.len(), FORMULAS.len(), "a margin per formula");
        let mut rng = Rng(21);
        for ((dim, lifted), margin) in FORMULAS.into_iter().zip(MARGIN) {
            let mut near = [0_usize; 2];
            for trial in 0..3000 {
                let family = trial % 2;
                let points = if family == 0 {
                    let offset = [0.0, 1.0, 1e3, 1e6][trial / 2 % 4];
                    near_degenerate(&mut rng, dim, lifted, offset)
                } else {
                    underflowing(&mut rng, dim, lifted, trial / 2)
                };
                let (error, bound) = error_and_bound(&points, lifted);
                assert!(
                    covers(&bound, &error, margin[family]),
                    "dim {dim}, lifted {lifted}, family {family}: {points:?}"
                );
                near[family] += usize::from(!covers(&bound, &error, 2 * margin[family]));
            }
            assert!(
                near.iter().all(|&n| n > 0),
                "dim {dim}, lifted {lifted}: {near:?}"
            );
        }
    }
}
