//! Input acceptance (design §3): validation, duplicate aggregation, the count
//! check, and the rank check that yields the lexicographically minimum basis.

use core::cmp::Ordering;

use super::polygon::{self, Chain, Rows};
use super::ConvexHullError;
use crate::predicates::{orient, ExactEvaluationExhausted, Sign};
use crate::small::Small;

/// Validated input. Past this point, code trusts that every coordinate is
/// finite and that there are at most `u32::MAX` points.
pub(crate) struct Input<'a> {
    dim: usize,
    points: &'a [f64],
    /// For each input index, the smallest index of a point equal to it.
    pub(crate) representative: Vec<u32>,
    /// The representatives, ascending.
    pub(crate) representatives: Vec<u32>,
    /// D = 2: how the chain gets its points, decided on the input points
    /// (#496). `None` for any other D, and where the polygon gave up the
    /// exact evaluation.
    pub(crate) chain: Option<Chain>,
    /// The lexicographically minimum affine basis: D + 1 representatives.
    pub(crate) spanning_points: Vec<u32>,
}

impl<'a> Input<'a> {
    /// The input of the points `points` (dimension `dim`), all distinct and
    /// each its own representative, with the affine basis `spanning_points`:
    /// the copy in space order that construction of D = 3 runs on (ADR 0009,
    /// #489).
    pub(crate) fn relabeled(dim: usize, points: &'a [f64], spanning_points: Vec<u32>) -> Self {
        let ids: Vec<u32> = (0..(points.len() / dim) as u32).collect();
        Self {
            dim,
            points,
            representative: ids.clone(),
            representatives: ids,
            chain: None,
            spanning_points,
        }
    }

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
    pub(crate) fn rows(&self) -> (&'a [f64], usize) {
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

/// Runs every input check of design §3 and returns the accepted input of a
/// hull, with the D = 2 chain planned.
pub(crate) fn accept(dim: usize, points: &[f64]) -> Result<Input<'_>, ConvexHullError> {
    accept_by(dim, points, representatives_of_hull)
}

/// [`accept`] for Delaunay sites, which build no D = 2 chain: duplicates by
/// hashing in every dimension.
pub(crate) fn accept_sites(dim: usize, points: &[f64]) -> Result<Input<'_>, ConvexHullError> {
    accept_by(dim, points, |dim, count, points| {
        (representatives_of(count, rows(dim, points)), None)
    })
}

/// The representatives of `count` points of dimension `dim`, row-major in
/// `points`, and the plan of the D = 2 chain.
type Representatives = fn(usize, usize, &[f64]) -> (Vec<u32>, Option<Chain>);

/// Point `i` of the rows `points` of dimension `dim`.
fn rows<'p>(dim: usize, points: &'p [f64]) -> impl Fn(u32) -> &'p [f64] {
    move |i| &points[i as usize * dim..(i as usize + 1) * dim]
}

/// [`accept`] with the duplicates found by `representatives_by`.
fn accept_by(
    dim: usize,
    points: &[f64],
    representatives_by: Representatives,
) -> Result<Input<'_>, ConvexHullError> {
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
    let point = rows(dim, points);
    let (representative, chain) = representatives_by(dim, count, points);
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
        chain,
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

