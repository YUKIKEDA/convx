//! The parallel build agrees with the sequential one (design §6): the same
//! batches, committed in the same order, give an identical hull.

// The record loader and the builds unwrap inputs that are valid by
// construction.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::record::Record;
use convx::ConvexHullBuilder;

fn agree(dim: usize, points: &[f64], case: &str) {
    let sequential = ConvexHullBuilder::new(dim, points).build();
    let parallel = ConvexHullBuilder::new(dim, points).parallel(true).build();
    assert_eq!(sequential, parallel, "{case}");
}

#[test]
fn parallel_false_is_the_default() {
    let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
    assert_eq!(
        ConvexHullBuilder::new(2, &points).build(),
        ConvexHullBuilder::new(2, &points).parallel(false).build()
    );
}

#[test]
fn every_hull_record_agrees() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hull");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 30);
    for path in paths {
        let record = Record::parse(&fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let points = record.family.points(record.dim, record.count, record.seed);
        agree(record.dim, &points, &path.display().to_string());
    }
}

#[test]
fn hand_cases_agree() {
    let cube: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1)))
        .collect();
    let mut cube_with_extras = cube.clone();
    cube_with_extras.extend([0.5, 0.5, 1.0, 1.0, 0.5, 0.0, 0.5, 0.5, 0.5]);
    let lift = f64::from_bits(1.0_f64.to_bits() + 1) - 1.0;
    let cases: Vec<(usize, Vec<f64>, &str)> = vec![
        (
            1,
            vec![3.0, -2.0, -0.0, 0.0, 3.0],
            "segment with duplicates",
        ),
        (
            2,
            vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.0, 0.5, 0.5],
            "square",
        ),
        (3, cube, "cube"),
        (
            3,
            cube_with_extras,
            "cube with face, edge, and interior points",
        ),
        (
            3,
            vec![
                0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, lift, 0.5, 0.5, -1.0,
            ],
            "near coplanar",
        ),
        (
            2,
            vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0],
            "collinear (DegenerateDimension)",
        ),
    ];
    for (dim, points, case) in cases {
        agree(dim, &points, case);
    }
}

#[test]
fn integer_grids_agree() {
    for dim in 2..=4 {
        let side = if dim <= 3 { 5 } else { 3 };
        let count = (side as usize).pow(dim as u32);
        let points: Vec<f64> = (0..count)
            .flat_map(|i| {
                (0..dim).map(move |a| ((i / (side as usize).pow(a as u32)) % side as usize) as f64)
            })
            .collect();
        agree(dim, &points, &format!("grid {side}^{dim}"));
    }
}
