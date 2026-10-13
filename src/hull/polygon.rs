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
use crate::small::Small;

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
/// counterclockwise, into `edges`; `false` when it has fewer than three
/// vertices. Classification of D = 3 builds it inside a facet, on the
/// facet's vertices projected to a coordinate plane (#504), through the
/// buffers of `chain`: built from nothing it allocated for each facet with
/// a point on it (#531).
pub(crate) fn extremes_polygon(
    rows: Rows<'_>,
    count: u32,
    chain: &mut ChainWork,
    edges: &mut Vec<Edge>,
) -> Result<bool, ExactEvaluationExhausted> {
    let extremes = discard_extremes(rows, 0..count);
    let cycle = chain.strict_cycle(rows, extremes.iter().copied())?;
    edges.clear();
    if cycle.len() < 3 {
        return Ok(false);
    }
    let n = cycle.len();
    edges.extend((0..n).map(|i| {
        let (a, b) = (rows.point(cycle[i]), rows.point(cycle[(i + 1) % n]));
        [[a[0], a[1]], [b[0], b[1]]]
    }));
    Ok(true)
}

/// The polygon of [`discard_edges`]. Fewer than three vertices mean no
/// polygon, and the list is empty.
///
/// The directions are compared as rounded sums and differences. Any input
/// points make a valid polygon, so the rounding only changes which ones it
/// takes.
fn discard_polygon(
    rows: Rows<'_>,
    points: impl Iterator<Item = u32> + Clone,
) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let extremes = discard_extremes(rows, points).to_vec();
    let cycle = strict_cycle_of_sorted(rows, sorted_by_coordinates(rows, extremes))?;
    Ok(if cycle.len() < 3 { Vec::new() } else { cycle })
}

/// The farthest of `points` in each of the eight directions of
/// [`discard_polygon`], ascending, each once.
fn discard_extremes(rows: Rows<'_>, points: impl Iterator<Item = u32> + Clone) -> Small<u32, 8> {
    // The eight keys of a point: its coordinates and their sums and
    // differences, each greatest at the farthest point of one direction.
    let keys = |r: u32| {
        let p = rows.point(r);
        let (x, y) = (p[0], p[1]);
        [x, -x, y, -y, x + y, x - y, y - x, -x - y]
    };
    // The greatest of each key first, and then the first point that reaches
    // each (#645). One pass that kept each maximum with its index tested
    // and stored eight times a point, 11 ms of the hull of 10^6 points; a
    // pass of maxima alone is eight of them side by side, and the second
    // pass stops at the last of the eight points (#644). The points are
    // those of the one pass: the first, in the order of `points`, whose key
    // is the greatest, so the smaller index on a tie, as the points ascend.
    let mut best = [f64::NEG_INFINITY; 8];
    for r in points.clone() {
        for (key, best) in keys(r).into_iter().zip(&mut best) {
            *best = best.max(key);
        }
    }
    let mut farthest = [None; 8];
    let mut missing = best.iter().filter(|b| **b > f64::NEG_INFINITY).count();
    for r in points {
        if missing == 0 {
            break;
        }
        let keys = keys(r);
        if !keys.iter().zip(&best).any(|(key, best)| key == best) {
            continue;
        }
        for ((key, best), farthest) in keys.into_iter().zip(&best).zip(&mut farthest) {
            // A key that is nowhere above negative infinity has no farthest
            // point, as in the one pass, whose test was strict.
            if key == *best && key > f64::NEG_INFINITY && farthest.is_none() {
                *farthest = Some(r);
                missing -= 1;
            }
        }
    }
    let mut found: [u32; 8] = [0; 8];
    let mut count = 0;
    for r in farthest.into_iter().flatten() {
        found[count] = r;
        count += 1;
    }
    found[..count].sort_unstable();
    let mut extremes = Small::new();
    for (i, &r) in found[..count].iter().enumerate() {
        if i == 0 || found[i - 1] != r {
            extremes.push(r);
        }
    }
    extremes
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

/// The test that discards a point clearly inside a counterclockwise convex
/// polygon: a box inside the polygon first, and the polygon's edges for a
/// point outside the box (#646).
///
/// A box is taken when its four corners are clearly inside by the edges.
/// The inside of a convex polygon is convex, so a box whose corners are
/// strictly inside lies strictly inside, and so does a point strictly
/// inside the box. Four comparisons then stand for up to eight filtered
/// orientations: a quarter of the pass over 10^6 points spread in a square
/// (#644).
///
/// Where the points are known ([`Self::about`]) the box is tried on them: a
/// sample of the points gives, on each coordinate, the values that leave a
/// share of the sample below and above, and the box between them is tried
/// from the widest share inward. It then holds the points wherever they
/// are dense, and not the middle of a polygon that a few far points
/// stretch. Otherwise, and when none of those is inside, it is tried about
/// the mean of the polygon's vertices at a few shares of the polygon's
/// extent. When nothing is inside, as for a thin polygon, there is no box
/// and the edges decide every point.
pub(crate) struct Discard<'e> {
    edges: &'e [Edge],
    /// The box as `[x low, x high, y low, y high]`, when there is one.
    inner: Option<[f64; 4]>,
}

