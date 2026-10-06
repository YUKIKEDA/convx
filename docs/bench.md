# Benchmarks

Measured wall times of convx against external references. How a reference is run, and what a timed section contains, is `docs/verification.md` (Performance sets). Rules for measuring: `.cursor/rules/bench.mdc`.

Each section names the convx commit it measured. A later speed change does not edit an old section; it adds a new one, or its own PR reports the new number next to the old.

## CGAL and Qhull, convx `74d345a` (#215)

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `74d345a`, rustc 1.97.0, `--release` with debug info, baseline target (runtime dispatch through `pulp`) |
| CGAL | 5.6 (Ubuntu `libcgal-dev`), g++ 13.3, `-std=c++17 -O3 -DNDEBUG -g -DCGAL_EIGEN3_ENABLED`, no `-march`; kernel `Epick`, `Epick_d<Dimension_tag<D>>` for D >= 4 |
| Qhull | 2020.2 (Ubuntu `qhull-bin`), `qconvex i s TI`, field "CPU seconds to compute hull (after input)" |
| Machine | Linux VM, 4 vCPU Intel Xeon @ 2.10 GHz (AVX-512) |
| Cores | One, every process pinned with `taskset -c 2` |
| Points | `benches/sets.txt`, seed 1, generator `xoshiro256starstar-v1`, written by `export_qhull_sets`; every tool reads the same file |
| Rounds | Tools alternated per round (convx, CGAL, Qhull). Under about 1 s: 5 rounds × 3 builds per process. Longer: 3 rounds × 1 build |
| Reported | Median, with min–max in parentheses. Ratios are medians divided; below 1 means convx is faster |

Timed sections:

- convx: `ConvexHullBuilder::new(dim, &pts).build()` or `DelaunayBuilder::new(dim, &pts).build()`. This includes acceptance, duplicate detection, classification of every point, and publication.
- CGAL hull: `convex_hull_2` (D = 2), `convex_hull_3` into a `Surface_mesh` (D = 3), `Triangulation::insert(begin, end)` (D >= 4; hull facets are the full cells incident to the infinite vertex). The D >= 4 class triangulates the interior too; CGAL has no output-sensitive dD hull.
- CGAL Delaunay: `Delaunay_triangulation_2` and `Delaunay_triangulation_3` built from the range, `Delaunay_triangulation::insert(begin, end)` for D >= 4.
- Reading the file and counting the output are outside every timed section.

The timing programs are on #215 and are not in the tree.

### Counts

convx and CGAL agree on every vertex, facet, and Delaunay simplex count below. Qhull agrees too, except hull `sphere` D = 2 at 10^5 (99,996 vertices against 100,000) and 10^6 (998,669 against 999,973): its default tolerance merges nearly collinear vertices. Those Qhull cells, marked \*, are not the same output.

### Hull

| Set | convx | CGAL | Qhull compute | convx / CGAL | convx / Qhull | Vertices / facets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 2.6 ms (1.7–3.1) | 1.0 ms (0.8–1.5) | 1.1 ms | 2.48 | 2.42 | 24 / 24 |
| `cube` D2 10^5 | 28.2 ms (19.0–29.7) | 10.7 ms (8.7–12.8) | 14.1 ms | 2.65 | 2.00 | 25 / 25 |
| `cube` D2 10^6 | 332 ms (237–457) | 109 ms (103–118) | 130 ms | 3.03 | 2.55 | 39 / 39 |
| `cube` D3 10^4 | 4.5 ms (2.9–7.4) | 5.0 ms (3.8–7.8) | 2.9 ms | 0.90 | 1.53 | 121 / 238 |
| `cube` D3 10^5 | 33.5 ms (26.4–39.5) | 48.9 ms (40.7–61.0) | 26.2 ms | 0.69 | 1.28 | 175 / 346 |
| `cube` D3 10^6 | 454 ms (298–549) | 1.10 s (0.74–1.43) | 270 ms | 0.41 | 1.68 | 285 / 566 |
| `cube` D4 10^4 | 22.8 ms (16.8–25.6) | 86.7 ms (75.5–104) | 14.7 ms | 0.26 | 1.55 | 404 / 2,320 |
| `cube` D4 10^5 | 97.2 ms (77.0–147) | 1.10 s (0.96–1.19) | 66.2 ms | 0.09 | 1.47 | 770 / 4,376 |
| `cube` D5 10^4 | 196 ms (153–217) | 427 ms (350–495) | 164 ms | 0.46 | 1.20 | 961 / 20,232 |
| `cube` D5 10^5 | 594 ms (549–601) | 3.12 s (3.06–3.18) | 530 ms | 0.19 | 1.12 | 2,339 / 48,818 |
| `cube` D6 10^4 | 2.14 s (1.87–2.30) | 4.74 s (4.40–5.23) | 2.75 s | 0.45 | 0.78 | 1,882 / 174,102 |
| `cube` D6 10^5 | 8.44 s (7.21–8.75) | 22.8 s (21.6–24.0) | 11.4 s | 0.37 | 0.74 | 5,444 / 518,754 |
| `sphere` D2 10^4 | 9.1 ms (8.3–10.4) | 1.5 ms (1.4–1.8) | 18.5 ms | 6.01 | 0.49 | 10,000 / 10,000 |
| `sphere` D2 10^5 | 104 ms (76–133) | 12.6 ms (11.9–17.8) | 204 ms\* | 8.32 | 0.51\* | 100,000 / 100,000 |
| `sphere` D2 10^6 | 1.44 s (1.22–1.65) | 174 ms (143–217) | 4.15 s\* | 8.29 | 0.35\* | 999,973 / 999,973 |
| `sphere` D3 10^4 | 128 ms (88–135) | 42.8 ms (34.3–57.8) | 42.0 ms | 3.00 | 3.05 | 10,000 / 19,996 |
| `sphere` D3 10^5 | 1.57 s (1.19–2.01) | 805 ms (637–964) | 675 ms | 1.95 | 2.32 | 100,000 / 199,996 |
| `sphere` D3 10^6 | 20.6 s (18.9–22.1) | 14.3 s (13.6–15.8) | 7.76 s | 1.44 | 2.66 | 1,000,000 / 1,999,996 |
| `sphere` D4 10^4 | 452 ms (376–548) | 224 ms (193–256) | 161 ms | 2.01 | 2.81 | 10,000 / 67,192 |
| `sphere` D4 10^5 | 6.17 s (5.80–6.27) | 2.30 s (2.05–2.30) | 2.74 s | 2.68 | 2.25 | 100,000 / 675,154 |
| `sphere` D5 10^4 | 3.03 s (2.98–3.26) | 1.84 s (1.79–1.87) | 1.70 s | 1.65 | 1.78 | 10,000 / 299,994 |
| `sphere` D5 10^5 | 41.5 s (40.8–42.3) | 19.8 s (19.5–20.1) | 21.5 s | 2.10 | 1.93 | 100,000 / 3,112,922 |
| `sphere` D6 10^4 | 23.3 s (23.0–25.7) | 19.3 s (18.9–19.4) | 15.4 s | 1.21 | 1.51 | 10,000 / 1,570,453 |

