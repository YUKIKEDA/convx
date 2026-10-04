//! The Voronoi diagram as the dual of the Delaunay complex before diagonals
//! are inserted (design §8).
//!
//! A finite vertex is one lower logical facet of the lift (a group of
//! cospherical sites from the Delaunay complex); its sites are the group's
//! sites, and when every site is on one sphere there is one vertex. Its
//! coordinates are the circumcenter of the group's lexicographically minimum
//! simplex, translated to one vertex and solved by Householder QR, or the
//! next simplex's when that is not finite. Coordinates never decide
//! topology.
//!
//! The polytope of a group has facets of two kinds: shared with another
//! group, or on the boundary of the site hull. Each boundary facet lies in
//! one logical facet of the site hull, and gives one ray from the group's
//! vertex along that facet's outward unit normal. Two cells meet in
//! dimension D - 1 exactly when their sites span an edge of some group's
//! polytope; the diagonals inside a group are not edges, so they get no
//! interface.

use std::collections::{BTreeMap, HashMap};

use faer::linalg::solvers::Solve;
use faer::Mat;

use crate::delaunay::complex;
use crate::hull::classify::classify;
use crate::hull::input::accept;
use crate::hull::publish::{for_each_facet_normal, inner_reference};
use crate::hull::simplicial::Execution;
use crate::hull::ConvexHullError;
use crate::normal::{binary_exponent, scale_by_power_of_two};
use crate::predicates::{orient, Sign};

/// Builds a [`VoronoiDiagram`].
///
/// ```
/// use convx::VoronoiBuilder;
///
/// // A square: its four corners are cocircular, so there is one finite
/// // vertex at the center, four rays, and no interface across a diagonal.
/// let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
/// let voronoi = VoronoiBuilder::new(2, &points).build()?;
/// assert_eq!(voronoi.vertices.len(), 1);
/// // The center (1, 1), solved in f64.
/// assert!(voronoi.vertices[0].coords.iter().all(|x| (x - 1.0).abs() < 1e-12));
/// assert_eq!(voronoi.vertices[0].sites, [0, 1, 2, 3]);
/// let pairs: Vec<[u32; 2]> = voronoi.interfaces.iter().map(|f| f.sites).collect();
/// assert_eq!(pairs, [[0, 1], [0, 3], [1, 2], [2, 3]]);
/// assert!(voronoi.cells.iter().all(|c| c.rays.len() == 2));
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct VoronoiBuilder<'a> {
    dim: usize,
    points: &'a [f64],
    execution: Execution,
}

impl<'a> VoronoiBuilder<'a> {
    /// A builder for sites of dimension `dim`, stored row-major in `points`.
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

    /// Builds the diagram.
    ///
    /// # Errors
    ///
    /// The failures of [`DelaunayBuilder::build`](crate::DelaunayBuilder::build),
    /// and [`ConvexHullError::NonFiniteCircumcenter`] when every simplex of
    /// some vertex has a non-finite circumcenter in `f64`.
    pub fn build(self) -> Result<VoronoiDiagram, ConvexHullError> {
        build(self.dim, self.points, self.execution)
    }
}

/// A Voronoi diagram of points in dimension D.
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiDiagram {
    /// Dimension D of the sites.
    pub dim: usize,
    /// For each input index, the smallest index of a point equal to it.
    pub representative: Vec<u32>,
    /// Finite vertices, in the lexicographic order of `sites`.
    pub vertices: Vec<VoronoiVertex>,
    /// One cell per representative, in ascending site order.
    pub cells: Vec<VoronoiCell>,
    /// Faces shared by two cells, in the lexicographic order of `sites`.
    pub interfaces: Vec<VoronoiInterface>,
}

/// A finite Voronoi vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiVertex {
    /// The circumcenter. Length D, finite.
    pub coords: Vec<f64>,
    /// The cospherical sites of this vertex, ascending. At least D + 1.
    pub sites: Vec<u32>,
}

