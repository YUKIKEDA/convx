//! One exact scaling of a whole input (design §1, #589).
//!
//! With every coordinate near `1e-300` the products of a determinant fall
//! into the subnormal range, where the filters' relative bounds do not
//! hold; near `1e300` they overflow. A build then decided most signs in the
//! exact stage, and the stages around the predicates worked on the same
//! coordinates: a Delaunay build of D = 3 took 15 times its time at either
//! end, and a hull of D = 2 took 15 to 21 times (`docs/bench.md`).
//!
//! Multiplying every coordinate by one power of two multiplies every
//! difference of two points by it, every orientation determinant and every
//! lifted one by a positive power of it, and so changes no sign: the hull,
//! the Delaunay triangulation, and the combinatorics of the Voronoi diagram
//! are those of the input. When no product is rounded, the scaled
//! coordinates are those reals exactly, so a build on them is a build of
//! the same exact problem. What a result publishes as coordinates is taken
//! from the input's own coordinates, not from the copy.

use crate::normal::{binary_exponent, scale_by_power_of_two};

/// From which binary exponent of the largest coordinate, on either side of
/// zero, the points of a hull are scaled. Inside it a hull is not slower:
/// its builds read the same from `2^-90` to `2^98`, and the fixed-size
/// planes of D = 3 and 4 take edges in `[2^-100, 2^100]`
/// ([`crate::normal::fixed_plane`]), which a coordinate near `2^-98`
/// already leaves.
pub(crate) const HULL_FROM: i32 = 64;

/// The same for the sites of a Delaunay triangulation or a Voronoi
/// diagram. The lifted entries are squares, and the first stage's bound
/// leaves its common form once the sum of its underflow term passes `2^60`
/// ([`crate::predicates`]): a Delaunay build of D = 3 read 1.08 of its time
/// with coordinates near `2^16`, 1.49 near `2^24`, and 1.46 near `2^30`.
pub(crate) const SITES_FROM: i32 = 12;

/// How many coordinates decide whether the input is looked at in full.
const SAMPLE: usize = 1024;

/// `points` times the power of two that brings the largest coordinate into
/// `[1, 2)`, when its binary exponent is beyond `from` on either side of
/// zero and that product is exact
/// for every one of them; `None` otherwise, and the build runs on `points`.
///
/// An ordinary input pays for a sample of its coordinates and nothing else:
/// whether to scale is a guess that changes no result, so it is made from
/// `SAMPLE` coordinates spread over the input. Only an input whose sample
/// is far from 1 is read in full, for its largest and its smallest
/// coordinate.
///
/// Scaled up, no product overflows, as the largest lands below 2, and a
/// subnormal coordinate only gains exponent. Scaled down, a product is
/// exact unless it lands below the normal range, which the smallest
/// nonzero coordinate decides. A coordinate that is not finite leaves the
/// input as it is, and acceptance reports it.
pub(crate) fn scaled_copy(points: &[f64], from: i32) -> Option<Vec<f64>> {
    let magnitude = |largest: f64, x: &f64| {
        let x = x.abs();
        if x > largest {
            x
        } else {
            largest
        }
    };
    let step = (points.len() / SAMPLE).max(1);
    let sampled = points.iter().step_by(step).fold(0.0_f64, magnitude);
    if !far_from_one(sampled, from) {
        return None;
    }
    if points.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let largest = points.iter().fold(0.0_f64, magnitude);
    if !far_from_one(largest, from) {
        return None;
    }
    let shift = -binary_exponent(largest);
    if shift < 0 {
        let smallest = points
            .iter()
            .filter(|&&x| x != 0.0)
            .fold(f64::INFINITY, |smallest, x| smallest.min(x.abs()));
        if binary_exponent(smallest) + shift < -1022 {
            return None;
        }
    }
    Some(
        points
            .iter()
            .map(|&x| scale_by_power_of_two(x, shift))
            .collect(),
    )
}

