//! The published convex hull (design §5, §9).

use core::cmp::Ordering;

use super::classify::{classify, Classified};
use super::input::{accept, minimum_basis, Input};
use super::ConvexHullError;
use crate::lists::Lists;
use crate::normal::{certified_side, facet_cofactors, facet_cofactors_in_lanes, working_normal};
use crate::predicates::{
    certified_cofactor_direction, orient, Cofactors, Direction, Sign, COFACTOR_LANES,
};
use crate::small::Small;

/// Builds a [`ConvexHull`] from row-major coordinates.
///
/// ```
/// use convx::ConvexHullBuilder;
///
/// // A unit square with its center.
/// let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
/// let hull = ConvexHullBuilder::new(2, &points).build()?;
/// assert_eq!(hull.vertices(), [0, 1, 2, 3]);
/// assert_eq!(hull.interior_points(), [4]);
/// assert_eq!(hull.facets().len(), 4);
/// let bottom = hull.facets().get(0).unwrap();
/// assert_eq!(bottom.vertices(), [0, 1]);
/// assert_eq!(bottom.normal(), [0.0, -1.0]);
/// assert_eq!(hull.volume(), 1.0);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ConvexHullBuilder<'a> {
    dim: usize,
    points: &'a [f64],
}

impl<'a> ConvexHullBuilder<'a> {
    /// A builder for points of dimension `dim`, stored row-major in `points`.
    #[must_use]
    pub fn new(dim: usize, points: &'a [f64]) -> Self {
        Self { dim, points }
    }

    /// Builds the hull.
    ///
    /// # Errors
    ///
    /// Any input failure of [`ConvexHullError`], in the order documented
    /// there, [`ConvexHullError::NonFiniteFacetPlane`] when a facet's public
    /// plane is not finite, and [`ConvexHullError::ExactEvaluationExhausted`].
    pub fn build(self) -> Result<ConvexHull, ConvexHullError> {
        let input = accept(self.dim, self.points)?;
        let hull = publish(classify(input)?)?;
        #[cfg(debug_assertions)]
        {
            if let Err(violation) = super::invariants::check(&hull, self.points) {
                debug_assert!(false, "convx hull invariant violated: {violation}");
            }
        }
        Ok(hull)
    }
}

/// A convex hull.
///
/// Every list is kept flat and published through borrowed views (design
/// §9). Facets are ordered by their vertex lists, lexicographically; a
/// facet's position is its public number.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexHull {
    dim: usize,
    representative: Vec<u32>,
    vertices: Vec<u32>,
    coplanar_points: Vec<u32>,
    interior_points: Vec<u32>,
    /// Per facet, in public order, its extreme points.
    facet_vertices: Lists<u32>,
    /// Per facet, its neighboring facets.
    facet_neighbors: Lists<u32>,
    /// D entries per facet: its outward unit normal.
    normals: Vec<f64>,
    /// Per facet, the offset of its plane.
    offsets: Vec<f64>,
    /// Boundary simplices, D vertices each, flattened in public order.
    simplex_vertices: Vec<u32>,
    /// The facet of each boundary simplex.
    simplex_facets: Vec<u32>,
    /// Input coordinates of the extreme points, for `volume`.
    vertex_coordinates: Vec<f64>,
}

/// The logical facets of a [`ConvexHull`], in public order.
#[derive(Clone, Copy)]
pub struct Facets<'a> {
    hull: &'a ConvexHull,
}

impl<'a> Facets<'a> {
    /// Number of facets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.hull.offsets.len()
    }

    /// Whether there are no facets. Never true for a built hull.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hull.offsets.is_empty()
    }

    /// The facet with public number `facet`, or `None` when no facet has
    /// that number.
    #[must_use]
    pub fn get(&self, facet: u32) -> Option<Facet<'a>> {
        ((facet as usize) < self.len()).then_some(Facet {
            hull: self.hull,
            index: facet,
        })
    }

    /// Every facet, in public number order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Facet<'a>> + 'a {
        let hull = self.hull;
        (0..self.len() as u32).map(move |index| Facet { hull, index })
    }
}

