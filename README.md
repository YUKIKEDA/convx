English | [日本語](README.ja.md)

# convx

convx is a pure Rust library for n-dimensional convex hulls, Delaunay triangulations, and Voronoi diagrams. Correctness is ordered as the sign of a geometric predicate, the topological decision from that sign, and the mutation that follows the decision. Public results are compared as normalized logical facets: vertex sets and neighbor sets.

The crate is not in the tree yet. The API below is the shape fixed by the specification. Implementation order is P1 through P4 in [docs/roadmap.md](docs/roadmap.md): predicates, sequential Quickhull, then the static API, Delaunay, Voronoi, and oracles.

## Predicates return `Sign`

An input `f64` is the coordinate its bit pattern names. Predicates do not translate or scale first. A rounded transform moves the exact zero. Each predicate returns `Sign` (`Negative`, `Zero`, `Positive`).

Coplanar means the orientation is `Zero`. Cospherical means the lifted orientation is `Zero`. Which side of a facet a point is on is the orientation of an affinely independent set of that face and the query point. The $x \cdot n + \mathrm{offset}$ of a published facet's `normal()` and `offset()` is not used for that decision.

Error is only the rounding of evaluating that predicate. The filter carries an absolute bound for the additions, subtractions, and multiplications actually performed. If that bound does not include FMA rounding, the evaluation does not contract to an FMA. When the absolute value of the computed value exceeds the bound, that sign is kept. Otherwise the predicate falls back to the exact sign of the same polynomial. The same fallback applies when the computed value and the bound are both 0 and the expression is not identically 0. A value that underflowed to 0 is not decided to be zero. The exact algorithm is not fixed. The returned sign must match the sign of this polynomial. The only failure is `ExactEvaluationExhausted`, when the exact-evaluation work space cannot be allocated. An ordinary `Vec` allocation failure aborts, as Rust does by default. Ill-conditioning itself is not a failure.

The public API has no tolerance parameter. Coplanar means the exact sign of the distance is zero.

Geometric degree $k$ is one less than the number of argument points, and is independent of the hull dimension $D$. $k \le 4$ (up to five points) uses a dedicated formula. $k > 4$ uses a filtered floating-point determinant. The public unit normal of every facet, and the working normal used by distance scans, is the certified cofactor direction, oriented outward.

A SIMD distance scan culls points the error bound proves strictly inside, so they are not fed to the predicate. Visibility and outsideness are the strict sign of the orientation of the facet vertices and the point. The certified working distance may prove that sign first, inside or outside; zero and everything near the plane go to the orientation. Dimension is also decided by predicate signs. A new point with exact sign zero against the current basis does not extend the span.

## Input is a row-major coordinate slice

The core takes row-major `&[f64]`. `dim == 0` fails with `NonPositiveDimension` before any division or remainder. $D \ge 1$, with no upper bound on dimension. The caller pays for the longer exact evaluation in high dimensions. The number of points is at most `u32::MAX`. Above that, `TooManyPoints` fails before duplicate removal. Public indices stay `u32`. `u32::MAX` is a sentinel that cannot collide with a point index.

`points.len()` must be a multiple of $D$, or the call fails with `LengthMismatch`. One non-finite coordinate fails the whole input with `NonFiniteCoordinate { index }`. `index` is the point number of the first such point in input order.

After that the order is duplicate aggregation, the count check, the rank check, then construction. Duplicates use `==`. `-0.0` and `+0.0` are the same point. Equality is not `to_bits` and not `total_cmp`. An implementation that orders points before choosing a representative canonicalizes signed zero to `+0.0` first. The representative is the smallest input index. Fewer than $D + 1$ representatives fails with `InsufficientPoints { actual, required }`. At least $D + 1$ representatives whose affine dimension is still below $D$ fails with `DegenerateDimension { actual_dim, spanning_points }`. A caller that projects uses the returned `spanning_points`: the lexicographically minimum basis, built by walking representative indices in order and keeping only points that strictly raise the affine dimension.

## What a convex hull publishes

A successful build assigns every input index one destination. `representative` is a `Vec<u32>` of length n. A representative `i` has `representative[i] == i`. A duplicate points at its representative. `representative[representative[i]] == representative[i]`. The representatives split into three lists, each sorted, pairwise disjoint, and together equal to the representative set.

| List              | Contents                                                            |
| ----------------- | ------------------------------------------------------------------- |
| `vertices`        | Extreme points. Removing one changes the hull                       |
| `coplanar_points` | Boundary points that are not extreme. Indices only, no owning facet |
| `interior_points` | The interior                                                        |

