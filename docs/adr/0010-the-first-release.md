# 0010. The first release: 0.1.0 on crates.io

## Status

Accepted.

## Date

2026-10-12

## Issue

#592 (R1), #593 (R1-1). Decided by the owner in the Grill on #592.

## Context

convx has had no release: `Cargo.toml` says `publish = false` and version 0.0.0, the repository has no license file, and no rule covers a published package. P7 is closed, the parity runs stand at 37 of 38 judged sets within 1.05 of Qhull and CGAL, and the owner opened the release.

## Decision

- **The version is 0.1.0, on crates.io.** A 0.x minor version may break the public API. The contract is what an external crate can name.
- **The license is `MIT OR Apache-2.0`.** The owner accepts one license or two; two is what Rust crates commonly carry and what the owner's gprx has, and it asks the least of a user. The dependencies (`faer`, `pulp`, `thiserror`) are checked against it in R1-2.
- **The MSRV is 1.89**, as `rust-version` says and as CI tests. It is raised only by a change the changelog names.
- **The owner runs `cargo publish`** and pushes the tag. No agent does.
- **A comparison is published with the release**, on standard data: `rbox` (Qhull's generator), the distributions of PBBS, and the Stanford scans, against Qhull, CGAL, and Rust crates. It times the construction of the same output (ADR 0006) and reports memory as the peak resident set and over time. It is a report, not a test.
- **The parity run stays the internal judgement** (ADR 0006) and is not the comparison.
- **No benchmark is followed by a change to the code it measured.** The filter's exact scaling (#589) is the first row of R1 for that reason.

## What is not decided here

- Whether a line-coverage gate joins CI before the release, as gprx has one. It is not a row of R1; the owner can add it.
- Which Rust crates enter the comparison: R1-7 surveys them first.
- The files of the package: R1-2.
