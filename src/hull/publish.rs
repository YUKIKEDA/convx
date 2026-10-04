//! The published convex hull (design §5, §9).

use super::classify::{classify, Classified};
use super::input::{accept, minimum_basis, Input};
use super::simplicial::Execution;
use super::ConvexHullError;
use crate::normal::{certified_side, facet_cofactors, facet_cofactors_in_lanes, unit_normal_with};
#[cfg(debug_assertions)]
use crate::predicates::Sign;
use crate::predicates::{orient, Cofactors, COFACTOR_LANES};

/// Builds a [`ConvexHull`] from row-major coordinates.
///
/// ```
/// use convx::ConvexHullBuilder;
///
/// // A unit square with its center.
/// let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.5];
/// let hull = ConvexHullBuilder::new(2, &points).build()?;
/// assert_eq!(hull.vertices, vec![0, 1, 2, 3]);
/// assert_eq!(hull.interior_points, vec![4]);
/// assert_eq!(hull.facets.len(), 4);
/// assert_eq!(hull.volume(), 1.0);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ConvexHullBuilder<'a> {
    dim: usize,
    points: &'a [f64],
    execution: Execution,
}

impl<'a> ConvexHullBuilder<'a> {
    /// A builder for points of dimension `dim`, stored row-major in `points`.
    #[must_use]
    pub fn new(dim: usize, points: &'a [f64]) -> Self {
        Self {
            dim,
            points,
            execution: Execution::Sequential,
        }
    }

    /// Plans the points of each batch on rayon's global pool when `enable`
    /// is true. Off by default.
    ///
    /// The batches and the commit order are the same either way, so the
    /// result is identical to the sequential build: the same logical facets,
    /// the same triangulation, and the same planes. The number of threads is
    /// rayon's, configured by the caller through rayon.
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

    /// Builds the hull.
    ///
    /// # Errors
    ///
    /// Any input failure of [`ConvexHullError`], in the order documented
    /// there, [`ConvexHullError::NonFiniteFacetPlane`] when a facet's public
    /// plane is not finite, and [`ConvexHullError::ExactEvaluationExhausted`].
    pub fn build(self) -> Result<ConvexHull, ConvexHullError> {
        let input = accept(self.dim, self.points)?;
        let hull = publish(classify(input, self.execution)?)?;
        #[cfg(debug_assertions)]
        if let Err(violation) = super::invariants::check(&hull, self.points) {
            debug_assert!(false, "convx hull invariant violated: {violation}");
        }
        Ok(hull)
    }
}

/// The supporting hyperplane `x . normal + offset = 0` of a facet.
///
/// Built only for the public result; topology never uses it.
#[derive(Clone, Debug, PartialEq)]
pub struct FacetPlane {
    /// Outward unit normal. Length D.
    pub normal: Vec<f64>,
    /// Offset of the plane `x . normal + offset = 0`.
    pub offset: f64,
}

/// A face of the hull of dimension D - 1.
#[derive(Clone, Debug, PartialEq)]
pub struct LogicalFacet {
    /// Extreme points of this face, ascending.
    pub vertices: Vec<u32>,
    /// The supporting hyperplane.
    pub plane: FacetPlane,
    /// Numbers of the neighboring facets, ascending.
    pub neighbors: Vec<u32>,
}

