English | [日本語](comparison.ja.md)

# convx compared with Qhull, CGAL, and Rust crates

A report, not a test. Every row that was run is here, the ones where another library is faster or smaller among them. Nothing below is met or not met. The crate's own judgement against CGAL and Qhull is the parity run of [verification.md](verification.md): a fixed set of rows with a threshold, taken on every change that could move it. This report is taken once a release, on more data and more libraries, and judges nothing.

Taken on 2026-10-18, convx at `536c11c`, whose library code is the one 0.1.1 publishes. Every number is in [bench/summary.json](bench/summary.json).

## How it was run

| | |
| --- | --- |
| Machine | Intel Core i5-13400F (6 performance and 4 efficient cores, 16 threads), 48 GB |
| System | Windows 11, WSL2, Ubuntu 22.04.2, kernel 5.15.133.1 |
| Pinned | Every process to one core (core 2) |
| Load | The processor's total load, logged each minute through the run: median 23%, at or below 40% in 90% of the minutes. CGAL and Qhull, unchanged since the comparison of 0.1.0 (2026-10-14), read 6% to 10% slower than there: the times are those of a slower state of the machine, and the ratios within a row, whose libraries take turns, are what compares |
| Rust | 1.99.0, release profile |
| C++ | g++ 11.4.0, `-O3 -DNDEBUG`; GMP 6.2.1 |

| Library | Version | Arithmetic |
| --- | --- | --- |
| convx | `536c11c` | Exact signs of every predicate, from `f64` input |
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

- **Hulls.** Another library is the fastest on 16 of the 42 hull rows. Qhull is faster on `cube` D3 at 10^6 (0.95), `sphere` D3 (0.95, 0.99, and 0.97), `sphere` D5 (0.97), the lattices of D = 3 and 4 (0.91 and 0.43), Plummer in D3 (0.81 and 0.64), and the Dragon and Buddha scans (0.91 and 0.87). CGAL is faster on the lattice of D = 2 (0.39) and on Kuzmin points at 10^5 and 10^6 (0.51 and 0.42), and slower at 10^7 (6.92). `geo` is faster on `cube` D2 at 10^6 (0.86) and on Kuzmin points (0.63 and 0.60). `parry3d-f64` is faster on `cube` D3 at 10^5 (0.87) and on Plummer (0.53 and 0.85).
- **Delaunay.** Another library is the fastest on 21 of the 38 Delaunay rows. CGAL is faster on 19, all in D ≥ 3 but the lattice of D = 2 (0.71): by 0.84 to 0.98 on `cube`, `near`, uniform in a ball, and Plummer in D3, by 0.93 to 0.95 on `cube` D4 and D5, by 0.99 on the lattice of D = 3, and by 0.82 to 0.87 on the four scans. Qhull is faster on the file of 10^3 points in D4 (0.91). `spade` is faster on 10^5 Kuzmin sites (0.92) and `delaunator` on `cube` D2 at 10^5 (0.79); on the other rows of D = 2 they are on, convx is faster.
- **Memory.** Qhull's hulls take the least memory on 36 of the 42 hull rows. `spade` and `delaunator` take 94% to 100% of convx's memory for a triangulation of D = 2. Qhull's triangulations of 10^6 and 10^7 Kuzmin sites take 57% and 33% of convx's, and leave most sites out.

### Where convx is faster

- **Hulls.** convx is the fastest library on 26 of the 42 hull rows. Its largest margins are on CGAL in D ≥ 4 (2.5 to 21 times on `cube`), on Qhull for points on a sphere in D2 (9.5 to 20 times), and on `parry3d-f64` for points on a sphere in D3 (2.9 and 33 times).
- **Delaunay.** convx is faster than CGAL on every row of D = 2 but the lattice (1.18 to 1.80), on points on a sphere in D3 and D4 (1.11 to 2.82), on the lattice of D = 4 (1.59), and on `cube` D5 at 10^3 (1.07). It is faster than `spade` and `delaunator` on three of their four rows each (1.06 to 1.14 and 1.04 to 1.48), and faster than Qhull on every Delaunay row but the file of 10^3 points in D4.
- **Memory.** convx's triangulations take 40% to 63% of CGAL's peak, but on the lattices (84% to 87%), and 25% to 52% of Qhull's in D3 on files of 10^4 points or more, but on the lattice (87%).

