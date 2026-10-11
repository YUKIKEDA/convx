//! D = 2 before and around the chain (design §6): the polygon of the
//! directional extremes that discards points, whether discarding pays, and
//! the cycle of strict left turns.
//!
//! These read the rows of the input, so acceptance can decide on the input
//! points, before duplicates are removed, whether the chain reads every
//! point. Where it does, the duplicates come from the chain's own sort by
//! coordinates; where the polygon discards most points, from hashing
//! (#496).

use crate::predicates::{orient, orient2_filter, ExactEvaluationExhausted, Sign};

/// The most points [`discard_edges`] samples.
const DISCARD_SAMPLE: usize = 1024;

/// Discarding goes ahead when at least one sampled point in this many is
/// proved inside. Timed on a unit circle with a share of its points moved
/// inside (10^5 and 10^6 points, #232): with one point in 64 inside,
/// discarding was 1 to 7% slower than not; with one in 32, the two were
/// equal; with one in 16, discarding was 3 to 8% faster.
const DISCARD_ONE_IN: usize = 32;

/// An edge of the discard polygon as the coordinates of its two ends.
pub(crate) type Edge = [[f64; 2]; 2];

/// How the D = 2 chain gets its points, decided in acceptance.
pub(crate) enum Chain {
    /// Every representative, sorted by [`coordinate_key`] and index: there
    /// is no polygon, or discarding would not pay.
    Every(Vec<u32>),
    /// The representatives not strictly inside the polygon of these edges,
    /// counterclockwise.
    Discard(Vec<Edge>),
}

/// The rows of a D = 2 input: point `i` is `rows[2 i..2 i + 2]`.
#[derive(Clone, Copy)]
pub(crate) struct Rows<'a>(pub(crate) &'a [f64]);

impl<'a> Rows<'a> {
    fn point(self, i: u32) -> &'a [f64] {
        &self.0[2 * i as usize..2 * i as usize + 2]
    }

    fn orient(self, ids: [u32; 3]) -> Result<Sign, ExactEvaluationExhausted> {
        orient(&ids.map(|i| self.point(i)))
    }
}

/// The edges of the polygon that discards points, when discarding pays:
/// `None` when there is no polygon, or when, of a stride of at most
/// [`DISCARD_SAMPLE`] of `points` in their order, fewer than one in
/// [`DISCARD_ONE_IN`] is proved strictly inside.
///
/// The polygon is the strict cycle of the farthest of `points` in each of
/// eight directions, counterclockwise; a tie keeps the earlier point.
/// `points` ascend, so on the input points with duplicates this is the
/// polygon of the representatives: the first of equal points is the
/// representative. Duplicates enter the sample, which changes only whether
/// discarding goes ahead, not the output.
///
/// A point that is not discarded costs its place in the sort and its
/// classification; one that is tested and not discarded costs the tests as
/// well. Where every point is extreme, as on a circle, the pass found
/// nothing and cost 4 to 6% of the build (#232).
pub(crate) fn discard_edges(
    rows: Rows<'_>,
    points: impl ExactSizeIterator<Item = u32> + Clone,
) -> Result<Option<Vec<Edge>>, ExactEvaluationExhausted> {
    let cycle = discard_polygon(rows, points.clone())?;
    if cycle.is_empty() {
        return Ok(None);
    }
    let edges = edges_of(rows, &cycle);
    let step = (points.len() / DISCARD_SAMPLE).max(1);
    let (mut seen, mut inside) = (0usize, 0usize);
    for p in points.step_by(step).take(DISCARD_SAMPLE) {
        seen += 1;
        inside += usize::from(clearly_inside(&edges, rows.point(p)));
    }
    Ok((inside * DISCARD_ONE_IN >= seen).then_some(edges))
}

/// The edges of the polygon of the farthest of the `count` points of
/// `rows` in each of eight directions, as [`discard_edges`] builds it,
/// counterclockwise; `None` when it has fewer than three vertices.
/// Classification of D = 3 builds it inside a facet, on the facet's
/// vertices projected to a coordinate plane (#504).
pub(crate) fn extremes_polygon(
    rows: Rows<'_>,
    count: u32,
) -> Result<Option<Vec<Edge>>, ExactEvaluationExhausted> {
    let cycle = discard_polygon(rows, 0..count)?;
    Ok((!cycle.is_empty()).then(|| edges_of(rows, &cycle)))
}

