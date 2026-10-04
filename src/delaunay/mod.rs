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
//!
//! When the lift is flat (every site on one sphere), the lower-side sign is
//! not used: the site hull is filled by the pulling triangulation of §7, on
//! the boundary that the hull core finds for the original sites.

use std::collections::HashSet;

use crate::hull::classify::placing;
use crate::hull::input::{accept, minimum_basis, Input};
use crate::hull::merge::merge;
use crate::hull::ridge::{fingerprint, pair_equal_keys_with_border};
use crate::hull::simplicial::{Execution, SimplicialHull};
use crate::hull::ConvexHullError;
use crate::predicates::{orient, Sign};
use crate::small::Small;

/// A neighbor slot with no simplex across it. Point and simplex numbers are
/// below `u32::MAX`, so this value names neither.
pub(crate) const NO_NEIGHBOR: u32 = u32::MAX;

/// Builds a [`DelaunayTriangulation`].
///
/// ```
/// use convx::DelaunayBuilder;
///
/// // A square: its four corners are cocircular, so the site hull is split
/// // by pulling from the smallest index into two triangles.
/// let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
/// let delaunay = DelaunayBuilder::new(2, &points).build()?;
/// let cells: Vec<&[u32]> = delaunay
///     .simplices
///     .iter()
///     .map(|s| s.vertices.as_slice())
///     .collect();
/// assert_eq!(cells, [&[0, 1, 2][..], &[0, 2, 3][..]]);
/// assert_eq!(delaunay.simplices[0].neighbors, [u32::MAX, 1, u32::MAX]);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct DelaunayBuilder<'a> {
    dim: usize,
    points: &'a [f64],
    execution: Execution,
}

impl<'a> DelaunayBuilder<'a> {
    /// A builder for sites of dimension `dim` (the original dimension, not
    /// the lifted one), stored row-major in `points`.
    #[must_use]
    pub fn new(dim: usize, points: &'a [f64]) -> Self {
        Self {
            dim,
            points,
            execution: Execution::Sequential,
        }
    }

    /// Plans the points of each batch on rayon's global pool when `enable`
    /// is true. Off by default. The result is identical either way.
    #[must_use]
    pub fn parallel(self, enable: bool) -> Self {
        Self {
            execution: if enable {
                Execution::Parallel
            } else {
                Execution::Sequential
            },
            ..self
        }
    }

    /// Builds the triangulation.
    ///
    /// # Errors
    ///
    /// The input failures of [`ConvexHullError`], in the order documented
    /// there; [`ConvexHullError::DegenerateDimension`] only when the sites
    /// do not span dimension D, with their original indices; and
    /// [`ConvexHullError::ExactEvaluationExhausted`]. Sites that all lie on
    /// one sphere succeed.
    pub fn build(self) -> Result<DelaunayTriangulation, ConvexHullError> {
        let complex = complex(self.dim, self.points, self.execution)?;
        let cells = complex.groups.into_iter().flat_map(|g| g.cells).collect();
        Ok(DelaunayTriangulation {
            dim: complex.dim,
            simplices: publish(complex.dim, self.points, cells)?,
            representative: complex.representative,
        })
    }
}

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
    /// `vertices[i]`, or `u32::MAX` on the boundary of the site hull.
    pub neighbors: Vec<u32>,
}

/// The Delaunay complex before diagonals are inserted (design §8): one
/// group per lower logical facet of the lift, with the simplices it is cut
/// into.
pub(crate) struct Complex {
    pub(crate) dim: usize,
    pub(crate) representative: Vec<u32>,
    pub(crate) groups: Vec<Group>,
}

/// One lower logical facet of the lift: cospherical sites and their cells.
pub(crate) struct Group {
    /// The sites, ascending; at least D + 1.
    pub(crate) sites: Vec<u32>,
    /// The simplices of the group, ascending vertex lists.
    pub(crate) cells: Vec<Vec<u32>>,
}

/// The Delaunay complex of `points` (dimension `dim`): the lower hull of the
/// lift, or the pulling triangulation as one group when the lift is flat.
pub(crate) fn complex(
    dim: usize,
    points: &[f64],
    execution: Execution,
) -> Result<Complex, ConvexHullError> {
    let complex = match lower_hull(dim, points, execution)? {
        Ok(complex) => complex,
        Err(flat) => pull(flat, execution)?,
    };
    debug_assert!(
        {
            let mut seen: Vec<u32> = complex
                .groups
                .iter()
                .flat_map(|g| g.sites.iter().copied())
                .collect();
            seen.sort_unstable();
            seen.dedup();
            let mut reps: Vec<u32> = (0..complex.representative.len() as u32)
                .filter(|&i| complex.representative[i as usize] == i)
                .collect();
            reps.sort_unstable();
            seen == reps
        },
        "every site is a vertex of the Delaunay complex"
    );
    Ok(complex)
}

/// The lower hull of the lift, or `Err` with the accepted sites when the
/// lift is flat. Input checks and `DegenerateDimension` concern the original
/// sites, with their original indices.
pub(crate) fn lower_hull(
    dim: usize,
    points: &[f64],
    execution: Execution,
) -> Result<Result<Complex, Input<'_>>, ConvexHullError> {
    let input = accept(dim, points)?;
    let lifted = match input.lift()? {
        Ok(lifted) => lifted,
        Err(flat) => return Ok(Err(flat)),
    };
    let hull = SimplicialHull::build(lifted, execution)?;
    let logical = merge(&hull)?;
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

    let mut groups = Vec::new();
    for group in logical.groups {
        let Some(outward) = group.simplices.first().and_then(|&id| hull.facets.get(id)) else {
            continue;
        };
        if lift_side(input, &outward.vertices)? != Sign::Negative {
            continue;
        }
        let sites = group.vertices;
        let cells = if sites.len() == d + 1 {
            vec![sites.clone()]
        } else {
            let mut split = placing(d, |i| input.point(i), &sites)?;
            debug_assert!(
                split.iter().all(|s| s.len() == d + 1),
                "a lower facet projects onto a full-dimensional region"
            );
            for cell in &mut split {
                cell.sort_unstable();
            }
            split
        };
        groups.push(Group { sites, cells });
    }
    Ok(Ok(Complex {
        dim: d,
        representative: input.representative.clone(),
        groups,
    }))
}

