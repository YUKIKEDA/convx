"""Runs convx, CGAL, and Qhull on each set, alternated per round, pinned to one core.

usage: runner.py <sets.txt> <results.jsonl>

Appends one JSON line per set; a set already in the results file is skipped,
so a run that stopped goes on where it was, and a set is run again by taking
its line out. The method is in docs/verification.md ("Parity run of P7").
"""
import json
import os
import re
import subprocess
import sys
import time

HOME = os.environ.get('PARITY_HOME', os.path.expanduser('~/convx-bench'))
# The copy with the phase marks: its builds give the phases only.
MARKED = HOME + '/marked/target/release/examples/parity_time'
# The judged build() comes from a copy without the marks, whose writes to
# standard error were inside the timed section (#518).
PLAIN = HOME + '/plain/target/release/examples/parity_time'
CGAL = HOME + '/cgal_time'
POINTS = HOME + '/plain/.dev/perf'
PIN = ['taskset', '-c', os.environ.get('PARITY_CORE', '2')]


def plain(mode, path, reps):
    out = subprocess.run(PIN + [PLAIN, mode, path, str(reps)], capture_output=True, text=True, check=True)
    builds = []
    for line in out.stderr.splitlines():
        w = line.split()
        if w and w[0] == 'build':
            builds.append({'build': float(w[1]), 'counts': [int(w[2]), int(w[3])], 'planes': float(w[4])})
    assert len(builds) == reps, out.stderr[-500:]
    return builds


def convx(mode, path, reps):
    """The judged builds from the plain copy, with the phases of as many
    builds of the marked copy, run right after it."""
    judged = plain(mode, path, reps)
    for b, m in zip(judged, marked(mode, path, reps)):
        b.update({k: m[k] for k in ('construct', 'after', 'publish')})
        b['marked'] = m['build']
    return judged


def marked(mode, path, reps):
    out = subprocess.run(PIN + [MARKED, mode, path, str(reps)], capture_output=True, text=True, check=True)
    builds, marks = [], {}
    for line in out.stderr.splitlines():
        w = line.split()
        if w[0] == 'mark':
            # Sub-hulls of coplanar faces mark again; the outer build's marks come first.
            marks.setdefault(w[1], float(w[2]))
        elif w[0] == 'build':
            builds.append({
                'build': float(w[1]),
                'construct': marks['construct_end'] - marks['construct_start'],
                'after': marks['publish_start'] - marks['construct_end'],
                'publish': marks['publish_end'] - marks['publish_start'],
                'counts': [int(w[2]), int(w[3])],
                'planes': float(w[4]),
            })
            marks = {}
    assert len(builds) == reps, out.stderr[-500:]
    return builds


def cgal(mode, path, reps):
    out = subprocess.run(PIN + [CGAL, mode, path, str(reps)], capture_output=True, text=True, check=True)
    rows = [l.split() for l in out.stdout.splitlines() if l.strip()]
    assert len(rows) == reps, out.stdout[-500:]
    # Delaunay lines carry a fourth field: CGAL's construction from the
    # points alone, the reference before #455.
    return [{'time': float(r[0]), 'counts': [int(r[1]), int(r[2])], **({'points': float(r[3])} if len(r) > 3 else {})} for r in rows]


def number(text, label):
    m = re.search(re.escape(label) + r'\s*:?\s*([0-9.]+)', text)
    return m.group(1) if m else None


def qhull(mode, path):
    tool = 'qconvex' if mode == 'hull' else 'qdelaunay'
    sink = HOME + '/qhull.out'
    start = time.perf_counter()
    out = subprocess.run(PIN + [tool, 'i', 's', 'TI', path, 'TO', sink], capture_output=True, text=True, check=True)
    whole = time.perf_counter() - start
    text = out.stderr + out.stdout
    return {
        'whole': whole,
        'compute': float(number(text, 'CPU seconds to compute hull (after input)')),
        'hyperplanes': int(number(text, 'Number of hyperplanes created')),
        'distance_tests': int(number(text, 'Number of distance tests for qhull')),
        'vertices': number(text, 'Number of vertices') or number(text, 'Number of input sites'),
        'facets': number(text, 'Number of facets') or number(text, 'Number of Delaunay regions'),
    }


def main():
    sets_file, results = sys.argv[1], sys.argv[2]
    done = set()
    if os.path.exists(results):
        for line in open(results):
            done.add(json.loads(line)['set'])
    for line in open(sets_file):
        if not line.strip() or line.startswith('#'):
            continue
        mode, family, dim, count = line.split()
        name = f'{mode} {family} {dim} {count}'
        if name in done:
            continue
        path = f'{POINTS}/{family}-d{dim}-n{count}-s1.txt'
        first = plain(mode, path, 1)[0]['build']
        rounds, reps = (5, 3) if first < 1.5 else (3, 1)
        record = {'set': name, 'rounds': rounds, 'reps': reps, 'convx': [], 'cgal': [], 'qhull': []}
        for _ in range(rounds):
            record['convx'] += convx(mode, path, reps)
            record['cgal'] += cgal(mode, path, reps)
            record['qhull'] += [qhull(mode, path) for _ in range(reps)]
        with open(results, 'a') as f:
            f.write(json.dumps(record) + '\n')
        print(name, 'done', time.strftime('%H:%M:%S'), flush=True)
    print('ALL DONE', flush=True)


main()