/// The polygon of [`discard_edges`]. Fewer than three vertices mean no
/// polygon, and the list is empty.
///
/// The directions are compared as rounded sums and differences. Any input
/// points make a valid polygon, so the rounding only changes which ones it
/// takes.
fn discard_polygon(
    rows: Rows<'_>,
    points: impl Iterator<Item = u32>,
) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let mut best = [f64::NEG_INFINITY; 8];
    let mut farthest = [None; 8];
    for r in points {
        let p = rows.point(r);
        let (x, y) = (p[0], p[1]);
        let keys = [x, -x, y, -y, x + y, x - y, y - x, -x - y];
        for ((key, best), farthest) in keys.into_iter().zip(&mut best).zip(&mut farthest) {
            // The points ascend, so a strict test keeps the smaller index
            // on a tie.
            if key > *best {
                *best = key;
                *farthest = Some(r);
            }
        }
    }
    let mut extremes: Vec<u32> = farthest.into_iter().flatten().collect();
    extremes.sort_unstable();
    extremes.dedup();
    let cycle = strict_cycle_of_sorted(rows, sorted_by_coordinates(rows, extremes))?;
    Ok(if cycle.len() < 3 { Vec::new() } else { cycle })
}

/// The edges of the counterclockwise convex `cycle`, each as the
/// coordinates of its two ends, gathered once for [`clearly_inside`]: every
/// point is tested against them, and read from the input they were most of
/// its cost on hull `cube` D2 (#324).
fn edges_of(rows: Rows<'_>, cycle: &[u32]) -> Vec<Edge> {
    let n = cycle.len();
    (0..n)
        .map(|i| {
            let (a, b) = (rows.point(cycle[i]), rows.point(cycle[(i + 1) % n]));
            [[a[0], a[1]], [b[0], b[1]]]
        })
        .collect()
}

/// `point` is a strict left turn of every edge of a counterclockwise convex
/// polygon (`edges`), by the certified filter: strictly inside it.
pub(crate) fn clearly_inside(edges: &[Edge], point: &[f64]) -> bool {
    edges
        .iter()
        .all(|[a, b]| orient2_filter(a, b, point) == Some(Sign::Positive))
}

/// The key the chain sorts a point by, with its index after it: x, then y,
/// each by [`f64::total_cmp`] after adding `+0.0`, so that `-0.0` and
/// `+0.0`, which are one value (design §3), have one key.
pub(crate) fn coordinate_key(p: &[f64]) -> (u64, u64) {
    (total_key(p[0] + 0.0), total_key(p[1] + 0.0))
}

/// `points` sorted by [`coordinate_key`], then index: one key per point,
/// read once, instead of two reads of the input per comparison (#322).
pub(crate) fn sorted_by_coordinates(rows: Rows<'_>, points: Vec<u32>) -> Vec<u32> {
    let mut keyed: Vec<(u64, u64, u32)> = points
        .iter()
        .map(|&i| {
            let (x, y) = coordinate_key(rows.point(i));
            (x, y, i)
        })
        .collect();
    drop(points);
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, _, i)| i).collect()
}

/// The cycle of strict left turns of `sorted`, counterclockwise: the lower
/// chain, then the upper chain without the two endpoints the lower one
/// already lists. `sorted` is distinct points sorted as
/// [`sorted_by_coordinates`] sorts them. Fewer than three points come back
/// as they are.
///
/// The chains read the sorted coordinates in order.
pub(crate) fn strict_cycle_of_sorted(
    rows: Rows<'_>,
    sorted: Vec<u32>,
) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let xy: Vec<[f64; 2]> = sorted
        .iter()
        .map(|&i| {
            let p = rows.point(i);
            [p[0], p[1]]
        })
        .collect();
    let n = sorted.len();
    if n < 3 {
        return Ok(sorted);
    }
    // As Akl–Toussaint's algorithm does, the points strictly below the line
    // from the first point to the last go to the lower chain, those strictly
    // above to the upper chain, and those on it to neither: a point on that
    // segment is no strict extreme. Each point is scanned once, where a
    // chain over every point scanned each twice (#515).
    let last = n as u32 - 1;
    let mut below: Vec<u32> = Vec::with_capacity(n / 2 + 2);
    let mut above: Vec<u32> = Vec::with_capacity(n / 2 + 2);
    below.push(0);
    above.push(last);
    for at in 1..last {
        let (a, b, c) = (0, last as usize, at as usize);
        let sign = match orient2_filter(&xy[a], &xy[b], &xy[c]) {
            Some(sign) => {
                #[cfg(debug_assertions)]
                debug_assert_eq!(sign, rows.orient([sorted[a], sorted[b], sorted[c]])?);
                sign
            }
            None => rows.orient([sorted[a], sorted[b], sorted[c]])?,
        };
        match sign {
            Sign::Negative => below.push(at),
            Sign::Positive => above.push(at),
            Sign::Zero => {}
        }
    }
    below.push(last);
    above.push(0);
    // The chains hold positions in `sorted`; the upper one runs from the
    // last point back to the first.
    let mut lower: Vec<u32> = Vec::with_capacity(below.len());
    for &at in &below {
        pop_until_left(rows, &sorted, &xy, &mut lower, at)?;
        lower.push(at);
    }
    let mut upper: Vec<u32> = Vec::with_capacity(above.len());
    upper.push(last);
    for &at in above[1..above.len() - 1].iter().rev() {
        pop_until_left(rows, &sorted, &xy, &mut upper, at)?;
        upper.push(at);
    }
    pop_until_left(rows, &sorted, &xy, &mut upper, 0)?;
    upper.push(0);
    let upper_middle = upper.len().saturating_sub(2);
    Ok(lower
        .into_iter()
        .chain(upper.into_iter().skip(1).take(upper_middle))
        .map(|at| sorted[at as usize])
        .collect())
}