/// The side of the lifted facet with vertices `outward` (outward order,
/// D + 1 sites): the orientation of the §7 test point, which equals the
/// orientation of the sites in the original space in the same order.
/// [`Sign::Negative`] is the lower side, [`Sign::Positive`] the upper side,
/// and [`Sign::Zero`] a facet of zero volume in the original space.
fn lift_side(input: &Input<'_>, outward: &[u32]) -> Result<Sign, ConvexHullError> {
    let points: Small<&[f64], 11> = outward.iter().map(|&v| input.point(v)).collect();
    Ok(orient(&points)?)
}

/// Orders, orients, and links the cells (ascending vertex lists) of sites
/// of dimension `d` stored row-major in `points`.
fn publish(
    d: usize,
    points: &[f64],
    mut cells: Vec<Vec<u32>>,
) -> Result<Vec<DelaunaySimplex>, ConvexHullError> {
    cells.sort_unstable();
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let mut oriented = Vec::with_capacity(cells.len());
    for mut vertices in cells {
        let points: Small<&[f64], 11> = vertices.iter().map(|&v| point(v)).collect();
        if orient(&points)? == Sign::Negative {
            vertices.swap(d - 1, d);
        }
        oriented.push(vertices);
    }
    // Each face as its sorted vertex list, packed in one buffer, with its
    // owner; a face lies in at most two simplices.
    let mut keys: Vec<u32> = Vec::with_capacity(oriented.len() * (d + 1) * d);
    let mut owners: Vec<(usize, usize)> = Vec::with_capacity(oriented.len() * (d + 1));
    for (s, vertices) in oriented.iter().enumerate() {
        for slot in 0..vertices.len() {
            let start = keys.len();
            keys.extend(
                vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v),
            );
            keys[start..].sort_unstable();
            owners.push((s, slot));
        }
    }
    let mut simplices: Vec<DelaunaySimplex> = oriented
        .into_iter()
        .map(|vertices| DelaunaySimplex {
            neighbors: vec![NO_NEIGHBOR; vertices.len()],
            vertices,
        })
        .collect();
    for (first, second) in pair_equal_keys_with_border(&keys, owners.len(), fingerprint) {
        let (a, slot_a) = owners[first];
        let (b, slot_b) = owners[second];
        simplices[a].neighbors[slot_a] = b as u32;
        simplices[b].neighbors[slot_b] = a as u32;
    }
    Ok(simplices)
}

/// The pulling triangulation of the site hull when every site lies on one
/// sphere (design §7).
///
/// The boundary comes from the hull core on the original sites, inside
/// Delaunay: no public plane is built, so none can fail. Every site lies on
/// the sphere, hence is extreme and a vertex of the simplicial hull, so the
/// merged groups are the facets with all their sites. A face is split from
/// its smallest site `v`: each facet of the face that does not contain `v`
/// is split by the same rule, and `v` joins every simplex found; a face that
/// is already a simplex is returned as it is. The recursion runs on an
/// explicit stack.
fn pull(sites: Input<'_>, execution: Execution) -> Result<Complex, ConvexHullError> {
    let d = sites.dim();
    let hull = SimplicialHull::build(sites, execution)?;
    let facets: Vec<Vec<u32>> = merge(&hull)?
        .groups
        .into_iter()
        .map(|g| g.vertices)
        .collect();
    let input = &hull.input;
    let point = |i: u32| input.point(i);
    let affine_dim = |set: &[u32]| -> Result<usize, ConvexHullError> {
        Ok(minimum_basis(d, set, point)?.len().saturating_sub(1))
    };

    // (face, its affine dimension, the sites pulled so far)
    let mut stack: Vec<(Vec<u32>, usize, Vec<u32>)> =
        vec![(input.representatives.clone(), d, Vec::new())];
    let mut cells = Vec::new();
    while let Some((face, k, pulled)) = stack.pop() {
        if face.len() == k + 1 {
            let mut cell = pulled;
            cell.extend(face);
            cell.sort_unstable();
            cells.push(cell);
            continue;
        }
        let v = face[0];
        // The facets of a face of dimension k are its intersections of
        // dimension k - 1 with the facets of the site hull.
        let mut sub: Vec<Vec<u32>> = Vec::new();
        for facet in &facets {
            let meet: Vec<u32> = face
                .iter()
                .copied()
                .filter(|x| facet.binary_search(x).is_ok())
                .collect();
            if meet.len() < k || meet.contains(&v) || sub.contains(&meet) {
                continue;
            }
            if affine_dim(&meet)? == k - 1 {
                sub.push(meet);
            }
        }
        for facet in sub {
            let mut next = pulled.clone();
            next.push(v);
            stack.push((facet, k - 1, next));
        }
    }
    // One lower facet: the whole site set is one Voronoi vertex (§8).
    Ok(Complex {
        dim: d,
        representative: input.representative.clone(),
        groups: vec![Group {
            sites: input.representatives.clone(),
            cells,
        }],
    })
}

#[cfg(test)]
mod tests;
