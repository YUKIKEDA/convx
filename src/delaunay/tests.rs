//! Expectations come from each case's statement or from an independent
//! brute-force reference in `i128`, never from the library's own output.

use super::*;

/// Whether the lift of `points` is flat (every site on one sphere).
fn is_flat(dim: usize, points: &[f64]) -> bool {
    let input = accept(dim, points).unwrap();
    let sites = insert::Sites::of(&input);
    flat(&input, &sites).unwrap()
}

/// The triangulation built by insertion; panics on a flat lift.
fn triangulate(dim: usize, points: &[f64]) -> DelaunayTriangulation {
    assert!(!is_flat(dim, points), "unexpected flat lift");
    DelaunayBuilder::new(dim, points).build().unwrap()
}

/// Ascending vertex lists of the simplices, sorted.
fn cells(t: &DelaunayTriangulation) -> Vec<Vec<u32>> {
    let mut cells: Vec<Vec<u32>> = t
        .simplices
        .iter()
        .map(|s| {
            let mut v = s.vertices.clone();
            v.sort_unstable();
            v
        })
        .collect();
    cells.sort();
    cells
}

/// Structural checks from §7: order, orientation, and neighbor symmetry
/// across the shared face.
fn check(t: &DelaunayTriangulation, points: &[f64]) {
    let d = t.dim;
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let sorted: Vec<Vec<u32>> = t
        .simplices
        .iter()
        .map(|s| {
            let mut v = s.vertices.clone();
            v.sort_unstable();
            v
        })
        .collect();
    assert!(
        sorted.windows(2).all(|w| w[0] < w[1]),
        "lexicographic order"
    );
    for (i, s) in t.simplices.iter().enumerate() {
        assert_eq!(s.vertices.len(), d + 1);
        let refs: Vec<&[f64]> = s.vertices.iter().map(|&v| point(v)).collect();
        assert_eq!(orient(&refs).unwrap(), Sign::Positive, "simplex {i}");
        for (slot, &n) in s.neighbors.iter().enumerate() {
            if n == NO_NEIGHBOR {
                continue;
            }
            let other = &t.simplices[n as usize];
            let back = other.neighbors.iter().position(|&b| b == i as u32).unwrap();
            let face = |v: &[u32], skip: usize| -> Vec<u32> {
                let mut f: Vec<u32> = v
                    .iter()
                    .enumerate()
                    .filter(|&(k, _)| k != skip)
                    .map(|(_, &x)| x)
                    .collect();
                f.sort_unstable();
                f
            };
            assert_eq!(face(&s.vertices, slot), face(&other.vertices, back));
        }
    }
}

#[test]
fn a_triangle_with_an_inner_site() {
    // (0,0), (1,0), (0,1), (0.1,0.1): the inner site joins every edge of
    // the triangle.
    let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.1, 0.1];
    let t = triangulate(2, &points);
    assert_eq!(cells(&t), vec![vec![0, 1, 3], vec![0, 2, 3], vec![1, 2, 3]]);
    check(&t, &points);
}

#[test]
fn partly_cocircular_square() {
    // The square's corners are cocircular, and the circle is empty, so the
    // square is one lower facet of four sites, split by placing in index
    // order: 0, 1, 2, then 3 beyond the ridge {0, 2}. Point 4 = (1, -3) lies
    // outside that circle and joins edge {0, 1}.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0];
    let t = triangulate(2, &points);
    let m = NO_NEIGHBOR;
    assert_eq!(
        t.simplices,
        vec![
            DelaunaySimplex {
                vertices: vec![0, 1, 2],
                neighbors: vec![m, 2, 1],
            },
            // (0,0), (2,0), (1,-3) is clockwise, so the last two swap.
            DelaunaySimplex {
                vertices: vec![0, 4, 1],
                neighbors: vec![m, 0, m],
            },
            DelaunaySimplex {
                vertices: vec![0, 2, 3],
                neighbors: vec![m, m, 0],
            },
        ]
    );
    check(&t, &points);
}