/// Drops the tail of `chain` while the point at position `at` is not a
/// strict left turn from it. Positions index `sorted` and its coordinates
/// `xy`.
fn pop_until_left(
    rows: Rows<'_>,
    sorted: &[u32],
    xy: &[[f64; 2]],
    chain: &mut Vec<u32>,
    at: u32,
) -> Result<(), ExactEvaluationExhausted> {
    while chain.len() >= 2 {
        let a = chain[chain.len() - 2] as usize;
        let b = chain[chain.len() - 1] as usize;
        let c = at as usize;
        let ids = [sorted[a], sorted[b], sorted[c]];
        let sign = if let Some(sign) = orient2_filter(&xy[a], &xy[b], &xy[c]) {
            #[cfg(debug_assertions)]
            debug_assert_eq!(sign, rows.orient(ids)?);
            sign
        } else {
            rows.orient(ids)?
        };
        if sign == Sign::Positive {
            break;
        }
        chain.pop();
    }
    Ok(())
}

/// A key that orders `x` as [`f64::total_cmp`] does: for a negative sign
/// every bit is flipped, otherwise only the sign bit. The two give the
/// order of the signed integer `total_cmp` compares, shifted to unsigned.
fn total_key(x: f64) -> u64 {
    let bits = x.to_bits();
    if bits >> 63 == 1 {
        !bits
    } else {
        bits | 1 << 63
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chain before #515: a lower and an upper chain over every point.
    fn two_passes(rows: Rows<'_>, sorted: &[u32]) -> Vec<u32> {
        let xy: Vec<[f64; 2]> = sorted
            .iter()
            .map(|&i| {
                let p = rows.point(i);
                [p[0], p[1]]
            })
            .collect();
        let mut lower: Vec<u32> = Vec::new();
        for at in 0..sorted.len() as u32 {
            pop_until_left(rows, sorted, &xy, &mut lower, at).unwrap();
            lower.push(at);
        }
        let mut upper: Vec<u32> = Vec::new();
        for at in (0..sorted.len() as u32).rev() {
            pop_until_left(rows, sorted, &xy, &mut upper, at).unwrap();
            upper.push(at);
        }
        let upper_middle = upper.len().saturating_sub(2);
        lower
            .into_iter()
            .chain(upper.into_iter().skip(1).take(upper_middle))
            .map(|at| sorted[at as usize])
            .collect()
    }

    /// The chain on the two halves is the chain over every point twice, on
    /// random points, small grids (many collinear points), points on a
    /// circle, and points on one line.
    #[test]
    fn the_halves_give_the_cycle_of_two_passes() {
        let mut state = 515_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut cases: Vec<Vec<f64>> = Vec::new();
        for n in [1, 2, 3, 4, 10, 200] {
            cases.push((0..2 * n).map(|_| (next() % 1000) as f64 / 7.0).collect());
            cases.push((0..2 * n).map(|_| (next() % 4) as f64).collect());
            cases.push(
                (0..n)
                    .flat_map(|i| {
                        let t = i as f64 / n as f64 * core::f64::consts::TAU;
                        [t.cos(), t.sin()]
                    })
                    .collect(),
            );
            cases.push((0..n).flat_map(|i| [i as f64, 2.0 * i as f64]).collect());
        }
        let mut compared = 0;
        for points in &cases {
            let rows = Rows(points);
            let count = points.len() as u32 / 2;
            let mut sorted = sorted_by_coordinates(rows, (0..count).collect());
            sorted
                .dedup_by(|a, b| coordinate_key(rows.point(*a)) == coordinate_key(rows.point(*b)));
            assert_eq!(
                strict_cycle_of_sorted(rows, sorted.clone()).unwrap(),
                two_passes(rows, &sorted),
                "{points:?}"
            );
            compared += 1;
        }
        assert_eq!(compared, 24);
    }
    /// `total_key` orders values as `f64::total_cmp` does, the order the
    /// chain's sort keeps (#322): signed zeros, subnormals, the ends of the
    /// finite range, and random values of both signs, pairwise.
    #[test]
    fn total_key_orders_as_total_cmp() {
        let mut values = vec![
            0.0,
            -0.0,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            f64::from_bits(1),
            -f64::from_bits(1),
            f64::MAX,
            f64::MIN,
            1.0,
            -1.0,
            0.5,
            -0.5,
        ];
        let mut state = 322_u64;
        for _ in 0..200 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let x = f64::from_bits(state);
            if x.is_finite() {
                values.push(x);
            }
        }
        for &a in &values {
            for &b in &values {
                assert_eq!(
                    super::total_key(a).cmp(&super::total_key(b)),
                    a.total_cmp(&b),
                    "{a:e} against {b:e}"
                );
            }
        }
    }
}
