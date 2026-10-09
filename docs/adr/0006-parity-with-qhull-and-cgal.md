# 0006. Phase P7 aims at parity with Qhull and CGAL on every benchmark set

## Status

Accepted

## Date

2026-10-09

## Issue

#310 (P7-1), from the Grill of 2026-10-09. The spike that follows: #311 (P7-2). The design row that follows: #312 (P7-3).

## Context

P6 rebuilt the construction core, the predicate filter, and the result storage. Some sets are still behind a reference. The baseline at the start of P7 (`docs/bench.md`, Baseline of P7) gives these ratios of `build()` to the reference with the same output:

| Sets | Reference | Ratio |
| --- | --- | ---: |
| Delaunay D = 2 and D = 3 | CGAL | 1.97 to 3.81 |
| Hull `sphere` D = 2 | CGAL `convex_hull_2` | 2.53 to 3.25 |
| Hull `cube` D = 2 | CGAL | 1.21 to 1.46 |
| Hull `sphere` D = 3 and D = 4 | Qhull | 1.33 to 1.56 |
| Delaunay D = 4 and D = 5 | CGAL | 1.17 to 1.52 |
| Hull `sphere` D = 5 | Qhull | 1.06 to 1.09 |
| Hull `cubesurf` D3 10^5 | CGAL, Qhull | 4.51, 5.90 |
| Hull `grid` D6 10^4 | Qhull | 10.48 |

Nine sets are met already, all of the hull: `cube` D = 3, D = 5, and D = 6 at every size, `cube` D4 10^4, and `sphere` D6 10^4.

Until now a speed row had a keep criterion against `main` and reported the ratio to Qhull or CGAL with no target (`.cursor/rules/bench.mdc`: no target before Qhull is measured). Qhull and CGAL are now measured on the owner's machine.

## Decision

The owner decided in the Grill of 2026-10-09:

- **Goal.** convx is at least as fast as Qhull and CGAL on every benchmark set. Phase P7 holds the rows that work toward it.
- **Sets.** Every set below, each from `benches/sets.txt`:
  - the hull sets of `docs/bench.md`: `cube` D2 to D6, `sphere` D2 to D6, at 10^4 to 10^6 points as listed there;
  - the Delaunay sets of `docs/bench.md`: `cube` and `sphere` D2 and D3 at 10^4 to 10^6 sites, D4 and D5 at 10^4;
  - hull `cubesurf` D3 10^5, whose points are uniform on the surface of [-1, 1]^3;
  - hull `grid` D6 10^4.
- **Judgement.** A set is met when the median of convx's `build()` is at most the median of each reference whose output is the same, in one run of the method below. The time of `planes()` after `build()` is reported next to it and is not judged. A facet's plane is computed on first use (#297). Qhull's `i` output and CGAL's polyhedron carry no plane equations.
- **Same output.**
  - A reference whose output differs is reported with a note, as a reference value, and is not judged against:
    - Qhull on hull `sphere` D2 and on every Delaunay `sphere` set, where it merges vertices or cospherical regions;
    - CGAL on hull `grid` D6 10^4, whose d-dimensional hull lists 3,661 vertices and 124,682 facets, where the hull has 89 vertices and 17 facets.
  - CGAL's `convex_hull_2` returns the hull points in order. It is judged against, and P7-3 (#312) changes what convx's `build()` computes so that the work is comparable.
  - On `cubesurf`, CGAL's triangulated polyhedron is judged against. It has the same 155 vertices, and convx publishes a triangulation of the boundary too.
- **Method.**
  - The owner's machine (Intel Core i5-13400F), Ubuntu 22.04 under WSL2, every process pinned with `taskset -c 2`.
  - convx is built `--release` with debug info and the baseline target. CGAL 5.4 is built with g++ 11.4 at `-O3`, without `-march`. Qhull is 2020.2 from `qhull-bin`, run as `qconvex` or `qdelaunay` `i s TI`; its compute time is read.
  - Every tool reads the same file, written by `export_qhull_sets`. Tools alternate per round.
  - The procedure is in `docs/verification.md` (Parity run of P7).
- **Constraints.** Exact predicates, determinism, no `unsafe`, and one thread (ADR 0004) stay. The public API is not released. A breaking change may be proposed, and it goes through a Grill like any design change.
- **A gap that cannot be closed** inside these constraints goes back to a Grill with its measurement. A row does not lower the goal, drop a set, or change the judgement.
- **Order.**
  1. Delaunay D = 2 and D = 3.
  2. Hull D = 2.
  3. Hull `sphere` D = 3 and D = 4.
  4. The degenerate sets.
  5. Delaunay D = 4 and D = 5.
  6. What remains.

  Delaunay D = 2 starts with a spike (#311), because the gap of about three times may need another data structure. The hull starts with the design of what `build()` computes (#312), in every dimension.
- **Keep criteria.** A speed row of P7 keeps its own keep criterion against `main` (`.cursor/rules/bench.mdc`). It reports its sets against this goal as met or not met, by this method.

Not chosen:

- Judging against the faster reference only. That is the same as judging against both, and less direct.
- Requiring each win to be beyond the spread. Small sets are noisy, and the median judges them more stably.
- Building every tool with `-march=native`. A library is distributed for a baseline target, and earlier measurements used the baseline target.
- Making a row per gap now. The spike and the design change what the later rows contain.
