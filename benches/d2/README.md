# The Delaunay build of D = 2 beside spade

The tools of R2-1 (#667): where the time of `DelaunayBuilder::new(2, points).build()` goes, beside `spade`'s `DelaunayTriangulation::bulk_load` on the same points. The results are in `docs/bench.md`.

| File | What |
| --- | --- |
| `probe/` | A program that times convx's build, convx's build again (the A/A control), and spade's bulk load (with the conversion of the points to `Point2`), taking turns in one process, on points uniform in the unit square; medians after a warm-up. With the feature `marked`, it also prints the median time of each stage of convx's build |
| `marks.py` | Writes the stage marks into a copy of the crate: scaling, acceptance, sites, flat, order, copy, insertion, after the insertion, publication. They never go into `src` (`.cursor/rules/bench.mdc`). It finds its places by the text of the lines it marks and stops when one is gone |
| `setup.sh` | Builds the probe against a plain copy of a commit and against a marked copy, under `~/convx-d2` |
| `ab.sh`, `ab.py` | Time the plain probe of several commits taking turns (a commit given twice, under two spellings, is the A/A control), and summarize the runs |

```sh
bash benches/d2/setup.sh <commit>
taskset -c 2 ~/convx-d2/bin/probe-plain plain 10 100 1000 10000 100000 1000000
taskset -c 2 ~/convx-d2/bin/probe-marked marked 10 100 1000 10000 100000 1000000
```

The probe builds on Windows too: copy `probe/`, set the path of `convx` in its `Cargo.toml`, and pin the process to one core. `~/convx-d2` is removed after a run (`.cursor/rules/scratch.mdc`).
