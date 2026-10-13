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
//!   those at the small one, so they cannot be per item. The first call
//!   of `planes()` and builds that end in an error are held to the same
//!   two checks (#567).
//!
//! The counter counts every thread of the process, so this test is its own
//! program, with no harness (`harness = false` in `Cargo.toml`, #575): the
//! harness runs a test on a thread beside its own, and four allocations
//! above a record showed on CI now and then, in the first cases.
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
//!   certify, as many as the clusters (#548);
//! - coordinates whose exponents are far apart: determinants that only
//!   the heap integers hold (#185).
//!
//! Growths found and removed: Voronoi per site (#523), the placing
//! triangulation per simplex (#524), and per face of a hull of D = 3 and
//! per point of a collinear prefix (#530), the pulling triangulation per
//! site (#539), the sub-hull of a face in D >= 4, built from nothing for
//! each face (#531), and the exact direction of a plane, per call (#548).
//! The copy in space order of a sub-hull of 2^17 points or more (#551) is
//! kept too; no case here reaches it, and a unit test of the hull's
//! storage covers it with the threshold lowered. The exact stage over
//! heap integers allocated in every call, and runs on a workspace kept
//! per thread (#185): its buffers are reserved by the first build of a
//! thread that needs them, so a later build on that thread counts fewer.
//! A hull of D = 9 or 10 allocated for every simplex it planned (#554);
//! hulls of D = 8 to 10 and Delaunay of D = 8 and 9 are counted at two
//! numbers of points. The contract reaches D = 10: in D >= 11 the lists
//! whose length follows the dimension are on the heap, by design (#554).
//! The first call of `planes()` allocates 4 to 10 times whatever the
//! facets, and a build that ends in an error at most 15 times whatever
//! the points (#567).
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
    /// The first call of `planes()` on a built hull; the build is not
    /// counted (#567).
    Planes,
    /// A build that ends in an error, for each builder (#567).
    HullFails,
    DelaunayFails,
    VoronoiFails,
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
    /// The generator's `cube`, `n` and `16 n` points, with the first
    /// coordinate scaled by 1e150 and the others by 1e-150. The exponents
    /// of one determinant are about 1,000 apart, more than the integers
    /// on the stack hold, so the exact stage runs on heap integers (#185).
    Scaled(usize),
    /// The generator's `cube`, `n` and `16 n` points, each point times
    /// `(1 - u)^-6` for a `u` hashed from its index: sites dense about the
    /// origin with a long tail. The insertion order's grid over their box
    /// holds most of them in a few cells, which it orders again (#620).
    Tail(usize),
    /// The generator's `cube` at two numbers of points, for the high
    /// dimensions, where sixteen times the points is out of reach: the
    /// facets grow about ten times between the two (#554).
    Few(usize, usize),
    /// The generator's `cube`, `n` and `16 n` points, with the last
    /// coordinate set to zero: the points do not span the dimension, and a
    /// build fails with `DegenerateDimension`.
    Flat(usize),
    /// The generator's `cube`, `n` and `16 n` points, with a NaN as the
    /// last coordinate of the last point: a build fails with
    /// `NonFiniteCoordinate` after it has read every point.
    NanLast(usize),
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
            Self::Scaled(n) => format!("cube with exponents far apart D{dim} n={n}"),
            Self::Tail(n) => format!("cube with a long tail D{dim} n={n}"),
            Self::Few(small, large) => format!("cube D{dim} n={small},{large}"),
            Self::Flat(n) => format!("cube in a hyperplane D{dim} n={n}"),
            Self::NanLast(n) => format!("cube with a NaN last D{dim} n={n}"),
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
            Self::Flat(n) => {
                let mut points = Family::Cube.points(dim, times * n, 521);
                for point in points.chunks_exact_mut(dim) {
                    point[dim - 1] = 0.0;
                }
                points
            }
            Self::NanLast(n) => {
                let mut points = Family::Cube.points(dim, times * n, 521);
                if let Some(last) = points.last_mut() {
                    *last = f64::NAN;
                }
                points
            }
            Self::Tail(n) => {
                let mut points = Family::Cube.points(dim, times * n, 521);
                for (i, point) in points.chunks_exact_mut(dim).enumerate() {
                    let hash = (i as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15);
                    let u = (hash >> 11) as f64 / (1_u64 << 53) as f64;
                    let radius = (1.0 - u).powi(-6);
                    point.iter_mut().for_each(|x| *x *= radius);
                }
                points
            }
            Self::Few(small, big) => Family::Cube.points(dim, if large { big } else { small }, 521),
            Self::Scaled(n) => Family::Cube
                .points(dim, times * n, 521)
                .iter()
                .enumerate()
                .map(|(i, x)| if i % dim == 0 { x * 1e150 } else { x * 1e-150 })
                .collect(),
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
        Kind::Planes => {
            let hull = ConvexHullBuilder::new(dim, points).build().unwrap();
            count(|| {
                std::hint::black_box(hull.planes().unwrap());
            })
        }
        Kind::HullFails => count(|| {
            std::hint::black_box(ConvexHullBuilder::new(dim, points).build().unwrap_err());
        }),
        Kind::DelaunayFails => count(|| {
            std::hint::black_box(DelaunayBuilder::new(dim, points).build().unwrap_err());
        }),
        Kind::VoronoiFails => count(|| {
            std::hint::black_box(VoronoiBuilder::new(dim, points).build().unwrap_err());
        }),
    }
}

