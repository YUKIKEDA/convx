English | [日本語](comparison.ja.md)

# convx compared with Qhull, CGAL, and Rust crates

A report, not a test. Every row that was run is here, the ones where another library is faster or smaller among them. Nothing below is met or not met. The crate's own judgement against CGAL and Qhull is the parity run of [verification.md](verification.md): a fixed set of rows with a threshold, taken on every change that could move it. This report is taken once a release, on more data and more libraries, and judges nothing.

Taken on 2026-10-19, convx at `3782373`, whose library code is the one 0.1.1 publishes. Every number is in [bench/summary.json](bench/summary.json).

## How it was run

| | |
| --- | --- |
| Machine | Intel Core i5-13400F (6 performance and 4 efficient cores, 16 threads), 48 GB |
| System | Windows 11, WSL2, Ubuntu 22.04.2, kernel 5.15.133.1 |
| Pinned | Every process to one core (core 2) |
| Load | The processor's total load, logged each minute through the run: median 10%, at or below 31% in 90% of the minutes, at most 52%. CGAL and Qhull, unchanged since the comparison of 0.1.0 (2026-10-14), read within 1% of it |
| Rust | 1.99.0, release profile |
| C++ | g++ 11.4.0, `-O3 -DNDEBUG`; GMP 6.2.1 |

| Library | Version | Arithmetic |
| --- | --- | --- |
| convx | `3782373` | Exact signs of every predicate, from `f64` input |
| CGAL | 5.4 | Exact predicates on `f64` (`Epick` in D = 2 and 3, `Epick_d` above) |
| Qhull | 2020.2 | Floating point; facets within a tolerance are merged |
| `spade` | 2.15.1 | Exact predicates |
| `delaunator` | 1.1.0 | Robust orientation; equal points are not merged |
| `delaunay` | 0.8.2 | Exact predicates |
| `parry3d-f64` | 0.31.1 | Floating point |
| `geo` | 0.33.1 | Robust orientation |

The method is [benches/compare/README.md](../benches/compare/README.md), and the rows are [benches/compare/rows.txt](../benches/compare/rows.txt).

- **Time** is the construction of each library's output, timed inside its own process; reading the file is outside it. The libraries of a row take turns, round by round: 5 rounds of 3 builds when convx's first build takes under 1.5 s, otherwise 3 rounds of 1. Qhull builds once a process and reports its own "CPU seconds to compute hull (after input)". A table gives the median, and in parentheses its ratio to convx's median: above 1 the library is slower than convx, below 1 faster.
- **Peak memory** is the peak resident set of a process that reads the file and builds once, the median of three, from the kernel's `ru_maxrss` read by GNU time. It is the whole process: the points read from the file are in it for every library.
- **Memory over time** is the resident set of one such process, sampled every 2 ms.

What each library's timed section produces is not the same, and that is part of what is compared:

- **convx**: what `build()` returns. For a hull, its vertices, its facets with their vertex lists and neighbors, and every input point classified as a vertex, a point on the boundary, an interior point, or a duplicate. For a triangulation, its simplices with their neighbors.
- **CGAL**: the hull's surface, and the triangulation's cells and neighbors written to flat arrays (the same output as convx where CGAL has it).
- **Qhull**: the facets by their vertices, written to a file.
- **The Rust crates**: each one's own structure (the adapters are in `benches/compare/rust/`).

## The data

| Tier | What | Files |
| --- | --- | --- |
| rbox | Qhull's generator, seed 1: uniform in a cube, on a sphere, a rotated integer lattice, and points with companions within 1e-8 | D = 2 to 8, 10^3 to 10^6 points |
| PBBS | The distributions of the Problem Based Benchmark Suite: uniform in a ball, Kuzmin (D = 2), Plummer (D = 3), from PBBS's formulas | 10^5 to 10^7 points |
| Scans | The Stanford 3D Scanning Repository: Bunny, Dragon, Happy Buddha, Armadillo | 35,947 to 543,652 points |

