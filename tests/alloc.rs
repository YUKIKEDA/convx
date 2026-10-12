//! The allocation contract of `build()` (`.cursor/rules/perf.mdc`, #521).
//!
//! A build returns owned results, so it allocates. The number of its
//! allocations does not grow with the number of points: each buffer is
//! allocated once and reused, not per point, per facet, per insertion, or
//! per face. Its reallocations grow only as buffers grow.
//!
//! Each case is counted at a small size and at about 16 times it:
//!
//! - **The ratchet.** Both counts stay at or below the recorded ones, within
//!   1% and 2 more ([`allowed`]): classification's hash maps take a random
//!   seed, and a few counts move by one between runs. A count may fall, and
//!   the record is then lowered; raising one needs an Issue.
//! - **The growth.** At the large size the allocations stay below twice
//!   those at the small one, so they cannot be per item.
//!
//! The generator's degenerate families (`grid`, `lattice`, `cluster`) keep
//! a fixed number of distinct points however many are drawn. Their faces
//! and cospherical groups do not grow, so an allocation per face shows
//! there as a fixed count. These inputs grow in structure instead (#530):
//!
//! - a prism over a k-gon: k faces that are not simplices;
//! - the full grid of side m: every unit cell is a cospherical group, and a
//!   whole side or face of points comes before the first affinely
//!   independent ones;
//! - a k-gon times a square in D = 4: k faces whose sub-hull has D = 3;
//! - the integer points of a sphere, alone and with a site outside: one
//!   cospherical group of all of them (#539, #540);
//! - clusters on a sphere: planes whose direction the filters do not
//!   certify, as many as the clusters (#548).
//!
//! Growths found and removed: Voronoi per site (#523), the placing
//! triangulation per simplex (#524), and per face of a hull of D = 3 and
//! per point of a collinear prefix (#530), the pulling triangulation per
//! site (#539), the sub-hull of a face in D >= 4, built from nothing for
//! each face (#531), and the exact direction of a plane, per call (#548).
//! The copy in space order of a sub-hull of 2^17 points or more (#551) is
//! kept too; no case here reaches it, and a unit test of the hull's
//! storage covers it with the threshold lowered.
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

/// The points of a case, at its small and its large size.
#[derive(Clone, Copy)]
enum Points {
    /// A family of the generator, `n` and `16 n` points.
    Drawn(Family, usize),
    /// A prism over a k-gon in D = 3, `k` and `16 k`: the points
    /// `(cos t, sin t, 0)` and `(cos t, sin t, 1)`. Each side face is a
    /// rectangle, exactly: its four points share `x` and `y` in pairs.
    Prism(usize),
    /// Every point of the integer grid of two sides, about 16 times apart
    /// in the number of points.
    FullGrid(usize, usize),
    /// A k-gon times the unit square in D = 4, `k` and `16 k`: k box facets
    /// and four prisms.
    Duoprism(usize),
    /// The integer points of the sphere of a squared radius, small and
    /// large, in D = 3: every site on one sphere, which Delaunay fills by
    /// the pulling triangulation. 480 points at 1,001 and 6,630 at
    /// 1,221,025.
    Sphere(i64, i64),
    /// [`Self::Sphere`] with one more site outside the sphere: the points
    /// of the sphere are one cospherical group beside other cells, split by
    /// the placing triangulation.
    SphereAndOutside(i64, i64),
    /// `k` and `16 k` points of the generator's `sphere`, each with the
    /// 2^D points at 0 or 2^-30 from it along every axis. A simplex with
    /// two vertices of one such cluster is a sliver, whose direction the
    /// cofactors in `f64` do not certify, so its plane takes the exact
    /// direction, and the planes grow with the clusters (#548).
    Clusters(usize),
}

impl Points {
    fn name(self, dim: usize) -> String {
        match self {
            Self::Drawn(family, n) => format!("{} D{dim} n={n}", family.name()),
            Self::Prism(k) => format!("prism k={k}"),
            Self::FullGrid(small, large) => format!("full grid D{dim} m={small},{large}"),
            Self::Duoprism(k) => format!("k-gon times a square k={k}"),
            Self::Sphere(small, large) => format!("integer sphere r2={small},{large}"),
            Self::SphereAndOutside(small, large) => {
                format!("integer sphere and a site outside r2={small},{large}")
            }
            Self::Clusters(k) => format!("clusters on a sphere D{dim} k={k}"),
        }
    }