#[test]
fn every_site_on_one_circle_is_flat() {
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
    assert!(
        is_flat(2, &points),
        "four cocircular sites have a flat lift"
    );
    // One site off the circle makes the lift span three dimensions.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, 1.0];
    assert!(!is_flat(2, &points));
}

#[test]
fn degenerate_sites_report_original_indices() {
    // 1 duplicates 0; all on the line y = x.
    let points = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0];
    assert_eq!(
        complex(2, &points, Execution::Sequential).err(),
        Some(ConvexHullError::DegenerateDimension {
            actual_dim: 1,
            spanning_points: vec![0, 2],
        })
    );
}

#[test]
fn duplicates_map_to_their_representative() {
    let points = [0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 4.0, 0.0, 1.0, 1.0];
    let t = triangulate(2, &points);
    assert_eq!(t.representative, vec![0, 1, 2, 1, 4]);
    assert!(t.simplices.iter().all(|s| !s.vertices.contains(&3)));
}

// Brute-force reference: a (D+1)-subset of sites in general position is a
// Delaunay simplex exactly when its circumsphere has no site strictly
// inside, decided by the lifted determinant over i128.

fn det(m: &[Vec<i128>]) -> i128 {
    if m.len() == 1 {
        return m[0][0];
    }
    (0..m.len())
        .map(|j| {
            let minor: Vec<Vec<i128>> = m[1..]
                .iter()
                .map(|r| {
                    r.iter()
                        .enumerate()
                        .filter(|&(c, _)| c != j)
                        .map(|(_, &x)| x)
                        .collect()
                })
                .collect();
            let t = m[0][j] * det(&minor);
            if j.is_multiple_of(2) {
                t
            } else {
                -t
            }
        })
        .sum()
}

fn plain(points: &[Vec<i64>], s: &[usize]) -> i128 {
    let o = &points[s[0]];
    let rows: Vec<Vec<i128>> = s[1..]
        .iter()
        .map(|&i| {
            points[i]
                .iter()
                .zip(o)
                .map(|(&x, &y)| i128::from(x - y))
                .collect()
        })
        .collect();
    det(&rows)
}

fn lifted(points: &[Vec<i64>], s: &[usize]) -> i128 {
    let norm = |p: &[i64]| {
        p.iter()
            .map(|&x| i128::from(x) * i128::from(x))
            .sum::<i128>()
    };
    let o = &points[s[0]];
    let rows: Vec<Vec<i128>> = s[1..]
        .iter()
        .map(|&i| {
            let p = &points[i];
            p.iter()
                .zip(o)
                .map(|(&x, &y)| i128::from(x - y))
                .chain(core::iter::once(norm(p) - norm(o)))
                .collect()
        })
        .collect();
    det(&rows)
}

fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current = (0..k).collect::<Vec<usize>>();
    loop {
        out.push(current.clone());
        let Some(i) = (0..k).rev().find(|&i| current[i] < n - k + i) else {
            return out;
        };
        current[i] += 1;
        for j in i + 1..k {
            current[j] = current[j - 1] + 1;
        }
    }
}

/// The Delaunay simplices by brute force, or `None` when the sites are not
/// in general position (D + 1 on a hyperplane, or D + 2 on a sphere).
fn brute_force(points: &[Vec<i64>]) -> Option<Vec<Vec<u32>>> {
    let n = points.len();
    let d = points[0].len();
    for s in subsets(n, d + 1) {
        if plain(points, &s) == 0 {
            return None;
        }
    }
    for s in subsets(n, d + 2) {
        if lifted(points, &s) == 0 {
            return None;
        }
    }
    let mut cells = Vec::new();
    for s in subsets(n, d + 1) {
        let orientation = plain(points, &s).signum();
        let empty = (0..n).filter(|q| !s.contains(q)).all(|q| {
            let mut with = s.clone();
            with.push(q);
            // q strictly inside the circumsphere: the lifted orientation
            // has the opposite sign of the simplex's own orientation.
            lifted(points, &with).signum() != -orientation
        });
        if empty {
            cells.push(s.iter().map(|&i| i as u32).collect());
        }
    }
    Some(cells)
}

fn random_sites(dim: usize, count: usize, seed: u64, range: u64) -> Vec<Vec<i64>> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            (0..dim)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state % range) as i64
                })
                .collect()
        })
        .collect()
}