### Time and peak memory

#### Convex hull, rbox: time

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^4 | 0.23 ms | 0.72 ms (3.13) | 0.60 ms (2.62) |  |  |
| `cube` | 2 | 10^5 | 2.33 ms | 8.56 ms (3.68) | 5.93 ms (2.55) | 2.37 ms (1.02) |  |
| `cube` | 2 | 10^6 | 31.0 ms | 59.3 ms (1.91) | 54.5 ms (1.76) | 26.7 ms (0.86) |  |
| `cube` | 3 | 10^4 | 1.31 ms | 2.77 ms (2.11) | 1.45 ms (1.11) |  |  |
| `cube` | 3 | 10^5 | 9.09 ms | 29.2 ms (3.21) | 11.3 ms (1.25) |  | 7.89 ms (0.87) |
| `cube` | 3 | 10^6 | 122 ms | 609 ms (5.00) | 116 ms (0.95) |  | 123 ms (1.01) |
| `cube` | 4 | 10^4 | 4.96 ms | 43.2 ms (8.72) | 6.51 ms (1.31) |  |  |
| `cube` | 4 | 10^5 | 27.3 ms | 572 ms (20.98) | 31.9 ms (1.17) |  |  |
| `cube` | 5 | 10^4 | 51.0 ms | 221 ms (4.33) | 86.0 ms (1.68) |  |  |
| `cube` | 5 | 10^5 | 209 ms | 1.69 s (8.06) | 385 ms (1.84) |  |  |
| `cube` | 6 | 10^4 | 600 ms | 2.28 s (3.80) | 1.95 s (3.25) |  |  |
| `cube` | 7 | 10^3 | 953 ms | 3.35 s (3.52) | 2.73 s (2.86) |  |  |
| `cube` | 8 | 10^3 | 9.23 s | 22.75 s (2.47) | 35.06 s (3.80) |  |  |
| `sphere` | 2 | 10^4 | 0.77 ms | 0.82 ms (1.06) | 7.30 ms (9.49) |  |  |
| `sphere` | 2 | 10^5 | 9.49 ms | 10.7 ms (1.13) | 117 ms (12.31) | 23.0 ms (2.42) |  |
| `sphere` | 2 | 10^6 | 105 ms | 167 ms (1.59) | 2.06 s (19.67) | 278 ms (2.66) |  |
| `sphere` | 3 | 10^4 | 20.9 ms | 24.5 ms (1.17) | 19.8 ms (0.95) |  |  |
| `sphere` | 3 | 10^5 | 262 ms | 417 ms (1.59) | 259 ms (0.99) |  | 747 ms (2.85) |
| `sphere` | 3 | 10^6 | 3.52 s | 8.94 s (2.54) | 3.42 s (0.97) |  | 116.69 s (33.17) |
| `sphere` | 4 | 10^4 | 98.4 ms | 132 ms (1.34) | 107 ms (1.09) |  |  |
| `sphere` | 4 | 10^5 | 1.27 s | 1.37 s (1.08) | 1.39 s (1.09) |  |  |
| `sphere` | 5 | 10^4 | 902 ms | 1.07 s (1.19) | 878 ms (0.97) |  |  |
| `sphere` | 6 | 10^4 | 7.37 s | 12.07 s (1.64) | 9.50 s (1.29) |  |  |
| `lattice` | 2 | 10^4 | 0.28 ms | 0.11 ms (0.39) | 0.47 ms (1.68) |  |  |
| `lattice` | 3 | 27,000 | 2.56 ms | 4.54 ms (1.77) | 2.34 ms (0.91) |  |  |
| `lattice` | 4 | 10^4 | 4.76 ms | 58.3 ms (12.24) | 2.03 ms (0.43) |  |  |
| `near` | 2 | 10^4 | 0.24 ms | 0.69 ms (2.93) | 0.58 ms (2.47) |  |  |
| `near` | 3 | 10^4 | 1.32 ms | 2.79 ms (2.11) | 1.53 ms (1.16) |  |  |
| `near` | 3 | 10^5 | 9.94 ms | 27.8 ms (2.80) | 11.8 ms (1.19) |  |  |

