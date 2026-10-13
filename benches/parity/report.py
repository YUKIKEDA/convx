"""The tables of a parity run.

usage: report.py <results.jsonl> [<results of an earlier run.jsonl>]

Prints the judged table in the form of docs/bench.md, the counts met at 1.05
and at 1.00, each set against the earlier run when one is given, and the sets
to run again: those whose slowest judged build is 1.5 times the fastest or
more (docs/verification.md).
"""
import json
import statistics as st
import sys

EXP = {"10000": "10^4", "100000": "10^5", "1000000": "10^6", "27000": "27,000"}


def fmt(x):
    if x >= 1:
        return f"{x:.2f} s"
    ms = x * 1e3
    if ms >= 100:
        return f"{ms:.0f} ms"
    if ms >= 10:
        return f"{ms:.1f} ms"
    return f"{ms:.2f} ms"


def load(path):
    rows = {}
    for line in open(path):
        r = json.loads(line)
        mode, fam, dim, n = r['set'].split()
        b = [x['build'] for x in r['convx']]
        bm = st.median(b)
        cg = st.median(x['time'] for x in r['cgal'])
        q = st.median(x['compute'] for x in r['qhull'])
        cp = st.median(x['points'] for x in r['cgal']) if 'points' in r['cgal'][0] else None
        # A reference whose output differs from convx's is shown as a
        # reference value and not judged (ADR 0006).
        qref = (mode == "hull" and fam == "sphere" and dim == "2") or (mode == "delaunay" and fam == "sphere")
        cref = fam == "grid"
        worst = max(([] if cref else [bm / cg]) + ([] if qref else [bm / q]))
        rows[r['set']] = dict(
            mode=mode, fam=fam, dim=dim, n=n, bm=bm, lo=min(b), hi=max(b), cg=cg, q=q, cp=cp,
            con=st.median(x['construct'] for x in r['convx']), rc=bm / cg, rq=bm / q, worst=worst, cref=cref, qref=qref,
            # Delaunay `cube` D3 is reported, not judged (ADR 0006, #513),
            # and so are the Delaunay lattices and the Kuzmin sites
            # (amendments of 2026-10-13).
            reported=(mode == 'delaunay' and (fam == 'cube' and dim == '3' or fam in ('lattice', 'kuzmin'))),
            counts=(r['convx'][0]['counts'], r['cgal'][0]['counts'], [r['qhull'][0]['vertices'], r['qhull'][0]['facets']]))
    return rows


def name(r):
    return f"{r['mode'].capitalize()} `{r['fam']}` D{r['dim']} {EXP.get(r['n'], r['n'])}"


def main():
    rows = load(sys.argv[1])
    earlier = load(sys.argv[2]) if len(sys.argv) > 2 else None

    print("| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |")
    print("| --- | ---: | ---: | ---: | ---: | --- | --- |")
    met105 = {"hull": 0, "delaunay": 0}
    met100 = dict(met105)
    judged = 0
    for r in rows.values():
        a, b = r['worst'] <= 1.05, r['worst'] <= 1.00
        if not r['reported']:
            judged += 1
            met105[r['mode']] += a
            met100[r['mode']] += b
        sc = f"{r['rc']:.2f}" + (" (ref.)" if r['cref'] else "")
        sp = f"{r['bm'] / r['cp']:.2f}" if r['cp'] else "–"
        sq = f"{r['rq']:.2f}" + (" (ref.)" if r['qref'] else "")
        ta, tb = ("reported", "reported") if r['reported'] else ('met' if a else 'not met', 'met' if b else 'not met')
        print(f"| {name(r)} | {fmt(r['bm'])} | {sc} | {sp} | {sq} | {ta} | {tb} |")
    print()
    print(f"Judged sets: {judged}. Met at 1.05: {sum(met105.values())} (hull {met105['hull']}, Delaunay {met105['delaunay']}). "
          f"Met at 1.00: {sum(met100.values())} (hull {met100['hull']}, Delaunay {met100['delaunay']}).")
    print()
    for r in rows.values():
        if r['reported']:
            print(f"{name(r)}: construction alone / CGAL {r['con'] / r['cg']:.2f}")
    print()

    if earlier:
        print("| Set | Earlier run | This run | convx `build()` | CGAL | Qhull |")
        print("| --- | ---: | ---: | ---: | ---: | ---: |")
        for s, r in rows.items():
            if s in earlier:
                p = earlier[s]
                print(f"| {name(r)} | {p['worst']:.2f} | {r['worst']:.2f} | {r['bm'] / p['bm']:.2f} | {r['cg'] / p['cg']:.2f} | {r['q'] / p['q']:.2f} |")
        print()

    print("Not met, or within 0.03 of 1.05:")
    for r in rows.values():
        if not r['reported'] and r['worst'] > 1.02:
            print(f"  {name(r)}: {r['worst']:.3f} (/ CGAL {r['rc']:.3f}, / Qhull {r['rq']:.3f})")
    print()
    print("Run again (slowest judged build 1.5 times the fastest or more):")
    for s, r in rows.items():
        if r['hi'] / r['lo'] >= 1.5:
            print(f"  {s}: {fmt(r['lo'])} to {fmt(r['hi'])} ({r['hi'] / r['lo']:.2f})")
    spread = [r['hi'] / r['lo'] for r in rows.values()]
    print(f"Spread of the judged builds: 1.2 or more on {sum(x >= 1.2 for x in spread)} of {len(spread)} sets, the largest {max(spread):.2f}.")
    print()
    print("Counts that differ between the tools (convx, CGAL, Qhull):")
    for s, r in rows.items():
        c, g, q = r['counts']
        if c != g:
            print(f"  {s}: convx {c}, CGAL {g}, Qhull {q}")


main()