impl core::fmt::Debug for Facets<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A face of the hull of dimension D - 1: a view into its [`ConvexHull`].
#[derive(Clone, Copy)]
pub struct Facet<'a> {
    hull: &'a ConvexHull,
    index: u32,
}

impl<'a> Facet<'a> {
    /// Extreme points of this face, ascending.
    #[must_use]
    pub fn vertices(&self) -> &'a [u32] {
        self.hull.facet_vertices.get(self.index as usize)
    }

    /// Outward unit normal of the supporting hyperplane. Length D. Built
    /// only for the public result; topology never uses it.
    #[must_use]
    pub fn normal(&self) -> &'a [f64] {
        let d = self.hull.dim;
        let i = self.index as usize;
        &self.hull.normals[i * d..(i + 1) * d]
    }

    /// Offset of the supporting hyperplane `x . normal + offset = 0`.
    #[must_use]
    pub fn offset(&self) -> f64 {
        self.hull.offsets[self.index as usize]
    }

    /// Numbers of the neighboring facets, ascending.
    #[must_use]
    pub fn neighbors(&self) -> &'a [u32] {
        self.hull.facet_neighbors.get(self.index as usize)
    }
}

impl core::fmt::Debug for Facet<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Facet")
            .field("vertices", &self.vertices())
            .field("normal", &self.normal())
            .field("offset", &self.offset())
            .field("neighbors", &self.neighbors())
            .finish()
    }
}

/// A simplex of the boundary complex.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundarySimplex<'a> {
    /// D vertices in outward order: ascending, with the last two swapped when
    /// that is what makes the order outward. For D = 1 the single endpoint.
    pub vertices: &'a [u32],
    /// Public number of the logical facet that contains this simplex.
    pub facet: u32,
}

/// The boundary simplicial complex of a [`ConvexHull`].
///
/// Simplices are in the lexicographic order of their ascending vertex lists
/// before the swap, the order in which [`ConvexHull::volume`] adds terms.
#[derive(Clone, Copy, Debug)]
pub struct TriangulationView<'a> {
    hull: &'a ConvexHull,
}

impl<'a> TriangulationView<'a> {
    /// Number of boundary simplices.
    #[must_use]
    pub fn len(&self) -> usize {
        self.hull.simplex_facets.len()
    }

    /// Whether there are no boundary simplices. Never true for a built hull.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hull.simplex_facets.is_empty()
    }

    /// The simplex at `index`, or `None` past the end.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<BoundarySimplex<'a>> {
        let d = self.hull.dim;
        let vertices = self.hull.simplex_vertices.get(index * d..(index + 1) * d)?;
        let facet = *self.hull.simplex_facets.get(index)?;
        Some(BoundarySimplex { vertices, facet })
    }

    /// All simplices, in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = BoundarySimplex<'a>> + 'a {
        let hull = self.hull;
        let d = hull.dim;
        hull.simplex_vertices
            .chunks_exact(d)
            .zip(&hull.simplex_facets)
            .map(|(vertices, &facet)| BoundarySimplex { vertices, facet })
    }
}

impl ConvexHull {
    /// Dimension D.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// For each input index, the smallest index of a point equal to it.
    #[must_use]
    pub fn representative(&self) -> &[u32] {
        &self.representative
    }

    /// Extreme points, ascending.
    #[must_use]
    pub fn vertices(&self) -> &[u32] {
        &self.vertices
    }

    /// Boundary points that are not extreme, ascending.
    #[must_use]
    pub fn coplanar_points(&self) -> &[u32] {
        &self.coplanar_points
    }

    /// Interior points, ascending.
    #[must_use]
    pub fn interior_points(&self) -> &[u32] {
        &self.interior_points
    }

