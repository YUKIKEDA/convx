# The parity run's tools

What the parity run of `docs/verification.md` ("Parity run of P7") is made with. The method and its reasons are there; the judgement is ADR 0006; each run's result is a section of `docs/bench.md`. These files were scratch until #581 and were rebuilt for two runs.

| File | What it is |
| --- | --- |
| `sets.txt` | The 41 sets of the run, one `mode family dim count` a line |
| `setup.sh` | Builds two copies of the measured commit, the CGAL program, and the point files |
| `parity_time.rs` | The example each copy is timed through: `build()`, the counts, and `planes()` |
| `marks.py` | Writes the phase marks into one of the copies. They never go into `src` (`bench.mdc`) |
| `cgal_time.cpp` | The CGAL reference (#215), with the Delaunay output of #455 and the D = 2 hull of #456 |
| `runner.py` | Times convx, CGAL, and Qhull on each set, alternated, pinned to one core |
| `report.py` | The tables of a run, and the sets to run again |

## What the machine needs

Linux or WSL2, with `cargo`, `python3`, `g++`, `taskset`, Qhull (`qconvex`, `qdelaunay`), and the headers of CGAL, Eigen 3, GMP, and MPFR. On Ubuntu: `qhull-bin`, `libcgal-dev`, `libeigen3-dev`, `libgmp-dev`, `libmpfr-dev`. Each section of `docs/bench.md` names the versions it ran.

## A run

Nothing else runs on the machine meanwhile, a build least of all.

```sh
bash benches/parity/setup.sh <commit>          # about ten minutes
cd ~/convx-bench
python3 <repo>/benches/parity/runner.py <repo>/benches/parity/sets.txt results.jsonl   # about 45 minutes
python3 <repo>/benches/parity/report.py results.jsonl [earlier.jsonl]
```

- `PARITY_HOME` is where everything is built and written (default `~/convx-bench`), and `PARITY_CORE` the core the tools are pinned to (default 2).
- The runner skips a set that is already in the results file. A run that stopped goes on where it was.
- **Sets to run again.** The report lists the sets whose slowest judged build is 1.5 times the fastest or more. Keep a copy of the results, take those sets' lines out, and run the runner again; the section records both readings.
- **A ratio between two commits** is not read from two runs. Time the two commits against each other in one sitting, alternated, as the sections of #569 and #577 do, with enough rounds that the minimum and the median agree.

## After a run

The results go into `docs/bench.md`. `PARITY_HOME` is removed: it holds two build trees and about 280 MB of point files (`scratch.mdc`).

## When the code moves

`marks.py` finds its places by the text of six lines in `src` and stops when one is gone. Move the mark to where that step is now, and say so in the run's section.
