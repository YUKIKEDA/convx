# 0002. Publish the certified cofactor direction as the unit normal

## Status

Accepted

## Date

2026-10-06

## Issue

#210 (design), from the spike #199. Implementation: #212.

## Context

Design §1 had the public unit normal of a facet with three or more points come from Householder QR of its scaled edge matrix: first `faer`'s, then this crate's own (#135). QR is backward stable, but it carries no error bound of its own. When one edge is nearly parallel to the span of the others, the null direction it returns can lie far from the true normal (#33). So every QR normal was compared with the facet's unit cofactor direction. The filter certifies that direction with an error bound ε ≤ 10^-10, or it is computed exactly. The QR normal was replaced by that direction when the two differed by more than ε + 10^-8. The published bound was ‖n − ĉ‖ ≤ 10^-8 + 2·10^-10.

The working normal used by distance scans was already that certified direction (#122), and a two-point facet already published it. Spike #199 found the published path evaluating the cofactors again and then running QR. On hull `sphere` D = 3 to D = 6 at 10^4, QR and its orientation took 9 to 10% of `build()`.

Accuracy on the same builds, as the distance (up to sign) from the exactly computed cofactor direction, rounded once:

| Set (10^4) | certified direction, max / mean | QR normal, max / mean | QR replaced |
| --- | --- | --- | ---: |
| `sphere` D = 3 | 2.1e-15 / 6.7e-17 | 1.1e-14 / 1.8e-16 | 0 |
| `sphere` D = 4 | 1.9e-14 / 1.4e-16 | 3.1e-14 / 2.7e-16 | 0 |
| `sphere` D = 5 | 5.6e-14 / 1.9e-16 | 9.2e-14 / 3.6e-16 | 0 |
| `cube` D = 3 to 6 | at most 1.3e-13 / 1.2e-16 | at most 1.3e-13 / 6.6e-16 | 0 |

The certified direction was nearer the exact direction on every set, 2 to 5 times on the mean. Its bound is proved, and QR's was only checked.

## Decision

Decided in the Grill on #210:

- The public unit normal of every facet, of any number of points, is the certified unit cofactor direction of the $D$ points the plane is built from, oriented outward. When the filter's bound exceeds 10^-10, the direction is computed exactly and rounded once. The published bound is ‖n − ĉ‖ ≤ ε ≤ 10^-10 for D ≤ 56294.
- The direction is computed when the plane is published, from the lexicographic basis of §5. It is not reused from construction, so a published normal depends only on the facet's vertex set.
- This crate's Householder QR is removed. The hull's planes and Voronoi's ray directions share the published path and change together. Voronoi's `faer` solve for circumcenters is a different computation and stays.

A later change that brings QR back for the public normal needs a measurement showing it more accurate than the certified direction, and a bound it can prove.
