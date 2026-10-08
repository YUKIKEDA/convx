# 0005. The Delaunay insertion is specialized for D = 2 and D = 3

## Status

Accepted

## Date

2026-10-08

## Issue

#288 (design), from the measurement of P6-13 (#275) and the profile of P6-15 (#285). Implementation: #294.

## Context

Delaunay runs one incremental insertion for every dimension (ADR 0001, design §7). Its mesh, its sites, and its predicates take the dimension as a value: rows are slices, a predicate dispatches on the number of its rows, and the rows of a predicate are assembled per call.

Measured after P6 (`docs/bench.md`, After P6):

- `build()` is 4.3 to 4.9 times CGAL at D = 2 and 2.0 to 3.7 at D = 3.
- It is 1.17 to 1.52 times CGAL at D = 4 and D = 5, where CGAL runs a dimension-generic triangulation too. At D = 2 and D = 3 CGAL runs classes written for that dimension.

Profiled on Delaunay `cube` at 10^5 sites (`docs/bench.md`, Profile after P6):

- Insertion is 73.5% (D = 2) and 76.8% (D = 3) of `build()`.
- Predicates are about half of `build()`: the in-sphere test 32.5% and 37.1%, and the orientation of point location 18.5% and 11.3%.
- At D = 2 the in-sphere test takes 2.4 s of 7.47 s, and the formula itself, with what it calls, 1.5 s of that. The rest, more than a third, is the dimension-generic entry above the formula.

D = 2 and D = 3 are the dimensions used most.

## Decision

Decided by the owner on 2026-10-08 (option A) and in the Grill of that day on #288:

- The incremental insertion is parameterized by the dimension and compiled separately for D = 2 and for D = 3, and once for every other dimension. The specialized instantiations hold coordinates in fixed-size rows and call the predicate formulas directly.
- There is one source. P6 keeps no second core beside the one it replaces, so modules written by hand per dimension were not chosen.
- Every instantiation runs the same insertion order and decides every conflict and every orientation by the same exact signs. It therefore publishes the same triangulation, the diagonals among cospherical sites included, and design §7 says so in one paragraph. The public API does not change.
- The agreement of the instantiations is a test. A test-only switch runs D = 2 and D = 3 through the dimension-generic instantiation, and tests compare the published results on general, cospherical, grid, and flat-lift inputs. Debug builds do not run both: that would double every Delaunay test at D = 2 and D = 3. The exact oracles run on what the specialized instantiation publishes.
- Voronoi reads the Delaunay result, so it is expected to gain with it; no Voronoi timing exists yet, and #294 measures it. The flat-lift path, which calls the hull core, is not specialized.
- The specialization is kept only if `build()` is faster beyond the spread on every Delaunay set of D = 2 and D = 3 from 10^5 sites, with nothing slower beyond the spread. Otherwise it is not merged, and #294 records the measurement. There is no speedup target; the ratio to CGAL is reported.

Not chosen: specializing only the entry of the predicates, which leaves the generic cost of point location and of the cavity; and dedicated paths for the hull at D = 3, whose cost is elsewhere (the planes of new facets and the pass after construction).

This does not reach CGAL at D = 2 by itself. The entry around the in-sphere formula is about 12% of `build()` there (0.9 s of 7.47 s), and `build()` is 4.3 to 4.9 times CGAL. What else differs (the insertion order, point location) is not measured yet and is not part of this decision.
