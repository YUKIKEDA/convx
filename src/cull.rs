//! Distance scan that culls points proved strictly inside a facet.
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
//! sign. The scan never decides visibility or outsideness.
//!
//! # Why the test is sound
//!
//! Let `u*` be the exact outward unit normal and `tau >= |n - u*|`. For the
//! exact value `S = (x - o) . n`, `(x - o) . u* <= S + |x - o| tau`. The
//! rounding of `w` is at most `gamma_{d+1} sum |x_j - o_j| |n_j|`, plus
//! `d * 2^-1074` for products that underflow. With `|n_j| <= 1 + d u` and
//! `l >= (1 - gamma_d) sum |x_j - o_j|`, both terms are covered by
//! `slope = 4 (d + 1) u + 2 tau` and `floor = (d + 1) 2^-1073`; the final
//! factor covers the rounding of the right-hand side itself.
//!
//! `tau` is certified once per facet from the cofactor vector `c` of the
//! facet's edges (see [`crate::predicates::direction_cofactors`]): with
//! computed `c^` and `|c - c^| <= E`, `|c/|c| - c^/|c^|| <= 2E / |c^|`, and
//! `|n - c^/|c^||` is evaluated in `f64` with a margin for its own rounding.
//! When the cofactors cannot be certified, no point is culled.
//!
//! # Paths
//!
//! The vectorized path uses `pulp` runtime dispatch with one point per lane.
//! Each lane performs the same operations in the same order as the scalar
//! path (subtract, multiply, add, absolute value, add; never `mul_add`), so
//! both paths return bitwise identical `w` and `l` and the same cull set on
//! every CPU.

// The hull build (P2-1, #8) is the first caller outside tests.
#![cfg_attr(not(test), allow(dead_code))]

use pulp::{Arch, Simd, WithSimd};

use crate::normal::{binary_exponent, scale_by_power_of_two};
use crate::predicates::{direction_cofactors, Sign};

const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.0;
/// 2^-1073.
const ETA: f64 = f64::from_bits(2);

/// A facet prepared for culling.
pub(crate) struct CullPlane {
    origin: Vec<f64>,
    normal: Vec<f64>,
    slope: f64,
    floor: f64,
}

impl CullPlane {
    /// Prepares `facet` (D points of dimension D) with its working unit
    /// `normal`, which must satisfy `orient_direction(facet, normal) ==
    /// outward`. Returns `None` when the plane error cannot be certified; the
    /// caller then culls nothing for this facet.
    pub(crate) fn new(facet: &[&[f64]], normal: &[f64], outward: Sign) -> Option<Self> {
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
        let tau = plane_error(facet, normal, side)?;
        let n = d as f64;
        let slope = (4.0 * (n + 1.0) * UNIT_ROUNDOFF + 2.0 * tau) * (1.0 + 4.0 * UNIT_ROUNDOFF);
        let floor = (n + 1.0) * ETA;
        slope.is_finite().then(|| Self {
            origin: facet[0].to_vec(),
            normal: normal.to_vec(),
            slope,
            floor,
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

    fn is_proved_inside(&self, w: f64, l: f64) -> bool {
        let threshold = (self.slope * l + self.floor) * (1.0 + 4.0 * UNIT_ROUNDOFF);
        w < -threshold
    }

    /// Marks `inside[i] = true` for each `indices[i]` whose point in the
    /// row-major `points` is proved strictly inside. Other entries are set
    /// to `false`.
    pub(crate) fn mark_inside(&self, points: &[f64], indices: &[u32], inside: &mut [bool]) {
        let mut w = vec![0.0; indices.len()];
        let mut l = vec![0.0; indices.len()];
        Arch::new().dispatch(Scan {
            plane: self,
            points,
            indices,
            w: &mut w,
            l: &mut l,
        });
        for ((flag, &w), &l) in inside.iter_mut().zip(&w).zip(&l) {
            *flag = self.is_proved_inside(w, l);
        }
    }

    /// Reference path without SIMD. Same result as [`Self::mark_inside`].
    #[cfg(test)]
    fn mark_inside_scalar(&self, points: &[f64], indices: &[u32], inside: &mut [bool]) {
        let d = self.origin.len();
        for (flag, &index) in inside.iter_mut().zip(indices) {
            let start = index as usize * d;
            let (w, l) = self.scalar_terms(&points[start..start + d]);
            *flag = self.is_proved_inside(w, l);
        }
    }
}

/// Certified upper bound on `|normal - side * c / |c||`.
fn plane_error(facet: &[&[f64]], normal: &[f64], side: f64) -> Option<f64> {
    let d = facet.len() as f64;
    // Scaling every coordinate by one power of two scales the cofactors by a
    // positive factor and leaves their direction unchanged. Use it only when
    // it is exact for every coordinate, so the cofactors stay those of the
    // facet's own points.
    let scaled = exact_unit_scaling(facet);
    let cofactors = match &scaled {
        Some(points) => {
            let refs: Vec<&[f64]> = points.iter().map(Vec::as_slice).collect();
            direction_cofactors(&refs)?
        }
        None => direction_cofactors(facet)?,
    };
    let bound: f64 = cofactors.iter().map(|&(_, e)| e).sum::<f64>() * (1.0 + d * UNIT_ROUNDOFF);
    let length = cofactors.iter().map(|&(c, _)| c * c).sum::<f64>().sqrt();
    let length_low = length * (1.0 - (d + 3.0) * UNIT_ROUNDOFF);
    if !(length_low.is_finite() && length_low > 2.0 * bound) {
        return None;
    }
    let deviation = normal
        .iter()
        .zip(&cofactors)
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

/// The facet scaled by the power of two that brings its largest magnitude
/// into [1, 2), when that scaling is exact for every coordinate.
fn exact_unit_scaling(facet: &[&[f64]]) -> Option<Vec<Vec<f64>>> {
    let largest = facet
        .iter()
        .flat_map(|p| p.iter())
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    if largest == 0.0 {
        return None;
    }
    let shift = -binary_exponent(largest);
    let scaled: Vec<Vec<f64>> = facet
        .iter()
        .map(|p| p.iter().map(|&x| scale_by_power_of_two(x, shift)).collect())
        .collect();
    // Bit equality after the round trip is the test for an exact scaling; it
    // does not decide a geometric sign.
    let exact = facet.iter().zip(&scaled).all(|(p, q)| {
        p.iter()
            .zip(q)
            .all(|(&x, &y)| scale_by_power_of_two(y, -shift) == x)
    });
    exact.then_some(scaled)
}

/// One lane per point; the per-lane sequence matches [`CullPlane::scalar_terms`].
struct Scan<'a> {
    plane: &'a CullPlane,
    points: &'a [f64],
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
                    *slot = self.points[index as usize * d + j];
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
            let start = self.indices[i] as usize * d;
            let (w, l) = self.plane.scalar_terms(&self.points[start..start + d]);
            self.w[i] = w;
            self.l[i] = l;
        }
    }
}

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
        plane.mark_inside(points, &indices, &mut fast);
        plane.mark_inside_scalar(points, &indices, &mut slow);
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
            plane.mark_inside(&points, &indices, &mut inside);
            for (i, &culled) in inside.iter().enumerate() {
                if culled {
                    let sign = distance_sign(&refs, &points[i * 3..i * 3 + 3]).unwrap();
                    assert_eq!(sign, Sign::Negative, "scale {scale}, point {i}");
                }
            }
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
        plane.mark_inside(&points, &indices, &mut inside);
        assert_eq!(inside, [true, false, true, false, true]);
    }
}
