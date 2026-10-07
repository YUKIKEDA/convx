//! The first stage of the orientations and lifted orientations with k ≤ 4
//! (#249): the determinant in `f64`, certified by a constant times the
//! permanent of the same expression, both evaluated in the same call.
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

use super::Sign;

const U: f64 = f64::EPSILON / 2.0;

/// `4 η`, four times the smallest subnormal.
const FOUR_ETA: f64 = f64::from_bits(4);

/// The relative constant `(n + 1) u` of a formula whose monomials carry at
/// most `n` roundings.
const fn relative(n: u32) -> f64 {
    (n + 1) as f64 * U
}

/// The sign of `det` when `|det|` exceeds `relative * permanent + 4 η x`.
#[inline(always)]
fn certify(det: f64, relative: f64, permanent: f64, x: f64) -> Option<Sign> {
    let bound = relative * permanent + FOUR_ETA * x;
    if det > bound {
        Some(Sign::Positive)
    } else if -det > bound {
        Some(Sign::Negative)
    } else {
        None
    }
}

/// The sign of the orientation of `origin` followed by `points`, or of the
/// lifted orientation when `lifted`, for k ≤ 4. `None` when the bound does
/// not certify it, or when no formula here covers the size.
#[inline]
pub(super) fn sign(origin: &[f64], points: &[&[f64]], lifted: bool) -> Option<Sign> {
    let dim = origin.len();
    let diff = |i: usize, j: usize| points[i][j] - origin[j];
    match (dim, lifted) {
        (2, false) => orient2([diff(0, 0), diff(0, 1)], [diff(1, 0), diff(1, 1)]),
        (3, false) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1), diff(i, 2)];
            orient3(row(0), row(1), row(2))
        }
        (4, false) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1), diff(i, 2), diff(i, 3)];
            orient4(row(0), row(1), row(2), row(3))
        }
        (1, true) => lifted1(diff(0, 0), diff(1, 0)),
        (2, true) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1)];
            lifted2(row(0), row(1), row(2))
        }
        (3, true) => {
            let row = |i: usize| [diff(i, 0), diff(i, 1), diff(i, 2)];
            lifted3(row(0), row(1), row(2), row(3))
        }
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
fn orient2(a: [f64; 2], b: [f64; 2]) -> Option<Sign> {
    let (det, permanent) = minor2(a, b);
    certify(det, relative(4), permanent, 1.0)
}

/// Orientation, k = 3: [`minor3`], `n = 8`. Each of the six products of the
/// minors underflows by at most `η / 2`, scaled by the column-2 entry it is
/// multiplied with; each of the three outer products by `η / 2`. So
/// `X = |a2| + |b2| + |c2| + 2`.
fn orient3(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<Sign> {
    let (a2, b2, c2) = (first2(a), first2(b), first2(c));
    let (det, permanent) = minor3((a, b, c), (minor2(b2, c2), minor2(a2, c2), minor2(a2, b2)));
    let x = (a[2].abs() + b[2].abs()) + c[2].abs() + 2.0;
    certify(det, relative(8), permanent, x)
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
fn orient4(a: [f64; 4], b: [f64; 4], c: [f64; 4], d: [f64; 4]) -> Option<Sign> {
    let rows = [a, b, c, d];
    let (det, permanent, _) = expand4(rows.map(first3), rows.map(|r| r[3]));
    let z = (a[2].abs() + b[2].abs()) + (c[2].abs() + d[2].abs());
    let s = (a[3].abs() + b[3].abs()) + (c[3].abs() + d[3].abs());
    certify(det, relative(12), permanent, s * (z + 2.0) + 2.0)
}

/// Lifted orientation, D = 1: rows `(a, a^2)` and `(b, b^2)`,
/// `det = a b^2 - a^2 b`. A square carries 3 roundings, a product 5, the
/// difference 6. Each square underflows by at most `η / 2`, scaled by the
/// other coordinate; each product by `η / 2`. So `X = |a| + |b| + 1`.
fn lifted1(a: f64, b: f64) -> Option<Sign> {
    let (la, lb) = (a * a, b * b);
    let (p, q) = (a * lb, la * b);
    let x = a.abs() + b.abs() + 1.0;
    certify(p - q, relative(6), p.abs() + q.abs(), x)
}

/// Lifted orientation, D = 2: rows `(x, y, x^2 + y^2)`, expanded along the
/// lifted column over the minors of the coordinates (the in-circle test).
/// A lifted entry carries 4 roundings, a minor 4, a term 9, and two sums
/// make `n = 11`. A lifted entry underflows by at most `η`, scaled by its
/// minor; a minor by `η`, scaled by its lifted entry; each outer product
/// by `η / 2`. So `X` is the sum of the minors' permanents, of the lifted
/// entries, and 2.
fn lifted2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<Sign> {
    let lift = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let (la, lb, lc) = (lift(a), lift(b), lift(c));
    let (bc, ac, ab) = (minor2(b, c), minor2(a, c), minor2(a, b));
    let det = (la * bc.0 - lb * ac.0) + lc * ab.0;
    let permanent = (la * bc.1 + lb * ac.1) + lc * ab.1;
    let x = ((bc.1 + ac.1) + ab.1) + ((la + lb) + lc) + 2.0;
    certify(det, relative(11), permanent, x)
}

/// Lifted orientation, D = 3: [`expand4`] with column 3 the lifted entries
/// `(x^2 + y^2) + z^2` (the in-sphere test). A lifted entry carries 5
/// roundings, a term 14, and two sums make `n = 16`. A lifted entry
/// underflows by at most `3 η / 2`, scaled by its minor's permanent; a minor
/// as in [`orient4`], scaled by its lifted entry; each outer product by
/// `η / 2`. So `X = 2 (sum of the minors' permanents) + (sum of the lifted
/// entries) (Z + 2) + 2`.
fn lifted3(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> Option<Sign> {
    let rows = [a, b, c, d];
    let lifts = rows.map(|p| (p[0] * p[0] + p[1] * p[1]) + p[2] * p[2]);
    let (det, permanent, minors) = expand4(rows, lifts);
    let z = (a[2].abs() + b[2].abs()) + (c[2].abs() + d[2].abs());
    let l = (lifts[0] + lifts[1]) + (lifts[2] + lifts[3]);
    let m = (minors[0] + minors[1]) + (minors[2] + minors[3]);
    certify(det, relative(16), permanent, 2.0 * m + l * (z + 2.0) + 2.0)
}

#[cfg(test)]
mod tests {
    use super::super::{exact, LiftedHeight, Rows};
    use super::*;

    /// The exact sign of the orientation (or lifted orientation) of
    /// `points`, from the exact stage.
    fn exact_of(points: &[Vec<f64>], lifted: bool) -> Sign {
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

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }

        fn unit(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// Every formula: (dimension of a point, lifted).
    const FORMULAS: [(usize, bool); 6] = [
        (2, false),
        (3, false),
        (4, false),
        (1, true),
        (2, true),
        (3, true),
    ];

    /// The number of points of a formula: k + 1, with k = dim (+ 1 when
    /// lifted).
    fn count(dim: usize, lifted: bool) -> usize {
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
    fn near_degenerate(rng: &mut Rng, dim: usize, lifted: bool, offset: f64) -> Vec<Vec<f64>> {
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
                let scale = 2f64.powi(e);
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
                                    rng.unit() * 2f64.powi(e)
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
        let base: [(usize, bool, Vec<Vec<f64>>); 6] = [
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
}
