//! The allocation contract of `build()` (`.cursor/rules/perf.mdc`, #521).
//!
//! A build returns owned results, so it allocates. The number of its
//! allocations does not grow with the number of points: each buffer is
//! allocated once and reused, not per point, per facet, per insertion, or
//! per face. Its reallocations grow only as buffers grow.
//!
//! Each case is counted at `n` points and at `16 n`:
//!
//! - **The ratchet.** Both counts stay at or below the recorded ones, within
//!   1% and 2 more ([`allowed`]): classification's hash maps take a random
//!   seed, and a few counts move by one between runs. A count may fall, and
//!   the record is then lowered; raising one needs an Issue.
//! - **The growth.** At `16 n` the allocations stay below twice those at
//!   `n`, so they cannot be per point. Two growths were found by the first
//!   count and removed: Voronoi allocated per site (#523), and the placing
//!   triangulation per simplex (#524).
//!
//! Debug builds run invariant checks inside every build, and those allocate
//! freely, so the counts hold for release builds only. `just test` runs this
//! file with `--release`. The counter is global to the process, so one test
//! function measures every case, with no other test thread allocating.

// Helpers unwrap results of inputs that are valid by construction.
#![allow(clippy::unwrap_used)]

mod common;

use common::generator::Family;
use convx::{ConvexHullBuilder, DelaunayBuilder, VoronoiBuilder};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// What is built.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Hull,
    Delaunay,
    Voronoi,
}

/// One case: what is built, on which points, and the counts recorded at
/// `n` and at `16 n` as `(allocations, reallocations)`.
struct Case {
    kind: Kind,
    family: Family,
    dim: usize,
    n: usize,
    small: (usize, usize),
    large: (usize, usize),
}

/// The allocations and reallocations of `f`.
fn count(f: impl FnOnce()) -> (usize, usize) {
    let region = Region::new(GLOBAL);
    f();
    let change = region.change();
    (change.allocations, change.reallocations)
}

/// A recorded count with its allowance for the hash maps' random seeds.
fn allowed(recorded: usize) -> usize {
    recorded + recorded / 100 + 2
}

fn build(kind: Kind, dim: usize, points: &[f64]) -> (usize, usize) {
    match kind {
        Kind::Hull => count(|| {
            std::hint::black_box(ConvexHullBuilder::new(dim, points).build().unwrap());
        }),
        Kind::Delaunay => count(|| {
            std::hint::black_box(DelaunayBuilder::new(dim, points).build().unwrap());
        }),
        Kind::Voronoi => count(|| {
            std::hint::black_box(VoronoiBuilder::new(dim, points).build().unwrap());
        }),
    }
}

