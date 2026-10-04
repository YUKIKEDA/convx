//! The convex hull: input acceptance, construction, classification, and the
//! published result.

pub(crate) mod classify;
mod error;
pub(crate) mod input;
#[cfg(any(test, debug_assertions))]
pub(crate) mod invariants;
pub(crate) mod merge;
pub(crate) mod publish;
mod ridge;
pub(crate) mod simplicial;

pub use error::ConvexHullError;
pub use publish::{
    BoundarySimplex, ConvexHull, ConvexHullBuilder, FacetPlane, LogicalFacet, TriangulationView,
};
