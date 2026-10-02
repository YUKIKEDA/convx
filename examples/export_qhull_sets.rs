//! Writes the performance sets of `benches/sets.txt` as Qhull input files
//! (docs/verification.md, Open a performance set).
//!
//! ```text
//! cargo run --release --example export_qhull_sets [-- <filter>]
//! ```
//!
//! Each set becomes `.dev/perf/<family>-d<dim>-n<count>-s<seed>.txt` in Qhull's
//! input format: the dimension, the point count, then one point per line.
//! Coordinates use Rust's shortest round-trip `f64` formatting, so Qhull reads
//! the same bit patterns convx is given. Existing files are kept. The
//! optional filter is split at `-` into tokens, and a set is kept only when
//! every token equals one `-`-separated token of its name: `d3-n10000` keeps
//! `cube-d3-n10000-s1` and `sphere-d3-n10000-s1`, not `cube-d3-n100000-s1`.
//!
//! These files are never read by a correctness test.

#[path = "../tests/common/generator.rs"]
mod generator;

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use generator::{Family, GENERATOR};

struct Set {
    family: Family,
    dim: usize,
    count: usize,
    seed: u64,
}

fn read_sets(text: &str) -> Result<Vec<Set>, String> {
    let mut generator = None;
    let mut sets = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["generator", id] => generator = Some((*id).to_string()),
            ["set", family, dim, count, seed] => {
                let parse = |s: &str| s.parse::<u64>().map_err(|e| format!("{line}: {e}"));
                sets.push(Set {
                    family: Family::from_name(family)
                        .ok_or_else(|| format!("unknown family in {line}"))?,
                    dim: parse(dim)? as usize,
                    count: parse(count)? as usize,
                    seed: parse(seed)?,
                });
            }
            _ => return Err(format!("cannot read: {line}")),
        }
    }
    match generator {
        Some(id) if id == GENERATOR => Ok(sets),
        Some(id) => Err(format!(
            "benches/sets.txt names generator {id}, the code is {GENERATOR}"
        )),
        None => Err("benches/sets.txt names no generator".into()),
    }
}

/// Whether every filter token is a whole `-`-separated token of `stem`.
fn matches(stem: &str, wanted: &[&str]) -> bool {
    let tokens: Vec<&str> = stem.split('-').collect();
    wanted.iter().all(|w| tokens.contains(w))
}

fn run() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(root.join("benches/sets.txt")).map_err(|e| e.to_string())?;
    let filter = std::env::args().nth(1).unwrap_or_default();
    let wanted: Vec<&str> = filter.split('-').filter(|t| !t.is_empty()).collect();
    let out = root.join(".dev/perf");
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    for set in read_sets(&text)? {
        let stem = format!(
            "{}-d{}-n{}-s{}",
            set.family.name(),
            set.dim,
            set.count,
            set.seed
        );
        if !matches(&stem, &wanted) {
            continue;
        }
        let name = format!("{stem}.txt");
        let path = out.join(&name);
        if path.exists() {
            println!("kept    {}", path.display());
            continue;
        }
        let points = set.family.points(set.dim, set.count, set.seed);
        let mut body = format!("{}\n{}\n", set.dim, set.count);
        for point in points.chunks_exact(set.dim) {
            let line: Vec<String> = point.iter().map(|x| format!("{x}")).collect();
            let _ = writeln!(body, "{}", line.join(" "));
        }
        fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("written {}", path.display());
    }
    Ok(())
}

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
