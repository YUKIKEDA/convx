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
//! sites so far.
//!
//! The simplices of one lower logical facet of the lift (a cospherical
//! group) are merged by walking neighbors whose lifted orientation is zero
//! (design §8). A group of more than D + 1 sites projects one to one onto
//! the original space, where its sites are split by the placing
//! triangulation of §3, so groups that share a face split it the same way
//! and the split does not depend on the insertion order.
//!
//! The result is read from the mesh (#255): a simplex that is a group by
//! itself is published as the mesh holds it, its orientation from the parity
//! of sorting its vertices and its neighbors from the mesh's links. Only the
//! faces of split groups are paired by their vertex sets.
//!
//! When the lift is flat (every site on one sphere), no insertion runs: the
//! site hull is filled by the pulling triangulation of §7, on the boundary
//! that the hull core finds for the original sites.

mod insert;

use crate::hull::classify::placing;
use crate::hull::input::{accept, minimum_basis, Input};
use crate::hull::merge::merge;
use crate::hull::ridge::{fingerprint, pair_equal_keys_with_border};
use crate::hull::simplicial::SimplicialHull;
use crate::hull::ConvexHullError;
use crate::lists::Lists;
use crate::predicates::{orient, Sign, Start};
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
/// let mut cells: Vec<&[u32]> = delaunay.simplices().iter().map(|s| s.vertices()).collect();
/// // The simplices come in the order the construction leaves.
/// cells.sort();
/// assert_eq!(cells, [&[0, 1, 2][..], &[0, 2, 3][..]]);
/// // Each names the other across the shared edge {0, 2}; the other faces
/// // are on the boundary.
/// let first = delaunay.simplices().get(0).unwrap();
/// let across: Vec<u32> = first.neighbors().iter().copied().filter(|&n| n != u32::MAX).collect();
/// assert_eq!(across, [1]);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct DelaunayBuilder<'a> {
    dim: usize,
    points: &'a [f64],
}

