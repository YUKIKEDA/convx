# 0007. Delaunay D = 2 inserts by edge flips

## Status

Accepted

## Date

2026-10-10

## Issue

#370 (the spike P7-22, and the Grill of 2026-10-10 on it). Implementation: #372 (P7-23). Amends ADR 0005 for D = 2.

## Context

ADR 0005 specialized the Delaunay insertion for D = 2 and D = 3 by compiling one procedure per dimension, and chose not to write a second core. After the rows that removed costs from that procedure (P7-19 to P7-21), Delaunay D2 was still 1.96 to 2.62 times CGAL on its six sets (#361).

- The spike P7-20 (#363) found that convx does CGAL's in-circle tests, about 9 a site on `cube` and 4 on `sphere`, and that its predicates are about a tenth of the mesh's time.
- CGAL's D = 2 class inserts a point into its triangle and flips edges. convx digs a cavity in every dimension, as CGAL does in 3D.
- A flip insertion for D = 2, outside the crate, with convx's order, stored sites, and predicates, built the mesh in 0.69, 0.67, 0.67, and 0.65 of the cavity's time on Delaunay `cube` and `sphere` D2 at 10^5 and 10^6. That is faster beyond the spread on all four sets, with the same triangles (`docs/bench.md`, "Delaunay D2: an insertion by edge flips").
- Its in-circle tests and flips per site were within 2% of CGAL's. The saving is in what the cavity writes and links: four freed and six new triangles a site and the turns around their ridges, against three new triangles and three flips.

The Grill had set a rule of at most 0.6 on each set before the measurement. The owner changed it after the measurement: 0.65 to 0.69 on all four sets, beyond the spread, is enough (#370).

## Decision

Decided by the owner on 2026-10-10, in the Grill on #370 and after the spike:

- Delaunay D = 2 inserts by edge flips. It walks from the last insertion without testing the face it came through, and inserts the site into its triangle (three triangles), or on an edge (four). It then flips every edge whose far vertex is in conflict with the site. The vertex at infinity is a vertex like the others, and a triangle at infinity is in conflict when the site is strictly beyond its hull edge, or strictly inside it.
- This is a second insertion core, for D = 2 only, which ADR 0005 did not choose. D >= 3 and the flat lift keep the generic insertion. The generic insertion still runs D = 2 under a test-only switch.
- The flip insertion uses the generic insertion's order (BRIO), its stored sites (in insertion order, #366), and its exact predicates. It publishes the same simplices, neighbors, and split of cospherical groups. A test compares the two on general position, grids, lattices with duplicates, and sites near and on one circle, as ADR 0005's test does.
- The published order is the flip insertion's construction order (design §7). It is deterministic and does not depend on the order the sites are stored in. It differs from the generic insertion's, which no test fixes (ADR 0005, #342).
- The flip insertion emits the mesh the pass after the insertion reads: vertices, neighbors, and the record of each face. A face whose in-circle test was zero during the flips is recorded cospherical, and one known to be strictly convex is recorded distinct. The pass after the insertion and publication are shared.
- Keep criterion of #372: faster beyond the spread on Delaunay `cube` and `sphere` D2 at 10^5 and 10^6, and nothing slower beyond the spread on the shorter runs of `docs/verification.md`.

Not chosen: a dedicated D = 2 publication, which would be a second copy of the pass after the insertion. Also not chosen: flips for D = 3, where CGAL digs a cavity too.

This does not reach CGAL at D = 2 by itself. On `cube` D2 10^6, the prototype's mesh took 414 ms; the order and the copy took about 90 ms, and the pass after the insertion about 160 ms. CGAL's whole build takes 471 ms (Linux).
