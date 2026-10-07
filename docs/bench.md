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

## Four timed sections, convx `029eb3f` (#250)

### Method

| Item | Value |
| --- | --- |
| convx | `feat/249-semi-static-filter` at `029eb3f` (P6-1 on top of `main` at `f9e24ea`), rustc 1.97.0, `--release` with debug info, baseline target |
| CGAL | 5.6 (Ubuntu `libcgal-dev`), g++ 13.3, the program and flags of #215 unchanged |
| Qhull | 2020.2 (Ubuntu `qhull-bin`), `qconvex i s TI <file> TO <out>` and `qdelaunay i s TI <file> TO <out>` |
| Machine | Linux VM, 4 vCPU Intel Xeon @ 2.10 GHz (AVX-512), the machine of the section above |
| Cores | One, every process pinned with `taskset -c 2` |
| Points | `benches/sets.txt`, seed 1, generator `xoshiro256starstar-v1`, written by `export_qhull_sets`; every tool reads the same file |
| Rounds | One unrecorded convx build per set first. Tools alternated per round (convx, CGAL, Qhull). When that build took under 1.5 s: 5 rounds × 3 builds per process (Qhull: 3 runs per round). Longer: 3 rounds × 1 |
| Reported | Median, with min–max in parentheses. Ratios are medians divided; below 1 means convx is faster |

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

## After P6, convx `b9d7736` (#275)

### Method

