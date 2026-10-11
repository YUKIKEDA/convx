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

use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::qr::no_pivoting as qr;
use faer::{get_global_parallelism, Conj, Mat, Par};

use crate::delaunay::{complex, NO_NEIGHBOR};
use crate::hull::classify::classify;
use crate::hull::input::accept;
use crate::hull::publish::{for_each_facet_normal, inner_reference};
use crate::hull::ConvexHullError;
use crate::lists::Lists;
use crate::normal::{binary_exponent, scale_by_power_of_two};
use crate::predicates::{orient, Sign};
use crate::small::Small;

/// Builds a [`VoronoiDiagram`].
///
/// ```
/// use convx::VoronoiBuilder;
///
/// // A square: its four corners are cocircular, so there is one finite
/// // vertex at the center, four rays, and no interface across a diagonal.
/// let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
/// let voronoi = VoronoiBuilder::new(2, &points).build()?;
/// assert_eq!(voronoi.vertices().len(), 1);
/// let center = voronoi.vertices().get(0).unwrap();
/// // The center (1, 1), solved in f64.
/// assert!(center.coords().iter().all(|x| (x - 1.0).abs() < 1e-12));
/// assert_eq!(center.sites(), [0, 1, 2, 3]);
/// let pairs: Vec<[u32; 2]> = voronoi.interfaces().iter().map(|f| f.sites()).collect();
/// assert_eq!(pairs, [[0, 1], [0, 3], [1, 2], [2, 3]]);
/// assert!(voronoi.cells().iter().all(|c| c.rays().len() == 2));
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct VoronoiBuilder<'a> {
    dim: usize,
    points: &'a [f64],
}

impl<'a> VoronoiBuilder<'a> {
    /// A builder for sites of dimension `dim`, stored row-major in `points`.
    #[must_use]
    pub fn new(dim: usize, points: &'a [f64]) -> Self {
        Self { dim, points }
    }

    /// Builds the diagram.
    ///
    /// # Errors
    ///
    /// The failures of [`DelaunayBuilder::build`](crate::DelaunayBuilder::build),
    /// and [`ConvexHullError::NonFiniteCircumcenter`] when every simplex of
    /// some vertex has a non-finite circumcenter in `f64`.
    pub fn build(self) -> Result<VoronoiDiagram, ConvexHullError> {
        build(self.dim, self.points)
    }
}

/// A Voronoi diagram of points in dimension D.
///
/// Every list is kept flat and published through borrowed views (design
/// §8, §9). Rays form one table, numbered by apex and then by
/// `hull_facet`; cells and interfaces hold numbers into it.
#[derive(Clone, Debug, PartialEq)]
pub struct VoronoiDiagram {
    dim: usize,
    representative: Vec<u32>,
    /// D coordinates per finite vertex.
    vertex_coords: Vec<f64>,
    vertex_sites: Lists<u32>,
    ray_apexes: Vec<u32>,
    /// D entries per ray.
    ray_directions: Vec<f64>,
    ray_hull_facets: Lists<u32>,
    cell_sites: Vec<u32>,
    cell_vertices: Lists<u32>,
    cell_rays: Lists<u32>,
    interface_sites: Vec<[u32; 2]>,
    interface_vertices: Lists<u32>,
    interface_rays: Lists<u32>,
}

impl VoronoiDiagram {
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

    /// Finite vertices, in the lexicographic order of their sites.
    #[must_use]
    pub fn vertices(&self) -> VoronoiVertices<'_> {
        VoronoiVertices { diagram: self }
    }

    /// Every ray, by apex, then by `hull_facet`.
    #[must_use]
    pub fn rays(&self) -> VoronoiRays<'_> {
        VoronoiRays { diagram: self }
    }

    /// One cell per representative, in ascending site order.
    #[must_use]
    pub fn cells(&self) -> VoronoiCells<'_> {
        VoronoiCells { diagram: self }
    }

    /// Faces shared by two cells, in the lexicographic order of their sites.
    #[must_use]
    pub fn interfaces(&self) -> VoronoiInterfaces<'_> {
        VoronoiInterfaces { diagram: self }
    }
}

