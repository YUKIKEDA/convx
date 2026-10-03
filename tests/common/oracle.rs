//! Brute-force exact oracles for Delaunay and Voronoi results (design §10).
//!
//! Every check runs over `i128` on integer coordinates, independently of the
//! library: plain and lifted orientations by cofactor expansion. The answer
//! is defined by these predicates, never by another implementation's
//! output (docs/verification.md).

use convx::{ConvexHullBuilder, DelaunayTriangulation, VoronoiDiagram};

/// Integer sites of dimension `dim` from exact integer `f64` coordinates.
pub fn integer_sites(dim: usize, points: &[f64]) -> Vec<Vec<i128>> {
    points
        .chunks_exact(dim)
        .map(|p| {
            p.iter()
                .map(|&x| {
                    assert_eq!(x.fract(), 0.0, "oracle sites must be integers");
                    x as i128
                })
                .collect()
        })
        .collect()
}

pub fn det(m: &[Vec<i128>]) -> i128 {
    if m.is_empty() {
        return 1;
    }
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

/// Orientation determinant of D + 1 sites in dimension D.
pub fn orientation(sites: &[Vec<i128>], s: &[u32]) -> i128 {
    let o = &sites[s[0] as usize];
    let rows: Vec<Vec<i128>> = s[1..]
        .iter()
        .map(|&i| {
            sites[i as usize]
                .iter()
                .zip(o)
                .map(|(x, y)| x - y)
                .collect()
        })
        .collect();
    det(&rows)
}

/// Lifted orientation determinant of D + 2 sites in dimension D.
pub fn lifted(sites: &[Vec<i128>], s: &[u32]) -> i128 {
    let norm = |p: &[i128]| p.iter().map(|x| x * x).sum::<i128>();
    let o = &sites[s[0] as usize];
    let rows: Vec<Vec<i128>> = s[1..]
        .iter()
        .map(|&i| {
            let p = &sites[i as usize];
            p.iter()
                .zip(o)
                .map(|(x, y)| x - y)
                .chain(core::iter::once(norm(p) - norm(o)))
                .collect()
        })
        .collect();
    det(&rows)
}

fn representatives(representative: &[u32]) -> Vec<u32> {
    (0..representative.len() as u32)
        .filter(|&i| representative[i as usize] == i)
        .collect()
}

fn face(v: &[u32], skip: usize) -> Vec<u32> {
    let mut f: Vec<u32> = v
        .iter()
        .enumerate()
        .filter(|&(k, _)| k != skip)
        .map(|(_, &x)| x)
        .collect();
    f.sort_unstable();
    f
}

/// Checks a Delaunay triangulation of integer sites against the exact
/// definition: orientation, empty circumspheres, neighbor symmetry across
/// the shared face, and coverage of the site hull (exact volume against the
/// public hull's boundary, and the Euler characteristic of the boundary).
pub fn check_delaunay(t: &DelaunayTriangulation, points: &[f64]) {
    let d = t.dim;
    let sites = integer_sites(d, points);
    let reps = representatives(&t.representative);
    let mut sorted: Vec<Vec<u32>> = Vec::new();
    for (i, s) in t.simplices.iter().enumerate() {
        assert_eq!(s.vertices.len(), d + 1);
        assert_eq!(s.neighbors.len(), d + 1);
        let o = orientation(&sites, &s.vertices);
        assert!(o > 0, "simplex {i} is not positively oriented");
        for &q in &reps {
            if s.vertices.contains(&q) {
                continue;
            }
            let mut with = s.vertices.clone();
            with.push(q);
            assert!(
                lifted(&sites, &with) >= 0,
                "site {q} inside the sphere of simplex {i}"
            );
        }
        for (slot, &n) in s.neighbors.iter().enumerate() {
            if n == u32::MAX {
                continue;
            }
            let other = &t.simplices[n as usize];
            let back = other.neighbors.iter().position(|&b| b == i as u32);
            let back = back.unwrap_or_else(|| panic!("neighbor {n} does not link back to {i}"));
            assert_eq!(face(&s.vertices, slot), face(&other.vertices, back));
        }
        let mut v = s.vertices.clone();
        v.sort_unstable();
        sorted.push(v);
    }
    assert!(
        sorted.windows(2).all(|w| w[0] < w[1]),
        "lexicographic order"
    );

    // Coverage: D! times the total volume equals the hull's, from the
    // public hull's boundary simplices joined to one extreme point.
    let total: i128 = t
        .simplices
        .iter()
        .map(|s| orientation(&sites, &s.vertices))
        .sum();
    let hull = ConvexHullBuilder::new(d, points).build().unwrap();
    if d >= 2 {
        let apex = hull.vertices[0];
        let hull_total: i128 = hull
            .triangulation()
            .iter()
            .map(|b| {
                let mut s = vec![apex];
                s.extend_from_slice(b.vertices);
                orientation(&sites, &s)
            })
            .sum();
        // Terms with the apex vanish; the others share one sign.
        assert_eq!(
            total,
            hull_total.abs(),
            "the simplices do not fill the site hull"
        );
    } else {
        let xs: Vec<i128> = hull
            .vertices
            .iter()
            .map(|&v| sites[v as usize][0])
            .collect();
        assert_eq!(total, xs.iter().max().unwrap() - xs.iter().min().unwrap());
    }
    // Every site is a vertex.
    for &r in &reps {
        assert!(
            t.simplices.iter().any(|s| s.vertices.contains(&r)),
            "site {r} missing"
        );
    }
    // The boundary faces form a closed (D - 1)-sphere: Euler characteristic
    // 2 for odd D, 0 for even D, counting every subface once.
    if d >= 2 {
        let mut faces: std::collections::BTreeSet<Vec<u32>> = Default::default();
        for s in &t.simplices {
            for (slot, &n) in s.neighbors.iter().enumerate() {
                if n != u32::MAX {
                    continue;
                }
                let f = face(&s.vertices, slot);
                for mask in 1_u64..(1 << f.len()) {
                    let sub: Vec<u32> = (0..f.len())
                        .filter(|&b| mask >> b & 1 == 1)
                        .map(|b| f[b])
                        .collect();
                    faces.insert(sub);
                }
            }
        }
        let euler: i64 = faces
            .iter()
            .map(|f| if f.len() % 2 == 1 { 1 } else { -1 })
            .sum();
        assert_eq!(
            euler,
            if d % 2 == 0 { 0 } else { 2 },
            "boundary Euler characteristic"
        );
    }
}

/// Checks that the Voronoi vertices are exactly the lower logical facets of
/// the lift: each vertex's sites are cospherical with no site strictly
/// inside and no other site on the sphere, and every Delaunay simplex lies in
/// exactly one vertex.
pub fn check_voronoi(v: &VoronoiDiagram, t: &DelaunayTriangulation, points: &[f64]) {
    let d = v.dim;
    let sites = integer_sites(d, points);
    let reps = representatives(&v.representative);
    for (i, vertex) in v.vertices.iter().enumerate() {
        // A simplex of the vertex's sites fixes its sphere.
        let basis = t
            .simplices
            .iter()
            .find(|s| s.vertices.iter().all(|x| vertex.sites.contains(x)))
            .unwrap_or_else(|| panic!("vertex {i} holds no Delaunay simplex"));
        for &q in &reps {
            let mut with = basis.vertices.clone();
            with.push(q);
            let l = lifted(&sites, &with);
            if vertex.sites.contains(&q) {
                assert_eq!(l, 0, "site {q} of vertex {i} is off its sphere");
            } else {
                assert!(l > 0, "site {q} is on or inside the sphere of vertex {i}");
            }
        }
    }
    for s in &t.simplices {
        let holders = v
            .vertices
            .iter()
            .filter(|x| s.vertices.iter().all(|y| x.sites.contains(y)))
            .count();
        assert_eq!(
            holders, 1,
            "simplex {:?} lies in {holders} vertices",
            s.vertices
        );
    }
}
