//! Distance scan that culls points proved strictly inside a facet, and the
//! proof of a strict side for one point.
//!
//! The scan evaluates, for each point `x`, the working distance
//! `w = sum_j (x_j - o_j) n_j` against the facet's working unit normal `n`
//! (from [`crate::normal::unit_normal`]) and origin vertex `o`, together with
//! `l = sum_j |x_j - o_j|`. A point is culled only when
//!
//! ```text
//! w < -((slope * l + floor) * (1 + 4u))
//! ```
//!
//! which proves that its exact orientation against the facet is the inside
//! sign. [`CullPlane::proved_side`] uses the same threshold for one point in
//! both directions: beyond it on either side, the working distance proves
//! the strict side, which is the exact orientation sign (design §1). Within
//! it, including on the plane, nothing is proved and the orientation
//! decides.
//!
//! # Why the test is sound
//!
//! Let `u*` be the exact outward unit normal and `tau >= |n - u*|`. For the
//! exact value `S = (x - o) . n`, `|(x - o) . u* - S| <= |x - o| tau`. The
//! rounding of `w` is at most `gamma_{d+1} sum |x_j - o_j| |n_j|`, plus
//! `d * 2^-1074` for products that underflow. With `|n_j| <= 1 + d u` and
//! `l >= (1 - gamma_d) sum |x_j - o_j|`, both terms are covered by
//! `slope = (4 (d + 1) u + 2 tau) (1 + 4u)` and `floor = (d + 1) 2^-1073`.
//! The factor `(1 + 4u)` in `slope` covers the rounding of evaluating
//! `slope` itself; the factor `(1 + 4u)` in the comparison covers the
//! rounding of `slope * l + floor`. Both round the threshold up, toward
//! culling less. Every bound is on an absolute value, so the same threshold
//! proves `(x - o) . u* > 0` when `w` exceeds it.
//!
//! `tau` is certified once per facet from the cofactor vector `c` of the
//! facet's edges (see [`crate::predicates::direction_cofactors`]): with
//! computed `c^` and `|c - c^| <= E`, `|c/|c| - c^/|c^|| <= 2E / |c^|`, and
//! `|n - c^/|c^||` is evaluated in `f64` with a margin for its own rounding.
//! When the cofactors cannot be certified, no point is culled.
//!
//! # Lifted facets
//!
//! For a facet of sites lifted to the paraboloid (design §1, §7, #109) the
//! points are rounded in their last coordinate only: each height is within
//! its stored bound `e` of the exact `|p|^2`. `u*` is then the unit normal of
//! the exact lifted plane, and `tau` bounds `|n - u*|` because the cofactors
//! are widened by [`crate::normal::lifted_facet_cofactors`]. Moving the query
//! `x` and the origin `o` to their exact heights changes `(x - o) . u*` by at
//! most `(e_x + e_o) |u*_last| <= e_x + e_o`. The origin's bound is added to
//! the floor, `floor' = (floor + e_o)(1 + 4u)`, and the query's just before
//! the comparison:
//!
//! ```text
//! w < -(((slope * l + floor') * (1 + 4u) + e_x) * (1 + 4u))
//! ```
//!
//! Each factor `(1 + 4u)` covers the roundings of the sum before it. The
//! scan reads `e_x` from the lifted row right after the coordinates. A plane
//! that is not lifted keeps the threshold above bit for bit.
//!
//! # Paths
//!
//! The vectorized path uses `pulp` runtime dispatch with one point per lane.
//! Each lane performs the same operations in the same order as the scalar
//! path (subtract, multiply, add, absolute value, add; never `mul_add`), so
//! both paths return bitwise identical `w` and `l` and the same cull set on
//! every CPU.

use pulp::{Arch, Simd, WithSimd};

#[cfg(test)]
use crate::normal::facet_cofactors;
use crate::predicates::Sign;

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;
/// 2^-1073.
const ETA: f64 = f64::from_bits(2);

/// A facet prepared for culling.
pub(crate) struct CullPlane {
    origin: Vec<f64>,
    normal: Vec<f64>,
    slope: f64,
    floor: f64,
    /// A facet of sites lifted to the paraboloid: the last coordinate of
    /// every point is a rounded height, and the threshold adds the query's
    /// height bound (#109).
    lifted: bool,
}

impl CullPlane {
    /// [`Self::with_cofactors`] evaluating the cofactors itself.
    #[cfg(test)]
    pub(crate) fn new(facet: &[&[f64]], normal: &[f64], outward: Sign) -> Option<Self> {
        Self::with_cofactors(facet, normal, outward, facet_cofactors(facet).as_deref())
    }

