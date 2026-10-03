//! Input acceptance (design §3): validation, duplicate aggregation, the count
//! check, and the rank check that yields the lexicographically minimum basis.

use core::cmp::Ordering;

use super::ConvexHullError;
use crate::predicates::{orient, orient_lifted_with, ExactEvaluationExhausted, LiftedHeight, Sign};

/// Validated input. Past this point, code trusts that every coordinate is
/// finite and that there are at most `u32::MAX` points.
pub(crate) struct Input<'a> {
    dim: usize,
    points: &'a [f64],
    /// For each input index, the smallest index of a point equal to it.
    pub(crate) representative: Vec<u32>,
    /// The representatives, ascending.
    pub(crate) representatives: Vec<u32>,
    /// The lexicographically minimum affine basis: D + 1 representatives.
    /// For a lifted input, D + 2: that basis and the first representative
    /// off the lifted hyperplane through it.
    pub(crate) spanning_points: Vec<u32>,
    /// The sites lifted to the paraboloid (design §7), when this input stands
    /// for them.
    lifted: Option<Lifted>,
}

/// The lift of every input point, by index: one row `(p, |p|^2, bound)` of
/// length D + 2 per point, so a predicate reads a site's coordinates and its
/// cached height from one place.
struct Lifted {
    dim: usize,
    rows: Vec<f64>,
}

impl Lifted {
    fn of(dim: usize, points: &[f64]) -> Self {
        let mut rows = Vec::with_capacity(points.len() / dim * (dim + 2));
        for p in points.chunks_exact(dim) {
            let height = LiftedHeight::of(p);
            rows.extend_from_slice(p);
            rows.push(height.value());
            rows.push(height.error());
        }
        Self { dim, rows }
    }

    fn row(&self, index: u32) -> &[f64] {
        let stride = self.dim + 2;
        let start = index as usize * stride;
        &self.rows[start..start + stride]
    }

    /// The site's input coordinates, copied bit for bit.
    fn site(&self, index: u32) -> &[f64] {
        &self.row(index)[..self.dim]
    }

    /// Rounded lifted coordinates `(p, |p|^2)`. They feed only working
    /// normals and distances; every sign comes from [`Input::orient`].
    fn coords(&self, index: u32) -> &[f64] {
        &self.row(index)[..self.dim + 1]
    }

    /// The filtered height the lifted orientation reads in place of the
    /// squares (#27).
    fn height(&self, index: u32) -> LiftedHeight {
        let row = self.row(index);
        LiftedHeight::stored(row[self.dim], row[self.dim + 1])
    }
}

impl<'a> Input<'a> {
    /// Dimension of the input sites.
    pub(crate) fn dim(&self) -> usize {
        self.dim
    }

    /// Dimension the hull core works in: D, or D + 1 for lifted sites.
    pub(crate) fn engine_dim(&self) -> usize {
        self.dim + usize::from(self.lifted.is_some())
    }

    /// Whether the sites stand lifted to the paraboloid.
    pub(crate) fn is_lifted(&self) -> bool {
        self.lifted.is_some()
    }

    /// Orientation of the points `indices` (engine dimension + 1 of them) in
    /// the engine space: the plain orientation, or the lifted one with the
    /// lifted coordinate as the polynomial `|p|^2`.
    pub(crate) fn orient(&self, indices: &[u32]) -> Result<Sign, ExactEvaluationExhausted> {
        // Up to INLINE points are gathered on the stack; a call does not
        // allocate for them.
        const INLINE: usize = 17;
        let n = indices.len();
        match &self.lifted {
            None if n <= INLINE => {
                let mut points: [&[f64]; INLINE] = [&[]; INLINE];
                for (slot, &i) in points.iter_mut().zip(indices) {
                    *slot = self.point(i);
                }
                orient(&points[..n])
            }
            None => {
                let points: Vec<&[f64]> = indices.iter().map(|&i| self.point(i)).collect();
                orient(&points)
            }
            Some(lifted) if n <= INLINE => {
                let mut points: [&[f64]; INLINE] = [&[]; INLINE];
                let mut heights = [LiftedHeight::of(&[]); INLINE];
                for ((slot, height), &i) in points.iter_mut().zip(&mut heights).zip(indices) {
                    *slot = lifted.site(i);
                    *height = lifted.height(i);
                }
                orient_lifted_with(&points[..n], &heights[..n])
            }
            Some(lifted) => {
                let points: Vec<&[f64]> = indices.iter().map(|&i| lifted.site(i)).collect();
                let heights: Vec<LiftedHeight> =
                    indices.iter().map(|&i| lifted.height(i)).collect();
                orient_lifted_with(&points, &heights)
            }
        }
    }

