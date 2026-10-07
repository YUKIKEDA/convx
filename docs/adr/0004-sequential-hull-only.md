# 0004. The hull is built sequentially only

## Status

Accepted

## Date

2026-10-07

## Issue

#266 (design), from the measurement on #256 under the follow-up of the Grill of 2026-10-07 (Q4). Implementation: #267. The spike on other parallel strategies: #269.

## Context

ADR 0003 kept a parallel hull only if `parallel(true)` on four threads beat the sequential build beyond the spread, on every hull set of `docs/bench.md` whose output has at least 10^4 facets. Otherwise `parallel(true)` would run the sequential build.

The parallel build was the round protocol of design §6:

- the first K = 64 candidates by outside distance;
- the T and H reservation;
- plans on rayon's pool;
- a commit in ascending input index.

#256 measured it on the facet store of #253, on a 4-core machine. The tables are in `docs/bench.md` (Parallel round protocol against the sequential hull).

- Four threads beat the sequential build beyond the spread on 2 of the 15 sets, lost on 5 (up to 1.47 times slower on sphere D3 10^4), and were inside the spread on 8.
- Selection and the ordered commit alone took 59% (sphere D3 10^5) and 49% (cube D6 10^4) of the sequential build's whole in-place insertion. Both are serial by the protocol's design.
- With only the plans parallel, Amdahl's bound at four threads was about 0.84 and 0.80 of the sequential build on those sets.
- The plans themselves barely scaled. A round's plans took about 0.25 ms, and the pool went to sleep during each round's serial selection and commit. Waking it cost about as much as the plans. The same machine scaled independent work 3.9 times.
- D = 2 has no parallel construction at all, so `parallel(true)` there could only add overhead.

## Decision

- The hull is built only by the sequential in-place construction.
- The batch extraction, K, the T and H reservation, the conflict check, and the ordered commit are removed from the specification (design §6, §10, §11) and from the code (#267).
- The `parallel` flag is removed from all three builders. Kept, it would be a switch that does nothing. The crate is unpublished (version 0.0.0, `publish = false`), so no caller needs it kept for compatibility. Delaunay and Voronoi already ran one sequential insertion whatever its value.
- With the flag go its debug comparison and its tests, and design §10's Parallel column. The column returns, as times at 1, 2, 4, 8, and 16 threads, with a parallel construction.
- `rayon` leaves the library's dependencies.

A parallel hull construction, with whatever public switch it needs, returns only through a Grill, with a measurement that it beats the sequential build under the criterion above. Coarse-grained schemes are admissible because the published hull does not depend on the insertion order (ADR 0003), and #269 measures them before any design work:

- hulls of chunks followed by a final hull;
- parallel merge, classification, and publication;
- parallel initial assignment.