fn general_position_matches(dim: usize, count: usize, trials: u64, range: u64) -> usize {
    let mut checked = 0;
    for seed in 1..=trials {
        let sites = random_sites(dim, count, seed * 7919, range);
        let Some(expected) = brute_force(&sites) else {
            continue;
        };
        let points: Vec<f64> = sites.iter().flatten().map(|&x| x as f64).collect();
        let t = triangulate(dim, &points);
        assert_eq!(cells(&t), expected, "D = {dim}, seed {seed}");
        check(&t, &points);
        // Scaling by 2^600 overflows every square in f64; the exact lift
        // keeps the same triangulation.
        let huge: Vec<f64> = points.iter().map(|x| x * 2f64.powi(600)).collect();
        assert_eq!(
            cells(&triangulate(dim, &huge)),
            expected,
            "D = {dim}, seed {seed}, huge"
        );
        checked += 1;
    }
    checked
}

#[test]
fn general_position_in_two_dimensions() {
    assert!(general_position_matches(2, 12, 40, 1000) >= 20);
}

#[test]
fn general_position_in_three_dimensions() {
    assert!(general_position_matches(3, 9, 20, 1000) >= 10);
}

/// The Delaunay conditions of `t` over the integer `sites`, by the `i128`
/// reference: every simplex is positive and has no site strictly inside its
/// circumsphere, every face without a neighbor has no site strictly beyond
/// it, and every representative is a vertex. Returns the number of
/// boundary faces with a site on their plane (a degenerate hull) and of
/// simplices with a cospherical site (a degenerate interior).
fn check_delaunay(t: &DelaunayTriangulation, sites: &[Vec<i64>]) -> (usize, usize) {
    let (mut flat_boundary, mut cospherical) = (0, 0);
    let mut used = vec![false; sites.len()];
    for s in &t.simplices {
        let cell: Vec<usize> = s.vertices.iter().map(|&v| v as usize).collect();
        let orientation = plain(sites, &cell);
        assert!(orientation > 0, "{cell:?} is positive");
        let mut on_sphere = false;
        for q in 0..sites.len() {
            if t.representative[q] != q as u32 || cell.contains(&q) {
                continue;
            }
            let mut with = cell.clone();
            with.push(q);
            let l = lifted(sites, &with);
            // q strictly inside: the lifted sign is the opposite of the
            // simplex's own (see `brute_force`).
            assert!(l.signum() != -orientation.signum(), "{q} inside {cell:?}");
            on_sphere |= l == 0;
        }
        cospherical += usize::from(on_sphere);
        for (slot, &n) in s.neighbors.iter().enumerate() {
            if n != NO_NEIGHBOR {
                continue;
            }
            // The face opposite `slot`, with the opposite vertex replaced by
            // each site: positive means beyond the face, away from it.
            let mut on_plane = false;
            for q in 0..sites.len() {
                if t.representative[q] != q as u32 || cell.contains(&q) {
                    continue;
                }
                let mut with = cell.clone();
                with[slot] = q;
                let side = plain(sites, &with);
                assert!(side >= 0, "{q} beyond the boundary face of {cell:?}");
                on_plane |= side == 0;
            }
            flat_boundary += usize::from(on_plane);
        }
        for &v in &cell {
            used[v] = true;
        }
    }
    for (q, &is_vertex) in used.iter().enumerate() {
        assert!(
            is_vertex || t.representative[q] != q as u32,
            "site {q} is a vertex"
        );
    }
    (flat_boundary, cospherical)
}