    /// Engine-space coordinates of point `index` for working normals and
    /// distances only. Lifted coordinates are rounded and may be infinite.
    pub(crate) fn coords(&self, index: u32) -> &[f64] {
        match &self.lifted {
            Some(lifted) => lifted.coords(index),
            None => self.point(index),
        }
    }

    /// The engine coordinates of every point, row-major, with the row stride:
    /// the input points, or the lifted rows `(p, |p|^2, bound)`. The engine
    /// coordinates of point `i` start at `rows[i * stride]`.
    pub(crate) fn engine_rows(&self) -> (&[f64], usize) {
        match &self.lifted {
            Some(lifted) => (&lifted.rows, lifted.dim + 2),
            None => (self.points, self.dim),
        }
    }

    /// A bound on the rounding of the last engine coordinate of point
    /// `index`: the height bound of a lifted site, and 0 otherwise.
    pub(crate) fn height_bound(&self, index: u32) -> f64 {
        match &self.lifted {
            Some(lifted) => lifted.height(index).error(),
            None => 0.0,
        }
    }

    /// The same sites lifted to the paraboloid, or `Err(self)` unchanged when
    /// the lift is flat: every site on one sphere, so the lifted points span
    /// only dimension D (design §7).
    pub(crate) fn lift(self) -> Result<Result<Self, Self>, ConvexHullError> {
        debug_assert!(self.lifted.is_none(), "already lifted");
        let lifted = Lifted::of(self.dim, self.points);
        let mut spanning = self.spanning_points.clone();
        let mut apex = None;
        for &p in &self.representatives {
            if spanning.contains(&p) {
                continue;
            }
            let mut indices = spanning.clone();
            indices.push(p);
            let points: Vec<&[f64]> = indices.iter().map(|&i| self.point(i)).collect();
            let heights: Vec<LiftedHeight> = indices.iter().map(|&i| lifted.height(i)).collect();
            if orient_lifted_with(&points, &heights)? != Sign::Zero {
                apex = Some(p);
                break;
            }
        }
        let Some(apex) = apex else {
            return Ok(Err(self));
        };
        spanning.push(apex);
        Ok(Ok(Self {
            spanning_points: spanning,
            lifted: Some(lifted),
            ..self
        }))
    }

    pub(crate) fn points(&self) -> &'a [f64] {
        self.points
    }

    /// Coordinates of point `index`.
    pub(crate) fn point(&self, index: u32) -> &'a [f64] {
        let start = index as usize * self.dim;
        &self.points[start..start + self.dim]
    }
}

/// Checks the dimension and the length before any coordinate is read.
/// Returns the number of points.
pub(crate) fn check_shape(dim: usize, len: usize) -> Result<usize, ConvexHullError> {
    if dim == 0 {
        return Err(ConvexHullError::NonPositiveDimension);
    }
    if !len.is_multiple_of(dim) {
        return Err(ConvexHullError::LengthMismatch { len, dim });
    }
    Ok(len / dim)
}

/// Point numbers are `u32` below `u32::MAX`, which stays free as the missing
/// index, so at most `u32::MAX` points are accepted.
pub(crate) fn check_count(count: usize) -> Result<usize, ConvexHullError> {
    if count > u32::MAX as usize {
        return Err(ConvexHullError::TooManyPoints { actual: count });
    }
    Ok(count)
}

/// Runs every input check of design §3 and returns the accepted input.
pub(crate) fn accept(dim: usize, points: &[f64]) -> Result<Input<'_>, ConvexHullError> {
    let count = check_shape(dim, points.len())?;
    // §3: non-finite coordinates are rejected first, then the count, before
    // duplicate removal.
    if let Some(index) = points
        .chunks_exact(dim)
        .position(|p| p.iter().any(|x| !x.is_finite()))
    {
        return Err(ConvexHullError::NonFiniteCoordinate { index });
    }
    let count = check_count(count)?;
    let point = |i: u32| &points[i as usize * dim..(i as usize + 1) * dim];

    let representative = representatives_of(count, point);
    let representatives: Vec<u32> = (0..count as u32)
        .filter(|&i| representative[i as usize] == i)
        .collect();
    if representatives.len() < dim + 1 {
        return Err(ConvexHullError::InsufficientPoints {
            actual: representatives.len(),
            required: dim + 1,
        });
    }
    let spanning_points = minimum_basis(dim, &representatives, point)?;
    if spanning_points.len() < dim + 1 {
        return Err(ConvexHullError::DegenerateDimension {
            actual_dim: spanning_points.len() - 1,
            spanning_points,
        });
    }
    Ok(Input {
        dim,
        points,
        representative,
        representatives,
        spanning_points,
        lifted: None,
    })
}