`ConvexHull` owns the three lists. `representative` is attached to the hull, the Delaunay triangulation, and the Voronoi diagram. Nearby points that differ in bits and fail `==` are not snapped together.

Classification runs after outside points have been absorbed and coplanar adjacent simplices have been merged into one logical facet. The distance sign is the orientation of $D$ affinely independent points of that face and the query point. A strictly positive distance to any logical facet means outside, and a successful result contains no outside point. Strictly negative distance to every facet means interior. A point that construction dropped while every sign it tested was strictly negative is already known to be interior (the proof is in design §3). A strictly zero distance to some facet, with the rest negative or zero, means on the boundary. Whether that boundary point becomes a vertex or a `coplanar_points` entry is an orientation, on the input coordinates, of whether it lies outside that face's convex hull.

Insertion keeps a simplicial complex. After every point is inserted, only simplices that are exactly coplanar across a shared ridge merge into one logical facet. There is no merge mid-insertion. One supporting plane's connected boundary is one logical facet.

Results keep their lists in flat arrays and publish them through borrowed views; no item owns a `Vec`. `hull.facets()` yields one `Facet` per logical facet, with `vertices()` (the extreme points in ascending index order), `normal()` and `offset()` (outward unit normal and offset), and `neighbors()` (the ascending set of neighbor facet numbers). Facets are ordered by lexicographic vertex lists, and that order is the public numbering. A position in `neighbors()` does not correspond to a shared ridge. Ridge vertex lists are not public.

Only the public plane is built by translating to a representative, scaling uniformly by the coordinate width, forming an `f64` normal, normalizing, matching the sign to the exact orientation (inside is negative), and computing the offset in the translated frame before mapping it back. A non-finite normal or offset fails with `NonFiniteFacetPlane`. That failure means the topology was already decided and the public plane could not be a finite `f64`. The plane is not used for topology. The same binary produces the same plane. Bit-identical floats across versions are not promised.

`volume()` returns the polyhedron volume. It is not used for topology. A successful build does not promise a finite return value. For $D = 1$ the volume is $|x_{\max} - x_{\min}|$. `triangulation()` returns the boundary simplicial complex, not a decomposition of the interior into $D$-simplices. The split of a coplanar region need not be geometrically unique, and it may change across versions. Within one binary, the split does not depend on the order of insertion. `boundary_cycle` returns a cycle only for $D$ of 1, 2, or 3, and returns `None` for $D \ge 4$ and for a facet number that is out of range.

Under Quickhull, a facet is visible to a point when that point is strictly outside it by orientation. The sequential build inserts one point at a time and changes the hull in place. It is the only build: a parallel build in rounds did not beat it when measured, so it and its `parallel` flag were removed (`docs/adr/0004-sequential-hull-only.md`). The published hull does not depend on the order of insertion, so any construction that reaches the same hull returns the same result. Across versions, what the public result promises, after normalization, is that logical-facet vertex sets and neighbor sets agree. Identical output bytes across versions are not promised.

## Delaunay is the lower hull of the lift

The Delaunay triangulation in dimension $D$ is the lower convex hull of the lift, projected back to the original space.

$$
x_{D+1} = \|x\|^2
$$

The input array is not extended. The formula is the definition. The height is passed to the exact sign as the polynomial $\sum_i x_i^2$ in the input coordinates. A non-finite `f64` intermediate skips the filter and continues to the exact sign. That input is not rejected. Caching the lift must not change the observable simplices or signs. Insphere is this lifted orientation. Another algebraic expression is allowed only when its sign matches this definition.

The triangulation is built by incremental insertion: sites are added one at a time, in a deterministic order the implementation chooses, and each insertion replaces the simplices whose circumsphere strictly contains the new site. Outside the site hull, simplices with a vertex at infinity stand for its facets. Conflicts are decided by exact signs only. Why insertion rather than the hull of the lift: `docs/adr/0001-incremental-delaunay.md`.

Published simplices are only the projection of the lower hull. Vertices are stored in ascending order, and a swap of the last two points makes the orientation positive in the original space. An exact orientation of zero stays in ascending order. A missing neighbor slot is `u32::MAX`. Simplex order is the lexicographic order of the ascending vertex lists before orientation is fixed.

When the original sites span $\mathbb{R}^D$ and every point lies on one sphere, no insertion runs. The interior of the site hull is filled with a pulling triangulation. That procedure stays inside Delaunay. Failure to build a public plane does not fail this split. `DegenerateDimension` is returned only when the affine dimension of the original sites is below $D$. With sites that span $\mathbb{R}^D$, the lift fails to span $D+1$ dimensions only in that case. The public convex hull still fails when the affine dimension is below $D$.

