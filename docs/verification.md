# Verification

Golden data is not a set that can be listed before the code exists. A phase checks the result that phase can produce. The next phase adds what it needs. Records from earlier phases stay.

This file names the flows that can exist. The judgment and the criteria inside a flow are written after the flows are named.

The library's meaning, the invariants, and the named completion inputs are the design (§10 and §11). This file does not restate them.

## What a phase can check

| Phase  | Becomes checkable                                                                                        | Records already in the tree                                                              |
| ------ | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| P1     | Predicate sign, the exact-sign fallback, and the predicate inputs named in the design                    | None                                                                                     |
| P2     | Hull topology, volume, the index partition, the hull invariants, the named hull inputs, hull simulations | P1 records stay. They are not hull expectations                                          |
| P3     | Sequential and parallel agreement on the same batch, in one binary                                       | Hull expectations stay. Parallel output does not replace them                            |
| P4     | Delaunay, Voronoi, and the named Delaunay inputs                                                         | Hull expectations stay. A Delaunay or Voronoi field is added only when P4 can produce it |
| P5     | Wider SIMD, caches, and faster predicates and classification                                              | Expectations stay. A faster path that disagrees is wrong                                 |
| Timing | Wall time against Qhull, after a correct sequential hull                                                 | Performance files stay separate from correctness records                                 |

A check is armed when the phase that produces that result is in the tree. A later result is not frozen early, and a record is not deleted because a later phase is still absent.

## Flows

### Arm a check

When a phase first produces a result the design asks to check, that check starts. It starts empty of frozen answers except the hand cases the design already states. Earlier checks keep running.

### Exercise before a frozen answer

Generated points can be checked against the invariants alone. Euler characteristic, sidedness, adjacency, and the index partition do not need an expected topology. This flow is how a new family is first run. It produces a pass or a failure. It does not write a record.

### Record a hand case

The completion inputs named in the design already have an expected result. The record copies that statement. The library's own output is not the source.

### Freeze a simulation

A seed has been run, and the invariants for the armed check hold. The normalized result of that run is committed as a regression lock for the observables that exist now. The lock is the first sequential result that satisfied the invariants. It is not a proof by a second implementation.

### Add a case

A new seed, dimension, family, or required failure enters a check that is already armed. The trigger is a phase that can now produce the result, a failure that the current set does not cover, or a review. Records already committed are left as they are.

### Extend a record

The same point set gains an observable a later phase can produce. A hull record gains a Delaunay or Voronoi expectation when P4 can produce it. The expectation already stored stays. Predicate cases do not grow a hull expectation. A hull record does not grow a predicate expectation.

### Steady run

Every test run rebuilds points from the recorded seed and compares the observables that are armed. It reads records. It does not write them. It does not call Qhull.

### Disagreement

A steady run does not match the frozen expectation. The flow stops at the failure. The record stays. Overwriting the record to match the run is not part of this flow.

### Revise because the design changed

A design change states a different result for an input that already has a record. The record is updated to the new statement in the same change as the design. Records the design change does not touch stay.

### Revise because the lock stored a wrong result

The frozen result itself violates the invariants, or it encodes a bug the change is fixing. That record is updated. The diff is the change under review. Records that still match a correct result stay.

### Change the generator

The same seed would build different points. Every correctness record tied to that generator becomes stale together. They are reviewed as one set. One file is not refreshed alone.

### Keep the result across a new path

A refactor, the parallel commit, or a P5 optimization produces a result the sequential lock already names. The lock stays. The new path is compared to it. Parallel agreement is the sequential path and the parallel path in the same binary, on the same batch. That comparison does not create a second record.

### Retire a record

The design no longer asks for that check, or a recorded case replaces it and covers it. The deletion is part of the change under review. A record is not retired because the phase that should produce it is not implemented yet, and it is not retired because a run disagreed.

### Record a required failure

The expected result is a named error (`DegenerateDimension`, `TooManyPoints`, `NonFiniteCoordinate`, and the other input failures the design names). The payload is the error, not a topology. Add, steady run, disagreement, revise, and retire apply to these records as they do to topology records.

### Check a transform

Translation, positive uniform scale, axis permutation, and input-order permutation are applied to an input that already has a record. The comparison rule is the one the design states for that transform. The transform does not create another record.

### Open a performance set

Timing starts after a correct sequential hull. A performance file is created then. It may use the same generator and the same seeds as a correctness record. It stores points to time. It does not store an expected topology. A later timing run reads it. A correctness run does not.

### Two records claim the same identity

Two records name the same generator, seed, dimension, point count, family, and observable. The run does not pick one. The conflict is resolved by revising or retiring one of them before the check is armed again.

## Situations with no flow

