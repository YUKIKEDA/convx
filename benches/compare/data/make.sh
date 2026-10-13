#!/bin/bash
# Writes the data files of the comparison under $COMPARE_HOME/data (default
# ~/convx-compare/data), and a line per file: its dimension, its points,
# and how many of them are distinct. Nothing is committed (publish.mdc).
#
# usage: make.sh <rbox|pbbs|scans>
set -e
set -o pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
OUT="${COMPARE_HOME:-$HOME/convx-compare}/data"
mkdir -p "$OUT"
case "${1:?usage: make.sh <rbox|pbbs|scans>}" in
  rbox)
    grep -v '^#' "$HERE/rbox.txt" | while read -r name dim count args; do
      [ -z "$name" ] && continue
      # shellcheck disable=SC2086
      rbox $args n > "$OUT/$name.txt"
      read -r d < "$OUT/$name.txt"
      n="$(sed -n 2p "$OUT/$name.txt")"
      if [ "$d" != "$dim" ] || [ "$n" != "$count" ]; then
        echo "$name: the file says D = $d and $n points, the list $dim and $count" >&2
        exit 1
      fi
      distinct="$(tail -n +3 "$OUT/$name.txt" | sort -u | wc -l)"
      echo "$name D$d points $n distinct $distinct"
    done
    ;;
  pbbs)
    grep -v '^#' "$HERE/pbbs.txt" | while read -r name dim count dist seed; do
      [ -z "$name" ] && continue
      python3 "$HERE/pbbs.py" "$dist" "$dim" "$count" "$seed" "$OUT/$name.txt"
      distinct="$(tail -n +3 "$OUT/$name.txt" | sort -u | wc -l)"
      echo "$name D$dim points $count distinct $distinct"
    done
    ;;
  scans)
    python3 "$HERE/scans.py" "$OUT"
    ;;
  *)
    echo "unknown tier: $1" >&2
    exit 1
    ;;
esac
