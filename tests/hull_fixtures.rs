//! Steady run of the frozen hull records (docs/verification.md). Each record
//! is rebuilt from its seed and compared with the observables it stores. This
//! test only reads records.

// The record loader unwraps files that the repository itself provides.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::generator::GENERATOR;
use common::record::Record;
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
    assert!(records.len() >= 30, "only {} hull records", records.len());
    let mut failures = Vec::new();
    for (path, record) in &records {
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
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn records_round_trip_through_text() {
    for (_, record) in records() {
        assert_eq!(Record::parse(&record.to_text()).unwrap(), record);
    }
}
