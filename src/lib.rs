//! N-dimensional convex hulls, Delaunay triangulations, and Voronoi diagrams
//! whose topology is decided by exact predicate signs.
//!
//! The specification is `docs/design.md` (canonical: `docs/design.ja.md`).

#![forbid(unsafe_code)]

mod arena;
mod cull;
mod normal;
mod predicates;

pub use predicates::Sign;