#### Convex hull, rbox: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^4 | 3.1 | 5.0 | 2.8 |  |  |
| `cube` | 2 | 10^5 | 7.6 | 10 | 5.3 | 7.4 |  |
| `cube` | 2 | 10^6 | 56 | 55 | 32 | 55 |  |
| `cube` | 3 | 10^4 | 3.6 | 5.6 | 2.9 |  |  |
| `cube` | 3 | 10^5 | 10 | 13 | 6.2 |  | 14 |
| `cube` | 3 | 10^6 | 102 | 96 | 42 |  | 126 |
| `cube` | 4 | 10^4 | 4.5 | 11 | 3.4 |  |  |
| `cube` | 4 | 10^5 | 13 | 55 | 7.6 |  |  |
| `cube` | 5 | 10^4 | 11 | 30 | 10.0 |  |  |
| `cube` | 5 | 10^5 | 29 | 132 | 25 |  |  |
| `cube` | 6 | 10^4 | 74 | 220 | 75 |  |  |
| `cube` | 7 | 10^3 | 102 | 226 | 104 |  |  |
| `cube` | 8 | 10^3 | 650 | 1514 | 650 |  |  |
| `sphere` | 2 | 10^4 | 3.8 | 5.2 | 5.0 |  |  |
| `sphere` | 2 | 10^5 | 13 | 13 | 29 | 8.1 |  |
| `sphere` | 2 | 10^6 | 104 | 80 | 299 | 64 |  |
| `sphere` | 3 | 10^4 | 9.7 | 12 | 7.4 |  |  |
| `sphere` | 3 | 10^5 | 72 | 79 | 53 |  | 120 |
| `sphere` | 3 | 10^6 | 714 | 734 | 505 |  | 1217 |
| `sphere` | 4 | 10^4 | 26 | 31 | 19 |  |  |
| `sphere` | 4 | 10^5 | 239 | 268 | 172 |  |  |
| `sphere` | 5 | 10^4 | 112 | 164 | 81 |  |  |
| `sphere` | 6 | 10^4 | 610 | 1087 | 450 |  |  |
| `lattice` | 2 | 10^4 | 3.0 | 4.8 | 2.7 |  |  |
| `lattice` | 3 | 27,000 | 4.7 | 6.6 | 3.7 |  |  |
| `lattice` | 4 | 10^4 | 4.1 | 13 | 2.9 |  |  |
| `near` | 2 | 10^4 | 3.1 | 5.0 | 2.8 |  |  |
| `near` | 3 | 10^4 | 3.6 | 5.5 | 2.8 |  |  |
| `near` | 3 | 10^5 | 10 | 13 | 6.5 |  |  |

#### Convex hull, PBBS: time

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 4.76 ms | 4.84 ms (1.02) | 6.72 ms (1.41) |  |  |
| `insphere` | 2 | 10^6 | 54.7 ms | 63.5 ms (1.16) | 70.4 ms (1.29) |  |  |
| `insphere` | 3 | 10^5 | 15.2 ms | 38.7 ms (2.55) | 18.9 ms (1.24) |  |  |
| `insphere` | 3 | 10^6 | 165 ms | 967 ms (5.87) | 203 ms (1.23) |  |  |
| `kuzmin` | 2 | 10^5 | 1.95 ms | 0.99 ms (0.51) | 4.26 ms (2.18) | 1.22 ms (0.63) |  |
| `kuzmin` | 2 | 10^6 | 26.8 ms | 11.3 ms (0.42) | 40.3 ms (1.50) | 16.0 ms (0.60) |  |
| `kuzmin` | 2 | 10^7 | 315 ms | 2.18 s (6.92) | 416 ms (1.32) |  |  |
| `plummer` | 3 | 10^5 | 6.81 ms | 18.5 ms (2.72) | 5.51 ms (0.81) |  | 3.58 ms (0.53) |
| `plummer` | 3 | 10^6 | 79.1 ms | 337 ms (4.26) | 50.6 ms (0.64) |  | 67.4 ms (0.85) |

