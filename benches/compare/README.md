# The comparison with other libraries

What the published comparison (`docs/comparison.md`, R1 of the roadmap) is made with. It is a report, not a test, and it is not the parity run: that is `benches/parity/`, the internal judgement of ADR 0006.

## A run

Linux or WSL2, with what the parity run needs (`benches/parity/README.md`) and nothing else running on the machine.

```sh
bash benches/compare/setup.sh <commit>            # the adapters of that commit and the CGAL program
bash benches/compare/data/make.sh rbox            # and pbbs, scans
cd ~/convx-compare
python3 <repo>/benches/compare/runner.py rows.txt results.jsonl
python3 <repo>/benches/compare/report.py results.jsonl
```

`COMPARE_HOME` is where everything is built and written (default `~/convx-compare`), and `COMPARE_CORE` the core every process is pinned to (default 2). `rows.txt` has one row a line: `<hull|delaunay> <data file name> <library> [<library> ...]`.

What a row records for each library:

- **Time:** the seconds of the construction of the output, timed by the library's adapter inside its own process, so reading the file is outside. The libraries of a row take turns, round by round: 5 rounds of 3 builds when the first library's first build takes under 1.5 s, otherwise 3 rounds of 1.
- **Counts:** vertices and facets of a hull, distinct sites and simplices of a triangulation. The report says where the libraries disagree, and prints the counts.
- **Peak memory:** the peak resident set of a process that reads the file and builds once, from the kernel (`ru_maxrss`), three times.
- **Memory over time:** the resident set of one such process, sampled every 2 ms from `/proc`, thinned to 400 samples.

Memory is taken after the timing of a row, so that sampling disturbs no time. It is the resident set of the whole process: the points read from the file are in it for every library.

## The libraries

Each is a program `<name> <hull|delaunay> <file> <reps>`, or is driven as one.

| Library | Adapter | What its timed section produces |
| --- | --- | --- |
| convx | `rust/src/bin/convx_run.rs` | What `build()` returns: for a hull its vertices, its facets with their vertex lists and neighbors, the coplanar and the interior points; for a triangulation its simplices with their neighbors. Exact signs |
| CGAL | `benches/parity/cgal_time.cpp` | The same output as convx where CGAL has it (ADR 0006): the hull's surface; the triangulation's cells and neighbors written to flat arrays. Exact predicates on `f64` (`Epick`, `Epick_d` above D = 3) |
| Qhull | `qconvex`, `qdelaunay` with `i s TI <file> TO <out>` | The facets by their vertices, written to a file; its own "CPU seconds to compute hull (after input)". One build a process. Qhull merges facets within a tolerance and is not exact: on the same points it can report fewer vertices, facets, or regions, and the report shows where |

| `spade` 2.15.1 | `rust/src/bin/spade_run.rs` | The Delaunay triangulation of D = 2, loaded in bulk: vertices, edges, and faces with their adjacency. Exact predicates |
| `delaunator` 1.1.0 | `rust/src/bin/delaunator_run.rs` | The Delaunay triangulation of D = 2 as triangles and half-edges. Robust orientation tests; equal points are not merged, so its first count is the points given |
| `delaunay` 0.8.2 | `rust/src/bin/delaunay_run.rs` | The Delaunay triangulation of D = 2 to 5, with its default options (the PL-manifold guarantee, which validates as it inserts). Exact predicates. It enters rows of 1,000 points only: it took 0.5 s for 1,000 points of D = 2 and 43 s for 10,000. Its lightest options returned an error on these files (the adapter's documentation has the case) |
| `parry3d-f64` 0.31.1 | `rust/src/bin/parry_run.rs` | The hull of D = 3 as vertices and triangles. Not exact: it can report fewer vertices. A facet of more than three points comes out as several triangles, so its second count is triangles |
| `geo` 0.33.1 | `rust/src/bin/geo_run.rs` | The hull of D = 2 as a polygon. Robust orientation tests |

The Rust adapters are one package, `rust/`, with a manifest and a lock file of its own: its dependencies are not convx's, and nothing of it ships. It needs Rust 1.98.1, which the `delaunay` crate asks for; convx's own MSRV is not touched.

Each crate enters the rows of its own dimension only, on one or two data files of a tier and two sizes; the full grid of data is for Qhull and CGAL (the owner's decision on #599). Not compared: `qhull` (bindings to Qhull, which is here itself), `chull`, `tritet`, `startin`, `voronoice`, `voronator`.

## Data

Three tiers, none of them committed as data (`.cursor/rules/publish.mdc`). `data/make.sh <tier>` writes the files under `$COMPARE_HOME/data` (default `~/convx-compare/data`) and prints, per file, its dimension, its points, and how many of them are distinct. Every file is a dimension, a count, and the points, as `rbox` writes them and as Qhull reads them.

| Tier | Made by | What it is |
| --- | --- | --- |
| `rbox` | `data/rbox.txt`, the arguments of Qhull's `rbox` | Uniform in a cube, on a sphere, a rotated integer lattice, and points with near-coincident companions; D = 2 to 8 |
| `pbbs` | `data/pbbs.txt` and `data/pbbs.py` | The distributions of the Problem Based Benchmark Suite: uniform in a ball, Kuzmin (D = 2), Plummer (D = 3). PBBS's formulas with another random source: the distribution of a PBBS file, not its points |
| `scans` | `data/scans.py` | The reconstructed surfaces of Bunny, Dragon, Happy Buddha, and Armadillo from the Stanford 3D Scanning Repository, downloaded and checked against a recorded SHA-256 |

`data/test_pbbs.py` checks each distribution against the closed form of its radial law (`python3 -m unittest` in `data/`).

The scans belong to the Stanford Computer Graphics Laboratory (http://graphics.stanford.edu/data/3Dscanrep/), which asks that the source be acknowledged and that the models not be used commercially without permission. They are downloaded by whoever runs the comparison and are not redistributed here.

The whole of the three tiers is about 950 MB (250 MB for `rbox`, 700 MB for the other two). It is removed after a run (`scratch.mdc`).
