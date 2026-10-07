# 0003. The sequential hull inserts one point at a time in place

## Status

Accepted

## Date

2026-10-07

## Issue

#251 (design), from the architecture review and Grill of 2026-10-07 (Q4). Implementation: #253 (sequential) and #256 (parallel).

## Context

Design §6 had the sequential and the parallel hull share one batch extraction and one commit. Each round took the first K = 64 candidates in packing order, reserved the facets T and ridges H of each, planned every taken point against the hull at the start of the round, and committed the plans in ascending input index. The sequential build ran the same steps on one thread. The text gave this sharing as the reason the two builds agree.

The sequential build paid for the protocol without using it:

- On hull `sphere` D = 4, memcpy was 9.6% of instructions and 45 to 48% of the L1 and last-level write misses. Most of it came from writing each plan and moving its simplices into the arena (`plan_region`, `insert_or_abort`; #199). After #209 trimmed the copies, memcpy was still about 10% of instructions on hull `sphere` D = 3.
- The candidate heap, the reservation marks, and the plan buffers are state that a one-point-at-a-time insertion does not need.
- With the protocol, `parallel(true)` gave 1.2 to 1.3 times on four cores (#178). #26 found the time in serial parts, among them the round's selection and the ordered commit.
- Timed in four sections (#250), construction alone on hull `sphere` D = 3 to D = 6 was 1.05 to 1.81 times Qhull's compute time, while Qhull created about as many hyperplanes as convx creates simplices. The gap is cost per operation, and the protocol is part of that cost.

The sharing is not what makes the builds agree. The published hull does not depend on the order of insertion. Each part of it is fixed by the input alone:

- A logical facet is unique as a face of the polytope, and its plane comes from its vertex set alone (§5, ADR 0002).
- A face that is not a simplex is split by the placing triangulation in index order (§3), and a face that is a simplex is itself.
- The index partition is fixed by whether each point is extreme, a non-extreme boundary point, or interior.
- `volume()` adds its terms in the order of that triangulation.

## Decision

Decided in the Grill of 2026-10-07 (Q4, option A):

- The sequential hull inserts outside points one at a time and changes the hull in place. The order is a deterministic order the implementation chooses, decided by the values and the order of the input alone. Each point is planned against the hull as it is at that point and applied at once; there is no reservation and no round.
- The batch extraction, K = 64, the T and H reservation, the conflict check, and the commit in ascending input index are the parallel build's procedure only.
- Sequential and parallel builds promise the same published hull on one binary. This follows from the uniqueness above. Debug builds compare the published result of every parallel build with that of the sequential build, in place of comparing a parallel round with the sequential application of the same batch.
- Whether the parallel hull stays is decided by measurement in #256. It stays only if four threads beat the sequential build beyond the spread on the timing sets with at least 10^4 facets. Otherwise a design Issue makes `parallel(true)` run the sequential build.

A later change that makes the sequential build depend on the parallel protocol again needs a measurement showing that the protocol costs the sequential build nothing beyond the spread.

## Outcome

#256 measured the parallel hull on the new store, and it did not meet the criterion above. `parallel(true)` runs the sequential build (`docs/adr/0004-parallel-runs-the-sequential-hull.md`).