    /// Rewrites the extreme points and the facet neighbor lists through
    /// owned copies, for the tests that break an invariant on purpose.
    #[cfg(test)]
    pub(crate) fn edit_lists(&mut self, edit: impl FnOnce(&mut Vec<u32>, &mut Vec<Vec<u32>>)) {
        let mut neighbors: Vec<Vec<u32>> =
            self.facet_neighbors.iter().map(<[u32]>::to_vec).collect();
        edit(&mut self.vertices, &mut neighbors);
        let mut lists = Lists::default();
        for list in &neighbors {
            lists.push(list);
        }
        self.facet_neighbors = lists;
    }

    /// The logical facets (design §5), in public order.
    #[must_use]
    pub fn facets(&self) -> Facets<'_> {
        Facets { hull: self }
    }

    /// The volume of the polytope. Not used for topology. Finiteness is not
    /// guaranteed.
    ///
    /// For D = 1 it is the length of the segment. For D >= 2 each boundary
    /// simplex, in outward order, contributes its signed `f64` volume with
    /// the extreme point of smallest index. Terms are added from the left in
    /// the order of [`ConvexHull::triangulation`], and the absolute value is
    /// returned.
    #[must_use]
    pub fn volume(&self) -> f64 {
        let d = self.dim;
        let coordinates = |v: u32| -> &[f64] {
            let k = self.vertices.binary_search(&v).unwrap_or(0);
            &self.vertex_coordinates[k * d..(k + 1) * d]
        };
        if d == 1 {
            let xs: Vec<f64> = self.vertices.iter().map(|&v| coordinates(v)[0]).collect();
            let low = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let high = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            return (high - low).abs();
        }
        let Some(&apex) = self.vertices.first() else {
            return 0.0;
        };
        let r = coordinates(apex);
        let mut factorial = 1.0;
        for k in 2..=d {
            factorial *= k as f64;
        }
        let mut total = 0.0;
        for simplex in self.triangulation().iter() {
            let rows: Vec<Vec<f64>> = simplex
                .vertices
                .iter()
                .map(|&v| coordinates(v).iter().zip(r).map(|(x, o)| x - o).collect())
                .collect();
            total += determinant(rows) / factorial;
        }
        total.abs()
    }

    /// The boundary simplicial complex. Each simplex has D vertices.
    ///
    /// The split of a coplanar region is not part of the stability promise
    /// across versions.
    #[must_use]
    pub fn triangulation(&self) -> TriangulationView<'_> {
        TriangulationView { hull: self }
    }

    /// The boundary cycle of facet number `facet`, derived from its vertices
    /// and outward normal. Defined only when D is 1, 2, or 3.
    ///
    /// For D = 1 the single endpoint, for D = 2 the two endpoints, and for
    /// D = 3 the polygon starting at its smallest vertex, counterclockwise
    /// seen from outside. Returns `None` for D >= 4 and for a number that is
    /// not a facet.
    #[must_use]
    pub fn boundary_cycle(&self, facet: u32) -> Option<Vec<u32>> {
        let vertices = self.facet_vertices.try_get(facet as usize)?;
        match self.dim {
            1 | 2 => Some(vertices.to_vec()),
            3 => {
                // Directed edges of the facet's outward triangles; interior
                // edges appear in both directions and cancel.
                let mut edges: Vec<(u32, u32)> = Vec::new();
                for simplex in self.triangulation().iter().filter(|s| s.facet == facet) {
                    let v = simplex.vertices;
                    for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                        if let Some(k) = edges.iter().position(|&e| e == (b, a)) {
                            edges.swap_remove(k);
                        } else {
                            edges.push((a, b));
                        }
                    }
                }
                let start = *vertices.first()?;
                let mut cycle = vec![start];
                let mut current = start;
                while cycle.len() < edges.len() {
                    let &(_, next) = edges.iter().find(|&&(a, _)| a == current)?;
                    cycle.push(next);
                    current = next;
                }
                Some(cycle)
            }
            _ => None,
        }
    }
}