/// A convex hull.
///
/// Facets are ordered by their vertex lists, lexicographically; a facet's
/// position is its public number.
#[derive(Clone, Debug, PartialEq)]
pub struct ConvexHull {
    /// Dimension D.
    pub dim: usize,
    /// For each input index, the smallest index of a point equal to it.
    pub representative: Vec<u32>,
    /// Extreme points, ascending.
    pub vertices: Vec<u32>,
    /// Boundary points that are not extreme, ascending.
    pub coplanar_points: Vec<u32>,
    /// Interior points, ascending.
    pub interior_points: Vec<u32>,
    /// The logical facets.
    pub facets: Vec<LogicalFacet>,
    /// Boundary simplices, D vertices each, flattened in public order.
    simplex_vertices: Vec<u32>,
    /// The facet of each boundary simplex.
    simplex_facets: Vec<u32>,
    /// Input coordinates of the extreme points, for `volume`.
    vertex_coordinates: Vec<f64>,
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
        let face = self.facets.get(facet as usize)?;
        match self.dim {
            1 | 2 => Some(face.vertices.clone()),
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
                let start = *face.vertices.first()?;
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

/// The public plane of a facet (design §5) from its [`for_each_facet_normal`]
/// basis and normal: the first D affinely independent vertices in
/// lexicographic order, translated, scaled, normal by QR, made unit, and
/// oriented by the exact sign so that the inside is negative.
fn facet_plane(
    input: &Input<'_>,
    basis: &[u32],
    normal: Vec<f64>,
) -> Result<FacetPlane, ConvexHullError> {
    let point = |i: u32| input.point(i);
    // The plane passes through the first basis point r: offset = -n . r.
    let offset = -normal
        .iter()
        .zip(point(basis[0]))
        .map(|(n, x)| n * x)
        .sum::<f64>();
    if !offset.is_finite() || normal.iter().any(|x| !x.is_finite()) {
        return Err(ConvexHullError::NonFiniteFacetPlane);
    }
    Ok(FacetPlane { normal, offset })
}

/// The outward unit normal of each facet (design §5), with the basis it is
/// built on: the first D affinely independent vertices in lexicographic
/// order, oriented by the exact sign so that the facet's inner point is
/// inside. `facets` holds each facet's vertices and inner point; `f`
/// receives the basis and the normal of each, in order, and the first error
/// of a facet or of `f` ends the walk. The cofactors of four bases at a
/// time share one elimination in lanes, bit for bit those of each alone.
pub(crate) fn for_each_facet_normal(
    input: &Input<'_>,
    facets: &[(&[u32], u32)],
    mut f: impl FnMut(Vec<u32>, Vec<f64>) -> Result<(), ConvexHullError>,
) -> Result<(), ConvexHullError> {
    let point = |i: u32| input.point(i);
    for chunk in facets.chunks(COFACTOR_LANES) {
        let bases: Vec<Result<Vec<u32>, ConvexHullError>> = chunk
            .iter()
            .map(|&(vertices, _)| facet_basis(input, vertices))
            .collect();
        let points: Vec<Vec<&[f64]>> = bases
            .iter()
            .map(|basis| basis.iter().flatten().map(|&v| point(v)).collect())
            .collect();
        let cofactors: Vec<Option<Cofactors>> =
            if chunk.len() == COFACTOR_LANES && bases.iter().all(Result::is_ok) {
                facet_cofactors_in_lanes(core::array::from_fn(|lane| &*points[lane])).to_vec()
            } else {
                points.iter().map(|p| facet_cofactors(p)).collect()
            };
        for (((&(_, inner), basis), points), cofactors) in
            chunk.iter().zip(bases).zip(&points).zip(cofactors)
        {
            let basis = basis?;
            let normal = oriented_normal(input, points, inner, cofactors)?;
            f(basis, normal)?;
        }
    }
    Ok(())
}

/// The first D affinely independent vertices of a facet in lexicographic
/// order.
fn facet_basis(input: &Input<'_>, facet_vertices: &[u32]) -> Result<Vec<u32>, ConvexHullError> {
    let d = input.dim();
    let point = |i: u32| input.point(i);
    // A facet with exactly D vertices spans its (D - 1)-flat, so they are
    // affinely independent and the walk of `minimum_basis` takes them all.
    Ok(if facet_vertices.len() == d {
        debug_assert_eq!(minimum_basis(d, facet_vertices, point)?, facet_vertices);
        facet_vertices.to_vec()
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
    cofactors: Option<Cofactors>,
) -> Result<Vec<f64>, ConvexHullError> {
    let point = |i: u32| input.point(i);
    let mut points = points.to_vec();
    // The cofactors certify the normal and usually prove the side of
    // `inner` too, without the orientation determinant.
    let proved = cofactors
        .as_deref()
        .and_then(|c| certified_side(&points, c, point(inner)));
    points.push(point(inner));
    let inside = match proved {
        Some(sign) => {
            debug_assert_eq!(
                sign,
                orient(&points)?,
                "the cofactors proved the wrong side"
            );
            sign
        }
        None => orient(&points)?,
    };
    points.pop();
    unit_normal_with(&points, inside.reversed(), cofactors.as_deref())?
        .ok_or(ConvexHullError::NonFiniteFacetPlane)
}

/// Numbers facets and simplices in the public order and builds the planes.
fn publish(c: Classified<'_>) -> Result<ConvexHull, ConvexHullError> {
    let d = c.input.dim();

    // Facets ordered by vertex list.
    let mut order: Vec<usize> = (0..c.faces.len()).collect();
    order.sort_by(|&a, &b| c.faces[a].vertices.cmp(&c.faces[b].vertices));
    let mut number = vec![0_u32; c.faces.len()];
    for (public, &internal) in order.iter().enumerate() {
        number[internal] = public as u32;
    }

    let queries: Vec<(&[u32], u32)> = order
        .iter()
        .map(|&internal| {
            let vertices = &c.faces[internal].vertices;
            (vertices.as_slice(), inner_reference(&c.vertices, vertices))
        })
        .collect();
    let mut facets = Vec::with_capacity(order.len());
    for_each_facet_normal(&c.input, &queries, |basis, normal| {
        let face = &c.faces[order[facets.len()]];
        let mut neighbors: Vec<u32> = face.neighbors.iter().map(|&n| number[n as usize]).collect();
        neighbors.sort_unstable();
        facets.push(LogicalFacet {
            vertices: face.vertices.clone(),
            plane: facet_plane(&c.input, &basis, normal)?,
            neighbors,
        });
        Ok(())
    })?;

    // Boundary simplices: ascending vertex lists, ordered lexicographically,
    // then the last two swapped where that makes the order outward. Every
    // simplex of the complex is in outward order, so the ascending list is
    // outward exactly when the sort is an even permutation: a transposition
    // of two vertices reverses the orientation sign.
    let mut simplices: Vec<(Vec<u32>, u32, bool)> = c
        .simplices
        .iter()
        .map(|s| {
            let mut sorted = s.vertices.clone();
            sorted.sort_unstable();
            (
                sorted,
                number[s.face as usize],
                odd_permutation(&s.vertices),
            )
        })
        .collect();
    simplices.sort();
    let mut simplex_vertices = Vec::with_capacity(simplices.len() * d);
    let mut simplex_facets = Vec::with_capacity(simplices.len());
    for (mut vertices, facet, odd) in simplices {
        if d >= 2 {
            #[cfg(debug_assertions)]
            {
                let inner = inner_reference(&c.vertices, &facets[facet as usize].vertices);
                let mut points: Vec<&[f64]> = vertices.iter().map(|&v| c.input.point(v)).collect();
                points.push(c.input.point(inner));
                debug_assert_eq!(
                    orient(&points)? == Sign::Positive,
                    odd,
                    "the parity of the sort decides the outward order"
                );
            }
            if odd {
                vertices.swap(d - 2, d - 1);
            }
        }
        simplex_vertices.extend(vertices);
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
        facets,
        simplex_vertices,
        simplex_facets,
        vertex_coordinates,
    })
}
