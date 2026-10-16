# Changelog

Every version of convx on crates.io, newest first. A 0.x minor version may break the public API; an entry says so when it does. The MSRV is raised only by a version that names it here.

## 0.1.0

The first release.

### Computes

- **Convex hulls** of points in any dimension `D >= 1`, from one row-major slice of `f64` (`ConvexHullBuilder`). A hull publishes its vertices, its logical facets (every simplex of one exact hyperplane together) with their neighbors, the points on its boundary that are not vertices, its interior points, and the representative of every duplicate. The planes, the volume, the boundary as simplices (`triangulation()`), and the cycle of a facet in `D <= 3` are computed from the decided topology.
- **Delaunay triangulations** of sites in any dimension (`DelaunayBuilder`): every site in a simplex, the neighbor across each face, sites on one circle or sphere split by a rule of their indices alone.
- **Voronoi diagrams** (`VoronoiBuilder`): vertices, rays, cells, and the pairs of cells that meet. A vertex is one group of cospherical Delaunay simplices, merged by exact signs. In `D >= 4` the diagram is these lists, not a complete cell complex.
- **Arrays instead of a slice**: `StaticConvexHull`, `StaticDelaunay`, and `StaticVoronoi` take `&[[f64; D]]` for `1 <= D <= 8`.

### Promises

- Every topological decision is the exact sign of a predicate on the given `f64` values: no tolerance, no perturbation, no merging of nearly equal points.
- The hull and the Delaunay triangulation published for an input do not depend on the order of construction.
- A build reports one `Error` in a fixed order of checks (the dimension, the length of the slice, a coordinate that is not finite, too many points, too few distinct points, points that do not span the dimension). `Error` is `#[non_exhaustive]`.
- A build's allocations do not grow with the number of points, on the kinds of input the crate's allocation test builds.

### Requires

- Rust 1.89 or later, and `std`.
- `faer`, `pulp`, and `thiserror`.
- No `unsafe` in this crate.

### Measured

A comparison with Qhull 2020.2, CGAL 5.4, `spade`, `delaunator`, `delaunay`, `parry3d-f64`, and `geo` on rbox, the PBBS distributions, and four Stanford scans is in [docs/comparison.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.md), rows where another library is faster included.
