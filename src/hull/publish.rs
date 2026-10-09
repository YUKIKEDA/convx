//! The published convex hull (design §5, §9).

use core::cmp::Ordering;
use std::sync::OnceLock;

use super::classify::{classify, Classified, FaceOrder, Faces};
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
/// // Planes are computed by the first call of `planes()`.
/// assert_eq!(hull.planes()?.get(0).unwrap().normal(), [0.0, -1.0]);
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
    /// there, and [`ConvexHullError::ExactEvaluationExhausted`]. The planes
    /// of the facets are not computed here ([`ConvexHull::planes`]).
    pub fn build(self) -> Result<ConvexHull, ConvexHullError> {
        let input = accept(self.dim, self.points)?;
        let hull = publish(classify(input)?);
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
/// §9). For D = 2 the facets are the edges of the boundary cycle,
/// counterclockwise from the smallest vertex; otherwise they are ordered by
/// their vertex lists, lexicographically. A facet's position is its public
/// number.
///
/// The planes of the facets are computed by the first call of
/// [`ConvexHull::planes`] and kept, and so is the order of the boundary
/// complex, by the first call that reads it. For D = 1 and D >= 3 the
/// public numbering of the facets is computed by the first call that reads
/// a facet number; for D = 2 the facets are stored in public order (design
/// §5). Two hulls are equal when every published value but the planes is
/// equal; whether either had computed its planes, numbered its facets, or
/// ordered its complex does not enter. `Debug` computes none of them.
#[derive(Clone)]
pub struct ConvexHull {
    dim: usize,
    representative: Vec<u32>,
    vertices: Vec<u32>,
    coplanar_points: Vec<u32>,
    interior_points: Vec<u32>,
    /// Per facet, in stored order, its extreme points.
    facet_vertices: Lists<u32>,
    /// Per facet, in stored order, its neighboring facets by stored
    /// number, ascending.
    facet_neighbors: Lists<u32>,
    /// The public numbering of the stored facets: given at `build()` for
    /// D = 2, computed on first use otherwise.
    numbering: OnceLock<Numbering>,
    /// Boundary simplices, D vertices each in outward order (ascending,
    /// with the last two swapped when that is outward), flattened in the
    /// order classification left them.
    simplex_vertices: Vec<u32>,
    /// The stored facet of each boundary simplex.
    simplex_facets: Vec<u32>,
    /// The boundary simplices in public order, once something has read the
    /// complex in order ([`ConvexHull::triangulation`]).
    simplex_order: OnceLock<Vec<u32>>,
    /// Input coordinates of the extreme points, for `volume` and the planes.
    vertex_coordinates: Vec<f64>,
    /// The planes of the facets once [`ConvexHull::planes`] has computed
    /// them, or `None` when a plane is not finite, which the input decides.
    /// A failure the input does not decide is not kept.
    planes: OnceLock<Option<PlaneSet>>,
}

/// How the stored facets are numbered publicly (design §5).
#[derive(Clone, Debug)]
enum Numbering {
    /// The stored order is the public order (D = 2).
    Stored,
    /// The lexicographic order of the vertex lists (D = 1 and D >= 3).
    Renumbered {
        /// The stored facet of each public number.
        order: Vec<u32>,
        /// The public number of each stored facet.
        number: Vec<u32>,
        /// Per facet, in public order, its neighbors by public number,
        /// ascending.
        neighbors: Lists<u32>,
    },
}

impl Numbering {
    /// The stored facet of public number `facet`.
    fn stored(&self, facet: u32) -> usize {
        match self {
            Self::Stored => facet as usize,
            Self::Renumbered { order, .. } => order[facet as usize] as usize,
        }
    }

    /// The public number of stored facet `stored`.
    fn public(&self, stored: u32) -> u32 {
        match self {
            Self::Stored => stored,
            Self::Renumbered { number, .. } => number[stored as usize],
        }
    }
}

/// The planes of every facet, in public facet order.
#[derive(Clone, Debug)]
struct PlaneSet {
    /// D entries per facet: its outward unit normal.
    normals: Vec<f64>,
    /// Per facet, the offset of its plane.
    offsets: Vec<f64>,
}

impl PartialEq for ConvexHull {
    /// Compares every published value but the planes (design §9): the
    /// facets are compared in their public numbering and the boundary
    /// complex simplex by simplex in its public order, which numbers and
    /// orders them when they were not. The planes are a function of the
    /// facets and the vertex coordinates, and computing them can fail, so
    /// they do not enter.
    fn eq(&self, other: &Self) -> bool {
        self.dim == other.dim
            && self.representative == other.representative
            && self.vertices == other.vertices
            && self.coplanar_points == other.coplanar_points
            && self.interior_points == other.interior_points
            && self
                .facets()
                .iter()
                .map(|f| (f.vertices(), f.neighbors()))
                .eq(other.facets().iter().map(|f| (f.vertices(), f.neighbors())))
            && self.vertex_coordinates == other.vertex_coordinates
            && self.triangulation().iter().eq(other.triangulation().iter())
    }
}

impl core::fmt::Debug for ConvexHull {
    /// Prints what `build` computed, and the numbering, the order, and the
    /// planes only when they have been computed: formatting computes
    /// nothing.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConvexHull")
            .field("dim", &self.dim)
            .field("representative", &self.representative)
            .field("vertices", &self.vertices)
            .field("coplanar_points", &self.coplanar_points)
            .field("interior_points", &self.interior_points)
            .field("facet_vertices", &self.facet_vertices)
            .field("facet_neighbors", &self.facet_neighbors)
            .field("numbering", &self.numbering.get())
            .field("simplex_vertices", &self.simplex_vertices)
            .field("simplex_facets", &self.simplex_facets)
            .field("simplex_order", &self.simplex_order.get())
            .field("vertex_coordinates", &self.vertex_coordinates)
            .field("planes", &self.planes.get())
            .finish()
    }
}