    /// Prepares `facet` (D points of dimension D) with its working unit
    /// `normal`, which must satisfy `orient_direction(facet, normal) ==
    /// outward`, and `cofactors`, the [`crate::normal::facet_cofactors`] of
    /// `facet`. Returns `None` when the plane error cannot be certified; the
    /// caller then culls nothing for this facet.
    pub(crate) fn with_cofactors(
        facet: &[&[f64]],
        normal: &[f64],
        outward: Sign,
        cofactors: Option<&[(f64, f64)]>,
    ) -> Option<Self> {
        let d = facet.len();
        debug_assert!(
            d >= 1 && normal.len() == d,
            "facet needs D points of dimension D"
        );
        let side = match outward {
            Sign::Positive => 1.0,
            Sign::Negative => -1.0,
            Sign::Zero => return None,
        };
        let tau = plane_error(cofactors?, normal, side)?;
        let n = d as f64;
        let slope = (4.0 * (n + 1.0) * UNIT_ROUNDOFF + 2.0 * tau) * (1.0 + 4.0 * UNIT_ROUNDOFF);
        let floor = (n + 1.0) * ETA;
        slope.is_finite().then(|| Self {
            origin: facet[0].to_vec(),
            normal: normal.to_vec(),
            slope,
            floor,
            lifted: false,
        })
    }

    /// [`Self::with_cofactors`] for a facet of sites lifted to the
    /// paraboloid. `facet` holds the rounded lifted coordinates, `cofactors`
    /// are the [`crate::normal::lifted_facet_cofactors`] of the facet,
    /// which bound the cofactors of the exact lift, and
    /// `origin_bound` bounds the rounding of the first vertex's height. A
    /// proved side is then the exact lifted orientation sign (#109).
    pub(crate) fn with_lifted_cofactors(
        facet: &[&[f64]],
        normal: &[f64],
        outward: Sign,
        cofactors: Option<&[(f64, f64)]>,
        origin_bound: f64,
    ) -> Option<Self> {
        let plane = Self::with_cofactors(facet, normal, outward, cofactors)?;
        // The origin's height error is added to the floor, rounded up.
        let floor = (plane.floor + origin_bound) * (1.0 + 4.0 * UNIT_ROUNDOFF);
        floor.is_finite().then_some(Self {
            floor,
            lifted: true,
            ..plane
        })
    }

    /// The working distance and the L1 distance of one point.
    fn scalar_terms(&self, point: &[f64]) -> (f64, f64) {
        let mut w = 0.0;
        let mut l = 0.0;
        for ((&x, &o), &n) in point.iter().zip(&self.origin).zip(&self.normal) {
            let diff = x - o;
            w += diff * n;
            l += diff.abs();
        }
        (w, l)
    }

    /// The certified threshold for a point at L1 distance `l` whose last
    /// coordinate is within `bound` of the exact one (0 unless lifted).
    fn threshold(&self, l: f64, bound: f64) -> f64 {
        let base = (self.slope * l + self.floor) * (1.0 + 4.0 * UNIT_ROUNDOFF);
        if self.lifted {
            (base + bound) * (1.0 + 4.0 * UNIT_ROUNDOFF)
        } else {
            debug_assert!(bound == 0.0, "only a lifted point has a height bound");
            base
        }
    }

    fn is_proved_inside(&self, w: f64, l: f64, bound: f64) -> bool {
        w < -self.threshold(l, bound)
    }

    /// The side of `point` that the working distance proves:
    /// [`Sign::Positive`] strictly outside, [`Sign::Negative`] strictly
    /// inside, or `None` when it proves neither. A proved sign is the exact
    /// orientation sign of the facet, in outward order, followed by the
    /// point (design §1).
    ///
    /// `bound` bounds the rounding of the point's last coordinate: the
    /// height bound of a lifted point, and 0 otherwise.
    pub(crate) fn proved_side(&self, point: &[f64], bound: f64) -> Option<Sign> {
        let (w, l) = self.scalar_terms(point);
        let threshold = self.threshold(l, bound);
        if w > threshold {
            Some(Sign::Positive)
        } else if w < -threshold {
            Some(Sign::Negative)
        } else {
            None
        }
    }

