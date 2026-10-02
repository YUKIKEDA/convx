//! N-dimensional convex hulls, Delaunay triangulations, and Voronoi diagrams
//! whose topology is decided by exact predicate signs.
//!
//! The specification is `docs/design.md` (canonical: `docs/design.ja.md`).

#![forbid(unsafe_code)]

mod arena;
mod cull;
mod hull;
mod normal;
mod predicates;

pub use hull::{
    BoundarySimplex, ConvexHull, ConvexHullBuilder, ConvexHullError, FacetPlane, LogicalFacet,
    TriangulationView,
};
pub use predicates::Sign;

#[cfg(test)]
mod phase1;