/// The logical facets of a [`ConvexHull`], in public order.
#[derive(Clone, Copy)]
pub struct Facets<'a> {
    hull: &'a ConvexHull,
    numbering: &'a Numbering,
}

impl<'a> Facets<'a> {
    /// Number of facets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.hull.facet_vertices.len()
    }

    /// Whether there are no facets. Never true for a built hull.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hull.facet_vertices.is_empty()
    }

    /// The facet with public number `facet`, or `None` when no facet has
    /// that number.
    #[must_use]
    pub fn get(&self, facet: u32) -> Option<Facet<'a>> {
        ((facet as usize) < self.len()).then_some(Facet {
            hull: self.hull,
            numbering: self.numbering,
            index: facet,
        })
    }

    /// Every facet, in public number order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Facet<'a>> + 'a {
        let (hull, numbering) = (self.hull, self.numbering);
        (0..self.len() as u32).map(move |index| Facet {
            hull,
            numbering,
            index,
        })
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
    numbering: &'a Numbering,
    /// The public number.
    index: u32,
}

impl<'a> Facet<'a> {
    /// Extreme points of this face, ascending.
    #[must_use]
    pub fn vertices(&self) -> &'a [u32] {
        self.hull
            .facet_vertices
            .get(self.numbering.stored(self.index))
    }

    /// Numbers of the neighboring facets, ascending.
    #[must_use]
    pub fn neighbors(&self) -> &'a [u32] {
        match self.numbering {
            Numbering::Stored => self.hull.facet_neighbors.get(self.index as usize),
            Numbering::Renumbered { neighbors, .. } => neighbors.get(self.index as usize),
        }
    }
}

impl core::fmt::Debug for Facet<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Facet")
            .field("vertices", &self.vertices())
            .field("neighbors", &self.neighbors())
            .finish()
    }
}

/// The planes of the facets of a [`ConvexHull`], by public facet number.
#[derive(Clone, Copy)]
pub struct Planes<'a> {
    dim: usize,
    set: &'a PlaneSet,
}

impl<'a> Planes<'a> {
    /// Number of planes: one per facet.
    #[must_use]
    pub fn len(&self) -> usize {
        self.set.offsets.len()
    }