/// The representatives of a hull's points, and for D = 2 the plan of the
/// chain.
///
/// D = 2 decides first whether the chain reads every point. Where it does,
/// the chain sorts every point by its coordinates, and equal points are
/// then adjacent: one sort finds the duplicates and orders the chain. Where
/// the polygon discards most points, hashing costs less than a sort of all
/// of them (#496).
fn representatives_of_hull(dim: usize, count: usize, points: &[f64]) -> (Vec<u32>, Option<Chain>) {
    let point = rows(dim, points);
    if dim != 2 {
        return (representatives_of(count, point), None);
    }
    match polygon::discard_edges(Rows(points), 0..count as u32) {
        Ok(None) => {
            let (representative, sorted) = representatives_by_coordinates(count, point);
            (representative, Some(Chain::Every(sorted)))
        }
        Ok(Some(edges)) => (
            representatives_of(count, point),
            Some(Chain::Discard(edges)),
        ),
        Err(ExactEvaluationExhausted) => (representatives_of(count, point), None),
    }
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

/// [`representatives_of`] for D = 2, with the representatives sorted as
/// the chain reads them ([`polygon::sorted_by_coordinates`]): equal points
/// have one key and are adjacent, and the smallest index of a class comes
/// first among them.
fn representatives_by_coordinates<'p>(
    count: usize,
    point: impl Fn(u32) -> &'p [f64],
) -> (Vec<u32>, Vec<u32>) {
    let mut keyed: Vec<(u64, u64, u32)> = (0..count as u32)
        .map(|i| {
            let (x, y) = polygon::coordinate_key(point(i));
            (x, y, i)
        })
        .collect();
    keyed.sort_unstable();
    let mut representative = vec![0_u32; count];
    let mut sorted = Vec::new();
    let mut class: Option<(u64, u64, u32)> = None;
    for &(x, y, i) in &keyed {
        representative[i as usize] = match class {
            Some((cx, cy, r)) if (cx, cy) == (x, y) => r,
            _ => {
                class = Some((x, y, i));
                sorted.push(i);
                i
            }
        };
    }
    (representative, sorted)
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
    let mut rows = Vec::new();
    Ok(minimum_basis_with(dim, representatives, point, &mut rows)?.to_vec())
}

/// [`minimum_basis`] with `rows` as its scratch: the projected rows of one
/// orientation go into that one buffer, so a caller that asks per facet
/// allocates nothing per facet, and no point tested costs an allocation. A
/// list per projected point grew with every point of a collinear or
/// coplanar prefix, a side's or a face's worth on a grid (#530).
pub(crate) fn minimum_basis_with<'p>(
    dim: usize,
    representatives: &[u32],
    point: impl Fn(u32) -> &'p [f64],
    rows: &mut Vec<f64>,
) -> Result<Small<u32, 11>, ConvexHullError> {
    let mut basis: Small<u32, 11> = Small::new();
    let Some((&first, rest)) = representatives.split_first() else {
        return Ok(basis);
    };
    basis.push(first);
    // The largest orientation: D + 1 points of D coordinates.
    rows.clear();
    rows.reserve((dim + 1) * dim);
    let mut axes: Small<usize, 11> = Small::new();
    for &p in rest {
        if basis.len() == dim + 1 {
            break;
        }
        for j in (0..dim).filter(|j| !axes.contains(j)) {
            rows.clear();
            for &q in basis.iter().chain(core::iter::once(&p)) {
                let q = point(q);
                rows.extend(axes.iter().map(|&a| q[a]).chain(core::iter::once(q[j])));
            }
            let refs: Small<&[f64], 12> = rows.chunks_exact(axes.len() + 1).collect();
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

    /// D = 2's one sort by coordinates gives the representatives of the
    /// sort by `compare_points`, and lists them in the chain's order: x,
    /// then y, signed zeros as one value, then index.
    #[test]
    fn sorting_by_coordinates_gives_the_same_representatives() {
        for count in [1, 2, 7, 100, 5000] {
            let points = duplicate_heavy(2, count, 0x4960 + count as u64);
            let point = |i: u32| &points[i as usize * 2..i as usize * 2 + 2];
            let (representative, sorted) = representatives_by_coordinates(count, point);
            assert_eq!(
                representative,
                representatives_by_sort(count, point),
                "count {count}"
            );
            let mut expected: Vec<u32> = (0..count as u32)
                .filter(|&i| representative[i as usize] == i)
                .collect();
            expected.sort_by(|&a, &b| compare_points(point(a), point(b)).then(a.cmp(&b)));
            assert_eq!(sorted, expected, "count {count}");
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