/// Whether `largest`, the largest magnitude of some coordinates, is finite,
/// nonzero, and has a binary exponent beyond `from` on either side of zero.
fn far_from_one(largest: f64, from: i32) -> bool {
    largest.is_finite() && largest != 0.0 && binary_exponent(largest).abs() > from
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two(e: i32) -> f64 {
        scale_by_power_of_two(1.0, e)
    }

    /// Ordinary coordinates are left alone, on both sides of the threshold's
    /// edge; beyond it the copy is the input times one power of two, exactly,
    /// with its largest magnitude in [1, 2).
    #[test]
    fn a_copy_is_the_input_times_one_power_of_two() {
        // The largest magnitude has the binary exponent 0.
        let base = [0.75, -0.5, 0.0, 0.3, -0.0, 0.123, 1.0 / 3.0, -1.999];
        for from in [HULL_FROM, SITES_FROM] {
            for e in [0, 1, from, -from] {
                let points: Vec<f64> = base.iter().map(|x| x * two(e)).collect();
                assert!(scaled_copy(&points, from).is_none(), "2^{e} from {from}");
            }
            for e in [from + 2, -from - 2, 300, -300, 1000, -1000, -1060] {
                let points: Vec<f64> = base.iter().map(|x| x * two(e)).collect();
                let copy = scaled_copy(&points, from).unwrap();
                let largest = copy.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
                assert!((1.0..2.0).contains(&largest), "2^{e}: {largest}");
                let shift = binary_exponent(copy[0]) - binary_exponent(points[0]);
                for (y, x) in copy.iter().zip(&points) {
                    // The same product, and nothing lost by it.
                    assert_eq!(y.to_bits(), scale_by_power_of_two(*x, shift).to_bits());
                    assert_eq!(scale_by_power_of_two(*y, -shift).to_bits(), x.to_bits());
                }
            }
        }
    }

    /// Subnormal coordinates scale up exactly. Huge coordinates beside one
    /// that a scale down would push below the normal range are left alone,
    /// and so is an input with a coordinate that is not finite, or with no
    /// nonzero coordinate.
    #[test]
    fn a_scale_that_would_round_is_not_taken() {
        let tiny = [
            f64::from_bits(1),
            f64::from_bits(3),
            -f64::from_bits(7),
            0.0,
        ];
        let copy = scaled_copy(&tiny, HULL_FROM).unwrap();
        assert_eq!(copy, [0.25, 0.75, -1.75, 0.0]);

        let huge = two(1000);
        // 2^-22 times 2^-1000 is the smallest normal number: still exact.
        assert!(scaled_copy(&[huge, two(-22)], HULL_FROM).is_some());
        // 1.5 * 2^-23 would land below the normal range and lose a bit.
        assert!(scaled_copy(&[huge, 1.5 * two(-23)], HULL_FROM).is_none());
        assert!(scaled_copy(&[huge, f64::from_bits(1)], HULL_FROM).is_none());

        assert!(scaled_copy(&[huge, f64::NAN], HULL_FROM).is_none());
        assert!(scaled_copy(&[huge, f64::INFINITY], HULL_FROM).is_none());
        assert!(scaled_copy(&[0.0, -0.0], HULL_FROM).is_none());
        assert!(scaled_copy(&[], HULL_FROM).is_none());
    }

    /// The sample decides only whether the input is read in full: an input
    /// whose sampled coordinates are ordinary is left alone, and one whose
    /// sample is far from 1 is scaled by its largest coordinate, sampled or
    /// not.
    #[test]
    fn the_sample_does_not_choose_the_scale() {
        let n = 4 * SAMPLE;
        let step = n / SAMPLE;
        // Far from 1 everywhere, and the largest between two samples.
        let mut points = vec![two(-900); n];
        points[step + 1] = two(-890);
        let copy = scaled_copy(&points, HULL_FROM).unwrap();
        assert_eq!(copy[step + 1], 1.0);
        assert_eq!(copy[0], two(-10));
        // Ordinary where sampled, with one huge coordinate between samples.
        let mut points = vec![0.5; n];
        points[step + 1] = two(900);
        assert!(scaled_copy(&points, HULL_FROM).is_none());
    }
}