#[rustfmt::skip]
const CASES: &[Case] = &[
    Case { kind: Kind::Hull, family: Family::Cube, dim: 2, n: 1000, small: (47, 23), large: (47, 33) },
    Case { kind: Kind::Hull, family: Family::Cube, dim: 3, n: 1000, small: (152, 197), large: (166, 261) },
    Case { kind: Kind::Hull, family: Family::Cube, dim: 4, n: 1000, small: (171, 337), large: (187, 489) },
    Case { kind: Kind::Hull, family: Family::Sphere, dim: 2, n: 1000, small: (43, 19), large: (43, 27) },
    Case { kind: Kind::Hull, family: Family::Sphere, dim: 3, n: 1000, small: (162, 339), large: (201, 687) },
    Case { kind: Kind::Hull, family: Family::Sphere, dim: 4, n: 1000, small: (172, 435), large: (202, 781) },
    Case { kind: Kind::Hull, family: Family::Grid, dim: 2, n: 1000, small: (48, 7), large: (48, 7) },
    Case { kind: Kind::Hull, family: Family::Grid, dim: 3, n: 1000, small: (474, 196), large: (474, 196) },
    Case { kind: Kind::Hull, family: Family::Grid, dim: 4, n: 1000, small: (3127, 1638), large: (3127, 1643) },
    Case { kind: Kind::Hull, family: Family::CubeSurf, dim: 3, n: 1000, small: (502, 372), large: (501, 484) },
    Case { kind: Kind::Hull, family: Family::CubeSurf, dim: 4, n: 1000, small: (1231, 1678), large: (1423, 2410) },
    Case { kind: Kind::Hull, family: Family::Cluster, dim: 2, n: 1000, small: (48, 18), large: (48, 18) },
    Case { kind: Kind::Hull, family: Family::Cluster, dim: 3, n: 1000, small: (1363, 415), large: (1322, 406) },
    Case { kind: Kind::Hull, family: Family::Cluster, dim: 4, n: 1000, small: (26131, 7833), large: (23286, 8853) },
    Case { kind: Kind::Delaunay, family: Family::Cube, dim: 2, n: 1000, small: (162, 38), large: (162, 55) },
    Case { kind: Kind::Delaunay, family: Family::Cube, dim: 3, n: 1000, small: (389, 72), large: (389, 92) },
    Case { kind: Kind::Delaunay, family: Family::Sphere, dim: 2, n: 1000, small: (162, 39), large: (162, 56) },
    Case { kind: Kind::Delaunay, family: Family::Sphere, dim: 3, n: 1000, small: (389, 75), large: (389, 96) },
    Case { kind: Kind::Delaunay, family: Family::Grid, dim: 2, n: 1000, small: (177, 49), large: (177, 49) },
    Case { kind: Kind::Delaunay, family: Family::Grid, dim: 3, n: 1000, small: (415, 112), large: (415, 112) },
    Case { kind: Kind::Delaunay, family: Family::Lattice, dim: 2, n: 1000, small: (177, 57), large: (177, 57) },
    Case { kind: Kind::Delaunay, family: Family::Lattice, dim: 3, n: 1000, small: (408, 131), large: (408, 131) },
    Case { kind: Kind::Delaunay, family: Family::NearSphere, dim: 2, n: 1000, small: (178, 66), large: (178, 66) },
    Case { kind: Kind::Delaunay, family: Family::NearSphere, dim: 3, n: 1000, small: (409, 129), large: (410, 146) },
    Case { kind: Kind::Delaunay, family: Family::Cube, dim: 4, n: 500, small: (825, 78), large: (825, 99) },
    Case { kind: Kind::Voronoi, family: Family::Cube, dim: 2, n: 250, small: (257, 118), large: (257, 165) },
    Case { kind: Kind::Voronoi, family: Family::Cube, dim: 3, n: 250, small: (576, 305), large: (593, 424) },
    Case { kind: Kind::Voronoi, family: Family::Sphere, dim: 2, n: 250, small: (253, 133), large: (253, 205) },
    Case { kind: Kind::Voronoi, family: Family::Sphere, dim: 3, n: 250, small: (588, 381), large: (617, 731) },
    Case { kind: Kind::Voronoi, family: Family::Lattice, dim: 2, n: 250, small: (274, 116), large: (274, 116) },
    Case { kind: Kind::Voronoi, family: Family::Lattice, dim: 3, n: 250, small: (1081, 471), large: (1047, 480) },
];

#[test]
fn builds_keep_the_allocation_contract() {
    if cfg!(debug_assertions) {
        // Debug builds run invariant checks that allocate (module docs).
        return;
    }
    // The counter is live: a vector of one element allocates once.
    let (allocations, _) = count(|| {
        std::hint::black_box(vec![0_u8; 1]);
    });
    assert_eq!(allocations, 1, "the counter is off");

    let record = std::env::var_os("CONVX_ALLOC_RECORD").is_some();
    let mut failures = Vec::new();
    for case in CASES {
        let name = format!(
            "{:?} {} D{} n={}",
            case.kind,
            case.family.name(),
            case.dim,
            case.n
        );
        let small = build(
            case.kind,
            case.dim,
            &case.family.points(case.dim, case.n, 521),
        );
        let large = build(
            case.kind,
            case.dim,
            &case.family.points(case.dim, 16 * case.n, 521),
        );
        if record {
            println!("{name}: small {small:?} large {large:?}");
            continue;
        }
        if small.0 > allowed(case.small.0) || small.1 > allowed(case.small.1) {
            failures.push(format!("{name}: {small:?} at n, recorded {:?}", case.small));
        }
        if large.0 > allowed(case.large.0) || large.1 > allowed(case.large.1) {
            failures.push(format!(
                "{name}: {large:?} at 16 n, recorded {:?}",
                case.large
            ));
        }
        if large.0 >= 2 * small.0 {
            failures.push(format!(
                "{name}: {} allocations at 16 n against {} at n",
                large.0, small.0
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