The method of the section above (#250), unchanged except:

| Item | Value |
| --- | --- |
| convx | `docs/269-spike-outcome` at `b9d7736`: P6-1 to P6-11 (#249 to #269) on top of `main` at `f9e24ea`. rustc 1.97.0, `--release` with debug info, baseline target |
| Phase timers | In the same scratch copy as the construction timer, not committed. Hull: construction (`SimplicialHull::build`), the pass after it (merge and classification), and publication. Delaunay: construction (the insertion order and `insert::Mesh::build`), the pass after it (groups, the draft, numbering), and publication |
| CGAL | 5.6, the program and flags of #215, rebuilt |
| Qhull | 2020.2, `qconvex i s TI <file> TO <out>` and `qdelaunay i s TI <file> TO <out>` |

The timing programs and the runner are on #275.

### Counts

convx and CGAL agree on every vertex, facet, and Delaunay simplex count below. Qhull agrees too, except where its tolerance merges nearly degenerate pieces, as in the section above: hull `sphere` D2 at 10^5 and 10^6 (99,996 and 998,669 vertices), and Delaunay `sphere` at every size and dimension (for example D3 10^6: 2,911,192 regions against 3,017,144 simplices). Those Qhull cells are not the same output.

### Hull

| Set | convx `build()` | convx construction | CGAL | Qhull compute | Qhull whole run | `build()` / CGAL | `build()` / Qhull compute | Construction / Qhull compute | `build()` / Qhull whole run | Construction / pass after / publication, % of `build()` | Qhull hyperplanes / distance tests | Vertices / facets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 0.74 ms (0.64 ms–0.95 ms) | 0.50 ms (0.43 ms–0.64 ms) | 0.65 ms (0.63 ms–0.75 ms) | 0.73 ms | 5.19 ms | 1.14 | 1.02 | 0.68 | 0.14 | 67 / 11 / 1 | 45 / 65,050 | 24 / 24 |
| `cube` D2 10^5 | 7.17 ms (6.55 ms–8.39 ms) | 4.45 ms (4.30 ms–4.65 ms) | 6.12 ms (5.96 ms–7.45 ms) | 7.28 ms | 28.8 ms | 1.17 | 0.99 | 0.61 | 0.25 | 62 / 10 / 0 | 48 / 699,388 | 25 / 25 |
| `cube` D2 10^6 | 91.1 ms (79.1 ms–108 ms) | 45.0 ms (42.2 ms–50.4 ms) | 65.4 ms (63.0 ms–74.1 ms) | 79.2 ms | 277 ms | 1.39 | 1.15 | 0.57 | 0.33 | 49 / 9 / 0 | 76 / 6,417,683 | 39 / 39 |
| `cube` D3 10^4 | 1.82 ms (1.64 ms–2.32 ms) | 1.48 ms (1.31 ms–1.81 ms) | 2.92 ms (2.70 ms–3.07 ms) | 1.86 ms | 7.32 ms | 0.62 | 0.98 | 0.79 | 0.25 | 81 / 6 / 6 | 755 / 138,265 | 121 / 238 |
| `cube` D3 10^5 | 16.5 ms (14.7 ms–18.3 ms) | 12.9 ms (11.9 ms–14.5 ms) | 51.1 ms (43.3 ms–57.5 ms) | 14.8 ms | 46.2 ms | 0.32 | 1.12 | 0.87 | 0.36 | 78 / 3 / 1 | 952 / 1,279,410 | 175 / 346 |
| `cube` D3 10^6 | 219 ms (194 ms–250 ms) | 173 ms (156 ms–193 ms) | 777 ms (721 ms–861 ms) | 175 ms | 463 ms | 0.28 | 1.25 | 0.99 | 0.47 | 79 / 3 / 0 | 1,888 / 13,617,348 | 285 / 566 |
| `cube` D4 10^4 | 9.42 ms (8.90 ms–12.4 ms) | 7.49 ms (7.05 ms–10.5 ms) | 52.2 ms (49.4 ms–64.5 ms) | 8.01 ms | 15.5 ms | 0.18 | 1.18 | 0.93 | 0.61 | 79 / 7 / 11 | 11,158 / 402,164 | 404 / 2,320 |
| `cube` D4 10^5 | 59.5 ms (57.4 ms–63.3 ms) | 52.4 ms (50.8 ms–56.5 ms) | 704 ms (667 ms–755 ms) | 42.5 ms | 84.1 ms | 0.08 | 1.40 | 1.23 | 0.71 | 88 / 3 / 4 | 20,086 / 3,590,089 | 770 / 4,376 |
| `cube` D5 10^4 | 82.6 ms (78.5 ms–87.6 ms) | 62.8 ms (59.9 ms–67.8 ms) | 271 ms (260 ms–296 ms) | 105 ms | 128 ms | 0.31 | 0.78 | 0.60 | 0.65 | 76 / 10 / 14 | 125,948 / 2,010,291 | 961 / 20,232 |
| `cube` D5 10^5 | 338 ms (322 ms–349 ms) | 277 ms (268 ms–295 ms) | 2.11 s (2.02 s–2.16 s) | 441 ms | 521 ms | 0.16 | 0.77 | 0.63 | 0.65 | 82 / 7 / 9 | 344,329 / 20,893,652 | 2,339 / 48,818 |
| `cube` D6 10^4 | 913 ms (892 ms–967 ms) | 633 ms (609 ms–675 ms) | 2.72 s (2.64 s–2.85 s) | 1.70 s | 1.86 s | 0.34 | 0.54 | 0.37 | 0.49 | 69 / 14 / 17 | 1,234,219 / 13,946,956 | 1,882 / 174,102 |
| `cube` D6 10^5 | 3.90 s (3.77 s–4.02 s) | 2.88 s (2.79 s–2.98 s) | 14.6 s (14.5 s–15.2 s) | 9.44 s | 10.3 s | 0.27 | 0.41 | 0.31 | 0.38 | 74 / 12 / 13 | 4,718,136 / 149,602,813 | 5,444 / 518,754 |
| `sphere` D2 10^4 | 3.49 ms (3.17 ms–4.22 ms) | 0.86 ms (0.81 ms–0.97 ms) | 0.79 ms (0.76 ms–1.03 ms) | 8.36 ms | 15.4 ms | 4.40 | 0.42 | 0.10 | 0.23 | 25 / 18 / 54 | 19,998 / 189,622 | 10,000 / 10,000 |
| `sphere` D2 10^5 | 45.2 ms (39.8 ms–52.9 ms) | 10.8 ms (9.90 ms–15.9 ms) | 9.95 ms (9.34 ms–11.2 ms) | 159 ms | 225 ms | 4.54 | 0.28 | 0.07 | 0.20 | 24 / 16 / 57 | 199,989 / 2,395,166 | 100,000 / 100,000 |
| `sphere` D2 10^6 | 766 ms (700 ms–821 ms) | 185 ms (170 ms–214 ms) | 119 ms (109 ms–139 ms) | 3.00 s | 3.99 s | 6.43 | 0.26 | 0.06 | 0.19 | 24 / 13 / 57 | 1,997,357 / 28,928,990 | 999,973 / 999,973 |
| `sphere` D3 10^4 | 46.4 ms (44.3 ms–50.3 ms) | 32.8 ms (31.4 ms–36.0 ms) | 34.8 ms (30.2 ms–43.3 ms) | 25.6 ms | 42.4 ms | 1.33 | 1.81 | 1.28 | 1.09 | 71 / 11 / 17 | 56,158 / 341,173 | 10,000 / 19,996 |
| `sphere` D3 10^5 | 669 ms (629 ms–700 ms) | 465 ms (440 ms–490 ms) | 502 ms (458 ms–603 ms) | 343 ms | 509 ms | 1.33 | 1.95 | 1.35 | 1.31 | 69 / 14 / 17 | 565,222 / 4,349,516 | 100,000 / 199,996 |
| `sphere` D3 10^6 | 10.6 s (10.3 s–10.7 s) | 7.48 s (7.24 s–7.73 s) | 11.0 s (10.1 s–11.3 s) | 5.26 s | 7.75 s | 0.97 | 2.02 | 1.42 | 1.37 | 70 / 13 / 15 | 5,654,825 / 52,940,589 | 1,000,000 / 1,999,996 |
| `sphere` D4 10^4 | 213 ms (205 ms–248 ms) | 149 ms (140 ms–178 ms) | 142 ms (138 ms–169 ms) | 138 ms | 191 ms | 1.50 | 1.55 | 1.08 | 1.12 | 70 / 13 / 17 | 258,048 / 889,697 | 10,000 / 67,192 |
| `sphere` D4 10^5 | 2.92 s (2.82 s–3.10 s) | 1.93 s (1.89 s–2.07 s) | 1.58 s (1.56 s–1.71 s) | 2.07 s | 2.90 s | 1.85 | 1.41 | 0.93 | 1.01 | 66 / 16 / 17 | 2,626,423 / 10,538,224 | 100,000 / 675,154 |
| `sphere` D5 10^4 | 1.39 s (1.34 s–1.47 s) | 943 ms (880 ms–1.02 s) | 1.19 s (1.12 s–1.23 s) | 1.12 s | 1.40 s | 1.17 | 1.25 | 0.84 | 1.00 | 68 / 16 / 17 | 1,413,713 / 3,017,965 | 10,000 / 299,994 |
| `sphere` D5 10^5 | 19.0 s (18.5 s–19.2 s) | 11.7 s (11.5 s–12.3 s) | 15.5 s (15.3 s–16.1 s) | 16.1 s | 21.0 s | 1.23 | 1.18 | 0.73 | 0.90 | 62 / 19 / 17 | 15,164,634 / 35,472,228 | 100,000 / 3,112,922 |
| `sphere` D6 10^4 | 10.9 s (10.8 s–11.0 s) | 7.45 s (7.37 s–7.58 s) | 12.0 s (11.8 s–12.2 s) | 12.7 s | 15.2 s | 0.91 | 0.86 | 0.59 | 0.72 | 68 / 15 / 17 | 8,596,125 / 14,508,402 | 10,000 / 1,570,453 |

Not timed: `sphere` D6 10^5, as in the sections above.

### Delaunay

| Set | convx `build()` | convx construction | CGAL | Qhull compute | Qhull whole run | `build()` / CGAL | `build()` / Qhull compute | Construction / Qhull compute | `build()` / Qhull whole run | Construction / pass after / publication, % of `build()` | Qhull hyperplanes / distance tests | Sites / simplices |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` D2 10^4 | 21.0 ms (20.8 ms–26.8 ms) | 16.6 ms (16.4 ms–20.6 ms) | 4.85 ms (4.72 ms–5.15 ms) | 23.9 ms | 38.8 ms | 4.33 | 0.88 | 0.69 | 0.54 | 79 / 19 / 1 | 56,426 / 377,169 | 10,000 / 19,974 |
| `cube` D2 10^5 | 258 ms (237 ms–286 ms) | 186 ms (174 ms–208 ms) | 51.9 ms (49.9 ms–58.3 ms) | 345 ms | 534 ms | 4.97 | 0.75 | 0.54 | 0.48 | 72 / 24 / 1 | 565,024 / 4,837,796 | 100,000 / 199,973 |
| `cube` D2 10^6 | 3.23 s (3.03 s–3.31 s) | 2.22 s (2.13 s–2.26 s) | 623 ms (622 ms–653 ms) | 5.10 s | 8.24 s | 5.18 | 0.63 | 0.44 | 0.39 | 69 / 29 / 0 | 5,661,165 / 56,857,785 | 1,000,000 / 1,999,959 |
| `sphere` D2 10^4 | 20.9 ms (20.3 ms–21.5 ms) | 18.0 ms (17.5 ms–18.5 ms) | 4.59 ms (4.49 ms–5.10 ms) | 32.2 ms | 46.1 ms | 4.56 | 0.65 | 0.56 | 0.45 | 86 / 12 / 0 | 45,385 / 437,697 | 10,000 / 9,998 |
| `sphere` D2 10^5 | 190 ms (171 ms–210 ms) | 153 ms (139 ms–174 ms) | 39.9 ms (37.5 ms–43.8 ms) | 468 ms | 655 ms | 4.77 | 0.41 | 0.33 | 0.29 | 80 / 17 / 0 | 450,890 / 5,578,213 | 100,000 / 99,998 |
| `sphere` D2 10^6 | 2.23 s (2.19 s–2.33 s) | 1.71 s (1.69 s–1.80 s) | 429 ms (414 ms–431 ms) | 21.2 s | 24.0 s | 5.21 | 0.11 | 0.08 | 0.09 | 77 / 21 / 0 | 6,096,936 / 140,732,829 | 1,000,000 / 1,000,025 |
| `cube` D3 10^4 | 117 ms (116 ms–138 ms) | 93.0 ms (91.5 ms–114 ms) | 33.1 ms (31.5 ms–39.8 ms) | 127 ms | 185 ms | 3.54 | 0.92 | 0.73 | 0.63 | 79 / 20 / 0 | 255,564 / 881,892 | 10,000 / 66,373 |
| `cube` D3 10^5 | 1.38 s (1.35 s–1.51 s) | 1.03 s (1.01 s–1.11 s) | 370 ms (354 ms–404 ms) | 1.80 s | 2.77 s | 3.72 | 0.76 | 0.57 | 0.50 | 75 / 25 / 0 | 2,602,230 / 11,306,061 | 100,000 / 671,608 |
| `cube` D3 10^6 | 16.9 s (16.5 s–17.1 s) | 11.6 s (11.3 s–11.6 s) | 3.86 s (3.71 s–3.99 s) | 30.0 s | 45.2 s | 4.38 | 0.56 | 0.39 | 0.37 | 68 / 31 / 0 | 26,219,307 / 125,621,001 | 1,000,000 / 6,747,791 |
| `sphere` D3 10^4 | 310 ms (304 ms–350 ms) | 279 ms (273 ms–319 ms) | 143 ms (141 ms–158 ms) | 164 ms | 215 ms | 2.17 | 1.89 | 1.70 | 1.44 | 90 / 10 / 0 | 191,857 / 1,121,621 | 10,000 / 30,038 |
| `sphere` D3 10^5 | 3.04 s (2.92 s–3.06 s) | 2.69 s (2.60 s–2.70 s) | 1.04 s (1.04 s–1.09 s) | 3.14 s | 4.11 s | 2.93 | 0.97 | 0.86 | 0.74 | 88 / 11 / 0 | 1,974,009 / 15,785,569 | 100,000 / 302,013 |
| `sphere` D3 10^6 | 25.3 s (25.1 s–25.6 s) | 22.0 s (21.7 s–22.3 s) | 7.36 s (7.32 s–7.37 s) | 35.3 s | 46.0 s | 3.44 | 0.72 | 0.62 | 0.55 | 87 / 13 / 0 | 19,314,633 / 137,640,502 | 1,000,000 / 3,017,144 |
| `cube` D4 10^4 | 1.23 s (1.15 s–1.35 s) | 986 ms (925 ms–1.10 s) | 759 ms (733 ms–815 ms) | 1.06 s | 1.42 s | 1.62 | 1.16 | 0.93 | 0.87 | 80 / 19 / 0 | 1,397,526 / 3,183,815 | 10,000 / 295,350 |
| `sphere` D4 10^4 | 4.50 s (4.49 s–4.58 s) | 4.00 s (3.96 s–4.07 s) | 2.93 s (2.85 s–3.02 s) | 1.73 s | 1.98 s | 1.53 | 2.60 | 2.31 | 2.27 | 89 / 11 / 0 | 1,078,477 / 4,371,357 | 10,000 / 128,710 |
| `cube` D5 10^4 | 11.2 s (11.1 s–11.7 s) | 8.89 s (8.81 s–9.40 s) | 9.03 s (8.67 s–9.07 s) | 12.4 s | 15.9 s | 1.25 | 0.91 | 0.72 | 0.71 | 79 / 20 / 0 | 8,593,628 / 15,161,004 | 10,000 / 1,551,630 |
| `sphere` D5 10^4 | 54.7 s (54.5 s–54.8 s) | 48.3 s (48.1 s–48.5 s) | 39.0 s (38.9 s–39.2 s) | 24.5 s | 27.3 s | 1.40 | 2.23 | 1.97 | 2.01 | 88 / 12 / 0 | 6,939,777 / 19,199,869 | 10,000 / 674,290 |

### Reading

- **Hull `cube`**: `build()` is 0.41 to 1.40 times Qhull's compute time.
  - It is at or below 1.0 on D2 10^5, D3 10^4, D5, and D6, with D6 10^5 at 0.41.
  - It is 1.02 to 1.40 on the rest, highest on D4 10^5, where construction alone is 1.23 times Qhull.
  - Against CGAL, `build()` is 0.08 to 0.62 at D >= 3 and 1.14 to 1.39 at D2.
- **Hull `sphere` D3 and D4**: still behind.
  - `build()` is 1.41 to 2.02 times Qhull's compute time, and construction alone 0.93 to 1.42.
  - Construction is about 66 to 71% of `build()`. The pass after it (11 to 16%) and publication (15 to 17%) are the rest.
  - Against CGAL: 0.97 to 1.85.
- **Hull `sphere` D5 and D6**: `build()` is 0.86 to 1.25 times Qhull's compute time and 0.91 to 1.23 times CGAL. Construction alone is 0.59 to 0.84 times Qhull.
- **Hull `sphere` D2**: `build()` is 4.4 to 6.4 times CGAL's `convex_hull_2`, which returns only the hull points.
  - Publication is 54 to 57% of `build()`, the pass after construction 13 to 18%, and construction 24 to 25%.
  - Construction alone is 1.1 to 1.6 times CGAL.
- **Delaunay against Qhull**: `build()` is 0.56 to 0.92 times Qhull's compute time on `cube` D2 and D3, and 0.91 to 1.16 on `cube` D4 and D5. The `sphere` cells are not the same output.
- **Delaunay against CGAL**: `build()` is 4.3 to 5.2 times CGAL at D = 2, 2.2 to 4.4 at D = 3, and 1.25 to 1.62 at D = 4 and 5.
  - Construction is 68 to 90% of `build()`, and the pass after it 10 to 31%.
  - Publication is under 1%.
- **Compared with the section above**: read ratios within one section. This run's CGAL was faster on the largest sets (hull `sphere` D3 10^6: 11.0 s here, 18.4 s there).