#[test]
fn degenerate_grids_are_delaunay() {
    // Integer sites with many on one line or plane of the hull boundary and
    // many cospherical (#189): a grid of 0..=3 in D = 2 and D = 3, the
    // boundary of a square, and lattice points on and inside a circle. Each
    // case must reach both degeneracies, so the outside-simplex rule for a
    // site on a hull plane and the cospherical merge both run.
    let mut cases: Vec<(usize, Vec<Vec<i64>>)> = Vec::new();
    cases.push((2, random_sites(2, 40, 3, 4)));
    cases.push((3, random_sites(3, 30, 5, 4)));
    let mut square: Vec<Vec<i64>> = Vec::new();
    for i in 0..=6 {
        square.extend([vec![i, 0], vec![6, i], vec![6 - i, 6], vec![0, 6 - i]]);
    }
    square.extend([vec![3, 3], vec![2, 4], vec![4, 1]]);
    cases.push((2, square));
    let mut circle: Vec<Vec<i64>> = [
        (5, 0),
        (4, 3),
        (3, 4),
        (0, 5),
        (-3, 4),
        (-4, 3),
        (-5, 0),
        (-4, -3),
        (-3, -4),
        (0, -5),
        (3, -4),
        (4, -3),
    ]
    .iter()
    .map(|&(x, y)| vec![x, y])
    .collect();
    // (5, 5) and (5, -5) put (5, 0) on a hull edge.
    circle.extend([vec![0, 0], vec![1, 2], vec![-2, 1], vec![5, 5], vec![5, -5]]);
    cases.push((2, circle));
    for (i, (dim, sites)) in cases.iter().enumerate() {
        let points: Vec<f64> = sites.iter().flatten().map(|&x| x as f64).collect();
        let t = triangulate(*dim, &points);
        check(&t, &points);
        let (flat_boundary, cospherical) = check_delaunay(&t, sites);
        assert!(
            flat_boundary > 0 && cospherical > 0,
            "case {i}: boundary sites on a plane {flat_boundary}, cospherical {cospherical}"
        );
        let parallel = DelaunayBuilder::new(*dim, &points)
            .parallel(true)
            .build()
            .unwrap();
        assert_eq!(t, parallel, "case {i}: parallel agrees");
    }
}

#[test]
fn parallel_execution_agrees() {
    let sites = random_sites(3, 60, 99, 1 << 20);
    let points: Vec<f64> = sites.iter().flatten().map(|&x| x as f64).collect();
    let sequential = triangulate(3, &points);
    let parallel = DelaunayBuilder::new(3, &points)
        .parallel(true)
        .build()
        .unwrap();
    assert_eq!(sequential, parallel);
    check(&sequential, &points);
}

#[test]
fn cospherical_groups_sharing_a_face_split_it_alike() {
    // The cube's eight corners are cospherical, and so are its bottom four
    // with the apex (1, 1, -5) below. The two lower facets meet in the bottom
    // square; placing in index order splits it the same way in both, so
    // every interior face lies in exactly two tetrahedra. The boundary is the
    // hull surface: top 2, sides 8, pyramid 4 triangles.
    let mut points: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    points.extend([1.0, 1.0, -5.0]);
    let t = triangulate(3, &points);
    check(&t, &points);
    let boundary: usize = t
        .simplices
        .iter()
        .map(|s| s.neighbors.iter().filter(|&&n| n == NO_NEIGHBOR).count())
        .sum();
    assert_eq!(boundary, 14);
    // Volumes of integer tetrahedra are exact sixths: cube 8, pyramid 20/3.
    let point = |i: u32| &points[i as usize * 3..i as usize * 3 + 3];
    let six_volume: f64 = t
        .simplices
        .iter()
        .map(|s| {
            let o = point(s.vertices[0]);
            let r: Vec<Vec<f64>> = s.vertices[1..]
                .iter()
                .map(|&v| point(v).iter().zip(o).map(|(x, y)| x - y).collect())
                .collect();
            r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
                - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
                + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
        })
        .sum();
    assert_eq!(six_volume, 6.0 * 8.0 + 40.0);
}

// Flat lift: the pulling triangulation (P4-2).

fn built(dim: usize, points: &[f64]) -> DelaunayTriangulation {
    let t = DelaunayBuilder::new(dim, points).build().unwrap();
    let parallel = DelaunayBuilder::new(dim, points)
        .parallel(true)
        .build()
        .unwrap();
    assert_eq!(t, parallel, "the parallel build differs");
    check(&t, points);
    t
}

