//! The hull invariants of design §10, checked in debug builds and in tests.
//!
//! 1. The Euler characteristic of the boundary simplicial complex.
//! 2. The same identity for the logical-facet complex, whose k-faces are the
//!    nonempty intersections of facet vertex sets of affine dimension k.
//! 3. No input point is strictly outside any logical facet.
//! 4. Neighbor symmetry on the simplex graph (every ridge lies in exactly two
//!    simplices), and on the logical-facet graph, whose lists are exactly the
//!    facets met in a ridge (a shared vertex set of affine dimension D - 2).
//! 5. The index partition.
//!
//! A violation is a convx bug, never a caller error.

use std::collections::{BTreeSet, HashMap};

use super::input::minimum_basis;
use super::publish::ConvexHull;
use crate::predicates::{orient, Sign};

/// Returns a description of the first violated invariant.
pub(crate) fn check(hull: &ConvexHull, points: &[f64]) -> Result<(), String> {
    let d = hull.dim;
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let expected_euler = if d % 2 == 0 { 0 } else { 2 };

    // 5. Index partition.
    let fixed: Vec<u32> = (0..hull.representative.len() as u32)
        .filter(|&i| hull.representative[i as usize] == i)
        .collect();
    let mut lists: Vec<u32> = hull
        .vertices
        .iter()
        .chain(&hull.coplanar_points)
        .chain(&hull.interior_points)
        .copied()
        .collect();
    let total = lists.len();
    lists.sort_unstable();
    lists.dedup();
    if lists.len() != total || lists != fixed {
        return Err("the three lists do not partition the representatives".into());
    }
    for (i, &r) in hull.representative.iter().enumerate() {
        if hull.representative[r as usize] != r || r as usize > i {
            return Err(format!(
                "representative of {i} is not a fixed point at or below it"
            ));
        }
    }

    // 1 and 4 on the simplicial complex.
    let mut faces: HashMap<usize, BTreeSet<Vec<u32>>> = HashMap::new();
    let mut ridges: HashMap<Vec<u32>, usize> = HashMap::new();
    for simplex in hull.triangulation().iter() {
        let mut sorted = simplex.vertices.to_vec();
        sorted.sort_unstable();
        for mask in 1_u64..(1 << d) {
            let subset: Vec<u32> = (0..d)
                .filter(|&b| mask >> b & 1 == 1)
                .map(|b| sorted[b])
                .collect();
            faces.entry(subset.len() - 1).or_default().insert(subset);
        }
        if d >= 2 {
            for skip in 0..d {
                let ridge: Vec<u32> = sorted
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != skip)
                    .map(|(_, &v)| v)
                    .collect();
                *ridges.entry(ridge).or_insert(0) += 1;
            }
        }
    }
    let euler: i64 = (0..d)
        .map(|k| {
            let count = faces.get(&k).map_or(0, BTreeSet::len) as i64;
            if k % 2 == 0 {
                count
            } else {
                -count
            }
        })
        .sum();
    if euler != expected_euler {
        return Err(format!(
            "simplicial Euler characteristic {euler}, expected {expected_euler}"
        ));
    }
    if let Some((ridge, count)) = ridges.iter().find(|&(_, &c)| c != 2) {
        return Err(format!("ridge {ridge:?} lies in {count} simplices"));
    }

    // 2. Logical-facet complex: close the facet vertex sets under
    // intersection and count each face by its affine dimension.
    let facet_sets: Vec<Vec<u32>> = hull.facets.iter().map(|f| f.vertices.clone()).collect();
    let mut all: BTreeSet<Vec<u32>> = facet_sets.iter().cloned().collect();
    let mut frontier: Vec<Vec<u32>> = facet_sets.clone();
    while let Some(face) = frontier.pop() {
        for facet in &facet_sets {
            let meet: Vec<u32> = face
                .iter()
                .copied()
                .filter(|v| facet.binary_search(v).is_ok())
                .collect();
            if !meet.is_empty() && all.insert(meet.clone()) {
                frontier.push(meet);
            }
        }
    }
    let mut logical_euler = 0_i64;
    for face in &all {
        let basis = minimum_basis(d, face, point).map_err(|e| e.to_string())?;
        let k = basis.len() - 1;
        if k >= d {
            return Err("a face spans the full dimension".into());
        }
        logical_euler += if k % 2 == 0 { 1 } else { -1 };
    }
    if logical_euler != expected_euler {
        return Err(format!(
            "logical Euler characteristic {logical_euler}, expected {expected_euler}"
        ));
    }

    // 4 on the logical-facet graph: two facets are neighbors exactly when
    // they meet in a ridge, a vertex set of affine dimension D - 2. In D = 1
    // there are no ridges and every list is empty.
    for (f, facet) in hull.facets.iter().enumerate() {
        let mut expected = Vec::new();
        if d >= 2 {
            for (g, other) in hull.facets.iter().enumerate() {
                if g == f {
                    continue;
                }
                let meet: Vec<u32> = facet
                    .vertices
                    .iter()
                    .copied()
                    .filter(|v| other.vertices.binary_search(v).is_ok())
                    .collect();
                if meet.len() < d - 1 {
                    continue;
                }
                let basis = minimum_basis(d, &meet, point).map_err(|e| e.to_string())?;
                if basis.len() == d - 1 {
                    expected.push(g as u32);
                }
            }
        }
        if facet.neighbors != expected {
            return Err(format!(
                "facet {f} lists neighbors {:?}, its ridges give {expected:?}",
                facet.neighbors
            ));
        }
    }

    // 3. No point strictly outside any facet. The side is the orientation of
    // D affinely independent facet vertices and the point, signed so that a
    // hull vertex off the facet is inside.
    for (f, facet) in hull.facets.iter().enumerate() {
        let basis: Vec<u32> = minimum_basis(d, &facet.vertices, point)
            .map_err(|e| e.to_string())?
            .into_iter()
            .take(d)
            .collect();
        let Some(&inner) = hull
            .vertices
            .iter()
            .find(|v| facet.vertices.binary_search(v).is_err())
        else {
            return Err(format!("facet {f} contains every vertex"));
        };
        let side = |q: u32| -> Result<Sign, String> {
            let mut pts: Vec<&[f64]> = basis.iter().map(|&v| point(v)).collect();
            pts.push(point(q));
            orient(&pts).map_err(|_| "exact evaluation exhausted".to_string())
        };
        let inside = side(inner)?;
        if inside == Sign::Zero {
            return Err(format!("facet {f} is not supporting"));
        }
        for q in 0..hull.representative.len() as u32 {
            if side(q)? == inside.reversed() {
                return Err(format!("point {q} is outside facet {f}"));
            }
        }
    }
    // Oriented triangulation: each simplex has a hull vertex off its facet
    // on the negative side.
    if d >= 2 {
        for simplex in hull.triangulation().iter() {
            let facet = &hull.facets[simplex.facet as usize];
            let Some(&inner) = hull
                .vertices
                .iter()
                .find(|v| facet.vertices.binary_search(v).is_err())
            else {
                return Err("a facet contains every vertex".into());
            };
            let mut pts: Vec<&[f64]> = simplex.vertices.iter().map(|&v| point(v)).collect();
            pts.push(point(inner));
            if orient(&pts).map_err(|_| "exact evaluation exhausted".to_string())? != Sign::Negative
            {
                return Err(format!(
                    "simplex {:?} is not in outward order",
                    simplex.vertices
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConvexHullBuilder;

    #[test]
    fn a_tampered_hull_is_rejected() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
        let hull = ConvexHullBuilder::new(2, &points).build().unwrap();
        assert_eq!(check(&hull, &points), Ok(()));

        let mut missing = hull.clone();
        missing.vertices.pop();
        assert!(check(&missing, &points).is_err());

        let mut one_sided = hull.clone();
        one_sided.facets[0].neighbors.pop();
        assert!(check(&one_sided, &points).is_err());

        // Dropping a shared ridge from both sides keeps the lists symmetric.
        let mut both_sides = hull.clone();
        let n = both_sides.facets[0].neighbors.remove(0);
        both_sides.facets[n as usize].neighbors.retain(|&m| m != 0);
        assert!(check(&both_sides, &points).is_err());

        // A facet listed as its own neighbor is not a ridge neighbor.
        let mut extra = hull.clone();
        extra.facets[0].neighbors.push(0);
        assert!(check(&extra, &points).is_err());

        // In D = 1 the lists are empty.
        let line = [0.0, 2.0, 1.0];
        let segment = ConvexHullBuilder::new(1, &line).build().unwrap();
        assert_eq!(check(&segment, &line), Ok(()));
        let mut linked = segment.clone();
        linked.facets[0].neighbors.push(1);
        linked.facets[1].neighbors.push(0);
        assert!(check(&linked, &line).is_err());

        let mut moved = points;
        moved[8] = 3.0;
        assert!(check(&hull, &moved).is_err(), "a point outside a facet");
    }
}