When several diagonals exist, the build returns the split that version's insertion order chose. Which sites are extreme is unique, so the extreme set is promised. The same binary returns the same diagonals for the same input; agreement across versions is not promised. Dimension degeneracy is reported from the affine dimension of the input sites, using those sites' indices.

Delaunay and Voronoi sites are the full representative set. Interior points and non-vertex boundary points are sites.

## Voronoi is the dual before diagonals

The Voronoi diagram is the dual of the Delaunay complex before diagonals are inserted. The published `DelaunayTriangulation` is that complex cut into simplices. There is no second algorithm. Degeneracy, duplicates, and cospherical points follow the hull and Delaunay rules. Voronoi adds none of its own.

A finite Voronoi vertex is one lower logical facet of the lifted hull. Simplices that are exactly coplanar across a shared ridge, by the lifted orientation, are merged by walking neighbors. Closeness of the `f64` circumcenter is not the merge test. A flat lift has one lower facet and therefore one Voronoi vertex. The vertex is not split per pulling-triangulation simplex. If no finite circumcenter can be formed, the build fails with `NonFiniteCircumcenter`. That error is not a geometric degeneracy. Delaunay simplices and neighbors are published as they were before the merge. Every vertex coordinate of a successful diagram is finite. The hull and the Delaunay triangulation do not compute circumcenters. Circumcenter rounding is not used to decide topology.

A `VoronoiInterface` exists only when two cells meet in dimension $D-1$. An interface with an empty `vertices` list is not built. When two distinct Voronoi vertices both contain sites $a$ and $b$, and the edge $ab$ is a face of both cells, the interface is bounded by finite vertices. When only one vertex contains $a$ and $b$, and the edge $ab$ lies on a logical facet of the site hull, the interface carries a ray. A diagonal interior to one vertex's site set is not an interface. An interface that is only rays is not built.

There is one ray per pair of a merged Voronoi vertex and a logical facet of the site hull in the original space. The direction is that facet's outward unit normal. An interior site's cell has no ray. A cell of a site on the hull boundary has at least one ray. For $D \ge 4$, vertex and ray pairs record incidence. They are not a complete cell complex.

## Calling it

`ConvexHullBuilder` takes a dimension and a point slice and returns the hull from `build`. Construction is sequential. `DelaunayBuilder` and `VoronoiBuilder` have the same shape. Both fail with `ConvexHullError`. Their `dim` is the original dimension, not the lifted one.

```rust
let hull = ConvexHullBuilder::new(2, points).build()?;
```

`points` is row-major and its length is a multiple of the dimension. The static API is a wrapper for stack arrays and monomorphization. It is a separate axis from swapping a solver. `StaticConvexHull` covers $1 \le D \le 8$. `StaticDelaunay` and `StaticVoronoi` cover $1 \le D \le 8$ too. They return `ConvexHull`, `DelaunayTriangulation`, and `VoronoiDiagram`. `build` takes `&[[f64; D]]` and passes `as_flattened()` to the core. That is the only conversion from `[[f64; D]]` to `&[f64]`. There is no `unsafe`.

Dependencies are `faer`, `pulp`, and `thiserror`. The implementation language is Rust. The build assumes `std`. The MSRV is 1.89. The distance kernel is runtime CPU detection through `pulp`, up to AVX-512.

The sign convention and the lift-as-formula are part of the Delaunay specification from the start.

## Verification

Debug builds and CI check the Euler characteristic of the simplicial complex and of the logical-facet complex, that no input point has a strictly positive distance to any logical facet, neighbor symmetry, and that `representative`'s fixed points match the three lists. Precomputed oracles live in `tests/fixtures/`. Vertex sets match exactly. Logical facets match exactly as normalized vertex sets. There is no global absolute tolerance on volume. Each fixture carries its own relative tolerance. A near-degenerate fixture whose denominator collapses is checked for topology only, not volume. Containment is a predicate.

Construction is sequential only, so on the same binary and input the published hull and the Delaunay simplices agree on every run.

Speedup numbers are written after the sequential core exists and Qhull has been measured. The reference Qhull run has no joggling and no input perturbation, in the same dimension. The comparison is logical facets, not a triangulated dump and not a joggled dump. The columns are performance, correctness, robustness, and memory. A parallel column returns with a parallel construction.

Formula expansions, the outside-of-ridge test, and the per-phase acceptance inputs are in [docs/design.md](docs/design.md) and [docs/design.ja.md](docs/design.ja.md). When the two differ, follow the Japanese file. Development procedure: [CONTRIBUTING.md](CONTRIBUTING.md). Agent entry: [AGENTS.md](AGENTS.md).