#[test]
fn square_is_pulled_from_its_smallest_site() {
    // Facets of the square without site 0: edges {1, 2} and {2, 3}.
    let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    let t = built(2, &points);
    let m = NO_NEIGHBOR;
    assert_eq!(
        t.simplices,
        vec![
            DelaunaySimplex {
                vertices: vec![0, 1, 2],
                neighbors: vec![m, 1, m],
            },
            DelaunaySimplex {
                vertices: vec![0, 2, 3],
                neighbors: vec![m, m, 0],
            },
        ]
    );
}

#[test]
fn cocircular_pentagon_is_a_fan_from_site_0() {
    // Five sites on the circle of radius 5 with exact rational coordinates
    // (a regular pentagon is not representable exactly). The edges without
    // site 0 are {1, 2}, {2, 3}, {3, 4}.
    let points = [5.0, 0.0, 3.0, 4.0, -3.0, 4.0, -5.0, 0.0, 0.0, -5.0];
    let t = built(2, &points);
    assert_eq!(cells(&t), vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4]]);
}

#[test]
fn cube_corners_are_six_tetrahedra_from_corner_0() {
    // The three squares away from corner 0 are each split from their own
    // smallest corner, and 0 joins every triangle: 6 tetrahedra, volume 8,
    // 12 boundary triangles.
    let points: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    let t = built(3, &points);
    assert_eq!(t.simplices.len(), 6);
    assert!(t.simplices.iter().all(|s| s.vertices.contains(&0)));
    let boundary: usize = t
        .simplices
        .iter()
        .map(|s| s.neighbors.iter().filter(|&&n| n == NO_NEIGHBOR).count())
        .sum();
    assert_eq!(boundary, 12);
    // Squares x = 2 (corners 1, 3, 5, 7), y = 2 (2, 3, 6, 7), z = 2 (4, 5,
    // 6, 7), each pulled from its smallest corner.
    assert_eq!(
        cells(&t),
        vec![
            vec![0, 1, 3, 7],
            vec![0, 1, 5, 7],
            vec![0, 2, 3, 7],
            vec![0, 2, 6, 7],
            vec![0, 4, 5, 7],
            vec![0, 4, 6, 7],
        ]
    );
}

#[test]
fn d_plus_one_sites_are_one_simplex() {
    let triangle = [0.0, 0.0, 3.0, 0.0, 0.0, 2.0];
    let t = built(2, &triangle);
    assert_eq!(cells(&t), vec![vec![0, 1, 2]]);
    assert_eq!(t.simplices[0].neighbors, vec![NO_NEIGHBOR; 3]);
    let tetrahedron = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(cells(&built(3, &tetrahedron)), vec![vec![0, 1, 2, 3]]);
}

#[test]
fn one_dimension() {
    // Two sites: a flat lift. More sites: the lower hull of the parabola,
    // one segment between consecutive sites.
    let two = [3.0, -1.0];
    let t = built(1, &two);
    // Ascending [0, 1] runs from 3 to -1, negative, so the two swap.
    assert_eq!(t.simplices[0].vertices, vec![1, 0]);
    let line = [0.0, 3.0, 1.0, 7.0];
    assert_eq!(
        cells(&built(1, &line)),
        vec![vec![0, 2], vec![1, 2], vec![1, 3]]
    );
}

#[test]
fn pulled_simplices_have_empty_circumspheres() {
    // Every pulled simplex of a cospherical set has the common sphere as its
    // circumsphere, with every site on it and none inside: the lifted
    // orientation of each simplex with any other site is zero.
    let points = [
        5.0, 0.0, 3.0, 4.0, -3.0, 4.0, -5.0, 0.0, 0.0, -5.0, 4.0, -3.0,
    ];
    let t = built(2, &points);
    for s in &t.simplices {
        for q in 0..6_u32 {
            let mut p: Vec<&[f64]> = s
                .vertices
                .iter()
                .map(|&v| &points[v as usize * 2..v as usize * 2 + 2])
                .collect();
            p.push(&points[q as usize * 2..q as usize * 2 + 2]);
            assert_eq!(crate::predicates::orient_lifted(&p).unwrap(), Sign::Zero);
        }
    }
    assert_eq!(t.simplices.len(), 4);
}
