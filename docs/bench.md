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

## Four timed sections, convx `430af36` (#250)

### Method

| Item | Value |
| --- | --- |
| convx | `029eb3f`, the head of #259 when this was measured (P6-1 on top of `main` at `f9e24ea`); #259 is on `main` as `430af36`. rustc 1.97.0, `--release` with debug info, baseline target |
| CGAL | 5.6 (Ubuntu `libcgal-dev`), g++ 13.3, the program and flags of #215 unchanged |
| Qhull | 2020.2 (Ubuntu `qhull-bin`), `qconvex i s TI <file> TO <out>` and `qdelaunay i s TI <file> TO <out>` |
| Machine | Linux VM, 4 vCPU Intel Xeon @ 2.10 GHz (AVX-512), the machine of the section above |
| Cores | One, every process pinned with `taskset -c 2` |
| Points | `benches/sets.txt`, seed 1, generator `xoshiro256starstar-v1`, written by `export_qhull_sets`; every tool reads the same file |
| Rounds | One unrecorded convx build per set first. Tools alternated per round (convx, CGAL, Qhull). When that build took under 1.5 s: 5 rounds × 3 builds per process (Qhull: 3 runs per round). Longer: 3 rounds × 1 |
| Reported | Median, with min–max in parentheses. Ratios are medians divided; below 1 means convx is faster |

#259 gained one commit after this measurement, `83f53c6`: each formula of the semi-static stage returns its determinant and bound, and a test reads them. The tables below were not timed again. On another machine, `build()` at `83f53c6` was timed against `b613adb`, whose source is that of `029eb3f`, on six sets at 10^5 (Delaunay `cube` and `sphere` D2 and D3, hull `sphere` D3, hull `cube` D4). Every ratio was 0.99 to 1.02, inside the spread; the table is on #259.

The four columns are those of `docs/verification.md` (Performance sets): convx `build()`; convx construction alone (`SimplicialHull::build`, or for Delaunay the insertion order and `insert::Mesh::build`), from a timer in a scratch copy that is not committed; Qhull's "CPU seconds to compute hull (after input)"; and the wall time of the whole Qhull process, which reads the file and writes the facet list. CGAL is its construction call only, as in #215. Qhull's work counters are from the same `s` summary: hyperplanes created / distance tests.

### Counts

convx and CGAL agree on every vertex, facet, and Delaunay simplex count below. Qhull agrees too, except the cells marked \*: hull `sphere` D2 at 10^5 and 10^6, where its default tolerance merges nearly collinear vertices (as in the section above), and Delaunay `sphere` at every size and dimension, where it merges nearly cospherical regions (D5 10^4: 673,807 regions against 674,290 simplices). Those Qhull cells are not the same output.

### Hull

| Set | convx `build()` | convx construction | CGAL | Qhull compute | Qhull whole run | `build()` / CGAL | `build()` / Qhull compute | Construction / Qhull compute | `build()` / Qhull whole run | Qhull hyperplanes / distance tests | Vertices / facets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 1.2 ms (1.1–1.5 ms) | 0.78 ms (0.74–1.2 ms) | 0.91 ms (0.87–1.1 ms) | 1.3 ms | 9.7 ms | 1.31 | 0.94 | 0.61 | 0.12 | 45 / 65,050 | 24 / 24 |
| `cube` D2 10^5 | 12.3 ms (10.7–15.3 ms) | 7.5 ms (7.1–9.2 ms) | 8.8 ms (8.5–10.4 ms) | 13.1 ms | 57.2 ms | 1.39 | 0.94 | 0.57 | 0.22 | 48 / 699,388 | 25 / 25 |
| `cube` D2 10^6 | 185 ms (166–222 ms) | 77.2 ms (73.7–103 ms) | 93.0 ms (90.4–96.1 ms) | 143 ms | 539 ms | 1.99 | 1.29 | 0.54 | 0.34 | 76 / 6,417,683 | 39 / 39 |
| `cube` D3 10^4 | 3.6 ms (3.3–4.6 ms) | 2.9 ms (2.7–3.9 ms) | 4.1 ms (3.9–6.9 ms) | 3.0 ms | 13.2 ms | 0.89 | 1.20 | 0.97 | 0.28 | 755 / 138,265 | 121 / 238 |
| `cube` D3 10^5 | 27.6 ms (24.2–40.6 ms) | 22.1 ms (20.2–32.9 ms) | 48.8 ms (45.8–66.2 ms) | 26.4 ms | 88.9 ms | 0.57 | 1.05 | 0.84 | 0.31 | 952 / 1,279,410 | 175 / 346 |
| `cube` D3 10^6 | 446 ms (394–482 ms) | 341 ms (303–375 ms) | 1.63 s (1.53–1.76 s) | 416 ms | 1.00 s | 0.27 | 1.07 | 0.82 | 0.45 | 1,888 / 13,617,348 | 285 / 566 |
| `cube` D4 10^4 | 20.1 ms (19.4–21.7 ms) | 16.7 ms (16.0–18.0 ms) | 91.7 ms (89.3–117 ms) | 13.1 ms | 26.3 ms | 0.22 | 1.53 | 1.28 | 0.76 | 11,158 / 402,164 | 404 / 2,320 |
| `cube` D4 10^5 | 105 ms (102–114 ms) | 93.3 ms (91.6–102 ms) | 1.33 s (1.26–1.40 s) | 84.8 ms | 166 ms | 0.08 | 1.24 | 1.10 | 0.63 | 20,086 / 3,590,089 | 770 / 4,376 |
| `cube` D5 10^4 | 218 ms (190–281 ms) | 167 ms (147–238 ms) | 507 ms (480–633 ms) | 205 ms | 240 ms | 0.43 | 1.06 | 0.81 | 0.91 | 125,948 / 2,010,291 | 961 / 20,232 |
| `cube` D5 10^5 | 880 ms (769–958 ms) | 721 ms (633–792 ms) | 4.02 s (3.87–4.27 s) | 954 ms | 1.13 s | 0.22 | 0.92 | 0.76 | 0.78 | 344,329 / 20,893,652 | 2,339 / 48,818 |
| `cube` D6 10^4 | 2.98 s (2.93–3.05 s) | 2.28 s (2.18–2.31 s) | 4.94 s (4.83–5.01 s) | 3.48 s | 3.85 s | 0.60 | 0.86 | 0.66 | 0.77 | 1,234,219 / 13,946,956 | 1,882 / 174,102 |
| `cube` D6 10^5 | 11.3 s (11.3–11.4 s) | 8.98 s (8.89–9.08 s) | 26.0 s (25.2–26.3 s) | 16.7 s | 17.9 s | 0.44 | 0.68 | 0.54 | 0.63 | 4,718,136 / 149,602,813 | 5,444 / 518,754 |
| `sphere` D2 10^4 | 8.2 ms (7.1–11.1 ms) | 1.4 ms (1.3–2.4 ms) | 1.2 ms (1.1–3.2 ms) | 15.8 ms | 28.0 ms | 6.71 | 0.52 | 0.09 | 0.29 | 19,998 / 189,622 | 10,000 / 10,000 |
| `sphere` D2 10^5 | 137 ms (121–158 ms) | 17.5 ms (16.5–22.8 ms) | 13.9 ms (12.8–15.7 ms) | 394 ms\* | 542 ms\* | 9.90 | 0.35 | 0.04 | 0.25 | 199,989 / 2,395,166 | 100,000 / 100,000 |
| `sphere` D2 10^6 | 2.26 s (2.23–2.34 s) | 584 ms (529–626 ms) | 168 ms (166–170 ms) | 7.21 s\* | 9.02 s\* | 13.49 | 0.31 | 0.08 | 0.25 | 1,997,357 / 28,928,990 | 999,973 / 999,973 |
| `sphere` D3 10^4 | 117 ms (101–139 ms) | 78.0 ms (68.9–102 ms) | 49.6 ms (44.9–64.5 ms) | 43.0 ms | 69.5 ms | 2.35 | 2.71 | 1.81 | 1.68 | 56,158 / 341,173 | 10,000 / 19,996 |
| `sphere` D3 10^5 | 1.83 s (1.78–1.92 s) | 1.21 s (1.20–1.23 s) | 1.43 s (1.24–1.45 s) | 852 ms | 1.22 s | 1.27 | 2.15 | 1.42 | 1.50 | 565,222 / 4,349,516 | 100,000 / 199,996 |
| `sphere` D3 10^6 | 25.0 s (24.8–25.4 s) | 16.8 s (16.8–17.3 s) | 18.4 s (18.1–19.0 s) | 10.0 s | 13.7 s | 1.36 | 2.50 | 1.68 | 1.83 | 5,654,825 / 52,940,589 | 1,000,000 / 1,999,996 |
| `sphere` D4 10^4 | 525 ms (480–577 ms) | 363 ms (323–419 ms) | 243 ms (231–266 ms) | 249 ms | 344 ms | 2.16 | 2.11 | 1.46 | 1.53 | 258,048 / 889,697 | 10,000 / 67,192 |
| `sphere` D4 10^5 | 7.70 s (7.40–7.72 s) | 5.04 s (4.96–5.17 s) | 2.72 s (2.70–2.76 s) | 3.73 s | 5.12 s | 2.83 | 2.06 | 1.35 | 1.50 | 2,626,423 / 10,538,224 | 100,000 / 675,154 |
| `sphere` D5 10^4 | 4.33 s (3.78–4.50 s) | 3.07 s (2.64–3.23 s) | 2.02 s (1.99–2.19 s) | 2.34 s | 3.03 s | 2.14 | 1.85 | 1.31 | 1.43 | 1,413,713 / 3,017,965 | 10,000 / 299,994 |
| `sphere` D5 10^5 | 55.0 s (54.0–55.8 s) | 38.1 s (37.7–39.2 s) | 23.4 s (22.8–23.7 s) | 30.9 s | 38.5 s | 2.35 | 1.78 | 1.23 | 1.43 | 15,164,634 / 35,472,228 | 100,000 / 3,112,922 |
| `sphere` D6 10^4 | 32.7 s (32.6–34.3 s) | 23.9 s (23.7–25.0 s) | 20.4 s (19.9–21.3 s) | 22.7 s | 26.8 s | 1.60 | 1.44 | 1.05 | 1.22 | 8,596,125 / 14,508,402 | 10,000 / 1,570,453 |

Not timed: `sphere` D6 10^5, as in the section above.

### Delaunay

| Set | convx `build()` | convx construction | CGAL | Qhull compute | Qhull whole run | `build()` / CGAL | `build()` / Qhull compute | Construction / Qhull compute | `build()` / Qhull whole run | Qhull hyperplanes / distance tests | Simplices |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 49.2 ms (44.8–74.5 ms) | 28.0 ms (26.8–40.2 ms) | 8.0 ms (7.2–10.3 ms) | 46.7 ms | 79.8 ms | 6.12 | 1.05 | 0.60 | 0.62 | 56,426 / 377,169 | 19,974 |
| `cube` D2 10^5 | 862 ms (806–1.00 s) | 324 ms (307–456 ms) | 80.4 ms (77.1–94.2 ms) | 729 ms | 1.14 s | 10.72 | 1.18 | 0.44 | 0.76 | 565,024 / 4,837,796 | 199,973 |
| `cube` D2 10^6 | 13.0 s (12.7–13.2 s) | 3.65 s (3.63–3.70 s) | 1.01 s (958–1.14 s) | 10.5 s | 15.3 s | 12.89 | 1.23 | 0.35 | 0.85 | 5,661,165 / 56,857,785 | 1,999,959 |
| `cube` D3 10^4 | 324 ms (301–381 ms) | 158 ms (149–182 ms) | 62.0 ms (56.8–89.5 ms) | 271 ms | 420 ms | 5.23 | 1.20 | 0.58 | 0.77 | 255,564 / 881,892 | 66,373 |
| `cube` D3 10^5 | 4.60 s (4.33–4.84 s) | 1.81 s (1.78–2.02 s) | 728 ms (670–752 ms) | 3.78 s | 5.68 s | 6.32 | 1.22 | 0.48 | 0.81 | 2,602,230 / 11,306,061 | 671,608 |
| `cube` D3 10^6 | 68.2 s (63.4–68.2 s) | 19.8 s (19.7–19.9 s) | 7.69 s (7.21–7.79 s) | 47.4 s | 67.0 s | 8.87 | 1.44 | 0.42 | 1.02 | 26,219,307 / 125,621,001 | 6,747,791 |
| `sphere` D2 10^4 | 46.3 ms (40.5–81.1 ms) | 34.9 ms (30.7–62.8 ms) | 8.0 ms (6.9–14.6 ms) | 61.6 ms\* | 85.6 ms\* | 5.79 | 0.75 | 0.57 | 0.54 | 45,385 / 437,697 | 9,998 |
| `sphere` D2 10^5 | 534 ms (492–653 ms) | 290 ms (277–423 ms) | 68.7 ms (61.6–117 ms) | 1.20 s\* | 1.60 s\* | 7.77 | 0.44 | 0.24 | 0.33 | 450,890 / 5,578,213 | 99,998 |
| `sphere` D2 10^6 | 6.75 s (6.74–7.22 s) | 2.96 s (2.90–3.00 s) | 755 ms (712–845 ms) | 47.9 s\* | 52.6 s\* | 8.94 | 0.14 | 0.06 | 0.13 | 6,096,936 / 140,732,829 | 1,000,025 |
| `sphere` D3 10^4 | 655 ms (590–938 ms) | 487 ms (451–673 ms) | 257 ms (238–376 ms) | 364 ms\* | 468 ms\* | 2.55 | 1.80 | 1.34 | 1.40 | 191,857 / 1,121,621 | 30,038 |
| `sphere` D3 10^5 | 6.42 s (6.41–6.65 s) | 4.53 s (4.50–4.78 s) | 1.97 s (1.96–2.03 s) | 6.20 s\* | 7.47 s\* | 3.26 | 1.04 | 0.73 | 0.86 | 1,974,009 / 15,785,569 | 302,013 |
| `sphere` D3 10^6 | 59.1 s (58.6–59.9 s) | 37.9 s (37.5–38.3 s) | 13.2 s (13.0–14.2 s) | 55.9 s\* | 69.1 s\* | 4.49 | 1.06 | 0.68 | 0.86 | 19,314,633 / 137,640,502 | 3,017,144 |
| `cube` D4 10^4 | 3.74 s (3.40–3.95 s) | 2.25 s (1.99–2.31 s) | 1.44 s (1.41–1.49 s) | 2.22 s | 2.95 s | 2.59 | 1.68 | 1.01 | 1.27 | 1,397,526 / 3,183,815 | 295,350 |
| `sphere` D4 10^4 | 11.2 s (10.8–11.8 s) | 8.67 s (8.30–9.11 s) | 5.27 s (5.09–5.52 s) | 3.77 s\* | 4.35 s\* | 2.12 | 2.96 | 2.30 | 2.57 | 1,078,477 / 4,371,357 | 128,710 |
| `cube` D5 10^4 | 30.4 s (29.9–30.6 s) | 18.6 s (18.5–18.9 s) | 15.8 s (15.1–15.8 s) | 22.8 s | 27.7 s | 1.93 | 1.33 | 0.82 | 1.10 | 8,593,628 / 15,161,004 | 1,551,630 |
| `sphere` D5 10^4 | 131.1 s (129.9–132.5 s) | 102.6 s (100.8–103.5 s) | 69.8 s (69.6–69.9 s) | 45.1 s\* | 49.0 s\* | 1.88 | 2.91 | 2.28 | 2.67 | 6,939,777 / 19,199,869 | 674,290 |

### Reading

- Hull `cube`: construction alone is 0.54 to 1.28 times Qhull's compute time, below 1 on every set but D4. Against Qhull's whole run, `build()` is 0.12 to 0.91. The gap of the section above on these sets was mostly acceptance and the pass after construction, not construction.
- Hull `sphere` D3 to D6: construction alone is 1.05 to 1.81 times Qhull's compute time, and `build()` 1.44 to 2.71. Construction is about two thirds of `build()`; Qhull creates about as many hyperplanes as convx creates simplices (#199), so the construction gap is cost per operation. That is P6-5. The rest, 27 to 35% of `build()`, is the pass after construction (P6-6).
- Hull `sphere` D2: construction takes 1.4 ms to 0.58 s of a 8.2 ms to 2.26 s `build()`. The pass after construction is most of the time, and `build()` is 7 to 13 times CGAL's `convex_hull_2`, which returns only the hull points. That is P6-6.
- Delaunay: construction alone is 0.35 to 0.60 times Qhull's compute time on `cube` D2 and D3, and 0.82 to 1.01 on `cube` D4 and D5 (the `sphere` cells are not the same output). `build()` stays 5.8 to 12.9 times CGAL at D = 2, 2.5 to 8.9 at D = 3, and 1.9 to 2.6 at D = 4 and 5. The pass after insertion (groups and publication) is 22 to 72% of `build()`, highest on `cube` at 10^6 (P6-7).
- This machine ran convx and CGAL somewhat slower than in the section above on the largest sets (hull `sphere` D3 10^6: CGAL 18.4 s here, 14.3 s there); read ratios within one section, not across them.

## Parallel round protocol against the sequential hull, convx `38cd816` (#256)

The measurement behind ADR 0004. The parallel build timed here is the round protocol that ADR removes: the first K = 64 candidates, the T and H reservation, plans on rayon's pool, and a commit in ascending input index.

### Method

| Item | Value |
| --- | --- |
| convx | `dca2e56`, the head of #265 when this was measured; #265 is on `main` as `38cd816`, which adds one debug assertion and no release code. rustc 1.97.0, `--release` |
| Machine | Linux VM, 4 cores, one thread per core |
| Cores | One thread: core 2. Two threads: cores 2–3. Four threads: cores 0–3. `RAYON_NUM_THREADS` matches |
| Points | `benches/sets.txt`, seed 1 |
| Timed | `ConvexHullBuilder::new(dim, &pts).build()`, with `parallel(true)` for the parallel columns |
| Reported | Median with min–max, in seconds |

### Keep criterion

Every hull set whose output has at least 10^4 facets. The `cube` sets below D = 5 have 24 to 4,376 facets and are left out. Five rounds for sets under 2 s, otherwise three. The four configurations alternate in each round, reversed on odd rounds. The last column says whether four threads differ from the sequential build beyond the spread.

| Set | Facets | Sequential | Parallel, 1 thread | Parallel, 2 threads | Parallel, 4 threads | 4 threads / sequential | Beyond the spread |
| --- | ---: | --- | --- | --- | --- | ---: | --- |
| `sphere` D2 10^4 | 10,000 | 0.0046 (0.0044–0.0053) | 0.0047 (0.0046–0.0051) | 0.0054 (0.0052–0.0069) | 0.0047 (0.0046–0.0049) | 1.02 | inside |
| `sphere` D2 10^5 | 100,000 | 0.0640 (0.0632–0.0686) | 0.0675 (0.0630–0.0819) | 0.0653 (0.0618–0.0827) | 0.0665 (0.0635–0.0693) | 1.04 | inside |
| `sphere` D2 10^6 | 999,973 | 1.126 (0.9846–1.209) | 1.064 (0.9503–1.282) | 1.097 (0.9746–1.203) | 1.151 (1.040–1.249) | 1.02 | inside |
| `sphere` D3 10^4 | 19,996 | 0.0604 (0.0557–0.0670) | 0.0722 (0.0674–0.0988) | 0.0732 (0.0702–0.0785) | 0.0889 (0.0807–0.0980) | 1.47 | slower |
| `sphere` D3 10^5 | 199,996 | 0.9513 (0.8442–1.022) | 1.097 (1.046–1.225) | 0.9992 (0.9770–1.113) | 1.147 (1.109–1.223) | 1.21 | slower |
| `sphere` D3 10^6 | 1,999,996 | 16.322 (16.056–17.539) | 17.664 (17.286–18.369) | 17.107 (16.216–17.561) | 17.951 (17.819–18.036) | 1.10 | slower |
| `sphere` D4 10^4 | 67,192 | 0.2971 (0.2663–0.3189) | 0.3390 (0.3151–0.3910) | 0.3047 (0.2814–0.3769) | 0.3623 (0.3208–0.4204) | 1.22 | slower |
| `sphere` D4 10^5 | 675,154 | 4.056 (3.964–4.272) | 4.816 (4.700–4.932) | 4.198 (4.192–4.303) | 4.459 (4.372–4.467) | 1.10 | slower |
| `sphere` D5 10^4 | 299,994 | 1.814 (1.734–1.933) | 2.315 (2.214–2.343) | 1.917 (1.813–2.222) | 1.827 (1.767–1.839) | 1.01 | inside |
| `sphere` D5 10^5 | 3,112,922 | 27.320 (25.511–27.509) | 32.175 (31.158–33.666) | 27.002 (26.935–27.568) | 25.165 (25.104–26.727) | 0.92 | inside |
| `sphere` D6 10^4 | 1,570,453 | 15.888 (15.717–16.069) | 18.882 (18.860–19.872) | 16.319 (15.418–17.621) | 13.816 (13.442–14.081) | 0.87 | faster |
| `cube` D5 10^4 | 20,232 | 0.1329 (0.1100–0.1446) | 0.1421 (0.1277–0.1712) | 0.1405 (0.1267–0.1623) | 0.1712 (0.1264–0.2125) | 1.29 | inside |
| `cube` D5 10^5 | 48,818 | 0.4464 (0.3684–0.5213) | 0.5089 (0.4829–0.5600) | 0.4632 (0.4079–0.4865) | 0.4534 (0.4306–0.5004) | 1.02 | inside |
| `cube` D6 10^4 | 174,102 | 1.187 (1.161–1.279) | 1.528 (1.422–1.679) | 1.375 (1.256–1.481) | 1.382 (1.214–1.432) | 1.16 | inside |
| `cube` D6 10^5 | 518,754 | 5.568 (5.534–5.721) | 6.578 (6.366–6.826) | 5.390 (5.376–6.189) | 5.048 (4.945–5.182) | 0.91 | faster |

### Phases

Phase timers in a scratch copy that is not committed; medians of 5 runs, in seconds. `select` is the batch extraction with the T and H reservation, `plan` is the round's plans on rayon's pool, `commit` is the ordered commit and the candidate push. `absorb` is the sequential build's whole in-place insertion.

| Set | Build | Total | Accept and initial simplex | `absorb` | `select` | `plan` | `commit` | Classify | Publish |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `sphere` D3 10^5 | sequential | 0.910 (0.873–1.005) | 0.008 | 0.600 | | | | 0.129 | 0.150 |
| `sphere` D3 10^5 | parallel, 1 thread | 1.082 (1.076–1.203) | 0.009 | | 0.242 | 0.425 | 0.112 | 0.136 | 0.162 |
| `sphere` D3 10^5 | parallel, 2 threads | 1.004 (0.956–1.099) | 0.009 | | 0.235 | 0.373 | 0.122 | 0.119 | 0.142 |
| `sphere` D3 10^5 | parallel, 4 threads | 1.138 (1.118–1.185) | 0.009 | | 0.267 | 0.433 | 0.146 | 0.129 | 0.148 |
| `cube` D6 10^4 | sequential | 1.256 (1.187–1.296) | 0.001 | 0.855 | | | | 0.181 | 0.206 |
| `cube` D6 10^4 | parallel, 1 thread | 1.584 (1.506–1.766) | 0.001 | | 0.283 | 0.768 | 0.138 | 0.167 | 0.195 |
| `cube` D6 10^4 | parallel, 2 threads | 1.284 (1.223–1.495) | 0.001 | | 0.255 | 0.498 | 0.137 | 0.164 | 0.206 |
| `cube` D6 10^4 | parallel, 4 threads | 1.233 (1.151–1.439) | 0.001 | | 0.260 | 0.424 | 0.153 | 0.167 | 0.203 |

### Reading

- Four threads are faster than the sequential build beyond the spread on 2 of the 15 sets (`sphere` D6 10^4 at 0.87, `cube` D6 10^5 at 0.91), slower on 5 (`sphere` D3 and D4, up to 1.47), and inside the spread on 8.
- `select` and `commit` together are 59% (`sphere` D3 10^5) and 49% (`cube` D6 10^4) of the sequential build's `absorb`. Both are serial in the protocol.
- Everything outside `plan` is serial: 61% and 52% of the one-thread parallel build. With `plan` scaling perfectly, four threads reach 0.76 s and 1.01 s, which is 0.84 and 0.80 of the sequential build.
- `plan` itself barely scales. On `sphere` D3 10^5, 1594 rounds of mostly 33 to 64 points take 0.41 s of `plan` at one thread and 0.36 s at four. A round's plans total about 0.25 ms, and the pool sleeps during each round's `select` and `commit`. On this machine a flat loop of independent work scales 3.9 times at four threads.
- D = 2 has no parallel construction: the polygon build is the same code either way.

## After P6 (#258 to #274), convx `40b6b7b` (#275)

Every hull and Delaunay timing set, in the four timed sections of #250, with each build split into construction, the pass after it, and publication.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `40b6b7b`, which holds P6-1 to P6-11 (#258 to #274). rustc 1.99.0, `--release` with debug info, baseline target |
| CGAL | 5.4 (Ubuntu 22.04 `libcgal-dev`), g++ 11.4, the program and flags of #215. The sections above used CGAL 5.6 and g++ 13.3 |
| Qhull | 2020.2 (Ubuntu 22.04 `qhull-bin`), `qconvex i s TI <file> TO <out>` and `qdelaunay i s TI <file> TO <out>` |
| Machine | Intel Core i5-13400F, Ubuntu 22.04 under WSL2 (kernel 5.15.133.1) on Windows 11. This is not the Linux VM of the sections above; absolute times are not comparable with them, and only ratios within this section are read |
| Cores | One, every process pinned with `taskset -c 2` |
| Points | `benches/sets.txt`, seed 1, generator `xoshiro256starstar-v1`, written by `export_qhull_sets`; every tool reads the same file |
| Rounds | One unrecorded convx build per set first. Tools alternated per round (convx, CGAL, Qhull). When that build took under 1.5 s: 5 rounds × 3 builds per process (Qhull: 3 runs per round). Longer: 3 rounds × 1 |
| Phase timers | In a scratch copy that is not committed: time marks printed to standard error around construction and around publication. Hull: construction is `SimplicialHull::build`, the pass after it is merge and classification, then publication. Delaunay: construction is the insertion order and `insert::Mesh::build`, the pass after it is the groups, the draft, and the numbering, then publication. `build()` is timed in the same binary |
| Reported | Median, with min–max in parentheses; Qhull's two columns are medians. Ratios are medians divided; below 1 means convx is faster. The three percentages are shares of `build()`; the rest is acceptance and duplicate detection |

The four columns are those of `docs/verification.md` (Performance sets). The whole run took 31 minutes. The CGAL program is on #215; the convx timing binary, the patch that adds the marks, and the runner are on #275.

### Counts

convx and CGAL agree on every vertex, facet, and Delaunay simplex count below. Qhull agrees too, except where its tolerance merges nearly degenerate pieces, as in the sections above: hull `sphere` D2 at 10^5 and 10^6 (99,996 and 998,669 vertices), and Delaunay `sphere` at every size and dimension (for example D3 10^6: 2,911,192 regions against 3,017,144 simplices). Those Qhull cells, marked \*, are not the same output, and neither are the ratios to Qhull on those rows.

### Hull

| Set | convx `build()` | convx construction | CGAL | Qhull compute | Qhull whole run | `build()` / CGAL | `build()` / Qhull compute | Construction / Qhull compute | `build()` / Qhull whole run | Construction / pass after / publication, % of `build()` | Qhull hyperplanes / distance tests | Vertices / facets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 0.64 ms (0.60 ms–0.83 ms) | 0.43 ms (0.41 ms–0.59 ms) | 0.52 ms (0.48 ms–0.84 ms) | 0.63 ms | 3.72 ms | 1.22 | 1.01 | 0.69 | 0.17 | 68 / 13 / 3 | 45 / 65,050 | 24 / 24 |
| `cube` D2 10^5 | 5.28 ms (4.99 ms–6.55 ms) | 3.65 ms (3.42 ms–4.13 ms) | 4.46 ms (4.23 ms–5.42 ms) | 5.54 ms | 23.4 ms | 1.18 | 0.95 | 0.66 | 0.23 | 69 / 9 / 1 | 48 / 699,388 | 25 / 25 |
| `cube` D2 10^6 | 66.4 ms (62.0 ms–73.8 ms) | 37.2 ms (35.0 ms–38.6 ms) | 44.6 ms (43.4 ms–60.2 ms) | 55.6 ms | 222 ms | 1.49 | 1.19 | 0.67 | 0.30 | 56 / 8 / 0 | 76 / 6,417,683 | 39 / 39 |
| `cube` D3 10^4 | 1.54 ms (1.43 ms–1.80 ms) | 1.21 ms (1.12 ms–1.31 ms) | 2.35 ms (2.29 ms–3.23 ms) | 1.64 ms | 5.69 ms | 0.66 | 0.94 | 0.74 | 0.27 | 79 / 7 / 7 | 755 / 138,265 | 121 / 238 |
| `cube` D3 10^5 | 13.9 ms (11.1 ms–19.1 ms) | 10.7 ms (9.01 ms–15.2 ms) | 38.4 ms (27.6 ms–74.4 ms) | 13.9 ms | 46.2 ms | 0.36 | 1.00 | 0.77 | 0.30 | 77 / 4 / 2 | 952 / 1,279,410 | 175 / 346 |
| `cube` D3 10^6 | 159 ms (138 ms–263 ms) | 124 ms (109 ms–186 ms) | 565 ms (509 ms–848 ms) | 145 ms | 402 ms | 0.28 | 1.09 | 0.86 | 0.40 | 78 / 2 / 0 | 1,888 / 13,617,348 | 285 / 566 |
| `cube` D4 10^4 | 7.71 ms (7.19 ms–8.76 ms) | 6.08 ms (5.56 ms–6.88 ms) | 47.4 ms (41.9 ms–55.4 ms) | 7.24 ms | 13.0 ms | 0.16 | 1.06 | 0.84 | 0.60 | 79 / 8 / 12 | 11,158 / 402,164 | 404 / 2,320 |
| `cube` D4 10^5 | 38.9 ms (36.8 ms–52.3 ms) | 34.1 ms (32.6 ms–44.7 ms) | 570 ms (539 ms–606 ms) | 36.2 ms | 73.5 ms | 0.07 | 1.07 | 0.94 | 0.53 | 88 / 3 / 4 | 20,086 / 3,590,089 | 770 / 4,376 |
| `cube` D5 10^4 | 69.3 ms (65.9 ms–75.6 ms) | 52.4 ms (50.2 ms–58.5 ms) | 236 ms (222 ms–244 ms) | 77.3 ms | 89.4 ms | 0.29 | 0.90 | 0.68 | 0.77 | 76 / 10 / 14 | 125,948 / 2,010,291 | 961 / 20,232 |
| `cube` D5 10^5 | 250 ms (243 ms–274 ms) | 207 ms (201 ms–232 ms) | 1.78 s (1.71 s–1.86 s) | 334 ms | 397 ms | 0.14 | 0.75 | 0.62 | 0.63 | 83 / 7 / 10 | 344,329 / 20,893,652 | 2,339 / 48,818 |
| `cube` D6 10^4 | 702 ms (681 ms–759 ms) | 506 ms (494 ms–542 ms) | 2.14 s (2.07 s–2.32 s) | 1.28 s | 1.42 s | 0.33 | 0.55 | 0.39 | 0.49 | 72 / 11 / 17 | 1,234,219 / 13,946,956 | 1,882 / 174,102 |
| `cube` D6 10^5 | 2.89 s (2.83 s–2.90 s) | 2.22 s (2.17 s–2.22 s) | 10.9 s (10.7 s–11.1 s) | 6.40 s | 6.96 s | 0.26 | 0.45 | 0.35 | 0.42 | 77 / 10 / 13 | 4,718,136 / 149,602,813 | 5,444 / 518,754 |
| `sphere` D2 10^4 | 2.79 ms (2.51 ms–3.25 ms) | 0.75 ms (0.68 ms–0.83 ms) | 0.67 ms (0.58 ms–0.73 ms) | 6.77 ms | 11.1 ms | 4.14 | 0.41 | 0.11 | 0.25 | 27 / 17 / 52 | 19,998 / 189,622 | 10,000 / 10,000 |
| `sphere` D2 10^5 | 32.2 ms (29.9 ms–36.5 ms) | 8.56 ms (7.99 ms–9.05 ms) | 7.51 ms (7.08 ms–8.30 ms) | 107 ms\* | 155 ms\* | 4.28 | 0.30\* | 0.08\* | 0.21\* | 27 / 16 / 54 | 199,989 / 2,395,166 | 100,000 / 100,000 |
| `sphere` D2 10^6 | 459 ms (429 ms–518 ms) | 125 ms (119 ms–147 ms) | 87.0 ms (83.3 ms–104 ms) | 1.90 s\* | 2.51 s\* | 5.27 | 0.24\* | 0.07\* | 0.18\* | 27 / 11 / 55 | 1,997,357 / 28,928,990 | 999,973 / 999,973 |
| `sphere` D3 10^4 | 38.9 ms (34.4 ms–42.1 ms) | 27.4 ms (24.2 ms–30.4 ms) | 24.3 ms (21.9 ms–32.2 ms) | 19.9 ms | 29.1 ms | 1.60 | 1.96 | 1.38 | 1.34 | 70 / 11 / 19 | 56,158 / 341,173 | 10,000 / 19,996 |
| `sphere` D3 10^5 | 467 ms (431 ms–503 ms) | 329 ms (305 ms–366 ms) | 398 ms (367 ms–528 ms) | 249 ms | 386 ms | 1.17 | 1.88 | 1.32 | 1.21 | 71 / 11 / 16 | 565,222 / 4,349,516 | 100,000 / 199,996 |
| `sphere` D3 10^6 | 6.70 s (6.60 s–7.23 s) | 4.65 s (4.59 s–5.15 s) | 7.43 s (7.35 s–7.52 s) | 3.37 s | 4.90 s | 0.90 | 1.99 | 1.38 | 1.37 | 69 / 14 / 16 | 5,654,825 / 52,940,589 | 1,000,000 / 1,999,996 |
| `sphere` D4 10^4 | 157 ms (151 ms–186 ms) | 115 ms (106 ms–134 ms) | 125 ms (116 ms–142 ms) | 95.8 ms | 129 ms | 1.25 | 1.64 | 1.20 | 1.22 | 73 / 12 / 18 | 258,048 / 889,697 | 10,000 / 67,192 |
| `sphere` D4 10^5 | 2.20 s (2.07 s–2.32 s) | 1.49 s (1.39 s–1.61 s) | 1.33 s (1.32 s–1.36 s) | 1.38 s | 1.93 s | 1.65 | 1.59 | 1.08 | 1.14 | 68 / 15 / 17 | 2,626,423 / 10,538,224 | 100,000 / 675,154 |
| `sphere` D5 10^4 | 1.00 s (959 ms–1.12 s) | 705 ms (690 ms–773 ms) | 995 ms (964 ms–1.10 s) | 841 ms | 1.06 s | 1.01 | 1.19 | 0.84 | 0.95 | 70 / 13 / 16 | 1,413,713 / 3,017,965 | 10,000 / 299,994 |
| `sphere` D5 10^5 | 13.3 s (13.2 s–13.9 s) | 8.75 s (8.75 s–9.29 s) | 11.3 s (11.0 s–11.5 s) | 10.6 s | 13.2 s | 1.18 | 1.26 | 0.83 | 1.00 | 66 / 17 / 18 | 15,164,634 / 35,472,228 | 100,000 / 3,112,922 |
| `sphere` D6 10^4 | 8.26 s (8.09 s–8.27 s) | 5.78 s (5.72 s–5.83 s) | 9.40 s (9.26 s–9.41 s) | 8.24 s | 9.66 s | 0.88 | 1.00 | 0.70 | 0.86 | 70 / 14 / 15 | 8,596,125 / 14,508,402 | 10,000 / 1,570,453 |

Not timed: `sphere` D6 10^5, as in the sections above.

### Delaunay

| Set | convx `build()` | convx construction | CGAL | Qhull compute | Qhull whole run | `build()` / CGAL | `build()` / Qhull compute | Construction / Qhull compute | `build()` / Qhull whole run | Construction / pass after / publication, % of `build()` | Qhull hyperplanes / distance tests | Sites / simplices |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 19.6 ms (18.8 ms–27.1 ms) | 15.2 ms (14.5 ms–22.7 ms) | 4.61 ms (3.90 ms–6.92 ms) | 20.3 ms | 30.0 ms | 4.26 | 0.97 | 0.75 | 0.65 | 77 / 20 / 1 | 56,426 / 377,169 | 10,000 / 19,974 |
| `cube` D2 10^5 | 206 ms (191 ms–213 ms) | 159 ms (150 ms–163 ms) | 44.4 ms (41.3 ms–52.1 ms) | 252 ms | 429 ms | 4.64 | 0.82 | 0.63 | 0.48 | 77 / 21 / 0 | 565,024 / 4,837,796 | 100,000 / 199,973 |
| `cube` D2 10^6 | 2.23 s (2.22 s–2.45 s) | 1.61 s (1.56 s–1.71 s) | 458 ms (457 ms–471 ms) | 3.29 s | 5.09 s | 4.88 | 0.68 | 0.49 | 0.44 | 72 / 27 / 0 | 5,661,165 / 56,857,785 | 1,000,000 / 1,999,959 |
| `sphere` D2 10^4 | 17.3 ms (16.9 ms–18.3 ms) | 14.9 ms (14.5 ms–15.8 ms) | 3.79 ms (3.59 ms–4.39 ms) | 22.6 ms\* | 29.3 ms\* | 4.57 | 0.77\* | 0.66\* | 0.59\* | 86 / 12 / 0 | 45,385 / 437,697 | 10,000 / 9,998 |
| `sphere` D2 10^5 | 141 ms (139 ms–145 ms) | 115 ms (113 ms–119 ms) | 32.7 ms (30.3 ms–36.1 ms) | 346 ms\* | 485 ms\* | 4.30 | 0.41\* | 0.33\* | 0.29\* | 82 / 16 / 0 | 450,890 / 5,578,213 | 100,000 / 99,998 |
| `sphere` D2 10^6 | 1.57 s (1.47 s–1.61 s) | 1.24 s (1.16 s–1.27 s) | 325 ms (307 ms–337 ms) | 11.9 s\* | 13.5 s\* | 4.83 | 0.13\* | 0.10\* | 0.12\* | 79 / 19 / 0 | 6,096,936 / 140,732,829 | 1,000,000 / 1,000,025 |
| `cube` D3 10^4 | 99.2 ms (96.9 ms–108 ms) | 79.9 ms (77.6 ms–88.8 ms) | 29.1 ms (28.1 ms–30.8 ms) | 93.7 ms | 131 ms | 3.41 | 1.06 | 0.85 | 0.76 | 81 / 19 / 0 | 255,564 / 881,892 | 10,000 / 66,373 |
| `cube` D3 10^5 | 1.12 s (1.10 s–1.15 s) | 864 ms (850 ms–889 ms) | 319 ms (308 ms–340 ms) | 1.29 s | 1.96 s | 3.52 | 0.87 | 0.67 | 0.57 | 77 / 22 / 0 | 2,602,230 / 11,306,061 | 100,000 / 671,608 |
| `cube` D3 10^6 | 12.1 s (11.7 s–12.3 s) | 8.85 s (8.46 s–8.90 s) | 3.33 s (3.23 s–3.50 s) | 15.0 s | 22.3 s | 3.65 | 0.81 | 0.59 | 0.54 | 73 / 27 / 0 | 26,219,307 / 125,621,001 | 1,000,000 / 6,747,791 |
| `sphere` D3 10^4 | 250 ms (240 ms–340 ms) | 224 ms (215 ms–241 ms) | 124 ms (122 ms–134 ms) | 110 ms\* | 136 ms\* | 2.01 | 2.28\* | 2.04\* | 1.83\* | 90 / 10 / 0 | 191,857 / 1,121,621 | 10,000 / 30,038 |
| `sphere` D3 10^5 | 2.50 s (2.32 s–2.56 s) | 2.24 s (2.07 s–2.29 s) | 920 ms (885 ms–973 ms) | 1.85 s\* | 2.31 s\* | 2.72 | 1.35\* | 1.21\* | 1.08\* | 89 / 10 / 0 | 1,974,009 / 15,785,569 | 100,000 / 302,013 |
| `sphere` D3 10^6 | 20.3 s (19.9 s–20.3 s) | 17.9 s (17.6 s–18.0 s) | 6.49 s (6.27 s–6.50 s) | 16.7 s\* | 21.8 s\* | 3.12 | 1.21\* | 1.07\* | 0.93\* | 89 / 11 / 0 | 19,314,633 / 137,640,502 | 1,000,000 / 3,017,144 |
| `cube` D4 10^4 | 899 ms (859 ms–1.06 s) | 740 ms (699 ms–867 ms) | 625 ms (607 ms–659 ms) | 784 ms | 1.09 s | 1.44 | 1.15 | 0.94 | 0.83 | 82 / 18 / 0 | 1,397,526 / 3,183,815 | 10,000 / 295,350 |
| `sphere` D4 10^4 | 3.88 s (3.87 s–3.97 s) | 3.47 s (3.46 s–3.55 s) | 2.55 s (2.50 s–2.56 s) | 1.26 s\* | 1.45 s\* | 1.52 | 3.07\* | 2.75\* | 2.67\* | 89 / 11 / 0 | 1,078,477 / 4,371,357 | 10,000 / 128,710 |
| `cube` D5 10^4 | 8.07 s (7.88 s–8.38 s) | 6.53 s (6.40 s–6.71 s) | 6.88 s (6.69 s–6.96 s) | 8.09 s | 9.94 s | 1.17 | 1.00 | 0.81 | 0.81 | 81 / 19 / 0 | 8,593,628 / 15,161,004 | 10,000 / 1,551,630 |
| `sphere` D5 10^4 | 47.6 s (47.6 s–47.9 s) | 42.2 s (42.1 s–42.4 s) | 35.4 s (35.2 s–35.6 s) | 14.8 s\* | 16.1 s\* | 1.34 | 3.21\* | 2.85\* | 2.95\* | 89 / 11 / 0 | 6,939,777 / 19,199,869 | 10,000 / 674,290 |

### Reading

- **Hull `cube`**: `build()` is 0.45 to 1.19 times Qhull's compute time.
  - It is at or below 1.0 on D2 10^5, D3 10^4 and 10^5, D5, and D6, with D6 10^5 at 0.45.
  - It is 1.01 to 1.19 on the rest, highest on D2 10^6.
  - Construction alone is 0.35 to 0.94 times Qhull, below 1 on every set.
  - Against CGAL, `build()` is 0.07 to 0.66 at D >= 3 and 1.18 to 1.49 at D2.
- **Hull `sphere` D3 and D4**: still behind.
  - `build()` is 1.59 to 1.99 times Qhull's compute time, and construction alone 1.08 to 1.38.
  - Construction is 68 to 73% of `build()`. The pass after it (11 to 15%) and publication (16 to 19%) are the rest.
  - Against CGAL: 0.90 to 1.65.
- **Hull `sphere` D5 and D6**: `build()` is 1.00 to 1.26 times Qhull's compute time and 0.88 to 1.18 times CGAL. Construction alone is 0.70 to 0.84 times Qhull.
- **Hull `sphere` D2**: `build()` is 4.1 to 5.3 times CGAL's `convex_hull_2`, which returns only the hull points.
  - Publication is 52 to 55% of `build()`, the pass after construction 11 to 17%, and construction 27%.
  - Construction alone is 1.1 to 1.4 times CGAL.
- **Delaunay against Qhull**: `build()` is 0.68 to 1.06 times Qhull's compute time on `cube` D2 and D3, and 1.00 to 1.15 on `cube` D4 and D5. The `sphere` cells are not the same output.
- **Delaunay against CGAL**: `build()` is 4.3 to 4.9 times CGAL at D = 2, 2.0 to 3.7 at D = 3, and 1.17 to 1.52 at D = 4 and 5.
  - Construction is 72 to 90% of `build()`, and the pass after it 10 to 27%.
  - Publication is at most 1%.
- **Compared with the sections above**: this section is from another machine and another CGAL and compiler version. Read ratios within one section.

## Profile after P6, convx `0630c0e` (#285)

Where `build()` spends its time on one set from each gap the section above left: hull `sphere` D3 against Qhull, hull `sphere` D2 against CGAL, and Delaunay D2 and D3 against CGAL.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `0630c0e`, rustc 1.97.1, `--release` with debug info, baseline target |
| Tool | Intel VTune Profiler 2026.4.0, `-collect hotspots -knob sampling-mode=sw` (user-mode sampling, 10 ms interval) |
| Machine | Intel Core i5-13400F, Windows 11 |
| Cores | The target pinned to one core (`start /affinity 10`) |
| Target | A scratch binary that generates the set (`tests/common/generator.rs`, seed 1) and calls `build()` in a loop: 20 builds of hull `sphere` D3 10^5, 12 of hull `sphere` D2 10^6, 40 of Delaunay `cube` D2 10^5, 9 of Delaunay `cube` D3 10^5 |
| Samples | CPU time 8.82 s, 5.48 s, 7.47 s, and 9.30 s, so 550 to 930 samples per set |
| Reported | Inclusive share of the process's CPU time, from the call tree (`-report gprof-cc`). The loop's `build()` is 98.9 to 99.7% of it; generating the points is the rest |

The scratch binary and the result directories are not kept. Inlining folds a callee into its caller, so a row below can hold the time of functions it inlined; rows of 5% and less are a few dozen samples.

### Hull `sphere` D3 10^5

| Part | Share of the process |
| --- | ---: |
| Construction (`SimplicialHull::build`) | 70.6% |
| Planning an insertion (`plan_region`) | 45.1% |
| The planes of the new facets (`plan_planes`) | 20.7% |
| Assigning the outside points again (`take_outside`) | 10.8% |
| Walking the visible region (`walk_region`) | 9.1% |
| Taking the next candidate (`BinaryHeap::pop`) | 7.7% |
| Applying the insertion (`commit`) | 6.0% |
| Publication (`publish`) | 17.2% |
| The pass after construction (`classify_built`) | 11.7% |
| Merge, inside that pass | 7.4% |
| `memmove` (`VCRUNTIME140.dll`), in every part | 7.6% |
| The heap (`ntdll.dll`), in every part | 4.1% |

The four rows under construction that follow `plan_region` are parts of it or beside it: `plan_planes` and `take_outside` are inside `plan_region`; the walk, the heap, and the commit are beside it.

### Hull `sphere` D2 10^6

| Part | Share of the process |
| --- | ---: |
| Publication (`publish`) | 55.5% |
| Edge unit normals (`edge_unit_normal`) | 15.7% |
| The lexicographic order of facets and of boundary simplices (`lexicographic_order`) | 7.3% |
| Writing the lists (`Lists::push`, `push_iter`) | about 5% each |
| Construction (`build_polygon`, the strict chain) | 26.6% |
| The stable sort of the points before the chain (`strict_cycle`) | 14.6% |
| Classification (`classify_chain`) | 11.6% |
| Acceptance and duplicate detection (`accept`) | 5.2% |

A second run of this set, 20 builds and 9.50 s of CPU time, was read with the callers of each function. It is the run that places the stable sort under `strict_cycle`. Its shares differ from the table by the noise of two runs: publication 57.2%, construction 27.0%, `strict_cycle` 24.6% with its sort 14.3%, and `edge_unit_normal` 17.3%. The table above is the first run.

### Delaunay `cube` at 10^5

| Part | D = 2 | D = 3 |
| --- | ---: | ---: |
| Insertion (`insert::Mesh::build`) | 73.5% | 76.8% |
| The in-sphere test (`Mesh::conflict`) | 32.5% | 37.1% |
| Point location (`Mesh::locate`) | 23.0% | 13.7% |
| The orientation predicate (`Sites::orient`) | 18.5% | 11.3% |
| The lifted predicate (`Sites::lifted`) | 32.8% | 38.4% |
| The semi-static stage (`semi_static::sign`) | 30.6% | 34.2% |
| The draft of the result (`Draft::finish`) | 7.6% | 7.5% |
| The insertion order (`brio`), D = 2 | 4.2% | not in the top rows |

At D = 2 the in-sphere test takes 2.4 s of the 7.47 s; the formula itself (`semi_static::lifted2`) is 1.5 s of that with what it calls, and the rest, more than a third, is the entry above it: `Sites::lifted`, `orient_lifted_with`, `sign_of`, `filtered`, and `estimate`.

### Reading

- **A share is not a saving.** It says where the time is, not how much a change would remove.
- **Hull `sphere` D3**: the largest single part is the plane of each new facet, a working normal with its error bound and the cull plane. Taking the next candidate from the heap is 7.7%, and `memmove` 7.6%, most of it called from the control flow of `?` and from array construction, that is, large values returned by value. Publication and the pass after construction are 29% together. P6-16 (#286) prices the heap and the moves.
- **Hull `sphere` D2**: publication is more than half: the edge normals are 15.7% of `build()`, the lexicographic order 7.3%, and writing the lists about 10%. The stable sort is in construction, where the chain needs its points in order. P6-17 (#287) is the design row for publication.
- **Delaunay D2 and D3**: insertion is three quarters, and predicates about half of `build()`. At D = 2 more than a third of the in-sphere time is in the dimension-generic entry around the formula. P6-18 (#288) is the design row for a dedicated insertion at D = 2 and D = 3.

## The order of candidates in hull construction, convx `0630c0e` (#286)

The spike P6-16: what the candidate heap of the sequential hull costs, and what values moved by value cost. It is the measurement behind the bucket queue of P6-19 (#291).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `0630c0e`, and scratch copies of it, one per variant. rustc 1.97.1, `--release`, baseline target |
| Machine | Intel Core i5-13400F, Windows 11 |
| Cores | One, every process pinned (processor affinity) |
| Timed | `ConvexHullBuilder::new(dim, &pts).build()` on sets generated with `tests/common/generator.rs`, seed 1 |
| Rounds | The variants alternated per round: 5 rounds × 3 builds, or 3 rounds × 1 for the long sets |
| Reported | Median with min–max, in ms. A ratio is the variant's median over the heap's; "inside the spread" when the two ranges overlap |

Every variant published the same vertex and facet counts as `main` and counted the facets it created. The patches of the variants are on #286; the scratch copies are not kept.

### Orders that need no heap

Last in, first out, and first in, first out, over all candidates.

| Set | Heap (`main`) | Last in, first out | Ratio | First in, first out | Ratio | Facets created: heap / LIFO / FIFO |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `sphere` D3 10^4 | 37.8 (36.6–44.2) | 35.2 (33.7–43.5) | 0.93, inside the spread | 40.2 (37.9–43.9) | 1.06, inside the spread | 51,757 / 59,603 / 55,777 |
| `sphere` D3 10^5 | 475 (458–518) | 392 (366–435) | 0.83 | 438 (421–491) | 0.92, inside the spread | 518,789 / 610,568 / 560,158 |
| `sphere` D3 10^6 | 7194 (6970–7209) | 4327 (4259–4472) | 0.60 | 5550 (5506–5556) | 0.77 | 5,191,620 / 6,187,862 / 5,607,429 |
| `sphere` D4 10^4 | 161 (154–169) | 178 (172–192) | 1.10 | 173 (161–188) | 1.07, inside the spread | 225,744 / 313,042 / 254,128 |
| `sphere` D4 10^5 | 1967 (1937–1976) | 2017 (1951–2053) | 1.03, inside the spread | 1990 (1930–2037) | 1.01, inside the spread | 2,299,988 / 3,414,742 / 2,610,810 |
| `sphere` D5 10^4 | 1021 (1014–1038) | 1235 (1221–1281) | 1.21 | 1118 (1084–1137) | 1.10 | 1,243,311 / 1,801,255 / 1,397,066 |
| `sphere` D6 10^4 | 8552 (8406–8671) | 10776 (10748–10914) | 1.26 | 9206 (8946–9244) | 1.08 | 7,648,576 / 11,071,464 / 8,486,464 |
| `cube` D3 10^6 | 153 (145–164) | 234 (222–243) | 1.53 | 180 (172–215) | 1.18 | 1,420 / 10,881 / 1,608 |
| `cube` D4 10^5 | 42.8 (40.0–46.9) | 162 (154–174) | 3.77 | 47.3 (44.7–50.7) | 1.10, inside the spread | 14,048 / 170,308 / 19,700 |
| `cube` D5 10^5 | 266 (254–293) | 2095 (2054–2165) | 7.87 | 363 (352–368) | 1.36 | 180,494 / 2,414,895 / 290,034 |
| `cube` D6 10^4 | 701 (687–765) | 3130 (3054–3427) | 4.46 | 1042 (1019–1127) | 1.49 | 676,344 / 4,134,420 / 1,138,176 |
| `cube` D6 10^5 | 3229 (3212–3266) | 28933 (28831–29658) | 8.96 | 5859 (5765–6019) | 1.81 | 2,161,784 / 29,498,054 / 4,358,876 |

### A bucket queue, and smaller inline lists

Candidates in buckets by the exponent and the top two mantissa bits of their distance, the highest bucket first, last in first out inside a bucket. The last two columns shrink to 4 the inline capacity of the plane-related short lists (`Small<(f64, f64), 10>`, `Small<&[f64], 10>`, `Small<f64, 8>`), which prices the copies of those values at D = 3 and D = 4; above D = 4 such a list falls back to the heap, so it is not timed there.

| Set | Heap (`main`) | Bucket queue | Ratio | Facets created: heap / buckets | Inline capacity 4 | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `sphere` D3 10^5 | 467 (452–500) | 429 (408–491) | 0.92, inside the spread | 518,789 / 517,417 | 458 (434–482) | 0.98, inside the spread |
| `sphere` D3 10^6 | 7006 (6981–7248) | 6153 (6122–6526) | 0.88 | 5,191,620 / 5,179,549 | 6809 (6696–7408) | 0.97, inside the spread |
| `sphere` D4 10^5 | 2221 (2192–2231) | 2081 (2065–2097) | 0.94 | 2,299,988 / 2,299,930 | 2153 (2113–2174) | 0.97 |
| `sphere` D5 10^4 | 1119 (1110–1144) | 1110 (1098–1115) | 0.99, inside the spread | 1,243,311 / 1,248,597 | not applicable above D = 4 |  |
| `cube` D3 10^6 | 164 (160–177) | 154 (147–163) | 0.94, inside the spread | 1,420 / 1,430 | 164 (157–183) | 1.00, inside the spread |
| `cube` D5 10^5 | 283 (276–306) | 275 (267–291) | 0.97, inside the spread | 180,494 / 177,548 | not applicable above D = 4 |  |
| `cube` D6 10^4 | 753 (741–800) | 774 (762–842) | 1.03, inside the spread | 676,344 / 706,020 | not applicable above D = 4 |  |
| `cube` D6 10^5 | 3254 (3215–3283) | 3283 (3221–3304) | 1.01, inside the spread | 2,161,784 / 2,203,644 | not applicable above D = 4 |  |

The width of a bucket, in single runs (ms), before the alternated run above:

| Mantissa bits | `sphere` D3 10^5 | `cube` D5 10^5 | `cube` D6 10^4 |
| ---: | ---: | ---: | ---: |
| Heap (`main`) | 461 to 481 | 250 to 262 | 683 to 689 |
| 0 | 397 to 424 | 254 to 270 | 761 to 774 |
| 2 | 422 to 438 | 248 to 250 | 702 to 717 |
| 4 | 425 to 431 | 255 to 264 | 706 to 718 |
| 8 | 432 to 444 | 257 to 257 | 707 to 708 |

A fourth order was tried and gave nothing: the farthest candidate of the facets the last insertion created, when its distance is at least τ times the heap's top (τ = 0, 0.25, 0.5, 0.75). On `sphere` D3 10^5 it timed 455 to 487 ms in single runs, against 446 to 481 ms for `main`.

### Reading

- **The heap's order is not waste.** Last in, first out is 0.60 to 0.83 of `main` on `sphere` D3 from 10^5 points while creating 18 to 19% more facets, 1.10 to 1.26 on `sphere` D4 10^4, D5, and D6, and 1.5 to 9.0 times slower on `cube`, where it creates 6 to 14 times the facets. First in, first out is 0.77 on `sphere` D3 10^6 and up to 1.81 on `cube`.
- **The bucket queue** keeps the order up to the width of a bucket. It is 0.88 on `sphere` D3 10^6 and 0.94 on `sphere` D4 10^5, beyond the spread, and inside the spread on the other six sets (0.92 to 1.03), creating within 5% of the heap's facets.
- **`cube` D6 10^4 is slower with the bucket queue by about 3%.** The alternated run is inside the spread there (1.03), but the single runs do not overlap (702 to 717 against 683 to 689), and the queue creates 4% more facets on that set. Two bits lose the least there among the widths tried.
- **Values moved by value** cost 0.97 to 1.00, beyond the spread only on `sphere` D4 10^5.
- The profile's 7.7% for `BinaryHeap::pop` at 10^5 understates the order's cost at 10^6: `main` takes 15 times as long for 10 times the points, and the last-in-first-out variant 11 times. What that order gains beyond the heap's own cost was not separated from the locality of working on new facets.

## Delaunay insertion specialized for D = 2 and D = 3, convx `b640ed5` (#294)

P6-21: the insertion compiled once for D = 2, once for D = 3, and once for every other dimension, against `main` at `f04212b`. `b640ed5` is the commit of the pull request of #294 that was measured.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `f04212b` and the head `b640ed5`. rustc 1.97.1, `--release`, baseline target |
| Machine | Intel Core i5-13400F, Windows 11 |
| Cores | One, every process pinned (processor affinity) |
| Timed | `DelaunayBuilder::new(dim, &pts).build()` on sets generated with `tests/common/generator.rs`, seed 1 |
| Rounds | Base and head alternated per round: 5 rounds × 3 builds, or 3 rounds × 1 for the long sets |
| Reported | Median with min–max, in ms. "inside the spread" when the two ranges overlap |
| Profile | Intel VTune Profiler 2026.4.0, user-mode sampling at 10 ms, the target pinned; inclusive shares of the process from the call tree |

Both sides published the same number of simplices on every set. The timing binary and the profile results are not kept.

### Wall time

| Set | `main` | Head | Head / `main` |
| --- | ---: | ---: | ---: |
| `cube` D2 10^4 | 19.6 (18.5–21.1) | 13.8 (13.2–14.7) | 0.71 |
| `cube` D2 10^5 | 214 (209–226) | 155 (147–178) | 0.73 |
| `cube` D2 10^6 | 2552 (2455–2600) | 1872 (1843–1874) | 0.73 |
| `sphere` D2 10^4 | 20.1 (18.7–21.7) | 15.9 (15.2–17.4) | 0.79 |
| `sphere` D2 10^5 | 166 (159–179) | 119 (116–126) | 0.72 |
| `sphere` D2 10^6 | 1705 (1686–1748) | 1212 (1207–1230) | 0.71 |
| `cube` D3 10^4 | 105 (103–110) | 85.4 (82.6–89.7) | 0.82 |
| `cube` D3 10^5 | 1166 (1114–1199) | 965 (940–1011) | 0.83 |
| `cube` D3 10^6 | 12586 (12554–12714) | 10626 (10562–10793) | 0.84 |
| `sphere` D3 10^4 | 289 (281–300) | 295 (285–310) | 1.02, inside the spread |
| `sphere` D3 10^5 | 2652 (2651–2671) | 2664 (2627–2693) | 1.00, inside the spread |
| `sphere` D3 10^6 | 22781 (22317–22883) | 22491 (22385–22901) | 0.99, inside the spread |
| `cube` D4 10^4 | 1002 (984–1042) | 1010 (990–1025) | 1.01, inside the spread |
| `sphere` D4 10^4 | 4406 (4386–4466) | 4340 (4331–4358) | 0.99 |
| `cube` D5 10^4 | 9100 (9042–9300) | 9154 (8963–9190) | 1.01, inside the spread |
| `sphere` D5 10^4 | 51652 (51176–52674) | 51543 (49568–52104) | 1.00, inside the spread |

### Profile of the head

Shares of the process on three sets at 10^5 sites: 50 builds of `cube` D2 (6.77 s of CPU time), 8 of `cube` D3 (6.98 s), and 3 of `sphere` D3 (7.56 s).

| Part | `cube` D2 | `cube` D3 | `sphere` D3 |
| --- | ---: | ---: | ---: |
| Insertion (`insert::Mesh::build`) | 66.3% | 75.2% | 89.8% |
| The in-sphere test (`Mesh::conflict`) | 36.1% | 36.7% | 77.0% |
| Point location (`Mesh::locate`) | 12.8% | 11.9% | 5.8% |
| The semi-static stage (`semi_static::sign`) | 37.5% | 42.5% | 15.1% |
| The exact stage (`exact::sign_exact`) | no sample | no sample | 62.6% |
| The draft of the result (`Draft::finish`) | 10.8% | 9.0% | 1.0% |

### Reading

- **D = 2**: `build()` is 0.71 to 0.79 of `main` on all six sets, beyond the spread.
- **D = 3, `cube`**: 0.82 to 0.84 on all three sets, beyond the spread.
- **D = 3, `sphere`**: no change, 0.99 to 1.02 inside the spread. On these sets the sites are near one sphere, the semi-static bound certifies few in-sphere tests, and the exact stage is 62.6% of `build()`. The specialization shortens the way to the first stage of a predicate; it does not touch the exact stage.
- **D = 4 and D = 5**: 0.99 to 1.01. `sphere` D4 reads 0.99 beyond the spread, which is 1.5% on three builds.
- Per build of `cube` D2 at 10^5, point location went from 23.0% of 187 ms to 12.8% of 135 ms, and the in-sphere test from 32.5% of 187 ms to 36.1% of 135 ms (the first figures are those of the profile after P6 above).
- The keep criterion of ADR 0005 first asked for every Delaunay set of D = 2 and D = 3 from 10^5 sites to be faster beyond the spread, which `sphere` D3 at 10^5 and 10^6 is not. The owner amended it on 2026-10-08 to the sets where the first stage of the predicates decides, with nothing slower on any set; this measurement meets that.

## ParGeo-style parallel hull prototypes at 1 to 16 threads, convx `f57627f` (#276)

The spike P6-14: parallel reservation rounds and a pseudohull filter, timed on the owner's 16-thread machine. The 4-core run is on #276.

### Method

| Item | Value |
| --- | --- |
| convx | branch `spike/276-pargeo` at `f57627f` (not for `main`): `examples/spike_276`, `SPIKE_MODE` = `seq`, `rounds`, `filter`, or `both`. rustc 1.97.1, `--release` |
| Machine | Intel Core i5-13400F, Windows 11: 10 cores and 16 logical processors, 6 performance cores with two threads each (logical 0 to 11) and 4 efficiency cores (logical 12 to 15) |
| Cores | `start /affinity` on the first T logical processors, `RAYON_NUM_THREADS` = T. So 2 threads are the two threads of one performance core, 4 are two cores, 8 are four cores, and 16 are all ten |
| Timed | The whole build of a point file of `benches/sets.txt`, seed 1, one build per process |
| Rounds | 3, the configurations in order and then reversed |
| Reported | Median, in seconds; the sequential column with min–max |

Every mode published the sequential hull on every set (`spike_276 check`, 27 of 27). The script is `scripts/spike-276.ps1` on that branch.

### Wall time

| Set | Facets | Sequential | Rounds, 1 | Rounds, 2 | Rounds, 4 | Rounds, 8 | Rounds, 16 | Filter, 1 | Filter, 16 | Both, 16 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `sphere` D2 10^6 | 999,973 | 0.506 (0.506–0.563) | 0.548 | 0.561 | 0.580 | 0.535 | 0.551 | 0.570 | 0.526 | 0.503 |
| `sphere` D3 10^5 | 199,996 | 0.495 (0.450–0.559) | 1.165 | 1.097 | 0.771 | 0.633 | 0.602 | 0.528 | 0.465 | 0.633 |
| `sphere` D3 10^6 | 1,999,996 | 7.595 (7.510–7.693) | 14.52 | 12.99 | 8.789 | 7.205 | 6.580 | 7.621 | 6.950 | 6.647 |
| `sphere` D4 10^5 | 675,154 | 2.146 (2.129–2.166) | 4.343 | 4.203 | 2.834 | 2.288 | 2.076 | 2.224 | 2.050 | 2.092 |
| `sphere` D5 10^4 | 299,994 | 1.105 (1.084–1.109) | 1.948 | 1.931 | 1.285 | 1.008 | 1.285 | 1.107 | 1.050 | 1.347 |
| `sphere` D5 10^5 | 3,112,922 | 14.33 (14.257–14.379) | 24.82 | 22.51 | 15.18 | 11.72 | 10.89 | 14.13 | 13.18 | 11.10 |
| `sphere` D6 10^4 | 1,570,453 | 8.446 (8.394–8.547) | 14.05 | 12.73 | 8.696 | 6.734 | 6.859 | 8.382 | 7.870 | 6.886 |
| `cube` D6 10^5 | 518,754 | 3.164 (3.163–3.195) | 12.75 | 12.54 | 8.517 | 7.396 | 8.431 | 3.768 | 2.914 | 7.834 |
| `cube` D3 10^6 | 566 | 0.163 (0.160–0.165) | 0.504 | 0.488 | 0.420 | 0.419 | 0.458 | 0.419 | 0.109 | 0.141 |

Ratios to the sequential build ("inside" when the ranges of the three runs overlap), the points the filter dropped, the phases of the rounds at 16 threads in seconds (walk / reserve / plan / commit), the work of the rounds at one thread against the sequential build, and their scaling from 1 to 16 threads:

| Set | Rounds, 4 | Rounds, 8 | Rounds, 16 | Both, 16 | Filter, 1 | Filter, 16 | Filtered points | Phases at 16 | Rounds 1 / sequential | Rounds 1 / rounds 16 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `sphere` D2 10^6 | 1.15, inside | 1.06, inside | 1.09, inside | 0.99, inside | 1.13, inside | 1.04, inside | 0 | no rounds (D = 2) | 1.08 | 0.99 |
| `sphere` D3 10^5 | 1.56 | 1.28 | 1.22 | 1.28 | 1.07, inside | 0.94, inside | 0 | 0.06 / 0.03 / 0.15 / 0.16 | 2.35 | 1.93 |
| `sphere` D3 10^6 | 1.16 | 0.95 | 0.87 | 0.88 | 1.00, inside | 0.92 | 0 | 0.65 / 0.15 / 1.34 / 1.69 | 1.91 | 2.21 |
| `sphere` D4 10^5 | 1.32 | 1.07 | 0.97 | 0.97 | 1.04 | 0.96 | 0 | 0.25 / 0.10 / 0.41 / 0.46 | 2.02 | 2.09 |
| `sphere` D5 10^4 | 1.16 | 0.91 | 1.16 | 1.22 | 1.00, inside | 0.95 | 0 | 0.15 / 0.08 / 0.52 / 0.19 | 1.76 | 1.52 |
| `sphere` D5 10^5 | 1.06 | 0.82 | 0.76 | 0.77 | 0.99 | 0.92 | 0 | 1.38 / 0.32 / 1.96 / 2.09 | 1.73 | 2.28 |
| `sphere` D6 10^4 | 1.03, inside | 0.80 | 0.81 | 0.82 | 0.99, inside | 0.93 | 0 | 1.03 / 0.27 / 2.09 / 0.93 | 1.66 | 2.05 |
| `cube` D6 10^5 | 2.69 | 2.34 | 2.66 | 2.48 | 1.19 | 0.92 | 29,880 | 1.39 / 0.40 / 5.07 / 0.66 | 4.03 | 1.51 |
| `cube` D3 10^6 | 2.57 | 2.57 | 2.81 | 0.86 | 2.56 | 0.66 | 981,963 | 0.01 / 0.01 / 0.34 / 0.03 | 3.08 | 1.10 |

### Reading

- **The rounds scale 1.5 to 2.3 times from 1 to 16 threads** on the `sphere` sets from D = 3, while one thread of them does 1.7 to 2.4 times the work of the sequential build. At 16 threads they are 0.76 to 0.87 of the sequential build on `sphere` D3 10^6, D5 10^5, and D6 10^4, and slower on `sphere` D3 10^5 (1.22) and D5 10^4 (1.16).
- **On `cube` the rounds are 2.7 to 2.8 times slower** at 16 threads. Inserting in point order builds facets that farthest-first never creates.
- **The commit and the plans do not shrink**: at 16 threads on `sphere` D3 10^6 the serial commit is 1.69 s and the plans 1.34 s of 6.58 s.
- **The filter** drops 98% of the points of `cube` D3 10^6 and is 0.66 there at 16 threads, and 2.56 times slower at one thread. On `cube` D6 10^5 it drops 30% and reads 0.92.
- **A run at 16 threads reads 4 to 8% faster than the pinned sequential run without doing less work.** The filter drops no point on the `sphere` sets and still reads 0.92 to 0.96 at 16 threads on those from D = 3. The sequential run is pinned to logical processor 0; a 16-thread run may use any processor. Ratios within that margin are not gains: the 0.97 of the rounds on `sphere` D4 10^5 is one of them.
- **Against the keep criterion of #256** (faster beyond the spread on every hull set with at least 10^4 facets): not met at 4, 8, or 16 threads by any mode.
- **The sequential build has moved since this branch.** The spike branch is from before the bucket queue (#292), which made `build()` 0.86 to 0.95 on the large `sphere` sets, and before the planes on first use (#297), another 0.87 to 0.90: about 0.75 to 0.85 together. Against today's `main`, the 0.76 to 0.87 of the rounds would be about level.

## Recorded planes in insertion and classification, PR #308 (#306)

P6-22: construction records the plane numbers a point is on, insertion takes a zero sign from the record, and classification starts a recorded point at its facet (design §3).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `c86c1bf`; the store-only build at `a040d9a` (every slot carries a plane number, nothing reads it); the head at `9024602`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11 |
| Cores | One, every process pinned (processor affinity, logical processor 2) |
| Timed | `ConvexHullBuilder::new(dim, &pts).build()` on sets generated with `tests/common/generator.rs`, seed 1. `cubesurf` is not a family there: the harness draws a point of the generator's `cube` and sets one coordinate, chosen by the same generator, to −1 or +1 |
| Rounds | The variants alternated per round: 5 rounds × 3 builds, or 3 rounds × 1 for `grid` D6 10^4 and the sets of `build()` over 1.5 s |
| Reported | Median with min–max. A ratio is the variant's median over `main`'s; "inside the spread" when the two ranges overlap |
| Qhull | 2020.2 (Ubuntu 22.04 `qhull-bin`) under WSL2 on the same machine, `qconvex i s TI <file> TO <out>`, its "CPU seconds to compute hull (after input)". `main` and the head were timed there too, with rustc 1.99.0, alternated with Qhull per round and pinned with `taskset -c 2`, so the ratios to Qhull are within that run |
| Profile | Intel VTune hotspots, software sampling, on the head |

Every variant published the same vertex and facet counts as `main` on every set.

### The two sets of the spike

| Set | `main` | Store-only | Ratio | Head | Ratio | Vertices / facets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `cubesurf` D3 10^5 | 448 ms (422 ms–482 ms) | 451 ms (427 ms–496 ms) | 1.01, inside the spread | 196 ms (189 ms–229 ms) | 0.44 | 155 / 169 |
| `grid` D6 10^4 | 5.99 s (5.87 s–6.00 s) | 5.97 s (5.91 s–6.13 s) | 1.00, inside the spread | 4.03 s (3.96 s–4.06 s) | 0.67 | 89 / 17 |

Against Qhull, under WSL2:

| Set | `main` | Head | Qhull compute | `main` / Qhull | Head / Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cubesurf` D3 10^5 | 436 ms (428 ms–443 ms) | 184 ms (180 ms–191 ms) | 31.7 ms (30.6 ms–34.6 ms) | 13.7 | 5.8 |
| `grid` D6 10^4 | 5.48 s (5.45 s–5.52 s) | 3.74 s (3.70 s–3.78 s) | 373 ms (369 ms–378 ms) | 14.7 | 10.0 |

Qhull's own counters (`qconvex s Ts`): on `cubesurf` D3 10^5, 168 points processed, 2,236,537 distance tests for the hull and 7,037 for merging, 155 vertices and 169 facets, as convx. On `grid` D6 10^4, 468 points processed, 8,004,117 distance tests for the hull and 586,608 for merging, and 17 facets, as convx; Qhull lists 193 vertices against convx's 89, because the vertices of its merged facets include points that are not extreme.

Shares of `build()` on the head, inclusive:

| Set | Side tests in insertion (`take_outside`) | Classification (`classify_built`) |
| --- | ---: | ---: |
| `cubesurf` D3 10^5 | 64.8% | 30.2% |
| `grid` D6 10^4 | 37.9% in the top-level build | 61.4%, of which most is the sub-hulls of coplanar faces, themselves mostly `take_outside` |

On `main` at `dbe136c` (#306) the same rows were 58.6% / 38.9% and 68.8% / 27.7%.

### The hull sets of this file

| Set | `main` | Head | Ratio | Vertices / facets |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 0.77 ms (0.62 ms–1.62 ms) | 0.68 ms (0.60 ms–0.89 ms) | 0.88, inside the spread | 24 / 24 |
| `cube` D2 10^5 | 6.97 ms (6.58 ms–8.71 ms) | 6.68 ms (6.38 ms–8.25 ms) | 0.96, inside the spread | 25 / 25 |
| `cube` D2 10^6 | 74.0 ms (65.6 ms–81.0 ms) | 74.5 ms (65.3 ms–86.3 ms) | 1.01, inside the spread | 39 / 39 |
| `cube` D3 10^4 | 1.55 ms (1.38 ms–2.36 ms) | 1.56 ms (1.35 ms–2.62 ms) | 1.01, inside the spread | 121 / 238 |
| `cube` D3 10^5 | 12.6 ms (11.1 ms–14.2 ms) | 12.2 ms (11.3 ms–13.4 ms) | 0.97, inside the spread | 175 / 346 |
| `cube` D3 10^6 | 144 ms (130 ms–156 ms) | 150 ms (133 ms–166 ms) | 1.04, inside the spread | 285 / 566 |
| `cube` D4 10^4 | 7.66 ms (6.61 ms–8.23 ms) | 7.28 ms (6.58 ms–8.58 ms) | 0.95, inside the spread | 404 / 2,320 |
| `cube` D4 10^5 | 40.9 ms (38.6 ms–51.7 ms) | 41.5 ms (37.6 ms–75.9 ms) | 1.02, inside the spread | 770 / 4,376 |
| `cube` D5 10^4 | 64.0 ms (59.7 ms–75.5 ms) | 64.2 ms (61.8 ms–68.7 ms) | 1.00, inside the spread | 961 / 20,232 |
| `cube` D5 10^5 | 255 ms (244 ms–276 ms) | 256 ms (246 ms–264 ms) | 1.00, inside the spread | 2,339 / 48,818 |
| `cube` D6 10^4 | 672 ms (653 ms–699 ms) | 680 ms (662 ms–719 ms) | 1.01, inside the spread | 1,882 / 174,102 |
| `cube` D6 10^5 | 3.13 s (3.09 s–3.33 s) | 3.14 s (2.91 s–3.20 s) | 1.00, inside the spread | 5,444 / 518,754 |
| `sphere` D2 10^4 | 2.13 ms (1.99 ms–2.78 ms) | 2.11 ms (1.89 ms–3.40 ms) | 0.99, inside the spread | 10,000 / 10,000 |
| `sphere` D2 10^5 | 22.5 ms (20.5 ms–30.4 ms) | 22.7 ms (21.4 ms–24.7 ms) | 1.01, inside the spread | 100,000 / 100,000 |
| `sphere` D2 10^6 | 320 ms (309 ms–342 ms) | 318 ms (300 ms–361 ms) | 0.99, inside the spread | 999,973 / 999,973 |
| `sphere` D3 10^4 | 33.1 ms (29.8 ms–49.8 ms) | 33.8 ms (31.2 ms–36.2 ms) | 1.02, inside the spread | 10,000 / 19,996 |
| `sphere` D3 10^5 | 408 ms (381 ms–450 ms) | 418 ms (385 ms–457 ms) | 1.02, inside the spread | 100,000 / 199,996 |
| `sphere` D3 10^6 | 5.89 s (5.53 s–5.92 s) | 5.94 s (5.80 s–6.18 s) | 1.01, inside the spread | 1,000,000 / 1,999,996 |
| `sphere` D4 10^4 | 147 ms (141 ms–160 ms) | 151 ms (144 ms–167 ms) | 1.03, inside the spread | 10,000 / 67,192 |
| `sphere` D4 10^5 | 1.95 s (1.86 s–1.96 s) | 1.97 s (1.90 s–2.02 s) | 1.01, inside the spread | 100,000 / 675,154 |
| `sphere` D5 10^4 | 1.02 s (994 ms–1.04 s) | 1.03 s (992 ms–1.10 s) | 1.01, inside the spread | 10,000 / 299,994 |
| `sphere` D5 10^5 | 13.19 s (13.09 s–13.67 s) | 13.72 s (12.93 s–13.90 s) | 1.04, inside the spread | 100,000 / 3,112,922 |
| `sphere` D6 10^4 | 8.34 s (8.29 s–8.51 s) | 8.56 s (8.45 s–8.56 s) | 1.03, inside the spread | 10,000 / 1,570,453 |

### Reading

- **The two sets of the spike are faster beyond the spread**: 0.44 on `cubesurf` D3 10^5 and 0.67 on `grid` D6 10^4. The store-only build is 1.00 to 1.01 on both, inside the spread.
- **Qhull is still ahead on both**: the head is 5.8 times Qhull's compute time on `cubesurf` D3 10^5 (13.7 before) and 10.0 times on `grid` D6 10^4 (14.7 before). The side tests of insertion are still 65% of `cubesurf`, and on `grid` D6 the sub-hulls of the coplanar faces, which classification builds one dimension down, are most of the build.
- **No hull set of this file is slower beyond the spread in this run, but the `sphere` sets from D = 3 are slower by about 1 to 3%.** Their medians are 1.01 to 1.04 here, and 0.97 to 1.02 in a separate run of `main`, the store-only build, and the head on `sphere` D3 10^5 and 10^6, D4 10^5, and D6 10^4, where the store-only build read 0.98 to 1.01. Their points are in general position, so nothing is recorded; what they still pay is a plane number per new simplex, the sign of the apex kept per tested facet, and a check that the record is empty.
- **Two earlier commits of this branch were slower on `sphere`.** At `52b0401` every walk looked up the apex's record for each facet it tested, and `sphere` D3 10^6 read 1.03 beyond the spread. At `3f62832` the walk looked up only an apex with a record, but the plane number was still part of the geometry every side test reads, and `sphere` D4 10^5 read 1.02 beyond the spread. The head keeps the number out of that geometry and passes it only to the scan of outside points.

## Baseline of P7, PR #313 (#310)

The yardstick of ADR 0006 at the start of P7: every set of the ADR, judged against each reference with the same output.

### Method

The procedure is "Parity run of P7" in `docs/verification.md`.

| Item | Value |
| --- | --- |
| convx | The head of #308, `6e27ca2`, with the generator change of this pull request (no library code changed). #308 merged as `6612a87` with the same `src/`. rustc 1.99.0, `--release` with debug info, baseline target |
| CGAL | 5.4 (Ubuntu 22.04 `libcgal-dev`), g++ 11.4, the program and flags of #215 |
| Qhull | 2020.2 (Ubuntu 22.04 `qhull-bin`), `qconvex i s TI <file> TO <out>` and `qdelaunay i s TI <file> TO <out>` |
| Machine | Intel Core i5-13400F, Ubuntu 22.04 under WSL2 (kernel 5.15.133.1) on Windows 11 |
| Cores | One, every process pinned with `taskset -c 2` |
| Points | `benches/sets.txt`, seed 1, generator `xoshiro256starstar-v1`, written by `export_qhull_sets` |
| Rounds | One unrecorded convx build per set, then convx, CGAL, Qhull alternated per round: 5 rounds × 3 builds when that build took under 1.5 s, otherwise 3 × 1 |
| Reported | Median with min–max in parentheses; Qhull's compute time is a median. A ratio is convx's median `build()` over the reference's median. "(ref.)" marks a reference whose output differs, which is not judged (ADR 0006). A set is met when every judged ratio is at most 1.00 |
| Phases | Shares of `build()`: construction, the pass after it, publication; the rest is acceptance and duplicate detection |

The 41 sets took 30 minutes. The scratch example, the marks, and the runner are on #310.

### Counts

convx, CGAL, and Qhull agree on every vertex and facet count of the hull and every Delaunay simplex count, except as follows.

- Qhull merges vertices on hull `sphere` D2 at 10^5 and 10^6 (99,996 and 998,669 vertices) and cospherical regions on every Delaunay `sphere` set (for example D3 10^6: 2,911,192 regions against 3,017,144 simplices).
- On hull `cubesurf` D3 10^5, CGAL's triangulated boundary has the same 155 vertices and 306 triangles against 169 logical facets.
- On hull `grid` D6 10^4, CGAL's d-dimensional hull lists 3,661 vertices and 124,682 facets against 89 and 17, and Qhull lists 193 vertices, those of its merged facets, against 89, with the same 17 facets.

### Hull

Met: 9 of 25.

| Set | convx `build()` | `planes()` after it | Construction / pass after / publication, % | CGAL | Qhull compute | `build()` / CGAL | `build()` / Qhull | P7 | Qhull hyperplanes / distance tests | Vertices / facets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| `cube` D2 10^4 | 0.58 ms (0.56 ms–0.68 ms) | 0.00 ms | 68 / 14 / 2 | 0.48 ms (0.45 ms–0.70 ms) | 0.56 ms | 1.21 | 1.02 | not met | 45 / 65,050 | 24 / 24 |
| `cube` D2 10^5 | 5.70 ms (5.46 ms–6.86 ms) | 0.01 ms | 67 / 11 / 1 | 4.58 ms (4.31 ms–5.17 ms) | 5.64 ms | 1.24 | 1.01 | not met | 48 / 699,388 | 25 / 25 |
| `cube` D2 10^6 | 66.9 ms (65.8 ms–74.2 ms) | 0.17 ms | 55 / 9 / 1 | 45.9 ms (44.3 ms–49.2 ms) | 56.8 ms | 1.46 | 1.18 | not met | 76 / 6,417,683 | 39 / 39 |
| `cube` D3 10^4 | 1.28 ms (1.21 ms–1.49 ms) | 0.05 ms | 81 / 7 / 2 | 2.17 ms (2.08 ms–2.53 ms) | 1.51 ms | 0.59 | 0.85 | met | 755 / 138,265 | 121 / 238 |
| `cube` D3 10^5 | 10.7 ms (9.68 ms–11.5 ms) | 0.08 ms | 81 / 3 / 1 | 26.6 ms (25.4 ms–29.1 ms) | 11.6 ms | 0.40 | 0.93 | met | 952 / 1,279,410 | 175 / 346 |
| `cube` D3 10^6 | 141 ms (136 ms–153 ms) | 0.27 ms | 77 / 2 / 0 | 576 ms (557 ms–592 ms) | 148 ms | 0.24 | 0.95 | met | 1,888 / 13,617,348 | 285 / 566 |
| `cube` D4 10^4 | 6.58 ms (6.33 ms–11.8 ms) | 0.51 ms | 86 / 9 / 4 | 43.9 ms (42.8 ms–46.6 ms) | 6.79 ms | 0.15 | 0.97 | met | 11,158 / 402,164 | 404 / 2,320 |
| `cube` D4 10^5 | 35.9 ms (33.7 ms–38.6 ms) | 1.02 ms | 90 / 4 / 1 | 567 ms (547 ms–599 ms) | 34.2 ms | 0.06 | 1.05 | not met | 20,086 / 3,590,089 | 770 / 4,376 |
| `cube` D5 10^4 | 58.5 ms (56.3 ms–62.5 ms) | 5.82 ms | 84 / 11 / 5 | 231 ms (222 ms–242 ms) | 76.7 ms | 0.25 | 0.76 | met | 125,948 / 2,010,291 | 961 / 20,232 |
| `cube` D5 10^5 | 230 ms (220 ms–256 ms) | 14.2 ms | 88 / 7 / 3 | 1.74 s (1.70 s–1.82 s) | 326 ms | 0.13 | 0.71 | met | 344,329 / 20,893,652 | 2,339 / 48,818 |
| `cube` D6 10^4 | 635 ms (617 ms–653 ms) | 62.6 ms | 81 / 12 / 6 | 2.08 s (2.06 s–2.13 s) | 1.27 s | 0.30 | 0.50 | met | 1,234,219 / 13,946,956 | 1,882 / 174,102 |
| `cube` D6 10^5 | 2.80 s (2.74 s–2.90 s) | 190 ms | 83 / 12 / 5 | 11.16 s (11.08 s–11.48 s) | 6.92 s | 0.25 | 0.40 | met | 4,718,136 / 149,602,813 | 5,444 / 518,754 |
| `sphere` D2 10^4 | 1.79 ms (1.53 ms–2.62 ms) | 0.40 ms | 42 / 32 / 17 | 0.61 ms (0.59 ms–0.66 ms) | 6.56 ms | 2.92 | 0.27 (ref.) | not met | 19,998 / 189,622 | 10,000 / 10,000 |
| `sphere` D2 10^5 | 20.4 ms (17.7 ms–23.1 ms) | 5.58 ms | 40 / 31 / 20 | 8.06 ms (7.10 ms–8.85 ms) | 112 ms | 2.53 | 0.18 (ref.) | not met | 199,989 / 2,395,166 | 100,000 / 100,000 |
| `sphere` D2 10^6 | 287 ms (270 ms–322 ms) | 116 ms | 47 / 31 / 14 | 88.1 ms (83.6 ms–95.5 ms) | 1.93 s | 3.25 | 0.15 (ref.) | not met | 1,997,357 / 28,928,990 | 999,973 / 999,973 |
| `sphere` D3 10^4 | 28.6 ms (28.2 ms–30.7 ms) | 4.23 ms | 80 / 14 / 5 | 22.6 ms (21.7 ms–24.1 ms) | 18.4 ms | 1.27 | 1.56 | not met | 56,158 / 341,173 | 10,000 / 19,996 |
| `sphere` D3 10^5 | 357 ms (343 ms–375 ms) | 44.7 ms | 79 / 14 / 6 | 407 ms (390 ms–432 ms) | 248 ms | 0.88 | 1.44 | not met | 565,222 / 4,349,516 | 100,000 / 199,996 |
| `sphere` D3 10^6 | 5.38 s (5.33 s–5.39 s) | 565 ms | 74 / 18 / 8 | 8.02 s (7.89 s–8.26 s) | 3.46 s | 0.67 | 1.56 | not met | 5,654,825 / 52,940,589 | 1,000,000 / 1,999,996 |
| `sphere` D4 10^4 | 133 ms (130 ms–142 ms) | 15.9 ms | 80 / 14 / 6 | 120 ms (118 ms–135 ms) | 96.2 ms | 1.11 | 1.39 | not met | 258,048 / 889,697 | 10,000 / 67,192 |
| `sphere` D4 10^5 | 1.77 s (1.72 s–1.79 s) | 170 ms | 76 / 17 / 7 | 1.29 s (1.28 s–1.31 s) | 1.33 s | 1.38 | 1.33 | not met | 2,626,423 / 10,538,224 | 100,000 / 675,154 |
| `sphere` D5 10^4 | 894 ms (866 ms–923 ms) | 86.4 ms | 80 / 13 / 6 | 978 ms (950 ms–1.01 s) | 840 ms | 0.91 | 1.06 | not met | 1,413,713 / 3,017,965 | 10,000 / 299,994 |
| `sphere` D5 10^5 | 11.76 s (11.68 s–11.80 s) | 995 ms | 73 / 19 / 9 | 11.16 s (10.97 s–11.46 s) | 10.81 s | 1.05 | 1.09 | not met | 15,164,634 / 35,472,228 | 100,000 / 3,112,922 |
| `sphere` D6 10^4 | 7.40 s (7.38 s–7.45 s) | 553 ms | 77 / 16 / 7 | 9.43 s (9.35 s–9.46 s) | 8.13 s | 0.78 | 0.91 | met | 8,596,125 / 14,508,402 | 10,000 / 1,570,453 |
| `cubesurf` D3 10^5 | 170 ms (168 ms–179 ms) | 0.22 ms | 78 / 21 / 0 | 37.8 ms (36.6 ms–46.3 ms) | 28.9 ms | 4.51 | 5.90 | not met | 501 / 2,236,537 | 155 / 169 |
| `grid` D6 10^4 | 3.64 s (3.62 s–3.65 s) | 2.44 ms | 37 / 62 / 0 | 254 ms (254 ms–256 ms) | 347 ms | 14.29 (ref.) | 10.48 | not met | 14,992 / 8,004,186 | 89 / 17 |

### Delaunay

Met: 0 of 16.

| Set | convx `build()` | Construction / pass after / publication, % | CGAL | Qhull compute | `build()` / CGAL | `build()` / Qhull | P7 | Qhull hyperplanes / distance tests | Sites / simplices |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| `cube` D2 10^4 | 12.6 ms (12.3 ms–13.6 ms) | 73 / 24 / 1 | 4.07 ms (3.78 ms–4.53 ms) | 19.1 ms | 3.09 | 0.66 | not met | 56,426 / 377,169 | 10,000 / 19,974 |
| `cube` D2 10^5 | 138 ms (135 ms–142 ms) | 72 / 26 / 1 | 42.3 ms (40.8 ms–45.4 ms) | 236 ms | 3.26 | 0.58 | not met | 565,024 / 4,837,796 | 100,000 / 199,973 |
| `cube` D2 10^6 | 1.71 s (1.66 s–1.71 s) | 65 / 33 / 0 | 465 ms (465 ms–466 ms) | 3.34 s | 3.67 | 0.51 | not met | 5,661,165 / 56,857,785 | 1,000,000 / 1,999,959 |
| `sphere` D2 10^4 | 14.6 ms (14.1 ms–16.1 ms) | 85 / 13 / 0 | 3.82 ms (3.69 ms–4.04 ms) | 23.4 ms | 3.81 | 0.62 (ref.) | not met | 45,385 / 437,697 | 10,000 / 9,998 |
| `sphere` D2 10^5 | 112 ms (108 ms–119 ms) | 79 / 18 / 1 | 32.3 ms (31.0 ms–35.2 ms) | 377 ms | 3.46 | 0.30 (ref.) | not met | 450,890 / 5,578,213 | 100,000 / 99,998 |
| `sphere` D2 10^6 | 1.12 s (1.08 s–1.15 s) | 74 / 23 / 0 | 318 ms (313 ms–339 ms) | 11.84 s | 3.51 | 0.09 (ref.) | not met | 6,096,936 / 140,732,829 | 1,000,000 / 1,000,025 |
| `cube` D3 10^4 | 80.0 ms (78.1 ms–81.6 ms) | 80 / 19 / 0 | 29.2 ms (28.0 ms–30.8 ms) | 92.8 ms | 2.74 | 0.86 | not met | 255,564 / 881,892 | 10,000 / 66,373 |
| `cube` D3 10^5 | 930 ms (908 ms–953 ms) | 75 / 24 / 0 | 308 ms (305 ms–321 ms) | 1.27 s | 3.02 | 0.73 | not met | 2,602,230 / 11,306,061 | 100,000 / 671,608 |
| `cube` D3 10^6 | 10.71 s (10.42 s–10.85 s) | 69 / 30 / 0 | 3.48 s (3.46 s–3.49 s) | 15.67 s | 3.08 | 0.68 | not met | 26,219,307 / 125,621,001 | 1,000,000 / 6,747,791 |
| `sphere` D3 10^4 | 259 ms (253 ms–359 ms) | 90 / 10 / 0 | 131 ms (126 ms–155 ms) | 114 ms | 1.97 | 2.27 (ref.) | not met | 191,857 / 1,121,621 | 10,000 / 30,038 |
| `sphere` D3 10^5 | 2.53 s (2.48 s–2.54 s) | 90 / 10 / 0 | 921 ms (914 ms–927 ms) | 1.90 s | 2.75 | 1.33 (ref.) | not met | 1,974,009 / 15,785,569 | 100,000 / 302,013 |
| `sphere` D3 10^6 | 20.00 s (19.75 s–20.19 s) | 89 / 11 / 0 | 6.38 s (6.36 s–6.41 s) | 17.34 s | 3.13 | 1.15 (ref.) | not met | 19,314,633 / 137,640,502 | 1,000,000 / 3,017,144 |
| `cube` D4 10^4 | 892 ms (867 ms–987 ms) | 81 / 17 / 0 | 629 ms (620 ms–669 ms) | 802 ms | 1.42 | 1.11 | not met | 1,397,526 / 3,183,815 | 10,000 / 295,350 |
| `sphere` D4 10^4 | 3.86 s (3.84 s–3.91 s) | 89 / 11 / 0 | 2.53 s (2.52 s–2.54 s) | 1.28 s | 1.52 | 3.01 (ref.) | not met | 1,078,477 / 4,371,357 | 10,000 / 128,710 |
| `cube` D5 10^4 | 8.13 s (7.99 s–8.29 s) | 81 / 19 / 0 | 6.93 s (6.86 s–6.95 s) | 8.20 s | 1.17 | 0.99 | not met | 8,593,628 / 15,161,004 | 10,000 / 1,551,630 |
| `sphere` D5 10^4 | 47.03 s (45.94 s–47.11 s) | 89 / 11 / 0 | 34.38 s (34.08 s–35.03 s) | 14.32 s | 1.37 | 3.28 (ref.) | not met | 6,939,777 / 19,199,869 | 10,000 / 674,290 |

### Reading

- **Met: 9 of 41.** All are hull sets: `cube` D3 at every size, `cube` D4 10^4, `cube` D5 and D6 at both sizes, and `sphere` D6 10^4.
- **Delaunay D2 and D3 against CGAL: 2.74 to 3.81, except `sphere` D3 10^4 at 1.97.** This is the largest gap among the dimensions used most. Construction is 65 to 90% of `build()`, and the pass after it is 10 to 33%. P7-2 (#311) starts here.
- **Hull D2 against CGAL.**
  - `sphere`: 2.53 to 3.25. Construction is 40 to 47% of `build()`, the pass after it 31 to 32%, and publication 14 to 20%; `convex_hull_2` returns only the hull points. P7-3 (#312) changes what `build()` computes.
  - `cube`: 1.21 to 1.46 against CGAL, and 1.01 to 1.18 against Qhull.
- **Hull `sphere` D3 and D4 against Qhull: 1.33 to 1.56.** Construction alone is 1.01 to 1.25 times Qhull's compute time. Against CGAL, D3 is 0.67 to 1.27 and D4 is 1.11 to 1.38.
- **Hull `sphere` D5: 1.06 to 1.09 against Qhull, and 0.91 to 1.05 against CGAL.**
- **Hull `cube` D4 10^5: 1.05 against Qhull.** It read 1.03 in the run behind the Grill and 1.07 in the section After P6 above.
- **The degenerate sets.**
  - `cubesurf` D3 10^5: 4.51 against CGAL and 5.90 against Qhull.
  - `grid` D6 10^4: 10.48 against Qhull, with the pass after construction at 62% of `build()`. That pass includes the sub-hulls of coplanar faces.
- **Delaunay D4 and D5 against CGAL: 1.17 to 1.52.** Against Qhull, `cube` D4 is 1.11 and `cube` D5 0.99.

## Delaunay D2: a CGAL-style structure, and the cost of a subnormal product, PR #318 (#311)

The spike P7-2. Does a CGAL-style structure for D = 2 halve convx's insertion? The decision rule of the Grill of 2026-10-09 applies: a prototype at most 0.5 of convx's insertion on the four sets leads to a dedicated structure; otherwise the next rows remove the costs of the current insertion one by one.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `6612a87`, and a scratch copy of it holding the prototype. rustc 1.97.1, `--release` with debug info, baseline target |
| Prototype | A module beside `delaunay::insert` in the scratch copy. It stores one record per triangle (three vertices, three neighbors). Its cavity is found through neighbors. Its new triangles are linked as a fan, because in D = 2 the cavity boundary is one polygon around the new site. It uses convx's insertion order (`brio`), the same walk, the same conflict rules, and convx's predicates through `insert::Plane`, so only the structure differs. It keeps no cospherical flag per face. Its code is on #311 |
| Machine | Intel Core i5-13400F, Windows 11 |
| Cores | One, every process pinned (logical processor 2) |
| Timed | The insertion phase: `brio` and `Mesh::build` for convx, `brio` and the prototype's build. Both run in one process on the same accepted input, alternating which goes first. CGAL's construction time comes from the P7 baseline above, under WSL2 on the same machine, so it is a reference, not a ratio |
| Rounds | 3 processes per variant and set, 6 builds of each per process at 10^5 and 4 at 10^6 |
| Reported | Median with min–max, in ms |

The prototype published the same triangles as convx on every set (compared as sorted vertex triples).

### Structure alone

| Set | convx insertion | Prototype | Prototype / convx | CGAL construction (baseline, WSL2) |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 110.5 (104.9–114.7) | 97.2 (92.4–108.4) | 0.88 | 42.3 |
| `sphere` D2 10^5 | 94.6 (91.0–100.7) | 86.2 (84.7–103.3) | 0.91 | 32.3 |
| `cube` D2 10^6 | 1209 (1175–1254) | 1094 (1074–1126) | 0.91 | 465 |
| `sphere` D2 10^6 | 866 (840–901) | 799 (781–855) | 0.92 | 318 |

**The rule gives local improvements**: 0.88 to 0.92, not 0.5. A site costs about 20 predicate calls in both, 9.0 in-sphere tests and 11.2 orientations on `cube` (4.0 and 12.8 on `sphere`). The walk takes 5.4 steps and the cavity has 4.0 triangles (4.4 and 3.0).

### The subnormal product of the first stage

One in-sphere test of the first stage (`semi_static::lifted2`), timed alone on random quadruples of `cube` D2 10^5 points, took 40.4 ns. The same formula without the term `4 η X` of its bound took 6.4 ns. That term multiplies `4 η = 2^-1072`, a subnormal, by `X ≥ 1`; the product alone took 28.1 ns per call. An addition with a subnormal operand cost nothing measurable: 3.0 ns with and without `η` in the update of `filter::Approx`. The other products of `η` (`cull.rs`, `normal.rs`) are made once per plane, not per point.

Every first-stage formula (`Estimate::new`, k ≤ 4) makes this product on every call: the orientations and in-sphere tests of Delaunay D = 2 and D = 3, and the first stage of the hull's orientations up to D = 4.

A sound bound without the product, in the scratch copy:

- Let `s = relative · permanent`.
- When `s ≥ 2^-960` and `X ≤ 2^60`, then `4 η X ≤ 2^-1012 ≤ 2^-52 s`, and `s (1 + 2^-51)`, rounded, is at least `s + 4 η X`.
- Otherwise the bound is computed as before, in a function marked cold and not inlined.

The cold function matters. Written as an `if` in place, the compiler evaluated both branches with a select, and no build was faster.

Insertion with that bound, convx and prototype in the same processes as above:

| Set | convx, new bound / convx | Prototype, new bound / convx |
| --- | ---: | ---: |
| `cube` D2 10^5 | 0.65 | 0.60 |
| `sphere` D2 10^5 | 0.86 | 0.77 |
| `cube` D2 10^6 | 0.70 | 0.64 |
| `sphere` D2 10^6 | 0.86 | 0.80 |

`build()` with the new bound against `main`, alternated, 5 rounds × 3 builds:

| Set | `main` | New bound | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^5 | 150.7 (145.8–160.5) | 115.3 (103.6–118.0) | 0.77 |
| Delaunay `sphere` D2 10^5 | 122.6 (117.0–129.3) | 104.3 (98.6–123.2) | 0.85, inside the spread |
| Delaunay `cube` D3 10^5 | 980.6 (954.7–1000.0) | 709.0 (691.1–746.2) | 0.72 |
| Hull `sphere` D3 10^5 | 402.9 (391.1–442.7) | 395.8 (386.7–411.4) | 0.98, inside the spread |
| Hull `sphere` D4 10^4 | 148.2 (143.8–158.2) | 149.1 (138.9–157.5) | 1.01, inside the spread |
| Hull `cube` D3 10^6 | 157.3 (148.6–165.0) | 149.3 (142.9–157.3) | 0.95, inside the spread |
| Hull `sphere` D2 10^5 | 22.5 (21.3–26.8) | 22.6 (21.5–25.7) | 1.01, inside the spread |

### Reading

- **The structure is not the gap.** A CGAL-style record per triangle with fan linking reads 0.88 to 0.92 of convx's insertion with the same predicates and order. By the Grill's rule, the next rows remove costs from the current insertion.
- **The largest single cost found is the subnormal product in the first-stage bound.**
  - Removing it soundly makes Delaunay `build()` 0.77 on `cube` D2 10^5 and 0.72 on `cube` D3 10^5, beyond the spread.
  - On `sphere` D2 10^5 it reads 0.85, inside the spread in this run.
  - The hull sets read 0.95 to 1.01, inside the spread. The hull's side tests are mostly decided by the cull plane's scan, which makes no such product per point.
- **Even with the new bound, the gap to CGAL stays large.** convx's insertion of `cube` D2 10^5 comes to about 72 ms against CGAL's 42 ms for the whole construction (different operating systems). The next profile decides the rows after the bound.

## A first-stage bound with no subnormal product, PR #319 (#316)

P7-4. The first stage (`semi_static`, k ≤ 4) computes its bound as `s (1 + 2^-50)` when `s = (n + 1) u P̂ ≥ 2^-960` and `X ≤ 2^60`, and as before otherwise, out of line and cold. The section of #311 above found the product `4 η X` with the subnormal `4 η` to cost about 28 ns per call.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `6612a87` against the head `a439094`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `ConvexHullBuilder::new(dim, &pts).build()` or `DelaunayBuilder::new(dim, &pts).build()` on every set of ADR 0006, generated with `tests/common/generator.rs`, seed 1 |
| Rounds | `main` and the head alternated per round: 5 rounds × 3 builds, or 3 × 1 for the sets over about 1.5 s |
| Reported | Median with min–max. A ratio is the head's median over `main`'s; "inside the spread" when the two ranges overlap |
| P7 judgement | The parity run of `docs/verification.md` on the head, under WSL2 on the same machine |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.72 ms (0.66 ms–0.80 ms) | 0.71 ms (0.65 ms–2.57 ms) | 0.98, inside the spread |
| Hull `cube` D2 10^5 | 6.30 ms (5.87 ms–6.98 ms) | 6.65 ms (5.85 ms–7.68 ms) | 1.06, inside the spread |
| Hull `cube` D2 10^6 | 73.1 ms (68.7 ms–88.3 ms) | 71.3 ms (68.6 ms–93.4 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^4 | 1.62 ms (1.38 ms–2.64 ms) | 1.61 ms (1.44 ms–2.52 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^5 | 12.2 ms (11.1 ms–13.5 ms) | 12.1 ms (11.1 ms–12.8 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^6 | 144 ms (140 ms–152 ms) | 145 ms (136 ms–157 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 7.23 ms (6.57 ms–8.29 ms) | 7.21 ms (6.62 ms–7.82 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 38.3 ms (36.5 ms–41.2 ms) | 37.5 ms (36.3 ms–46.3 ms) | 0.98, inside the spread |
| Hull `cube` D5 10^4 | 62.8 ms (61.8 ms–66.3 ms) | 62.8 ms (59.2 ms–71.2 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^5 | 240 ms (235 ms–245 ms) | 239 ms (234 ms–252 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 645 ms (616 ms–672 ms) | 643 ms (614 ms–673 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^5 | 2.95 s (2.95 s–2.95 s) | 2.96 s (2.94 s–2.97 s) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 2.07 ms (2.02 ms–2.81 ms) | 2.10 ms (1.99 ms–2.70 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^5 | 21.5 ms (20.5 ms–24.2 ms) | 21.2 ms (20.2 ms–23.4 ms) | 0.98, inside the spread |
| Hull `sphere` D2 10^6 | 289 ms (276 ms–330 ms) | 293 ms (270 ms–320 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^4 | 31.2 ms (30.2 ms–33.9 ms) | 31.2 ms (29.7 ms–32.9 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^5 | 369 ms (360 ms–381 ms) | 370 ms (360 ms–382 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^6 | 5.69 s (5.68 s–5.75 s) | 5.70 s (5.69 s–5.71 s) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 137 ms (135 ms–175 ms) | 139 ms (133 ms–144 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.78 s (1.78 s–1.81 s) | 1.81 s (1.80 s–1.83 s) | 1.02, inside the spread |
| Hull `sphere` D5 10^4 | 914 ms (892 ms–997 ms) | 907 ms (890 ms–980 ms) | 0.99, inside the spread |
| Hull `sphere` D5 10^5 | 12.11 s (12.04 s–12.14 s) | 12.07 s (11.73 s–12.18 s) | 1.00, inside the spread |
| Hull `sphere` D6 10^4 | 7.21 s (7.10 s–7.29 s) | 7.25 s (7.16 s–7.30 s) | 1.01, inside the spread |
| Hull `cubesurf` D3 10^5 | 188 ms (185 ms–192 ms) | 176 ms (170 ms–183 ms) | 0.94 |
| Hull `grid` D6 10^4 | 3.83 s (3.69 s–3.85 s) | 3.85 s (3.76 s–3.85 s) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 14.2 ms (12.9 ms–15.3 ms) | 10.3 ms (9.67 ms–12.4 ms) | 0.73 |
| Delaunay `cube` D2 10^5 | 152 ms (146 ms–159 ms) | 114 ms (108 ms–124 ms) | 0.75 |
| Delaunay `cube` D2 10^6 | 1.83 s (1.81 s–1.85 s) | 1.46 s (1.42 s–1.48 s) | 0.80 |
| Delaunay `cube` D3 10^4 | 85.6 ms (82.9 ms–91.3 ms) | 58.3 ms (55.8 ms–75.6 ms) | 0.68 |
| Delaunay `cube` D3 10^5 | 958 ms (937 ms–975 ms) | 690 ms (680 ms–727 ms) | 0.72 |
| Delaunay `cube` D3 10^6 | 10.89 s (10.78 s–11.16 s) | 8.22 s (8.18 s–8.31 s) | 0.75 |
| Delaunay `cube` D4 10^4 | 998 ms (984 ms–1.04 s) | 990 ms (969 ms–1.01 s) | 0.99, inside the spread |
| Delaunay `cube` D5 10^4 | 8.93 s (8.87 s–8.95 s) | 8.84 s (8.78 s–8.88 s) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^4 | 16.4 ms (15.5 ms–17.4 ms) | 14.1 ms (12.9 ms–16.3 ms) | 0.86, inside the spread |
| Delaunay `sphere` D2 10^5 | 120 ms (118 ms–145 ms) | 102 ms (98.1 ms–105 ms) | 0.85 |
| Delaunay `sphere` D2 10^6 | 1.22 s (1.20 s–1.23 s) | 1.05 s (1.03 s–1.16 s) | 0.87 |
| Delaunay `sphere` D3 10^4 | 300 ms (294 ms–312 ms) | 276 ms (269 ms–298 ms) | 0.92, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.80 s (2.73 s–2.82 s) | 2.50 s (2.49 s–2.52 s) | 0.90 |
| Delaunay `sphere` D3 10^6 | 22.05 s (21.65 s–22.14 s) | 19.59 s (19.44 s–19.70 s) | 0.89 |
| Delaunay `sphere` D4 10^4 | 4.26 s (4.25 s–4.33 s) | 4.27 s (4.25 s–4.28 s) | 1.00, inside the spread |
| Delaunay `sphere` D5 10^4 | 49.73 s (49.55 s–52.25 s) | 51.85 s (50.02 s–52.12 s) | 1.04, inside the spread |

### P7 judgement of the head

Met: 9 of 41, as in the baseline. Ratios of `build()`; "(ref.)" marks a reference that is not judged (ADR 0006).

| Set | convx `build()` | / CGAL | / Qhull | P7 |
| --- | ---: | ---: | ---: | --- |
| Hull `cube` D2 10^4 | 0.62 ms | 1.19 | 0.99 | not met |
| Hull `cube` D2 10^5 | 5.44 ms | 1.23 | 0.97 | not met |
| Hull `cube` D2 10^6 | 69.8 ms | 1.56 | 1.22 | not met |
| Hull `cube` D3 10^4 | 1.32 ms | 0.60 | 0.89 | met |
| Hull `cube` D3 10^5 | 10.4 ms | 0.42 | 0.89 | met |
| Hull `cube` D3 10^6 | 141 ms | 0.25 | 0.98 | met |
| Hull `cube` D4 10^4 | 6.43 ms | 0.15 | 0.96 | met |
| Hull `cube` D4 10^5 | 34.3 ms | 0.06 | 1.02 | not met |
| Hull `cube` D5 10^4 | 56.7 ms | 0.25 | 0.78 | met |
| Hull `cube` D5 10^5 | 221 ms | 0.13 | 0.69 | met |
| Hull `cube` D6 10^4 | 626 ms | 0.30 | 0.49 | met |
| Hull `cube` D6 10^5 | 2.72 s | 0.25 | 0.42 | met |
| Hull `sphere` D2 10^4 | 1.68 ms | 2.70 | 0.25 (ref.) | not met |
| Hull `sphere` D2 10^5 | 19.6 ms | 2.58 | 0.18 (ref.) | not met |
| Hull `sphere` D2 10^6 | 286 ms | 3.28 | 0.15 (ref.) | not met |
| Hull `sphere` D3 10^4 | 28.4 ms | 1.27 | 1.51 | not met |
| Hull `sphere` D3 10^5 | 354 ms | 0.89 | 1.44 | not met |
| Hull `sphere` D3 10^6 | 5.35 s | 0.67 | 1.62 | not met |
| Hull `sphere` D4 10^4 | 134 ms | 1.09 | 1.35 | not met |
| Hull `sphere` D4 10^5 | 1.70 s | 1.32 | 1.29 | not met |
| Hull `sphere` D5 10^4 | 870 ms | 0.91 | 1.05 | not met |
| Hull `sphere` D5 10^5 | 11.63 s | 1.05 | 1.09 | not met |
| Hull `sphere` D6 10^4 | 7.43 s | 0.79 | 0.90 | met |
| Hull `cubesurf` D3 10^5 | 171 ms | 4.39 | 5.92 | not met |
| Hull `grid` D6 10^4 | 3.68 s | 14.48 (ref.) | 10.66 | not met |
| Delaunay `cube` D2 10^4 | 9.03 ms | 2.23 | 0.49 | not met |
| Delaunay `cube` D2 10^5 | 102 ms | 2.38 | 0.43 | not met |
| Delaunay `cube` D2 10^6 | 1.33 s | 2.88 | 0.41 | not met |
| Delaunay `sphere` D2 10^4 | 12.6 ms | 3.19 | 0.54 (ref.) | not met |
| Delaunay `sphere` D2 10^5 | 92.5 ms | 2.91 | 0.25 (ref.) | not met |
| Delaunay `sphere` D2 10^6 | 972 ms | 3.05 | 0.08 (ref.) | not met |
| Delaunay `cube` D3 10^4 | 57.4 ms | 1.95 | 0.62 | not met |
| Delaunay `cube` D3 10^5 | 681 ms | 2.17 | 0.53 | not met |
| Delaunay `cube` D3 10^6 | 8.22 s | 2.40 | 0.52 | not met |
| Delaunay `sphere` D3 10^4 | 241 ms | 1.84 | 2.01 (ref.) | not met |
| Delaunay `sphere` D3 10^5 | 2.23 s | 2.40 | 1.15 (ref.) | not met |
| Delaunay `sphere` D3 10^6 | 17.88 s | 2.76 | 1.04 (ref.) | not met |
| Delaunay `cube` D4 10^4 | 872 ms | 1.39 | 1.09 | not met |
| Delaunay `sphere` D4 10^4 | 3.81 s | 1.52 | 2.99 (ref.) | not met |
| Delaunay `cube` D5 10^4 | 7.92 s | 1.17 | 0.99 | not met |
| Delaunay `sphere` D5 10^4 | 48.61 s | 1.38 | 3.29 (ref.) | not met |

### Reading

- **Keep criterion met.** Delaunay `cube` D2 reads 0.75 at 10^5 and 0.80 at 10^6, and `cube` D3 reads 0.72 at 10^5 and 0.75 at 10^6, all beyond the spread. No set is slower beyond the spread.
- Delaunay `sphere` D2 reads 0.85 to 0.87 and `sphere` D3 0.89 to 0.92, beyond the spread except at 10^4. Delaunay D4 and D5, whose predicates do not use the first stage, read 0.99 to 1.04, inside the spread.
- The hull sets read 0.98 to 1.06, inside the spread, except `cubesurf` D3 10^5 at 0.94, beyond it. The hull decides most side tests by the cull plane's scan, which makes no such product per point.
- **Against CGAL, Delaunay D2 and D3 now read 1.84 to 3.19**, from 1.97 to 3.81 in the baseline. No set changes its P7 judgement. P7-5 (#317) profiles what remains.

## Profile of Delaunay D2 and D3 after P7-4, PR #333 (#317)

P7-5. Where Delaunay `build()` spends its time once the first-stage bound makes no subnormal product (#319), and which rows follow.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `980e394`, which holds P7-4. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Profile | Intel VTune hotspots, software sampling, on `DelaunayBuilder::new(dim, &pts).build()` repeated (30 builds at D2, 5 at D3), sets generated with `tests/common/generator.rs`, seed 1. Inclusive shares of `build()`; each row assigned from its callers in the call tree |
| Phase timers | A scratch copy of the same commit with `Instant` marks around each phase and counters per site, not committed. Each insertion takes six marks. On Windows they make the build about 20% slower (`cube` D2 10^5: 136 ms against about 114 ms), so the times below read as shares, not as costs |

### Shares of `build()` (profile)

| Phase | `cube` D2 10^5 | `sphere` D2 10^5 | `cube` D3 10^5 |
| --- | ---: | ---: | ---: |
| Insertion (`Mesh::build`) | 58.3% | 69.4% | 66.4% |
| — conflict tests (in-sphere) | 19.1% | 29.1% | 21.4% |
| — of which the exact stage (`Sites::lifted`) | below 2% | 17.9% | below 2% |
| — point location (`locate`) | 16.4% | 20.9% | 5.2% |
| — allocation of simplices | 2.8% | 3.7% | 4.7% |
| — the rest of `insert` (cavity search, new simplices, linking) | about 20% | about 15% | about 35% |
| Public order of the simplices (`Draft::finish`, its sort) | 14.3% (9.5%) | 8.7% (6.0%) | 12.7% (8.0%) |
| Ascending vertex lists of the draft (`ascending`) | 3.4% | below 2% | 4.3% |
| Insertion order (`brio`) | 5.0% | 7.9% | below 2% |
| Acceptance | 2.9% | below 2% | below 2% |

### Phase timers and counts per site

Times in ms per build, from the instrumented copy:

| Phase | `cube` D2 10^5 | `sphere` D2 10^5 | `cube` D3 10^5 | `cube` D2 10^6 |
| --- | ---: | ---: | ---: | ---: |
| Acceptance, sites, flat test | 2.3 | 2.3 | 3.1 | 32.2 |
| Insertion order | 8.4 | 8.5 | 9.0 | 83.6 |
| Point location | 26.6 | 27.3 | 54.4 | 335 |
| Cavity search | 27.9 | 41.0 | 238 | 319 |
| New simplices and their links | 23.6 | 17.1 | 196 | 236 |
| Cospherical groups | 6.4 | 3.8 | 58.2 | 108 |
| Draft of the single simplices | 11.8 | 8.8 | 63.5 | 139 |
| Merged groups | 0.2 | 0.3 | 0.9 | 2.2 |
| Face pairing of the draft | 0.9 | 0.5 | 2.6 | 6.0 |
| Public order and numbering | 15.3 | 7.2 | 85.6 | 288 |
| Publication | 1.0 | 0.6 | 3.8 | 9.6 |

| Per site | `cube` D2 | `sphere` D2 | `cube` D3 |
| --- | ---: | ---: | ---: |
| In-sphere tests | 9.03 | 3.96 | 45.9 |
| Of which reach the exact stage | 0.0000 | 0.355 | 0.0000 |
| Orientations (walk and outside simplices) | 11.3 | 12.8 | 20.2 |
| Walk steps | 5.46 | 4.41 | 8.76 |
| Simplices in the cavity | 4.02 | 2.98 | 20.1 |

### Reading

- **Insertion is 58 to 69% of `build()`; the predicates are about half of it.** The rest of `insert` is the cavity search, the new simplices, and their links. At D3 that rest is about 35% of `build()`: the instrumented copy spends 196 ms on new simplices and links against 238 ms on the cavity search.
- **The public order of the simplices costs 9 to 14%.** It is a lexicographic sort of every simplex, which CGAL does not do. The draft's ascending vertex lists add 3 to 4%. Together with the cospherical groups and the draft, the pass after insertion is 25% (`cube` D2) and 29% (`cube` D3) of the instrumented time.
- **On `sphere` D2 the exact stage is 17.9% of `build()`.** Its sites are near one circle, and 9% of the in-sphere tests reach the exact stage, against none on `cube`.
- **Point location is 16 to 21% at D2,** at 4.4 to 5.5 steps a site with about two orientations a step. It is 5% at D3.
- What the shares do not show: how much each row would remove. The instrumented times include the marks, and a share is not a saving.

### Rows that follow, in the order of the profile

| Row | Kind | What |
| --- | --- | --- |
| P7-10 | Feat | Delaunay: the new simplices of a cavity linked without turning around ridges, and the cavity search (the rest of `insert`, about 20 to 35%) |
| P7-11 | Docs | Delaunay: the public order of the simplices on first use (9 to 14%, with the draft's ascending lists 3 to 4%), set after a Grill |
| P7-12 | Spike | Delaunay `sphere` D2: why 9% of the in-sphere tests reach the exact stage, and what a stage between would decide (17.9%) |
| P7-13 | Spike | Delaunay D2: insertion order and point location against CGAL's (16 to 21%, with the order 5 to 8%) |

## D2 facets in cycle order and the facet numbering on first use, PR #337 (#321)

P7-6. For D = 2 classification writes the facets, the neighbors, and the boundary simplices in one pass over the boundary cycle, and publication sorts nothing. For D = 1 and D >= 3 the hull keeps the facets as classification leaves them, and the first call that reads a facet number orders them (design §5, #312).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `00e4423` against the head `05c030a`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, on every set of ADR 0006, generated with `tests/common/generator.rs`, seed 1. The counts are read after the timer stops, so the head's numbering on first use is not in the timed section, as ADR 0006 judges `build()` |
| Rounds | `main` and the head alternated per round: 5 rounds × 3 builds, or 3 × 1 for the sets over about 1.5 s; a second run of 10 rounds × 3 on hull `sphere` D3 |
| P7 judgement | The parity run of `docs/verification.md` on the head, under WSL2 on the same machine. Its `planes()` column now includes the numbering for D >= 3 |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.71 ms (0.65 ms–1.15 ms) | 0.68 ms (0.61 ms–0.80 ms) | 0.95, inside the spread |
| Hull `cube` D2 10^5 | 6.63 ms (5.91 ms–7.23 ms) | 6.42 ms (5.67 ms–8.01 ms) | 0.97, inside the spread |
| Hull `cube` D2 10^6 | 72.9 ms (69.2 ms–90.1 ms) | 75.6 ms (70.4 ms–79.5 ms) | 1.04, inside the spread |
| Hull `cube` D3 10^4 | 1.51 ms (1.39 ms–1.91 ms) | 1.57 ms (1.37 ms–2.03 ms) | 1.03, inside the spread |
| Hull `cube` D3 10^5 | 12.7 ms (11.9 ms–13.5 ms) | 12.3 ms (11.6 ms–13.7 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^6 | 149 ms (143 ms–163 ms) | 144 ms (142 ms–150 ms) | 0.97, inside the spread |
| Hull `cube` D4 10^4 | 7.11 ms (6.56 ms–8.43 ms) | 7.15 ms (6.36 ms–9.26 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 37.9 ms (36.6 ms–41.9 ms) | 38.6 ms (36.9 ms–40.9 ms) | 1.02, inside the spread |
| Hull `cube` D5 10^4 | 62.7 ms (61.0 ms–65.9 ms) | 60.5 ms (58.1 ms–69.9 ms) | 0.97, inside the spread |
| Hull `cube` D5 10^5 | 242 ms (235 ms–300 ms) | 235 ms (228 ms–243 ms) | 0.97, inside the spread |
| Hull `cube` D6 10^4 | 663 ms (644 ms–686 ms) | 618 ms (611 ms–635 ms) | 0.93 |
| Hull `cube` D6 10^5 | 2.94 s (2.92 s–2.95 s) | 2.82 s (2.81 s–2.85 s) | 0.96 |
| Hull `sphere` D2 10^4 | 2.09 ms (1.91 ms–2.95 ms) | 1.59 ms (1.49 ms–2.62 ms) | 0.76, inside the spread |
| Hull `sphere` D2 10^5 | 21.8 ms (20.4 ms–25.1 ms) | 18.3 ms (17.1 ms–19.3 ms) | 0.84 |
| Hull `sphere` D2 10^6 | 295 ms (288 ms–328 ms) | 237 ms (231 ms–246 ms) | 0.80 |
| Hull `sphere` D3 10^4 | 32.1 ms (31.1 ms–34.3 ms) | 30.7 ms (28.6 ms–32.4 ms) | 0.96, inside the spread |
| Hull `sphere` D3 10^5 | 383 ms (376 ms–397 ms) | 366 ms (355 ms–377 ms) | 0.95, inside the spread |
| Hull `sphere` D3 10^6 | 5.62 s (5.59 s–5.62 s) | 5.25 s (5.23 s–5.31 s) | 0.93 |
| Hull `sphere` D4 10^4 | 145 ms (140 ms–155 ms) | 136 ms (133 ms–161 ms) | 0.94, inside the spread |
| Hull `sphere` D4 10^5 | 1.78 s (1.78 s–1.81 s) | 1.66 s (1.64 s–1.67 s) | 0.93 |
| Hull `sphere` D5 10^4 | 949 ms (936 ms–966 ms) | 900 ms (886 ms–912 ms) | 0.95 |
| Hull `sphere` D5 10^5 | 12.29 s (12.19 s–12.29 s) | 11.30 s (11.13 s–11.60 s) | 0.92 |
| Hull `sphere` D6 10^4 | 7.55 s (7.50 s–7.57 s) | 7.07 s (7.03 s–7.10 s) | 0.94 |
| Hull `cubesurf` D3 10^5 | 173 ms (169 ms–178 ms) | 173 ms (171 ms–178 ms) | 1.00, inside the spread |
| Hull `grid` D6 10^4 | 3.82 s (3.80 s–3.87 s) | 3.81 s (3.80 s–3.82 s) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 10.1 ms (9.40 ms–10.8 ms) | 10.7 ms (9.21 ms–13.1 ms) | 1.05, inside the spread |
| Delaunay `cube` D2 10^5 | 109 ms (106 ms–124 ms) | 111 ms (109 ms–114 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^6 | 1.40 s (1.37 s–1.43 s) | 1.40 s (1.38 s–1.43 s) | 1.00, inside the spread |
| Delaunay `cube` D3 10^4 | 55.4 ms (53.2 ms–58.5 ms) | 56.2 ms (53.3 ms–66.7 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^5 | 658 ms (644 ms–692 ms) | 653 ms (636 ms–698 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^6 | 7.97 s (7.96 s–8.02 s) | 7.92 s (7.87 s–8.00 s) | 0.99, inside the spread |
| Delaunay `cube` D4 10^4 | 977 ms (923 ms–1.02 s) | 984 ms (915 ms–1.02 s) | 1.01, inside the spread |
| Delaunay `cube` D5 10^4 | 9.13 s (8.93 s–9.21 s) | 9.06 s (8.96 s–9.08 s) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^4 | 13.8 ms (13.4 ms–14.5 ms) | 13.9 ms (13.0 ms–15.7 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^5 | 102 ms (94.5 ms–127 ms) | 101 ms (95.2 ms–117 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^6 | 1.02 s (988 ms–1.12 s) | 1.02 s (987 ms–1.17 s) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 266 ms (261 ms–284 ms) | 271 ms (261 ms–288 ms) | 1.02, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.46 s (2.46 s–2.52 s) | 2.48 s (2.45 s–2.50 s) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^6 | 19.74 s (18.98 s–19.90 s) | 19.88 s (19.12 s–20.62 s) | 1.01, inside the spread |
| Delaunay `sphere` D4 10^4 | 4.38 s (4.33 s–4.42 s) | 4.35 s (4.30 s–4.48 s) | 0.99, inside the spread |
| Delaunay `sphere` D5 10^4 | 50.81 s (50.23 s–53.42 s) | 52.08 s (50.76 s–53.03 s) | 1.02, inside the spread |

The second run, 10 rounds × 3:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `sphere` D3 10^4 | 32.6 ms (30.5 ms–38.4 ms) | 31.1 ms (28.9 ms–36.1 ms) | 0.95, inside the spread |
| Hull `sphere` D3 10^5 | 397 ms (375 ms–441 ms) | 374 ms (357 ms–396 ms) | 0.94, inside the spread |

### P7 judgement of the head

Met: 9 of 41, as before. Ratios of `build()`; "(ref.)" marks a reference that is not judged (ADR 0006).

| Set | convx `build()` | / CGAL | / Qhull | P7 |
| --- | ---: | ---: | ---: | --- |
| Hull `cube` D2 10^4 | 0.56 ms | 1.13 | 0.98 | not met |
| Hull `cube` D2 10^5 | 5.74 ms | 1.27 | 0.97 | not met |
| Hull `cube` D2 10^6 | 73.2 ms | 1.53 | 1.22 | not met |
| Hull `cube` D3 10^4 | 1.43 ms | 0.61 | 0.87 | met |
| Hull `cube` D3 10^5 | 10.7 ms | 0.40 | 0.90 | met |
| Hull `cube` D3 10^6 | 144 ms | 0.25 | 0.96 | met |
| Hull `cube` D4 10^4 | 6.92 ms | 0.15 | 0.95 | met |
| Hull `cube` D4 10^5 | 35.6 ms | 0.06 | 1.03 | not met |
| Hull `cube` D5 10^4 | 56.2 ms | 0.24 | 0.75 | met |
| Hull `cube` D5 10^5 | 216 ms | 0.13 | 0.67 | met |
| Hull `cube` D6 10^4 | 599 ms | 0.29 | 0.48 | met |
| Hull `cube` D6 10^5 | 2.59 s | 0.24 | 0.39 | met |
| Hull `sphere` D2 10^4 | 1.32 ms | 1.98 | 0.20 (ref.) | not met |
| Hull `sphere` D2 10^5 | 15.8 ms | 2.02 | 0.14 (ref.) | not met |
| Hull `sphere` D2 10^6 | 226 ms | 2.58 | 0.12 (ref.) | not met |
| Hull `sphere` D3 10^4 | 27.9 ms | 1.20 | 1.50 | not met |
| Hull `sphere` D3 10^5 | 343 ms | 0.85 | 1.38 | not met |
| Hull `sphere` D3 10^6 | 5.04 s | 0.62 | 1.49 | not met |
| Hull `sphere` D4 10^4 | 127 ms | 1.05 | 1.29 | not met |
| Hull `sphere` D4 10^5 | 1.63 s | 1.27 | 1.21 | not met |
| Hull `sphere` D5 10^4 | 849 ms | 0.87 | 1.03 | not met |
| Hull `sphere` D5 10^5 | 10.80 s | 0.97 | 1.01 | not met |
| Hull `sphere` D6 10^4 | 7.05 s | 0.74 | 0.84 | met |
| Hull `cubesurf` D3 10^5 | 171 ms | 4.45 | 5.82 | not met |
| Hull `grid` D6 10^4 | 3.67 s | 14.16 (ref.) | 10.67 | not met |
| Delaunay `cube` D2 10^4 | 9.28 ms | 2.24 | 0.50 | not met |
| Delaunay `cube` D2 10^5 | 104 ms | 2.43 | 0.44 | not met |
| Delaunay `cube` D2 10^6 | 1.36 s | 2.92 | 0.41 | not met |
| Delaunay `sphere` D2 10^4 | 13.0 ms | 3.31 | 0.56 (ref.) | not met |
| Delaunay `sphere` D2 10^5 | 94.8 ms | 3.00 | 0.25 (ref.) | not met |
| Delaunay `sphere` D2 10^6 | 976 ms | 3.08 | 0.08 (ref.) | not met |
| Delaunay `cube` D3 10^4 | 57.7 ms | 1.96 | 0.62 | not met |
| Delaunay `cube` D3 10^5 | 682 ms | 2.20 | 0.54 | not met |
| Delaunay `cube` D3 10^6 | 8.01 s | 2.34 | 0.51 | not met |
| Delaunay `sphere` D3 10^4 | 231 ms | 1.81 | 2.05 (ref.) | not met |
| Delaunay `sphere` D3 10^5 | 2.23 s | 2.42 | 1.18 (ref.) | not met |
| Delaunay `sphere` D3 10^6 | 17.66 s | 2.76 | 1.02 (ref.) | not met |
| Delaunay `cube` D4 10^4 | 884 ms | 1.40 | 1.10 | not met |
| Delaunay `sphere` D4 10^4 | 3.83 s | 1.51 | 3.00 (ref.) | not met |
| Delaunay `cube` D5 10^4 | 8.01 s | 1.17 | 0.99 | not met |
| Delaunay `sphere` D5 10^4 | 47.63 s | 1.35 | 3.25 (ref.) | not met |

### Reading

- **Hull `sphere` D2 reads 0.84 at 10^5 and 0.80 at 10^6, beyond the spread.** Against CGAL it now reads 1.98 to 2.58, against 2.53 to 3.25 before (section of #319).
- **The keep criterion of #321 is not met on hull `sphere` D3 10^5.** It reads 0.95 in the first run and 0.94 in a second run of ten rounds, both inside the spread. Its numbering on first use saves the sort of about 200,000 facets, a few percent of a build that construction dominates. Hull `sphere` D3 10^6 reads 0.93, D4 10^5 0.93, D5 0.92 to 0.95, and D6 0.94, all beyond the spread, as are `cube` D6 at 0.93 and 0.96.
- No set is slower beyond the spread. The Delaunay sets, whose path this does not change, read 0.99 to 1.05, inside the spread.
- No set changes its P7 judgement. Hull `sphere` D5 now reads 1.01 to 1.03 against Qhull.

## Delaunay D3: new tetrahedra linked by face keys, PR #343 (#329)

P7-10, first part. At D = 3 the faces through the new site of the new tetrahedra are paired by their two other vertices, in an open-addressing table, instead of by a turn around each ridge through the cavity. D = 2 keeps the turn; a fan of the new triangles was tried and is not shipped (below). Timed by the shorter method of #341.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `82ed620` against the head `207bf41`, and two earlier commits of this branch. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1. The sets: those the keep criterion of #329 names (`cube` D2 and D3 at 10^5 and 10^6), the Delaunay path at 10^4 and 10^5, and the hull guard sets (`docs/verification.md`, Shorter runs of P7) |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds, or 5 × 1 for the sets over about 1.5 s |
| Before the change | A scratch copy of `main` with timers around the two passes of `Mesh::insert`: on `cube` D3 10^5 the turns took 154 ms of a 760 ms build at 47 turn steps a site, and on `cube` D2 10^5 14 ms of 133 ms (#329) |

Every set published the same counts on both sides.

### The head (`207bf41`)

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 10.5 ms (9.48 ms–12.4 ms) | 10.5 ms (9.41 ms–12.7 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 114 ms (107 ms–129 ms) | 112 ms (107 ms–126 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^4 | 14.5 ms (13.5 ms–16.3 ms) | 14.4 ms (13.3 ms–15.6 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^5 | 103 ms (97.9 ms–110 ms) | 104 ms (96.6 ms–108 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 58.6 ms (55.0 ms–70.2 ms) | 53.9 ms (52.0 ms–59.9 ms) | 0.92, inside the spread |
| Delaunay `cube` D3 10^5 | 694 ms (678 ms–725 ms) | 650 ms (623 ms–709 ms) | 0.94, inside the spread |
| Delaunay `sphere` D3 10^4 | 275 ms (269 ms–289 ms) | 277 ms (268 ms–296 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^6 | 1.46 s (1.42 s–1.51 s) | 1.45 s (1.42 s–1.48 s) | 1.00, inside the spread |
| Hull `sphere` D3 10^5 | 366 ms (352 ms–393 ms) | 363 ms (348 ms–389 ms) | 0.99, inside the spread |
| Hull `cube` D3 10^5 | 11.8 ms (11.2 ms–13.3 ms) | 11.7 ms (11.1 ms–13.8 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.52 s (2.51 s–2.60 s) | 2.46 s (2.44 s–2.57 s) | 0.98, inside the spread |
| Delaunay `cube` D3 10^6 | 7.89 s (7.77 s–8.11 s) | 7.41 s (7.28 s–7.74 s) | 0.94 |

### Earlier commits of this branch

`9e71305` linked D2 and D3 by keys and cleared the whole table at every insertion:

| Set | `main` | That commit | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 11.1 ms (10.4 ms–14.2 ms) | 10.9 ms (10.0 ms–12.5 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^5 | 121 ms (111 ms–134 ms) | 116 ms (107 ms–122 ms) | 0.96, inside the spread |
| Delaunay `sphere` D2 10^4 | 14.2 ms (13.0 ms–15.6 ms) | 14.6 ms (13.3 ms–15.8 ms) | 1.03, inside the spread |
| Delaunay `sphere` D2 10^5 | 103 ms (98.9 ms–111 ms) | 102 ms (99.0 ms–121 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^4 | 58.3 ms (55.5 ms–64.3 ms) | 52.9 ms (50.6 ms–59.4 ms) | 0.91, inside the spread |
| Delaunay `cube` D3 10^5 | 680 ms (667 ms–728 ms) | 627 ms (605 ms–701 ms) | 0.92, inside the spread |
| Delaunay `sphere` D3 10^4 | 271 ms (266 ms–286 ms) | 266 ms (260 ms–298 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^6 | 1.49 s (1.30 s–2.10 s) | 1.46 s (1.28 s–1.57 s) | 0.98, inside the spread |
| Hull `sphere` D3 10^5 | 335 ms (324 ms–358 ms) | 332 ms (323 ms–349 ms) | 0.99, inside the spread |
| Hull `cube` D3 10^5 | 11.4 ms (11.0 ms–12.3 ms) | 11.5 ms (10.9 ms–12.9 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.29 s (2.26 s–2.38 s) | 2.24 s (2.21 s–2.27 s) | 0.98, inside the spread |
| Delaunay `cube` D3 10^6 | 7.31 s (7.31 s–7.37 s) | 6.82 s (6.79 s–6.86 s) | 0.93 |

A commit between them (not kept) freed only the used positions and linked D2 as a fan, with one slot per site for the triangle whose boundary edge starts there:

| Set | `main` | That commit | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 10.6 ms (9.45 ms–11.5 ms) | 9.88 ms (9.01 ms–11.9 ms) | 0.93, inside the spread |
| Delaunay `cube` D2 10^5 | 114 ms (107 ms–126 ms) | 107 ms (101 ms–117 ms) | 0.94, inside the spread |
| Delaunay `sphere` D2 10^4 | 14.3 ms (13.1 ms–17.2 ms) | 14.1 ms (13.1 ms–19.6 ms) | 0.98, inside the spread |
| Delaunay `sphere` D2 10^5 | 104 ms (97.5 ms–115 ms) | 103 ms (97.1 ms–111 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^4 | 58.4 ms (56.7 ms–65.5 ms) | 55.5 ms (51.2 ms–60.7 ms) | 0.95, inside the spread |
| Delaunay `cube` D3 10^5 | 706 ms (683 ms–732 ms) | 646 ms (632 ms–684 ms) | 0.92, inside the spread |
| Delaunay `sphere` D3 10^4 | 278 ms (271 ms–290 ms) | 274 ms (266 ms–284 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^6 | 1.46 s (1.44 s–1.52 s) | 1.41 s (1.38 s–1.49 s) | 0.96, inside the spread |
| Hull `sphere` D3 10^5 | 370 ms (361 ms–385 ms) | 372 ms (363 ms–391 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^5 | 12.4 ms (11.3 ms–13.8 ms) | 12.4 ms (11.5 ms–13.9 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.50 s (2.46 s–2.58 s) | 2.45 s (2.44 s–2.56 s) | 0.98, inside the spread |
| Delaunay `cube` D3 10^6 | 8.05 s (7.74 s–8.14 s) | 7.47 s (7.24 s–7.61 s) | 0.93 |

### Reading

- **`cube` D3 10^6 reads 0.93 and 0.94, beyond the spread.** `cube` D3 10^5 reads 0.92, 0.95, 0.92, and 0.94 in four runs, always inside the spread through one slow build. The turns were about 20% of that build, and the key table recovers about a third of that.
- **The D2 fan is not shipped.** It read 0.93 to 0.96 on `cube` D2 in three runs, inside the spread every time, and `bench.mdc` reverts a gain inside the spread. The turns were about 10% of the D2 build.
- Clearing the whole table at every insertion, in `9e71305`, read the same on `cube` D3 (0.92 and 0.93) as freeing only the used positions; the head frees only those.
- No set is slower beyond the spread. The D2 sets, whose code is `main`'s, read 0.99 to 1.01. The hull guard sets read 0.99.

## Delaunay simplices in construction order, PR #342 (#335)

P7-14. The Delaunay triangulation publishes its simplices in the order the construction leaves, and publication sorts nothing (design §7, #336). Voronoi sorts the cells of each cospherical group itself, as design §8 requires. This row was timed by the full method, which it started before #341 shortened it.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `632ad70` against the head `b5aabf6`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, on every set of ADR 0006, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Rounds | `main` and the head alternated per round: 5 rounds × 3 builds, or 3 × 1 for the sets over about 1.5 s; a second run of 10 rounds × 3 on three Delaunay sets at 10^5 |
| P7 judgement | The parity run of `docs/verification.md` on the head, under WSL2 on the same machine |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.69 ms (0.62 ms–0.98 ms) | 0.67 ms (0.62 ms–0.88 ms) | 0.96, inside the spread |
| Hull `cube` D2 10^5 | 6.12 ms (5.68 ms–7.23 ms) | 6.23 ms (5.63 ms–7.60 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^6 | 73.2 ms (68.4 ms–81.4 ms) | 75.6 ms (71.5 ms–94.9 ms) | 1.03, inside the spread |
| Hull `cube` D3 10^4 | 1.57 ms (1.39 ms–2.10 ms) | 1.58 ms (1.45 ms–1.76 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^5 | 12.6 ms (11.6 ms–13.4 ms) | 12.6 ms (11.5 ms–13.7 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^6 | 153 ms (145 ms–172 ms) | 151 ms (148 ms–202 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^4 | 7.01 ms (6.46 ms–7.72 ms) | 7.13 ms (6.64 ms–8.29 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^5 | 38.0 ms (36.7 ms–43.4 ms) | 38.5 ms (35.4 ms–55.3 ms) | 1.01, inside the spread |
| Hull `cube` D5 10^4 | 60.7 ms (58.8 ms–65.1 ms) | 62.0 ms (59.3 ms–65.1 ms) | 1.02, inside the spread |
| Hull `cube` D5 10^5 | 237 ms (232 ms–249 ms) | 239 ms (234 ms–250 ms) | 1.01, inside the spread |
| Hull `cube` D6 10^4 | 648 ms (637 ms–661 ms) | 637 ms (627 ms–661 ms) | 0.98, inside the spread |
| Hull `cube` D6 10^5 | 2.68 s (2.67 s–2.72 s) | 2.66 s (2.65 s–2.68 s) | 0.99, inside the spread |
| Hull `sphere` D2 10^4 | 1.81 ms (1.50 ms–2.81 ms) | 1.67 ms (1.55 ms–2.77 ms) | 0.92, inside the spread |
| Hull `sphere` D2 10^5 | 18.2 ms (17.3 ms–19.9 ms) | 18.3 ms (17.1 ms–22.1 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^6 | 243 ms (233 ms–249 ms) | 242 ms (235 ms–256 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 31.0 ms (29.7 ms–33.0 ms) | 31.1 ms (29.8 ms–35.2 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^5 | 369 ms (357 ms–383 ms) | 369 ms (359 ms–385 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^6 | 5.00 s (4.97 s–5.03 s) | 4.99 s (4.99 s–5.00 s) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 140 ms (137 ms–170 ms) | 141 ms (136 ms–151 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.70 s (1.69 s–1.70 s) | 1.69 s (1.68 s–1.70 s) | 0.99, inside the spread |
| Hull `sphere` D5 10^4 | 911 ms (894 ms–931 ms) | 910 ms (893 ms–952 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^5 | 11.48 s (11.45 s–11.55 s) | 11.50 s (11.38 s–11.60 s) | 1.00, inside the spread |
| Hull `sphere` D6 10^4 | 7.08 s (7.06 s–7.15 s) | 7.13 s (7.03 s–7.18 s) | 1.01, inside the spread |
| Hull `cubesurf` D3 10^5 | 173 ms (170 ms–181 ms) | 178 ms (171 ms–201 ms) | 1.03, inside the spread |
| Hull `grid` D6 10^4 | 3.83 s (3.82 s–3.94 s) | 3.85 s (3.84 s–3.91 s) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 10.2 ms (9.24 ms–11.3 ms) | 9.27 ms (8.22 ms–11.5 ms) | 0.91, inside the spread |
| Delaunay `cube` D2 10^5 | 112 ms (109 ms–117 ms) | 96.6 ms (91.3 ms–102 ms) | 0.87 |
| Delaunay `cube` D2 10^6 | 1.45 s (1.42 s–1.50 s) | 1.15 s (1.14 s–1.18 s) | 0.80 |
| Delaunay `cube` D3 10^4 | 56.8 ms (53.9 ms–61.8 ms) | 51.6 ms (49.3 ms–55.1 ms) | 0.91, inside the spread |
| Delaunay `cube` D3 10^5 | 681 ms (671 ms–709 ms) | 597 ms (570 ms–694 ms) | 0.88, inside the spread |
| Delaunay `cube` D3 10^6 | 8.10 s (8.09 s–8.18 s) | 6.74 s (6.70 s–6.74 s) | 0.83 |
| Delaunay `cube` D4 10^4 | 955 ms (912 ms–987 ms) | 918 ms (889 ms–940 ms) | 0.96, inside the spread |
| Delaunay `cube` D5 10^4 | 9.28 s (8.88 s–9.44 s) | 8.89 s (8.75 s–9.06 s) | 0.96, inside the spread |
| Delaunay `sphere` D2 10^4 | 14.1 ms (13.2 ms–15.6 ms) | 13.8 ms (12.2 ms–15.3 ms) | 0.98, inside the spread |
| Delaunay `sphere` D2 10^5 | 103 ms (99.0 ms–115 ms) | 93.4 ms (89.3 ms–101 ms) | 0.90, inside the spread |
| Delaunay `sphere` D2 10^6 | 1.04 s (1.02 s–1.07 s) | 925 ms (907 ms–967 ms) | 0.89 |
| Delaunay `sphere` D3 10^4 | 278 ms (265 ms–287 ms) | 267 ms (263 ms–280 ms) | 0.96, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.53 s (2.52 s–2.65 s) | 2.47 s (2.47 s–2.58 s) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^6 | 19.50 s (19.18 s–19.69 s) | 18.93 s (18.16 s–19.03 s) | 0.97 |
| Delaunay `sphere` D4 10^4 | 4.18 s (4.14 s–4.22 s) | 4.24 s (4.16 s–4.29 s) | 1.02, inside the spread |
| Delaunay `sphere` D5 10^4 | 53.06 s (48.90 s–53.76 s) | 52.61 s (48.73 s–59.00 s) | 0.99, inside the spread |

The second run, 10 rounds × 3:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D3 10^5 | 693 ms (659 ms–768 ms) | 604 ms (585 ms–641 ms) | 0.87 |
| Delaunay `sphere` D2 10^5 | 104 ms (94.8 ms–149 ms) | 97.7 ms (88.7 ms–136 ms) | 0.94, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.60 s (2.42 s–2.85 s) | 2.55 s (2.38 s–2.79 s) | 0.98, inside the spread |

### P7 judgement of the head

Met: 11 of 41. Hull `cube` D4 10^5 (0.93 against Qhull) and hull `sphere` D5 10^4 (0.99) are met in this run; neither path changed, so they moved by run-to-run noise. Ratios of `build()`; "(ref.)" marks a reference that is not judged (ADR 0006).

| Set | convx `build()` | / CGAL | / Qhull | P7 |
| --- | ---: | ---: | ---: | --- |
| Hull `cube` D2 10^4 | 0.61 ms | 1.15 | 0.98 | not met |
| Hull `cube` D2 10^5 | 6.47 ms | 1.32 | 1.04 | not met |
| Hull `cube` D2 10^6 | 72.4 ms | 1.48 | 1.20 | not met |
| Hull `cube` D3 10^4 | 1.34 ms | 0.58 | 0.82 | met |
| Hull `cube` D3 10^5 | 11.0 ms | 0.39 | 0.87 | met |
| Hull `cube` D3 10^6 | 152 ms | 0.25 | 0.99 | met |
| Hull `cube` D4 10^4 | 6.49 ms | 0.15 | 0.97 | met |
| Hull `cube` D4 10^5 | 34.6 ms | 0.06 | 0.93 | met |
| Hull `cube` D5 10^4 | 57.8 ms | 0.26 | 0.79 | met |
| Hull `cube` D5 10^5 | 214 ms | 0.13 | 0.67 | met |
| Hull `cube` D6 10^4 | 613 ms | 0.30 | 0.50 | met |
| Hull `cube` D6 10^5 | 2.57 s | 0.24 | 0.40 | met |
| Hull `sphere` D2 10^4 | 1.29 ms | 2.14 | 0.20 (ref.) | not met |
| Hull `sphere` D2 10^5 | 15.1 ms | 2.02 | 0.14 (ref.) | not met |
| Hull `sphere` D2 10^6 | 216 ms | 2.45 | 0.12 (ref.) | not met |
| Hull `sphere` D3 10^4 | 27.6 ms | 1.20 | 1.41 | not met |
| Hull `sphere` D3 10^5 | 330 ms | 0.85 | 1.40 | not met |
| Hull `sphere` D3 10^6 | 4.92 s | 0.63 | 1.47 | not met |
| Hull `sphere` D4 10^4 | 128 ms | 1.02 | 1.33 | not met |
| Hull `sphere` D4 10^5 | 1.63 s | 1.27 | 1.25 | not met |
| Hull `sphere` D5 10^4 | 854 ms | 0.86 | 0.99 | met |
| Hull `sphere` D5 10^5 | 11.42 s | 1.00 | 1.00 | not met |
| Hull `sphere` D6 10^4 | 7.65 s | 0.77 | 0.85 | met |
| Hull `cubesurf` D3 10^5 | 186 ms | 4.52 | 5.95 | not met |
| Hull `grid` D6 10^4 | 3.86 s | 14.26 (ref.) | 10.57 | not met |
| Delaunay `cube` D2 10^4 | 8.70 ms | 2.05 | 0.42 | not met |
| Delaunay `cube` D2 10^5 | 98.8 ms | 2.23 | 0.38 | not met |
| Delaunay `cube` D2 10^6 | 1.14 s | 2.33 | 0.32 | not met |
| Delaunay `sphere` D2 10^4 | 12.4 ms | 3.20 | 0.52 (ref.) | not met |
| Delaunay `sphere` D2 10^5 | 93.7 ms | 2.70 | 0.23 (ref.) | not met |
| Delaunay `sphere` D2 10^6 | 895 ms | 2.73 | 0.07 (ref.) | not met |
| Delaunay `cube` D3 10^4 | 55.0 ms | 1.82 | 0.55 | not met |
| Delaunay `cube` D3 10^5 | 615 ms | 1.88 | 0.47 | not met |
| Delaunay `cube` D3 10^6 | 6.88 s | 1.97 | 0.43 | not met |
| Delaunay `sphere` D3 10^4 | 246 ms | 1.84 | 2.02 (ref.) | not met |
| Delaunay `sphere` D3 10^5 | 2.22 s | 2.38 | 1.14 (ref.) | not met |
| Delaunay `sphere` D3 10^6 | 17.78 s | 2.73 | 0.99 (ref.) | not met |
| Delaunay `cube` D4 10^4 | 857 ms | 1.33 | 1.05 | not met |
| Delaunay `sphere` D4 10^4 | 3.91 s | 1.52 | 2.95 (ref.) | not met |
| Delaunay `cube` D5 10^4 | 8.07 s | 1.15 | 0.94 | not met |
| Delaunay `sphere` D5 10^4 | 48.87 s | 1.37 | 3.23 (ref.) | not met |

### Reading

- **Delaunay `cube` gains beyond the spread**: 0.87 at D2 10^5, 0.80 at D2 10^6, 0.83 at D3 10^6, and 0.88 then 0.87 at D3 10^5. The second run is beyond the spread; the first overlapped through one build of the head.
- **The keep criterion of #335 is not met on `sphere` D2 10^5 and `sphere` D3 10^5.** They read 0.90 and 0.94, and 0.98 twice, inside the spread both times. At 10^6 they read 0.89 and 0.97, beyond the spread. On `sphere` the cells are fewer per site and the in-sphere tests reach the exact stage more often (section of #333), so the sort is a smaller share. By the rule written after #337, a keep criterion names only the sets where the profile predicts a saving beyond the spread; this criterion was written before that rule.
- No set is slower beyond the spread. Delaunay D4 and D5 read 0.96 to 1.02, inside the spread, and every hull set is inside it.
- Against CGAL, Delaunay `cube` D2 now reads 2.05 to 2.33 and `cube` D3 1.82 to 1.97, against 2.24 to 2.92 and 1.96 to 2.34 in the run of #337.

## Which stage decides the in-sphere tests of Delaunay, PR #351 (#331)

The spike P7-12. It asks which stage of the predicates decides each lifted orientation (the in-sphere test), what the tests that reach the exact stage cost, and what a stage between would decide.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `145ce0a`, and a scratch copy of it with counters and `Instant` marks in the predicates and in the D = 2, D = 3 shapes of `delaunay::insert`, not committed. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Sets | Delaunay `sphere` and `cube` at D2 to D5, generated with `tests/common/generator.rs`, seed 1 |
| Inputs that reach the exact stage | Written to a file by the scratch copy: all 35,506 of `sphere` D2 10^5, and the first 250,000 of `sphere` D3 10^5, each with the sign convx returned |
| Stages compared on those inputs, outside the crate | Shewchuk's adaptive `incircle` and `insphere` (the `robust` crate 1.1.0, a port of his predicates); and a double-double evaluation of the lifted determinant on fixed-size rows. The double-double differences `p_i - p_0` are exact (TwoSum), and the lifted column and the expansion are in double-double, with the products split by FMA or by Dekker's method. It certifies when `|det| > 10^-28 · permanent`, a rough bound that is not proved |
| Timed | Per call, over the whole file, the best of several passes |

### Where the in-sphere tests are decided

Per build:

| Set | Lifted orientations | Not decided by the first stage | Decided by the running filter | Exact stage | Exact stage, time / `build()` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `sphere` D2 10^5 | 424,524 | 8.4% | 1 | 35,507, 495 ns each | 17.6 ms / 107 ms |
| `sphere` D2 10^6 | 4,262,662 | 2.1% | 1 | 87,604, 516 ns each | 45.2 ms / 991 ms |
| `cube` D2 10^5 | 984,398 | 0% | — | 0 | — |
| `sphere` D3 10^5 | 2,102,110 | 82.7% | 1 | 1,738,898, 1,039 ns each | 1.81 s / 2.86 s |
| `sphere` D3 10^4 | 206,308 | 93.2% | 1 | 192,225, 1,044 ns each | 201 ms / 307 ms |
| `cube` D3 10^5 | 5,097,001 | 0% | — | 0 | — |
| `sphere` D4 10^4 | 1,177,480 | no first stage at k = 5 | 1 | 1,177,479, 3,231 ns each | 3.80 s / 4.50 s |
| `cube` D4 10^4 | 2,834,200 | no first stage at k = 5 | all, 231 ns each (655 ms) | 0 | — |
| `sphere` D5 10^4 | 7,454,137 | no first stage at k = 6 | 1 | 7,454,136, 6,155 ns each | 45.9 s / 52.3 s |
| `cube` D5 10^4 | 17,583,753 | no first stage at k = 6 | all (5.08 s of 10.0 s) | 0 | — |

On `sphere` D2 and D3, a test the first stage of the D = 2 and D = 3 shapes does not certify goes to `Sites::lifted`. That repeats the first stage, then runs the running filter, which decides nothing on these sets: 304 ms on `sphere` D3 10^5. Only then does it reach the exact stage.

### Stages between, on the inputs that reach the exact stage

| Inputs | convx now, per test reaching it | Shewchuk adaptive (`robust`) | Double-double, FMA | Double-double, Dekker split | Signs |
| --- | ---: | ---: | ---: | ---: | --- |
| `sphere` D2 10^5 (35,506) | about 700 ns | 220 ns | 43 ns | 52 ns | all agree; the double-double bound leaves none open |
| `sphere` D3 10^5 (250,000) | about 1,340 ns | 2,240 ns | 87 ns | 108 ns | all agree; the double-double bound leaves none open |

### Reading

- **Near-cospherical sites send most in-sphere tests to the exact stage**: 83 to 93% on `sphere` D3 and all on `sphere` D4 and D5. The exact stage is 63% of `build()` on `sphere` D3 10^5, 84% on `sphere` D4 10^4, and 88% on `sphere` D5 10^4. These sets read 1.84 to 2.73 against CGAL (#342).
- **A double-double stage would decide every one of them on these inputs**, at 43 to 108 ns against about 700 to 1,340 ns now. Its bound here is rough. A real stage needs a proved bound, as the first stage has (`semi_static`), and the Dekker split keeps it on the baseline target.
- **Shewchuk's adaptive predicates do not help at D3**: 2,240 ns against convx's 1,340 ns, since near-cospherical inputs go through most of their stages. At D2 they take 220 ns.
- **At D4 and D5 there is no first stage**, so every in-sphere test runs the running filter: about half of `build()` on `cube` D4 and D5, at 231 ns a test at D4. The first stage covers determinants of size k ≤ 4 (`semi_static`); the in-sphere tests of D4 and D5 have k = 5 and k = 6, since the lifted orientation of dimension D has size D + 1.
- **The repeated first stage and the running filter** cost 304 ms on `sphere` D3 10^5 without deciding anything.
- What this does not show: the saving of each stage in `build()`. The counters and marks slow the build, and a stage's bound may leave more open than the rough one did.

### Rows that follow

| Row | Kind | What |
| --- | --- | --- |
| P7-15 | Docs | Predicates: a double-double stage, with its proved bound, between the first stage and the exact stage; for which sizes; FMA or a split. Set after a Grill |
| P7-16 | Feat | Predicates: a first stage for determinants of size k = 5 and 6: the lifted orientations of D4 and D5, and the orientations of D5 and D6 |

## A first stage for determinants of size five, PR #354 (#350)

P7-16. `semi_static` gains a first stage for k = 5: the lifted orientation of D4 (the in-sphere test of Delaunay D4) and the plain orientation of D5. The determinant is expanded along its last column over shared minors, and the permanent of the bound is replaced by the product of the column sums (module docs, Size five). Size six stays with the running filter.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `145ce0a` against the head `5a1d6d6`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341): the sets of the keep criterion, the path sets of both paths at 10^4 and 10^5, and the guard sets hull `cube` D5 and D6 10^4 and `sphere` D5 10^4, since the hull's orientations of size five and six change too |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds, or 5 × 1 for hull `sphere` D4 10^5, Delaunay `sphere` D3 10^5, and Delaunay `cube` D5 10^4 |
| Per test | 1,000 random rows in [-1, 1], 500 repetitions, the first stage against the running filter (`filtered_value`), in one process |

Every set published the same counts on both sides.

### Per test

| Size | First stage | Running filter |
| --- | ---: | ---: |
| k = 5 (lifted D4, plain D5) | 143 ns | 208 ns |
| k = 6 (lifted D5, plain D6) | 296 ns | 270 ns |

With the same expansion at k = 6, Delaunay `cube` D5 10^4 read 1.06 against `main` in a first run of three rounds, slower beyond the spread. Size six is therefore left to the running filter, which stays the f64 stage there (design §1).

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.72 ms (0.60 ms–1.18 ms) | 0.72 ms (0.62 ms–1.14 ms) | 1.00, inside the spread |
| Hull `cube` D2 10^5 | 6.24 ms (5.69 ms–11.4 ms) | 6.32 ms (5.68 ms–9.60 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^4 | 1.58 ms (1.36 ms–2.44 ms) | 1.61 ms (1.38 ms–2.17 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^5 | 12.7 ms (11.4 ms–16.5 ms) | 12.8 ms (11.9 ms–14.8 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 7.25 ms (6.46 ms–12.3 ms) | 7.43 ms (6.41 ms–8.49 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^5 | 38.5 ms (37.2 ms–41.3 ms) | 39.6 ms (37.0 ms–44.1 ms) | 1.03, inside the spread |
| Hull `cube` D5 10^4 | 61.4 ms (57.4 ms–75.8 ms) | 61.2 ms (58.1 ms–67.7 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 629 ms (613 ms–723 ms) | 629 ms (603 ms–704 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 176 ms (171 ms–203 ms) | 180 ms (172 ms–210 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^4 | 1.77 ms (1.53 ms–2.28 ms) | 1.71 ms (1.50 ms–2.58 ms) | 0.97, inside the spread |
| Hull `sphere` D2 10^5 | 19.0 ms (17.3 ms–20.7 ms) | 18.9 ms (16.9 ms–24.1 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 31.4 ms (28.8 ms–35.5 ms) | 31.9 ms (29.0 ms–41.1 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^5 | 378 ms (365 ms–458 ms) | 380 ms (368 ms–426 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 142 ms (135 ms–154 ms) | 142 ms (135 ms–171 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.75 s (1.67 s–1.81 s) | 1.80 s (1.70 s–1.80 s) | 1.03, inside the spread |
| Hull `sphere` D5 10^4 | 929 ms (896 ms–977 ms) | 932 ms (903 ms–989 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 8.67 ms (8.18 ms–10.6 ms) | 8.81 ms (8.10 ms–9.96 ms) | 1.02, inside the spread |
| Delaunay `cube` D2 10^5 | 95.7 ms (89.8 ms–111 ms) | 95.1 ms (88.9 ms–112 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^4 | 46.6 ms (45.1 ms–49 ms) | 46.8 ms (43.4 ms–52.1 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 529 ms (507 ms–552 ms) | 529 ms (512 ms–551 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 920 ms (864 ms–1.04 s) | 766 ms (689 ms–845 ms) | 0.83, faster beyond the spread |
| Delaunay `cube` D5 10^4 | 8.96 s (8.49 s–9.04 s) | 8.83 s (8.56 s–9.10 s) | 0.98, inside the spread |
| Delaunay `sphere` D2 10^4 | 13.0 ms (12.5 ms–14.7 ms) | 13.0 ms (12.2 ms–14.3 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 91.9 ms (89.1 ms–97.1 ms) | 90.5 ms (88.1 ms–98.1 ms) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^4 | 255 ms (250 ms–326 ms) | 255 ms (250 ms–280 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 2.43 s (2.41 s–2.57 s) | 2.42 s (2.40 s–2.46 s) | 0.99, inside the spread |

### Reading

- **Delaunay `cube` D4 10^4 is 17% faster** (0.83), beyond the spread, as the keep criterion asks: every in-sphere test there has k = 5, and the running filter was 58% of `build()` (#350).
- **Delaunay `cube` D5 10^4 is unchanged** (0.98, inside the spread). Its in-sphere tests have k = 6, which this row leaves to the running filter, so the keep criterion's D5 half is not met.
- **The hull is unchanged**: its orientations of size five and six are almost all decided by the cull scan or the cofactors (profile on #350).
- **Size five's bound is looser**: over the cancelling inputs of the margin test it is about 500 times the exact error, against 3 to 8 times for k ≤ 4, because the column sums stand for the permanent. How many tests of `cube` D4 it leaves open was not counted.
- What this does not show: Delaunay `sphere` D4, whose tests go to the exact stage (#331); that is P7-17 (#352).

### After review

`22faa87` removed the lifted underflow term from `X` in the timed path, so the head `aba5af9` was timed again against `main` at `14f506c` (docs only since `145ce0a`), by the same method and sets. Delaunay `cube` D4 10^4: 894 ms (873 ms–985 ms) against 737 ms (718 ms–794 ms), 0.82, faster beyond the spread. Delaunay `cube` D5 10^4: 8.68 s (8.64 s–8.71 s) against 8.64 s (8.55 s–8.73 s), 1.00, inside the spread. Every other set read 0.95 to 1.03, inside the spread, with the same counts.

## A double-double stage before the exact one, PR #356 (#352)

P7-17. For 2 ≤ k ≤ 6, plain and lifted, a test the `f64` stage leaves open is evaluated in double-double, with Dekker's product and a bound derived once per size (`double_double`, design §1). For k ≤ 5 the `f64` stage is the first stage and the running filter no longer runs; for k = 6 the `f64` stage is the running filter. The D2 and D3 shapes of Delaunay start after the first stage they already ran.

### Method

| Item | Value |
| --- | --- |
| convx | The head of #354 at `7ee1228` (the base this PR stacks on) against the head `429ac24`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341), the guard sets hull `cube` D5 and D6 and `sphere` D5 10^4, and the sets of the keep criterion: Delaunay `sphere` D2 10^5, D3 10^4 to 10^6, D4 10^4, and D5 10^4 |
| Rounds | The base and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for hull `sphere` D4 10^5, Delaunay `sphere` D3 10^5, `cube` D5 10^4, and `sphere` D4 10^4; 3 × 1 for Delaunay `sphere` D3 10^6 and D5 10^4. Delaunay `sphere` D2 10^5, inside the spread in that run, was run again alone: 20 rounds × 3 builds |

Every set published the same counts on both sides.

### Against the base

| Set | Base (#354) | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.70 ms (0.65 ms–3.87 ms) | 0.70 ms (0.63 ms–0.98 ms) | 0.99, inside the spread |
| Hull `cube` D2 10^5 | 6.37 ms (5.70 ms–7.56 ms) | 6.62 ms (5.74 ms–8.98 ms) | 1.04, inside the spread |
| Hull `cube` D3 10^4 | 1.50 ms (1.36 ms–1.92 ms) | 1.47 ms (1.32 ms–1.83 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 12.2 ms (11.4 ms–16.6 ms) | 12.2 ms (11.3 ms–13.3 ms) | 0.99, inside the spread |
| Hull `cube` D4 10^4 | 7.22 ms (6.45 ms–15.0 ms) | 7.21 ms (6.39 ms–26.6 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 39.2 ms (37.0 ms–65.3 ms) | 37.4 ms (36.2 ms–46.7 ms) | 0.96, inside the spread |
| Hull `cube` D5 10^4 | 60.0 ms (57.3 ms–63.6 ms) | 59.2 ms (56.7 ms–63.7 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 630 ms (591 ms–656 ms) | 628 ms (585 ms–702 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 172 ms (167 ms–216 ms) | 99.1 ms (95.9 ms–109 ms) | 0.58, faster beyond the spread |
| Hull `sphere` D2 10^4 | 1.65 ms (1.51 ms–2.22 ms) | 1.70 ms (1.56 ms–2.30 ms) | 1.03, inside the spread |
| Hull `sphere` D2 10^5 | 17.6 ms (16.5 ms–21.5 ms) | 17.8 ms (16.7 ms–22.7 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^4 | 30.3 ms (28.5 ms–42.4 ms) | 30.9 ms (29.0 ms–38.1 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^5 | 369 ms (344 ms–418 ms) | 375 ms (357 ms–539 ms) | 1.02, inside the spread |
| Hull `sphere` D4 10^4 | 136 ms (131 ms–167 ms) | 135 ms (131 ms–173 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^5 | 1.65 s (1.65 s–1.69 s) | 1.65 s (1.63 s–1.66 s) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 854 ms (831 ms–888 ms) | 855 ms (838 ms–899 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 8.92 ms (8.17 ms–10.8 ms) | 9.16 ms (8.20 ms–10.3 ms) | 1.03, inside the spread |
| Delaunay `cube` D2 10^5 | 93.4 ms (91.3 ms–102 ms) | 95.1 ms (91.1 ms–123 ms) | 1.02, inside the spread |
| Delaunay `cube` D3 10^4 | 47.3 ms (45.1 ms–56.4 ms) | 48.0 ms (46.1 ms–52.5 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^5 | 549 ms (536 ms–581 ms) | 550 ms (538 ms–586 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 738 ms (724 ms–760 ms) | 725 ms (710 ms–781 ms) | 0.98, inside the spread |
| Delaunay `cube` D5 10^4 | 8.62 s (8.35 s–8.67 s) | 8.54 s (8.40 s–8.64 s) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^4 | 13.2 ms (12.4 ms–15.1 ms) | 8.51 ms (7.98 ms–9.93 ms) | 0.65, faster beyond the spread |
| Delaunay `sphere` D2 10^5 | 92.7 ms (88.9 ms–99.5 ms) | 77.8 ms (75.3 ms–93.4 ms) | 0.84, inside the spread |
| Delaunay `sphere` D3 10^4 | 267 ms (261 ms–299 ms) | 97.0 ms (91.2 ms–106 ms) | 0.36, faster beyond the spread |
| Delaunay `sphere` D3 10^5 | 2.39 s (2.37 s–2.40 s) | 952 ms (938 ms–953 ms) | 0.40, faster beyond the spread |
| Delaunay `sphere` D3 10^6 | 17.8 s (17.8 s–18.0 s) | 7.83 s (7.81 s–8.33 s) | 0.44, faster beyond the spread |
| Delaunay `sphere` D4 10^4 | 4.33 s (4.33 s–4.35 s) | 1.17 s (1.17 s–1.19 s) | 0.27, faster beyond the spread |
| Delaunay `sphere` D5 10^4 | 47.0 s (46.8 s–49.6 s) | 14.5 s (14.5 s–16.4 s) | 0.31, faster beyond the spread |

Delaunay `sphere` D2 10^5 in the second run, 20 rounds × 3 builds: base 94.7 ms (88.6 ms–103 ms), head 78.8 ms (75.3 ms–88.0 ms), 0.83, faster beyond the spread.

### The fixtures

The test binaries that build the fixtures and cases, release, one thread, pinned, 11 alternated runs each: `hull_fixtures` base 588 ms (567 ms–611 ms), head 485 ms (470 ms–493 ms), 0.82; `delaunay_cases` 167 ms (161 ms–175 ms) and 167 ms (159 ms–182 ms), 1.00; `voronoi_cases` 165 ms (163 ms–173 ms) and 164 ms (156 ms–171 ms), 1.00. A first run without pinning read 1.82 on `delaunay_cases`; pinned, the same binaries agree, and every test that reached the double-double stage there and stayed open was an exact zero that the running filter left open too.

### Reading

- **Every set of the keep criterion is faster beyond the spread**: Delaunay `sphere` D3 0.36 to 0.44, D4 0.27, D5 0.31, and D2 10^5 0.83. The exact stage was 63% to 88% of these builds (#331); the double-double stage decides the tests it took.
- **Hull `cubesurf` D3 10^5 is 0.58**, and Delaunay `sphere` D2 10^4 0.65: their orientations left open by the first stage went through the running filter, which decided nothing on them, before the exact stage.
- **Nothing is slower beyond the spread.** The `cube` sets are unchanged: on Delaunay `cube` D2 and D3 the first stage decided every in-sphere test (#331), so neither the running filter nor the new stage is reached there. Delaunay `cube` D5 10^4 (k = 6), where the running filter decides every test, is unchanged.
- What this does not show: how many tests reach the double-double stage and how many it leaves to the exact one. Against Qhull and CGAL, the full parity run after this row's merge says.

## The P7 parity run after P7-16 and P7-17, PR #361 (#360)

Due after the third speed row merged since #341: P7-10 (#343), P7-14 (#342), P7-16 (#354), and P7-17 (#356) have merged since the last full run (#342).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `73f0fde`, `--release` with debug info |
| Machine | Intel Core i5-13400F, WSL2 on Windows 11, the parity run of `docs/verification.md` |
| References | Qhull and CGAL as in the baseline (#313), on the same machine |
| Judgement | `build()` against each judged reference, the median of the runs; "(ref.)" marks a reference that is not judged (ADR 0006) |

### P7 judgement

Met: 13 of 41 (hull 9, Delaunay 4), against 11 in the run of #342.

| Set | convx `build()` | / CGAL | / Qhull | P7 |
| --- | ---: | ---: | ---: | --- |
| Hull `cube` D2 10^4 | 0.57 ms | 1.18 | 1.01 | not met |
| Hull `cube` D2 10^5 | 5.77 ms | 1.28 | 1.02 | not met |
| Hull `cube` D2 10^6 | 68.1 ms | 1.49 | 1.19 | not met |
| Hull `cube` D3 10^4 | 1.28 ms | 0.58 | 0.85 | met |
| Hull `cube` D3 10^5 | 10.4 ms | 0.41 | 0.89 | met |
| Hull `cube` D3 10^6 | 139 ms | 0.25 | 0.94 | met |
| Hull `cube` D4 10^4 | 6.62 ms | 0.15 | 0.97 | met |
| Hull `cube` D4 10^5 | 34.8 ms | 0.06 | 1.02 | not met |
| Hull `cube` D5 10^4 | 55.4 ms | 0.25 | 0.75 | met |
| Hull `cube` D5 10^5 | 216 ms | 0.13 | 0.67 | met |
| Hull `cube` D6 10^4 | 615 ms | 0.30 | 0.49 | met |
| Hull `cube` D6 10^5 | 2.60 s | 0.23 | 0.39 | met |
| Hull `sphere` D2 10^4 | 1.55 ms | 2.50 | 0.23 (ref.) | not met |
| Hull `sphere` D2 10^5 | 15.9 ms | 2.14 | 0.14 (ref.) | not met |
| Hull `sphere` D2 10^6 | 226 ms | 2.57 | 0.12 (ref.) | not met |
| Hull `sphere` D3 10^4 | 27.3 ms | 1.21 | 1.46 | not met |
| Hull `sphere` D3 10^5 | 335 ms | 0.83 | 1.37 | not met |
| Hull `sphere` D3 10^6 | 5.00 s | 0.62 | 1.46 | not met |
| Hull `sphere` D4 10^4 | 127 ms | 1.06 | 1.33 | not met |
| Hull `sphere` D4 10^5 | 1.62 s | 1.27 | 1.22 | not met |
| Hull `sphere` D5 10^4 | 849 ms | 0.87 | 1.02 | not met |
| Hull `sphere` D5 10^5 | 10.86 s | 0.98 | 1.01 | not met |
| Hull `sphere` D6 10^4 | 6.99 s | 0.75 | 0.86 | met |
| Hull `cubesurf` D3 10^5 | 102 ms | 2.77 | 3.58 | not met |
| Hull `grid` D6 10^4 | 2.24 s | 8.81 (ref.) | 6.53 | not met |
| Delaunay `cube` D2 10^4 | 8.03 ms | 2.01 | 0.43 | not met |
| Delaunay `cube` D2 10^5 | 88.2 ms | 2.13 | 0.38 | not met |
| Delaunay `cube` D2 10^6 | 1.08 s | 2.29 | 0.33 | not met |
| Delaunay `sphere` D2 10^4 | 7.37 ms | 1.96 | 0.31 (ref.) | not met |
| Delaunay `sphere` D2 10^5 | 70.7 ms | 2.20 | 0.19 (ref.) | not met |
| Delaunay `sphere` D2 10^6 | 823 ms | 2.62 | 0.07 (ref.) | not met |
| Delaunay `cube` D3 10^4 | 46.9 ms | 1.60 | 0.51 | not met |
| Delaunay `cube` D3 10^5 | 554 ms | 1.77 | 0.44 | not met |
| Delaunay `cube` D3 10^6 | 6.29 s | 1.86 | 0.40 | not met |
| Delaunay `sphere` D3 10^4 | 80.1 ms | 0.63 | 0.70 (ref.) | met |
| Delaunay `sphere` D3 10^5 | 787 ms | 0.86 | 0.42 (ref.) | met |
| Delaunay `sphere` D3 10^6 | 6.89 s | 1.10 | 0.41 (ref.) | not met |
| Delaunay `cube` D4 10^4 | 700 ms | 1.15 | 0.93 | not met |
| Delaunay `sphere` D4 10^4 | 918 ms | 0.37 | 0.75 (ref.) | met |
| Delaunay `cube` D5 10^4 | 7.46 s | 1.12 | 0.95 | not met |
| Delaunay `sphere` D5 10^4 | 11.69 s | 0.34 | 0.83 (ref.) | met |

### What moved

- **Delaunay `sphere` D3 10^4 and 10^5, D4 10^4, and D5 10^4 are met** (0.63, 0.86, 0.37, 0.34 against CGAL), against 1.84, 2.38, 1.52, and 1.37 in the run of #342. The double-double stage (#356) decides the in-sphere tests that went to the exact stage.
- **Delaunay `sphere` D3 10^6** reads 1.10 against CGAL, from 2.73.
- **Delaunay `cube` D4 10^4** reads 1.15, from 1.33 (the first stage of size five, #354). `cube` D5 10^4 reads 1.12, from 1.15; its tests have k = 6 and are decided by the running filter.
- **Hull `cube` D4 10^5 and `sphere` D5 10^4** read 1.02 against Qhull, not met, after 0.93 and 0.99 in the run of #342. Neither path changed between the two runs, so they moved by run-to-run noise, as that section said.

### The sets not met

| Path | Sets | Decided by |
| --- | --- | --- |
| Hull D2 | `cube` 10^4 to 10^6 (1.18 to 1.49), `sphere` 10^4 to 10^6 (2.14 to 2.57) | CGAL; P7-7 (#322) and P7-9 (#324) |
| Hull D3 to D5 | `sphere` D3 10^4 to 10^6 (Qhull 1.37 to 1.46), `sphere` D4 10^4 and 10^5 (1.06 to 1.33), `sphere` D5 10^4 and 10^5 (Qhull 1.02 and 1.01), `cube` D4 10^5 (Qhull 1.02) | Qhull, and CGAL on `sphere` D4 |
| Hull, degenerate | `cubesurf` D3 10^5 (2.77 and 3.58), `grid` D6 10^4 (Qhull 6.53) | both |
| Delaunay D2 | `cube` and `sphere`, 10^4 to 10^6 (1.96 to 2.62) | CGAL; P7-13 (#332) |
| Delaunay D3 | `cube` 10^4 to 10^6 (1.60 to 1.86), `sphere` 10^6 (1.10) | CGAL |
| Delaunay D4, D5 | `cube` D4 10^4 (1.15), `cube` D5 10^4 (1.12) | CGAL |

## Delaunay D2: insertion order and point location against CGAL's, PR #364 (#332)

The spike P7-13. It asks whether a Hilbert order, a cheaper sort of the order's keys, or a walk from the last insertion would close the gap of Delaunay D2 to CGAL (1.96 to 2.62 in the run of #361).

### Method

| Item | Value |
| --- | --- |
| convx | A copy of `main` at `208bfd9` outside the crate, with atomic counters and phase timers, and two variants chosen by environment variables: a Hilbert key in place of the Morton key under the same rounds of BRIO, and a stable LSD radix sort of `(round, key)` in place of `sort_unstable_by_key`. rustc 1.97.1, `--release` with debug info |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, on the sets of `tests/common/generator.rs`, seed 1. The four variants alternated per round: 10 rounds × 3 builds at 10^5, 5 × 1 at 10^6 |
| Location | Timed in separate runs, with a timer around every call; that adds about 2 ns a call to the location's own time |
| Published | The sorted vertex lists of the published simplices, hashed |

### Per site and per phase

| Set | Variant | `build()` | Ratio | Walk steps | Orientations | Order | Mesh | Location | Union | Draft |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | base | 99.5 ms | 1.00 | 5.46 | 11.27 | 8.0 ms | 68.1 ms | 24.5 ms | 6.6 ms | 11.6 ms |
| | Hilbert | 93.5 ms | 0.94, inside the spread | 4.13 | 8.93 | 8.0 ms | 62.3 ms | 19.4 ms | 6.2 ms | 11.5 ms |
| | radix | 98.3 ms | 0.99, inside the spread | 5.46 | 11.27 | 6.8 ms | 67.8 ms | 24.5 ms | 6.4 ms | 11.6 ms |
| | both | 93.1 ms | 0.94, inside the spread | 4.13 | 8.93 | 7.1 ms | 62.2 ms | 19.8 ms | 6.4 ms | 11.5 ms |
| `sphere` D2 10^5 | base | 79.1 ms | 1.00 | 4.41 | 8.82 | 7.4 ms | 55.5 ms | 23.0 ms | 3.6 ms | 8.2 ms |
| | Hilbert | 81.3 ms | 1.03, inside the spread | 4.96 | 9.92 | 8.0 ms | 57.5 ms | 25.3 ms | 3.6 ms | 8.2 ms |
| | radix | 78.5 ms | 0.99, inside the spread | 4.41 | 8.82 | 6.8 ms | 55.5 ms | 23.0 ms | 3.8 ms | 8.1 ms |
| | both | 80.9 ms | 1.02, inside the spread | 4.96 | 9.92 | 6.9 ms | 57.5 ms | 25.2 ms | 3.5 ms | 8.0 ms |
| `cube` D2 10^6 | base | 1.17 s | 1.00 | 5.41 | 11.20 | 82.6 ms | 796 ms | 325 ms | 108 ms | 134 ms |
| | Hilbert | 1.12 s | 0.96, faster beyond the spread | 4.09 | 8.85 | 84.2 ms | 745 ms | 272 ms | 106 ms | 134 ms |
| | radix | 1.16 s | 0.99, inside the spread | 5.41 | 11.20 | 70.5 ms | 792 ms | 332 ms | 105 ms | 134 ms |
| | both | 1.11 s | 0.94, faster beyond the spread | 4.09 | 8.85 | 72.6 ms | 741 ms | 273 ms | 106 ms | 132 ms |
| `sphere` D2 10^6 | base | 894 ms | 1.00 | 4.42 | 8.84 | 81.8 ms | 627 ms | 318 ms | 48.3 ms | 87.5 ms |
| | Hilbert | 933 ms | 1.04, slower beyond the spread | 4.99 | 9.98 | 85.1 ms | 658 ms | 338 ms | 48.5 ms | 85.0 ms |
| | radix | 882 ms | 0.99, inside the spread | 4.42 | 8.84 | 67.3 ms | 634 ms | 311 ms | 48.4 ms | 86.1 ms |
| | both | 904 ms | 1.01, inside the spread | 4.99 | 9.98 | 68.8 ms | 650 ms | 336 ms | 48.5 ms | 87.9 ms |

Medians. "Mesh" is `Mesh::build`, the location inside it; "Union" is the pass that merges cospherical simplices, with 810,031 lifted tests on `cube` D2 10^6 and 279,321 on `sphere`; "Draft" builds the published rows. Every variant published the same simplices as the base on every set.

### Reading

- **The order is not the gap.** A Hilbert key cuts the walk on `cube` (5.41 to 4.09 steps a site) and saves 4 to 6% there, but lengthens it on `sphere`, whose sites lie on a circle (4.42 to 4.99), and costs 4% at 10^6. The radix sort saves 12 to 15 ms of the order at 10^6, about 1% of `build()`, inside the spread. Neither is a row.
- **The walk already starts where CGAL's does**: from a simplex the last insertion created (`Mesh::locate`). The location is 36 to 51% of the mesh build, timed with its own timers, at 4.4 to 5.5 steps and 8.8 to 11.3 orientations a site.
- **The gap is the insertion and the pass after it.** On `cube` D2 10^6 the mesh takes 796 ms, about 470 ms of it outside the location, and the pass after it 242 ms. CGAL's whole build takes 471 ms. The pass (union and draft) is 21% of `build()` on `cube` and 15% on `sphere`.

### Rows that follow

| Row | Kind | What |
| --- | --- | --- |
| P7-19 (#362) | Feat | Delaunay: the pass after the insertion at the cost of its tests |
| P7-20 (#363) | Spike | Delaunay D2: the insertion's own work against CGAL's |

## Delaunay: the pass after the insertion, PR #365 (#362)

P7-19. The pass after the insertion merges cospherical simplices and builds the published rows (`delaunay::inserted`). It took 19 to 24% of `build()` on Delaunay `cube` D2 and D3 (#332, and the profile and its correction on #362). Three changes:
- the union skips a face the insertion recorded as distinct without reading its neighbor, and finds the far vertex among the neighbor's vertices rather than through its links;
- a simplex at infinity keeps `NO_NEIGHBOR` in the numbering, so a published link is one read;
- the vertices of each simplex of D = 2 and 3 are sorted by a network of selected swaps, `min` and `max` of one word per vertex.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `208bfd9` against the head `8ff70e6`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341), the guard sets, and the keep criterion's Delaunay `cube` D2 and D3 at 10^5 and 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a second. A second run of 20 rounds × 3 builds (10 × 1 and 6 × 1 for D3) on hull `cubesurf` D3 10^5 and Delaunay `cube` D2 10^5 and D3 10^5 and 10^6 |
| Phases | A copy of each side outside the crate, with timers around the union and the draft |

Every set published the same counts on both sides, and the copies published the same simplices (a hash of the sorted vertex lists).

### The pass

| Set | Union, `main` | Union, head | Draft, `main` | Draft, head |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 6.4 ms | 5.7 ms | 12.1 ms | 8.8 ms |
| `cube` D3 10^5 | 65.6 ms | 52.6 ms | 61.8 ms | 38.9 ms |
| `cube` D2 10^6 | 110 ms | 93.3 ms | 143 ms | 67 ms |

The head's draft of `cube` D2 10^6 is from the final form of the sort; the other head columns from the form before it, with the same union.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.66 ms (0.62 ms–1.26 ms) | 0.67 ms (0.61 ms–0.82 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^5 | 6.06 ms (5.62 ms–6.95 ms) | 6.26 ms (5.60 ms–7.01 ms) | 1.03, inside the spread |
| Hull `cube` D3 10^4 | 1.54 ms (1.35 ms–2.29 ms) | 1.49 ms (1.31 ms–2.68 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^5 | 11.9 ms (11.2 ms–13.0 ms) | 11.7 ms (11.1 ms–23.6 ms) | 0.99, inside the spread |
| Hull `cube` D4 10^4 | 6.68 ms (6.31 ms–7.56 ms) | 6.52 ms (6.23 ms–8.92 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^5 | 35.3 ms (34.4 ms–46.9 ms) | 35.2 ms (34.4 ms–42.6 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 56.9 ms (55.0 ms–60.0 ms) | 56.0 ms (54.7 ms–60.2 ms) | 0.98, inside the spread |
| Hull `cube` D6 10^4 | 580 ms (569 ms–607 ms) | 583 ms (568 ms–597 ms) | 1.01, inside the spread |
| Hull `cubesurf` D3 10^5 | 94.2 ms (92.1 ms–101 ms) | 107 ms (104 ms–111 ms) | 1.14, slower beyond the spread |
| Hull `sphere` D2 10^4 | 1.57 ms (1.51 ms–2.10 ms) | 1.57 ms (1.50 ms–2.72 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^5 | 17.4 ms (16.3 ms–19.3 ms) | 17.2 ms (16.4 ms–20.3 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^4 | 29.9 ms (28.3 ms–49.9 ms) | 29.6 ms (28.0 ms–42.5 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 325 ms (321 ms–339 ms) | 326 ms (320 ms–352 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 127 ms (124 ms–136 ms) | 127 ms (122 ms–137 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.65 s (1.64 s–1.69 s) | 1.65 s (1.62 s–1.65 s) | 0.99, inside the spread |
| Hull `sphere` D5 10^4 | 840 ms (810 ms–869 ms) | 833 ms (814 ms–861 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 8.34 ms (8.05 ms–9.47 ms) | 7.82 ms (7.66 ms–9.09 ms) | 0.94, inside the spread |
| Delaunay `cube` D2 10^5 | 93.4 ms (84.3 ms–105 ms) | 87.8 ms (78.8 ms–97.7 ms) | 0.94, inside the spread |
| Delaunay `cube` D2 10^6 | 1.10 s (1.09 s–1.13 s) | 1.03 s (1.01 s–1.06 s) | 0.94, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 48.6 ms (45.8 ms–54.2 ms) | 45.7 ms (43.5 ms–54.1 ms) | 0.94, inside the spread |
| Delaunay `cube` D3 10^5 | 552 ms (540 ms–570 ms) | 514 ms (492 ms–552 ms) | 0.93, inside the spread |
| Delaunay `cube` D3 10^6 | 5.55 s (5.50 s–6.01 s) | 5.21 s (5.16 s–5.86 s) | 0.94, inside the spread |
| Delaunay `cube` D4 10^4 | 743 ms (730 ms–763 ms) | 718 ms (705 ms–791 ms) | 0.97, inside the spread |
| Delaunay `cube` D5 10^4 | 8.46 s (8.40 s–8.62 s) | 8.24 s (8.08 s–8.39 s) | 0.97, faster beyond the spread |
| Delaunay `sphere` D2 10^4 | 8.77 ms (7.85 ms–10.2 ms) | 8.17 ms (7.50 ms–10.4 ms) | 0.93, inside the spread |
| Delaunay `sphere` D2 10^5 | 77.6 ms (74.3 ms–85.4 ms) | 73.3 ms (69.4 ms–84.9 ms) | 0.95, inside the spread |
| Delaunay `sphere` D3 10^4 | 96.9 ms (93.9 ms–103 ms) | 94.8 ms (93.2 ms–105 ms) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^5 | 941 ms (939 ms–957 ms) | 926 ms (918 ms–933 ms) | 0.98, faster beyond the spread |

The second run: hull `cubesurf` D3 10^5 1.11, inside the spread through one build; Delaunay `cube` D2 10^5 0.94, inside the spread through one build of the base; `cube` D3 10^5 0.93 and D3 10^6 0.93, both faster beyond the spread.

### Reading

- **The keep criterion is met on the Delaunay sets, not as written on hull `cubesurf` D3 10^5** (next item). Delaunay `cube` D2 10^6 0.94, D3 10^5 0.93, and D3 10^6 0.93 are faster beyond the spread. D2 10^5 read 0.94, 0.94, and 0.96 in three runs, its ranges overlapping only through single builds, so the difference is reported (`bench.mdc`).
- **Hull `cubesurf` D3 10^5 read 1.14 and 1.11, but no hull code changed.** VTune showed different inlining in the hull's functions between the two binaries. Built with one codegen unit each (`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`), `cubesurf` read 1.00 (108.7 ms against 108.7 ms, 15 rounds × 3), and Delaunay `cube` D3 10^5 still 0.94. The change in the `delaunay` module moved the partition of the crate into codegen units, and the hull's code with it.
- **What remains of the pass** is the union's lifted tests and the random reads they need, about 93 ms on `cube` D2 10^6 for 810,031 faces, and the rows themselves. A test reads a neighbor's vertices and four sites' coordinates. A test made at the insertion, while they are in cache, would run on every face between new simplices, most of which later insertions destroy.

### After review

`8bd29e7` made the words of the sort a `Small` that spills to the heap, for simplices above 11 vertices (#365 review). It touches the timed path, so the head `8bd29e7` was timed again against `main` at `208bfd9`, by the same method and sets:

- Delaunay `cube` D2 10^6: 0.93, faster beyond the spread.
- Delaunay `cube` D3 10^5: 0.93, and D3 10^6: 0.91. Inside the spread, each through one build of the head (497 ms against a base minimum of 483 ms, and 6.46 s against 5.89 s); as in the run before, the medians agree.
- Delaunay `cube` D2 10^5: 0.95, inside the spread.
- The other Delaunay sets: 0.94 to 1.01.
- Hull `cubesurf` D3 10^5: 1.13, slower beyond the spread, as before review. Every other hull set read 0.99 to 1.02. Every set published the same counts.

## Delaunay D2: the insertion's own work against CGAL's, PR #367 (#363)

The spike P7-20. It counts what one insertion does in convx and in CGAL, times the parts of convx's insertion, and prototypes the largest cost.

### Method

| Item | Value |
| --- | --- |
| convx | Copies of `main` at `208bfd9` outside the crate, with atomic counters, phase timers enabled by an environment variable, and the prototypes as variants. rustc 1.97.1, `--release` with debug info; Windows 11, every process pinned (logical processor 2); the variants alternated per round, 10 rounds × 3 builds at 10^5 and 5 × 1 at 10^6 |
| CGAL | `Delaunay_triangulation_2` over Epick, on the point files of the parity run, under WSL2 on the same machine, pinned with `taskset`. Counts with traits that wrap `Orientation_2` and `Side_of_oriented_circle_2`; times with Epick itself, 5 runs |
| Published | The sorted vertex lists of the published simplices, hashed; every variant matched `main` on every set |

### Per site

| Set | Side | In-circle tests | Orientations | Cavity simplices | New simplices | Flips | Turn steps |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 | convx | 9.03 | 11.28 | 4.02 | 6.02 | — | 12.06 |
| | CGAL | 9.11 | 6.64 | — | — | 3.06 | — |
| `sphere` D2 | convx | 3.96 | 12.81 | 2.98 | 4.98 | — | 8.93 |
| | CGAL | 4.00 | 11.15 | — | — | 1.99 | — |

At 10^5; 10^6 gives the same counts within 1%. CGAL's flips are its positive in-circle answers, each of which flips an edge.

### The parts of convx's insertion

Timed with a timer at each boundary, which adds about 150 ms at 10^6 to a mesh of 760 ms (`cube`) and 630 ms (`sphere`):

| Set | Location | Cavity search | Creation | Links | Freeing |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^6 | 308 ms | 298 ms | 126 ms | 138 ms | 37 ms |
| `sphere` D2 10^6 | 308 ms | 227 ms | 96 ms | 108 ms | 38 ms |

The first stage alone, on 10^6 random points of the unit square, takes 5.7 ns a lifted orientation and 2.3 ns an orientation: about 80 ns of predicates a site, against about 760 ns of mesh. VTune puts a fifth of `cube` D2 10^6 on the line of the first stage that loads the coordinates (`semi_static.rs`, the closure `diff`).

### Prototypes

| Set | `main` | Sites in insertion order | Vertices and neighbors in one array | Walk that skips its entry face |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 100 ms | 0.97, inside the spread | 1.00 | 1.01 |
| `sphere` D2 10^5 | 82.0 ms | 0.95, inside the spread | 1.04 | 1.00 |
| `cube` D2 10^6 | 1.20 s | **0.87, faster beyond the spread** | 1.02 | 1.00 |
| `sphere` D2 10^6 | 928 ms | **0.83, faster beyond the spread** | 1.03 | 0.98 |

Two different runs; each ratio is against `main` in its own run. Each run's `main` is given for the first prototype. The orientations per site of the walk that skips its entry face fall from 11.2 to 8.7 (`cube`) and 12.8 to 11.1 (`sphere`).

- **Sites in insertion order:** the site rows copied into the order of `first` then the BRIO order before `Mesh::build`. The mesh is built on those indices, and the indices are mapped back for the published rows and the merged groups. The copy is inside the timed build. Unpinned and only twice, `cube` D3 10^5 and 10^6 read 0.92 to 0.99.
- **Vertices and neighbors in one array:** one row of 2k per simplex in place of two arrays of k.
- **Walk that skips its entry face:** `Mesh::locate` does not test the face it came through, as CGAL's remembering walk does.

### Reading

- **convx does CGAL's predicates, and they are cheap.** The in-circle tests per site match CGAL's. The orientations are 1.7 times CGAL's on `cube`, from the walk. The predicates are about a tenth of the mesh's time.
- **The cost is where the sites are.** BRIO visits sites in a spatial order, but they are stored by input index, so consecutive insertions read coordinates far apart in memory. Stored in insertion order, the mesh of `cube` D2 10^6 drops from 814 to 668 ms and of `sphere` from 656 to 513 ms. CGAL allocates its vertices, and their points, in the order of its spatial sort.
- **Neither the layout of the mesh's arrays nor the walk's tests matter at this size.** One array for vertices and neighbors read 1.00 to 1.04. A walk with a fifth fewer orientations read 0.98 to 1.01.
- After the prototype, the mesh of `cube` D2 10^6 takes 668 ms against CGAL's whole build of 471 ms (on Linux). A profile after the row that follows says what remains.

### Rows that follow

| Row | Kind | What |
| --- | --- | --- |
| P7-21 (#366) | Feat | Delaunay: the sites in the order of their insertion |

## Delaunay: the sites in the order of their insertion, PR #369 (#366)

P7-21. The insertion read each site's row by its input index, while BRIO visits the sites in a spatial order. Those loads were a quarter of Delaunay D2 at 10^6 sites (#363, and the profile on #366). The mesh is now built on a copy of the rows in insertion order. The sites keep their input index:
- it seeds the walk, so the simplices are created and published in the same order as before;
- it maps the published rows and the merged groups back to the input's indices.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `214b6fe` against the head `a77c1b5`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341), the guard sets, and the keep criterion's Delaunay `cube` and `sphere` D2 10^6, with `cube` D3 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a second |
| Published | The simplices, their neighbors, and the representatives, in their published order, hashed: the same on Delaunay `cube` and `sphere` D2 10^5, D3 2 × 10^4, D4 3000 and 2000, and `grid` D2 and D3 |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.68 ms (0.61 ms–1.16 ms) | 0.66 ms (0.61 ms–0.81 ms) | 0.96, inside the spread |
| Hull `cube` D2 10^5 | 6.19 ms (5.70 ms–7.70 ms) | 6.22 ms (5.66 ms–6.80 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^4 | 1.55 ms (1.35 ms–2.01 ms) | 1.52 ms (1.36 ms–2.16 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 11.7 ms (11.1 ms–13.7 ms) | 11.8 ms (11.3 ms–15.2 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^4 | 6.90 ms (6.38 ms–8.25 ms) | 6.89 ms (6.30 ms–7.93 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 38.5 ms (36.9 ms–41.0 ms) | 38.2 ms (36.3 ms–46.2 ms) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 60.6 ms (58.7 ms–71.1 ms) | 59.8 ms (57.6 ms–61.5 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 637 ms (614 ms–723 ms) | 634 ms (613 ms–759 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 113 ms (109 ms–121 ms) | 113 ms (109 ms–120 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 1.63 ms (1.51 ms–2.34 ms) | 1.67 ms (1.51 ms–2.14 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^5 | 18.4 ms (16.4 ms–21.2 ms) | 18.2 ms (16.9 ms–22.3 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^4 | 29.9 ms (28.4 ms–44.3 ms) | 30.5 ms (28.5 ms–39.8 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^5 | 346 ms (338 ms–368 ms) | 347 ms (341 ms–367 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 138 ms (133 ms–147 ms) | 137 ms (134 ms–143 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^5 | 1.65 s (1.64 s–1.70 s) | 1.64 s (1.62 s–1.66 s) | 0.99, inside the spread |
| Hull `sphere` D5 10^4 | 901 ms (885 ms–921 ms) | 892 ms (878 ms–916 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 8.44 ms (7.96 ms–10.2 ms) | 8.67 ms (8.09 ms–9.54 ms) | 1.03, inside the spread |
| Delaunay `cube` D2 10^5 | 89.8 ms (86.6 ms–97.5 ms) | 87.7 ms (85.1 ms–93.9 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^6 | 979 ms (965 ms–995 ms) | 830 ms (820 ms–840 ms) | 0.85, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 46.0 ms (43.7 ms–49.4 ms) | 45.1 ms (43.5 ms–49.5 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^5 | 509 ms (501 ms–530 ms) | 486 ms (476 ms–502 ms) | 0.96, inside the spread |
| Delaunay `cube` D3 10^6 | 5.26 s (5.07 s–5.29 s) | 4.78 s (4.69 s–4.94 s) | 0.91, faster beyond the spread |
| Delaunay `cube` D4 10^4 | 746 ms (737 ms–776 ms) | 721 ms (709 ms–749 ms) | 0.97, inside the spread |
| Delaunay `cube` D5 10^4 | 8.10 s (7.29 s–8.52 s) | 7.57 s (7.30 s–8.62 s) | 0.93, inside the spread |
| Delaunay `sphere` D2 10^4 | 8.16 ms (7.57 ms–9.25 ms) | 8.21 ms (7.68 ms–17.7 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^5 | 72.1 ms (69.1 ms–78.0 ms) | 70.8 ms (68.6 ms–75.7 ms) | 0.98, inside the spread |
| Delaunay `sphere` D2 10^6 | 747 ms (734 ms–756 ms) | 634 ms (630 ms–642 ms) | 0.85, faster beyond the spread |
| Delaunay `sphere` D3 10^4 | 94.6 ms (91.6 ms–103 ms) | 95.7 ms (93.3 ms–106 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^5 | 918 ms (916 ms–933 ms) | 897 ms (894 ms–905 ms) | 0.98, faster beyond the spread |

### Reading

- **The keep criterion is met.** Delaunay `cube` and `sphere` D2 10^6 read 0.85, faster beyond the spread, close to the prototype of #363 (0.87 and 0.83).
- **D3 gains too.** `cube` D3 10^6 read 0.91 and `sphere` D3 10^5 0.98, both beyond the spread. D2 and D3 at 10^4 and 10^5 are inside the spread. The profile put the loads at 2 to 13% there.
- **Nothing is slower beyond the spread.** Hull `cubesurf` D3 10^5, which read 1.11 to 1.14 after #365, read 1.00 here: no hull code changed in either PR (#368).
- **The copy's memory** is the rows, `(D + 2)` `f64` per site, and the input index, one `u32` per site, on both sides: about 36 MB more at D2 10^6, against a mesh of about 2 × 10^6 simplices of 3 vertices, 3 neighbors, and 3 records each.
- **The walk's seed.** In the prototype it came from the stored index, which changed the order of the published simplices without changing their set. The prototype's hash of the sorted vertex lists did not see that, so this head compares the published order itself. A test builds the same inputs with both storage orders (`the_storage_order_of_the_sites_changes_nothing_published`).

## Delaunay D2: an insertion by edge flips, PR #371 (#370)

The spike P7-22. Does an insertion by edge flips, CGAL's method for D = 2, build the mesh in at most 0.6 of the time of convx's cavity insertion? The rule was set in the Grill of 2026-10-10 (on #370): at most 0.6 on each of Delaunay `cube` and `sphere` D2 at 10^5 and 10^6 leads to a dedicated D = 2 insertion and ADR 0007; otherwise the current insertion keeps being improved.

### Method

| Item | Value |
| --- | --- |
| convx | A copy of the head of #369 (`06dbcad`, the sites in insertion order) outside the crate. It has one entry that builds the D = 2 mesh of the same input, order, and stored sites, either with `Mesh::build` (the cavity) or with the prototype. rustc 1.97.1, `--release` with debug info |
| Prototype | CGAL's method: a walk from the last insertion that does not test the face it came through; the point inserted into its triangle (three triangles, or four on an edge); the edges whose far vertex conflicts flipped. The vertex at infinity is a vertex like the others, and a triangle at infinity conflicts by the orientation of its hull edge. Each triangle is a record of three vertices and three neighbors. The same predicates as the cavity (the first stage, double-double, exact), through the D = 2 shape |
| Timed | The mesh build alone, location included, after the order and the copy. Pinned (logical processor 2), the two variants alternated per round: 10 rounds × 3 builds at 10^5, 6 × 1 at 10^6 |
| Compared | The finite triangles of both, as sorted input indices, hashed |

### Results

| Set | Cavity | Flips | Ratio | Walk steps | Orientations | In-circle tests | Flips |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 59.6 ms (57.0–69.3) | 41.0 ms (37.5–52.0) | 0.69, faster beyond the spread | 5.55 | 11.88 | 9.03 | 3.02 |
| `sphere` D2 10^5 | 49.9 ms (47.5–57.1) | 33.4 ms (32.4–36.1) | 0.67, faster beyond the spread | 3.88 | 11.53 | 3.96 | 1.98 |
| `cube` D2 10^6 | 614 ms (609–630) | 414 ms (408–437) | 0.67, faster beyond the spread | 5.52 | 11.84 | 9.04 | 3.02 |
| `sphere` D2 10^6 | 471 ms (459–488) | 306 ms (300–325) | 0.65, faster beyond the spread | 3.90 | 11.57 | 3.98 | 1.99 |

Per site, for the flips. The prototype's triangles matched the cavity's on every set. CGAL, counted in #363: 9.11 and 4.00 in-circle tests, 3.06 and 1.99 flips a site.

### Reading

- **The rule is not met.** The flips build the mesh in 0.65 to 0.69 of the cavity's time on all four sets, beyond the spread, against a rule of at most 0.6. By the Grill's rule the current insertion would keep being improved. The owner changed the rule after the measurement (#370): the flips are adopted for D = 2 (ADR 0007, P7-23, #372).
- **The flips do CGAL's work.** Their in-circle tests and flips per site match CGAL's to within 2%. The cavity does the same in-circle tests (#363), so the saving is in what the cavity writes and links: four freed and six new triangles a site, and the turns around the ridges, against three new triangles and three flips.
- **What a dedicated path would leave.** On `cube` D2 10^6, the flips' mesh takes 414 ms. CGAL's whole build takes 471 ms on Linux. The order and the copy (about 90 ms) and the pass after the insertion (about 160 ms after #365) come on top.

## Delaunay D2 inserted by edge flips, PR #373 (#372)

P7-23, ADR 0007. The D = 2 shape inserts each site into its triangle (three triangles, or four on an edge) and flips the edges whose far triangle conflicts with it, as CGAL does. It fills the same mesh as the cavity, so the pass after the insertion and publication are shared:
- an edge not flipped records what its in-circle test said;
- the faces of a changed triangle are forgotten, on both sides.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `dd31cbf` against the head `ee51e54`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341), the guard sets, and the keep criterion's Delaunay `cube` and `sphere` D2 at 10^5 and 10^6, with `cube` D3 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a second |
| Phases | Copies of both sides outside the crate, with timers around `Mesh::build` and the union of cospherical simplices, and a count of the faces the union tests |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.69 ms (0.63 ms–0.98 ms) | 0.70 ms (0.61 ms–1.06 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^5 | 6.48 ms (5.72 ms–7.87 ms) | 6.18 ms (5.53 ms–7.12 ms) | 0.95, inside the spread |
| Hull `cube` D3 10^4 | 1.54 ms (1.37 ms–1.74 ms) | 1.47 ms (1.36 ms–2.53 ms) | 0.96, inside the spread |
| Hull `cube` D3 10^5 | 13.0 ms (11.5 ms–16.0 ms) | 12.7 ms (11.5 ms–14.4 ms) | 0.97, inside the spread |
| Hull `cube` D4 10^4 | 6.50 ms (6.30 ms–7.33 ms) | 6.53 ms (6.29 ms–7.58 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 35.3 ms (34.1 ms–39.8 ms) | 35.4 ms (34.3 ms–37.6 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 55.5 ms (54.3 ms–60.8 ms) | 55.9 ms (54.1 ms–58.3 ms) | 1.01, inside the spread |
| Hull `cube` D6 10^4 | 572 ms (563 ms–597 ms) | 573 ms (560 ms–602 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 104 ms (102 ms–117 ms) | 104 ms (102 ms–109 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 1.62 ms (1.53 ms–2.66 ms) | 1.63 ms (1.50 ms–2.23 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^5 | 18.7 ms (17.0 ms–20.1 ms) | 18.5 ms (17.3 ms–19.7 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^4 | 28.4 ms (27.3 ms–32.5 ms) | 28.3 ms (27.1 ms–91.3 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 330 ms (321 ms–359 ms) | 328 ms (321 ms–344 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^4 | 125 ms (122 ms–181 ms) | 125 ms (122 ms–130 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.49 s (1.49 s–1.62 s) | 1.53 s (1.49 s–1.70 s) | 1.02, inside the spread |
| Hull `sphere` D5 10^4 | 813 ms (801 ms–852 ms) | 814 ms (802 ms–831 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 8.07 ms (7.69 ms–8.90 ms) | 7.29 ms (7.04 ms–8.00 ms) | 0.90, inside the spread |
| Delaunay `cube` D2 10^5 | 82.5 ms (77.6 ms–86.0 ms) | 74.8 ms (71.5 ms–87.5 ms) | 0.91, inside the spread |
| Delaunay `cube` D2 10^6 | 851 ms (842 ms–857 ms) | 775 ms (764 ms–780 ms) | 0.91, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 43.1 ms (42.0 ms–50.0 ms) | 43.2 ms (41.6 ms–47.4 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 447 ms (434 ms–478 ms) | 445 ms (433 ms–475 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^6 | 4.80 s (4.76 s–5.64 s) | 4.84 s (4.76 s–5.58 s) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 669 ms (658 ms–693 ms) | 662 ms (653 ms–693 ms) | 0.99, inside the spread |
| Delaunay `cube` D5 10^4 | 7.51 s (7.45 s–7.61 s) | 7.48 s (7.40 s–7.66 s) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.98 ms (7.45 ms–9.37 ms) | 7.29 ms (6.85 ms–7.96 ms) | 0.91, inside the spread |
| Delaunay `sphere` D2 10^5 | 68.1 ms (66.0 ms–80.9 ms) | 60.3 ms (58.3 ms–62.9 ms) | 0.89, faster beyond the spread |
| Delaunay `sphere` D2 10^6 | 641 ms (641 ms–647 ms) | 575 ms (570 ms–633 ms) | 0.90, faster beyond the spread |
| Delaunay `sphere` D3 10^4 | 88.7 ms (87.1 ms–98.4 ms) | 88.7 ms (86.9 ms–92.4 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 954 ms (878 ms–1.01 s) | 945 ms (852 ms–964 ms) | 0.99, inside the spread |

### The mesh and the union

| Set | Mesh, `main` | Mesh, head | Union, `main` | Union, head | Faces tested, `main` | Faces tested, head |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^6 | 554 ms | 481 ms | 61.3 ms | 67.0 ms | 810,031 | 1,415,765 |
| `sphere` D2 10^6 | 436 ms | 365 ms | 34.8 ms | 36.0 ms | 279,321 | 496,599 |

One unpinned build each, for proportions.

### Reading

- **The keep criterion is met.**
  - Delaunay `cube` and `sphere` D2 10^6 read 0.91 and 0.90, and `sphere` D2 10^5 0.89, faster beyond the spread.
  - `cube` D2 10^5 read 0.91, inside the spread through one build of the head (87.5 ms against a base minimum of 77.6 ms). The first run, before the last commit, read 0.94 and 0.91 on D2 10^5 and 0.95 and 0.91 on D2 10^6.
  - Delaunay D2 10^4 read 0.90 (`cube`) and 0.91 (`sphere`), inside the spread. The other sets read 0.95 to 1.02, inside the spread.
- **The mesh gains less than the prototype of #370.** The mesh reads 0.87 (`cube`) and 0.84 (`sphere`) of the cavity's, against 0.67 and 0.65 for the prototype. The prototype held each triangle in one record of three vertices and three neighbors. The mesh keeps five arrays (vertices, neighbors, records, liveness, marks) and the records' upkeep. The first commit forgot the records of a changed triangle's faces by searching every neighbor's slot. The second, timed here, searches once a split and twice a flip, which took D2 10^6 from 0.95 and 0.91 to 0.91 and 0.90.
- **The union tests more faces but costs about the same.** It tests 1.75 times as many faces, since the flips record only the edges they test. The time grows by 1 to 6 ms, as most of the union is the walk over the cells.
- One record per triangle, as the prototype had, is where the rest of the prototype's gain lies. It belongs to a review of the mesh's layout as a whole, not to this row.

## Delaunay: one record per cell of the mesh, PR #375 (#374)

The spike P7-24. Does one record per cell speed up the Delaunay mesh? Today the mesh keeps each cell in five arrays: vertices, neighbors, face records, liveness, and marks. The rule was set in the Grill of 2026-10-10 (on #374): the mesh build faster beyond the spread on Delaunay `cube` and `sphere` D2 and D3 at 10^6, and nothing Delaunay slower beyond the spread, leads to a row that adopts it.

### Method

| Item | Value |
| --- | --- |
| convx | The head of #373 (`ead7e2b`, D = 2 by edge flips) against a copy of it outside the crate. In the copy, every cell of every shape is one record of `2k + 2` words: the vertices, the neighbors, the mark, and a word with two bits of record per face and the liveness in its top bit. rustc 1.97.1, `--release` with debug info |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()`, the two alternated per round: 10 rounds × 3 builds at D2 10^5, 8 and 6 × 1 at D3 10^5, 6 × 1 at D2 10^6, 4 and 3 × 1 at D3 10^6 |
| Published | The simplices, their neighbors, and the representatives, in published order, hashed: identical on Delaunay `cube` and `sphere` D2 10^5, D3 2 × 10^4, D4 3000 and 2000, and `grid` D2 and D3. The copy passed the Delaunay tests in debug |

### Results

| Set | Five arrays | One record | Ratio |
| --- | ---: | ---: | ---: |
| `cube` D2 10^5 | 78.7 ms (74.6–96.5) | 80.2 ms (75.5–106) | 1.02, inside the spread |
| `sphere` D2 10^5 | 60.7 ms (58.3–64.3) | 60.9 ms (59.2–71.1) | 1.00, inside the spread |
| `cube` D3 10^5 | 472 ms (463–490) | 477 ms (473–493) | 1.01, inside the spread |
| `sphere` D3 10^5 | 881 ms (868–892) | 884 ms (878–890) | 1.00, inside the spread |
| `cube` D2 10^6 | 806 ms (794–816) | 817 ms (812–822) | 1.01, inside the spread |
| `sphere` D2 10^6 | 594 ms (587–601) | 601 ms (595–609) | 1.01, inside the spread |
| `cube` D3 10^6 | 5.34 s (5.10–5.43) | 5.31 s (5.07–5.34) | 0.99, inside the spread |
| `sphere` D3 10^6 | 7.78 s (7.68–7.83) | 7.78 s (7.71–7.80) | 1.00, inside the spread |

A second copy kept the five arrays but recorded nothing on the faces: no forgetting, no records, so the pass after the insertion tests every face. It read 0.97 on `cube` D2 10^6, 1.00 on `sphere` D2 10^6, and 0.96 on `cube` D2 10^5, inside the spread.

### Reading

- **The rule is not met.** One record per cell reads 0.99 to 1.02 on all eight sets. The mesh's layout is not where its time goes. Cells are allocated in insertion order, so the cells an insertion touches are close in each of the five arrays, and the lines it reads stay in the cache.
- **The faces' records cost little.** Without them the build reads 0.96 to 1.00.
- **The gap between the flip prototype and its implementation is mostly the harness.** The prototype read 0.65 to 0.69 of the cavity's mesh (#370), the implementation 0.84 to 0.87 (#373). In the prototype's harness, the cavity's mesh of `cube` D2 10^6 took 614 ms; in the pipeline of `build()`, 554 ms. The two ratios were measured against different bases, and the prototype's base was slower.
- What follows: no row adopts a layout. The Grill planned the arrays of the pass after the insertion, and the hull's facet store, as rows after this result. They are not opened on the layout hypothesis. A row on them starts from a profile that shows memory there.

## The P7 parity run after P7-19 to P7-23, PR #379 (#378)

Due after three speed rows merged since the run of #361: P7-19 (#365), P7-21 (#369), and P7-23 (#373).

### Method

As in the run of #361: `main` at `340b7cc`, `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. "(ref.)" marks a reference that is not judged (ADR 0006).

### P7 judgement

Met: 13 of 41 (hull 9, Delaunay 4), as in the run of #361.

| Set | convx `build()` | / CGAL | / Qhull | P7 |
| --- | ---: | ---: | ---: | --- |
| Hull `cube` D2 10^4 | 0.57 ms | 1.18 | 1.00 | not met |
| Hull `cube` D2 10^5 | 5.45 ms | 1.19 | 0.95 | not met |
| Hull `cube` D2 10^6 | 76.1 ms | 1.52 | 1.25 | not met |
| Hull `cube` D3 10^4 | 1.35 ms | 0.62 | 0.90 | met |
| Hull `cube` D3 10^5 | 10.5 ms | 0.41 | 0.87 | met |
| Hull `cube` D3 10^6 | 159 ms | 0.24 | 0.94 | met |
| Hull `cube` D4 10^4 | 6.70 ms | 0.15 | 1.01 | not met |
| Hull `cube` D4 10^5 | 35.3 ms | 0.06 | 1.04 | not met |
| Hull `cube` D5 10^4 | 56.8 ms | 0.25 | 0.77 | met |
| Hull `cube` D5 10^5 | 219 ms | 0.12 | 0.66 | met |
| Hull `cube` D6 10^4 | 637 ms | 0.30 | 0.47 | met |
| Hull `cube` D6 10^5 | 2.78 s | 0.24 | 0.40 | met |
| Hull `sphere` D2 10^4 | 1.42 ms | 2.17 | 0.20 (ref.) | not met |
| Hull `sphere` D2 10^5 | 16.5 ms | 2.07 | 0.14 (ref.) | not met |
| Hull `sphere` D2 10^6 | 254 ms | 2.87 | 0.12 (ref.) | not met |
| Hull `sphere` D3 10^4 | 27.7 ms | 1.21 | 1.51 | not met |
| Hull `sphere` D3 10^5 | 359 ms | 0.82 | 1.38 | not met |
| Hull `sphere` D3 10^6 | 5.47 s | 0.61 | 1.47 | not met |
| Hull `sphere` D4 10^4 | 130 ms | 1.07 | 1.31 | not met |
| Hull `sphere` D4 10^5 | 1.73 s | 1.33 | 1.21 | not met |
| Hull `sphere` D5 10^4 | 883 ms | 0.89 | 0.99 | met |
| Hull `sphere` D5 10^5 | 11.71 s | 1.04 | 1.01 | not met |
| Hull `sphere` D6 10^4 | 7.50 s | 0.78 | 0.85 | met |
| Hull `cubesurf` D3 10^5 | 102 ms | 2.70 | 3.54 | not met |
| Hull `grid` D6 10^4 | 2.23 s | 8.61 (ref.) | 6.55 | not met |
| Delaunay `cube` D2 10^4 | 7.19 ms | 1.81 | 0.39 | not met |
| Delaunay `cube` D2 10^5 | 75.8 ms | 1.79 | 0.31 | not met |
| Delaunay `cube` D2 10^6 | 809 ms | 1.71 | 0.22 | not met |
| Delaunay `sphere` D2 10^4 | 6.41 ms | 1.60 | 0.27 (ref.) | not met |
| Delaunay `sphere` D2 10^5 | 59.0 ms | 1.81 | 0.15 (ref.) | not met |
| Delaunay `sphere` D2 10^6 | 583 ms | 1.81 | 0.05 (ref.) | not met |
| Delaunay `cube` D3 10^4 | 46.6 ms | 1.57 | 0.49 | not met |
| Delaunay `cube` D3 10^5 | 518 ms | 1.65 | 0.37 | not met |
| Delaunay `cube` D3 10^6 | 5.65 s | 1.62 | 0.33 | not met |
| Delaunay `sphere` D3 10^4 | 79.4 ms | 0.62 | 0.66 (ref.) | met |
| Delaunay `sphere` D3 10^5 | 752 ms | 0.84 | 0.38 (ref.) | met |
| Delaunay `sphere` D3 10^6 | 6.48 s | 1.04 | 0.35 (ref.) | not met |
| Delaunay `cube` D4 10^4 | 717 ms | 1.16 | 0.89 | not met |
| Delaunay `sphere` D4 10^4 | 926 ms | 0.37 | 0.69 (ref.) | met |
| Delaunay `cube` D5 10^4 | 7.44 s | 1.10 | 0.87 | not met |
| Delaunay `sphere` D5 10^4 | 11.82 s | 0.35 | 0.77 (ref.) | met |

### What moved

- **Delaunay D2:**
  - `cube` reads 1.71 to 1.81 against CGAL, from 2.01 to 2.29 in the run of #361.
  - `sphere` reads 1.60 to 1.81, from 1.96 to 2.62.
  - The three rows that changed it: the pass after the insertion (#365), the sites in insertion order (#369), and the insertion by edge flips (#373).
- **Delaunay `sphere` D3 10^6** reads 1.04, from 1.10. `cube` D3 reads 1.57 to 1.65, from 1.60 to 1.86.
- **Hull `sphere` D5 10^4** reads 0.99 against Qhull and is met; **hull `cube` D4 10^4** reads 1.01 and is not met. Neither path changed: these sets lie within a few percent of 1.00, where runs differ (#361).

## Hull D2: the chain's points sorted by keys read once, PR #380 (#322)

P7-7. The D = 2 hull sorted the points kept for its chain with a stable sort whose comparison read two input points. On hull `sphere` D2, where no point is discarded, that sort was 24 to 31% of `build()` (the profile on #322). Each point is now read once into keys that order as `f64::total_cmp` does. The keys and the index are sorted unstably, and the index makes them unique, so the order is the same. The chains then read the sorted coordinates in order.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `340b7cc` against the head `c6b6fcf`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341), the guard sets, and the keep criterion's hull `sphere` D2 10^5 and 10^6, with `cube` D2 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a second and for D2 10^6. Second runs: hull `sphere` D2 10^5 20 × 3 and `cube` D2 10^6 10 × 1; and all three D2 sets built with one codegen unit on each side |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.67 ms (0.60 ms–1.10 ms) | 0.68 ms (0.61 ms–1.04 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^5 | 6.04 ms (5.56 ms–7.26 ms) | 6.25 ms (5.71 ms–7.63 ms) | 1.04, inside the spread |
| Hull `cube` D2 10^6 | 63.6 ms (63.0 ms–64.1 ms) | 67.5 ms (67.0 ms–70.8 ms) | 1.06, slower beyond the spread |
| Hull `cube` D3 10^4 | 1.59 ms (1.34 ms–1.95 ms) | 1.61 ms (1.36 ms–2.65 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^5 | 12.6 ms (11.8 ms–14.2 ms) | 12.6 ms (11.2 ms–16.9 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 6.62 ms (6.32 ms–8.17 ms) | 6.68 ms (6.28 ms–7.79 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 35.3 ms (34.6 ms–48.6 ms) | 35.8 ms (34.6 ms–39.8 ms) | 1.01, inside the spread |
| Hull `cube` D5 10^4 | 55.7 ms (54.2 ms–62.7 ms) | 55.9 ms (54.8 ms–59.7 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 577 ms (566 ms–597 ms) | 581 ms (572 ms–594 ms) | 1.01, inside the spread |
| Hull `cubesurf` D3 10^5 | 105 ms (103 ms–112 ms) | 106 ms (103 ms–116 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^4 | 1.60 ms (1.52 ms–7.13 ms) | 1.51 ms (1.40 ms–1.94 ms) | 0.94, inside the spread |
| Hull `sphere` D2 10^5 | 18.8 ms (17.6 ms–21.7 ms) | 17.1 ms (15.6 ms–18.9 ms) | 0.91, inside the spread |
| Hull `sphere` D2 10^6 | 207 ms (204 ms–212 ms) | 166 ms (165 ms–168 ms) | 0.80, faster beyond the spread |
| Hull `sphere` D3 10^4 | 28.7 ms (27.2 ms–36.9 ms) | 28.2 ms (27.2 ms–40.3 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 331 ms (320 ms–345 ms) | 334 ms (326 ms–365 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^4 | 126 ms (123 ms–134 ms) | 128 ms (123 ms–139 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.49 s (1.49 s–1.51 s) | 1.50 s (1.47 s–1.52 s) | 1.01, inside the spread |
| Hull `sphere` D5 10^4 | 821 ms (803 ms–842 ms) | 830 ms (808 ms–906 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 7.41 ms (7.09 ms–8.33 ms) | 7.39 ms (7.02 ms–7.98 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 73.7 ms (72.0 ms–80.1 ms) | 73.4 ms (71.7 ms–84.0 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^4 | 41.8 ms (40.7 ms–44.5 ms) | 42.0 ms (40.7 ms–46.6 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 447 ms (440 ms–473 ms) | 443 ms (436 ms–458 ms) | 0.99, inside the spread |
| Delaunay `cube` D4 10^4 | 660 ms (646 ms–704 ms) | 679 ms (673 ms–692 ms) | 1.03, inside the spread |
| Delaunay `cube` D5 10^4 | 7.33 s (7.24 s–7.36 s) | 7.34 s (7.28 s–7.39 s) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.00 ms (6.75 ms–7.61 ms) | 6.90 ms (6.77 ms–8.72 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^5 | 58.5 ms (57.0 ms–65.7 ms) | 58.3 ms (57.2 ms–61.8 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 88.2 ms (86.2 ms–92.5 ms) | 88.1 ms (86.2 ms–90.5 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 841 ms (834 ms–850 ms) | 835 ms (831 ms–847 ms) | 0.99, inside the spread |

The second runs:

| Set | Default build | One codegen unit |
| --- | --- | --- |
| hull `sphere` D2 10^5 | 0.90: 18.7 ms (17.2–25.5) against 16.8 ms (15.6–22.9), inside the spread through the head's slowest builds | 0.90, inside the spread |
| hull `sphere` D2 10^6 | — | 0.77: 237 ms against 181 ms, faster beyond the spread |
| hull `cube` D2 10^6 | 1.02: 72.7 ms against 74.1 ms, inside the spread | 1.03, inside the spread |

### Reading

- **The keep criterion is met.**
  - Hull `sphere` D2 10^6 reads 0.80, and 0.77 with one codegen unit, faster beyond the spread.
  - Hull `sphere` D2 10^5 reads 0.90 or 0.91 in four runs, each inside the spread through single slow builds, so the difference is reported (`bench.mdc`).
- **Hull `cube` D2 10^6 read 1.06 in the first run, then 1.02 and 1.03.** On `cube` the discard leaves few points to sort, and the profile put the sort at 4% of `build()` there. With one codegen unit it reads 1.03, inside the spread, so the first run is read as code generation and run-to-run noise.
- The other sets: 0.94 to 1.04, inside the spread.

## Hull D2: the discard against edges gathered once, PR #381 (#324)

P7-9. Before the D = 2 chain, every representative is tested against the polygon of the eight directional extremes, one certified orientation per edge (design §6). Each test read the edge's two ends from the input again. On hull `cube` D2 that test was 32 to 41% of `build()` (the profile on #324). The edges' coordinates are now gathered once. The same certified orientation decides, so the same points are discarded.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `340b7cc` against the head `30d8076`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md` (#341), the guard sets, and the keep criterion's hull `cube` D2 10^5 and 10^6, with `sphere` D2 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a second and for D2 10^6. A second run of hull `cube` D2 10^5, 20 × 3 |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.70 ms (0.60 ms–0.87 ms) | 0.61 ms (0.52 ms–0.80 ms) | 0.87, inside the spread |
| Hull `cube` D2 10^5 | 6.28 ms (5.54 ms–7.36 ms) | 5.29 ms (4.74 ms–6.95 ms) | 0.84, inside the spread |
| Hull `cube` D2 10^6 | 65.4 ms (65.0 ms–66.9 ms) | 58.5 ms (56.4 ms–59.4 ms) | 0.90, faster beyond the spread |
| Hull `cube` D3 10^4 | 1.63 ms (1.37 ms–2.43 ms) | 1.62 ms (1.34 ms–2.10 ms) | 0.99, inside the spread |
| Hull `cube` D3 10^5 | 12.7 ms (11.8 ms–19.3 ms) | 13.0 ms (11.5 ms–25.4 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^4 | 6.55 ms (6.32 ms–9.07 ms) | 6.52 ms (6.34 ms–8.49 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 35.8 ms (34.1 ms–39.8 ms) | 35.3 ms (34.2 ms–36.9 ms) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 55.4 ms (54.1 ms–63.7 ms) | 55.7 ms (54.5 ms–63.2 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 578 ms (568 ms–604 ms) | 580 ms (571 ms–613 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 104 ms (101 ms–114 ms) | 103 ms (100 ms–114 ms) | 0.99, inside the spread |
| Hull `sphere` D2 10^4 | 1.65 ms (1.50 ms–2.03 ms) | 1.68 ms (1.52 ms–2.64 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^5 | 18.3 ms (17.0 ms–20.3 ms) | 18.5 ms (17.1 ms–21.4 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^6 | 212 ms (208 ms–235 ms) | 211 ms (208 ms–217 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^4 | 28.9 ms (28.2 ms–31.0 ms) | 28.8 ms (27.8 ms–30.1 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^5 | 335 ms (325 ms–381 ms) | 334 ms (324 ms–351 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 126 ms (123 ms–134 ms) | 126 ms (123 ms–145 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.47 s (1.45 s–1.48 s) | 1.47 s (1.46 s–1.48 s) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 821 ms (800 ms–893 ms) | 828 ms (815 ms–882 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 7.40 ms (7.09 ms–9.60 ms) | 7.36 ms (7.02 ms–8.54 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 74.5 ms (71.8 ms–80.3 ms) | 72.9 ms (71.1 ms–78.0 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^4 | 42.1 ms (40.8 ms–77.2 ms) | 41.7 ms (40.9 ms–43.0 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 444 ms (441 ms–491 ms) | 448 ms (439 ms–483 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 649 ms (638 ms–665 ms) | 658 ms (638 ms–674 ms) | 1.01, inside the spread |
| Delaunay `cube` D5 10^4 | 7.41 s (7.28 s–7.66 s) | 7.47 s (7.34 s–7.75 s) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^4 | 6.90 ms (6.66 ms–7.64 ms) | 6.91 ms (6.68 ms–7.61 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 58.2 ms (56.5 ms–61.2 ms) | 58.2 ms (56.4 ms–61.8 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 89.1 ms (86.5 ms–94.4 ms) | 88.5 ms (86.0 ms–92.1 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^5 | 837 ms (826 ms–841 ms) | 833 ms (827 ms–838 ms) | 1.00, inside the spread |

The second run of hull `cube` D2 10^5, 20 × 3: 6.18 ms (5.46–10.05) against 5.26 ms (4.71–12.85), 0.85.

### Reading

- **The keep criterion is met.**
  - Hull `cube` D2 10^6 reads 0.90, faster beyond the spread.
  - Hull `cube` D2 10^5 reads 0.84 and 0.85 in two runs, its ranges overlapping only through single slow builds, so the difference is reported (`bench.mdc`).
  - Hull `cube` D2 10^4 reads 0.87 too.
- **Nothing is slower beyond the spread.** Hull `sphere` D2 reads 0.99 to 1.02: there the sample finds nothing inside and no point is tested. The other sets read 0.98 to 1.02.

## Hull `cubesurf` and `grid`: where the time goes against Qhull, PR #386 (#368)

The spike P7-25 asks where the time goes on the two degenerate hull sets, the furthest from parity (#379): `cubesurf` D3 10^5 is 2.70 against CGAL and 3.54 against Qhull; `grid` D6 10^4 is 6.55 against Qhull. It also asks how much of `cubesurf`'s spread is code generation (#365).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `340b7cc`. VTune hotspots, software sampling, one pinned core: 20 builds of `cubesurf` D3 10^5, 3 of `grid` D6 10^4. A copy outside the crate that counts every plain orientation, those with a column of differences that is all zero, and those whose exact sign is zero. A second copy that answers zero for such a column, after the first stage and before the later stages |
| Qhull | The work counters of the parity run of #379 (`qconvex Qt Ts`): hyperplanes and distance tests |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2); Qhull under WSL2 on the same machine |
| Timed | `build()`, `main` and the copy alternated per round: 8 rounds × 3 builds of `cubesurf` D3 10^5, `cube` D3 10^5; 3 × 1 of `grid` D6 10^4; 4 × 1 of `sphere` D3 10^5; 6 × 3 of Delaunay `cube` D2 10^5. `main` built with one codegen unit against the default, 10 × 3 on `cubesurf` |

### Where the time goes

| Set | Orientations | Exactly zero | Zero with a column of zeros | Profile |
| --- | ---: | ---: | ---: | --- |
| `cubesurf` D3 10^5 | 110,187 | 100,308 | 100,308 | the orientations 32% of `build()`: the exact stage 19%, double-double 8%; the classification after construction about 38% |
| `grid` D6 10^4 | 1,666,863 | 561,584 | 483,662 | the running filter of k = 6 57% of `build()` |
| `cube` D3 10^5 | 7 | 0 | 0 | |
| `sphere` D3 10^5 | 7 | 0 | 0 | |

| Set | convx | Qhull: hyperplanes, distance tests | Qhull's time |
| --- | ---: | --- | ---: |
| `cubesurf` D3 10^5 | 102 ms | 501, 2,236,537 | 28.8 ms |
| `grid` D6 10^4 | 2.23 s | 14,992, 8,004,186 | 340 ms |

### Prototype: a zero column decides zero

| Set | `main` | Prototype | Ratio |
| --- | ---: | ---: | ---: |
| hull `cubesurf` D3 10^5 | 114 ms (110–126) | 83.6 ms (81.0–91.3) | 0.73, faster beyond the spread |
| hull `grid` D6 10^4 | 2.49 s (2.49–2.51) | 2.13 s (2.12–2.14) | 0.85, faster beyond the spread |
| hull `cube` D3 10^5 | 12.5 ms (11.6–14.0) | 12.8 ms (11.3–16.4) | 1.03, inside the spread |
| hull `sphere` D3 10^5 | 366 ms (365–367) | 371 ms (366–379) | 1.01, inside the spread |
| Delaunay `cube` D2 10^5 | 81.8 ms (79.1–87.8) | 79.5 ms (77.9–85.8) | 0.97, inside the spread |

With one codegen unit, `main`'s `cubesurf` D3 10^5 read 0.99 of the default build (114 ms against 113 ms), inside the spread.

### Reading

- **Most of the degenerate work is exact zeros on hyperplanes parallel to the axes.** On `cubesurf` every zero has a column of differences that is all zero, and on `grid` 86% of them. `a - b` is zero in `f64` exactly when `a == b`, so such a column is exact and the determinant is zero. A bound can never certify a zero, so each of them ran the later stages. Answering zero for that column reads `cubesurf` 0.73 and `grid` 0.85. General inputs never reach it.
- **`grid`'s gap is the cost of a side test.** Qhull's 8.0 million distance tests take 340 ms, about 40 ns each, a dot product with a hyperplane computed once per facet. convx's 1.7 million orientations take about 2.2 s, a 6 × 6 determinant each. The `grid` coordinates are small integers, so each facet's hyperplane is exact in `f64`, and a dot product with it would give the exact side, zero included.
- **`cubesurf`'s spread is not code generation on `main`.** It read 0.99 with one codegen unit; the 1.11 to 1.14 of #365 came from that change's partition.
- After the zero column, `cubesurf`'s classification after construction (about 38%) is next.

### Rows that follow

| Row | Kind | What |
| --- | --- | --- |
| P7-26 (#384) | Feat | Predicates: a zero column of differences decides zero before the later stages |
| P7-27 (#385) | Spike | Hull: exact planes of integer inputs, side tests by a dot product |

## Predicates: a zero column decides zero, PR #389 (#384)

P7-26. On the degenerate hull sets, most orientations that reach the later stages are exactly zero, with a coordinate column whose differences are all zero (P7-25, #386). A bound never certifies a zero, so each of them ran double-double, or the running filter at k = 6, and then the exact stage. `a - b` is zero in `f64` exactly when `a == b`, so such a column, with the direction row's entry zero too, makes the determinant exactly zero. The check runs after the first stage and before every later stage (design §1). A sign the first stage certifies costs nothing more.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `c8998c7` against the head `0cf94e9`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2) |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `grid` D6 10^4 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a second |

Every set published the same counts on both sides.

After this measurement, the branch merged `main` (`77fb3c2`), which brought #380 and #381 (the hull D2 chain and its discard) and #388 (a test). The shipped head `4ffb02c` was not timed again: the sets of the keep criterion are D3 and D6 and do not run the D2 code, and #388 changes a test only.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.69 ms (0.61 ms–0.80 ms) | 0.70 ms (0.62 ms–2.00 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^5 | 6.23 ms (5.61 ms–7.01 ms) | 6.31 ms (5.76 ms–7.35 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^4 | 1.56 ms (1.35 ms–1.95 ms) | 1.59 ms (1.34 ms–2.26 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^5 | 12.3 ms (11.5 ms–14.9 ms) | 12.1 ms (11.2 ms–15.4 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^4 | 6.48 ms (6.23 ms–7.65 ms) | 6.70 ms (6.22 ms–7.51 ms) | 1.03, inside the spread |
| Hull `cube` D4 10^5 | 34.9 ms (33.8 ms–39.3 ms) | 34.8 ms (33.9 ms–37.9 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 56.1 ms (54.5 ms–63.4 ms) | 57.7 ms (55.1 ms–63.0 ms) | 1.03, inside the spread |
| Hull `cube` D6 10^4 | 574 ms (564 ms–605 ms) | 576 ms (564 ms–593 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 106 ms (103 ms–112 ms) | 78.0 ms (76.9 ms–86.3 ms) | 0.73, faster beyond the spread |
| Hull `grid` D6 10^4 | 2.25 s (2.25 s–2.26 s) | 1.94 s (1.94 s–1.97 s) | 0.86, faster beyond the spread |
| Hull `sphere` D2 10^4 | 1.58 ms (1.52 ms–2.08 ms) | 1.69 ms (1.50 ms–2.30 ms) | 1.07, inside the spread |
| Hull `sphere` D2 10^5 | 18.0 ms (16.9 ms–20.9 ms) | 17.6 ms (16.7 ms–21.9 ms) | 0.98, inside the spread |
| Hull `sphere` D3 10^4 | 29.5 ms (27.5 ms–38.9 ms) | 29.6 ms (27.6 ms–44.1 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^5 | 325 ms (315 ms–373 ms) | 332 ms (318 ms–374 ms) | 1.02, inside the spread |
| Hull `sphere` D4 10^4 | 126 ms (123 ms–140 ms) | 127 ms (122 ms–138 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.76 s (1.73 s–1.91 s) | 1.73 s (1.69 s–1.82 s) | 0.98, inside the spread |
| Hull `sphere` D5 10^4 | 827 ms (790 ms–855 ms) | 826 ms (794 ms–873 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 7.44 ms (7.15 ms–8.16 ms) | 7.43 ms (7.10 ms–8.08 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 73.9 ms (71.9 ms–78.8 ms) | 74.8 ms (72.4 ms–84.7 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 42.4 ms (41.3 ms–44.4 ms) | 42.4 ms (41.3 ms–47.1 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 454 ms (441 ms–476 ms) | 454 ms (442 ms–515 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 757 ms (657 ms–833 ms) | 802 ms (695 ms–1.01 s) | 1.06, inside the spread |
| Delaunay `cube` D5 10^4 | 7.50 s (7.46 s–8.94 s) | 7.66 s (7.43 s–8.67 s) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^4 | 6.94 ms (6.74 ms–9.03 ms) | 7.00 ms (6.74 ms–7.88 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^5 | 58.7 ms (56.9 ms–67.8 ms) | 58.7 ms (57.8 ms–61.1 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 89.1 ms (87.4 ms–100 ms) | 89.0 ms (86.7 ms–98.8 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 981 ms (946 ms–1.08 s) | 971 ms (938 ms–1.05 s) | 0.99, inside the spread |

### Reading

- **The keep criterion is met.** Hull `cubesurf` D3 10^5 reads 0.73 and `grid` D6 10^4 0.86, both faster beyond the spread, as in the prototype of P7-25 (0.73 and 0.85).
- **Nothing is slower beyond the spread.** Delaunay `cube` D4 10^4 reads 1.06 and hull `sphere` D2 10^4 1.07, each inside the spread, its range overlapping `main`'s. The other sets read 0.98 to 1.03.
- **`grid`'s remaining gap** is the cost of a side test against Qhull's dot product, the spike P7-27 (#385).

## Hull `grid`: missing cull planes, exact planes, and classification, PR #395 (#385)

The spike P7-27 asks how many of hull `grid` D6 10^4's side tests a facet with an exact plane could decide, what such a test costs against the orientation, and which rows follow. The reading of P7-25 (#386) put `grid`'s gap in the cost of each side test. The counts below place it elsewhere: most of the side tests that reached the orientation were on facets with no cull plane at all.

### Method

| Item | Value |
| --- | --- |
| convx | The head of #389 (`4ffb02c`: `main` at `f7b4968` with P7-26), copied outside the crate three times: the base; prototype A, a cull plane from the exact direction where the filtered cofactors fail; prototype A + B, exact integer planes on top. A fourth copy carried counters and phase timers, with the prototypes switched at run time |
| Prototype A | When `direction_cofactors` returns `None`, the working normal is the exact cofactor direction, rounded once, within `D · 2^-49` of the exact unit direction (design §1). The cull plane takes `tau` from that error, in the threshold of `src/cull.rs` |
| Prototype B | When every input coordinate is an integer of magnitude `M ≤ 2^20`, each facet's cofactors are computed exactly by fraction-free elimination in `i128`. When `2 M Σ |c_j| ≤ 2^52`, every sum `Σ (x_j − o_j) c_j` is exact in `f64`. The facet then decides every side, zero included, by that sum, in the scan and in single tests |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2). rustc 1.97.1, `--release` with debug info |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1. The base and A alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A and A + B the same way, on the integer sets |
| Runs | Both timings ran with nothing else on the machine. An earlier pair of runs overlapped an unpinned comparison loop and was discarded; its ratios agreed |
| Checked | The published hull's vertices and facet vertex sets, hashed, the same for the base, A, and A + B: every family of the generator, D = 2 to 7, 10^3 and 10^4 points (D = 7 at 10^3; D ≥ 6 at 10^4 for the integer families only). The crate's suite in debug with A, where every proved side is checked against the orientation; A + B in debug on hull `grid` and `lattice` D5 10^4 and D6 2,000, and `onsphere` D5 10^4 |

### Where the side tests went

Hull `grid` D6 10^4, counted in one build:

| | Base | A | A + B |
| --- | ---: | ---: | ---: |
| Working planes without filtered cofactors | 9,139 of 77,304 | 0 of 68,165 | 0 of 68,165 |
| Side tests that reached the orientation | 935,808 | 350,577 | 0 |
| of which on a facet with no cull plane | 656,841 | 0 | 0 |
| of which zero | 350,577 | 350,577 | 0 |
| Sides decided by an exact plane | – | – | 3,592,105 |
| Orientations in `build()` | 1,154,911 | 569,680 | 219,096 |
| Construction, one build | 1,416 ms | 127 ms | 88 ms |
| Classification, one build | 677 ms | 512 ms | 489 ms |

The orientations left with A + B are all in classification, in the hulls it builds of each facet's points one dimension down. With A + B, classification spent:

- 366 ms finding the extreme points of the facets. The recursion built 12 hulls in D5, 120 in D4, 960 in D3, and 5,760 in D2, taking 70, 47, 51, and 20 ms. The 6-cube has 60 faces of dimension 4, 160 of dimension 3, and 240 of dimension 2, so the recursion finds each lower face many times over.
- 243 ms in the placing triangulation, at all depths. It rebuilds a map of every ridge of every simplex for each point it places.

With A, the same phase timers on other inputs:

| Set | `build()` | Construction | Placing |
| --- | ---: | ---: | ---: |
| Hull `lattice` D6 10^4 | 2.16 s | 166 ms | about 1.47 s |
| Hull `cubesurf` D5 10^4 | 2.04 s | 307 ms | 1.45 s |
| Hull `cubesurf` D3 10^5 | 117 ms | 43 ms | 2 ms; `distance_zeros` 64 ms |

### A against the base

| Set | Base | A | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cluster` D5 10^4 | 928 ms (908 ms–937 ms) | 790 ms (771 ms–809 ms) | 0.85, faster beyond the spread |
| Hull `cube` D2 10^4 | 0.59 ms (0.52 ms–0.90 ms) | 0.60 ms (0.53 ms–1.12 ms) | 1.01, inside the spread |
| Hull `cube` D2 10^5 | 5.52 ms (4.96 ms–7.73 ms) | 5.50 ms (4.74 ms–7.11 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^4 | 1.49 ms (1.32 ms–2.45 ms) | 1.46 ms (1.35 ms–2.12 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 12.2 ms (11.2 ms–13.4 ms) | 12.0 ms (11.1 ms–13.2 ms) | 0.99, inside the spread |
| Hull `cube` D4 10^4 | 6.78 ms (6.30 ms–11.3 ms) | 6.95 ms (6.37 ms–8.34 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^5 | 37.4 ms (35.6 ms–39.9 ms) | 37.2 ms (35.0 ms–39.7 ms) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 58.7 ms (55.6 ms–67.3 ms) | 59.8 ms (55.6 ms–64.4 ms) | 1.02, inside the spread |
| Hull `cube` D6 10^4 | 644 ms (618 ms–664 ms) | 640 ms (621 ms–707 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 81.9 ms (78.2 ms–94.0 ms) | 82.1 ms (77.3 ms–84.3 ms) | 1.00, inside the spread |
| Hull `grid` D5 10^4 | 32.3 ms (30.0 ms–35.5 ms) | 29.1 ms (26.8 ms–31.9 ms) | 0.90, inside the spread |
| Hull `grid` D6 10^4 | 2.00 s (1.98 s–2.01 s) | 598 ms (587 ms–641 ms) | 0.30, faster beyond the spread |
| Hull `lattice` D5 10^4 | 58.2 ms (57.0 ms–61.0 ms) | 48.6 ms (46.7 ms–52.1 ms) | 0.83, faster beyond the spread |
| Hull `lattice` D6 10^4 | 4.70 s (4.67 s–4.86 s) | 1.93 s (1.90 s–1.97 s) | 0.41, faster beyond the spread |
| Hull `nearsphere` D5 10^4 | 524 ms (518 ms–544 ms) | 501 ms (484 ms–512 ms) | 0.96, faster beyond the spread |
| Hull `onsphere` D5 10^4 | 38.6 ms (36.1 ms–44.0 ms) | 38.3 ms (36.2 ms–40.9 ms) | 0.99, inside the spread |
| Hull `sphere` D2 10^4 | 1.55 ms (1.41 ms–20.4 ms) | 1.53 ms (1.39 ms–2.30 ms) | 0.99, inside the spread |
| Hull `sphere` D2 10^5 | 16.4 ms (14.9 ms–17.9 ms) | 16.3 ms (15.0 ms–18.2 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 30.1 ms (28.0 ms–32.7 ms) | 30.2 ms (28.3 ms–32.8 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^5 | 366 ms (350 ms–432 ms) | 360 ms (344 ms–374 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^4 | 134 ms (128 ms–141 ms) | 132 ms (128 ms–143 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^5 | 1.68 s (1.67 s–1.70 s) | 1.67 s (1.66 s–1.68 s) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 888 ms (856 ms–992 ms) | 884 ms (850 ms–954 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 7.98 ms (7.40 ms–10.4 ms) | 7.99 ms (7.46 ms–32.3 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 82.9 ms (75.4 ms–95.9 ms) | 81.4 ms (75.9 ms–86.0 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^4 | 46.8 ms (43.2 ms–53.0 ms) | 46.1 ms (43.8 ms–54.2 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 501 ms (486 ms–550 ms) | 506 ms (487 ms–538 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 727 ms (707 ms–753 ms) | 724 ms (704 ms–776 ms) | 1.00, inside the spread |
| Delaunay `cube` D5 10^4 | 7.98 s (7.83 s–8.55 s) | 7.99 s (7.86 s–8.24 s) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.39 ms (6.93 ms–9.16 ms) | 7.40 ms (6.90 ms–9.59 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 63.4 ms (58.3 ms–77.9 ms) | 63.4 ms (60.1 ms–74.4 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 96.5 ms (92.2 ms–104 ms) | 96.5 ms (94.0 ms–112 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 918 ms (903 ms–928 ms) | 910 ms (901 ms–927 ms) | 0.99, inside the spread |

### A + B against A

| Set | A | A + B | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D3 10^5 | 11.6 ms (10.9 ms–13.9 ms) | 11.4 ms (10.9 ms–21.6 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 588 ms (560 ms–666 ms) | 617 ms (590 ms–685 ms) | 1.05, inside the spread |
| Hull `grid` D3 10^5 | 1.39 ms (1.20 ms–1.75 ms) | 2.13 ms (1.92 ms–2.42 ms) | 1.54, slower beyond the spread |
| Hull `grid` D4 10^4 | 2.10 ms (1.85 ms–2.97 ms) | 1.94 ms (1.76 ms–3.62 ms) | 0.92, inside the spread |
| Hull `grid` D5 10^4 | 27.3 ms (26.1 ms–29.9 ms) | 24.6 ms (23.4 ms–26.9 ms) | 0.90, inside the spread |
| Hull `grid` D6 10^4 | 595 ms (582 ms–608 ms) | 520 ms (519 ms–528 ms) | 0.88, faster beyond the spread |
| Hull `lattice` D3 10^5 | 1.47 ms (1.35 ms–1.85 ms) | 2.25 ms (2.03 ms–2.89 ms) | 1.54, slower beyond the spread |
| Hull `lattice` D4 10^4 | 2.66 ms (2.53 ms–3.29 ms) | 2.30 ms (2.17 ms–2.74 ms) | 0.86, inside the spread |
| Hull `lattice` D5 10^4 | 45.0 ms (43.1 ms–50.5 ms) | 36.5 ms (34.9 ms–39.4 ms) | 0.81, faster beyond the spread |
| Hull `lattice` D6 10^4 | 1.95 s (1.94 s–2.00 s) | 1.96 s (1.93 s–1.97 s) | 1.01, inside the spread |
| Hull `onsphere` D4 10^4 | 1.07 ms (0.99 ms–1.30 ms) | 1.31 ms (1.26 ms–1.87 ms) | 1.22, inside the spread |
| Hull `onsphere` D5 10^4 | 35.7 ms (34.4 ms–43.2 ms) | 41.2 ms (39.6 ms–46.9 ms) | 1.15, inside the spread |

### Reading

- **Missing cull planes were most of `grid`'s gap.**
  - On integer edges in D ≥ 5, the filtered elimination meets pivots that are exactly zero and cannot certify them. 12% of the working planes then had no cull plane, and every point tested against them ran a 7 × 7 orientation.
  - A certifies the plane from the exact direction the facet already computes. It reads hull `grid` D6 10^4 0.30 (2.00 s to 598 ms, against Qhull's 340 ms in #379), `lattice` D6 10^4 0.41, `lattice` D5 0.83, `cluster` D5 0.85, and `nearsphere` D5 0.96, all faster beyond the spread.
  - Every other set is inside the spread, with the same counts. That is the row P7-28 (#392).
- **Exact planes decide the rest of the side tests, but add little.**
  - B leaves no orientation in construction. It reads `grid` D6 0.88 and `lattice` D5 0.81 against A, faster beyond the spread, and `grid` D4 and D5 and `lattice` D4 0.86 to 0.92.
  - It costs a pass over the input and an exact elimination per facet. `grid` and `lattice` D3 10^5 read 1.54, slower beyond the spread, and `onsphere` D4 and D5 1.15 to 1.22. `cube` D6 reads 1.05, from the wider plane rows of the store.
  - After A, construction is 88 to 127 ms of `grid`'s build, and classification is the rest. No row is proposed for B.
- **Classification is what remains on the degenerate sets.**
  - Placing is quadratic in the points of a face: about 1.47 s of `lattice` D6, 1.45 s of `cubesurf` D5 10^4, and 243 ms of `grid` D6. That is P7-29 (#393).
  - The recursion finds each lower face again from every facet that contains it: about 200 ms of `grid` D6. How a face is recognized across recursions branches the design, so P7-30 (#394) is set after its Grill.
  - On `cubesurf` D3 10^5, the remaining classification is the walk of `distance_zeros` over the recorded points, 64 ms; it is left to the parity run after P7-28.

## Hull: a cull plane from the exact direction, PR #399 (#392)

P7-28. A facet whose working normal is the exact cofactor direction, rounded once, now has a cull plane: the direction's error bound `D · 2^-49` is the plane's `tau` (design §1). Two kinds of facet take that path. Those whose filtered elimination cannot certify a pivot, as on integer edges with a pivot that is exactly zero, had no cull plane before. Those whose filtered cofactors certify the direction only loosely (above `10^-10`) had one bounded by those loose cofactors. Every certified working normal now has a cull plane, so the facet store no longer has a state with a normal and no plane, and a side test reads one plane where it read two (the normal and the cull plane).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `219b274` against the head `bb68a8e`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `grid` D5 and D6, `lattice` D5 and D6, `cluster` D5, and `nearsphere` D5 10^4 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |
| Counts | A copy of the head with counters (not committed): the working planes by certificate, and the orientations of one build |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cluster` D5 10^4 | 989 ms (931 ms–1.01 s) | 575 ms (564 ms–583 ms) | 0.58, faster beyond the spread |
| Hull `cube` D2 10^4 | 0.60 ms (0.51 ms–0.71 ms) | 0.60 ms (0.52 ms–0.90 ms) | 1.01, inside the spread |
| Hull `cube` D2 10^5 | 5.15 ms (4.72 ms–6.37 ms) | 5.18 ms (4.70 ms–5.92 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^4 | 1.54 ms (1.34 ms–1.98 ms) | 1.56 ms (1.31 ms–2.21 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^5 | 12.4 ms (11.5 ms–17.1 ms) | 12.4 ms (11.3 ms–14.1 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 6.69 ms (6.27 ms–7.79 ms) | 6.52 ms (6.21 ms–7.88 ms) | 0.97, inside the spread |
| Hull `cube` D4 10^5 | 35.4 ms (34.4 ms–39.9 ms) | 35.6 ms (34.4 ms–37.9 ms) | 1.01, inside the spread |
| Hull `cube` D5 10^4 | 56.0 ms (54.5 ms–63.8 ms) | 55.7 ms (53.8 ms–59.8 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 579 ms (564 ms–593 ms) | 573 ms (565 ms–591 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 77.1 ms (75.7 ms–83.4 ms) | 61.0 ms (59.6 ms–64.0 ms) | 0.79, faster beyond the spread |
| Hull `grid` D5 10^4 | 29.2 ms (28.6 ms–31.6 ms) | 26.1 ms (25.6 ms–27.7 ms) | 0.89, faster beyond the spread |
| Hull `grid` D6 10^4 | 1.95 s (1.94 s–1.96 s) | 584 ms (581 ms–586 ms) | 0.30, faster beyond the spread |
| Hull `lattice` D5 10^4 | 53.6 ms (52.2 ms–70.3 ms) | 43.2 ms (42.8 ms–45.1 ms) | 0.81, faster beyond the spread |
| Hull `lattice` D6 10^4 | 4.67 s (4.64 s–4.68 s) | 1.95 s (1.95 s–1.96 s) | 0.42, faster beyond the spread |
| Hull `nearsphere` D5 10^4 | 564 ms (558 ms–579 ms) | 533 ms (526 ms–541 ms) | 0.95, faster beyond the spread |
| Hull `sphere` D2 10^4 | 1.55 ms (1.39 ms–2.10 ms) | 1.51 ms (1.38 ms–1.90 ms) | 0.98, inside the spread |
| Hull `sphere` D2 10^5 | 16.6 ms (15.0 ms–18.1 ms) | 16.2 ms (14.9 ms–17.9 ms) | 0.98, inside the spread |
| Hull `sphere` D3 10^4 | 31.7 ms (29.6 ms–46.6 ms) | 30.1 ms (27.8 ms–43.1 ms) | 0.95, inside the spread |
| Hull `sphere` D3 10^5 | 334 ms (326 ms–362 ms) | 327 ms (321 ms–345 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^4 | 127 ms (124 ms–141 ms) | 124 ms (122 ms–131 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^5 | 1.48 s (1.47 s–1.50 s) | 1.45 s (1.44 s–1.47 s) | 0.98, inside the spread |
| Hull `sphere` D5 10^4 | 823 ms (808 ms–854 ms) | 815 ms (802 ms–844 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 7.32 ms (7.11 ms–11.4 ms) | 7.43 ms (7.12 ms–8.24 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^5 | 72.9 ms (71.3 ms–77.5 ms) | 74.0 ms (72.1 ms–80.7 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 42.0 ms (41.0 ms–45.0 ms) | 42.1 ms (40.7 ms–45.0 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 446 ms (439 ms–462 ms) | 447 ms (436 ms–466 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 659 ms (649 ms–675 ms) | 671 ms (658 ms–689 ms) | 1.02, inside the spread |
| Delaunay `cube` D5 10^4 | 7.56 s (7.41 s–7.82 s) | 7.56 s (7.47 s–7.62 s) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.04 ms (6.71 ms–8.43 ms) | 7.07 ms (6.85 ms–7.98 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^5 | 58.4 ms (56.9 ms–66.7 ms) | 59.0 ms (57.4 ms–68.3 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^4 | 87.8 ms (86.2 ms–91.6 ms) | 88.4 ms (86.3 ms–91.5 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^5 | 832 ms (819 ms–846 ms) | 837 ms (826 ms–845 ms) | 1.01, inside the spread |

### Where the change acts

Counted on the head, one build each:

| Set | Planes certified by cofactors | Exact, no filtered cofactors | Exact, loose filtered cofactors | Orientations (`main` with P7-26) |
| --- | ---: | ---: | ---: | ---: |
| Hull `grid` D6 10^4 | 68,165 | 9,139 | 0 | 569,680 (1,154,911) |
| Hull `cluster` D5 10^4 | 49,083 | 4,039 | 22,258 | 271,748 (658,658) |
| Hull `cubesurf` D3 10^5 | 633 | 0 | 0 | 106,010 (106,010) |

The orientations of `main` are those of the spike P7-27 (#395) on the head of #389, which is `main`'s predicate path.

### Reading

- **The keep criterion is met.**
  - Hull `grid` D6 10^4 reads 0.30 (1.95 s to 584 ms; Qhull 340 ms in #379) and `grid` D5 0.89.
  - `lattice` D6 reads 0.42 and `lattice` D5 0.81.
  - All four are faster beyond the spread, as in the prototype of P7-27 (0.30, 0.90, 0.41, 0.83).
- **`cluster` D5 reads 0.58, against 0.85 in the prototype.** The prototype took the exact error only where no filtered cofactors existed. Here 22,258 of its planes had loose filtered cofactors, and the exact error bounds them more tightly. Its orientations fall from 658,658 to 271,748.
- **Hull `cubesurf` D3 10^5 reads 0.79, faster beyond the spread, with the same orientations.**
  - None of its planes takes the exact path.
  - With one codegen unit on both sides, it reads 0.76 (81.8 ms to 62.5 ms, 10 × 3), so this is not the partition into codegen units.
  - A phase timer (not committed) puts the difference in classification's `distance_zeros`: 40 ms on `main`, 24.5 ms on the head, three builds each.
  - That walk calls the side test about half a million times. Each call now reads one plane from the store instead of a normal and a cull plane.
- **Nothing is slower beyond the spread.** The other sets read 0.95 to 1.02.

### The exact direction's error

The new planes take `tau` from the exact direction's bound, `k 2^-49 = 16 k u` (`u = 2^-53`). The review asked whether it holds. A test in a copy of the crate (not committed) wrote 2,690 facets and the directions of the exact path (`cofactor_direction_from` without filtered cofactors). The facets were k = 2 to 10, five kinds of 60 facets each: integers 0..3, integers -1000..1000, uniform in [-1, 1], uniform times 10^200, and thin facets with edges 2^-40 off parallel. A script computed each facet's exact cofactors as rationals and the distance to the exact unit direction to 80 digits.

| k | Largest error | Derived bound `(k/2 + 5) u` | Bound used, `16 k u` |
| --- | ---: | ---: | ---: |
| 2 | 1.44 u | 6 u | 32 u |
| 3 | 1.55 u | 6.5 u | 48 u |
| 4 | 2.14 u | 7 u | 64 u |
| 5 | 2.12 u | 7.5 u | 80 u |
| 6 | 1.89 u | 8 u | 96 u |
| 7 | 1.80 u | 8.5 u | 112 u |
| 8 | 1.82 u | 9 u | 128 u |
| 9 | 2.66 u | 9.5 u | 144 u |
| 10 | 2.03 u | 10 u | 160 u |

No facet exceeded the bound. The derivation is on `cofactor_direction_from` and in design §1. The bound used is 10 to 60 times the largest error; tightening it is P7-31 (#400).

## The P7 parity run after P7-7, P7-9, P7-26, and P7-28, PR #404 (#401)

Due after the speed rows merged since the run of #379: P7-7 (#380), P7-9 (#381), P7-26 (#389), and P7-28 (#399). It is the first run judged within 5% of each reference (ADR 0006, amendment of 2026-10-10).

### Method

As in the run of #379: `main` at `d4f4463`, `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. "(ref.)" marks a reference that is not judged (ADR 0006). A set is met at 1.05 when convx's `build()` is at most 1.05 times each judged reference, and at 1.00 when at most the time of each.

The two runs were on different days. On the sets no row of this period touched (hull D5 and D6, and Delaunay), convx's `build()` read 0.98 of the run of #379 (median; 0.91 to 1.05). CGAL read 0.995 (0.79 to 1.04) and Qhull 0.96 (0.82 to 1.03). A ratio compares both sides of one run, so it does not carry that difference.

### P7 judgement

Met at 1.05: 19 of 41 (hull 14, Delaunay 5); 17 in the run of #379, read again under the amendment. Met at 1.00: 16 of 41 (hull 12, Delaunay 4); 13 in the run of #379.

| Set | convx `build()` | / CGAL | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.47 ms | 1.00 | 0.85 | met | met |
| Hull `cube` D2 10^5 | 4.43 ms | 1.01 | 0.81 | met | not met |
| Hull `cube` D2 10^6 | 56.1 ms | 1.27 | 1.01 | not met | not met |
| Hull `cube` D3 10^4 | 1.22 ms | 0.57 | 0.83 | met | met |
| Hull `cube` D3 10^5 | 10.4 ms | 0.41 | 0.92 | met | met |
| Hull `cube` D3 10^6 | 134 ms | 0.26 | 0.97 | met | met |
| Hull `cube` D4 10^4 | 6.26 ms | 0.15 | 0.96 | met | met |
| Hull `cube` D4 10^5 | 33.3 ms | 0.06 | 1.01 | met | not met |
| Hull `cube` D5 10^4 | 54.2 ms | 0.25 | 0.75 | met | met |
| Hull `cube` D5 10^5 | 208 ms | 0.13 | 0.67 | met | met |
| Hull `cube` D6 10^4 | 596 ms | 0.29 | 0.49 | met | met |
| Hull `cube` D6 10^5 | 2.53 s | 0.24 | 0.40 | met | met |
| Hull `sphere` D2 10^4 | 1.17 ms | 1.87 | 0.18 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 13.1 ms | 1.80 | 0.13 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 159 ms | 1.83 | 0.09 (ref.) | not met | not met |
| Hull `sphere` D3 10^4 | 26.3 ms | 1.22 | 1.49 | not met | not met |
| Hull `sphere` D3 10^5 | 320 ms | 0.80 | 1.30 | not met | not met |
| Hull `sphere` D3 10^6 | 5.03 s | 0.60 | 1.47 | not met | not met |
| Hull `sphere` D4 10^4 | 126 ms | 0.99 | 1.28 | not met | not met |
| Hull `sphere` D4 10^5 | 1.63 s | 1.24 | 1.19 | not met | not met |
| Hull `sphere` D5 10^4 | 857 ms | 0.87 | 0.99 | met | met |
| Hull `sphere` D5 10^5 | 11.03 s | 0.96 | 0.98 | met | met |
| Hull `sphere` D6 10^4 | 6.93 s | 0.73 | 0.83 | met | met |
| Hull `cubesurf` D3 10^5 | 60.1 ms | 1.57 | 2.05 | not met | not met |
| Hull `grid` D6 10^4 | 570 ms | 2.21 (ref.) | 1.64 | not met | not met |
| Delaunay `cube` D2 10^4 | 7.15 ms | 1.75 | 0.38 | not met | not met |
| Delaunay `cube` D2 10^5 | 73.8 ms | 1.75 | 0.31 | not met | not met |
| Delaunay `cube` D2 10^6 | 790 ms | 1.67 | 0.23 | not met | not met |
| Delaunay `sphere` D2 10^4 | 6.67 ms | 1.63 | 0.28 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 59.5 ms | 1.79 | 0.15 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^6 | 573 ms | 1.79 | 0.05 (ref.) | not met | not met |
| Delaunay `cube` D3 10^4 | 46.6 ms | 1.55 | 0.49 | not met | not met |
| Delaunay `cube` D3 10^5 | 509 ms | 1.61 | 0.39 | not met | not met |
| Delaunay `cube` D3 10^6 | 5.51 s | 1.58 | 0.35 | not met | not met |
| Delaunay `sphere` D3 10^4 | 81.8 ms | 0.63 | 0.70 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 758 ms | 0.83 | 0.39 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 6.56 s | 1.02 | 0.37 (ref.) | met | not met |
| Delaunay `cube` D4 10^4 | 748 ms | 1.17 | 0.92 | not met | not met |
| Delaunay `sphere` D4 10^4 | 965 ms | 0.38 | 0.74 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.78 s | 1.11 | 0.93 | not met | not met |
| Delaunay `sphere` D5 10^4 | 12.28 s | 0.35 | 0.82 (ref.) | met | met |

### What moved

- **Hull `grid` D6 10^4** reads 1.64 against Qhull, from 6.55: `build()` took 0.26 of its time in the run of #379. It went through P7-26 (a zero column decides zero) and P7-28 (a cull plane from the exact direction). What remains is classification (the spike P7-27, #395): P7-29 (#393) and P7-30 (#394).
- **Hull `cubesurf` D3 10^5** reads 2.05 against Qhull and 1.57 against CGAL, from 3.54 and 2.70. `build()` took 0.59 of its time: P7-26 decides its zero orientations by a zero column, and P7-28 cut classification's `distance_zeros` from 40 ms to 24.5 ms on Windows (#399).
- **Hull D2:**
  - `cube` D2 10^4 and 10^5 read 1.00 and 1.01 against CGAL, from 1.18 and 1.19, and are met at 1.05. `cube` D2 10^6 reads 1.27, from 1.52.
  - `sphere` D2 reads 1.80 to 1.87 against CGAL, from 2.07 to 2.87.
  - These are the chain's sort (P7-7, #380) and the discard (P7-9, #381).
- **Met at 1.00 now:** hull `cube` D4 10^4 (0.96 against Qhull, from 1.01) and `sphere` D5 10^5 (0.98, from 1.04). `cube` D4 10^5 reads 1.01 (from 1.04), met at 1.05 only. No row changed their paths: they lie within a few percent of 1.00, where runs differ.
- **Delaunay** did not move beyond a few percent. No row of this period changed it.

## Hull: placing keeps its boundary and projects into one buffer, PR #405 (#393)

P7-29. The placing triangulation (design §3), which classification uses for every face that is not a simplex and Delaunay for every cospherical group, made two changes:

- **It keeps the boundary ridges of its complex from one point to the next.** For each point placed, it used to gather every ridge of every simplex into a map again.
  - A point that does not raise the dimension now adds a simplex on each boundary ridge it is beyond. Each ridge of the added simplices then leaves the boundary if it was on it, or joins it.
  - A point that raises the dimension gathers the boundary again, at most `D` times per call.
  - Each boundary ridge keeps the side of its opposite vertex, which does not change until the dimension does.
- **It projects the rows of each orientation into one buffer reused across calls.** It used to allocate a `Vec` per row. After the first change, a profile still put `placing` at 51% of hull `lattice` D6 10^4; the allocations were most of it.

The triangulation is the same, simplex for simplex and in the same order; a test compares it with the earlier implementation, kept as a test-only reference.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `e94f5f6` against the head `44142cb`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `grid` D5 and D6, `lattice` D5 and D6, `cubesurf` D5 10^4, and Delaunay `grid` and `lattice` D3 10^4 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |
| Profiles | A copy with a timer around `placing` (not committed), on `main` (recorded on #393), on the first change, and on the head |

Every set published the same counts on both sides.

### Where `placing` went

| Set | `main` | First change (`b4fb530`) | Head |
| --- | ---: | ---: | ---: |
| Hull `lattice` D6 10^4 | 1,407 ms of 1,989 (71%) | 600 ms of 1,185 (51%) | 211 ms of 776 (27%) |
| Hull `cubesurf` D5 10^4 | 1,288 ms of 1,734 (74%) | 265 ms of 709 (37%) | 55 ms of 490 (11%) |
| Hull `grid` D6 10^4 | 215 ms of 609 (35%) | 121 ms of 504 (24%) | 56 ms of 436 (13%) |

One build each in the copy with the timer; the head's column is the head's code with the timer.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.61 ms (0.55 ms–1.09 ms) | 0.61 ms (0.56 ms–0.78 ms) | 1.00, inside the spread |
| Hull `cube` D2 10^5 | 5.48 ms (5.14 ms–9.90 ms) | 5.66 ms (5.13 ms–7.50 ms) | 1.03, inside the spread |
| Hull `cube` D3 10^4 | 1.57 ms (1.34 ms–4.95 ms) | 1.53 ms (1.32 ms–4.74 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^5 | 12.3 ms (11.6 ms–14.9 ms) | 12.2 ms (11.3 ms–14.5 ms) | 0.99, inside the spread |
| Hull `cube` D4 10^4 | 6.85 ms (6.44 ms–9.02 ms) | 6.82 ms (6.31 ms–8.70 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 36.1 ms (34.5 ms–45.4 ms) | 36.2 ms (34.4 ms–39.2 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 57.8 ms (54.9 ms–109 ms) | 57.7 ms (55.3 ms–67.1 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 588 ms (569 ms–610 ms) | 588 ms (565 ms–606 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 62.9 ms (60.2 ms–70.0 ms) | 61.5 ms (58.5 ms–73.7 ms) | 0.98, inside the spread |
| Hull `cubesurf` D5 10^4 | 1.84 s (1.82 s–1.91 s) | 507 ms (484 ms–518 ms) | 0.27, faster beyond the spread |
| Hull `grid` D5 10^4 | 29.3 ms (27.6 ms–33.5 ms) | 25.2 ms (24.3 ms–29.9 ms) | 0.86, inside the spread |
| Hull `grid` D6 10^4 | 599 ms (588 ms–611 ms) | 432 ms (420 ms–436 ms) | 0.72, faster beyond the spread |
| Hull `lattice` D5 10^4 | 48.3 ms (45.7 ms–57.5 ms) | 40.3 ms (37.5 ms–43.6 ms) | 0.83, faster beyond the spread |
| Hull `lattice` D6 10^4 | 2.03 s (2.02 s–2.06 s) | 772 ms (757 ms–782 ms) | 0.38, faster beyond the spread |
| Hull `sphere` D2 10^4 | 1.52 ms (1.39 ms–1.78 ms) | 1.59 ms (1.40 ms–2.27 ms) | 1.05, inside the spread |
| Hull `sphere` D2 10^5 | 16.8 ms (15.6 ms–33.1 ms) | 17.1 ms (16.1 ms–23.9 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^4 | 30.8 ms (28.1 ms–39.2 ms) | 31.0 ms (28.4 ms–41.3 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^5 | 342 ms (329 ms–385 ms) | 347 ms (323 ms–369 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^4 | 130 ms (121 ms–142 ms) | 130 ms (123 ms–142 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.57 s (1.53 s–1.74 s) | 1.60 s (1.52 s–1.80 s) | 1.02, inside the spread |
| Hull `sphere` D5 10^4 | 843 ms (812 ms–884 ms) | 844 ms (818 ms–892 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 7.37 ms (7.13 ms–9.96 ms) | 7.25 ms (7.08 ms–8.91 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^5 | 75.3 ms (72.3 ms–126 ms) | 73.3 ms (70.5 ms–81.9 ms) | 0.97, inside the spread |
| Delaunay `cube` D3 10^4 | 42.1 ms (40.4 ms–45.5 ms) | 41.7 ms (40.3 ms–48.0 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 452 ms (432 ms–497 ms) | 446 ms (430 ms–507 ms) | 0.99, inside the spread |
| Delaunay `cube` D4 10^4 | 697 ms (683 ms–735 ms) | 706 ms (665 ms–796 ms) | 1.01, inside the spread |
| Delaunay `cube` D5 10^4 | 7.72 s (7.62 s–8.05 s) | 7.94 s (7.64 s–8.01 s) | 1.03, inside the spread |
| Delaunay `grid` D3 10^4 | 1.92 ms (1.75 ms–2.65 ms) | 1.56 ms (1.39 ms–2.51 ms) | 0.81, inside the spread |
| Delaunay `lattice` D3 10^4 | 3.32 ms (3.01 ms–6.00 ms) | 2.41 ms (2.19 ms–3.55 ms) | 0.73, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.03 ms (6.76 ms–7.37 ms) | 7.01 ms (6.78 ms–7.68 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 59.4 ms (56.8 ms–71.6 ms) | 58.6 ms (56.3 ms–61.9 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^4 | 88.9 ms (86.3 ms–96.9 ms) | 88.0 ms (86.2 ms–102 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^5 | 862 ms (856 ms–893 ms) | 856 ms (840 ms–876 ms) | 0.99, inside the spread |

The first change alone, timed the same way against `main` before the second: hull `cubesurf` D5 10^4 0.38, `lattice` D6 0.57, `grid` D6 0.84, all faster beyond the spread.

### Reading

- **The keep criterion is met.**
  - Hull `cubesurf` D5 10^4 reads 0.27 (1.84 s to 507 ms), `lattice` D6 0.38 (2.03 s to 772 ms), and `grid` D6 0.72 (599 ms to 432 ms), all faster beyond the spread.
  - `grid` D6 is now about 1.27 times Qhull's 340 ms (#379). It was 6.55 times in #379 and 1.64 times in the parity run of #404.
- **`lattice` D5 reads 0.83, faster beyond the spread.** `grid` D5 reads 0.86, inside the spread.
- **Delaunay `lattice` and `grid` D3 10^4 read 0.73 and 0.81.** Their cospherical groups are split by `placing`. Their ranges overlap `main`'s only through single slow builds of `main` (6.00 ms and 2.65 ms against medians of 3.32 and 1.92), so the difference is reported.
- **Nothing is slower beyond the spread.** The other sets read 0.97 to 1.05.
- What remains of `grid` D6 is the recursion over lower faces, P7-30 (#394), after its Grill.

## Hull: each face's extreme points once per build, PR #406 (#394)

P7-30. Classification finds a facet's extreme points from the hull of its points one dimension down, recursively (design §3). The profile on #394 put that recursion at 282 ms of hull `grid` D6 10^4's 428 ms: it built 12 hulls in D5, 120 in D4, 960 in D3, and 5,760 in D2, where the 6-cube has 60, 160, and 240 faces of those dimensions. The lower levels also triangulated and listed what the level above does not read. The Grill of 2026-10-10 on #394 chose two changes, in two commits:

- **A, faces once** (`3965c5a`): the extreme points of a face are kept per build with the face's candidates as ascending input indices. A face that several facets share is read back. The extreme points of a point set depend on the set alone, so the result is the same.
- **B, vertices only below** (`245927b`): a hull one dimension down is built for its vertices only, without placing, point lists, or neighbor links.

### Method

| Item | Value |
| --- | --- |
| convx | The base is P7-29 (#405) as timed on its head, the code `main` has since `b8a9bcd` (the commit #405 gained after the timing changed a test only), against A and B. A and B were timed as the same changes before this branch moved onto `b8a9bcd`; their trees are those of `3965c5a` and `245927b`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `grid` D5 and D6, `lattice` D5 and D6, `cubesurf` D5 10^4, and Delaunay `grid` and `lattice` D3 10^4 |
| Rounds | The base, A, and B alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |
| Output | The published hull of the base and of B, hashed in release, the same on hull `grid`, `lattice`, `cubesurf`, `onsphere`, `nearsphere`, and `cluster` in D5 10^4 and D6 at 2,000 and 10^4 points (18 inputs) |

Every set published the same counts in all three.

### Against the base

| Set | Base | A | Head (B) | A / base | Head / base | Head / A |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.62 ms | 0.61 ms | 0.63 ms | 0.98, inside | 1.01, inside | 1.03, inside |
| Hull `cube` D2 10^5 | 5.54 ms | 5.36 ms | 5.67 ms | 0.97, inside | 1.02, inside | 1.06, inside |
| Hull `cube` D3 10^4 | 1.58 ms | 1.57 ms | 1.58 ms | 0.99, inside | 1.00, inside | 1.01, inside |
| Hull `cube` D3 10^5 | 12.7 ms | 13.1 ms | 13.0 ms | 1.03, inside | 1.02, inside | 0.99, inside |
| Hull `cube` D4 10^4 | 7.21 ms | 7.05 ms | 7.13 ms | 0.98, inside | 0.99, inside | 1.01, inside |
| Hull `cube` D4 10^5 | 38.8 ms | 39.0 ms | 38.6 ms | 1.01, inside | 1.00, inside | 0.99, inside |
| Hull `cube` D5 10^4 | 61.2 ms | 60.8 ms | 61.2 ms | 0.99, inside | 1.00, inside | 1.01, inside |
| Hull `cube` D6 10^4 | 615 ms | 615 ms | 617 ms | 1.00, inside | 1.00, inside | 1.00, inside |
| Hull `cubesurf` D3 10^5 | 66.3 ms | 67.1 ms | 65.2 ms | 1.01, inside | 0.98, inside | 0.97, inside |
| Hull `cubesurf` D5 10^4 | 526 ms | 524 ms | 524 ms | 1.00, inside | 1.00, inside | 1.00, inside |
| Hull `grid` D5 10^4 | 25.1 ms | 18.2 ms | 16.5 ms | 0.72, faster beyond | 0.66, faster beyond | 0.91, inside |
| Hull `grid` D6 10^4 | 451 ms | 301 ms | 283 ms | 0.67, faster beyond | 0.63, faster beyond | 0.94, inside |
| Hull `lattice` D5 10^4 | 39.6 ms | 30.6 ms | 28.5 ms | 0.77, faster beyond | 0.72, faster beyond | 0.93, inside |
| Hull `lattice` D6 10^4 | 798 ms | 593 ms | 527 ms | 0.74, faster beyond | 0.66, faster beyond | 0.89, faster beyond |
| Hull `sphere` D2 10^4 | 1.60 ms | 1.56 ms | 1.57 ms | 0.98, inside | 0.98, inside | 1.00, inside |
| Hull `sphere` D2 10^5 | 16.8 ms | 16.1 ms | 16.6 ms | 0.96, inside | 0.99, inside | 1.03, inside |
| Hull `sphere` D3 10^4 | 30.8 ms | 30.6 ms | 31.2 ms | 0.99, inside | 1.02, inside | 1.02, inside |
| Hull `sphere` D3 10^5 | 375 ms | 374 ms | 373 ms | 1.00, inside | 0.99, inside | 1.00, inside |
| Hull `sphere` D4 10^4 | 139 ms | 139 ms | 138 ms | 1.00, inside | 0.99, inside | 0.99, inside |
| Hull `sphere` D4 10^5 | 1.71 s | 1.69 s | 1.70 s | 0.98, inside | 0.99, inside | 1.01, inside |
| Hull `sphere` D5 10^4 | 899 ms | 890 ms | 899 ms | 0.99, inside | 1.00, inside | 1.01, inside |
| Delaunay `cube` D2 10^4 | 7.65 ms | 8.12 ms | 7.62 ms | 1.06, inside | 1.00, inside | 0.94, inside |
| Delaunay `cube` D2 10^5 | 78.4 ms | 81.6 ms | 80.0 ms | 1.04, inside | 1.02, inside | 0.98, inside |
| Delaunay `cube` D3 10^4 | 47.1 ms | 47.2 ms | 47.6 ms | 1.00, inside | 1.01, inside | 1.01, inside |
| Delaunay `cube` D3 10^5 | 511 ms | 511 ms | 514 ms | 1.00, inside | 1.01, inside | 1.01, inside |
| Delaunay `cube` D4 10^4 | 775 ms | 757 ms | 751 ms | 0.98, inside | 0.97, inside | 0.99, inside |
| Delaunay `cube` D5 10^4 | 8.37 s | 8.52 s | 8.46 s | 1.02, inside | 1.01, inside | 0.99, inside |
| Delaunay `grid` D3 10^4 | 1.55 ms | 1.51 ms | 1.62 ms | 0.98, inside | 1.05, inside | 1.07, inside |
| Delaunay `lattice` D3 10^4 | 2.46 ms | 2.46 ms | 2.42 ms | 1.00, inside | 0.98, inside | 0.98, inside |
| Delaunay `sphere` D2 10^4 | 7.16 ms | 7.51 ms | 7.13 ms | 1.05, inside | 1.00, inside | 0.95, inside |
| Delaunay `sphere` D2 10^5 | 65.3 ms | 66.5 ms | 66.4 ms | 1.02, inside | 1.02, inside | 1.00, inside |
| Delaunay `sphere` D3 10^4 | 98.9 ms | 99.5 ms | 99.6 ms | 1.01, inside | 1.01, inside | 1.00, inside |
| Delaunay `sphere` D3 10^5 | 943 ms | 943 ms | 926 ms | 1.00, inside | 0.98, inside | 0.98, inside |

"inside" is inside the spread.

### Reading

- **The keep criterion is met.**
  - Hull `grid` D6 10^4 reads 0.63 (451 ms to 283 ms), `lattice` D6 0.66, `grid` D5 0.66, and `lattice` D5 0.72, all faster beyond the spread.
  - `grid` D6 now takes less than Qhull's 340 ms (#379): about 0.83 of it. It was 6.55 times in #379 and 1.64 times in the parity run of #404.
- **Most of it is A.** The memo alone reads 0.67 to 0.77 on those sets. B adds 0.89 on `lattice` D6, faster beyond the spread, and 0.91 to 0.94 inside the spread on the others.
- **`cubesurf` D5 does not move (1.00).** Its ten facet hulls in D4 need no level below them (the profile on #394), so there is no lower face to share.
- **Nothing is slower beyond the spread.** The other sets read 0.96 to 1.07 against the base in either commit.

## Where the time goes on the general sets left, PR #413 (#408)

The spike P7-32 profiles the sets in general position that the parity run of #404 left unmet: Delaunay D2 and D3, and hull `sphere` D2 to D4. It names the rows that follow.

### Method

| Item | Value |
| --- | --- |
| convx | P7-29 (#405) and P7-30 (#406): the code of `7d694c3`, profiled as the same changes before #406 moved onto `b8a9bcd`. rustc 1.97.1, `--release` with debug info |
| Profile | VTune hotspots, software sampling, the process pinned (logical processor 2): Delaunay `cube` and `sphere` D2 10^5 (50 builds each), `cube` D3 10^5 (8), hull `sphere` D2 10^5 (200), `sphere` D3 10^5 (10), `sphere` D4 10^4 (30). Inclusive shares of the process, from the top-down tree |
| Counts | A copy with counters (not committed): Delaunay's orientations and in-sphere tests, the hull's planes made and side tests, one build each |
| References | The parity run of #404: `build()` phases, CGAL and Qhull times, and Qhull's counters |

The profiles include the generation of the points once per process, a few percent.

### Where the time goes

| Set (#404 ratio) | Shares of the process |
| --- | --- |
| Delaunay `cube` D2 10^5 (1.75 vs CGAL) | insertion by flips 62%: in-circle tests 21%, locating 19% (orientations), its own lines 14%; BRIO order 10%; `ascending` 6%; publication 4% |
| Delaunay `sphere` D2 10^5 (1.79) | the same shape; cell allocation and its copies 5% |
| Delaunay `cube` D3 10^5 (1.61) | insertion 78%: its own lines (the cavity) 35%, in-sphere tests 27%, locating 9%, cell allocation 7%; `ascending` 5% |
| Hull `sphere` D2 10^5 (1.80 vs CGAL) | the chain 41% (sort 15%, left turns 12%, discard 7%); `classify_chain` 32% (its own loop 19%, a sort of the cycle's vertices 10%); `publish` 16% (reallocation 4%); acceptance 8% |
| Hull `sphere` D3 10^5 (1.30 vs Qhull) | planes of new simplices 29% (cofactors in lanes, certification, cull plane, copies); side tests 13% (5% reading the cull plane from the store); coplanar merge 8%; outside sets 13%; facet numbering 4% |
| Hull `sphere` D4 10^4 (1.28) | planes 31%; side tests 12%; coplanar merge 10%; facet numbering 5%; store allocation 7% |

### Work against cost per operation

| Set | Work | Cost per operation |
| --- | --- | --- |
| Delaunay `cube` D2 10^5 | 1,444,781 orientations (14 per point), 903,364 in-circle tests (9 per point) | about 8 ns per orientation, 16 ns per in-circle test |
| Delaunay `sphere` D2 10^5 | 1,347,345 orientations, 396,345 in-circle tests | |
| Delaunay `cube` D3 10^5 | 2,020,271 orientations, 4,590,425 in-sphere tests (46 per point) | about 27 ns per in-sphere test; the cavity's own lines about 1.8 µs per point |
| Hull `sphere` D3 10^5 | 517,421 planes, 1,034,242 side tests; Qhull 565,222 hyperplanes, 4,349,516 distance tests | about 180 ns per plane |
| Hull `sphere` D4 10^4 | 225,621 planes, 485,984 side tests; Qhull 258,048 hyperplanes, 889,697 distance tests | |

### Reading

- **The hull D3 and D4 gaps are the cost per facet, not the work.**
  - convx makes about as many planes as Qhull makes hyperplanes, and runs fewer side tests than Qhull's distance tests.
  - Each plane costs about 180 ns in D3, through the general cofactor path; that is P7-33 (#409), a fixed-size path for D = 3 and 4.
  - The coplanar merge finds nothing on these inputs and costs 8 to 10%. Whether it can be skipped when construction recorded no zero sign needs a proof, so P7-34 (#410) is set after its Grill.
  - Facet numbering (4 to 5%) and reading the cull plane from the store (4 to 5%) are smaller. They are read again in the profile after P7-33 and P7-34.
- **Hull `sphere` D2 is classification and publication,** 48% together, around a chain that is 41%: P7-35 (#411).
- **Delaunay's predicates are cheap per call.** What is left is the insertion's own lines: 35% of `cube` D3 (about 1.8 µs per point), and in D2 a spread of the insertion's lines (14%), the BRIO order (10%), and `ascending` (6%). Which lines own it needs line-level counts: the spike P7-36 (#412).
- **Allocation and copies** (`memcpy`, `RtlReAllocateHeap`) are 4 to 10% of each set, spread over the callers above: cell allocation in Delaunay, the planes and the store in the hull, and `publish`. Each row above takes the part in its own path.

## Hull: planes of D = 3 and 4 by a fixed-size path, PR #416 (#409)

P7-33. The spike P7-32 (#413) put the planes of new simplices at 29 to 31% of hull `sphere` D3 10^5 and D4 10^4, about 180 ns per plane in D3, as many planes as Qhull's hyperplanes.

A facet of three or four points now takes its working normal and its cull threshold from `fixed_plane`: fixed-size arrays, the small cofactors, one certification, and the threshold from the certified error. It applies within a range, every coordinate at most 2^100 and every edge entry zero or in [2^-100, 2^100]. There the general path's power-of-two scaling changes no bit, and the cull plane's `tau` from the cofactors is the certified error itself, so the bits are the general path's. Debug builds check every plane against it.

Two prototypes came first, timed against `main` on the hull sets of D3 to D6:
- taking the certified error as `tau` alone read 0.95 to 0.99;
- also skipping the scaling where it changes no bit read 0.93 to 0.96, inside the spread.

The fixed-size path read 0.76 and 0.77 on `sphere` D3. So the time was the general path's lists and steps per plane, not the arithmetic of the scaling or of `plane_error`.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `95d79a9` against the head `57775e7`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `grid` and `lattice` D5 and D6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.63 ms (0.53 ms–0.78 ms) | 0.61 ms (0.53 ms–0.72 ms) | 0.97, inside the spread |
| Hull `cube` D2 10^5 | 5.68 ms (5.30 ms–6.89 ms) | 5.50 ms (4.91 ms–6.31 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^4 | 1.59 ms (1.34 ms–2.09 ms) | 1.53 ms (1.27 ms–2.56 ms) | 0.96, inside the spread |
| Hull `cube` D3 10^5 | 12.7 ms (11.5 ms–25.7 ms) | 12.4 ms (11.1 ms–15.3 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^4 | 6.74 ms (6.30 ms–7.89 ms) | 5.76 ms (5.30 ms–7.06 ms) | 0.85, inside the spread |
| Hull `cube` D4 10^5 | 35.7 ms (34.1 ms–39.1 ms) | 33.7 ms (32.2 ms–35.5 ms) | 0.94, inside the spread |
| Hull `cube` D5 10^4 | 57.8 ms (54.8 ms–62.2 ms) | 56.1 ms (53.8 ms–62.5 ms) | 0.97, inside the spread |
| Hull `cube` D6 10^4 | 600 ms (571 ms–632 ms) | 599 ms (567 ms–628 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 61.6 ms (57.1 ms–73.1 ms) | 61.2 ms (57.6 ms–70.5 ms) | 0.99, inside the spread |
| Hull `grid` D5 10^4 | 15.3 ms (14.7 ms–18.7 ms) | 14.6 ms (14.2 ms–17.5 ms) | 0.95, inside the spread |
| Hull `grid` D6 10^4 | 265 ms (265 ms–278 ms) | 259 ms (258 ms–270 ms) | 0.98, inside the spread |
| Hull `lattice` D5 10^4 | 25.9 ms (25.2 ms–28.3 ms) | 25.2 ms (24.6 ms–27.0 ms) | 0.98, inside the spread |
| Hull `lattice` D6 10^4 | 508 ms (498 ms–518 ms) | 501 ms (490 ms–508 ms) | 0.99, inside the spread |
| Hull `sphere` D2 10^4 | 1.60 ms (1.40 ms–2.15 ms) | 1.56 ms (1.39 ms–2.30 ms) | 0.98, inside the spread |
| Hull `sphere` D2 10^5 | 16.9 ms (15.9 ms–18.3 ms) | 17.6 ms (16.0 ms–18.9 ms) | 1.04, inside the spread |
| Hull `sphere` D3 10^4 | 28.5 ms (27.4 ms–32.2 ms) | 21.6 ms (20.5 ms–23.1 ms) | 0.76, faster beyond the spread |
| Hull `sphere` D3 10^5 | 342 ms (323 ms–415 ms) | 264 ms (251 ms–292 ms) | 0.77, faster beyond the spread |
| Hull `sphere` D4 10^4 | 130 ms (124 ms–142 ms) | 96.5 ms (91.8 ms–106 ms) | 0.74, faster beyond the spread |
| Hull `sphere` D4 10^5 | 1.53 s (1.52 s–1.54 s) | 1.17 s (1.15 s–1.21 s) | 0.76, faster beyond the spread |
| Hull `sphere` D5 10^4 | 850 ms (813 ms–945 ms) | 845 ms (806 ms–894 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 7.49 ms (7.10 ms–9.59 ms) | 7.44 ms (7.17 ms–9.80 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^5 | 78.1 ms (72.5 ms–82.0 ms) | 78.3 ms (72.9 ms–89.3 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^4 | 43.7 ms (41.9 ms–47.4 ms) | 43.1 ms (41.1 ms–47.4 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 466 ms (442 ms–513 ms) | 464 ms (441 ms–480 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 684 ms (645 ms–721 ms) | 689 ms (648 ms–814 ms) | 1.01, inside the spread |
| Delaunay `cube` D5 10^4 | 7.78 s (7.70 s–8.67 s) | 7.74 s (7.65 s–8.77 s) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.03 ms (6.83 ms–7.71 ms) | 7.02 ms (6.83 ms–8.87 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 61.5 ms (57.8 ms–67.3 ms) | 61.5 ms (57.5 ms–66.3 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 91.4 ms (86.1 ms–96.8 ms) | 90.0 ms (86.3 ms–95.6 ms) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^5 | 859 ms (831 ms–877 ms) | 855 ms (854 ms–942 ms) | 0.99, inside the spread |

### Reading

- **The keep criterion is met.**
  - Hull `sphere` D3 10^5 reads 0.77 (342 ms to 264 ms) and D3 10^4 0.76.
  - D4 reads 0.74 at 10^4 (130 ms to 97 ms) and 0.76 at 10^5 (1.53 s to 1.17 s).
  - All four are faster beyond the spread.
- **Against the references of #404**, the ratios of that run times these speedups, indicative until the next parity run:
  - `sphere` D3 10^5: 1.30 to about 1.00 against Qhull;
  - D3 10^4: 1.49 to about 1.13;
  - D4 10^4: 1.28 to about 0.95;
  - D4 10^5: 1.24 to about 0.94.
- **Nothing is slower beyond the spread.** The other sets read 0.94 to 1.04. `cube` D4 reads 0.85 and 0.94; its hulls make fewer planes.

## Hull D2: the boundary complex in flat lists, PR #417 (#411)

P7-35. The spike P7-32 (#413) put `classify_chain` at 32% of hull `sphere` D2 10^5, where every point is extreme, and `publish` at 16%. A first change took the sort of the cycle's vertices out (one pass over the ascending representatives) and sized the published vertex coordinates up front (a `flat_map` gave no size hint, and the list grew by reallocation). It read 0.94, inside the spread. A line-level profile of `classify_chain` then put 81% of its own time on one line: building a record per simplex, two inline lists of 40 bytes each, about 90 bytes per simplex. `publish` spent most of its own time on those records, and on dropping them.

The boundary complex is now `Complex`, flat lists of vertices, faces, and neighbors, as the faces already were, read through a borrowed view per simplex. The published hull is the same.

### Method

| Item | Value |
| --- | --- |
| convx | The base is P7-33 (#416), timed on its head `57775e7`, whose code `main` has since `5ba92ef`, against this change timed as `66774a0`; after #416 merged, the branch moved onto `5ba92ef`, and the change is `59af24b` with the same code. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `cube` and `sphere` D2 10^6, `grid` and `lattice` D5 and D6 |
| Rounds | The base and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about a tenth of a second |
| Output | The published hull of the base and of the head, hashed in release (its `Debug` form and its volume's bits), the same on 42 inputs: `cube`, `sphere`, `grid`, `lattice`, `cubesurf`, `onsphere`, and `cluster`, in D2 to D6 |

Every set published the same counts on both sides.

### Against the base

| Set | Base | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.62 ms (0.54 ms–0.73 ms) | 0.62 ms (0.53 ms–0.82 ms) | 1.00, inside the spread |
| Hull `cube` D2 10^5 | 5.37 ms (4.85 ms–8.20 ms) | 5.71 ms (4.96 ms–8.27 ms) | 1.06, inside the spread |
| Hull `cube` D2 10^6 | 58.7 ms (56.9 ms–63.6 ms) | 60.7 ms (56.8 ms–62.6 ms) | 1.04, inside the spread |
| Hull `cube` D3 10^4 | 1.43 ms (1.23 ms–4.31 ms) | 1.49 ms (1.26 ms–2.58 ms) | 1.04, inside the spread |
| Hull `cube` D3 10^5 | 12.4 ms (11.2 ms–14.4 ms) | 12.6 ms (11.1 ms–14.2 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^4 | 5.59 ms (5.27 ms–6.69 ms) | 5.63 ms (5.30 ms–7.91 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 34.5 ms (32.5 ms–43.7 ms) | 34.1 ms (32.6 ms–45.8 ms) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 57.0 ms (54.0 ms–62.8 ms) | 57.4 ms (54.7 ms–66.4 ms) | 1.01, inside the spread |
| Hull `cube` D6 10^4 | 594 ms (565 ms–630 ms) | 592 ms (569 ms–671 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 60.7 ms (58.0 ms–66.5 ms) | 60.4 ms (57.2 ms–66.4 ms) | 1.00, inside the spread |
| Hull `grid` D5 10^4 | 14.6 ms (14.2 ms–16.2 ms) | 14.7 ms (14.3 ms–16.7 ms) | 1.01, inside the spread |
| Hull `grid` D6 10^4 | 259 ms (255 ms–271 ms) | 261 ms (258 ms–271 ms) | 1.01, inside the spread |
| Hull `lattice` D5 10^4 | 25.7 ms (24.5 ms–29.6 ms) | 25.5 ms (24.8 ms–29.1 ms) | 0.99, inside the spread |
| Hull `lattice` D6 10^4 | 501 ms (493 ms–513 ms) | 502 ms (489 ms–516 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 1.60 ms (1.52 ms–1.94 ms) | 1.27 ms (1.14 ms–2.23 ms) | 0.79, inside the spread |
| Hull `sphere` D2 10^5 | 16.9 ms (15.2 ms–20.5 ms) | 12.9 ms (11.4 ms–19.4 ms) | 0.76, inside the spread |
| Hull `sphere` D2 10^6 | 174 ms (173 ms–178 ms) | 134 ms (126 ms–137 ms) | 0.77, faster beyond the spread |
| Hull `sphere` D3 10^4 | 23.0 ms (21.8 ms–41.0 ms) | 23.3 ms (21.2 ms–25.0 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^5 | 270 ms (255 ms–301 ms) | 263 ms (248 ms–344 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^4 | 96.4 ms (91.9 ms–106 ms) | 94.2 ms (89.8 ms–99.5 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^5 | 1.17 s (1.14 s–1.19 s) | 1.19 s (1.17 s–1.20 s) | 1.02, inside the spread |
| Hull `sphere` D5 10^4 | 849 ms (805 ms–900 ms) | 845 ms (808 ms–880 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 7.55 ms (7.21 ms–21.1 ms) | 7.62 ms (7.13 ms–10.9 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^5 | 75.9 ms (73.6 ms–81.4 ms) | 76.8 ms (72.7 ms–91.9 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 43.4 ms (41.3 ms–67.1 ms) | 43.5 ms (40.8 ms–58.6 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 470 ms (442 ms–505 ms) | 454 ms (442 ms–484 ms) | 0.96, inside the spread |
| Delaunay `cube` D4 10^4 | 684 ms (644 ms–793 ms) | 688 ms (649 ms–740 ms) | 1.01, inside the spread |
| Delaunay `cube` D5 10^4 | 7.76 s (7.66 s–7.84 s) | 7.75 s (7.62 s–7.81 s) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.12 ms (6.78 ms–9.27 ms) | 7.19 ms (6.91 ms–8.71 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^5 | 59.9 ms (57.0 ms–64.5 ms) | 60.2 ms (58.5 ms–65.3 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^4 | 91.4 ms (86.9 ms–96.0 ms) | 92.4 ms (87.0 ms–103 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^5 | 848 ms (822 ms–866 ms) | 854 ms (835 ms–877 ms) | 1.01, inside the spread |

### Reading

- **The keep criterion is met.**
  - Hull `sphere` D2 10^5 reads 0.76 (16.9 ms to 12.9 ms). Its ranges overlap only through one slow build of the head (19.4 ms; the next is 15.4 ms against the base's fastest 15.2 ms), and a first run of 8 × 3 read 0.75, faster beyond the spread. So the difference is reported (`bench.mdc`).
  - `sphere` D2 10^6 reads 0.77 (175 ms to 134 ms), faster beyond the spread, and D2 10^4 0.79.
- **Against CGAL in #404**, `sphere` D2 read 1.80 to 1.87; times these speedups, about 1.4, indicatively.
- **Nothing is slower beyond the spread.** The other sets read 0.96 to 1.06, including `grid` and `lattice` D5 and D6, whose faces are triangulated by placing into the same complex.

## Delaunay D2 and D3: where the insertion's own time goes, PR #424 (#412)

The spike P7-36 takes line-level profiles of Delaunay `cube` D2 and D3 10^5. The spike P7-32 (#413) left the insertion's own lines there as the largest share.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `d466627`. rustc 1.97.1, `--release` with debug info |
| Profile | VTune hotspots, software sampling, the process pinned (logical processor 2): `cube` D2 10^5, 60 builds; `cube` D3 10^5, 8 builds. Inclusive shares from the top-down tree; the own time of the insertion, `locate`, `alloc`, and `brio` by source line (`-source-object function=…`) |

### Shares of the process

| | D2 10^5 | D3 10^5 |
| --- | ---: | ---: |
| Insertion, inclusive | 61.5% | 78.9% |
| of which its own lines | 13% | 35% |
| of which in-circle or in-sphere tests | 18.8% | 31.2% |
| of which locating (orientations) | 17.1% | 4.9% |
| of which cell allocation | 4.5% | 6.5% |
| BRIO order | 11.0% | – |

### Where the own lines go

| Function | Line | Share of its own time |
| --- | --- | ---: |
| D3 `insert` (1.27 s own) | `self.link_by_keys(&boundary, &created)`: pairing the new tetrahedra's faces (inlined; see the correction below) | 59.7% |
| | `self.conflict(n, q)` (outside the test itself) | 10.4% |
| D2 `insert_by_flips` (0.61 s own) | `self.flip(c, n, q)` | 55.5% |
| | finding the slot, reading the neighbor, recording | 9 to 7% each |
| D2 `locate` (0.12 s own) | the random offset of the walk (`% k`), twice per step | 28% |
| D3 `alloc` (0.14 s own) | copying the vertices, filling the neighbors and records | 100% |

### Reading

- **D3: pairing the new faces is about 9% of the build, not 21%** (corrected, #425). `link_by_keys` pairs about 60 faces per point through an open-addressing table keyed by an edge. The 21% read the line of its call while it is inlined into `insert`; see the correction below. P7-37 (#421) is withdrawn.
- **The predicates are about 30% of D2 and 35% of D3**, at 8 to 27 ns per call through the semi-static first stage. A static filter, which bounds every call of one input once, is a design question (certification for every accepted input, the fallback, the stages after it): P7-38 (#422), set after its Grill.
- **D2's BRIO order is 11%**: the space-filling keys about 5% and their sort about 4%. That is P7-39 (#423).
- **Smaller items, read again after these rows:** D2's flips (about 7% of the build), the walk's modulo (about 4%), and cell allocation (4 to 7%).

### Correction: `link_by_keys` without inlining (#425)

The first step of P7-37 (#421) profiled `link_by_keys` in a copy of `main` (`8e1cbf0`) with `#[inline(never)]` on it. The copy timed the same as `main`, pinned and alternated: `cube` D3 10^5 1.01, `cube` D3 10^4 1.00, `sphere` D3 10^4 1.01. Its profile (VTune, pinned, 8 builds of `cube` D3 10^5) put `link_by_keys`' own time at 0.31 s, about 39 ms per build and 8.4% of the process. Its lines:

- the table write, 28%;
- the key comparison and the read of the other entry, 28%;
- the hash, 9%.

`insert`'s own time fell to 14.3%. Within it, the cavity walk (marks, the stack, reading neighbors) is about 35%, the call into the conflict test 19%, and the search for the back link 13%.

So the line of the call took time that is not `link_by_keys`' own while the function was inlined. Its share is about 9% of the build. A change would save part of that, within the spread of `cube` D3 10^5 (about ±5%), so no keep criterion can be named, and P7-37 is withdrawn (owner, 2026-10-10).

## Predicates: a first stage bounded by the columns' largest entries, prototype (#422)

P7-38's Grill (2026-10-10) asked for a prototype before the implementation. A copy of `main` (`8e1cbf0`) gave four formulas a first stage bounded by a constant times the product of the columns' largest absolute entries (CGAL's form), with the present stage after it: the orientation of k = 2 and 3, and the lifted orientation of D = 2 and 3. Its constants were generous, for timing only, not derived.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `8e1cbf0` against the copy. rustc 1.97.1, `--release` with debug info |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Rounds | Alternated per round: 8 × 3 builds of the D2 sets, 6 × 1 of `cube` D3, 5 × 1 of hull `sphere` D3 |

Every set published the same counts on both sides.

| Set | `main` | Copy | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^5 | 82.5 ms (75.3–98.5) | 83.8 ms (77.6–92.1) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^5 | 67.6 ms (62.9–74.8) | 70.3 ms (65.4–76.1) | 1.04, inside the spread |
| Delaunay `cube` D3 10^5 | 515 ms (502–533) | 483 ms (474–493) | 0.94, faster beyond the spread |
| Hull `sphere` D3 10^5 | 294 ms (291–300) | 286 ms (285–301) | 0.97, inside the spread |

### Reading

- **D2 does not move.** The in-circle test's arithmetic, with its permanent, is about 9% of the build. Most of what the spikes P7-32 and P7-36 counted as the predicates is around the call: the wrapper `sphere_sign`, a `position` search for the infinite vertex in `conflict`, and reading the sites.
- **D3 gains about 6%**, against new constants to derive and test and an ADR.
- The keep criterion asked D2 and D3 to be faster. **P7-38 is withdrawn** (owner, 2026-10-10). The call paths get the spike P7-40 (#427).

## Delaunay: where the predicates' call paths spend their time, PR #431 (#427)

The spike P7-40 follows the prototype of P7-38 (#428), which left D2 unchanged when the first stage's bound got cheaper. It profiles the predicates' call paths of Delaunay `cube` and `sphere` D2 10^5 and `cube` D3 10^5 without inlining, as `bench.mdc` asks since #425.

### Method

| Item | Value |
| --- | --- |
| convx | A copy of `main` (`232dd97`) with `#[inline(never)]` on `Shape::orient` and `Shape::lifted`, `sphere_sign`, `replaced`, `conflict`, `sign_of`, and `semi_static::sign`. rustc 1.97.1, `--release` with debug info |
| Same build | The copy against `main`, pinned and alternated: `cube` D2 10^5 0.98, `sphere` D2 10^5 0.98, `cube` D3 10^5 0.95, all inside the spread, the same counts |
| Profile | VTune hotspots, software sampling, pinned: 60 builds of each D2 set, 8 of `cube` D3. Shares of the process, own and inclusive |

### Where the time goes

| | `cube` D2 10^5 | `sphere` D2 10^5 | `cube` D3 10^5 |
| --- | ---: | ---: | ---: |
| Insertion, inclusive | 60.8% | – | 77.9% |
| of which its own lines | 12.5% | 7.5% | 34.4% |
| Predicate arithmetic (`semi_static`), own | about 20% | about 15% | about 15% |
| The wrappers (`sphere_sign`, `replaced`, `conflict`, `Shape::orient`), own | about 5% | about 9% | about 6% |
| BRIO order | 10.2% | – | 1.2% |
| `ascending` | 5.1% | – | 3.1% |
| The orientation pass in `complex` | 4.4% | – | 2.0% |
| `complex`'s own lines | about 3% | about 4% | about 8% |
| `publish` | 2.8% | – | – |
| `memcpy` and the heap, spread over `ascending`, cell allocation, `complex`, `publish`, and `Sites` | about 9% | about 9% | about 5% |

### Reading

- **D2's gap is spread, not in one place.** The predicates' arithmetic is about a fifth of the build. The rest is spread over the insertion's own lines, the wrappers, BRIO, and the passes after insertion, each a few percent.
- **The wrappers are glue, a few percent each.** For D2 and D3, `Plane` and `Space` already call the first stage on the points alone and gather the stored lifted heights only when a later stage runs; the generic `Sites::lifted`, which gathers them for every call, serves the other dimensions. What the wrappers do here is copy a cell's vertex ids and `q`, look up each site's row, and search the cell for the infinite vertex (three or four ids). Half of their 5 to 9% would be inside the spread of D2, so no row is proposed for them; P7-42 (#430) was opened on a reading of `Sites::lifted` and is withdrawn (owner, 2026-10-10).
- **The passes after insertion** are about 15% of `cube` D2 (`ascending`, the orientation pass, `complex`'s own lines, `publish`, with their copies): P7-41 (#429).
- **The BRIO order** is P7-39 (#423), about 10% of D2.
- D3's insertion's own lines remain the largest share (34%). #425 put `link_by_keys` at about 9% of them; the rest is the cavity walk.

## Delaunay: the cells of D = 2 and 3 sorted in arrays, PR #432 (#429)

P7-41. The spike P7-40 (#431) put the passes after insertion at about 15% of Delaunay `cube` D2 10^5. A copy of `main` with `#[inline(never)]` on `inserted`, `ascending`, `Draft::finish`, and `publish` timed the same as `main` (0.97 to 1.01) and put, of the build:

- `ascending` at 5.6%, half of it copies (`memcpy`);
- `inserted`'s own lines at about 5%, with another 3.2% of copies under it.

`ascending` returned each cell's sorted vertices as a list of eleven pairs inline, about 180 bytes, copied on every call. Cells of three and four vertices (D = 2 and 3) now go through `ascending_fixed`, which sorts in arrays with the same network, and are written to the draft from those arrays. `ascending` takes the same function for three and four vertices, so the network is in one place. The published triangulation is the same.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `4e4b081` (the code of `232dd97`) against the head `832b8fe`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with Delaunay `cube` and `sphere` D2 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.56 ms (0.54 ms–1.13 ms) | 0.57 ms (0.53 ms–1.21 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^5 | 5.61 ms (4.86 ms–8.21 ms) | 5.44 ms (4.79 ms–7.03 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^4 | 1.50 ms (1.25 ms–2.20 ms) | 1.47 ms (1.25 ms–2.03 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 12.3 ms (11.0 ms–14.1 ms) | 12.6 ms (11.1 ms–15.5 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^4 | 5.64 ms (5.26 ms–6.78 ms) | 5.55 ms (5.32 ms–6.65 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^5 | 33.5 ms (32.5 ms–37.1 ms) | 33.5 ms (32.8 ms–35.2 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 55.8 ms (54.4 ms–59.6 ms) | 55.5 ms (53.7 ms–59.1 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 566 ms (550 ms–588 ms) | 560 ms (551 ms–666 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 59.9 ms (58.4 ms–64.3 ms) | 60.0 ms (58.8 ms–64.5 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 1.24 ms (1.12 ms–4.80 ms) | 1.21 ms (1.11 ms–3.84 ms) | 0.97, inside the spread |
| Hull `sphere` D2 10^5 | 12.4 ms (11.3 ms–16.8 ms) | 12.5 ms (11.3 ms–15.3 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 22.4 ms (21.0 ms–26.2 ms) | 22.2 ms (21.1 ms–26.5 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 254 ms (245 ms–295 ms) | 253 ms (246 ms–293 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 92.3 ms (90.3 ms–97.9 ms) | 91.6 ms (89.5 ms–95.6 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^5 | 1.19 s (1.14 s–1.25 s) | 1.17 s (1.15 s–1.23 s) | 0.98, inside the spread |
| Hull `sphere` D5 10^4 | 805 ms (774 ms–874 ms) | 797 ms (772 ms–826 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 7.70 ms (7.22 ms–8.21 ms) | 7.38 ms (6.69 ms–7.82 ms) | 0.96, inside the spread |
| Delaunay `cube` D2 10^5 | 73.4 ms (72.0 ms–82.2 ms) | 67.6 ms (65.6 ms–73.3 ms) | 0.92, inside the spread |
| Delaunay `cube` D2 10^6 | 835 ms (793 ms–902 ms) | 759 ms (738 ms–764 ms) | 0.91, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 41.5 ms (40.7 ms–44.4 ms) | 40.1 ms (38.9 ms–43.9 ms) | 0.97, inside the spread |
| Delaunay `cube` D3 10^5 | 454 ms (435 ms–468 ms) | 434 ms (425 ms–455 ms) | 0.96, inside the spread |
| Delaunay `cube` D4 10^4 | 671 ms (655 ms–740 ms) | 694 ms (684 ms–723 ms) | 1.03, inside the spread |
| Delaunay `cube` D5 10^4 | 7.52 s (7.43 s–7.57 s) | 7.62 s (7.57 s–7.81 s) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.07 ms (6.88 ms–9.16 ms) | 6.69 ms (6.42 ms–7.49 ms) | 0.95, inside the spread |
| Delaunay `sphere` D2 10^5 | 59.0 ms (57.7 ms–62.9 ms) | 55.0 ms (53.8 ms–57.2 ms) | 0.93, faster beyond the spread |
| Delaunay `sphere` D2 10^6 | 583 ms (578 ms–589 ms) | 545 ms (539 ms–565 ms) | 0.94, faster beyond the spread |
| Delaunay `sphere` D3 10^4 | 89.2 ms (87.3 ms–99.6 ms) | 87.7 ms (85.8 ms–92.0 ms) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^5 | 846 ms (841 ms–863 ms) | 836 ms (834 ms–871 ms) | 0.99, inside the spread |

### Reading

- **The keep criterion is met.**
  - Delaunay `sphere` D2 10^5 reads 0.93, faster beyond the spread.
  - `cube` D2 10^5 reads 0.92. Its ranges overlap only through the head's two slowest builds (72.9 ms and 73.3 ms, against `main`'s fastest 72.0 ms); the other 28 are faster than any build of `main`, so the difference is reported (`bench.mdc`).
  - At 10^6, `cube` reads 0.91 and `sphere` 0.94, both faster beyond the spread.
- **D3 moves less** (`cube` D3 0.96 and 0.97, inside the spread): the passes after insertion are about 6% of it, and its insertion's own lines are the largest share (#431).
- **Nothing is slower beyond the spread.** `cube` D4 reads 1.03 and D5 1.01, inside the spread; they take the general `ascending`.

## Delaunay: the BRIO keys by masks and one sorted key, PR #434 (#423)

P7-39. The BRIO order took 11.5% of Delaunay `cube` D2 10^5 (#423):

- the keys 5.8%, of which the interleaving of their bits one at a time 2.8% and the normalized cell collected in `Small` 1.6%;
- the sort of `(Reverse(round), code, site)` tuples 5.4%.

Now:

- D = 2 and 3 spread each cell's bits with masks, which put bit `b` of coordinate `j` where the loop put it (`b d + d - 1 - j`); the loop stays for other dimensions;
- each site gets one `u128`, `(32 - round) << 96 | code << 32 | site`, which orders exactly as the tuple, and the keys are sorted as integers.

The order is the same, site for site; the previous implementation is the tests' reference.

### Method

| Item | Value |
| --- | --- |
| convx | The base is P7-41, the head of #432 (`832b8fe` with its docs), against the head `2ec36f8`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with Delaunay `cube` and `sphere` D2 10^6 |
| Rounds | The base and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A second run of `cube` D2 10^5, 20 × 3 |

Every set published the same counts on both sides.

### Against the base

| Set | Base | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.59 ms (0.50 ms–1.02 ms) | 0.60 ms (0.55 ms–0.99 ms) | 1.02, inside the spread |
| Hull `cube` D2 10^5 | 5.28 ms (4.76 ms–6.21 ms) | 5.41 ms (4.96 ms–8.09 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^4 | 1.48 ms (1.25 ms–1.83 ms) | 1.41 ms (1.26 ms–3.06 ms) | 0.95, inside the spread |
| Hull `cube` D3 10^5 | 11.7 ms (11.2 ms–12.7 ms) | 11.7 ms (11.1 ms–14.6 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^4 | 5.70 ms (5.28 ms–6.71 ms) | 5.60 ms (5.26 ms–6.88 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^5 | 34.3 ms (32.8 ms–37.0 ms) | 34.1 ms (33.0 ms–67.2 ms) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 56.6 ms (54.6 ms–60.0 ms) | 56.3 ms (54.6 ms–62.0 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 624 ms (609 ms–643 ms) | 620 ms (602 ms–643 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 60.9 ms (59.9 ms–66.7 ms) | 61.3 ms (59.4 ms–64.1 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^4 | 1.24 ms (1.13 ms–4.92 ms) | 1.22 ms (1.11 ms–4.56 ms) | 0.98, inside the spread |
| Hull `sphere` D2 10^5 | 12.1 ms (11.3 ms–20.9 ms) | 11.7 ms (10.9 ms–14.1 ms) | 0.97, inside the spread |
| Hull `sphere` D3 10^4 | 22.8 ms (20.7 ms–47.0 ms) | 22.0 ms (21.0 ms–25.1 ms) | 0.96, inside the spread |
| Hull `sphere` D3 10^5 | 262 ms (255 ms–309 ms) | 260 ms (254 ms–289 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^4 | 94.3 ms (91.3 ms–100 ms) | 94.0 ms (90.5 ms–99.5 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.26 s (1.25 s–1.29 s) | 1.26 s (1.25 s–1.28 s) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 888 ms (830 ms–911 ms) | 888 ms (864 ms–920 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 7.30 ms (6.78 ms–13.0 ms) | 6.79 ms (6.29 ms–8.37 ms) | 0.93, inside the spread |
| Delaunay `cube` D2 10^5 | 73.7 ms (71.5 ms–79.8 ms) | 69.7 ms (67.6 ms–80.7 ms) | 0.94, inside the spread |
| Delaunay `cube` D2 10^6 | 717 ms (712 ms–775 ms) | 678 ms (669 ms–736 ms) | 0.95, inside the spread |
| Delaunay `cube` D3 10^4 | 43.9 ms (42.0 ms–47.7 ms) | 43.6 ms (40.9 ms–49.3 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 474 ms (466 ms–555 ms) | 467 ms (457 ms–491 ms) | 0.99, inside the spread |
| Delaunay `cube` D4 10^4 | 699 ms (685 ms–816 ms) | 668 ms (649 ms–786 ms) | 0.96, inside the spread |
| Delaunay `cube` D5 10^4 | 8.21 s (7.57 s–8.42 s) | 8.29 s (7.55 s–8.41 s) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^4 | 7.22 ms (6.45 ms–9.29 ms) | 6.45 ms (6.12 ms–7.72 ms) | 0.89, inside the spread |
| Delaunay `sphere` D2 10^5 | 61.6 ms (56.9 ms–64.3 ms) | 55.9 ms (53.6 ms–69.2 ms) | 0.91, inside the spread |
| Delaunay `sphere` D2 10^6 | 588 ms (584 ms–591 ms) | 547 ms (541 ms–549 ms) | 0.93, faster beyond the spread |
| Delaunay `sphere` D3 10^4 | 95.3 ms (91.5 ms–111 ms) | 92.6 ms (90.8 ms–103 ms) | 0.97, inside the spread |
| Delaunay `sphere` D3 10^5 | 898 ms (890 ms–902 ms) | 891 ms (883 ms–901 ms) | 0.99, inside the spread |

The second run of `cube` D2 10^5, 20 × 3: 71.1 ms (67.8–78.9) against 67.2 ms (64.3–72.8), 0.94.

### Reading

- **Every Delaunay D2 set is faster by its median:** 0.89 to 0.95, at 10^4, 10^5, and 10^6.
- **`sphere`:**
  - `sphere` D2 10^6 reads 0.93, faster beyond the spread.
  - `sphere` D2 10^5 reads 0.91. Its ranges overlap through the base's fastest build (56.9 ms; the next is 58.9 ms) and the head's three slowest; 27 of the head's 30 builds are faster than the base's second fastest. The difference is reported (`bench.mdc`).
- **`cube` D2 10^5 reads 0.94 in both runs.** Its ranges overlap by more than single builds: 6 of the head's 30 are slower than the base's fastest in the first run. The keep criterion asks it to be faster beyond the spread, and it is not, though two runs agree on the difference. Whether to keep the change on that is the owner's call (`bench.mdc`).
- **Nothing is slower beyond the spread.** D3, D4, and D5 read 0.96 to 1.01, the hull 1.00.

## The P7 parity run after P7-29 to P7-41, PR #438 (#437)

Due after the speed rows merged since the run of #404: P7-29 (#405), P7-30 (#406), P7-33 (#416), P7-35 (#417), P7-41 (#432), and P7-39 (#434).

### Method

As in the run of #404: `main` at `3a59a89`, `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. "(ref.)" marks a reference that is not judged (ADR 0006). A set is met at 1.05 when convx's `build()` is at most 1.05 times each judged reference, and at 1.00 when at most the time of each.

The two runs were on the same day, at different hours. On the sets no row of this period touched (hull `cube` and `sphere` D5 and D6), convx's `build()` read 0.97 of the run of #404 (median; 0.96 to 1.03). CGAL read 1.01 (0.97 to 1.05) and Qhull 1.00 (0.96 to 1.07).

### P7 judgement

Met at 1.05: 23 of 41 (hull 18, Delaunay 5); 19 in the run of #404. Met at 1.00: 20 of 41 (hull 15, Delaunay 5); 16 in the run of #404.

| Set | convx `build()` | / CGAL | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.49 ms | 1.03 | 0.87 | met | not met |
| Hull `cube` D2 10^5 | 4.66 ms | 1.07 | 0.86 | not met | not met |
| Hull `cube` D2 10^6 | 58.6 ms | 1.33 | 1.07 | not met | not met |
| Hull `cube` D3 10^4 | 1.14 ms | 0.53 | 0.77 | met | met |
| Hull `cube` D3 10^5 | 9.99 ms | 0.39 | 0.87 | met | met |
| Hull `cube` D3 10^6 | 133 ms | 0.26 | 0.95 | met | met |
| Hull `cube` D4 10^4 | 5.09 ms | 0.12 | 0.78 | met | met |
| Hull `cube` D4 10^5 | 32.0 ms | 0.06 | 0.97 | met | met |
| Hull `cube` D5 10^4 | 51.8 ms | 0.24 | 0.73 | met | met |
| Hull `cube` D5 10^5 | 206 ms | 0.13 | 0.67 | met | met |
| Hull `cube` D6 10^4 | 589 ms | 0.28 | 0.45 | met | met |
| Hull `cube` D6 10^5 | 2.60 s | 0.23 | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.97 ms | 1.54 | 0.15 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 11.2 ms | 1.39 | 0.10 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 128 ms | 1.45 | 0.06 (ref.) | not met | not met |
| Hull `sphere` D3 10^4 | 19.3 ms | 0.84 | 1.01 | met | not met |
| Hull `sphere` D3 10^5 | 255 ms | 0.61 | 1.01 | met | not met |
| Hull `sphere` D3 10^6 | 4.17 s | 0.50 | 1.20 | not met | not met |
| Hull `sphere` D4 10^4 | 91.2 ms | 0.74 | 0.93 | met | met |
| Hull `sphere` D4 10^5 | 1.23 s | 0.97 | 0.91 | met | met |
| Hull `sphere` D5 10^4 | 819 ms | 0.83 | 0.97 | met | met |
| Hull `sphere` D5 10^5 | 10.63 s | 0.95 | 0.99 | met | met |
| Hull `sphere` D6 10^4 | 6.75 s | 0.70 | 0.79 | met | met |
| Hull `cubesurf` D3 10^5 | 58.7 ms | 1.50 | 2.03 | not met | not met |
| Hull `grid` D6 10^4 | 244 ms | 0.96 (ref.) | 0.71 | met | met |
| Delaunay `cube` D2 10^4 | 6.16 ms | 1.60 | 0.33 | not met | not met |
| Delaunay `cube` D2 10^5 | 66.2 ms | 1.57 | 0.27 | not met | not met |
| Delaunay `cube` D2 10^6 | 700 ms | 1.48 | 0.21 | not met | not met |
| Delaunay `sphere` D2 10^4 | 5.75 ms | 1.51 | 0.25 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 52.3 ms | 1.59 | 0.14 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^6 | 506 ms | 1.57 | 0.04 (ref.) | not met | not met |
| Delaunay `cube` D3 10^4 | 44.6 ms | 1.51 | 0.48 | not met | not met |
| Delaunay `cube` D3 10^5 | 481 ms | 1.50 | 0.37 | not met | not met |
| Delaunay `cube` D3 10^6 | 5.28 s | 1.59 | 0.34 | not met | not met |
| Delaunay `sphere` D3 10^4 | 75.4 ms | 0.60 | 0.68 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 723 ms | 0.81 | 0.40 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 6.20 s | 0.99 | 0.37 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 697 ms | 1.13 | 0.90 | not met | not met |
| Delaunay `sphere` D4 10^4 | 923 ms | 0.37 | 0.75 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.38 s | 1.11 | 0.94 | not met | not met |
| Delaunay `sphere` D5 10^4 | 11.75 s | 0.34 | 0.83 (ref.) | met | met |

### What moved

- **Met at 1.05 now:**
  - Hull `grid` D6 10^4 reads 0.71 against Qhull, from 1.64; `build()` took 0.43 of its time. This is P7-29 (#405) and P7-30 (#406): `placing` keeps its boundary, and each face's extreme points are found once.
  - Hull `sphere` D3 10^4 and 10^5 read 1.01 against Qhull, from 1.49 and 1.30. `sphere` D4 10^4 and 10^5 read 0.93 and 0.97, from 1.28 and 1.24. `build()` took 0.72 to 0.80 of its time, from the fixed-size planes of D = 3 and 4 (P7-33, #416).
  - Delaunay `sphere` D3 10^6 reads 0.99 against CGAL, from 1.02, and is met at 1.00 as well: its `build()` read 0.94 of the run of #404, from P7-41 and P7-39.
- **No longer met at 1.05:** hull `cube` D2 10^5 reads 1.07 against CGAL, from 1.01. Its `build()` read 1.05 times the run of #404 (4.66 ms against 4.43 ms), while CGAL and Qhull read 0.99 on it. `cube` D2 10^4 reads 1.03 (from 1.00) and 10^6 1.33 (from 1.27). P7-35 (#417) timed `cube` D2 10^5 and 10^6 at 1.06 and 1.04 against its base, inside the spread; this run reads the same direction.
- **Moved but still not met:**
  - Hull `sphere` D3 10^6 reads 1.20 against Qhull, from 1.47.
  - Hull `sphere` D2 reads 1.39 to 1.54 against CGAL, from 1.80 to 1.87: `build()` took 0.81 to 0.85 of its time, from the flat boundary complex (P7-35, #417).
  - Delaunay D2 reads 1.48 to 1.60 against CGAL, from 1.63 to 1.79: `build()` took 0.86 to 0.90 of its time, from the cells sorted in arrays (P7-41, #432) and the BRIO keys by masks (P7-39, #434).
  - Delaunay `cube` D3 10^5 reads 1.50, from 1.61 (`build()` 0.94).

### What is not met

18 sets, 13 of them Delaunay:

| Group | Sets | Worst ratio |
| --- | ---: | --- |
| Delaunay `cube` and `sphere` D2 | 6 | 1.48 to 1.60 against CGAL |
| Delaunay `cube` D3 | 3 | 1.50 to 1.59 against CGAL |
| Delaunay `cube` D4 and D5 10^4 | 2 | 1.13 and 1.11 against CGAL |
| Hull `sphere` D2 | 3 | 1.39 to 1.54 against CGAL |
| Hull `cube` D2 10^5 and 10^6 | 2 | 1.07 and 1.33 against CGAL |
| Hull `sphere` D3 10^6 | 1 | 1.20 against Qhull |
| Hull `cubesurf` D3 10^5 | 1 | 2.03 against Qhull, 1.50 against CGAL |

## Hull D2: the points off the polygon in one pass, PR #441 (#440)

P7-43. The parity run of #438 read hull `cube` D2 10^5 at 1.07 against CGAL, no longer met at 1.05 (1.01 in the run of #404).

### Across P7-35 (#439)

The base of #417 (`5ba92ef`), #417 (`d466627`), and `main` (`3a59a89`), pinned and alternated, 20 rounds × 3 builds:

| Set | `5ba92ef` | `d466627` | `3a59a89` |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.61 ms (0.53 ms–1.38 ms) | 0.61 ms (0.53 ms–0.79 ms), 1.00 | 0.61 ms (0.53 ms–3.99 ms), 0.99 |
| Hull `cube` D2 10^5 | 5.35 ms (4.79 ms–10.4 ms) | 5.52 ms (4.89 ms–9.41 ms), 1.03 | 5.49 ms (4.89 ms–6.33 ms), 1.03 |
| Hull `cube` D2 10^6 | 64.6 ms (60.9 ms–94.0 ms) | 65.5 ms (62.0 ms–83.0 ms), 1.01 | 66.3 ms (60.9 ms–105 ms), 1.03 |
| Hull `sphere` D2 10^5 | 16.5 ms (15.0 ms–27.7 ms) | 12.6 ms (11.1 ms–16.9 ms), 0.77 | 12.3 ms (10.9 ms–14.8 ms), 0.75 |

Inside the spread, but in the direction of #417's own timing (1.06 and 1.04) and of the parity run (1.05).

A VTune profile of `main` on `cube` D2 10^5 (software sampling, pinned, 2,000 builds) put `classify_chain` at 11.3% of `build()`. Of that, the lists of the points off the cycle took 8.2%:

| Line | Share of `build()` |
| --- | ---: |
| `others`, collected through `filter` | 4.4%, of which growing the list 1.9% |
| `interior_points`, through `zip`, `filter`, and `map` | 2.9%, of which growing the list 1.2% |
| `coplanar_points` | 0.9% |
| The cycle's vertices, the pass #417 added | 0.6% |

P7-35 classified a strict polygon without sorts or regrowth for `sphere` D2, where every point is on the cycle. On `cube` D2 nearly every point is off it, and its lists still grew by reallocation, in four passes.

### The change

One pass over the ascending representatives writes the cycle's vertices, the coplanar points, and the interior points, each ascending. The vertices and the interior points are sized up front. What the build knows of each representative (on the cycle, proved interior, or neither) is one list of a three-state type, where it was two lists of booleans.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `3a59a89` against the head `01df6af`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with hull `cube` D2 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A second run of hull `cube` D2 10^5 and Delaunay `cube` D4 10^4, 20 × 3 |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.57 ms (0.53 ms–1.16 ms) | 0.51 ms (0.47 ms–0.75 ms) | 0.88, inside the spread |
| Hull `cube` D2 10^5 | 5.39 ms (4.88 ms–6.08 ms) | 4.88 ms (4.37 ms–5.45 ms) | 0.91, inside the spread |
| Hull `cube` D2 10^6 | 58.0 ms (56.8 ms–62.3 ms) | 51.4 ms (50.5 ms–54.1 ms) | 0.89, faster beyond the spread |
| Hull `sphere` D2 10^4 | 1.16 ms (1.12 ms–1.44 ms) | 1.16 ms (1.09 ms–1.50 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^5 | 12.2 ms (11.1 ms–26.1 ms) | 11.9 ms (10.8 ms–13.2 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^4 | 1.42 ms (1.27 ms–1.91 ms) | 1.42 ms (1.25 ms–2.20 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^5 | 11.6 ms (10.9 ms–12.5 ms) | 11.3 ms (10.9 ms–12.6 ms) | 0.98, inside the spread |
| Hull `cubesurf` D3 10^5 | 58.1 ms (57.2 ms–61.1 ms) | 57.9 ms (56.9 ms–62.2 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 21.0 ms (19.9 ms–22.8 ms) | 20.6 ms (20.2 ms–23.2 ms) | 0.98, inside the spread |
| Hull `sphere` D3 10^5 | 250 ms (245 ms–270 ms) | 249 ms (241 ms–272 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 5.44 ms (5.12 ms–6.70 ms) | 5.55 ms (5.28 ms–6.29 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^5 | 33.1 ms (32.3 ms–45.0 ms) | 33.4 ms (31.7 ms–37.7 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^4 | 91.7 ms (89.0 ms–98.6 ms) | 91.2 ms (88.4 ms–101 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^5 | 1.15 s (1.12 s–1.19 s) | 1.12 s (1.11 s–1.14 s) | 0.98, inside the spread |
| Hull `cube` D5 10^4 | 54.9 ms (53.1 ms–64.1 ms) | 53.9 ms (52.3 ms–57.6 ms) | 0.98, inside the spread |
| Hull `sphere` D5 10^4 | 789 ms (775 ms–828 ms) | 784 ms (765 ms–895 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 564 ms (558 ms–580 ms) | 563 ms (552 ms–596 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 6.39 ms (6.20 ms–7.59 ms) | 6.46 ms (6.16 ms–7.20 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^5 | 64.7 ms (63.0 ms–72.6 ms) | 64.9 ms (62.6 ms–76.2 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 672 ms (669 ms–699 ms) | 677 ms (673 ms–686 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^4 | 6.29 ms (6.07 ms–8.68 ms) | 6.34 ms (6.18 ms–6.82 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^5 | 52.5 ms (50.2 ms–56.6 ms) | 52.2 ms (51.0 ms–55.4 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^6 | 513 ms (505 ms–521 ms) | 517 ms (516 ms–526 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 40.2 ms (38.5 ms–47.4 ms) | 39.6 ms (38.5 ms–41.5 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 427 ms (418 ms–472 ms) | 427 ms (418 ms–456 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 87.3 ms (85.4 ms–93.5 ms) | 87.7 ms (85.6 ms–101 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 828 ms (819 ms–837 ms) | 835 ms (825 ms–839 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 667 ms (653 ms–711 ms) | 695 ms (682 ms–734 ms) | 1.04, inside the spread |
| Delaunay `cube` D5 10^4 | 7.62 s (7.48 s–7.80 s) | 7.54 s (7.48 s–7.62 s) | 0.99, inside the spread |

The second run, 20 × 3: hull `cube` D2 10^5 5.34 ms (4.81–6.47) against 4.75 ms (4.32–8.16), 0.89; Delaunay `cube` D4 10^4 711 ms (678–764) against 742 ms (710–778), 1.04.

### Delaunay `cube` D4

The change does not reach Delaunay, yet `cube` D4 10^4 read 1.04 in both runs, every build of the head slower than the fastest of `main`. Two checks:

- **`main` against itself**, in the same harness, 20 × 3: 735 ms against 735 ms, 1.00. The order of the alternation does not favour either side.
- **Both built with one codegen unit** (`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`), 20 × 3: Delaunay `cube` D4 10^4 735 ms (704–765) against 738 ms (711–776), 1.00; hull `cube` D2 10^5 5.35 ms (4.79–8.31) against 4.98 ms (4.50–6.44), 0.93.

The 1.04 follows how the crate is split into codegen units, not the code of Delaunay, and goes away with one unit; the gain on `cube` D2 stays.

### Reading

- **Hull `cube` D2 10^6 reads 0.89, faster beyond the spread.**
- **Hull `cube` D2 10^5 reads 0.91 and 0.89 in two runs**, and 0.93 with one codegen unit. Its ranges overlap by more than single builds: 15 of the head's 30 builds in the first run are slower than the fastest of `main`. The keep criterion asks it to be faster beyond the spread, and it is not, though three runs agree on the difference. Whether to keep the change on that is the owner's call (`bench.mdc`). `cube` D2 10^4 reads 0.88.
- **`sphere` D2 reads 0.98 and 1.00.** Its points are on the cycle, where nothing changed.
- **Nothing is slower beyond the spread.** Delaunay `cube` D4 10^4 reads 1.04 inside the spread; it is the code layout above. The other sets read 0.98 to 1.02.

## Delaunay D2 and D3 against CGAL, per operation, PR #445 (#444)

The spike P7-44. The parity run of #438 leaves 13 Delaunay sets unmet. Earlier work had already ruled out the obvious D2 causes:

- convx already ran CGAL's in-circle tests and flips in D2 (#367);
- three prototypes each read 1.00: one array for vertices and neighbors, a walk that skips its entry face, and a static filter (#428).

This spike measures CGAL at the grain of convx's profiles (Grill of 2026-10-10).

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `3a59a89` under WSL2 (the build of the parity run of #438), and `03d7247` on Windows for the prototypes. rustc 1.97.1, `--release` with debug info |
| CGAL | The build of the parity run, `Delaunay_triangulation_2` and `_3` over `Epick`. A scratch program (not committed) counts the predicate calls through a kernel derived from `Epick`, and times `spatial_sort` and the insertions of the sorted points apart |
| Instructions and caches | valgrind 3.18.1 (cachegrind and callgrind) under WSL2, the LL cache simulated from the host's L3. One build is the difference between runs of 3 builds and of 1, which removes reading the file |
| Timing | On Windows, pinned (logical processor 2), alternated per round, 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |
| The pass after the insertion | Phase marks inside `inserted` in a scratch copy of `main`, under WSL2, pinned; the second and third of 3 builds |

### CGAL's work and time

| Set | In-circle or in-sphere tests, CGAL / convx | Orientations, CGAL / convx | CGAL's spatial sort | CGAL's insertions |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 910,699 / 903,364 | 664,425 / 1,444,781 | 12 ms | 29 ms |
| `cube` D3 10^5 | 4,427,975 / 4,590,425 | 1,235,764 / 2,020,271 | 13 ms | 300 ms |
| `cube` D2 10^6 | 9,140,926 | 6,680,110 | 146 ms | 305 to 357 ms |
| `cube` D3 10^6 | 44,810,945 | 12,291,134 | 165 ms | 3.14 to 3.37 s |

convx's counts are those of #413. The tests that decide the triangulation match within 4%. convx's walk makes 1.6 to 2.2 times CGAL's orientations.

### Instructions and cache misses of one build

| Set | Instructions, convx / CGAL | Data reads | D1 read misses | LL read misses |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 866 M / 315 M (2.75) | 213 M / 71 M | 1.80 M / 0.87 M | 31 k / under 1 k |
| `cube` D3 10^5 | 5,402 M / 2,552 M (2.12) | 1,361 M / 589 M | 11.9 M / 6.2 M | 1.64 M / 1.61 M |

The two builds cover different steps:

- convx's `build()` includes accepting the input, the order, the pass after the insertion, and publication;
- CGAL's includes its spatial sort.

The LL misses are the same in D3. **The gap is the instructions convx runs, not memory.** Its time ratio (1.5 to 1.6) is below its instruction ratio because the extra instructions are arithmetic and run at a higher rate.

### Where the instructions go (callgrind, own instructions)

| `cube` D2 10^5 | Instructions | Per call |
| --- | ---: | ---: |
| `semi_static::sign`, the generic first stage, for the orientations | 183 M | about 127 |
| `Mesh::replaced`, the orientation's wrapper | 104 M | about 72 |
| `semi_static::lifted2`, the in-circle formula | 78 M | about 87 |
| `Mesh::sphere_sign`, the in-circle's wrapper | 72 M | about 80 |
| `Mesh::conflict` | 42 M | |
| `Mesh::build`: the walk and the flips, inlined | 180 M | |
| `complex` and `Draft::finish`: the pass after the insertion | 85 M | |
| The BRIO order | about 42 M | |

CGAL's own counts:

- Its statically filtered `Orientation_2` takes 12 M instructions for 664,425 calls, about 18 each.
- Its whole triangulation code (`Triangulation_2`, its data structure, and `Delaunay_triangulation_2`, with the in-circle test inlined) takes about 220 M. That is about what convx's walk and flips take without the predicates.

In D3 the predicates and their wrappers are 3.3 G of 5.4 G. The rest of the insertion (1.5 G) is less than CGAL's whole insertion with its predicates (about 2.2 G).

The cause is the entry:

- The fixed shapes `Plane` and `Space` (ADR 0005) pass their rows as slices to the generic `semi_static::sign`.
  - It is not inlined.
  - It chooses its formula by a `match` on the dimension.
  - It checks the bounds of every index and takes the differences through closures.
- The wrappers copy the vertex ids into an array of 18 entries, which is zeroed on every call.

The code also departs from two ADRs:

- ADR 0005 has the fixed shapes "call the predicate formulas directly"; they reach them only through this entry.
- ADR 0007 has the D = 2 walk skip the face it came through; `locate` tests every face.

### Prototypes

Two copies of `main` (`03d7247`), not committed:

1. **Fixed-size entry:** `Plane` and `Space` read each site as an array of D coordinates and call `orient2`, `lifted2`, `orient3`, or `lifted3` through an inlined entry on arrays.
2. **Arrays in the wrappers, and the walk of ADR 0007:** prototype 1, plus:
   - `sphere_sign` and `replaced` build arrays of 3, 4, or 5 ids;
   - `locate` skips the face to the simplex it came from. That face's orientation is positive, so the walk and its result are the same.

Both publish the same counts. Every test passes on the second, and debug builds compare each fixed-size sign with the generic one.

| Set | `main` | Prototype 1 | Prototype 2 |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 7.09 ms (6.40 ms–10.0 ms) | 0.92 | 0.89, inside the spread |
| Delaunay `cube` D2 10^5 | 71.7 ms (67.4 ms–87.6 ms) | 0.90 | 0.89, inside the spread |
| Delaunay `sphere` D2 10^5 | 57.7 ms (52.7 ms–75.6 ms) | 0.90 | 0.88, inside the spread |
| Delaunay `cube` D2 10^6 | 721 ms (721 ms–742 ms) | 0.90, faster beyond the spread | 0.90, faster beyond the spread |
| Delaunay `sphere` D2 10^6 | 546 ms (537 ms–564 ms) | 0.91 | 0.89, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 43.5 ms (40.2 ms–48.9 ms) | 0.95 | 0.91, inside the spread |
| Delaunay `cube` D3 10^5 | 467 ms (456 ms–483 ms) | 0.96 | 0.92, faster beyond the spread |
| Delaunay `sphere` D3 10^4 | 94.0 ms (89.6 ms–99.5 ms) | 0.98 | 0.97, inside the spread |
| Delaunay `cube` D4 10^4 | 754 ms (720 ms–805 ms) | 0.98 | 0.96, inside the spread |

The three were alternated in one run.

- A first run of prototype 1 alone read the same: D2 0.89 to 0.94, `cube` D3 0.95 to 0.96, and hull `cube` and `sphere` D3 0.99 and 1.04, inside the spread.
- D4 does not take these paths; its 0.96 is inside the spread.

After prototype 2, `cube` D2 10^5 runs about 640 M instructions and `cube` D3 10^5 about 4.0 G.

- The in-circle test still costs about 137 instructions a call, and the in-sphere test about 307.
- In each, the first stage computes the permanent for its bound: a second sum of absolute products, as long as the determinant.

### The pass after the insertion

Milliseconds of one build, under WSL2:

| Set | Tests of the faces the insertion left unknown | The draft: rows sorted, links | `finish` and `publish` | Share of `build()` |
| --- | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 6.4 | 3.8 | 1.2 | 17% |
| `sphere` D2 10^5 | 4.2 | 2.6 | 0.7 | 14% |
| `cube` D3 10^5 | 49 | 14 | 4.5 | 14% |
| `cube` D4 10^4 | 70 | 29 | 2.2 | 14% |
| `cube` D5 10^4 | 760 | 252 | 12.5 | 14% |

The faces the insertion leaves unknown lie between two new simplices whose boundary faces lie in different cavity simplices (module docs of `insert`). The pass tests each with a lifted orientation.

In D4 and D5 those tests are about 10% of `build()`, about the whole gap to CGAL (1.13 and 1.11). Construction alone is 0.97 and 0.96 of CGAL's whole build (#438).

### Reading

- **The gap is instructions, in the predicates' entry.** convx does CGAL's tests and misses the cache as often, but runs 2.1 to 2.75 times the instructions. Most of the excess is the generic entry that the fixed shapes go through.
- **Prototype 2 gains 11% on D2 and 8% on `cube` D3.** It is faster beyond the spread on the D2 sets of 10^6 and on `cube` D3 10^5. On the parity ratios of #438 that is about 1.39 for `cube` D2 10^5 and 1.38 for `cube` D3 10^5: not parity by itself.
- **What remains after it:**
  - The first stage's bound, the permanent, about as long as the determinant. P7-38 (#428) made the bound cheaper and read 1.00, but the entry still cost about 200 instructions a call then. The bound is now a larger share of a shorter call.
  - The pass after the insertion: 14 to 17% of `build()` in every dimension, most of it the tests of the faces left unknown.
  - In D2, accepting the input and making the sites: about 3 ms of 66.
- **The rows that follow, from the Grill on #444:**
  - P7-45 (#446): the fixed-size entry, with the wrappers and the walk of ADR 0007 (prototype 2), and ADR 0008 for dedicated D = 2 and D = 3 paths;
  - P7-46 (#447): the first stage's bound, prototyped on top of it;
  - P7-47 (#448): a proof that decides the faces left unknown without a test, grilled before any row.

## Delaunay D2 and D3: the predicates on fixed-size rows, PR #449 (#446)

P7-45, the first dedicated path of ADR 0008. The spike P7-44 (#445) found that the fixed shapes `Plane` and `Space` reached the first-stage formulas through the generic `semi_static::sign`, on slices: about 127 instructions per orientation against about 18 in CGAL. It also found that the walk tested the face it came through.

### The change

- **The entry:** `Plane` and `Space` read each site as an array of D coordinates (`Sites::fixed`) and call the formula through `first_stage_fixed`, an inlined entry on arrays. The generic `first_stage` had no other caller and is removed.
- **The wrappers:** `sphere_sign` and `replaced` move into `Shape`. `Plane` and `Space` build the ids of a predicate in arrays of 3, 4, or 5; `Any` keeps its buffer on the stack.
- **The walk (ADR 0007):** `locate` skips the face to the simplex it came from. That orientation is not negative, so the walk takes the same steps. Debug builds check it on every skip.
- **Checks:**
  - debug builds compare every fixed-size sign with the generic one, as before;
  - a test compares `fixed_sign` with the generic entry, and each certified sign with the exact one. It covers D = 2 and 3, plain and lifted, on random points, points one ulp from an exact zero, and points scaled from 2^-60 to 2^60;
  - swapping two rows in `fixed_sign` makes that test fail.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `03d7247` against the head `754b878`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Timed | `build()` alone, generated with `tests/common/generator.rs`, seed 1; the counts are read after the timer stops |
| Sets | The shorter run of `docs/verification.md`, with Delaunay `cube` D3 10^6 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A second run of hull D2, 20 × 3 |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.56 ms (0.50 ms–0.75 ms) | 0.57 ms (0.49 ms–0.68 ms) | 1.03, inside the spread |
| Hull `cube` D2 10^5 | 5.06 ms (4.51 ms–6.46 ms) | 5.28 ms (4.80 ms–6.37 ms) | 1.04, inside the spread |
| Hull `sphere` D2 10^4 | 1.17 ms (1.09 ms–1.40 ms) | 1.22 ms (1.11 ms–1.50 ms) | 1.05, inside the spread |
| Hull `sphere` D2 10^5 | 12.4 ms (11.1 ms–17.0 ms) | 12.6 ms (11.3 ms–14.8 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^4 | 1.47 ms (1.28 ms–7.38 ms) | 1.49 ms (1.24 ms–3.36 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^5 | 12.8 ms (11.4 ms–14.8 ms) | 12.7 ms (11.2 ms–14.3 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 64.4 ms (61.5 ms–70.0 ms) | 65.5 ms (61.2 ms–68.1 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^4 | 23.2 ms (21.3 ms–35.1 ms) | 23.4 ms (21.5 ms–39.3 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^5 | 279 ms (270 ms–297 ms) | 285 ms (272 ms–341 ms) | 1.02, inside the spread |
| Hull `cube` D4 10^4 | 5.92 ms (5.24 ms–7.09 ms) | 5.92 ms (5.31 ms–6.83 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 35.8 ms (33.2 ms–39.4 ms) | 36.3 ms (34.2 ms–41.1 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^4 | 101 ms (97.6 ms–105 ms) | 102 ms (97.8 ms–114 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.29 s (1.28 s–1.35 s) | 1.28 s (1.27 s–1.31 s) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 59.6 ms (55.0 ms–64.2 ms) | 59.6 ms (57.0 ms–64.5 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 887 ms (864 ms–919 ms) | 886 ms (867 ms–995 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 617 ms (602 ms–654 ms) | 617 ms (595 ms–647 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 6.62 ms (6.31 ms–8.83 ms) | 5.84 ms (5.48 ms–6.94 ms) | 0.88, inside the spread |
| Delaunay `cube` D2 10^5 | 70.3 ms (67.9 ms–76.6 ms) | 60.4 ms (56.2 ms–66.2 ms) | 0.86, faster beyond the spread |
| Delaunay `cube` D2 10^6 | 741 ms (727 ms–813 ms) | 661 ms (628 ms–696 ms) | 0.89, faster beyond the spread |
| Delaunay `sphere` D2 10^4 | 6.59 ms (6.25 ms–7.93 ms) | 5.80 ms (5.55 ms–6.85 ms) | 0.88, inside the spread |
| Delaunay `sphere` D2 10^5 | 56.6 ms (53.2 ms–59.9 ms) | 49.1 ms (47.2 ms–52.1 ms) | 0.87, faster beyond the spread |
| Delaunay `sphere` D2 10^6 | 571 ms (563 ms–589 ms) | 488 ms (477 ms–493 ms) | 0.86, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 44.0 ms (41.3 ms–47.2 ms) | 38.7 ms (37.5 ms–41.9 ms) | 0.88, inside the spread |
| Delaunay `cube` D3 10^5 | 465 ms (443 ms–486 ms) | 418 ms (399 ms–440 ms) | 0.90, faster beyond the spread |
| Delaunay `cube` D3 10^6 | 4.93 s (4.90 s–4.99 s) | 4.46 s (4.43 s–4.52 s) | 0.90, faster beyond the spread |
| Delaunay `sphere` D3 10^4 | 93.2 ms (89.2 ms–104 ms) | 89.4 ms (85.2 ms–94.4 ms) | 0.96, inside the spread |
| Delaunay `sphere` D3 10^5 | 921 ms (912 ms–938 ms) | 886 ms (873 ms–889 ms) | 0.96, faster beyond the spread |
| Delaunay `cube` D4 10^4 | 757 ms (728 ms–813 ms) | 741 ms (715 ms–826 ms) | 0.98, inside the spread |
| Delaunay `cube` D5 10^4 | 8.58 s (8.46 s–8.76 s) | 8.71 s (8.54 s–8.81 s) | 1.02, inside the spread |

### Hull D2

The change does not reach the hull. Hull `cube` D2 read 1.03 and 1.04 in both runs, inside the spread; `sphere` D2 read 1.00 to 1.05. As `bench.mdc` asks for a path the change does not reach, two checks, 20 × 3:

- **`main` against itself:** hull `cube` D2 10^4 0.97 and 10^5 0.96. These sets differ by 3 to 4% between identical builds in this harness.
- **Both sides built with one codegen unit:**
  - hull `cube` D2 10^4 1.01 and 10^5 1.02;
  - Delaunay `cube` D2 10^5 0.87 and `cube` D3 10^4 0.90, the gain unchanged.

### Reading

- **The keep criterion is met.**
  - Delaunay D2 at 10^6 reads 0.89 (`cube`) and 0.86 (`sphere`), faster beyond the spread.
  - `cube` D3 10^5 reads 0.90, faster beyond the spread, and `cube` D3 10^6 0.90.
- **Every D2 set reads 0.86 to 0.89, and `cube` D3 0.88 to 0.90.** That is a little more than prototype 2 of #445 (0.88 to 0.89 and 0.92), which built its entry from the same formulas.
- **`sphere` D3 reads 0.96.** Its in-sphere tests reach the exact stage more often (#301), and that stage did not change.
- **D4 and D5 read 0.98 and 1.02, inside the spread.** They take the generic path.
- **Nothing is slower beyond the spread.** Hull D2's 1.03 to 1.04 is within what identical builds differ by, and reads 1.01 to 1.02 with one codegen unit.
- On the parity ratios of #438, this is about 1.35 for `cube` D2 10^5 and for `cube` D3 10^5. The first stage's bound (P7-46, #447) and the faces the insertion leaves unknown (P7-47, #448) follow.

## Delaunay D2 and D3: the first stage's bound on fixed rows, prototype, PR #450 (#447)

The spike P7-46. After P7-45 (#446), the in-circle test costs about 137 instructions a call and the in-sphere test about 307 (#445). In each, the present first stage computes the permanent for its bound, a second sum of absolute products as long as the determinant.

P7-38's prototype (#428) tried a bound from the columns' largest entries, CGAL's form, and read 1.00 on D2, while the generic entry still cost about 200 instructions a call. The Grill on #444 asked for that bound again, prototyped on P7-45, and a feature row only if it gains.

### The prototype

A copy of P7-45 (`afae8ca`), not committed:

- In `fixed_sign`, before the present stage, the four formulas (orientation of D = 2 and 3, lifted orientation of D = 2 and 3) are evaluated without their permanents.
- Each is certified when it exceeds a constant times the product of the columns' largest absolute differences, CGAL's form. For the lifted orientations, that product is taken with the square of the largest column twice.
- A column's largest entry outside [2^-199, 2^199] (about 10^-60 to 10^60) skips the new stage.
- What it does not certify goes to the present stage, then the later ones.
- The constants are generous (about four times CGAL's), for timing only, not derived.

Every test passes, and debug builds compare every sign with the generic path.

### Method

| Item | Value |
| --- | --- |
| convx | P7-45 at `afae8ca` against the copy. rustc 1.97.1, `--release` with debug info |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Rounds | P7-45 and the copy alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |

Every set published the same counts on both sides.

| Set | P7-45 | Prototype | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D3 10^5 | 12.0 ms (11.1 ms–13.7 ms) | 12.0 ms (11.1 ms–13.2 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 22.7 ms (21.1 ms–26.6 ms) | 22.8 ms (21.2 ms–25.5 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 6.14 ms (5.51 ms–8.15 ms) | 6.00 ms (5.55 ms–7.51 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^5 | 60.6 ms (56.3 ms–66.6 ms) | 61.0 ms (57.2 ms–65.4 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^6 | 658 ms (646 ms–676 ms) | 660 ms (653 ms–671 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 6.24 ms (5.62 ms–24.9 ms) | 6.61 ms (5.99 ms–7.30 ms) | 1.06, inside the spread |
| Delaunay `sphere` D2 10^5 | 47.9 ms (44.8 ms–59.8 ms) | 52.7 ms (49.2 ms–59.1 ms) | 1.10, inside the spread |
| Delaunay `sphere` D2 10^6 | 486 ms (463 ms–536 ms) | 511 ms (503 ms–532 ms) | 1.05, inside the spread |
| Delaunay `cube` D3 10^4 | 37.8 ms (35.8 ms–41.1 ms) | 37.3 ms (35.1 ms–56.4 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 428 ms (397 ms–452 ms) | 415 ms (388 ms–437 ms) | 0.97, inside the spread |
| Delaunay `cube` D3 10^6 | 4.63 s (4.55 s–4.67 s) | 4.55 s (4.47 s–4.62 s) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^4 | 93.5 ms (89.7 ms–103 ms) | 95.8 ms (92.6 ms–130 ms) | 1.02, inside the spread |
| Delaunay `sphere` D3 10^5 | 885 ms (865 ms–897 ms) | 909 ms (905 ms–922 ms) | 1.03, slower beyond the spread |
| Delaunay `cube` D4 10^4 | 768 ms (746 ms–794 ms) | 776 ms (757 ms–878 ms) | 1.01, inside the spread |


### Reading

- **No set is faster beyond the spread.**
  - `cube` D2 reads 0.98 to 1.01.
  - `cube` D3 reads 0.97 to 0.99.
- **`sphere` is slower:**
  - `sphere` D2 reads 1.05 to 1.10;
  - `sphere` D3 10^5 reads 1.03, slower beyond the spread.

  Their sites lie near one circle or sphere, so the in-circle and in-sphere determinants are small against the columns' extent. The new stage leaves those open, and the present stage then evaluates the formula again with its permanent.
- **The permanent is not the cost it looked.** Its terms are the absolute values of products the determinant already forms, so dropping it saves little. A bound that ignores how the entries combine certifies fewer of the calls that matter on `sphere`.
- **No feature row follows** (Grill on #444). The pass after the insertion, P7-47 (#448), is next.

## Delaunay: the faces left unknown, by an orientation, PR #452 (#448, #451)

The spike P7-47 (#448) and the row P7-48 (#451). The pass after the insertion tested each face the insertion left unknown with a lifted orientation of D + 2 sites. Those are faces between two new simplices whose boundary faces lie in different cavity simplices. The tests were about 10% of `build()` in D4 and D5 (#445).

### Counts (P7-47)

A scratch copy of P7-45 (`afae8ca`) under WSL2, one build each. "Lattice" is `{0..m-1}^D`: 300², 20³, 9⁴, 6⁵.

| Set | Finite faces | Left unknown | Cospherical among them | F dependent | Missed by the rule |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cube` D3 10^5 | 1,343,043 | 506,576 | 0 | 0 | 0 |
| `sphere` D3 10^5 | 504,028 | 187,835 | 0 | 0 | 0 |
| `cube` D4 10^4 | 737,215 | 325,881 | 0 | 0 | 0 |
| `sphere` D4 10^4 | 288,179 | 126,112 | 0 | 0 | 0 |
| `cube` D5 10^4 | 4,644,774 | 2,260,561 | 0 | 0 | 0 |
| Lattice D2 | 267,605 | 124,638 | 0 | 2,217 | 0 |
| Lattice D3 | 79,708 | 31,453 | 8,122 | 8,390 | 0 |
| Lattice D4 | 228,082 | 108,446 | 52,510 | 43,235 | 0 |
| Lattice D5 | 990,006 | 541,601 | 348,236 | 263,192 | 0 |

F is the D + 1 sites of the two simplices other than the newest. The rule tests a face by the lifted orientation only when F is dependent. "Missed" counts the cospherical pairs that are not joined in the end.

Two other rules fall short:

- **Skipping every test** misses 2,398 of 8,122 cospherical pairs on lattice D3, 8,102 of 52,510 on D4, and 38,962 of 348,236 on D5.
- **Deciding only when both boundary faces share their outside simplex**, the one exact shortcut, covers 6 to 20% of the faces left unknown.

The lemma behind the rule and its proof are in design §7 (Grill on #448).

### The change (P7-48)

- **The rule:** a face left unknown is tested by the orientation of F. The lifted orientation runs only when that orientation is zero. The newest site is the one latest in the insertion order.
- **The test-only switch:** it tests every face left unknown by the lifted orientation.
- **The test:** a new test compares the simplices and neighbors published with and without the switch. Inputs: lattices of D = 2 to 5, integer grids, general position, sites near one sphere, and integer sites on one sphere with its center.
  - The simplices are compared as sets: the merged groups are numbered in the order their faces are joined.
  - The test takes about 12 s in debug.
- **Mutation check:** dropping the orientation gate, so that every such face is skipped, makes that test fail.

### Method

| Item | Value |
| --- | --- |
| convx | P7-45 at `afae8ca` against the head `4813b67`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Sets | The shorter run of `docs/verification.md`, with Delaunay `sphere` D4 10^4 and `cube` D3 10^6 |
| Rounds | P7-45 and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A second run of `cube` D3, 20 × 3, and one with both sides built with one codegen unit |

Every set published the same counts on both sides.

### Against P7-45

| Set | P7-45 | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.59 ms (0.52 ms–0.77 ms) | 0.56 ms (0.52 ms–0.77 ms) | 0.95, inside the spread |
| Hull `cube` D2 10^5 | 5.31 ms (4.79 ms–6.03 ms) | 5.06 ms (4.59 ms–5.89 ms) | 0.95, inside the spread |
| Hull `sphere` D2 10^4 | 1.26 ms (1.12 ms–1.49 ms) | 1.23 ms (1.17 ms–1.39 ms) | 0.97, inside the spread |
| Hull `sphere` D2 10^5 | 12.5 ms (11.2 ms–13.7 ms) | 12.2 ms (11.1 ms–14.0 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^4 | 1.53 ms (1.36 ms–2.34 ms) | 1.49 ms (1.28 ms–1.98 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 12.5 ms (11.3 ms–15.9 ms) | 12.5 ms (12.1 ms–21.8 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 62.2 ms (57.9 ms–79.5 ms) | 61.8 ms (58.4 ms–66.9 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^4 | 22.4 ms (20.9 ms–30.6 ms) | 22.2 ms (20.4 ms–23.6 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 257 ms (246 ms–283 ms) | 266 ms (246 ms–285 ms) | 1.03, inside the spread |
| Hull `cube` D4 10^4 | 5.45 ms (5.22 ms–7.86 ms) | 5.49 ms (5.26 ms–6.51 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 33.1 ms (32.1 ms–40.8 ms) | 33.2 ms (32.1 ms–35.7 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 92.3 ms (89.6 ms–99.4 ms) | 93.2 ms (89.6 ms–101 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.14 s (1.13 s–1.16 s) | 1.13 s (1.12 s–1.15 s) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 59.5 ms (56.5 ms–73.0 ms) | 59.0 ms (56.8 ms–66.0 ms) | 0.99, inside the spread |
| Hull `sphere` D5 10^4 | 812 ms (793 ms–956 ms) | 808 ms (791 ms–913 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 573 ms (562 ms–594 ms) | 570 ms (561 ms–584 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 5.60 ms (5.38 ms–6.13 ms) | 5.69 ms (5.49 ms–6.73 ms) | 1.02, inside the spread |
| Delaunay `cube` D2 10^5 | 55.9 ms (54.4 ms–62.1 ms) | 56.3 ms (54.7 ms–62.7 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^6 | 598 ms (594 ms–608 ms) | 596 ms (592 ms–598 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.63 ms (5.42 ms–6.72 ms) | 5.57 ms (5.43 ms–5.91 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^5 | 45.3 ms (44.1 ms–49.0 ms) | 45.0 ms (44.0 ms–50.9 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^6 | 442 ms (438 ms–460 ms) | 441 ms (433 ms–465 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^4 | 36.1 ms (34.9 ms–39.7 ms) | 36.9 ms (36.0 ms–39.8 ms) | 1.02, inside the spread |
| Delaunay `cube` D3 10^5 | 383 ms (374 ms–403 ms) | 390 ms (383 ms–405 ms) | 1.02, inside the spread |
| Delaunay `cube` D3 10^6 | 4.14 s (4.13 s–4.19 s) | 4.22 s (4.17 s–4.24 s) | 1.02, inside the spread |
| Delaunay `sphere` D3 10^4 | 84.1 ms (82.2 ms–91.5 ms) | 79.3 ms (77.9 ms–81.5 ms) | 0.94, faster beyond the spread |
| Delaunay `sphere` D3 10^5 | 804 ms (800 ms–806 ms) | 759 ms (753 ms–774 ms) | 0.94, faster beyond the spread |
| Delaunay `cube` D4 10^4 | 679 ms (663 ms–751 ms) | 632 ms (609 ms–653 ms) | 0.93, faster beyond the spread |
| Delaunay `sphere` D4 10^4 | 1.10 s (1.08 s–1.19 s) | 996 ms (979 ms–1.08 s) | 0.91, faster beyond the spread |
| Delaunay `cube` D5 10^4 | 7.55 s (7.45 s–7.68 s) | 7.33 s (7.25 s–7.37 s) | 0.97, faster beyond the spread |


The second run of `cube` D3, 20 × 3: 10^4 1.02 and 10^5 1.01, inside the spread. With both sides built with one codegen unit, 20 × 3:

- `cube` D3 10^4 and 10^5 read 1.00;
- `cube` D4 10^4 reads 0.96.

### Reading

- **The keep criterion is met.** Delaunay `cube` D4 10^4 reads 0.93 and `cube` D5 10^4 0.97, both faster beyond the spread.
- **`sphere` D4 10^4 reads 0.91 and `sphere` D3 0.94**, faster beyond the spread.
- **`cube` D3 does not gain.** It read 1.01 to 1.02 in two runs, and 1.00 with one codegen unit. Its tests of the faces left unknown were already on the fixed-size entry of P7-45, where an orientation of four sites costs about as much as reading them.
- **D2 reads 0.99 to 1.02.** Its insertion by flips leaves few faces unknown.
- **Nothing is slower beyond the spread.**

## Delaunay D2 to D4 after P7-45 and P7-48: where the time goes, PR #459 (#454)

The spike P7-49. A parity run of the head of P7-48 (`4813b67`, under WSL2, not recorded as a parity section) read Delaunay against CGAL:

- D2 at 1.28 to 1.37, from 1.48 to 1.60 (#438);
- `cube` D3 at 1.22 to 1.29, from 1.50 to 1.59;
- `cube` D4 10^4 at 1.10, and `cube` D5 10^4 at 1.07.

### Method

| Item | Value |
| --- | --- |
| convx | `4813b67` under WSL2, `--release` with debug info, with the phase marks of the parity run |
| Phases | `build()` split into construction, the pass after the insertion, publication, and what comes before the insertion, from that parity run; the pass after the insertion split further by marks in a scratch copy |
| Instructions | callgrind, one build, aggregated by source file (inlined code keeps its file) |
| CGAL with the same output | A scratch program (not committed): `Delaunay_triangulation_2/3` built from (point, input index) pairs, with a vertex base and a cell base with info. Each finite cell's vertices (input indices) and neighbors (cell numbers) are then written into flat arrays. Pinned, medians of 3 builds |

### Construction alone is about CGAL's whole build

| Set | `build()` / CGAL | Construction / CGAL | Pass after the insertion | Publication | Before the insertion |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 1.36 | 1.04 | 19.2% | 1.5% | 2.9% |
| `sphere` D2 10^5 | 1.37 | 1.10 | 14.2% | 0.9% | 4.3% |
| `cube` D2 10^6 | 1.29 | 0.96 | 18.3% | 1.3% | 5.9% |
| `cube` D3 10^5 | 1.26 | 1.04 | 14.7% | 0.7% | 2.0% |
| `cube` D3 10^6 | 1.29 | 1.03 | 18.2% | 0.6% | 1.0% |
| `cube` D4 10^4 | 1.10 | 0.98 | 10.0% | 0.2% | 0.8% |
| `cube` D5 10^4 | 1.07 | 0.95 | 11.3% | 0.1% | 0.0% |

The pass after the insertion, in milliseconds of one build:

| Set | Tests of the faces left unknown | Draft | `finish` and `publish` |
| --- | ---: | ---: | ---: |
| `cube` D2 10^5 | 6.3 | 3.9 | 1.2 |
| `cube` D3 10^5 | 40 (49 before P7-48) | 14 | 4.2 |
| `cube` D4 10^4 | 38 (70) | 28 | 2.2 |
| `cube` D5 10^4 | 588 (760) | 245 | 12 |

In `cube` D3 the tests take about 80 ns a face over 506,576 faces. That is reading the neighbor's vertices and the sites, not arithmetic.

### CGAL with the same output

ADR 0006 compares "with the same output". CGAL's reference times the construction from the points and produces no output.

| Set | The reference: construction from the points | From (point, index) pairs | Writing the arrays | Same output | Increase |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^5 | 41.6 ms | 50.1 ms | 3.7 ms | 53.8 ms | +29% |
| `sphere` D2 10^5 | 32.3 ms | 38.9 ms | 2.7 ms | 41.6 ms | +29% |
| `cube` D2 10^6 | 481 ms | 651 ms | 45 ms | 696 ms | +45% |
| `cube` D3 10^5 | 319 ms | 328 ms | 20.6 ms | 349 ms | +9% |
| `cube` D3 10^6 | 3.21 s | 3.44 s | 250 ms | 3.69 s | +15% |

What convx publishes beyond this is left out of it: each simplex's sites in ascending order, and the cospherical groups. Against it, convx's `build()` would read about 1.05 on `cube` D2 10^5, 1.06 on `sphere` D2 10^5, 0.89 on `cube` D2 10^6, 1.15 on `cube` D3 10^5, and 1.12 on `cube` D3 10^6.

### Instructions by source file

| Set | Predicate arithmetic (`semi_static.rs`) | Insertion (`insert.rs`) | Bounds checks of indexing |
| --- | ---: | ---: | ---: |
| `cube` D2 10^5 | 96 M (14%) | 168 M (25%) | 69 M (10%) |
| `cube` D3 10^5 | 1,117 M (29%) | 1,014 M (26%) | 377 M (10%) |
| `cube` D4 10^4 | 5,803 M (61%) | 1,012 M (11%) | 264 M (3%) |

### Reading

- **Construction is at CGAL's pace.** What remains of D2 and D3 is the pass after the insertion (14 to 19%), publication, and in D2 the work before the insertion.
- **CGAL's Delaunay reference does less than the same output.** With input indices and the arrays written, it takes 9 to 45% longer. The Grill on #454 decided that the reference gives the same output: P7-50 (#455).
- **The hull's references** are audited the same way after it: P7-51 (#456).
- **Rows that follow either way**, each prototyped first:
  - the pass after the insertion in one walk for D2 and D3: P7-52 (#457);
  - fixed-size storage of cells and sites for D2 and D3, against the 10% of bounds checks: P7-53 (#458).

## Parity: do the hull's references produce the same output? PR #461 (#456)

The spike P7-51. After P7-50 (#455) made CGAL's Delaunay reference give convx's output, the Grill on #454 asked the same of the hull's references.

### What each produces

| Reference | What its timed section produces | Against convx's `build()` |
| --- | --- | --- |
| Qhull (`qconvex`, compute) | The merged facets, their vertices and neighbors, the coplanar points | Lacks the interior list (one pass over the points) and the triangulation of non-simplicial facets |
| CGAL D = 2 (`convex_hull_2`) | The extreme points as copies, in order | Lacks indices, and the coplanar and interior lists |
| CGAL D = 3 (`convex_hull_3` into a `Surface_mesh`) | A triangulated surface of point copies, with its connectivity | Lacks merged facets, indices, and the two lists |
| CGAL D >= 4 (`CGAL::Triangulation`) | The full cells around the infinite vertex | Likewise |

convx's `build()` publishes the representatives, the vertices, the coplanar and interior points, the logical facets with their extreme points and neighbors, and the boundary simplices. The planes, and the public numbering for D >= 3, are computed on first use.

### Qhull's options

Pinned, medians of 3, under WSL2, Qhull's compute time:

| Set | Default | `Qc` | `Qc Qi` |
| --- | ---: | ---: | ---: |
| `sphere` D3 10^4 | 22.8 ms | 22.1 ms | 20.4 ms |
| `sphere` D3 10^5 | 266 ms | 273 ms | 230 ms |
| `sphere` D3 10^6 | 3.50 s | 3.53 s | 3.33 s |
| `cubesurf` D3 10^5 | 31.6 ms | 28.8 ms | 28.4 ms |
| `cube` D2 10^6 | 55.1 ms | 56.6 ms | 270 ms |
| `cube` D4 10^5 | 37.2 ms | 37.0 ms | 246 ms |
| `sphere` D5 10^5 | 11.16 s | 10.98 s | 10.39 s |

Keeping the coplanar points (`Qc`) reads as the default. `Qi` assigns each interior point to its nearest facet, more than convx does.

### CGAL's D = 2 hull as input indices

| Set | From the points | As input indices (`Convex_hull_traits_adapter_2`) |
| --- | ---: | ---: |
| `cube` D2 10^5 | 4.5 ms | 5.6 ms |
| `cube` D2 10^6 | 43 ms | 68 ms |
| `sphere` D2 10^4 | 0.69 ms | 0.77 ms |
| `sphere` D2 10^5 | 8.0 ms | 10.1 ms |
| `sphere` D2 10^6 | 93 ms | 157 ms |

The index form of `convex_hull_3` in this CGAL takes no traits adapter, so D = 3 was not measured as indices.

### Reading

- **Qhull stands:** its compute holds what convx publishes but the interior list.
- **CGAL's D = 2 hull** is judged as input indices, an amendment of ADR 0006 (Grill on #456). On the parity run of `4813b67` that would move:
  - hull `cube` D2 10^6 from 1.17 to about 0.74;
  - `sphere` D2 10^4 from 1.44 to about 1.29;
  - `sphere` D2 10^5 from 1.43 to about 1.13;
  - `sphere` D2 10^6 from 1.43 to about 0.85.
- **CGAL's D >= 3 hull references stand:** no set of P7 not met has CGAL as its larger ratio there.
- The next parity run uses both amendments and reports the ratios to the references from the points beside them.

## Delaunay D2 and D3: one walk after the insertion, and bounds checks, prototypes, PR #462 (#457, #458)

P7-52 and P7-53, from the Grill on #454. Each was to become a feature row only if its prototype gained. The spike P7-49 (#459) had found:

- construction at CGAL's pace;
- the pass after the insertion at 14 to 19% of `build()`, its tests of the faces left unknown at about 80 ns a face in `cube` D3;
- bounds checks of indexing at about 10% of the instructions.

### Method

| Item | Value |
| --- | --- |
| convx | The head of P7-48 (`4813b67`) against each copy, not committed. rustc 1.97.1, `--release` with debug info |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Rounds | Alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |

Every set published the same counts on both sides, and every test passed on each copy.

### P7-52: the pass after the insertion in one walk

For D = 2 and 3, the copy numbers the finite simplices first. It then writes the draft in the same walk that tests the faces, as if no group merged, and writes it again as before only when a group did.

| Set | Base | One walk | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 6.08 ms (5.53 ms–7.39 ms) | 6.13 ms (5.52 ms–6.77 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^5 | 58.2 ms (54.4 ms–73.5 ms) | 58.0 ms (54.8 ms–76.6 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 591 ms (590 ms–616 ms) | 598 ms (597 ms–599 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.58 ms (5.38 ms–6.08 ms) | 5.55 ms (5.40 ms–5.85 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 45.1 ms (43.4 ms–64.0 ms) | 45.0 ms (43.9 ms–58.8 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^6 | 438 ms (435 ms–453 ms) | 438 ms (438 ms–466 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^4 | 36.7 ms (35.7 ms–39.9 ms) | 36.8 ms (35.0 ms–41.4 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 390 ms (377 ms–409 ms) | 389 ms (378 ms–410 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^6 | 4.19 s (4.17 s–4.24 s) | 4.19 s (4.18 s–4.23 s) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 80.5 ms (78.6 ms–92.0 ms) | 80.2 ms (78.4 ms–90.3 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 651 ms (637 ms–666 ms) | 645 ms (634 ms–661 ms) | 0.99, inside the spread |

Every set reads 0.99 to 1.01. The second walk was not the cost: the tests read each neighbor's vertices and four sites at random, in either order.

### P7-53: how much the bounds checks can cost

Fixed-size storage would keep one check per cell or site. To bound what any such storage could save, a copy removed the checks entirely from the hot accessors: `Sites::fixed`, `Mesh::vertices_of`, `Mesh::neighbor`, and `Mesh::set_neighbor`. This used `unsafe` unchecked reads, allowed in that copy only; convx forbids `unsafe`, so the copy measures a ceiling, not a change.

| Set | Base | Unchecked | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 6.19 ms (5.58 ms–7.03 ms) | 5.94 ms (5.36 ms–6.74 ms) | 0.96, inside the spread |
| Delaunay `cube` D2 10^5 | 61.2 ms (57.3 ms–67.1 ms) | 60.7 ms (55.5 ms–72.5 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^6 | 646 ms (643 ms–661 ms) | 630 ms (609 ms–635 ms) | 0.98, faster beyond the spread |
| Delaunay `sphere` D2 10^4 | 5.89 ms (5.47 ms–6.73 ms) | 5.83 ms (5.41 ms–6.62 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^5 | 49.1 ms (45.9 ms–63.9 ms) | 46.8 ms (44.1 ms–62.3 ms) | 0.95, inside the spread |
| Delaunay `sphere` D2 10^6 | 470 ms (467 ms–475 ms) | 451 ms (449 ms–463 ms) | 0.96, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 40.3 ms (38.3 ms–44.1 ms) | 38.6 ms (36.9 ms–43.5 ms) | 0.96, inside the spread |
| Delaunay `cube` D3 10^5 | 437 ms (423 ms–499 ms) | 415 ms (397 ms–465 ms) | 0.95, inside the spread |
| Delaunay `cube` D3 10^6 | 4.84 s (4.62 s–4.87 s) | 4.69 s (4.42 s–4.78 s) | 0.97, inside the spread |
| Delaunay `sphere` D3 10^4 | 85.2 ms (81.2 ms–99.4 ms) | 85.0 ms (80.9 ms–93.9 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 692 ms (674 ms–824 ms) | 697 ms (677 ms–735 ms) | 1.01, inside the spread |

Removing every check of those accessors reads 0.95 to 0.99 on D2 and 0.95 to 0.97 on `cube` D3. Only the D2 sets of 10^6 are faster beyond the spread. Fixed-size arrays would keep part of the checks, so they would save less than this ceiling.

### Reading

- **No feature row follows either prototype** (Grill on #454).
  - One walk saves nothing.
  - Removing the checks entirely would save at most 1 to 5%, within the spread of most sets, and a safe storage would save less.
- **What remains of D2 and D3 after construction** is reading memory at random in the pass after the insertion. The next parity run, with the references of P7-50 and P7-51, places the sets again.

## The P7 parity run under the same-output references, PR #466 (#463)

Due after P7-45 (#449) and P7-48 (#452). It is the first run under the references of P7-50 (#460) and P7-51 (#461): CGAL's Delaunay and D = 2 hull references give the same output as convx (ADR 0006, amendments of 2026-10-11).

### Method

As in the run of #438: `main` at `e42ca61`, `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. "(ref.)" marks a reference that is not judged (ADR 0006). A set is met at 1.05 when convx's `build()` is at most 1.05 times each judged reference, and at 1.00 when at most the time of each.

- **"/ CGAL"** is against the reference with the same output.
- **"/ CGAL from the points"** is against the construction from the points, the reference before the amendments. It is given for the Delaunay sets and the D = 2 hull sets, where the reference changed.
- **The runs of #438 and this one were on different days.** On the sets no row of this period touched (hull `cube` and `sphere` D5 and D6), convx's `build()` read 0.98 of the run of #438 (median; 0.96 to 1.01), CGAL 0.96 (0.95 to 0.99), and Qhull 0.95 (0.92 to 1.00).
- **The counts of the three tools agree** but on hull `cubesurf` D3 and `grid` D6, as `docs/verification.md` expects.

### P7 judgement

Met at 1.05: 28 of 41 (hull 20, Delaunay 8); 23 in the run of #438. Met at 1.00: 25 of 41 (hull 18, Delaunay 7); 20 in the run of #438. Against the references from the points instead, 23 and 20 would be met.

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.45 ms | 0.77 | 0.90 | 0.80 | met | met |
| Hull `cube` D2 10^5 | 4.34 ms | 0.80 | 1.02 | 0.79 | met | met |
| Hull `cube` D2 10^6 | 52.5 ms | 0.79 | 1.20 | 0.95 | met | met |
| Hull `cube` D3 10^4 | 1.18 ms | 0.56 | – | 0.79 | met | met |
| Hull `cube` D3 10^5 | 9.52 ms | 0.38 | – | 0.82 | met | met |
| Hull `cube` D3 10^6 | 133 ms | 0.25 | – | 0.95 | met | met |
| Hull `cube` D4 10^4 | 5.05 ms | 0.12 | – | 0.77 | met | met |
| Hull `cube` D4 10^5 | 31.8 ms | 0.06 | – | 0.96 | met | met |
| Hull `cube` D5 10^4 | 52.3 ms | 0.24 | – | 0.73 | met | met |
| Hull `cube` D5 10^5 | 206 ms | 0.13 | – | 0.67 | met | met |
| Hull `cube` D6 10^4 | 575 ms | 0.29 | – | 0.48 | met | met |
| Hull `cube` D6 10^5 | 2.50 s | 0.24 | – | 0.40 | met | met |
| Hull `sphere` D2 10^4 | 1.00 ms | 1.37 | 1.64 | 0.15 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 10.6 ms | 1.12 | 1.44 | 0.10 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 122 ms | 0.84 | 1.41 | 0.07 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 18.6 ms | 0.87 | – | 1.06 | not met | not met |
| Hull `sphere` D3 10^5 | 244 ms | 0.64 | – | 1.04 | met | not met |
| Hull `sphere` D3 10^6 | 3.95 s | 0.52 | – | 1.22 | not met | not met |
| Hull `sphere` D4 10^4 | 89.4 ms | 0.75 | – | 0.98 | met | met |
| Hull `sphere` D4 10^5 | 1.16 s | 0.97 | – | 0.92 | met | met |
| Hull `sphere` D5 10^4 | 796 ms | 0.84 | – | 0.98 | met | met |
| Hull `sphere` D5 10^5 | 10.37 s | 0.95 | – | 1.01 | met | not met |
| Hull `sphere` D6 10^4 | 6.67 s | 0.73 | – | 0.85 | met | met |
| Hull `cubesurf` D3 10^5 | 54.7 ms | 1.48 | – | 1.94 | not met | not met |
| Hull `grid` D6 10^4 | 236 ms | 0.97 (ref.) | – | 0.71 | met | met |
| Delaunay `cube` D2 10^4 | 5.35 ms | 1.18 | 1.35 | 0.30 | not met | not met |
| Delaunay `cube` D2 10^5 | 55.2 ms | 1.07 | 1.30 | 0.24 | not met | not met |
| Delaunay `cube` D2 10^6 | 579 ms | 0.88 | 1.35 | 0.18 | met | met |
| Delaunay `sphere` D2 10^4 | 4.92 ms | 1.15 | 1.32 | 0.22 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 43.7 ms | 1.09 | 1.42 | 0.12 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^6 | 416 ms | 0.86 | 1.45 | 0.04 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 35.1 ms | 1.11 | 1.19 | 0.40 | not met | not met |
| Delaunay `cube` D3 10^5 | 376 ms | 1.13 | 1.24 | 0.31 | not met | not met |
| Delaunay `cube` D3 10^6 | 4.12 s | 1.12 | 1.28 | 0.28 | not met | not met |
| Delaunay `sphere` D3 10^4 | 66.1 ms | 0.52 | 0.51 | 0.60 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 631 ms | 0.68 | 0.69 | 0.35 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.49 s | 0.83 | 0.85 | 0.33 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 675 ms | 1.05 | 1.10 | 0.87 | not met | not met |
| Delaunay `sphere` D4 10^4 | 845 ms | 0.34 | 0.34 | 0.69 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.02 s | 1.02 | 1.07 | 0.90 | met | not met |
| Delaunay `sphere` D5 10^4 | 10.66 s | 0.31 | 0.31 | 0.75 (ref.) | met | met |

### What moved

- **Delaunay, by P7-45 and P7-48:** `build()` took 0.78 to 0.87 of the run of #438 on D2 and `cube` D3, and 0.87 to 0.89 on `sphere` D3. Against CGAL with the same output:
  - `cube` and `sphere` D2 10^6 read 0.88 and 0.86, now met;
  - `cube` D2 10^5 reads 1.07 and `sphere` D2 10^5 1.09; 10^4 reads 1.18 and 1.15;
  - `cube` D3 reads 1.11 to 1.13;
  - `cube` D4 10^4 reads 1.05, just outside, and `cube` D5 10^4 1.02, now met.
- **Hull D2, by the reference as input indices:** `cube` D2 reads 0.77 to 0.80, and `sphere` D2 10^6 0.84, now met. `sphere` D2 10^4 and 10^5 read 1.37 and 1.12.
- **Against the references from the points,** Delaunay D2 reads 1.30 to 1.45 and `cube` D3 1.19 to 1.28: the change of the reference lowers those ratios by 0.08 to 0.59.
- **Hull `sphere` D3** reads 1.06, 1.04, and 1.22 against Qhull. Its 10^4 set moved from 1.01 to 1.06 while convx's `build()` read 0.96 of the run of #438: Qhull read faster in this run.

### What is not met

13 sets:

| Group | Sets | Worst ratio |
| --- | ---: | --- |
| Delaunay `cube` and `sphere` D2 10^4 and 10^5 | 4 | 1.07 to 1.18 against CGAL |
| Delaunay `cube` D3 | 3 | 1.11 to 1.13 against CGAL |
| Delaunay `cube` D4 10^4 | 1 | 1.05 against CGAL |
| Hull `sphere` D2 10^4 and 10^5 | 2 | 1.37 and 1.12 against CGAL |
| Hull `sphere` D3 10^4 and 10^6 | 2 | 1.06 and 1.22 against Qhull |
| Hull `cubesurf` D3 10^5 | 1 | 1.94 against Qhull, 1.48 against CGAL |

## Where the time goes on the 13 sets left, PR #471 (#467)

The spike P7-54. The parity run of #466 left 13 sets not met. As P7-32 (#413) did for the run of #404, it reads their phases and their instructions, and names the rows that follow.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `e42ca61` (phases, from the parity run) and `2d2ee4f` (callgrind, counters), under WSL2, `--release` with debug info |
| Instructions | callgrind, one build each, inclusive by function |
| Merge counters | A scratch copy that prints, per merge, whether `Records` is empty, the ridges joined, and the simplices that inherited a plane number. The inputs are every family of `tests/common/generator.rs`, D = 3 to 6, 30, 200, and 2,000 points, seeds 1 to 5: 480 builds and 241,132 sub-hulls inside them |

### Phases of `build()`

| Set | Ratio | Construction | Pass after it | Publication | Before it |
| --- | ---: | ---: | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 1.18 | 77.8% | 16.5% | 1.7% | 4.1% |
| Delaunay `cube` D2 10^5 | 1.07 | 76.9% | 18.2% | 1.4% | 3.5% |
| Delaunay `sphere` D2 10^4 | 1.15 | 84.8% | 10.4% | 1.2% | 3.7% |
| Delaunay `sphere` D2 10^5 | 1.09 | 79.8% | 13.6% | 0.9% | 5.7% |
| Delaunay `cube` D3 10^4 | 1.11 | 84.1% | 14.2% | 0.8% | 1.0% |
| Delaunay `cube` D3 10^5 | 1.13 | 84.3% | 14.2% | 0.8% | 0.7% |
| Delaunay `cube` D3 10^6 | 1.12 | 79.8% | 18.6% | 0.6% | 1.0% |
| Delaunay `cube` D4 10^4 | 1.05 | 89.3% | 10.1% | 0.2% | 0.4% |
| Hull `sphere` D2 10^4 | 1.37 | 59.0% | 10.8% | 16.2% | 14.1% |
| Hull `sphere` D2 10^5 | 1.12 | 59.1% | 11.3% | 15.9% | 13.7% |
| Hull `sphere` D3 10^4 | 1.06 | 80.0% | 16.7% | 2.3% | 1.0% |
| Hull `sphere` D3 10^6 | 1.22 | 77.7% | 20.5% | 1.1% | 0.7% |
| Hull `cubesurf` D3 10^5 | 1.94 | 45.3% | 51.7% | 0.1% | 2.9% |

### The hull, by function

| Set | Part | Share of the program |
| --- | --- | ---: |
| `sphere` D3 10^5 | `classify_built` | 12.6% |
| | of which the coplanar merge (`merge`) | 8.2% |
| | of which `found_vertices` | 9.3% |
| `cubesurf` D3 10^5 | `found_vertices` | 60.2% |
| | its own lines | 19.0% (own) |
| | the side tests of points against faces (`side`, own, from `found_vertices` and construction) | 27.0% (own) |
| | sorting `u32` lists, everywhere in the build | about 6% (own) |
| `sphere` D2 10^5 | `accept` | 4.5% |
| | `classify_built` | 5.6% |

`cubesurf` D3 10^5 has about 16,000 points on each face of the cube. In `distance_zeros`, a point with no record is tested against every group, and a point on a face has distance zero, which the cull cannot prove and the exact sign must. The sort's share was first read from the inclusive cost of the recursive `quicksort` (23%), which counts the recursion again at each level; its own cost is about 6% (#468).

### The coplanar merge (P7-34, #410)

- **Skipping it on a condition does not hold.**
  - 12 of the 480 builds joined ridges while `Records` was empty: `Records` holds the zero signs of outside points, not those of the visibility walk.
  - None of the 480 joined a ridge while no simplex had inherited a plane number, but 10 of the sub-hulls did.
  - As in the lemma of #448, two adjacent coplanar simplices of one insertion force a zero sign in the walk only when their D sites other than the apex are affinely independent.
- **Most of its instructions build each group's lists:** about 5 to 6% of the program, against about 1.5% for the ridge tests, while every group is one simplex.

The Grill on #410 asked for a prototype of a path that writes the groups of one simplex each straight into presized lists, with debug builds comparing it with the general path on every merge. It is not committed. Against `main` (`2d2ee4f`), pinned and alternated, 10 rounds × 3 builds; 5 × 1 over about half a second:

| Set | `main` | Fast path | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.55 ms (0.48 ms–0.82 ms) | 0.54 ms (0.48 ms–0.70 ms) | 0.98, inside the spread |
| Hull `cube` D2 10^5 | 4.89 ms (4.38 ms–5.80 ms) | 4.92 ms (4.55 ms–5.88 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 1.18 ms (1.07 ms–1.34 ms) | 1.20 ms (1.10 ms–1.52 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^5 | 12.2 ms (11.1 ms–13.3 ms) | 12.1 ms (11.0 ms–15.3 ms) | 0.99, inside the spread |
| Hull `cube` D3 10^4 | 1.50 ms (1.26 ms–1.81 ms) | 1.47 ms (1.26 ms–2.29 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 11.7 ms (10.7 ms–13.5 ms) | 11.7 ms (10.6 ms–22.3 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 58.5 ms (56.5 ms–64.0 ms) | 59.4 ms (57.4 ms–71.0 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^4 | 20.8 ms (20.2 ms–21.8 ms) | 20.5 ms (19.6 ms–24.0 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 251 ms (243 ms–310 ms) | 246 ms (238 ms–258 ms) | 0.98, inside the spread |
| Hull `cube` D4 10^4 | 5.50 ms (5.22 ms–6.62 ms) | 5.50 ms (5.20 ms–6.63 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 33.0 ms (31.9 ms–34.6 ms) | 33.2 ms (31.7 ms–35.2 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^4 | 91.2 ms (88.6 ms–101 ms) | 89.6 ms (87.0 ms–105 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^5 | 1.10 s (1.09 s–1.13 s) | 1.09 s (1.07 s–1.10 s) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 54.6 ms (53.6 ms–57.4 ms) | 54.3 ms (52.8 ms–57.2 ms) | 0.99, inside the spread |
| Hull `cube` D5 10^5 | 214 ms (210 ms–224 ms) | 213 ms (209 ms–222 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 801 ms (785 ms–820 ms) | 799 ms (778 ms–826 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 569 ms (558 ms–595 ms) | 566 ms (557 ms–624 ms) | 0.99, inside the spread |

The prototype reads 0.98 to 0.99 on hull `sphere` D3 and D4, inside the spread. The instructions it removes run fast, and the time they took was smaller than their share of the instructions. P7-34 is withdrawn after measurement.

### Reading

- **Delaunay:** the pass after the insertion (10 to 19%) is about each set's gap (5 to 18%). P7-56 (#469) prototypes deciding the faces left unknown when the insertion links them.
- **Hull `cubesurf`:** `found_vertices` is about two thirds of `build()`, most of it the exact side tests of points on the faces against every group, and its own lines. P7-55 (#468) starts from that: how many points are scanned without a record, how many groups each meets, and the cost of a zero sign.
- **Hull `sphere` D3:** the merge's list building gives 1 to 2%, inside the spread; P7-34 is withdrawn. What is left there is construction itself (78 to 80%).
- **Hull `sphere` D2 10^4 and 10^5:** acceptance, publication, and classification share the time. The spike P7-57 (#470) follows.

## Hull: the recorded points of a face tested against its neighbors together, PR #472 (#468)

P7-55. The spike P7-54 (#471) put hull `cubesurf` D3 10^5 (1.94 against Qhull) mostly in classification.

### Counts before the change

A scratch copy of `main` (`2d2ee4f`) counted, in `distance_zeros`:

- **All 99,833 points not on the complex start from a record:** they were found on a plane during construction, and none is scanned against every group.
- **The walk from each record** tested the point, one at a time, against every neighbor of its start group. There are 169 groups, with at most 31 neighbors each and 3 at the median. That made 2,578,023 side tests, none of them zero, and took 24 ms of a 64 ms build, with timers in.

### The change

- The recorded points are taken by their start group.
- Each group's points are tested against each of its neighbors together, through that neighbor's cull plane (`CullPlane::mark_sides`), as the scan of points without a record already did.
- Only a side the plane does not prove is evaluated exactly.
- A point found on a neighbor's plane walks on from there, one at a time, as before. The signs, and so the classification, are the same.
- **Checks:** debug builds compare every proved side with the exact one, and test every recorded point against every group, as before. Dropping the walk on from a neighbor's plane makes that check fire in four tests.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `2d2ee4f` against the head `3c0f38e`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Sets | The shorter run of `docs/verification.md`, with hull `sphere` D4 10^4 and `cube` D5 10^5 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A second run of four sets, 20 × 3, and one with both sides built with one codegen unit |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.53 ms (0.47 ms–0.66 ms) | 0.51 ms (0.47 ms–0.63 ms) | 0.96, inside the spread |
| Hull `cube` D2 10^5 | 4.83 ms (4.45 ms–6.20 ms) | 5.08 ms (4.56 ms–8.31 ms) | 1.05, inside the spread |
| Hull `sphere` D2 10^4 | 1.19 ms (1.10 ms–2.14 ms) | 1.24 ms (1.10 ms–2.05 ms) | 1.04, inside the spread |
| Hull `sphere` D2 10^5 | 11.9 ms (10.8 ms–14.5 ms) | 12.1 ms (10.7 ms–13.2 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^4 | 1.46 ms (1.24 ms–1.92 ms) | 1.48 ms (1.29 ms–1.90 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^5 | 12.1 ms (10.9 ms–15.3 ms) | 12.3 ms (11.0 ms–29.7 ms) | 1.02, inside the spread |
| Hull `cubesurf` D3 10^5 | 59.0 ms (57.6 ms–65.4 ms) | 47.9 ms (46.3 ms–51.4 ms) | 0.81, faster beyond the spread |
| Hull `sphere` D3 10^4 | 22.4 ms (20.5 ms–56.5 ms) | 22.8 ms (21.2 ms–28.3 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^5 | 249 ms (242 ms–265 ms) | 252 ms (248 ms–272 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^4 | 5.76 ms (5.36 ms–8.63 ms) | 5.69 ms (5.29 ms–10.3 ms) | 0.99, inside the spread |
| Hull `cube` D4 10^5 | 33.5 ms (32.6 ms–36.1 ms) | 33.3 ms (32.4 ms–36.6 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 92.0 ms (89.2 ms–98.6 ms) | 92.7 ms (89.6 ms–101 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.13 s (1.12 s–1.17 s) | 1.13 s (1.12 s–1.15 s) | 0.99, inside the spread |
| Hull `cube` D5 10^4 | 54.8 ms (53.2 ms–58.7 ms) | 54.8 ms (53.3 ms–67.5 ms) | 1.00, inside the spread |
| Hull `cube` D5 10^5 | 212 ms (208 ms–237 ms) | 212 ms (207 ms–219 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 802 ms (785 ms–818 ms) | 799 ms (780 ms–833 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 559 ms (551 ms–608 ms) | 558 ms (547 ms–576 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 5.55 ms (5.38 ms–7.39 ms) | 5.57 ms (5.41 ms–6.63 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 55.3 ms (54.0 ms–59.4 ms) | 55.3 ms (54.4 ms–58.4 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 600 ms (594 ms–608 ms) | 602 ms (592 ms–664 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.49 ms (5.37 ms–5.97 ms) | 5.47 ms (5.37 ms–5.88 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 44.7 ms (43.4 ms–47.3 ms) | 44.5 ms (43.6 ms–48.9 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^6 | 441 ms (440 ms–462 ms) | 454 ms (439 ms–490 ms) | 1.03, inside the spread |
| Delaunay `cube` D3 10^4 | 36.3 ms (35.4 ms–40.5 ms) | 35.8 ms (35.1 ms–38.8 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 385 ms (375 ms–396 ms) | 383 ms (373 ms–397 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 79.4 ms (77.8 ms–85.1 ms) | 81.2 ms (78.5 ms–88.2 ms) | 1.02, inside the spread |
| Delaunay `sphere` D3 10^5 | 756 ms (753 ms–775 ms) | 765 ms (762 ms–785 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 643 ms (630 ms–673 ms) | 665 ms (652 ms–688 ms) | 1.03, inside the spread |
| Delaunay `cube` D5 10^4 | 7.40 s (7.31 s–7.45 s) | 7.25 s (7.24 s–7.37 s) | 0.98, inside the spread |

**The second run (20 × 3):** hull `cubesurf` D3 10^5 reads 0.81. Hull `cube` D2 10^5 and `sphere` D2 10^4 read 1.00. Delaunay `cube` D4 10^4 reads 1.04.

**With one codegen unit on both sides (20 × 3):** `cubesurf` D3 10^5 reads 0.80, faster beyond the spread; Delaunay `cube` D4 10^4 reads 1.02.

### Reading

- **Hull `cubesurf` D3 10^5 reads 0.81, faster beyond the spread,** and 0.80 with one codegen unit. On the parity ratio of #466 that is about 1.57 against Qhull, from 1.94.
- **Nothing is slower beyond the spread.**
  - Hull `cube` D2 10^5 and `sphere` D2 10^4, which the change does not reach, read 1.05 and 1.04 in the first run and 1.00 in the second.
  - Delaunay `cube` D4 10^4, which it does not reach either, read 1.03 and 1.04, and 1.02 with one codegen unit.
- What remains of `cubesurf` is construction (45%) and the rest of classification.

## Delaunay: deciding the faces left unknown at linking, prototype, PR #473 (#469)

P7-56, from the Grill on #467. The pass after the Delaunay insertion tests each face left unknown by one orientation of F, the D + 1 sites other than the newest (P7-48, #452). Those tests read the neighbor's vertices at random (#459). The prototype decided F's dependence instead when the cavity insertion links two new simplices, and recorded an independent face, so that the pass reads nothing more of it.

### The prototype

A copy of `main` (`2d2ee4f`), not committed, for D >= 3:

- In `link_by_keys` (D = 3) and in the turn around ridges (D >= 4), a link between two new simplices from different cavity simplices takes the orientation of F there.
- A face whose F is independent is recorded `Independent`, and the pass after the insertion treats it as not cospherical without reading the neighbor.
- Every test passed, and the test-only switch that tests every face left unknown by the lifted orientation compared the same.

Timed against `main`, pinned and alternated: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. Every set published the same counts on both sides.

| Set | `main` | At linking | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 6.16 ms (5.58 ms–6.77 ms) | 6.08 ms (5.68 ms–8.66 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^5 | 60.5 ms (55.7 ms–75.9 ms) | 59.9 ms (55.0 ms–67.2 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.61 ms (5.46 ms–6.13 ms) | 5.72 ms (5.48 ms–6.32 ms) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^5 | 45.2 ms (44.1 ms–48.5 ms) | 45.6 ms (44.2 ms–50.9 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 37.7 ms (36.1 ms–43.9 ms) | 38.5 ms (37.0 ms–44.8 ms) | 1.02, inside the spread |
| Delaunay `cube` D3 10^5 | 393 ms (384 ms–450 ms) | 398 ms (390 ms–414 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^6 | 4.11 s (4.08 s–4.36 s) | 4.17 s (4.04 s–4.26 s) | 1.02, inside the spread |
| Delaunay `sphere` D3 10^4 | 79.3 ms (78.4 ms–85.1 ms) | 81.1 ms (78.8 ms–84.6 ms) | 1.02, inside the spread |
| Delaunay `sphere` D3 10^5 | 764 ms (756 ms–772 ms) | 769 ms (765 ms–770 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 637 ms (623 ms–651 ms) | 768 ms (760 ms–796 ms) | 1.21, slower beyond the spread |
| Delaunay `sphere` D4 10^4 | 995 ms (981 ms–1.03 s) | 1.06 s (1.04 s–1.07 s) | 1.06, slower beyond the spread |
| Delaunay `cube` D5 10^4 | 7.40 s (7.39 s–7.59 s) | 9.70 s (9.53 s–9.80 s) | 1.31, slower beyond the spread |

### Reading

- **Slower, not faster.**
  - `cube` D4 10^4 reads 1.21 and `cube` D5 10^4 1.31, slower beyond the spread;
  - `cube` D3 reads 1.01 to 1.02, and D2, which the change does not reach, 0.99 to 1.02.
- **The linking sees about five times the faces the pass tests.** Most links are undone by later insertions: 2,722,510 links against 506,576 faces tested on `cube` D3 10^5 (#452). An orientation on data just read costs about what the pass saved, in D3. In D4 and D5 each orientation is the larger formula, and the extra ones cost more than the random reads.
- **No feature row follows from this prototype.** As the Grill on #467 decided, a pass after the insertion dedicated to D2 and D3 (ADR 0008) goes to a Grill.

## Delaunay D2: a face left unknown needs no test, PR #475 (#474)

P7-58, from the Grill on #469. The pass after the insertion tested each face left unknown by one orientation of F, the D + 1 sites of the two simplices other than the newest (P7-48, #452).

### The change

For D = 2 (ADR 0008), a face left unknown joins nothing directly. The pass reads neither its triangle across nor any site.

- F is three distinct sites, and three distinct sites on a line lie on no circle. So the F of two cospherical triangles is never dependent.
- By the lemma of design §7, two cospherical triangles with an independent F are already joined through recorded faces. Design §7 says so in both languages.
- **Checks:** the test of #452 compares, on D = 2 among others, what is published with every face left unknown tested by the lifted orientation. A mutation that also skips D = 3 makes it fail.

### Method

| Item | Value |
| --- | --- |
| convx | P7-55 (`76c3c32`, the code of `3c0f38e`) against the head `3ad04bf`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Sets | The shorter run of `docs/verification.md` |
| Rounds | Alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. A second run of three D2 sets, 20 × 3 |

Every set published the same counts on both sides.

### Against P7-55

| Set | P7-55 | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.53 ms (0.48 ms–0.80 ms) | 0.54 ms (0.48 ms–1.15 ms) | 1.01, inside the spread |
| Hull `cube` D2 10^5 | 4.81 ms (4.43 ms–6.00 ms) | 4.90 ms (4.49 ms–5.56 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^4 | 1.15 ms (1.09 ms–4.51 ms) | 1.18 ms (1.08 ms–1.71 ms) | 1.03, inside the spread |
| Hull `sphere` D2 10^5 | 12.0 ms (10.8 ms–27.8 ms) | 12.0 ms (10.8 ms–13.0 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^4 | 1.45 ms (1.25 ms–2.47 ms) | 1.48 ms (1.34 ms–2.89 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^5 | 11.5 ms (11.0 ms–12.7 ms) | 11.4 ms (10.9 ms–12.9 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 47.6 ms (46.5 ms–54.7 ms) | 47.6 ms (46.6 ms–50.9 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 20.9 ms (20.1 ms–23.2 ms) | 21.1 ms (20.3 ms–23.0 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^5 | 250 ms (244 ms–271 ms) | 251 ms (244 ms–261 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^4 | 5.44 ms (5.20 ms–6.37 ms) | 5.49 ms (5.20 ms–6.12 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 33.2 ms (31.9 ms–44.4 ms) | 32.7 ms (31.6 ms–38.7 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^4 | 91.1 ms (88.7 ms–96.3 ms) | 92.0 ms (89.2 ms–94.5 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.14 s (1.14 s–1.21 s) | 1.15 s (1.12 s–1.16 s) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 54.5 ms (53.1 ms–59.8 ms) | 54.2 ms (52.7 ms–56.8 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 788 ms (770 ms–813 ms) | 786 ms (772 ms–802 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 563 ms (544 ms–623 ms) | 563 ms (539 ms–604 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^4 | 5.64 ms (5.46 ms–6.34 ms) | 5.41 ms (5.26 ms–7.50 ms) | 0.96, inside the spread |
| Delaunay `cube` D2 10^5 | 56.4 ms (55.2 ms–65.9 ms) | 54.0 ms (52.2 ms–60.1 ms) | 0.96, inside the spread |
| Delaunay `cube` D2 10^6 | 590 ms (587 ms–600 ms) | 559 ms (552 ms–577 ms) | 0.95, faster beyond the spread |
| Delaunay `sphere` D2 10^4 | 5.60 ms (5.39 ms–16.2 ms) | 5.49 ms (5.27 ms–6.23 ms) | 0.98, inside the spread |
| Delaunay `sphere` D2 10^5 | 45.0 ms (43.8 ms–49.8 ms) | 44.2 ms (43.5 ms–50.4 ms) | 0.98, inside the spread |
| Delaunay `sphere` D2 10^6 | 446 ms (446 ms–451 ms) | 427 ms (420 ms–437 ms) | 0.96, faster beyond the spread |
| Delaunay `cube` D3 10^4 | 36.8 ms (35.4 ms–40.5 ms) | 36.7 ms (35.2 ms–48.8 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 389 ms (382 ms–429 ms) | 391 ms (383 ms–409 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^4 | 80.3 ms (78.2 ms–94.3 ms) | 80.2 ms (77.8 ms–92.3 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 763 ms (751 ms–782 ms) | 763 ms (753 ms–777 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 660 ms (651 ms–694 ms) | 660 ms (650 ms–733 ms) | 1.00, inside the spread |
| Delaunay `cube` D5 10^4 | 7.23 s (7.16 s–7.50 s) | 7.30 s (7.21 s–7.43 s) | 1.01, inside the spread |

The second run, 20 × 3: `cube` D2 10^5 0.95, `sphere` D2 10^5 0.98, `cube` D2 10^4 0.98, all inside the spread.

### Reading

- **Delaunay D2 reads 0.95 to 0.98.**
  - `cube` and `sphere` D2 10^6 read 0.95 and 0.96, faster beyond the spread.
  - `cube` and `sphere` D2 10^5 read 0.96 and 0.98 in the first run and 0.95 and 0.98 in the second, inside the spread.
- **The keep criterion asked for D2 10^5 faster beyond the spread, and it is not.** Two runs agree on the direction, and the sets of 10^6 are faster beyond the spread. Whether to keep the change on that is the owner's call (`bench.mdc`).
- **Nothing else moved:** D3 to D5 and the hull read 0.99 to 1.03, inside the spread.
- The tests were about 11% of `cube` D2 10^5 in the scratch marks of #459, which include the reads around them. Without those marks, the saving reads 2 to 5%.

## Hull D2: the spike P7-57 and the chain's keys sorted by radix, prototype, PR #478 (#470, #477)

### The spike P7-57 (#470)

Hull `sphere` D2 10^4 and 10^5 read 1.37 and 1.12 against CGAL as input indices (#466). callgrind, `e42ca61` under WSL2, gave the own instructions of one build of `sphere` D2 10^5. One build is the difference between runs of 3 builds and 1, and `planes()`, which the harness times after `build()`, is left out.

| Part | Instructions | Share of `build()` |
| --- | ---: | ---: |
| sorting the chain's keys, `(u64, u64, u32)` by comparison (`strict_cycle`) | about 37 M | about 23% |
| `SimplicialHull::build`, the left turns, and `strict_cycle`'s own lines | about 52 M | about 32% |
| publication (`ConvexHullBuilder::build`, inlined) | about 16 M | about 10% |
| `classify_built` | about 16 M | about 10% |
| `accept` | about 10 M | about 6% |

Every point of `sphere` D2 is extreme and goes through the sort. The Grill on #470 named the sort as the row P7-59 (#477), prototyped first.

### P7-59's prototype

A copy of P7-58 (`3ad04bf`), not committed (its diff is on #477):

- The keys were sorted by the x key in a least-significant-digit radix sort over its bytes, skipping a byte every key shares.
- Each run of equal x keys was then sorted by `(y, index)`, which gives the same order as the comparison sort.
- A test against `sort_unstable` passed, and failed when the runs were left unsorted. Every test passed.

Timed against P7-58, pinned and alternated: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second. Every set published the same counts on both sides.

| Set | P7-58 | Radix | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.53 ms (0.48 ms–3.77 ms) | 0.53 ms (0.47 ms–0.69 ms) | 1.00, inside the spread |
| Hull `cube` D2 10^5 | 4.82 ms (4.45 ms–5.28 ms) | 4.86 ms (4.36 ms–9.05 ms) | 1.01, inside the spread |
| Hull `cube` D2 10^6 | 52.1 ms (51.5 ms–56.0 ms) | 53.4 ms (51.2 ms–59.6 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^4 | 1.17 ms (1.09 ms–1.80 ms) | 1.21 ms (1.13 ms–1.64 ms) | 1.03, inside the spread |
| Hull `sphere` D2 10^5 | 12.0 ms (10.8 ms–13.0 ms) | 12.4 ms (11.3 ms–13.5 ms) | 1.03, inside the spread |
| Hull `sphere` D2 10^6 | 127 ms (125 ms–133 ms) | 130 ms (127 ms–138 ms) | 1.02, inside the spread |
| Hull `cube` D3 10^4 | 1.49 ms (1.25 ms–2.08 ms) | 1.46 ms (1.26 ms–3.73 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 11.7 ms (10.9 ms–16.2 ms) | 11.6 ms (11.0 ms–23.4 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 46.8 ms (45.4 ms–65.9 ms) | 46.8 ms (45.8 ms–50.0 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 20.6 ms (20.2 ms–22.4 ms) | 20.9 ms (20.1 ms–22.3 ms) | 1.02, inside the spread |
| Hull `sphere` D3 10^5 | 246 ms (239 ms–258 ms) | 248 ms (241 ms–260 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^4 | 5.33 ms (5.20 ms–6.27 ms) | 5.39 ms (5.21 ms–6.26 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 32.2 ms (31.4 ms–35.3 ms) | 32.9 ms (31.9 ms–34.4 ms) | 1.02, inside the spread |
| Hull `sphere` D4 10^4 | 89.6 ms (87.1 ms–98.2 ms) | 91.0 ms (88.6 ms–98.4 ms) | 1.02, inside the spread |
| Hull `sphere` D4 10^5 | 1.15 s (1.13 s–1.23 s) | 1.16 s (1.13 s–1.18 s) | 1.01, inside the spread |
| Hull `cube` D5 10^4 | 53.7 ms (52.6 ms–56.0 ms) | 53.8 ms (52.7 ms–58.1 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 804 ms (788 ms–852 ms) | 807 ms (795 ms–836 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 567 ms (551 ms–613 ms) | 575 ms (559 ms–600 ms) | 1.01, inside the spread |

### Reading

- **The radix sort is not faster.** Hull `sphere` D2 reads 1.02 to 1.03, `cube` D2 1.00 to 1.02, inside the spread. Delaunay, which does not sort these keys, reads 0.98 to 1.01.
- **Instructions are not time here.** Each radix pass moves every 20-byte key, and the comparison sort's instructions run at a high rate. The 23% share of instructions did not carry over to time, as with the merge of P7-34 (#471) and the estimate of P7-58 (#475).
- **P7-59 is withdrawn after measurement.** A parity run follows, then the Grill on Delaunay D3's pass after the insertion.

## The P7 parity run after P7-55 and P7-58, PR #482 (#479)

Due after P7-55 (#472) and P7-58 (#475), under the same-output references of ADR 0006 (amendments of 2026-10-11).

### Method

As in the run of #466: `main` at `110a208`, `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. "(ref.)" marks a reference that is not judged (ADR 0006). "/ CGAL" is against the reference with the same output; "/ CGAL from the points" is against the construction from the points, given where the reference changed.

The runs of #466 and this one were on the same day. On the sets no row of this period touched (hull `cube` and `sphere` D5 and D6), convx's `build()` read 0.99 of the run of #466 (median; 0.97 to 1.00), CGAL 1.00 (0.97 to 1.01), and Qhull 0.99 (0.99 to 1.01).

### P7 judgement

Met at 1.05: 30 of 41 (hull 20, Delaunay 10); 28 in the run of #466. Met at 1.00: 24 of 41 (hull 17, Delaunay 7); 25 in the run of #466. Against the references from the points instead, 23 and 20 would be met.

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.48 ms | 0.83 | 0.96 | 0.86 | met | met |
| Hull `cube` D2 10^5 | 4.26 ms | 0.79 | 1.00 | 0.78 | met | met |
| Hull `cube` D2 10^6 | 55.3 ms | 0.82 | 1.26 | 1.00 | met | not met |
| Hull `cube` D3 10^4 | 1.19 ms | 0.56 | – | 0.80 | met | met |
| Hull `cube` D3 10^5 | 9.60 ms | 0.38 | – | 0.84 | met | met |
| Hull `cube` D3 10^6 | 134 ms | 0.25 | – | 0.95 | met | met |
| Hull `cube` D4 10^4 | 5.19 ms | 0.12 | – | 0.79 | met | met |
| Hull `cube` D4 10^5 | 30.8 ms | 0.06 | – | 0.92 | met | met |
| Hull `cube` D5 10^4 | 52.0 ms | 0.24 | – | 0.73 | met | met |
| Hull `cube` D5 10^5 | 205 ms | 0.13 | – | 0.67 | met | met |
| Hull `cube` D6 10^4 | 567 ms | 0.28 | – | 0.47 | met | met |
| Hull `cube` D6 10^5 | 2.49 s | 0.23 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.95 ms | 1.26 | 1.44 | 0.14 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 10.6 ms | 1.13 | 1.43 | 0.10 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 122 ms | 0.83 | 1.43 | 0.07 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 18.8 ms | 0.85 | – | 1.06 | not met | not met |
| Hull `sphere` D3 10^5 | 243 ms | 0.66 | – | 1.04 | met | not met |
| Hull `sphere` D3 10^6 | 3.94 s | 0.52 | – | 1.22 | not met | not met |
| Hull `sphere` D4 10^4 | 88.2 ms | 0.75 | – | 0.97 | met | met |
| Hull `sphere` D4 10^5 | 1.16 s | 0.96 | – | 0.92 | met | met |
| Hull `sphere` D5 10^4 | 779 ms | 0.83 | – | 0.97 | met | met |
| Hull `sphere` D5 10^5 | 10.24 s | 0.97 | – | 1.01 | met | not met |
| Hull `sphere` D6 10^4 | 6.47 s | 0.71 | – | 0.83 | met | met |
| Hull `cubesurf` D3 10^5 | 43.4 ms | 1.19 | – | 1.55 | not met | not met |
| Hull `grid` D6 10^4 | 234 ms | 0.97 (ref.) | – | 0.70 | met | met |
| Delaunay `cube` D2 10^4 | 5.09 ms | 1.13 | 1.32 | 0.28 | not met | not met |
| Delaunay `cube` D2 10^5 | 52.5 ms | 1.03 | 1.29 | 0.24 | met | not met |
| Delaunay `cube` D2 10^6 | 540 ms | 0.83 | 1.27 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 4.84 ms | 1.12 | 1.29 | 0.21 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 42.3 ms | 1.06 | 1.36 | 0.12 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^6 | 396 ms | 0.82 | 1.38 | 0.04 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 34.6 ms | 1.12 | 1.18 | 0.40 | not met | not met |
| Delaunay `cube` D3 10^5 | 375 ms | 1.13 | 1.24 | 0.31 | not met | not met |
| Delaunay `cube` D3 10^6 | 4.14 s | 1.11 | 1.29 | 0.28 | not met | not met |
| Delaunay `sphere` D3 10^4 | 65.7 ms | 0.51 | 0.51 | 0.60 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 636 ms | 0.68 | 0.70 | 0.35 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.47 s | 0.82 | 0.85 | 0.34 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 661 ms | 1.03 | 1.09 | 0.87 | met | not met |
| Delaunay `sphere` D4 10^4 | 852 ms | 0.34 | 0.35 | 0.70 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 6.97 s | 1.02 | 1.05 | 0.90 | met | not met |
| Delaunay `sphere` D5 10^4 | 10.56 s | 0.31 | 0.31 | 0.75 (ref.) | met | met |

### What moved

- **Hull `cubesurf` D3 10^5** reads 1.55 against Qhull and 1.19 against CGAL, from 1.94 and 1.48: `build()` took 0.79 of its time, from P7-55.
- **Delaunay D2:** `build()` took 0.93 to 0.98 of the run of #466, from P7-58.
  - `cube` D2 10^5 reads 1.03, now met;
  - `sphere` D2 10^5 reads 1.06, and the 10^4 sets 1.13 and 1.12.
- **Delaunay `cube` D4 10^4** reads 1.03, now met, inside the run-to-run difference of a set no row touched.
- **Met at 1.00 fell by one:** hull `cube` D2 10^6 reads 1.00 against Qhull (0.95 before), and Delaunay `cube` D5 10^4 1.02. Neither path changed beyond a few percent.

### What is not met

11 sets:

| Group | Sets | Worst ratio |
| --- | ---: | --- |
| Delaunay `cube` D3 | 3 | 1.11 to 1.13 against CGAL |
| Delaunay D2 10^4, and `sphere` D2 10^5 | 3 | 1.06 to 1.13 against CGAL |
| Hull `sphere` D2 10^4 and 10^5 | 2 | 1.26 and 1.13 against CGAL |
| Hull `sphere` D3 10^4 and 10^6 | 2 | 1.06 and 1.22 against Qhull |
| Hull `cubesurf` D3 10^5 | 1 | 1.55 against Qhull |

## Hull `sphere` D3 at 10^6, and the merge at less cost, PR #486 (#484, #485)

### The spike P7-61 (#484)

Hull `sphere` D3 reads 1.06, 1.04, and 1.22 against Qhull at 10^4, 10^5, and 10^6 (#479). VTune, `main` at `110a208` on Windows, pinned, software sampling, 40 builds of 10^5 and 3 of 10^6, gave these inclusive shares of `ConvexHullBuilder::build`:

| Part | 10^5 | 10^6 |
| --- | ---: | ---: |
| construction (`SimplicialHull::build`) | 79.4% | 78.1% |
| of which outside sets (`take_outside`, the cull's SIMD scan) | 17.5% | 20.1% |
| of which planes of new simplices | 11.9% | 10.1% |
| of which `walk_region` (side tests of the visible region) | 12.6% | 16.7% |
| of which `commit` | 11.2% | 9.8% |
| classification (`classify_built`) | 17.9% | 20.2% |
| of which the coplanar merge (`merge_except`) | 11.6% | 15.3% |

Within the merge at 10^6:

- the ridge tests (`side`, reading each simplex's cull plane): 31.5% of it;
- building each group's neighbor list (iterators, `SlotMarks::get`): 28.5%;
- `dense.get` in the ridge loop: about 11%.

The parts that grow with the input are those that read memory at random. The list building that P7-34's prototype removed read 1 to 2% at 10^5 (#471); by time it is about 4.4% of `build()` at 10^6. The Grill on #484 brought it back as P7-62, with the ridge loop's lookup by slot, and set a spike on construction's random reads after it.

### The change (P7-62, #485)

- When no ridge joins two simplices, each group of one simplex has its vertices and neighbors written straight into presized lists, without the per-group gathering, sort, and dedup. Debug builds compare the result with the general path on every merge.
- The ridge loop looks up each neighbor's dense index in an array by slot, in place of `SlotMarks`. The neighbors of a live simplex are live, and debug builds check it.
- Removing a sort in the fast path makes the debug check fire in three tests.

### Method

| Item | Value |
| --- | --- |
| convx | `main` at `110a208` against the head `ff29ae3`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Sets | The shorter run of `docs/verification.md`, with hull `sphere` D3 10^6, `sphere` D4 10^4, and `cube` D5 10^5 |
| Rounds | `main` and the head alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |

Every set published the same counts on both sides.

### Against `main`

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.56 ms (0.47 ms–0.73 ms) | 0.56 ms (0.47 ms–1.28 ms) | 1.00, inside the spread |
| Hull `cube` D2 10^5 | 5.14 ms (4.49 ms–6.19 ms) | 5.17 ms (4.63 ms–7.46 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^4 | 1.23 ms (1.13 ms–2.07 ms) | 1.20 ms (1.12 ms–1.68 ms) | 0.98, inside the spread |
| Hull `sphere` D2 10^5 | 12.8 ms (11.1 ms–15.4 ms) | 12.8 ms (11.6 ms–26.2 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^4 | 1.52 ms (1.28 ms–3.51 ms) | 1.50 ms (1.33 ms–1.89 ms) | 0.99, inside the spread |
| Hull `cube` D3 10^5 | 13.1 ms (11.7 ms–17.3 ms) | 12.8 ms (11.7 ms–25.1 ms) | 0.98, inside the spread |
| Hull `cubesurf` D3 10^5 | 48.2 ms (47.0 ms–53.4 ms) | 48.2 ms (46.6 ms–50.3 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 21.5 ms (20.9 ms–25.5 ms) | 20.8 ms (20.1 ms–23.8 ms) | 0.97, inside the spread |
| Hull `sphere` D3 10^5 | 260 ms (255 ms–285 ms) | 255 ms (245 ms–277 ms) | 0.98, inside the spread |
| Hull `sphere` D3 10^6 | 4.39 s (4.34 s–4.43 s) | 4.12 s (4.09 s–4.15 s) | 0.94, faster beyond the spread |
| Hull `cube` D4 10^4 | 5.68 ms (5.25 ms–6.94 ms) | 5.61 ms (5.20 ms–6.10 ms) | 0.99, inside the spread |
| Hull `cube` D4 10^5 | 33.6 ms (31.7 ms–36.7 ms) | 33.7 ms (32.1 ms–37.2 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 93.3 ms (90.1 ms–106 ms) | 90.6 ms (88.0 ms–95.6 ms) | 0.97, inside the spread |
| Hull `sphere` D4 10^5 | 1.20 s (1.19 s–1.21 s) | 1.18 s (1.15 s–1.19 s) | 0.98, faster beyond the spread |
| Hull `cube` D5 10^4 | 55.5 ms (53.4 ms–60.2 ms) | 54.6 ms (53.5 ms–62.5 ms) | 0.98, inside the spread |
| Hull `cube` D5 10^5 | 217 ms (210 ms–230 ms) | 216 ms (211 ms–229 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 824 ms (810 ms–861 ms) | 810 ms (796 ms–836 ms) | 0.98, inside the spread |
| Hull `cube` D6 10^4 | 573 ms (562 ms–587 ms) | 569 ms (557 ms–601 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 5.47 ms (5.26 ms–6.54 ms) | 5.42 ms (5.23 ms–6.28 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^5 | 54.0 ms (52.9 ms–60.6 ms) | 54.4 ms (53.0 ms–62.0 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^6 | 576 ms (566 ms–583 ms) | 575 ms (572 ms–591 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.60 ms (5.43 ms–6.87 ms) | 5.58 ms (5.29 ms–6.54 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 44.8 ms (43.3 ms–47.4 ms) | 44.8 ms (43.8 ms–49.7 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^6 | 437 ms (434 ms–456 ms) | 433 ms (429 ms–450 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^4 | 37.2 ms (35.7 ms–44.5 ms) | 37.5 ms (35.9 ms–39.6 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^5 | 390 ms (383 ms–399 ms) | 392 ms (384 ms–407 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 78.9 ms (77.2 ms–82.0 ms) | 78.9 ms (77.2 ms–90.0 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 767 ms (765 ms–776 ms) | 772 ms (764 ms–803 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 651 ms (635 ms–709 ms) | 641 ms (631 ms–733 ms) | 0.99, inside the spread |
| Delaunay `cube` D5 10^4 | 7.55 s (7.47 s–7.61 s) | 7.55 s (7.49 s–7.57 s) | 1.00, inside the spread |

### Reading

- **The keep criterion is met.** Hull `sphere` D3 10^6 reads 0.94, faster beyond the spread: 4.39 s to 4.12 s.
  - Hull `sphere` D4 10^5 reads 0.98, also faster beyond the spread.
  - The other hull sets read 0.97 to 1.01.
- On the parity ratio of #479 that is about 1.15 against Qhull for `sphere` D3 10^6, from 1.22.
- **Nothing is slower beyond the spread.** Delaunay, which does not merge, reads 0.99 to 1.01.
- What remains of the gap at 10^6 is construction's random reads: the walk's side tests and the outside sets' scan. A spike on its cache misses follows (Grill on #484).

## Hull D3 at 10^6: cache misses, and construction in space order, PR #490 (#487, #489)

### The spike P7-63 (#487)

Hull `sphere` D3 read 1.22 against Qhull at 10^6 (#479), about 1.15 after P7-62. The parts that grow with the input read memory at random (#484).

**cachegrind, whole programs** (`110a208`, under WSL2, the LL cache as the host's 20 MiB L3; convx's harness also runs `planes()` after `build()`):

| Run | Instructions | LL read misses | LL write misses |
| --- | ---: | ---: | ---: |
| convx `sphere` D3 10^5 | 3.22 G | 1.48 M | 1.73 M |
| Qhull `sphere` D3 10^5 | 3.11 G | 4.01 M | 1.08 M |
| convx `sphere` D3 10^6 | 34.3 G | 112.6 M | 32.3 M |
| Qhull `sphere` D3 10^6 | 33.0 G | 75.9 M | 13.6 M |

**callgrind with cache simulation, inside `ConvexHullBuilder::build` only, 10^6:** 96.5 M LL read misses and 30.0 M write misses.

| Source | LL read misses | LL write misses |
| --- | ---: | ---: |
| the cull's SIMD scan of the outside sets (point coordinates read by index) | 14.5 M | – |
| the side tests (each simplex's cull plane) | 16.6 M | – |
| the facet store | 15.8 M | 2.3 M |
| `memcpy`, many small copies | 7.4 M | 11.5 M |
| the merge (before P7-62) | about 9.8 M | – |

**Two prototypes**, against P7-62 (`ff29ae3`), pinned and alternated:

- **The outside sets' compaction into a reused buffer** read 0.96 to 1.04 on hull D3 and D4, inside the spread.
- **The points given in Morton order** (the harness sorted them before any timing; the copy's cost not counted) read:
  - hull `sphere` and `cube` D3 10^6: 0.82;
  - `cubesurf` D3 10^5: 0.84;
  - the sets of 10^5: 0.95 to 1.02;
  - D4: 0.98 to 1.01.

The Grill on #487 decided construction on a copy in space order, with an ADR (P7-64).

### The change (P7-64, #489)

ADR 0009. For D = 3 and from 2^17 representatives:

- construction runs on a copy of the representatives in Morton order, with consecutive labels;
- the built hull is renamed back to the input's indices before classification (the simplices' vertices, the outside sets and their candidates, the points proved interior, and the records);
- the order comes from a counting sort over a grid of about four points per cell; the points of a cell keep the order of their indices.

`spread2` and `spread3` moved to `src/morton.rs`, shared with Delaunay's BRIO. `hpc.mdc` and both design files (§6) say the same.

**On the way:**

- A first version sorted a 128-bit key per point and started at 2^16. It read 0.85 on `sphere` D3 10^6, but 1.12 on `cube` D3 10^6 (slower beyond the spread) and 1.24 on `cube` D3 10^5: the sort cost more than it saved where most points are discarded early.
- The counting sort read 0.97 on `cube` D3 10^6 and 1.08 on `cube` D3 10^5, so the threshold rose to 2^17.

**Checks:**

- A test publishes the same hull through the public API with and without the copy, on random points in a cube and near a sphere, integer grids with duplicates, points on the faces of a cube, and clusters of near-duplicates. A test-only switch lowers the threshold. It takes about 19 s in debug.
- Leaving the points proved interior in the copy's labels makes it fail.

### Method

| Item | Value |
| --- | --- |
| convx | P7-62 at `ff29ae3` against the head `9460825`. rustc 1.97.1, `--release` with debug info, baseline target |
| Machine | Intel Core i5-13400F, Windows 11, every process pinned (logical processor 2), nothing else running |
| Sets | The shorter run of `docs/verification.md`, with hull `sphere` and `cube` D3 10^6 |
| Rounds | Alternated per round: 10 rounds × 3 builds; 5 × 1 for the sets over about half a second |

Every set published the same counts on both sides.

### Against P7-62

| Set | P7-62 | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.55 ms (0.47 ms–0.80 ms) | 0.53 ms (0.47 ms–1.13 ms) | 0.97, inside the spread |
| Hull `cube` D2 10^5 | 5.17 ms (4.41 ms–8.36 ms) | 5.12 ms (4.54 ms–8.98 ms) | 0.99, inside the spread |
| Hull `sphere` D2 10^4 | 1.23 ms (1.14 ms–2.15 ms) | 1.25 ms (1.11 ms–2.11 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^5 | 11.6 ms (10.6 ms–36.7 ms) | 12.1 ms (11.0 ms–21.9 ms) | 1.04, inside the spread |
| Hull `cube` D3 10^4 | 1.39 ms (1.24 ms–1.63 ms) | 1.37 ms (1.23 ms–1.58 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^5 | 11.3 ms (10.8 ms–13.2 ms) | 11.1 ms (10.5 ms–12.0 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^6 | 133 ms (131 ms–137 ms) | 136 ms (131 ms–142 ms) | 1.02, inside the spread |
| Hull `cubesurf` D3 10^5 | 48.6 ms (46.8 ms–50.0 ms) | 48.9 ms (47.3 ms–56.6 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^4 | 20.3 ms (19.8 ms–22.4 ms) | 20.4 ms (19.8 ms–23.6 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^5 | 244 ms (239 ms–265 ms) | 243 ms (238 ms–269 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^6 | 3.73 s (3.70 s–3.74 s) | 3.27 s (3.23 s–3.27 s) | 0.88, faster beyond the spread |
| Hull `cube` D4 10^4 | 5.47 ms (5.15 ms–6.06 ms) | 5.53 ms (5.21 ms–6.77 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 34.1 ms (32.5 ms–47.8 ms) | 33.3 ms (32.3 ms–37.4 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^4 | 90.5 ms (89.3 ms–97.8 ms) | 91.5 ms (88.9 ms–97.6 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.11 s (1.08 s–1.17 s) | 1.09 s (1.08 s–1.10 s) | 0.98, inside the spread |
| Hull `cube` D5 10^4 | 54.3 ms (52.8 ms–57.1 ms) | 54.6 ms (53.1 ms–59.3 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 796 ms (779 ms–828 ms) | 790 ms (777 ms–840 ms) | 0.99, inside the spread |
| Hull `cube` D6 10^4 | 563 ms (555 ms–598 ms) | 566 ms (555 ms–610 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 5.42 ms (5.26 ms–6.27 ms) | 5.47 ms (5.17 ms–7.27 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^5 | 54.0 ms (52.3 ms–63.5 ms) | 54.1 ms (52.8 ms–58.4 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 560 ms (556 ms–573 ms) | 559 ms (556 ms–561 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.53 ms (5.34 ms–6.66 ms) | 5.55 ms (5.38 ms–17.2 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 43.7 ms (42.9 ms–47.0 ms) | 44.0 ms (42.8 ms–50.2 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^6 | 423 ms (420 ms–427 ms) | 423 ms (417 ms–426 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^4 | 37.3 ms (36.0 ms–40.0 ms) | 36.9 ms (35.6 ms–39.2 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^5 | 390 ms (384 ms–402 ms) | 389 ms (383 ms–414 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 79.8 ms (78.5 ms–88.1 ms) | 80.4 ms (78.4 ms–105 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^5 | 757 ms (755 ms–776 ms) | 761 ms (754 ms–768 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 650 ms (641 ms–681 ms) | 650 ms (640 ms–669 ms) | 1.00, inside the spread |
| Delaunay `cube` D5 10^4 | 7.27 s (7.07 s–7.33 s) | 7.15 s (7.09 s–7.32 s) | 0.98, inside the spread |

### Reading

- **Hull `sphere` D3 10^6 reads 0.88, faster beyond the spread:** 3.73 s to 3.27 s. On the parity ratio, that is about 1.01 against Qhull, from about 1.15 after P7-62.
- **Hull `cube` D3 10^6 reads 1.02, inside the spread.** The keep criterion asked it to be faster beyond the spread too, and it is not: what the order saves there, the copy and its counting sort spend. Whether to keep the change on `sphere` alone is the owner's call (`bench.mdc`).
- **Nothing is slower beyond the spread.** The sets under 2^17 representatives do not take the copy, and read 0.97 to 1.04.

## Hull: the spike P7-65, and the exact side tests through the fixed-size first stage, prototype, PR #495 (#493, #494)

### The spike P7-65 (#493)

The head of P7-64 (`2431f39` code), on Windows, pinned, VTune with software sampling: 60 builds of hull `cubesurf` D3 10^5 and 3,000 of `sphere` D2 10^4. Inclusive shares of `ConvexHullBuilder::build`.

**`cubesurf` D3 10^5** (1.55 against Qhull):

| Part | Share |
| --- | ---: |
| construction | 53.0% |
| of which `take_outside` | 41.1% |
| of which the cull's SIMD scan | 15.9% |
| of which exact side tests (`side` → `Input::orient` → the generic `semi_static::sign`) | 15.4% |
| of which the first stage's underflow path (`with_underflow_term`, `#[cold]`) | 7.3% |
| of which sorting `u32` lists | 6.4% |
| classification (`found_vertices`) | 41.6% |
| of which the cull's SIMD scan (the walk of P7-55) | 17.7% |
| acceptance | 4.0% |

The points lie exactly on the cube's faces. Their side tests against simplices of those faces are zero, so the cull cannot prove them, and the first stage's bound takes the path for tiny permanents.

**`sphere` D2 10^4** (1.26 against CGAL as input indices):

| Part | Share |
| --- | ---: |
| construction (`build_polygon`) | 68.9% |
| of which sorting the chain's keys | 22.3% |
| of which the left turns (`pop_until_left`) | 22.9% |
| of which `discard_polygon` | 15.4% |
| acceptance (`representatives_by_hash`) | 14.0% |
| publication | 12.0% |
| classification (`classify_chain`) | 5.1% |

The Grill on #493 chose P7-66 (below), then P7-67: hull D2's duplicates found from the chain's sort in place of hashing.

### The prototype P7-66 (#494)

`Input::orient` sent the side tests of D = 2 (three points) and D = 3 (four points) to `first_stage_fixed`, the first stage on arrays that P7-45 gave the Delaunay shapes (#449). When it left the sign open, the later stages ran from `Start::AfterFirstStage`. Debug builds compared each sign with the generic one, and every test passed; rows given in the wrong order failed three tests. The diff is on #494 (`06931cd`).

**Against P7-64** (`9460825` code), pinned and alternated, the shorter run of `docs/verification.md` with hull `sphere` and `cube` D3 10^6: 10 rounds × 3 builds, 5 × 1 for the sets over about half a second. Every set published the same counts on both sides.

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.56 ms (0.49 ms–0.90 ms) | 0.54 ms (0.48 ms–0.96 ms) | 0.96, inside the spread |
| Hull `cube` D2 10^5 | 5.06 ms (4.53 ms–6.52 ms) | 5.09 ms (4.51 ms–6.27 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^4 | 1.23 ms (1.11 ms–1.97 ms) | 1.26 ms (1.10 ms–2.14 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^5 | 12.1 ms (11.0 ms–13.0 ms) | 11.9 ms (10.7 ms–13.2 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^4 | 1.47 ms (1.25 ms–1.77 ms) | 1.49 ms (1.33 ms–2.31 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^5 | 11.9 ms (10.7 ms–15.0 ms) | 11.8 ms (10.8 ms–21.5 ms) | 0.98, inside the spread |
| Hull `cube` D3 10^6 | 143 ms (137 ms–148 ms) | 138 ms (137 ms–140 ms) | 0.97, inside the spread |
| Hull `cubesurf` D3 10^5 | 48.3 ms (47.0 ms–50.5 ms) | 46.9 ms (45.6 ms–52.9 ms) | 0.97, inside the spread |
| Hull `sphere` D3 10^4 | 20.3 ms (20.0 ms–23.5 ms) | 20.1 ms (19.7 ms–23.5 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 242 ms (237 ms–257 ms) | 243 ms (235 ms–258 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^6 | 3.26 s (3.24 s–3.30 s) | 3.27 s (3.22 s–3.33 s) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 5.55 ms (5.22 ms–6.72 ms) | 5.53 ms (5.11 ms–6.59 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 33.1 ms (32.2 ms–36.6 ms) | 33.1 ms (32.2 ms–36.1 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 90.3 ms (87.4 ms–100 ms) | 88.5 ms (87.0 ms–92.4 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^5 | 1.09 s (1.08 s–1.11 s) | 1.09 s (1.08 s–1.11 s) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 54.5 ms (53.3 ms–58.1 ms) | 53.4 ms (52.3 ms–56.9 ms) | 0.98, inside the spread |
| Hull `sphere` D5 10^4 | 790 ms (780 ms–834 ms) | 774 ms (756 ms–796 ms) | 0.98, inside the spread |
| Hull `cube` D6 10^4 | 557 ms (545 ms–572 ms) | 550 ms (536 ms–567 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 5.37 ms (5.20 ms–6.53 ms) | 5.34 ms (5.21 ms–6.02 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^5 | 53.0 ms (52.0 ms–57.1 ms) | 53.1 ms (51.8 ms–56.6 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 564 ms (558 ms–584 ms) | 562 ms (559 ms–574 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.49 ms (5.26 ms–6.39 ms) | 5.44 ms (5.29 ms–5.73 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^5 | 43.7 ms (42.9 ms–47.7 ms) | 43.4 ms (42.6 ms–45.9 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^6 | 420 ms (419 ms–426 ms) | 426 ms (421 ms–433 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^4 | 36.1 ms (35.2 ms–38.8 ms) | 36.6 ms (35.4 ms–38.7 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^5 | 379 ms (372 ms–404 ms) | 380 ms (375 ms–397 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 78.7 ms (76.6 ms–80.4 ms) | 78.8 ms (77.0 ms–83.3 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 761 ms (753 ms–782 ms) | 764 ms (754 ms–770 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 659 ms (643 ms–730 ms) | 656 ms (634 ms–770 ms) | 1.00, inside the spread |
| Delaunay `cube` D5 10^4 | 7.32 s (7.27 s–7.78 s) | 7.42 s (7.35 s–7.93 s) | 1.01, inside the spread |

**Again, on the sets it reaches:** 30 rounds × 3 builds, 15 × 1 for 10^6.

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D3 10^5 | 12.1 ms (10.6 ms–14.0 ms) | 12.1 ms (10.6 ms–14.2 ms) | 1.00, inside the spread |
| Hull `cube` D3 10^6 | 151 ms (145 ms–156 ms) | 151 ms (146 ms–161 ms) | 1.00, inside the spread |
| Hull `cubesurf` D3 10^5 | 53.6 ms (49.5 ms–66.2 ms) | 51.6 ms (46.6 ms–69.8 ms) | 0.96, inside the spread |
| Hull `sphere` D3 10^4 | 22.6 ms (20.4 ms–31.4 ms) | 21.7 ms (20.3 ms–24.7 ms) | 0.96, inside the spread |

**A second prototype** removed `#[cold]` and `#[inline(never)]` from `with_underflow_term`, on top of P7-66 and against it, 20 rounds × 3 builds:

- hull `cubesurf` D3 10^5 read 0.97, inside the spread;
- Delaunay `cube` D3 10^5 read 1.54, `sphere` D3 10^4 1.18, and `cube` D2 10^5 1.60: the bound's common path got slower where the underflow path is rare, as in #318.

### Reading

- **P7-66 is withdrawn after measurement.** Hull `cubesurf` D3 10^5 read 0.97 and 0.96, inside the spread both times (`bench.mdc`: a gain inside the spread is reverted). The exact side tests were 15.4% of the build, and about half of that was the underflow path, which the fixed first stage takes too.
- **`#[cold]` on the underflow path stays.**
- P7-67 follows, then a parity run.

## Hull D2: the duplicates found from the chain's sort, where the chain reads every point, PR #498 (#496)

The spike P7-65 (#493) put hull `sphere` D2 10^4 at 1.26 against CGAL as input indices, with acceptance's hashing at 14.0% of the build and the chain's sort at 22.3%. The chain sorts every representative by (x, y, index), which puts equal points next to each other.

### The first prototype

`fbe26dc` sorted every point of D = 2 in acceptance, found the duplicates as equal neighbors, and handed the sorted representatives to the chain. Against P7-64 (`9460825` code), pinned and alternated, 20 rounds × 3 builds:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.54 ms (0.47 ms–1.11 ms) | 0.72 ms (0.61 ms–1.71 ms) | 1.34, inside the spread |
| Hull `cube` D2 10^5 | 5.11 ms (4.46 ms–6.22 ms) | 7.58 ms (6.59 ms–10.7 ms) | 1.48, slower beyond the spread |
| Hull `sphere` D2 10^4 | 1.24 ms (1.11 ms–4.89 ms) | 1.18 ms (1.03 ms–1.76 ms) | 0.95, inside the spread |
| Hull `sphere` D2 10^5 | 12.8 ms (10.9 ms–20.6 ms) | 11.4 ms (10.3 ms–19.1 ms) | 0.89, inside the spread |
| Hull `cube` D3 10^4 | 1.46 ms (1.21 ms–1.89 ms) | 1.41 ms (1.22 ms–2.02 ms) | 0.97, inside the spread |

Again on hull `sphere` D2, 40 rounds × 3 builds:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `sphere` D2 10^4 | 1.22 ms (1.09 ms–2.99 ms) | 1.14 ms (1.00 ms–2.17 ms) | 0.93, inside the spread |
| Hull `sphere` D2 10^5 | 12.4 ms (11.0 ms–39.0 ms) | 11.0 ms (9.79 ms–15.5 ms) | 0.89, inside the spread |

Hull `cube` D2 got slower beyond the spread. There the discard polygon removes most points, so the chain sorts few, and a sort of every point costs more than the hashing. The Grill on #496 chose to decide the polygon first.

### The change (P7-67)

- **The plan on the input points.** For a hull of D = 2, acceptance computes the discard polygon and whether discarding pays on the input points, before duplicates are removed. This is the same polygon as before: on a tie the earlier point is kept, and the first of equal points is their representative. Duplicates can enter the sample, which changes only whether discarding goes ahead.
- **Where the chain reads every point**, one sort by x, then y, then index finds the duplicates, and the chain reads the representatives in that order without sorting again. The keys add `+0.0` to each coordinate, so `-0.0` and `+0.0` share one key (design §3).
- **Where discarding pays**, duplicates are found by hashing, as before. The polygon's edges are passed to construction, so the extremes are not found twice.
- **Where the polygon gives up the exact evaluation**, acceptance leaves no plan. Construction plans again on the representatives, gives up the same way, and returns the same error after acceptance's own errors, as before.
- **New module.** The polygon, the sample, and the chain of strict left turns moved to `src/hull/polygon.rs`, as functions of the rows.
- **Delaunay.** Delaunay accepts its sites through `accept_sites`, which hashes in every dimension. A first version planned the chain for Delaunay D2 too, which never reads it, and Delaunay `sphere` D2 10^6 read 1.07.
- **Design.** §6 (both languages) says the same.

**Checks:**

- A test compares the representatives of the one sort with those of the sort by `compare_points`, on inputs with many duplicates and signed zeros. It also compares the sorted list with an independent sort. Comparing classes on x alone makes it fail.
- Every test passes in debug, in release, and under WSL.

### Against P7-64

`9460825` code against the head `d2ba949`, pinned and alternated, 20 rounds × 3 builds, and 8 × 1 for 10^6:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.52 ms (0.47 ms–1.19 ms) | 0.51 ms (0.46 ms–0.70 ms) | 0.99, inside the spread |
| Hull `cube` D2 10^5 | 4.69 ms (4.23 ms–5.83 ms) | 4.63 ms (4.41 ms–5.90 ms) | 0.99, inside the spread |
| Hull `sphere` D2 10^4 | 1.17 ms (1.08 ms–1.67 ms) | 1.06 ms (0.99 ms–5.15 ms) | 0.91, inside the spread |
| Hull `sphere` D2 10^5 | 11.3 ms (10.7 ms–15.3 ms) | 10.7 ms (9.65 ms–33.2 ms) | 0.94, inside the spread |
| Delaunay `cube` D2 10^4 | 5.45 ms (5.21 ms–6.18 ms) | 5.43 ms (5.22 ms–6.54 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 53.1 ms (51.7 ms–57.8 ms) | 52.9 ms (51.5 ms–95.1 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 615 ms (608 ms–623 ms) | 613 ms (609 ms–621 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.50 ms (5.27 ms–6.21 ms) | 5.50 ms (5.26 ms–6.13 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^5 | 44.0 ms (42.6 ms–54.2 ms) | 43.7 ms (42.6 ms–47.0 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^6 | 461 ms (454 ms–480 ms) | 462 ms (459 ms–477 ms) | 1.00, inside the spread |

The whole shorter run of `docs/verification.md`, timed on `db5c785` (before Delaunay's acceptance was split off), 10 rounds × 3 builds, 5 × 1 for the sets over about half a second:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.57 ms (0.50 ms–1.29 ms) | 0.56 ms (0.48 ms–6.34 ms) | 0.99, inside the spread |
| Hull `cube` D2 10^5 | 5.08 ms (4.52 ms–9.07 ms) | 5.18 ms (4.49 ms–8.03 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^4 | 1.27 ms (1.18 ms–2.54 ms) | 1.20 ms (1.02 ms–3.15 ms) | 0.94, inside the spread |
| Hull `sphere` D2 10^5 | 12.6 ms (11.0 ms–18.9 ms) | 11.0 ms (10.1 ms–13.4 ms) | 0.87, inside the spread |
| Hull `cube` D3 10^4 | 1.44 ms (1.25 ms–1.86 ms) | 1.48 ms (1.30 ms–3.35 ms) | 1.03, inside the spread |
| Hull `cube` D3 10^5 | 12.6 ms (11.0 ms–17.2 ms) | 12.2 ms (11.3 ms–17.1 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^6 | 153 ms (150 ms–159 ms) | 152 ms (150 ms–153 ms) | 0.99, inside the spread |
| Hull `cubesurf` D3 10^5 | 52.5 ms (50.3 ms–59.1 ms) | 53.2 ms (50.9 ms–65.0 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^4 | 22.7 ms (20.9 ms–78.0 ms) | 22.8 ms (20.6 ms–26.7 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^5 | 275 ms (261 ms–342 ms) | 277 ms (265 ms–303 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^6 | 3.66 s (3.42 s–3.82 s) | 3.64 s (3.50 s–3.83 s) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 5.77 ms (5.37 ms–6.67 ms) | 5.81 ms (5.31 ms–8.90 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 35.7 ms (34.1 ms–37.9 ms) | 36.6 ms (35.0 ms–39.6 ms) | 1.03, inside the spread |
| Hull `sphere` D4 10^4 | 96.9 ms (93.1 ms–104 ms) | 96.7 ms (93.3 ms–105 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.22 s (1.21 s–1.25 s) | 1.23 s (1.22 s–1.27 s) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 58.6 ms (56.7 ms–63.1 ms) | 59.7 ms (56.7 ms–64.3 ms) | 1.02, inside the spread |
| Hull `sphere` D5 10^4 | 865 ms (846 ms–893 ms) | 868 ms (840 ms–909 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 611 ms (596 ms–633 ms) | 608 ms (584 ms–642 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 5.58 ms (5.30 ms–7.09 ms) | 6.00 ms (5.44 ms–6.98 ms) | 1.08, inside the spread |
| Delaunay `cube` D2 10^5 | 57.3 ms (55.1 ms–87.4 ms) | 57.9 ms (55.1 ms–61.2 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^6 | 615 ms (605 ms–637 ms) | 616 ms (614 ms–630 ms) | 1.00, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.96 ms (5.43 ms–7.17 ms) | 6.09 ms (5.65 ms–6.75 ms) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^5 | 46.5 ms (44.5 ms–51.9 ms) | 49.9 ms (47.1 ms–52.2 ms) | 1.07, inside the spread |
| Delaunay `sphere` D2 10^6 | 459 ms (454 ms–466 ms) | 493 ms (490 ms–497 ms) | 1.07, slower beyond the spread |
| Delaunay `cube` D3 10^4 | 39.3 ms (36.9 ms–47.8 ms) | 38.6 ms (36.6 ms–41.2 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^5 | 416 ms (399 ms–437 ms) | 412 ms (398 ms–427 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^4 | 81.7 ms (80.6 ms–86.9 ms) | 81.7 ms (79.9 ms–89.5 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 819 ms (817 ms–858 ms) | 822 ms (814 ms–833 ms) | 1.00, inside the spread |
| Delaunay `cube` D4 10^4 | 676 ms (664 ms–702 ms) | 664 ms (650 ms–695 ms) | 0.98, inside the spread |
| Delaunay `cube` D5 10^4 | 8.14 s (8.06 s–8.37 s) | 8.12 s (8.06 s–8.32 s) | 1.00, inside the spread |

Every set published the same counts on both sides.

### Reading

- **Hull `sphere` D2 10^5 reads 0.87 to 0.94 over three runs.** Each run's interquartile ranges are apart, but the full ranges overlap through outliers.
- **Hull `sphere` D2 10^4 reads 0.91 to 0.94.** Its interquartile ranges overlap in one run.
- **Hull `cube` D2 reads 0.99 to 1.02, inside the spread.** That set keeps the hashing.
- **Delaunay D2 reads 0.99 to 1.00 since `d2ba949`.** The full shorter run's Delaunay rows were timed before that commit.
- **Nothing else is slower beyond the spread.**
- **The keep criterion** asked hull `sphere` D2 10^4 and 10^5 to be faster beyond the spread. By the rule on repeated runs (`bench.mdc`), 10^5 meets it; 10^4 agrees in direction but not beyond its spread. Whether to keep the change is the owner's call.

## The P7 parity run after P7-62, P7-64, and P7-67, PR #500 (#499)

Due after P7-62 (#486), P7-64 (#490), and P7-67 (#498), under the same-output references of ADR 0006 (amendments of 2026-10-11).

### Method

As in the run of #479: `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. convx is the head of #498 at `3bf1812`, whose code is that of `main` at `dc6523f`. "(ref.)" marks a reference that is not judged (ADR 0006). "/ CGAL" is against the reference with the same output; "/ CGAL from the points" is against the construction from the points.

The machine ran slower on this day than in the run of #479. On the sets no row of this period touched (hull `cube` and `sphere` D5 and D6), against the run of #479:

- convx's `build()` read 1.02 to 1.06;
- CGAL read 1.01 to 1.07;
- Qhull read 1.04 to 1.10.

The ratios between them moved by a few percent at most.

### P7 judgement

| Threshold | Met | Hull | Delaunay | In the run of #479 |
| --- | ---: | ---: | ---: | ---: |
| 1.05 | 32 of 41 | 21 | 11 | 30 |
| 1.00 | 26 of 41 | 19 | 7 | 24 |

Against the references from the points instead, 24 sets would be met at 1.05 and 22 at 1.00.

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.47 ms | 0.79 | 0.95 | 0.82 | met | met |
| Hull `cube` D2 10^5 | 4.55 ms | 0.80 | 1.00 | 0.81 | met | met |
| Hull `cube` D2 10^6 | 56.4 ms | 0.79 | 1.22 | 0.97 | met | met |
| Hull `cube` D3 10^4 | 1.26 ms | 0.55 | – | 0.80 | met | met |
| Hull `cube` D3 10^5 | 10.7 ms | 0.41 | – | 0.88 | met | met |
| Hull `cube` D3 10^6 | 143 ms | 0.23 | – | 0.95 | met | met |
| Hull `cube` D4 10^4 | 5.70 ms | 0.12 | – | 0.77 | met | met |
| Hull `cube` D4 10^5 | 32.9 ms | 0.06 | – | 0.95 | met | met |
| Hull `cube` D5 10^4 | 55.4 ms | 0.24 | – | 0.74 | met | met |
| Hull `cube` D5 10^5 | 216 ms | 0.12 | – | 0.65 | met | met |
| Hull `cube` D6 10^4 | 587 ms | 0.28 | – | 0.44 | met | met |
| Hull `cube` D6 10^5 | 2.54 s | 0.23 | – | 0.38 | met | met |
| Hull `sphere` D2 10^4 | 0.97 ms | 1.29 | 1.42 | 0.14 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 10.9 ms | 1.11 | 1.40 | 0.10 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 110 ms | 0.69 | 1.25 | 0.06 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 19.7 ms | 0.87 | – | 1.06 | not met | not met |
| Hull `sphere` D3 10^5 | 260 ms | 0.61 | – | 1.02 | met | not met |
| Hull `sphere` D3 10^6 | 3.47 s | 0.42 | – | 1.03 | met | not met |
| Hull `sphere` D4 10^4 | 87.6 ms | 0.72 | – | 0.95 | met | met |
| Hull `sphere` D4 10^5 | 1.15 s | 0.93 | – | 0.87 | met | met |
| Hull `sphere` D5 10^4 | 808 ms | 0.83 | – | 0.97 | met | met |
| Hull `sphere` D5 10^5 | 9.96 s | 0.93 | – | 0.93 | met | met |
| Hull `sphere` D6 10^4 | 6.66 s | 0.70 | – | 0.78 | met | met |
| Hull `cubesurf` D3 10^5 | 46.4 ms | 1.19 | – | 1.61 | not met | not met |
| Hull `grid` D6 10^4 | 242 ms | 0.96 (ref.) | – | 0.70 | met | met |
| Delaunay `cube` D2 10^4 | 5.28 ms | 1.11 | 1.32 | 0.28 | not met | not met |
| Delaunay `cube` D2 10^5 | 55.0 ms | 1.01 | 1.32 | 0.23 | met | not met |
| Delaunay `cube` D2 10^6 | 566 ms | 0.82 | 1.26 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 4.89 ms | 1.08 | 1.23 | 0.21 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 42.7 ms | 1.01 | 1.32 | 0.11 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 416 ms | 0.80 | 1.41 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 37.6 ms | 1.14 | 1.22 | 0.40 | not met | not met |
| Delaunay `cube` D3 10^5 | 408 ms | 1.14 | 1.27 | 0.31 | not met | not met |
| Delaunay `cube` D3 10^6 | 4.43 s | 1.06 | 1.26 | 0.27 | not met | not met |
| Delaunay `sphere` D3 10^4 | 67.4 ms | 0.52 | 0.51 | 0.57 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 658 ms | 0.68 | 0.70 | 0.34 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.69 s | 0.82 | 0.84 | 0.31 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 700 ms | 1.04 | 1.09 | 0.84 | met | not met |
| Delaunay `sphere` D4 10^4 | 880 ms | 0.34 | 0.35 | 0.67 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.61 s | 1.03 | 1.07 | 0.89 | met | not met |
| Delaunay `sphere` D5 10^4 | 11.24 s | 0.31 | 0.32 | 0.74 (ref.) | met | met |

### What moved

- **Hull `sphere` D3 10^6** reads 1.03 against Qhull, from 1.22, and is now met. `build()` took 0.88 of the run of #479, from P7-62 and P7-64.
- **Hull `sphere` D2:** `build()` took 0.90 of the run of #479 at 10^6, and 1.02 at 10^4 and 10^5, against a machine about 5% slower.
  - 10^5 reads 1.11 against CGAL, from 1.13, and 10^4 reads 1.29, from 1.26. Neither is met.
  - Under WSL, P7-67 saved less than the 0.87 to 0.94 it read on Windows (#498).
  - The phase timer "construct" is not comparable with the run of #479 for D = 2: the sort moved from construction into acceptance.
- **Delaunay `sphere` D2 10^5** reads 1.01, from 1.06, and is now met. No row changed Delaunay D2 in this period, so this is within the run-to-run difference.
- **Hull `cube` D2 10^6** reads 0.97 against Qhull, from 1.00, and is now met at 1.00.
- **Hull `cube` D3 10^6** reads 0.95 against Qhull, still met. P7-64's copy did not make it slower here.

### What is not met

9 sets:

| Group | Sets | Worst ratio |
| --- | ---: | --- |
| Delaunay `cube` D3 | 3 | 1.06 to 1.14 against CGAL |
| Delaunay D2 10^4 | 2 | 1.11 (`cube`) and 1.08 (`sphere`) against CGAL |
| Hull `sphere` D2 10^4 and 10^5 | 2 | 1.29 and 1.11 against CGAL |
| Hull `sphere` D3 10^4 | 1 | 1.06 against Qhull |
| Hull `cubesurf` D3 10^5 | 1 | 1.61 against Qhull |

## Hull `cubesurf` D3: the spike P7-68, and classification's shortcut inside a facet, PR #505 (#503, #504)

### The spike P7-68 (#503)

#### Counts, one build of `ConvexHullBuilder::build`

`main` at `839505b` with scratch counters (not committed), on the files of the parity run.

| Count | `cubesurf` D3 10^5 | `sphere` D3 10^5 | `cube` D3 10^5 |
| --- | ---: | ---: | ---: |
| plans (points inserted) | 163 | 99,996 | 193 |
| simplices allocated | 633 | 517,421 | 886 |
| points scanned by `take_outside` | 1,849,741 | 4,769,551 | 863,842 |
| of which proved by the cull plane's scan | 1,669,120 | 4,769,551 | 863,842 |
| of which zero by a record (no evaluation) | 80,698 | 0 | 0 |
| of which zero by `side` (evaluated) | 99,923 | 0 | 0 |
| orientations in construction | 99,999 | 7 | 7 |
| orientations that reached the exact stage | 0 | 0 | 0 |
| classification: points scanned in the walk (start group × neighbors) | 2,578,023 | 0 | 0 |
| classification: `side` calls | 495 | 299,994 | 519 |
| hull: vertices / facets / coplanar points | 155 / 169 / 99,845 | 100,000 / 199,996 / 0 | 175 / 346 / 0 |

#### Qhull on the same files (`qconvex s Ts`, under WSL, pinned)

| Count | `cubesurf` D3 10^5 | `sphere` D3 10^5 | `cube` D3 10^5 |
| --- | ---: | ---: | ---: |
| points processed | 168 | 100,000 | 205 |
| hyperplanes created | 501 | 565,222 | 952 |
| distance tests for partitioning | 2,046,860 | 3,335,261 | 983,234 |
| total distance tests | 5,028,602 | 8,340,842 | 1,288,155 |
| coplanar points found while partitioning | 99,832 | – | – |
| facets merged into a coplanar horizon | 163 | – | – |
| time, `i s` | 31 to 34 ms | 254 to 283 ms | 12.5 to 21.7 ms |
| time, `i s Qc` | 31 to 32 ms | 239 to 270 ms | 11.3 to 12.5 ms |
| time, `i s Qc Qi` | 31 to 32 ms | 228 to 235 ms | 51.9 to 56.3 ms |

On `cubesurf`, Qhull finds the coplanar points while it partitions, and keeping them (`Qc`) costs nothing.

#### Time, VTune on Windows

`main` (`839505b` code), 100 builds of `cubesurf` D3 10^5, software sampling. Inclusive shares of `ConvexHullBuilder::build`:

| Part | Share |
| --- | ---: |
| construction | 51.5% |
| of which `take_outside` | 36.2% |
| of which `side` (the zero tests, through the first stage) | 15.3% |
| of which the first stage's underflow path | 7.8% |
| of which the cull plane's scan | 8.4% |
| of which sorting `u32` lists | 7.5% |
| classification (`found_vertices`) | 41.1% |
| of which the walk's cull plane scan (2.58 M pairs) | 18.8% |
| of which the 2D hulls of each face's points (`accept` and `build` of D = 2) | 10.2% |
| acceptance | 5.5% |

#### Reading

- **Construction does about the work Qhull does:** 163 insertions against 168, 633 simplices against 501 hyperplanes, 1.85 M scanned points against 2.05 M partitioning tests. It is not more insertions or more side tests, and no test reached the exact stage.
- **What convx does and Qhull does not is classification, 41% of the build.** Every point on a face is scanned against each neighbor group of its start group, about 26 on average. Each face's points are then hulled in 2D to decide which are extreme. The gap to Qhull (1.61) is about this share.
- **Each zero test in construction costs about 70 ns:** 99,923 of them are 15.3% of the build, half of it the first stage's underflow path. Every point on a face is evaluated once against its own plane before a record covers it.

The Grill on #503 chose classification's shortcut first (P7-69), then a cheaper certificate of a zero side in construction, then what construction's sort of `u32` lists orders.

### The change (P7-69, #504)

For D = 3, classification builds a polygon per start group, from the group's vertices projected to a coordinate plane: the farthest vertex in each of eight directions, as D = 2's discard polygon (`polygon::extremes_polygon`). A recorded point strictly inside that polygon is in the facet's relative interior. It lies on no other facet's plane and is not extreme, so it is marked on the boundary. It skips the walk over neighbors, and it is not a candidate of the facet's extremes. The proof is in design §3, in both languages.

**Checks:**

- Debug builds test every point the shortcut decides against every group: zero on its own, negative on the others.
- `recorded_planes_change_no_partition` now checks:
  - every point left out of a facet's zero points is inside that facet's polygon;
  - the shortcut decides some points on the cube surface.
- A polygon that contains every point fails five tests, and turning the shortcut off fails the updated test.
- Every test passes in debug, in release, and under WSL.

### Against `main`

`main` at `839505b` against the head `b9dfca3`, pinned and alternated, the shorter run of `docs/verification.md` with hull `sphere` and `cube` D3 10^6: 10 rounds × 3 builds, 5 × 1 for the sets over about half a second. Every set published the same counts on both sides.

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.54 ms (0.47 ms–1.23 ms) | 0.57 ms (0.48 ms–0.93 ms) | 1.05, inside the spread |
| Hull `cube` D2 10^5 | 5.02 ms (4.34 ms–6.18 ms) | 5.24 ms (4.63 ms–9.02 ms) | 1.04, inside the spread |
| Hull `sphere` D2 10^4 | 1.11 ms (1.02 ms–1.50 ms) | 1.14 ms (1.01 ms–3.92 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^5 | 11.0 ms (9.88 ms–21.2 ms) | 11.2 ms (10.1 ms–17.7 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^4 | 1.48 ms (1.24 ms–2.09 ms) | 1.44 ms (1.29 ms–1.97 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^5 | 11.6 ms (10.8 ms–14.8 ms) | 12.2 ms (10.9 ms–20.9 ms) | 1.05, inside the spread |
| Hull `cube` D3 10^6 | 151 ms (149 ms–153 ms) | 153 ms (149 ms–160 ms) | 1.02, inside the spread |
| Hull `cubesurf` D3 10^5 | 48.5 ms (47.1 ms–59.5 ms) | 33.8 ms (32.6 ms–39.4 ms) | 0.70, faster beyond the spread |
| Hull `sphere` D3 10^4 | 22.2 ms (20.4 ms–27.7 ms) | 21.9 ms (20.1 ms–34.9 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^5 | 269 ms (250 ms–336 ms) | 267 ms (252 ms–316 ms) | 0.99, inside the spread |
| Hull `sphere` D3 10^6 | 3.69 s (3.29 s–3.85 s) | 3.71 s (3.28 s–3.76 s) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 5.72 ms (5.23 ms–6.92 ms) | 5.80 ms (5.25 ms–7.11 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 33.4 ms (32.2 ms–43.8 ms) | 33.7 ms (32.4 ms–48.3 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^4 | 88.5 ms (86.9 ms–92.8 ms) | 89.1 ms (87.7 ms–100 ms) | 1.01, inside the spread |
| Hull `sphere` D4 10^5 | 1.23 s (1.21 s–1.25 s) | 1.23 s (1.23 s–1.25 s) | 1.00, inside the spread |
| Hull `cube` D5 10^4 | 54.6 ms (53.0 ms–56.4 ms) | 54.6 ms (52.9 ms–58.5 ms) | 1.00, inside the spread |
| Hull `sphere` D5 10^4 | 818 ms (772 ms–862 ms) | 827 ms (768 ms–886 ms) | 1.01, inside the spread |
| Hull `cube` D6 10^4 | 559 ms (536 ms–636 ms) | 562 ms (545 ms–626 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 5.46 ms (5.24 ms–6.39 ms) | 5.45 ms (5.31 ms–6.34 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 57.1 ms (52.4 ms–63.6 ms) | 56.7 ms (52.5 ms–64.7 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^6 | 625 ms (607 ms–630 ms) | 618 ms (610 ms–630 ms) | 0.99, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.99 ms (5.33 ms–7.73 ms) | 5.71 ms (5.39 ms–6.96 ms) | 0.95, inside the spread |
| Delaunay `sphere` D2 10^5 | 47.5 ms (44.9 ms–52.7 ms) | 47.7 ms (45.2 ms–60.2 ms) | 1.01, inside the spread |
| Delaunay `sphere` D2 10^6 | 473 ms (467 ms–480 ms) | 464 ms (458 ms–475 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^4 | 39.6 ms (38.2 ms–53.2 ms) | 39.7 ms (37.7 ms–43.3 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 432 ms (416 ms–450 ms) | 433 ms (388 ms–522 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 80.6 ms (78.3 ms–91.1 ms) | 79.8 ms (78.1 ms–92.3 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^5 | 858 ms (833 ms–867 ms) | 873 ms (835 ms–886 ms) | 1.02, inside the spread |
| Delaunay `cube` D4 10^4 | 645 ms (632 ms–688 ms) | 646 ms (632 ms–724 ms) | 1.00, inside the spread |
| Delaunay `cube` D5 10^4 | 8.47 s (8.25 s–9.07 s) | 8.36 s (8.29 s–8.66 s) | 0.99, inside the spread |

**Under WSL** (as in the parity run), 10 rounds × 3 builds, alternated with Qhull (`qconvex i s`) on the parity run's file:

| `cubesurf` D3 10^5 | Median | Range |
| --- | ---: | --- |
| convx `main` | 47.2 ms | 43.9 to 53.6 ms |
| convx head | 33.0 ms | 30.5 to 44.8 ms |
| Qhull | 31.2 ms | 29.8 to 32.1 ms |

### Reading

- **Hull `cubesurf` D3 10^5 reads 0.70, faster beyond the spread,** on Windows and under WSL alike. Against Qhull it goes from 1.51 to 1.06 under WSL, just outside 1.05. Against CGAL (1.19 in the parity run of #499), it would be about 0.84.
- **Nothing else is slower beyond the spread.** Hull `cube` D2 reads 1.04 to 1.05, inside the spread. The change does not reach D = 2, and P7-67's runs put that set at 0.99 to 1.02.
- **The next rows of the Grill on #503** address construction: the zero tests (15.3% of the build before this change) and the sort of `u32` lists (7.5%).

## Predicates: the first stage skips its bound for an exactly zero determinant, PR #507 (#506)

### The change (P7-70)

The spike P7-68 (#503) found the zero side tests of hull `cubesurf` D3 10^5 at 15.3% of the build, half of it in the first stage's underflow path. The points lie exactly on axis-aligned faces, so a zero test's permanent is 0, and the bound took the form `s + 4 η X`, a product with a subnormal factor (about 28 ns, #311). A determinant that is exactly zero is never certified, since the bound is never negative. `has_zero_column` then decided the sign right after the first stage.

`Estimate` now keeps what its bound is computed from, and `sign` returns no sign for a determinant that is exactly zero before it computes the bound. Every sign is the same. The module docs of `semi_static` say why. This is the cheaper zero certificate of the Grill on #503: the certificate existed already, and the cost was the first stage before it.

**Checks:** every test passes in debug, in release, and under WSL. Returning `Zero` for an exactly zero determinant fails four tests, through the debug comparison with the exact sign.

### Against `main`

`main` at `839505b` against `cf3cd60`, pinned and alternated, the shorter run: 10 rounds × 3 builds, 5 × 1 for the sets over about half a second. Every set published the same counts on both sides. The machine was noisier than in earlier runs.

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.58 ms (0.48 ms–1.15 ms) | 0.57 ms (0.49 ms–0.84 ms) | 0.99, inside the spread |
| Hull `cube` D2 10^5 | 5.09 ms (4.54 ms–9.41 ms) | 5.21 ms (4.63 ms–6.88 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^4 | 1.15 ms (1.06 ms–1.62 ms) | 1.16 ms (1.02 ms–3.28 ms) | 1.01, inside the spread |
| Hull `sphere` D2 10^5 | 11.3 ms (9.99 ms–14.0 ms) | 11.5 ms (9.95 ms–13.0 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^4 | 1.54 ms (1.36 ms–2.24 ms) | 1.50 ms (1.35 ms–3.01 ms) | 0.97, inside the spread |
| Hull `cube` D3 10^5 | 12.5 ms (11.1 ms–15.1 ms) | 12.6 ms (11.0 ms–21.4 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^6 | 154 ms (150 ms–161 ms) | 154 ms (151 ms–173 ms) | 1.01, inside the spread |
| Hull `cubesurf` D3 10^5 | 52.7 ms (48.5 ms–72.3 ms) | 48.9 ms (44.5 ms–53.1 ms) | 0.93, inside the spread |
| Hull `sphere` D3 10^4 | 22.2 ms (20.6 ms–25.2 ms) | 23.3 ms (20.3 ms–31.4 ms) | 1.05, inside the spread |
| Hull `sphere` D3 10^5 | 273 ms (258 ms–307 ms) | 275 ms (260 ms–298 ms) | 1.01, inside the spread |
| Hull `sphere` D3 10^6 | 3.74 s (3.69 s–4.08 s) | 3.74 s (3.71 s–3.76 s) | 1.00, inside the spread |
| Hull `cube` D4 10^4 | 5.67 ms (5.26 ms–7.26 ms) | 5.74 ms (5.23 ms–6.97 ms) | 1.01, inside the spread |
| Hull `cube` D4 10^5 | 36.0 ms (33.1 ms–37.7 ms) | 36.0 ms (33.3 ms–41.5 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^4 | 97.5 ms (91.0 ms–109 ms) | 97.6 ms (92.7 ms–112 ms) | 1.00, inside the spread |
| Hull `sphere` D4 10^5 | 1.26 s (1.24 s–1.39 s) | 1.27 s (1.23 s–1.36 s) | 1.01, inside the spread |
| Hull `cube` D5 10^4 | 59.2 ms (54.4 ms–64.1 ms) | 58.3 ms (56.5 ms–63.5 ms) | 0.98, inside the spread |
| Hull `sphere` D5 10^4 | 870 ms (844 ms–1.01 s) | 867 ms (834 ms–983 ms) | 1.00, inside the spread |
| Hull `cube` D6 10^4 | 599 ms (588 ms–627 ms) | 603 ms (587 ms–638 ms) | 1.01, inside the spread |
| Delaunay `cube` D2 10^4 | 6.00 ms (5.34 ms–6.66 ms) | 5.84 ms (5.23 ms–8.08 ms) | 0.97, inside the spread |
| Delaunay `cube` D2 10^5 | 59.0 ms (55.7 ms–62.4 ms) | 57.8 ms (54.7 ms–64.7 ms) | 0.98, inside the spread |
| Delaunay `cube` D2 10^6 | 636 ms (619 ms–665 ms) | 607 ms (594 ms–630 ms) | 0.96, inside the spread |
| Delaunay `sphere` D2 10^4 | 6.21 ms (5.56 ms–7.71 ms) | 6.03 ms (5.50 ms–7.45 ms) | 0.97, inside the spread |
| Delaunay `sphere` D2 10^5 | 49.0 ms (46.5 ms–69.4 ms) | 47.5 ms (46.1 ms–52.1 ms) | 0.97, inside the spread |
| Delaunay `sphere` D2 10^6 | 495 ms (469 ms–520 ms) | 473 ms (455 ms–507 ms) | 0.95, inside the spread |
| Delaunay `cube` D3 10^4 | 41.1 ms (38.0 ms–44.8 ms) | 40.1 ms (38.0 ms–45.6 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^5 | 435 ms (421 ms–457 ms) | 433 ms (412 ms–478 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 88.7 ms (85.2 ms–108 ms) | 88.4 ms (83.9 ms–103 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^5 | 827 ms (819 ms–858 ms) | 835 ms (822 ms–892 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 746 ms (702 ms–856 ms) | 766 ms (753 ms–900 ms) | 1.03, inside the spread |
| Delaunay `cube` D5 10^4 | 8.74 s (8.21 s–9.72 s) | 8.65 s (8.56 s–9.01 s) | 0.99, inside the spread |

**Again, 30 rounds × 3 builds, and 10 × 1 for Delaunay `cube` D4 10^4:**

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cubesurf` D3 10^5 | 52.6 ms (46.9 ms–68.7 ms) | 47.8 ms (43.4 ms–71.6 ms) | 0.91, inside the spread |
| Hull `sphere` D3 10^4 | 22.6 ms (20.4 ms–39.9 ms) | 22.8 ms (20.2 ms–31.8 ms) | 1.01, inside the spread |
| Delaunay `cube` D4 10^4 | 749 ms (713 ms–788 ms) | 777 ms (738 ms–805 ms) | 1.04, inside the spread |

**With one codegen unit on both sides**, 20 rounds × 3 builds, and 10 × 1 for Delaunay `cube` D4 10^4:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cubesurf` D3 10^5 | 56.9 ms (50.4 ms–72.7 ms) | 50.8 ms (46.2 ms–66.4 ms) | 0.89, inside the spread |
| Delaunay `cube` D4 10^4 | 818 ms (773 ms–870 ms) | 795 ms (704 ms–834 ms) | 0.97, inside the spread |

**Under WSL**, `cubesurf` D3 10^5 on the parity run's file, 10 rounds × 3 builds, alternated with Qhull (`qconvex i s`). The last row is P7-69's head (`b9dfca3`) with this change applied, in a scratch copy:

| Build | Median | Interquartile range | Against Qhull |
| --- | ---: | --- | ---: |
| convx `main` | 47.7 ms | 46.2 to 49.3 ms | 1.56 |
| this change | 43.6 ms | 42.0 to 44.8 ms | 1.42 |
| Qhull | 30.6 ms | 29.2 to 31.5 ms | – |
| convx `main`, second run | 47.6 ms | 45.6 to 50.3 ms | 1.57 |
| P7-69 and this change | 29.0 ms | 27.8 to 30.1 ms | 0.95 |
| Qhull, second run | 30.4 ms | 29.3 to 31.2 ms | – |

### Reading

- **Hull `cubesurf` D3 10^5 reads 0.89 to 0.93 over three runs on Windows, and 0.91 under WSL.** Each run's interquartile ranges are apart; the full ranges overlap through outliers on a noisy machine.
- **With P7-69 (#505), `cubesurf` reads 0.95 against Qhull under WSL,** from 1.57: within 1.05 and 1.00.
- **Delaunay `cube` D4 10^4** read 1.03 and 1.04 in the separate builds. With one codegen unit on both sides it reads 0.97, so the difference was the partition into codegen units, not the change (`bench.mdc`).
- **Nothing else is slower beyond the spread.**

## Delaunay `cube` D3 and hull `sphere` D2 under WSL: the spike P7-71, the Hilbert order withdrawn, and P7-72's bound, PR #511 (#509, #510)

### The spike P7-71 (#509)

The parity run of #499 left these sets outside 1.05 against CGAL:

- Delaunay `cube` D3 at 1.06 to 1.14;
- Delaunay D2 10^4 at 1.08 and 1.11;
- hull `sphere` D2 at 1.29 and 1.11.

**perf under WSL** (software clock, no inlined frames) on the parity run's builds:

- **Hull `sphere` D2 10^5.** convx's `build()`: sorting the chain's keys about 25%, the left turns about 21%, the discard polygon about 11%, publication about 10%, classification about 9%. CGAL's `ch_akl_toussaint` splits the points into four regions by their extremes, and sorts and scans each region once. Its sort is about 55% of its time.
- **Delaunay `cube` D3.** Insertion took almost all of convx's time, inlined into two functions, so it could not be set beside CGAL function by function.

**Counts per site** (scratch counters in convx, not committed; callgrind call counts in CGAL):

| Per site | convx `cube` D3 10^4 | CGAL `cube` D3 10^4 | convx `cube` D2 10^4 | CGAL `cube` D2 10^4 |
| --- | ---: | ---: | ---: | ---: |
| in-sphere or in-circle tests | 44.4 | 42.8 | 8.9 | 9.0 |
| orientations | 16.8 | 4.4 | 11.8 | 3.0 |

The orientations were first read as a walk four times CGAL's. **That reading was wrong.** CGAL walks in plain doubles (`inexact_locate`, inlined), then calls `exact_locate` once per site. Its counted orientations are only the exact steps at the end.

**Where the gap is.** The parity run's phase timers already split convx into construction and the pass after the insertion:

| Delaunay `cube` D3 | convx construction | convx pass after the insertion | CGAL, whole |
| --- | ---: | ---: | ---: |
| 10^4 | 31.8 ms | 5.3 ms | 33.0 ms |
| 10^5 | 337 ms | 61 ms | 357 ms |
| 10^6 | 3,547 ms | 809 ms | 4,169 ms |

convx's construction alone is about CGAL's whole time, its output included. The gap is the pass after the insertion, as #469 found, and P7-60 (#483) addresses it.

### The Hilbert order (P7-73, #510), withdrawn

`3cc7a82` ordered the rounds of Delaunay D2 and D3 along a Hilbert curve (Skilling's transform on a grid of a few cells per site), in place of Morton order. A test checked that consecutive cells of the curve share a face on full grids. The walk shortened little: on `cube` D3 10^4, 7.6 simplices and 14.9 orientations per site, from 8.9 and 16.8. Against `main` (`839505b`), pinned and alternated, 15 rounds × 3 builds, 6 × 1 for 10^6:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 5.69 ms (5.30 ms–8.92 ms) | 5.98 ms (5.45 ms–7.94 ms) | 1.05, inside the spread |
| Delaunay `cube` D2 10^5 | 56.2 ms (52.7 ms–74.9 ms) | 58.0 ms (55.0 ms–63.2 ms) | 1.03, inside the spread |
| Delaunay `cube` D2 10^6 | 582 ms (550 ms–609 ms) | 671 ms (585 ms–700 ms) | 1.15, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.62 ms (5.38 ms–6.55 ms) | 6.37 ms (6.16 ms–7.26 ms) | 1.13, inside the spread |
| Delaunay `sphere` D2 10^5 | 44.1 ms (42.6 ms–55.5 ms) | 55.9 ms (53.6 ms–62.1 ms) | 1.27, inside the spread |
| Delaunay `sphere` D2 10^6 | 460 ms (441 ms–470 ms) | 684 ms (651 ms–728 ms) | 1.49, slower beyond the spread |
| Delaunay `cube` D3 10^4 | 36.3 ms (35.1 ms–41.5 ms) | 36.8 ms (35.7 ms–41.4 ms) | 1.01, inside the spread |
| Delaunay `cube` D3 10^5 | 401 ms (381 ms–522 ms) | 404 ms (380 ms–469 ms) | 1.01, inside the spread |
| Delaunay `sphere` D3 10^4 | 82.8 ms (77.8 ms–111 ms) | 82.3 ms (77.8 ms–123 ms) | 0.99, inside the spread |
| Delaunay `sphere` D3 10^5 | 799 ms (790 ms–807 ms) | 804 ms (777 ms–861 ms) | 1.01, inside the spread |

**A bound that cannot ship.** A scratch copy's walk used plain-double orientations with no exact check. It was timed against the same copy without the change, 15 rounds:

| Set | Ratio |
| --- | ---: |
| Delaunay `cube` D3 10^4 | 1.00 |
| Delaunay `cube` D3 10^5 | 1.00 |
| Delaunay `cube` D2 10^4 | 0.94 |

The walk's predicates are not where Delaunay `cube` D3 loses. P7-73 is withdrawn after measurement.

### P7-72's bound

The chain needs every point sorted already, for the duplicates (P7-67), so Akl–Toussaint's split would save mainly the second scan. That scan's cost was measured on a scratch copy: the upper pass run once more, its result dropped, against the same binary, 15 rounds.

| Set | One upper pass |
| --- | ---: |
| hull `sphere` D2 10^4 | 8.9% of the build |
| hull `sphere` D2 10^5 | 9.9% of the build |

The split adds a side test per point, so it would save at most about 5%: neither set would be met by it alone. P7-72 waits until it can be combined with another saving (Grill on #509).

A first attempt removed the upper pass instead. The wrong hull made classification about three times slower, so it bounded nothing.

## Delaunay: the pass after the insertion, P7-60 withdrawn, PR #513 (#483)

The spike P7-71 (#509) put Delaunay `cube` D3's gap to CGAL in the pass after the insertion: 14 to 18% of `build()`. It joins cospherical simplices into groups.

**The pass, stage by stage.** A scratch copy of `main` (`3a4d5ad`) with timers, Delaunay `cube` D3 10^5, three or four builds per variant. A variant reads its switch once per build. A first try that read it per face doubled the pass and was discarded.

| Stage | Time |
| --- | ---: |
| the pass over faces | 38 to 49 ms |
| the same, without the gate's orientation (cannot ship) | 28 to 39 ms |
| the same, without any work on unknown faces (cannot ship) | 10 to 11 ms |
| writing the draft | 14 to 16 ms |
| `Draft::finish` | 2 to 3 ms |
| renumbering, splitting merged groups | under 0.02 ms |

506,576 faces were unknown. Each read its neighbor's vertices for the far vertex, about 34 ns: a cache miss on a cell stored anywhere in the mesh. Each also took one orientation of four sites, its gate. None needed the lifted test, and no group merged.

**The prototype `185181e`** found the far vertex as the neighbor's vertex sum less the face's, in wrapping arithmetic, and the neighbor's finiteness from it, with one read. Against `main`, pinned and alternated, 20 rounds × 3 builds, 8 × 1 for the sets over half a second:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 5.65 ms (5.20 ms–9.69 ms) | 5.57 ms (5.20 ms–8.58 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^4 | 38.3 ms (35.9 ms–64.8 ms) | 37.6 ms (34.9 ms–47.1 ms) | 0.98, inside the spread |
| Delaunay `cube` D3 10^5 | 428 ms (393 ms–456 ms) | 408 ms (381 ms–460 ms) | 0.96, inside the spread |
| Delaunay `sphere` D3 10^4 | 83.4 ms (78.5 ms–94.9 ms) | 82.0 ms (78.8 ms–89.3 ms) | 0.98, inside the spread |
| Delaunay `sphere` D3 10^5 | 790 ms (775 ms–841 ms) | 785 ms (774 ms–806 ms) | 0.99, inside the spread |
| Delaunay `cube` D4 10^4 | 702 ms (692 ms–745 ms) | 671 ms (660 ms–703 ms) | 0.96, inside the spread |

With the timers on the branch, the pass took 35 to 40 ms, against 38 to 42 on `main`: the prototype removed comparisons, not the read. P7-60 is withdrawn after measurement.

**What is left.**

- Storing each face's far vertex during the insertion would add a write to every link it makes, in the 81% of the build that is insertion.
- A cheaper gate leaves at most its 9 ms.
- Ordering the faces by neighbor costs a sort of about 500,000 entries.

The Grill on #483 recorded the gap as the price of convx's unique output, and ADR 0006 now reports Delaunay `cube` D3 against CGAL without judging it (amendment of 2026-10-11).

## The P7 parity run after P7-69 and P7-70, PR #514 (#512)

Due after P7-69 (#505) and P7-70 (#507). Delaunay `cube` D3 is reported, not judged, by the amendment of ADR 0006 in #513.

### Method

As in the run of #499: `--release` with debug info, the parity run of `docs/verification.md` under WSL2 on the Intel Core i5-13400F, against Qhull and CGAL on the same machine. convx is `main` at `3a4d5ad`. "(ref.)" marks a reference that is not judged; "reported" marks a set reported, not judged (ADR 0006).

On the sets no row of this period touched (hull `cube` and `sphere` D5 and D6), against the run of #499:

- convx's `build()` read 0.97 to 1.01;
- CGAL read 0.96 to 1.01;
- Qhull read 0.93 to 0.96.

The references ran a few percent faster on this day than convx did, so the ratios lean about 2% against convx.

### P7 judgement

| Threshold | Met | Hull | Delaunay | Judged sets | In the run of #499 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1.05 | 33 of 38 | 22 | 11 | 38 | 32 of 41 |
| 1.00 | 27 of 38 | 20 | 7 | 38 | 26 of 41 |

The three sets of Delaunay `cube` D3 are reported. Against the references from the points instead, 25 sets would be met at 1.05 and 23 at 1.00.

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.45 ms | 0.79 | 0.91 | 0.81 | met | met |
| Hull `cube` D2 10^5 | 4.30 ms | 0.78 | 0.98 | 0.77 | met | met |
| Hull `cube` D2 10^6 | 52.9 ms | 0.74 | 1.15 | 0.90 | met | met |
| Hull `cube` D3 10^4 | 1.27 ms | 0.55 | – | 0.84 | met | met |
| Hull `cube` D3 10^5 | 10.3 ms | 0.41 | – | 0.88 | met | met |
| Hull `cube` D3 10^6 | 143 ms | 0.24 | – | 0.99 | met | met |
| Hull `cube` D4 10^4 | 5.12 ms | 0.12 | – | 0.78 | met | met |
| Hull `cube` D4 10^5 | 32.1 ms | 0.06 | – | 0.95 | met | met |
| Hull `cube` D5 10^4 | 53.7 ms | 0.24 | – | 0.75 | met | met |
| Hull `cube` D5 10^5 | 214 ms | 0.13 | – | 0.68 | met | met |
| Hull `cube` D6 10^4 | 574 ms | 0.28 | – | 0.47 | met | met |
| Hull `cube` D6 10^5 | 2.53 s | 0.23 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.90 ms | 1.23 | 1.47 | 0.14 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 10.3 ms | 1.07 | 1.39 | 0.10 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 111 ms | 0.75 | 1.29 | 0.06 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 19.1 ms | 0.89 | – | 1.07 | not met | not met |
| Hull `sphere` D3 10^5 | 242 ms | 0.63 | – | 1.02 | met | not met |
| Hull `sphere` D3 10^6 | 3.32 s | 0.43 | – | 1.01 | met | not met |
| Hull `sphere` D4 10^4 | 86.5 ms | 0.72 | – | 0.93 | met | met |
| Hull `sphere` D4 10^5 | 1.14 s | 0.94 | – | 0.88 | met | met |
| Hull `sphere` D5 10^4 | 789 ms | 0.83 | – | 0.98 | met | met |
| Hull `sphere` D5 10^5 | 10.04 s | 0.93 | – | 0.98 | met | met |
| Hull `sphere` D6 10^4 | 6.55 s | 0.71 | – | 0.81 | met | met |
| Hull `cubesurf` D3 10^5 | 27.2 ms | 0.74 | – | 0.97 | met | met |
| Hull `grid` D6 10^4 | 237 ms | 0.92 (ref.) | – | 0.69 | met | met |
| Delaunay `cube` D2 10^4 | 5.14 ms | 1.04 | 1.27 | 0.27 | met | not met |
| Delaunay `cube` D2 10^5 | 54.4 ms | 1.02 | 1.29 | 0.23 | met | not met |
| Delaunay `cube` D2 10^6 | 528 ms | 0.79 | 1.19 | 0.16 | met | met |
| Delaunay `sphere` D2 10^4 | 4.78 ms | 1.12 | 1.31 | 0.21 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 41.0 ms | 1.02 | 1.33 | 0.11 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 392 ms | 0.80 | 1.36 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 34.8 ms | 1.12 | 1.18 | 0.39 | reported | reported |
| Delaunay `cube` D3 10^5 | 372 ms | 1.10 | 1.21 | 0.30 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.08 s | 1.08 | 1.25 | 0.27 | reported | reported |
| Delaunay `sphere` D3 10^4 | 68.6 ms | 0.52 | 0.53 | 0.59 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 637 ms | 0.68 | 0.70 | 0.34 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.50 s | 0.82 | 0.85 | 0.33 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 697 ms | 1.07 | 1.13 | 0.88 | not met | not met |
| Delaunay `sphere` D4 10^4 | 879 ms | 0.34 | 0.34 | 0.67 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.48 s | 1.05 | 1.08 | 0.92 | met | not met |
| Delaunay `sphere` D5 10^4 | 10.94 s | 0.31 | 0.32 | 0.73 (ref.) | met | met |

**Delaunay `cube` D3, reported** (ADR 0006): convx's construction alone, from the phase timers, reads 0.94, 0.92, and 0.86 of CGAL's whole time at 10^4, 10^5, and 10^6.

### What moved

- **Hull `cubesurf` D3 10^5** reads 0.97 against Qhull, from 1.61, and is now met: `build()` took 0.59 of its time, from P7-69 and P7-70.
- **Delaunay `cube` D2 10^4** reads 1.04, from 1.11, and is now met. No row of this period changed Delaunay D2 beyond P7-70's first stage, so the move is mostly the day.
- **Delaunay `cube` D4 10^4** reads 1.07, from 1.04, and is no longer met. convx's `build()` took 1.00 of the run of #499, and CGAL 0.97.
- **Hull `sphere` D2 10^5** reads 1.07, from 1.11; 10^4 reads 1.23, from 1.29.

### What is not met

5 judged sets:

| Set | Ratio |
| --- | --- |
| Hull `sphere` D2 10^4 | 1.23 against CGAL |
| Hull `sphere` D2 10^5 | 1.07 against CGAL |
| Hull `sphere` D3 10^4 | 1.07 against Qhull |
| Delaunay `sphere` D2 10^4 | 1.12 against CGAL |
| Delaunay `cube` D4 10^4 | 1.07 against CGAL |

## Hull D2: the chain on the halves below and above the line through its ends, PR #516 (#515)

### The change (P7-72)

Hull `sphere` D2 read 1.23 at 10^4 and 1.07 at 10^5 against CGAL in the parity run of #512. The chain scanned every point twice, once for the lower chain and once for the upper. One upper pass was 8.9 to 9.9% of the build (#509).

As Akl–Toussaint's algorithm does, the sorted points are now split by the line through the first and the last:

- those strictly below make the lower chain with the two ends;
- those strictly above make the upper chain;
- those on the line go to neither, since a point on that segment is no strict extreme.

Each point is scanned once, and the cycle is the same. Design §6 says so in both languages.

**Checks:**

- A test compares the cycle with the two-pass chain on random points, small grids (many collinear points), points on a circle, and points on one line. Sending the points below and above to the wrong chains fails it.
- Every test passes in debug, in release, and under WSL.

The second part of the row, a cheaper pass over the discard polygon's extremes, was not needed: the split alone brings both sets within 1.05 under WSL. It is taken up again if a parity run puts them back outside.

### Against `main`

`main` at `3a4d5ad` against the head `80515e9`, pinned and alternated, 30 rounds × 3 builds:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.58 ms (0.50 ms–1.00 ms) | 0.57 ms (0.49 ms–4.73 ms) | 0.98, inside the spread |
| Hull `cube` D2 10^5 | 5.21 ms (4.83 ms–7.96 ms) | 5.23 ms (4.78 ms–7.00 ms) | 1.00, inside the spread |
| Hull `sphere` D2 10^4 | 1.20 ms (1.02 ms–9.01 ms) | 1.07 ms (0.89 ms–2.52 ms) | 0.89, inside the spread |
| Hull `sphere` D2 10^5 | 10.8 ms (9.95 ms–19.5 ms) | 9.87 ms (8.94 ms–13.1 ms) | 0.91, inside the spread |

**Under WSL**, on the parity run's files, 10 rounds of 200 builds (10^4) and 20 builds (10^5), alternated with CGAL's reference (`convex_hull_2` on input indices):

| Set | convx `main` | convx head | CGAL | `main` / CGAL | head / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: |
| hull `sphere` D2 10^4 | 0.839 ms | 0.722 ms | 0.732 ms | 1.15 | 0.99 |
| hull `sphere` D2 10^5 | 10.40 ms | 9.20 ms | 9.71 ms | 1.07 | 0.95 |

The interquartile ranges of `main` and the head are apart on both sets.

**The shorter run**, 10 rounds × 3 builds, 5 × 1 for the sets over about half a second. Every set published the same counts on both sides:

| Set | `main` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.60 ms (0.55 ms–0.90 ms) | 0.60 ms (0.55 ms–1.02 ms) | 1.01, inside the spread |
| Hull `cube` D2 10^5 | 5.44 ms (4.82 ms–9.27 ms) | 5.55 ms (4.82 ms–9.76 ms) | 1.02, inside the spread |
| Hull `sphere` D2 10^4 | 1.15 ms (1.03 ms–1.30 ms) | 1.04 ms (0.89 ms–2.52 ms) | 0.90, inside the spread |
| Hull `sphere` D2 10^5 | 11.8 ms (11.1 ms–17.8 ms) | 10.6 ms (9.93 ms–22.0 ms) | 0.89, inside the spread |
| Hull `cube` D3 10^4 | 1.64 ms (1.38 ms–2.42 ms) | 1.56 ms (1.37 ms–3.23 ms) | 0.95, inside the spread |
| Hull `cube` D3 10^5 | 12.9 ms (10.9 ms–23.6 ms) | 13.1 ms (12.1 ms–22.6 ms) | 1.01, inside the spread |
| Hull `cube` D3 10^6 | 144 ms (136 ms–157 ms) | 150 ms (140 ms–172 ms) | 1.04, inside the spread |
| Hull `cubesurf` D3 10^5 | 29.5 ms (28.6 ms–34.1 ms) | 29.5 ms (28.5 ms–36.0 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^4 | 22.6 ms (20.6 ms–36.7 ms) | 23.8 ms (20.3 ms–38.4 ms) | 1.05, inside the spread |
| Hull `sphere` D3 10^5 | 248 ms (239 ms–313 ms) | 249 ms (241 ms–276 ms) | 1.00, inside the spread |
| Hull `sphere` D3 10^6 | 3.47 s (3.34 s–3.67 s) | 3.52 s (3.30 s–3.88 s) | 1.01, inside the spread |
| Hull `cube` D4 10^4 | 5.48 ms (5.20 ms–5.99 ms) | 5.49 ms (5.24 ms–6.22 ms) | 1.00, inside the spread |
| Hull `cube` D4 10^5 | 33.7 ms (32.8 ms–39.1 ms) | 33.4 ms (32.3 ms–41.5 ms) | 0.99, inside the spread |
| Hull `sphere` D4 10^4 | 93.6 ms (89.1 ms–104 ms) | 91.8 ms (88.5 ms–131 ms) | 0.98, inside the spread |
| Hull `sphere` D4 10^5 | 1.33 s (1.11 s–1.38 s) | 1.22 s (1.09 s–1.30 s) | 0.92, inside the spread |
| Hull `cube` D5 10^4 | 60.3 ms (54.5 ms–70.6 ms) | 57.7 ms (54.0 ms–70.5 ms) | 0.96, inside the spread |
| Hull `sphere` D5 10^4 | 804 ms (779 ms–938 ms) | 833 ms (783 ms–1.02 s) | 1.04, inside the spread |
| Hull `cube` D6 10^4 | 568 ms (549 ms–650 ms) | 564 ms (550 ms–655 ms) | 0.99, inside the spread |
| Delaunay `cube` D2 10^4 | 5.37 ms (5.20 ms–6.29 ms) | 5.38 ms (5.13 ms–8.31 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^5 | 55.4 ms (51.8 ms–83.0 ms) | 55.6 ms (52.4 ms–74.4 ms) | 1.00, inside the spread |
| Delaunay `cube` D2 10^6 | 599 ms (578 ms–625 ms) | 612 ms (594 ms–669 ms) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^4 | 5.83 ms (5.33 ms–10.2 ms) | 5.93 ms (5.40 ms–7.71 ms) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^5 | 45.2 ms (42.8 ms–69.7 ms) | 45.9 ms (42.9 ms–50.0 ms) | 1.02, inside the spread |
| Delaunay `sphere` D2 10^6 | 470 ms (438 ms–512 ms) | 464 ms (457 ms–476 ms) | 0.99, inside the spread |
| Delaunay `cube` D3 10^4 | 36.4 ms (35.7 ms–41.5 ms) | 36.4 ms (35.7 ms–39.4 ms) | 1.00, inside the spread |
| Delaunay `cube` D3 10^5 | 391 ms (379 ms–471 ms) | 391 ms (379 ms–447 ms) | 1.00, inside the spread |
| Delaunay `sphere` D3 10^4 | 85.7 ms (77.8 ms–134 ms) | 82.8 ms (78.2 ms–120 ms) | 0.97, inside the spread |
| Delaunay `sphere` D3 10^5 | 779 ms (763 ms–882 ms) | 826 ms (790 ms–863 ms) | 1.06, inside the spread |
| Delaunay `cube` D4 10^4 | 688 ms (662 ms–793 ms) | 693 ms (666 ms–848 ms) | 1.01, inside the spread |
| Delaunay `cube` D5 10^4 | 8.22 s (7.93 s–8.52 s) | 8.00 s (7.78 s–8.54 s) | 0.97, inside the spread |

### Reading

- **Hull `sphere` D2 reads 0.86 to 0.91 against `main`,** on Windows and under WSL, more than the bound of one pass suggested (#509). Under WSL it reads 0.99 and 0.95 against CGAL, within 1.05.
- **Hull `cube` D2 reads 0.98 to 1.02.** There the discard polygon leaves few points for the chain.
- **Nothing is slower beyond the spread.** Hull `cube` D3 10^6, `sphere` D3 10^4, `sphere` D5 10^4, and Delaunay `sphere` D3 10^5 read 1.04 to 1.06 inside the spread. The change reaches only the D = 2 chain.

## The final P7 parity run, and the phase marks taken out of the judged time, PR #522 (#518)

P7 closes with this run (Grill on #512). Delaunay `cube` D3 is reported, not judged (ADR 0006, amendment of #513).

### A fault in the method, fixed

Through the run of #512, the judged `build()` came from the build with the phase marks. Each mark writes a line to standard error inside `build()`. A first final run of `b6934d5` timed the marked build. A build of the same commit without the marks, under WSL, pinned, 6 alternated rounds:

| Set | With the marks | Without | Ratio |
| --- | ---: | ---: | ---: |
| hull `sphere` D2 10^4 | 0.784 ms | 0.733 ms | 1.070 |
| hull `sphere` D3 10^4 | 19.43 ms | 19.02 ms | 1.021 |
| Delaunay `sphere` D2 10^4 | 4.80 ms | 4.70 ms | 1.020 |
| Delaunay `cube` D2 10^4 | 5.34 ms | 5.29 ms | 1.009 |
| Delaunay `sphere` D2 10^5 | 43.3 ms | 43.8 ms | 0.989 |

`bench.mdc` keeps timers added to measure out of the timed section. The runner now builds a second copy of the measured commit without the marks. In each round it runs that copy for the judged `build()`, then the marked copy for the phases only. `docs/verification.md` says so. Every earlier parity run judged the marked build, so its sets of 10^4 read up to a few percent against convx.

In this run, the marked builds against the plain ones, sets of 10^4:

| Set | Without the marks | With the marks | Ratio |
| --- | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.497 ms | 0.513 ms | 1.032 |
| Hull `cube` D3 10^4 | 1.196 ms | 1.176 ms | 0.983 |
| Hull `cube` D4 10^4 | 5.088 ms | 5.335 ms | 1.049 |
| Hull `cube` D5 10^4 | 51.909 ms | 52.351 ms | 1.009 |
| Hull `cube` D6 10^4 | 561.010 ms | 569.551 ms | 1.015 |
| Hull `sphere` D2 10^4 | 0.809 ms | 0.878 ms | 1.085 |
| Hull `sphere` D3 10^4 | 20.565 ms | 21.229 ms | 1.032 |
| Hull `sphere` D4 10^4 | 87.240 ms | 92.186 ms | 1.057 |
| Hull `sphere` D5 10^4 | 763.601 ms | 791.682 ms | 1.037 |
| Hull `sphere` D6 10^4 | 6431.647 ms | 6420.618 ms | 0.998 |
| Hull `grid` D6 10^4 | 236.484 ms | 235.331 ms | 0.995 |
| Delaunay `cube` D2 10^4 | 4.946 ms | 4.965 ms | 1.004 |
| Delaunay `sphere` D2 10^4 | 4.787 ms | 4.844 ms | 1.012 |
| Delaunay `cube` D3 10^4 | 35.029 ms | 34.993 ms | 0.999 |
| Delaunay `sphere` D3 10^4 | 65.791 ms | 66.337 ms | 1.008 |
| Delaunay `cube` D4 10^4 | 698.014 ms | 703.430 ms | 1.008 |
| Delaunay `sphere` D4 10^4 | 862.928 ms | 873.971 ms | 1.013 |
| Delaunay `cube` D5 10^4 | 7379.951 ms | 8077.897 ms | 1.095 |
| Delaunay `sphere` D5 10^4 | 11607.670 ms | 11421.459 ms | 0.984 |

### Method

As in the run of #512, with the fix above: `main` at `b6934d5`, under WSL2, against Qhull and CGAL on the same machine. On the sets no row of this period touched (hull `cube` and `sphere` D5 and D6), against the run of #512, convx's `build()` read 0.95 to 1.02, CGAL 0.94 to 1.06, and Qhull 0.97 to 1.10.

### P7 judgement

| Threshold | Met of the 38 judged sets | Hull | Delaunay |
| --- | ---: | ---: | ---: |
| 1.05 | 34 | 24 | 10 |
| 1.00 | 29 | 22 | 7 |

Against the references from the points instead, 27 sets would be met at 1.05 and 24 at 1.00.

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.50 ms | 0.76 | 0.92 | 0.79 | met | met |
| Hull `cube` D2 10^5 | 4.89 ms | 0.79 | 1.01 | 0.80 | met | met |
| Hull `cube` D2 10^6 | 62.2 ms | 0.92 | 1.38 | 1.10 | not met | not met |
| Hull `cube` D3 10^4 | 1.20 ms | 0.56 | – | 0.81 | met | met |
| Hull `cube` D3 10^5 | 9.58 ms | 0.38 | – | 0.83 | met | met |
| Hull `cube` D3 10^6 | 136 ms | 0.25 | – | 0.99 | met | met |
| Hull `cube` D4 10^4 | 5.09 ms | 0.12 | – | 0.77 | met | met |
| Hull `cube` D4 10^5 | 30.5 ms | 0.06 | – | 0.93 | met | met |
| Hull `cube` D5 10^4 | 51.9 ms | 0.24 | – | 0.73 | met | met |
| Hull `cube` D5 10^5 | 204 ms | 0.13 | – | 0.67 | met | met |
| Hull `cube` D6 10^4 | 561 ms | 0.28 | – | 0.46 | met | met |
| Hull `cube` D6 10^5 | 2.58 s | 0.22 | – | 0.36 | met | met |
| Hull `sphere` D2 10^4 | 0.81 ms | 1.01 | 1.20 | 0.11 (ref.) | met | not met |
| Hull `sphere` D2 10^5 | 9.67 ms | 0.99 | 1.25 | 0.08 (ref.) | met | met |
| Hull `sphere` D2 10^6 | 113 ms | 0.69 | 1.23 | 0.05 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 20.6 ms | 0.85 | – | 0.99 | met | met |
| Hull `sphere` D3 10^5 | 259 ms | 0.59 | – | 0.99 | met | met |
| Hull `sphere` D3 10^6 | 3.50 s | 0.42 | – | 1.00 | met | not met |
| Hull `sphere` D4 10^4 | 87.2 ms | 0.72 | – | 0.92 | met | met |
| Hull `sphere` D4 10^5 | 1.12 s | 0.89 | – | 0.86 | met | met |
| Hull `sphere` D5 10^4 | 764 ms | 0.80 | – | 0.96 | met | met |
| Hull `sphere` D5 10^5 | 10.02 s | 0.91 | – | 0.97 | met | met |
| Hull `sphere` D6 10^4 | 6.43 s | 0.69 | – | 0.81 | met | met |
| Hull `cubesurf` D3 10^5 | 28.3 ms | 0.70 | – | 0.91 | met | met |
| Hull `grid` D6 10^4 | 236 ms | 0.94 (ref.) | – | 0.70 | met | met |
| Delaunay `cube` D2 10^4 | 4.95 ms | 1.09 | 1.25 | 0.28 | not met | not met |
| Delaunay `cube` D2 10^5 | 51.4 ms | 1.01 | 1.28 | 0.23 | met | not met |
| Delaunay `cube` D2 10^6 | 522 ms | 0.78 | 1.21 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 4.79 ms | 1.11 | 1.23 | 0.21 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 42.3 ms | 1.05 | 1.36 | 0.12 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 401 ms | 0.82 | 1.29 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 35.0 ms | 1.12 | 1.19 | 0.39 | reported | reported |
| Delaunay `cube` D3 10^5 | 377 ms | 1.12 | 1.25 | 0.31 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.10 s | 1.09 | 1.26 | 0.27 | reported | reported |
| Delaunay `sphere` D3 10^4 | 65.8 ms | 0.51 | 0.51 | 0.60 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 631 ms | 0.67 | 0.69 | 0.34 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.49 s | 0.81 | 0.83 | 0.31 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 698 ms | 1.06 | 1.11 | 0.88 | not met | not met |
| Delaunay `sphere` D4 10^4 | 863 ms | 0.34 | 0.34 | 0.69 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.38 s | 1.01 | 1.05 | 0.85 | met | not met |
| Delaunay `sphere` D5 10^4 | 11.61 s | 0.32 | 0.33 | 0.77 (ref.) | met | met |

**Delaunay `cube` D3, reported:** convx's construction alone reads 0.94, 0.95, and 0.87 of CGAL's whole time at 10^4, 10^5, and 10^6.

### What moved since the run of #512

- **Hull `sphere` D2 10^4** reads 1.01, from 1.23, and **10^5** 0.99, from 1.07. Both are now met, from P7-72 (#516) and from the marks taken out.
- **Hull `sphere` D3 10^4** reads 0.99 against Qhull, from 1.07. Its marks were 3.2% of its build.
- **Hull `cube` D2 10^6** reads 1.10 against Qhull, from 0.90, and is no longer met. Its `build()` read 1.18 of the run of #512. Timed directly under WSL, the commits before and after #516 read 54.8 ms and 56.7 ms (1.034, 8 alternated rounds): P7-72 costs hull `cube` D2 2 to 3% (1.023 at 10^5). The rest is the day; the set took 62.2 ms in this run.
- **Delaunay `cube` D2 10^4** reads 1.09, from 1.04, with no change to its code.

### What is not met

| Set | Ratio | Reason |
| --- | --- | --- |
| Hull `cube` D2 10^6 | 1.10 against Qhull | P7-72's 3% and the day; 0.90 in the run of #512 |
| Delaunay `cube` D2 10^4 | 1.09 against CGAL | between 1.04 and 1.11 over the last four runs, with no change to its code |
| Delaunay `sphere` D2 10^4 | 1.11 against CGAL | constant factors spread over the insertion of sites outside the hull (#512); 1.04 to 1.12 over three runs in a row |
| Delaunay `cube` D4 10^4 | 1.06 against CGAL | 1.02 to 1.07 over the last runs |

Delaunay `cube` D3 (1.09 to 1.12) is reported: the price of the cospherical groups (ADR 0006).

## Inputs whose structure grows: faces and cospherical groups of many points, PRs #535, #542, #543, #547, #549, #552, #553, #555, #556 (#531, #533, #534, #539, #540, #541, #548, #551, #185, #554)

### Why these inputs

The parity sets and the generator's degenerate families keep a fixed number of distinct points on a face or a sphere. A step that is quadratic in the points of one face, or of one cospherical group, does not show on them. After P7 the allocation contract was tested on inputs whose faces and groups grow (#530), and timing the same inputs showed four such steps. Each was found by a series at doubling sizes, and placed by a profile (VTune, user-mode sampling, on the binary where cargo built it).

**The inputs:**

- **A prism over a k-gon**, D = 3: the points `(cos t, sin t, 0)` and `(cos t, sin t, 1)`. Two faces of k points.
- **A k-gon times a square**, D = 4: four facets that are prisms over the k-gon, with 2k points each.
- **The integer points of a sphere**, D = 3, of a squared radius chosen for its count: 168 at 101, 480 at 1,001, 1,920 at 10,001, 2,880 at 40,001, 6,630 at 1,221,025. Alone, every site is on one sphere. With one more site outside the sphere, they are one cospherical group beside other cells.

**Method, for every table below:** release builds, one `build()` per timing, the minimum over the builds of a run. Windows, the machine of the earlier sections. Each table says whether its runs were pinned to one core and alternated. Every "before" and "after" is the base and the head of the pull request named, built in scratch copies.

### The placing triangulation of a face in a plane, PR #535 (#533, #534)

The placing triangulation tested each point against every boundary ridge of the complex built so far, about k^2 / 2 orientations for a face of k points. A face in a plane is now read from the cyclic order of its points.

The hull of a prism over a k-gon, base `d26b203`, head `bcf07ab`, pinned and alternated:

| k | Before | After |
| ---: | ---: | ---: |
| 2,048 | 138 ms | 11.3 ms |
| 4,096 | 533 ms | 22.2 ms |

The head alone read 1.4 ms at k = 256 and 89.7 ms at k = 16,384: linear in k.

### The pulling triangulation, PR #542 (#539)

When every site is on one sphere, the pulling triangulation intersected each face with every facet of the site hull. The facets of a face are now found from the facets at its vertices.

Delaunay D = 3, the integer points of a sphere alone, base `356c8ee`, head `f9b2f39`, the minimum of 5, one after the other at each size, not pinned:

| Sites | Before | After | Allocations before | Allocations after |
| ---: | ---: | ---: | ---: | ---: |
| 168 | 1.21 ms | 1.26 ms | 3,206 | 130 |
| 480 | 4.33 ms | 2.10 ms | 8,118 | 186 |
| 1,920 | 47.1 ms | 8.13 ms | 28,395 | 289 |
| 6,630 | 468 ms | 30.5 ms | 81,192 | 262 |

The measurement of #531, on the same base, read 411 ms at 6,630 sites.

### The facets and edges of a cospherical group of a Voronoi build, PR #543 (#541)

Every pair of a group's sites was asked whether it is an edge, against every facet of the group: cubic in the sites of one group. A profile by line put 88% of the samples of the 480-site input on that test. The facets now come from the group's boundary faces joined across their ridges, and the edges from the edges of those faces.

Voronoi D = 3, the integer points of a sphere alone, base #542's head, the minimum of 1 to 5, not pinned:

| Sites | Before | After |
| ---: | ---: | ---: |
| 168 | 11.4 ms | 1.79 ms |
| 480 | 189 ms | 5.44 ms |
| 1,920 | 17,208 ms | 20.4 ms |
| 2,880 | 59,929 ms | 29.9 ms |

The row at 2,880 sites took its "before" from an earlier scratch state than the other rows (the retrospective of #543).

Guards, pinned and alternated, the minimum of four rounds:

| Voronoi input | Before | After | Ratio |
| --- | ---: | ---: | ---: |
| `lattice` D3, 4,000 points | 2.179 ms | 2.137 ms | 0.98 |
| `lattice` D2, 4,000 points | 0.295 ms | 0.317 ms | 1.07 |
| full grid 20^3 | 167.4 ms | 168.5 ms | 1.01 |
| full grid 100^2 | 24.78 ms | 24.15 ms | 0.97 |
| full grid 7^4 | 452.3 ms | 440.4 ms | 0.97 |
| `cube` D3, 30,000 points | 348.6 ms | 334.9 ms | 0.96 |
| `cube` D2, 100,000 points | 199.3 ms | 185.5 ms | 0.93 |

`lattice` D2 reads 0.240 ms on both sides with one codegen unit.

### The placing triangulation of three dimensions or more, PR #547 (#540)

**Before, on `main` at `356c8ee`** (the measurement of #531), one after the other, not pinned:

| Input | Size | Time |
| --- | ---: | ---: |
| Hull D = 4, a k-gon times a square | k = 256 | 64.8 ms |
| | k = 512 | 223 ms |
| | k = 1,024 | 889 ms |
| | k = 2,048 | 3,281 ms |
| Delaunay D = 3, a sphere and a site outside | 481 sites | 13.8 ms |
| | 1,921 sites | 173 ms |
| | 2,881 sites | 371 ms |
| | 6,631 sites | 1,907 ms |

Both are quadratic. The profile puts 77% of the samples of the first at k = 1,024 and 91% of the second at 6,631 sites in the placing triangulation and the orientations it calls.

**The change:** once the complex has the dimension of the points, the boundary ridges keep their neighbors. One ridge a point is beyond is searched for from a ridge of the point before, and the others follow across shared faces. The search is ordered by a height in `f64` that decides nothing (design §3).

**The prototype**, a scratch copy of `main` at `1467557`, the minimum of 2, one pair of builds, not pinned; the whole `Debug` form of each result has the same hash on both sides:

| Input | Size | Before | After |
| --- | ---: | ---: | ---: |
| Hull D = 4, a k-gon times a square | k = 512 | 222 ms | 43.4 ms |
| | k = 1,024 | 857 ms | 81.9 ms |
| | k = 2,048 | 3,349 ms | 171 ms |
| The same, index order shuffled | k = 512 | 243 ms | 64.1 ms |
| | k = 1,024 | 896 ms | 137 ms |
| | k = 2,048 | 3,492 ms | 283 ms |
| Delaunay D = 3, a sphere and a site outside | 1,921 sites | 182 ms | 27.4 ms |
| | 2,881 sites | 400 ms | 36.2 ms |
| | 6,631 sites | 2,017 ms | 92.0 ms |
| The same, index order shuffled | 1,921 sites | 174 ms | 31.3 ms |
| | 2,881 sites | 402 ms | 51.7 ms |
| | 6,631 sites | 1,989 ms | 145 ms |

**The order of the search**, three guides measured on the k-gon times a square at k = 1,024:

| Guide | In order | Shuffled |
| --- | ---: | ---: |
| The distance to a ridge's centroid | 163 ms | 167 ms |
| The height over a ridge alone | 81.1 ms | 541 ms |
| The height, then the nearest vertex of a ridge (kept) | 81.9 ms | 137 ms |

The first walks through the many ridges of one face of a prism before it reaches the side the point is beyond. The second searches the whole of a face whose ridges share one height.

**Small sets:** with every set on the path, Delaunay full grid 20^3 read 1.08 and `lattice` D3 1.10. The path is taken when more than 16 points are left once the complex has its dimension.

**The head `f901ab8` against `main` at `1467557`**, pinned and alternated, the minimum of four rounds, the same hash on both sides in every row:

| Input | Before | After | Ratio |
| --- | ---: | ---: | ---: |
| Hull D = 4, a 2,048-gon times a square | 3,372 ms | 167.6 ms | 0.05 |
| Delaunay D = 3, a sphere of 6,630 points and a site outside | 2,017 ms | 87.9 ms | 0.04 |
| Hull D = 4, a 16-gon times a square | 1.420 ms | 1.323 ms | 0.93 |
| Delaunay full grid 20^3 | 138.3 ms | 138.1 ms | 1.00 |
| Delaunay full grid 7^4 | 407.6 ms | 385.4 ms | 0.95 |
| Delaunay, integers of [-2, 2]^3, 4,000 points | 1.567 ms | 1.602 ms | 1.02 |
| Delaunay, integers of [-2, 2]^4, 4,000 points | 77.61 ms | 77.53 ms | 1.00 |
| Hull D = 4, 20,000 grid points on the faces of a cube | 5.828 ms | 5.990 ms | 1.03 |
| Delaunay D = 3, 30,000 points in general position | 113.8 ms | 115.8 ms | 1.02 |
| Hull D = 4, 30,000 points in general position | 11.10 ms | 11.10 ms | 1.00 |

With one codegen unit on both sides the three rows at 1.02 and 1.03 read 1.01, 1.00, and 1.00.

### The sub-hull of a face in kept storage, PR #549 (#531)

A face that is not a simplex is classified through a hull of one dimension less. In D >= 4 that hull was built from nothing for each face, about a hundred allocations a face over about 70 call sites, each once per sub-hull (heap profiles by call site at k = 32 and 256, `dhat`, scratch). It is now built in the storage of the one before (design, the allocation contract).

Allocations of one `build()`, at the small and the large size of `tests/alloc.rs`, base #547's head, head `6ac7b6d`:

| Case | Before | After |
| --- | --- | --- |
| Hull, a k-gon times a square, k = 8 and 128 | 1,305 and 13,186 | 225 and 319 |
| Hull `cubesurf` D4 | 1,076 and 1,267 | 328 and 368 |
| Hull `grid` D4 | 1,700 and 1,700 | 276 and 274 |
| Hull `cluster` D4 | 16,345 and 13,884 | 2,740 and 1,960 |
| Hull `cluster` D3 | 477 and 436 | 234 and 227 |

Over the 44 cases the allocations at the large size fall from 42,197 to 14,002 in sum. Of the 3,393 left on hull `cluster` D4, 2,404 are the exact direction of a plane, two per call (#548).

**The exact direction, PR #552 (#548):** returned inline. The generator's `cluster` has eight clusters however many points are drawn, so its count was fixed; clusters that grow show the growth (k points of `sphere`, each with the 2^D points at 0 or 2^-30 from it along every axis; a simplex with two vertices of one cluster is a sliver, which the cofactors in `f64` do not certify). Base #549's head, head `06104b5`:

| Case | Before | After |
| --- | --- | --- |
| Hull, clusters on a sphere D4, k = 16 and 256 | 1,820 and 7,171 | 286 and 341 |
| Hull, clusters on a sphere D3, k = 32 and 512 | 239 and 457 | 169 and 217 |
| Hull `cluster` D4 | 2,740 and 1,960 | 336 and 328 |
| Hull `cluster` D3 | 234 and 227 | 176 and 177 |

Its time does not move: over ten alternated launches, pinned, the hull of 1,024 clusters in D4 has a median of 78.4 ms before and 78.2 ms after, and of 256 clusters 29.9 ms and 30.0 ms, with the same hash of the result.

Time, pinned and alternated, the minimum of eight rounds, one pair of builds, the same hash of the whole result on both sides in every row:

| Input | Before | After | Ratio |
| --- | ---: | ---: | ---: |
| Hull D = 4, a 2,048-gon times a square | 165.9 ms | 150.0 ms | 0.90 |
| Hull D = 4, a 256-gon times a square | 20.18 ms | 18.22 ms | 0.90 |
| Hull D = 4, a 16-gon times a square | 1.336 ms | 1.185 ms | 0.89 |
| Hull `cube` D2, 200,000 points | 9.853 ms | 9.921 ms | 1.01 |
| Hull `cube` D3, 200,000 points | 26.96 ms | 26.52 ms | 0.98 |
| Hull `cube` D4, 30,000 points | 11.01 ms | 11.29 ms | 1.03 |
| Hull `cube` D5, 5,000 points | 34.84 ms | 34.49 ms | 0.99 |
| Hull `sphere` D3, 100,000 points | 252.3 ms | 256.1 ms | 1.02 |
| Hull `cubesurf` D3, 100,000 points | 30.83 ms | 30.22 ms | 0.98 |
| Hull `cubesurf` D4, 20,000 points | 36.76 ms | 37.51 ms | 1.02 |
| Hull `grid` D3, 100,000 points (64 distinct) | 1.181 ms | 1.279 ms | 1.08 |
| Hull D = 3, a prism over a 2,048-gon | 9.826 ms | 9.961 ms | 1.01 |
| Delaunay `cube` D3, 30,000 points | 116.6 ms | 115.4 ms | 0.99 |
| Voronoi `cube` D3, 20,000 points | 213.6 ms | 213.4 ms | 1.00 |
| Delaunay D = 3, a sphere of 6,630 points and a site outside | 87.55 ms | 88.33 ms | 1.01 |
| Delaunay full grid 16^3 | 67.59 ms | 66.38 ms | 0.98 |

**Hull `grid` D3** reads 1.08 by the minimum. Over 20 alternated launches its median is 1.297 ms before and 1.281 ms after, and its first quartile 1.274 and 1.269; the base has a few faster launches (1.17 ms) that the head did not show (1.25 ms at best). With timers inside the build, construction read 114 to 125 us on the head against 125 to 169 us, and classification 158 to 170 us against 164 to 297 us. The rest of that build is acceptance of the 100,000 points, which the change does not touch. The gap of the minimum is not explained.

Three costs found on the way and removed, each on a guard that read slower in a first version:

| Guard | What cost | Read | After |
| --- | --- | ---: | ---: |
| Hull `cubesurf` D3, 100,000 points | a sort of (start group, point) pairs where a list per group had been; now a counting sort | 1.04 to 1.06 | 1.00 |
| Hull `grid` D3, 100,000 points | a list of flags per point kept with the level, so the lists after it found no freed storage of that size | median 1.04 | below |
| Hull `grid` D3, 100,000 points | flags written over every page of three lists, where a zeroed allocation touches only the pages it sets | median 1.04 | median 0.99 |

**The copy in space order, PR #553 (#551):** a hull of D = 3 with 2^17 representatives or more is built on a copy of its points in Morton order, and its lists are now kept with the hull's storage. The path at the top level, base #552's head, head `de4bcfe`, pinned and alternated, six rounds, the same hash of the result on both sides:

| Hull | Before, minimum and median | After, minimum and median | Ratio of the minimum |
| --- | ---: | ---: | ---: |
| `sphere` D3, 10^6 points | 3,346 and 3,494 ms | 3,284 and 3,489 ms | 0.98 |
| `sphere` D3, 200,000 points | 536.9 and 542.3 ms | 533.7 and 549.8 ms | 0.99 |
| `cube` D3, 10^6 points | 165.8 and 172.8 ms | 164.8 and 169.7 ms | 0.99 |
| `cubesurf` D3, 10^6 points | 309.8 and 314.4 ms | 306.2 and 314.5 ms | 0.99 |
| `cube` D3, 100,000 points (below the threshold) | 12.23 and 12.77 ms | 12.51 and 12.55 ms | 1.02 |

**The exact stage over heap integers, PR #555 (#185):** found by a scan of 90 inputs at two sizes after #551 (every builder; points, faces, cospherical groups, and uncertified planes growing; D = 2 to 6): the one input whose count grew per item was the generator's `cube` with its first coordinate scaled by 1e150 and the others by 1e-150. The exponents of one determinant are then about 1,000 apart, more than the 1,024 bits of the integers on the stack, and the heap integers take it, reserving every buffer in every call. They run on a workspace kept per thread now. Base `main` at `94d9d5c`, head `66a0346`.

Allocations of one `build()`:

| Hull | Points | Before | After |
| --- | ---: | ---: | ---: |
| D = 3 | 100 | 17,864 | 144 |
| D = 3 | 1,600 | 69,384 | 120 |
| D = 4 | 50 | 132,838 | 120 |
| D = 4 | 800 | 840,505 | 128 |

The counts after are those of `tests/alloc.rs`, where the small size runs first on the thread and reserves the workspace.

Time, pinned and alternated, five rounds, the same hash of the result on both sides:

| Hull | Before, minimum and median | After, minimum and median | Ratio of the minimum |
| --- | ---: | ---: | ---: |
| D = 3, 400 such points | 5.33 and 5.80 ms | 3.39 and 3.47 ms | 0.64 |
| D = 3, 1,600 such points | 7.21 and 7.62 ms | 5.09 and 5.18 ms | 0.71 |
| D = 4, 200 such points | 67.6 and 74.8 ms | 52.1 and 53.9 ms | 0.77 |
| D = 4, 400 such points | 142.1 and 150.2 ms | 96.7 and 103.4 ms | 0.68 |
| D = 5, 100 such points | 719.8 and 744.4 ms | 589.6 and 611.3 ms | 0.82 |
| `cube` D3, 200,000 points | 26.75 and 28.64 ms | 26.93 and 28.81 ms | 1.01 |
| `cube` D4, 30,000 points | 11.31 and 12.39 ms | 12.37 and 12.49 ms | 1.09 |
| `sphere` D3, 100,000 points | 264.0 and 267.0 ms | 263.9 and 264.7 ms | 1.00 |
| `cubesurf` D4, 20,000 points | 37.67 and 38.03 ms | 36.62 and 37.28 ms | 0.97 |
| clusters on a sphere D4, k = 256 | 28.04 and 28.27 ms | 27.59 and 27.95 ms | 0.98 |
| Delaunay `cube` D3, 30,000 points | 117.5 and 121.9 ms | 117.4 and 118.2 ms | 1.00 |

`cube` D4 reads 1.09 by the minimum and 1.01 by the median: one launch of the base was faster than its others. It does not reach the heap integers.

The heap integers also take every determinant of a size above 10, so a hull of D >= 10. The generator's `cube`, one `build()`, not pinned, the same hash on both sides:

| Hull | Facets | Allocations before | Allocations after | Time before | Time after |
| --- | ---: | ---: | ---: | ---: | ---: |
| D = 11, 14 points | 108 | 65,819 | 3,474 | 23.8 ms | 21.1 ms |
| D = 11, 18 points | 1,246 | 756,970 | 41,054 | 276 ms | 220 ms |
| D = 12, 16 points | 291 | 374,855 | 10,459 | 173 ms | 144 ms |

What is left there grows with the facets and is not the heap integers: the planes of new simplices allocate from D = 9 up, in lists that are inline only up to a length of 8 or 10 (#554). D = 7 and 8 do not grow (116 and 139 allocations at 60 and 240 points in D = 7).

**Hulls of D = 9 and 10, PR #556 (#554):** the planes of new simplices allocated in two lists of capacity 8 (the normal of a cull plane, and the key of a ridge in the plan of a region) and in the filter's rows on the heap for sizes 10 and 11. Base #555's head, head `6f96ed7`. Allocations of one `build()` of the generator's `cube`:

| Build | Points | Before | After |
| --- | ---: | ---: | ---: |
| Hull D = 8 | 40 and 120 | 111 and 126 | 111 and 126 |
| Hull D = 9 | 24 and 44 | 6,782 and 73,634 | 97 and 111 |
| Hull D = 10 | 20 and 32 | 16,063 and 184,032 | 94 and 103 |
| Delaunay D = 8 | 16 and 26 | 7,354 and 7,354 | 7,354 and 7,354 |
| Delaunay D = 9 | 14 and 22 | 11,312 and 22,306 | 11,066 and 11,066 |

Delaunay's count is fixed in the points and large: about 7,000 and 11,000 allocations of a build in D = 8 and 9 whatever the sites. That is inside the contract and has not been traced.

Time, pinned and alternated, five rounds, minimum and median, the same hash on both sides:

| Hull | Before | After | Ratio of the minimum |
| --- | ---: | ---: | ---: |
| `cube` D3, 200,000 points | 26.67 and 27.40 ms | 26.41 and 26.86 ms | 0.99 |
| `cube` D4, 30,000 points | 12.44 and 12.55 ms | 12.60 and 12.95 ms | 1.01 |
| `cube` D5, 5,000 points | 34.65 and 37.34 ms | 35.38 and 37.94 ms | 1.02 |
| `cube` D6, 1,500 points | 126.4 and 129.1 ms | 125.6 and 129.3 ms | 0.99 |
| `cube` D8, 60 points | 87.56 and 88.67 ms | 86.03 and 87.89 ms | 0.98 |
| `sphere` D3, 100,000 points | 248.6 and 259.0 ms | 262.9 and 268.1 ms | 1.06 |
| `cubesurf` D4, 20,000 points | 37.36 and 39.14 ms | 37.12 and 39.98 ms | 0.99 |
| Delaunay `cube` D3, 30,000 points | 114.4 and 121.6 ms | 114.9 and 122.1 ms | 1.00 |
| `cube` D9, 44 points | 261.4 and 276.4 ms | 265.2 and 282.4 ms | 1.01 |
| `cube` D9, 60 points | 992.1 and 1,005 ms | 984.5 and 1,006 ms | 0.99 |
| `cube` D10, 32 points | 944.4 and 969.3 ms | 889.4 and 918.5 ms | 0.94 |

`sphere` D3 read 1.06 in that run. Over eight rounds more it read 0.95 by the minimum and 0.92 by the median, and with one codegen unit 1.02 and 0.99; at 400,000 points 1.01 and 1.01. `cube` D5 read 1.04 and 1.01 in the eight rounds, 1.02 and 1.02 with one codegen unit. The machine's spread in these runs was several percent.

### The scans behind the allocation contract (#185, #554, #560)

The contract covers D <= 10 (design; the owner's decision of 2026-10-13 on #554). Three scans counted the allocations of one `build()` at two sizes or more, on release builds, each input once:

- **D = 2 to 6, 90 inputs** (on #553's head): every builder; points, faces, cospherical groups, and uncertified planes growing. It found the heap exact stage (#185).
- **D = 7 to 11, the generator's `cube`** (on #555's head): it found the lists of capacity 8 and the filter's rows of D = 9 and 10 (#554).
- **D = 5 to 10, degenerate inputs** (on #556's head, #560): the table below. No count grows per item.

Points and allocations, `points:allocations`:

| Build | Input | D | Counts |
| --- | --- | ---: | --- |
| Hull | a k-gon times a cube | 7 | 128:512, 256:563, 512:618 |
| Hull | a k-gon times a cube | 8 | 192:604, 256:619 |
| Hull | `cubesurf` | 7 | 100:226, 200:280, 400:318 |
| Hull | `cubesurf` | 8 | 80:118, 160:247 |
| Hull | `cubesurf` | 9 | 60:117, 100:222 |
| Hull | `cubesurf` | 10 | 40:109, 60:119 |
| Hull | points on the faces of a cube, on a grid of 1/64 | 7 | 200:281, 800:330, 3,200:364 |
| Hull | the same | 8 | 200:260, 800:323 |
| Hull | the same | 9 | 200:259, 600:329 |
| Hull | the full grid of side 2 and of side 3 | 7 | 128:478, 2,187:612 |
| Hull | the same | 8 | 256:570, 6,561:737 |
| Hull | the corners of a cube | 9 | 512:666 |
| Hull | clusters on a sphere | 7 | 1,280:766, 1,792:790 |
| Hull | every point twice | 7 | 60:99, 120:114 |
| Hull | every point twice | 9 | 40:94, 60:105 |
| Hull | `nearsphere` | 5 | 200:393, 800:535, 3,200:580, 12,800:571 |
| Hull | `nearsphere` | 6 | 200:513, 800:713, 3,200:826 |
| Hull | `nearsphere` | 7 | 100:257, 200:326, 400:686, 800:767, 1,600:812 |
| Hull | `nearsphere` | 8 | 200:292, 400:427 |
| Hull | exponents far apart | 7 | 20:203, 30:101 |
| Hull | exponents far apart | 9 | 14:130, 18:91 |
| Hull | the integer points of a sphere | 7 | 84:449, 280:485 |
| Hull, with `planes()` | a k-gon times a cube | 7 | 128:522, 256:573 |
| Hull, with `planes()` | `cube` | 9 | 24:109, 44:117 |
| Delaunay | `cube` | 7 | 14:4,670, 20:4,670, 28:4,670 |
| Delaunay | `cube` | 9 | 14:11,066, 22:11,066 |
| Delaunay | the full grid of side 3 and of side 4 | 5 | 243:1,591, 1,024:1,591 |
| Delaunay | unit cubes in a row | 5 | 64:1,591, 256:1,591, 1,024:1,591 |
| Delaunay | unit cubes in a row | 6 | 128:2,840, 512:2,840, 2,048:2,840 |
| Delaunay | unit cubes in a row | 7 | 256:4,709, 768:4,709 |
| Delaunay | the integer points of a sphere | 5 | 40:98, 112:110, 200:111 |
| Delaunay | the same | 6 | 60:103, 160:106 |
| Delaunay | the same and a site outside | 5 | 41:1,603, 113:1,603, 201:1,605 |
| Delaunay | the same and a site outside | 6 | 61:2,838, 161:2,841 |
| Delaunay | every point twice | 6 | 40:2,804, 80:2,804 |
| Voronoi | `cube` | 5 | 40:1,727, 80:1,738, 160:1,744 |
| Voronoi | `cube` | 6 | 30:2,961, 50:2,967 |
| Voronoi | `cube` | 7 | 20:4,815, 28:4,816 |
| Voronoi | unit cubes in a row | 5 | 64:2,005, 256:2,005, 1,024:2,005 |
| Voronoi | unit cubes in a row | 6 | 128:3,363, 512:3,368 |
| Voronoi | the integer points of a sphere | 5 | 40:430, 112:373 |
| Voronoi | the same and a site outside | 5 | 41:1,941, 113:1,882 |

**Hull `nearsphere`** rises the most: 326 to 686 between 200 and 400 points in D = 7. It levels off, 767 at 800 and 812 at 1,600, and in D = 5 it stays at 571 to 580 from 3,200 points on. Its distinct points are a fixed set, so its faces stop growing; the rise is the buffers of deeper levels of faces being reserved as those faces first appear.

**Not run**, because one build takes from ten minutes to more than an hour: the hull of the corners of a 10-cube; a k-gon times a cube in D = 9 and 10; clusters on a sphere in D = 8; Delaunay of the full grid in D = 7.

**The first call of `planes()` and builds that end in an error** were counted afterwards (#567), on `main` with #566, release:

- `planes()`, apart from the build, on `cube`, `sphere`, `cubesurf`, `cluster`, and `grid` in D = 2 to 5 at 500 and 8,000 points: 5 or 6 allocations, for 4 facets and for 238,164.
- A build that fails, for each builder, in D = 2 to 5 at 200 and 3,200 points: 5 to 17 allocations for points in a hyperplane or on a line (`DegenerateDimension`), 5 to 10 for one point repeated (`InsufficientPoints`), 2 for a NaN in the last point (`NonFiniteCoordinate`); the same at both sizes.

`tests/alloc.rs` holds both to the ratchet and to the growth check from #567 on: six cases of `planes()` and seven failing builds.

**Seen on the way, and not a growth per item:**

- A Delaunay build allocates a fixed number of times that rises with the dimension, whatever the sites: 1,591 in D = 5, 2,840 in D = 6, 4,670 in D = 7, 7,354 in D = 8, 11,066 in D = 9. The spike #562 traced it (heap profiles by call site at D = 3, 5, 7, and 9): 73% to 99.6% of a build's blocks were in `Mesh::link_initial`, which built a sorted list for both faces of each of its (D + 2)(D + 1)^3 comparisons. With each face sorted once (#564), base `main` at `fcd029c`:

  | Delaunay `cube` | Before | After |
  | --- | ---: | ---: |
  | D = 2 | 154 | 47 |
  | D = 3 | 376 | 57 |
  | D = 4 | 806 | 57 |
  | D = 8 | 7,354 | 65 |
  | D = 9 | 11,066 | 67 |

  Over the 53 cases of `tests/alloc.rs` the allocations at the large size fall from 31,870 to 7,935 in sum.
- The hull of the 512 corners of a 9-cube took 406 s, and of 256 corners in D = 8 10 s. The spike #563 counted and profiled it: the sub-hulls built are the distinct faces of the cube, each once (in D = 8: 1,792 of dimension 3, 1,120 of 4, 448 of 5, 112 of 6, 16 of 7, which is C(k, j) 2^(k - j)), and every other request is answered by the memo. The time is predicates on points that are as degenerate as points get. Of the 10.6 s of D = 8, the sub-hulls and their classification are 44.8% (the planes of their new simplices about 32%), the placing triangulation on the surface 16.5%, the placing triangulation before a facet's complex has its dimension 15.9% (#565), and the construction of the top hull 12.6%.

### Reading

Each of the four steps was quadratic or cubic in the points of one face or group, and is linear on these inputs in the given order. Shuffled, the sphere grows a little faster than linearly under the search of #547 (2.8 times the time for 2.3 times the sites). The sub-hull of each face then stopped allocating for each face (#531), which took another tenth off the k-gon times a square.

## A parity run after the allocation rows, and hull `sphere` D2 slower since #549, PR #572 (#569)

Between the final P7 run and this one, the rows of the allocation contract merged (#527 to #568). Each was timed on its own inputs and guards, and none was run against the references. This run asks where the sets stand now.

### Method

That of the final P7 run (`docs/verification.md`, "Parity run of P7"): `main` at `5846643`, under WSL2, against Qhull 2020.2 and CGAL 5.4 on the same machine, `rustc` 1.99.0, `g++` 11.4.0; the judged `build()` from a copy without the phase marks. The runner, the CGAL program (#456), and the marks were rebuilt from the Issues, since the scratch of the final run had left the machine.

Two things about this run:

- **I built a copy of the crate on the machine while the run was in its sixth set.** The three sets of hull `cube` D3 were run again after the others. Their first medians were 1.29 ms, 11.4 ms, and 142.5 ms, and the second 1.21 ms, 10.4 ms, and 142.8 ms; the table has the second.
- **The run is noisier than the final one.** The slowest build of a set is 1.2 to 2.6 times its fastest on 22 of the 41 sets, and the references moved too: on the Delaunay sets of D = 3 to 5, CGAL read 1.01 to 1.26 of its time in the final run and Qhull 1.00 to 1.60. A ratio that moved by a few hundredths says nothing by itself, so the two commits were also timed against each other directly (below).

### Judgement

| Threshold | Met of the 38 judged sets | Hull | Delaunay | In the final P7 run |
| --- | ---: | ---: | ---: | ---: |
| 1.05 | 33 | 22 | 11 | 34 |
| 1.00 | 26 | 19 | 7 | 29 |

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.47 ms | 0.75 | 0.91 | 0.83 | met | met |
| Hull `cube` D2 10^5 | 4.39 ms | 0.76 | 0.95 | 0.73 | met | met |
| Hull `cube` D2 10^6 | 54.8 ms | 0.80 | 1.21 | 0.93 | met | met |
| Hull `cube` D3 10^4 | 1.21 ms | 0.55 | – | 0.74 | met | met |
| Hull `cube` D3 10^5 | 10.4 ms | 0.40 | – | 0.86 | met | met |
| Hull `cube` D3 10^6 | 143 ms | 0.23 | – | 0.93 | met | met |
| Hull `cube` D4 10^4 | 5.24 ms | 0.12 | – | 0.79 | met | met |
| Hull `cube` D4 10^5 | 32.1 ms | 0.06 | – | 0.94 | met | met |
| Hull `cube` D5 10^4 | 57.4 ms | 0.26 | – | 0.78 | met | met |
| Hull `cube` D5 10^5 | 210 ms | 0.13 | – | 0.67 | met | met |
| Hull `cube` D6 10^4 | 573 ms | 0.28 | – | 0.47 | met | met |
| Hull `cube` D6 10^5 | 2.52 s | 0.23 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.90 ms | 1.22 | 1.33 | 0.13 (ref.) | not met | not met |
| Hull `sphere` D2 10^5 | 10.5 ms | 1.10 | 1.43 | 0.10 (ref.) | not met | not met |
| Hull `sphere` D2 10^6 | 114 ms | 0.74 | 1.31 | 0.06 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 20.9 ms | 0.85 | – | 1.05 | not met | not met |
| Hull `sphere` D3 10^5 | 262 ms | 0.61 | – | 1.01 | met | not met |
| Hull `sphere` D3 10^6 | 3.44 s | 0.41 | – | 1.01 | met | not met |
| Hull `sphere` D4 10^4 | 90.4 ms | 0.73 | – | 0.93 | met | met |
| Hull `sphere` D4 10^5 | 1.21 s | 0.95 | – | 0.90 | met | met |
| Hull `sphere` D5 10^4 | 813 ms | 0.84 | – | 0.98 | met | met |
| Hull `sphere` D5 10^5 | 11.21 s | 1.00 | – | 0.99 | met | not met |
| Hull `sphere` D6 10^4 | 6.87 s | 0.71 | – | 0.80 | met | met |
| Hull `cubesurf` D3 10^5 | 29.7 ms | 0.74 | – | 0.98 | met | met |
| Hull `grid` D6 10^4 | 229 ms | 0.89 (ref.) | – | 0.65 | met | met |
| Delaunay `cube` D2 10^4 | 5.30 ms | 1.13 | 1.27 | 0.29 | not met | not met |
| Delaunay `cube` D2 10^5 | 55.7 ms | 1.02 | 1.32 | 0.23 | met | not met |
| Delaunay `cube` D2 10^6 | 565 ms | 0.82 | 1.26 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 4.98 ms | 1.13 | 1.28 | 0.22 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 43.8 ms | 1.03 | 1.32 | 0.12 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 422 ms | 0.83 | 1.36 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 35.4 ms | 1.08 | 1.17 | 0.38 | reported | reported |
| Delaunay `cube` D3 10^5 | 385 ms | 1.12 | 1.22 | 0.30 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.20 s | 1.07 | 1.20 | 0.26 | reported | reported |
| Delaunay `sphere` D3 10^4 | 72.2 ms | 0.52 | 0.51 | 0.55 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 667 ms | 0.63 | 0.65 | 0.34 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 7.04 s | 0.84 | 0.89 | 0.28 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 802 ms | 1.03 | 1.08 | 0.73 | met | not met |
| Delaunay `sphere` D4 10^4 | 1.02 s | 0.33 | 0.33 | 0.51 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 9.57 s | 1.04 | 1.10 | 1.00 | met | not met |
| Delaunay `sphere` D5 10^4 | 12.00 s | 0.33 | 0.33 | 0.80 (ref.) | met | met |

**Delaunay `cube` D3, reported:** convx's construction alone reads 0.91, 0.94, and 0.91 of CGAL's whole time at 10^4, 10^5, and 10^6.

### Against the final P7 run

The judged ratio of each set in the two runs, and this run's median over the final run's for convx's `build()` and for each reference. The final run's times are read from its table, so the last three columns carry its rounding.

| Set | Final P7 run | This run | convx `build()` | CGAL | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.79 | 0.83 | 0.93 | 0.94 | 0.89 |
| Hull `cube` D2 10^5 | 0.80 | 0.76 | 0.90 | 0.94 | 0.99 |
| Hull `cube` D2 10^6 | 1.10 | 0.93 | 0.88 | 1.02 | 1.04 |
| Hull `cube` D3 10^4 | 0.81 | 0.74 | 1.01 | 1.03 | 1.10 |
| Hull `cube` D3 10^5 | 0.83 | 0.86 | 1.09 | 1.04 | 1.04 |
| Hull `cube` D3 10^6 | 0.99 | 0.93 | 1.05 | 1.14 | 1.12 |
| Hull `cube` D4 10^4 | 0.77 | 0.79 | 1.03 | 1.04 | 1.01 |
| Hull `cube` D4 10^5 | 0.93 | 0.94 | 1.05 | 1.09 | 1.04 |
| Hull `cube` D5 10^4 | 0.73 | 0.78 | 1.11 | 1.04 | 1.03 |
| Hull `cube` D5 10^5 | 0.67 | 0.67 | 1.03 | 1.05 | 1.02 |
| Hull `cube` D6 10^4 | 0.46 | 0.47 | 1.02 | 1.02 | 1.01 |
| Hull `cube` D6 10^5 | 0.36 | 0.39 | 0.98 | 0.94 | 0.91 |
| Hull `sphere` D2 10^4 | 1.01 | 1.22 | 1.11 | 0.92 | 0.91 |
| Hull `sphere` D2 10^5 | 0.99 | 1.10 | 1.08 | 0.98 | 0.91 |
| Hull `sphere` D2 10^6 | 0.69 | 0.74 | 1.01 | 0.94 | 0.91 |
| Hull `sphere` D3 10^4 | 0.99 | 1.05 | 1.02 | 1.01 | 0.95 |
| Hull `sphere` D3 10^5 | 0.99 | 1.01 | 1.01 | 0.98 | 0.99 |
| Hull `sphere` D3 10^6 | 1.00 | 1.01 | 0.98 | 1.02 | 0.97 |
| Hull `sphere` D4 10^4 | 0.92 | 0.93 | 1.04 | 1.02 | 1.03 |
| Hull `sphere` D4 10^5 | 0.89 | 0.95 | 1.08 | 1.00 | 1.02 |
| Hull `sphere` D5 10^4 | 0.96 | 0.98 | 1.06 | 1.01 | 1.05 |
| Hull `sphere` D5 10^5 | 0.97 | 1.00 | 1.12 | 1.01 | 1.10 |
| Hull `sphere` D6 10^4 | 0.81 | 0.80 | 1.07 | 1.03 | 1.08 |
| Hull `cubesurf` D3 10^5 | 0.91 | 0.98 | 1.05 | 1.00 | 0.98 |
| Hull `grid` D6 10^4 | 0.70 | 0.65 | 0.97 | 1.03 | 1.04 |
| Delaunay `cube` D2 10^4 | 1.09 | 1.13 | 1.07 | 1.03 | 1.05 |
| Delaunay `cube` D2 10^5 | 1.01 | 1.02 | 1.08 | 1.07 | 1.06 |
| Delaunay `cube` D2 10^6 | 0.78 | 0.82 | 1.08 | 1.03 | 1.08 |
| Delaunay `sphere` D2 10^4 | 1.11 | 1.13 | 1.04 | 1.02 | 1.00 |
| Delaunay `sphere` D2 10^5 | 1.05 | 1.03 | 1.04 | 1.05 | 1.06 |
| Delaunay `sphere` D2 10^6 | 0.82 | 0.83 | 1.05 | 1.04 | 0.91 |
| Delaunay `cube` D3 10^4 | 1.12 | 1.08 | 1.01 | 1.05 | 1.03 |
| Delaunay `cube` D3 10^5 | 1.12 | 1.12 | 1.02 | 1.02 | 1.05 |
| Delaunay `cube` D3 10^6 | 1.09 | 1.07 | 1.02 | 1.04 | 1.05 |
| Delaunay `sphere` D3 10^4 | 0.51 | 0.52 | 1.10 | 1.08 | 1.21 |
| Delaunay `sphere` D3 10^5 | 0.67 | 0.63 | 1.06 | 1.12 | 1.06 |
| Delaunay `sphere` D3 10^6 | 0.81 | 0.84 | 1.28 | 1.24 | 1.44 |
| Delaunay `cube` D4 10^4 | 1.06 | 1.03 | 1.15 | 1.18 | 1.38 |
| Delaunay `sphere` D4 10^4 | 0.34 | 0.33 | 1.18 | 1.20 | 1.60 |
| Delaunay `cube` D5 10^4 | 1.01 | 1.04 | 1.30 | 1.26 | 1.10 |
| Delaunay `sphere` D5 10^4 | 0.32 | 0.33 | 1.03 | 1.01 | 1.00 |

### The two commits timed directly

`b6934d5` and `5846643`, plain builds with debug information, under WSL, pinned and alternated, 3 to 8 rounds of 1 to 20 builds:

| Set | `b6934d5` (minimum, median) | `5846643` (minimum, median) | Ratio of the minimum | Of the median |
| --- | ---: | ---: | ---: | ---: |
| Hull `sphere` D2 10^4 | 0.695, 0.787 ms | 0.758, 0.846 ms | 1.09 | 1.08 |
| Hull `sphere` D2 10^5 | 8.53, 10.04 ms | 9.61, 10.87 ms | 1.13 | 1.08 |
| Hull `sphere` D2 10^6 | 100.9, 111.0 ms | 111.1, 116.2 ms | 1.10 | 1.05 |
| Hull `cube` D2 10^6 | 58.5, 63.2 ms | 52.6, 58.0 ms | 0.90 | 0.92 |
| Hull `sphere` D3 10^4 | 18.08, 20.46 ms | 18.59, 20.78 ms | 1.03 | 1.02 |
| Hull `cube` D3 10^5 | 9.37, 10.97 ms | 9.48, 10.79 ms | 1.01 | 0.98 |
| Hull `cubesurf` D3 10^5 | 26.36, 29.48 ms | 25.93, 29.60 ms | 0.98 | 1.00 |
| Hull `cube` D5 10^4 | 53.2, 56.4 ms | 52.6, 56.9 ms | 0.99 | 1.01 |
| Hull `sphere` D4 10^5 | 1,167, 1,185 ms | 1,203, 1,205 ms | 1.03 | 1.02 |
| Hull `sphere` D5 10^4 | 841, 847 ms | 839, 863 ms | 1.00 | 1.02 |
| Delaunay `cube` D2 10^4 | 4.77, 5.30 ms | 5.00, 5.42 ms | 1.05 | 1.02 |
| Delaunay `sphere` D3 10^4 | 70.2, 72.2 ms | 68.6, 71.5 ms | 0.98 | 0.99 |
| Delaunay `cube` D4 10^4 | 710, 724 ms | 719, 725 ms | 1.01 | 1.00 |
| Delaunay `cube` D5 10^4 | 7,502, 7,577 ms | 7,318, 7,355 ms | 0.98 | 0.97 |

### Hull `sphere` D2 is 8 to 10% slower, since #549

It is the one set family that moved against convx beyond the spread, and two of its sizes left 1.05: 1.22 against CGAL at 10^4, from 1.01, and 1.10 at 10^5, from 0.99.

Every commit between the two runs that touches `src`, built without debug information, hull `sphere` D2 10^5 and hull `cube` D2 10^6, 8 alternated rounds of 5 and of 2 builds, pinned:

| Commit | `sphere` D2 10^5 (minimum, median) | `cube` D2 10^6 (minimum, median) |
| --- | ---: | ---: |
| `b6934d5` | 8.04, 9.22 ms | 49.7, 54.0 ms |
| `9c824dc` (#528) | 8.71, 9.42 ms | 52.2, 56.8 ms |
| `384b73f` (#532) | 7.96, 9.06 ms | 50.7, 53.1 ms |
| `356c8ee` (#535) | 8.56, 9.24 ms | 51.0, 54.4 ms |
| `accd8ce` (#547) | 8.20, 9.24 ms | 51.2, 54.0 ms |
| `6713f61` (#549) | 9.29, 10.16 ms | 49.2, 52.5 ms |
| `9feb3db` (#552) | 9.06, 9.81 ms | 53.0, 55.1 ms |
| `94d9d5c` (#553) | 9.29, 10.14 ms | 50.8, 55.5 ms |
| `c7180d5` (#555) | 8.88, 10.02 ms | 51.7, 53.7 ms |
| `f1893ad` (#556) | 8.88, 10.09 ms | 50.0, 55.1 ms |
| `5846643` | 9.24, 10.10 ms | 50.3, 54.2 ms |

`sphere` D2 steps at `6713f61`, the sub-hull of a face in the storage of the one before (PR #549), and stays there. That pull request timed hull `sphere` D3 and the `cube` sets as guards and no set of D = 2. Which of its changes costs the time has not been profiled; it is #570.

**Hull `cube` D2 10^6 did not get faster.** It reads 0.93 against Qhull, from 1.10, and 0.90 between the two commits in the builds with debug information. In the builds without, the same two commits read 50.9 and 54.1 ms, 1.06 the other way, and the commits between them show no step. The difference follows the build and not the code. The set is met in this run and was not in the final one, on that.

### What is not met

| Set | Ratio | Reason |
| --- | --- | --- |
| Hull `sphere` D2 10^4 | 1.22 against CGAL | slower since #549 (#570); 1.01 in the final run |
| Hull `sphere` D2 10^5 | 1.10 against CGAL | the same; 0.99 in the final run |
| Hull `sphere` D3 10^4 | 1.055 against Qhull | Qhull read 0.95 of its time in the final run and convx 1.02; directly, the two commits read 1.03 by the minimum and 1.02 by the median, inside the spread of that table; 0.99 in the final run |
| Delaunay `cube` D2 10^4 | 1.13 against CGAL | 1.04 to 1.13 over the last five runs, with no change to its code |
| Delaunay `sphere` D2 10^4 | 1.13 against CGAL | 1.04 to 1.13 over four runs in a row (#512) |

Delaunay `cube` D4 10^4 reads 1.03, from 1.06, and is met; directly its two commits read 1.01 and 1.00.

### Reading

Of the rows since the final run, one changed a parity set beyond the spread: #549, on hull `sphere` D2. No other set moved against convx in the direct timing, where the largest other ratios are 1.03 (hull `sphere` D3 10^4 and D4 10^5) and 1.05 by the minimum with 1.02 by the median (Delaunay `cube` D2 10^4). The guards of a pull request did not cover the path it changed: `build()` reaches the D = 2 hull through code that #549 touched, and a guard of D = 2 was not in its list.

## Hull `sphere` D2: the first start of a list out of the push, PR #573 (#570)

The parity run of #569 found hull `sphere` D2 8 to 10% slower since PR #549, under WSL.

### The cause

The phases of one `build()` of hull `sphere` D2, summed by timers around them in scratch copies, under WSL, pinned and alternated, the median of 5 runs of 40 builds (10^5) and of 6 builds (10^6):

| Points | Commit | `build()` | Acceptance | Construction | Classification | Publication |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 10^5 | `accd8ce`, before #549 | 10.05 ms | 5.14 ms | 1.71 ms | 1.27 ms | 2.04 ms |
| 10^5 | `5846643`, `main` | 10.82 ms | 5.08 ms | 1.77 ms | 2.15 ms | 2.05 ms |
| 10^5 | `5229af2`, this change | 10.17 ms | 5.14 ms | 1.75 ms | 1.34 ms | 2.10 ms |
| 10^6 | `accd8ce` | 108.6 ms | 61.1 ms | 20.6 ms | 11.5 ms | 16.6 ms |
| 10^6 | `5846643` | 117.5 ms | 59.3 ms | 21.4 ms | 20.0 ms | 17.1 ms |
| 10^6 | `5229af2` | 108.6 ms | 58.5 ms | 21.6 ms | 12.0 ms | 16.3 ms |

Only classification moved, and the code of the classification of a polygon did not change in #549. It writes the edges and the simplices of the cycle into flat lists, one push per edge. #549 made a default `Lists` allocate nothing, and for that every push tested whether the first start was written and wrote it when it was not. That write, in the body of the push, cost 9 ns a point on a hull of D = 2 whose points are all extreme. It is now a cold function out of line, and the push keeps only the test.

### Time

Plain builds, pinned and alternated, minimum and median in ms; the ratios are against the first column.

Under WSL, 5 to 8 rounds:

| Set | `accd8ce`, before #549 | `5846643`, `main` | `5229af2`, this change |
| --- | ---: | ---: | ---: |
| Hull `sphere` D2 10^4 | 0.740, 0.781 | 0.830, 0.879 (1.12, 1.13) | 0.689, 0.791 (0.93, 1.01) |
| Hull `sphere` D2 10^5 | 8.33, 9.99 | 9.50, 10.76 (1.14, 1.08) | 8.79, 10.17 (1.06, 1.02) |
| Hull `sphere` D2 10^6 | 99.2, 107.5 | 105.6, 114.8 (1.07, 1.07) | 103.4, 112.0 (1.04, 1.04) |
| Hull `cube` D2 10^5 | 3.97, 4.49 | 4.02, 4.40 (1.01, 0.98) | 3.91, 4.38 (0.98, 0.98) |
| Hull `cube` D2 10^6 | 53.8, 56.1 | 53.9, 55.6 (1.00, 0.99) | 52.1, 55.6 (0.97, 0.99) |
| Hull `sphere` D3 10^5 | 256.9, 263.7 | 251.4, 266.2 (0.98, 1.01) | 249.1, 266.9 (0.97, 1.01) |
| Hull `cube` D3 10^6 | 133.3, 142.0 | 132.8, 142.8 (1.00, 1.01) | 132.3, 145.4 (0.99, 1.02) |
| Hull `cubesurf` D3 10^5 | 26.9, 28.5 | 27.6, 29.6 (1.03, 1.04) | 26.1, 29.4 (0.97, 1.03) |
| Hull `cube` D4 10^5 | 32.3, 34.2 | 31.7, 34.6 (0.98, 1.01) | 31.9, 34.4 (0.99, 1.01) |
| Hull `sphere` D4 10^4 | 89.9, 92.7 | 91.9, 95.8 (1.02, 1.03) | 89.8, 95.4 (1.00, 1.03) |
| Hull `grid` D6 10^4 | 237.1, 239.2 | 231.1, 236.5 (0.97, 0.99) | 232.2, 236.5 (0.98, 0.99) |
| Delaunay `cube` D2 10^5 | 53.6, 55.9 | 53.7, 57.5 (1.00, 1.03) | 53.8, 56.9 (1.00, 1.02) |
| Delaunay `sphere` D2 10^5 | 40.8, 44.0 | 43.7, 46.6 (1.07, 1.06) | 42.1, 45.0 (1.03, 1.02) |
| Delaunay `cube` D3 10^5 | 388.4, 396.3 | 397.2, 401.0 (1.02, 1.01) | 391.0, 399.2 (1.01, 1.01) |
| Delaunay of 20,000 `lattice` points, D3 | 2.45, 2.75 | 2.46, 2.62 (1.00, 0.95) | 2.46, 2.64 (1.00, 0.96) |

Hull `sphere` D2 at 10^5 and 10^6 still reads 1.02 to 1.06 of `accd8ce` in this table. In the table of the phases, taken after it, the same three builds read 10.05, 10.82, and 10.17 ms and 108.6, 117.5, and 108.6 ms, with classification at 1.04 to 1.05 of `accd8ce`. The test that stays in the push may be what is left; it was not separated. Removing it would put a branch in every read of a list instead.

Delaunay `sphere` D2 10^5 read 1.07 and 1.06 on `main` and 1.03 and 1.02 with the change: it writes its cells through the same lists. The parity run did not show it.

On Windows, `main` against the change, 4 to 12 rounds:

| Set | `5846643`, `main` | `5229af2`, this change |
| --- | ---: | ---: |
| Hull `sphere` D2 10^4 | 0.980, 1.058 | 0.982, 1.059 (1.00, 1.00) |
| Hull `sphere` D2 10^5 | 9.20, 10.29 | 9.21, 10.14 (1.00, 0.99) |
| Hull `sphere` D2 10^6 | 105.7, 113.8 | 108.3, 112.8 (1.02, 0.99) |
| Hull `cube` D2 10^6 | 56.2, 59.6 | 56.6, 59.9 (1.01, 1.00) |
| Hull `sphere` D3 10^5 | 253.9, 270.5 | 252.7, 270.6 (1.00, 1.00) |
| Hull `cube` D3 10^6 | 140.6, 146.3 | 134.9, 146.1 (0.96, 1.00) |
| Hull `cubesurf` D3 10^5 | 28.7, 32.0 | 28.7, 30.6 (1.00, 0.96) |
| Hull `cube` D4 10^5 | 32.6, 35.9 | 33.0, 35.6 (1.01, 0.99) |
| Hull `grid` D6 10^4 | 244.7, 251.5 | 246.7, 254.2 (1.01, 1.01) |
| Delaunay `cube` D2 10^5 | 54.1, 55.5 | 54.3, 55.8 (1.00, 1.01) |
| Delaunay `sphere` D2 10^5 | 44.4, 47.7 | 45.0, 47.7 (1.02, 1.00) |
| Delaunay `cube` D3 10^5 | 396.3, 413.3 | 392.2, 411.8 (0.99, 1.00) |

**The regression does not show on Windows.** The commit before #549, #549, and `main` read 0.83 to 0.94 ms at 10^4 and 8.9 to 9.8 ms at 10^5 there, in no order, in three alternated rounds. #549 was timed on Windows only, and its guards had no set of D = 2; either would have missed it, and the two together did. `bench.mdc` now asks both of a change to a container every builder uses.

The inputs of #549 (the k-gon times a square, the prism) were not timed again: the change touches no path of theirs but the push of a list.

## The parity run again, after #573, PR #579 (#577)

The run of #569 found hull `sphere` D2 slower and was noisy. #573 removed that cause. This run judges the sets again.

### Method

That of the run of #569: `main` at `ce01e84`, under WSL2, Qhull 2020.2, CGAL 5.4, `rustc` 1.99.0, `g++` 11.4.0, the judged `build()` from the copy without the phase marks. Nothing of mine ran on the machine during the run.

**Four sets in a row were disturbed all the same,** from hull `sphere` D3 10^6 to hull `sphere` D5 10^4: the slowest build of each was 1.5 to 2.2 times its fastest, and the references moved as much. Those four and two sets at the edge of 1.05 were run again right after the run. The tables below have the second reading. Both:

| Set | First: `build()` (fastest to slowest), / CGAL, / Qhull | Again |
| --- | --- | --- |
| Hull `sphere` D2 10^4 | 0.80 ms (0.70 to 0.91), 1.06, 0.12 | 0.85 ms (0.76 to 1.08), 1.02, 0.11 |
| Hull `sphere` D3 10^4 | 20.9 ms (19.2 to 22.7), 0.87, 1.06 | 22.5 ms (21.3 to 34.7), 0.88, 1.09 |
| Hull `sphere` D3 10^6 | 5.46 s (3.38 to 5.64), 0.45, 1.21 | 3.48 s (3.41 to 3.53), 0.44, 1.00 |
| Hull `sphere` D4 10^4 | 106 ms (93 to 200), 0.74, 0.82 | 93.1 ms (87.8 to 98.4), 0.75, 0.94 |
| Hull `sphere` D4 10^5 | 1.43 s (1.27 to 2.10), 0.91, 0.74 | 1.18 s (1.14 to 1.23), 0.94, 0.88 |
| Hull `sphere` D5 10^4 | 925 ms (817 to 1,240), 0.89, 1.03 | 828 ms (796 to 877), 0.82, 1.01 |

With the first readings 33 sets are met at 1.05; with the second, 35. Of the 41 sets, 18 still have a slowest build 1.2 times the fastest or more (22 in the run of #569), the largest 1.71.

### Judgement

| Threshold | Met of the 38 judged sets | Hull | Delaunay | Run of #569 | Final P7 run |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1.05 | 35 | 24 | 11 | 33 | 34 |
| 1.00 | 29 | 21 | 8 | 26 | 29 |

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.47 ms | 0.74 | 0.89 | 0.77 | met | met |
| Hull `cube` D2 10^5 | 4.41 ms | 0.72 | 0.93 | 0.73 | met | met |
| Hull `cube` D2 10^6 | 56.4 ms | 0.79 | 1.21 | 0.97 | met | met |
| Hull `cube` D3 10^4 | 1.23 ms | 0.52 | – | 0.77 | met | met |
| Hull `cube` D3 10^5 | 10.3 ms | 0.38 | – | 0.82 | met | met |
| Hull `cube` D3 10^6 | 141 ms | 0.23 | – | 0.94 | met | met |
| Hull `cube` D4 10^4 | 5.56 ms | 0.12 | – | 0.78 | met | met |
| Hull `cube` D4 10^5 | 34.4 ms | 0.06 | – | 0.95 | met | met |
| Hull `cube` D5 10^4 | 56.8 ms | 0.25 | – | 0.75 | met | met |
| Hull `cube` D5 10^5 | 221 ms | 0.12 | – | 0.65 | met | met |
| Hull `cube` D6 10^4 | 600 ms | 0.28 | – | 0.46 | met | met |
| Hull `cube` D6 10^5 | 2.63 s | 0.23 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.85 ms | 1.02 | 1.13 | 0.11 (ref.) | met | not met |
| Hull `sphere` D2 10^5 | 9.28 ms | 0.94 | 1.19 | 0.08 (ref.) | met | met |
| Hull `sphere` D2 10^6 | 106 ms | 0.67 | 1.20 | 0.05 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 22.5 ms | 0.88 | – | 1.09 | not met | not met |
| Hull `sphere` D3 10^5 | 264 ms | 0.62 | – | 0.99 | met | met |
| Hull `sphere` D3 10^6 | 3.48 s | 0.44 | – | 1.00 | met | not met |
| Hull `sphere` D4 10^4 | 93.1 ms | 0.75 | – | 0.94 | met | met |
| Hull `sphere` D4 10^5 | 1.18 s | 0.94 | – | 0.88 | met | met |
| Hull `sphere` D5 10^4 | 828 ms | 0.82 | – | 1.01 | met | not met |
| Hull `sphere` D5 10^5 | 10.45 s | 0.92 | – | 0.95 | met | met |
| Hull `sphere` D6 10^4 | 6.81 s | 0.70 | – | 0.79 | met | met |
| Hull `cubesurf` D3 10^5 | 28.1 ms | 0.74 | – | 0.97 | met | met |
| Hull `grid` D6 10^4 | 223 ms | 0.87 (ref.) | – | 0.64 | met | met |
| Delaunay `cube` D2 10^4 | 5.07 ms | 1.07 | 1.27 | 0.27 | not met | not met |
| Delaunay `cube` D2 10^5 | 53.8 ms | 1.04 | 1.32 | 0.23 | met | not met |
| Delaunay `cube` D2 10^6 | 554 ms | 0.81 | 1.23 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 5.23 ms | 1.17 | 1.34 | 0.22 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 44.8 ms | 1.00 | 1.31 | 0.11 (ref.) | met | met |
| Delaunay `sphere` D2 10^6 | 421 ms | 0.83 | 1.36 | 0.04 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 34.6 ms | 1.09 | 1.15 | 0.38 | reported | reported |
| Delaunay `cube` D3 10^5 | 384 ms | 1.13 | 1.23 | 0.31 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.06 s | 1.04 | 1.17 | 0.26 | reported | reported |
| Delaunay `sphere` D3 10^4 | 67.5 ms | 0.49 | 0.49 | 0.54 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 637 ms | 0.67 | 0.69 | 0.34 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.59 s | 0.82 | 0.85 | 0.33 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 679 ms | 1.05 | 1.11 | 0.86 | met | not met |
| Delaunay `sphere` D4 10^4 | 867 ms | 0.35 | 0.35 | 0.67 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.25 s | 1.03 | 1.06 | 0.91 | met | not met |
| Delaunay `sphere` D5 10^4 | 11.41 s | 0.32 | 0.32 | 0.76 (ref.) | met | met |

**Delaunay `cube` D3, reported:** convx's construction alone reads 0.93, 0.92, and 0.86 of CGAL's whole time at 10^4, 10^5, and 10^6.

### Against the two runs before

The judged ratio of each set in the three runs, and this run's median over that of the run of #569 for convx's `build()` and for each reference (read from that run's table, so with its rounding).

| Set | Final P7 run | Run of #569 | This run | convx `build()` against #569 | CGAL | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Hull `cube` D2 10^4 | 0.79 | 0.83 | 0.77 | 1.00 | 1.01 | 1.09 |
| Hull `cube` D2 10^5 | 0.80 | 0.76 | 0.73 | 1.00 | 1.05 | 1.01 |
| Hull `cube` D2 10^6 | 1.10 | 0.93 | 0.97 | 1.03 | 1.04 | 0.99 |
| Hull `cube` D3 10^4 | 0.81 | 0.74 | 0.77 | 1.02 | 1.08 | 0.98 |
| Hull `cube` D3 10^5 | 0.83 | 0.86 | 0.82 | 0.99 | 1.03 | 1.03 |
| Hull `cube` D3 10^6 | 0.99 | 0.93 | 0.94 | 0.99 | 0.97 | 0.98 |
| Hull `cube` D4 10^4 | 0.77 | 0.79 | 0.78 | 1.06 | 1.05 | 1.08 |
| Hull `cube` D4 10^5 | 0.93 | 0.94 | 0.95 | 1.07 | 1.09 | 1.06 |
| Hull `cube` D5 10^4 | 0.73 | 0.78 | 0.75 | 0.99 | 1.05 | 1.03 |
| Hull `cube` D5 10^5 | 0.67 | 0.67 | 0.65 | 1.05 | 1.10 | 1.08 |
| Hull `cube` D6 10^4 | 0.46 | 0.47 | 0.46 | 1.05 | 1.05 | 1.06 |
| Hull `cube` D6 10^5 | 0.36 | 0.39 | 0.39 | 1.04 | 1.02 | 1.03 |
| Hull `sphere` D2 10^4 | 1.01 | 1.22 | 1.02 | 0.94 | 1.13 | 1.08 |
| Hull `sphere` D2 10^5 | 0.99 | 1.10 | 0.94 | 0.88 | 1.03 | 1.07 |
| Hull `sphere` D2 10^6 | 0.69 | 0.74 | 0.67 | 0.93 | 1.03 | 1.02 |
| Hull `sphere` D3 10^4 | 0.99 | 1.05 | 1.09 | 1.07 | 1.03 | 1.03 |
| Hull `sphere` D3 10^5 | 0.99 | 1.01 | 0.99 | 1.01 | 1.00 | 1.03 |
| Hull `sphere` D3 10^6 | 1.00 | 1.01 | 1.00 | 1.01 | 0.94 | 1.02 |
| Hull `sphere` D4 10^4 | 0.92 | 0.93 | 0.94 | 1.03 | 1.00 | 1.01 |
| Hull `sphere` D4 10^5 | 0.89 | 0.95 | 0.94 | 0.97 | 0.99 | 1.00 |
| Hull `sphere` D5 10^4 | 0.96 | 0.98 | 1.01 | 1.02 | 1.04 | 0.99 |
| Hull `sphere` D5 10^5 | 0.97 | 1.00 | 0.95 | 0.93 | 1.01 | 0.97 |
| Hull `sphere` D6 10^4 | 0.81 | 0.80 | 0.79 | 0.99 | 1.01 | 1.00 |
| Hull `cubesurf` D3 10^5 | 0.91 | 0.98 | 0.97 | 0.95 | 0.94 | 0.95 |
| Hull `grid` D6 10^4 | 0.70 | 0.65 | 0.64 | 0.98 | 1.00 | 0.99 |
| Delaunay `cube` D2 10^4 | 1.09 | 1.13 | 1.07 | 0.96 | 1.01 | 1.01 |
| Delaunay `cube` D2 10^5 | 1.01 | 1.02 | 1.04 | 0.97 | 0.95 | 0.95 |
| Delaunay `cube` D2 10^6 | 0.78 | 0.82 | 0.81 | 0.98 | 0.99 | 1.00 |
| Delaunay `sphere` D2 10^4 | 1.11 | 1.13 | 1.17 | 1.05 | 1.01 | 1.04 |
| Delaunay `sphere` D2 10^5 | 1.05 | 1.03 | 1.00 | 1.02 | 1.06 | 1.09 |
| Delaunay `sphere` D2 10^6 | 0.82 | 0.83 | 0.83 | 1.00 | 1.00 | 0.85 |
| Delaunay `cube` D3 10^4 | 1.12 | 1.08 | 1.09 | 0.98 | 0.97 | 0.97 |
| Delaunay `cube` D3 10^5 | 1.12 | 1.12 | 1.13 | 1.00 | 0.99 | 0.95 |
| Delaunay `cube` D3 10^6 | 1.09 | 1.07 | 1.04 | 0.97 | 1.00 | 0.97 |
| Delaunay `sphere` D3 10^4 | 0.51 | 0.52 | 0.49 | 0.93 | 0.99 | 0.95 |
| Delaunay `sphere` D3 10^5 | 0.67 | 0.63 | 0.67 | 0.95 | 0.89 | 0.97 |
| Delaunay `sphere` D3 10^6 | 0.81 | 0.84 | 0.82 | 0.79 | 0.81 | 0.67 |
| Delaunay `cube` D4 10^4 | 1.06 | 1.03 | 1.05 | 0.85 | 0.83 | 0.72 |
| Delaunay `sphere` D4 10^4 | 0.34 | 0.33 | 0.35 | 0.85 | 0.81 | 0.64 |
| Delaunay `cube` D5 10^4 | 1.01 | 1.04 | 1.03 | 0.76 | 0.77 | 0.83 |
| Delaunay `sphere` D5 10^4 | 0.32 | 0.33 | 0.32 | 0.95 | 0.98 | 1.00 |

### Hull `sphere` D2 is back

It reads 1.02 at 10^4 (1.06 in the first reading), 0.94 at 10^5, and 0.67 at 10^6, from 1.22, 1.10, and 0.74 in the run of #569. Its `build()` reads 0.88 to 0.94 of that run where the references read 0.96 to 1.13. The final P7 run's commit against `main`, directly, 10 alternated rounds of 20 builds at 10^4: 0.720 and 0.683 ms by the minimum (0.95), 0.797 and 0.782 ms by the median (0.98).

### What is not met

| Set | Ratio | Reason |
| --- | --- | --- |
| Hull `sphere` D3 10^4 | 1.06 and 1.09 against Qhull | `main` builds it 3.5% slower than the final P7 run's commit, which equalled Qhull (below, #578) |
| Delaunay `cube` D2 10^4 | 1.07 against CGAL | 1.04 to 1.13 over the last six runs, with no step between the commits (below) |
| Delaunay `sphere` D2 10^4 | 1.17 against CGAL | 1.04 to 1.17 over five runs in a row (#512), with no step between the commits |

Hull `cube` D2 10^6 (0.97) and Delaunay `cube` D4 10^4 (1.050) are met, and were not in the final P7 run. Neither is settled: the first follows the build's layout (the section of #569), and the second has read 1.02 to 1.07 over the runs.

### Hull `sphere` D3 10^4 is 3.5% slower than at the final P7 run

The run of #569 read it at 1.055, and the two commits read 1.03 and 1.02 directly, which that section called inside the spread. It was not. `b6934d5` against `main`, plain builds, pinned and alternated, 14 rounds of 5 builds: 18.02 and 18.73 ms by the minimum (1.039), 20.31 and 21.03 ms by the median (1.035). Qhull on the same file in the same minutes: 18.03 ms by the minimum and 20.15 ms by the median of 6.

Delaunay `sphere` D2 and `cube` D2 at 10^4 read 1.04 and 1.03, and 1.06 and 1.01, in the same sitting (10 rounds of 5 builds).

Every commit between the two that touches `src`, 12 alternated rounds of 5 builds, minimum and median in ms, with the ratios to `b6934d5`:

| Commit | Hull `sphere` D3 10^4 | Delaunay `sphere` D2 10^4 | Delaunay `cube` D2 10^4 |
| --- | ---: | ---: | ---: |
| `b6934d5` | 17.94, 19.18 | 4.56, 4.94 | 4.74, 5.21 |
| `42bfbfa` (#527) | 18.14, 19.63 (1.01, 1.02) | 4.55, 4.87 (1.00, 0.99) | 4.65, 5.08 (0.98, 0.98) |
| `9c824dc` (#528) | 18.08, 19.12 (1.01, 1.00) | 4.55, 4.88 (1.00, 0.99) | 4.71, 5.17 (0.99, 0.99) |
| `384b73f` (#532) | 18.03, 19.64 (1.01, 1.02) | 4.50, 4.85 (0.99, 0.98) | 4.72, 5.14 (1.00, 0.99) |
| `356c8ee` (#535) | 17.86, 19.28 (1.00, 1.01) | 4.55, 4.77 (1.00, 0.97) | 4.75, 5.06 (1.00, 0.97) |
| `6b5fa06` (#542) | 17.63, 19.22 (0.98, 1.00) | 4.57, 4.92 (1.00, 1.00) | 4.73, 5.01 (1.00, 0.96) |
| `1467557` (#543) | 17.87, 19.20 (1.00, 1.00) | 4.64, 4.94 (1.02, 1.00) | 4.79, 5.27 (1.01, 1.01) |
| `accd8ce` (#547) | 18.05, 19.00 (1.01, 0.99) | 4.48, 4.81 (0.98, 0.97) | 4.77, 5.12 (1.01, 0.98) |
| `6713f61` (#549) | 18.54, 19.79 (1.03, 1.03) | 4.60, 5.03 (1.01, 1.02) | 4.74, 5.25 (1.00, 1.01) |
| `9feb3db` (#552) | 18.14, 19.57 (1.01, 1.02) | 4.55, 4.99 (1.00, 1.01) | 4.69, 5.16 (0.99, 0.99) |
| `94d9d5c` (#553) | 18.32, 19.56 (1.02, 1.02) | 4.54, 4.86 (1.00, 0.98) | 4.71, 5.10 (0.99, 0.98) |
| `c7180d5` (#555) | 18.21, 19.46 (1.01, 1.01) | 4.55, 4.78 (1.00, 0.97) | 4.74, 5.09 (1.00, 0.98) |
| `f1893ad` (#556) | 18.68, 20.01 (1.04, 1.04) | 4.59, 4.92 (1.01, 1.00) | 4.78, 5.05 (1.01, 0.97) |
| `b0d288d` (#566) | 18.62, 19.54 (1.04, 1.02) | 4.67, 4.95 (1.02, 1.00) | 4.84, 5.19 (1.02, 1.00) |
| `ce01e84`, `main` | 18.52, 19.79 (1.03, 1.03) | 4.64, 4.91 (1.02, 0.99) | 4.83, 5.21 (1.02, 1.00) |

**Hull `sphere` D3** rises at #549 and again at #556, by 1 to 3% each, about the spread of one commit in this table; the table suggests those two and does not show them. It is #578.

**The two Delaunay sets** show no step: 0.96 to 1.02 over the fifteen commits. The 1.03 to 1.06 of the direct timing above was the sitting, and the rows since P7 did not move them.

### Reading

- The rows since the final P7 run cost two parity sets: hull `sphere` D2 by 8 to 10%, found and fixed (#570), and hull `sphere` D3 10^4 by about 3.5%, found here and open (#578). A set that sat at 0.99 had no room for either.
- A ratio of 1.02 to 1.03 between two commits is not "inside the spread" when the minimum and the median agree; it takes more rounds, not a verdict. `bench.mdc` has the rule for a set that reads slower on the same side as an earlier run, and it applied.
- A parity run on this machine is disturbed in stretches of minutes whatever I leave alone. Sets whose slowest build is 1.5 times the fastest are run again before they are judged.

## Hull `sphere` D3 10^4: the flat lists keep where each list ends, PR #580 (#578)

The run of #577 left hull `sphere` D3 10^4 outside 1.05, and `main` built it 3.5% slower than the final P7 run's commit.

### Where the time went

The phases of one `build()` of hull `sphere` D3 10^4, summed by timers in scratch copies, under WSL, pinned and alternated, 14 runs of 10 builds; the minimum of `build()` over the runs, and the medians of the runs' means, in ms:

| Commit | `build()` (minimum, median) | Acceptance | Construction | Classification | Publication |
| --- | ---: | ---: | ---: | ---: | ---: |
| `b6934d5`, the final P7 run | 17.95, 20.22 | 0.10 | 16.73 | 2.84 | 0.40 |
| `accd8ce`, before #549 | 17.75, 20.33 | 0.10 | 16.76 | 2.86 | 0.49 |
| `6713f61` (#549) | 18.41, 20.63 | 0.11 | 16.83 | 3.30 | 0.49 |
| `c7180d5` (#555) | 18.13, 20.57 | 0.11 | 16.72 | 3.14 | 0.42 |
| `f1893ad` (#556) | 18.67, 20.83 | 0.10 | 16.97 | 3.11 | 0.48 |
| `ce01e84`, `main` | 18.68, 20.38 | 0.11 | 16.70 | 3.12 | 0.48 |

**Classification** rose by 16% at #549 and stayed 10% above. **Publication** reads 0.40 to 0.42 ms on two commits and 0.48 to 0.49 on four, in no order of time: it follows the build, not a change. **Construction** shows no step in this table; #556, which the timing by commit of #577 pointed at, does not show here.

With two more timers inside classification, in a second sitting (the medians of 14 runs):

| Commit | Classification | The merge | The rest of finding the vertices | After it |
| --- | ---: | ---: | ---: | ---: |
| `accd8ce`, before #549 | 2.72 | 1.72 | 0.33 | 0.68 |
| `ce01e84`, `main` | 2.91 | 1.76 | 0.38 | 0.77 |
| `e1b3775`, this change | 2.86 | 1.71 | 0.34 | 0.81 |

The last column does not compare across #549: since it, the buffers of the merge and of the sub-hulls are kept in a workspace that is dropped after the vertices are found, and before it they were dropped inside the first two columns.

### The change

Since #549 the logical facets of the merge are flat lists (`Lists`), and so are the faces written at the end. A `Lists` held a leading start, and #549 let a default value allocate nothing by writing that start in the first push; #573 moved the write out of line and left the test. The array now holds the end of each list and no leading start: a default value allocates nothing, a push writes one entry and tests nothing, and reading list `i` takes the end of the list before it, where the lookup one before the first list is out of bounds and reads as 0. A read is two bounds-checked loads, as before.

### Time

The final P7 run's commit, `main`, and the change, plain builds under WSL, pinned and alternated, minimum and median in ms, with the ratios to the first column; 5 to 14 rounds:

| Set | `b6934d5` | `ce01e84`, `main` | `e1b3775`, this change |
| --- | ---: | ---: | ---: |
| Hull `sphere` D3 10^4 | 18.55, 20.19 | 18.66, 21.01 (1.01, 1.04) | 18.43, 20.48 (0.99, 1.01) |
| Hull `sphere` D2 10^4 | 0.718, 0.796 | 0.699, 0.778 (0.97, 0.98) | 0.686, 0.776 (0.96, 0.98) |
| Hull `sphere` D2 10^5 | 8.85, 9.83 | 8.45, 9.68 (0.95, 0.98) | 8.48, 9.49 (0.96, 0.97) |
| Hull `sphere` D3 10^5 | 242.5, 260.0 | 255.4, 261.2 (1.05, 1.00) | 244.0, 259.7 (1.01, 1.00) |
| Hull `cube` D3 10^5 | 9.45, 10.43 | 9.24, 10.25 (0.98, 0.98) | 9.33, 10.47 (0.99, 1.00) |
| Hull `cubesurf` D3 10^5 | 26.71, 28.84 | 26.82, 28.71 (1.00, 1.00) | 25.93, 28.58 (0.97, 0.99) |
| Hull `cube` D2 10^6 | 58.8, 61.1 | 50.9, 54.7 (0.87, 0.90) | 52.2, 56.5 (0.89, 0.93) |
| Hull `sphere` D4 10^4 | 86.5, 91.4 | 86.5, 93.3 (1.00, 1.02) | 89.7, 92.8 (1.04, 1.01) |
| Hull `grid` D6 10^4 | 241.4, 251.1 | 231.7, 239.0 (0.96, 0.95) | 230.7, 240.0 (0.96, 0.96) |
| Delaunay `sphere` D2 10^4 | 4.62, 5.16 | 4.95, 5.26 (1.07, 1.02) | 4.82, 5.33 (1.04, 1.03) |
| Delaunay `cube` D2 10^4 | 4.93, 5.42 | 4.91, 5.48 (1.00, 1.01) | 4.82, 5.41 (0.98, 1.00) |
| Delaunay `cube` D2 10^5 | 50.7, 53.4 | 53.6, 55.7 (1.06, 1.04) | 52.4, 55.6 (1.03, 1.04) |
| Delaunay `sphere` D3 10^4 | 67.5, 71.8 | 66.6, 71.3 (0.99, 0.99) | 67.9, 72.2 (1.01, 1.01) |
| Delaunay `cube` D3 10^5 | 400.8, 409.3 | 388.5, 394.9 (0.97, 0.97) | 392.1, 399.8 (0.98, 0.98) |

Hull `sphere` D3 10^4 reads 1.01 by the median and 0.99 by the minimum against the final P7 run's commit, where `main` reads 1.04 and 1.01 in this sitting and read 1.035 and 1.039 over 14 rounds in the sitting of #577. In the timing by phase its `build()` reads 19.21, 19.99, and 19.46 ms by the median for `accd8ce`, `main`, and the change (1.04 and 1.01). About one percent is left in these builds; the section on one codegen unit below places it.

**Delaunay `cube` D2 10^5** reads 1.03 to 1.06 of the final P7 run's commit on `main` and with the change, and Delaunay `sphere` D2 10^4 1.02 to 1.07. The next two sections say what that is.

On Windows, `main` against the change, 4 to 12 rounds:

| Set | `ce01e84`, `main` | `e1b3775`, this change |
| --- | ---: | ---: |
| Hull `sphere` D3 10^4 | 20.46, 22.44 | 20.19, 22.54 (0.99, 1.00) |
| Hull `sphere` D2 10^4 | 0.974, 1.064 | 0.956, 1.056 (0.98, 0.99) |
| Hull `sphere` D2 10^5 | 9.43, 10.29 | 9.41, 10.43 (1.00, 1.01) |
| Hull `sphere` D3 10^5 | 243.9, 259.2 | 247.5, 259.2 (1.02, 1.00) |
| Hull `cube` D3 10^6 | 140.8, 151.4 | 140.0, 149.1 (0.99, 0.99) |
| Hull `cubesurf` D3 10^5 | 27.96, 29.87 | 27.70, 29.59 (0.99, 0.99) |
| Hull `cube` D4 10^5 | 32.5, 34.5 | 32.1, 34.1 (0.99, 0.99) |
| Hull `grid` D6 10^4 | 241.0, 256.9 | 245.5, 252.5 (1.02, 0.98) |
| Delaunay `cube` D2 10^5 | 52.1, 55.5 | 51.9, 53.8 (1.00, 0.97) |
| Delaunay `sphere` D2 10^5 | 43.7, 48.2 | 44.3, 47.3 (1.01, 0.98) |
| Delaunay `cube` D3 10^5 | 388.1, 414.0 | 398.3, 410.1 (1.03, 0.99) |
| Delaunay of 20,000 `lattice` points, D3 | 2.48, 2.74 | 2.48, 2.77 (1.00, 1.01) |
| Voronoi `cube` D3, 30,000 points | 315.6, 339.7 | 309.2, 337.0 (0.98, 0.99) |

### Delaunay D2 by commit

Every commit since the final P7 run that touches `src`, and this change, plain builds under WSL, 16 alternated rounds of 2 builds (10^5) and of 8 (10^4), minimum and median in ms, with the ratios to `b6934d5`:

| Commit | `cube` D2 10^5 | `sphere` D2 10^5 | `cube` D2 10^4 | `sphere` D2 10^4 |
| --- | ---: | ---: | ---: | ---: |
| `b6934d5` | 50.53, 54.11 | 39.83, 43.36 | 4.64, 4.99 | 4.46, 4.78 |
| `42bfbfa` (#527) | 50.14, 54.01 (0.99, 1.00) | 39.40, 43.53 (0.99, 1.00) | 4.67, 5.03 (1.01, 1.01) | 4.48, 4.73 (1.00, 0.99) |
| `9c824dc` (#528) | 50.29, 53.36 (1.00, 0.99) | 39.20, 43.06 (0.98, 0.99) | 4.65, 4.92 (1.00, 0.99) | 4.46, 4.76 (1.00, 1.00) |
| `384b73f` (#532) | 50.13, 53.83 (0.99, 0.99) | 39.63, 43.20 (1.00, 1.00) | 4.67, 4.95 (1.01, 0.99) | 4.47, 4.76 (1.00, 1.00) |
| `356c8ee` (#535) | 51.54, 53.89 (1.02, 1.00) | 40.21, 43.77 (1.01, 1.01) | 4.67, 4.99 (1.01, 1.00) | 4.45, 4.84 (1.00, 1.01) |
| `6b5fa06` (#542) | 50.25, 53.94 (0.99, 1.00) | 40.56, 43.18 (1.02, 1.00) | 4.65, 4.88 (1.00, 0.98) | 4.46, 4.75 (1.00, 0.99) |
| `1467557` (#543) | 52.13, 54.69 (1.03, 1.01) | 41.38, 44.32 (1.04, 1.02) | 4.77, 5.06 (1.03, 1.01) | 4.63, 4.94 (1.04, 1.03) |
| `accd8ce` (#547) | 50.91, 53.64 (1.01, 0.99) | 40.29, 43.14 (1.01, 0.99) | 4.64, 4.96 (1.00, 0.99) | 4.49, 4.73 (1.00, 0.99) |
| `6713f61` (#549) | 50.32, 54.80 (1.00, 1.01) | 40.25, 43.38 (1.01, 1.00) | 4.66, 5.02 (1.00, 1.00) | 4.49, 4.77 (1.01, 1.00) |
| `9feb3db` (#552) | 50.66, 54.77 (1.00, 1.01) | 40.36, 43.63 (1.01, 1.01) | 4.74, 4.98 (1.02, 1.00) | 4.50, 4.72 (1.01, 0.99) |
| `94d9d5c` (#553) | 50.78, 54.48 (1.00, 1.01) | 39.90, 42.49 (1.00, 0.98) | 4.62, 4.89 (1.00, 0.98) | 4.46, 4.72 (1.00, 0.99) |
| `c7180d5` (#555) | 51.30, 54.04 (1.02, 1.00) | 40.54, 43.41 (1.02, 1.00) | 4.69, 4.99 (1.01, 1.00) | 4.47, 4.76 (1.00, 1.00) |
| `f1893ad` (#556) | 50.66, 54.70 (1.00, 1.01) | 39.99, 42.89 (1.00, 0.99) | 4.71, 4.96 (1.01, 0.99) | 4.52, 4.74 (1.01, 0.99) |
| `b0d288d` (#566) | 50.94, 54.84 (1.01, 1.01) | 40.89, 45.05 (1.03, 1.04) | 4.69, 5.02 (1.01, 1.01) | 4.61, 4.81 (1.03, 1.01) |
| `ce01e84`, `main` | 51.00, 55.33 (1.01, 1.02) | 41.55, 44.01 (1.04, 1.02) | 4.80, 5.00 (1.03, 1.00) | 4.58, 4.83 (1.03, 1.01) |
| `e1b3775`, this change | 51.12, 54.37 (1.01, 1.00) | 40.88, 43.79 (1.03, 1.01) | 4.74, 5.04 (1.02, 1.01) | 4.55, 4.89 (1.02, 1.02) |

Two things stand out, and neither is work added to a Delaunay build:

- `1467557` reads 1.03 to 1.04 by the minimum on all four sets and the commit after it reads 1.00 again. It changed how a Voronoi build finds the facets of a cospherical group, which a Delaunay build does not run.
- From `b0d288d` on, the `sphere` sets read 1.03 to 1.04 by the minimum. It changed the linking of the D + 2 initial cells, which runs once in a build.

### With one codegen unit

The same sets built with `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`, where the compiler lays the crate out as one unit, 16 alternated rounds, minimum and median in ms, with the ratios to `b6934d5`:

| Set | `b6934d5` | `f1893ad` (#556) | `b0d288d` (#566) | `ce01e84`, `main` | `e1b3775`, this change |
| --- | ---: | ---: | ---: | ---: | ---: |
| Delaunay `cube` D2 10^5 | 51.06, 54.87 | 50.65, 53.25 (0.99, 0.97) | 51.02, 54.10 (1.00, 0.99) | 50.34, 54.02 (0.99, 0.98) | – |
| Delaunay `sphere` D2 10^5 | 40.08, 43.33 | 39.24, 43.49 (0.98, 1.00) | 40.20, 43.10 (1.00, 0.99) | 40.25, 42.48 (1.00, 0.98) | – |
| Delaunay `cube` D2 10^4 | 4.69, 4.95 | 4.63, 4.93 (0.99, 1.00) | 4.71, 4.93 (1.00, 1.00) | 4.57, 4.98 (0.97, 1.01) | – |
| Delaunay `sphere` D2 10^4 | 4.44, 4.68 | 4.48, 4.75 (1.01, 1.02) | 4.47, 4.81 (1.01, 1.03) | 4.45, 4.68 (1.00, 1.00) | – |
| Hull `sphere` D3 10^4 | 16.97, 18.75 | – | – | 17.16, 18.98 (1.01, 1.01) | 17.03, 18.81 (1.00, 1.00) |
| Hull `sphere` D2 10^5 | 8.34, 9.68 | – | – | 8.32, 9.87 (1.00, 1.02) | 8.46, 9.63 (1.02, 0.99) |
| Hull `sphere` D3 10^5 (one build a round) | 230.4, 238.8 | – | – | 228.6, 252.9 (0.99, 1.06) | 236.8, 247.1 (1.03, 1.03) |

- **The Delaunay sets of D = 2 are where they were at the final P7 run**: `main` reads 0.97 to 1.01 of it. The 1.02 to 1.07 of the default builds is where the compiler put the code in those builds, not what the code does. The rows since P7 did not slow Delaunay D2.
- **Hull `sphere` D3 10^4 is back**: `main` reads 1.01 by the minimum and by the median, and the change 1.00 and 1.00. The percent left in the default builds is layout too.
- **One codegen unit is itself 6 to 8% faster on hull `sphere` D3 10^4**: 16.97 to 17.16 ms by the minimum, against 17.9 to 18.7 ms in the default builds of the same commits. The parity run judges the default build, as a user's `cargo build --release` makes it, and it stays so: the owner decided on 2026-10-12 to keep judging the default build (#578). A reading with one codegen unit is a way to tell layout from work, and is reported beside the judgement, not in its place.

**Not timed:** the hulls of D = 9 and 10 of #556. The change is in `src/lists.rs` alone and #556's lists are not `Lists`. The set was not judged against Qhull and CGAL again.

## The sets not met or at the edge, judged again after #580, PR #584 (#583)

The run of #577 left three sets outside 1.05 and several within a few hundredths of it. #580 then brought hull `sphere` D3 10^4 back to its time at the final P7 run. These thirteen sets are judged again on `main` at `d9b6725`, by the method of that run, with the tools of `benches/parity/` as committed (their first use for a record). No set's slowest build reached 1.5 times its fastest (the largest 1.44), so none was run again.

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^6 | 55.8 ms | 0.80 | 1.24 | 0.96 | met | met |
| Hull `sphere` D2 10^4 | 0.81 ms | 1.01 | 1.19 | 0.11 (ref.) | met | not met |
| Hull `sphere` D3 10^4 | 19.1 ms | 0.87 | – | 0.98 | met | met |
| Hull `sphere` D3 10^5 | 254 ms | 0.61 | – | 1.01 | met | not met |
| Hull `sphere` D3 10^6 | 3.40 s | 0.42 | – | 1.03 | met | not met |
| Hull `sphere` D5 10^4 | 846 ms | 0.84 | – | 0.95 | met | met |
| Hull `cubesurf` D3 10^5 | 28.5 ms | 0.70 | – | 0.93 | met | met |
| Delaunay `cube` D2 10^4 | 5.23 ms | 1.04 | 1.23 | 0.26 | met | not met |
| Delaunay `cube` D2 10^5 | 56.6 ms | 1.00 | 1.29 | 0.22 | met | met |
| Delaunay `sphere` D2 10^4 | 5.37 ms | 1.10 | 1.30 | 0.21 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 47.9 ms | 1.03 | 1.39 | 0.11 (ref.) | met | not met |
| Delaunay `cube` D4 10^4 | 694 ms | 1.03 | 1.09 | 0.87 | met | not met |
| Delaunay `cube` D5 10^4 | 7.55 s | 1.04 | 1.11 | 0.94 | met | not met |

Against the run of #577 (its second readings):

| Set | Run of #577 | This run |
| --- | ---: | ---: |
| Hull `cube` D2 10^6 | 0.97 | 0.96 |
| Hull `sphere` D2 10^4 | 1.02 | 1.01 |
| Hull `sphere` D3 10^4 | 1.09 | 0.98 |
| Hull `sphere` D3 10^5 | 0.99 | 1.01 |
| Hull `sphere` D3 10^6 | 1.00 | 1.03 |
| Hull `sphere` D5 10^4 | 1.01 | 0.95 |
| Hull `cubesurf` D3 10^5 | 0.97 | 0.93 |
| Delaunay `cube` D2 10^4 | 1.07 | 1.04 |
| Delaunay `cube` D2 10^5 | 1.04 | 1.00 |
| Delaunay `sphere` D2 10^4 | 1.17 | 1.10 |
| Delaunay `sphere` D2 10^5 | 1.00 | 1.03 |
| Delaunay `cube` D4 10^4 | 1.05 | 1.03 |
| Delaunay `cube` D5 10^4 | 1.03 | 1.04 |

- **Hull `sphere` D3 10^4 is met again:** 0.98 against Qhull, from 1.06 and 1.09. Its `build()` reads 19.1 ms, from 20.9 and 22.5 ms.
- **One set of the thirteen is not met:** Delaunay `sphere` D2 10^4, 1.10 against CGAL, as in every run since #512.
- **Delaunay `cube` D2 10^4 reads 1.04** and is met in this run. It has read 1.04 to 1.13 over seven runs with no change to its code, on either side of 1.05; it is not settled by this reading.
- With these thirteen rows in place of theirs in the run of #577, and the other 25 as that run read them, 37 of the 38 judged sets are met at 1.05 and 30 at 1.00. That is two runs put together, two hours and two commits apart (`ce01e84` and `d9b6725`, which differ by #580, the tools of #581, and documents), and not one run.
- Five of the thirteen sit between 1.02 and 1.05: Delaunay `cube` D2 10^4, D4 10^4, and D5 10^4, Delaunay `sphere` D2 10^5, and hull `sphere` D3 10^6. A run can put any of them on either side.

## The placing search at every dimension the complex reaches, PR #587 (#565)

The spike #563 found 15.9% of the hull of the corners of a cube in D = 8 in `Placing::scan`. The search on the surface of #540 started only once the complex had the dimension of all the points; the corners of a cube in index order reach dimension k at corner 2^(k - 1), so the first half of a facet's points were placed by the scan of every boundary ridge. The search now starts after each point that raises the complex to three dimensions or more, and stops at the next point that raises it.

**The same simplices in the same order:** the whole `Debug` form of the result has the same hash on both sides on every input below.

The hull of the corners of a cube, base `d9b6725`, head `2fc0452`, one `build()`:

| D | Corners | Windows, pinned, before | After | WSL, pinned, before | After |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 32 | 2.42 ms | 2.50 ms | – | – |
| 6 | 64 | 43.6 ms | 46.6 ms | – | – |
| 7 | 128 | 647 ms | 631 ms | 586 ms | 578 ms (0.99) |
| 8 | 256 | 9.04 s | 7.65 s (0.85) | 8.19 s | 6.99 s (0.85) |

D = 5 to 7 on Windows are the minimum of three alternated runs of 20, 5, and 2 builds (in D = 6 the three read 43.6, 48.6, and 48.4 ms before and 46.6, 47.0, and 47.7 after); D = 8 the minimum of three.

Inside the placing, by timers in scratch copies, Windows, one build:

| D | Commit | `build()` | The scan | The search |
| ---: | --- | ---: | ---: | ---: |
| 7 | `d9b6725` | 643 ms | 11 ms (1.8%) | 161 ms (25.0%) |
| 7 | `2fc0452` | 615 ms | 1 ms (0.1%) | 166 ms (27.0%) |
| 8 | `d9b6725` | 8,473 ms | 1,390 ms (16.4%) | 1,693 ms (20.0%) |
| 8 | `2fc0452` | 7,208 ms | 6 ms (0.1%) | 1,910 ms (26.5%) |

The guards, pinned and alternated, minimum and median in ms, with the ratios of the head to the base:

| Input | Windows, base | Windows, head | WSL, base | WSL, head |
| --- | ---: | ---: | ---: | ---: |
| Hull, a 1,024-gon times a square, D4 | 79.7, 82.8 | 80.3, 82.6 (1.01, 1.00) | 70.3, 73.3 | 70.5, 73.3 (1.00, 1.00) |
| Hull, a 256-gon times a cube, D5 | 455.2, 483.3 | 450.8, 476.5 (0.99, 0.99) | 409.7, 426.3 | 418.6, 426.2 (1.02, 1.00) |
| Delaunay, the integer points of a sphere and a site outside, 1,921 sites, D3 | 23.74, 24.42 | 23.58, 23.96 (0.99, 0.98) | 21.99, 22.83 | 20.90, 22.34 (0.95, 0.98) |
| Delaunay, the full grid of side 8, D4 | 737.4, 761.5 | 732.8, 765.7 (0.99, 1.01) | 700.4, 718.2 | 695.9, 716.2 (0.99, 1.00) |
| Hull `cube` D4 10^5 | 34.89, 36.78 | 34.95, 36.69 (1.00, 1.00) | 31.16, 33.71 | 31.12, 33.28 (1.00, 0.99) |
| Hull `sphere` D3 10^5 | 273.0, 296.2 | 268.7, 287.3 (0.98, 0.97) | 248.3, 256.7 | 252.2, 262.6 (1.02, 1.02) |
| Hull `sphere` D3 10^4 | – | – | 18.74, 20.13 | 18.73, 20.05 (1.00, 1.00) |
| Hull `cubesurf` D4, 20,000 points | 39.98, 41.89 | 39.04, 42.80 (0.98, 1.02) | 37.71, 40.79 | 36.45, 40.72 (0.97, 1.00) |
| Hull `grid` D6 10^4 | 256.3, 278.6 | 257.0, 271.2 (1.00, 0.97) | 223.9, 232.3 | 224.4, 230.9 (1.00, 0.99) |
| Delaunay of 20,000 `lattice` points, D3 | 2.479, 2.781 | 2.503, 2.805 (1.01, 1.01) | 2.093, 2.264 | 2.098, 2.305 (1.00, 1.02) |
| Delaunay of 3,000 `lattice` points, D4 | 77.79, 86.04 | 79.94, 85.31 (1.03, 0.99) | 72.43, 75.37 | 72.78, 75.03 (1.00, 1.00) |

5 to 12 rounds. The Windows columns and the first WSL sitting were timed on `0e44b0d`; the head differs from it in a test only. Hull `sphere` D3 10^5 reads 1.02 and 1.02 under WSL over 10 rounds of 2 builds, 0.98 and 0.97 on Windows, and 1.04 and 0.96 in an earlier WSL sitting of 6 rounds that started while the machine was still busy from the tests; at 10^4 it reads 1.00 over 12 rounds of 5. A hull of `sphere` D3 has no face of more than three points and never enters the placing. In that earlier sitting the 1,024-gon times a square read 1.09 and 1.21 and hull `cube` D4 1.05 and 1.02; the table has their second sitting.

`tests/alloc.rs` passes with no record moved.

## The last rows of P7: two withdrawn after measurement, and the lifted underflow terms removed, PR #588 (#323, #400, #355)

### P7-8 and P7-31, withdrawn

- **P7-8, faster duplicate detection (#323).** Two changes of 2026-10-09 read 0.98 to 1.02 on hull `cube` D2 and D3 and Delaunay `cube` D2 at 10^6, inside the spread; the time charged to the table is the first read of each point. The Issue has the tables.
- **P7-31, the exact direction's error bound from its derivation (#400).** Counted before any change, on `main` at `9a8785b`: with the bound at the derived `(k/2 + 5) u` in place of `k 2^-49`, the side tests a cull plane leaves to the orientation are 808,554 of 2,611,341 either way on hull `grid` D6 10^4, 1,229,151 of 5,655,537 either way on hull `lattice` D6 10^4, and 233,077 against 233,089 on hull `cluster` D5 10^4. The points a plane of these sets does not prove are on the plane, at a working distance of exactly zero; none sits between the two thresholds.

### P7-18, the lifted formulas of k up to 4 (#355)

`lifted1`, `lifted2`, and `lifted3` held, in the sum `X` of their underflow term `4 η X`, a part for the underflow of their lifted entries: `|a| + |b|`, the sum of the minors' permanents, and twice that sum. Dropping it failed no test. It is redundant, proved in the functions' documentation, and removed:

- **`lifted2`:** the permanent of a minor of two rows is at most half the sum of their lifted entries (`|x_i y_j| <= (x_i^2 + y_j^2) / 2`), so the three permanents add up to at most the sum of the lifted entries, which `X` holds for the minors' underflow.
- **`lifted3`:** expanded along the third coordinate, the four minors' permanents add up to at most `Z` times the sum of the lifted entries, which `X` holds too.
- **`lifted1`:** `X` has no such sum. A square's underflow error scaled by the other coordinate is either within `η`, or the monomial `|a| b^2` of the permanent is a normal number and the slack of the relative term covers it.

**Searched before it was proved.** In a scratch copy, 400,000 inputs per formula with each row at its own scale between the smallest subnormal and `2^300`: with the terms dropped, the exact error of the computed determinant never exceeded half the bound. With the sum of the lifted entries dropped as well, the same inputs put the error above the bound on 14% (D = 2) and 18% (D = 3) of them, and a wrong sign was certified on 981 and 7,074: the search finds what is there.

`lifted_rows_of_mixed_scales_are_covered` keeps 4,000 such inputs per formula, with the bound at twice the exact error and every certified sign compared with the exact one. Five mutations of the terms that stay each fail a test (the pull request lists them).

**Time**, `main` at `9a8785b` against `91425cf`, pinned and alternated, minimum and median in ms with the ratios of the head; the result has the same hash on both sides on every set:

| Set | WSL, base | WSL, head | Windows, base | Windows, head |
| --- | ---: | ---: | ---: | ---: |
| Delaunay `cube` D2 10^4 | 4.85, 5.43 | 4.65, 5.42 (0.96, 1.00) | 5.21, 5.95 | 5.22, 5.81 (1.00, 0.98) |
| Delaunay `sphere` D2 10^4 | 4.66, 5.28 | 4.63, 5.22 (0.99, 0.99) | 5.50, 6.07 | 5.40, 6.06 (0.98, 1.00) |
| Delaunay `cube` D2 10^5 | 53.5, 59.0 | 53.3, 59.5 (1.00, 1.01) | 53.2, 59.1 | 52.5, 57.5 (0.99, 0.97) |
| Delaunay `sphere` D2 10^5 | 40.6, 44.9 | 41.0, 44.5 (1.01, 0.99) | 42.4, 44.1 | 42.6, 44.5 (1.00, 1.01) |
| Delaunay `cube` D3 10^4 | 32.1, 35.2 | 31.6, 35.1 (0.99, 1.00) | 35.1, 39.4 | 35.5, 39.1 (1.01, 0.99) |
| Delaunay `sphere` D3 10^4 | 65.8, 68.0 | 65.5, 67.2 (1.00, 0.99) | 82.0, 92.5 | 82.7, 92.2 (1.01, 1.00) |
| Delaunay `cube` D3 10^5 | 381.9, 406.3 | 378.7, 386.5 (0.99, 0.95) | 411.1, 426.6 | 410.6, 432.1 (1.00, 1.01) |
| Delaunay of 20,000 `lattice` points, D3 | 2.38, 3.53 | 2.37, 3.59 (1.00, 1.02) | 2.46, 2.83 | 2.50, 2.79 (1.02, 0.99) |
| Voronoi `cube` D2 10^5 | 200.9, 226.7 | 202.6, 227.7 (1.01, 1.00) | 201.7, 214.7 | 200.5, 211.8 (0.99, 0.99) |
| Hull `sphere` D3 10^4 | 21.9, 30.3 | 22.2, 30.4 (1.01, 1.00) | 20.6, 22.0 | 20.8, 22.5 (1.01, 1.02) |
| Hull `cube` D3 10^5 | 12.0, 19.4 | 11.8, 16.6 (0.98, 0.86) | 11.3, 12.2 | 11.0, 12.4 (0.98, 1.01) |

5 to 16 rounds. Three Windows rows are a second sitting of 14 to 16 rounds: in the first, of 8, Delaunay `cube` D3 10^5 read 1.04 and 1.04, `cube` D2 10^5 1.02 and 1.03, and `sphere` D2 10^5 0.99 and 1.05. The last three WSL rows were disturbed (their medians stand far above their minima on both sides). The change takes three operations out of an in-circle test and five out of an in-sphere test, and no set moves beyond the spread: it is not a speed change and claims none.

## The full parity run after the last rows of P7, PR #603 (#591)

One run of the 41 sets on `main` at `df80378`, which has the placing search at every dimension (#587) and the lifted formulas without their redundant underflow terms (#588). The count of 37 of 38 in the section of #583 was two runs put together; this is one. Method: `docs/verification.md`, with `benches/parity/` as committed; WSL2, Qhull 2020.2, CGAL 5.4, `rustc` 1.99.0, `g++` 11.4.0.

It is the record of `main` before the filter's exact scaling (#589), which goes first in R1; a run after that row follows.

### Judgement

| Threshold | Met of the 38 judged sets | Hull | Delaunay | Run of #577 | Final P7 run |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1.05 | 36 | 25 | 11 | 35 | 34 |
| 1.00 | 29 | 21 | 8 | 29 | 29 |

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.47 ms | 0.75 | 0.87 | 0.76 | met | met |
| Hull `cube` D2 10^5 | 4.37 ms | 0.78 | 0.98 | 0.78 | met | met |
| Hull `cube` D2 10^6 | 54.8 ms | 0.80 | 1.21 | 0.97 | met | met |
| Hull `cube` D3 10^4 | 1.13 ms | 0.53 | – | 0.77 | met | met |
| Hull `cube` D3 10^5 | 10.5 ms | 0.40 | – | 0.86 | met | met |
| Hull `cube` D3 10^6 | 144 ms | 0.26 | – | 0.95 | met | met |
| Hull `cube` D4 10^4 | 5.64 ms | 0.13 | – | 0.80 | met | met |
| Hull `cube` D4 10^5 | 33.9 ms | 0.06 | – | 0.97 | met | met |
| Hull `cube` D5 10^4 | 54.2 ms | 0.24 | – | 0.73 | met | met |
| Hull `cube` D5 10^5 | 239 ms | 0.13 | – | 0.67 | met | met |
| Hull `cube` D6 10^4 | 605 ms | 0.27 | – | 0.46 | met | met |
| Hull `cube` D6 10^5 | 2.63 s | 0.24 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.80 ms | 0.99 | 1.15 | 0.11 (ref.) | met | met |
| Hull `sphere` D2 10^5 | 10.5 ms | 0.97 | 1.26 | 0.08 (ref.) | met | met |
| Hull `sphere` D2 10^6 | 114 ms | 0.65 | 1.22 | 0.05 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 21.0 ms | 0.88 | – | 1.04 | met | not met |
| Hull `sphere` D3 10^5 | 260 ms | 0.63 | – | 1.01 | met | not met |
| Hull `sphere` D3 10^6 | 3.96 s | 0.42 | – | 0.98 | met | met |
| Hull `sphere` D4 10^4 | 107 ms | 0.79 | – | 0.91 | met | met |
| Hull `sphere` D4 10^5 | 1.24 s | 0.93 | – | 0.89 | met | met |
| Hull `sphere` D5 10^4 | 895 ms | 0.86 | – | 1.00 | met | not met |
| Hull `sphere` D5 10^5 | 10.77 s | 0.99 | – | 1.02 | met | not met |
| Hull `sphere` D6 10^4 | 6.76 s | 0.69 | – | 0.80 | met | met |
| Hull `cubesurf` D3 10^5 | 31.3 ms | 0.76 | – | 0.97 | met | met |
| Hull `grid` D6 10^4 | 234 ms | 0.91 (ref.) | – | 0.66 | met | met |
| Delaunay `cube` D2 10^4 | 5.10 ms | 1.09 | 1.26 | 0.27 | not met | not met |
| Delaunay `cube` D2 10^5 | 53.4 ms | 0.96 | 1.26 | 0.22 | met | met |
| Delaunay `cube` D2 10^6 | 606 ms | 0.85 | 1.26 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 5.04 ms | 1.16 | 1.24 | 0.22 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 41.8 ms | 1.04 | 1.35 | 0.12 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 417 ms | 0.82 | 1.32 | 0.04 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 37.0 ms | 1.09 | 1.12 | 0.35 | reported | reported |
| Delaunay `cube` D3 10^5 | 381 ms | 1.07 | 1.23 | 0.30 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.18 s | 1.07 | 1.21 | 0.25 | reported | reported |
| Delaunay `sphere` D3 10^4 | 74.2 ms | 0.51 | 0.51 | 0.56 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 697 ms | 0.69 | 0.70 | 0.35 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.65 s | 0.81 | 0.84 | 0.33 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 683 ms | 1.05 | 1.10 | 0.87 | met | not met |
| Delaunay `sphere` D4 10^4 | 880 ms | 0.34 | 0.34 | 0.70 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.36 s | 1.02 | 1.05 | 0.92 | met | not met |
| Delaunay `sphere` D5 10^4 | 12.56 s | 0.34 | 0.33 | 0.74 (ref.) | met | met |

**Delaunay `cube` D3, reported:** convx's construction alone reads 0.93, 0.88, and 0.87 of CGAL's whole time at 10^4, 10^5, and 10^6.

### The sets run again

Three sets had a slowest build 1.5 times the fastest or more, and two more were run with them: hull `sphere` D3 10^4, at the edge of 1.05, and hull `cube` D6 10^5, whose `build()` read 3.26 s against 2.63 s in the run of #577. The table above has the second reading. With the first readings 34 sets are met at 1.05, with the second 36.

| Set | First: `build()` (fastest to slowest), / CGAL, / Qhull | Again |
| --- | --- | --- |
| Hull `sphere` D3 10^4 | 22.2 ms (20.6 to 27.0), 0.84, 1.05 | 21.0 ms (18.8 to 22.3), 0.88, 1.04 |
| Hull `sphere` D3 10^5 | 297 ms (262 to 436), 0.62, 1.11 | 260 ms (239 to 294), 0.63, 1.01 |
| Hull `cubesurf` D3 10^5 | 28.0 ms (26.6 to 40.2), 0.72, 0.97 | 31.3 ms (28.4 to 37.5), 0.76, 0.97 |
| Hull `cube` D6 10^5 | 3.26 s (2.88 to 4.29), 0.14, 0.38 | 2.63 s (2.60 to 2.67), 0.24, 0.39 |
| Delaunay `sphere` D5 10^4 | 11.59 s (11.29 to 37.72), 0.32, 0.78 | 12.56 s (11.17 to 15.45), 0.34, 0.74 |

After it, 23 of the 41 sets still have a slowest build 1.2 times the fastest or more, the largest 1.48. During this run I edited files, made commits, and searched the web, and built nothing; the machine was not otherwise idle either.

### What is not met

| Set | Ratio | Reason |
| --- | --- | --- |
| Delaunay `cube` D2 10^4 | 1.09 against CGAL | 1.04 to 1.13 over eight runs, with no change to its code |
| Delaunay `sphere` D2 10^4 | 1.16 against CGAL | 1.04 to 1.17 over seven runs in a row (#512) |

Three more sit between 1.03 and 1.05: hull `sphere` D3 10^4 (1.04 against Qhull), Delaunay `sphere` D2 10^5 (1.04 against CGAL), and Delaunay `cube` D4 10^4 (1.045). Delaunay `cube` D5 10^4 and hull `sphere` D5 10^5 read 1.02.

### Reading

- Hull `sphere` D2 and D3 are inside 1.05 at every size, as before the rows that cost them (#570, #578).
- The two sets not met are Delaunay of D = 2 at 10^4, where CGAL's whole run is under 5 ms: the same two as at the end of P7.
- The rows since the run of #577 (#580, #587, #588) moved no set out. The corners of a cube, which #587 is about, are not a parity set.

## Inputs far from 1: one exact scaling of the whole input, PR #606 (#589)

### What was slow, and how widely

`main` at `df80378`, release, Windows, pinned, the best of 2 to 5 builds; the generator's points with every coordinate multiplied by the scale (a power of two is an exact scaling; `1e300` and `1e-300` are not, and give other points of the same shape):

| Input | Scale 1 | 2^-1000 | 1e-300 | 2^-200 | 2^-98 | 2^24 | 2^60 | 1e300 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Hull `cube` D2, 100,000 points | 5.0 ms | 71.9 | 81.0 | 4.7 | 4.6 | 5.0 | 5.2 | 80.4 |
| Hull `sphere` D3, 20,000 | 46.4 ms | 127.7 | 124.3 | 56.5 | 59.5 | 46.4 | 44.1 | 61.0 |
| Hull `cubesurf` D3, 20,000 | 6.0 ms | 63.1 | 67.6 | 6.0 | 5.7 | 5.6 | 6.1 | 10.6 |
| Hull `cube` D5, 5,000 | 37.6 ms | 95.6 | 93.0 | 36.1 | 36.0 | 41.7 | 38.6 | 37.6 |
| Delaunay `cube` D2, 20,000 | 11.3 ms | 122.4 | 117.6 | 10.6 | 10.6 | 11.4 | 17.0 | 118.4 |
| Delaunay `cube` D3, 20,000 | 78.3 ms | 1,153.7 | 1,166.7 | 148.5 | 77.8 | 108.2 | 105.1 | 1,212.4 |
| Delaunay `cube` D4, 3,000 | 189.9 ms | 2,610.1 | 2,516.6 | 2,392.5 | 182.2 | 221.4 | 227.6 | 2,520.7 |
| Voronoi `cube` D2, 20,000 | 32.6 ms | 157.8 | 167.5 | 35.5 | 35.2 | 34.4 | 40.6 | 160.8 |

- At the ends of the range a build took 2.5 to 16 times its time. The Issue had measured one hull at 3.3 times.
- **It is not only the ends.** A Delaunay build of D = 3 reads 1.4 times its time with coordinates near `2^24`, sixteen million, and of D = 4 1.2 times; a second series put the start at `2^16` (1.08 for D = 3, 1.14 for D = 4), and nothing down to `2^-24`. The lifted entries are squares, and the first stage's bound leaves its common form once the sum in its underflow term passes `2^60`.
- Counted at 1e-300: of the predicates that reach the last decision, nearly all go to the exact stage on the hull of D = 2 and on Delaunay (1,052,048 of 1,052,049; 1,338,612 of 1,338,614 in D = 3), but hull `cube` D3 takes 5 times its time with six predicates there. The rest of a build works on the same coordinates: the cull scans, the fixed-size planes of D = 3 and 4 (edges in `[2^-100, 2^100]` only), the lifted heights.

### The method of the Grill on #179, tried and withdrawn

Scaling the entries of one predicate where the filter left its sign open: no predicate reached the exact stage any more, and at 1e-300 Delaunay `cube` D3 went from 1,158 to 238 ms (scale 1: 79), Delaunay D2 from 123 to 43 ms (11), hull `cube` D2 from 116 to 79 ms (5.5), and the hulls of D = 3 and 5 did not move. Each predicate ran its first stage, failed, and ran again. The owner took the other method on 2026-10-12.

### The change

Before acceptance, an input whose largest coordinate is far from 1 is copied times the power of two that brings that coordinate into [1, 2), when the product is exact for every coordinate; the build runs on the copy. Signs do not change, so what is published as indices is the same; the hull's vertex coordinates (from which its planes and its volume come) and the vertices of a Voronoi diagram are taken from the input itself. A sample of 1,024 coordinates decides whether the input is read in full, so an ordinary input pays for the sample only. The thresholds: `2^12` for sites, `2^64` for the points of a hull, `2^300` for a hull of D = 2.

The same table with the change. The row of the hull of D = 2 is from the head, `2bb0004`; the others are from `36d2bae`, which differs from the head in the threshold of the hull of D = 2 alone:

| Input | Scale 1 | 2^-1000 | 1e-300 | 2^-200 | 2^-98 | 2^24 | 2^60 | 1e300 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Hull `cube` D2, 100,000 points | 5.1 ms | 5.7 | 5.7 | 5.0 | 4.7 | 5.0 | 5.1 | 6.4 |
| Hull `sphere` D3, 20,000 | 46.7 ms | 44.5 | 46.6 | 47.0 | 47.1 | 45.5 | 45.9 | 46.1 |
| Hull `cubesurf` D3, 20,000 | 6.0 ms | 5.6 | 6.2 | 6.1 | 5.6 | 5.5 | 5.6 | 6.6 |
| Hull `cube` D5, 5,000 | 38.2 ms | 35.7 | 36.1 | 35.6 | 36.2 | 40.1 | 39.0 | 37.0 |
| Delaunay `cube` D2, 20,000 | 11.2 ms | 11.5 | 10.7 | 11.4 | 10.8 | 11.0 | 11.7 | 10.9 |
| Delaunay `cube` D3, 20,000 | 80.4 ms | 79.5 | 76.8 | 79.3 | 77.6 | 79.1 | 78.1 | 78.5 |
| Delaunay `cube` D4, 3,000 | 194.7 ms | 189.5 | 186.0 | 184.2 | 183.2 | 180.6 | 183.4 | 184.8 |
| Voronoi `cube` D2, 20,000 | 34.9 ms | 33.0 | 33.3 | 33.1 | 33.3 | 32.1 | 33.1 | 33.3 |

Every build is at its time of scale 1, but the hull of D = 2 at the ends, which reads 1.1 to 1.25 of it: the copy and its passes over the coordinates, on a build that otherwise reads every point once. The whole `Debug` form of a Delaunay triangulation and the vertices, facets, and coplanar points of a hull had the same hash at every scale, before and after.

**The hull of D = 2 has a threshold of its own.** With `2^64` its copy cost 0.7 ms of a 5 ms build at `2^-98` and `2^-200` (5.4 and 5.5 ms against 4.6 and 4.7), where `main` was not slow: its determinants are products of two differences and underflow only near `2^-511`. It is scaled from `2^300`.

### Ordinary inputs

Scale 1, where nothing is scaled, `df80378` against `36d2bae` under WSL, pinned and alternated, 5 to 10 rounds, the ratios of the minimum and of the median: hull `sphere` D2 10^4 1.02 and 1.00, 10^5 1.06 and 1.03; hull `cube` D2 10^6 1.00 and 1.01; hull `sphere` D3 10^4 1.02 and 1.02; hull `cube` D3 10^5 0.97 and 1.00; hull `cubesurf` D3 10^5 0.99 and 0.99; hull `cube` D4 10^5 0.99 and 0.98; hull `grid` D6 10^4 1.00 and 1.02; Delaunay `cube` D2 10^4 1.00 and 1.02, `sphere` D2 10^4 1.01 and 1.00, `cube` D2 10^5 0.98 and 0.99, `cube` D3 10^4 1.00 and 1.02, 10^5 0.99 and 0.98; Voronoi `cube` D2 10^5 1.04 and 1.03.

The four that read above 1.02 were timed again over 10 to 14 rounds, in default builds and with one codegen unit:

| Set | Default build | One codegen unit |
| --- | ---: | ---: |
| Hull `sphere` D2 10^5 | 0.97, 1.01 | 1.01, 1.03 |
| Voronoi `cube` D2 10^5 | 1.02, 1.00 | 0.99, 1.01 |
| Hull `sphere` D3 10^4 | 0.98, 1.01 | 0.99, 1.00 |
| Delaunay `cube` D3 10^4 | 1.01, 1.01 | 1.00, 1.01 |

Not timed on Windows at scale 1 beyond the first column of the tables above, which reads the same on both sides.

`tests/alloc.rs`: the two cases whose coordinates have exponents far apart are scaled (their largest coordinate is near 1e150 and one scale holds the smallest), and count one allocation more at the small size, 145 for 144 and 121 for 120, inside the allowance. No record is changed.

A parity run follows this change (the owner's order: no benchmark before the code it measures is final).

## The parity run after the input's exact scaling, PR #610 (#609)

One run of the 41 sets on `main` at `327e5a1`, after #606. No parity set is scaled: their coordinates are in [-1, 1) or small integers. The run checks that what #606 put before acceptance costs nothing that shows. Method: `docs/verification.md`, `benches/parity/`; WSL2, Qhull 2020.2, CGAL 5.4, `rustc` 1.99.0, `g++` 11.4.0.

### Judgement

| Threshold | Met of the 38 judged sets | Hull | Delaunay | Run of #591, before #606 |
| --- | ---: | ---: | ---: | ---: |
| 1.05 | 36 | 25 | 11 | 36 |
| 1.00 | 30 | 22 | 8 | 29 |

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.44 ms | 0.78 | 0.92 | 0.79 | met | met |
| Hull `cube` D2 10^5 | 4.49 ms | 0.81 | 1.05 | 0.82 | met | met |
| Hull `cube` D2 10^6 | 53.3 ms | 0.80 | 1.21 | 0.97 | met | met |
| Hull `cube` D3 10^4 | 1.17 ms | 0.51 | – | 0.79 | met | met |
| Hull `cube` D3 10^5 | 9.81 ms | 0.38 | – | 0.86 | met | met |
| Hull `cube` D3 10^6 | 136 ms | 0.25 | – | 0.97 | met | met |
| Hull `cube` D4 10^4 | 5.05 ms | 0.12 | – | 0.77 | met | met |
| Hull `cube` D4 10^5 | 31.4 ms | 0.06 | – | 0.94 | met | met |
| Hull `cube` D5 10^4 | 53.0 ms | 0.24 | – | 0.73 | met | met |
| Hull `cube` D5 10^5 | 206 ms | 0.13 | – | 0.67 | met | met |
| Hull `cube` D6 10^4 | 574 ms | 0.28 | – | 0.47 | met | met |
| Hull `cube` D6 10^5 | 2.60 s | 0.23 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.86 ms | 1.02 | 1.15 | 0.11 (ref.) | met | not met |
| Hull `sphere` D2 10^5 | 9.17 ms | 0.93 | 1.14 | 0.08 (ref.) | met | met |
| Hull `sphere` D2 10^6 | 106 ms | 0.67 | 1.14 | 0.05 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 21.4 ms | 0.86 | – | 1.05 | met | not met |
| Hull `sphere` D3 10^5 | 265 ms | 0.45 | – | 0.88 | met | met |
| Hull `sphere` D3 10^6 | 3.58 s | 0.40 | – | 1.00 | met | met |
| Hull `sphere` D4 10^4 | 93.2 ms | 0.74 | – | 0.95 | met | met |
| Hull `sphere` D4 10^5 | 1.18 s | 0.94 | – | 0.88 | met | met |
| Hull `sphere` D5 10^4 | 817 ms | 0.83 | – | 0.97 | met | met |
| Hull `sphere` D5 10^5 | 11.38 s | 1.00 | – | 0.87 | met | not met |
| Hull `sphere` D6 10^4 | 6.88 s | 0.65 | – | 0.76 | met | met |
| Hull `cubesurf` D3 10^5 | 28.4 ms | 0.76 | – | 0.97 | met | met |
| Hull `grid` D6 10^4 | 224 ms | 0.90 (ref.) | – | 0.65 | met | met |
| Delaunay `cube` D2 10^4 | 5.50 ms | 1.07 | 1.26 | 0.27 | not met | not met |
| Delaunay `cube` D2 10^5 | 53.5 ms | 0.94 | 1.22 | 0.23 | met | met |
| Delaunay `cube` D2 10^6 | 540 ms | 0.80 | 1.22 | 0.17 | met | met |
| Delaunay `sphere` D2 10^4 | 4.86 ms | 1.12 | 1.29 | 0.21 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 45.4 ms | 1.01 | 1.33 | 0.11 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 404 ms | 0.82 | 1.38 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 36.1 ms | 1.14 | 1.20 | 0.39 | reported | reported |
| Delaunay `cube` D3 10^5 | 396 ms | 1.09 | 1.23 | 0.30 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.17 s | 1.08 | 1.22 | 0.26 | reported | reported |
| Delaunay `sphere` D3 10^4 | 66.6 ms | 0.52 | 0.52 | 0.59 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 646 ms | 0.68 | 0.70 | 0.35 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.47 s | 0.82 | 0.85 | 0.33 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 689 ms | 1.05 | 1.11 | 0.88 | met | not met |
| Delaunay `sphere` D4 10^4 | 877 ms | 0.35 | 0.34 | 0.68 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.37 s | 1.03 | 1.08 | 0.89 | met | not met |
| Delaunay `sphere` D5 10^4 | 11.16 s | 0.31 | 0.32 | 0.74 (ref.) | met | met |

**Delaunay `cube` D3, reported:** convx's construction alone reads 0.92, 0.89, and 0.91 of CGAL's whole time at 10^4, 10^5, and 10^6.

### The sets run again

Two sets had a slowest build 1.5 times the fastest or more (hull `sphere` D3 10^5, 5.1 times, and D5 10^5), and four at the edge of 1.05 were run with them. The table above has the second reading. With the first readings 32 sets are met at 1.05, with the second 36.

| Set | First: `build()` (fastest to slowest), / CGAL, / Qhull | Again |
| --- | --- | --- |
| Hull `sphere` D2 10^4 | 0.84 ms (0.72 ms to 1.07 ms), 1.07, 0.12 | 0.86 ms (0.78 ms to 1.30 ms), 1.02, 0.11 |
| Hull `sphere` D3 10^4 | 20.2 ms (18.5 ms to 20.9 ms), 0.91, 1.10 | 21.4 ms (19.4 ms to 24.9 ms), 0.86, 1.05 |
| Hull `sphere` D3 10^5 | 259 ms (242 ms to 1.24 s), 0.59, 0.97 | 265 ms (243 ms to 385 ms), 0.45, 0.88 |
| Hull `sphere` D5 10^5 | 11.95 s (10.54 s to 16.85 s), 1.08, 1.07 | 11.38 s (10.41 s to 12.06 s), 1.00, 0.87 |
| Delaunay `cube` D2 10^4 | 5.05 ms (4.82 ms to 5.82 ms), 1.05, 0.27 | 5.50 ms (5.33 ms to 6.41 ms), 1.07, 0.27 |
| Delaunay `sphere` D2 10^5 | 42.8 ms (41.2 ms to 45.5 ms), 1.05, 0.12 | 45.4 ms (42.3 ms to 49.7 ms), 1.01, 0.11 |

The references move between two readings minutes apart as convx does: on hull `sphere` D3 10^4 convx's median went from 20.2 to 21.4 ms and its ratio to Qhull from 1.10 to 1.05, so Qhull's own median moved by a tenth. After the second readings two sets still have a slowest build 1.5 times the fastest (hull `sphere` D2 10^4, 1.66, and D3 10^5, 1.58), and 17 of the 41 one of 1.2 times or more.

### What is not met

| Set | Ratio | Reason |
| --- | --- | --- |
| Delaunay `cube` D2 10^4 | 1.07 against CGAL (1.05 in the first reading) | 1.04 to 1.13 over nine runs, with no change to its code |
| Delaunay `sphere` D2 10^4 | 1.12 against CGAL | 1.04 to 1.17 over eight runs in a row (#512) |

Hull `sphere` D3 10^4 reads 1.046 against Qhull (1.10 in the first reading), Delaunay `cube` D4 10^4 1.047, and Delaunay `cube` D5 10^4 1.03.

### Reading

- The judgement is that of the run before #606: 36 at 1.05, and the same two sets not met.
- Hull `sphere` D3 10^4 read 1.10 once. It was timed directly against `df80378` for #606, under WSL over 14 rounds: 0.98 by the minimum and 1.01 by the median in default builds, 0.99 and 1.00 with one codegen unit (the section of #589). That change did not move it; its ratio to Qhull has read 0.98 to 1.10 over the last five readings.
- Nothing is claimed for the parity sets from #606: it is about inputs far from 1, and none of these is.

### A fault in the tools, fixed here

The first attempt at this run did not start. #606 changed the first lines of the hull's and the triangulation's `build()`, and `benches/parity/marks.py` no longer found the two lines it writes its marks beside, as its README says can happen. My script that started the run piped the setup into `tail`, which hid the failure, and the runner then stopped on a missing program. The report I read next was the file of the run of #591, left in the scratch; its numbers equalled that run's to the digit, which is how it showed. `marks.py` finds the lines as they are now, and the run was started again with a check that both programs exist.

## A zero proved by the first stage's own arithmetic, PR #616 (#615, spike #614)

### What the spike found

The trial of the comparison's adapters (#613) had a row no parity set has: the Delaunay triangulation of `rbox 10000 M3,4 D2`, a rotated 100 x 100 integer lattice, at 18.3 ms against CGAL's 5.8 ms, where 10^5 uniform sites are level. `main` at `04e1612`, release, Windows, pinned, a scratch copy with timers around the phases and counters in the predicates' last decision, the mean of 20 builds in ms (the timers cost time; the shares matter):

| Input | Build | Acceptance | Insertion | Pass over the faces | Singles' draft | Placing of the groups | Finish |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10,000 uniform sites, D2 | 5.8 | 0.16 | 4.07 | 0.33 | 0.34 | 0 | 0.12 |
| The lattice 100 x 100, D2 | 35.2 | 0.17 | 24.62 | 0.46 | 0.23 | 7.10 | 1.86 |
| The same rotated by (3, 4) | 26.4 | 0.17 | 16.60 | 0.43 | 0.28 | 6.29 | 1.89 |
| 10,648 uniform sites, D3 | 39.9 | 0.17 | 32.80 | 3.76 | 1.47 | 0 | 0.42 |
| The lattice 22^3, D3 | 233.6 | 0.26 | 141.73 | 18.12 | 0.73 | 62.18 | 9.41 |

Every predicate the exact stage decided on a lattice was a zero: four cocircular sites, or a site on a line or a plane of the lattice. No bound certifies a zero, so each one ran the first stage, the double-double stage, and the exact stage, about a microsecond. On the lattice of D = 3 those calls took 137 of the 234 ms.

### The change

Where the first stage left the sign open and no column is zero, a zero is returned when the stage's arithmetic was exact: every difference formed without a rounding, all of them integers times one power of two and `w` bits wide, with `ceil(log2 k!) + k w <= 52` for an orientation and `ceil(log2 k!) + 2 + (k + 1) w <= 52` for a lifted one (design §1; the proof is at `semi_static::exact_zero`). A path that certifies a sign does not reach it.

Counts for one build, `8c8e008` and the head `cd05b1f`:

| Input | Decided by the exact stage, before | After | Zeros proved by exactness | Zeros by a zero column |
| --- | ---: | ---: | ---: | ---: |
| Uniform sites, D2 and D3 | 0 | 0 | 0 | 0 |
| Lattice 100 x 100, D2 | 17,337 | 0 | 17,337 | 13,795 |
| Rotated by (3, 4) | 31,978 | 0 | 31,978 | 0 |
| Lattice 316 x 316 rotated | 313,980 | 0 | 313,980 | 6 |
| The same times 1000 | 313,980 | 5,974 | 308,006 | 6 |
| Lattice 100 x 100 times 0.1 | 12,446 | 12,446 | 0 | 13,841 |
| Lattice 22^3, D3 | 124,996 | 0 | 124,996 | 100,107 |
| Lattice 22^3 rotated | 186,582 | 0 | 186,582 | 28,715 |
| Lattice 8^4, D4 | 233,352 | 0 | 233,352 | 285,698 |
| Hull, lattice 30^3 rotated | 4,269 | 0 | 4,269 | 3,191 |

The before column of the rows the spike did not count is the sum of the two after it. A lattice times 0.1 has coordinates with 52 binary digits and is not helped; times 1000 its differences are wider by ten bits, and the widest leave the bound.

### Time

Windows, release, pinned, the best of 24 builds over three alternated rounds, `8c8e008` against the head's sources. The whole `Debug` form of every result has one hash on both sides.

| Input | `8c8e008` | Head | Ratio |
| --- | ---: | ---: | ---: |
| Delaunay, 10,000 uniform sites, D2 | 5.20 ms | 5.22 | 1.00 |
| Delaunay, lattice 100 x 100 | 15.49 | 9.85 | 0.64 |
| The same rotated by (3, 4) | 17.22 | 10.18 | 0.59 |
| The same times 0.1 | 16.30 | 16.50 | 1.01 |
| Delaunay, lattice 316 x 316 rotated | 182.27 | 109.88 | 0.60 |
| The same times 1000 | 183.29 | 112.61 | 0.61 |
| Delaunay, 10,648 uniform sites, D3 | 37.55 | 37.38 | 1.00 |
| Delaunay, lattice 22^3 | 175.59 | 82.20 | 0.47 |
| The same rotated | 171.59 | 83.99 | 0.49 |
| Delaunay, lattice 8^4, D4 | 675.50 | 240.47 | 0.36 |
| Voronoi, lattice 100 x 100 | 24.40 | 18.59 | 0.76 |
| Hull, lattice 30^3 rotated | 3.55 | 3.09 | 0.87 |
| Hull, lattice 12^4, D4 | 8.10 | 7.16 | 0.88 |
| Hull, 100,000 uniform points, D3 | 10.63 | 10.68 | 1.00 |
| Delaunay, 100,000 uniform sites, D2 | 50.93 | 50.92 | 1.00 |

Under WSL, pinned and alternated, `8c8e008` against `cd05b1f`, the ratios of the minimum and of the median. The lattices are `rbox 10000 M3,4 D2`, `rbox 27000 M3,4,5 D3`, and `rbox 10000 M1,0 D4`:

| Set | Builds a side | `8c8e008` min | `cd05b1f` min | Ratios |
| --- | ---: | ---: | ---: | ---: |
| Delaunay lattice D2 | 64 | 15.66 ms | 9.18 | 0.59, 0.59 |
| Delaunay lattice D3 | 12 | 464.0 | 215.5 | 0.46, 0.49 |
| Delaunay lattice D4 | 4 | 1,819.4 | 673.1 | 0.37, 0.37 |
| Hull of the lattice D3 | 48 | 2.67 | 2.35 | 0.88, 0.86 |
| Delaunay `cube` D2 10^4 | 80 | 4.85 | 4.86 | 1.00, 1.00 |
| Delaunay `sphere` D2 10^4 | 96 | 4.72 | 4.78 | 1.01, 1.02 |
| Delaunay `cube` D2 10^5 | 16 | 52.37 | 51.71 | 0.99, 1.02 |
| Delaunay `cube` D3 10^4 | 24 | 34.33 | 34.56 | 1.01, 0.98 |
| Delaunay `cube` D3 10^5 | 6 | 388.5 | 385.0 | 0.99, 0.97 |
| Delaunay `sphere` D3 10^4 | 20 | 67.34 | 67.71 | 1.01, 0.99 |
| Hull `sphere` D3 10^4 | 50 | 19.02 | 19.04 | 1.00, 1.01 |
| Hull `grid` D6 10^4 | 10 | 227.6 | 203.0 | 0.89, 0.91 |

Four guards read above 1.02 in that run (Delaunay `cube` D4 3,000 1.04 and 1.03 over 5 builds, hull `sphere` D2 10^5 1.08 and 1.03, hull `cube` D3 10^5 1.04 and 1.02, hull `cubesurf` D3 10^5 1.02 and 1.06) and were timed again, in the default build and with one codegen unit:

| Set | Builds a side | Default build | One codegen unit |
| --- | ---: | ---: | ---: |
| Delaunay `cube` D4 3,000 | 14 | 1.02, 0.99 | 0.98, 0.98 |
| Hull `sphere` D2 10^5 | 70 | 1.01, 1.02 | 0.96, 0.98 |
| Hull `cube` D3 10^5 | 70 | 1.02, 1.00 | 0.97, 0.99 |
| Hull `cubesurf` D3 10^5 | 60 | 1.00, 1.01 | 1.00, 0.99 |

### A slowdown of the first form, removed

The first form of the test (`317cda2`) read every difference before its bound refused the width. A difference of two `f64` near each other is often exact, so coordinates with 52 binary digits passed the first check, and sites on a sphere, which fail the first stage at almost every in-sphere test, paid for the whole scan: Delaunay `sphere` D3 10^4 read 1.07 and 1.10 of `8c8e008` under WSL (12 builds a side) and hull `sphere` D3 10^4 1.04 and 1.04. The width is now tested at each difference against the widest the bound allows, and such an input leaves at its first nonzero difference; the rows above are of that form.

### What remains on a lattice

With the zeros proved, the insertion on a lattice takes the time it takes on as many uniform sites. The rest of the excess (10 ms against 5.2 in D = 2, 82 against 37 in D = 3 on Windows) is the cospherical groups themselves: collecting and sorting a group's sites, the placing of each group (about a third of that block; 33 of 87 ms in D = 3), and the finish of the draft, about 3.7 and 2.0 ms of the build in D = 2 by the scratch timers. It is put to the owner whether a row reduces it before the comparison is published.

The parity sets gain two Delaunay lattices, reported and not judged (ADR 0006, amendment of 2026-10-13). A parity run follows this change.

## The comparison's data screened once against CGAL and Qhull, #618

Every data file of `benches/compare/data` (rbox, the distributions of PBBS, the Stanford scans) through `benches/compare/runner.py` once, on `cdfa5a5` (the head of #616; `main` at `883ca77` has the same crate), under WSL, pinned. Times are the best build in ms: 15 builds a library under 1.5 s a build, 3 above. It is a screening, taken to find what to look at before the comparison is run, and nothing of it is published: the machine was not idle for all of it (another process used a quarter to a half of the CPU at times), and a row of three builds read up to 0.15 off when it was timed again.

A ratio above 1.05, the parity's margin, is in bold (the owner's word of 2026-10-13: a row at 1.2 is looked at too).

| Hull of | convx | CGAL | Qhull | convx / the faster | Peak memory, MB |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cube-d2-n10000` | 0.35 | 0.70 | 0.61 | 0.57 | 8 / 8 / 8 |
| `cube-d2-n100000` | 3.13 | 8.06 | 5.27 | 0.59 | 9 / 10 / 9 |
| `cube-d2-n1000000` | 52.0 | 57.9 | 56.6 | 0.92 | 56 / 55 / 32 |
| `insphere-d2-n100000` | 7.50 | 4.21 | 6.14 | **1.78** | 9 / 9 / 9 |
| `insphere-d2-n1000000` | 103 | 63.9 | 73.2 | **1.60** | 55 / 53 / 32 |
| `kuzmin-d2-n100000` | 2.51 | 0.89 | 3.77 | **2.81** | 9 / 9 / 9 |
| `kuzmin-d2-n1000000` | 54.4 | 11.4 | 45.8 | **4.76** | 55 / 43 / 25 |
| `kuzmin-d2-n10000000` | 531 | 2,181 | 390 | **1.36** | 527 / 644 / 308 |
| `lattice-d2-n10000` | 0.36 | 0.11 | 0.46 | **3.30** | 8 / 8 / 8 |
| `near-d2-n10000` | 0.35 | 0.66 | 0.58 | 0.61 | 8 / 8 / 8 |
| `sphere-d2-n10000` | 0.82 | 0.78 | 7.27 | **1.05** | 8 / 8 / 8 |
| `sphere-d2-n100000` | 8.54 | 9.32 | 106 | 0.92 | 13 / 12 / 29 |
| `sphere-d2-n1000000` | 99.5 | 163 | 2,159 | 0.61 | 104 / 80 / 299 |
| `armadillo-d3-n172974` | 28.2 | 54.9 | 22.2 | **1.27** | 22 / 21 / 9 |
| `buddha-d3-n543652` | 102 | 190 | 70.8 | **1.45** | 70 / 55 / 25 |
| `bunny-d3-n35947` | 7.73 | 10.8 | 7.84 | 0.99 | 9 / 9 / 9 |
| `cube-d3-n10000` | 1.41 | 2.70 | 1.52 | 0.93 | 8 / 8 / 8 |
| `cube-d3-n100000` | 10.4 | 26.9 | 10.6 | 0.99 | 10 / 14 / 9 |
| `cube-d3-n1000000` | 158 | 723 | 123 | **1.29** | 108 / 96 / 42 |
| `dragon-d3-n437645` | 67.2 | 129 | 48.9 | **1.37** | 53 / 45 / 20 |
| `insphere-d3-n100000` | 16.3 | 35.5 | 16.6 | 0.99 | 10 / 15 / 9 |
| `insphere-d3-n1000000` | 241 | 1,300 | 256 | 0.94 | 115 / 98 / 45 |
| `lattice-d3-n27000` | 2.46 | 4.10 | 1.99 | **1.24** | 9 / 9 / 9 |
| `near-d3-n10000` | 1.37 | 2.70 | 1.61 | 0.85 | 8 / 8 / 8 |
| `near-d3-n100000` | 9.98 | 23.3 | 10.5 | 0.95 | 10 / 14 / 9 |
| `plummer-d3-n100000` | 10.3 | 15.6 | 5.04 | **2.05** | 10 / 14 / 9 |
| `plummer-d3-n1000000` | 191 | 346 | 52.4 | **3.64** | 109 / 96 / 41 |
| `sphere-d3-n10000` | 21.0 | 25.7 | 21.8 | 0.96 | 10 / 11 / 8 |
| `sphere-d3-n100000` | 246 | 394 | 236 | 1.04 | 72 / 80 / 53 |
| `sphere-d3-n1000000` | 3,930 | 9,869 | 3,937 | 1.00 | 710 / 734 / 505 |
| `cube-d4-n10000` | 5.20 | 49.2 | 6.85 | 0.76 | 8 / 11 / 8 |
| `cube-d4-n100000` | 29.5 | 567 | 31.4 | 0.94 | 13 / 55 / 9 |
| `lattice-d4-n10000` | 3.99 | 53.3 | 1.93 | **2.07** | 8 / 13 / 8 |
| `sphere-d4-n10000` | 105 | 125 | 130 | 0.85 | 29 / 31 / 19 |
| `sphere-d4-n100000` | 1,229 | 1,249 | 1,446 | 0.98 | 243 / 270 / 172 |
| `cube-d5-n10000` | 54.2 | 243 | 110 | 0.49 | 12 / 30 / 10 |
| `cube-d5-n100000` | 207 | 1,761 | 397 | 0.52 | 30 / 132 / 25 |
| `sphere-d5-n10000` | 869 | 1,051 | 930 | 0.93 | 112 / 165 / 81 |
| `cube-d6-n10000` | 649 | 2,386 | 2,592 | 0.27 | 76 / 220 / 75 |
| `sphere-d6-n10000` | 7,304 | 10,083 | 9,056 | 0.81 | 611 / 1087 / 450 |
| `cube-d7-n1000` | 908 | - | 2,989 | 0.30 | 104 / 104 |
| `cube-d8-n1000` | 9,864 | - | 37,960 | 0.26 | 642 / 650 |

| Delaunay of | convx | CGAL | Qhull | convx / the faster | Peak memory, MB |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cube-d2-n10000` | 5.32 | 4.91 | 21.7 | **1.08** | 8 / 8 / 8 |
| `cube-d2-n100000` | 52.7 | 51.4 | 404 | 1.02 | 33 / 43 / 62 |
| `cube-d2-n1000000` | 563 | 728 | 5,916 | 0.77 | 276 / 385 / 599 |
| `insphere-d2-n100000` | 52.2 | 51.6 | 235 | 1.01 | 33 / 43 / 53 |
| `insphere-d2-n1000000` | 557 | 710 | 3,660 | 0.79 | 276 / 384 / 505 |
| `kuzmin-d2-n100000` | 59.5 | 52.7 | 636 | **1.13** | 33 / 43 / 41 |
| `kuzmin-d2-n1000000` | 2,922 | 773 | 5,035 | **3.78** | 279 / 384 / 93 |
| `lattice-d2-n10000` | 10.2 | 5.55 | 44.2 | **1.83** | 8 / 8 / 10 |
| `near-d2-n10000` | 5.58 | 5.21 | 34.9 | **1.07** | 8 / 8 / 8 |
| `sphere-d2-n10000` | 5.23 | 4.91 | 30.6 | **1.07** | 8 / 8 / 8 |
| `sphere-d2-n100000` | 41.6 | 39.1 | 342 | **1.06** | 28 / 40 / 61 |
| `sphere-d2-n1000000` | 416 | 524 | 5,239 | 0.79 | 234 / 355 / 591 |
| `armadillo-d3-n172974` | 685 | 618 | 2,397 | **1.11** | 155 / 239 / 303 |
| `buddha-d3-n543652` | 2,541 | 2,122 | 14,390 | **1.20** | 504 / 760 / 1189 |
| `bunny-d3-n35947` | 129 | 118 | 415 | **1.09** | 35 / 53 / 65 |
| `cube-d3-n10000` | 34.4 | 31.5 | 93.1 | **1.09** | 11 / 17 / 19 |
| `cube-d3-n100000` | 374 | 343 | 2,213 | **1.09** | 92 / 136 / 209 |
| `cube-d3-n1000000` | 4,191 | 3,827 | 27,100 | **1.10** | 824 / 1314 / 2078 |
| `dragon-d3-n437645` | 2,114 | 1,769 | 9,469 | **1.20** | 396 / 613 / 793 |
| `insphere-d3-n100000` | 374 | 351 | 1,326 | **1.07** | 92 / 136 / 172 |
| `insphere-d3-n1000000` | 4,249 | 3,830 | 16,420 | **1.11** | 823 / 1313 / 1696 |
| `lattice-d3-n27000` | 212 | 131 | 890 | **1.63** | 53 / 35 / 36 |
| `near-d3-n10000` | 36.0 | 35.4 | 141 | 1.01 | 11 / 17 / 22 |
| `near-d3-n100000` | 386 | 374 | 2,149 | 1.03 | 89 / 132 / 206 |
| `plummer-d3-n100000` | 397 | 369 | 2,208 | **1.08** | 91 / 137 / 206 |
| `plummer-d3-n1000000` | 4,350 | 4,109 | 30,870 | **1.06** | 826 / 1317 / 2082 |
| `sphere-d3-n10000` | 66.8 | 119 | 118 | 0.57 | 9 / 14 / 21 |
| `sphere-d3-n100000` | 622 | 817 | 1,669 | 0.76 | 58 / 101 / 185 |
| `sphere-d3-n1000000` | 5,648 | 6,063 | 21,780 | 0.93 | 542 / 961 / 1827 |
| `cube-d4-n10000` | 699 | 671 | 856 | 1.04 | 41 / 68 / 80 |
| `cube-d4-n100000` | 8,856 | 7,555 | 23,760 | **1.17** | 399 / 655 / 1128 |
| `lattice-d4-n10000` | 660 | 692 | 1,691 | 0.95 | 69 / 45 / 44 |
| `sphere-d4-n10000` | 865 | 2,437 | 1,344 | 0.64 | 28 / 45 / 97 |
| `cube-d5-n10000` | 8,371 | 7,152 | 8,981 | **1.17** | 228 / 391 / 449 |

The Delaunay triangulation of `kuzmin-d2-n10000000` is not in the table: one build of convx had not ended after 17 minutes, and the row was stopped. CGAL built it in 11.0 s.

Counts. convx and CGAL, both exact, give the same sites and simplices on every triangulation, and the same vertices and facets on every hull but three. Where the facets differ, CGAL counts what convx merges: triangles for a facet of four coplanar points (Buddha: 12,358 against 12,357; the lattice of D = 3: 12 against 6), and in D = 4 the lattice's coplanar points as vertices (5,904 against 16). Qhull reports fewer vertices, facets, or regions on 35 rows, as the comparison's README says it can; on the lattices its regions are the merged ones (9,801 for 19,602 triangles).

### What is above 1.05, by kind

**1. Delaunay of Kuzmin sites, D = 2: more than linear.** 1.13 of CGAL at 10^5, 3.78 at 10^6, and no end at 10^7. The insertion order sorts each round by a Morton code of the coordinates quantized on a uniform grid over the bounding box (2^21 cells an axis in D = 2). Kuzmin's radius has a long tail: the box of 10^6 sites is about 10^6 wide, half the sites are within a radius of 1.7, and they fall into a few cells, where the order is by index. A scratch copy with counters: 105 steps of the walk a site at 10^6, against 5.5 on uniform sites. A scratch copy whose cells are the ranks of the coordinates, which follow the density, built the 10^6 in 749 ms for 2,881 (Windows; CGAL 773 ms under WSL), and cost 2 to 16% on uniform sites for its two sorts. So the cause is the grid, and the fix is an order that refines a crowded cell, at no cost where no cell is crowded.

**2. Delaunay in D = 3, uniform and scans: 1.04 to 1.18 of CGAL.** Uniform sites read 1.06 to 1.10 from 10^4 to 10^6 (three sizes that read 1.16 to 1.24 with three builds read 1.08, 1.07, and 1.15 with six, and 437,645 sites read 1.04 in a second run of six). It is the gap of the parity's reported set, Delaunay `cube` D3 (ADR 0006). The two large scans are above it: Dragon 1.13 and Buddha 1.18 with six builds a side, beside 1.04 for as many uniform sites. Not exact zeros: of Dragon's predicates, 32,151 are left open by the first stage, 31,865 of them decided by a zero column, and 275 reach the exact stage. The walk takes 9.8 steps a site for 8.7 on uniform sites and a cavity has 21.8 simplices for 20.3. The rank order of kind 1 does not help (2,016 ms for 1,962). What the rest is, is not found here; a CPU profile beside CGAL would say. D = 4 and 5 read 1.17 on three builds (`cube-d4-n100000`, `cube-d5-n10000`) and 1.04 at `cube-d4-n10000`.

**3. Hulls of inputs whose points are nearly all deep inside: 1.3 to 4.8 of the faster.** Kuzmin D2 (2.8 at 10^5, 4.8 at 10^6, 1.4 at 10^7), a ball D2 (1.8, 1.6), Plummer D3 (2.1, 3.6), `cube` D3 at 10^6 (1.29), the scans (1.27 to 1.45 of Qhull). convx takes on them what it takes on a cube of the same size (52 to 54 ms for 10^6 points of D = 2); CGAL and Qhull are fast there, 11 ms for the Kuzmin, because a point inside the first few facets costs them one rejection. convx reads every point more than once before construction: it finds the duplicates of every point (design §3: the published hull has each point's representative, and its interior and coplanar points) and scales and culls. What that floor is made of by part, and whether a point culled strictly inside could skip the rest, is not measured here.

**4. Lattices.** Delaunay 1.83 in D = 2 and 1.63 in D = 3 after #615, 0.95 in D = 4: the cospherical groups (#617). The hull of the lattice of D = 2 reads 3.3 of CGAL (0.36 ms for 0.11) and of D = 4 2.1 of Qhull (4.0 ms for 1.9), on outputs of 4 and 16 vertices; D = 3 1.24. Small times, the same floor as kind 3.

**5. Small sets of D = 2, Delaunay: 1.06 to 1.08** (`cube`, `near`, `sphere` at 10^4 and `sphere` at 10^5). The two parity sets that are not met (Delaunay `cube` and `sphere` D2 10^4); at 10^6 convx reads 0.77 to 0.79 of CGAL.

At or under 1.05: the hulls of D = 4 to 8 (0.26 to 0.98), the hulls of `cube`, `sphere`, `near`, and a ball in D = 3 up to 10^5 (0.85 to 1.04), the triangulations of D = 2 at 10^5 to 10^6 but Kuzmin (0.77 to 1.02), and of sites on a sphere in D = 3 and 4 (0.57 to 0.93). Peak memory is under CGAL's on every triangulation but the lattices (level in D = 2, above in D = 3 and 4).

### Not covered, and why

- **A Voronoi diagram.** Qhull builds one (`qvoronoi`), and CGAL in D = 2; neither produces what convx publishes (cells, interfaces, and rays as lists), so a row would time different outputs. It is for the comparison's document to say, not for a row.
- **Sites that nearly coincide** beyond `near` (five within 1e-8), and **D >= 7** beyond two hulls of 1,000 points: no data file.

## The insertion order refines a crowded cell, PR #621 (#620)

The screening (#618) found the Delaunay triangulation of Kuzmin sites growing faster than linearly. The insertion order sorted each round by a Morton code on one uniform grid over the sites' bounding box, and a long tail put half the sites in a few cells, where consecutive insertions were far apart.

### The change

A cell with more than 16 sites of a round is ordered again by the Morton code on the bounding box of those sites, and so on while a cell is crowded. Where no cell is crowded the order is the order before, site for site, so the triangulations of such inputs are published in the same order as before. The list of crowded cells is allocated only when there is one.

Steps of the walk a site, counted in a scratch copy: 105 before and 5.4 after on 10^6 Kuzmin sites, beside 5.5 on 10^6 uniform sites. At 10^5 Kuzmin sites it is 8.6 after, and 8.2 with a threshold of 4 or of 1: what remains there is not the threshold's.

### Time

Under WSL, pinned and alternated, `883ca77` against `0d88144`, and CGAL's program on the same file between them; ms, the minimum, and the ratios of the minimum and of the median to `883ca77`:

| Delaunay of | Builds a side | `883ca77` | `0d88144` | Ratios | CGAL | convx / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10^5 Kuzmin sites, D2 | 18 | 58.52 | 57.16 | 0.98, 0.96 | 51.24 | 1.12 |
| 10^6 Kuzmin sites, D2 | 6 | 2,702.7 | 577.1 | 0.21, 0.22 | 697.3 | 0.83 |
| 10^7 Kuzmin sites, D2 | 3 | no end in 17 minutes (#618) | 6,471.3 | | 9,070.5 | 0.71 |
| 10^6 Plummer sites, D3 | 4 | 4,127.7 | 4,164.7 | 1.01, 0.99 | 3,764.8 | 1.11 |

The guards, the same two commits:

| Delaunay of | Builds a side | `883ca77` | `0d88144` | Ratios |
| --- | ---: | ---: | ---: | ---: |
| `rbox 10000 M3,4 D2` | 64 | 9.05 | 8.64 | 0.95, 0.98 |
| `rbox 27000 M3,4,5 D3` | 12 | 213.7 | 216.9 | 1.02, 1.01 |
| `cube` D2 10^4 | 80 | 4.70 | 4.67 | 0.99, 1.00 |
| `sphere` D2 10^4 | 80 | 4.66 | 4.67 | 1.00, 1.00 |
| `cube` D2 10^5 | 20 | 51.17 | 51.25 | 1.00, 1.00 |
| `cube` D2 10^6 | 8 | 537.5 | 537.3 | 1.00, 1.02 |
| `cube` D3 10^4 | 24 | 33.86 | 33.81 | 1.00, 0.99 |
| `cube` D3 10^5 | 8 | 365.3 | 366.8 | 1.00, 1.00 |
| `sphere` D3 10^4 | 16 | 65.80 | 66.27 | 1.01, 1.01 |
| `cube` D4 3,000 | 8 | 177.7 | 175.6 | 0.99, 0.99 |
| `sphere` D4 3,000 | 6 | 242.2 | 239.8 | 0.99, 0.99 |
| `cube` D5 1,000 | 6 | 417.5 | 417.4 | 1.00, 1.01 |

On Windows, not pinned, the best of two or three builds, `883ca77` against the first commit of the change (`1cf0249`): 10^6 Kuzmin sites 2,870 to 610 ms, 10^5 65.2 to 64.6; `cube` D2 10^6 571 and 585, `cube` D3 10^5 401 and 404, `sphere` D2 10^5 45.8 and 46.1, 10^6 Plummer sites 4,429 and 4,467. The machine was in use, and these are not a judgement.

`tests/alloc.rs` gains the Delaunay triangulation of sites with a long tail in D = 2 and 3 (49 and 59 allocations at both sizes); no record of another case is changed.

The parity sets gain the Delaunay triangulation of 10^6 Kuzmin sites, reported and not judged (ADR 0006, the second amendment of 2026-10-13).

## What a hull does to every point before and during construction, #622

The screening (#618) read hulls at 1.3 to 4.8 times the faster of CGAL and Qhull on inputs whose points are nearly all deep inside. A scratch copy of `0d88144` with a timer at each part; Windows, release, not pinned, the best of five builds, 10^6 points each, in ms:

| Hull of | Duplicates and validation | Order in space | Copy in that order | Construction | Classification | Publication | Sum | The faster other (WSL, #618) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Kuzmin, D2 | 32.3 | | | 6.2 | 1.8 | 0.8 | 41.2 | CGAL 11.4 |
| A ball, D2 | 32.4 | | | 28.2 | 34.3 | 0.8 | 95.7 | CGAL 63.9 |
| `cube`, D2 | 33.4 | | | 18.1 | 2.2 | 0.8 | 54.6 | Qhull 56.6 |
| Plummer, D3 | 26.3 | 14.7 | 5.6 | 109.6 | 6.5 | 0.8 | 164.3 | Qhull 52.4 |
| `cube`, D3 | 25.7 | 17.9 | 17.0 | 65.9 | 7.9 | 0.9 | 135.9 | Qhull 122.6 |

The scaling's sample is under 0.05 ms everywhere.

### Reading

**The duplicates are a floor of 26 to 33 ms for 10^6 points, whatever the points are.** In D = 2 it is the sort of every point by its coordinates, which also orders the chain; from D = 3 a hash of every point. It is three quarters of the Kuzmin hull, where CGAL's whole build takes a third of it: CGAL finds four extreme points, discards what is inside their quadrilateral, and sorts the rest. The published output needs it for every point: `representative` has an entry for each of the n points (design §3), so a point proved strictly inside still has its duplicates found. It is the price of that list, unless the list is made on demand, as the planes are.

**In D = 3 the points are ordered in space before any of them is discarded** (ADR 0009): 20 to 35 ms for 10^6 points, on inputs of which a few thousand at most are ever outside a facet. On the Plummer the order's grid is also the one of #620: one grid over a box with a long tail.

**The first simplex is not chosen.** It is the simplex on `spanning_points`, the first D + 1 points by index that span the space, which design §5 leaves free. On the Plummer it is a small simplex in the dense center, and construction reads 8.3 million points in its scans, eight passes over the input. With a first simplex of extreme points (the least and the greatest of a coordinate, the farthest from their line, the farthest from their plane), tried in the scratch copy: construction 112.5 to 68.7 ms, 7.0 million points read. On `cube` D3 at 10^6 it reads the same 8.5 million and takes 70.3 and 78.9 ms, the three passes that find the extremes included; on `sphere` D3 at 10^5 and `cube` D3 at 10^5 the build reads 256 and 266 ms, 12.9 and 11.9 ms. Qhull starts from such a simplex, and a point inside it costs Qhull D + 1 distances and nothing more.

**A ball in D = 2** spends 34 ms in classification, sixteen times the cube's: its hull has more vertices and its points lie near the boundary, where the polygon's cull proves less. Not looked at further here.

### What could be done, for the owner

1. **The first simplex from extreme points, D >= 3.** A third of the Plummer's construction; nothing on a cube. The design allows it; the published hull does not depend on it.
2. **Discard before ordering in space, D = 3.** Scan against the first simplex's facets first, and order in space only the points that are outside one. Removes most of the 20 to 35 ms on every input of D = 3 with points inside, and with (1) it is what Qhull does. ADR 0009 would be amended.
3. **Duplicates on demand.** `representative` computed when it is first read, as the planes are, with the duplicates among the vertices and the boundary points found during the build (those the output needs at once). Removes 26 to 33 ms for 10^6 points from every hull that does not read the list, and is the only way to CGAL's time on the Kuzmin of D = 2. It changes design §3 and the order in which failures are reported (a duplicate is found before a degenerate dimension is), so it is a Grill, and it would be a change to the public contract of 0.1.0.
4. **Classification of a ball in D = 2**: a spike of its own if (1) to (3) leave it above the margin.

Without (3), a hull of points nearly all inside stays above CGAL and Qhull by the cost of the duplicates, and the comparison says so with the reason: the output lists every point's representative, and the interior and the coplanar points, which theirs do not.

## The first simplex from extreme points, PR #626 (#624)

The spike #622 found construction in D >= 3 starting from the simplex on `spanning_points`, the first points by index that span the space, and reading an input dense about a center eight times.

### The change

For D >= 3 the first simplex is chosen on a sample of at most 4,096 representatives taken at one stride: the least and the greatest of the first coordinate, then, D - 1 times, the point farthest from the flat through those chosen. The distances are in `f64` and only choose; the simplex is taken when its exact orientation is not zero, and otherwise the simplex is the one on `spanning_points`. Every published list is as before: the suite's frozen results pass, and a test compares the two builds in D = 3 to 6.

A sample, and not every point: on `cube` D3 at 10^6 the three passes over every point cost what they saved (70.3 and 78.9 ms of construction in the scratch copy of #622).

### Time

Under WSL, pinned and alternated, `c96a715` (the branch's base, the crate of `0d88144`) against `4936253`; ms, the minimum, and the ratios of the minimum and of the median. Qhull's time is one run of `qconvex` on the same file, beside them and not alternated.

| Hull of | Builds a side | `c96a715` | `4936253` | Ratios | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10^6 Plummer points, D3 | 40 | 163.97 | 79.80 | 0.49, 0.50 | 49.2 |
| 10^5 Plummer points, D3 | 64 | 10.12 | 5.46 | 0.54, 0.55 | 5.5 |
| 10^6 points in a ball, D3 | 40 | 185.64 | 151.03 | 0.81, 0.87 | 218.6 |
| Dragon | 40 | 64.79 | 48.35 | 0.75, 0.75 | 50.3 |
| Buddha | 40 | 105.51 | 84.21 | 0.80, 0.82 | 74.0 |
| Armadillo | 40 | 26.79 | 19.67 | 0.73, 0.74 | 23.5 |
| `rbox 27000 M3,4,5 D3` | 64 | 2.40 | 1.98 | 0.83, 0.81 | 2.3 |
| `cube` D3 10^4 | 80 | 1.10 | 1.13 | 1.03, 0.99 | |
| `cube` D3 10^5 | 50 | 9.38 | 8.32 | 0.89, 0.90 | |
| `cube` D3 10^6 | 24 | 129.44 | 128.11 | 0.99, 0.95 | |
| `sphere` D3 10^4 | 50 | 19.03 | 18.61 | 0.98, 0.98 | |
| `sphere` D3 10^5 | 16 | 252.89 | 251.72 | 1.00, 0.99 | |
| `cubesurf` D3 10^5 | 40 | 26.53 | 25.93 | 0.98, 0.95 | |
| `cube` D4 10^4 | 64 | 4.98 | 4.71 | 0.95, 0.96 | |
| `cube` D4 10^5 | 40 | 31.59 | 24.90 | 0.79, 0.81 | |
| `sphere` D4 10^4 | 16 | 88.78 | 89.37 | 1.01, 1.02 | |
| `cube` D5 10^4 | 24 | 54.37 | 51.56 | 0.95, 0.93 | |
| `cube` D5 10^5 | 6 | 215.24 | 208.81 | 0.97, 0.98 | |
| `sphere` D5 10^4 | 4 | 845.90 | 840.27 | 0.99, 1.00 | |
| `cube` D6 10^4 | 5 | 618.12 | 597.20 | 0.97, 0.94 | |
| `grid` D6 10^4 | 12 | 201.19 | 173.46 | 0.86, 0.86 | |

Delaunay, which does not go through this code but on sites that are all on one sphere: `sphere` D3 10^4 1.02 and 1.02 over 12 builds, `cube` D3 10^4 0.99 and 1.01 over 18.

The scans, which the screening read at 1.27 to 1.45 of Qhull, are at 0.84 (Armadillo), 0.96 (Dragon), and 1.14 (Buddha) of it. The Plummer points are at 1.6 of Qhull at 10^6 and level at 10^5: what remains at 10^6 is the duplicates and the order in space before any point is discarded (#622, #625).

On Windows, not pinned, the best of five to sixteen builds, `c96a715` against the first commit of the change (`07b7e7b`): Plummer 10^6 170 to 88 ms, a ball 201 to 175, `cube` D3 10^5 11.4 to 10.2 and 10^6 137 and 137, `sphere` D3 10^4 20.0 and 20.2, `cube` D4 10^5 32.5 to 26.8, `sphere` D4 10^4 92 and 92, `cube` D5 10^4 55.0 to 53.2, `grid` D6 10^4 204 to 177.

### `tests/alloc.rs`

Construction now meets its facets in another order, so its buffers grow at other moments: of the 68 cases, 18 count more allocations or reallocations than their record somewhere and 22 only fewer, by a few each way (some of the falls are older than this change: a record may fall without being written) (the largest rise is `cubesurf` D3 at the large size, 343 reallocations for 319; the largest fall is a prism, 311 for 376). No count grows with the points: the large size stays under twice the small on every case. Every record is set to the counts of this commit (#624 is the Issue that raises them).

## The discard before the order in space, withdrawn after measurement (#625)

The spike #622 read 20 to 35 ms in ordering 10^6 points of D = 3 in space and proposed to discard first what the first simplex proves inside, and to order the rest.

### What was tried

In the space order's path (D = 3, from 2^17 representatives): the first simplex on the input, every representative assigned to it as a build in the input's order starts, the points proved strictly inside recorded as interior, and the copy in Morton order made of the others; then the build on the copy as before. `0c18d0f`, with a test of what the copy leaves out against the exact orientation; the suite passed.

### Time

Under WSL, pinned and alternated, `e782604` (the first simplex of #624) against `0c18d0f`; ms, the minimum, and the ratios of the minimum and of the median:

| Hull of | Builds a side | `e782604` | `0c18d0f` | Ratios |
| --- | ---: | ---: | ---: | ---: |
| 10^6 Plummer points | 40 | 70.55 | 63.09 | 0.89, 0.90 |
| 10^6 points in a ball | 40 | 144.44 | 174.61 | 1.21, 1.19 |
| Dragon | 40 | 46.74 | 55.01 | 1.18, 1.20 |
| Buddha | 40 | 78.56 | 90.91 | 1.16, 1.16 |
| Armadillo | 40 | 18.80 | 22.49 | 1.20, 1.18 |
| `cube` 2 x 10^5 | 40 | 17.71 | 22.18 | 1.25, 1.21 |
| `cube` 10^6 | 24 | 118.10 | 136.45 | 1.16, 1.15 |
| `cubesurf` 10^6 | 12 | 300.02 | 332.38 | 1.11, 1.10 |
| `sphere` 2 x 10^5 | 6 | 516.95 | 524.81 | 1.02, 1.03 |
| `sphere` 10^6 | 3 | 3,363.8 | 3,497.3 | 1.04, 1.03 |

Below 2^17 points, where the path is not taken, the two read the same: 10^5 Plummer points 1.03 and 0.99, `cube` 10^5 0.99 and 1.01, `sphere` 10^4 and 10^5 1.00 and 0.99, 1.01, `cubesurf` 10^5 0.99, the lattice 0.97.

### Why

A simplex holds little of most inputs. A tetrahedron holds a third of a cube at most and an eighth of a ball; of 600 uniform points in a cube the first simplex proved 68 inside. So the copy still held nearly every point, and the first pass over the input, in the input's order, was paid on top of the pass the build on the copy makes again. Only an input dense about a center has most of its points inside the first simplex, and the Plummer gained a tenth.

The spike's sentence that a few thousand points at most are ever outside a facet is about the hull once it has grown, not about its first simplex; the proposal read one as the other. Discarding against a hull of more facets before the order is another design (the build would run its first insertions in the input's order), and no row is opened for it here.

### Left as it is

The code is removed (`no-dead-code.mdc`); the branch's two commits are reverted. The 10^6 Plummer points stand at 70.6 ms against Qhull's 53.7 ms in this run (1.31; one run of `qconvex`), from 3.6 times in the screening.

## The duplicates on demand, measured for the Grill, and the hull of a ball in D = 2 (#622, #628)

### What the duplicates cost, by switching them off

For the Grill on making `representative` and `interior_points` on demand. A scratch copy of `e782604` (the first simplex of #624) whose hash of every point can be switched off, on inputs without duplicates: the time without it is the most the change could give. Windows, pinned, the best of 5 to 10 builds, two rounds, ms:

| Hull of | With the duplicates | Without | The faster other (WSL, #618 and #624) |
| --- | ---: | ---: | ---: |
| 10^6 Kuzmin points, D2 | 40.2, 41.3 | 23.2, 22.8 | CGAL 11.4 |
| 10^6 points in a ball, D2 | 103.5, 106.8 | 81.4, 78.2 | CGAL 63.9 |
| `cube` D2 10^6 | 54.4, 53.5 | 34.8, 36.1 | Qhull 56.6 |
| `cube` D2 10^5 | 4.5, 4.7 | 3.5, 3.6 | |
| `sphere` D2 10^6 | 105.7, 111.6 | 108.6, 106.9 | |
| 10^6 Plummer points, D3 | 88.9, 87.3 | 64.8, 64.7 | Qhull 49 to 54 |
| 10^6 points in a ball, D3 | 171.1, 166.8 | 149.7, 149.0 | Qhull 207 to 219 |
| `cube` D3 10^6 | 145.5, 131.3 | 111.7, 106.5 | Qhull 122.6 |
| `cube` D3 10^5 | 10.6, 10.3 | 8.6, 8.8 | |
| `sphere` D3 10^5 | 256.6, 271.2 | 261.6, 267.8 | |
| `cube` D4 10^5 | 26.6, 27.5 | 27.7, 25.1 | |
| `cube` D5 10^5 | 225.1, 235.0 | 225.2, 215.8 | |

17 to 25 ms for 10^6 points in D = 2 and 3, a fifth to two fifths of those builds, and nothing where the chain sorts every point, on a sphere, or from D = 4. It does not reach CGAL on the Kuzmin (23 ms against 11.4). A hull that kept a copy of the input to find the duplicates later would give part of it back.

### The hull of points in a ball, D = 2 (#628)

The same scratch copy with a count in the chain's classification; Windows, not pinned, the best of three builds:

| Hull of | Vertices | Points located on the polygon | The loop that locates them | The build |
| --- | ---: | ---: | ---: | ---: |
| 10^6 points in a ball | 343 | 99,366 | 35.3 ms | 101.6 ms |
| 10^5 points in a ball | 158 | 9,960 | 3.5 | 10.0 |
| `cube` 10^6 | 39 | 1,262 | 2.7 | 57.0 |
| `cube` 10^5 | 25 | 341 | 0.3 | 5.6 |

The polygon of the farthest points in eight directions, which discards interior points before the chain, is an octagon: it holds nine tenths of a disc, and the tenth outside it is 99,366 points, none of them a vertex but 343. Each is then located on the hull by `polygon_edge`: two orientations against the fan's first and last edges, a bisection of the fan from the first vertex by orientation, and one orientation against the edge found. With 343 vertices that is about eleven exact-sign calls a point, 355 ns, and a third of the build. On a cube 1,262 points are located and it does not show.

CGAL's `convex_hull_2` (Akl and Toussaint) discards against a quadrilateral of four extremes, sorts each of the four regions outside it, and scans each; a point that the scan drops is not looked at again, and it publishes the vertices only.

What could be done: the cycle is the lower chain and the upper chain, each sorted by the coordinates the chain sorted the points by. A point is between two consecutive vertices of each by a bisection on those coordinates, without a predicate, and one orientation against the edge of each chain there decides whether it is inside, on the lower edge, or on the upper one: two exact signs a point for eleven, and no change to what is published. Expected: about 30 of the 35 ms, and the 10^6 ball at about 1.1 of CGAL from 1.6. It is put to the owner.

## A point off the chain located by its coordinates, PR #631 (#630)

The spike #628 found the hull of points in a ball of D = 2 locating 99,366 of 10^6 points on its polygon by about eleven exact-sign calls each, a bisection of the fan from the first vertex by orientation: 35 ms of a 102 ms build.

### The change

The cycle is the lower chain and the upper chain, each ascending or descending by the key the chain sorted the points by. A point off the cycle is placed between two consecutive vertices of each chain by a bisection on that key, which asks no predicate, and one orientation against each of the two edges decides whether it is on an edge. What is published is as before; the search before it is kept as the reference of the test.

The keys of the cycle's vertices are copied when at least as many points are to be located as the cycle has vertices, and read from the input otherwise.

### Time

Under WSL, pinned and alternated, `5067600` (the branch's base, the crate of `e782604`) against `e6c7c92`, and CGAL's program on the same file between them; ms, the minimum, and the ratios of the minimum and of the median to `5067600`:

| Hull of, D2 | Builds a side | `5067600` | `e6c7c92` | Ratios | CGAL | convx / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10^6 points in a ball | 40 | 88.12 | 64.61 | 0.73, 0.74 | 55.93 | 1.16 |
| 10^5 points in a ball | 80 | 7.21 | 5.10 | 0.71, 0.71 | 4.19 | 1.22 |
| 10^6 Kuzmin points | 70 | 34.14 | 34.45 | 1.01, 1.00 | 9.56 | 3.60 |
| 10^5 Kuzmin points | 128 | 2.35 | 2.41 | 1.03, 0.99 | 0.88 | 2.74 |
| `rbox 10000 M3,4 D2` | 128 | 0.30 | 0.29 | 0.98, 0.97 | 0.10 | 2.9 |
| `cube` 10^4 | 128 | 0.39 | 0.38 | 0.97, 0.97 | | |
| `cube` 10^5 | 128 | 3.67 | 3.66 | 1.00, 0.98 | | |
| `cube` 10^6 | 70 | 46.97 | 47.22 | 1.01, 1.00 | | |
| `sphere` 10^4 | 128 | 0.68 | 0.68 | 1.00, 1.01 | | |
| `sphere` 10^5 | 70 | 8.13 | 8.01 | 0.99, 1.00 | | |
| `sphere` 10^6 | 30 | 89.13 | 91.73 | 1.03, 0.99 | | |
| `grid` 10^5 | 112 | 1.40 | 1.43 | 1.02, 1.01 | | |
| `cube` D3 10^5 (not touched) | 50 | 8.00 | 8.10 | 1.01, 0.99 | | |

The two whose minimum read above 1.02 were timed again, in the default build and with one codegen unit:

| Set | Builds a side | Default build | One codegen unit |
| --- | ---: | ---: | ---: |
| 10^5 Kuzmin points | 192 | 0.99, 1.00 | 1.02, 1.00 |
| `sphere` 10^6 | 48 | 1.05, 1.00 | 1.02, 1.01 |

The medians are at 1.00. The minimum of `sphere` at 10^6 moves by 0.03 to 0.05 from one run to the next on both sides (its builds spread from 89 to 104 ms), and the change adds to that build a bisection of the cycle and nothing a point.

The ball is at 1.16 and 1.22 of CGAL, from 1.58 and 1.72. The Kuzmin and the lattice are where they were: what they take is the duplicates of every point (#622), which stay in the build by the owner's decision of 2026-10-13.

### Forms that were measured and left

- **The keys of every vertex copied, always**: the ball at 0.70, and `sphere` D2 at 1.14, 1.14, and 1.36 of its time at 10^4, 10^5, and 10^6. A hull whose every point is a vertex has as many vertices as points and locates none.
- **The keys read from the input, always**: `sphere` back at 1.00 to 1.05, the ball at 0.78.
- **A remainder for each key read** (the place in the cycle taken modulo its length): with the keys copied where points are located, the ball stayed at 0.79 and 0.76; a division for each of about seventeen keys a point. Without it, and with the copied keys searched by the standard bisection of a slice, 0.73.

## What the cospherical groups of a lattice cost after the zeros are proved, #617

The Delaunay triangulation of a lattice reads 1.64 of CGAL's time in D = 2 and 1.63 in D = 3 after #615 (the screening #618; under WSL the builds are 8.6 and 217 ms against CGAL's 5.6 and 131 ms, and 660 against 692 in D = 4). A scratch copy of `e6c7c92` with a timer at each part of the build and counters; Windows, not pinned, one build each of `rbox 10000 M3,4 D2`, `rbox 27000 M3,4,5 D3`, and `rbox 10000 M1,0 D4`, beside as many uniform sites. The timers cost time themselves, most in the parts that run once a group (a build of the lattice of D = 2 reads 13.7 ms with them and 9.2 without), so the shares matter.

| Delaunay of | Insertion | The pass that finds the groups | Single simplices | Sorting the groups' members | A group's sites collected | Placing | Its cells written | Faces paired (`finish`) | Build |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| The lattice, D2 | 5.76 | 0.49 | 0.03 | 0.58 | 0.66 | 1.30 | 1.30 | 1.97 | 13.7 |
| 10,000 uniform sites, D2 | 4.80 | 0.34 | 0.39 | | | | | 0.07 | 6.2 |
| The lattice, D3 | 89.7 | 15.9 | 0.3 | 4.2 | 6.8 | 89.1 | 9.7 | 30.7 | 250.9 |
| 27,000 uniform sites, D3 | 84.9 | 9.1 | 4.7 | | | | | 0.6 | 102.0 |
| The lattice, D4 | 370.7 | 65.1 | 1.3 | 4.6 | 9.4 | 191.6 | 12.4 | 39.8 | 697.6 |
| 10,000 uniform sites, D4 | 612.9 | 39.6 | 27.8 | | | | | 1.1 | 684.6 |

| Lattice | Groups | Simplices merged | Sites a group | Cells a group | Orientations the placing asks, a group |
| --- | ---: | ---: | ---: | ---: | ---: |
| D2 | 9,801 | 19,602 | 4 | 2 | 1 |
| D3 | 24,389 | 145,573 | 8 | 6 | 45 |
| D4 | 6,561 | 153,874 | 16 | 24 | 318 |

### Reading

- **The insertion is level with uniform sites** in D = 2 and 3, as #615 left it, and under them in D = 4.
- **The placing is the largest part from D = 3**: 36% of the build in D = 3 and 27% in D = 4. It is its orientations: 45 for the eight corners of a unit cube, 3.65 microseconds a group, nearly all of them zeros on the cube's coplanar fours, each proved by the first stage's exact arithmetic at about 80 ns. In D = 2 it asks one orientation a group, and its 1.3 ms is the bookkeeping of a run of the general procedure on four points.
- **Pairing the faces** (`Draft::finish`) is the second part: every face of every cell of a merged group is paired by its vertex set, a sort of all of them, where a single simplex keeps the links of the mesh. 12% in D = 3; in D = 2 it is the largest part after the insertion.
- **The pass that finds the groups** takes 7 to 26 ms more than on uniform sites: a lifted orientation for each face whose record the insertion left open.

What CGAL does not do is all of it: it returns the diagonals the insertion left, which depend on the order of insertion, and no groups. convx merges the cospherical simplices (the groups are the vertices of the Voronoi diagram, design §8) and splits each again by the placing triangulation in index order, so that what is published does not depend on the order of insertion (design §5, §7).

### What could be done, for the owner

1. **D = 2, a group of four sites, without the placing.** The placing triangulation of four points in convex position, in index order, is the triangle of the three smallest and the triangle of the largest with the edge it sees: its diagonal joins the two neighbors of the largest index around the quadrilateral. The mesh has the group as two triangles: when their common edge does not hold the largest index they are that triangulation, and otherwise it is the other diagonal. No predicate, and the two cells' orientations are those of the mesh. Expected: the placing and most of the writing of the cells, about 2 of the 9.2 ms; the lattice of D = 2 at about 1.3 of CGAL. A group of more than four sites goes through the placing as now.
2. **D >= 3, the placing of a group that is an exact translate of one placed before.** On a lattice every unit cell is the same eight points up to a translation, in the same order of indices. When the differences of a group's sites from its first site are formed without a rounding (the test of #615), two groups with the same differences and the same order of indices are exact translates, every orientation among them has the same sign, and the placing triangulation of one is that of the other with the sites renamed. A table from the differences to the triangulation placed would place one cube and copy 24,388. Expected: most of the 89 ms in D = 3 and of the 192 ms in D = 4; the lattice of D = 3 at about 1.0 of CGAL from 1.63. It helps inputs that repeat a cell (lattices, voxels) and nothing else, and it is a table whose key and whose proof have to be exact.
3. **The faces inside a group linked by the placing**, and only the faces between groups paired by their vertices: part of the 31 ms in D = 3. The placing triangulation's own adjacency is not kept today.

Or none: the comparison states that the triangulation of a lattice takes 1.6 times CGAL's time because the output is the one triangulation of each cospherical group that does not depend on the order of insertion, with the groups for the Voronoi diagram.

## A group of two triangles written without the placing, PR #635 (#633)

The spike #617 found the Delaunay triangulation of a lattice of D = 2 running the placing procedure for each of its groups of four cocircular sites, 9,801 on 100 x 100 sites.

### The change

In D = 2 a group of two triangles is written from the mesh: the placing triangulation of four points in convex position, in index order, is the triangle of the three smallest and the triangle of the largest with the edge between its two neighbors, so its diagonal is the one that does not hold the largest index. The mesh's two triangles are kept when their common edge is that diagonal, and the other two are written otherwise. The sign of each cell's ascending order is the parity of the sort from the mesh's counterclockwise order. No predicate is asked. A group of more than four sites goes through the placing as before.

### Time

Under WSL, pinned and alternated, `ba4361a` (the branch's base, the crate of `e6c7c92`) against `9174fa4`, and CGAL's program on the same file between them; ms, the minimum, and the ratios of the minimum and of the median to `ba4361a`:

| Delaunay of | Builds a side | `ba4361a` | `9174fa4` | Ratios | CGAL | convx / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `rbox 10000 M3,4 D2` | 80 | 8.75 | 7.58 | 0.87, 0.84 | 5.03 | 1.51 |
| `rbox 99856 M3,4 D2` | 16 | 113.41 | 97.14 | 0.86, 0.87 | 58.16 | 1.67 |
| `rbox 27000 M3,4,5 D3` | 12 | 229.17 | 229.07 | 1.00, 1.03 | 134.57 | 1.70 |
| `rbox 10000 M1,0 D4` | 3 | 674.65 | 687.40 | 1.02, 1.01 | 712.79 | 0.96 |
| 10^5 Kuzmin sites, D2 | 24 | 61.19 | 60.65 | 0.99, 0.99 | 56.40 | 1.08 |
| `cube` D2 10^4 | 96 | 4.94 | 4.83 | 0.98, 1.01 | | |
| `sphere` D2 10^4 | 96 | 4.87 | 4.91 | 1.01, 1.00 | | |
| `cube` D2 10^5 | 24 | 51.11 | 51.11 | 1.00, 0.99 | | |
| `sphere` D2 10^5 | 20 | 41.52 | 41.64 | 1.00, 1.00 | | |
| `cube` D3 10^4 | 24 | 34.30 | 34.26 | 1.00, 1.00 | | |
| `sphere` D3 10^4 | 16 | 66.80 | 66.95 | 1.00, 1.00 | | |

The lattices of D = 2 are at 0.86 to 0.87 of their time and at 1.51 and 1.67 of CGAL, from 1.74 and 1.95 in this run. The spike had written "about 1.3 of CGAL" for this change, from the timers of the parts; it measures 1.51. The lattices of D = 3 and 4 do not go through the change: their medians read 1.03 and 1.01 over 12 and 3 builds.

## Translates of a placed group, and the links inside a group: both prototyped and measured, #634

The spike #617 gave an expected gain for each from the timers of the parts. Neither is taken or left on such a figure (the owner's word of 2026-10-13), so both were built in a scratch copy of `9174fa4` (the head with #633), switched by an environment variable read once a build:

- **Translates.** From D = 3, the differences of a group's sites from its first site, formed without a rounding (the two-sum test of #615; a group with a difference that rounds goes through the placing), are the key of a table. A group whose key is in the table takes the cells and their signs of the group placed before, with the sites renamed by their place in the group.
- **Links.** The faces two cells of one group share are linked where the group is written, found by matching the group's cells against each other (or copied with the translate), and `Draft::finish` pairs by vertex sets only the faces left: those between groups and on the hull. In D = 2 the diagonal of a group of two triangles is linked the same way.

What every mode publishes has one hash, as the whole `Debug` form of the triangulation, on lattices of D = 2 (100 x 100, rotated, times 0.1, 316 x 316), D = 3 (22^3, rotated) and D = 4 (8^4), on a Voronoi diagram of a lattice, and on uniform sites. The table held one entry on each lattice: 9,260 of the 9,261 groups of 22^3 took it, and 2,400 of 2,401 of 8^4.

### Time

Under WSL, pinned, on an idle machine, the four modes of one binary and CGAL's program taking turns; ms, the minimum, and the ratios of the minimum and of the median to the head:

| Delaunay of | Builds a mode | Head | Translates | Links | Both | CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `rbox 10000 M3,4 D2` | 80 | 7.49 | 7.54 (1.01, 1.00) | 7.21 (0.96, 0.94) | 7.20 (0.96, 0.96) | 4.98 |
| `rbox 99856 M3,4 D2` | 16 | 99.31 | 99.94 (1.01, 0.98) | 89.40 (0.90, 0.86) | 89.85 (0.90, 0.85) | 58.68 |
| `rbox 27000 M3,4,5 D3` | 16 | 232.62 | 145.12 (0.62, 0.62) | 218.48 (0.94, 0.93) | 125.35 (0.54, 0.57) | 133.73 |
| `rbox 125000 M3,4,5 D3` | 4 | 1,173.4 | 739.3 (0.63, 0.63) | 1,121.0 (0.96, 0.95) | 666.4 (0.57, 0.57) | 662.0 |
| `rbox 10000 M1,0 D4` | 4 | 684.3 | 495.1 (0.72, 0.72) | 676.9 (0.99, 0.98) | 461.1 (0.67, 0.67) | 710.8 |
| `cube` D2 10^5 | 16 | 53.48 | 53.07 (0.99, 1.03) | 53.56 (1.00, 1.00) | 53.47 (1.00, 0.99) | |
| `cube` D3 10^4 | 24 | 35.06 | 35.10 (1.00, 1.00) | 34.98 (1.00, 1.02) | 35.17 (1.00, 1.00) | |
| `sphere` D3 10^4 | 12 | 67.60 | 67.18 (0.99, 1.01) | 67.53 (1.00, 0.99) | 67.42 (1.00, 0.99) | |
| `cube` D4 3,000 | 5 | 181.11 | 181.40 (1.00, 1.01) | 180.36 (1.00, 0.99) | 181.04 (1.00, 0.99) | |

- **Translates**: the lattices of D = 3 at 0.62 and 0.63 of their time and of D = 4 at 0.72; nothing in D = 2, where #633 took the placing away.
- **Links**: 0.90 to 0.96 in D = 2, 0.94 to 0.96 in D = 3, 0.99 in D = 4, alone.
- **Both**: 0.54 to 0.57 in D = 3 and 0.67 in D = 4. After the translates the links take a further 14% and 10% in D = 3 and 7% in D = 4: the pairing they shorten is a larger share of a build without the placing.
- **Against CGAL** with both: the lattices of D = 3 at 0.94 and 1.01 (from 1.74 and 1.77 in this run), of D = 4 at 0.65, of D = 2 at 1.45 and 1.53 (from 1.50 and 1.69).
- Uniform sites and sites on a sphere read 0.99 to 1.00 by the minimum in every mode.

The spike's sentence that the links would give "part of the 31 ms" of D = 3, and the recommendation against them that followed, were estimates. Measured, the links are 6% alone and 14% on top of the translates.

### What each needs to be a row

- **Translates.** The proof of the key: two groups with the same exact differences, in the same order of indices, are translates, the sign of every orientation among their sites is the same, and so is every step of the placing; a difference that rounds takes the group out. The table must not grow with the groups: on sites whose groups are all different (sites drawn near a sphere) the prototype would add an entry a group, which the allocation contract forbids, so the table holds a fixed number of entries and stops taking new ones.
- **Links.** The prototype matches every pair of a group's cells, which is quadratic in them: right for a cube's six cells, not for a group of a thousand. The row takes the links from the placing itself, which knows the two simplices on every ridge it keeps, or matches a large group's faces by a sort of that group alone.

## A group that is an exact translate of one placed before takes its cells, PR #638 (#637)

The prototype of #634 measured the lattices of D = 3 at 0.62 of their time with a table of the groups placed, by the exact differences of their sites from their first site. This is that change in the crate.

### The change

A group that would go to the placing is looked up by its differences, each formed without a rounding (a group with a difference that rounds is placed). A group found takes the cells of the one placed before, by the place of each site in its group, and their signs. The table holds sixteen groups and is given up when it is full and a group is not in it, so it does not grow with the groups. The proof is at `Translates` and the rule in design §7.

### Time

Under WSL, pinned and alternated, on an idle machine, `cacf885` (the branch's base, the crate of `9174fa4`) against `ec334f3`, and CGAL's program on the same file between them; ms, the minimum, and the ratios of the minimum and of the median to `cacf885`:

| Delaunay of | Builds a side | `cacf885` | `ec334f3` | Ratios | CGAL | convx / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `rbox 27000 M3,4,5 D3` | 12 | 216.88 | 132.71 | 0.61, 0.61 | 129.58 | 1.02 |
| `rbox 10000 M1,0 D4` | 4 | 668.93 | 478.75 | 0.72, 0.73 | 707.94 | 0.68 |
| `rbox 10000 M3,4 D2` | 80 | 7.40 | 7.36 | 0.99, 1.01 | 4.97 | 1.48 |
| `rbox 99856 M3,4 D2` | 16 | 90.97 | 89.48 | 0.98, 1.01 | 55.94 | 1.60 |
| 10^5 Kuzmin sites, D2 | 24 | 58.92 | 57.55 | 0.98, 1.00 | 51.79 | 1.11 |
| `cube` D2 10^4 | 96 | 4.81 | 4.82 | 1.00, 1.05 | | |
| `sphere` D2 10^4 | 96 | 4.78 | 4.77 | 1.00, 0.94 | | |
| `cube` D2 10^5 | 24 | 51.98 | 52.41 | 1.01, 1.01 | | |
| `sphere` D2 10^5 | 20 | 42.20 | 41.88 | 0.99, 1.01 | | |
| `cube` D3 10^4 | 24 | 34.42 | 33.84 | 0.98, 1.01 | | |
| `sphere` D3 10^4 | 16 | 67.21 | 67.08 | 1.00, 1.02 | | |
| `grid` D3 10^4 (integers in [0, 3]) | 24 | 0.73 | 0.73 | 1.00, 0.98 | | |
| `lattice` D3 10^4 (integers in [-2, 2]) | 24 | 1.10 | 1.10 | 1.00, 1.00 | | |
| `cube` D4 3,000 | 5 | 187.70 | 186.37 | 0.99, 0.99 | | |

The lattice of D = 3 is at 1.02 of CGAL, from 1.67 in this run, and that of D = 4 at 0.68. The lattices of D = 2 are not touched: their groups of four sites do not go to the placing (#633). The medians of `cube` and `sphere` D2 at 10^4 read 1.05 and 0.94 with their minimums at 1.00; neither build has a group.

### `tests/alloc.rs`

The table's lists are allocated when a build has a group that goes to the placing: six allocations and about nineteen reallocations more on the cases with such groups (Delaunay `grid` D3 80 and 131 for 74 and 112), the same at the small and the large size. The Delaunay cases of D = 2 whose groups have four sites count twelve fewer, the placing's own buffers (#633: `lattice` D2 54 for 66). A hull of D = 2 counts one more, the keys of its cycle (#630). Every record is set to the counts of this commit.

## The faces inside a group linked where it is written, PR #642 (#641); the draft sized for the groups, withdrawn (#640)

### The links (#641)

`Draft::finish` paired every face of every cell of a merged group by its vertex set. The faces two cells of one group share are now linked when the group's cells are written: paired among the faces of that group alone, in the table the pairing already uses; copied with the cells when the group is a translate of one placed before (#637); and, for a group of two triangles of D = 2, the diagonal linked directly. `finish` pairs what is left, the faces between groups and on the hull. The prototype of #634 had measured it.

Under WSL, pinned, `1bdad5b` (the branch's base, the crate of `ec334f3`), the same commit built a second time as a control, and `eb1a5ad`, taking turns with CGAL's program on the files; ms, the minimum, and the ratios of the minimum and of the median to the base. The machine was in use during the run: the processor read 10% before it and 12% after.

| Delaunay of | Builds a side | Base | Control | `eb1a5ad` | CGAL | convx / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `rbox 10000 M3,4 D2` | 80 | 7.53 | 7.62 (1.01, 0.99) | 7.31 (0.97, 0.97) | 5.08 | 1.44 |
| `rbox 99856 M3,4 D2` | 16 | 100.72 | 99.69 (0.99, 1.02) | 90.83 (0.90, 0.92) | 58.31 | 1.56 |
| `rbox 27000 M3,4,5 D3` | 16 | 146.11 | 144.35 (0.99, 0.99) | 127.81 (0.87, 0.87) | 135.17 | 0.95 |
| `rbox 125000 M3,4,5 D3` | 4 | 765.37 | 739.45 (0.97, 0.98) | 671.34 (0.88, 0.87) | 664.52 | 1.01 |
| `rbox 10000 M1,0 D4` | 4 | 483.07 | 486.08 (1.01, 0.99) | 456.83 (0.95, 0.95) | 716.48 | 0.64 |
| Buddha | 6 | 2,738.4 | 2,730.3 (1.00, 0.97) | 2,774.0 (1.01, 1.00) | 2,244.9 | 1.24 |
| 543,652 uniform sites, D3 | 5 | 2,445.0 | 2,423.0 (0.99, 0.98) | 2,477.3 (1.01, 1.00) | 2,185.2 | 1.13 |
| 10^5 Kuzmin sites, D2 | 24 | 65.06 | 60.39 (0.93, 1.00) | 61.81 (0.95, 0.97) | 56.98 | 1.08 |
| `cube` D2 10^4 | 96 | 4.90 | 4.96 (1.01, 1.00) | 4.92 (1.01, 1.00) | | |
| `sphere` D2 10^4 | 96 | 4.81 | 4.83 (1.01, 1.00) | 4.81 (1.00, 1.01) | | |
| `cube` D2 10^5 | 20 | 56.16 | 55.30 (0.99, 0.99) | 55.20 (0.98, 0.98) | | |
| `cube` D3 10^4 | 24 | 36.15 | 35.90 (0.99, 1.03) | 35.90 (0.99, 1.00) | | |
| `sphere` D3 10^4 | 16 | 69.67 | 74.07 (1.06, 0.98) | 73.75 (1.06, 0.98) | | |
| `grid` D3 10^4 | 24 | 0.83 | 0.82 (1.00, 0.97) | 0.82 (0.99, 0.95) | | |
| `lattice` D3 10^4 | 24 | 1.13 | 1.13 (1.00, 1.00) | 1.14 (1.01, 0.97) | | |
| `cube` D4 3,000 | 5 | 203.19 | 207.26 (1.02, 1.03) | 198.11 (0.98, 1.04) | | |

The lattices are at 0.87 to 0.97 of their time; where the control itself reads off 1.00 (Kuzmin, `sphere` D3, the larger lattice of D = 3, `cube` D4) the change reads as the control does. With #633 and #637, the lattices of D = 3 are level with CGAL (0.95 and 1.01, from 1.63 in the screening), that of D = 4 is at 0.64, and those of D = 2 at 1.44 and 1.56 (from 1.83).

### The control, and the load

The owner's question of 2026-10-13: does a processor at 10 to 20% move a pinned timing. It had been assumed, and a run had been held back for it. Measured in the two runs of this section with the base built twice and alternated, under 10 to 14%: the control read 0.98 to 1.02 by the minimum on thirteen sets of sixteen in each, and up to 0.07 off on the others: rows of four builds, builds of a millisecond, and in the second run two sets of sixteen and twenty-four builds (`sphere` D3 1.06, Kuzmin 0.93). So such a load is not a reason to wait; the control is in every run from here, and its ratio is the spread the run's other ratios are read against. The rule that a run waits for an idle machine (`bench.mdc`, written the same day) is replaced by that.

### The draft sized for the groups, withdrawn (#640)

A scratch copy with timers had read 45 ms for writing the 28 cells of Buddha's 14 groups: the draft is allocated for the single simplices, and the first cell of a group grows its lists. `2262ab2` allocated it for the groups' simplices and a quarter more.

| Delaunay of, under WSL, the run before the one above (control beside it) | Control | `2262ab2` |
| --- | ---: | ---: |
| `rbox 10000 M3,4 D2` | 1.01, 1.00 | 1.06, 1.04 |
| `rbox 99856 M3,4 D2` | 1.00, 1.00 | 0.99, 1.00 |
| `rbox 27000 M3,4,5 D3` | 1.00, 0.99 | 1.01, 0.98 |
| `rbox 10000 M1,0 D4` | 0.99, 1.01 | 0.98, 1.00 |
| Buddha | 1.01, 1.00 | 1.01, 1.00 |
| 543,652 uniform sites, D3 | 1.01, 1.00 | 1.00, 1.01 |

Nothing on Buddha, which it was for, and the small lattice of D = 2 at 1.06, as in the run before that one (1.07). On Windows, pinned, the best of 2 to 12 builds over three rounds: Buddha 2,995 and 2,941 ms, the small lattice 9.45 and 9.14, the lattice of D = 3 162.6 and 161.1, and Dragon, which has no group and does not reach the change, 2,346 and 2,285: the differences are of the size of that one. The 45 ms were the timers' reading and are not in the build's time; under Linux a list that grows is moved without a copy. The change is reverted (`bd43b26`). It did cut the reallocations of the builds with groups (Delaunay `lattice` D3 118 for 159), which `tests/alloc.rs` counts and the time does not show.

### `tests/alloc.rs`

The links' scratch is four lists more in a build with a group that goes to the placing or to the table: six allocations more on those cases (Delaunay `grid` D3 86 for 80), the same at both sizes. Every record is set to the counts of `eb1a5ad`.

## The Delaunay triangulation of the scans beside CGAL, #639

The screening (#618) read Dragon at 1.13 and Buddha at 1.18 of CGAL, above uniform sites of the same size.

### By phase, beside uniform sites of the same number

Under WSL, pinned, `ab64d1b` with the parity's marks, six builds each taking turns with CGAL's program; ms, the minimum. "Before" is acceptance, the copy, and the order of insertion; "after" is the pass that finds the groups, the draft, and its finish.

| Delaunay of | Build | Before | Insertion | After | Publication | CGAL | convx / CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Dragon, 437,645 sites | 2,007 | 13 | 1,583 | 375 | 13 | 1,686 | 1.19 |
| 437,645 uniform sites | 1,698 | 14 | 1,385 | 287 | 12 | 1,607 | 1.06 |
| Buddha, 543,652 sites | 2,440 | 18 | 1,924 | 476 | 19 | 2,171 | 1.12 |
| 543,652 uniform sites | 2,232 | 18 | 1,761 | 403 | 15 | 2,001 | 1.12 |

- **Buddha is not above uniform sites**: 1.12 for both. What the screening read on it is the gap of uniform sites of D = 3 at that size, the parity's reported set (ADR 0006).
- **Dragon is**: its insertion takes 14% more than that of as many uniform sites and what follows 31% more, 18% in all, where CGAL takes 5% more.

Dragon has 2,100 duplicate points (435,545 distinct of 437,645) and no cospherical group; Buddha has 14 groups of 28 cells.

### Instructions and simulated cache misses, convx

`4f606ea`, callgrind with its cache simulation, the build alone (collection toggled on `DelaunayBuilder::build`):

| | Dragon | 437,645 uniform sites | Ratio |
| --- | ---: | ---: | ---: |
| Instructions | 17,545,739,071 | 16,333,846,924 | 1.074 |
| Data reads and writes | 5,669,466,649 | 5,282,150,704 | 1.073 |
| First-level data misses | 78,184,548 | 69,259,993 | 1.129 |
| Last-level data misses | 20,845,475 | 19,575,996 | 1.065 |

The shares by function are the same on both to a tenth of a percent: the conflict test 20.5%, the insertion's own lines 17.1% and 10.8%, and so on down the list. So Dragon is 7.4% more of the same work, which agrees with its counts (9.8 steps of the walk a site for 8.7, 21.8 simplices a cavity for 20.3, #618), and not one step that it makes dear. Of the 5.6 million first-level read misses it adds, 3.1 million are in the insertion's own reads of the mesh (19.7 million for 16.6): its simplices are visited in an order that follows the surface less well than uniform sites follow space.

The time grows by 18% and the instructions by 7%; the simulation shows 13% more first-level misses and does not simulate branches.

### What could not be measured here

CGAL's side of the same table. Its program does not run usefully under valgrind (the run on Dragon had not ended after 44 minutes, where convx took two and a half; its interval filter depends on the rounding mode, which valgrind does not keep, is the likely reason and was not verified), and `perf` reports the hardware counters as not supported under WSL2. So whether CGAL does 5% more work on Dragon, or as much more as convx with less memory cost for it, is not known.

### Reading

Nothing is proposed: no step of convx stands out on Dragon, and no change was measured that takes its 18% toward CGAL's 5%. Dragon is in the comparison as it measures. The reallocation of the draft that a scratch timer read on Buddha's 14 groups was a row of its own and was withdrawn after measurement (#640).

## A hull of D = 2 of points nearly all inside: what is left beside the duplicates, #644

The duplicates on demand were left out of 0.1.0 (#622), and the hull of 10^6 Kuzmin points stayed at 3.6 times CGAL's time (#631), twice with the duplicates switched off. The owner asked that what is left be looked at.

### By part

A scratch copy of `54c6cbc` (the head of the stack then, the crate of `main` at `bb6d622`) with a timer between the parts; Windows, pinned, the best of each part over 5 to 40 builds, ms. The timers and the best of each part make the sums smaller than a build's time; the shares are what is read.

| Hull of, D2 | Sample of the scaling | Validation | The discard polygon | Duplicates | List of representatives | Basis | The discard pass | Sort and chain | Classification | Publication | Points kept |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10^6 Kuzmin points | 0.02 | 1.75 | 11.49 | 41.98 | 2.60 | 0.03 | 6.40 | 0.04 | 2.03 | 1.29 | 5 |
| 10^6 points in a cube | 0.02 | 1.88 | 10.65 | 42.31 | 2.34 | 0.03 | 19.91 | 0.19 | 2.29 | 1.28 | 1,301 |
| 10^6 points in a ball | 0.02 | 1.69 | 10.27 | 40.86 | 2.41 | 0.03 | 24.20 | 8.37 | 10.45 | 1.48 | 99,709 |
| 10^5 Kuzmin points | 0.01 | 0.10 | 1.06 | 1.49 | 0.10 | 0.01 | 0.52 | 0.02 | 0.15 | 0.07 | 4 |
| `rbox 10000 M3,4 D2` | 0.00 | 0.02 | 0.13 | 0.13 | 0.03 | 0.01 | 0.08 | 0.04 | 0.06 | 0.03 | 492 |

- **The duplicates** are the largest part everywhere at 10^6: one table of twice the points, read at a random place for each.
- **The discard polygon**, the farthest points in eight directions, is one pass over every point that keeps eight maxima and their indices: 10 to 11 ms.
- **The discard pass** tests each point against the polygon's edges by the filtered orientation: 6 ms where every point is at the center, 20 to 24 ms where points lie all over the polygon.
- The lattice has no part above 0.13 ms: its 0.29 ms against CGAL's 0.10 (#631) is every part's fixed cost together.

CGAL's `convex_hull_2` finds four extreme points in one pass, discards what is inside their quadrilateral, and sorts and scans the four regions outside it. It finds no duplicates and lists no interior points.

### Prototypes

In the same scratch copy, switched by environment variables, one binary; ms of the part they change, and the points kept:

| Prototype | Kuzmin 10^6 | Cube 10^6 | Ball 10^6 |
| --- | ---: | ---: | ---: |
| The polygon as it is | 11.34 (5 kept) | 13.31 (1,301) | 11.26 (99,709) |
| The farthest points by the eight maxima, then the first point that reaches each | 3.67 (5) | 4.48 (1,301) | 4.55 (99,709) |
| The farthest points of a sample of 4,096 | 0.14 (1,691) | 0.16 (20,367) | 0.16 (108,673) |
| The discard pass as it is | 6.75 | 20.68 | 24.44 |
| A box inside the polygon tested before its edges | 6.66 | 15.91 | 20.97 |
| The duplicates as they are | 33.81 | 41.76 | 35.58 |
| The duplicates bucket by bucket of the hash | 22.87 | 19.79 | 20.56 |
| The duplicates with the slots of a batch of 32 points read ahead | 36.15 | | |

- **The maxima first** gives the same eight points, so the same polygon, in a third of the time. The sample is faster still and keeps more points, 20,367 for 1,301 on a cube, which the sort and the classification then pay for (1.45 and 3.46 ms for 0.23 and 1.69): the maxima first is the better of the two.
- **The box** helps where points are spread (a quarter of the pass on a cube) and not on Kuzmin points.
- **The buckets**: the points are scattered, as hash and index, into buckets of about a thousand by the top bits of the hash, in index order, and each bucket is searched in a table that fits the first-level cache. A third to a half of the duplicates' time at 10^6 (28.69 to 18.67 on a cube of D = 3 in another run), and 1.28 to 1.43 ms at 10^5, so it starts from some size. Reading a batch of slots ahead in the one table was slower.

All three together, by the sum of the parts: Kuzmin 58.83 to 41.96 (0.71), a cube 83.80 to 47.82 (0.57), a ball 95.83 to 70.78 (0.74). The duplicates read 34 to 43 ms for one configuration from run to run on this machine, so these are sizes and not a judgement; each row is timed under WSL with a control.

### Why the box discarded more on the lattice

With the box, 436 points of the lattice were kept for 492. The 56 are points that share a coordinate with a vertex of the polygon, as `[0, 175]` with the vertex at the origin. `orient2_filter` computes `(a0 - c0) (b1 - c1) - (a1 - c1) (b0 - c0)` and returns nothing when the first product is zero, whatever the second is. When one of its two factors is exactly zero the first product is exactly zero and the determinant is the second product's negation, whose sign is known. The points are interior; the filter did not say so, and they went to the sort and the exact test.

### What follows

Three rows, by the owner's word of 2026-10-13: the farthest points by the maxima first (#645), the box, with the filter's zero product (#646), and the duplicates in buckets (#647). The hull of Kuzmin points will still be above CGAL by the duplicates, which stay in the build.

## The farthest points of the discard polygon by the maxima first, PR #649 (#645)

The polygon that discards interior points of a hull of D = 2 is the strict cycle of the farthest points in eight directions. One pass kept each maximum with its index, eight tests and stores a point: 10 to 11 ms of 10^6 points (#644). The eight maxima are now found first, a pass of maxima alone, and then the first point that reaches each, a pass that stops at the last of the eight. The points are those of the one pass, so the polygon and what it discards are the same.

Under WSL, pinned, five builds of the crate taking turns with CGAL's program on the files: `bb6d622` (`main`), the same built a second time as the control, and the three rows of #644 one on another (`0153e36`, `77b172a`, `c7ddd9b`). The processor read 3% before the run and 16% after. ms, the minimum; the ratios are of the minimum and of the median, of the row to the commit before it, and of the control to `main`.

| Hull of | Builds a side | `bb6d622` | `0153e36` | Ratios | Control | CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10^6 Kuzmin points, D2 | 40 | 35.17 | 28.89 | 0.82, 0.83 | 1.01, 1.00 | 9.83 |
| 10^5 Kuzmin points, D2 | 80 | 2.39 | 1.87 | 0.78, 0.76 | 1.02, 0.99 | 0.90 |
| 10^6 points in a ball, D2 | 40 | 67.08 | 60.33 | 0.90, 0.91 | 1.00, 0.99 | 57.81 |
| 10^5 points in a ball, D2 | 80 | 5.13 | 4.69 | 0.91, 0.88 | 0.99, 0.99 | 4.35 |
| `rbox 10000 M3,4 D2` | 96 | 0.28 | 0.23 | 0.80, 0.81 | 1.00, 1.01 | 0.10 |
| 10^6 Plummer points, D3 | 40 | 72.97 | 73.80 | 1.01, 1.02 | 1.01, 0.98 | 270.3 |
| `cube` D2 10^4 | 96 | 0.35 | 0.29 | 0.83, 0.83 | 1.00, 0.99 |  |
| `cube` D2 10^5 | 80 | 3.62 | 3.13 | 0.86, 0.84 | 1.02, 1.01 |  |
| `cube` D2 3 x 10^5 | 40 | 13.30 | 11.03 | 0.83, 0.84 | 0.97, 1.00 |  |
| `cube` D2 6 x 10^5 | 40 | 27.25 | 24.82 | 0.91, 0.87 | 1.05, 1.00 |  |
| `cube` D2 10^6 | 40 | 54.42 | 44.18 | 0.81, 0.87 | 0.98, 0.97 |  |
| `sphere` D2 10^5 | 40 | 9.08 | 8.52 | 0.94, 0.94 | 1.04, 1.02 |  |
| `sphere` D2 10^6 | 18 | 94.86 | 94.10 | 0.99, 0.95 | 1.04, 1.07 |  |
| `grid` D2 10^6 (sixteen distinct points) | 30 | 16.19 | 8.85 | 0.55, 0.60 | 1.00, 1.02 |  |
| `cube` D3 10^5 | 40 | 8.44 | 8.41 | 1.00, 1.01 | 0.99, 0.97 |  |
| `cube` D3 3 x 10^5 | 30 | 30.02 | 32.02 | 1.07, 1.06 | 1.00, 1.05 |  |
| `cube` D3 10^6 | 24 | 118.3 | 124.2 | 1.05, 1.03 | 1.04, 1.01 |  |
| `sphere` D3 10^5 | 10 | 255.9 | 259.0 | 1.01, 1.03 | 1.02, 1.01 |  |
| `cubesurf` D3 10^5 | 30 | 26.50 | 26.21 | 0.99, 0.98 | 1.00, 0.99 |  |
| `cubesurf` D3 10^6 | 10 | 320.2 | 316.9 | 0.99, 0.98 | 1.00, 1.00 |  |
| `cube` D4 10^6 | 10 | 362.3 | 357.1 | 0.99, 1.00 | 0.99, 1.01 |  |
| Delaunay `cube` D2 10^6 | 5 | 580.7 | 571.5 | 0.98, 1.03 | 0.99, 0.97 |  |
| Delaunay `cube` D3 3 x 10^5 | 4 | 1,278.8 | 1,269.9 | 0.99, 1.00 | 0.95, 0.98 |  |

Every hull of D = 2 that discards is at 0.78 to 0.91 of its time, and `grid` at 10^6, sixteen distinct points a million times, at 0.55: there the one pass stored at every point that tied a maximum. The hulls of D = 3 and 4 and the triangulations do not go through the change; `cube` D3 reads 1.07 and 1.05 at 3 x 10^5 and 10^6 with its control at 1.00 and 1.04 by the minimum and 1.05 and 1.01 by the median, and 0.97 and 1.05 again one commit on, which changes nothing there either.

## A box inside the discard polygon, and the filter's left product that is exactly zero, PR #650 (#646)

The pass that discards the points clearly inside the polygon tested each against up to eight edges by the filtered orientation: 20 to 24 ms of 10^6 points where they are spread (#644).

**The box.** A box whose four corners are clearly inside the polygon lies inside it, so a point strictly inside the box is discarded by four comparisons. The box is tried on a sample of 255 of the points: on each coordinate, between the values that leave a share of the sample below and above, from 3% inward. It then holds the points where they are dense. When none of those is inside it is tried about the mean of the polygon's vertices, which is also what the facets of a hull of D = 3 use for the points on them.

**The filter.** `orient2_filter` returned nothing when its left product was zero, whatever the right one was. When a factor of the left product is exactly zero the determinant is the right product negated, and its sign is that of the right product's two factors. A point that shares a coordinate with a vertex of the polygon was left to the sort and the exact orientation for it: 56 points of a lattice of 10,000.

Under WSL, pinned, five builds of the crate taking turns with CGAL's program on the files: `bb6d622` (`main`), the same built a second time as the control, and the three rows of #644 one on another (`0153e36`, `77b172a`, `c7ddd9b`). The processor read 3% before the run and 16% after. ms, the minimum; the ratios are of the minimum and of the median, of the row to the commit before it, and of the control to `main`.

| Hull of | Builds a side | `0153e36` | `77b172a` | Ratios | Control | CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10^6 Kuzmin points, D2 | 40 | 28.89 | 27.97 | 0.97, 0.99 | 1.01, 1.00 | 9.83 |
| 10^5 Kuzmin points, D2 | 80 | 1.87 | 1.67 | 0.89, 0.89 | 1.02, 0.99 | 0.90 |
| 10^6 points in a ball, D2 | 40 | 60.33 | 57.58 | 0.95, 0.93 | 1.00, 0.99 | 57.81 |
| 10^5 points in a ball, D2 | 80 | 4.69 | 4.16 | 0.89, 0.91 | 0.99, 0.99 | 4.35 |
| `rbox 10000 M3,4 D2` | 96 | 0.23 | 0.23 | 0.99, 1.00 | 1.00, 1.01 | 0.10 |
| 10^6 Plummer points, D3 | 40 | 73.80 | 75.34 | 1.02, 1.01 | 1.01, 0.98 | 270.3 |
| `cube` D2 10^4 | 96 | 0.29 | 0.19 | 0.66, 0.68 | 1.00, 0.99 |  |
| `cube` D2 10^5 | 80 | 3.13 | 2.03 | 0.65, 0.66 | 1.02, 1.01 |  |
| `cube` D2 3 x 10^5 | 40 | 11.03 | 6.98 | 0.63, 0.75 | 0.97, 1.00 |  |
| `cube` D2 6 x 10^5 | 40 | 24.82 | 17.00 | 0.69, 0.72 | 1.05, 1.00 |  |
| `cube` D2 10^6 | 40 | 44.18 | 33.87 | 0.77, 0.77 | 0.98, 0.97 |  |
| `sphere` D2 10^5 | 40 | 8.52 | 8.39 | 0.99, 0.98 | 1.04, 1.02 |  |
| `sphere` D2 10^6 | 18 | 94.10 | 88.18 | 0.94, 1.00 | 1.04, 1.07 |  |
| `grid` D2 10^6 (sixteen distinct points) | 30 | 8.85 | 9.03 | 1.02, 0.98 | 1.00, 1.02 |  |
| `cube` D3 10^5 | 40 | 8.41 | 8.30 | 0.99, 0.99 | 0.99, 0.97 |  |
| `cube` D3 3 x 10^5 | 30 | 32.02 | 28.99 | 0.91, 0.98 | 1.00, 1.05 |  |
| `cube` D3 10^6 | 24 | 124.2 | 124.4 | 1.00, 0.96 | 1.04, 1.01 |  |
| `sphere` D3 10^5 | 10 | 259.0 | 261.5 | 1.01, 0.97 | 1.02, 1.01 |  |
| `cubesurf` D3 10^5 | 30 | 26.21 | 24.80 | 0.95, 0.96 | 1.00, 0.99 |  |
| `cubesurf` D3 10^6 | 10 | 316.9 | 308.8 | 0.97, 0.99 | 1.00, 1.00 |  |
| `cube` D4 10^6 | 10 | 357.1 | 365.0 | 1.02, 1.01 | 0.99, 1.01 |  |
| Delaunay `cube` D2 10^6 | 5 | 571.5 | 577.2 | 1.01, 0.95 | 0.99, 0.97 |  |
| Delaunay `cube` D3 3 x 10^5 | 4 | 1,269.9 | 1,284.4 | 1.01, 1.02 | 0.95, 0.98 |  |

A cube of D = 2 is at 0.63 to 0.77 of its time and a ball at 0.89 to 0.95. Kuzmin points at 10^5 are at 0.89.

The first form of the box was about the mean of the polygon's vertices alone (`adb04f3`). A polygon that a few far points stretch has no such box, and 10^5 Kuzmin points then paid for the box's test and the edges: 2.01 ms for 1.87 in the run before this one, and a cube at 10^5 0.81 of its time where the box on the sample gives 0.65.

## The duplicates found bucket by bucket of the hash, PR #651 (#647)

The duplicates of every point were found in one table of twice the points, read at a random place for each: the largest part of a hull whose points are nearly all inside (#622, #644). From 2^19 points they are now found in buckets: the points are scattered, as the low half of their hash and their index, into buckets of about a thousand by the top bits of the hash, in index order, and each bucket is searched in a table of its own size. The scatter takes eight bytes a point, what the one table took. The representatives are the same.

Under WSL, pinned, five builds of the crate taking turns with CGAL's program on the files: `bb6d622` (`main`), the same built a second time as the control, and the three rows of #644 one on another (`0153e36`, `77b172a`, `c7ddd9b`). The processor read 3% before the run and 16% after. ms, the minimum; the ratios are of the minimum and of the median, of the row to the commit before it, and of the control to `main`.

| Hull of | Builds a side | `77b172a` | `c7ddd9b` | Ratios | Control | CGAL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10^6 Kuzmin points, D2 | 40 | 27.97 | 20.62 | 0.74, 0.86 | 1.01, 1.00 | 9.83 |
| 10^5 Kuzmin points, D2 | 80 | 1.67 | 1.71 | 1.03, 1.03 | 1.02, 0.99 | 0.90 |
| 10^6 points in a ball, D2 | 40 | 57.58 | 47.75 | 0.83, 0.87 | 1.00, 0.99 | 57.81 |
| 10^5 points in a ball, D2 | 80 | 4.16 | 4.19 | 1.01, 1.02 | 0.99, 0.99 | 4.35 |
| `rbox 10000 M3,4 D2` | 96 | 0.23 | 0.22 | 0.99, 1.00 | 1.00, 1.01 | 0.10 |
| 10^6 Plummer points, D3 | 40 | 75.34 | 63.29 | 0.84, 0.91 | 1.01, 0.98 | 270.3 |
| `cube` D2 10^4 | 96 | 0.19 | 0.20 | 1.02, 1.04 | 1.00, 0.99 |  |
| `cube` D2 10^5 | 80 | 2.03 | 2.07 | 1.02, 0.99 | 1.02, 1.01 |  |
| `cube` D2 3 x 10^5 | 40 | 6.98 | 7.15 | 1.02, 1.05 | 0.97, 1.00 |  |
| `cube` D2 6 x 10^5 | 40 | 17.00 | 14.01 | 0.82, 0.89 | 1.05, 1.00 |  |
| `cube` D2 10^6 | 40 | 33.87 | 26.05 | 0.77, 0.71 | 0.98, 0.97 |  |
| `sphere` D2 10^5 | 40 | 8.39 | 8.14 | 0.97, 1.00 | 1.04, 1.02 |  |
| `sphere` D2 10^6 | 18 | 88.18 | 90.78 | 1.03, 1.02 | 1.04, 1.07 |  |
| `grid` D2 10^6 (sixteen distinct points) | 30 | 9.03 | 9.19 | 1.02, 1.01 | 1.00, 1.02 |  |
| `cube` D3 10^5 | 40 | 8.30 | 8.48 | 1.02, 1.01 | 0.99, 0.97 |  |
| `cube` D3 3 x 10^5 | 30 | 28.99 | 29.26 | 1.01, 0.98 | 1.00, 1.05 |  |
| `cube` D3 10^6 | 24 | 124.4 | 117.2 | 0.94, 0.97 | 1.04, 1.01 |  |
| `sphere` D3 10^5 | 10 | 261.5 | 255.7 | 0.98, 1.03 | 1.02, 1.01 |  |
| `cubesurf` D3 10^5 | 30 | 24.80 | 25.74 | 1.04, 1.00 | 1.00, 0.99 |  |
| `cubesurf` D3 10^6 | 10 | 308.8 | 296.6 | 0.96, 0.97 | 1.00, 1.00 |  |
| `cube` D4 10^6 | 10 | 365.0 | 364.2 | 1.00, 0.97 | 0.99, 1.01 |  |
| Delaunay `cube` D2 10^6 | 5 | 577.2 | 563.6 | 0.98, 0.98 | 0.99, 0.97 |  |
| Delaunay `cube` D3 3 x 10^5 | 4 | 1,284.4 | 1,261.3 | 0.98, 0.97 | 0.95, 0.98 |  |

From 2^19 points a hull of D = 2 is at 0.74 to 0.83 of its time, Plummer points of D = 3 at 0.84, a cube of D = 3 at 10^6 at 0.94. Below 2^19 the one table is kept, and those sets read as the control does.

Two things the first form (`1ce0a18`, from 2^18 points, every input) showed in the run before this one, and what was done:

- **Points that are mostly duplicates.** `grid` D2 at 10^6 took 18.65 ms in buckets for 8.71: sixteen distinct points fill a few buckets, and the one table they keep small is the faster. A sample of 1,024 points decides: when fewer than three quarters of its hashes are distinct, the one table is used. `grid` reads 1.02 here.
- **Where the buckets start.** At 3 x 10^5 points a cube of D = 2 took 10.12 ms in buckets for 9.49 in the one table; they start at 2^19, and at 6 x 10^5 points they read 0.82 here.

With the three rows, against `main`: 10^6 Kuzmin points 35.17 to 20.62 ms (0.59; CGAL 9.83), 10^6 points in a ball 67.08 to 47.75 (0.71; CGAL 57.81), a cube of D = 2 at 10^6 54.42 to 26.05 (0.48). The Kuzmin hull stays above CGAL by its duplicates, which the build keeps (#622).

## The parity run after the rows of R1, #652

One run of the 44 sets on `main` at `217d389`, after #651: the 38 judged sets, Delaunay `cube` D3, and the three sets added since the run of #609 (two Delaunay lattices and 10^6 Kuzmin sites, reported). Method: `docs/verification.md`, `benches/parity/`; WSL2, Qhull 2020.2, CGAL 5.4. Eleven sets that were not met, at the edge, or spread were run again; the table has their second reading.

**The machine was not free.** A benchmark of another project used one core through both readings and the comparison below; the processor read 18% and 10% about the first reading, 12% and 9% about the second. The judged builds of 19 sets spread by 1.2 or more, and a build takes about a tenth longer than in the run of #609 on sets the changes do not reach (Delaunay `sphere` D3 10^4 74.5 ms for 66.6).

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^6 | 39.3 ms | 0.47 | 0.81 | 0.59 | met | met |
| Hull `cube` D3 10^4 | 1.18 ms | 0.55 | – | 0.76 | met | met |
| Hull `cube` D3 10^5 | 9.29 ms | 0.37 | – | 0.77 | met | met |
| Hull `cube` D3 10^6 | 165 ms | 0.15 | – | 0.81 | met | met |
| Hull `cube` D4 10^4 | 5.30 ms | 0.11 | – | 0.74 | met | met |
| Hull `cube` D4 10^5 | 27.8 ms | 0.05 | – | 0.80 | met | met |
| Hull `cube` D5 10^4 | 54.4 ms | 0.23 | – | 0.71 | met | met |
| Hull `cube` D5 10^5 | 220 ms | 0.12 | – | 0.58 | met | met |
| Hull `cube` D6 10^4 | 667 ms | 0.29 | – | 0.39 | met | met |
| Hull `cube` D6 10^5 | 2.99 s | 0.23 | – | 0.32 | met | met |
| Hull `sphere` D2 10^5 | 10.9 ms | 1.02 | 1.32 | 0.07 (ref.) | met | not met |
| Hull `sphere` D2 10^6 | 102 ms | 0.60 | 1.10 | 0.05 (ref.) | met | met |
| Hull `sphere` D3 10^5 | 269 ms | 0.56 | – | 1.00 | met | met |
| Hull `sphere` D3 10^6 | 4.35 s | 0.38 | – | 0.99 | met | met |
| Hull `sphere` D4 10^4 | 101 ms | 0.77 | – | 0.86 | met | met |
| Hull `sphere` D5 10^4 | 934 ms | 0.89 | – | 0.92 | met | met |
| Hull `sphere` D6 10^4 | 7.94 s | 0.76 | – | 0.78 | met | met |
| Hull `cubesurf` D3 10^5 | 28.1 ms | 0.70 | – | 0.92 | met | met |
| Hull `grid` D6 10^4 | 185 ms | 0.68 (ref.) | – | 0.52 | met | met |
| Delaunay `cube` D2 10^5 | 58.8 ms | 0.97 | 1.31 | 0.21 | met | met |
| Delaunay `cube` D2 10^6 | 595 ms | 0.74 | 1.26 | 0.14 | met | met |
| Delaunay `sphere` D2 10^6 | 445 ms | 0.77 | 1.41 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 37.6 ms | 1.08 | 1.21 | 0.33 | reported | reported |
| Delaunay `cube` D3 10^5 | 440 ms | 1.13 | 1.32 | 0.26 | reported | reported |
| Delaunay `cube` D3 10^6 | 5.02 s | 1.12 | 1.36 | 0.23 | reported | reported |
| Delaunay `sphere` D3 10^4 | 74.5 ms | 0.52 | 0.53 | 0.46 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 714 ms | 0.69 | 0.72 | 0.26 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 6.00 s | 0.85 | 0.88 | 0.26 (ref.) | met | met |
| Delaunay `sphere` D4 10^4 | 948 ms | 0.35 | 0.36 | 0.58 (ref.) | met | met |
| Delaunay `sphere` D5 10^4 | 11.56 s | 0.32 | 0.32 | 0.62 (ref.) | met | met |
| Delaunay `lattice` D2 10^4 | 9.01 ms | 1.54 | 1.81 | 0.22 | reported | reported |
| Delaunay `lattice` D3 27,000 | 137 ms | 0.96 | 1.00 | 0.13 | reported | reported |
| Delaunay `kuzmin` D2 10^6 | 652 ms | 0.84 | 1.34 | 0.13 | reported | reported |
| Hull `cube` D2 10^4 | 0.30 ms | 0.47 | 0.54 | 0.47 | met | met |
| Hull `cube` D2 10^5 | 3.05 ms | 0.48 | 0.61 | 0.49 | met | met |
| Hull `sphere` D2 10^4 | 0.76 ms | 0.92 | 1.01 | 0.10 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 23.1 ms | 0.88 | – | 1.04 | met | not met |
| Hull `sphere` D4 10^5 | 1.45 s | 1.07 | – | 0.83 | not met | not met |
| Hull `sphere` D5 10^5 | 12.76 s | 1.10 | – | 0.89 | not met | not met |
| Delaunay `cube` D2 10^4 | 5.39 ms | 1.07 | 1.27 | 0.28 | not met | not met |
| Delaunay `sphere` D2 10^4 | 5.02 ms | 1.08 | 1.31 | 0.20 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 47.5 ms | 1.00 | 1.44 | 0.10 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 741 ms | 1.08 | 1.16 | 0.74 | not met | not met |
| Delaunay `cube` D5 10^4 | 7.45 s | 1.02 | 1.05 | 0.80 | met | not met |

### Judgement

| Threshold | Met of the 38 judged sets, first reading | Second reading | Run of #609 |
| --- | ---: | ---: | ---: |
| 1.05 | 32 | 33 | 36 |
| 1.00 | 29 | 30 | 30 |

### The sets read twice

| Set | First | Second | Against | Run of #609 |
| --- | ---: | ---: | --- | ---: |
| Delaunay `cube` D2 10^4 | 1.07 | 1.07 | CGAL | 1.07 |
| Delaunay `sphere` D2 10^4 | 1.16 | 1.08 | CGAL | 1.12 |
| Delaunay `cube` D4 10^4 | 1.07 | 1.08 | CGAL | 1.05 |
| Hull `sphere` D4 10^5 | 1.03 | 1.07 | CGAL | 0.94 |
| Hull `sphere` D5 10^5 | 1.05 | 1.10 | CGAL | 1.00 |
| Hull `sphere` D3 10^4 | 1.09 | 1.04 | Qhull | 1.05 |
| Delaunay `cube` D5 10^4 | 1.06 | 1.02 | CGAL | 1.03 |
| Delaunay `sphere` D2 10^5 | 1.03 | 1.00 | CGAL | 1.01 |
| Hull `cube` D2 10^4 | 0.48 | 0.47 | Qhull | 0.79 |
| Hull `cube` D2 10^5 | 0.43 | 0.49 | Qhull | 0.82 |
| Hull `sphere` D2 10^4 | 0.97 | 0.92 | CGAL | 1.02 |

Not met in the second reading: the two Delaunay sets of D = 2 at 10^4, as before, and Delaunay `cube` D4 10^4 and hull `sphere` D4 and D5 at 10^5, which were met in the run of #609.

### The three that read worse: commit against commit

A ratio between two commits is not read from two runs. `327e5a1` (the commit of #609's run), the same built a second time as the control, and `217d389`, taking turns, pinned, under the same load; ms, the minimum, and the ratios of the minimum and of the median to `327e5a1`:

| Set | Builds a side | `327e5a1` | Control | `217d389` |
| --- | ---: | ---: | ---: | ---: |
| Hull `sphere` D4 10^5 | 6 | 1,315.7 | 1,319.6 (1.00, 1.00) | 1,345.0 (1.02, 1.01) |
| Hull `sphere` D5 10^5 | 3 | 11,672 | 11,536 (0.99, 0.99) | 11,582 (0.99, 0.99) |
| Hull `sphere` D4 10^4 | 18 | 92.52 | 90.62 (0.98, 1.00) | 93.78 (1.01, 1.01) |
| Hull `sphere` D5 10^4 | 5 | 894.0 | 890.1 (1.00, 0.99) | 896.6 (1.00, 1.00) |
| Hull `sphere` D3 10^4 | 50 | 19.34 | 19.36 (1.00, 1.01) | 19.07 (0.99, 1.02) |
| Hull `sphere` D3 10^5 | 12 | 283.2 | 275.8 (0.97, 1.00) | 280.7 (0.99, 1.01) |
| Delaunay `cube` D4 10^4 | 6 | 706.8 | 709.9 (1.00, 1.01) | 697.5 (0.99, 0.99) |
| Delaunay `cube` D5 10^4 | 3 | 7,877 | 7,233 (0.92, 0.99) | 7,820 (0.99, 1.00) |
| Delaunay `cube` D2 10^4 | 96 | 4.88 | 4.89 (1.00, 1.01) | 4.80 (0.98, 1.00) |
| Delaunay `sphere` D2 10^4 | 96 | 4.73 | 4.72 (1.00, 1.02) | 4.73 (1.00, 1.04) |
| Delaunay `cube` D3 10^5 | 6 | 397.8 | 405.1 (1.02, 1.03) | 397.8 (1.00, 1.01) |

convx builds these sets in the time it built them at `327e5a1`: 0.98 to 1.02. What moved is the ratio to CGAL under this run's conditions: on hull `sphere` D4 10^5 convx read 1.18 s and CGAL 1.26 s in the run of #609, and 1.45 s and 1.36 s here, both slower and convx by more. The other benchmark on the machine is the difference between the two days that is known; whether it is the cause was not measured.

### What moved since the run of #609

The larger of the ratios to CGAL and to Qhull, the one a set is judged by:

| Set | Run of #609 | This run |
| --- | ---: | ---: |
| Hull `cube` D2 10^4, 10^5, 10^6 | 0.79, 0.82, 0.97 | 0.47, 0.49, 0.59 |
| Hull `cube` D3 10^4, 10^5, 10^6 | 0.79, 0.86, 0.97 | 0.76, 0.77, 0.81 |
| Hull `cube` D4 10^4, 10^5 | 0.77, 0.94 | 0.74, 0.80 |
| Hull `cube` D5 10^4, 10^5 | 0.73, 0.67 | 0.71, 0.58 |
| Hull `grid` D6 10^4 | 0.65 | 0.52 |

Reported sets: the Delaunay lattice of D = 3 at 0.96 of CGAL and of D = 2 at 1.54; 10^6 Kuzmin sites at 0.84; Delaunay `cube` D3 at 1.08 to 1.13.

### Reading

- No set is slower than at `327e5a1` where the two were timed together, and the hulls of a cube are faster in every dimension.
- The count of sets met, 33 for 36, is this run's conditions on three sets whose ratio to CGAL sits near 1.05, and not a change of the crate. The judgement of those three wants a reading on a free machine, which the comparison's run (R1-8) needs as well.
- The two Delaunay sets of D = 2 at 10^4 stay not met, as since P7.

## The parity run of #652 read again, with the machine's load logged, #658

The run of #652 left the judgement of three sets to a reading on a freer machine. The 44 sets on `217d389` again (`main` has changed in documents alone since), one reading, with the processor's total load and the processes that used the most of each minute logged every minute by a script beside the run.

**The machine was freer than in #652 and not free.** The other project's benchmark was not running. Over the 48 minutes of the run the load read 8% at the least, 20% at the median, 33% at the ninth decile; the editor took 13 to 36 seconds of processor in 12 of the minutes, with the file indexer beside it. The judged builds of 23 sets spread by 1.2 or more, the largest 3.62 (Delaunay `cube` D3 10^4, reported).

| Set | convx `build()` | / CGAL | / CGAL from the points | / Qhull | At 1.05 | At 1.00 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| Hull `cube` D2 10^4 | 0.29 ms | 0.45 | 0.52 | 0.46 | met | met |
| Hull `cube` D2 10^5 | 2.81 ms | 0.36 | 0.52 | 0.44 | met | met |
| Hull `cube` D2 10^6 | 28.8 ms | 0.40 | 0.63 | 0.49 | met | met |
| Hull `cube` D3 10^4 | 1.20 ms | 0.52 | – | 0.74 | met | met |
| Hull `cube` D3 10^5 | 9.39 ms | 0.36 | – | 0.78 | met | met |
| Hull `cube` D3 10^6 | 130 ms | 0.20 | – | 0.85 | met | met |
| Hull `cube` D4 10^4 | 5.10 ms | 0.11 | – | 0.75 | met | met |
| Hull `cube` D4 10^5 | 26.6 ms | 0.05 | – | 0.75 | met | met |
| Hull `cube` D5 10^4 | 55.0 ms | 0.23 | – | 0.71 | met | met |
| Hull `cube` D5 10^5 | 208 ms | 0.12 | – | 0.62 | met | met |
| Hull `cube` D6 10^4 | 598 ms | 0.28 | – | 0.46 | met | met |
| Hull `cube` D6 10^5 | 2.66 s | 0.23 | – | 0.39 | met | met |
| Hull `sphere` D2 10^4 | 0.76 ms | 0.93 | 1.08 | 0.10 (ref.) | met | met |
| Hull `sphere` D2 10^5 | 9.55 ms | 0.89 | 1.14 | 0.08 (ref.) | met | met |
| Hull `sphere` D2 10^6 | 100 ms | 0.57 | 1.07 | 0.05 (ref.) | met | met |
| Hull `sphere` D3 10^4 | 19.6 ms | 0.85 | – | 1.08 | not met | not met |
| Hull `sphere` D3 10^5 | 248 ms | 0.63 | – | 1.03 | met | not met |
| Hull `sphere` D3 10^6 | 3.48 s | 0.42 | – | 1.02 | met | not met |
| Hull `sphere` D4 10^4 | 89.1 ms | 0.76 | – | 0.94 | met | met |
| Hull `sphere` D4 10^5 | 1.25 s | 0.96 | – | 0.91 | met | met |
| Hull `sphere` D5 10^4 | 798 ms | 0.83 | – | 0.99 | met | met |
| Hull `sphere` D5 10^5 | 10.22 s | 0.95 | – | 1.00 | met | met |
| Hull `sphere` D6 10^4 | 6.62 s | 0.72 | – | 0.82 | met | met |
| Hull `cubesurf` D3 10^5 | 26.2 ms | 0.69 | – | 0.90 | met | met |
| Hull `grid` D6 10^4 | 177 ms | 0.71 (ref.) | – | 0.52 | met | met |
| Delaunay `cube` D2 10^4 | 4.97 ms | 1.07 | 1.23 | 0.27 | not met | not met |
| Delaunay `cube` D2 10^5 | 53.5 ms | 1.01 | 1.29 | 0.23 | met | not met |
| Delaunay `cube` D2 10^6 | 566 ms | 0.79 | 1.20 | 0.16 | met | met |
| Delaunay `sphere` D2 10^4 | 5.24 ms | 1.21 | 1.34 | 0.22 (ref.) | not met | not met |
| Delaunay `sphere` D2 10^5 | 44.1 ms | 1.02 | 1.30 | 0.12 (ref.) | met | not met |
| Delaunay `sphere` D2 10^6 | 428 ms | 0.82 | 1.39 | 0.03 (ref.) | met | met |
| Delaunay `cube` D3 10^4 | 73.5 ms | 1.13 | 1.30 | 0.21 | reported | reported |
| Delaunay `cube` D3 10^5 | 400 ms | 1.15 | 1.26 | 0.31 | reported | reported |
| Delaunay `cube` D3 10^6 | 4.31 s | 1.10 | 1.24 | 0.26 | reported | reported |
| Delaunay `sphere` D3 10^4 | 72.5 ms | 0.53 | 0.54 | 0.58 (ref.) | met | met |
| Delaunay `sphere` D3 10^5 | 679 ms | 0.68 | 0.70 | 0.35 (ref.) | met | met |
| Delaunay `sphere` D3 10^6 | 5.73 s | 0.81 | 0.80 | 0.33 (ref.) | met | met |
| Delaunay `cube` D4 10^4 | 691 ms | 1.06 | 1.12 | 0.88 | not met | not met |
| Delaunay `sphere` D4 10^4 | 941 ms | 0.36 | 0.36 | 0.70 (ref.) | met | met |
| Delaunay `cube` D5 10^4 | 7.39 s | 1.02 | 0.96 | 0.76 | met | not met |
| Delaunay `sphere` D5 10^4 | 11.38 s | 0.32 | 0.32 | 0.68 (ref.) | met | met |
| Delaunay `lattice` D2 10^4 | 7.95 ms | 1.47 | 1.70 | 0.22 | reported | reported |
| Delaunay `lattice` D3 27,000 | 126 ms | 0.95 | 0.98 | 0.14 | reported | reported |
| Delaunay `kuzmin` D2 10^6 | 664 ms | 0.92 | 1.32 | 0.15 | reported | reported |

### Judgement

| Threshold | Run of #609 | #652, second reading | This run |
| --- | ---: | ---: | ---: |
| 1.05 | 36 | 33 | 34 |
| 1.00 | 30 | 30 | 29 |

| Set | Run of #609 | #652, second reading | This run | Against |
| --- | ---: | ---: | ---: | --- |
| Hull `sphere` D4 10^5 | 0.94 | 1.07 | 0.96 | CGAL |
| Hull `sphere` D5 10^5 | 1.00 | 1.10 | 1.00 | Qhull here, CGAL before |
| Delaunay `cube` D4 10^4 | 1.05 | 1.08 | 1.06 | CGAL |
| Hull `sphere` D3 10^4 | 1.05 | 1.04 | 1.08 | Qhull |
| Delaunay `cube` D2 10^4 | 1.07 | 1.07 | 1.07 | CGAL |
| Delaunay `sphere` D2 10^4 | 1.12 | 1.08 | 1.21 | CGAL |

### A reading that is not used

A reading before this one, on the same commit, is not used. Its judged builds spread by 1.2 or more on 31 of the 44 sets and by up to 9.61 (hull `sphere` D4 10^5: 1.25 s to 12.03 s), and it counted 30 sets met. No load was logged through it, so its cause was not measured; while it ran, the working tree of this repository was edited in 26 files, searched through `target/`, and switched between branches, each of which sets the editor's checker building. The log of this run was added for that: a reading whose minutes cannot be accounted for is not a reading.

### Reading

- Hull `sphere` D4 and D5 at 10^5 are met, at the ratios of the run of #609 (0.96 and 1.00 for 0.94 and 1.00). Their not being met in #652 was that run's conditions, as the commit against commit there said.
- Delaunay `cube` D4 10^4 sits on the threshold: 1.05, 1.08, 1.06 in three runs, with the build itself unchanged since `327e5a1` (0.99, #652). It is a set at the edge, not a regression.
- Hull `sphere` D3 10^4 reads 1.08 of Qhull here and 1.04 and 1.05 before; its build did not change either (0.99, #652). The two Delaunay sets of D = 2 at 10^4 stay not met, as since P7.
- A count of sets met moves by two or three between runs of one commit on this machine while it is in use. The comparison of R1-8 is taken with the editor closed and the load logged, and a ratio within 0.05 of a threshold is stated as at the threshold.

## Where the time of the Delaunay build of D = 2 goes, beside spade, #667

interpx's linear interpolator builds on `DelaunayBuilder::new(2, points).build()`, and found that build 1.3 to 2.4 times `spade`'s whole triangulation (`DelaunayTriangulation::bulk_load`, spade 2.15.1) on points uniform in the unit square (#667). This is R2-1, the measurement the owner asked for before anything is prototyped. The tools are `benches/d2/` (a probe and the stage marks, which never go into `src`), on `main` at `dab62ba` (0.1.0 and the roadmap).

Method: one process, pinned to core 2 under WSL2 (Ubuntu 22.04, i5-13400F); per round convx, convx again (the control), and spade, taking turns; medians of 7 to 20,000 rounds after a warm-up. spade's time includes building its `Point2` vector from the slice, which convx reads directly. The processor's load read 48% and 37% before the run and 21% after it. On Windows 11 the same probe, pinned by its affinity, reproduces interpx's table.

| Points | convx | Control | `spade` | convx / `spade` | Control / convx | Windows: convx | Windows: `spade` | Windows: convx / `spade` |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 3.14 µs | 3.05 µs | 1.59 µs | 1.97 | 0.97 | 8.50 µs | 3.70 µs | 2.30 |
| 100 | 27.84 µs | 27.63 µs | 20.07 µs | 1.39 | 0.99 | 58.00 µs | 43.60 µs | 1.33 |
| 1,000 | 0.51 ms | 0.50 ms | 0.34 ms | 1.49 | 0.98 | 0.87 ms | 0.60 ms | 1.45 |
| 10^4 | 5.44 ms | 5.60 ms | 4.12 ms | 1.32 | 1.03 | 6.50 ms | 4.58 ms | 1.42 |
| 10^5 | 55.98 ms | 58.09 ms | 46.35 ms | 1.21 | 1.04 | 63.44 ms | 47.78 ms | 1.33 |
| 10^6 | 606.04 ms | 584.48 ms | 542.17 ms | 1.12 | 0.96 | 609.76 ms | 538.59 ms | 1.13 |

The control reads 0.96 to 1.04 of convx. Windows reads 2.30 at 10 points and 1.33 to 1.45 from 100 to 10^5, as interpx found; at 10 points both libraries take 2.3 to 2.7 times their time under WSL.

### The stages of convx's build

The share of each stage in the marked build (its total within 6% of the plain one), and the insertion alone against spade's whole build:

| Points | Scaling | Acceptance | Sites | Flat | Order | Copy | Insertion | After the insertion | Publication | Insertion / `spade` |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 1% | 9% | 3% | 3% | 9% | 3% | 55% | 13% | 5% | 1.16 |
| 100 | 1% | 4% | 2% | 0% | 10% | 1% | 67% | 13% | 1% | 0.94 |
| 1,000 | 0% | 2% | 1% | 0% | 6% | 1% | 77% | 11% | 2% | 1.11 |
| 10^4 | 0% | 2% | 1% | 0% | 7% | 1% | 75% | 14% | 2% | 0.97 |
| 10^5 | 0% | 2% | 1% | 0% | 7% | 1% | 73% | 14% | 1% | 0.85 |
| 10^6 | 0% | 3% | 1% | 0% | 8% | 3% | 69% | 16% | 1% | 0.76 |

- **The insertion** (the walk, the flips, and the in-circle tests: `Mesh::build` and `conflict` in a profile) is 55 to 77% of the build, and alone it takes 0.76 to 1.16 of spade's whole build: under spade's at 100 points and from 10^4, above it at 10 and 1,000.
- **After the insertion** (the union of cospherical faces, the drafts of the cells, the rows written for publication) is 11 to 16%: work spade does not do in this form, the largest part of the gap from 10^4 points.
- **The order** (BRIO with the Morton sort of each round) is 6 to 10%. spade orders its points by their distance to a centre (a sort of `Point2` by key in its profile).
- **At 10 points**, the stages around the insertion are 45%: the acceptance with the duplicates 9%, the order 9%, after the insertion 13%. The profile at 10 points has `malloc`, `free`, and `realloc` at 15% and building the vector of lifted heights at 6%; Windows' allocator makes the same counts dearer, which is why Windows reads 2.30 where WSL reads 1.97.

spade's bulk load is not an incremental insertion with a walk: it inserts points by their distance to a centre onto the hull of those inserted so far, and flips after each (`single_bulk_insertion_step`, `Hull::insert` in the profile).

### What could be prototyped

Each is measured against spade's bulk load on the same probe before it is taken or left (`.cursor/rules/bench.mdc`):

1. **After the insertion, for D = 2**: the cells and their neighbors written from the mesh in one pass when no face was recorded cospherical, and the union only where one was. The stage is 13 to 16%.
2. **The order**: a cheaper order for D = 2 (one sort by a space-filling key, or spade's by distance), with the walk's length measured beside it. The stage is 6 to 10%, and the order feeds the insertion's walk.
3. **Small inputs**: the allocations of one build counted and cut (one buffer for the lifted heights, buffers sized once), and the acceptance's path for a few points. 45% of the build at 10 points is outside the insertion.
4. **The insertion itself**: spade's way, insertion by distance onto the growing hull, as a path of D = 2 (the owner allows dedicated paths of D = 2 and 3). The largest change and the one stage that is already at spade's from 10^4 points.

## The prototypes for the Delaunay build of D = 2, measured beside spade, #667

The prototypes of R2-1, each timed by `benches/d2/ab.sh` against the commit before it and that commit built a second time (the control), three runs each, pinned to core 2 under WSL2; the ratios are of the median build, and convx / spade is read in the same process, which the machine's load moves less. Taken or left by those numbers (the owner, 2026-10-18: take what is measured to help, and optimize the paths of D = 2 and 3 where there is room).

| Change | Ratio to the commit before | Control | Decision |
| --- | --- | --- | --- |
| No face walked after a D = 2 insertion that recorded no cospherical face | 0.88 to 0.98 from 100 points; 1.00 at 10 | 0.97 to 1.03 | Taken |
| A Delaunay build keeps no groups (only a Voronoi diagram reads them) | 0.97 to 1.04 | 0.94 to 1.00 | Left: within the control |
| The order of D = 2 on `u64` keys, a grid of 13 bits a coordinate | 0.94 to 0.98; 0.94 at 10^4 | 0.98 to 1.01 | Taken, as one order for every dimension with the key a parameter |
| The mesh's growing lists sized once | 0.97 at 10 points | 1.00 | Taken |
| The walk's signs read again for the edge a site lies on (two orientations a site fewer) | 1.01 to 1.02 | 0.99 to 1.01 | Left: slower; an orientation costs less than passing its sign |
| The order of D = 2 along the Hilbert curve | 1.09 at 10, 1.04 at 100, 0.97 to 0.99 above | 0.95 to 1.01 | Left: its code costs more than its walk saves |
| The insertion by flips keeps candidates for cospherical faces, tested again after it, not a record per face | convx / spade 1.32 to 1.23 at 1,000, 1.20 to 1.11 at 10^4, 1.01 to 0.91 at 10^6 | load: 0.86 to 1.10 | Taken (a throwaway build without the records bounded it at 0.86 to 0.92) |
| The inside sign once a dimension, and the flips' mesh without the cavity's lists | 0.82 at 10 points; 0.99 to 1.01 above | 0.99 to 1.01 | Taken: 57 allocations and reallocations of a build of 10 points to 45 |
| The small working lists on the stack, the union's parents made when used | 0.96 at 10 points; 0.99 to 1.00 above | 0.99 to 1.02 | Taken: 45 to 34 |
| The flips' mesh without the cavity's marks | 0.98 to 1.01 | 0.99 to 1.01 | Left: within the control |

What a site of D = 2 costs the insertion, counted at 10^3 to 10^5 points: 11.9 orientations, 9.0 in-circle tests, 5.5 steps of the walk, 3.0 flips. The flips and in-circle tests are what any insertion by flips needs; the walk is 35% of the insertion's time and the split and flips the rest.

### Everything taken, against `main`

| Points | WSL: ratio to `main` | Control | convx / spade, `main` → taken | Windows: ratio to `main` | convx / spade, `main` → taken |
| ---: | ---: | ---: | --- | ---: | --- |
| 10 | 0.72 | 0.99 | 2.04 → 1.42 | 0.68 | 2.32 → 1.58 |
| 100 | 0.87 | 1.01 | 1.40 → 1.22 | 0.86 | 1.37 → 1.19 |
| 1,000 | 0.82 | 1.01 | 1.47 → 1.22 | 0.81 | 1.47 → 1.21 |
| 10^4 | 0.85 | 1.01 | 1.34 → 1.10 | 0.82 | 1.43 → 1.14 |
| 10^5 | 0.86 | 0.96 | 1.26 → 1.04 | 0.84 | 1.29 → 1.04 |
| 10^6 | 0.93 | 0.98 | 1.11 → 0.98 | 0.86 | 1.18 → 1.00 |

What is left between 100 and 10^4 points is the insertion itself: spade's bulk load inserts each site, by its distance to a centre, onto the hull of the sites before it, and walks nowhere.

## The Delaunay build of D = 3 beside CGAL, #667

The same probe with `PROBE_DIM=3` (`benches/d2/`), points uniform in the unit cube, and CGAL 5.4 (`benches/parity/cgal_time.cpp`, three processes a size) on the same points written by the probe; pinned to core 2 under WSL2, on the head of #669.

| Points | convx | CGAL | convx / CGAL | Insertion alone / CGAL | After the insertion |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 6.0 us | 4.0 us | 1.51 | 0.96 | 18% |
| 100 | 228 us | 199 us | 1.15 | 0.93 | 16% |
| 1,000 | 3.57 ms | 3.16 ms | 1.13 | 0.91 | 17% |
| 10^4 | 42.9 ms | 36.7 ms | 1.17 | 0.94 | 17% |
| 10^5 | 484 ms | 402 ms | 1.20 | 1.01 | 20% |

As in D = 2 before #669, the insertion alone is at CGAL's whole build, and the gap is the work after it: of it the walk of the faces is 12 to 14% of the build (70 ms at 10^5 points), the cells written 4%. The faces walked are those between two tetrahedra one insertion made, left undecided until then.

- Deciding those faces in the insertion, where the four sites besides the new one are at hand (affinely independent: the face joins nothing; coplanar: left for the lifted test after it), changes nothing alone: 1.02 to 1.04 of its base from 100 to 10^4 points, within 0.99 to 1.00 of the control at 10^5, since the faces are still walked.
- With it, the walk is skipped when the insertion left no face undecided and recorded none cospherical: 0.93, 0.95, 0.97, 0.95, and 0.92 of the base at 10 to 10^5 points, against a control of 0.99, 1.00, 0.99, 0.97, and 0.94. Taken.

A face left undecided by four coplanar sites came, in every input the tests and a pyramid of five cospherical sites (a square and an apex, with the apex at every index among sites outside the sphere) built, with a face recorded cospherical as well: four coplanar sites on a sphere are on one circle, and inserting the last of them meets a simplex of the other three. So a mutation that never marks a face undecided fails no test; the mark is kept because nothing proves that a record always comes with it.

### D = 2 inserted radially, as spade's bulk load orders it

A prototype of the insertion of D = 2 in spade's order, the insertion by flips kept: the initial triangle the site nearest the centre of the box, the site nearest it, and the third of the smallest circumradius off their line; the rest by distance from that circumcenter, so that a site lies, nearly always, just outside the hull before it; and each walk started from the triangle the last insertion in the site's sector of the angle about the centre left. Against its base (the head of #671), with the control:

| Points | Radial | Control |
| ---: | ---: | ---: |
| 10 | 1.13 | 1.00 |
| 100 | 1.02 | 0.99 |
| 1,000 | 1.02 | 1.01 |
| 10^4 | 1.20 | 1.01 |
| 10^5 | 1.37 | 1.01 |
| 10^6 | 1.38 | 0.92 |

Left. Consecutive sites of that order are far apart in the input and in the mesh, where BRIO keeps them near; what of the 1.37 is the memory and what the flips was not measured. spade's bulk load keeps its own structure for the hull and is not a walk over a triangulation.

### D = 2 on half-edges

The insertion by flips on a structure of its own: a triangle holds its three vertices and, for each edge, the half-edge across it, so a split or a flip links the triangles it changes without looking for the slot that points back; the order, the walk, the predicates, and the candidates are those of the mesh, and a test holds what the two publish equal. Each change timed as above, against the commit before it and its control.

| Change | Ratio to the commit before | Control | Decision |
| --- | --- | --- | --- |
| The triangles on half-edges | 0.96 at 10 points, 0.99 to 1.00 from 1,000; convx / spade 1.21 to 1.21 at 1,000, 1.10 to 1.08 at 10^4, 0.98 to 0.94 at 10^6 | 0.92 to 1.04 | Taken with the next row |
| The triangles holding their vertices' coordinates, so that a test reads no other site's row | 0.94 to 1.09 | 0.98 to 1.03 | Left: within the control; the triangle grows from 24 to 72 bytes |
| The conflict and orientation tests inlined into the walk and the flips, where they had been calls returning a `Result` through memory (a profile gave them 49% of the samples) | 0.89 to 0.98; convx / spade 1.22 to 1.13 at 1,000, 1.08 to 1.03 at 10^4, 0.95 to 0.92 at 10^6 | 0.94 to 1.01 | Taken |
| The walk's signs read again for the edge a site lies on, on half-edges | 1.02 to 1.05 | 0.94 to 1.03 | Left: slower, as in the mesh |
| One round of Morton order instead of BRIO (a measurement) | 0.91 at 1,000, 1.04 to 1.05 at 10^4 and 10^5 | 0.96 to 1.03 | Left: the rounds cost the walk nothing measurable |
| A split pushes its new triangles whole, where it pushed blanks built on the stack and read back across two stores (VTune: loads blocked by store forwarding, 7% of the clockticks) | 0.93 to 0.96 from 10 to 10^4; convx / spade 1.14 to 1.11 at 1,000, 0.90 to 0.87 at 10^6 | 0.96 to 1.08 | Taken |
| The walk tests every edge of a step and chooses the edge without a branch on each sign (VTune: bad speculation, half the slots of the walk) | convx / spade 1.10 to 1.15 at 1,000, 1.16 to 1.25 at 100 | 0.98 to 1.09 | Left: the orientation it adds costs more than the mispredictions it removes |
| No list for the first simplex and the order of the copy in insertion order; the representatives and the order's keys sized once; the rows of the sites made once, from the input, where they were made in the input's order and copied | 0.97 at 10 points, 0.96 to 1.01 above | 0.97 to 1.04 | Taken: 29 allocations and reallocations of a build of 10 points to 22 |
| The order with its first simplex in one list, owned by the rows; no pairing of a complex of single cells | 0.94 at 10 points, 0.97 to 0.99 above | 0.97 to 1.00 | Taken: 22 to 18 |
| The links of a drafted cell appended at once, not pushed one by one (VTune: the drafted cells 330 instructions a site) | 1.05 to 1.08 from 100 to 10^4 points | 0.98 to 1.00 | Left: slower |
| A triangulation of single cells built for itself published from the half-edges directly, each triangle turned to start at its smallest site: no draft, sort, parity, or group | 0.87 at 10 points, 0.93 to 0.98 above; convx / spade 1.12 to 0.98 at 10, 1.10 to 1.06 at 1,000, 0.88 to 0.82 at 10^6 | 0.98 to 1.01 | Taken |
| The order of D = 2 sorted by radix on its round and code, a byte a pass, from 128 sites | convx / spade 1.05 to 1.04 at 1,000, 0.97 to 0.97 at 10^4 | 0.97 to 1.03 | Left: within the control; perf gives the sort a third of the order |
| The triangles published directly written in place, three at a time, into lists zeroed first (VTune: the direct publication 365 instructions a site, most in its pushes) | convx / spade 1.05 to 1.07 at 1,000, 0.89 to 0.93 at 10^5 | 0.96 to 1.02 | Left: slower |

Everything taken, against the head of #671, with the control of that head built twice:

| Points | Ratio to #671 | Control | convx / spade, #671 → taken |
| ---: | ---: | ---: | --- |
| 10 | 0.63 | 0.94 | 1.45 → 0.96 |
| 100 | 0.95 | 1.07 | 1.20 → 1.06 |
| 1,000 | 0.89 | 1.05 | 1.22 → 1.04 |
| 10^4 | 0.85 | 1.00 | 1.10 → 0.97 |
| 10^5 | 0.82 | 1.01 | 1.02 → 0.89 |
| 10^6 | 0.85 | 1.03 | 0.96 → 0.83 |

Where the build of 1,000 points spends its time, on the head with the tests inlined: by perf in WSL2 (4,000 samples a second), 78% of the samples in the insertion, most of them in the first stage of the predicates, the order 5%, the pass after the insertion and publication 7%, the acceptance and the rows of the sites 4%. By VTune's hardware sampling on Windows (`uarch-exploration`, one core), memory is not what holds it: 2.5% of the slots are memory bound, and the core retires 3.1 instructions a clock. The build retires about 4,600 instructions a site, of which the conflict tests of the flips are 1,580 (about 175 a test, the in-circle filter about 60 of them), and 23% of the slots are lost to mispredicted branches, half of the walk's: the edge a step leaves by and the outcome of a conflict test are not predictable. The stages of the head (`benches/d2/marks.py`), at 1,000 points: the insertion 288 us, under spade's whole build of 334 us; the pass after the insertion 28 us, the order 21 us, the acceptance 10 us, publication 5 us, the rows of the sites 4 us. A second VTune profile, on the head with the splits fixed: 8% fewer clockticks, loads blocked by store forwarding 7.1% to 2.6% of them, 22% of the slots lost to mispredicted branches, 1.3% memory bound; the pass after the insertion 5.3% of the clockticks and 450 instructions a site, two thirds of them in drafting the cells. What is left between 100 and 1,000 points is the work around the insertion, and in the insertion that count of instructions and those branches. After the direct publication, at 1,000 points: the groups 3 us and the cells written and published 14 us, where the pass and publication took 33 us; the order 21 us, of which perf gives the comparison sort a third and the keys (the round's hash, the box, the Morton codes) the rest; the acceptance 10 us. What is left at 100 to 1,000 points, 4 to 6%, is spread over those. A third VTune profile on that head: 39.8 billion clockticks where the second had 41.6, loads blocked by store forwarding 1.1% of them; the order 4% of the clockticks, spread over the box (0.7%), the Morton codes (1.1%), the keys (0.3%), and the sort (0.6%), none of which would move the build by more than about 0.5%.
