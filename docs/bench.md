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
| D3 `insert` (1.27 s own) | `self.link_by_keys(&boundary, &created)`: pairing the new tetrahedra's faces | 59.7% |
| | `self.conflict(n, q)` (outside the test itself) | 10.4% |
| D2 `insert_by_flips` (0.61 s own) | `self.flip(c, n, q)` | 55.5% |
| | finding the slot, reading the neighbor, recording | 9 to 7% each |
| D2 `locate` (0.12 s own) | the random offset of the walk (`% k`), twice per step | 28% |
| D3 `alloc` (0.14 s own) | copying the vertices, filling the neighbors and records | 100% |

### Reading

- **D3: pairing the new faces is about 21% of the build.** `link_by_keys` pairs about 60 faces per point through an open-addressing table keyed by an edge. Whether the time is the key and the probe, the new cells' reads, or their writes needs an instruction-level profile of that function; that is the first step of P7-37 (#421).
- **The predicates are about 30% of D2 and 35% of D3**, at 8 to 27 ns per call through the semi-static first stage. A static filter, which bounds every call of one input once, is a design question (certification for every accepted input, the fallback, the stages after it): P7-38 (#422), set after its Grill.
- **D2's BRIO order is 11%**: the space-filling keys about 5% and their sort about 4%. That is P7-39 (#423).
- **Smaller items, read again after these rows:** D2's flips (about 7% of the build), the walk's modulo (about 4%), and cell allocation (4 to 7%).