#### Convex hull, PBBS: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | geo | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 7.6 | 9.5 | 5.2 |  |  |
| `insphere` | 2 | 10^6 | 55 | 53 | 32 |  |  |
| `insphere` | 3 | 10^5 | 10 | 15 | 6.4 |  |  |
| `insphere` | 3 | 10^6 | 105 | 99 | 44 |  |  |
| `kuzmin` | 2 | 10^5 | 7.4 | 8.1 | 4.7 | 7.3 |  |
| `kuzmin` | 2 | 10^6 | 55 | 43 | 25 | 54 |  |
| `kuzmin` | 2 | 10^7 | 527 | 644 | 308 |  |  |
| `plummer` | 3 | 10^5 | 10 | 13 | 6.2 |  | 14 |
| `plummer` | 3 | 10^6 | 95 | 96 | 41 |  | 129 |

#### Convex hull, scans: time

| Data | D | Points | convx | CGAL | Qhull | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 7.41 ms | 11.6 ms (1.57) | 8.35 ms (1.13) | 9.16 ms (1.24) |
| `dragon` | 3 | 437,645 | 54.9 ms | 140 ms (2.55) | 50.0 ms (0.91) | 133 ms (2.42) |
| `buddha` | 3 | 543,652 | 89.9 ms | 211 ms (2.35) | 78.2 ms (0.87) |  |
| `armadillo` | 3 | 172,974 | 23.6 ms | 65.2 ms (2.76) | 25.6 ms (1.09) |  |

#### Convex hull, scans: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | parry3d-f64 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 5.9 | 8.1 | 4.4 | 9.8 |
| `dragon` | 3 | 437,645 | 48 | 45 | 20 | 62 |
| `buddha` | 3 | 543,652 | 67 | 55 | 24 |  |
| `armadillo` | 3 | 172,974 | 20 | 21 | 8.8 |  |