/// The shares of the polygon's extent tried for the box of [`Discard`]
/// about the mean of its vertices, largest first.
const BOX_SHARES: [f64; 6] = [0.7, 0.6, 0.5, 0.4, 0.3, 0.2];

/// The most points [`Discard::about`] samples, and the shares of the sample
/// left outside the box on each side of a coordinate, smallest first.
const BOX_SAMPLE: usize = 255;
const BOX_TAILS: [f64; 6] = [0.03, 0.06, 0.12, 0.2, 0.3, 0.4];

impl<'e> Discard<'e> {
    /// The test for the polygon of `edges`.
    pub(crate) fn new(edges: &'e [Edge]) -> Self {
        Self {
            edges,
            inner: Self::about_the_vertices(edges),
        }
    }

    /// The test for the polygon of `edges`, with a box about where
    /// `points` are, read at one stride.
    pub(crate) fn about(edges: &'e [Edge], rows: Rows<'_>, points: &[u32]) -> Self {
        let stride = points.len().div_ceil(BOX_SAMPLE).max(1);
        let (mut xs, mut ys) = ([0.0; BOX_SAMPLE], [0.0; BOX_SAMPLE]);
        let mut count = 0;
        for &r in points.iter().step_by(stride).take(BOX_SAMPLE) {
            let p = rows.point(r);
            (xs[count], ys[count]) = (p[0], p[1]);
            count += 1;
        }
        let (xs, ys) = (&mut xs[..count], &mut ys[..count]);
        xs.sort_unstable_by(f64::total_cmp);
        ys.sort_unstable_by(f64::total_cmp);
        let inner = BOX_TAILS
            .iter()
            .filter_map(|tail| {
                let low = (tail * count as f64) as usize;
                let high = count.checked_sub(1 + low)?;
                (low < high).then(|| [xs[low], xs[high], ys[low], ys[high]])
            })
            .find(|b| Self::holds_the_box(edges, b))
            .or_else(|| Self::about_the_vertices(edges));
        Self { edges, inner }
    }

    /// A box about the mean of the polygon's vertices, if one of
    /// [`BOX_SHARES`] of its extent is inside.
    fn about_the_vertices(edges: &[Edge]) -> Option<[f64; 4]> {
        let count = edges.len() as f64;
        let (mut cx, mut cy) = (0.0, 0.0);
        for [vertex, _] in edges {
            cx += vertex[0] / count;
            cy += vertex[1] / count;
        }
        let (mut hx, mut hy) = (0.0_f64, 0.0_f64);
        for [vertex, _] in edges {
            hx = hx.max((vertex[0] - cx).abs());
            hy = hy.max((vertex[1] - cy).abs());
        }
        BOX_SHARES
            .iter()
            .map(|share| {
                [
                    cx - share * hx,
                    cx + share * hx,
                    cy - share * hy,
                    cy + share * hy,
                ]
            })
            .find(|b| Self::holds_the_box(edges, b))
    }

    /// Whether the four corners of `b` are clearly inside the polygon.
    fn holds_the_box(edges: &[Edge], b: &[f64; 4]) -> bool {
        [[b[0], b[2]], [b[1], b[2]], [b[1], b[3]], [b[0], b[3]]]
            .iter()
            .all(|corner| clearly_inside(edges, corner))
    }

    /// Whether `point` is strictly inside the box.
    #[inline(always)]
    fn in_the_box(&self, point: &[f64]) -> bool {
        self.inner.is_some_and(|b| {
            (point[0] > b[0]) & (point[0] < b[1]) & (point[1] > b[2]) & (point[1] < b[3])
        })
    }

    /// Whether `point` is proved strictly inside the polygon.
    #[inline(always)]
    pub(crate) fn clearly_inside(&self, point: &[f64]) -> bool {
        self.in_the_box(point) || clearly_inside(self.edges, point)
    }
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
    let mut work = ChainWork::default();
    work.sort(rows, points.iter().copied());
    drop(points);
    work.sorted
}

