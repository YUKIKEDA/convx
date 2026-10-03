//! Input acceptance (design §3): validation, duplicate aggregation, the count
//! check, and the rank check that yields the lexicographically minimum basis.

use core::cmp::Ordering;

use super::ConvexHullError;
use crate::predicates::{orient, Sign};

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
    pub(crate) spanning_points: Vec<u32>,
}

impl<'a> Input<'a> {
    pub(crate) fn dim(&self) -> usize {
        self.dim
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
}
