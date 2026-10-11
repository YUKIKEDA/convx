# 0008. Delaunay may have dedicated paths for D = 2 and D = 3

## Status

Accepted. Amends ADR 0005 and ADR 0007.

## Date

2026-10-10

## Issue

#444 (the spike P7-44, and the Grill of 2026-10-10 on it). Implementation of the first path: #446 (P7-45).

## Context

ADR 0005 specialized the Delaunay insertion by compiling one procedure per dimension, and did not choose code written by hand per dimension. ADR 0007 added a second insertion core for D = 2. It did not choose a dedicated D = 2 publication.

After the parity run of #438, 13 Delaunay sets were still not met. The spike P7-44 (#444, `docs/bench.md`) measured CGAL at the grain of convx's profiles on `cube` D2 and D3 at 10^5 sites:

- CGAL's in-circle and in-sphere tests match convx's within 4%.
- Its last-level cache misses are the same in D3.
- One convx build runs 2.75 times CGAL's instructions in D2 and 2.12 times in D3.
- Most of the excess is the entry of the predicates. The fixed shapes of ADR 0005 reach the formulas through the generic first stage, on slices: about 127 instructions per orientation, against about 18 in CGAL's static filter.
- A prototype with a fixed-size entry read 0.88 to 0.89 of `main` on D2 and 0.92 on `cube` D3 10^5.
- After it, the pass after the insertion is still 14 to 17% of `build()`.

D = 2 and D = 3 are the dimensions used most (ADR 0005). CGAL itself has classes written for each of them.

## Decision

Decided by the owner on 2026-10-10, in the Grill on #444:

- For D = 2 and D = 3, Delaunay may have dedicated paths, optimized on their own:
  - the insertion;
  - the entry of the predicates;
  - the pass after the insertion;
  - publication.
- Each dedicated path returns what the generic path returns:
  - the same exact signs;
  - the same simplices, neighbors, and split of cospherical groups.
- A dedicated path is checked against the generic one:
  - by a test that compares what the two publish, as ADR 0005 and ADR 0007 require for the insertion;
  - or, for a predicate, by a debug assertion on every call.
- Each dedicated path is its own row, with its own keep criterion (`.cursor/rules/bench.mdc`).
- `.cursor/rules/hpc.mdc` still holds: points stay row-major. A dedicated path reads a site's coordinates as a fixed-size array from those rows, and keeps no second copy of the sites.
- This amends ADR 0005, which kept one source, and ADR 0007, which did not choose a dedicated D = 2 publication.

The first dedicated path is the entry of the predicates (P7-45, #446):

- `Plane` and `Space` read each site as an array of D coordinates and call the first-stage formula on those arrays.
- Their wrappers build the ids of a predicate in arrays of a fixed size.
- `locate` skips the face it came through, as ADR 0007 decided.

Not chosen:

- keeping one source for every dimension (ADR 0005), which leaves the generic entry's cost on the dimensions used most;
- computing the published rows on first use, which moves the pass after the insertion out of `build()` without making it cheaper (Grill on #444).