/// The cycle of strict left turns of `sorted`, counterclockwise: the lower
/// chain, then the upper chain without the two endpoints the lower one
/// already lists. `sorted` is distinct points sorted as
/// [`sorted_by_coordinates`] sorts them. Fewer than three points come back
/// as they are.
pub(crate) fn strict_cycle_of_sorted(
    rows: Rows<'_>,
    sorted: Vec<u32>,
) -> Result<Vec<u32>, ExactEvaluationExhausted> {
    let mut work = ChainWork {
        sorted,
        ..ChainWork::default()
    };
    work.cycle_of_sorted(rows)?;
    Ok(work.cycle)
}

/// The buffers of a chain of D = 2, kept from one run to the next (#530):
/// classification runs one chain per face of a hull of D = 3 that is not a
/// simplex, and a run on a kept workspace allocates nothing.
#[derive(Default)]
pub(crate) struct ChainWork {
    keyed: Vec<(u64, u64, u32)>,
    /// The points in the chain's order.
    sorted: Vec<u32>,
    /// The coordinates of `sorted`, which the chains read in order.
    xy: Vec<[f64; 2]>,
    below: Vec<u32>,
    above: Vec<u32>,
    lower: Vec<u32>,
    upper: Vec<u32>,
    cycle: Vec<u32>,
}

impl ChainWork {
    /// The cycle of strict left turns of the distinct `points` of `rows`,
    /// counterclockwise from the first in the chain's order.
    pub(crate) fn strict_cycle(
        &mut self,
        rows: Rows<'_>,
        points: impl Iterator<Item = u32>,
    ) -> Result<&[u32], ExactEvaluationExhausted> {
        self.sort(rows, points);
        self.cycle_of_sorted(rows)?;
        Ok(&self.cycle)
    }

    /// `points` into `self.sorted`, by [`coordinate_key`], then index.
    fn sort(&mut self, rows: Rows<'_>, points: impl Iterator<Item = u32>) {
        self.keyed.clear();
        self.keyed.extend(points.map(|i| {
            let (x, y) = coordinate_key(rows.point(i));
            (x, y, i)
        }));
        self.keyed.sort_unstable();
        self.sorted.clear();
        self.sorted.extend(self.keyed.iter().map(|&(_, _, i)| i));
    }

    /// [`strict_cycle_of_sorted`] of `self.sorted`, into `self.cycle`.
    ///
    /// The chains read the sorted coordinates in order.
    fn cycle_of_sorted(&mut self, rows: Rows<'_>) -> Result<(), ExactEvaluationExhausted> {
        let Self {
            sorted,
            xy,
            below,
            above,
            lower,
            upper,
            cycle,
            ..
        } = self;
        cycle.clear();
        let n = sorted.len();
        if n < 3 {
            cycle.extend_from_slice(sorted);
            return Ok(());
        }
        xy.clear();
        xy.extend(sorted.iter().map(|&i| {
            let p = rows.point(i);
            [p[0], p[1]]
        }));
        // As Akl–Toussaint's algorithm does, the points strictly below the
        // line from the first point to the last go to the lower chain, those
        // strictly above to the upper chain, and those on it to neither: a
        // point on that segment is no strict extreme. Each point is scanned
        // once, where a chain over every point scanned each twice (#515).
        let last = n as u32 - 1;
        below.clear();
        above.clear();
        below.reserve(n / 2 + 2);
        above.reserve(n / 2 + 2);
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
        lower.clear();
        lower.reserve(below.len());
        for &at in below.iter() {
            pop_until_left(rows, sorted, xy, lower, at)?;
            lower.push(at);
        }
        upper.clear();
        upper.reserve(above.len());
        upper.push(last);
        for &at in above[1..above.len() - 1].iter().rev() {
            pop_until_left(rows, sorted, xy, upper, at)?;
            upper.push(at);
        }
        pop_until_left(rows, sorted, xy, upper, 0)?;
        upper.push(0);
        let upper_middle = upper.len().saturating_sub(2);
        cycle.reserve(lower.len() + upper_middle);
        cycle.extend(
            lower
                .iter()
                .chain(upper.iter().skip(1).take(upper_middle))
                .map(|&at| sorted[at as usize]),
        );
        Ok(())
    }
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

