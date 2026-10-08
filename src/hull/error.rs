//! The error type shared by the hull, the Delaunay triangulation, and the
//! Voronoi diagram (design §3).

use crate::predicates::ExactEvaluationExhausted;

/// Why a build failed.
///
/// Input checks run in this order: [`NonPositiveDimension`](Self::NonPositiveDimension),
/// [`LengthMismatch`](Self::LengthMismatch), [`NonFiniteCoordinate`](Self::NonFiniteCoordinate),
/// [`TooManyPoints`](Self::TooManyPoints), then duplicate removal,
/// [`InsufficientPoints`](Self::InsufficientPoints), and
/// [`DegenerateDimension`](Self::DegenerateDimension).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConvexHullError {
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
    /// a finite `f64`. Returned by `ConvexHull::planes`, not by `build`:
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

impl From<ExactEvaluationExhausted> for ConvexHullError {
    fn from(_: ExactEvaluationExhausted) -> Self {
        Self::ExactEvaluationExhausted
    }
}
