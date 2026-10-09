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

use std::collections::BTreeMap;

use faer::linalg::solvers::Solve;
use faer::Mat;

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

/// A facet of the site hull: extreme points, a basis of its hyperplane, and
/// its outward unit normal.
struct HullFacet {
    vertices: Vec<u32>,
    basis: Vec<u32>,
    normal: Vec<f64>,
}

/// A ray of one vertex with the face of the group's polytope it leaves.
struct GroupRay {
    /// The site-hull facet the face lies in, by index.
    hull_facet: usize,
    /// The ray's number in the diagram's table.
    number: u32,
    /// The group's sites on the site-hull facet.
    face: Vec<u32>,
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
    let mut hull_facets = Vec::with_capacity(hull.faces.len());
    for_each_facet_normal(&hull.input, &queries, |basis, normal| {
        hull_facets.push(HullFacet {
            vertices: hull.faces.get(hull_facets.len()).vertices.to_vec(),
            basis: basis.to_vec(),
            normal: normal.to_vec(),
        });
        Ok(())
    })?;

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
    for facet in &hull_facets {
        for &v in &facet.vertices {
            incident_start[v as usize + 1] += 1;
        }
    }
    for i in 1..incident_start.len() {
        incident_start[i] += incident_start[i - 1];
    }
    let mut incident = vec![0_u32; incident_start[incident_start.len() - 1]];
    let mut next = incident_start.clone();
    for (f, facet) in hull_facets.iter().enumerate() {
        for &v in &facet.vertices {
            incident[next[v as usize]] = f as u32;
            next[v as usize] += 1;
        }
    }
    let contains_face = |hull_facet: &HullFacet, face: &[u32]| -> Result<bool, ConvexHullError> {
        for &s in face {
            let set: Small<u32, 11> = hull_facet.basis.iter().copied().chain([s]).collect();
            if !on_hyperplane(&orient_of, &set)? {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let scan_hull = |face: &[u32]| -> Result<Option<usize>, ConvexHullError> {
        for (f, hull_facet) in hull_facets.iter().enumerate() {
            if contains_face(hull_facet, face)? {
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
    let mut rays_of: Vec<Vec<GroupRay>> = Vec::with_capacity(groups.len());
    let mut interfaces: BTreeMap<[u32; 2], (Vec<u32>, Vec<u32>)> = BTreeMap::new();
    let mut tile_keys: Vec<u32> = Vec::new();
    let mut tile_hull: Vec<bool> = Vec::new();
    for (apex, &g) in groups.iter().enumerate() {
        let apex = apex as u32;
        let sites = group_sites.get(g as usize);
        let cells = members.get(g as usize);
        vertex_coords.extend(vertex_coords_of(d, &point, cells.iter().map(|&c| row(c)))?);
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
        let mut tile_list: Vec<usize> = (0..tile_hull.len()).collect();
        tile_list.sort_unstable_by(|&a, &b| key(a).cmp(key(b)));
        let mut facets: Vec<(Vec<u32>, bool)> = Vec::new();
        for i in tile_list {
            let tile = key(i);
            // A simplex has no site on the hyperplane of a face but the face's.
            let facet = if cells.len() == 1 {
                tile.to_vec()
            } else {
                let mut facet = Vec::new();
                for &s in sites {
                    let set: Small<u32, 11> = tile.iter().copied().chain([s]).collect();
                    if tile.contains(&s) || on_hyperplane(&orient_of, &set)? {
                        facet.push(s);
                    }
                }
                facet
            };
            if facets.iter().any(|(f, _)| *f == facet) {
                continue;
            }
            facets.push((facet, tile_hull[i]));
        }

        // One ray per boundary facet, along the site-hull facet it lies in.
        // The face spans D - 1 dimensions, so one logical facet holds it; a
        // site that is a hull vertex is a vertex of that facet.
        let mut group_rays = Vec::new();
        for (face, _) in facets.iter().filter(|(_, on_hull)| *on_hull) {
            let mut found = None;
            if let Some(&v) = face
                .iter()
                .find(|&&v| incident_start[v as usize] < incident_start[v as usize + 1])
            {
                for &f in &incident[incident_start[v as usize]..incident_start[v as usize + 1]] {
                    if contains_face(&hull_facets[f as usize], face)? {
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
            group_rays.push(GroupRay {
                hull_facet,
                number: 0,
                face: face.clone(),
            });
        }
        // The rays of one apex, by `hull_facet`, numbered after those of
        // every earlier apex: the order of the diagram's ray table.
        group_rays.sort_by(|x, y| {
            hull_facets[x.hull_facet]
                .vertices
                .cmp(&hull_facets[y.hull_facet].vertices)
        });
        for group_ray in &mut group_rays {
            let hull_facet = &hull_facets[group_ray.hull_facet];
            group_ray.number = ray_apexes.len() as u32;
            ray_apexes.push(apex);
            ray_directions.extend_from_slice(&hull_facet.normal);
            ray_hull_facets.push(&hull_facet.vertices);
        }

        // Interfaces: the edges of the group's polytope.
        for (i, &a) in sites.iter().enumerate() {
            for &b in &sites[i + 1..] {
                if !is_edge(d, sites.len(), &facets, a, b) {
                    continue;
                }
                let entry = interfaces.entry([a, b]).or_default();
                entry.0.push(apex);
                for group_ray in &group_rays {
                    if group_ray.face.contains(&a) && group_ray.face.contains(&b) {
                        entry.1.push(group_ray.number);
                    }
                }
            }
        }
        rays_of.push(group_rays);
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
    for group_ray in rays_of.iter().flatten() {
        for &site in &group_ray.face {
            if let Some(cell) = cell_index(&cell_of, site) {
                pairs.push((cell as u32, group_ray.number));
            }
        }
    }
    let cell_rays = lists_of(representatives.len(), &mut pairs);
    let mut interface_sites = Vec::with_capacity(interfaces.len());
    let mut interface_vertices = Lists::with_capacity(interfaces.len(), 2 * interfaces.len());
    let mut interface_rays = Lists::with_capacity(interfaces.len(), 0);
    for (sites, (mut vertices, mut rays)) in interfaces {
        vertices.sort_unstable();
        vertices.dedup();
        rays.sort_unstable();
        interface_sites.push(sites);
        interface_vertices.push(&vertices);
        interface_rays.push(&rays);
    }
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

/// The circumcenter of the lexicographically minimum cell with a finite
/// one, trying `cells`, which come in that order (design §8).
fn vertex_coords_of<'p>(
    d: usize,
    point: &impl Fn(u32) -> &'p [f64],
    cells: impl Iterator<Item = &'p [u32]>,
) -> Result<Vec<f64>, ConvexHullError> {
    for cell in cells {
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
