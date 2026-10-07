//! The Delaunay triangulation as the lower hull of the lift (design §7).
//!
//! Each site `p` stands for the lifted point `(p, |p|^2)` in dimension
//! D + 1. The input array is not extended: every sign comes from the lifted
//! orientation, whose last column is the polynomial `|p|^2` evaluated by the
//! filter or exactly.
//!
//! The triangulation is built by incremental insertion ([`insert`]): the
//! representatives join one at a time, in a deterministic order, and each
//! keeps the complex the projection of the lower hull of the lift of the
//! sites so far. `parallel` runs the same insertion (design §6).
//!
//! The simplices of one lower logical facet of the lift (a cospherical
//! group) are merged by walking neighbors whose lifted orientation is zero
//! (design §8). A group of more than D + 1 sites projects one to one onto
//! the original space, where its sites are split by the placing
//! triangulation of §3, so groups that share a face split it the same way
//! and the split does not depend on the insertion order.
//!
//! When the lift is flat (every site on one sphere), no insertion runs: the
//! site hull is filled by the pulling triangulation of §7, on the boundary
//! that the hull core finds for the original sites.

mod insert;

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
/// let cells: Vec<&[u32]> = delaunay.simplices().iter().map(|s| s.vertices()).collect();
/// assert_eq!(cells, [&[0, 1, 2][..], &[0, 2, 3][..]]);
/// let first = delaunay.simplices().get(0).unwrap();
/// assert_eq!(first.neighbors(), [u32::MAX, 1, u32::MAX]);
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

    /// Runs the hull core of a flat lift on rayon's global pool when
    /// `enable` is true. The insertion itself is sequential either way
    /// (design §6), so the result is identical. Off by default.
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
        let (vertices, neighbors) = publish(complex.dim, self.points, cells)?;
        Ok(DelaunayTriangulation {
            dim: complex.dim,
            representative: complex.representative,
            vertices,
            neighbors,
        })
    }
}

/// A Delaunay triangulation of points in dimension D.
///
/// The simplices are kept as flat rows of D + 1 entries and published
/// through borrowed views (design §7, §9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelaunayTriangulation {
    dim: usize,
    representative: Vec<u32>,
    /// D + 1 sites per simplex, in public order.
    vertices: Vec<u32>,
    /// D + 1 neighbor numbers per simplex.
    neighbors: Vec<u32>,
}

impl DelaunayTriangulation {
    /// Dimension D of the sites.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// For each input index, the smallest index of a point equal to it.
    #[must_use]
    pub fn representative(&self) -> &[u32] {
        &self.representative
    }

    /// The simplices, in the lexicographic order of their ascending vertex
    /// lists before orientation is fixed.
    #[must_use]
    pub fn simplices(&self) -> Simplices<'_> {
        Simplices { delaunay: self }
    }
}

/// The simplices of a [`DelaunayTriangulation`]; a simplex's position is
/// its number.
#[derive(Clone, Copy)]
pub struct Simplices<'a> {
    delaunay: &'a DelaunayTriangulation,
}

impl<'a> Simplices<'a> {
    /// Number of simplices.
    #[must_use]
    pub fn len(&self) -> usize {
        self.delaunay.vertices.len() / (self.delaunay.dim + 1)
    }

    /// Whether there are no simplices. Never true for a built triangulation.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.delaunay.vertices.is_empty()
    }

    /// The simplex numbered `simplex`, or `None` when no simplex has that
    /// number.
    #[must_use]
    pub fn get(&self, simplex: u32) -> Option<DelaunaySimplex<'a>> {
        ((simplex as usize) < self.len()).then_some(DelaunaySimplex {
            delaunay: self.delaunay,
            index: simplex,
        })
    }

    /// Every simplex, in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = DelaunaySimplex<'a>> + 'a {
        let delaunay = self.delaunay;
        (0..self.len() as u32).map(move |index| DelaunaySimplex { delaunay, index })
    }
}

impl core::fmt::Debug for Simplices<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A D-simplex of a [`DelaunayTriangulation`]: a view into it.
#[derive(Clone, Copy)]
pub struct DelaunaySimplex<'a> {
    delaunay: &'a DelaunayTriangulation,
    index: u32,
}

impl<'a> DelaunaySimplex<'a> {
    fn row(&self, list: &'a [u32]) -> &'a [u32] {
        let k = self.delaunay.dim + 1;
        let i = self.index as usize;
        &list[i * k..(i + 1) * k]
    }

    /// D + 1 sites, ascending except that the last two are swapped when that
    /// makes the orientation positive. When the exact orientation is zero
    /// the ascending order is kept.
    #[must_use]
    pub fn vertices(&self) -> &'a [u32] {
        self.row(&self.delaunay.vertices)
    }

    /// `neighbors()[i]` is the simplex across the face opposite
    /// `vertices()[i]`, or `u32::MAX` on the boundary of the site hull.
    #[must_use]
    pub fn neighbors(&self) -> &'a [u32] {
        self.row(&self.delaunay.neighbors)
    }
}