    /// Whether there are no planes. Never true for a built hull.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.set.offsets.is_empty()
    }

    /// The plane of the facet with public number `facet`, or `None` when no
    /// facet has that number.
    #[must_use]
    pub fn get(&self, facet: u32) -> Option<Plane<'a>> {
        ((facet as usize) < self.len()).then_some(Plane {
            dim: self.dim,
            set: self.set,
            index: facet,
        })
    }

    /// Every plane, in public facet number order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Plane<'a>> + 'a {
        let (dim, set) = (self.dim, self.set);
        (0..self.len() as u32).map(move |index| Plane { dim, set, index })
    }
}

impl core::fmt::Debug for Planes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// The supporting hyperplane of one facet: a view into its [`ConvexHull`].
/// Built only for the public result; topology never uses it.
#[derive(Clone, Copy)]
pub struct Plane<'a> {
    dim: usize,
    set: &'a PlaneSet,
    index: u32,
}

impl<'a> Plane<'a> {
    /// Outward unit normal of the hyperplane. Length D.
    #[must_use]
    pub fn normal(&self) -> &'a [f64] {
        let i = self.index as usize;
        &self.set.normals[i * self.dim..(i + 1) * self.dim]
    }

    /// Offset of the hyperplane `x . normal + offset = 0`.
    #[must_use]
    pub fn offset(&self) -> f64 {
        self.set.offsets[self.index as usize]
    }
}

impl core::fmt::Debug for Plane<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Plane")
            .field("normal", &self.normal())
            .field("offset", &self.offset())
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
#[derive(Clone, Copy)]
pub struct TriangulationView<'a> {
    hull: &'a ConvexHull,
    /// The simplices in public order.
    order: &'a [u32],
}

