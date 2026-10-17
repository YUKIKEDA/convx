//! The error type shared by the hull, the Delaunay triangulation, and the
//! Voronoi diagram (design §3).

use crate::predicates::ExactEvaluationExhausted;

/// Why a build, or a number asked of a result, failed.
///
/// Every fallible function of the crate returns this type: the builds of a
/// hull, of a Delaunay triangulation, and of a Voronoi diagram, and
/// [`ConvexHull::planes`](crate::ConvexHull::planes).
///
/// # The order of failures
///
/// A build checks its input in a fixed order and returns the first failure:
///
/// 1. [`NonPositiveDimension`](Self::NonPositiveDimension)
/// 2. [`LengthMismatch`](Self::LengthMismatch)
/// 3. [`NonFiniteCoordinate`](Self::NonFiniteCoordinate), naming the first
///    such point
/// 4. [`TooManyPoints`](Self::TooManyPoints)
/// 5. [`InsufficientPoints`](Self::InsufficientPoints), counted after
///    duplicates are merged
/// 6. [`DegenerateDimension`](Self::DegenerateDimension)
///
/// An input that passes these has a hull and a Delaunay triangulation. What
/// can still fail is a number in `f64` that is not used for any decision
/// ([`NonFiniteFacetPlane`](Self::NonFiniteFacetPlane),
/// [`NonFiniteCircumcenter`](Self::NonFiniteCircumcenter)) and the
/// allocation of the exact evaluation's work space
/// ([`ExactEvaluationExhausted`](Self::ExactEvaluationExhausted)).
///
/// The enum is `#[non_exhaustive]`: a later version may add a variant, so a
/// `match` on it needs a wildcard arm.
///
/// # Examples
///
/// ```
/// use convx::{ConvexHullBuilder, Error};
///
/// // The second point has a coordinate that is not finite.
/// let points = [0.0, 0.0, f64::NAN, 1.0, 1.0, 0.0];
/// match ConvexHullBuilder::new(2, &points).build() {
///     Err(Error::NonFiniteCoordinate { index }) => assert_eq!(index, 1),
///     Err(other) => panic!("another failure: {other}"),
///     Ok(_) => panic!("built"),
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The dimension is zero.
    #[error("the dimension must be at least 1")]
    NonPositiveDimension,
    /// The coordinate slice length is not a multiple of the dimension.
    #[error("{len} coordinates do not divide into points of dimension {dim}")]
    LengthMismatch {
        /// Length of the coordinate slice.
        len: usize,
        /// The requested dimension.
        dim: usize,
    },
    /// A point has a non-finite coordinate.
    #[error("point {index} has a non-finite coordinate")]
    NonFiniteCoordinate {
        /// Point number. The first point, in input order, that has a
        /// non-finite coordinate.
        index: usize,
    },
    /// More than `u32::MAX` points were given.
    #[error("{actual} points exceed the limit of u32::MAX")]
    TooManyPoints {
        /// The number of input points.
        actual: usize,
    },
    /// Fewer than D + 1 distinct points remain after duplicates are merged.
    #[error("{actual} distinct points; at least {required} are needed")]
    InsufficientPoints {
        /// The number of distinct points (representatives).
        actual: usize,
        /// D + 1.
        required: usize,
    },
    /// The distinct points span an affine subspace of dimension below D.
    #[error("the points span an affine subspace of dimension {actual_dim}")]
    DegenerateDimension {
        /// The affine dimension of the input.
        actual_dim: usize,
        /// Walking representatives in ascending order, the points that
        /// strictly raise the affine dimension. Length `actual_dim + 1`.
        spanning_points: Vec<u32>,
    },
    /// The hull topology was decided, and a public plane could not be made
    /// a finite `f64`. Returned by [`ConvexHull::planes`](crate::ConvexHull::planes), not by `build`:
    /// the hull is built, and the plane is not used for topology.
    #[error("a facet plane is not representable as finite f64")]
    NonFiniteFacetPlane,
    /// The Delaunay topology was decided, and a circumcenter could not be made
    /// a finite `f64`. This is not a geometric degeneracy. The hull and the
    /// Delaunay triangulation do not return this error.
    #[error("a Voronoi vertex is not representable as finite f64")]
    NonFiniteCircumcenter,
    /// The work space for exact evaluation could not be allocated. An
    /// ordinary `Vec` allocation failure aborts, as Rust does by default.
    #[error("the work space for exact evaluation could not be allocated")]
    ExactEvaluationExhausted,
}

impl From<ExactEvaluationExhausted> for Error {
    fn from(_: ExactEvaluationExhausted) -> Self {
        Self::ExactEvaluationExhausted
    }
}