/// A numbered list of views into a [`VoronoiDiagram`], one type per item.
macro_rules! collection {
    ($(#[$meta:meta])* $name:ident, $item:ident, $len:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy)]
        pub struct $name<'a> {
            diagram: &'a VoronoiDiagram,
        }

        impl<'a> $name<'a> {
            /// Number of items.
            #[must_use]
            pub fn len(&self) -> usize {
                let len: fn(&VoronoiDiagram) -> usize = $len;
                len(self.diagram)
            }

            /// Whether there are no items.
            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.len() == 0
            }

            /// The item numbered `index`, or `None` when no item has that
            /// number.
            #[must_use]
            pub fn get(&self, index: u32) -> Option<$item<'a>> {
                ((index as usize) < self.len()).then_some($item {
                    diagram: self.diagram,
                    index,
                })
            }

            /// Every item, in order.
            pub fn iter(&self) -> impl ExactSizeIterator<Item = $item<'a>> + 'a {
                let diagram = self.diagram;
                (0..self.len() as u32).map(move |index| $item { diagram, index })
            }
        }

        impl core::fmt::Debug for $name<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_list().entries(self.iter()).finish()
            }
        }
    };
}

collection!(
    /// The finite vertices of a [`VoronoiDiagram`].
    VoronoiVertices,
    VoronoiVertex,
    |d| d.vertex_sites.len()
);
collection!(
    /// The rays of a [`VoronoiDiagram`].
    VoronoiRays,
    VoronoiRay,
    |d| d.ray_apexes.len()
);
collection!(
    /// The cells of a [`VoronoiDiagram`].
    VoronoiCells,
    VoronoiCell,
    |d| d.cell_sites.len()
);
collection!(
    /// The interfaces of a [`VoronoiDiagram`].
    VoronoiInterfaces,
    VoronoiInterface,
    |d| d.interface_sites.len()
);

/// A finite Voronoi vertex: a view into its [`VoronoiDiagram`].
#[derive(Clone, Copy)]
pub struct VoronoiVertex<'a> {
    diagram: &'a VoronoiDiagram,
    index: u32,
}

impl<'a> VoronoiVertex<'a> {
    /// The circumcenter. Length D, finite.
    #[must_use]
    pub fn coords(&self) -> &'a [f64] {
        let d = self.diagram.dim;
        let i = self.index as usize;
        &self.diagram.vertex_coords[i * d..(i + 1) * d]
    }

    /// The cospherical sites of this vertex, ascending. At least D + 1.
    #[must_use]
    pub fn sites(&self) -> &'a [u32] {
        self.diagram.vertex_sites.get(self.index as usize)
    }
}

/// An unbounded edge, a finite vertex and a facet of the site hull: a view
/// into its [`VoronoiDiagram`].
#[derive(Clone, Copy)]
pub struct VoronoiRay<'a> {
    diagram: &'a VoronoiDiagram,
    index: u32,
}

impl<'a> VoronoiRay<'a> {
    /// Number of the apex in [`VoronoiDiagram::vertices`].
    #[must_use]
    pub fn apex(&self) -> u32 {
        self.diagram.ray_apexes[self.index as usize]
    }

    /// The outward unit normal of the site-hull facet. Length D.
    #[must_use]
    pub fn direction(&self) -> &'a [f64] {
        let d = self.diagram.dim;
        let i = self.index as usize;
        &self.diagram.ray_directions[i * d..(i + 1) * d]
    }

    /// The extreme points of that site-hull facet, ascending.
    #[must_use]
    pub fn hull_facet(&self) -> &'a [u32] {
        self.diagram.ray_hull_facets.get(self.index as usize)
    }
}

/// The rays numbered `numbers` in `diagram`, in that order.
fn rays_of<'a>(
    diagram: &'a VoronoiDiagram,
    numbers: &'a [u32],
) -> impl ExactSizeIterator<Item = VoronoiRay<'a>> + 'a {
    numbers
        .iter()
        .map(move |&index| VoronoiRay { diagram, index })
}

/// The Voronoi cell of one site: a view into its [`VoronoiDiagram`].
#[derive(Clone, Copy)]
pub struct VoronoiCell<'a> {
    diagram: &'a VoronoiDiagram,
    index: u32,
}

impl<'a> VoronoiCell<'a> {
    /// The site.
    #[must_use]
    pub fn site(&self) -> u32 {
        self.diagram.cell_sites[self.index as usize]
    }