impl core::fmt::Debug for DelaunaySimplex<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DelaunaySimplex")
            .field("vertices", &self.vertices())
            .field("neighbors", &self.neighbors())
            .finish()
    }
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
/// lift built by insertion, or the pulling triangulation as one group when
/// the lift is flat. Input checks and `DegenerateDimension` concern the
/// original sites, with their original indices.
pub(crate) fn complex(
    dim: usize,
    points: &[f64],
    execution: Execution,
) -> Result<Complex, ConvexHullError> {
    let input = accept(dim, points)?;
    let sites = insert::Sites::of(&input);
    let complex = if flat(&input, &sites)? {
        pull(input, execution)?
    } else {
        inserted(&input, &sites)?
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

/// Whether the lift is flat: every representative is cospherical with the
/// minimum basis, so the lifted sites span only dimension D (design §7).
fn flat(input: &Input<'_>, sites: &insert::Sites) -> Result<bool, ConvexHullError> {
    let basis = &input.spanning_points;
    let mut ids = basis.clone();
    ids.push(0);
    for &p in &input.representatives {
        if basis.contains(&p) {
            continue;
        }
        *ids.last_mut().unwrap_or(&mut 0) = p;
        if sites.lifted(&ids)? != Sign::Zero {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The complex by insertion, its cospherical groups merged (design §8).
fn inserted(input: &Input<'_>, sites: &insert::Sites) -> Result<Complex, ConvexHullError> {
    let d = input.dim();
    let first = &input.spanning_points;
    let rest: Vec<u32> = input
        .representatives
        .iter()
        .copied()
        .filter(|p| !first.contains(p))
        .collect();
    let order = insert::brio(sites, &rest);
    let mesh = insert::Mesh::build(sites, first, &order)?;

    // Union of the finite simplices that share a face and are cospherical
    // across it: the far vertex of the neighbor has lifted orientation zero
    // against the simplex.
    let cells: Vec<u32> = mesh.finite_cells().collect();
    let mut dense = vec![u32::MAX; mesh.slots()];
    for (i, &c) in cells.iter().enumerate() {
        dense[c as usize] = i as u32;
    }
    let mut parent: Vec<usize> = (0..cells.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut ids: Vec<u32> = Vec::with_capacity(d + 2);
    for (a, &c) in cells.iter().enumerate() {
        for slot in 0..=d {
            let n = mesh.neighbor(c, slot);
            if n < c || !mesh.is_finite(n) {
                continue;
            }
            let Some(back) = (0..=d).find(|&t| mesh.neighbor(n, t) == c) else {
                debug_assert!(false, "neighbors are symmetric");
                continue;
            };
            ids.clear();
            ids.extend_from_slice(mesh.vertices_of(c));
            ids.push(mesh.vertices_of(n)[back]);
            if mesh.lifted_ids(&ids)? == Sign::Zero {
                let (x, y) = (
                    root(&mut parent, a),
                    root(&mut parent, dense[n as usize] as usize),
                );
                parent[x] = y;
            }
        }
    }
    let mut members: Vec<(usize, u32)> = (0..cells.len())
        .map(|a| (root(&mut parent, a), cells[a]))
        .collect();
    members.sort_unstable();
    let mut groups = Vec::new();
    for run in members.chunk_by(|x, y| x.0 == y.0) {
        let mut sites: Vec<u32> = run
            .iter()
            .flat_map(|&(_, c)| mesh.vertices_of(c).iter().copied())
            .collect();
        sites.sort_unstable();
        sites.dedup();
        let cells = if run.len() == 1 {
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
    groups.sort_unstable_by(|a, b| a.sites.cmp(&b.sites));
    Ok(Complex {
        dim: d,
        representative: input.representative.clone(),
        groups,
    })
}

/// Orders, orients, and links the cells (ascending vertex lists) of sites
/// of dimension `d` stored row-major in `points`, into flat rows of
/// D + 1 vertices and D + 1 neighbors per simplex.
fn publish(
    d: usize,
    points: &[f64],
    mut cells: Vec<Vec<u32>>,
) -> Result<(Vec<u32>, Vec<u32>), ConvexHullError> {
    cells.sort_unstable();
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let k = d + 1;
    let mut vertices = Vec::with_capacity(cells.len() * k);
    for cell in cells {
        let start = vertices.len();
        vertices.extend_from_slice(&cell);
        let points: Small<&[f64], 11> = cell.iter().map(|&v| point(v)).collect();
        if orient(&points)? == Sign::Negative {
            vertices.swap(start + d - 1, start + d);
        }
    }
    let oriented: Vec<&[u32]> = vertices.chunks_exact(k).collect();
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
    let mut neighbors = vec![NO_NEIGHBOR; vertices.len()];
    for (first, second) in pair_equal_keys_with_border(&keys, owners.len(), fingerprint) {
        let (a, slot_a) = owners[first];
        let (b, slot_b) = owners[second];
        neighbors[a * k + slot_a] = b as u32;
        neighbors[b * k + slot_b] = a as u32;
    }
    Ok((vertices, neighbors))
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
    let facets: Vec<Vec<u32>> = merge(&hull)?.vertices.iter().map(<[u32]>::to_vec).collect();
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
