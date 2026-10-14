"""Runs the libraries of the comparison on each row, alternated, pinned to one core.

usage: runner.py <rows.txt> <results.jsonl>

A row is `<hull|delaunay> <data file name> <library> [<library> ...]`. For each
row and library the runner records, as one JSON line:

- the seconds of the construction of the output, as the library's adapter
  times it inside its own process (reading the file is outside);
- the two counts of the output (vertices and facets, or sites and simplices);
- the peak resident set of each process, from the kernel (`ru_maxrss`), read
  by GNU time;
- the resident set over time of one process that builds once, sampled from
  `/proc/<pid>/statm`.

A row already in the results file is skipped, so a run that stopped goes on
where it was. Each library runs in a process of its own; the libraries of a
row take turns, round by round. Rounds and builds per process follow the
parity run (docs/verification.md): 5 rounds of 3 builds when the first
library's first build takes under 1.5 s, otherwise 3 rounds of 1.
"""
import json
import os
import re
import subprocess
import sys
import time

HOME = os.environ.get('COMPARE_HOME', os.path.expanduser('~/convx-compare'))
DATA = HOME + '/data'
BIN = HOME + '/bin'
PIN = ['taskset', '-c', os.environ.get('COMPARE_CORE', '2')]
PAGE = os.sysconf('SC_PAGE_SIZE')


def adapter(name):
    """A library whose adapter prints `build <seconds> <a> <b>` per build."""
    def command(operation, path, reps):
        return [f'{BIN}/{name}', operation, path, str(reps)]

    def parse(out, err):
        return [{'time': float(w[1]), 'counts': [int(w[2]), int(w[3])]}
                for w in (line.split() for line in out.splitlines()) if w and w[0] == 'build']
    return command, parse


def cgal():
    """The CGAL program of the parity run: `<seconds> <a> <b>` per build."""
    def command(operation, path, reps):
        return [f'{BIN}/cgal_time', operation, path, str(reps)]

    def parse(out, err):
        return [{'time': float(w[0]), 'counts': [int(w[1]), int(w[2])]}
                for w in (line.split() for line in out.splitlines()) if w]
    return command, parse


def qhull():
    """`qconvex` or `qdelaunay`, one build a process: the time is Qhull's own
    "CPU seconds to compute hull (after input)", and the output is written."""
    def number(text, label):
        m = re.search(re.escape(label) + r'\s*:?\s*([0-9.]+)', text)
        return m.group(1) if m else None

    def command(operation, path, reps):
        tool = 'qconvex' if operation == 'hull' else 'qdelaunay'
        return [tool, 'i', 's', 'TI', path, 'TO', HOME + '/qhull.out']

    def parse(out, err):
        text = err + out
        a = number(text, 'Number of vertices') or number(text, 'Number of input sites')
        b = number(text, 'Number of facets') or number(text, 'Number of Delaunay regions')
        return [{'time': float(number(text, 'CPU seconds to compute hull (after input)')),
                 'counts': [int(a), int(b)]}]
    return command, parse


LIBRARIES = {
    'convx': adapter('convx_run'),
    'spade': adapter('spade_run'),
    'delaunator': adapter('delaunator_run'),
    'delaunay': adapter('delaunay_run'),
    'parry3d': adapter('parry_run'),
    'geo': adapter('geo_run'),
    'cgal': cgal(),
    'qhull': qhull(),
}
# A library that builds once a process, whatever is asked.
ONE_BUILD = {'qhull'}


def run(library, operation, path, reps, timeline=False):
    """One process. Returns its builds and, when asked, its resident set over
    time as (seconds since it started, bytes)."""
    command, parse = LIBRARIES[library]
    start = time.perf_counter()
    process = subprocess.Popen(PIN + command(operation, path, reps), stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True)
    samples = []
    if timeline:
        statm = f'/proc/{process.pid}/statm'
        while process.poll() is None:
            try:
                with open(statm) as f:
                    samples.append((time.perf_counter() - start, int(f.read().split()[1]) * PAGE))
            except (OSError, IndexError, ValueError):
                break
            time.sleep(0.002)
    out, err = process.communicate()
    if process.returncode != 0:
        raise SystemExit(f'{library} {operation} {path}: exit {process.returncode}\n{err[-800:]}')
    builds = parse(out, err)
    assert builds, f'{library} {operation} {path}: no build in its output\n{out[-400:]}{err[-400:]}'
    return builds, samples


def peak_of(library, operation, path, reps):
    """The peak resident set, in bytes, of one process of `library`.

    GNU time starts the process and reads its `ru_maxrss`. A process forked
    from this runner would not do: the kernel's peak of a process counts what
    it held before `exec`, and a fork holds the runner's own pages, which grow
    with the results (a runner of 200 MB made a hull of 10^4 points read
    212 MB; through GNU time it reads 3 MB)."""
    command, _ = LIBRARIES[library]
    argv = ['/usr/bin/time', '-f', 'peak %M'] + PIN + command(operation, path, reps)
    done = subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if done.returncode != 0:
        raise SystemExit(f'{library} {operation} {path}: exit {done.returncode}\n{done.stderr[-800:]}')
    lines = [line for line in done.stderr.splitlines() if line.startswith('peak ')]
    assert lines, f'{library} {operation} {path}: no peak from GNU time\n{done.stderr[-400:]}'
    return int(lines[-1].split()[1]) * 1024


def thin(samples, most=400):
    """At most `most` samples, with the largest of each run of them kept."""
    if len(samples) <= most:
        return samples
    step = -(-len(samples) // most)
    return [max(samples[i:i + step], key=lambda s: s[1]) for i in range(0, len(samples), step)]


def main():
    rows_file, results = sys.argv[1], sys.argv[2]
    done = set()
    if os.path.exists(results):
        for line in open(results):
            done.add(json.loads(line)['row'])
    for line in open(rows_file):
        if not line.strip() or line.startswith('#'):
            continue
        operation, data, *libraries = line.split()
        row = f'{operation} {data}'
        if row in done:
            continue
        path = f'{DATA}/{data}.txt'
        first = run(libraries[0], operation, path, 1)[0][0]['time']
        rounds, reps = (5, 3) if first < 1.5 else (3, 1)
        record = {'row': row, 'rounds': rounds, 'reps': reps, 'libraries': {}}
        for library in libraries:
            record['libraries'][library] = {'builds': [], 'peak': [], 'timeline': []}
        for _ in range(rounds):
            for library in libraries:
                n = 1 if library in ONE_BUILD else reps
                for _ in range(reps if library in ONE_BUILD else 1):
                    builds, _ = run(library, operation, path, n)
                    record['libraries'][library]['builds'] += builds
        # Memory after the timing, so that sampling disturbs no time: the
        # peak of three processes of one build, and one timeline.
        for library in libraries:
            entry = record['libraries'][library]
            entry['peak'] = [peak_of(library, operation, path, 1) for _ in range(3)]
            _, samples = run(library, operation, path, 1, timeline=True)
            entry['timeline'] = thin(samples)
        with open(results, 'a') as f:
            f.write(json.dumps(record) + '\n')
        print(row, 'done', time.strftime('%H:%M:%S'), flush=True)
    print('ALL DONE', flush=True)


if __name__ == '__main__':
    main()