/// Lexicographic order of coordinates by `partial_cmp`, under which `-0.0`
/// and `+0.0` compare equal, as the point identity of design §3 requires.
fn compare_points(a: &[f64], b: &[f64]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.partial_cmp(y).unwrap_or(Ordering::Equal))
        .find(|o| o.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// For each index, the smallest index of a point that is `==` to it in every
/// coordinate.
fn representatives_of<'p>(count: usize, point: impl Fn(u32) -> &'p [f64]) -> Vec<u32> {
    let mut order: Vec<u32> = (0..count as u32).collect();
    order.sort_unstable_by(|&a, &b| compare_points(point(a), point(b)).then(a.cmp(&b)));
    let mut representative = vec![0_u32; count];
    let mut current = None;
    for &i in &order {
        let rep = match current {
            Some(r) if compare_points(point(r), point(i)) == Ordering::Equal => r,
            _ => i,
        };
        current = Some(rep);
        representative[i as usize] = rep;
    }
    representative
}

/// Walks `representatives` in ascending order and keeps each point that
/// strictly raises the affine dimension. The basis is grown one point at a
/// time; whether a point lies in the affine span is decided by exact signs.
///
/// The basis `B` of affine dimension `r` is kept together with `r`
/// coordinates `S` on which its projection is still affinely independent.
/// Then `aff(B)` is the graph of an affine map from the `S` coordinates to
/// the others, and a point `p` lies in it exactly when, for every other
/// coordinate `j`, the orientation of `B` and `p` projected onto `S ∪ {j}` is
/// zero. The first `j` with a nonzero sign joins `S`, and `p` joins `B`.
pub(crate) fn minimum_basis<'p>(
    dim: usize,
    representatives: &[u32],
    point: impl Fn(u32) -> &'p [f64],
) -> Result<Vec<u32>, ConvexHullError> {
    let Some((&first, rest)) = representatives.split_first() else {
        return Ok(Vec::new());
    };
    let mut basis = vec![first];
    let mut axes: Vec<usize> = Vec::with_capacity(dim);
    let mut projected: Vec<Vec<f64>> = Vec::with_capacity(dim + 2);
    for &p in rest {
        if basis.len() == dim + 1 {
            break;
        }
        for j in (0..dim).filter(|j| !axes.contains(j)) {
            projected.clear();
            projected.extend(basis.iter().chain(core::iter::once(&p)).map(|&q| {
                let q = point(q);
                axes.iter()
                    .map(|&a| q[a])
                    .chain(core::iter::once(q[j]))
                    .collect()
            }));
            let refs: Vec<&[f64]> = projected.iter().map(Vec::as_slice).collect();
            if orient(&refs)? != Sign::Zero {
                axes.push(j);
                basis.push(p);
                break;
            }
        }
    }
    Ok(basis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_errors_in_order() {
        assert_eq!(
            check_shape(0, 0).err(),
            Some(ConvexHullError::NonPositiveDimension)
        );
        assert_eq!(
            check_shape(0, 7).err(),
            Some(ConvexHullError::NonPositiveDimension)
        );
        assert_eq!(
            check_shape(3, 7).err(),
            Some(ConvexHullError::LengthMismatch { len: 7, dim: 3 })
        );
        assert_eq!(check_shape(3, 9), Ok(3));
    }

    #[test]
    fn too_many_points_at_the_count_boundary() {
        assert_eq!(check_count(u32::MAX as usize), Ok(u32::MAX as usize));
        assert_eq!(
            check_count(u32::MAX as usize + 1).err(),
            Some(ConvexHullError::TooManyPoints {
                actual: u32::MAX as usize + 1
            })
        );
    }

    #[test]
    fn first_non_finite_point_is_reported() {
        let points = [0.0, 0.0, 1.0, f64::NAN, f64::INFINITY, 2.0, 3.0, 3.0];
        assert_eq!(
            accept(2, &points).err(),
            Some(ConvexHullError::NonFiniteCoordinate { index: 1 })
        );
    }

    #[test]
    fn signed_zeros_are_one_point_with_the_smallest_index() {
        let points = [1.0, 0.0, -0.0, 0.0, 0.0, 1.0, 0.0, -0.0, 1.0, 0.0];
        let input = accept(2, &points).unwrap();
        assert_eq!(input.representative, vec![0, 1, 2, 1, 0]);
        assert_eq!(input.representatives, vec![0, 1, 2]);
    }

    #[test]
    fn too_few_distinct_points() {
        let points = [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0];
        assert_eq!(
            accept(2, &points).err(),
            Some(ConvexHullError::InsufficientPoints {
                actual: 2,
                required: 3
            })
        );
    }

    #[test]
    fn collinear_points_report_the_minimum_basis() {
        // Points 0, 1 coincide; 2 lies on the line through 0 and 3.
        let points = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 5.0, 5.0];
        assert_eq!(
            accept(2, &points).err(),
            Some(ConvexHullError::DegenerateDimension {
                actual_dim: 1,
                spanning_points: vec![0, 2]
            })
        );
    }

    #[test]
    fn coplanar_points_in_three_dimensions() {
        // All on z = x + y.
        let points = [0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 2.0, 3.0, 5.0];
        assert_eq!(
            accept(3, &points).err(),
            Some(ConvexHullError::DegenerateDimension {
                actual_dim: 2,
                spanning_points: vec![0, 1, 2]
            })
        );
    }

    #[test]
    fn basis_skips_points_in_the_current_span() {
        // 0, 1 on the x axis; 2 also on it; 3 raises to 2D; 4 raises to 3D.
        let points = [
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 7.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0,
        ];
        let input = accept(3, &points).unwrap();
        assert_eq!(input.spanning_points, vec![0, 1, 3, 4]);
    }

    /// Exact lifted orientation of four integer sites of dimension 2: the
    /// sign of the 3 x 3 determinant of `(p - o, |p|^2 - |o|^2)` in `i128`.
    fn lifted_sign_2d(points: &[[i64; 2]]) -> Sign {
        let norm = |p: [i64; 2]| i128::from(p[0]).pow(2) + i128::from(p[1]).pow(2);
        let o = points[0];
        let m: Vec<[i128; 3]> = points[1..]
            .iter()
            .map(|&p| {
                [
                    i128::from(p[0] - o[0]),
                    i128::from(p[1] - o[1]),
                    norm(p) - norm(o),
                ]
            })
            .collect();
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        match det.cmp(&0) {
            Ordering::Less => Sign::Negative,
            Ordering::Equal => Sign::Zero,
            Ordering::Greater => Sign::Positive,
        }
    }

    #[test]
    fn cached_heights_give_the_exact_lifted_sign() {
        // Eight sites on the circle of radius 5, two just off it, and the
        // centre: many quadruples are exactly cocircular, and the others are
        // one unit from it. Scaling by 2^e multiplies the lifted determinant
        // by a positive power of two, and a translation keeps it, so the
        // integer sign is the expected sign of every case. At 2^-540 the
        // squares underflow and at 2^520 they overflow, so the cached heights
        // cannot certify and the exact stage decides; at 2^40 with the shift
        // the filter fails on cancellation.
        let sites: [[i64; 2]; 11] = [
            [5, 0],
            [0, 5],
            [-5, 0],
            [0, -5],
            [3, 4],
            [4, 3],
            [-3, 4],
            [4, -3],
            [3, 5],
            [5, 1],
            [0, 0],
        ];
        let cases: [(i32, f64); 6] = [
            (0, 0.0),
            (-540, 0.0),
            (520, 0.0),
            (40, 0.0),
            (40, 1.0e15),
            (0, 1.0e15),
        ];
        let mut decided = [0usize; 3];
        for (e, shift) in cases {
            let scale = 2f64.powi(e);
            let points: Vec<f64> = sites
                .iter()
                .flat_map(|p| p.map(|x| x as f64 * scale + shift))
                .collect();
            let Ok(input) = accept(2, &points).unwrap().lift().unwrap() else {
                panic!("the sites are not cocircular, so the lift is not flat (2^{e}, {shift})");
            };
            let n = sites.len() as u32;
            for a in 0..n {
                for b in a + 1..n {
                    for c in b + 1..n {
                        for d in c + 1..n {
                            let indices = [a, b, c, d];
                            let expected = lifted_sign_2d(&indices.map(|i| sites[i as usize]));
                            assert_eq!(
                                input.orient(&indices).unwrap(),
                                expected,
                                "2^{e}, shift {shift}, {indices:?}"
                            );
                            decided[expected as usize] += 1;
                        }
                    }
                }
            }
        }
        assert!(
            decided.iter().all(|&c| c > 0),
            "every sign occurs: {decided:?}"
        );
    }
}