impl core::fmt::Debug for TriangulationView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
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
        self.order
            .get(index)
            .map(|&stored| self.hull.stored_simplex(stored as usize))
    }

    /// All simplices, in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = BoundarySimplex<'a>> + 'a {
        let hull = self.hull;
        self.order
            .iter()
            .map(move |&stored| hull.stored_simplex(stored as usize))
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
        let mut neighbors: Vec<Vec<u32>> = self
            .facets()
            .iter()
            .map(|f| f.neighbors().to_vec())
            .collect();
        edit(&mut self.vertices, &mut neighbors);
        let mut lists = Lists::default();
        for list in &neighbors {
            lists.push(list);
        }
        match self.numbering.get_mut() {
            Some(Numbering::Renumbered { neighbors, .. }) => *neighbors = lists,
            _ => self.facet_neighbors = lists,
        }
    }

    /// The logical facets (design §5), in public order.
    ///
    /// For D = 1 and D >= 3 the first call numbers the facets and keeps
    /// the numbering; for D = 2 they are stored in public order.
    #[must_use]
    pub fn facets(&self) -> Facets<'_> {
        Facets {
            hull: self,
            numbering: self.numbering(),
        }
    }

    /// The public numbering of the facets, computed on first use.
    fn numbering(&self) -> &Numbering {
        self.numbering.get_or_init(|| self.number_facets())
    }

    /// The lexicographic order of the stored vertex lists, ties by stored
    /// position, and the neighbor lists renumbered in it (design §5).
    fn number_facets(&self) -> Numbering {
        let n = self.facet_vertices.len();
        let order = lexicographic_order(n, |i| self.facet_vertices.get(i), |a, b| a.cmp(&b));
        let mut number = vec![0_u32; n];
        for (public, &stored) in order.iter().enumerate() {
            number[stored as usize] = public as u32;
        }
        let mut neighbors =
            Lists::with_capacity(n, self.facet_neighbors.iter().map(<[u32]>::len).sum());
        for (public, &stored) in order.iter().enumerate() {
            neighbors.push_iter(
                self.facet_neighbors
                    .get(stored as usize)
                    .iter()
                    .map(|&s| number[s as usize]),
            );
            neighbors.get_mut(public).sort_unstable();
        }
        Numbering::Renumbered {
            order,
            number,
            neighbors,
        }
    }

    /// The planes of the facets (design §5), by public facet number.
    ///
    /// The first call computes the plane of every facet and keeps them;
    /// later calls, from any thread, return what was kept.
    ///
    /// # Errors
    ///
    /// [`ConvexHullError::NonFiniteFacetPlane`] when a normal or an offset
    /// is not finite. The input decides that, so it is kept as well: every
    /// later call returns it, and no plane is published.
    ///
    /// [`ConvexHullError::ExactEvaluationExhausted`] when the work space of
    /// an exact evaluation could not be allocated. That is not kept: the
    /// next call computes the planes again.
    ///
    /// The hull itself is built either way.
    pub fn planes(&self) -> Result<Planes<'_>, ConvexHullError> {
        self.kept_planes(|| self.plane_set())
    }

    /// [`Self::planes`] with the computation as an argument: what is kept
    /// is a set of planes or the fact that a plane is not finite. Several
    /// threads may compute at once; one result is kept, and all return it.
    fn kept_planes(
        &self,
        compute: impl FnOnce() -> Result<PlaneSet, ConvexHullError>,
    ) -> Result<Planes<'_>, ConvexHullError> {
        let kept = match self.planes.get() {
            Some(kept) => kept,
            None => match compute() {
                Ok(set) => self.planes.get_or_init(|| Some(set)),
                Err(ConvexHullError::NonFiniteFacetPlane) => self.planes.get_or_init(|| None),
                Err(other) => return Err(other),
            },
        };
        match kept {
            Some(set) => Ok(Planes { dim: self.dim, set }),
            None => Err(ConvexHullError::NonFiniteFacetPlane),
        }
    }

    /// The plane of every facet, from the facets and the coordinates of the
    /// extreme points alone.
    fn plane_set(&self) -> Result<PlaneSet, ConvexHullError> {
        let d = self.dim;
        // Every facet vertex is an extreme point; its coordinates are read
        // through its position among the extreme points.
        let mut position = vec![0_u32; self.representative.len()];
        for (k, &v) in self.vertices.iter().enumerate() {
            position[v as usize] = k as u32;
        }
        let points = VertexPoints {
            dim: d,
            coordinates: &self.vertex_coordinates,
            position: &position,
        };
        let queries: Vec<(&[u32], u32)> = self
            .facets()
            .iter()
            .map(|facet| {
                let vertices = facet.vertices();
                (vertices, inner_reference(&self.vertices, vertices))
            })
            .collect();
        let mut normals = Vec::with_capacity(queries.len() * d);
        let mut offsets = Vec::with_capacity(queries.len());
        for_each_facet_normal(&points, &queries, |basis, normal| {
            offsets.push(plane_offset(&points, basis, normal)?);
            normals.extend_from_slice(normal);
            Ok(())
        })?;
        Ok(PlaneSet { normals, offsets })
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
    /// The simplices are put in their public order by the first call, and
    /// that order is kept. The split of a coplanar region is not part of
    /// the stability promise across versions.
    #[must_use]
    pub fn triangulation(&self) -> TriangulationView<'_> {
        TriangulationView {
            hull: self,
            order: self.simplex_order.get_or_init(|| self.ordered_simplices()),
        }
    }

    /// The boundary simplex stored at position `stored`, with the public
    /// number of its facet.
    fn stored_simplex(&self, stored: usize) -> BoundarySimplex<'_> {
        let d = self.dim;
        BoundarySimplex {
            vertices: &self.simplex_vertices[stored * d..(stored + 1) * d],
            facet: self.numbering().public(self.simplex_facets[stored]),
        }
    }

    /// The stored boundary simplices in public order: the lexicographic
    /// order of their ascending vertex lists, then the facet, then an even
    /// sort before an odd one. A stored list is ascending but for its last
    /// two vertices, which are swapped when the sort was odd.
    fn ordered_simplices(&self) -> Vec<u32> {
        let d = self.dim;
        let numbering = self.numbering();
        let facet = |i: usize| numbering.public(self.simplex_facets[i]);
        let row = |i: usize| &self.simplex_vertices[i * d..(i + 1) * d];
        // The ascending list of simplex `i` as its first D - 2 vertices and
        // its last two in order, and whether the stored two are swapped.
        let split = |i: usize| -> (&[u32], (u32, u32), bool) {
            let row = row(i);
            if d < 2 {
                return (row, (0, 0), false);
            }
            let (a, b) = (row[d - 2], row[d - 1]);
            (&row[..d - 2], (a.min(b), a.max(b)), a > b)
        };
        // The first two items order two lists whenever they differ, so the
        // lists are read only on equal keys (as `lexicographic_order`).
        let key = |i: usize| {
            let (head, (low, high), _) = split(i);
            let (first, second) = match (head.first(), head.get(1)) {
                (Some(&x), Some(&y)) => (x, y),
                (Some(&x), None) => (x, low),
                _ if d >= 2 => (low, high),
                _ => (head.first().copied().unwrap_or(0), 0),
            };
            u64::from(first) << 32 | u64::from(second)
        };
        let mut keyed: Vec<(u64, u32)> = (0..self.simplex_facets.len())
            .map(|i| (key(i), i as u32))
            .collect();
        keyed.sort_unstable_by(|&(ka, a), &(kb, b)| {
            let (a, b) = (a as usize, b as usize);
            ka.cmp(&kb).then_with(|| {
                let (head_a, tail_a, odd_a) = split(a);
                let (head_b, tail_b, odd_b) = split(b);
                (head_a, tail_a, facet(a), odd_a).cmp(&(head_b, tail_b, facet(b), odd_b))
            })
        });
        keyed.into_iter().map(|(_, i)| i).collect()
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
        if facet as usize >= self.facet_vertices.len() {
            return None;
        }
        let stored = self.numbering().stored(facet);
        let vertices = self.facet_vertices.get(stored);
        match self.dim {
            1 | 2 => Some(vertices.to_vec()),
            3 => {
                // Directed edges of the facet's outward triangles; interior
                // edges appear in both directions and cancel.
                let mut edges: Vec<(u32, u32)> = Vec::new();
                // Any order of the facet's triangles gives the same cycle,
                // so the stored order is read and nothing is sorted.
                let triangles = (0..self.simplex_facets.len())
                    .filter(|&i| self.simplex_facets[i] as usize == stored)
                    .map(|i| &self.simplex_vertices[i * 3..i * 3 + 3]);
                for v in triangles {
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

/// Coordinates by input index, for the plane of a facet: the accepted input
/// during a build, or the extreme points of a built hull.
pub(crate) trait Points {
    /// Dimension D.
    fn dim(&self) -> usize;
    /// The coordinates of point `index`.
    fn point(&self, index: u32) -> &[f64];
}

impl Points for Input<'_> {
    fn dim(&self) -> usize {
        Input::dim(self)
    }

    fn point(&self, index: u32) -> &[f64] {
        Input::point(self, index)
    }
}

/// The extreme points of a built hull, by input index.
struct VertexPoints<'a> {
    dim: usize,
    /// D coordinates per extreme point, in ascending index order.
    coordinates: &'a [f64],
    /// For an extreme point's input index, its position in that order.
    position: &'a [u32],
}

impl Points for VertexPoints<'_> {
    fn dim(&self) -> usize {
        self.dim
    }

    fn point(&self, index: u32) -> &[f64] {
        let start = self.position[index as usize] as usize * self.dim;
        &self.coordinates[start..start + self.dim]
    }
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
fn plane_offset(
    input: &impl Points,
    basis: &[u32],
    normal: &[f64],
) -> Result<f64, ConvexHullError> {
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
fn edge_unit_normal(input: &impl Points, vertices: &[u32], inner: u32) -> Option<Direction> {
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
    input: &impl Points,
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
    input: &impl Points,
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
fn facet_basis(input: &impl Points, facet_vertices: &[u32]) -> Result<Basis, ConvexHullError> {
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
    input: &impl Points,
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

/// Keeps the facets and simplices as classification left them. Their
/// public numbering is that order for D = 2 and is computed on first use
/// otherwise ([`ConvexHull::facets`]); the planes are left to
/// [`ConvexHull::planes`] (design §5).
fn publish(c: Classified<'_>) -> ConvexHull {
    let d = c.input.dim();
    let numbering = match c.order {
        FaceOrder::Public => OnceLock::from(Numbering::Stored),
        FaceOrder::Classification => OnceLock::new(),
    };
    let Faces {
        vertices: facet_vertices,
        neighbors: facet_neighbors,
    } = c.faces;

    // Boundary simplices: ascending vertex lists, then the last two swapped
    // where that makes the order outward. Every simplex of the complex is
    // in outward order, so the ascending list is outward exactly when the
    // sort is an even permutation: a transposition of two vertices reverses
    // the orientation sign. Their public order is left to the first reader
    // of the complex ([`ConvexHull::triangulation`]).
    let mut simplex_vertices = Vec::with_capacity(c.simplices.len() * d);
    let mut simplex_facets = Vec::with_capacity(c.simplices.len());
    for s in &c.simplices {
        debug_assert_eq!(s.vertices.len(), d);
        let start = simplex_vertices.len();
        simplex_vertices.extend_from_slice(&s.vertices);
        simplex_vertices[start..].sort_unstable();
        let facet = s.face;
        if d >= 2 {
            let odd = odd_permutation(&s.vertices);
            #[cfg(debug_assertions)]
            {
                let inner = inner_reference(&c.vertices, facet_vertices.get(facet as usize));
                let mut points: Vec<&[f64]> = simplex_vertices[start..]
                    .iter()
                    .map(|&v| c.input.point(v))
                    .collect();
                points.push(c.input.point(inner));
                debug_assert_eq!(
                    orient(&points).ok().map(|sign| sign == Sign::Positive),
                    Some(odd),
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
    ConvexHull {
        dim: d,
        representative: c.input.representative.clone(),
        vertices: c.vertices,
        coplanar_points: c.coplanar_points,
        interior_points: c.interior_points,
        facet_vertices,
        facet_neighbors,
        numbering,
        simplex_vertices,
        simplex_facets,
        simplex_order: OnceLock::new(),
        vertex_coordinates,
        planes: OnceLock::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hull::simplicial::tests::Rng;

    #[test]
    fn the_boundary_complex_is_ordered_on_first_use() {
        // Random points, and integer grids whose facets are not simplices:
        // the order the view returns is that of the tuples (ascending
        // vertex list, facet, parity of the sort), from an independent sort.
        let mut rng = Rng(293);
        for (dim, count, grid) in [
            (1, 9, false),
            (2, 60, false),
            (3, 200, false),
            (4, 80, false),
            (2, 40, true),
            (3, 120, true),
            (4, 200, true),
        ] {
            let points: Vec<f64> = (0..dim * count)
                .map(|_| {
                    if grid {
                        (rng.next() % 4) as f64
                    } else {
                        rng.unit()
                    }
                })
                .collect();
            let hull = ConvexHullBuilder::new(dim, &points).build().unwrap();
            // The invariant check of a debug build reads the complex; a
            // clone made before any read has not ordered it.
            let fresh = hull.clone();
            if !cfg!(debug_assertions) {
                assert!(fresh.simplex_order.get().is_none(), "build orders nothing");
            }
            let listed: Vec<(Vec<u32>, u32, bool)> = fresh
                .triangulation()
                .iter()
                .map(|s| {
                    let mut ascending = s.vertices.to_vec();
                    ascending.sort_unstable();
                    let odd = ascending != s.vertices;
                    (ascending, s.facet, odd)
                })
                .collect();
            assert!(fresh.simplex_order.get().is_some());
            let mut expected = listed.clone();
            expected.sort();
            assert_eq!(listed, expected, "D = {dim}, grid {grid}");
            assert_eq!(listed.len(), fresh.triangulation().len());
            // `get` follows the same order, and ends where the list ends.
            for (i, simplex) in fresh.triangulation().iter().enumerate() {
                assert_eq!(fresh.triangulation().get(i), Some(simplex));
            }
            assert_eq!(fresh.triangulation().get(listed.len()), None);
            // The cycle of a facet does not need the order.
            let unread = hull.clone();
            let cycles: Vec<_> = (0..unread.facets().len() as u32)
                .map(|f| unread.boundary_cycle(f))
                .collect();
            let read: Vec<_> = (0..fresh.facets().len() as u32)
                .map(|f| fresh.boundary_cycle(f))
                .collect();
            assert_eq!(cycles, read);
        }
    }

    /// A hull as `build` returns it, without the invariant check of debug
    /// builds, which reads the facets.
    fn published(dim: usize, points: &[f64]) -> ConvexHull {
        publish(classify(accept(dim, points).unwrap()).unwrap())
    }

    #[test]
    fn the_facets_are_numbered_on_first_use_above_d2() {
        // Random points and integer grids, whose facets are not simplices:
        // the public numbering is the lexicographic order of the vertex
        // lists, from an independent sort, with the neighbors renumbered.
        let mut rng = Rng(321);
        for (dim, count, grid) in [
            (1, 9, false),
            (3, 200, false),
            (4, 80, false),
            (3, 120, true),
            (4, 200, true),
        ] {
            let points: Vec<f64> = (0..dim * count)
                .map(|_| {
                    if grid {
                        (rng.next() % 4) as f64
                    } else {
                        rng.unit()
                    }
                })
                .collect();
            let hull = published(dim, &points);
            assert!(hull.numbering.get().is_none(), "build numbers nothing");
            // Formatting numbers nothing either.
            let _ = format!("{hull:?}");
            assert!(hull.numbering.get().is_none());
            let stored: Vec<Vec<u32>> = hull.facet_vertices.iter().map(<[u32]>::to_vec).collect();
            let mut expected = stored.clone();
            expected.sort();
            let public: Vec<Vec<u32>> = hull
                .facets()
                .iter()
                .map(|f| f.vertices().to_vec())
                .collect();
            assert_eq!(public, expected, "D = {dim}, grid {grid}");
            // A neighbor is the public number of a stored neighbor, and the
            // relation is symmetric.
            let number = |list: &[u32]| public.iter().position(|p| p == list).unwrap() as u32;
            for (s, list) in stored.iter().enumerate() {
                let mut renumbered: Vec<u32> = hull
                    .facet_neighbors
                    .get(s)
                    .iter()
                    .map(|&n| number(&stored[n as usize]))
                    .collect();
                renumbered.sort_unstable();
                let facet = hull.facets().get(number(list)).unwrap();
                assert_eq!(facet.neighbors(), renumbered);
                for &n in facet.neighbors() {
                    let other = hull.facets().get(n).unwrap();
                    assert!(other.neighbors().contains(&number(list)));
                }
            }
            // The facet of a simplex is a public number whose facet holds
            // the simplex's vertices.
            for simplex in hull.triangulation().iter() {
                let facet = hull.facets().get(simplex.facet).unwrap();
                assert!(simplex
                    .vertices
                    .iter()
                    .all(|v| facet.vertices().binary_search(v).is_ok()));
            }
            // Comparing numbers both sides, and numbering first or not
            // changes nothing; a clone copies the numbering made so far.
            let (a, b) = (published(dim, &points), published(dim, &points));
            let numbered = a.clone();
            let _ = numbered.facets();
            assert!(numbered.numbering.get().is_some());
            assert!(a == b && b == numbered && numbered == a);
            assert!(a.numbering.get().is_some() && b.numbering.get().is_some());
            assert!(numbered.clone().numbering.get().is_some());
        }
    }

    #[test]
    fn the_facets_of_d2_follow_the_cycle() {
        // The facets of D = 2 are stored in public order: the boundary
        // cycle, counterclockwise from the smallest vertex (design §5).
        let mut rng = Rng(3210);
        for (count, grid) in [(3, false), (60, false), (400, false), (40, true)] {
            let points: Vec<f64> = (0..2 * count)
                .map(|_| {
                    if grid {
                        (rng.next() % 4) as f64
                    } else {
                        rng.unit()
                    }
                })
                .collect();
            let hull = published(2, &points);
            assert!(matches!(hull.numbering.get(), Some(Numbering::Stored)));
            let n = hull.facets().len();
            assert_eq!(n, hull.vertices().len());
            // Walk the cycle: facet i ends where facet i + 1 starts, and
            // every turn is a strict left turn.
            let p = |v: u32| &points[v as usize * 2..v as usize * 2 + 2];
            let mut cycle = vec![hull.vertices()[0]];
            for i in 0..n {
                let facet = hull.facets().get(i as u32).unwrap();
                let here = *cycle.last().unwrap();
                let pair = facet.vertices();
                assert!(pair.contains(&here), "facet {i} starts at {here}");
                cycle.push(if pair[0] == here { pair[1] } else { pair[0] });
                let (prev, next) = ((i + n - 1) % n, (i + 1) % n);
                let mut expected = vec![prev as u32, next as u32];
                expected.sort_unstable();
                assert_eq!(facet.neighbors(), expected);
            }
            assert_eq!(cycle.first(), cycle.last(), "the cycle closes");
            for w in cycle
                .windows(3)
                .chain([[cycle[n - 1], cycle[0], cycle[1]].as_slice()])
            {
                let (a, b, c) = (p(w[0]), p(w[1]), p(w[2]));
                let turn = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                assert!(turn > 0.0, "counterclockwise at {:?}", w);
            }
            assert_eq!(cycle[0], *hull.vertices().iter().min().unwrap());
        }
    }

    #[test]
    fn a_failure_the_input_does_not_decide_is_not_kept() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let hull = ConvexHullBuilder::new(2, &points).build().unwrap();
        // Exhausted work space: returned, and nothing is kept.
        let exhausted = ConvexHullError::ExactEvaluationExhausted;
        assert_eq!(
            hull.kept_planes(|| Err(exhausted.clone())).err(),
            Some(exhausted)
        );
        assert!(hull.planes.get().is_none());
        // The next call computes the planes.
        assert_eq!(hull.planes().unwrap().len(), 4);
        // A plane that is not finite is kept: the computation does not run
        // again, and neither does one that would now succeed.
        let other = ConvexHullBuilder::new(2, &points).build().unwrap();
        let not_finite = ConvexHullError::NonFiniteFacetPlane;
        assert_eq!(
            other.kept_planes(|| Err(not_finite.clone())).err(),
            Some(not_finite.clone())
        );
        assert!(matches!(other.planes.get(), Some(None)));
        assert_eq!(other.planes().err(), Some(not_finite));
    }

    #[test]
    fn equality_compares_the_boundary_complex() {
        // Two hulls of one input are equal, whether or not the complex of
        // either has been ordered.
        let points = [
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
        ];
        let hull = ConvexHullBuilder::new(3, &points).build().unwrap();
        let ordered = ConvexHullBuilder::new(3, &points).build().unwrap();
        assert_eq!(ordered.triangulation().len(), 12);
        assert_eq!(hull, ordered);
        // A complex with two simplices of different facets exchanged is
        // another hull, although the facets and the partition are the same.
        let mut moved = ConvexHullBuilder::new(3, &points).build().unwrap();
        let other = (0..moved.simplex_facets.len())
            .find(|&i| moved.simplex_facets[i] != moved.simplex_facets[0])
            .unwrap();
        moved.simplex_facets.swap(0, other);
        moved.simplex_order = OnceLock::new();
        assert_ne!(hull, moved);
        // So is one with a simplex in the other orientation.
        let mut turned = ConvexHullBuilder::new(3, &points).build().unwrap();
        turned.simplex_vertices.swap(1, 2);
        turned.simplex_order = OnceLock::new();
        assert_ne!(hull, turned);
    }

    #[test]
    fn formatting_a_hull_computes_no_plane() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let hull = ConvexHullBuilder::new(2, &points).build().unwrap();
        assert!(hull.planes.get().is_none(), "build computes no plane");
        let before = format!("{hull:?}");
        assert!(hull.planes.get().is_none(), "Debug computes no plane");
        assert!(before.contains("planes: None"), "{before}");
        hull.planes().unwrap();
        let after = format!("{hull:?}");
        assert!(after.contains("normals: [0.0, -1.0"), "{after}");
    }

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
