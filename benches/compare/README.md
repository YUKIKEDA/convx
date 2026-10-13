# The comparison with other libraries

What the published comparison (`docs/comparison.md`, R1 of the roadmap) is made with. It is a report, not a test, and it is not the parity run: that is `benches/parity/`, the internal judgement of ADR 0006.

## Data

Three tiers, none of them committed as data (`.cursor/rules/publish.mdc`). `data/make.sh <tier>` writes the files under `$COMPARE_HOME/data` (default `~/convx-compare/data`) and prints, per file, its dimension, its points, and how many of them are distinct. Every file is a dimension, a count, and the points, as `rbox` writes them and as Qhull reads them.

| Tier | Made by | What it is |
| --- | --- | --- |
| `rbox` | `data/rbox.txt`, the arguments of Qhull's `rbox` | Uniform in a cube, on a sphere, a rotated integer lattice, and points with near-coincident companions; D = 2 to 8 |
| `pbbs` | `data/pbbs.txt` and `data/pbbs.py` | The distributions of the Problem Based Benchmark Suite: uniform in a ball, Kuzmin (D = 2), Plummer (D = 3). PBBS's formulas with another random source: the distribution of a PBBS file, not its points |
| `scans` | `data/scans.py` | The reconstructed surfaces of Bunny, Dragon, Happy Buddha, and Armadillo from the Stanford 3D Scanning Repository, downloaded and checked against a recorded SHA-256 |

`data/test_pbbs.py` checks each distribution against the closed form of its radial law (`python3 -m unittest` in `data/`).

The scans belong to the Stanford Computer Graphics Laboratory (http://graphics.stanford.edu/data/3Dscanrep/), which asks that the source be acknowledged and that the models not be used commercially without permission. They are downloaded by whoever runs the comparison and are not redistributed here.

The whole of the three tiers is about 950 MB (250 MB for `rbox`, 700 MB for the other two). It is removed after a run (`scratch.mdc`).
