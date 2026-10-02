//! The convex hull: input acceptance, construction, classification, and the
//! published result.

pub(crate) mod classify;
mod error;
pub(crate) mod input;
pub(crate) mod merge;
mod publish;
pub(crate) mod simplicial;

pub use error::ConvexHullError;
pub use publish::{
    BoundarySimplex, ConvexHull, ConvexHullBuilder, FacetPlane, LogicalFacet, TriangulationView,
};
