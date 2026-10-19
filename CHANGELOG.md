# Changelog

Every version of convx on crates.io, newest first, in the form of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). A 0.x minor version may break the public API; an entry says so when it does. The MSRV is raised only by a version that names it here.

## [0.1.1] - 2026-10-19

A faster Delaunay triangulation of D = 2. Nothing breaks 0.1.0: no public item changes, and every triangulation, hull, and diagram has the simplices, facets, neighbors, and counts that 0.1.0 returns for the same input.

### Changed

- **The Delaunay triangulation of D = 2** builds in 0.76 of 0.1.0's time (0.68 to 0.95 on the comparison's files) and with 0.61 of its peak memory (0.59 to 0.95). It inserts by edge flips on a structure of its own, and a triangulation whose cospherical sites form no group is written from that structure directly.
- **The order of the simplices of a D = 2 triangulation** is not 0.1.0's: the insertion order and the construction changed. The order is the same for the same input on one binary and was not promised across versions; the simplices and their neighbors are the same.
- **Fewer allocations** in every build: the rows of the sites are made once, and the lists of the insertion order are one.
- **The Delaunay triangulation of D = 3** skips its pass over the faces when its insertion found no face cospherical. On the comparison's files of D = 3 to 5 it reads 1.02 of 0.1.0's time (0.94 to 1.07), with CGAL and Qhull at 1.01 in the same run: unchanged.

### Measured

The comparison with Qhull, CGAL, and the Rust crates was taken again on this version's code: [docs/comparison.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.md). The Delaunay triangulation of D = 2 is now faster than CGAL's on every file of D = 2 but the lattice.

## [0.1.0] - 2026-10-17

The first release.

### Computes

- **Convex hulls** of points in any dimension `D >= 1`, from one row-major slice of `f64` (`ConvexHullBuilder`). A hull returns its vertices, its logical facets (every simplex of one exact hyperplane together) with their neighbors, the points on its boundary that are not vertices, its interior points, and the representative of every duplicate. The planes, the volume, the boundary as simplices (`triangulation()`), and the cycle of a facet in `D <= 3` are computed from the decided topology.
- **Delaunay triangulations** of sites in any dimension (`DelaunayBuilder`): every site a vertex, the neighbor across each face, sites on one circle or sphere split by a rule of their indices alone.
- **Voronoi diagrams** (`VoronoiBuilder`): vertices, rays, cells, and the pairs of cells that meet. A vertex is one group of cospherical Delaunay simplices, merged by exact signs. In `D >= 4` the diagram is these lists, not a complete cell complex.
- **Arrays instead of a slice**: `StaticConvexHull`, `StaticDelaunay`, and `StaticVoronoi` take `&[[f64; D]]` for `1 <= D <= 8`.

### Promises

- Every topological decision is the exact sign of a predicate on the given `f64` values: no tolerance, no perturbation, no merging of nearly equal points.
- The hull returned for an input does not depend on the order of construction, and one binary returns the same hull, triangulation, and diagram for the same input on every run.
- A build reports one `Error` in a fixed order of checks (the dimension, the length of the slice, a coordinate that is not finite, too many points, too few distinct points, points that do not span the dimension). `Error` is `#[non_exhaustive]`.
- A build's allocations do not grow with the number of points, on the kinds of input the crate's allocation test builds.

### Requires

- Rust 1.89 or later, and `std`.
- `faer`, `pulp`, and `thiserror`.
- No `unsafe` in this crate.

### Measured

A comparison with Qhull 2020.2, CGAL 5.4, `spade`, `delaunator`, `delaunay`, `parry3d-f64`, and `geo` on rbox, the PBBS distributions, and four Stanford scans is in [docs/comparison.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.md), rows where another library is faster included.