Refreshing every expectation because a run was inconvenient. Taking Qhull's triangulation as the expected logical facets. Loading a performance file as a correctness oracle. Deleting a record because a later phase is not implemented yet. Writing a Delaunay or Voronoi expectation before that phase can produce it.

## Hull records

The judgment inside the flows above, for the hull check armed in Phase 2.

### Record format

One plain-text file per record under `tests/fixtures/hull/`, named `<family>-d<dim>-n<count>-s<seed>.txt`. The header names the generator, family, dimension, point count, and seed. The observables are `vertices`, `coplanar_points`, `interior_points`, one `facet` line per logical facet (ascending vertex set, facets in lexicographic order), and `volume` with its relative tolerance or `volume unchecked`. Plain text keeps a revision readable line by line. The parser and the writer live in `tests/common/record.rs`.

A record of D <= 5 also stores the Delaunay and Voronoi observables, appended after the hull observables: `delaunay_sites` (the sites that are Delaunay vertices, ascending: duplicates reduce to their representative), one `delaunay_simplex` line per Delaunay simplex that is a whole Voronoi vertex (exactly D + 1 sites, ascending, in lexicographic order), one `voronoi_vertex` line per vertex (its cospherical sites), one `voronoi_ray` line per distinct ray (its apex, then its `hull_facet`; rays are numbered in this order), one `voronoi_cell` line per site (the site, then its vertices), one `voronoi_cell_rays` line per cell with rays (the site, then its ray numbers), one `voronoi_interface` line per interface (the two sites, then its vertices), and one `voronoi_interface_rays` line per interface with rays (the two sites, then its ray numbers). These are the topology the design promises. The simplices inside a cospherical group, its diagonals, are not frozen: §7 fixes them only between the sequential and parallel paths of one binary, so a record holds the group as its `voronoi_vertex` line alone. Circumcenter coordinates and ray directions are solved in f64 and are not frozen; they are checked on the hand cases. A record of D = 6 stores no Delaunay observable, and the steady run fails if one appears. The bound follows the debug steady-run time, not the design.

### Generator

`tests/common/generator.rs`: xoshiro256\*\* seeded through SplitMix64, with the families `cube`, `sphere`, `grid` (integers in [0, 3], exact coplanarities and duplicates), `cluster` (near-duplicates a few 2^-30 apart), `lattice` (integers in [-2, 2]), `onsphere` (integer points drawn with replacement from one sphere |p|^2 = R, with R = 4, 25, 9, 4 for D = 1, 2, 3, and above: many cospherical sites and duplicates), and `nearsphere` (`onsphere` with each coordinate moved by -1, 0, or +1 times 2^-8). The three new families exist for the Delaunay and Voronoi observables. Adding them leaves the points of the older families unchanged, so the generator id stays. Every record stores the generator id. Any change to the generator bumps the id, and the steady run then fails on every record of the old id until they are reviewed together (Change the generator).

### Freeze