Not timed: `sphere` D6 10^5 (one convx run estimated at about 5 minutes from the D5 growth), and D = 7 and D = 8.

### Delaunay

| Set | convx | CGAL | convx / CGAL | Simplices |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 40.2 ms (36.0–47.8) | 7.2 ms (6.5–8.3) | 5.60 | 19,974 |
| `cube` D2 10^5 | 654 ms (516–771) | 84.2 ms (67.7–94.6) | 7.76 | 199,973 |
| `cube` D2 10^6 | 7.82 s (7.57–8.17) | 942 ms (777–1,000) | 8.30 | 1,999,959 |
| `sphere` D2 10^4 | 63.4 ms (56.0–83.2) | 6.7 ms (5.9–9.3) | 9.52 | 9,998 |
| `sphere` D2 10^5 | 875 ms (677–986) | 73.7 ms (65.4–84.9) | 11.88 | 99,998 |
| `sphere` D2 10^6 | 8.77 s (8.45–8.85) | 676 ms (640–677) | 12.97 | 1,000,025 |
| `cube` D3 10^4 | 435 ms (343–456) | 62.8 ms (46.1–68.0) | 6.93 | 66,373 |
| `cube` D3 10^5 | 3.99 s (3.97–4.04) | 601 ms (529–662) | 6.64 | 671,608 |
| `cube` D3 10^6 | 46.8 s (45.9–47.8) | 6.06 s (5.65–6.27) | 7.71 | 6,747,791 |
| `sphere` D3 10^4 | 653 ms (467–790) | 266 ms (215–309) | 2.46 | 30,038 |
| `sphere` D3 10^5 | 6.40 s (5.89–6.42) | 1.83 s (1.44–1.94) | 3.50 | 302,013 |
| `sphere` D3 10^6 | 63.5 s (62.8–65.7) | 12.3 s (11.8–12.8) | 5.16 | 3,017,144 |
| `cube` D4 10^4 | 2.54 s (2.49–2.68) | 1.18 s (1.11–1.32) | 2.16 | 295,350 |
| `sphere` D4 10^4 | 9.11 s (8.92–9.92) | 4.64 s (4.22–5.00) | 1.96 | 128,710 |
| `cube` D5 10^4 | 20.5 s (20.1–20.8) | 13.5 s (13.5–13.9) | 1.52 | 1,551,630 |
| `sphere` D5 10^4 | 99.8 s (96.4–101.1) | 65.0 s (63.6–65.2) | 1.54 | 674,290 |

### Reading

- Hull, few vertices (`cube`), D >= 3: convx is ahead of CGAL (0.09 to 0.90), and the gap widens with n and D. At D >= 4 this partly reflects CGAL's triangulation of the interior. Against Qhull these rows stay 1.1 to 1.7, except D = 6 (0.74 to 0.78).
- Hull D = 2: convx is behind CGAL's dedicated planar algorithm by 2.5 to 3 (`cube`) and 6 to 8 (`sphere`). The dedicated D = 2 hull is #176.
- Hull, all points extreme (`sphere`), D >= 3: convx is behind CGAL by 1.2 to 3.0, the per-simplex construction cost of #199 and #209.
- Delaunay: convx is behind CGAL everywhere, by 5.6 to 13 at D = 2, 2.5 to 7.7 at D = 3, and 1.5 to 2.2 at D = 4 and 5. The gap is largest where CGAL has dedicated 2D and 3D classes.
