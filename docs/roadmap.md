# Roadmap

Procedure: [CONTRIBUTING.md](../CONTRIBUTING.md). Design: [design.ja.md](design.ja.md) and [design.md](design.md). When the two design files differ, follow `design.ja.md`.

Acceptance text stays on each Issue and in the design (§10 and §11). This file keeps the ID, title, Issue, and status.

## Current work

None. The Grill of 2026-10-07 on the P6-11 spike (#269) chose to add no parallel hull now. The next row is named with the measurement of convx after P6.

## Dependencies

```text
M0 → P1 → P2 → P3
              → P4
              → P5
              → P6
```

P3, P4, and P5 follow a correct sequential hull (P2). P4 does not wait for P3. P5 does not gate P4. P6 replaces the construction core, the predicate filter, and the result storage after the measurements of P5; its rows come before the remaining P5 rows. The sign convention and the lift formula are part of P4 from the start.

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
| P5-40 | Docs  | Let the filter rescale tiny inputs by an exact power of two                                               | #179  | Set after Grill on #179  |
| P5-41 | Spike | How many NonFiniteCircumcenter failures have a circumcenter that fits in f64?                             | #180  | Not started              |
| P5-42 | Spike | Why does hull sphere D2 wall time grow faster than its instruction count?                                 | #182  | Done                     |
| P5-43 | Spike | Coplanar points are scanned again for every new facet: can construction stop rescanning them?             | #183  | Done                     |
| P5-44 | Feat  | Reuse the work space of the heap exact stage across predicate calls                                       | #185  | Not started              |
| P5-45 | Spike | Near-cospherical sites send almost every lifted predicate to the exact stage                              | #187  | Not started              |
| P5-46 | Feat  | Build Delaunay and Voronoi by incremental insertion in every dimension (design #188)                      | #189  | Done                     |
| P5-47 | Feat  | Deterministic parallel insertion for Delaunay and Voronoi                                                 | #190  | Set after Grill on #190  |
| P5-48 | Spike | Where does hull sphere time go at D = 3 to D = 6?                                                         | #199  | Done                     |
| P5-49 | Spike | Why is the low-dimension cube hull slower than Qhull's compute time?                                      | #200  | Done                     |
| P5-50 | Bug   | Run the filtered elimination under AVX2, not AVX-512 (the D = 8 lifted 9 x 9 was 24x slower)              | #195  | Done                     |
| P5-51 | Feat  | Vectorize the filtered elimination of size 10 and up (heap rows)                                          | #203  | Not started              |
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
| P6-12 | Docs | Let construction record the planes a point lies on                                   | #236  | Set after Grill on #236 |

## Intentionally out of scope

Items land here only through Grill → Issue.

- A parallel hull build (#256, #269; Grill of 2026-10-07 on #269, option A). The round protocol lost to the sequential build, and of the coarse-grained strategies only parallel publication was never slower, with a ceiling of about 0.78 of the sequential build on four cores. The single-thread construction decides the comparison with Qhull, so no parallel path or `parallel` switch is added now. It returns through a Grill with a measurement that meets a stated criterion (`docs/adr/0004-sequential-hull-only.md`). A spike is such a measurement and adds no path to the crate: #276 asks whether a ParGeo-style fully parallel hull beats the sequential build, and its answer goes to a Grill.
- Lock-free allocation (was P5-2, #26). Construction inserts one point at a time on one thread (§6), so no insert or remove in the facet store is concurrent. On the P2-7 `cube` sets measured in #26, inserts and removes in the arena of that time took under 1% of the build. It returns through Grill with a parallel construction.
