# Roadmap

Procedure: [CONTRIBUTING.md](../CONTRIBUTING.md). Design: [design.ja.md](design.ja.md) and [design.md](design.md). When the two design files differ, follow `design.ja.md`.

Acceptance text stays on each Issue and in the design (§10 and §11). This file keeps the ID, title, Issue, and status.

## Current work

P5-16 (#120): hull `cube` D6 10^4 on one core within 1.39 s, taken in measured steps. P5-4 (#28) follows; the measurement on #28 bounds its gain on the `cube` sets.

## Dependencies

```text
M0 → P1 → P2 → P3
              → P4
              → P5
```

P3, P4, and P5 follow a correct sequential hull (P2). P4 does not wait for P3. P5 does not gate P4. The sign convention and the lift formula are part of P4 from the start. P5-3 and P5-4 are the omissions the design allows after that. They are not out of scope.

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

| ID    | Kind | Title                                                         | Issue | Status                 |
| ----- | ---- | ------------------------------------------------------------- | ----- | ---------------------- |
| P5-1  | Feat | Wider SIMD, including AVX-512                                 | #25   | Done                   |
| P5-3  | Feat | Cache of lifted coordinates                                   | #27   | Done                   |
| P5-4  | Feat | Cut off search of the upper hull and of faces already decided | #28   | Not started            |
| P5-5  | Feat | Vector cull kernel faster than its scalar lanes               | #70   | Set after Grill on #70 |
| P5-6  | Feat | Filtered determinant cheap for every k > 4                    | #72   | Done                   |
| P5-7  | Feat | Faster distance-zero classification                           | #73   | Done                   |
| P5-8  | Feat | Linear batch extraction                                       | #75   | Done                   |
| P5-9  | Feat | Certified distance proves a strict side                       | #79   | Done                   |
| P5-10 | Feat | Visibility search without SipHash                             | #85   | Done                   |
| P5-11 | Feat | Cheaper certification of a new facet's working normal         | #86   | Done                   |
| P5-12 | Feat | Faster exact stage for predicates the filter cannot decide    | #96   | Done                   |
| P5-13 | Feat | Certified cull and strict-side proof for lifted facets        | #109  | Done                   |
| P5-14 | Feat | Skip the visibility walk for candidates a round rejects       | #110  | Done                   |
| P5-15 | Feat | One shared elimination for a facet's cofactors                | #111  | Done                   |
| P5-16 | Feat | Hull cube D6 10^4 within 1.39 s on one core                   | #120  | In progress            |
| P5-19 | Feat | Factor the published normal with one Householder QR           | #135  | Done                   |

## Intentionally out of scope

Items land here only through Grill → Issue.

- Lock-free allocation (was P5-2, #26). The commit runs on one thread in ascending input index (§6), so no arena insert or remove is concurrent, and on the P2-7 `cube` sets measured in #26 they took under 1% of the build. It returns through Grill if the commit becomes parallel.
