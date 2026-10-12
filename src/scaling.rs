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
