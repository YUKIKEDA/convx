//! The Delaunay triangulation as the lower hull of the lift (design §7).
//!
//! Each site `p` stands for the lifted point `(p, |p|^2)` in dimension
//! D + 1. The input array is not extended: every sign comes from the lifted
//! orientation, whose last column is the polynomial `|p|^2` evaluated by the
//! filter or exactly. The hull core runs on the lifted sites with the same
//! build path as the convex hull (batch rounds, the coplanar merge).
//!
//! A logical facet of the lifted hull is on the lower side when the test
//! point of §7, the first vertex of the facet's outward order moved by +1 in
//! the lifted coordinate only, has negative orientation against that order.
//! With the first vertex as origin, the test point's row is the lifted unit
//! vector, so the determinant expands to the orientation in the original
//! space of the facet's vertices projected, in the same order. That sign is
//! the one computed; neither index order nor the public normal decides.
//!
//! Every lifted site is extreme, since the paraboloid is strictly convex,
//! so every site becomes a vertex of the simplicial lifted hull and lies on
//! the lower hull. A lower logical facet with more than D + 1
//! sites (a cospherical group) projects one to one onto the original space,
//! where its sites are split by the placing triangulation of §3, so groups
//! that share a face split it the same way.

// P4-2 (#20) publishes `DelaunayBuilder`, the first caller outside tests.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{HashMap, HashSet};

use crate::hull::classify::placing;
use crate::hull::input::{accept, Input};
use crate::hull::merge::merge;
use crate::hull::simplicial::{Execution, SimplicialHull};
use crate::hull::ConvexHullError;
use crate::predicates::{orient, Sign};

/// A neighbor slot with no simplex across it. Point and simplex numbers are
/// below `u32::MAX`, so this value names neither.
pub const NO_NEIGHBOR: u32 = u32::MAX;

/// A Delaunay triangulation of points in dimension D.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelaunayTriangulation {
    /// Dimension D of the sites.
    pub dim: usize,
    /// For each input index, the smallest index of a point equal to it.
    pub representative: Vec<u32>,
    /// The simplices, in the lexicographic order of their ascending vertex
    /// lists before orientation is fixed.
    pub simplices: Vec<DelaunaySimplex>,
}

/// A D-simplex of a [`DelaunayTriangulation`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelaunaySimplex {
    /// D + 1 sites, ascending except that the last two are swapped when that
    /// makes the orientation positive. When the exact orientation is zero
    /// the ascending order is kept.
    pub vertices: Vec<u32>,
    /// `neighbors[i]` is the simplex across the face opposite
    /// `vertices[i]`, or [`NO_NEIGHBOR`] on the boundary of the site hull.
    pub neighbors: Vec<u32>,
}

/// The lower hull of the lift, or the sites when the lift is flat.
pub(crate) enum Lower<'a> {
    Triangulation(DelaunayTriangulation),
    /// Every site on one sphere: the pulling triangulation of P4-2 (#20).
    Flat(Input<'a>),
}

/// Builds the Delaunay triangulation of `points` (dimension `dim`) as the
/// lower hull of the lift. Input checks and `DegenerateDimension` concern
/// the original sites, with their original indices.
pub(crate) fn lower_hull(
    dim: usize,
    points: &[f64],
    execution: Execution,
) -> Result<Lower<'_>, ConvexHullError> {
    let input = accept(dim, points)?;
    let lifted = match input.lift()? {
        Ok(lifted) => lifted,
        Err(flat) => return Ok(Lower::Flat(flat)),
    };
    let hull = SimplicialHull::build(lifted, execution)?;
    let groups = merge(&hull)?;
    let input = &hull.input;
    let d = input.dim();

    // Every representative is a vertex of the simplicial lifted hull: a
    // site never inserted would lie on the final hull inside the convex hull
    // of inserted sites, which a strictly convex paraboloid rules out. So a
    // lower group's vertex set already holds every site on its hyperplane.
    debug_assert!(
        {
            let on_complex: HashSet<u32> = hull
                .facets
                .iter()
                .flat_map(|(_, f)| f.vertices.iter().copied())
                .collect();
            input.representatives.iter().all(|r| on_complex.contains(r))
        },
        "every lifted site is a vertex of the simplicial hull"
    );

    let mut cells: Vec<Vec<u32>> = Vec::new();
    for group in &groups.groups {
        let Some(outward) = group
            .simplices
            .first()
            .and_then(|&id| hull.facets.get(id))
            .map(|s| s.vertices.clone())
        else {
            continue;
        };
        if lift_side(input, &outward)? != Sign::Negative {
            continue;
        }
        let vertices = &group.vertices;
        if vertices.len() == d + 1 {
            cells.push(vertices.clone());
        } else {
            let split = placing(d, |i| input.point(i), vertices)?;
            debug_assert!(
                split.iter().all(|s| s.len() == d + 1),
                "a lower facet projects onto a full-dimensional region"
            );
            cells.extend(split);
        }
    }
    let simplices = publish(input, cells)?;
    debug_assert!(
        input
            .representatives
            .iter()
            .all(|r| simplices.iter().any(|s| s.vertices.contains(r))),
        "every site is a vertex of the lower hull"
    );
    Ok(Lower::Triangulation(DelaunayTriangulation {
        dim: d,
        representative: input.representative.clone(),
        simplices,
    }))
}

/// The side of the lifted facet with vertices `outward` (outward order,
/// D + 1 sites): the orientation of the §7 test point, which equals the
/// orientation of the sites in the original space in the same order.
/// [`Sign::Negative`] is the lower side, [`Sign::Positive`] the upper side,
/// and [`Sign::Zero`] a facet of zero volume in the original space.
fn lift_side(input: &Input<'_>, outward: &[u32]) -> Result<Sign, ConvexHullError> {
    let points: Vec<&[f64]> = outward.iter().map(|&v| input.point(v)).collect();
    Ok(orient(&points)?)
}

/// Orders, orients, and links the cells (ascending vertex lists).
fn publish(
    input: &Input<'_>,
    mut cells: Vec<Vec<u32>>,
) -> Result<Vec<DelaunaySimplex>, ConvexHullError> {
    cells.sort_unstable();
    let d = input.dim();
    let mut oriented = Vec::with_capacity(cells.len());
    for mut vertices in cells {
        let points: Vec<&[f64]> = vertices.iter().map(|&v| input.point(v)).collect();
        if orient(&points)? == Sign::Negative {
            vertices.swap(d - 1, d);
        }
        oriented.push(vertices);
    }
    let mut faces: HashMap<Vec<u32>, Vec<(usize, usize)>> = HashMap::new();
    for (s, vertices) in oriented.iter().enumerate() {
        for slot in 0..vertices.len() {
            let mut face: Vec<u32> = vertices
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != slot)
                .map(|(_, &v)| v)
                .collect();
            face.sort_unstable();
            faces.entry(face).or_default().push((s, slot));
        }
    }
    let mut simplices: Vec<DelaunaySimplex> = oriented
        .into_iter()
        .map(|vertices| DelaunaySimplex {
            neighbors: vec![NO_NEIGHBOR; vertices.len()],
            vertices,
        })
        .collect();
    for sharing in faces.values() {
        debug_assert!(sharing.len() <= 2, "a face lies in at most two simplices");
        if let [(a, slot_a), (b, slot_b)] = sharing[..] {
            simplices[a].neighbors[slot_a] = b as u32;
            simplices[b].neighbors[slot_b] = a as u32;
        }
    }
    Ok(simplices)
}

#[cfg(test)]
mod tests;