/// `f64` determinant by Gaussian elimination with partial pivoting.
fn determinant(mut m: Vec<Vec<f64>>) -> f64 {
    let n = m.len();
    let mut det = 1.0;
    for col in 0..n {
        let Some(pivot) = (col..n).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))
        else {
            return 0.0;
        };
        if pivot != col {
            m.swap(pivot, col);
            det = -det;
        }
        let p = m[col][col];
        if p == 0.0 {
            return 0.0;
        }
        det *= p;
        let (upper, lower) = m.split_at_mut(col + 1);
        for row in lower {
            let factor = row[col] / p;
            for (x, &y) in row[col + 1..].iter_mut().zip(&upper[col][col + 1..]) {
                *x -= factor * y;
            }
        }
    }
    det
}

/// The numbers `0..n` ordered by `list` lexicographically, equal lists by
/// `ties`.
///
/// A key of the first two items, a missing item read as zero, orders two
/// lists whenever the keys differ, so the lists are read only on equal keys.
fn lexicographic_order<'a>(
    n: usize,
    list: impl Fn(usize) -> &'a [u32],
    ties: impl Fn(usize, usize) -> Ordering,
) -> Vec<u32> {
    let key = |items: &[u32]| {
        let item = |k: usize| u64::from(items.get(k).copied().unwrap_or(0));
        item(0) << 32 | item(1)
    };
    let mut keyed: Vec<(u64, u32)> = (0..n).map(|i| (key(list(i)), i as u32)).collect();
    keyed.sort_unstable_by(|&(ka, a), &(kb, b)| {
        let (a, b) = (a as usize, b as usize);
        ka.cmp(&kb)
            .then_with(|| list(a).cmp(list(b)))
            .then_with(|| ties(a, b))
    });
    keyed.into_iter().map(|(_, i)| i).collect()
}

/// Whether sorting `vertices` (distinct) ascending is an odd permutation:
/// the parity of its inversions.
fn odd_permutation(vertices: &[u32]) -> bool {
    let mut odd = false;
    for (i, a) in vertices.iter().enumerate() {
        for b in &vertices[i + 1..] {
            odd ^= a > b;
        }
    }
    odd
}

/// A point strictly inside relative to `facet_vertices`: the smallest hull
/// vertex that is not on that facet.
pub(crate) fn inner_reference(vertices: &[u32], facet_vertices: &[u32]) -> u32 {
    vertices
        .iter()
        .copied()
        .find(|v| facet_vertices.binary_search(v).is_err())
        .unwrap_or(vertices[0])
}

/// The offset of the public plane of a facet (design §5) from its
/// [`for_each_facet_normal`] basis and normal: the plane passes through the
/// first basis point `r`, so the offset is `-n . r`.
fn plane_offset(input: &Input<'_>, basis: &[u32], normal: &[f64]) -> Result<f64, ConvexHullError> {
    let offset = -normal
        .iter()
        .zip(input.point(basis[0]))
        .map(|(n, x)| n * x)
        .sum::<f64>();
    if !offset.is_finite() || normal.iter().any(|x| !x.is_finite()) {
        return Err(ConvexHullError::NonFiniteFacetPlane);
    }
    Ok(offset)
}

/// The outward unit normal of one edge, when the two-point cofactors certify
/// it. `None` leaves the facet on the general path.
fn edge_unit_normal(input: &Input<'_>, vertices: &[u32], inner: u32) -> Option<Direction> {
    let a = input.point(vertices[0]);
    let b = input.point(vertices[1]);
    let cofactors = crate::predicates::two_point_cofactors(a, b)?;
    let (direction, _) = certified_cofactor_direction(2, Some(&cofactors))?;
    let side = certified_side(&[a, b], &cofactors, input.point(inner))?;
    if side == Sign::Zero {
        return None;
    }
    #[cfg(debug_assertions)]
    debug_assert_eq!(orient(&[a, b, input.point(inner)]).ok(), Some(side));
    let mut normal = [direction[0], direction[1]];
    if side == Sign::Positive {
        normal[0] = -normal[0];
        normal[1] = -normal[1];
    }
    Some(normal.as_slice().into())
}

