"""The figures and the numbers of the published comparison, from a run's results.

usage: publish.py <results.jsonl> <out dir> <meta.json>

Writes into <out dir> (`docs/bench/` for the published one):

- `summary.json`: the run's description from <meta.json>, and for every row and
  library every build's time, the median and the minimum, the three peaks and
  their median, and the two counts;
- `hull-time.svg`, `delaunay-time.svg`: the median time against the number of
  points, one panel per kind of data and dimension, one line per library;
- `memory.svg`: the resident set over the time of one process that builds once,
  on three rows. A dot is a sample; a line without dots is a time in which no
  sample was read.

Needs matplotlib. Draws nothing it does not read from the results.
"""
import json
import os
import re
import statistics as st
import sys

import matplotlib

matplotlib.use('svg')
import matplotlib.pyplot as plt  # noqa: E402

plt.rcParams['svg.hashsalt'] = 'convx'
plt.rcParams['svg.fonttype'] = 'none'
plt.rcParams['font.size'] = 9

# One colour and marker per library, the same in every figure.
STYLE = {
    'convx': ('#1f4e9c', 'o'),
    'cgal': ('#c0392b', 's'),
    'qhull': ('#7f8c8d', '^'),
    'spade': ('#27ae60', 'D'),
    'delaunator': ('#8e44ad', 'v'),
    'delaunay': ('#d35400', 'P'),
    'parry3d': ('#16a085', 'X'),
    'geo': ('#b7950b', '*'),
}
NAME = {'cgal': 'CGAL', 'qhull': 'Qhull', 'parry3d': 'parry3d-f64'}

# Panels of the two time figures: (kind of data, dimension).
PANELS = [('cube', 2), ('cube', 3), ('sphere', 2), ('sphere', 3), ('insphere', 2), ('kuzmin', 2),
          ('insphere', 3), ('plummer', 3)]
MEMORY_ROWS = ['delaunay cube-d2-n1000000', 'delaunay cube-d3-n1000000', 'delaunay kuzmin-d2-n10000000']


def parse(row):
    operation, data = row.split()
    m = re.fullmatch(r'(.+)-d(\d+)-n(\d+)', data)
    return operation, m.group(1), int(m.group(2)), int(m.group(3))


def summary(rows, meta):
    out = dict(meta)
    out['rows'] = []
    for r in rows:
        operation, kind, dim, points = parse(r['row'])
        libraries = {}
        for name, entry in r['libraries'].items():
            times = [b['time'] for b in entry['builds']]
            libraries[name] = {
                'seconds': times,
                'median_seconds': st.median(times),
                'min_seconds': min(times),
                'peak_bytes': entry['peak'],
                'median_peak_bytes': st.median(entry['peak']),
                'counts': entry['builds'][0]['counts'],
            }
        out['rows'].append({'operation': operation, 'data': f'{kind}-d{dim}-n{points}', 'kind': kind,
                            'dim': dim, 'points': points, 'rounds': r['rounds'], 'reps': r['reps'],
                            'libraries': libraries})
    return out


def time_figure(rows, operation, path):
    panels = []
    for kind, dim in PANELS:
        series = {}
        for r in rows:
            op, k, d, n = parse(r['row'])
            if (op, k, d) != (operation, kind, dim):
                continue
            for name, entry in r['libraries'].items():
                series.setdefault(name, []).append((n, st.median(b['time'] for b in entry['builds'])))
        if sum(len(s) for s in series.values()) and max(len(s) for s in series.values()) >= 2:
            panels.append((kind, dim, series))
    cols = 3
    rows_of_panels = -(-len(panels) // cols)
    fig, axes = plt.subplots(rows_of_panels, cols, figsize=(9, 2.8 * rows_of_panels), squeeze=False)
    for ax in axes.flat[len(panels):]:
        ax.set_visible(False)
    for ax, (kind, dim, series) in zip(axes.flat, panels):
        for name in STYLE:
            if name not in series:
                continue
            points = sorted(series[name])
            colour, marker = STYLE[name]
            ax.plot([p[0] for p in points], [p[1] for p in points], marker=marker, color=colour,
                    label=NAME.get(name, name), linewidth=1.2, markersize=4)
        ax.set_xscale('log')
        ax.set_yscale('log')
        ax.set_title(f'{kind}, D = {dim}')
        ax.set_xlabel('points')
        ax.set_ylabel('seconds (median)')
        ax.grid(True, which='major', linewidth=0.3)
        ax.legend(fontsize=7)
    fig.suptitle(f'{"Convex hull" if operation == "hull" else "Delaunay triangulation"}: time against size')
    fig.tight_layout()
    fig.savefig(path, metadata={'Date': None})
    plt.close(fig)


def memory_figure(rows, path):
    chosen = [r for name in MEMORY_ROWS for r in rows if r['row'] == name]
    fig, axes = plt.subplots(1, len(chosen), figsize=(9, 2.8), squeeze=False)
    for ax, r in zip(axes.flat, chosen):
        for name in STYLE:
            entry = r['libraries'].get(name)
            if not entry or not entry['timeline']:
                continue
            colour, _ = STYLE[name]
            ax.plot([s[0] for s in entry['timeline']], [s[1] / 2**20 for s in entry['timeline']],
                    color=colour, label=NAME.get(name, name), linewidth=0.8, marker='.', markersize=2)
        ax.set_title(r['row'])
        ax.set_xlabel('seconds since the process started')
        ax.set_ylabel('resident set, MB')
        ax.grid(True, linewidth=0.3)
        ax.legend(fontsize=7)
    fig.suptitle('Resident set over the time of one process that reads the file and builds once')
    fig.tight_layout()
    fig.savefig(path, metadata={'Date': None})
    plt.close(fig)


def main():
    results, out, meta_path = sys.argv[1], sys.argv[2], sys.argv[3]
    rows = [json.loads(line) for line in open(results)]
    meta = json.load(open(meta_path))
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, 'summary.json'), 'w', newline='\n') as f:
        json.dump(summary(rows, meta), f, indent=1)
        f.write('\n')
    time_figure(rows, 'hull', os.path.join(out, 'hull-time.svg'))
    time_figure(rows, 'delaunay', os.path.join(out, 'delaunay-time.svg'))
    memory_figure(rows, os.path.join(out, 'memory.svg'))


if __name__ == '__main__':
    main()