#### Delaunay, rbox: time

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator | delaunay |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^3 | 0.37 ms | 0.45 ms (1.21) | 1.80 ms (4.89) |  |  | 494 ms (1338.06) |
| `cube` | 3 | 10^3 | 3.27 ms | 3.02 ms (0.92) | 7.68 ms (2.35) |  |  | 3.95 s (1205.37) |
| `cube` | 4 | 10^3 | 55.4 ms | 51.9 ms (0.94) | 50.2 ms (0.91) |  |  | 142.09 s (2567.06) |
| `cube` | 5 | 10^3 | 472 ms | 503 ms (1.07) | 479 ms (1.02) |  |  |  |
| `cube` | 2 | 10^4 | 3.59 ms | 4.89 ms (1.36) | 19.2 ms (5.36) |  |  |  |
| `cube` | 2 | 10^5 | 39.1 ms | 56.0 ms (1.43) | 418 ms (10.67) | 42.4 ms (1.08) | 31.0 ms (0.79) |  |
| `cube` | 2 | 10^6 | 465 ms | 825 ms (1.77) | 7.64 s (16.43) | 529 ms (1.14) | 574 ms (1.23) |  |
| `cube` | 3 | 10^4 | 39.6 ms | 36.2 ms (0.91) | 111 ms (2.79) |  |  |  |
| `cube` | 3 | 10^5 | 455 ms | 418 ms (0.92) | 3.06 s (6.72) |  |  |  |
| `cube` | 3 | 10^6 | 5.22 s | 4.40 s (0.84) | 34.31 s (6.57) |  |  |  |
| `cube` | 4 | 10^4 | 727 ms | 679 ms (0.93) | 907 ms (1.25) |  |  |  |
| `cube` | 5 | 10^4 | 8.06 s | 7.68 s (0.95) | 9.44 s (1.17) |  |  |  |
| `sphere` | 2 | 10^4 | 4.13 ms | 4.85 ms (1.18) | 25.8 ms (6.25) |  |  |  |
| `sphere` | 2 | 10^5 | 39.1 ms | 47.5 ms (1.21) | 472 ms (12.07) |  |  |  |
| `sphere` | 2 | 10^6 | 418 ms | 614 ms (1.47) | 6.46 s (15.45) |  |  |  |
| `sphere` | 3 | 10^4 | 71.4 ms | 122 ms (1.71) | 130 ms (1.82) |  |  |  |
| `sphere` | 3 | 10^5 | 653 ms | 873 ms (1.34) | 1.96 s (3.01) |  |  |  |
| `sphere` | 3 | 10^6 | 5.85 s | 6.47 s (1.11) | 22.06 s (3.77) |  |  |  |
| `sphere` | 4 | 10^4 | 887 ms | 2.50 s (2.82) | 1.29 s (1.45) |  |  |  |
| `lattice` | 2 | 10^4 | 7.65 ms | 5.42 ms (0.71) | 37.0 ms (4.84) |  |  |  |
| `lattice` | 3 | 27,000 | 141 ms | 140 ms (0.99) | 906 ms (6.45) |  |  |  |
| `lattice` | 4 | 10^4 | 461 ms | 734 ms (1.59) | 1.75 s (3.80) |  |  |  |
| `near` | 2 | 10^4 | 3.61 ms | 4.79 ms (1.33) | 25.3 ms (7.02) |  |  |  |
| `near` | 3 | 10^4 | 37.2 ms | 36.3 ms (0.98) | 137 ms (3.68) |  |  |  |
| `near` | 3 | 10^5 | 392 ms | 383 ms (0.98) | 2.06 s (5.25) |  |  |  |

#### Delaunay, rbox: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator | delaunay |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `cube` | 2 | 10^3 | 2.9 | 5.1 | 2.9 |  |  | 9.9 |
| `cube` | 3 | 10^3 | 3.5 | 6.2 | 4.1 |  |  | 26 |
| `cube` | 4 | 10^3 | 6.0 | 11 | 9.7 |  |  | 141 |
| `cube` | 5 | 10^3 | 21 | 37 | 39 |  |  |  |
| `cube` | 2 | 10^4 | 4.3 | 8.2 | 7.6 |  |  |  |
| `cube` | 2 | 10^5 | 19 | 43 | 62 | 18 | 18 |  |
| `cube` | 2 | 10^6 | 167 | 384 | 599 | 163 | 162 |  |
| `cube` | 3 | 10^4 | 10 | 18 | 19 |  |  |  |
| `cube` | 3 | 10^5 | 83 | 136 | 209 |  |  |  |
| `cube` | 3 | 10^6 | 769 | 1313 | 2078 |  |  |  |
| `cube` | 4 | 10^4 | 39 | 68 | 80 |  |  |  |
| `cube` | 5 | 10^4 | 225 | 391 | 449 |  |  |  |
| `sphere` | 2 | 10^4 | 4.1 | 8.1 | 8.4 |  |  |  |
| `sphere` | 2 | 10^5 | 17 | 40 | 61 |  |  |  |
| `sphere` | 2 | 10^6 | 140 | 354 | 591 |  |  |  |
| `sphere` | 3 | 10^4 | 7.2 | 14 | 21 |  |  |  |
| `sphere` | 3 | 10^5 | 50 | 101 | 185 |  |  |  |
| `sphere` | 3 | 10^6 | 459 | 961 | 1827 |  |  |  |
| `sphere` | 4 | 10^4 | 26 | 45 | 96 |  |  |  |
| `lattice` | 2 | 10^4 | 7.1 | 8.3 | 9.4 |  |  |  |
| `lattice` | 3 | 27,000 | 31 | 36 | 36 |  |  |  |
| `lattice` | 4 | 10^4 | 37 | 45 | 44 |  |  |  |
| `near` | 2 | 10^4 | 4.3 | 8.3 | 8.4 |  |  |  |
| `near` | 3 | 10^4 | 9.8 | 18 | 22 |  |  |  |
| `near` | 3 | 10^5 | 79 | 132 | 206 |  |  |  |

