//! Freezes hull records (docs/verification.md, Freeze a simulation).
//!
//! ```text
//! cargo run --example freeze_hull_fixture -- <family> <dim> <count> <seed>
//! cargo run --example freeze_hull_fixture -- --standard
//! cargo run --example freeze_hull_fixture -- --extend
//! ```
//!
//! A record is written only after the build passes the §10 invariants, which
//! debug builds of convx check inside every build, so this tool refuses to run
//! without debug assertions. Records of D <= 5 also get the Delaunay and
//! Voronoi observables, written only after the exact oracles pass on the
//! integer families (`nearsphere` is scaled by 2^8 to integers first). It
//! never overwrites an existing record: revising one is a reviewed change by
//! hand (docs/verification.md). `--extend` appends the Delaunay and Voronoi
//! observables to existing records of D <= 5 that lack them, leaving every
//! stored line as it is (Extend a record).

#[path = "../tests/common/generator.rs"]
mod generator;
// The steady-run test uses the parser and comparison; this tool does not.
#[allow(dead_code)]
#[path = "../tests/common/oracle.rs"]
mod oracle;
#[allow(dead_code)]
#[path = "../tests/common/record.rs"]
mod record;

use std::fs;
use std::path::PathBuf;

use convx::{ConvexHullBuilder, DelaunayBuilder, VoronoiBuilder};
use generator::{Family, GENERATOR};
use record::{DelaunayRecord, Record, DELAUNAY_MAX_DIM};

/// The standard set: every family in D = 2..=6 with seed 1, plus a second
/// seed for the cube and grid families. Existing records are kept, so adding
/// a family only adds its records (Add a case).
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

/// The Delaunay and Voronoi observables, after the exact oracles pass on
/// the integer families.
fn delaunay(family: Family, dim: usize, points: &[f64]) -> Result<DelaunayRecord, String> {
    let observed = DelaunayRecord::of(dim, points)?;
    let integer = match family {
        Family::NearSphere => Some(points.iter().map(|x| x * 256.0).collect::<Vec<f64>>()),
        f if f.is_integer() => Some(points.to_vec()),
        _ => None,
    };
    if let Some(sites) = integer {
        // Scaling by a power of two keeps every exact sign, so the scaled
        // build has the same topology; the oracles run on it.
        let t = DelaunayBuilder::new(dim, &sites)
            .build()
            .map_err(|e| e.to_string())?;
        let v = VoronoiBuilder::new(dim, &sites)
            .build()
            .map_err(|e| e.to_string())?;
        oracle::check_delaunay(&t, &sites);
        oracle::check_voronoi(&v, &t, &sites);
        if DelaunayRecord::of(dim, &sites)? != observed {
            return Err("the scaled sites changed the topology".into());
        }
    }
    Ok(observed)
}

fn records_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hull")
}

/// Appends the Delaunay and Voronoi observables to every record of
/// D <= 5 that lacks them.
fn extend() -> Result<(), String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(records_dir())
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    paths.sort();
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let record = Record::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if record.dim > DELAUNAY_MAX_DIM || record.delaunay.is_some() {
            println!("kept     {}", path.display());
            continue;
        }
        let points = record.family.points(record.dim, record.count, record.seed);
        let observed = delaunay(record.family, record.dim, &points)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        fs::write(&path, text + &observed.to_text())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("extended {}", path.display());
    }
    Ok(())
}

fn freeze(family: Family, dim: usize, count: usize, seed: u64) -> Result<(), String> {
    let points = family.points(dim, count, seed);
    let hull = ConvexHullBuilder::new(dim, &points)
        .build()
        .map_err(|e| format!("{} d{dim} n{count} s{seed}: {e}", family.name()))?;
    let mut record = Record::of(GENERATOR, family, dim, count, seed, &points, &hull);
    if dim <= DELAUNAY_MAX_DIM {
        record.delaunay = Some(
            delaunay(family, dim, &points)
                .map_err(|e| format!("{} d{dim} n{count} s{seed}: {e}", family.name()))?,
        );
    }
    let path = records_dir().join(record.file_name());
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
        [flag] if flag == "--extend" => extend(),
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
        _ => Err(
            "usage: freeze_hull_fixture <family> <dim> <count> <seed> | --standard | --extend"
                .into(),
        ),
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
