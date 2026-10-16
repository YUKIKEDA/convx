// The crate's documentation is the README: what is computed, an example of
// each result, what is promised and what is not, and the limits. Its
// examples run as doctests.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod arena;
mod cull;
mod delaunay;
mod error;
mod hull;
mod lists;
mod morton;
mod normal;
mod predicates;
mod scaling;
mod small;
mod static_api;
mod voronoi;

pub use delaunay::{DelaunayBuilder, DelaunaySimplex, DelaunayTriangulation, Simplices};
pub use error::Error;
pub use hull::{
    BoundarySimplex, ConvexHull, ConvexHullBuilder, Facet, Facets, Plane, Planes, TriangulationView,
};
pub use static_api::{StaticConvexHull, StaticDelaunay, StaticVoronoi};
pub use voronoi::{
    VoronoiBuilder, VoronoiCell, VoronoiCells, VoronoiDiagram, VoronoiInterface, VoronoiInterfaces,
    VoronoiRay, VoronoiRays, VoronoiVertex, VoronoiVertices,
};

/// The examples of the Japanese README, run as doctests like those of the
/// English one, which is the crate's documentation.
#[cfg(doctest)]
#[doc = include_str!("../README.ja.md")]
struct JapaneseReadme;

#[cfg(test)]
mod phase1;