    /// Marks `inside[i] = true` for each `indices[i]` whose point in the
    /// row-major `points` is proved strictly inside. Other entries are set
    /// to `false`.
    ///
    /// Point `i` starts at `rows[i * stride]`. A lifted plane reads the
    /// height bound right after the point's coordinates, as the lifted rows
    /// store it.
    pub(crate) fn mark_inside(
        &self,
        rows: &[f64],
        stride: usize,
        indices: &[u32],
        inside: &mut [bool],
    ) {
        let d = self.origin.len();
        debug_assert!(
            stride >= d + usize::from(self.lifted),
            "a row holds the point, and its bound when lifted"
        );
        let mut w = vec![0.0; indices.len()];
        let mut l = vec![0.0; indices.len()];
        Arch::new().dispatch(Scan {
            plane: self,
            points: rows,
            stride,
            indices,
            w: &mut w,
            l: &mut l,
        });
        for (((flag, &w), &l), &index) in inside.iter_mut().zip(&w).zip(&l).zip(indices) {
            *flag = self.is_proved_inside(w, l, self.bound_of(rows, stride, index));
        }
    }

    /// The height bound of point `index` in `rows`, or 0 for a plain plane.
    fn bound_of(&self, rows: &[f64], stride: usize, index: u32) -> f64 {
        if self.lifted {
            rows[index as usize * stride + self.origin.len()]
        } else {
            0.0
        }
    }

    /// Reference path without SIMD. Same result as [`Self::mark_inside`],
    /// with the same rows, stride, and height bounds.
    #[cfg(test)]
    pub(crate) fn mark_inside_scalar(
        &self,
        rows: &[f64],
        stride: usize,
        indices: &[u32],
        inside: &mut [bool],
    ) {
        let d = self.origin.len();
        for (flag, &index) in inside.iter_mut().zip(indices) {
            let start = index as usize * stride;
            let (w, l) = self.scalar_terms(&rows[start..start + d]);
            *flag = self.is_proved_inside(w, l, self.bound_of(rows, stride, index));
        }
    }
}

/// Certified upper bound on `|normal - side * c / |c||`, from the filtered
/// `cofactors` of the facet (`crate::normal::facet_cofactors`): scaling
/// every coordinate by one exact power of two scales the cofactors by a
/// positive factor and leaves their direction unchanged.
fn plane_error(cofactors: &[(f64, f64)], normal: &[f64], side: f64) -> Option<f64> {
    let d = cofactors.len() as f64;
    let bound: f64 = cofactors.iter().map(|&(_, e)| e).sum::<f64>() * (1.0 + d * UNIT_ROUNDOFF);
    let length = cofactors.iter().map(|&(c, _)| c * c).sum::<f64>().sqrt();
    let length_low = length * (1.0 - (d + 3.0) * UNIT_ROUNDOFF);
    if !(length_low.is_finite() && length_low > 2.0 * bound) {
        return None;
    }
    let deviation = normal
        .iter()
        .zip(cofactors)
        .map(|(&n, &(c, _))| {
            let diff = n - side * (c / length);
            diff * diff
        })
        .sum::<f64>()
        .sqrt();
    let tau = (deviation + 2.0 * bound / length_low + 4.0 * (d + 4.0) * UNIT_ROUNDOFF)
        * (1.0 + 8.0 * UNIT_ROUNDOFF);
    tau.is_finite().then_some(tau)
}

/// One lane per point; the per-lane sequence matches [`CullPlane::scalar_terms`].
struct Scan<'a> {
    plane: &'a CullPlane,
    points: &'a [f64],
    stride: usize,
    indices: &'a [u32],
    w: &'a mut [f64],
    l: &'a mut [f64],
}

impl WithSimd for Scan<'_> {
    type Output = ();

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let d = self.plane.origin.len();
        let lanes = S::F64_LANES;
        let blocks = self.indices.len() / lanes;
        let mut column = vec![0.0; lanes];
        for block in 0..blocks {
            let indices = &self.indices[block * lanes..(block + 1) * lanes];
            let mut w = simd.splat_f64s(0.0);
            let mut l = simd.splat_f64s(0.0);
            for j in 0..d {
                for (slot, &index) in column.iter_mut().zip(indices) {
                    *slot = self.points[index as usize * self.stride + j];
                }
                let (x, _) = S::as_simd_f64s(&column);
                let diff = simd.sub_f64s(x[0], simd.splat_f64s(self.plane.origin[j]));
                w = simd.add_f64s(
                    w,
                    simd.mul_f64s(diff, simd.splat_f64s(self.plane.normal[j])),
                );
                l = simd.add_f64s(l, simd.abs_f64s(diff));
            }
            let range = block * lanes..(block + 1) * lanes;
            let (w_out, _) = S::as_mut_simd_f64s(&mut self.w[range.clone()]);
            w_out[0] = w;
            let (l_out, _) = S::as_mut_simd_f64s(&mut self.l[range]);
            l_out[0] = l;
        }
        for i in blocks * lanes..self.indices.len() {
            let start = self.indices[i] as usize * self.stride;
            let (w, l) = self.plane.scalar_terms(&self.points[start..start + d]);
            self.w[i] = w;
            self.l[i] = l;
        }
    }
}

