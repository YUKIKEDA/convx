English | [日本語](https://github.com/YUKIKEDA/convx/blob/main/README.ja.md)

# convx

Convex hulls, Delaunay triangulations, and Voronoi diagrams in any dimension, in pure Rust, with every topological decision made by the exact sign of a predicate.

- **Exact.** Which side of a facet a point is on, and whether points are coplanar or cospherical, is the sign of a determinant of the input coordinates, evaluated exactly when floating-point arithmetic cannot certify it. There is no tolerance and no perturbation of the input.
- **Any dimension.** One implementation for every `D >= 1`, with dedicated paths for `D = 2` and `D = 3`.
- **Degenerate input is input.** Duplicate points, points on a facet, cocircular and cospherical sites, and lattices are handled by the same rules as points in general position, and the result says what each input point became.
- **No `unsafe`** in this crate.

```toml
[dependencies]
convx = "0.1"
```

## Before the first call

- **Points are one row-major slice of `f64`.** With dimension `dim`, point `i` is `points[i * dim..(i + 1) * dim]`. `StaticConvexHull`, `StaticDelaunay`, and `StaticVoronoi` take `&[[f64; D]]` instead, for `1 <= D <= 8`.
- **A point is named by its index in the input, as a `u32`.** A duplicate is named by the smallest index of a point equal to it.
- **A facet is a logical facet.** The simplices of the boundary that lie exactly on one hyperplane are one facet: a cube has six facets of four vertices, not twelve triangles.
- **Every fallible call returns `convx::Error`.** A build checks its input in a fixed order and returns the first failure.
- **The version is 0.1.0.** A 0.x minor version may break the public API.

## A first example

```rust
use convx::ConvexHullBuilder;

fn main() -> Result<(), convx::Error> {
    // A unit square, its center, the midpoint of an edge, and a copy of a corner.
    let points = [
        0.0, 0.0, // 0
        1.0, 0.0, // 1
        1.0, 1.0, // 2
        0.0, 1.0, // 3
        0.5, 0.5, // 4: inside
        0.5, 0.0, // 5: on an edge
        0.0, 0.0, // 6: the point 0 again
    ];
    let hull = ConvexHullBuilder::new(2, &points).build()?;

    // Every input point is one of these, or a duplicate of one.
    assert_eq!(hull.vertices(), [0, 1, 2, 3]);
    assert_eq!(hull.coplanar_points(), [5]);
    assert_eq!(hull.interior_points(), [4]);
    assert_eq!(hull.representative()[6], 0);
    Ok(())
}
```

## Usage

### Convex hulls

`ConvexHullBuilder` builds a `ConvexHull`. Its facets are read through `Facets` and `Facet`, their planes through `Planes` and `Plane`, and the boundary as simplices through `TriangulationView` and `BoundarySimplex`.

```rust
use convx::{BoundarySimplex, ConvexHull, ConvexHullBuilder, Facet, Facets, Plane, Planes, TriangulationView};

fn main() -> Result<(), convx::Error> {
    // A square pyramid: a base of four corners and an apex.
    let points = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, // 0 to 3: the base
        0.5, 0.5, 1.0, // 4: the apex
    ];
    let hull: ConvexHull = ConvexHullBuilder::new(3, &points).build()?;
    assert_eq!(hull.dim(), 3);

    // Five facets: the base, one facet of four vertices, and four triangles.
    let facets: Facets = hull.facets();
    assert_eq!(facets.len(), 5);
    let base: Facet = facets.get(0).unwrap();
    assert_eq!(base.vertices(), [0, 1, 2, 3]);
    assert_eq!(base.neighbors(), [1, 2, 3, 4]);
    // The base's vertices in order around it, counterclockwise seen from outside.
    assert_eq!(hull.boundary_cycle(0), Some(vec![0, 2, 3, 1]));

    // The planes are computed in `f64` by the first call, and kept.
    let planes: Planes = hull.planes()?;
    let floor: Plane = planes.get(0).unwrap();
    assert_eq!(floor.normal(), [0.0, 0.0, -1.0]);
    assert_eq!(floor.offset(), 0.0);

    // The boundary as triangles: the base is split in two.
    let triangles: TriangulationView = hull.triangulation();
    assert_eq!(triangles.len(), 6);
    let first: BoundarySimplex = triangles.get(0).unwrap();
    assert_eq!(first.facet(), 0);
    assert_eq!(first.vertices().len(), 3);

    // Volume in `f64`: a third of the base times the height.
    assert!((hull.volume() - 1.0 / 3.0).abs() < 1e-15);
    Ok(())
}
```

### Delaunay triangulations

`DelaunayBuilder` builds a `DelaunayTriangulation`. Its simplices are read through `Simplices` and `DelaunaySimplex`.

```rust
use convx::{DelaunayBuilder, DelaunaySimplex, DelaunayTriangulation, Simplices};

fn main() -> Result<(), convx::Error> {
    // The corners of a square and its center.
    let sites = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    let delaunay: DelaunayTriangulation = DelaunayBuilder::new(2, &sites).build()?;
    assert_eq!(delaunay.dim(), 2);
    assert_eq!(delaunay.representative(), [0, 1, 2, 3, 4]);

    // Four triangles, each with the center as a vertex.
    let simplices: Simplices = delaunay.simplices();
    assert_eq!(simplices.len(), 4);
    for simplex in simplices.iter() {
        let simplex: DelaunaySimplex = simplex;
        assert!(simplex.vertices().contains(&4));
        // The simplex across each face, or `u32::MAX` on the hull of the sites.
        assert_eq!(simplex.neighbors().len(), 3);
    }
    Ok(())
}
```

Every site is in the triangulation, the ones inside the hull of the sites included. Sites on one circle or sphere are found by exact signs, grouped, and split by a rule of their indices alone.

### Voronoi diagrams

`VoronoiBuilder` builds a `VoronoiDiagram`: finite vertices (`VoronoiVertices`, `VoronoiVertex`), rays (`VoronoiRays`, `VoronoiRay`), one cell per site (`VoronoiCells`, `VoronoiCell`), and the faces where two cells meet (`VoronoiInterfaces`, `VoronoiInterface`).

```rust
use convx::{
    VoronoiBuilder, VoronoiCell, VoronoiCells, VoronoiDiagram, VoronoiInterface,
    VoronoiInterfaces, VoronoiRay, VoronoiRays, VoronoiVertex, VoronoiVertices,
};

fn main() -> Result<(), convx::Error> {
    // The four corners of a square are on one circle: one vertex at its center.
    let sites = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
    let voronoi: VoronoiDiagram = VoronoiBuilder::new(2, &sites).build()?;
    assert_eq!(voronoi.dim(), 2);
    assert_eq!(voronoi.representative(), [0, 1, 2, 3]);

    let vertices: VoronoiVertices = voronoi.vertices();
    let center: VoronoiVertex = vertices.get(0).unwrap();
    assert_eq!(center.sites(), [0, 1, 2, 3]);
    assert!(center.coords().iter().all(|x| (x - 1.0).abs() < 1e-12));

    // A ray leaves the center through each side of the square.
    let rays: VoronoiRays = voronoi.rays();
    let down: VoronoiRay = rays.get(0).unwrap();
    assert_eq!(down.apex(), 0);
    assert_eq!(down.hull_facet(), [0, 1]);
    assert_eq!(down.direction(), [0.0, -1.0]);

    // Every cell is unbounded: one vertex and two rays.
    let cells: VoronoiCells = voronoi.cells();
    let corner: VoronoiCell = cells.get(0).unwrap();
    assert_eq!(corner.site(), 0);
    assert_eq!(corner.vertices(), [0]);
    assert_eq!(corner.ray_numbers().len(), 2);
    assert_eq!(corner.rays().count(), 2);

    // Neighboring corners meet; opposite ones do not.
    let interfaces: VoronoiInterfaces = voronoi.interfaces();
    let pairs: Vec<[u32; 2]> = interfaces.iter().map(|face: VoronoiInterface| face.sites()).collect();
    assert_eq!(pairs, [[0, 1], [0, 3], [1, 2], [2, 3]]);
    let bottom: VoronoiInterface = interfaces.get(0).unwrap();
    assert_eq!(bottom.vertices(), [0]);
    assert_eq!(bottom.ray_numbers().len(), 1);
    assert_eq!(bottom.rays().count(), 1);
    Ok(())
}
```

A Voronoi vertex is one group of cospherical Delaunay simplices, grouped by exact signs and not by the closeness of circumcenters.

### Arrays instead of a slice

`StaticConvexHull`, `StaticDelaunay`, and `StaticVoronoi` take `&[[f64; D]]` for `1 <= D <= 8` and return the same results as the builders. Another `D` does not compile.

```rust
use convx::{StaticConvexHull, StaticDelaunay, StaticVoronoi};

fn main() -> Result<(), convx::Error> {
    let tetrahedron = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(StaticConvexHull::<3>::build(&tetrahedron)?.facets().len(), 4);
    assert_eq!(StaticDelaunay::<3>::build(&tetrahedron)?.simplices().len(), 1);
    assert_eq!(StaticVoronoi::<3>::build(&tetrahedron)?.vertices().len(), 1);
    Ok(())
}
```

### Errors

Every build returns `convx::Error` on failure, and so does `planes()`. The enum is non-exhaustive: a `match` on it has a wildcard arm.

```rust
use convx::{ConvexHullBuilder, Error};

// Three points on one line do not span the plane.
let points = [0.0, 0.0, 1.0, 1.0, 2.0, 2.0];
match ConvexHullBuilder::new(2, &points).build() {
    Err(Error::DegenerateDimension { actual_dim, spanning_points }) => {
        // The points span a line, and points 0 and 1 span it.
        assert_eq!(actual_dim, 1);
        assert_eq!(spanning_points, [0, 1]);
    }
    Err(other) => panic!("another failure: {other}"),
    Ok(_) => panic!("built"),
}
```

## What is promised

- **The combinatorial result is that of exact arithmetic on the given `f64` values**: the vertices, the facets and their neighbors, the classification of every point, the Delaunay simplices, and the Voronoi incidences. An input coordinate is the number its bits name.
- **Every input index has one destination.** `representative()` maps a duplicate to the smallest index of an equal point (`-0.0` equals `+0.0`); the representatives are the vertices, the points on the boundary that are not vertices, and the interior points, each list ascending.
- **The hull published does not depend on the order of construction**, and the same binary gives the same hull, triangulation, and diagram for the same input on every run.
- **Failures come in a fixed order**: the dimension, the length of the slice, a coordinate that is not finite (the first such point is named), too many points, too few distinct points, and points that do not span the dimension (`DegenerateDimension`, with the points that span what they do span).
- **The number of allocations of a build does not grow with the number of points**, on the kinds of input the crate's allocation test builds.

## What is not

- **The numbers a result carries are `f64`**: a plane's normal and offset, the volume, and the coordinates of a Voronoi vertex are rounded, and are not used for any decision. `planes()` and a Voronoi build can fail when such a number is not finite; the topology is decided before that.
- **Across versions**, the diagonals chosen inside a group of cospherical sites, and the order in which simplices are listed, may change.
- **In `D >= 4`** a Voronoi diagram lists vertices, rays, cells, and the pairs of cells that meet, not a complete cell complex.
- There is **no tolerance to set**, no merging of nearly equal points, no joggling. Points that differ in one bit are different points.
- Construction is **sequential**, and a result is **not updated** by adding or removing a point.

## Limits

| | |
| --- | --- |
| Dimension | `D >= 1`; exact evaluation costs more as `D` grows. The array API covers `1 <= D <= 8` |
| Points | At most `u32::MAX`; indices are `u32` |
| Coordinates | Finite `f64` |
| Rust | 1.89 or later; `std` |
| Dependencies | `faer`, `pulp`, `thiserror` |

## Performance

A comparison with Qhull, CGAL, and Rust crates on standard data, with the machine, the versions, and what each library outputs, is in [docs/comparison.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.md). It is a report: rows where another library is faster are in it. The crate's own judgement against CGAL and Qhull is the parity run of [docs/verification.md](https://github.com/YUKIKEDA/convx/blob/main/docs/verification.md), a fixed set of rows with a threshold, and is kept apart from it.

## How it works

The specification is [docs/design.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.md) (canonical: [docs/design.ja.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.ja.md)), how results are checked is [docs/verification.md](https://github.com/YUKIKEDA/convx/blob/main/docs/verification.md), and the measurements behind each decision are in [docs/bench.md](https://github.com/YUKIKEDA/convx/blob/main/docs/bench.md).

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
