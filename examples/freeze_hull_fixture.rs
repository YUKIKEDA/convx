//! Freezes hull records (docs/verification.md, Freeze a simulation).
//!
//! ```text
//! cargo run --example freeze_hull_fixture -- <family> <dim> <count> <seed>
//! cargo run --example freeze_hull_fixture -- --standard
//! ```
//!
//! A record is written only after the build passes the §10 invariants, which
//! debug builds of convx check inside every build, so this tool refuses to run
//! without debug assertions. It never overwrites an existing record: revising
//! one is a reviewed change by hand (docs/verification.md).

#[path = "../tests/common/generator.rs"]
mod generator;
// The steady-run test uses the parser and comparison; this tool does not.
#[allow(dead_code)]
#[path = "../tests/common/record.rs"]
mod record;

use std::fs;
use std::path::PathBuf;

use convx::ConvexHullBuilder;
use generator::{Family, GENERATOR};
use record::Record;

/// The standard set: every family in D = 2..=6 with seed 1, plus a second
/// seed for the cube and grid families.
fn standard() -> Vec<(Family, usize, u64)> {
    let mut set = Vec::new();
    for family in Family::ALL {
        for dim in 2..=6 {
            set.push((family, dim, 1));
        }
    }
    for family in [Family::Cube, Family::Grid] {
        for dim in 2..=6 {
            set.push((family, dim, 2));
        }
    }
    set
}

/// Point count per dimension, small enough for debug runs of the checker.
fn count_for(dim: usize) -> usize {
    match dim {
        0..=3 => 60,
        4 => 40,
        5 => 25,
        _ => 18,
    }
}

fn freeze(family: Family, dim: usize, count: usize, seed: u64) -> Result<(), String> {
    let points = family.points(dim, count, seed);
    let hull = ConvexHullBuilder::new(dim, &points)
        .build()
        .map_err(|e| format!("{} d{dim} n{count} s{seed}: {e}", family.name()))?;
    let record = Record::of(GENERATOR, family, dim, count, seed, &points, &hull);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/hull")
        .join(record.file_name());
    if path.exists() {
        println!("kept   {}", path.display());
        return Ok(());
    }
    fs::write(&path, record.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("frozen {}", path.display());
    Ok(())
}

fn run() -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err(
            "run without --release: the invariants are checked only with debug assertions".into(),
        );
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--standard" => {
            for (family, dim, seed) in standard() {
                freeze(family, dim, count_for(dim), seed)?;
            }
            Ok(())
        }
        [family, dim, count, seed] => {
            let family =
                Family::from_name(family).ok_or_else(|| format!("unknown family {family}"))?;
            let parse = |s: &str| s.parse::<u64>().map_err(|e| format!("{s}: {e}"));
            freeze(
                family,
                parse(dim)? as usize,
                parse(count)? as usize,
                parse(seed)?,
            )
        }
        _ => Err("usage: freeze_hull_fixture <family> <dim> <count> <seed> | --standard".into()),
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