/// An unbounded edge: a finite vertex and a facet of the site hull.
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiRay {
    /// Index of the apex in [`VoronoiDiagram::vertices`].
    pub apex: u32,
    /// The outward unit normal of the site-hull facet. Length D.
    pub direction: Vec<f64>,
    /// The extreme points of that site-hull facet, ascending.
    pub hull_facet: Vec<u32>,
}

/// The Voronoi cell of one site.
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiCell {
    /// The site.
    pub site: u32,
    /// Incident finite vertices, ascending.
    pub vertices: Vec<u32>,
    /// Incident rays, by apex, then by `hull_facet`. Empty for an interior
    /// site; at least one for a site on the boundary of the site hull.
    pub rays: Vec<VoronoiRay>,
}

/// The face where two cells meet in dimension D - 1.
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiInterface {
    /// The two sites, ascending.
    pub sites: [u32; 2],
    /// Finite vertices of the face, ascending. Never empty.
    pub vertices: Vec<u32>,
    /// Rays of the face, by apex, then by `hull_facet`.
    pub rays: Vec<VoronoiRay>,
}

/// A facet of the site hull: extreme points, a basis of its hyperplane, and
/// its outward unit normal.
struct HullFacet {
    vertices: Vec<u32>,
    basis: Vec<u32>,
    normal: Vec<f64>,
}

/// A ray of one vertex with the face of the group's polytope it leaves.
struct GroupRay {
    ray: VoronoiRay,
    /// The group's sites on the site-hull facet.
    face: Vec<u32>,
}

