"""The tables of a run of the comparison.

usage: report.py <results.jsonl>

Per row: each library's median time and its ratio to convx's, its peak
resident set (the median of three processes), and whether the counts of the
outputs agree. A report, not a judgement: nothing here is met or not met.
"""
import json
import statistics as st
import sys


def seconds(x):
    if x >= 1:
        return f"{x:.2f} s"
    if x >= 0.1:
        return f"{x * 1e3:.0f} ms"
    if x >= 0.01:
        return f"{x * 1e3:.1f} ms"
    return f"{x * 1e3:.2f} ms"


def megabytes(x):
    return f"{x / 2**20:.0f} MB" if x >= 10 * 2**20 else f"{x / 2**20:.1f} MB"


def main():
    rows = [json.loads(line) for line in open(sys.argv[1])]
    libraries = []
    for r in rows:
        for name in r['libraries']:
            if name not in libraries:
                libraries.append(name)
    print("| Row | " + " | ".join(f"{name}: time, peak" for name in libraries) + " | Counts |")
    print("| --- | " + " | ".join("---:" for _ in libraries) + " | --- |")
    for r in rows:
        cells, counts = [], {}
        base = st.median(b['time'] for b in r['libraries']['convx']['builds']) if 'convx' in r['libraries'] else None
        for name in libraries:
            entry = r['libraries'].get(name)
            if entry is None:
                cells.append("–")
                continue
            t = st.median(b['time'] for b in entry['builds'])
            ratio = f" ({t / base:.2f})" if base and name != 'convx' else ""
            cells.append(f"{seconds(t)}{ratio}, {megabytes(st.median(entry['peak']))}")
            counts[name] = tuple(entry['builds'][0]['counts'])
        same = len(set(counts.values())) == 1
        note = "agree" if same else "; ".join(f"{k} {v[0]}/{v[1]}" for k, v in counts.items())
        print(f"| {r['row']} | " + " | ".join(cells) + f" | {note} |")


if __name__ == '__main__':
    main()
