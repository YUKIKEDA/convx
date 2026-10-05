# 0001. Build Delaunay by incremental insertion

## Status

Accepted

## Date

2026-10-05

## Issue

#188 (design), from the spike #175. Implementation: #189.

## Context

Design §7 defines the Delaunay triangulation as the lower hull of the sites lifted by $x_{D+1} = |x|^2$, projected back. Until this decision it was also built that way: the hull core (Quickhull, §6) ran on the lifted sites. The definition is Qhull's construction too. Every lifted site is extreme, so the outside sets stay large for the whole build, and the lifted hull also builds and then discards its upper side.

After #172, #173, and #174, the lifted path was still several times slower than Qhull on Delaunay `cube` D3 10^5, and it grew faster than n at D2. CGAL's Delaunay builds by inserting sites one at a time in a spatial order. Published comparisons put it several times faster than Qhull in 3D.

Spike #175 wrote a throwaway incremental insertion with convx's own exact predicates:

- Bowyer–Watson in BRIO order
- a walk from the last created simplex
- ghost simplices for the outside of the site hull

The spike was timed against the lifted path at #186's head (release, one core, `cube` seed 1):

| Build | Insertion | Lifted Quickhull | Qhull `qdelaunay` |
| --- | --- | --- | --- |
| D2 10^5 | 0.26 s | 1.29 s | 0.69 s |
| D2 10^6 | 2.95 s | 16.7 s | 8.41 s |
| D3 10^5 | 2.34 s | 5.07 s | 3.08 s |
| Peak RSS, D3 10^5 | 43 MB | 414 MB | 171 MB |

On those sets it gave the identical simplex set. It did not yet handle degenerate input: a site on a hull facet's plane, walk termination, and the flat lift.

## Decision

Delaunay and Voronoi are built by incremental insertion, in every dimension. The definition in §7 does not change: unique simplices are those of the lower hull of the lift, and the signs are the same exact lifted orientations. The hull keeps Quickhull. Decided in the Grill on #175:

- `parallel(true)` runs the same sequential insertion for Delaunay and Voronoi. One binary then gives the same simplices for both settings, diagonals included, as §6 and §7 promise. The cost is that Delaunay gains nothing from more cores. A deterministic parallel insertion is a later roadmap row (P5-47, #190), set after its own Grill.
- The flat lift keeps the pulling triangulation of §7. It is known before construction, so it never reaches the insertion.
- The design fixes only a deterministic insertion order chosen by the implementation, not BRIO or a curve. Diagonals inside cospherical groups are promised only within one binary, as before.
- Voronoi keeps building the site hull with the hull core for its rays. That was 0.3% to 2.2% of a Voronoi build when measured.
- Code used only by the lifted Quickhull of Delaunay is removed with the change:
  - the hull core on lifted sites
  - the lifted-height cache (P5-3)
  - the upper-hull cutoff (P5-4)
  - the certified cull of lifted facets (#109)

  The lifted orientation predicate stays: §7 defines cospherical with it, and §8 merges with it.
- If insertion is slower than the lifted path in some dimension, that dimension comes back to Grill before the change merges.
