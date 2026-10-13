# Roadmap

Procedure: [CONTRIBUTING.md](../CONTRIBUTING.md). Design: [design.ja.md](design.ja.md) and [design.md](design.md). When the two design files differ, follow `design.ja.md`.

Acceptance text stays on each Issue and in the design (§10 and §11). This file keeps the ID, title, Issue, and status.

## Current work

The next phase is R1, the first release on crates.io with a published comparison (#592); its rows are below. R1-9 (the filter's exact scaling, #589) is first, by the owner's word of 2026-10-12: no benchmark is taken and then followed by a change to the code it measured. A parity run after it confirms that nothing else moved, and R1-1 follows. That scaling, the rules, the package, and the comparison's data and tools are done (R1-1, R1-2, R1-5, R1-6, R1-9). The trial of the comparison's adapters found a row no parity set has, the Delaunay triangulation of a lattice at three times CGAL's time (R1-11, #614); a zero is now proved where the first stage's arithmetic is exact (R1-12, #615), and the parity sets report two Delaunay lattices. Every data file of the comparison was then run once beside CGAL and Qhull (R1-14, #618), and rows above the parity's margin were found that no parity set has: the Delaunay triangulation of Kuzmin sites grows faster than linearly, the two large scans read 1.13 and 1.18 of CGAL in D = 3, and a hull of points that are nearly all deep inside takes 1.3 to 4.8 times the faster library (`docs/bench.md`). The owner's order of 2026-10-13: the insertion order first (R1-15, #620, done: a crowded cell of its grid is ordered again, 10^6 Kuzmin sites build in 0.21 of the time and 10^7 in 0.71 of CGAL's), then a spike on what a hull does to every point before construction (R1-16, #622, done: the duplicates of every point, the order in space before any point is discarded, and a first simplex that is not chosen; the owner's order: the first simplex from extreme points (R1-17, #624, done: the Plummer hull in half the time, the scans in 0.73 to 0.80 of it), then the discard before the order in space (R1-18, #625, withdrawn after measurement: a simplex holds little of most inputs, and every hull of 2^17 points or more but the Plummer read 1.02 to 1.25 of its time), `representative` on demand is not taken for 0.1.0 (the owner's decision after its measurement, under Intentionally out of scope below); and the hull of a ball in D = 2 locates a point near its boundary by two exact signs where it took eleven (R1-19, #628, and R1-20, #630, done: 0.73 of its time, 1.16 of CGAL from 1.58); the lattices' spike (R1-13, #617, done) found the placing of each group and the pairing of every face of a merged group, and the owner's order of 2026-10-13: a group of four sites of D = 2 written without the placing (R1-21, #633, done: the lattices of D = 2 at 0.86 of their time), and the two other changes prototyped and measured before either is taken or left (R1-22, #634)), the lattices (R1-13, #617), a CPU profile of the scans beside CGAL, a parity run, and the comparison (R1-8).

P7 is done (Grill on #512, #518). Its final parity run met 34 of the 38 judged sets at 1.05; Delaunay `cube` D3 is reported, not judged (ADR 0006). A parity run after the allocation rows (#569, `main` at `5846643`) met 33: hull `sphere` D2 builds 8 to 10% slower since #549 and left 1.05 at 10^4 and 10^5, and no other set moved beyond the spread when the two commits were timed against each other. The cause was a write in the body of every push of a flat list, which #549 added so that a default list allocates nothing; it is out of line now, and hull `sphere` D2 is back at the time before #549 under WSL (#570, done). Judged again on `main` at `ce01e84` (#577), 35 of the 38 sets are met at 1.05: hull `sphere` D2 is back, and what is not met is hull `sphere` D3 10^4 (1.06 to 1.09 against Qhull; `main` built it 3.5% slower than the final P7 run's commit; the flat lists now keep where each list ends, a push tests nothing, and the set reads 1.01 of that commit by the median under WSL in the default build and 1.00 with one codegen unit, #578; judged again on `main` at `d9b6725` it reads 0.98 against Qhull, and of the thirteen sets that were not met or at the edge only Delaunay `sphere` D2 10^4 is not met (1.10), with five between 1.02 and 1.05, #583) and Delaunay `cube` D2 and `sphere` D2 at 10^4, as before. The performance rules and the tested allocation contract are in place (#521, `.cursor/rules/perf.mdc`, `tests/alloc.rs`). What stays open, until the owner opens the next phase: hull `cube` D2 10^6 (P7-72's 3% and the day), Delaunay `cube` D2 and `sphere` D2 10^4, and Delaunay `cube` D4 10^4; the growths per site of Voronoi (#523), per simplex of the placing triangulation (#524), and per face of a hull of D = 3 (#530) are gone. The placing triangulation of points that span a plane is read from their cyclic order (#534): the hull of a prism over a k-gon was quadratic in k (#533) and is linear. Points that span three dimensions or more (a face of a hull of D >= 4, a cospherical group of Delaunay D >= 3) took the test of every boundary ridge, quadratic in the points of one face or group (measured on #531). The owner split the work into rows (Grills on #531 and #541, 2026-10-12), and three are done: the pulling triangulation of sites that all lie on one sphere (#539); the facets and edges of a cospherical group of a Voronoi build, which were cubic in its sites (#541); and the placing triangulation of three dimensions or more, which reaches the ridges a point is beyond through the ridges' neighbors (#540). The last of that order is done too: the sub-hull of a face in D >= 4 is built in the storage of the one before, and `tests/alloc.rs` has no exception (#531). Its heap profile found an allocation per exact direction of a plane in the predicates, now returned inline (#548; owner, 2026-10-13). The copy in space order of a sub-hull of D = 3 with 2^17 points or more keeps its lists with the hull's storage too (#551). A scan of 90 inputs at two sizes then found one more, which had had a row since #184: the exact stage over heap integers (P5-44, #185), reached by coordinates of exponents far apart and by D > 10. It runs on a workspace kept per thread now. The scan stopped at D = 6. Counted afterwards, a hull of D = 9 or 10 allocated for every simplex it planned, in two lists of capacity 8 and in the filter's rows of sizes 10 and 11; they are inline and on the stack now (#554). In D >= 11 every list whose length follows the dimension is on the heap. The owner decided on 2026-10-13 that the contract covers D <= 10 (Grill on #554): the design says so, and D >= 11 is not a row. A scan of degenerate inputs in D = 5 to 10 found no count that grows per item (#560); `docs/bench.md` has what it ran and what it left out. Two things it saw were spikes by the owner's decision of 2026-10-13. The fixed allocations of a Delaunay build were the links of its initial simplices (#562), now made from faces sorted once (#564). The hull of a cube's corners builds each distinct face once (#563); the part of its time that was the scan of the placing triangulation before a facet's complex has its dimension is gone: the search on the surface runs at every dimension the complex reaches, and the corners of a cube in D = 8 take 0.85 of their time (#565). What a sub-hull has to compute beyond its vertices is a design question, not a row. A cheaper pass over the D = 2 discard polygon's extremes returns if a parity run puts hull `sphere` D2 back outside 1.05 (#515). Hull `cubesurf` D3's sort of `u32` lists (Grill on #503) and Delaunay D2's walk in plain doubles (Grill on #509) return when a later phase takes up the sets that stay open. Three rows of P7 were not started and stay open, in the order the owner set on 2026-10-10: P7-8 (#323), acceptance; P7-31 (#400), the exact direction's bound; P7-18 (#355), whether the lifted underflow terms of the first-stage formulas of k ≤ 4 are needed (the bound is conservative as it stands).
P7 is done (Grill on #512, #518). Its final parity run met 34 of the 38 judged sets at 1.05; Delaunay `cube` D3 is reported, not judged (ADR 0006). A parity run after the allocation rows (#569, `main` at `5846643`) met 33: hull `sphere` D2 builds 8 to 10% slower since #549 and left 1.05 at 10^4 and 10^5, and no other set moved beyond the spread when the two commits were timed against each other. The cause was a write in the body of every push of a flat list, which #549 added so that a default list allocates nothing; it is out of line now, and hull `sphere` D2 is back at the time before #549 under WSL (#570, done). Judged again on `main` at `ce01e84` (#577), 35 of the 38 sets are met at 1.05: hull `sphere` D2 is back, and what is not met is hull `sphere` D3 10^4 (1.06 to 1.09 against Qhull; `main` built it 3.5% slower than the final P7 run's commit; the flat lists now keep where each list ends, a push tests nothing, and the set reads 1.01 of that commit by the median under WSL in the default build and 1.00 with one codegen unit, #578; judged again on `main` at `d9b6725` it reads 0.98 against Qhull, and of the thirteen sets that were not met or at the edge only Delaunay `sphere` D2 10^4 is not met (1.10), with five between 1.02 and 1.05, #583). One full run on `main` at `df80378`, after #587 and #588, met 36 of the 38 at 1.05 and 29 at 1.00; Delaunay `cube` D2 and `sphere` D2 at 10^4 are the two not met (#591). After the input's exact scaling (#589), the run on `327e5a1` reads the same: 36 at 1.05, 30 at 1.00, and the same two not met (#609) and Delaunay `cube` D2 and `sphere` D2 at 10^4, as before. The performance rules and the tested allocation contract are in place (#521, `.cursor/rules/perf.mdc`, `tests/alloc.rs`). What stays open, until the owner opens the next phase: hull `cube` D2 10^6 (P7-72's 3% and the day), Delaunay `cube` D2 and `sphere` D2 10^4, and Delaunay `cube` D4 10^4; the growths per site of Voronoi (#523), per simplex of the placing triangulation (#524), and per face of a hull of D = 3 (#530) are gone. The placing triangulation of points that span a plane is read from their cyclic order (#534): the hull of a prism over a k-gon was quadratic in k (#533) and is linear. Points that span three dimensions or more (a face of a hull of D >= 4, a cospherical group of Delaunay D >= 3) took the test of every boundary ridge, quadratic in the points of one face or group (measured on #531). The owner split the work into rows (Grills on #531 and #541, 2026-10-12), and three are done: the pulling triangulation of sites that all lie on one sphere (#539); the facets and edges of a cospherical group of a Voronoi build, which were cubic in its sites (#541); and the placing triangulation of three dimensions or more, which reaches the ridges a point is beyond through the ridges' neighbors (#540). The last of that order is done too: the sub-hull of a face in D >= 4 is built in the storage of the one before, and `tests/alloc.rs` has no exception (#531). Its heap profile found an allocation per exact direction of a plane in the predicates, now returned inline (#548; owner, 2026-10-13). The copy in space order of a sub-hull of D = 3 with 2^17 points or more keeps its lists with the hull's storage too (#551). A scan of 90 inputs at two sizes then found one more, which had had a row since #184: the exact stage over heap integers (P5-44, #185), reached by coordinates of exponents far apart and by D > 10. It runs on a workspace kept per thread now. The scan stopped at D = 6. Counted afterwards, a hull of D = 9 or 10 allocated for every simplex it planned, in two lists of capacity 8 and in the filter's rows of sizes 10 and 11; they are inline and on the stack now (#554). In D >= 11 every list whose length follows the dimension is on the heap. The owner decided on 2026-10-13 that the contract covers D <= 10 (Grill on #554): the design says so, and D >= 11 is not a row. A scan of degenerate inputs in D = 5 to 10 found no count that grows per item (#560); `docs/bench.md` has what it ran and what it left out. Two things it saw were spikes by the owner's decision of 2026-10-13. The fixed allocations of a Delaunay build were the links of its initial simplices (#562), now made from faces sorted once (#564). The hull of a cube's corners builds each distinct face once (#563); the part of its time that was the scan of the placing triangulation before a facet's complex has its dimension is gone: the search on the surface runs at every dimension the complex reaches, and the corners of a cube in D = 8 take 0.85 of their time (#565). What a sub-hull has to compute beyond its vertices is a design question, not a row. A cheaper pass over the D = 2 discard polygon's extremes returns if a parity run puts hull `sphere` D2 back outside 1.05 (#515). Hull `cubesurf` D3's sort of `u32` lists (Grill on #503) and Delaunay D2's walk in plain doubles (Grill on #509) return when a later phase takes up the sets that stay open. Three rows of P7 were not started and stay open, in the order the owner set on 2026-10-10: P7-8 (#323), acceptance; P7-31 (#400), the exact direction's bound; P7-18 (#355), whether the lifted underflow terms of the first-stage formulas of k ≤ 4 are needed (the bound is conservative as it stands).

## Dependencies

```text
M0 → P1 → P2 → P3
              → P4
              → P5
              → P6 → P7
```

P3, P4, and P5 follow a correct sequential hull (P2). P4 does not wait for P3. P5 does not gate P4. P6 replaces the construction core, the predicate filter, and the result storage after the measurements of P5; its rows come before the remaining P5 rows. P7 works toward parity with Qhull and CGAL on the core that P6 built. The sign convention and the lift formula are part of P4 from the start.

## M0

| ID   | Kind | Title                   | Issue | Status |
| ---- | ---- | ----------------------- | ----- | ------ |
| M0-1 | Task | Crate, justfile, and CI | #2    | Done   |

## P1

Predicate kernel.

| ID   | Kind | Title                                                                                                                          | Issue | Status |
| ---- | ---- | ------------------------------------------------------------------------------------------------------------------------------ | ----- | ------ |
| P1-1 | Feat | Predicate kernel: orientation, distance sign, coplanar, error bound, exact-sign fallback, k ≤ 4 formulas, filtered determinant | #3    | Done   |
| P1-2 | Feat | Unit normal by Householder QR                                                                                                  | #4    | Done   |
| P1-3 | Feat | Single-threaded generational arena                                                                                             | #5    | Done   |
| P1-4 | Feat | SIMD distance cull                                                                                                             | #6    | Done   |
| P1-5 | Test | Phase 1 predicate inputs                                                                                                       | #7    | Done   |

## P2

Sequential Quickhull.

| ID   | Kind | Title                                                                    | Issue | Status |
| ---- | ---- | ------------------------------------------------------------------------ | ----- | ------ |
| P2-1 | Feat | Sequential Quickhull through insertion, kept simplicial                  | #8    | Done   |
| P2-2 | Feat | Merge coplanar simplices into logical facets after insertion             | #9    | Done   |
| P2-3 | Feat | Distance-zero classification and the index partition                     | #10   | Done   |
| P2-4 | Feat | Published hull surface: plane, triangulation, boundary cycle, and volume | #11   | Done   |
| P2-5 | Test | Phase 2 hull inputs and invariants                                       | #12   | Done   |
| P2-6 | Test | Deterministic hull simulations and frozen fixtures                       | #13   | Done   |
| P2-7 | Task | Deterministic point sets for the Qhull timing comparison                 | #14   | Done   |

## P3

Parallel commit. The initial simplex is the same function as in P2.

| ID   | Kind | Title                                                                   | Issue | Status |
| ---- | ---- | ----------------------------------------------------------------------- | ----- | ------ |
| P3-1 | Feat | Batch extraction shared with the sequential build                       | #15   | Done   |
| P3-2 | Feat | Reserve T and H, and drop conflicts on prospective simplices            | #16   | Done   |
| P3-3 | Feat | Worker-local mutation and commit in input-index order                   | #17   | Done   |
| P3-4 | Test | Debug agreement of sequential batch application and the parallel commit | #18   | Done   |

## P4

Static API, Delaunay, Voronoi, and oracles. Pulling triangulation and the cospherical merge are internal to this phase.

| ID   | Kind | Title                                                              | Issue | Status |
| ---- | ---- | ------------------------------------------------------------------ | ----- | ------ |
| P4-1 | Feat | Delaunay as the lower hull of the lift                             | #19   | Done   |
| P4-2 | Feat | Pulling triangulation when the lift is flat                        | #20   | Done   |
| P4-3 | Feat | Voronoi dual, merging cospherical simplices into one vertex        | #21   | Done   |
| P4-4 | Feat | Static API                                                         | #22   | Done   |
| P4-5 | Test | Oracles and the Phase 4 Delaunay inputs                            | #23   | Done   |
| P4-6 | Test | Deterministic Delaunay and Voronoi simulations and frozen fixtures | #24   | Done   |

## P5

After a correct sequential hull. These rows do not block P4.

| ID    | Kind  | Title                                                                                                     | Issue | Status                   |
| ----- | ----- | --------------------------------------------------------------------------------------------------------- | ----- | ------------------------ |
| P5-1  | Feat  | Wider SIMD, including AVX-512                                                                             | #25   | Done                     |
| P5-3  | Feat  | Cache of lifted coordinates                                                                               | #27   | Done                     |
| P5-4  | Feat  | Leave non-lower simplices out of the coplanar merge                                                       | #28   | Done                     |
| P5-5  | Feat  | Vector cull kernel faster than its scalar lanes                                                           | #70   | Done                     |
| P5-6  | Feat  | Filtered determinant cheap for every k > 4                                                                | #72   | Done                     |
| P5-7  | Feat  | Faster distance-zero classification                                                                       | #73   | Done                     |
| P5-8  | Feat  | Linear batch extraction                                                                                   | #75   | Done                     |
| P5-9  | Feat  | Certified distance proves a strict side                                                                   | #79   | Done                     |
| P5-10 | Feat  | Visibility search without SipHash                                                                         | #85   | Done                     |
| P5-11 | Feat  | Cheaper certification of a new facet's working normal                                                     | #86   | Done                     |
| P5-12 | Feat  | Faster exact stage for predicates the filter cannot decide                                                | #96   | Done                     |
| P5-13 | Feat  | Certified cull and strict-side proof for lifted facets                                                    | #109  | Done                     |
| P5-14 | Feat  | Skip the visibility walk for candidates a round rejects                                                   | #110  | Done                     |
| P5-15 | Feat  | One shared elimination for a facet's cofactors                                                            | #111  | Done                     |
| P5-16 | Feat  | Hull cube D6 10^4 within 1.39 s on one core                                                               | #120  | Done                     |
| P5-19 | Feat  | Factor the published normal with one Householder QR                                                       | #135  | Done                     |
| P5-20 | Feat  | Run the k > 4 elimination in lanes with inline cofactors                                                  | #143  | Done                     |
| P5-21 | Feat  | Cut per-insertion allocation and relinking in the hull build                                              | #145  | Done                     |
| P5-22 | Feat  | Evaluate the cofactors of four new simplices at once in lanes                                             | #147  | Done                     |
| P5-23 | Feat  | Gather, scale, and store the four lane facets inside the vectorized call                                  | #149  | Done                     |
| P5-24 | Feat  | Publish the hull without per-item vectors and indirect sorts                                              | #151  | Done                     |
| P5-26 | Feat  | Build the ridge keys of a new simplex from one sorted horizon ridge                                       | #154  | Done                     |
| P5-27 | Feat  | Find the vertices an insertion loses without sorting the region's vertices                                | #156  | Done                     |
| P5-28 | Feat  | Examine only the first 64 candidates of each round                                                        | #159  | Done                     |
| P5-29 | Feat  | Classify and publish without per-facet copies; pair Delaunay faces without a hash map                     | #163  | Done                     |
| P5-30 | Feat  | Build the Voronoi cells, tiles, and rays without quadratic scans                                          | #165  | Done                     |
| P5-31 | Feat  | Pass the cull origin instead of copying it into every plane                                               | #167  | Done                     |
| P5-32 | Bug   | Spread two-vertex keys over the face pairing table                                                        | #170  | Done                     |
| P5-33 | Feat  | Pick a round's candidates without a pass over every pending candidate                                     | #172  | Done                     |
| P5-34 | Feat  | Exact stage without heap big integers for the predicates coplanar inputs send past the filter             | #173  | Done                     |
| P5-35 | Feat  | Certify the working normal of a facet with 2 to 4 points as cheaply as the lane path                      | #174  | Done                     |
| P5-36 | Spike | Would an incremental Delaunay in BRIO order beat the lifted Quickhull at D = 2 and 3?                     | #175  | Done                     |
| P5-37 | Spike | Would a core monomorphized on D speed up the static API, and would a dedicated D = 2 hull beat Quickhull? | #176  | Done                     |
| P5-38 | Spike | Where does the peak memory of output-heavy builds go?                                                     | #177  | Absorbed by P6-6 (#254)  |
| P5-39 | Spike | Why does parallel(true) give only 1.2 to 1.3 times on four cores?                                         | #178  | Absorbed by P6-8 (#256)  |
| P5-40 | Docs  | Let the filter rescale tiny inputs by an exact power of two                                               | #179  | Done                     |
| P5-41 | Spike | How many NonFiniteCircumcenter failures have a circumcenter that fits in f64?                             | #180  | Done                     |
| P5-42 | Spike | Why does hull sphere D2 wall time grow faster than its instruction count?                                 | #182  | Done                     |
| P5-43 | Spike | Coplanar points are scanned again for every new facet: can construction stop rescanning them?             | #183  | Done                     |
| P5-44 | Feat  | Reuse the work space of the heap exact stage across predicate calls                                       | #185  | Done                     |
| P5-45 | Spike | Near-cospherical sites send almost every lifted predicate to the exact stage                              | #187  | Not started              |
| P5-46 | Feat  | Build Delaunay and Voronoi by incremental insertion in every dimension (design #188)                      | #189  | Done                     |
| P5-47 | Feat  | Deterministic parallel insertion for Delaunay and Voronoi                                                 | #190  | Set after Grill on #190  |
| P5-48 | Spike | Where does hull sphere time go at D = 3 to D = 6?                                                         | #199  | Done                     |
| P5-49 | Spike | Why is the low-dimension cube hull slower than Qhull's compute time?                                      | #200  | Done                     |
| P5-50 | Bug   | Run the filtered elimination under AVX2, not AVX-512 (the D = 8 lifted 9 x 9 was 24x slower)              | #195  | Done                     |
| P5-51 | Feat  | Vectorize the filtered elimination of size 10 and up (heap rows)                                          | #203  | Closed: done by #554     |
| P5-52 | Feat  | Cut the copies and cache lines of the simplex record that hull construction writes and walks              | #209  | Done                     |
| P5-53 | Docs  | Publish the certified cofactor direction as every facet's unit normal                                     | #210  | Done                     |
| P5-54 | Feat  | Publish the certified cofactor direction and remove the published QR                                      | #212  | Done                     |
| P5-55 | Feat  | Find duplicate points and list interior points without sorting every input point                          | #213  | Done                     |
| P5-56 | Feat  | Partition outside points without re-testing what the scan proved or re-gathering scattered rows           | #214  | Done                     |
| P5-57 | Spike | How does convx compare with CGAL on the hull and Delaunay timing sets?                                    | #215  | Done                     |
| P5-58 | Spike | How much of the pass after construction can a hull of single-simplex facets skip?                         | #228  | Done                     |
| P5-59 | Docs  | One filtered strict chain for the D = 2 hull                                                               | #230  | Done                     |
| P5-60 | Feat  | Build every D = 2 hull by the filtered strict chain                                                        | #232  | Done                     |
| P5-61 | Spike | Extremes of a coplanar face come from recursive sub-hulls: where does that time go?                       | #237  | Done                     |
| P5-62 | Spike | What does hull construction in general position pay per simplex, at 10^4 and at 10^6 points?              | #247  | Done                     |
| P5-63 | Feat  | Predicates: the filter scales one predicate's entries by an exact power of two                            | #589  | Done                     |

## P6

Architecture rework. Rows follow the review and Grill of 2026-10-07. Each row keeps every test, fixture, and oracle passing; no new core stays beside the one it replaces.

| ID   | Kind | Title                                                                                | Issue | Status      |
| ---- | ---- | ------------------------------------------------------------------------------------ | ----- | ----------- |
| P6-1 | Feat | Semi-static filter for orientations and lifted orientations with k ≤ 4               | #249  | Done        |
| P6-2 | Task | Time Qhull's whole run and convx's construction alone next to `build()`              | #250  | Done        |
| P6-3 | Docs | Sequential hull inserts one point at a time in place; batch extraction only parallel | #251  | Done        |
| P6-4 | Docs | Store the hull, Delaunay, and Voronoi results in flat arrays behind views             | #252  | Done        |
| P6-5 | Feat | Facet store sized to the hull's dimension and sequential Quickhull in place          | #253  | Done        |
| P6-6 | Feat | Flat result storage and views for the hull, Delaunay, and Voronoi                    | #254  | Done        |
| P6-7 | Feat | Delaunay: link across the cavity boundary and publish from the mesh                  | #255  | Done        |
| P6-8 | Feat | Parallel hull on the new facet store, kept only if it beats the sequential build     | #256  | Done        |
| P6-9 | Docs | The hull is built sequentially only; the parallel round protocol and flag leave      | #266  | Done        |
| P6-10 | Refactor | Remove the parallel round protocol and the `parallel` flag                       | #267  | Done        |
| P6-11 | Spike | Which coarse-grained parallel hull strategies can beat the sequential build?     | #269  | Done        |
| P6-12 | Docs | Let construction record the planes a point lies on                                   | #236  | Done        |
| P6-13 | Task | Measure convx after P6 against Qhull and CGAL; rank the remaining gaps             | #275  | Done        |
| P6-14 | Spike | Can a ParGeo-style fully parallel hull beat the sequential build at 4 and 16 threads? | #276  | Done        |
| P6-15 | Task | Profile the three gaps that remain after P6                                         | #285  | Done        |
| P6-16 | Spike | Hull construction: what do the candidate heap and by-value moves cost on sphere D3? | #286  | Done        |
| P6-17 | Docs | Publish without redundant work, and compute facet planes on first use               | #287  | Done        |
| P6-18 | Docs | A dedicated Delaunay insertion for D = 2 and D = 3                                  | #288  | Done        |
| P6-19 | Feat | Hull: take candidates from a bucket queue by distance                               | #291  | Done        |
| P6-20 | Feat | Hull: planes and the boundary complex on first use; publish without redundant work  | #293  | Done        |
| P6-21 | Feat | Delaunay: specialize the insertion for D = 2 and D = 3                              | #294  | Done        |
| P6-22 | Feat | Hull: record the planes a point lies on and use them in insertion and classification | #306  | Done        |

## P7

Parity with Qhull and CGAL: `build()` at most the time of each reference with the same output, on every benchmark set (ADR 0006, Grill of 2026-10-09), judged within 5% (amendment of 2026-10-10, #382). The rows after P7-3 come from the spike P7-2 and the design P7-3. P7-2 led to local improvements of the current insertion: P7-4, then the profile P7-5. P7-3 led to P7-6, and its Grill to the speed rows P7-7 to P7-9 for hull D2 and acceptance. The profile P7-5 led to P7-10 to P7-13 for Delaunay, and the Grill of P7-11 to P7-14. The spike P7-12 led to P7-15 and P7-16 for the predicates, and the Grill of P7-15 to P7-17. The review of P7-16 led to P7-18. The spike P7-13 led to P7-19 and P7-20. The spike P7-20 led to P7-21. The Grill of 2026-10-10 on #370 led to P7-22, and its outcome, with the owner's change of the rule, to ADR 0007 and P7-23. The Grill of 2026-10-10 on #374 led to P7-24. The review of #365 led to the spike P7-25, and P7-25 to P7-26 and P7-27. P7-27 led to P7-28 to P7-30. The review of #399 led to P7-31. The parity run of #404 led to the spike P7-32, and P7-32 to P7-33 to P7-36. P7-36 led to P7-37 to P7-39. The prototype of P7-38 led to P7-40, and P7-40 to P7-41 and P7-42 (withdrawn).

| ID   | Kind  | Title                                                                                   | Issue | Status      |
| ---- | ----- | --------------------------------------------------------------------------------------- | ----- | ----------- |
| P7-1 | Task  | Record the goal of parity with Qhull and CGAL, the baseline, and the method             | #310  | Done        |
| P7-2 | Spike | Delaunay D2: does a CGAL-style structure halve the insertion?                           | #311  | Done        |
| P7-3 | Docs  | Hull: `build()` computes only what the result needs; the rest on first use              | #312  | Done        |
| P7-4 | Feat  | Predicates: a first-stage bound with no subnormal product                               | #316  | Done        |
| P7-5 | Task  | Profile Delaunay D2 and D3 after P7-4 and name the next rows                            | #317  | Done        |
| P7-6 | Feat  | Hull: D2 facets in cycle order and the facet numbering on first use                     | #321  | Done        |
| P7-7 | Feat  | Hull D2: a faster sort of the chain's points                                            | #322  | Done        |
| P7-8 | Feat  | Acceptance: faster duplicate detection                                                  | #323  | Withdrawn after measurement (#323) |
| P7-9 | Feat  | Hull D2: a faster discard before the chain                                              | #324  | Done        |
| P7-10 | Feat | Delaunay: link the cavity's new simplices without turning around ridges                  | #329  | Done        |
| P7-11 | Docs | Delaunay: what `build()` computes; the public order of the simplices                     | #330  | Done        |
| P7-12 | Spike | Delaunay `sphere` D2: in-sphere tests that reach the exact stage                        | #331  | Done        |
| P7-13 | Spike | Delaunay D2: insertion order and point location against CGAL's                          | #332  | Done        |
| P7-14 | Feat | Delaunay: publish the simplices in construction order                                    | #335  | Done        |
| P7-15 | Docs | Predicates: a double-double stage between the first stage and the exact stage            | #349  | Done        |
| P7-16 | Feat | Predicates: a first stage for determinants of size 5 and 6                               | #350  | Done        |
| P7-17 | Feat | Predicates: a double-double stage for the dedicated formulas                             | #352  | Done        |
| P7-18 | Fix  | Predicates: the lifted underflow terms of the first-stage formulas of k ≤ 4              | #355  | Done        |
| P7-19 | Feat | Delaunay: the pass after the insertion at the cost of its tests                         | #362  | Done        |
| P7-20 | Spike | Delaunay D2: the insertion's own work against CGAL's                                   | #363  | Done        |
| P7-21 | Feat | Delaunay: the sites in the order of their insertion                                      | #366  | Done        |
| P7-22 | Spike | Delaunay D2: an insertion by edge flips                                                 | #370  | Done        |
| P7-23 | Feat | Delaunay D2: insert by edge flips (ADR 0007)                                            | #372  | Done        |
| P7-24 | Spike | Delaunay: one record per cell of the mesh                                              | #374  | Done        |
| P7-25 | Spike | Hull `cubesurf` and `grid`: where the time goes against Qhull                          | #368  | Done        |
| P7-26 | Feat | Predicates: a zero column of differences decides zero before the later stages           | #384  | Done        |
| P7-27 | Spike | Hull: exact planes of integer inputs, side tests by a dot product                       | #385  | Done        |
| P7-28 | Feat  | Hull: a cull plane from the exact direction when the filtered cofactors fail            | #392  | Done        |
| P7-29 | Feat  | Hull: placing keeps the boundary ridges across points                                   | #393  | Done        |
| P7-30 | Feat  | Hull: each face of the lattice classified once                                          | #394  | Done        |
| P7-31 | Feat  | Predicates: the exact direction's error bound from its derivation                       | #400  | Withdrawn after measurement (#400) |
| P7-32 | Spike | Where the time goes on the general sets left: Delaunay D2 and D3, hull sphere D2 to D4  | #408  | Done        |
| P7-33 | Feat  | Hull: planes of D = 3 and 4 by a fixed-size path                                        | #409  | Done        |
| P7-34 | Feat  | Hull: the coplanar merge on an input with no zero sign recorded                         | #410  | Withdrawn after measurement (#471) |
| P7-35 | Feat  | Hull D2: classify and publish a strict polygon without sorts or regrowth                | #411  | Done        |
| P7-36 | Spike | Delaunay D2 and D3: where the insertion's own time goes, line by line                   | #412  | Done        |
| P7-37 | Feat  | Delaunay D3: pair the new tetrahedra's faces at less cost                               | #421  | Withdrawn after measurement (#425) |
| P7-38 | Feat  | Predicates: a static filter for Delaunay's predicates                                   | #422  | Withdrawn after measurement (#428) |
| P7-39 | Feat  | Delaunay D2: the BRIO order at less cost                                                | #423  | Done        |
| P7-40 | Spike | Delaunay: where the predicates' call paths spend their time, line by line               | #427  | Done        |
| P7-41 | Feat  | Delaunay: the passes after insertion at less cost                                       | #429  | Done        |
| P7-42 | Feat  | Delaunay: the predicates' wrappers gather only what the first stage reads               | #430  | Withdrawn after review (#431) |
| P7-43 | Feat  | Hull D2: the points off the polygon in one pass, without regrowth                       | #440  | Done        |
| P7-44 | Spike | Delaunay D2 and D3 against CGAL, per operation and per cache miss                       | #444  | Done        |
| P7-45 | Feat  | Delaunay D2 and D3: the predicates on fixed-size rows, and ADR 0007's walk              | #446  | Done        |
| P7-46 | Spike | Delaunay D2 and D3: the first stage's bound on fixed rows, after P7-45                  | #447  | Done        |
| P7-47 | Spike | Delaunay: decide the faces the insertion leaves unknown without a test                  | #448  | Done        |
| P7-48 | Feat  | Delaunay: a face left unknown by an orientation, lifted only when dependent             | #451  | Done        |
| P7-49 | Spike | Delaunay D2 to D4 after P7-45 and P7-48: where the time goes                            | #454  | Done        |
| P7-50 | Docs  | Parity: CGAL's Delaunay reference with the same output                                  | #455  | Done        |
| P7-51 | Spike | Parity: do the hull's references produce the same output?                               | #456  | Done        |
| P7-52 | Feat  | Delaunay D2 and D3: the pass after the insertion in one walk                            | #457  | Withdrawn after measurement (#462) |
| P7-53 | Feat  | Delaunay D2 and D3: cells and sites in fixed-size arrays                                | #458  | Withdrawn after measurement (#462) |
| P7-54 | Spike | Where the time goes on the 13 sets the parity run of #466 left                          | #467  | Done        |
| P7-55 | Feat  | Hull: the recorded points of a face tested against its neighbors together               | #468  | Done        |
| P7-56 | Feat  | Delaunay: decide the faces left unknown when the insertion links them                   | #469  | Withdrawn after measurement (#473) |
| P7-57 | Spike | Hull sphere D2 10^4 and 10^5: acceptance, publication, and classification               | #470  | Done        |
| P7-58 | Feat  | Delaunay D2: a face left unknown needs no test                                          | #474  | Done        |
| P7-59 | Feat  | Hull D2: the chain's keys sorted by radix                                               | #477  | Withdrawn after measurement (#478) |
| P7-60 | Feat  | Delaunay: the pass after the insertion reads a finiteness flag, not the neighbor        | #483  | Withdrawn after measurement (#513) |
| P7-61 | Spike | Hull sphere D3 at 10^6: why the gap to Qhull opens                                      | #484  | Done        |
| P7-62 | Feat  | Hull: the merge's groups of one simplex and its dense index, at less cost               | #485  | Done        |
| P7-63 | Spike | Hull sphere D3: cache misses of construction against Qhull                              | #487  | Done        |
| P7-64 | Feat  | Hull D3: construction on a copy of the points in Morton order                           | #489  | Done        |
| P7-65 | Spike | Hull `cubesurf` D3 and `sphere` D2 10^4: where the time goes                            | #493  | Done        |
| P7-66 | Feat  | Hull D2 and D3: exact side tests through the fixed-size first stage                     | #494  | Withdrawn after measurement (#495) |
| P7-67 | Feat  | Hull D2: the duplicates found from the chain's sort, where the chain reads every point  | #496  | Done        |
| P7-68 | Spike | Hull `cubesurf` D3: the counts of construction and classification against Qhull       | #503  | Done        |
| P7-69 | Feat  | Hull D3: classification's shortcut for points inside a facet                          | #504  | Done        |
| P7-70 | Feat  | Predicates: the first stage skips its bound for an exactly zero determinant          | #506  | Done        |
| P7-71 | Spike | Delaunay `cube` D3 and hull `sphere` D2 under WSL: where the time goes against CGAL   | #509  | Done        |
| P7-72 | Feat  | Hull D2: the chain on the halves below and above the line through its ends             | #515  | Done        |
| P7-73 | Feat  | Delaunay D2 and D3: insertion along a Hilbert curve                                    | #510  | Withdrawn after measurement (#511) |

## R1 (the first release on crates.io, parent #592)

The owner opened this phase on 2026-10-12 (Grill on #592). The comparison it publishes is a report on standard data (`rbox`, the distributions of PBBS, the Stanford scans) against Qhull, CGAL, and Rust crates; the parity run of ADR 0006 stays the internal judgement.

| ID | Kind | Title | Issue | Status |
| --- | --- | --- | --- | --- |
| R1-1 | Task | Publish rules: crates.io, the license, and what 0.1.0 promises, in scope | #593 | Done |
| R1-2 | Task | The package: license files, Cargo metadata, and the files that ship | #594 | Done |
| R1-3 | Docs | The README and the crate documentation state what 0.1.0 is | #595 | Not started |
| R1-4 | Task | Audit the public items and their rustdoc | #596 | Not started |
| R1-5 | Task | The comparison's tools: one runner for several libraries, with time and memory | #597 | Done |
| R1-6 | Task | The comparison's data: rbox, the distributions of PBBS, and the Stanford scans | #598 | Done |
| R1-7 | Task | Rust crates in the comparison | #599 | Not started |
| R1-8 | Docs | Run the comparison and publish it: the document, the figures, and the numbers | #600 | Not started |
| R1-9 | Feat | The filter's exact scaling before the release (P5-63) | #601 | Done |
| R1-10 | Task | 0.1.0: the changelog, the version, and `publish = false` removed | #602 | Not started |
| R1-11 | Spike | Delaunay of a lattice in D = 2: three times CGAL's time | #614 | Done |
| R1-12 | Feat | Predicates: a zero proved by the first stage's own arithmetic where it is exact | #615 | Done |
| R1-13 | Spike | Delaunay of a lattice: what the cospherical groups cost after the zeros are proved | #617 | Done |
| R1-14 | Spike | The comparison's data screened once against CGAL and Qhull before it is run | #618 | Done |
| R1-15 | Feat | Delaunay: the insertion order refines a crowded cell of its grid | #620 | Done |
| R1-16 | Spike | Hull: what every point costs before construction, on inputs whose points are nearly all inside | #622 | Done |
| R1-17 | Feat | Hull: the first simplex from extreme points, D >= 3 | #624 | Done |
| R1-18 | Feat | Hull D3: the points inside the first simplex are discarded before the order in space | #625 | Withdrawn after measurement (#625) |
| R1-19 | Spike | Hull D2 of points in a ball: construction and classification | #628 | Done |
| R1-20 | Feat | Hull D2: a point off the chain is located by its coordinates and two orientations | #630 | Done |
| R1-21 | Feat | Delaunay D2: a group of four sites is written without the placing | #633 | Done |
| R1-22 | Spike | Delaunay groups: translates of a placed group, and the links inside a group, prototyped and measured | #634 | In progress |

## Intentionally out of scope

Items land here only through Grill → Issue.

- `representative` and `interior_points` computed on demand (#622; Grill of 2026-10-13, the owner's decision after the measurement). The duplicates of every point take 17 to 25 ms of a hull of 10^6 points in D = 2 and 3, a fifth to two fifths of those builds, and nothing on a sphere or from D = 4; without them the hull of 10^6 Kuzmin points still takes twice CGAL's time. Finding them when the lists are first read needs the hull to keep the input or to borrow it, which changes the public type, and moves where a failure is reported. 0.1.0 keeps the lists in the build, and the comparison states the reason a hull of points nearly all inside is slower than CGAL and Qhull. If it is wanted later, a builder option by which the caller states that the points are distinct is an addition and not a break.
- A Voronoi diagram for sites whose coordinates spread over hundreds of orders of magnitude (#180; the owner's decision of 2026-10-12). Of 3,739 inputs with such coordinates on which Voronoi returned `NonFiniteCircumcenter`, 76% have a vertex beyond `f64`, which design §8 makes a failure, and 24% have every vertex inside `f64` and fail because the `f64` solve of the circumcenter is not accurate on systems that mix those scales. Reaching them needs a solve that scales each row and column, or an exact one. No parity set and no generator family is near such inputs. It returns if a user's input fails this way.
- A parallel hull build (#256, #269; Grill of 2026-10-07 on #269, option A). The round protocol lost to the sequential build, and of the coarse-grained strategies only parallel publication was never slower, with a ceiling of about 0.78 of the sequential build on four cores. The single-thread construction decides the comparison with Qhull, so no parallel path or `parallel` switch is added now. It returns through a Grill with a measurement that meets a stated criterion (`docs/adr/0004-sequential-hull-only.md`). A spike is such a measurement and adds no path to the crate: P6-14 (#276) asks whether a ParGeo-style fully parallel hull beats the sequential build, and its answer goes to a Grill.
- Lock-free allocation (was P5-2, #26). Construction inserts one point at a time on one thread (§6), so no insert or remove in the facet store is concurrent. On the P2-7 `cube` sets measured in #26, inserts and removes in the arena of that time took under 1% of the build. It returns through Grill with a parallel construction.
