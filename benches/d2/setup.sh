#!/bin/bash
# Builds the probe of the Delaunay build of D = 2 twice under $D2_HOME
# (default ~/convx-d2): against a plain copy of convx at <commit>, and
# against a copy with the stage marks of marks.py, which never go into src.
#
# usage: setup.sh <commit>
# then:  taskset -c 2 ~/convx-d2/bin/probe-plain plain 10 100 1000
#        taskset -c 2 ~/convx-d2/bin/probe-marked marked 10 100 1000
set -e
set -o pipefail
COMMIT="${1:?usage: setup.sh <commit>}"
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
H="${D2_HOME:-$HOME/convx-d2}"
CARGO="$(command -v cargo || echo "$HOME/.cargo/bin/cargo")"
rm -rf "$H" && mkdir -p "$H/plain" "$H/marked" "$H/bin"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$H/plain"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$H/marked"
python3 "$HERE/marks.py" "$H/marked"
for copy in plain marked; do
  mkdir -p "$H/probe-$copy"
  cp -r "$HERE/probe/src" "$HERE/probe/Cargo.lock" "$H/probe-$copy/"
  sed "s#CONVX_PATH#$H/$copy#" "$HERE/probe/Cargo.toml" > "$H/probe-$copy/Cargo.toml"
  features=""
  [ "$copy" = marked ] && features="--features marked"
  # shellcheck disable=SC2086
  (cd "$H/probe-$copy" && CARGO_TARGET_DIR="$H/target-$copy" "$CARGO" build --release --locked -q $features)
  cp "$H/target-$copy/release/probe" "$H/bin/probe-$copy"
done
echo "$COMMIT" > "$H/commit"
ls -l "$H/bin"
