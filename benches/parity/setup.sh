#!/bin/bash
# Builds what a parity run needs under $PARITY_HOME (default ~/convx-bench):
# two copies of the measured commit (plain, and with the phase marks), the
# CGAL program, and the point files.
#
# usage: setup.sh <commit>
set -e
COMMIT="${1:?usage: setup.sh <commit>}"
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
H="${PARITY_HOME:-$HOME/convx-bench}"
CARGO="$(command -v cargo || echo "$HOME/.cargo/bin/cargo")"

rm -rf "$H"
mkdir -p "$H/plain" "$H/marked"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$H/plain"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$H/marked"
python3 "$HERE/marks.py" "$H/marked"
for copy in plain marked; do
  cp "$HERE/parity_time.rs" "$H/$copy/examples/"
  (cd "$H/$copy" && CARGO_PROFILE_RELEASE_DEBUG=true "$CARGO" build --release -q --example parity_time)
done
cd "$H/plain"
for f in d2 d3 d4-n10000 d4-n100000 d5-n10000 d5-n100000 d6-n10000 d6-n100000 cubesurf grid-d6; do
  "$CARGO" run --release -q --example export_qhull_sets -- $f
done
# The Delaunay lattices (ADR 0006, amendment of 2026-10-13) are rbox's:
# rotated integer lattices, every unit cell a cospherical group.
rbox 10000 M3,4 D2 n > "$H/plain/.dev/perf/lattice-d2-n10000-s1.txt"
rbox 27000 M3,4,5 D3 n > "$H/plain/.dev/perf/lattice-d3-n27000-s1.txt"
# Kuzmin sites (amendment of 2026-10-13, #620): dense about the origin with a
# long tail, by the comparison's generator of the PBBS distributions.
python3 "$REPO/benches/compare/data/pbbs.py" kuzmin 2 1000000 1 "$H/plain/.dev/perf/kuzmin-d2-n1000000-s1.txt"
g++ -std=c++17 -O3 -DNDEBUG -g -DCGAL_EIGEN3_ENABLED -I/usr/include/eigen3 -o "$H/cgal_time" "$HERE/cgal_time.cpp" -lgmp -lmpfr
echo "$COMMIT" > "$H/commit"
echo "point files: $(ls "$H/plain/.dev/perf" | wc -l)"
echo "SETUP DONE in $H"