/// The P2-7 timing sets' generator, for the cull timing below.
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/common/generator.rs"]
mod generator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normal::unit_normal;
    use crate::predicates::distance_sign;

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
    }

    fn prepare(facet: &[Vec<f64>]) -> CullPlane {
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        let normal = unit_normal(&refs, Sign::Positive).unwrap().unwrap();
        CullPlane::new(&refs, &normal, Sign::Positive).unwrap()
    }

    /// Every culled point must have the exact inside sign; returns the number
    /// culled.
    fn check_sound(facet: &[Vec<f64>], points: &[f64]) -> usize {
        let d = facet.len();
        let plane = prepare(facet);
        let n = points.len() / d;
        let indices: Vec<u32> = (0..n as u32).collect();
        let mut fast = vec![false; n];
        let mut slow = vec![false; n];
        plane.mark_inside(points, plane.origin.len(), &indices, &mut fast);
        plane.mark_inside_scalar(points, plane.origin.len(), &indices, &mut slow);
        assert_eq!(fast, slow, "SIMD and scalar paths disagree");
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        for (i, &culled) in fast.iter().enumerate() {
            if culled {
                let sign = distance_sign(&refs, &points[i * d..(i + 1) * d]).unwrap();
                assert_eq!(sign, Sign::Negative, "point {i} culled with sign {sign:?}");
            }
        }
        fast.iter().filter(|&&c| c).count()
    }

    #[test]
    fn random_points_cull_soundly_and_usefully() {
        let mut rng = Rng(3);
        for d in 1..=6 {
            let facet: Vec<Vec<f64>> = (0..d)
                .map(|_| (0..d).map(|_| rng.unit()).collect())
                .collect();
            let points: Vec<f64> = (0..1000 * d).map(|_| rng.unit() * 4.0).collect();
            let culled = check_sound(&facet, &points);
            assert!(culled > 300, "d = {d}: only {culled} of 1000 culled");
        }
    }

    #[test]
    fn points_on_and_next_to_the_plane_are_not_culled() {
        // Facet on the line y = x / 3 in 2D, with points on it and 1 ulp off.
        let facet = vec![vec![0.0, 0.0], vec![3.0, 1.0]];
        let mut points = Vec::new();
        for i in 1..200 {
            let x = i as f64 * 0.37;
            let y = x / 3.0;
            for y in [
                y,
                f64::from_bits(y.to_bits() + 1),
                f64::from_bits(y.to_bits() - 1),
            ] {
                points.extend_from_slice(&[x, y]);
            }
        }
        check_sound(&facet, &points);
    }

    #[test]
    fn huge_and_tiny_coordinates_are_sound() {
        for scale in [1e150, 1e-150, f64::MIN_POSITIVE, 1e300] {
            let facet = [
                vec![0.0, 0.0, 0.0],
                vec![scale, 0.0, 0.0],
                vec![0.0, scale, 0.0],
            ];
            let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
            let Some(normal) = unit_normal(&refs, Sign::Positive).unwrap() else {
                continue;
            };
            let Some(plane) = CullPlane::new(&refs, &normal, Sign::Positive) else {
                continue;
            };
            let mut rng = Rng(9);
            let points: Vec<f64> = (0..300).map(|_| rng.unit() * scale).collect();
            let indices: Vec<u32> = (0..100).collect();
            let mut inside = vec![false; 100];
            plane.mark_inside(&points, plane.origin.len(), &indices, &mut inside);
            for (i, &culled) in inside.iter().enumerate() {
                if culled {
                    let sign = distance_sign(&refs, &points[i * 3..i * 3 + 3]).unwrap();
                    assert_eq!(sign, Sign::Negative, "scale {scale}, point {i}");
                }
            }
        }
    }

    /// Culls `points` against `facet` with the given working `normal`, and
    /// checks that no point outside or on the exact plane is culled. Returns
    /// the cull flags.
    fn cull_with(facet: &[Vec<f64>], normal: &[f64], points: &[Vec<f64>]) -> Vec<bool> {
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        let outward = crate::predicates::orient_direction(&refs, normal).unwrap();
        let plane = CullPlane::new(&refs, normal, outward).expect("certified");
        let flat: Vec<f64> = points.iter().flatten().copied().collect();
        let indices: Vec<u32> = (0..points.len() as u32).collect();
        let mut inside = vec![false; points.len()];
        plane.mark_inside(&flat, plane.origin.len(), &indices, &mut inside);
        for (p, &culled) in points.iter().zip(&inside) {
            if culled {
                assert_eq!(
                    distance_sign(&refs, p).unwrap(),
                    outward.reversed(),
                    "culled {p:?}"
                );
            }
        }
        inside
    }

    #[test]
    fn a_tilted_working_normal_keeps_outside_points() {
        // Review of #34: the facet y = 0 with outward (0, 1), scanned with a
        // working normal tilted by 0.1 rad. The outside point (-1, 0.01) has
        // a negative working distance; tau must keep it.
        let facet = [vec![0.0, 0.0], vec![1.0, 0.0]];
        let normal = [0.1_f64.sin(), 0.1_f64.cos()];
        let points = [vec![-1.0, 0.01], vec![0.5, -10.0]];
        let inside = cull_with(&facet, &normal, &points);
        assert_eq!(inside, [false, true]);
    }

    #[test]
    fn a_45_degree_working_normal_keeps_outside_points() {
        // Review of #34: the facet of #33 with the 45-degree-off normal
        // (-1, 0, 0) that QR alone returned. (2, 4, 3) is outside the exact
        // plane and must be kept. With tau near 0.77 the slope exceeds 1, and
        // |w| <= |x - o| <= l for every point, so nothing can be proved inside:
        // even a point far on the inner side is kept. That is sound, and the
        // checked normal of #33 never hands such a tilt to the cull.
        let t = 2f64.powi(-48);
        let facet = [
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 4.0],
            vec![1.0 + t, 2.0 + t, 4.0],
        ];
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        let outward = crate::predicates::orient_direction(&refs, &[-1.0, 0.0, 0.0]).unwrap();
        let outside = vec![2.0, 4.0, 3.0];
        assert_eq!(distance_sign(&refs, &outside).unwrap(), outward);
        let deep: Vec<f64> = [-100.0, 100.0]
            .iter()
            .map(|&k| vec![1.0 + k, 2.0 - k, 3.0])
            .find(|p| distance_sign(&refs, p).unwrap() == outward.reversed())
            .unwrap();
        let inside = cull_with(&facet, &[-1.0, 0.0, 0.0], &[outside, deep.clone()]);
        assert_eq!(inside, [false, false]);
        // With the checked normal the same deep point is culled.
        let normal = unit_normal(&refs, outward).unwrap().unwrap();
        let inside = cull_with(&facet, &normal, &[vec![2.0, 4.0, 3.0], deep]);
        assert_eq!(inside, [false, true]);
    }

    #[test]
    fn huge_horizontal_facets_keep_points_just_outside() {
        for s in [1e16, 1e50, 1e100, 1e200] {
            let facet = [vec![s, s, s], vec![2.0 * s, s, s], vec![s, 2.0 * s, s]];
            let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
            let up = f64::from_bits(s.to_bits() + 1);
            let above = vec![1.5 * s, 1.5 * s, up];
            let outward = distance_sign(&refs, &above).unwrap();
            let normal = unit_normal(&refs, outward).unwrap().unwrap();
            let below = vec![1.5 * s, 1.5 * s, 0.0];
            let inside = cull_with(&facet, &normal, &[above, below]);
            assert_eq!(inside, [false, true], "s = {s}");
        }
    }

    #[test]
    fn indices_select_points_in_any_order() {
        // Facet (0, 0) -> (1, 0): orientation is the sign of y, so y < 0 is
        // the inside side.
        let facet = vec![vec![0.0, 0.0], vec![1.0, 0.0]];
        let plane = prepare(&facet);
        let points = [0.5, -1.0, 0.5, 1.0, 0.5, -2.0, 0.5, 0.0, 0.2, -3.0];
        let indices = [4, 1, 0, 3, 2];
        let mut inside = [false; 5];
        plane.mark_inside(&points, plane.origin.len(), &indices, &mut inside);
        assert_eq!(inside, [true, false, true, false, true]);
    }

    /// `w` and `l` of every index.
    type Terms = (Vec<f64>, Vec<f64>);

    /// Writes `w` and `l` of every index through one instruction-set level.
    fn fill_terms<S: Simd>(
        simd: S,
        plane: &CullPlane,
        points: &[f64],
        indices: &[u32],
        w: &mut [f64],
        l: &mut [f64],
    ) {
        simd.vectorize(Scan {
            plane,
            points,
            stride: plane.origin.len(),
            indices,
            w,
            l,
        });
    }

    /// `w` and `l` of every index through one instruction-set level.
    fn terms_with<S: Simd>(simd: S, plane: &CullPlane, points: &[f64], indices: &[u32]) -> Terms {
        let mut w = vec![0.0; indices.len()];
        let mut l = vec![0.0; indices.len()];
        fill_terms(simd, plane, points, indices, &mut w, &mut l);
        (w, l)
    }

    /// The terms through every level this CPU runs, by name: scalar lanes,
    /// then the x86 levels from AVX2 (the P1-4 path) to AVX-512, or Neon on
    /// aarch64.
    fn every_level(
        plane: &CullPlane,
        points: &[f64],
        indices: &[u32],
    ) -> Vec<(&'static str, Terms)> {
        let mut out = vec![("scalar", terms_with(pulp::Scalar, plane, points, indices))];
        #[cfg(target_arch = "x86_64")]
        {
            if let Some(simd) = pulp::x86::V3::try_new() {
                out.push(("x86-v3", terms_with(simd, plane, points, indices)));
            }
            if let Some(simd) = pulp::x86::V4::try_new() {
                out.push(("x86-v4", terms_with(simd, plane, points, indices)));
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            if let Some(simd) = pulp::aarch64::Neon::try_new() {
                out.push(("neon", terms_with(simd, plane, points, indices)));
            }
        }
        out
    }

    #[test]
    fn every_instruction_set_level_returns_the_same_terms_and_cull_set() {
        // Lane counts 1, 2, 4, and 8 with every tail length (0..=17 covers
        // every remainder of 8 twice), two longer runs, magnitudes far from
        // 1, and indices out of order.
        let mut rng = Rng(11);
        for d in 1..=8 {
            for count in (0..=17).chain([64, 101]) {
                for scale in [1.0, 1e-100, 1e100] {
                    let facet: Vec<Vec<f64>> = (0..d)
                        .map(|_| (0..d).map(|_| rng.unit() * scale).collect())
                        .collect();
                    let plane = prepare(&facet);
                    let points: Vec<f64> = (0..d * count).map(|_| rng.unit() * scale).collect();
                    let indices: Vec<u32> = (0..count as u32).rev().collect();
                    let reference: Vec<(u64, u64)> = indices
                        .iter()
                        .map(|&i| {
                            let start = i as usize * d;
                            let (w, l) = plane.scalar_terms(&points[start..start + d]);
                            (w.to_bits(), l.to_bits())
                        })
                        .collect();
                    for (name, (w, l)) in every_level(&plane, &points, &indices) {
                        let bits: Vec<(u64, u64)> = w
                            .iter()
                            .zip(&l)
                            .map(|(w, l)| (w.to_bits(), l.to_bits()))
                            .collect();
                        assert_eq!(bits, reference, "{name}, D = {d}, {count} points");
                    }
                    let mut dispatched = vec![false; count];
                    let mut scalar = vec![false; count];
                    plane.mark_inside(&points, plane.origin.len(), &indices, &mut dispatched);
                    plane.mark_inside_scalar(&points, plane.origin.len(), &indices, &mut scalar);
                    assert_eq!(dispatched, scalar, "cull set, D = {d}, {count} points");
                }
            }
        }
    }

    /// Before/after timing of the cull on the P2-7 sets, per level. Run by
    /// hand: `cargo test --release --lib cull_timing -- --ignored
    /// --nocapture`. The facet is each set's first D points; the cost per
    /// point does not depend on the plane.
    #[test]
    #[ignore = "measurement, run by hand in release"]
    fn cull_timing() {
        use std::time::Instant;
        let sets =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/benches/sets.txt"))
                .unwrap();
        for line in sets.lines().filter(|l| l.starts_with("set ")) {
            let field: Vec<&str> = line.split_whitespace().collect();
            let family = super::generator::Family::from_name(field[1]).unwrap();
            let dim: usize = field[2].parse().unwrap();
            let count: usize = field[3].parse().unwrap();
            let seed: u64 = field[4].parse().unwrap();
            let points = family.points(dim, count, seed);
            let facet: Vec<Vec<f64>> = points
                .chunks_exact(dim)
                .take(dim)
                .map(<[f64]>::to_vec)
                .collect();
            let plane = prepare(&facet);
            let indices: Vec<u32> = (0..count as u32).collect();
            let mut row = format!("{} d{dim} n{count}:", field[1]);
            let reps = (2_000_000 / count).clamp(3, 50);
            let mut w = vec![0.0; count];
            let mut l = vec![0.0; count];
            for (name, _) in every_level(&plane, &points, &indices[..1]) {
                // Only the level's kernel is timed: the buffers are reused,
                // and the cull test after it is the same scalar loop on
                // every level.
                let mut best = f64::MAX;
                for _ in 0..reps {
                    let start = Instant::now();
                    match name {
                        "scalar" => {
                            fill_terms(pulp::Scalar, &plane, &points, &indices, &mut w, &mut l)
                        }
                        #[cfg(target_arch = "x86_64")]
                        "x86-v3" => fill_terms(
                            pulp::x86::V3::try_new().unwrap(),
                            &plane,
                            &points,
                            &indices,
                            &mut w,
                            &mut l,
                        ),
                        #[cfg(target_arch = "x86_64")]
                        "x86-v4" => fill_terms(
                            pulp::x86::V4::try_new().unwrap(),
                            &plane,
                            &points,
                            &indices,
                            &mut w,
                            &mut l,
                        ),
                        #[cfg(target_arch = "aarch64")]
                        "neon" => fill_terms(
                            pulp::aarch64::Neon::try_new().unwrap(),
                            &plane,
                            &points,
                            &indices,
                            &mut w,
                            &mut l,
                        ),
                        _ => unreachable!(),
                    }
                    std::hint::black_box((&w, &l));
                    best = best.min(start.elapsed().as_secs_f64());
                }
                let culled = w
                    .iter()
                    .zip(&l)
                    .filter(|&(&w, &l)| plane.is_proved_inside(w, l, 0.0))
                    .count();
                std::hint::black_box(culled);
                row += &format!(" {name} {:.2} ns/point;", best / count as f64 * 1e9);
            }
            println!("{row}");
        }
    }

    /// The side of each point that `plane` proves, checked against the
    /// exact side relative to `outward`: a proved sign must be that side,
    /// and a point on the exact plane is never proved. Returns how many
    /// points were proved outside and inside.
    fn check_proved(
        facet: &[&[f64]],
        outward: Sign,
        plane: &CullPlane,
        points: &[Vec<f64>],
    ) -> (usize, usize) {
        let (mut outside, mut inside) = (0, 0);
        for p in points {
            let exact = distance_sign(facet, p).unwrap();
            let side = if exact == Sign::Zero {
                Sign::Zero
            } else if exact == outward {
                Sign::Positive
            } else {
                Sign::Negative
            };
            if let Some(proved) = plane.proved_side(p, 0.0) {
                assert_eq!(proved, side, "{p:?} proved on the wrong side");
                if proved == Sign::Positive {
                    outside += 1;
                } else {
                    inside += 1;
                }
            }
        }
        (outside, inside)
    }

    /// A lifted plane through the rounded points `facet` (heights exact
    /// except as `bounds` says) of the horizontal plane `h = 0`.
    fn lifted_plane(bounds: &[f64]) -> CullPlane {
        let facet = [
            vec![0.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
        ];
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        let normal = unit_normal(&refs, Sign::Positive).unwrap().unwrap();
        let cofactors = crate::normal::facet_cofactors(&refs).unwrap();
        let widened = crate::normal::lifted_facet_cofactors(&refs, &cofactors, bounds);
        CullPlane::with_lifted_cofactors(
            &refs,
            &normal,
            Sign::Positive,
            widened.as_deref(),
            bounds[0],
        )
        .expect("certified")
    }

    #[test]
    fn lifted_threshold_covers_the_height_bounds() {
        // The rounded points span h = 0. Each query's rounded height is off
        // by at most a bound under which the exact point may lie on the
        // exact plane, so no side may be proved. Dropping the query's bound
        // from the threshold, or the origin's from the floor, proves one.
        let delta = 1e-3;
        // The query's own height is uncertain.
        let exact_vertices = lifted_plane(&[0.0, 0.0, 0.0]);
        assert_eq!(
            exact_vertices.proved_side(&[0.25, 0.25, delta], delta),
            None
        );
        assert_eq!(
            exact_vertices.proved_side(&[0.25, 0.25, -delta], delta),
            None
        );
        // The origin's height is uncertain; the query sits right above it.
        let uncertain_origin = lifted_plane(&[delta, 0.0, 0.0]);
        assert_eq!(uncertain_origin.proved_side(&[0.0, 0.0, delta], 0.0), None);
        assert_eq!(uncertain_origin.proved_side(&[0.0, 0.0, -delta], 0.0), None);
        // Far beyond every bound, both sides are proved.
        assert_eq!(
            exact_vertices.proved_side(&[0.25, 0.25, 1.0], delta),
            Some(Sign::Positive)
        );
        assert_eq!(
            uncertain_origin.proved_side(&[0.25, 0.25, -1.0], 0.0),
            Some(Sign::Negative)
        );
    }

    #[test]
    fn proved_sides_are_the_exact_sides() {
        // Random facets and points in D = 1..=6 at several magnitudes, plus
        // points on the plane and one ulp off it: every proved side is the
        // exact one, and both directions are proved often.
        let mut rng = Rng(79);
        for d in 1..=6 {
            // 1e±45 keeps the cofactors of a D = 6 facet within f64, so the
            // huge and tiny magnitudes reach the assertions.
            for scale in [1.0, 1e-45, 1e45, 3.0] {
                let facet: Vec<Vec<f64>> = (0..d)
                    .map(|_| (0..d).map(|_| rng.unit() * scale).collect())
                    .collect();
                let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
                let normal = unit_normal(&refs, Sign::Positive)
                    .unwrap()
                    .unwrap_or_else(|| panic!("d = {d}, scale {scale}: no normal"));
                let plane = CullPlane::new(&refs, &normal, Sign::Positive)
                    .unwrap_or_else(|| panic!("d = {d}, scale {scale}: no certified plane"));
                let mut points: Vec<Vec<f64>> = (0..400)
                    .map(|_| (0..d).map(|_| rng.unit() * 4.0 * scale).collect())
                    .collect();
                // Points on the plane: affine combinations of the vertices,
                // and their neighbors one ulp away in the last coordinate.
                for _ in 0..50 {
                    let weights: Vec<f64> = (0..d).map(|_| rng.unit()).collect();
                    let total: f64 = weights.iter().sum();
                    let on: Vec<f64> = (0..d)
                        .map(|j| {
                            facet
                                .iter()
                                .zip(&weights)
                                .map(|(v, w)| v[j] * w / total)
                                .sum()
                        })
                        .collect();
                    for step in [0_i64, 1, -1] {
                        let mut q = on.clone();
                        let last = q[d - 1];
                        if last != 0.0 {
                            q[d - 1] = f64::from_bits((last.to_bits() as i64 + step) as u64);
                        }
                        points.push(q);
                    }
                }
                let (outside, inside) = check_proved(&refs, Sign::Positive, &plane, &points);
                assert!(
                    outside > 50 && inside > 50,
                    "d = {d}, scale {scale}: {outside} outside, {inside} inside proved"
                );
            }
        }
    }

    #[test]
    fn a_tilted_working_normal_proves_neither_side_wrongly() {
        // The facet y = 0 with outward (0, 1), scanned with a working normal
        // tilted by 0.1 rad (review of #34). (-1, 0.01) is outside and
        // (1, -0.01) inside, but the tilted distance has the opposite sign
        // for both; tau must keep either from being proved.
        let facet = [vec![0.0, 0.0], vec![1.0, 0.0]];
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        let normal = [0.1_f64.sin(), 0.1_f64.cos()];
        let outward = crate::predicates::orient_direction(&refs, &normal).unwrap();
        let plane = CullPlane::new(&refs, &normal, outward).expect("certified");
        let points = vec![
            vec![-1.0, 0.01],
            vec![1.0, -0.01],
            vec![0.5, 10.0],
            vec![0.5, -10.0],
        ];
        let (outside, inside) = check_proved(&refs, outward, &plane, &points);
        assert_eq!((outside, inside), (1, 1), "only the far points are proved");
    }

    #[test]
    fn a_45_degree_working_normal_proves_no_side() {
        // The facet of #33 with the 45-degree-off normal (-1, 0, 0) that QR
        // alone returned. tau near 0.77 makes the slope exceed 1, so no
        // distance can prove a side: the outside point (2, 4, 3) and a point
        // far on the inner side both go to the orientation.
        let t = 2f64.powi(-48);
        let facet = [
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 4.0],
            vec![1.0 + t, 2.0 + t, 4.0],
        ];
        let refs: Vec<&[f64]> = facet.iter().map(Vec::as_slice).collect();
        let normal = [-1.0, 0.0, 0.0];
        let outward = crate::predicates::orient_direction(&refs, &normal).unwrap();
        let plane = CullPlane::new(&refs, &normal, outward).expect("certified");
        let outside = vec![2.0, 4.0, 3.0];
        assert_eq!(distance_sign(&refs, &outside).unwrap(), outward);
        let deep: Vec<f64> = [-100.0, 100.0]
            .iter()
            .map(|&k| vec![1.0 + k, 2.0 - k, 3.0])
            .find(|p| distance_sign(&refs, p).unwrap() == outward.reversed())
            .unwrap();
        let far_outside: Vec<f64> = [-100.0, 100.0]
            .iter()
            .map(|&k| vec![1.0 + k, 2.0 - k, 3.0])
            .find(|p| distance_sign(&refs, p).unwrap() == outward)
            .unwrap();
        let points = vec![outside, deep, far_outside];
        assert_eq!(check_proved(&refs, outward, &plane, &points), (0, 0));
    }
}
