English | [日本語](https://github.com/YUKIKEDA/convx/blob/main/README.ja.md)

# convx

Convex hulls, Delaunay triangulations, and Voronoi diagrams in any dimension, in pure Rust, with every topological decision made by the exact sign of a predicate.

- **Exact.** Which side of a facet a point is on, and whether points are coplanar or cospherical, is the sign of a determinant of the input coordinates, evaluated exactly when floating-point arithmetic cannot certify it. There is no tolerance and no perturbation of the input.
- **Any dimension.** One implementation for every `D >= 1`, with dedicated paths where they pay (`D = 2` and `D = 3`).
- **Degenerate input is input.** Duplicate points, points on a facet, cocircular and cospherical sites, and lattices are handled by the same rules as points in general position, and the result says what each input point became.
- **No `unsafe`** in this crate.

```toml
[dependencies]
convx = "0.1"
```

The version is 0.1.0. A 0.x minor version may break the public API.

## A convex hull

Points are one row-major slice of `f64`: `D` coordinates a point.

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

    // Four edges, each with its two vertices and its two neighbors.
    assert_eq!(hull.facets().len(), 4);
    for facet in hull.facets().iter() {
        assert_eq!(facet.vertices().len(), 2);
        assert_eq!(facet.neighbors().len(), 2);
    }

    // Planes and volume are computed in `f64` from the decided topology.
    assert_eq!(hull.volume(), 1.0);
    let planes = hull.planes()?;
    assert_eq!(planes.len(), 4);
    Ok(())
}
```

A facet is a *logical* facet: simplices that are exactly coplanar are one facet, so a cube has six facets of four vertices, not twelve triangles. `triangulation()` gives the boundary as simplices when that is what is wanted.

## A Delaunay triangulation

```rust
use convx::DelaunayBuilder;

fn main() -> Result<(), convx::Error> {
    // The corners of a square and its center.
    let sites = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    let delaunay = DelaunayBuilder::new(2, &sites).build()?;

    // Four triangles, each with the center as a vertex.
    assert_eq!(delaunay.simplices().len(), 4);
    for simplex in delaunay.simplices().iter() {
        assert!(simplex.vertices().contains(&4));
        // The neighbor across each face, or `u32::MAX` on the hull of the sites.
        assert_eq!(simplex.neighbors().len(), 3);
    }
    Ok(())
}
```

Every site is in the triangulation, the ones inside the hull of the sites included. Sites on one circle or sphere are found by exact signs, merged into one group, and split again by one rule, the same that splits a facet of a hull.

## A Voronoi diagram

```rust
use convx::VoronoiBuilder;

fn main() -> Result<(), convx::Error> {
    let sites = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    let voronoi = VoronoiBuilder::new(2, &sites).build()?;

    // The cell of the center is a square of four vertices and has no ray.
    let center = voronoi.cells().iter().find(|cell| cell.site() == 4).unwrap();
    assert_eq!(center.vertices().len(), 4);
    assert_eq!(center.ray_numbers().len(), 0);

    // A cell of a corner is unbounded: it has rays.
    let corner = voronoi.cells().iter().find(|cell| cell.site() == 0).unwrap();
    assert!(!corner.ray_numbers().is_empty());
    Ok(())
}
```

A Voronoi vertex is one group of cospherical Delaunay simplices, merged by exact signs and not by the closeness of circumcenters: four cocircular sites give one vertex.

## Arrays instead of a slice

`StaticConvexHull`, `StaticDelaunay`, and `StaticVoronoi` take `&[[f64; D]]` for `1 <= D <= 8` and return the same results.

```rust
use convx::StaticConvexHull;

fn main() -> Result<(), convx::Error> {
    let tetrahedron = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let hull = StaticConvexHull::<3>::build(&tetrahedron)?;
    assert_eq!(hull.facets().len(), 4);
    Ok(())
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

A comparison with Qhull, CGAL, and Rust crates on standard data, with the machine, the versions, and what each library outputs, is in [docs/comparison.md](https://github.com/YUKIKEDA/convx/blob/main/docs/comparison.md). It is a report: rows where another library is faster are in it.

## How it works

The specification is [docs/design.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.md) (canonical: [docs/design.ja.md](https://github.com/YUKIKEDA/convx/blob/main/docs/design.ja.md)), how results are checked is [docs/verification.md](https://github.com/YUKIKEDA/convx/blob/main/docs/verification.md), and the measurements behind each decision are in [docs/bench.md](https://github.com/YUKIKEDA/convx/blob/main/docs/bench.md).

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-APACHE) or [MIT license](https://github.com/YUKIKEDA/convx/blob/main/LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
