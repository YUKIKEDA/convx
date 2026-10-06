//! Input acceptance (design §3): validation, duplicate aggregation, the count
//! check, and the rank check that yields the lexicographically minimum basis.

use core::cmp::Ordering;

use super::ConvexHullError;
use crate::predicates::{orient, ExactEvaluationExhausted, Sign};

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
    /// Dimension of the input sites.
    pub(crate) fn dim(&self) -> usize {
        self.dim
    }

    /// Orientation of the points `indices` (D + 1 of them).
    pub(crate) fn orient(&self, indices: &[u32]) -> Result<Sign, ExactEvaluationExhausted> {
        // Up to INLINE points are gathered on the stack; a call does not
        // allocate for them.
        const INLINE: usize = 17;
        let n = indices.len();
        if n <= INLINE {
            let mut points: [&[f64]; INLINE] = [&[]; INLINE];
            for (slot, &i) in points.iter_mut().zip(indices) {
                *slot = self.point(i);
            }
            orient(&points[..n])
        } else {
            let points: Vec<&[f64]> = indices.iter().map(|&i| self.point(i)).collect();
            orient(&points)
        }
    }

    /// Every input point, row-major, with the row stride D: point `i`
    /// starts at `rows[i * stride]`.
    pub(crate) fn rows(&self) -> (&[f64], usize) {
        (self.points, self.dim)
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
///
/// Points are inserted in index order into an open-addressing table keyed
/// by their coordinate bits, so the first point of each class to arrive,
/// the smallest index, is its representative (#213). A probe sequence
/// longer than [`MAX_PROBES`] abandons the table for
/// [`representatives_by_sort`], which gives the same result, so an input
/// built to collide costs O(n log n), not O(n^2).
fn representatives_of<'p>(count: usize, point: impl Fn(u32) -> &'p [f64]) -> Vec<u32> {
    representatives_by_hash(count, &point).unwrap_or_else(|| representatives_by_sort(count, point))
}

/// Longest probe sequence of [`representatives_by_hash`] before it gives up.
const MAX_PROBES: usize = 64;

/// [`representatives_of`] by hashing, or `None` when a probe sequence
/// exceeds [`MAX_PROBES`].
fn representatives_by_hash<'p>(
    count: usize,
    point: &impl Fn(u32) -> &'p [f64],
) -> Option<Vec<u32>> {
    // At most half full.
    let slots = (2 * count).next_power_of_two().max(2);
    let mask = slots - 1;
    let mut table = vec![u32::MAX; slots];
    let mut representative = vec![0_u32; count];
    for i in 0..count as u32 {
        let p = point(i);
        let mut slot = (point_hash(p) as usize) & mask;
        let mut probes = 0;
        representative[i as usize] = loop {
            let entry = table[slot];
            if entry == u32::MAX {
                table[slot] = i;
                break i;
            }
            if compare_points(point(entry), p) == Ordering::Equal {
                break entry;
            }
            probes += 1;
            if probes > MAX_PROBES {
                return None;
            }
            slot = (slot + 1) & mask;
        };
    }
    Some(representative)
}

/// A hash of the coordinates under which points that are `==` agree:
/// adding `+0.0` turns `-0.0` into `+0.0` and leaves every other finite
/// value unchanged, so equal coordinates have equal bits.
fn point_hash(p: &[f64]) -> u64 {
    let mut h: u64 = 0x9e37_79b9_7f4a_7c15;
    for &x in p {
        h = (h ^ (x + 0.0).to_bits()).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h ^= h >> 31;
    }
    // Final mix, so the low bits that pick the slot depend on every bit.
    h = (h ^ (h >> 30)).wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

/// [`representatives_of`] by sorting the indices by coordinates.
fn representatives_by_sort<'p>(count: usize, point: impl Fn(u32) -> &'p [f64]) -> Vec<u32> {
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

    /// Points of `dim` coordinates drawn from a small set of values, so
    /// many are duplicates, with signed zeros among them.
    fn duplicate_heavy(dim: usize, count: usize, seed: u64) -> Vec<f64> {
        let values = [-0.0, 0.0, 1.0, -1.0, 0.5, 3.0];
        let mut state = seed;
        (0..dim * count)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                values[(state % values.len() as u64) as usize]
            })
            .collect()
    }

    #[test]
    fn hashing_and_sorting_give_the_same_representatives() {
        for dim in 1..=4 {
            for count in [1, 2, 7, 100, 5000] {
                let points = duplicate_heavy(dim, count, 0x1234 + (dim * count) as u64);
                let point = |i: u32| &points[i as usize * dim..(i as usize + 1) * dim];
                let by_hash = representatives_by_hash(count, &point)
                    .expect("ordinary input stays within the probe bound");
                let by_sort = representatives_by_sort(count, point);
                assert_eq!(by_hash, by_sort, "dim {dim}, count {count}");
                // Each representative is the smallest index equal to it.
                for (i, &r) in by_hash.iter().enumerate() {
                    assert!(r as usize <= i);
                    assert_eq!(compare_points(point(r), point(i as u32)), Ordering::Equal);
                    assert_eq!(by_hash[r as usize], r);
                }
            }
        }
    }

    #[test]
    fn colliding_hashes_fall_back_to_the_sort() {
        // Distinct points whose hashes all pick the same slot: the probe
        // sequence passes the bound, the table gives up, and the sort gives
        // the same answer as for any other input.
        // `count` distinct points and one duplicate: the table has the slot
        // count of `count + 1` points.
        let count = 4 * MAX_PROBES - 1;
        let slots = (2 * (count + 1)).next_power_of_two();
        let mut points = Vec::new();
        let mut candidate = 0.0_f64;
        while points.len() < count {
            candidate += 1.0;
            if point_hash(&[candidate]) as usize & (slots - 1) == 0 {
                points.push(candidate);
            }
        }
        // A duplicate of an early point, so a representative is not the
        // point itself.
        points.push(points[3]);
        let n = points.len();
        let point = |i: u32| &points[i as usize..i as usize + 1];
        assert_eq!(
            (2 * n).next_power_of_two(),
            slots,
            "the table has the slot count the points were chosen for"
        );
        assert!(representatives_by_hash(n, &point).is_none());
        let representative = representatives_of(n, point);
        assert_eq!(representative, representatives_by_sort(n, point));
        assert_eq!(representative[n - 1], 3);
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
