//! N-dimensional convex hulls, Delaunay triangulations, and Voronoi diagrams
//! whose topology is decided by exact predicate signs.
//!
//! The specification is `docs/design.md` (canonical: `docs/design.ja.md`).

#![forbid(unsafe_code)]

mod arena;
mod cull;
mod delaunay;
mod hull;
mod normal;
mod predicates;
mod static_api;
mod voronoi;

pub use delaunay::{DelaunayBuilder, DelaunaySimplex, DelaunayTriangulation};
pub use hull::{
    BoundarySimplex, ConvexHull, ConvexHullBuilder, ConvexHullError, FacetPlane, LogicalFacet,
    TriangulationView,
};
pub use predicates::Sign;
pub use static_api::{StaticConvexHull, StaticDelaunay, StaticVoronoi};
pub use voronoi::{
    VoronoiBuilder, VoronoiCell, VoronoiDiagram, VoronoiInterface, VoronoiRay, VoronoiVertex,
};

#[cfg(test)]
mod phase1;