#### Delaunay, PBBS: time

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 39.8 ms | 54.6 ms (1.37) | 236 ms (5.94) |  |  |
| `insphere` | 2 | 10^6 | 406 ms | 704 ms (1.73) | 3.38 s (8.32) |  |  |
| `insphere` | 3 | 10^5 | 381 ms | 354 ms (0.93) | 1.26 s (3.30) |  |  |
| `insphere` | 3 | 10^6 | 4.24 s | 3.90 s (0.92) | 15.45 s (3.64) |  |  |
| `kuzmin` | 2 | 10^5 | 45.6 ms | 55.6 ms (1.22) | 703 ms (15.41) | 41.8 ms (0.92) | 47.2 ms (1.04) |
| `kuzmin` | 2 | 10^6 | 457 ms | 737 ms (1.61) | 4.59 s (10.04) | 486 ms (1.06) | 678 ms (1.48) |
| `kuzmin` | 2 | 10^7 | 5.33 s | 9.59 s (1.80) | 22.20 s (4.16) |  |  |
| `plummer` | 3 | 10^5 | 407 ms | 361 ms (0.89) | 2.06 s (5.05) |  |  |
| `plummer` | 3 | 10^6 | 4.49 s | 3.99 s (0.89) | 28.27 s (6.30) |  |  |

#### Delaunay, PBBS: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull | spade | delaunator |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `insphere` | 2 | 10^5 | 19 | 43 | 53 |  |  |
| `insphere` | 2 | 10^6 | 167 | 384 | 505 |  |  |
| `insphere` | 3 | 10^5 | 79 | 135 | 172 |  |  |
| `insphere` | 3 | 10^6 | 743 | 1312 | 1696 |  |  |
| `kuzmin` | 2 | 10^5 | 19 | 43 | 41 | 18 | 18 |
| `kuzmin` | 2 | 10^6 | 163 | 383 | 93 | 162 | 163 |
| `kuzmin` | 2 | 10^7 | 1605 | 3672 | 529 |  |  |
| `plummer` | 3 | 10^5 | 82 | 137 | 207 |  |  |
| `plummer` | 3 | 10^6 | 745 | 1316 | 2082 |  |  |

#### Delaunay, scans: time

| Data | D | Points | convx | CGAL | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 143 ms | 125 ms (0.87) | 450 ms (3.15) |
| `dragon` | 3 | 437,645 | 2.06 s | 1.77 s (0.86) | 8.99 s (4.37) |
| `buddha` | 3 | 543,652 | 2.62 s | 2.16 s (0.82) | 13.14 s (5.02) |
| `armadillo` | 3 | 172,974 | 771 ms | 656 ms (0.85) | 2.42 s (3.14) |

#### Delaunay, scans: peak memory (MB)

| Data | D | Points | convx | CGAL | Qhull |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bunny` | 3 | 35,947 | 31 | 53 | 65 |
| `dragon` | 3 | 437,645 | 362 | 613 | 792 |
| `buddha` | 3 | 543,652 | 476 | 759 | 1190 |
| `armadillo` | 3 | 172,974 | 142 | 239 | 303 |


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

- **The `delaunay` crate in D = 5.** One process of three builds of 1,000 points had not finished after 81 minutes in the comparison of 0.1.0, and the row is run without it. In D = 2 to 4 it took 0.49 s, 3.9 s, and 149 s for 1,000 points.
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