    /// The filtered orientation agrees with the exact one wherever it gives
    /// a sign, and gives one where its left product is exactly zero and the
    /// determinant is not (#646). Every triple of a small integer grid, where
    /// points share coordinates; the same grid at 2^-600, where the products
    /// of two differences underflow to zero while the differences do not;
    /// and both mixed, a zero factor on the left with an underflowing
    /// product on the right.
    #[test]
    fn the_filter_decides_where_its_left_product_is_exactly_zero() {
        let tiny = 2f64.powi(-600);
        let (mut decided_on_a_zero_left, mut undecided_nonzero, mut zeros) = (0, 0, 0);
        for scales in [[1.0, 1.0], [tiny, tiny], [1.0, tiny], [tiny, 1.0]] {
            let grid: Vec<[f64; 2]> = (0..25)
                .map(|i| [f64::from(i % 5) * scales[0], f64::from(i / 5) * scales[1]])
                .collect();
            for a in &grid {
                for b in &grid {
                    for c in &grid {
                        let exact = orient(&[&a[..], &b[..], &c[..]]).unwrap();
                        let filtered = orient2_filter(a, b, c);
                        assert!(
                            filtered.is_none() || filtered == Some(exact),
                            "{a:?} {b:?} {c:?}: {filtered:?} for {exact:?}"
                        );
                        assert_ne!(filtered, Some(Sign::Zero));
                        let left_is_zero = a[0] == c[0] || b[1] == c[1];
                        if exact == Sign::Zero {
                            zeros += 1;
                        } else if left_is_zero {
                            // The right product alone is the determinant.
                            assert_eq!(filtered, Some(exact), "{a:?} {b:?} {c:?}");
                            decided_on_a_zero_left += 1;
                        } else if filtered.is_none() {
                            undecided_nonzero += 1;
                        }
                    }
                }
            }
        }
        assert!(decided_on_a_zero_left > 10_000, "{decided_on_a_zero_left}");
        // Underflow with no zero factor is left to the exact orientation.
        assert!(undecided_nonzero > 100, "{undecided_nonzero}");
        assert!(zeros > 1000, "{zeros}");
    }

