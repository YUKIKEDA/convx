# Roadmap

Procedure: [CONTRIBUTING.md](../CONTRIBUTING.md). Design: [design.ja.md](design.ja.md) and [design.md](design.md). When the two design files differ, follow `design.ja.md`.

Acceptance text stays on each Issue and in the design (§10 and §11). This file keeps the ID, title, Issue, and status.

## Current work

M0-1. No Issue yet.

## Dependencies

```text
M0 → P1 → P2 → P3
              → P4
              → P5
```

P3, P4, and P5 follow a correct sequential hull (P2). P4 does not wait for P3. P5 does not gate P4. The sign convention and the lift formula are part of P4 from the start. P5-3 and P5-4 are the omissions the design allows after that. They are not out of scope.

## M0

| ID   | Kind | Title                   | Issue | Status      |
| ---- | ---- | ----------------------- | ----- | ----------- |
| M0-1 | Task | Crate, justfile, and CI |       | Not started |

## P1

Predicate kernel.

| ID   | Kind | Title                                                                                                                          | Issue | Status      |
| ---- | ---- | ------------------------------------------------------------------------------------------------------------------------------ | ----- | ----------- |
| P1-1 | Feat | Predicate kernel: orientation, distance sign, coplanar, error bound, exact-sign fallback, k ≤ 4 formulas, filtered determinant |       | Not started |
| P1-2 | Feat | Unit normal by Householder QR                                                                                                  |       | Not started |
| P1-3 | Feat | Single-threaded generational arena                                                                                             |       | Not started |
| P1-4 | Feat | SIMD distance cull                                                                                                             |       | Not started |
| P1-5 | Test | Phase 1 predicate inputs                                                                                                       |       | Not started |

## P2

Sequential Quickhull.

| ID   | Kind | Title                                                                    | Issue | Status      |
| ---- | ---- | ------------------------------------------------------------------------ | ----- | ----------- |
| P2-1 | Feat | Sequential Quickhull through insertion, kept simplicial                  |       | Not started |
| P2-2 | Feat | Merge coplanar simplices into logical facets after insertion             |       | Not started |
| P2-3 | Feat | Distance-zero classification and the index partition                     |       | Not started |
| P2-4 | Feat | Published hull surface: plane, triangulation, boundary cycle, and volume |       | Not started |
| P2-5 | Test | Phase 2 hull inputs and invariants                                       |       | Not started |
| P2-6 | Test | Deterministic hull simulations and frozen fixtures                       |       | Not started |
| P2-7 | Task | Deterministic point sets for the Qhull timing comparison                 |       | Not started |

## P3

Parallel commit. The initial simplex is the same function as in P2.

| ID   | Kind | Title                                                                   | Issue | Status      |
| ---- | ---- | ----------------------------------------------------------------------- | ----- | ----------- |
| P3-1 | Feat | Batch extraction shared with the sequential build                       |       | Not started |
| P3-2 | Feat | Reserve T and H, and drop conflicts on prospective simplices            |       | Not started |
| P3-3 | Feat | Worker-local mutation and commit in input-index order                   |       | Not started |
| P3-4 | Test | Debug agreement of sequential batch application and the parallel commit |       | Not started |

## P4

Static API, Delaunay, Voronoi, and oracles. Pulling triangulation and the cospherical merge are internal to this phase.

| ID   | Kind | Title                                                              | Issue | Status      |
| ---- | ---- | ------------------------------------------------------------------ | ----- | ----------- |
| P4-1 | Feat | Delaunay as the lower hull of the lift                             |       | Not started |
| P4-2 | Feat | Pulling triangulation when the lift is flat                        |       | Not started |
| P4-3 | Feat | Voronoi dual, merging cospherical simplices into one vertex        |       | Not started |
| P4-4 | Feat | Static API                                                         |       | Not started |
| P4-5 | Test | Oracles and the Phase 4 Delaunay inputs                            |       | Not started |
| P4-6 | Test | Deterministic Delaunay and Voronoi simulations and frozen fixtures |       | Not started |

## P5

After a correct sequential hull. These rows do not block P4.

| ID   | Kind | Title                                                         | Issue | Status      |
| ---- | ---- | ------------------------------------------------------------- | ----- | ----------- |
| P5-1 | Feat | Wider SIMD, including AVX-512                                 |       | Not started |
| P5-2 | Feat | Lock-free allocation                                          |       | Not started |
| P5-3 | Feat | Cache of lifted coordinates                                   |       | Not started |
| P5-4 | Feat | Cut off search of the upper hull and of faces already decided |       | Not started |

## Intentionally out of scope

Items land here only through Grill → Issue.