fn build(
    dim: usize,
    points: &[f64],
    execution: Execution,
) -> Result<VoronoiDiagram, ConvexHullError> {
    let complex = complex(dim, points, execution)?;
    let d = complex.dim;
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let orient_of = |set: &[u32]| -> Result<Sign, ConvexHullError> {
        let refs: Vec<&[f64]> = set.iter().map(|&v| point(v)).collect();
        Ok(orient(&refs)?)
    };

    // The site hull of the original sites, for the rays' facets.
    let hull = classify(accept(dim, points)?, execution)?;
    let queries: Vec<(&[u32], u32)> = hull
        .faces
        .iter()
        .map(|face| {
            let vertices = face.vertices.as_slice();
            (vertices, inner_reference(&hull.vertices, vertices))
        })
        .collect();
    let mut hull_facets = Vec::with_capacity(hull.faces.len());
    for_each_facet_normal(&hull.input, &queries, |basis, normal| {
        hull_facets.push(HullFacet {
            vertices: hull.faces[hull_facets.len()].vertices.clone(),
            basis,
            normal,
        });
        Ok(())
    })?;

    let mut groups = complex.groups;
    groups.sort_unstable_by(|a, b| a.sites.cmp(&b.sites));

    // How many cells of the whole complex contain each face of D sites: one
    // means the face is on the boundary of the site hull.
    let mut face_count: HashMap<Vec<u32>, usize> = HashMap::new();
    for group in &groups {
        for cell in &group.cells {
            for face in faces_of(cell) {
                *face_count.entry(face).or_insert(0) += 1;
            }
        }
    }

    let mut vertices = Vec::with_capacity(groups.len());
    let mut rays_of: Vec<Vec<GroupRay>> = Vec::with_capacity(groups.len());
    let mut interfaces: BTreeMap<[u32; 2], (Vec<u32>, Vec<VoronoiRay>)> = BTreeMap::new();
    for (index, group) in groups.iter().enumerate() {
        let apex = index as u32;
        vertices.push(VoronoiVertex {
            coords: vertex_coords(d, &point, &group.cells)?,
            sites: group.sites.clone(),
        });

        // Facets of the group's polytope: each face of a cell that no other
        // cell of the group shares tiles one; the facet is every site of the
        // group on that face's hyperplane.
        let mut tiles: HashMap<Vec<u32>, usize> = HashMap::new();
        for cell in &group.cells {
            for face in faces_of(cell) {
                *tiles.entry(face).or_insert(0) += 1;
            }
        }
        let mut tile_list: Vec<Vec<u32>> = tiles
            .into_iter()
            .filter(|&(_, count)| count == 1)
            .map(|(face, _)| face)
            .collect();
        tile_list.sort_unstable();
        let mut facets: Vec<(Vec<u32>, bool)> = Vec::new();
        for tile in tile_list {
            let mut facet = Vec::new();
            for &s in &group.sites {
                let mut set = tile.clone();
                set.push(s);
                if tile.contains(&s) || on_hyperplane(&orient_of, &set)? {
                    facet.push(s);
                }
            }
            if facets.iter().any(|(f, _)| *f == facet) {
                continue;
            }
            let on_hull = face_count.get(&tile).copied() == Some(1);
            facets.push((facet, on_hull));
        }

        // One ray per boundary facet, along the site-hull facet it lies in.
        let mut group_rays = Vec::new();
        for (face, _) in facets.iter().filter(|(_, on_hull)| *on_hull) {
            let mut found = None;
            for hull_facet in &hull_facets {
                let mut inside = true;
                for &s in face {
                    let mut set = hull_facet.basis.clone();
                    set.push(s);
                    if !on_hyperplane(&orient_of, &set)? {
                        inside = false;
                        break;
                    }
                }
                if inside {
                    found = Some(hull_facet);
                    break;
                }
            }
            let Some(hull_facet) = found else {
                debug_assert!(false, "a boundary face lies in a site-hull facet");
                continue;
            };
            group_rays.push(GroupRay {
                ray: VoronoiRay {
                    apex,
                    direction: hull_facet.normal.clone(),
                    hull_facet: hull_facet.vertices.clone(),
                },
                face: face.clone(),
            });
        }

        // Interfaces: the edges of the group's polytope.
        for (i, &a) in group.sites.iter().enumerate() {
            for &b in &group.sites[i + 1..] {
                if !is_edge(d, group.sites.len(), &facets, a, b) {
                    continue;
                }
                let entry = interfaces.entry([a, b]).or_default();
                entry.0.push(apex);
                for group_ray in &group_rays {
                    if group_ray.face.contains(&a) && group_ray.face.contains(&b) {
                        entry.1.push(group_ray.ray.clone());
                    }
                }
            }
        }
        rays_of.push(group_rays);
    }

    let representatives: Vec<u32> = (0..complex.representative.len() as u32)
        .filter(|&i| complex.representative[i as usize] == i)
        .collect();
    let cells = representatives
        .iter()
        .map(|&site| {
            let vertices: Vec<u32> = (0..groups.len() as u32)
                .filter(|&v| groups[v as usize].sites.binary_search(&site).is_ok())
                .collect();
            let mut rays: Vec<VoronoiRay> = rays_of
                .iter()
                .flatten()
                .filter(|r| r.face.contains(&site))
                .map(|r| r.ray.clone())
                .collect();
            sort_rays(&mut rays);
            VoronoiCell {
                site,
                vertices,
                rays,
            }
        })
        .collect();
    let interfaces = interfaces
        .into_iter()
        .map(|(sites, (mut vertices, mut rays))| {
            vertices.sort_unstable();
            vertices.dedup();
            sort_rays(&mut rays);
            VoronoiInterface {
                sites,
                vertices,
                rays,
            }
        })
        .collect();
    Ok(VoronoiDiagram {
        dim: d,
        representative: complex.representative,
        vertices,
        cells,
        interfaces,
    })
}

/// The faces of D sites of a cell of D + 1 sites, each ascending.
fn faces_of(cell: &[u32]) -> impl Iterator<Item = Vec<u32>> + '_ {
    (0..cell.len()).map(move |skip| {
        let mut face: Vec<u32> = cell
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != skip)
            .map(|(_, &v)| v)
            .collect();
        face.sort_unstable();
        face
    })
}