The scans belong to the Stanford Computer Graphics Laboratory (http://graphics.stanford.edu/data/3Dscanrep/). They are downloaded by whoever runs the comparison and are not redistributed here.

## What each library does that the others do not

- **Dimensions.** convx builds hulls and triangulations in every dimension (D = 2 to 8 here). CGAL and Qhull do too (D = 2 to 8 here). `spade`, `delaunator`, and `geo` are D = 2 alone, `parry3d-f64` D = 3 alone, and `delaunay` D = 2 to 5.
- **Exactness.** convx, CGAL, and `spade` decide with exact signs; on the same points they report the same vertices and facets, simplices and neighbors. Qhull and `parry3d-f64` are floating point and can report fewer vertices, facets, or sites (the counts below).
- **Logical facets.** A facet of convx is every simplex of one exact hyperplane together: a lattice cube has 6 facets. CGAL reports the triangles of D = 3 (12), and on the lattice of D = 4, whose hull has 16 vertices, it reports 5,904. Qhull merges facets within a tolerance, which merges exactly coplanar ones and also nearly coplanar ones.
- **Cospherical sites.** convx and CGAL split a group of sites on one sphere into simplices; Qhull reports the group as one region (a lattice of D = 2 gives 9,801 squares from Qhull and 19,602 triangles from convx and CGAL).
- **Every input point accounted for.** convx's output says what each input point became: a vertex, a point on the boundary, an interior point, or a duplicate of which point. That is in its timed section and in no other library's.
- **Sites dropped.** Qhull's Delaunay triangulation leaves out sites it finds too close to others: of 10^5 Kuzmin sites, 57,938 are in its output.

## Results

The figures give the median time against the number of points for the kinds of data that have several sizes, and the resident set over the time of one process on three rows of 10^6 points or more.

![Convex hull: time against size](bench/hull-time.svg)

![Delaunay triangulation: time against size](bench/delaunay-time.svg)

![Resident set over time](bench/memory.svg)

In the memory figure a dot is a sample. A line without dots is a time in which no sample was read: the sampling reads `/proc`, which waits while the process maps memory.

### Where another library is faster

- **Hulls.** Another library is the fastest on 16 of the 42 hull rows. Qhull is faster on `cube` D3 at 10^6 (0.89), `sphere` D3 at 10^4 and 10^5 (0.94 and 0.99), `sphere` D5 (0.98), the lattices of D = 3 and 4 (0.98 and 0.41), Plummer in D3 (0.85 and 0.64), and the Dragon and Buddha scans (0.94 and 0.82). CGAL is faster on the lattice of D = 2 (0.40) and on Kuzmin points at 10^5 and 10^6 (0.51 and 0.39), and slower at 10^7 (6.48); uniform in a ball in D2 at 10^5, the two are level (1.00). `geo` is faster on `cube` D2 at 10^6 (0.98) and on Kuzmin points (0.62 and 0.62). `parry3d-f64` is faster on `cube` D3 (0.85 and 0.97) and on Plummer (0.45 and 0.82).
- **Delaunay.** Another library is the fastest on 21 of the 38 Delaunay rows. CGAL is faster on 18: by 0.72 on the lattice of D = 2, by 0.87 to 0.98 on `cube`, `near`, uniform in a ball, and Plummer in D = 3 to 5, and by 0.81 to 0.88 on the four scans. Qhull is faster on the files of 10^3 points in D4 and D5 (0.91 and 0.90). `spade` is faster on 10^5 Kuzmin sites (0.90), and `delaunator` on `cube` D2 at 10^5 (0.78) and on 10^5 Kuzmin sites (0.99); on the other rows of D = 2 they are on, convx is faster.
- **Memory.** Qhull's hulls take the least memory on 36 of the 42 hull rows. `spade` and `delaunator` take 94% to 100% of convx's memory for a triangulation of D = 2. Qhull's triangulations of 10^6 and 10^7 Kuzmin sites take 57% and 33% of convx's, and leave most sites out.

### Where convx is faster

- **Hulls.** convx is the fastest library on 26 of the 42 hull rows. Its largest margins are on CGAL in D ≥ 4 (2.7 to 22 times on `cube`), on Qhull for points on a sphere in D2 (9.4 to 19 times), and on `parry3d-f64` for points on a sphere in D3 (3.0 and 29 times).
- **Delaunay.** convx is faster than CGAL on every row of D = 2 but the lattice (1.15 to 1.81), on points on a sphere in D3 and D4 (1.13 to 2.84), on the lattices of D = 3 and 4 (1.07 and 1.54), and on `cube` D5 at 10^3 (1.05). It is faster than `spade` on three of its four rows (1.06 to 1.20) and than `delaunator` on two (1.15 and 1.47), and faster than Qhull on every Delaunay row but the two files of 10^3 points in D4 and D5.
- **Memory.** convx's triangulations take 40% to 62% of CGAL's peak, but on the lattices (76% to 87%), and 25% to 52% of Qhull's in D3 on files of 10^4 points or more, but on the lattice (87%).

### Time and peak memory

#### Convex hull, rbox: time

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^4 | 0.22 ms | 0.71 ms (3.17) | 0.58 ms (2.60) |  |  |
| `cube` | 2 | 10^5 | 2.29 ms | 8.59 ms (3.75) | 5.68 ms (2.48) | 2.36 ms (1.03) |  |
| `cube` | 2 | 10^6 | 25.5 ms | 56.9 ms (2.23) | 53.3 ms (2.09) | 25.1 ms (0.98) |  |
| `cube` | 3 | 10^4 | 1.30 ms | 2.62 ms (2.01) | 1.37 ms (1.05) |  |  |
| `cube` | 3 | 10^5 | 8.53 ms | 29.1 ms (3.41) | 10.8 ms (1.26) |  | 7.27 ms (0.85) |
| `cube` | 3 | 10^6 | 123 ms | 518 ms (4.21) | 110 ms (0.89) |  | 120 ms (0.97) |
| `cube` | 4 | 10^4 | 4.96 ms | 43.1 ms (8.69) | 6.31 ms (1.27) |  |  |
| `cube` | 4 | 10^5 | 25.3 ms | 549 ms (21.72) | 31.1 ms (1.23) |  |  |
| `cube` | 5 | 10^4 | 50.5 ms | 214 ms (4.25) | 84.0 ms (1.66) |  |  |
| `cube` | 5 | 10^5 | 200 ms | 1.63 s (8.12) | 371 ms (1.85) |  |  |
| `cube` | 6 | 10^4 | 569 ms | 2.18 s (3.84) | 1.78 s (3.13) |  |  |
| `cube` | 7 | 10^3 | 839 ms | 3.08 s (3.67) | 2.39 s (2.85) |  |  |
| `cube` | 8 | 10^3 | 7.94 s | 21.23 s (2.67) | 31.24 s (3.93) |  |  |
| `sphere` | 2 | 10^4 | 0.70 ms | 0.74 ms (1.06) | 6.59 ms (9.39) |  |  |
| `sphere` | 2 | 10^5 | 8.42 ms | 9.72 ms (1.15) | 104 ms (12.35) | 21.2 ms (2.51) |  |
| `sphere` | 2 | 10^6 | 95.6 ms | 152 ms (1.59) | 1.82 s (19.04) | 255 ms (2.66) |  |
| `sphere` | 3 | 10^4 | 19.5 ms | 23.5 ms (1.21) | 18.3 ms (0.94) |  |  |
| `sphere` | 3 | 10^5 | 235 ms | 373 ms (1.59) | 233 ms (0.99) |  | 697 ms (2.97) |
| `sphere` | 3 | 10^6 | 3.21 s | 7.66 s (2.39) | 3.37 s (1.05) |  | 93.87 s (29.24) |
| `sphere` | 4 | 10^4 | 86.4 ms | 120 ms (1.38) | 90.9 ms (1.05) |  |  |
| `sphere` | 4 | 10^5 | 1.11 s | 1.23 s (1.11) | 1.29 s (1.16) |  |  |
| `sphere` | 5 | 10^4 | 812 ms | 992 ms (1.22) | 794 ms (0.98) |  |  |
| `sphere` | 6 | 10^4 | 6.53 s | 9.78 s (1.50) | 7.84 s (1.20) |  |  |
| `lattice` | 2 | 10^4 | 0.26 ms | 0.10 ms (0.40) | 0.42 ms (1.65) |  |  |
| `lattice` | 3 | 27,000 | 2.04 ms | 4.01 ms (1.96) | 2.00 ms (0.98) |  |  |
| `lattice` | 4 | 10^4 | 4.25 ms | 49.4 ms (11.64) | 1.76 ms (0.41) |  |  |
| `near` | 2 | 10^4 | 0.23 ms | 0.70 ms (2.99) | 0.58 ms (2.45) |  |  |
| `near` | 3 | 10^4 | 1.33 ms | 2.74 ms (2.06) | 1.56 ms (1.17) |  |  |
| `near` | 3 | 10^5 | 8.67 ms | 25.5 ms (2.94) | 10.7 ms (1.23) |  |  |

#### Convex hull, rbox: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^4 | 3.2 | 5.0 | 2.8 |  |  |
| `cube` | 2 | 10^5 | 7.6 | 11 | 5.3 | 7.5 |  |
| `cube` | 2 | 10^6 | 56 | 56 | 31 | 56 |  |
| `cube` | 3 | 10^4 | 3.4 | 5.6 | 2.9 |  |  |
| `cube` | 3 | 10^5 | 10 | 13 | 6.2 |  | 15 |
| `cube` | 3 | 10^6 | 101 | 96 | 43 |  | 126 |
| `cube` | 4 | 10^4 | 4.5 | 11 | 3.4 |  |  |
| `cube` | 4 | 10^5 | 13 | 55 | 7.5 |  |  |
| `cube` | 5 | 10^4 | 11 | 30 | 10.0 |  |  |
| `cube` | 5 | 10^5 | 29 | 132 | 25 |  |  |
| `cube` | 6 | 10^4 | 74 | 220 | 75 |  |  |
| `cube` | 7 | 10^3 | 102 | 226 | 104 |  |  |
| `cube` | 8 | 10^3 | 649 | 1514 | 650 |  |  |
| `sphere` | 2 | 10^4 | 3.9 | 5.2 | 4.9 |  |  |
| `sphere` | 2 | 10^5 | 13 | 13 | 29 | 8.1 |  |
| `sphere` | 2 | 10^6 | 104 | 80 | 298 | 63 |  |
| `sphere` | 3 | 10^4 | 9.6 | 12 | 7.3 |  |  |
| `sphere` | 3 | 10^5 | 72 | 79 | 53 |  | 119 |
| `sphere` | 3 | 10^6 | 712 | 734 | 505 |  | 1219 |
| `sphere` | 4 | 10^4 | 26 | 31 | 19 |  |  |
| `sphere` | 4 | 10^5 | 240 | 268 | 172 |  |  |
| `sphere` | 5 | 10^4 | 111 | 164 | 81 |  |  |
| `sphere` | 6 | 10^4 | 611 | 1087 | 450 |  |  |
| `lattice` | 2 | 10^4 | 3.1 | 4.8 | 2.8 |  |  |
| `lattice` | 3 | 27,000 | 4.6 | 6.5 | 3.5 |  |  |
| `lattice` | 4 | 10^4 | 4.1 | 13 | 2.9 |  |  |
| `near` | 2 | 10^4 | 3.1 | 5.0 | 2.8 |  |  |
| `near` | 3 | 10^4 | 3.7 | 5.5 | 3.0 |  |  |
| `near` | 3 | 10^5 | 10 | 13 | 6.5 |  |  |

#### Convex hull, PBBS: time

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 4.50 ms | 4.50 ms (1.00) | 6.26 ms (1.39) |  |  |
| `insphere` | 2 | 10^6 | 54.3 ms | 59.5 ms (1.10) | 64.6 ms (1.19) |  |  |
| `insphere` | 3 | 10^5 | 13.8 ms | 37.4 ms (2.71) | 17.3 ms (1.25) |  |  |
| `insphere` | 3 | 10^6 | 158 ms | 811 ms (5.14) | 192 ms (1.21) |  |  |
| `kuzmin` | 2 | 10^5 | 1.87 ms | 0.96 ms (0.51) | 3.85 ms (2.06) | 1.17 ms (0.62) |  |
| `kuzmin` | 2 | 10^6 | 26.8 ms | 10.4 ms (0.39) | 39.6 ms (1.48) | 16.6 ms (0.62) |  |
| `kuzmin` | 2 | 10^7 | 323 ms | 2.09 s (6.48) | 387 ms (1.20) |  |  |
| `plummer` | 3 | 10^5 | 6.52 ms | 17.7 ms (2.72) | 5.53 ms (0.85) |  | 2.95 ms (0.45) |
| `plummer` | 3 | 10^6 | 75.6 ms | 307 ms (4.06) | 48.3 ms (0.64) |  | 61.7 ms (0.82) |

#### Convex hull, PBBS: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 7.5 | 9.4 | 5.3 |  |  |
| `insphere` | 2 | 10^6 | 55 | 53 | 32 |  |  |
| `insphere` | 3 | 10^5 | 10 | 14 | 6.4 |  |  |
| `insphere` | 3 | 10^6 | 105 | 98 | 43 |  |  |
| `kuzmin` | 2 | 10^5 | 7.5 | 8.1 | 4.8 | 7.3 |  |
| `kuzmin` | 2 | 10^6 | 55 | 43 | 25 | 54 |  |
| `kuzmin` | 2 | 10^7 | 526 | 644 | 308 |  |  |
| `plummer` | 3 | 10^5 | 10 | 13 | 6.0 |  | 14 |
| `plummer` | 3 | 10^6 | 96 | 96 | 42 |  | 130 |

#### Convex hull, scans: time

| Data | D | Points | convx | CGAL | Qhull | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 6.78 ms | 11.5 ms (1.69) | 8.44 ms (1.25) | 8.33 ms (1.23) |
| `dragon` | 3 | 437,645 | 50.4 ms | 123 ms (2.44) | 47.5 ms (0.94) | 127 ms (2.51) |
| `buddha` | 3 | 543,652 | 84.6 ms | 180 ms (2.13) | 69.0 ms (0.82) |  |
| `armadillo` | 3 | 172,974 | 20.9 ms | 54.3 ms (2.60) | 22.4 ms (1.07) |  |

#### Convex hull, scans: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 6.0 | 8.2 | 4.5 | 9.7 |
| `dragon` | 3 | 437,645 | 49 | 45 | 20 | 62 |
| `buddha` | 3 | 543,652 | 68 | 55 | 25 |  |
| `armadillo` | 3 | 172,974 | 20 | 21 | 8.9 |  |

#### Delaunay, rbox: time

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator | delaunay |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^3 | 0.32 ms | 0.42 ms (1.32) | 1.66 ms (5.19) |  |  | 459 ms (1433.54) |
| `cube` | 3 | 10^3 | 3.19 ms | 2.78 ms (0.87) | 7.20 ms (2.26) |  |  | 3.74 s (1171.58) |
| `cube` | 4 | 10^3 | 53.4 ms | 49.8 ms (0.93) | 48.6 ms (0.91) |  |  | 136.29 s (2550.78) |
| `cube` | 5 | 10^3 | 433 ms | 455 ms (1.05) | 389 ms (0.90) |  |  |  |
| `cube` | 2 | 10^4 | 3.67 ms | 4.60 ms (1.25) | 18.8 ms (5.11) |  |  |  |
| `cube` | 2 | 10^5 | 36.9 ms | 50.9 ms (1.38) | 368 ms (9.99) | 40.5 ms (1.10) | 28.8 ms (0.78) |  |
| `cube` | 2 | 10^6 | 397 ms | 698 ms (1.76) | 5.21 s (13.10) | 476 ms (1.20) | 456 ms (1.15) |  |
| `cube` | 3 | 10^4 | 36.4 ms | 31.5 ms (0.87) | 90.2 ms (2.48) |  |  |  |
| `cube` | 3 | 10^5 | 375 ms | 342 ms (0.91) | 2.00 s (5.35) |  |  |  |
| `cube` | 3 | 10^6 | 4.21 s | 3.77 s (0.90) | 25.01 s (5.94) |  |  |  |
| `cube` | 4 | 10^4 | 680 ms | 650 ms (0.95) | 765 ms (1.12) |  |  |  |
| `cube` | 5 | 10^4 | 7.22 s | 7.10 s (0.98) | 7.69 s (1.07) |  |  |  |
| `sphere` | 2 | 10^4 | 3.80 ms | 4.37 ms (1.15) | 22.8 ms (6.00) |  |  |  |
| `sphere` | 2 | 10^5 | 34.6 ms | 39.6 ms (1.15) | 318 ms (9.20) |  |  |  |
| `sphere` | 2 | 10^6 | 367 ms | 482 ms (1.31) | 4.64 s (12.64) |  |  |  |
| `sphere` | 3 | 10^4 | 67.9 ms | 117 ms (1.72) | 106 ms (1.56) |  |  |  |
| `sphere` | 3 | 10^5 | 624 ms | 833 ms (1.33) | 1.56 s (2.50) |  |  |  |
| `sphere` | 3 | 10^6 | 5.31 s | 5.99 s (1.13) | 18.76 s (3.53) |  |  |  |
| `sphere` | 4 | 10^4 | 863 ms | 2.46 s (2.84) | 1.23 s (1.42) |  |  |  |
| `lattice` | 2 | 10^4 | 7.33 ms | 5.31 ms (0.72) | 35.6 ms (4.86) |  |  |  |
| `lattice` | 3 | 27,000 | 122 ms | 131 ms (1.07) | 865 ms (7.06) |  |  |  |
| `lattice` | 4 | 10^4 | 453 ms | 698 ms (1.54) | 1.63 s (3.59) |  |  |  |
| `near` | 2 | 10^4 | 3.83 ms | 4.84 ms (1.26) | 25.6 ms (6.69) |  |  |  |
| `near` | 3 | 10^4 | 37.0 ms | 34.5 ms (0.93) | 129 ms (3.48) |  |  |  |
| `near` | 3 | 10^5 | 382 ms | 375 ms (0.98) | 1.93 s (5.06) |  |  |  |

#### Delaunay, rbox: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator | delaunay |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^3 | 2.9 | 5.1 | 2.9 |  |  | 9.7 |
| `cube` | 3 | 10^3 | 3.5 | 6.1 | 4.1 |  |  | 26 |
| `cube` | 4 | 10^3 | 6.0 | 11 | 9.6 |  |  | 143 |
| `cube` | 5 | 10^3 | 21 | 37 | 39 |  |  |  |
| `cube` | 2 | 10^4 | 4.3 | 8.2 | 7.6 |  |  |  |
| `cube` | 2 | 10^5 | 19 | 43 | 62 | 18 | 18 |  |
| `cube` | 2 | 10^6 | 167 | 384 | 599 | 162 | 162 |  |
| `cube` | 3 | 10^4 | 10 | 18 | 19 |  |  |  |
| `cube` | 3 | 10^5 | 80 | 136 | 209 |  |  |  |
| `cube` | 3 | 10^6 | 769 | 1313 | 2078 |  |  |  |
| `cube` | 4 | 10^4 | 39 | 68 | 80 |  |  |  |
| `cube` | 5 | 10^4 | 225 | 391 | 449 |  |  |  |
| `sphere` | 2 | 10^4 | 4.0 | 8.0 | 8.4 |  |  |  |
| `sphere` | 2 | 10^5 | 17 | 40 | 61 |  |  |  |
| `sphere` | 2 | 10^6 | 140 | 353 | 591 |  |  |  |
| `sphere` | 3 | 10^4 | 7.3 | 14 | 21 |  |  |  |
| `sphere` | 3 | 10^5 | 49 | 101 | 185 |  |  |  |
| `sphere` | 3 | 10^6 | 459 | 960 | 1827 |  |  |  |
| `sphere` | 4 | 10^4 | 23 | 45 | 96 |  |  |  |
| `lattice` | 2 | 10^4 | 7.0 | 8.3 | 9.5 |  |  |  |
| `lattice` | 3 | 27,000 | 31 | 36 | 36 |  |  |  |
| `lattice` | 4 | 10^4 | 34 | 45 | 44 |  |  |  |
| `near` | 2 | 10^4 | 4.3 | 8.3 | 8.4 |  |  |  |
| `near` | 3 | 10^4 | 9.8 | 18 | 22 |  |  |  |
| `near` | 3 | 10^5 | 77 | 133 | 206 |  |  |  |

#### Delaunay, PBBS: time

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 37.1 ms | 52.6 ms (1.42) | 224 ms (6.04) |  |  |
| `insphere` | 2 | 10^6 | 396 ms | 682 ms (1.72) | 3.26 s (8.23) |  |  |
| `insphere` | 3 | 10^5 | 389 ms | 352 ms (0.91) | 1.23 s (3.18) |  |  |
| `insphere` | 3 | 10^6 | 4.02 s | 3.79 s (0.94) | 14.97 s (3.72) |  |  |
| `kuzmin` | 2 | 10^5 | 44.8 ms | 53.9 ms (1.21) | 599 ms (13.39) | 40.3 ms (0.90) | 44.2 ms (0.99) |
| `kuzmin` | 2 | 10^6 | 438 ms | 709 ms (1.62) | 4.30 s (9.83) | 464 ms (1.06) | 644 ms (1.47) |
| `kuzmin` | 2 | 10^7 | 5.03 s | 9.12 s (1.81) | 20.80 s (4.14) |  |  |
| `plummer` | 3 | 10^5 | 385 ms | 350 ms (0.91) | 1.96 s (5.08) |  |  |
| `plummer` | 3 | 10^6 | 4.12 s | 3.83 s (0.93) | 26.69 s (6.48) |  |  |

#### Delaunay, PBBS: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 19 | 43 | 53 |  |  |
| `insphere` | 2 | 10^6 | 167 | 384 | 505 |  |  |
| `insphere` | 3 | 10^5 | 81 | 136 | 172 |  |  |
| `insphere` | 3 | 10^6 | 743 | 1312 | 1696 |  |  |
| `kuzmin` | 2 | 10^5 | 19 | 43 | 41 | 18 | 18 |
| `kuzmin` | 2 | 10^6 | 164 | 383 | 93 | 163 | 162 |
| `kuzmin` | 2 | 10^7 | 1606 | 3669 | 528 |  |  |
| `plummer` | 3 | 10^5 | 81 | 137 | 207 |  |  |
| `plummer` | 3 | 10^6 | 745 | 1315 | 2082 |  |  |

#### Delaunay, scans: time

| Data | D | Points | convx | CGAL | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 138 ms | 121 ms (0.88) | 409 ms (2.96) |
| `dragon` | 3 | 437,645 | 1.97 s | 1.70 s (0.86) | 8.18 s (4.14) |
| `buddha` | 3 | 543,652 | 2.58 s | 2.09 s (0.81) | 12.32 s (4.77) |
| `armadillo` | 3 | 172,974 | 722 ms | 630 ms (0.87) | 2.26 s (3.14) |

#### Delaunay, scans: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 31 | 53 | 65 |
| `dragon` | 3 | 437,645 | 363 | 612 | 792 |
| `buddha` | 3 | 543,652 | 474 | 759 | 1189 |
| `armadillo` | 3 | 172,974 | 143 | 239 | 303 |


### Where the outputs' counts differ

A hull's counts are its vertices and its facets (CGAL: triangles in D = 3, simplices above; `parry3d-f64`: triangles). A triangulation's are its distinct sites and its simplices (Qhull: regions). Rows not listed agree.

| Operation | Data | D | Points | Counts |
| --- | --- | ---: | ---: | --- |
| hull | `cube` | 2 | 100,000 | convx 26 / 26; CGAL 26 / 26; Qhull 25 / 25; geo 26 / 26 |
| hull | `cube` | 2 | 1,000,000 | convx 32 / 32; CGAL 32 / 32; Qhull 24 / 24; geo 32 / 32 |
| hull | `cube` | 3 | 100,000 | convx 183 / 362; CGAL 183 / 362; Qhull 178 / 345; parry3d-f64 180 / 356 |
| hull | `cube` | 3 | 1,000,000 | convx 304 / 604; CGAL 304 / 604; Qhull 279 / 520; parry3d-f64 283 / 562 |
| hull | `cube` | 4 | 100,000 | convx 742 / 4,193; CGAL 742 / 4,193; Qhull 741 / 4,162 |
| hull | `cube` | 5 | 100,000 | convx 2,274 / 48,608; CGAL 2,274 / 48,608; Qhull 2,274 / 48,495 |
| hull | `cube` | 6 | 10,000 | convx 1,870 / 170,250; CGAL 1,870 / 170,250; Qhull 1,870 / 170,190 |
| hull | `cube` | 7 | 1,000 | convx 623 / 221,260; CGAL 623 / 221,260; Qhull 623 / 221,247 |
| hull | `cube` | 8 | 1,000 | convx 767 / 1,309,711; CGAL 767 / 1,309,711; Qhull 767 / 1,309,651 |
| hull | `sphere` | 2 | 100,000 | convx 99,999 / 99,999; CGAL 99,999 / 99,999; Qhull 99,997 / 99,997; geo 99,999 / 99,999 |
| hull | `sphere` | 2 | 1,000,000 | convx 999,872 / 999,872; CGAL 999,872 / 999,872; Qhull 998,720 / 998,720; geo 999,872 / 999,872 |
| hull | `lattice` | 3 | 27,000 | convx 8 / 6; CGAL 8 / 12; Qhull 8 / 6 |
| hull | `lattice` | 4 | 10,000 | convx 16 / 8; CGAL 5,904 / 25,559; Qhull 16 / 8 |
| hull | `near` | 3 | 100,000 | convx 176 / 348; CGAL 176 / 348; Qhull 173 / 339 |
| hull | `buddha` | 3 | 543,652 | convx 6,181 / 12,357; CGAL 6,181 / 12,358; Qhull 6,181 / 12,357 |
| delaunay | `cube` | 2 | 100,000 | convx 100,000 / 199,972; CGAL 100,000 / 199,972; Qhull 100,000 / 199,963; spade 100,000 / 199,972; delaunator 100,000 / 199,972 |
| delaunay | `cube` | 2 | 1,000,000 | convx 1,000,000 / 1,999,966; CGAL 1,000,000 / 1,999,966; Qhull 1,000,000 / 1,999,861; spade 1,000,000 / 1,999,966; delaunator 1,000,000 / 1,999,966 |
| delaunay | `cube` | 3 | 100,000 | convx 100,000 / 671,796; CGAL 100,000 / 671,796; Qhull 100,000 / 671,787 |
| delaunay | `cube` | 3 | 1,000,000 | convx 1,000,000 / 6,748,017; CGAL 1,000,000 / 6,748,017; Qhull 1,000,000 / 6,747,463 |
| delaunay | `sphere` | 2 | 10,000 | convx 10,000 / 9,998; CGAL 10,000 / 9,998; Qhull 10,000 / 9,985 |
| delaunay | `sphere` | 2 | 100,000 | convx 100,000 / 99,999; CGAL 100,000 / 99,999; Qhull 99,999 / 99,996 |
| delaunay | `sphere` | 2 | 1,000,000 | convx 1,000,000 / 1,000,126; CGAL 1,000,000 / 1,000,126; Qhull 999,782 / 999,556 |
| delaunay | `sphere` | 3 | 10,000 | convx 10,000 / 30,054; CGAL 10,000 / 30,054; Qhull 10,000 / 29,493 |
| delaunay | `sphere` | 3 | 100,000 | convx 100,000 / 301,399; CGAL 100,000 / 301,399; Qhull 100,000 / 300,338 |
| delaunay | `sphere` | 3 | 1,000,000 | convx 1,000,000 / 3,014,104; CGAL 1,000,000 / 3,014,104; Qhull 1,000,000 / 3,004,242 |
| delaunay | `sphere` | 4 | 10,000 | convx 10,000 / 128,732; CGAL 10,000 / 128,732; Qhull 10,000 / 128,524 |
| delaunay | `lattice` | 2 | 10,000 | convx 10,000 / 19,602; CGAL 10,000 / 19,602; Qhull 10,000 / 9,801 |
| delaunay | `lattice` | 3 | 27,000 | convx 27,000 / 146,334; CGAL 27,000 / 146,334; Qhull 27,000 / 24,389 |
| delaunay | `lattice` | 4 | 10,000 | convx 10,000 / 157,464; CGAL 10,000 / 157,464; Qhull 10,000 / 6,561 |
| delaunay | `near` | 2 | 10,000 | convx 10,000 / 19,977; CGAL 10,000 / 19,977; Qhull 9,525 / 18,502 |
| delaunay | `near` | 3 | 10,000 | convx 10,000 / 64,177; CGAL 10,000 / 64,177; Qhull 9,914 / 63,208 |
| delaunay | `near` | 3 | 100,000 | convx 100,000 / 650,949; CGAL 100,000 / 650,949; Qhull 99,194 / 641,412 |
| delaunay | `kuzmin` | 2 | 100,000 | convx 100,000 / 199,994; CGAL 100,000 / 199,994; Qhull 57,938 / 83,898; spade 100,000 / 199,994; delaunator 100,000 / 199,994 |
| delaunay | `kuzmin` | 2 | 1,000,000 | convx 1,000,000 / 1,999,993; CGAL 1,000,000 / 1,999,993; Qhull 81,805 / 115,418; spade 1,000,000 / 1,999,993; delaunator 1,000,000 / 1,999,993 |
| delaunay | `kuzmin` | 2 | 10,000,000 | convx 10,000,000 / 19,999,993; CGAL 10,000,000 / 19,999,993; Qhull 39,772 / 56,200 |
| delaunay | `plummer` | 3 | 100,000 | convx 100,000 / 675,750; CGAL 100,000 / 675,750; Qhull 100,000 / 675,747 |
| delaunay | `plummer` | 3 | 1,000,000 | convx 1,000,000 / 6,765,810; CGAL 1,000,000 / 6,765,810; Qhull 1,000,000 / 6,764,920 |
| delaunay | `buddha` | 3 | 543,652 | convx 543,524 / 3,867,976; CGAL 543,524 / 3,867,976; Qhull 543,523 / 3,867,954 |

## Not compared

- **The `delaunay` crate in D = 5.** One process of three builds of 1,000 points had not finished after 81 minutes in the comparison of 0.1.0, and the row is run without it. In D = 2 to 4 it took 0.46 s, 3.7 s, and 136 s for 1,000 points.
- **Crates not compared**, chosen as the main ones and not as a survey: `qhull` (bindings to Qhull, which is here itself), `chull`, `tritet`, `startin`, `voronoice`, `voronator`.
- **Voronoi diagrams.** No other library here builds the same output; convx's diagram is its Delaunay triangulation and a linear solve per vertex.
- **Threads.** Every library here was run on one core. CGAL has parallel triangulations; convx builds sequentially.

## Taking it again

```sh
bash benches/compare/setup.sh <commit>
for tier in rbox pbbs scans; do bash benches/compare/data/make.sh $tier; done
cd ~/convx-compare
python3 <repo>/benches/compare/runner.py <repo>/benches/compare/rows.txt results.jsonl
python3 <repo>/benches/compare/publish.py results.jsonl <repo>/docs/bench <meta.json>
```

`publish.py` writes `summary.json` and the figures. The data, about 950 MB, is removed after a run.
