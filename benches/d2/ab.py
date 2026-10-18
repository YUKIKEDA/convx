"""Summarizes ab.sh: per size and commit, the median over the runs of convx's median
build, its ratio to the base commit's, and the median of convx / spade.

usage: ab.py <out file> <base commit as given to ab.sh>
"""
import json
import statistics as st
import sys
from collections import defaultdict


def main():
    rows = [json.loads(line) for line in open(sys.argv[1]) if line.startswith('{')]
    base = sys.argv[2]
    by = defaultdict(list)
    for r in rows:
        by[(r['n'], r['label'])].append(r)
    labels = list(dict.fromkeys(r['label'] for r in rows))
    sizes = sorted({r['n'] for r in rows})
    print('| Points | ' + ' | '.join(labels) + ' | ' + ' | '.join(f'{l} / spade' for l in labels) + ' |')
    print('| ---: | ' + ' | '.join('---:' for _ in labels * 2) + ' |')
    for n in sizes:
        b = st.median(r['convx'] for r in by[(n, base)])
        cells = []
        for label in labels:
            m = st.median(r['convx'] for r in by[(n, label)])
            unit = f'{m * 1e6:.2f} us' if m < 1e-3 else f'{m * 1e3:.2f} ms'
            cells.append(f'{unit} ({m / b:.3f})')
        ratios = [f"{st.median(r['convx'] / r['spade'] for r in by[(n, label)]):.3f}" for label in labels]
        print(f'| {n} | ' + ' | '.join(cells) + ' | ' + ' | '.join(ratios) + ' |')


if __name__ == '__main__':
    main()