    fn at(self, dim: usize, large: bool) -> Vec<f64> {
        let times = if large { 16 } else { 1 };
        let circle = |k: usize| {
            (0..k).map(move |i| {
                let t = i as f64 / k as f64 * core::f64::consts::TAU;
                (t.cos(), t.sin())
            })
        };
        match self {
            Self::Drawn(family, n) => family.points(dim, times * n, 521),
            Self::Clusters(k) => Family::Sphere
                .points(dim, times * k, 521)
                .chunks_exact(dim)
                .flat_map(|center| {
                    (0..1_u32 << dim).flat_map(move |corner| {
                        center.iter().enumerate().map(move |(a, x)| {
                            x + f64::from((corner >> a) & 1) / f64::from(1_u32 << 30)
                        })
                    })
                })
                .collect(),
            Self::Prism(k) => circle(times * k)
                .flat_map(|(x, y)| [x, y, 0.0, x, y, 1.0])
                .collect(),
            Self::FullGrid(small, big) => {
                let m = if large { big } else { small };
                (0..m.pow(dim as u32))
                    .flat_map(|i| (0..dim).map(move |a| ((i / m.pow(a as u32)) % m) as f64))
                    .collect()
            }
            Self::Duoprism(k) => circle(times * k)
                .flat_map(|(x, y)| {
                    [
                        x, y, 0.0, 0.0, x, y, 1.0, 0.0, x, y, 0.0, 1.0, x, y, 1.0, 1.0,
                    ]
                })
                .collect(),
            Self::Sphere(small, big) | Self::SphereAndOutside(small, big) => {
                let squared = if large { big } else { small };
                let r = (squared as f64).sqrt() as i64 + 1;
                let mut points = Vec::new();
                if matches!(self, Self::SphereAndOutside(..)) {
                    points.extend([3.0 * r as f64, 0.0, 0.0]);
                }
                for x in -r..=r {
                    for y in -r..=r {
                        let rest = squared - x * x - y * y;
                        let z = (rest.max(0) as f64).sqrt().round() as i64;
                        if rest < 0 || z * z != rest {
                            continue;
                        }
                        points.extend([x as f64, y as f64, z as f64]);
                        if z != 0 {
                            points.extend([x as f64, y as f64, -z as f64]);
                        }
                    }
                }
                points
            }
        }
    }
}