/// [`for_each_facet_normal`] for edges. A facet the filter does not certify
/// takes the general path.
fn edge_normals(
    input: &Input<'_>,
    facets: &[(&[u32], u32)],
    mut f: impl FnMut(&[u32], &[f64]) -> Result<(), ConvexHullError>,
) -> Result<(), ConvexHullError> {
    for &(vertices, inner) in facets {
        let Some(normal) = edge_unit_normal(input, vertices, inner) else {
            let basis = facet_basis(input, vertices)?;
            let points: BasisPoints<'_> = basis.iter().map(|&v| input.point(v)).collect();
            let cofactors = facet_cofactors(&points);
            let normal = oriented_normal(input, &points, inner, cofactors.as_deref())?;
            f(&basis, &normal)?;
            continue;
        };
        f(vertices, &normal)?;
    }
    Ok(())
}

/// The outward unit normal of each facet (design §5), with the basis it is
/// built on: the first D affinely independent vertices in lexicographic
/// order, oriented by the exact sign so that the facet's inner point is
/// inside. `facets` holds each facet's vertices and inner point; `f`
/// receives the basis and the normal of each, in order, and the first error
/// of a facet or of `f` ends the walk. The cofactors of four bases at a
/// time share one elimination in lanes, bit for bit those of each alone.
/// No facet allocates: the normal is lent to `f` from the stack.
pub(crate) fn for_each_facet_normal(
    input: &Input<'_>,
    facets: &[(&[u32], u32)],
    mut f: impl FnMut(&[u32], &[f64]) -> Result<(), ConvexHullError>,
) -> Result<(), ConvexHullError> {
    if input.dim() == 2 && facets.iter().all(|(vertices, _)| vertices.len() == 2) {
        return edge_normals(input, facets, f);
    }
    let point = |i: u32| input.point(i);
    for chunk in facets.chunks(COFACTOR_LANES) {
        let bases: [Result<Basis, ConvexHullError>; COFACTOR_LANES] =
            core::array::from_fn(|lane| match chunk.get(lane) {
                Some(&(vertices, _)) => facet_basis(input, vertices),
                None => Ok(Basis::new()),
            });
        let points: [BasisPoints<'_>; COFACTOR_LANES] =
            core::array::from_fn(|lane| match &bases[lane] {
                Ok(basis) => basis.iter().map(|&v| point(v)).collect(),
                Err(_) => BasisPoints::new(),
            });
        let cofactors: [Option<Cofactors>; COFACTOR_LANES] =
            if chunk.len() == COFACTOR_LANES && bases.iter().all(Result::is_ok) {
                facet_cofactors_in_lanes(core::array::from_fn(|lane| &*points[lane]))
            } else {
                core::array::from_fn(|lane| {
                    let basis = lane < chunk.len() && bases[lane].is_ok();
                    basis.then(|| facet_cofactors(&points[lane])).flatten()
                })
            };
        for (((&(_, inner), basis), points), cofactors) in
            chunk.iter().zip(bases).zip(&points).zip(&cofactors)
        {
            let basis = basis?;
            let normal = oriented_normal(input, points, inner, cofactors.as_deref())?;
            f(&basis, &normal)?;
        }
    }
    Ok(())
}

/// A facet basis: D vertex numbers, inline up to the lane range.
type Basis = Small<u32, 10>;

/// The points of a [`Basis`], with room for the inner point.
type BasisPoints<'a> = Small<&'a [f64], 11>;

/// The first D affinely independent vertices of a facet in lexicographic
/// order.
fn facet_basis(input: &Input<'_>, facet_vertices: &[u32]) -> Result<Basis, ConvexHullError> {
    let d = input.dim();
    let point = |i: u32| input.point(i);
    // A facet with exactly D vertices spans its (D - 1)-flat, so they are
    // affinely independent and the walk of `minimum_basis` takes them all.
    Ok(if facet_vertices.len() == d {
        debug_assert_eq!(minimum_basis(d, facet_vertices, point)?, facet_vertices);
        Basis::from(facet_vertices)
    } else {
        minimum_basis(d, facet_vertices, point)?
            .into_iter()
            .take(d)
            .collect()
    })
}

/// The unit normal of the hyperplane through the basis `points`, oriented
/// by the exact sign so that `inner` is inside. `cofactors` are the
/// [`facet_cofactors`] of `points`.
fn oriented_normal(
    input: &Input<'_>,
    points: &[&[f64]],
    inner: u32,
    cofactors: Option<&[(f64, f64)]>,
) -> Result<Direction, ConvexHullError> {
    let point = |i: u32| input.point(i);
    let with_inner =
        || -> BasisPoints<'_> { points.iter().copied().chain([point(inner)]).collect() };
    // The cofactors certify the normal and usually prove the side of
    // `inner` too, without the orientation determinant.
    let proved = cofactors.and_then(|c| certified_side(points, c, point(inner)));
    let inside = match proved {
        Some(sign) => {
            debug_assert_eq!(
                sign,
                orient(&with_inner())?,
                "the cofactors proved the wrong side"
            );
            sign
        }
        None => orient(&with_inner())?,
    };
    // The published normal is the working normal, the certified cofactor
    // direction (design §1).
    working_normal(points, inside.reversed(), cofactors)?
        .ok_or(ConvexHullError::NonFiniteFacetPlane)
}

/// Numbers facets and simplices in the public order and builds the planes.
fn publish(c: Classified<'_>) -> Result<ConvexHull, ConvexHullError> {
    let d = c.input.dim();

    // Facets ordered by vertex list.
    let order = lexicographic_order(c.faces.len(), |i| c.faces.vertices.get(i), |a, b| a.cmp(&b));
    let mut number = vec![0_u32; c.faces.len()];
    for (public, &internal) in order.iter().enumerate() {
        number[internal as usize] = public as u32;
    }

    // Every list of the result is written flat, in public order, so no
    // facet allocates (#254).
    let facet_count = order.len();
    let mut facet_vertices =
        Lists::with_capacity(facet_count, c.faces.vertices.iter().map(<[u32]>::len).sum());
    let mut facet_neighbors = Lists::with_capacity(
        facet_count,
        c.faces.neighbors.iter().map(<[u32]>::len).sum(),
    );
    for (public, &internal) in order.iter().enumerate() {
        let internal = internal as usize;
        facet_vertices.push(c.faces.vertices.get(internal));
        facet_neighbors.push_iter(
            c.faces
                .neighbors
                .get(internal)
                .iter()
                .map(|&n| number[n as usize]),
        );
        facet_neighbors.get_mut(public).sort_unstable();
    }
    let queries: Vec<(&[u32], u32)> = facet_vertices
        .iter()
        .map(|vertices| (vertices, inner_reference(&c.vertices, vertices)))
        .collect();
    let mut normals = Vec::with_capacity(facet_count * d);
    let mut offsets = Vec::with_capacity(facet_count);
    for_each_facet_normal(&c.input, &queries, |basis, normal| {
        offsets.push(plane_offset(&c.input, basis, normal)?);
        normals.extend_from_slice(normal);
        Ok(())
    })?;
    drop(queries);

    // Boundary simplices: ascending vertex lists, ordered lexicographically,
    // then the last two swapped where that makes the order outward. Every
    // simplex of the complex is in outward order, so the ascending list is
    // outward exactly when the sort is an even permutation: a transposition
    // of two vertices reverses the orientation sign.
    let mut sorted = Vec::with_capacity(c.simplices.len() * d);
    let mut simplex_number = Vec::with_capacity(c.simplices.len());
    let mut simplex_odd = Vec::with_capacity(c.simplices.len());
    for s in &c.simplices {
        debug_assert_eq!(s.vertices.len(), d);
        let start = sorted.len();
        sorted.extend_from_slice(&s.vertices);
        sorted[start..].sort_unstable();
        simplex_number.push(number[s.face as usize]);
        simplex_odd.push(odd_permutation(&s.vertices));
    }
    let row = |i: usize| &sorted[i * d..(i + 1) * d];
    // Equal vertex lists then go by facet and parity, the order of the
    // tuples `(sorted, facet, odd)`.
    let simplex_order = lexicographic_order(c.simplices.len(), row, |a, b| {
        (simplex_number[a], simplex_odd[a]).cmp(&(simplex_number[b], simplex_odd[b]))
    });
    let mut simplex_vertices = Vec::with_capacity(sorted.len());
    let mut simplex_facets = Vec::with_capacity(simplex_order.len());
    for &i in &simplex_order {
        let i = i as usize;
        let (facet, odd) = (simplex_number[i], simplex_odd[i]);
        let start = simplex_vertices.len();
        simplex_vertices.extend_from_slice(row(i));
        if d >= 2 {
            #[cfg(debug_assertions)]
            {
                let inner = inner_reference(&c.vertices, facet_vertices.get(facet as usize));
                let mut points: Vec<&[f64]> = row(i).iter().map(|&v| c.input.point(v)).collect();
                points.push(c.input.point(inner));
                debug_assert_eq!(
                    orient(&points)? == Sign::Positive,
                    odd,
                    "the parity of the sort decides the outward order"
                );
            }
            if odd {
                simplex_vertices.swap(start + d - 2, start + d - 1);
            }
        }
        simplex_facets.push(facet);
    }

    let vertex_coordinates: Vec<f64> = c
        .vertices
        .iter()
        .flat_map(|&v| c.input.point(v).iter().copied())
        .collect();
    Ok(ConvexHull {
        dim: d,
        representative: c.input.representative.clone(),
        vertices: c.vertices,
        coplanar_points: c.coplanar_points,
        interior_points: c.interior_points,
        facet_vertices,
        facet_neighbors,
        normals,
        offsets,
        simplex_vertices,
        simplex_facets,
        vertex_coordinates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hull::simplicial::tests::Rng;

    #[test]
    fn lexicographic_order_is_the_order_of_the_lists_then_the_ties() {
        let mut rng = Rng(151);
        let alphabet = [0, 1, 2, u32::MAX - 1, u32::MAX];
        for trial in 0..400 {
            // Short alphabets and a shared stem make long common prefixes,
            // lists that are prefixes of others, and equal lists.
            let stem: Vec<u32> = (0..rng.next() % 4)
                .map(|_| alphabet[(rng.next() % 5) as usize])
                .collect();
            let n = 1 + (rng.next() % 40) as usize;
            let lists: Vec<Vec<u32>> = (0..n)
                .map(|_| {
                    let mut list = if rng.next().is_multiple_of(2) {
                        stem.clone()
                    } else {
                        Vec::new()
                    };
                    let extra = rng.next() % 4;
                    list.extend((0..extra).map(|_| alphabet[(rng.next() % 5) as usize]));
                    list
                })
                .collect();
            let tag: Vec<u64> = (0..n).map(|_| rng.next() % 3).collect();
            let order = lexicographic_order(
                n,
                |i| lists[i].as_slice(),
                |a, b| tag[a].cmp(&tag[b]).then(b.cmp(&a)),
            );
            let mut expected: Vec<u32> = (0..n as u32).collect();
            expected.sort_by(|&a, &b| {
                let (a, b) = (a as usize, b as usize);
                (&lists[a], tag[a], core::cmp::Reverse(a)).cmp(&(
                    &lists[b],
                    tag[b],
                    core::cmp::Reverse(b),
                ))
            });
            assert_eq!(order, expected, "trial {trial}: {lists:?} {tag:?}");
        }
    }
}
