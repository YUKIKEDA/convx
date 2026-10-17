#!/bin/bash
# Times the plain probe of several commits, taking turns, pinned to one core.
#
# usage: ab.sh <out file> <runs> <commit> [<commit> ...] -- <n> [<n> ...]
# A commit may be given twice, under two spellings of its hash (7 and 12
# characters), as the A/A control. ab.py summarizes the out file.
set -e
set -o pipefail
OUT=$1; shift
RUNS=$1; shift
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
H="${D2_HOME:-$HOME/convx-d2-ab}"
CORE="${D2_CORE:-2}"
CARGO="$(command -v cargo || echo "$HOME/.cargo/bin/cargo")"
COMMITS=()
while [ "$1" != "--" ]; do COMMITS+=("$1"); shift; done
shift
rm -rf "$H" && mkdir -p "$H"
for c in "${COMMITS[@]}"; do
  mkdir -p "$H/$c/src" "$H/$c/probe"
  git -C "$REPO" archive "$c" | tar -x -C "$H/$c/src"
  cp -r "$HERE/probe/src" "$HERE/probe/Cargo.lock" "$H/$c/probe/"
  sed "s#CONVX_PATH#$H/$c/src#" "$HERE/probe/Cargo.toml" > "$H/$c/probe/Cargo.toml"
  (cd "$H/$c/probe" && CARGO_TARGET_DIR="$H/target-$c" "$CARGO" build --release --locked -q)
done
: > "$OUT"
for _ in $(seq "$RUNS"); do
  for c in "${COMMITS[@]}"; do
    taskset -c "$CORE" "$H/target-$c/release/probe" "$c" "$@" >> "$OUT"
  done
done
rm -rf "$H"
echo "ab: ${#COMMITS[@]} commits, $RUNS runs"