    /// Incident finite vertices, ascending.
    #[must_use]
    pub fn vertices(&self) -> &'a [u32] {
        self.diagram.cell_vertices.get(self.index as usize)
    }

    /// Numbers in [`VoronoiDiagram::rays`] of the incident rays, ascending:
    /// by apex, then by `hull_facet`. Empty for an interior site; at least
    /// one for a site on the boundary of the site hull.
    #[must_use]
    pub fn ray_numbers(&self) -> &'a [u32] {
        self.diagram.cell_rays.get(self.index as usize)
    }

    /// The incident rays, in the order of [`Self::ray_numbers`].
    pub fn rays(&self) -> impl ExactSizeIterator<Item = VoronoiRay<'a>> + 'a {
        rays_of(self.diagram, self.ray_numbers())
    }
}

/// The face where two cells meet in dimension D - 1: a view into its
/// [`VoronoiDiagram`].
#[derive(Clone, Copy)]
pub struct VoronoiInterface<'a> {
    diagram: &'a VoronoiDiagram,
    index: u32,
}

impl<'a> VoronoiInterface<'a> {
    /// The two sites, ascending.
    #[must_use]
    pub fn sites(&self) -> [u32; 2] {
        self.diagram.interface_sites[self.index as usize]
    }

    /// Finite vertices of the face, ascending. Never empty.
    #[must_use]
    pub fn vertices(&self) -> &'a [u32] {
        self.diagram.interface_vertices.get(self.index as usize)
    }

    /// Numbers in [`VoronoiDiagram::rays`] of the face's rays, ascending.
    #[must_use]
    pub fn ray_numbers(&self) -> &'a [u32] {
        self.diagram.interface_rays.get(self.index as usize)
    }

    /// The face's rays, in the order of [`Self::ray_numbers`].
    pub fn rays(&self) -> impl ExactSizeIterator<Item = VoronoiRay<'a>> + 'a {
        rays_of(self.diagram, self.ray_numbers())
    }
}

impl core::fmt::Debug for VoronoiVertex<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VoronoiVertex")
            .field("coords", &self.coords())
            .field("sites", &self.sites())
            .finish()
    }
}

impl core::fmt::Debug for VoronoiRay<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VoronoiRay")
            .field("apex", &self.apex())
            .field("direction", &self.direction())
            .field("hull_facet", &self.hull_facet())
            .finish()
    }
}

impl core::fmt::Debug for VoronoiCell<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VoronoiCell")
            .field("site", &self.site())
            .field("vertices", &self.vertices())
            .field("ray_numbers", &self.ray_numbers())
            .finish()
    }
}

impl core::fmt::Debug for VoronoiInterface<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VoronoiInterface")
            .field("sites", &self.sites())
            .field("vertices", &self.vertices())
            .field("ray_numbers", &self.ray_numbers())
            .finish()
    }
}

/// The facets of the site hull, flat (#523): each one's extreme points, a
/// basis of its hyperplane, and its outward unit normal, `d` values each.
struct HullFacets {
    vertices: Lists<u32>,
    basis: Lists<u32>,
    normals: Vec<f64>,
}

/// The facets of one group's polytope, flat and reused from one group to
/// the next (#523): each facet's sites, and whether it lies on the site
/// hull.
#[derive(Default)]
struct GroupFacets {
    sites: Lists<u32>,
    on_hull: Vec<bool>,
}