#[rustfmt::skip]
const CASES: &[Case] = &[
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cube, 1000), dim: 2, small: (40, 21), large: (40, 31) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cube, 1000), dim: 3, small: (128, 185), large: (137, 266) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cube, 1000), dim: 4, small: (139, 332), large: (155, 486) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Sphere, 1000), dim: 2, small: (35, 17), large: (35, 25) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Sphere, 1000), dim: 3, small: (138, 341), large: (170, 679) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Sphere, 1000), dim: 4, small: (144, 428), large: (169, 773) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Grid, 1000), dim: 2, small: (41, 5), large: (41, 5) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Grid, 1000), dim: 3, small: (149, 115), large: (149, 115) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Grid, 1000), dim: 4, small: (265, 368), large: (265, 368) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::CubeSurf, 1000), dim: 3, small: (183, 262), large: (189, 343) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::CubeSurf, 1000), dim: 4, small: (309, 682), large: (357, 948) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cluster, 1000), dim: 2, small: (41, 16), large: (41, 16) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cluster, 1000), dim: 3, small: (174, 253), large: (173, 224) },
    Case { kind: Kind::Hull, points: Points::Drawn(Family::Cluster, 1000), dim: 4, small: (336, 635), large: (320, 619) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Cube, 1000), dim: 2, small: (49, 37), large: (49, 54) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Cube, 1000), dim: 3, small: (57, 71), large: (57, 91) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Sphere, 1000), dim: 2, small: (49, 38), large: (49, 55) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Sphere, 1000), dim: 3, small: (57, 74), large: (57, 95) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Grid, 1000), dim: 2, small: (54, 25), large: (54, 25) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Grid, 1000), dim: 3, small: (86, 106), large: (86, 106) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Lattice, 1000), dim: 2, small: (54, 28), large: (54, 28) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Lattice, 1000), dim: 3, small: (86, 118), large: (86, 118) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::NearSphere, 1000), dim: 2, small: (80, 59), large: (80, 59) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::NearSphere, 1000), dim: 3, small: (87, 157), large: (102, 199) },
    Case { kind: Kind::Delaunay, points: Points::Tail(1000), dim: 2, small: (51, 38), large: (51, 56) },
    Case { kind: Kind::Delaunay, points: Points::Tail(1000), dim: 3, small: (59, 71), large: (59, 89) },
    Case { kind: Kind::Delaunay, points: Points::Drawn(Family::Cube, 500), dim: 4, small: (57, 76), large: (57, 97) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Cube, 250), dim: 2, small: (136, 110), large: (137, 157) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Cube, 250), dim: 3, small: (219, 290), large: (240, 409) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Sphere, 250), dim: 2, small: (132, 125), large: (132, 197) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Sphere, 250), dim: 3, small: (229, 368), large: (258, 722) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Lattice, 250), dim: 2, small: (153, 78), large: (153, 78) },
    Case { kind: Kind::Voronoi, points: Points::Drawn(Family::Lattice, 250), dim: 3, small: (297, 380), large: (294, 348) },
    Case { kind: Kind::Hull, points: Points::Prism(64), dim: 3, small: (122, 206), large: (126, 311) },
    Case { kind: Kind::Hull, points: Points::FullGrid(8, 32), dim: 2, small: (41, 14), large: (41, 26) },
    Case { kind: Kind::Hull, points: Points::FullGrid(4, 10), dim: 3, small: (146, 106), large: (146, 172) },
    Case { kind: Kind::Delaunay, points: Points::FullGrid(8, 32), dim: 2, small: (54, 39), large: (54, 67) },
    Case { kind: Kind::Delaunay, points: Points::FullGrid(4, 10), dim: 3, small: (86, 83), large: (86, 123) },
    Case { kind: Kind::Voronoi, points: Points::FullGrid(8, 32), dim: 2, small: (153, 107), large: (153, 181) },
    Case { kind: Kind::Voronoi, points: Points::FullGrid(4, 10), dim: 3, small: (291, 273), large: (291, 421) },
    Case { kind: Kind::Hull, points: Points::Sphere(1001, 1_221_025), dim: 3, small: (154, 335), large: (187, 654) },
    Case { kind: Kind::Delaunay, points: Points::Sphere(1001, 1_221_025), dim: 3, small: (126, 308), large: (159, 636) },
    Case { kind: Kind::Voronoi, points: Points::Sphere(1001, 1_221_025), dim: 3, small: (340, 813), large: (406, 1562) },
    Case { kind: Kind::Delaunay, points: Points::SphereAndOutside(1001, 1_221_025), dim: 3, small: (102, 249), large: (102, 350) },
    Case { kind: Kind::Voronoi, points: Points::SphereAndOutside(1001, 1_221_025), dim: 3, small: (325, 768), large: (350, 1263) },
    Case { kind: Kind::Hull, points: Points::Clusters(32), dim: 3, small: (171, 207), large: (214, 427) },
    Case { kind: Kind::Hull, points: Points::Clusters(16), dim: 4, small: (279, 436), large: (333, 720) },
    Case { kind: Kind::Hull, points: Points::Scaled(100), dim: 3, small: (145, 136), large: (121, 205) },
    Case { kind: Kind::Hull, points: Points::Scaled(50), dim: 4, small: (121, 206), large: (129, 321) },
    Case { kind: Kind::Hull, points: Points::Few(40, 120), dim: 8, small: (110, 335), large: (130, 473) },
    Case { kind: Kind::Hull, points: Points::Few(24, 44), dim: 9, small: (99, 293), large: (111, 379) },
    Case { kind: Kind::Hull, points: Points::Few(20, 32), dim: 10, small: (94, 288), large: (106, 366) },
    Case { kind: Kind::Delaunay, points: Points::Few(16, 26), dim: 8, small: (65, 63), large: (65, 91) },
    Case { kind: Kind::Delaunay, points: Points::Few(14, 22), dim: 9, small: (67, 53), large: (67, 88) },
    Case { kind: Kind::Planes, points: Points::Drawn(Family::Sphere, 1000), dim: 2, small: (4, 0), large: (4, 0) },
    Case { kind: Kind::Planes, points: Points::Drawn(Family::Sphere, 1000), dim: 3, small: (9, 0), large: (9, 0) },
    Case { kind: Kind::Planes, points: Points::Drawn(Family::Sphere, 500), dim: 4, small: (9, 0), large: (9, 0) },
    Case { kind: Kind::Planes, points: Points::Drawn(Family::CubeSurf, 500), dim: 4, small: (10, 0), large: (10, 0) },
    Case { kind: Kind::Planes, points: Points::Clusters(16), dim: 4, small: (10, 0), large: (10, 0) },
    Case { kind: Kind::Planes, points: Points::Few(20, 32), dim: 10, small: (9, 0), large: (9, 0) },
    Case { kind: Kind::HullFails, points: Points::Flat(200), dim: 2, small: (15, 12), large: (15, 20) },
    Case { kind: Kind::HullFails, points: Points::Flat(200), dim: 4, small: (5, 6), large: (5, 10) },
    Case { kind: Kind::DelaunayFails, points: Points::Flat(200), dim: 3, small: (5, 6), large: (5, 10) },
    Case { kind: Kind::VoronoiFails, points: Points::Flat(200), dim: 3, small: (5, 6), large: (5, 10) },
    Case { kind: Kind::HullFails, points: Points::NanLast(200), dim: 3, small: (0, 0), large: (0, 0) },
    Case { kind: Kind::DelaunayFails, points: Points::NanLast(200), dim: 3, small: (0, 0), large: (0, 0) },
    Case { kind: Kind::VoronoiFails, points: Points::NanLast(200), dim: 2, small: (0, 0), large: (0, 0) },
    Case { kind: Kind::Hull, points: Points::Duoprism(8), dim: 4, small: (214, 310), large: (281, 667) },
];

fn main() {
    if cfg!(debug_assertions) {
        // Debug builds run invariant checks that allocate (module docs).
        println!("alloc: not counted in a debug build");
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
        // Twice the small count, or three more where that count is zero
        // or one: a build that fails before it allocates counts none.
        if large.0 >= (2 * small.0).max(small.0 + 3) {
            failures.push(format!(
                "{name}: {} allocations at the large size against {} at the small",
                large.0, small.0
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if !record {
        println!("alloc: {} cases keep the allocation contract", CASES.len());
    }
}