    /// What [`Discard`] discards is strictly inside the polygon by the exact
    /// orientation against every edge, and it discards whatever the edges
    /// alone do (#646). Polygons of the farthest points of: points in a
    /// square, in a disc, a rotated integer lattice (points share coordinates
    /// with its vertices), points dense about the origin with a long tail,
    /// and points near a segment, a polygon too thin for a box. The box
    /// about the points is found on all but the thin one and holds a
    /// quarter of the points or more; about the vertices it is found on the
    /// first three.
    #[test]
    fn the_box_discards_only_points_strictly_inside() {
        let mut state = 0x6460_0001_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1_u64 << 53) as f64
        };
        let mut sets: Vec<(&str, Vec<f64>)> = Vec::new();
        sets.push(("square", (0..4000).map(|_| next() - 0.5).collect()));
        let mut disc = Vec::new();
        while disc.len() < 4000 {
            let (x, y) = (2.0 * next() - 1.0, 2.0 * next() - 1.0);
            if x * x + y * y <= 1.0 {
                disc.extend([x, y]);
            }
        }
        sets.push(("disc", disc));
        let lattice = (0..40 * 40).flat_map(|i| {
            let (u, v) = (f64::from(i % 40), f64::from(i / 40));
            [3.0 * u - 4.0 * v, 4.0 * u + 3.0 * v]
        });
        sets.push(("lattice", lattice.collect()));
        let mut tail = Vec::new();
        for _ in 0..2000 {
            let radius = (1.0 - next()).powi(-3);
            tail.extend([radius * (next() - 0.5), radius * (next() - 0.5)]);
        }
        sets.push(("tail", tail));
        let thin = (0..2000).flat_map(|_| {
            let t = next();
            [t * 1000.0, t * 1000.0 + 1e-9 * (next() - 0.5)]
        });
        sets.push(("thin", thin.collect()));
        for (name, points) in &sets {
            let rows = Rows(points);
            let count = (points.len() / 2) as u32;
            let cycle = discard_polygon(rows, 0..count).unwrap();
            assert!(cycle.len() >= 3, "{name}");
            let edges = edges_of(rows, &cycle);
            let all: Vec<u32> = (0..count).collect();
            // About the vertices, a few far points leave no box on the long
            // tail; about the points there is one.
            let about_vertices = Discard::new(&edges);
            match *name {
                "square" | "disc" | "lattice" => assert!(about_vertices.inner.is_some(), "{name}"),
                "thin" | "tail" => assert!(about_vertices.inner.is_none(), "{name}"),
                _ => unreachable!(),
            }
            let discard = Discard::about(&edges, rows, &all);
            let has_box = discard.inner.is_some();
            assert_eq!(has_box, *name != "thin", "{name}: {:?}", discard.inner);
            let (mut by_the_box, mut by_the_box_alone) = (0, 0);
            for r in 0..count {
                let p = rows.point(r);
                let strictly_inside = (0..cycle.len()).all(|i| {
                    let (a, b) = (cycle[i], cycle[(i + 1) % cycle.len()]);
                    orient(&[rows.point(a), rows.point(b), p]).unwrap() == Sign::Positive
                });
                let by_edges = clearly_inside(&edges, p);
                if discard.in_the_box(p) {
                    assert!(strictly_inside, "{name}: point {r}");
                    by_the_box += 1;
                    by_the_box_alone += usize::from(!by_edges);
                }
                assert_eq!(
                    discard.clearly_inside(p),
                    discard.in_the_box(p) || by_edges,
                    "{name}: point {r}"
                );
                assert!(
                    !discard.clearly_inside(p) || strictly_inside,
                    "{name}: point {r}"
                );
            }
            assert_eq!(by_the_box > 0, has_box, "{name}");
            // The box about the points holds a good share of them: a
            // quarter at the least, on a disc, whose box is the smallest.
            assert!(
                !has_box || by_the_box * 4 > count as usize,
                "{name}: {by_the_box}"
            );
            // The box about the vertices discards only points strictly
            // inside as well.
            for r in 0..count {
                let p = rows.point(r);
                assert!(
                    !about_vertices.in_the_box(p) || clearly_inside(&edges, p),
                    "{name}: point {r}"
                );
            }
            // With the filter's zero left product decided, the edges alone
            // discard what the box does.
            assert_eq!(by_the_box_alone, 0, "{name}");
        }
    }

    /// The farthest points before #645: one pass that keeps each maximum
    /// with its index, a strict test keeping the smaller index on a tie.
    fn farthest_in_one_pass(rows: Rows<'_>, points: impl Iterator<Item = u32>) -> Vec<u32> {
        let mut best = [f64::NEG_INFINITY; 8];
        let mut farthest = [None; 8];
        for r in points {
            let p = rows.point(r);
            let (x, y) = (p[0], p[1]);
            let keys = [x, -x, y, -y, x + y, x - y, y - x, -x - y];
            for ((key, best), farthest) in keys.into_iter().zip(&mut best).zip(&mut farthest) {
                if key > *best {
                    *best = key;
                    *farthest = Some(r);
                }
            }
        }
        let mut found: Vec<u32> = farthest.into_iter().flatten().collect();
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The farthest points by the maxima first are those of the one pass
    /// (#645): on random points; on small grids, where every direction has
    /// many ties and the smaller index is to be kept; on points that are all
    /// one point, on one point, and on none; with both zeros; on every
    /// third point of a set; and on coordinates so large that a sum
    /// overflows, to an infinity on one side and, for the opposite
    /// direction, to a key that no point is above.
    #[test]
    fn the_farthest_points_are_those_of_one_pass() {
        let mut state = 0x6450_0001_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut sets: Vec<Vec<f64>> = Vec::new();
        for count in [1_usize, 2, 3, 9, 100, 5000] {
            sets.push(
                (0..2 * count)
                    .map(|_| (next() >> 11) as f64 / (1_u64 << 53) as f64 - 0.5)
                    .collect(),
            );
            sets.push((0..2 * count).map(|_| (next() % 4) as f64).collect());
            sets.push(
                (0..2 * count)
                    .map(|_| (next() % 3) as f64 - 1.0)
                    .map(|x| x * 0.0)
                    .collect(),
            );
        }
        sets.push(Vec::new());
        sets.push([7.5, -2.0].repeat(50));
        // Every sum of the two coordinates overflows: `x + y` is positive
        // infinity for all, and `-x - y` negative infinity for all.
        sets.push(
            (0..40)
                .flat_map(|i| [1.0e308 + f64::from(i) * 1.0e292, 1.5e308])
                .collect(),
        );
        // Some do and some do not.
        sets.push(
            (0..40)
                .flat_map(|i| [f64::from(i % 5) * 4.0e307, f64::from(i % 3) * 8.0e307])
                .collect(),
        );
        let mut differing = 0;
        for points in &sets {
            let rows = Rows(points);
            let count = (points.len() / 2) as u32;
            let every = farthest_in_one_pass(rows, 0..count);
            assert_eq!(discard_extremes(rows, 0..count).to_vec(), every);
            let third = || (0..count).step_by(3);
            let of_third = farthest_in_one_pass(rows, third());
            assert_eq!(discard_extremes(rows, third()).to_vec(), of_third);
            differing += usize::from(every != of_third);
        }
        // The points given decide the answer: a third of them has other
        // farthest points on most sets.
        assert!(differing > 8, "{differing}");
    }

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