fn build(dim: usize, points: &[f64]) -> Result<VoronoiDiagram, ConvexHullError> {
    let complex = complex(dim, points)?;
    let d = complex.dim;
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let orient_of = |set: &[u32]| -> Result<Sign, ConvexHullError> {
        let refs: Small<&[f64], 11> = set.iter().map(|&v| point(v)).collect();
        Ok(orient(&refs)?)
    };

    // The site hull of the original sites, for the rays' facets.
    let hull = classify(accept(dim, points)?)?;
    let queries: Vec<(&[u32], u32)> = hull
        .faces
        .iter()
        .map(|face| {
            (
                face.vertices,
                inner_reference(&hull.vertices, face.vertices),
            )
        })
        .collect();
    let mut hull_facets = HullFacets {
        vertices: Lists::with_capacity(hull.faces.len(), hull.faces.len() * d),
        basis: Lists::with_capacity(hull.faces.len(), hull.faces.len() * d),
        normals: Vec::with_capacity(hull.faces.len() * d),
    };
    for_each_facet_normal(&hull.input, &queries, |basis, normal| {
        let f = hull_facets.vertices.len();
        hull_facets.vertices.push(hull.faces.get(f).vertices);
        hull_facets.basis.push(basis);
        hull_facets.normals.extend_from_slice(normal);
        Ok(())
    })?;
    let hull_count = hull_facets.vertices.len();

    // Groups in the order of their site lists, and each group's cells in
    // the lexicographic order of their rows, from which a vertex takes its
    // coordinates (design §8). The Delaunay order is the construction's
    // (design §7), so the cells of each group are sorted here; a group of a
    // set in general position has one cell.
    let k = d + 1;
    let group_sites = &complex.sites;
    let mut groups: Vec<u32> = (0..group_sites.len() as u32).collect();
    groups.sort_unstable_by(|&a, &b| group_sites.get(a as usize).cmp(group_sites.get(b as usize)));
    let mut members_of: Vec<(u32, u32)> = complex
        .group
        .iter()
        .enumerate()
        .map(|(c, &g)| (g, c as u32))
        .collect();
    let mut members = lists_of(group_sites.len(), &mut members_of);
    drop(members_of);
    let row = |c: u32| &complex.cells[c as usize * k..(c as usize + 1) * k];
    for g in 0..members.len() {
        members
            .get_mut(g)
            .sort_unstable_by(|&a, &b| row(a).cmp(row(b)));
    }

    // The site-hull facets incident to each hull vertex, as offsets into one
    // list.
    let mut incident_start = vec![0_usize; points.len() / d + 1];
    for f in 0..hull_count {
        for &v in hull_facets.vertices.get(f) {
            incident_start[v as usize + 1] += 1;
        }
    }
    for i in 1..incident_start.len() {
        incident_start[i] += incident_start[i - 1];
    }
    let mut incident = vec![0_u32; incident_start[incident_start.len() - 1]];
    let mut next = incident_start.clone();
    for f in 0..hull_count {
        for &v in hull_facets.vertices.get(f) {
            incident[next[v as usize]] = f as u32;
            next[v as usize] += 1;
        }
    }
    let contains_face = |f: usize, face: &[u32]| -> Result<bool, ConvexHullError> {
        for &s in face {
            let set: Small<u32, 11> = hull_facets
                .basis
                .get(f)
                .iter()
                .copied()
                .chain([s])
                .collect();
            if !on_hyperplane(&orient_of, &set)? {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let scan_hull = |face: &[u32]| -> Result<Option<usize>, ConvexHullError> {
        for f in 0..hull_count {
            if contains_face(f, face)? {
                return Ok(Some(f));
            }
        }
        Ok(None)
    };

    let mut vertex_coords = Vec::with_capacity(groups.len() * d);
    let mut vertex_sites = Lists::with_capacity(groups.len(), groups.len() * (d + 1));
    let mut ray_apexes: Vec<u32> = Vec::new();
    let mut ray_directions: Vec<f64> = Vec::new();
    let mut ray_hull_facets = Lists::default();
    // Each ray's face: the group's sites on its site-hull facet.
    let mut ray_faces: Lists<u32> = Lists::default();
    // Interfaces as (sites, item) entries, sorted into lists at the end. A
    // group in general position is a simplex, whose every pair of sites is
    // an edge.
    let mut interface_apexes: Vec<([u32; 2], u32)> =
        Vec::with_capacity(groups.len() * k * (k - 1) / 2);
    let mut interface_ray_entries: Vec<([u32; 2], u32)> = Vec::new();
    // Buffers of one group, reused from one group to the next (#523).
    let mut circumcenters = Circumcenters::new(d);
    let mut tile_keys: Vec<u32> = Vec::new();
    let mut tile_hull: Vec<bool> = Vec::new();
    let mut tile_list: Vec<usize> = Vec::new();
    let mut facets = GroupFacets::default();
    let mut facet: Vec<u32> = Vec::new();
    let mut group_rays: Vec<(usize, usize)> = Vec::new();
    let mut meet: Vec<u32> = Vec::new();
    for (apex, &g) in groups.iter().enumerate() {
        let apex = apex as u32;
        let sites = group_sites.get(g as usize);
        let cells = members.get(g as usize);
        vertex_coords_of(
            &point,
            cells.iter().map(|&c| row(c)),
            &mut circumcenters,
            &mut vertex_coords,
        )?;
        vertex_sites.push(sites);

        // Facets of the group's polytope: each face of a cell whose cell
        // across is in another group, or on the site hull, tiles one; the
        // facet is every site of the group on that face's hyperplane.
        tile_keys.clear();
        tile_hull.clear();
        for &c in cells {
            for skip in 0..k {
                let across = complex.neighbors[c as usize * k + skip];
                if across != NO_NEIGHBOR && complex.group[across as usize] == g {
                    continue;
                }
                tile_keys.extend(
                    row(c)
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != skip)
                        .map(|(_, &v)| v),
                );
                tile_hull.push(across == NO_NEIGHBOR);
            }
        }
        let key = |i: usize| &tile_keys[i * d..(i + 1) * d];
        tile_list.clear();
        tile_list.extend(0..tile_hull.len());
        tile_list.sort_unstable_by(|&a, &b| key(a).cmp(key(b)));
        facets.sites.clear();
        facets.on_hull.clear();
        for &i in &tile_list {
            let tile = key(i);
            // A simplex has no site on the hyperplane of a face but the face's.
            facet.clear();
            if cells.len() == 1 {
                facet.extend_from_slice(tile);
            } else {
                for &s in sites {
                    let set: Small<u32, 11> = tile.iter().copied().chain([s]).collect();
                    if tile.contains(&s) || on_hyperplane(&orient_of, &set)? {
                        facet.push(s);
                    }
                }
            }
            if facets.sites.iter().any(|f| f == facet.as_slice()) {
                continue;
            }
            facets.sites.push(&facet);
            facets.on_hull.push(tile_hull[i]);
        }

        // One ray per boundary facet, along the site-hull facet it lies in.
        // The face spans D - 1 dimensions, so one logical facet holds it; a
        // site that is a hull vertex is a vertex of that facet.
        group_rays.clear();
        for (j, face) in facets.sites.iter().enumerate() {
            if !facets.on_hull[j] {
                continue;
            }
            let mut found = None;
            if let Some(&v) = face
                .iter()
                .find(|&&v| incident_start[v as usize] < incident_start[v as usize + 1])
            {
                for &f in &incident[incident_start[v as usize]..incident_start[v as usize + 1]] {
                    if contains_face(f as usize, face)? {
                        found = Some(f as usize);
                        break;
                    }
                }
                debug_assert_eq!(
                    found,
                    scan_hull(face)?,
                    "the incident facets hold the boundary face"
                );
            } else {
                found = scan_hull(face)?;
            }
            let Some(hull_facet) = found else {
                debug_assert!(false, "a boundary face lies in a site-hull facet");
                continue;
            };
            group_rays.push((hull_facet, j));
        }
        // The rays of one apex, by `hull_facet`, numbered after those of
        // every earlier apex: the order of the diagram's ray table. The sort
        // is stable, as before.
        group_rays.sort_by(|x, y| {
            hull_facets
                .vertices
                .get(x.0)
                .cmp(hull_facets.vertices.get(y.0))
        });
        let first_ray = ray_apexes.len() as u32;
        for &(hull_facet, j) in &group_rays {
            ray_apexes.push(apex);
            ray_directions
                .extend_from_slice(&hull_facets.normals[hull_facet * d..(hull_facet + 1) * d]);
            ray_hull_facets.push(hull_facets.vertices.get(hull_facet));
            ray_faces.push(facets.sites.get(j));
        }

        // Interfaces: the edges of the group's polytope.
        for (i, &a) in sites.iter().enumerate() {
            for &b in &sites[i + 1..] {
                if !is_edge(d, sites.len(), &facets.sites, a, b, &mut meet) {
                    continue;
                }
                interface_apexes.push(([a, b], apex));
                for number in first_ray..ray_apexes.len() as u32 {
                    let face = ray_faces.get(number as usize);
                    if face.contains(&a) && face.contains(&b) {
                        interface_ray_entries.push(([a, b], number));
                    }
                }
            }
        }
    }

    let representatives: Vec<u32> = (0..complex.representative.len() as u32)
        .filter(|&i| complex.representative[i as usize] == i)
        .collect();
    // Each cell's vertices in group order and its rays in number order, as
    // (cell, item) pairs sorted into flat lists; every site of a group is a
    // representative.
    let mut cell_of = vec![u32::MAX; complex.representative.len()];
    for (c, &site) in representatives.iter().enumerate() {
        cell_of[site as usize] = c as u32;
    }
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for (v, &g) in groups.iter().enumerate() {
        for &site in group_sites.get(g as usize) {
            if let Some(cell) = cell_index(&cell_of, site) {
                pairs.push((cell as u32, v as u32));
            }
        }
    }
    let cell_vertices = lists_of(representatives.len(), &mut pairs);
    pairs.clear();
    for (number, face) in ray_faces.iter().enumerate() {
        for &site in face {
            if let Some(cell) = cell_index(&cell_of, site) {
                pairs.push((cell as u32, number as u32));
            }
        }
    }
    let cell_rays = lists_of(representatives.len(), &mut pairs);
    // The interfaces in the order of their sites, each one's vertices
    // ascending without repeats and its rays ascending.
    interface_apexes.sort_unstable();
    interface_ray_entries.sort_unstable();
    let mut interface_sites: Vec<[u32; 2]> = Vec::new();
    let mut interface_vertices = Lists::with_capacity(0, interface_apexes.len());
    let mut interface_rays = Lists::with_capacity(0, interface_ray_entries.len());
    let mut rays_rest = &interface_ray_entries[..];
    for run in interface_apexes.chunk_by(|x, y| x.0 == y.0) {
        let sites = run[0].0;
        interface_sites.push(sites);
        let mut last = None;
        interface_vertices.push_iter(run.iter().map(|&(_, apex)| apex).filter(|&apex| {
            let new = last != Some(apex);
            last = Some(apex);
            new
        }));
        let take = rays_rest.iter().take_while(|(s, _)| *s == sites).count();
        interface_rays.push_iter(rays_rest[..take].iter().map(|&(_, number)| number));
        rays_rest = &rays_rest[take..];
    }
    debug_assert!(
        rays_rest.is_empty(),
        "every interface ray has its interface"
    );
    Ok(VoronoiDiagram {
        dim: d,
        representative: complex.representative,
        vertex_coords,
        vertex_sites,
        ray_apexes,
        ray_directions,
        ray_hull_facets,
        cell_sites: representatives,
        cell_vertices,
        cell_rays,
        interface_sites,
        interface_vertices,
        interface_rays,
    })
}

/// The lists of `count` owners from (owner, item) pairs: owner `i`'s list
/// is its items in ascending order. Sorts `pairs`.
fn lists_of(count: usize, pairs: &mut [(u32, u32)]) -> Lists<u32> {
    pairs.sort_unstable();
    let mut lists = Lists::with_capacity(count, pairs.len());
    let mut rest = &pairs[..];
    for owner in 0..count as u32 {
        let take = rest.iter().take_while(|&&(o, _)| o == owner).count();
        lists.push_iter(rest[..take].iter().map(|&(_, item)| item));
        rest = &rest[take..];
    }
    lists
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
/// pair. `meet` is scratch, reused from one call to the next.
fn is_edge(
    d: usize,
    count: usize,
    facets: &Lists<u32>,
    a: u32,
    b: u32,
    meet: &mut Vec<u32>,
) -> bool {
    if count == d + 1 {
        return true;
    }
    let mut first = true;
    meet.clear();
    for facet in facets.iter() {
        if !(facet.contains(&a) && facet.contains(&b)) {
            continue;
        }
        if first {
            meet.extend_from_slice(facet);
            first = false;
        } else {
            meet.retain(|s| facet.contains(s));
        }
    }
    !first && meet.as_slice() == [a, b]
}

/// The cell of `site` when `site` is a representative. A group's sites and a
/// ray's face are representatives. `u32::MAX` fills every other index and
/// numbers no cell (design §3).
fn cell_index(cell_of: &[u32], site: u32) -> Option<usize> {
    let cell = cell_of.get(site as usize).copied();
    debug_assert!(
        matches!(cell, Some(c) if c != u32::MAX),
        "every site of a group is a representative"
    );
    cell.filter(|&c| c != u32::MAX).map(|c| c as usize)
}

/// Appends to `coords` the circumcenter of the lexicographically minimum
/// cell with a finite one, trying `cells`, which come in that order
/// (design §8).
fn vertex_coords_of<'p>(
    point: &impl Fn(u32) -> &'p [f64],
    cells: impl Iterator<Item = &'p [u32]>,
    circumcenters: &mut Circumcenters,
    coords: &mut Vec<f64>,
) -> Result<(), ConvexHullError> {
    for cell in cells {
        if circumcenters.compute(point, cell) {
            coords.extend_from_slice(&circumcenters.center);
            return Ok(());
        }
    }
    Err(ConvexHullError::NonFiniteCircumcenter)
}

/// What the circumcenter of a cell needs, kept from one vertex to the next
/// so that a vertex allocates nothing (#523). The factorization is the one
/// `Mat::qr` and its `solve_in_place` run, through the same functions with
/// the same block size and parallelism, so each coordinate keeps its bits.
struct Circumcenters {
    d: usize,
    /// The translated and scaled edges, row by row.
    scaled: Vec<f64>,
    /// `2 (v_i - v_0)`, factored in place.
    a: Mat<f64>,
    basis: Mat<f64>,
    r: Mat<f64>,
    coeff: Mat<f64>,
    rhs: Mat<f64>,
    work: MemBuffer,
    par: Par,
    /// The last circumcenter computed.
    center: Vec<f64>,
}

impl Circumcenters {
    fn new(d: usize) -> Self {
        let block = qr::factor::recommended_block_size::<f64>(d, d);
        let par = get_global_parallelism();
        let need = qr::factor::qr_in_place_scratch::<f64>(d, d, block, par, Default::default())
            .or(qr::solve::solve_in_place_scratch::<f64>(d, block, 1, par));
        Self {
            d,
            scaled: vec![0.0; d * d],
            a: Mat::zeros(d, d),
            basis: Mat::zeros(d, d),
            r: Mat::zeros(d, d),
            coeff: Mat::zeros(block, d),
            rhs: Mat::zeros(d, 1),
            work: MemBuffer::new(need),
            par,
            center: vec![0.0; d],
        }
    }

    /// The circumcenter of the D + 1 sites `cell` into `self.center`, with
    /// the first site moved to the origin: `2 (v_i - v_0) . x =
    /// |v_i - v_0|^2`, solved by Householder QR, then `v_0 + x`. The
    /// translated edges are first scaled by the power of two that brings
    /// their width into [1, 2), and the solution scaled back, both exactly,
    /// so squares of large or tiny edges do not overflow or underflow.
    /// `false` when any value is not finite.
    fn compute<'p>(&mut self, point: &impl Fn(u32) -> &'p [f64], cell: &[u32]) -> bool {
        let d = self.d;
        let origin = point(cell[0]);
        for (i, &v) in cell[1..].iter().enumerate() {
            for (j, (x, o)) in point(v).iter().zip(origin).enumerate() {
                self.scaled[i * d + j] = x - o;
            }
        }
        let width = self.scaled.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
        if !width.is_finite() || width == 0.0 {
            return false;
        }
        let shift = -binary_exponent(width);
        for x in &mut self.scaled {
            *x = scale_by_power_of_two(*x, shift);
        }
        let scaled = &self.scaled;
        for i in 0..d {
            for j in 0..d {
                self.a[(i, j)] = 2.0 * scaled[i * d + j];
            }
            self.rhs[(i, 0)] = scaled[i * d..(i + 1) * d]
                .iter()
                .map(|x| x * x)
                .sum::<f64>();
        }
        // `Qr::new`: factor a copy in place, with zeroed coefficients, then
        // split it into the Householder basis (unit lower) and R (upper).
        self.coeff.fill(0.0);
        qr::factor::qr_in_place(
            self.a.as_mut(),
            self.coeff.as_mut(),
            self.par,
            MemStack::new(&mut self.work),
            Default::default(),
        );
        self.r.fill(0.0);
        self.r.copy_from_triangular_upper(self.a.as_ref());
        self.basis.copy_from(self.a.as_ref());
        for j in 0..d {
            for i in 0..j {
                self.basis[(i, j)] = 0.0;
            }
            self.basis[(j, j)] = 1.0;
        }
        // `solve_in_place`.
        qr::solve::solve_in_place_with_conj(
            self.basis.as_ref(),
            self.coeff.as_ref(),
            self.r.as_ref(),
            Conj::No,
            self.rhs.as_mut(),
            self.par,
            MemStack::new(&mut self.work),
        );
        for (i, (c, o)) in self.center.iter_mut().zip(origin).enumerate() {
            *c = o + scale_by_power_of_two(self.rhs[(i, 0)], -shift);
        }
        self.center.iter().all(|x| x.is_finite())
    }
}

#[cfg(test)]
mod tests;
