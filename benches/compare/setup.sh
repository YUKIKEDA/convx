#!/bin/bash
# Builds what a run of the comparison needs under $COMPARE_HOME (default
# ~/convx-compare): the adapters of the measured commit, and the CGAL
# program. The data is made by data/make.sh.
#
# usage: setup.sh <commit>
set -e
set -o pipefail
COMMIT="${1:?usage: setup.sh <commit>}"
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
H="${COMPARE_HOME:-$HOME/convx-compare}"
CARGO="$(command -v cargo || echo "$HOME/.cargo/bin/cargo")"

rm -rf "$H/src" "$H/bin"
mkdir -p "$H/src" "$H/bin"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$H/src"
(cd "$H/src/benches/compare/rust" && CARGO_TARGET_DIR="$H/target" "$CARGO" build --release --locked -q)
for program in "$H"/src/benches/compare/rust/src/bin/*.rs; do
  name="$(basename "$program" .rs)"
  cp "$H/target/release/$name" "$H/bin/"
done
g++ -std=c++17 -O3 -DNDEBUG -g -DCGAL_EIGEN3_ENABLED -I/usr/include/eigen3 -o "$H/bin/cgal_time" "$H/src/benches/parity/cgal_time.cpp" -lgmp -lmpfr
echo "$COMMIT" > "$H/commit"
ls "$H/bin"
echo "SETUP DONE in $H"