`cargo run --example freeze_hull_fixture -- <family> <dim> <count> <seed>`, or `-- --standard` for the standard set (each family in D = 2..=6 with seed 1, plus seed 2 for `cube` and `grid`). The tool runs only with debug assertions, so the §10 invariant checker runs inside the build it records. It never overwrites an existing file. For D <= 5 it also writes the Delaunay and Voronoi observables. Before it writes them, the exact oracles of `tests/common/oracle.rs` check the triangulation and the Voronoi vertices over `i128` (empty spheres, orientation, coverage, one vertex per lower logical facet of the lift, and the interfaces and rays derived from the supporting hyperplanes of the site hull and of each vertex's polytope). The oracle needs integer sites: it runs on `grid`, `lattice`, and `onsphere` directly, and on `nearsphere` after scaling every coordinate by 2^8, where the tool also checks that the scaled topology equals the unscaled one. Families with non-integer coordinates (`cube`, `sphere`, `cluster`) are frozen under the §10 invariant checker alone. `-- --extend` appends these observables to every record of D <= 5 that lacks them (Extend a record): the lines already stored stay, and the diff only adds lines. Point counts stay small enough that a debug steady run, with the checker active, finishes in seconds.

### Volume tolerance

A frozen volume carries a relative tolerance of `1e-12`. A record whose volume is near zero relative to its extent is written `volume unchecked`, and only its topology is compared (§10). Near zero means below `f64::EPSILON / 1e-12` (about `2.2e-4`) times `w^D`, where `w` is the largest side of the bounding box: one rounding of a term at that scale already exceeds the tolerance. `Record::of` applies the rule when it freezes. In the standard set it marks `cluster-d6-n18-s1` (volume `3.07e-11`, `w` about 1.93); the next smallest record is above `3e-3` times `w^D`.

### Steady run

`tests/hull_fixtures.rs` reads every record, rebuilds the points from the seed, and compares all stored observables. It never writes. For D <= 5 it rebuilds the Voronoi diagram, whose sites, cells, and interfaces carry the Delaunay simplices, and compares the stored Delaunay and Voronoi observables too. Records run in parallel on the rayon pool. A mismatch fails the test, and the record stays as it is (Disagreement).

### Revise

A record is revised only by editing the file in a change under review, under "Revise because the design changed" or "Revise because the lock stored a wrong result". The diff of the record is part of that review. Deleting the file and freezing again is the same revision and is reviewed the same way.

## Performance sets

The judgment inside "Open a performance set".

`benches/sets.txt` lists each set as `set <family> <dim> <count> <seed>` under the generator id it was written for: the `cube` and `sphere` families of `tests/common/generator.rs`, D = 2..=8, 10^4, 10^5, and 10^6 points, seed 1. The list stores no expected topology. `cargo run --release --example export_qhull_sets [-- <filter>]` writes each set to `.dev/perf/` (not in git) in Qhull's input format: the dimension, the count, then one point per line in shortest round-trip `f64` decimal, so Qhull reads the same bit patterns convx is given. Existing files are kept. The filter is matched by `-`-separated tokens: `-- d3-n10000` writes the two D = 3 sets of 10^4 points, not those of 10^5 or 10^6.

The Qhull reference is run as `qconvex i s TI <file>`: `i` lists the vertices of each facet, which are the logical facets to compare, and `s` adds the summary (input points, dimension, vertex and facet counts); `s` alone prints no facet vertex sets. No `QJ` (no joggle), no `Qt` (logical facets, not a triangulated output), no other option that perturbs the input, in the same dimension as the set. Timings compare logical facets with logical facets. A correctness test never reads these files, and a timing run never writes a correctness record.

The Delaunay reference is `qdelaunay i s TI <file>`: `i` lists the Delaunay regions, which merge cospherical simplices as convx's groups do, and `s` adds the summary. Its default `Qbb` scales the lifted coordinate; no option perturbs the input. Its region count is compared with convx's simplex count only on sets in general position, where the two are the same.

Four timed sections are reported side by side (#250), so that a gap can be read as construction cost or as the work around it:

| Column | Timed section |
| :--- | :--- |
| convx `build()` | `ConvexHullBuilder::new(dim, &pts).build()` or `DelaunayBuilder::new(dim, &pts).build()`: acceptance, duplicate detection, construction, and the pass after it (merge, classification, publication; for Delaunay, cospherical groups and publication) |
| convx construction | The hull: `SimplicialHull::build`, from the accepted input to the simplicial hull. Delaunay: `delaunay::inserted` from its start through `insert::Mesh::build`, the insertion order included. Acceptance, duplicate detection, and every pass after construction are outside it |
| Qhull compute | Qhull's "CPU seconds to compute hull (after input)" from the `s` summary |
| Qhull whole run | Wall time of the whole `qconvex i s TI <file> TO <out>` (or `qdelaunay`) process, pinned like the others: start-up, reading and parsing the file, computing, and writing the facet list to a file |

The construction timer is not committed (`.cursor/rules/bench.mdc`). It goes into a scratch copy of the measured commit: `SimplicialHull::build` becomes a wrapper that times the original body and prints `construct <seconds>` to standard error, and `delaunay::inserted` prints the same line right after `insert::Mesh::build`. The timing binary pairs each line with the build it belongs to. Qhull's work counters come from the same `s` summary: points processed, hyperplanes created, and distance tests. They say whether a gap is the amount of work or its cost per operation.

CGAL is the second reference (#215). It decides signs exactly on the input `f64`, as convx does, with the kernel `Epick` (`Epick_d<Dimension_tag<D>>` above D = 3); it is timed on the same files, built with `g++ -O3 -DNDEBUG -g` and no `-march`, as convx is built for the baseline target. The timed section is construction only; reading the file and counting the output are outside it. The hull is `CGAL::convex_hull_2` for D = 2, `CGAL::convex_hull_3` into a `Surface_mesh` for D = 3, and for D >= 4 `CGAL::Triangulation`, built with range `insert`, whose hull facets are the full cells incident to the infinite vertex. Delaunay is `Delaunay_triangulation_2` and `Delaunay_triangulation_3` built from the range, and `CGAL::Delaunay_triangulation` with range `insert` for D >= 4. CGAL's output is simplicial, so its facet count is compared with convx's logical facets only on sets in general position, where the two are the same; a set where they differ is reported, not timed. Delaunay is compared by finite simplex count. Measured numbers, and any speedup target, are written only in the change that first measures them (`.cursor/rules/bench.mdc`). Reference comparisons are kept in `docs/bench.md`, one section per measured convx commit.