/// Whether the last site of `set` lies on the hyperplane through the first
/// D sites (D + 1 sites of dimension D).
fn on_hyperplane(
    orient_of: &impl Fn(&[u32]) -> Result<Sign, ConvexHullError>,
    set: &[u32],
) -> Result<bool, ConvexHullError> {
    Ok(orient_of(set)? == Sign::Zero)
}

/// Whether sites `a` and `b` span an edge of the polytope whose facets are
/// `facets` and which has `count` sites: in a simplex every pair does;
/// otherwise the intersection of the facets holding both is exactly the
/// pair.
fn is_edge(d: usize, count: usize, facets: &[(Vec<u32>, bool)], a: u32, b: u32) -> bool {
    if count == d + 1 {
        return true;
    }
    let mut meet: Option<Vec<u32>> = None;
    for (facet, _) in facets {
        if !(facet.contains(&a) && facet.contains(&b)) {
            continue;
        }
        meet = Some(match meet {
            None => facet.clone(),
            Some(m) => m.into_iter().filter(|s| facet.contains(s)).collect(),
        });
    }
    meet.is_some_and(|m| m == [a, b])
}

fn sort_rays(rays: &mut [VoronoiRay]) {
    rays.sort_by(|x, y| {
        x.apex
            .cmp(&y.apex)
            .then_with(|| x.hull_facet.cmp(&y.hull_facet))
    });
}

/// The circumcenter of the lexicographically minimum cell with a finite
/// one, trying the cells in that order (design §8).
fn vertex_coords<'p>(
    d: usize,
    point: &impl Fn(u32) -> &'p [f64],
    cells: &[Vec<u32>],
) -> Result<Vec<f64>, ConvexHullError> {
    let mut order: Vec<&Vec<u32>> = cells.iter().collect();
    order.sort_unstable();
    for cell in order {
        if let Some(center) = circumcenter(d, point, cell) {
            return Ok(center);
        }
    }
    Err(ConvexHullError::NonFiniteCircumcenter)
}

/// The circumcenter of the D + 1 sites `cell`, with the first site moved to
/// the origin: `2 (v_i - v_0) . x = |v_i - v_0|^2`, solved by Householder QR,
/// then `v_0 + x`. The translated edges are first scaled by the power of two
/// that brings their width into [1, 2), and the solution scaled back, both
/// exactly, so squares of large or tiny edges do not overflow or underflow.
/// `None` when any value is not finite.
fn circumcenter<'p>(d: usize, point: &impl Fn(u32) -> &'p [f64], cell: &[u32]) -> Option<Vec<f64>> {
    let origin = point(cell[0]);
    let edges: Vec<Vec<f64>> = cell[1..]
        .iter()
        .map(|&v| point(v).iter().zip(origin).map(|(x, o)| x - o).collect())
        .collect();
    let width = edges
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0.0_f64, f64::max);
    if !width.is_finite() || width == 0.0 {
        return None;
    }
    let shift = -binary_exponent(width);
    let scaled: Vec<Vec<f64>> = edges
        .iter()
        .map(|e| e.iter().map(|&x| scale_by_power_of_two(x, shift)).collect())
        .collect();
    let a = Mat::from_fn(d, d, |i, j| 2.0 * scaled[i][j]);
    let mut rhs = Mat::from_fn(d, 1, |i, _| scaled[i].iter().map(|x| x * x).sum::<f64>());
    a.qr().solve_in_place(&mut rhs);
    let center: Vec<f64> = (0..d)
        .map(|i| origin[i] + scale_by_power_of_two(rhs[(i, 0)], -shift))
        .collect();
    center.iter().all(|x| x.is_finite()).then_some(center)
}

#[cfg(test)]
mod tests;
