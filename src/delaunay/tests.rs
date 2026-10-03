//! Expectations come from each case's statement or from an independent
//! brute-force reference in `i128`, never from the library's own output.

use super::*;

fn triangulate(dim: usize, points: &[f64]) -> DelaunayTriangulation {
    match lower_hull(dim, points, Execution::Sequential).unwrap() {
        Lower::Triangulation(t) => t,
        Lower::Flat(_) => panic!("unexpected flat lift"),
    }
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
fn the_section_7_example() {
    // (0,0), (1,0), (0,1), (0.1,0.1): the first three form the upper facet
    // of the lifted tetrahedron; the point directly above them is on the
    // positive side of their outward order.
    let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.1, 0.1];
    let Ok(input) = accept(2, &points).unwrap().lift().unwrap() else {
        panic!("the lift spans three dimensions");
    };
    // Outward order of {0, 1, 2}: point 3 is inside, so negative.
    let mut outward = vec![0, 1, 2];
    let mut with_inner = outward.clone();
    with_inner.push(3);
    if input.orient(&with_inner).unwrap() == Sign::Positive {
        outward.swap(0, 1);
    }
    assert_eq!(lift_side(&input, &outward).unwrap(), Sign::Positive);

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
    let Lower::Flat(sites) = lower_hull(2, &points, Execution::Sequential).unwrap() else {
        panic!("four cocircular sites have a flat lift");
    };
    // The sites come back unlifted, for the pulling triangulation (P4-2).
    assert!(!sites.is_lifted());
    assert_eq!(sites.representatives, vec![0, 1, 2, 3]);
}

#[test]
fn degenerate_sites_report_original_indices() {
    // 1 duplicates 0; all on the line y = x.
    let points = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0];
    assert_eq!(
        lower_hull(2, &points, Execution::Sequential).err(),
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
            if j % 2 == 0 {
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

#[test]
fn parallel_execution_agrees() {
    let sites = random_sites(3, 60, 99, 1 << 20);
    let points: Vec<f64> = sites.iter().flatten().map(|&x| x as f64).collect();
    let sequential = triangulate(3, &points);
    let Lower::Triangulation(parallel) = lower_hull(3, &points, Execution::Parallel).unwrap()
    else {
        panic!("unexpected flat lift");
    };
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