/// One case: what is built, on which points, and the counts recorded at
/// the small and the large size as `(allocations, reallocations)`.
struct Case {
    kind: Kind,
    points: Points,
    dim: usize,
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
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cube, 1000), dim: 2, small: (39, 21), large: (39, 31) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cube, 1000), dim: 3, small: (131, 197), large: (145, 261) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cube, 1000), dim: 4, small: (142, 336), large: (158, 488) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Sphere, 1000), dim: 2, small: (35, 17), large: (35, 25) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Sphere, 1000), dim: 3, small: (141, 339), large: (180, 687) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Sphere, 1000), dim: 4, small: (143, 434), large: (173, 780) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Grid, 1000), dim: 2, small: (40, 5), large: (40, 5) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Grid, 1000), dim: 3, small: (151, 141), large: (151, 141) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Grid, 1000), dim: 4, small: (276, 420), large: (274, 414) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::CubeSurf, 1000), dim: 3, small: (187, 282), large: (192, 319) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::CubeSurf, 1000), dim: 4, small: (328, 719), large: (366, 979) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cluster, 1000), dim: 2, small: (40, 16), large: (40, 16) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cluster, 1000), dim: 3, small: (176, 257), large: (177, 230) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cluster, 1000), dim: 4, small: (336, 650), large: (328, 621) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Cube, 1000), dim: 2, small: (154, 37), large: (154, 54) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Cube, 1000), dim: 3, small: (376, 71), large: (376, 91) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Sphere, 1000), dim: 2, small: (154, 38), large: (154, 55) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Sphere, 1000), dim: 3, small: (376, 74), large: (376, 95) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Grid, 1000), dim: 2, small: (173, 47), large: (173, 47) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Grid, 1000), dim: 3, small: (392, 111), large: (392, 111) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Lattice, 1000), dim: 2, small: (173, 55), large: (173, 55) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Lattice, 1000), dim: 3, small: (392, 130), large: (392, 130) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::NearSphere, 1000), dim: 2, small: (173, 76), large: (173, 76) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::NearSphere, 1000), dim: 3, small: (393, 128), large: (408, 181) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Cube, 500), dim: 4, small: (806, 76), large: (806, 97) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Cube, 250), dim: 2, small: (241, 112), large: (241, 159) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Cube, 250), dim: 3, small: (542, 301), large: (559, 420) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Sphere, 250), dim: 2, small: (237, 127), large: (237, 199) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Sphere, 250), dim: 3, small: (554, 377), large: (583, 727) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Lattice, 250), dim: 2, small: (271, 110), large: (271, 110) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Lattice, 250), dim: 3, small: (609, 395), large: (604, 401) },
    Case { kind: Kind::Hull, points: Points::Prism(64), dim: 3, small: (133, 223), large: (162, 376) },
    Case { kind: Kind::Hull, points: Points::FullGrid(8, 32), dim: 2, small: (40, 14), large: (40, 26) },
    Case { kind: Kind::Hull, points: Points::FullGrid(4, 10), dim: 3, small: (148, 138), large: (148, 192) },
    Case { kind: Kind::Delaunay, points: Points::FullGrid(8, 32), dim: 2, small: (173, 77), large: (173, 129) },
    Case { kind: Kind::Delaunay, points: Points::FullGrid(4, 10), dim: 3, small: (392, 112), large: (392, 182) },
    Case { kind: Kind::Voronoi, points: Points::FullGrid(8, 32), dim: 2, small: (271, 148), large: (271, 246) },
    Case { kind: Kind::Voronoi, points: Points::FullGrid(4, 10), dim: 3, small: (599, 336), large: (599, 502) },
    Case { kind: Kind::Hull, points: Points::Sphere(1001, 1_221_025), dim: 3, small: (158, 318), large: (192, 661) },
    Case { kind: Kind::Delaunay, points: Points::Sphere(1001, 1_221_025), dim: 3, small: (130, 313), large: (164, 641) },
    Case { kind: Kind::Voronoi, points: Points::Sphere(1001, 1_221_025), dim: 3, small: (348, 803), large: (416, 1576) },
    Case { kind: Kind::Delaunay, points: Points::SphereAndOutside(1001, 1_221_025), dim: 3, small: (408, 222), large: (408, 307) },
    Case { kind: Kind::Voronoi, points: Points::SphereAndOutside(1001, 1_221_025), dim: 3, small: (627, 725), large: (664, 1235) },
    Case { kind: Kind::Hull, points: Points::Clusters(32), dim: 3, small: (169, 231), large: (217, 419) },
    Case { kind: Kind::Hull, points: Points::Clusters(16), dim: 4, small: (286, 439), large: (341, 720) },
    Case { kind: Kind::Hull, points: Points::Duoprism(8), dim: 4, small: (225, 361), large: (305, 689) },
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
        let name = format!("{:?} {}", case.kind, case.points.name(case.dim));
        let small = build(case.kind, case.dim, &case.points.at(case.dim, false));
        let large = build(case.kind, case.dim, &case.points.at(case.dim, true));
        if record {
            println!("{name}: small {small:?} large {large:?}");
            continue;
        }
        if small.0 > allowed(case.small.0) || small.1 > allowed(case.small.1) {
            failures.push(format!(
                "{name}: {small:?} at the small size, recorded {:?}",
                case.small
            ));
        }
        if large.0 > allowed(case.large.0) || large.1 > allowed(case.large.1) {
            failures.push(format!(
                "{name}: {large:?} at the large size, recorded {:?}",
                case.large
            ));
        }
        if large.0 >= 2 * small.0 {
            failures.push(format!(
                "{name}: {} allocations at the large size against {} at the small",
                large.0, small.0
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
