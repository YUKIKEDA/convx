//! Steady run of the frozen hull records (docs/verification.md). Each record
//! is rebuilt from its seed and compared with the observables it stores. This
//! test only reads records.

// The record loader unwraps files that the repository itself provides.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use common::generator::GENERATOR;
use common::record::{DelaunayRecord, Record, DELAUNAY_MAX_DIM};
use convx::ConvexHullBuilder;

fn records() -> Vec<(PathBuf, Record)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hull");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| entries.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    paths.sort();
    paths
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .map(|p| {
            let text = fs::read_to_string(&p).unwrap();
            let record = Record::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (p, record)
        })
        .collect()
}

#[test]
fn frozen_hulls_match() {
    let records = records();
    assert!(records.len() >= 45, "only {} hull records", records.len());
    // Records are independent; they run on rayon's pool to keep the debug
    // steady run short.
    let failures: Vec<String> = records
        .par_iter()
        .flat_map_iter(|(path, record)| check_record(path, record))
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Every mismatch of one record against a rebuild from its seed.
fn check_record(path: &Path, record: &Record) -> Vec<String> {
    let mut failures = Vec::new();
    {
        assert_eq!(
            record.generator,
            GENERATOR,
            "{} was frozen with another generator; records of one generator are reviewed together",
            path.display()
        );
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            record.file_name()
        );
        let points = record.family.points(record.dim, record.count, record.seed);
        let hull = ConvexHullBuilder::new(record.dim, &points).build().unwrap();
        for error in record.compare(&hull) {
            failures.push(format!("{}: {error}", path.display()));
        }
        // The volume is stored `unchecked` exactly when freezing would write
        // it so, so the near-zero rule is not bypassed by hand.
        let fresh = Record::of(
            &record.generator,
            record.family,
            record.dim,
            record.count,
            record.seed,
            &points,
            &hull,
        );
        // Delaunay and Voronoi observables are armed for D <= 5.
        match (&record.delaunay, record.dim <= DELAUNAY_MAX_DIM) {
            (Some(stored), true) => match DelaunayRecord::of(record.dim, &points) {
                Ok(now) => {
                    for error in stored.compare(&now) {
                        failures.push(format!("{}: {error}", path.display()));
                    }
                }
                Err(e) => failures.push(format!("{}: {e}", path.display())),
            },
            (None, true) => failures.push(format!(
                "{}: no Delaunay observables for D <= {DELAUNAY_MAX_DIM}",
                path.display()
            )),
            (Some(_), false) => failures.push(format!(
                "{}: Delaunay observables beyond D = {DELAUNAY_MAX_DIM}",
                path.display()
            )),
            (None, false) => {}
        }
        if fresh.volume.is_none() != record.volume.is_none() {
            failures.push(format!(
                "{}: the volume mode differs from the freeze rule",
                path.display()
            ));
        }
    }
    failures
}

#[test]
fn records_round_trip_through_text() {
    for (_, record) in records() {
        assert_eq!(Record::parse(&record.to_text()).unwrap(), record);
    }
}