impl<'a> DelaunayBuilder<'a> {
    /// A builder for sites of dimension `dim` (the original dimension, not
    /// the lifted one), stored row-major in `points`.
    #[must_use]
    pub fn new(dim: usize, points: &'a [f64]) -> Self {
        Self { dim, points }
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
        Ok(publish(complex(self.dim, self.points)?))
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

    /// The simplices, in the order the construction leaves: the same for
    /// the same input on one binary, and not promised across versions or for
    /// a permutation of the input (design §7).
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

/// The Delaunay complex before diagonals are inserted (design §8), and the
/// simplices it is cut into, numbered and linked in public order: the order
/// the construction leaves (design §7).
pub(crate) struct Complex {
    pub(crate) dim: usize,
    pub(crate) representative: Vec<u32>,
    /// Per lower logical facet of the lift (a group of cospherical sites),
    /// its sites, ascending; at least D + 1. Groups are in no fixed order.
    pub(crate) sites: Lists<u32>,
    /// D + 1 sites per cell, ascending; cells in the order of the draft,
    /// which is the order of [`DelaunayTriangulation`].
    pub(crate) cells: Vec<u32>,
    /// D + 1 per cell: the cell across the face opposite `cells[c * k + i]`,
    /// or [`NO_NEIGHBOR`] on the boundary of the site hull.
    pub(crate) neighbors: Vec<u32>,
    /// Per cell, whether the ascending order has negative orientation.
    pub(crate) negative: Vec<bool>,
    /// Per cell, its group in [`Self::sites`].
    pub(crate) group: Vec<u32>,
}

/// A face whose cell across is not known yet; it is paired by its vertex
/// set when the complex is numbered. Cell numbers stay below it.
const UNKNOWN: u32 = u32::MAX - 1;

/// The cells of a complex before they are numbered.
struct Draft {
    k: usize,
    /// D + 1 sites per cell, ascending.
    rows: Vec<u32>,
    /// As `rows`: the draft cell across each face, [`NO_NEIGHBOR`] on the
    /// boundary of the site hull, or [`UNKNOWN`].
    links: Vec<u32>,
    negative: Vec<bool>,
    group: Vec<u32>,
    sites: Lists<u32>,
}

impl Draft {
    fn new(k: usize, cells: usize) -> Self {
        Self {
            k,
            rows: Vec::with_capacity(cells * k),
            links: Vec::with_capacity(cells * k),
            negative: Vec::with_capacity(cells),
            group: Vec::with_capacity(cells),
            sites: Lists::with_capacity(cells, cells * k),
        }
    }

    fn len(&self) -> usize {
        self.negative.len()
    }

    fn row(&self, c: usize) -> &[u32] {
        &self.rows[c * self.k..(c + 1) * self.k]
    }

    /// Adds a group of the cells `cells` (ascending vertex lists, each with
    /// whether that order is negative), every face to be paired by its
    /// vertex set.
    fn push_group(&mut self, sites: &[u32], cells: &[(Vec<u32>, bool)]) {
        let group = self.sites.len() as u32;
        self.sites.push(sites);
        for (row, negative) in cells {
            self.rows.extend_from_slice(row);
            self.links.extend(core::iter::repeat_n(UNKNOWN, self.k));
            self.negative.push(*negative);
            self.group.push(group);
        }
    }

    /// Pairs the faces left [`UNKNOWN`] by their vertex sets. The cells keep
    /// their draft order and numbers, which are the public ones: nothing is
    /// sorted for publication (design §7).
    fn finish(mut self, dim: usize, representative: Vec<u32>) -> Complex {
        let k = self.k;
        let n = self.len();
        // Faces to pair: each lies in one cell (the site hull's boundary) or
        // in two. Only cells of merged groups and their neighbors have any.
        let mut keys: Vec<u32> = Vec::new();
        let mut owners: Vec<(u32, usize)> = Vec::new();
        for c in 0..n {
            for slot in 0..k {
                if self.links[c * k + slot] != UNKNOWN {
                    continue;
                }
                let row = self.row(c);
                keys.extend(
                    row.iter()
                        .enumerate()
                        .filter(|&(i, _)| i != slot)
                        .map(|(_, &v)| v),
                );
                owners.push((c as u32, slot));
            }
        }
        for &(c, slot) in &owners {
            self.links[c as usize * k + slot] = NO_NEIGHBOR;
        }
        for (a, b) in pair_equal_keys_with_border(&keys, owners.len(), fingerprint) {
            let ((ca, sa), (cb, sb)) = (owners[a], owners[b]);
            self.links[ca as usize * k + sa] = cb;
            self.links[cb as usize * k + sb] = ca;
        }
        drop((keys, owners));
        Complex {
            dim,
            representative,
            sites: self.sites,
            cells: self.rows,
            neighbors: self.links,
            negative: self.negative,
            group: self.group,
        }
    }
}

/// The ascending order of the D + 1 distinct vertices of a positive simplex
/// `row`, each with its slot in `row`, and whether that order is negative:
/// the parity of the sort. Every swap is a transposition, so the parity is
/// that of their count. Three and four vertices (D = 2 and 3) go through a
/// sorting network whose swaps are selected, not branched on: the order of
/// the vertices is not predictable, and the branches of an insertion sort
/// cost about half of building the published rows (#362).
fn ascending(row: &[u32]) -> (Small<(u32, usize), 11>, bool) {
    // Each vertex above its slot in one word: distinct vertices order the
    // words as they order themselves. On the stack up to 11 vertices (D = 10),
    // on the heap above, as the sorted list is.
    let mut words: Small<u64, 11> = row
        .iter()
        .enumerate()
        .map(|(slot, &v)| u64::from(v) << 32 | slot as u64)
        .collect();
    let mut negative = false;
    let mut exchange = |a: &mut [u64], i: usize, j: usize| {
        negative ^= a[i] > a[j];
        (a[i], a[j]) = (a[i].min(a[j]), a[i].max(a[j]));
    };
    match row.len() {
        3 => {
            for (i, j) in [(0, 1), (1, 2), (0, 1)] {
                exchange(&mut words, i, j);
            }
        }
        4 => {
            for (i, j) in [(0, 1), (2, 3), (0, 2), (1, 3), (1, 2)] {
                exchange(&mut words, i, j);
            }
        }
        n => {
            for i in 1..n {
                for j in (1..=i).rev() {
                    if words[j - 1] < words[j] {
                        break;
                    }
                    exchange(&mut words, j - 1, j);
                }
            }
        }
    }
    let sorted = words
        .iter()
        .map(|&w| ((w >> 32) as u32, (w & u64::from(u32::MAX)) as usize))
        .collect();
    (sorted, negative)
}

/// Whether the mesh is built on the sites in the input's order instead of
/// the order of insertion: a test's override on this thread, to show that
/// the storage order changes nothing published. Always false otherwise.
fn stored_in_input_order() -> bool {
    #[cfg(test)]
    {
        tests::STORED_IN_INPUT_ORDER.with(core::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}

/// Whether D = 2 and D = 3 take the shape of any dimension: a test's
/// override on this thread, to compare the shapes. Always false otherwise.
fn generic_insertion() -> bool {
    #[cfg(test)]
    {
        tests::GENERIC_INSERTION.with(core::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}

/// The Delaunay complex of `points` (dimension `dim`): the lower hull of the
/// lift built by insertion, or the pulling triangulation as one group when
/// the lift is flat. Input checks and `DegenerateDimension` concern the
/// original sites, with their original indices.
pub(crate) fn complex(dim: usize, points: &[f64]) -> Result<Complex, ConvexHullError> {
    let input = accept(dim, points)?;
    let sites = insert::Sites::of(&input);
    let complex = if flat(&input, &sites)? {
        pull(input)?
    } else {
        // One insertion, compiled per shape (design §7, ADR 0005).
        match input.dim() {
            2 if !generic_insertion() => inserted(insert::Plane, &input, &sites)?,
            3 if !generic_insertion() => inserted(insert::Space, &input, &sites)?,
            d => inserted(insert::Any::of(d), &input, &sites)?,
        }
    };
    debug_assert!(
        {
            let mut seen: Vec<u32> = complex.sites.iter().flatten().copied().collect();
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
        if sites.lifted(&ids, Start::FirstStage)? != Sign::Zero {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The complex by insertion, its cospherical groups merged (design §8).
///
/// A simplex cospherical with none of its neighbors is a group by itself and
/// a cell as it stands in the mesh: its orientation is the parity of sorting
/// its vertices, since the mesh stores it positive, and its neighbors are
/// the mesh's. Only merged groups are split again by placing, and only their
/// faces are paired by vertex set.
///
/// The mesh is built on a copy of the sites in insertion order, so that
/// consecutive insertions read nearby rows: by input index they lie far
/// apart, and their loads were a quarter of D2 at 10^6 sites (#363, #366).
/// Its vertices are mapped back to the input's indices for the published
/// rows and groups.
fn inserted<S: insert::Shape>(
    shape: S,
    input: &Input<'_>,
    sites: &insert::Sites,
) -> Result<Complex, ConvexHullError> {
    let d = input.dim();
    let k = d + 1;
    let first = &input.spanning_points;
    let rest: Vec<u32> = input
        .representatives
        .iter()
        .copied()
        .filter(|p| !first.contains(p))
        .collect();
    let order = insert::brio(sites, &rest);
    let inserted_order: Vec<u32> = first.iter().chain(&order).copied().collect();
    let (local, local_first, local_order) = if stored_in_input_order() {
        let input_order: Vec<u32> = (0..input.representative.len() as u32).collect();
        (sites.permuted(&input_order), first.clone(), order)
    } else {
        let local_order = (k as u32..inserted_order.len() as u32).collect();
        (
            sites.permuted(&inserted_order),
            (0..k as u32).collect(),
            local_order,
        )
    };
    let mesh = insert::Mesh::build(shape, &local, &local_first, &local_order)?;
    let site = |v: u32| local.input_index(v);

    // Union of the finite simplices that share a face and are cospherical
    // across it: the far vertex of the neighbor has lifted orientation zero
    // against the simplex. The insertion decided this for every face it
    // linked to a simplex outside the cavity; the faces between simplices
    // it created together are tested here.
    let slots = mesh.slots();
    let mut parent: Vec<u32> = (0..slots as u32).collect();
    fn root(parent: &mut [u32], mut i: u32) -> u32 {
        while parent[i as usize] != i {
            parent[i as usize] = parent[parent[i as usize] as usize];
            i = parent[i as usize];
        }
        i
    }
    let mut ids: Vec<u32> = Vec::with_capacity(k + 1);
    let mut merged = false;
    // Each finite simplex numbered as a group by itself, in the order of
    // the walk; numbered again below when a group merged. A simplex at
    // infinity keeps `NO_NEIGHBOR`, so a link is one read of `draft_of`.
    let mut draft_of = vec![NO_NEIGHBOR; slots];
    let mut singles = 0_u32;
    for c in mesh.finite_cells() {
        draft_of[c as usize] = singles;
        singles += 1;
        for slot in 0..k {
            let n = mesh.neighbor(c, slot);
            if n < c {
                continue;
            }
            let across = mesh.across(c, slot);
            // The insertion records a face as distinct or cospherical only
            // between finite simplices; an unknown one may have a simplex at
            // infinity. A distinct face is skipped without reading `n`.
            if across == insert::Across::Unknown && !mesh.is_finite(n) {
                continue;
            }
            let mut test = || -> Result<bool, ConvexHullError> {
                // The far vertex of `n`: the one not in `c`. Found among the
                // vertices of `n`, which the finiteness check read, rather
                // than through the links of `n`.
                let vertices = mesh.vertices_of(c);
                let Some(&far) = mesh.vertices_of(n).iter().find(|v| !vertices.contains(v)) else {
                    debug_assert!(false, "neighbors differ in one vertex");
                    return Ok(false);
                };
                ids.clear();
                ids.extend_from_slice(vertices);
                ids.push(far);
                Ok(mesh.lifted_ids(&ids)? == Sign::Zero)
            };
            let cospherical = match across {
                insert::Across::Cospherical => true,
                insert::Across::Distinct => false,
                insert::Across::Unknown => test()?,
            };
            debug_assert!(
                across == insert::Across::Unknown || mesh.is_finite(n) && test()? == cospherical,
                "the insertion's record matches the lifted orientation"
            );
            if cospherical {
                let (x, y) = (root(&mut parent, c), root(&mut parent, n));
                parent[x as usize] = y;
                merged = true;
            }
        }
    }

    // A simplex alone in its group keeps its mesh links. When a group
    // merged, `parent` is flattened so that it names each simplex's root,
    // the singles are numbered again, and the others are marked `UNKNOWN`.
    if merged {
        let mut size = vec![0_u32; slots];
        for c in mesh.finite_cells() {
            let r = root(&mut parent, c);
            parent[c as usize] = r;
            size[r as usize] += 1;
        }
        singles = 0;
        for c in mesh.finite_cells() {
            draft_of[c as usize] = if size[parent[c as usize] as usize] == 1 {
                singles += 1;
                singles - 1
            } else {
                UNKNOWN
            };
        }
    }
    let mut draft = Draft::new(k, singles as usize);
    for c in mesh.finite_cells() {
        if draft_of[c as usize] == UNKNOWN {
            continue;
        }
        let vertices: Small<u32, 11> = mesh.vertices_of(c).iter().map(|&v| site(v)).collect();
        let (sorted, negative) = ascending(&vertices);
        let start = draft.rows.len();
        draft.rows.extend(sorted.iter().map(|&(v, _)| v));
        draft.links.extend(
            sorted
                .iter()
                .map(|&(_, slot)| draft_of[mesh.neighbor(c, slot) as usize]),
        );
        draft.negative.push(negative);
        draft.group.push(draft.sites.len() as u32);
        draft.sites.push(&draft.rows[start..]);
    }

    if merged {
        let mut members: Vec<(u32, u32)> = mesh
            .finite_cells()
            .filter(|&c| draft_of[c as usize] == UNKNOWN)
            .map(|c| (parent[c as usize], c))
            .collect();
        members.sort_unstable();
        let point = |i: u32| input.point(i);
        for run in members.chunk_by(|x, y| x.0 == y.0) {
            let mut group: Vec<u32> = run
                .iter()
                .flat_map(|&(_, c)| mesh.vertices_of(c).iter().map(|&v| site(v)))
                .collect();
            group.sort_unstable();
            group.dedup();
            let split = placing(d, point, &group)?;
            debug_assert!(
                split.iter().all(|s| s.len() == k),
                "a lower facet projects onto a full-dimensional region"
            );
            let cells = oriented(d, point, split)?;
            draft.push_group(&group, &cells);
        }
    }
    drop(mesh);
    Ok(draft.finish(d, input.representative.clone()))
}

/// The cells `split`, each as its ascending vertex list and whether that
/// order has negative orientation.
fn oriented<'p>(
    d: usize,
    point: impl Fn(u32) -> &'p [f64],
    split: Vec<Vec<u32>>,
) -> Result<Vec<(Vec<u32>, bool)>, ConvexHullError> {
    debug_assert!(split.iter().all(|s| s.len() == d + 1), "D + 1 sites a cell");
    split
        .into_iter()
        .map(|mut cell| {
            cell.sort_unstable();
            let points: Small<&[f64], 11> = cell.iter().map(|&v| point(v)).collect();
            let negative = orient(&points)? == Sign::Negative;
            Ok((cell, negative))
        })
        .collect()
}

/// The triangulation of `complex`: its cells in their order, with the last
/// two vertices of a negative cell swapped, and its neighbors moved with
/// them (design §7).
fn publish(complex: Complex) -> DelaunayTriangulation {
    let Complex {
        dim,
        representative,
        cells: mut vertices,
        mut neighbors,
        negative,
        ..
    } = complex;
    let k = dim + 1;
    for (c, _) in negative.iter().enumerate().filter(|(_, &n)| n) {
        vertices.swap(c * k + dim - 1, c * k + dim);
        neighbors.swap(c * k + dim - 1, c * k + dim);
    }
    DelaunayTriangulation {
        dim,
        representative,
        vertices,
        neighbors,
    }
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
fn pull(sites: Input<'_>) -> Result<Complex, ConvexHullError> {
    let d = sites.dim();
    let hull = SimplicialHull::build(sites)?;
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
    let cells = oriented(d, point, cells)?;
    let mut draft = Draft::new(d + 1, cells.len());
    draft.push_group(&input.representatives, &cells);
    Ok(draft.finish(d, input.representative.clone()))
}

#[cfg(test)]
mod tests;
