# 0009. Hull D = 3 construction runs on a copy of the points in space order

## Status

Accepted. Makes an exception to `.cursor/rules/hpc.mdc`'s rule against a second copy of the points, for the hull of D = 3.

## Date

2026-10-11

## Issue

#487 (the spike P7-63, and the Grill of 2026-10-11 on it). Implementation: #489 (P7-64).

## Context

Hull `sphere` D3 read 1.04 against Qhull at 10^5 and 1.22 at 10^6 (#479). The spike P7-61 (#484) found that the parts that grow with the input read memory at random:

- the outside sets' scan reads point coordinates by index;
- the side tests read each simplex's cull plane.

The spike P7-63 (#487) counted the misses inside `build()` at 10^6: 96.5 M last-level read misses and 30.0 M write misses. Qhull, with reading its file included, had 75.9 M and 13.6 M. At 10^5 convx missed less than Qhull.

The outside sets hold point indices. In the input's order, the points of one outside set lie anywhere in the input, and at 10^6 the input does not fit in the last-level cache. Given the points in Morton order instead, with the cost of ordering them left out, construction read:

- 0.82 on hull `sphere` D3 10^6 and on `cube` D3 10^6;
- 0.84 on `cubesurf` D3 10^5;
- 0.95 to 1.02 at 10^5;
- no gain in D = 4.

`.cursor/rules/hpc.mdc` forbids keeping a second copy of the points so that a gather becomes contiguous. Its one exception is the Delaunay sites.

## Decision

Decided by the owner on 2026-10-11, in the Grill on #487:

- **The copy.** For D = 3 and from `SPACE_ORDER_FROM` representatives (2^17), construction runs on a copy of the representatives in Morton order. The order comes from a counting sort over a grid of a few points per cell; the points of a cell keep the order of their indices. The copy's points carry consecutive labels.
- **Back to the input.** After construction, the built hull is renamed back to the input's indices:
  - the simplices' vertices;
  - the outside sets and their candidates;
  - the points proved interior;
  - the records.

  Classification and publication then read the input as before.
- **The same output.** The published hull does not depend on the order of construction (design §6), so the output is the same. A test checks it through the public API, on general and degenerate inputs, with the threshold lowered.
- **The rule.** This is the second exception to `.cursor/rules/hpc.mdc`'s rule, after the Delaunay sites. The copy holds the representatives' coordinates once, row-major, and lives only during construction.
- **The threshold.** At 10^5 the copy's cost was about what it saved: `cube` D3 10^5 read 1.08 with a threshold of 2^16. So the copy starts at 2^17 representatives.

Not chosen:

- a sort of a 128-bit key per point, which cost more than it saved on `cube` D3 10^6 (1.12);
- reusing the outside sets' buffers, which saved nothing (#487);
- D = 4 and above, where the order did not matter.
