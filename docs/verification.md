# Verification

Golden data is not a set that can be listed before the code exists. A phase checks the result that phase can produce. The next phase adds what it needs. Records from earlier phases stay.

This file names the flows that can exist. The judgment and the criteria inside a flow are written after the flows are named.

The library's meaning, the invariants, and the named completion inputs are the design (§10 and §11). This file does not restate them.

## What a phase can check

| Phase  | Becomes checkable                                                                                        | Records already in the tree                                                              |
| ------ | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| P1     | Predicate sign, the exact-sign fallback, and the predicate inputs named in the design                    | None                                                                                     |
| P2     | Hull topology, volume, the index partition, the hull invariants, the named hull inputs, hull simulations | P1 records stay. They are not hull expectations                                          |
| P3     | Sequential and parallel agreement on the same batch, in one binary                                       | Hull expectations stay. Parallel output does not replace them                            |
| P4     | Delaunay, Voronoi, and the named Delaunay inputs                                                         | Hull expectations stay. A Delaunay or Voronoi field is added only when P4 can produce it |
| P5     | Wider SIMD, allocation, caches, and search cutoffs                                                       | Expectations stay. A faster path that disagrees is wrong                                 |
| Timing | Wall time against Qhull, after a correct sequential hull                                                 | Performance files stay separate from correctness records                                 |

A check is armed when the phase that produces that result is in the tree. A later result is not frozen early, and a record is not deleted because a later phase is still absent.

## Flows

### Arm a check

When a phase first produces a result the design asks to check, that check starts. It starts empty of frozen answers except the hand cases the design already states. Earlier checks keep running.

### Exercise before a frozen answer

Generated points can be checked against the invariants alone. Euler characteristic, sidedness, adjacency, and the index partition do not need an expected topology. This flow is how a new family is first run. It produces a pass or a failure. It does not write a record.

### Record a hand case

The completion inputs named in the design already have an expected result. The record copies that statement. The library's own output is not the source.

### Freeze a simulation

A seed has been run, and the invariants for the armed check hold. The normalized result of that run is committed as a regression lock for the observables that exist now. The lock is the first sequential result that satisfied the invariants. It is not a proof by a second implementation.

### Add a case

A new seed, dimension, family, or required failure enters a check that is already armed. The trigger is a phase that can now produce the result, a failure that the current set does not cover, or a review. Records already committed are left as they are.

### Extend a record

The same point set gains an observable a later phase can produce. A hull record gains a Delaunay or Voronoi expectation when P4 can produce it. The expectation already stored stays. Predicate cases do not grow a hull expectation. A hull record does not grow a predicate expectation.

### Steady run

Every test run rebuilds points from the recorded seed and compares the observables that are armed. It reads records. It does not write them. It does not call Qhull.

### Disagreement

A steady run does not match the frozen expectation. The flow stops at the failure. The record stays. Overwriting the record to match the run is not part of this flow.

### Revise because the design changed

A design change states a different result for an input that already has a record. The record is updated to the new statement in the same change as the design. Records the design change does not touch stay.

### Revise because the lock stored a wrong result

The frozen result itself violates the invariants, or it encodes a bug the change is fixing. That record is updated. The diff is the change under review. Records that still match a correct result stay.

### Change the generator

The same seed would build different points. Every correctness record tied to that generator becomes stale together. They are reviewed as one set. One file is not refreshed alone.

### Keep the result across a new path

A refactor, the parallel commit, or a P5 optimization produces a result the sequential lock already names. The lock stays. The new path is compared to it. Parallel agreement is the sequential path and the parallel path in the same binary, on the same batch. That comparison does not create a second record.

### Retire a record

The design no longer asks for that check, or a recorded case replaces it and covers it. The deletion is part of the change under review. A record is not retired because the phase that should produce it is not implemented yet, and it is not retired because a run disagreed.

### Record a required failure

The expected result is a named error (`DegenerateDimension`, `TooManyPoints`, `NonFiniteCoordinate`, and the other input failures the design names). The payload is the error, not a topology. Add, steady run, disagreement, revise, and retire apply to these records as they do to topology records.

### Check a transform

Translation, positive uniform scale, axis permutation, and input-order permutation are applied to an input that already has a record. The comparison rule is the one the design states for that transform. The transform does not create another record.

### Open a performance set

Timing starts after a correct sequential hull. A performance file is created then. It may use the same generator and the same seeds as a correctness record. It stores points to time. It does not store an expected topology. A later timing run reads it. A correctness run does not.

### Two records claim the same identity

Two records name the same generator, seed, dimension, point count, family, and observable. The run does not pick one. The conflict is resolved by revising or retiring one of them before the check is armed again.

## Situations with no flow

Refreshing every expectation because a run was inconvenient. Taking Qhull's triangulation as the expected logical facets. Loading a performance file as a correctness oracle. Deleting a record because a later phase is not implemented yet. Writing a Delaunay or Voronoi expectation before that phase can produce it.
