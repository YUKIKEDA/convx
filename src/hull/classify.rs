//! Distance-zero classification and the index partition (design §3, §5).
//!
//! After insertion and the coplanar merge, every representative that is not
//! a vertex of the simplicial complex is classified by the exact sign of its
//! distance to each logical facet: all negative is interior, and a zero on
//! some facet puts it on that facet's boundary set. A point that
//! construction recorded on the plane of a simplex of the final complex
//! starts at that simplex's facet and moves only to neighboring facets at
//! distance zero (design §3); every other point is tested against every
//! facet.
//!
//! The facet's points (its simplicial vertices and the zero-distance points)
//! all lie on the facet's supporting hyperplane. Their extreme points are the
//! facet's vertices. They are found exactly: the hyperplane is projected onto
//! D - 1 coordinates by dropping an axis along which it is not vertical (an
//! affine bijection of the hyperplane, so extremeness is preserved), and the
//! hull of the projected points is built recursively down to D = 1.
//!
//! This decides, with the same exact signs, the cases §3 lists: a
//! zero-distance point outside `conv(V)` becomes a vertex, one inside stays a
//! non-vertex boundary point. It also decides a case §3 does not name: a
//! simplicial vertex that was extreme when it was inserted and later fell on
//! the relative interior of a face or an edge is not extreme, so it moves to
//! `coplanar_points`. `vertices` is then exactly the set of extreme points.
//!
//! Every facet other than a single simplex with extreme vertices is then
//! triangulated by placing its extreme points in index order (see [`place`]).
//! §3 asks for a re-triangulation only of a facet that gained a vertex, from
//! the lexicographically minimum basis; but two facets that share a lower
//! face must split it the same way, or the boundary does not close. A facet
//! kept as built and a re-triangulated neighbor can disagree there, and so
//! can two neighbors triangulated basis-first. The placing triangulation
//! restricts to the placing triangulation of every face in the same order, so
//! all facets agree. Simplex neighbors are then linked across the updated
//! simplices; face neighbors are those of the merged groups.

use std::collections::{BinaryHeap, HashMap};

use super::input::{accept, Input, InputWork};
use super::merge::{merge_into, LogicalFacets, MergeWork};
use super::polygon::{self, clearly_inside, ChainWork, Edge, Rows};
use super::ridge::{fingerprint, pair_equal_keys};
use super::simplicial::{HullWork, SimplicialHull};
use super::ConvexHullError;
use crate::arena::{FacetId, SlotMarks};
use crate::lists::Lists;
use crate::predicates::{orient, orient_direction, Sign};
use crate::small::Small;

/// The simplices of the boundary complex, in flat lists (#411): simplex `s`
/// has the vertices `vertices[s * d..(s + 1) * d]`, the face `faces[s]`, and
/// the neighbors `neighbors[s * links..(s + 1) * links]`, where `links` is D,
/// or 0 for D = 1, whose segments have no ridge. A list per simplex cost a
/// third of the classification of a polygon of every point extreme.
#[derive(Clone)]
pub(crate) struct Complex {
    d: usize,
    links: usize,
    vertices: Vec<u32>,
    faces: Vec<u32>,
    neighbors: Vec<u32>,
}

/// One simplex of a [`Complex`].
#[derive(Clone, Copy)]
pub(crate) struct ComplexSimplex<'a> {
    /// D vertices.
    pub(crate) vertices: &'a [u32],
    /// The face that contains this simplex.
    pub(crate) face: u32,
    /// `neighbors[i]` is the simplex across the ridge opposite
    /// `vertices[i]`. Empty for D = 1.
    pub(crate) neighbors: &'a [u32],
}

impl Complex {
    /// An empty complex of dimension `d`, with room for `simplices`.
    fn with_capacity(d: usize, simplices: usize) -> Self {
        let links = if d == 1 { 0 } else { d };
        Self {
            d,
            links,
            vertices: Vec::with_capacity(simplices * d),
            faces: Vec::with_capacity(simplices),
            neighbors: Vec::with_capacity(simplices * links),
        }
    }

    /// Appends a simplex of `face` with its neighbors, or with every
    /// neighbor [`UNLINKED`] when `neighbors` is `None`.
    fn push(&mut self, vertices: &[u32], face: u32, neighbors: Option<&[u32]>) {
        debug_assert_eq!(vertices.len(), self.d, "a simplex has D vertices");
        self.vertices.extend_from_slice(vertices);
        self.faces.push(face);
        match neighbors {
            Some(neighbors) => {
                debug_assert_eq!(neighbors.len(), self.links, "D neighbors, none for D = 1");
                self.neighbors.extend_from_slice(neighbors);
            }
            None => self
                .neighbors
                .extend(core::iter::repeat_n(UNLINKED, self.links)),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.faces.len()
    }

    pub(crate) fn get(&self, s: usize) -> ComplexSimplex<'_> {
        ComplexSimplex {
            vertices: &self.vertices[s * self.d..(s + 1) * self.d],
            face: self.faces[s],
            neighbors: &self.neighbors[s * self.links..(s + 1) * self.links],
        }
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = ComplexSimplex<'_>> + '_ {
        (0..self.len()).map(move |s| self.get(s))
    }

    fn neighbors_mut(&mut self, s: usize) -> &mut [u32] {
        &mut self.neighbors[s * self.links..(s + 1) * self.links]
    }
}

/// The logical facets with their extreme points, in flat lists (#254).
#[derive(Clone, Default)]
pub(crate) struct Faces {
    /// Per face, its extreme points, ascending.
    pub(crate) vertices: Lists<u32>,
    /// Per face, its neighboring faces, ascending.
    pub(crate) neighbors: Lists<u32>,
}

/// One face of [`Faces`].
#[derive(Clone, Copy)]
pub(crate) struct Face<'a> {
    pub(crate) vertices: &'a [u32],
}

impl Faces {
    pub(crate) fn len(&self) -> usize {
        self.vertices.len()
    }

    pub(crate) fn get(&self, i: usize) -> Face<'_> {
        Face {
            vertices: self.vertices.get(i),
        }
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = Face<'_>> + '_ {
        (0..self.len()).map(move |i| self.get(i))
    }
}

/// The order classification leaves the faces in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaceOrder {
    /// The public order: the boundary cycle of D = 2 (design §5).
    Public,
    /// Construction order; the public numbering is computed on first use.
    Classification,
}

/// The hull after classification, with its faces in the order `order`
/// names.
pub(crate) struct Classified<'a> {
    pub(crate) input: Input<'a>,
    pub(crate) faces: Faces,
    pub(crate) order: FaceOrder,
    pub(crate) simplices: Complex,
    pub(crate) vertices: Vec<u32>,
    pub(crate) coplanar_points: Vec<u32>,
    pub(crate) interior_points: Vec<u32>,
}

/// The chain edge `point` lies on, or `None` when it is strictly inside.
///
/// `cycle` is the extreme vertices counterclockwise. The fan from
/// `cycle[0]` names the triangle, and one orientation against that
/// triangle's outer edge separates an interior point from a boundary point.
fn polygon_edge(
    input: &Input<'_>,
    cycle: &[u32],
    point: u32,
) -> Result<Option<usize>, ConvexHullError> {
    let n = cycle.len();
    debug_assert!(n >= 3, "a polygon has at least three vertices");
    let turn = |a: usize, b: usize| input.orient(&[cycle[a], cycle[b], point]);
    if turn(0, 1)? == Sign::Zero {
        return Ok(Some(0));
    }
    debug_assert_ne!(
        turn(0, 1)?,
        Sign::Negative,
        "a point is outside the polygon"
    );
    if turn(0, n - 1)? == Sign::Zero {
        return Ok(Some(n - 1));
    }
    let mut lo = 1usize;
    let mut hi = n - 1;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if turn(0, mid)? == Sign::Positive {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let side = input.orient(&[cycle[lo], cycle[(lo + 1) % n], point])?;
    debug_assert_ne!(side, Sign::Negative, "a point is outside the polygon");
    Ok(if side == Sign::Zero { Some(lo) } else { None })
}

/// Builds and classifies the hull of an accepted input.
pub(crate) fn classify(input: Input<'_>) -> Result<Classified<'_>, ConvexHullError> {
    let built = SimplicialHull::build(input)?;
    classify_built(built, &mut FoundFaces::default())
}

/// The extreme points of the faces found so far in one build, each keyed by
/// the face's candidates as ascending input indices (#394). The extreme
/// points of a point set depend on the set alone, so a face that several
/// facets share is found once, from whichever reaches it first, and a face
/// reached with other candidates is only found again.
#[derive(Default)]
struct FoundFaces {
    extremes: FaceMemo,
    /// The buffers of one face, kept from one face to the next, so that a
    /// face allocates nothing (#530, #531): the chain that finds the
    /// extreme points of a face in a plane, those points as places among
    /// the candidates, and the buffers of each level of the recursion, one
    /// set per hull being classified at a time.
    chain: ChainWork,
    places: Vec<u32>,
    levels: Vec<Level>,
}

/// The extreme points of the faces found so far, by their candidates as
/// input indices, in flat lists: a key and a list allocated per face
/// (#531).
#[derive(Default)]
struct FaceMemo {
    /// By the fingerprint of a key, the last face stored with it; `next`
    /// chains the faces of one fingerprint.
    first: HashMap<u64, u32>,
    next: Vec<u32>,
    keys: Lists<u32>,
    found: Lists<u32>,
}

impl FaceMemo {
    fn get(&self, key: &[u32]) -> Option<&[u32]> {
        let mut at = *self.first.get(&fingerprint(key))?;
        loop {
            if self.keys.get(at as usize) == key {
                return Some(self.found.get(at as usize));
            }
            at = self.next[at as usize];
            if at == u32::MAX {
                return None;
            }
        }
    }

    fn insert(&mut self, key: &[u32], found: impl IntoIterator<Item = u32>) {
        let at = self.keys.len() as u32;
        self.keys.push(key);
        self.found.push_iter(found);
        let previous = self.first.insert(fingerprint(key), at);
        self.next.push(previous.unwrap_or(u32::MAX));
    }
}

/// The buffers of one level of the recursion of [`found_vertices`]: what
/// it fills for the hull it classifies, what [`face_extremes`] fills for
/// one face of that hull, and the sub-hull of that face with what is found
/// on it.
#[derive(Default)]
struct Level {
    merge: MergeWork,
    skipped: Vec<bool>,
    zeros: ZerosWork,
    zero_points: Lists<u32>,
    candidates: Vec<u32>,
    found: Vec<u32>,
    projected: Vec<f64>,
    key: Vec<u32>,
    basis_rows: Vec<f64>,
    input: InputWork,
    hull: HullWork,
    sub: Found,
}

/// Each facet's extreme points, when they are not the facet's vertices: a
/// single simplex with no other point on its plane has none of its own.
#[derive(Default)]
struct Extremes {
    lists: Lists<u32>,
    own: Vec<bool>,
}

impl Extremes {
    fn len(&self) -> usize {
        self.own.len()
    }

    fn get(&self, g: usize) -> Option<&[u32]> {
        self.own[g].then(|| self.lists.get(g))
    }
}

/// The vertices of the hull of the candidates of a face, projected to
/// `projected` (dimension `d` >= 3), into `faces.places`, ascending:
/// nothing else of its classification is read. `global` maps the
/// candidates to the input's indices. The hull, its input, and what is
/// found on it use the storage of `level`.
fn sub_vertices(
    d: usize,
    projected: &[f64],
    faces: &mut FoundFaces,
    level: &mut Level,
    global: &[u32],
) -> Result<(), ConvexHullError> {
    #[cfg(test)]
    tests::SUB_HULLS.with(|c| c.set(c.get() + 1));
    let input = Input::of_distinct(
        d,
        projected,
        core::mem::take(&mut level.input),
        &mut level.basis_rows,
    )?;
    let hull = SimplicialHull::build_with(input, core::mem::take(&mut level.hull))?;
    debug_assert!(!hull.strict_edges, "a hull of D >= 3 is not a polygon");
    // Taken in place: an empty `Found` allocates its lists.
    found_vertices_into(&hull, faces, Some(global), &mut level.sub)?;
    faces.places.clear();
    faces.places.extend_from_slice(&level.sub.vertices);
    let (input, work) = hull.into_work();
    level.hull = work;
    level.input = input.into_work();
    Ok(())
}

/// Classifies a strict polygon from its extreme cycle.
///
/// The cycle vertices are the extreme points. A representative that missed
/// the chain is on one edge or strictly inside; it is not a new vertex. One
/// the build proved interior (design §6) is not tested against the cycle.
fn classify_chain(hull: SimplicialHull<'_>) -> Result<Classified<'_>, ConvexHullError> {
    let n = hull.polygon.len();
    // The lists below are filtered from the representatives, in their
    // order, and published ascending without a sort.
    debug_assert!(
        hull.input.representatives.is_sorted(),
        "the representatives are ascending"
    );
    // What the build knows of each representative.
    #[derive(Clone, Copy, PartialEq)]
    enum Known {
        Nothing,
        OnCycle,
        ProvedInterior,
    }
    let mut known = vec![Known::Nothing; hull.input.representative.len()];
    for &v in &hull.polygon {
        known[v as usize] = Known::OnCycle;
    }
    for &p in &hull.proved_interior {
        debug_assert!(
            known[p as usize] != Known::OnCycle,
            "a discarded point is not a vertex"
        );
        known[p as usize] = Known::ProvedInterior;
    }
    // One pass over the ascending representatives writes the three lists,
    // each ascending, without a sort (#411). The cycle's vertices and the
    // points off it are sized up front; a list collected through a filter
    // grew by reallocation (#440).
    let representatives = &hull.input.representatives;
    let mut vertices = Vec::with_capacity(n);
    let mut coplanar_points = Vec::new();
    let mut interior_points = Vec::with_capacity(representatives.len().saturating_sub(n));
    for &point in representatives {
        match known[point as usize] {
            Known::OnCycle => vertices.push(point),
            Known::ProvedInterior => {
                debug_assert!(
                    polygon_edge(&hull.input, &hull.polygon, point)?.is_none(),
                    "point {point} was discarded and is not strictly inside"
                );
                interior_points.push(point);
            }
            Known::Nothing => {
                if polygon_edge(&hull.input, &hull.polygon, point)?.is_some() {
                    coplanar_points.push(point);
                } else {
                    interior_points.push(point);
                }
            }
        }
    }
    debug_assert_eq!(vertices.len(), n, "every cycle vertex is a representative");

    // The public order (design §5): edge `i` joins `cycle[i]` and
    // `cycle[i + 1]`, where `cycle` is the counterclockwise polygon from
    // its smallest vertex. Its neighbors are edges `i - 1` and `i + 1`. One
    // pass writes every list in that order; nothing is sorted.
    let first = (0..n).min_by_key(|&i| hull.polygon[i]).unwrap_or(0);
    // `cycle(i)` for `i <= n`, without a division per edge.
    let cycle = |i: usize| {
        let at = first + i;
        hull.polygon[if at >= n { at - n } else { at }]
    };
    let mut faces = Faces {
        vertices: Lists::with_capacity(n, 2 * n),
        neighbors: Lists::with_capacity(n, 2 * n),
    };
    let mut simplices = Complex::with_capacity(2, n);
    for i in 0..n {
        let (start, end) = (cycle(i), cycle(i + 1));
        let prev = (if i == 0 { n - 1 } else { i - 1 }) as u32;
        let next = (if i + 1 == n { 0 } else { i + 1 }) as u32;
        faces.vertices.push(&[start.min(end), start.max(end)]);
        faces.neighbors.push(&[prev.min(next), prev.max(next)]);
        // `neighbors[i]` is the simplex across the ridge opposite `vertices[i]`.
        // The tip is first, so slot 0 faces the previous edge and slot 1 the next.
        simplices.push(&[end, start], i as u32, Some(&[prev, next]));
    }
    Ok(Classified {
        input: hull.input,
        faces,
        order: FaceOrder::Public,
        simplices,
        vertices,
        coplanar_points,
        interior_points,
    })
}

/// The vertices of a built simplicial hull that is not a strict polygon,
/// with what classification reads on the way: the logical facets, the
/// simplicial vertices, the other representatives and whether each is on
/// the boundary, and each facet's extreme points (`None` when they are the
/// facet's vertices).
#[derive(Default)]
struct Found {
    groups: LogicalFacets,
    on_complex: Vec<bool>,
    others: Vec<u32>,
    on_boundary: Vec<bool>,
    extremes: Extremes,
    is_vertex: Vec<bool>,
    vertices: Vec<u32>,
}

/// Finds the vertices of `hull`: the extreme points of each logical facet,
/// through `faces`. `global` maps the hull's indices to the input's; `None`
/// when they are the input's.
fn found_vertices(
    hull: &SimplicialHull<'_>,
    faces: &mut FoundFaces,
    global: Option<&[u32]>,
) -> Result<Found, ConvexHullError> {
    let mut found = Found::default();
    found_vertices_into(hull, faces, global, &mut found)?;
    Ok(found)
}

/// [`found_vertices`] into `out`, whose lists keep their storage, with the
/// buffers of one level of `faces`.
fn found_vertices_into(
    hull: &SimplicialHull<'_>,
    faces: &mut FoundFaces,
    global: Option<&[u32]>,
    out: &mut Found,
) -> Result<(), ConvexHullError> {
    let mut level = faces.levels.pop().unwrap_or_default();
    let result = found_at(hull, faces, &mut level, global, out);
    faces.levels.push(level);
    result
}

fn found_at(
    hull: &SimplicialHull<'_>,
    faces: &mut FoundFaces,
    level: &mut Level,
    global: Option<&[u32]>,
    out: &mut Found,
) -> Result<(), ConvexHullError> {
    let Found {
        groups,
        on_complex,
        others,
        on_boundary,
        extremes,
        is_vertex,
        vertices,
    } = out;
    merge_into(hull, groups, &mut level.merge)?;
    let groups = &*groups;
    // A group's member simplices, in outward order.
    let members = |g: usize| {
        groups
            .groups
            .get(g)
            .iter()
            .filter_map(|id| hull.facets.get(*id))
            .map(|s| s.vertices())
    };

    // Simplicial vertices.
    let points = hull.input.representative.len();
    on_complex.clear();
    on_complex.resize(points, false);
    for g in 0..groups.groups.len() {
        for vertices in members(g) {
            for &v in vertices {
                on_complex[v as usize] = true;
            }
        }
    }

    // Distance signs of the other representatives against every group.
    // Points construction proved strictly inside are interior already
    // (design §3) and skip the scan.
    let skipped = &mut level.skipped;
    skipped.clear();
    skipped.resize(points, false);
    for &p in &hull.proved_interior {
        debug_assert!(!on_complex[p as usize], "a dropped point is not a vertex");
        // A point leaves every outside set when it is proved interior, and
        // a lost vertex is in none, so no later plan meets it again.
        debug_assert!(!skipped[p as usize], "point {p} was proved interior twice");
        skipped[p as usize] = true;
    }
    others.clear();
    others.extend(
        hull.input
            .representatives
            .iter()
            .copied()
            .filter(|&p| !on_complex[p as usize] && !skipped[p as usize]),
    );
    distance_zeros_into(
        hull,
        groups,
        others,
        on_boundary,
        &mut level.zero_points,
        &mut level.zeros,
    )?;

    // Extreme points of each group; `None` when they are the group's
    // vertices, a single simplex with no other point on its plane.
    let count = groups.groups.len();
    extremes.lists.clear();
    extremes.lists.reserve(count, 0);
    extremes.own.clear();
    extremes.own.reserve(count);
    // One list of candidates and one of extreme points for every facet.
    let mut candidates = core::mem::take(&mut level.candidates);
    let mut found = core::mem::take(&mut level.found);
    for (g, group) in groups.groups.iter().enumerate() {
        if group.len() == 1 && level.zero_points.get(g).is_empty() {
            extremes.lists.push(&[]);
            extremes.own.push(false);
            continue;
        }
        candidates.clear();
        candidates.extend_from_slice(groups.vertices.get(g));
        candidates.extend_from_slice(level.zero_points.get(g));
        candidates.sort_unstable();
        candidates.dedup();
        let plane = members(g).next().unwrap_or(&[]);
        face_extremes(
            &hull.input,
            plane,
            &candidates,
            faces,
            level,
            global,
            &mut found,
        )?;
        extremes.lists.push(&found);
        extremes.own.push(true);
    }
    level.candidates = candidates;
    level.found = found;

    is_vertex.clear();
    is_vertex.resize(points, false);
    for g in 0..count {
        for &v in extremes.get(g).unwrap_or(groups.vertices.get(g)) {
            is_vertex[v as usize] = true;
        }
    }
    vertices.clear();
    vertices.extend(
        hull.input
            .representatives
            .iter()
            .copied()
            .filter(|&p| is_vertex[p as usize]),
    );
    Ok(())
}

/// Classifies a built simplicial hull. Its `proved_interior` points go to
/// `interior_points` without a scan.
fn classify_built<'a>(
    hull: SimplicialHull<'a>,
    faces: &mut FoundFaces,
) -> Result<Classified<'a>, ConvexHullError> {
    if hull.strict_edges {
        return classify_chain(hull);
    }
    let Found {
        groups,
        on_complex,
        others,
        on_boundary,
        extremes,
        is_vertex,
        vertices,
    } = found_vertices(&hull, faces, None)?;
    let d = hull.input.dim();
    // Every representative is a vertex, on the complex, on the boundary
    // by the scan, or interior: either proved during construction or
    // scanned strictly inside. One pass over the ascending representatives
    // lists the points of each class in order, without sorting the
    // interior points, which are almost all of a large input (#213).
    debug_assert!(
        hull.input.representatives.is_sorted(),
        "the representatives are ascending"
    );
    let mut boundary = vec![false; hull.input.representative.len()];
    for (&p, &b) in others.iter().zip(&on_boundary) {
        boundary[p as usize] = b;
    }
    let mut coplanar_points: Vec<u32> = Vec::new();
    let mut interior_points: Vec<u32> = Vec::new();
    for &p in &hull.input.representatives {
        let i = p as usize;
        if on_complex[i] || boundary[i] {
            if !is_vertex[i] {
                coplanar_points.push(p);
            }
        } else {
            debug_assert!(!is_vertex[i], "vertex {p} is on no facet");
            interior_points.push(p);
        }
    }
    debug_assert_eq!(
        interior_points.len(),
        hull.proved_interior.len() + on_boundary.iter().filter(|&&b| !b).count(),
        "an interior point is proved during construction or scanned inside"
    );

    // Boundary simplices: kept as built, or re-triangulated by placing.
    // At least one simplex per face, and exactly one in general position.
    let mut simplices = Complex::with_capacity(d, extremes.len());
    let mut faces = Faces {
        vertices: Lists::with_capacity(extremes.len(), extremes.len() * d),
        neighbors: Lists::with_capacity(extremes.len(), extremes.len() * d),
    };
    // Simplices kept as built, by their index here and their construction id.
    let mut kept: Vec<(u32, FacetId)> = Vec::with_capacity(extremes.len());
    // One workspace for the placing triangulation of every face (#524).
    let mut placing = Placing::default();
    for g in 0..extremes.len() {
        let extreme = extremes.get(g);
        let group = groups.groups.get(g);
        let group_vertices = groups.vertices.get(g);
        let first = simplices.len() as u32;
        // A single simplex whose vertices are all extreme is kept; every
        // other facet is re-triangulated by placing, so facets that share a
        // lower face split it the same way.
        let single = group.len() == 1;
        match extreme {
            Some(extreme) if !(single && extreme == group_vertices) => {
                let q = vertices
                    .iter()
                    .copied()
                    .find(|v| extreme.binary_search(v).is_err())
                    .unwrap_or(extreme[0]);
                let (len, placed) =
                    placing.run(d, Some(d - 1), |i| hull.input.point(i), extreme)?;
                debug_assert_eq!(len, d, "a facet spans D - 1 dimensions");
                for simplex in placed.chunks_exact(len) {
                    simplices.push(&oriented(&hull.input, simplex, q)?, g as u32, None);
                }
                faces.vertices.push(extreme);
            }
            _ => {
                if let Some(&id) = group.first() {
                    if let Some(s) = hull.facets.get(id) {
                        kept.push((first, id));
                        simplices.push(s.vertices(), g as u32, None);
                    }
                }
                faces.vertices.push(group_vertices);
            }
        }
        faces.neighbors.push(groups.neighbors.get(g));
    }
    // A kept simplex has its construction vertex order, so its neighbor
    // across slot i during construction, when kept too, is its neighbor
    // across slot i here: each ridge of the closed complex has two sides.
    let mut index_of: SlotMarks<u32> = SlotMarks::default();
    for &(s, id) in &kept {
        index_of.insert(id, s);
    }
    for &(s, id) in &kept {
        if let Some(facet) = hull.facets.get(id) {
            for (slot, neighbor) in facet.neighbors().enumerate() {
                if let Some(other) = index_of.get(neighbor) {
                    simplices.neighbors_mut(s as usize)[slot] = other;
                }
            }
        }
    }
    #[cfg(debug_assertions)]
    let mut paired_all = (simplices.clone(), faces.clone());
    link_neighbors(d, &mut simplices, &faces);
    #[cfg(debug_assertions)]
    {
        paired_all.0.neighbors.fill(UNLINKED);
        link_neighbors(d, &mut paired_all.0, &paired_all.1);
        debug_assert!(
            simplices.neighbors == paired_all.0.neighbors,
            "kept construction links differ from pairing every ridge"
        );
    }

    Ok(Classified {
        input: hull.input,
        faces,
        order: FaceOrder::Classification,
        simplices,
        vertices,
        coplanar_points,
        interior_points,
    })
}

/// Per point, whether some group's distance is zero, and per group its
/// points at distance zero, ascending. A point strictly inside its facet's
/// polygon (D = 3) is on the boundary and in no list (#504).
#[cfg(test)]
type Zeros = (Vec<bool>, Lists<u32>);

/// The working lists of [`distance_zeros_into`], kept from one hull to the
/// next (#531).
#[derive(Default)]
struct ZerosWork {
    zero_pairs: Vec<(u32, u32)>,
    starts: Vec<Option<u32>>,
    numbers: Vec<(u32, u32)>,
    scanned: Vec<u32>,
    scanned_at: Vec<usize>,
    sides: Vec<Option<Sign>>,
    /// The places among the others, by start group and ascending within
    /// one: `by_start[offsets[g]..offsets[g + 1]]` for group `g`. `ks` is
    /// the places of one start group.
    by_start: Vec<u32>,
    offsets: Vec<u32>,
    ks: Vec<u32>,
    tested: Vec<usize>,
    stack: Vec<u32>,
    points: Vec<u32>,
    zeros: Vec<(u32, u32)>,
    polygon: PolygonWork,
}

/// The buffers of [`facet_polygon`]: the facet's vertices on a coordinate
/// plane, the chain that orders their extremes, and the polygon's edges.
#[derive(Default)]
struct PolygonWork {
    rows: Vec<f64>,
    chain: ChainWork,
    edges: Vec<Edge>,
}

/// [`distance_zeros_into`] with lists of its own.
#[cfg(test)]
fn distance_zeros(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    others: &[u32],
) -> Result<Zeros, ConvexHullError> {
    let (mut on_boundary, mut zero_points) = (Vec::new(), Lists::default());
    distance_zeros_into(
        hull,
        groups,
        others,
        &mut on_boundary,
        &mut zero_points,
        &mut ZerosWork::default(),
    )?;
    Ok((on_boundary, zero_points))
}

/// The exact distance-zero signs of `others` against every group, into
/// `on_boundary` and `zero_points`. Points construction proved strictly
/// inside are not among `others`.
fn distance_zeros_into(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    others: &[u32],
    on_boundary: &mut Vec<bool>,
    zero_points: &mut Lists<u32>,
    work: &mut ZerosWork,
) -> Result<(), ConvexHullError> {
    let ZerosWork {
        zero_pairs,
        starts,
        numbers,
        scanned,
        scanned_at,
        sides,
        by_start,
        offsets,
        ks,
        tested,
        stack,
        points,
        zeros,
        polygon,
    } = work;
    on_boundary.clear();
    on_boundary.resize(others.len(), false);
    // (group, point) for each zero, sorted into one list per group at the
    // end: a list per group allocated per facet with a point on it (#530).
    zero_pairs.clear();
    // The group of a recorded number of each other point, if a simplex of
    // the final complex carries one, and the points with none, which are
    // scanned against every group.
    recorded_groups_into(hull, groups, others, starts, numbers);
    scanned.clear();
    scanned.extend(
        others
            .iter()
            .zip(starts.iter())
            .filter(|&(_, start)| start.is_none())
            .map(|(&p, _)| p),
    );
    scanned_at.clear();
    scanned_at.extend((0..others.len()).filter(|&k| starts[k].is_none()));
    if !scanned.is_empty() {
        sides.clear();
        sides.resize(scanned.len(), None);
        for (g, group) in groups.groups.iter().enumerate() {
            let Some(simplex) = group.first().and_then(|id| hull.facets.get(*id)) else {
                continue;
            };
            sides.fill(None);
            if let Some(cull) = simplex.cull() {
                let (rows, stride) = hull.input.rows();
                let origin = hull.input.point(simplex.vertices()[0]);
                cull.mark_sides(origin, rows, stride, scanned, sides);
            }
            for (k, &p) in scanned.iter().enumerate() {
                let side = match sides[k] {
                    Some(proved) => {
                        // `side` checks its own proof against the
                        // orientation in debug builds, so this checks the
                        // scan's.
                        #[cfg(debug_assertions)]
                        debug_assert_eq!(
                            hull.side(simplex, p)?,
                            proved,
                            "the scan proved the wrong side of point {p}"
                        );
                        proved
                    }
                    None => hull.side(simplex, p)?,
                };
                debug_assert!(
                    side != Sign::Positive,
                    "a point is outside the finished hull"
                );
                if side == Sign::Zero {
                    on_boundary[scanned_at[k]] = true;
                    zero_pairs.push((g as u32, p));
                }
            }
        }
    }
    // A recorded point is on its start group's plane. The groups at
    // distance zero from it are connected through neighbors, and every
    // other group is strictly negative (design §3). The points are taken by
    // start group, and each group's points are tested against each of its
    // neighbors together, through that neighbor's cull plane, as the scan
    // above does; only what the plane does not prove is evaluated. One
    // point at a time, that walk was most of classifying a hull whose
    // points lie on its faces (#468). A point found on a neighbor's plane
    // walks on from there as before.
    let side_of = |g: u32, p: u32| -> Result<Sign, ConvexHullError> {
        match groups
            .groups
            .get(g as usize)
            .first()
            .and_then(|id| hull.facets.get(*id))
        {
            Some(simplex) => hull.side(simplex, p),
            None => {
                debug_assert!(false, "a group has a live simplex");
                Ok(Sign::Negative)
            }
        }
    };
    // The places of the points of each start group, ascending, the groups
    // in their order, by a counting sort: a list per group allocated per
    // group (#531), and a sort of the pairs cost 5% of a hull whose points
    // lie on its faces.
    let count = groups.groups.len();
    offsets.clear();
    offsets.resize(count + 2, 0);
    for start in starts.iter().flatten() {
        offsets[*start as usize + 2] += 1;
    }
    for g in 2..offsets.len() {
        offsets[g] += offsets[g - 1];
    }
    by_start.clear();
    by_start.resize(offsets[count + 1] as usize, 0);
    for (k, start) in starts.iter().enumerate() {
        if let Some(start) = *start {
            let slot = &mut offsets[start as usize + 1];
            by_start[*slot as usize] = k as u32;
            *slot += 1;
        }
    }
    let (rows, stride) = hull.input.rows();
    tested.clear();
    tested.resize(groups.groups.len(), usize::MAX);
    // `zeros`: per point of the start group (by its place in `points`), a
    // neighbor whose plane holds it.
    for start in 0..count as u32 {
        let run = &by_start[offsets[start as usize] as usize..offsets[start as usize + 1] as usize];
        if run.is_empty() {
            continue;
        }
        ks.clear();
        ks.extend_from_slice(run);
        // D = 3: a point strictly inside the polygon of its facet's extreme
        // vertices is in the facet's relative interior, so it lies on no
        // other facet's plane and is not extreme (design §3). It is on the
        // boundary, and neither the walk nor the facet's extremes need it
        // (#504).
        if hull.input.dim() == 3 {
            if let Some(inner) = facet_polygon(hull, groups, start, polygon)? {
                #[cfg(debug_assertions)]
                for &k in ks.iter() {
                    let p = others[k as usize];
                    if inner.contains(hull.input.point(p)) {
                        for g in 0..groups.groups.len() as u32 {
                            debug_assert_eq!(
                                side_of(g, p)?,
                                if g == start {
                                    Sign::Zero
                                } else {
                                    Sign::Negative
                                },
                                "point {p} inside facet {start} against facet {g}"
                            );
                        }
                    }
                }
                ks.retain(|&k| {
                    let inside = inner.contains(hull.input.point(others[k as usize]));
                    on_boundary[k as usize] |= inside;
                    !inside
                });
                if ks.is_empty() {
                    continue;
                }
            }
        }
        points.clear();
        points.extend(ks.iter().map(|&k| others[k as usize]));
        zeros.clear();
        for &n in groups.neighbors.get(start as usize) {
            let Some(simplex) = groups
                .groups
                .get(n as usize)
                .first()
                .and_then(|id| hull.facets.get(*id))
            else {
                debug_assert!(false, "a group has a live simplex");
                continue;
            };
            sides.clear();
            sides.resize(points.len(), None);
            if let Some(cull) = simplex.cull() {
                let origin = hull.input.point(simplex.vertices()[0]);
                cull.mark_sides(origin, rows, stride, points, sides);
            }
            for (i, &p) in points.iter().enumerate() {
                let side = match sides[i] {
                    Some(proved) => {
                        #[cfg(debug_assertions)]
                        debug_assert_eq!(
                            hull.side(simplex, p)?,
                            proved,
                            "the scan proved the wrong side of point {p}"
                        );
                        proved
                    }
                    None => hull.side(simplex, p)?,
                };
                debug_assert!(
                    side != Sign::Positive,
                    "a point is outside the finished hull"
                );
                if side == Sign::Zero {
                    zeros.push((i as u32, n));
                }
            }
        }
        zeros.sort_unstable();
        let mut next_zero = 0;
        for (i, &k) in ks.iter().enumerate() {
            let k = k as usize;
            let p = points[i];
            #[cfg(debug_assertions)]
            debug_assert_eq!(
                side_of(start, p)?,
                Sign::Zero,
                "point {p} is recorded on a plane it is off"
            );
            on_boundary[k] = true;
            zero_pairs.push((start, p));
            tested[start as usize] = k;
            for &n in groups.neighbors.get(start as usize) {
                tested[n as usize] = k;
            }
            stack.clear();
            while next_zero < zeros.len() && zeros[next_zero].0 == i as u32 {
                let n = zeros[next_zero].1;
                zero_pairs.push((n, p));
                stack.push(n);
                next_zero += 1;
            }
            while let Some(g) = stack.pop() {
                for &n in groups.neighbors.get(g as usize) {
                    if tested[n as usize] == k {
                        continue;
                    }
                    tested[n as usize] = k;
                    let side = side_of(n, p)?;
                    debug_assert!(
                        side != Sign::Positive,
                        "a point is outside the finished hull"
                    );
                    if side == Sign::Zero {
                        zero_pairs.push((n, p));
                        stack.push(n);
                    }
                }
            }
            // Debug builds check the walk against every group.
            #[cfg(debug_assertions)]
            for g in 0..groups.groups.len() as u32 {
                if tested[g as usize] != k {
                    debug_assert_eq!(
                        side_of(g, p)?,
                        Sign::Negative,
                        "the walk from a record missed a facet through point {p}"
                    );
                }
            }
        }
    }
    zero_pairs.sort_unstable();
    zero_points.clear();
    zero_points.reserve(groups.groups.len(), zero_pairs.len());
    let mut rest = &zero_pairs[..];
    for g in 0..groups.groups.len() as u32 {
        let take = rest.iter().take_while(|&&(group, _)| group == g).count();
        zero_points.push_iter(rest[..take].iter().map(|&(_, p)| p));
        rest = &rest[take..];
    }
    Ok(())
}

/// The polygon of the extreme vertices of a facet of D = 3, in two of the
/// three coordinates.
struct FacetPolygon<'w> {
    /// The coordinates kept, ascending.
    axes: [usize; 2],
    edges: &'w [Edge],
}

impl FacetPolygon<'_> {
    /// Whether `point`, on the facet's plane, is strictly inside the
    /// polygon. The projection drops a coordinate on which the plane's
    /// normal is not zero, so it maps the plane onto the coordinate plane
    /// one to one, and strict insideness carries over.
    fn contains(&self, point: &[f64]) -> bool {
        clearly_inside(self.edges, &[point[self.axes[0]], point[self.axes[1]]])
    }
}

/// The polygon of [`polygon::extremes_polygon`] on the vertices of group
/// `g` (D = 3), projected to the coordinate plane on which the facet's
/// first simplex has a nonzero orientation; `None` when the polygon has
/// fewer than three vertices. It is built in the buffers of `work`.
fn facet_polygon<'w>(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    g: u32,
    work: &'w mut PolygonWork,
) -> Result<Option<FacetPolygon<'w>>, ConvexHullError> {
    let PolygonWork { rows, chain, edges } = work;
    let Some(simplex) = groups
        .groups
        .get(g as usize)
        .first()
        .and_then(|id| hull.facets.get(*id))
    else {
        debug_assert!(false, "a group has a live simplex");
        return Ok(None);
    };
    let [a, b, c] = [0, 1, 2].map(|i| hull.input.point(simplex.vertices()[i]));
    let (u, v) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let normal = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    // The coordinate dropped: the largest rounded normal component first,
    // then the others, until the projected simplex is not flat.
    let mut drops = [0, 1, 2];
    drops.sort_by(|&i, &j| normal[j].abs().total_cmp(&normal[i].abs()));
    for drop in drops {
        let axes = match drop {
            0 => [1, 2],
            1 => [0, 2],
            _ => [0, 1],
        };
        let project = |p: &[f64]| [p[axes[0]], p[axes[1]]];
        let (pa, pb, pc) = (project(a), project(b), project(c));
        if orient(&[&pa, &pb, &pc])? == Sign::Zero {
            continue;
        }
        let vertices = groups.vertices.get(g as usize);
        rows.clear();
        rows.extend(vertices.iter().flat_map(|&v| project(hull.input.point(v))));
        let found = polygon::extremes_polygon(Rows(rows), vertices.len() as u32, chain, edges)?;
        return Ok(found.then_some(FacetPolygon { axes, edges }));
    }
    debug_assert!(false, "a simplex of D = 3 projects to a triangle");
    Ok(None)
}

/// Per point of `others`, the group of a simplex of the final complex whose
/// plane number construction recorded for that point, if any.
#[cfg(test)]
fn recorded_groups(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    others: &[u32],
) -> Vec<Option<u32>> {
    let mut starts = Vec::new();
    recorded_groups_into(hull, groups, others, &mut starts, &mut Vec::new());
    starts
}

/// [`recorded_groups`] into `starts`, with `numbers` as its scratch.
fn recorded_groups_into(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    others: &[u32],
    starts: &mut Vec<Option<u32>>,
    numbers: &mut Vec<(u32, u32)>,
) {
    starts.clear();
    starts.resize(others.len(), None);
    if hull.records.is_empty() {
        return;
    }
    // Simplices with one number lie in one plane, so in one logical facet.
    numbers.clear();
    numbers.extend(groups.groups.iter().enumerate().flat_map(|(g, group)| {
        group
            .iter()
            .filter_map(|id| hull.facets.get(*id))
            .map(move |s| (s.number(), g as u32))
    }));
    numbers.sort_unstable();
    for (start, &p) in starts.iter_mut().zip(others) {
        *start = hull.records.numbers(p).find_map(|n| {
            numbers
                .binary_search_by_key(&n, |&(number, _)| number)
                .ok()
                .map(|i| numbers[i].1)
        });
    }
}

/// Extreme points of `candidates`, which lie on the hyperplane through the
/// D affinely independent points `plane`.
fn face_extremes(
    input: &Input<'_>,
    plane: &[u32],
    candidates: &[u32],
    faces: &mut FoundFaces,
    level: &mut Level,
    global: Option<&[u32]>,
    extremes: &mut Vec<u32>,
) -> Result<(), ConvexHullError> {
    let mut key = core::mem::take(&mut level.key);
    let result = face_extremes_with(
        input, plane, candidates, faces, level, global, &mut key, extremes,
    );
    level.key = key;
    result
}

/// [`face_extremes`] with `key` as the buffer of the face's key.
#[allow(clippy::too_many_arguments)]
fn face_extremes_with(
    input: &Input<'_>,
    plane: &[u32],
    candidates: &[u32],
    faces: &mut FoundFaces,
    level: &mut Level,
    global: Option<&[u32]>,
    key: &mut Vec<u32>,
    extremes: &mut Vec<u32>,
) -> Result<(), ConvexHullError> {
    let d = input.dim();
    extremes.clear();
    if d == 1 {
        extremes.extend_from_slice(candidates);
        return Ok(());
    }
    // The candidates as input indices. Each level's candidates ascend and
    // map to ascending indices, so the key ascends too. A face of the top
    // level is reached once, from its own facet, so only the faces below it
    // are remembered, and only a level whose sub-hull classifies faces of
    // its own (D - 1 >= 3) needs the key as its map to the input.
    key.clear();
    match global {
        Some(global) => key.extend(candidates.iter().map(|&c| global[c as usize])),
        None if d >= 4 => key.extend_from_slice(candidates),
        None => {}
    }
    debug_assert!(key.is_sorted(), "a face's key ascends");
    // Tests compare the published result with and without the memo.
    #[cfg(test)]
    let forget = !tests::FACES_REMEMBERED.with(core::cell::Cell::get);
    #[cfg(not(test))]
    let forget = false;
    let remember = global.is_some() && !forget;
    if remember {
        if let Some(found) = faces.extremes.get(key) {
            // Back to this level's indices, through the key's order.
            extremes.extend(found.iter().map(|v| {
                let at = key.binary_search(v);
                debug_assert!(
                    at.is_ok(),
                    "a face's extreme points are among its candidates"
                );
                candidates[at.unwrap_or_default()]
            }));
            return Ok(());
        }
    }
    // An axis along which the hyperplane is not vertical: its cofactor is
    // nonzero, so dropping that coordinate is a bijection of the hyperplane.
    let plane_points: Small<&[f64], 11> = plane.iter().map(|&v| input.point(v)).collect();
    let mut unit: Small<f64, 11> = core::iter::repeat_n(0.0, d).collect();
    let mut axis = None;
    for j in 0..d {
        unit[j] = 1.0;
        let sign = orient_direction(&plane_points, &unit)?;
        unit[j] = 0.0;
        if sign != Sign::Zero {
            axis = Some(j);
            break;
        }
    }
    let Some(axis) = axis else {
        debug_assert!(false, "a hyperplane has a nonzero cofactor");
        extremes.extend_from_slice(candidates);
        return Ok(());
    };
    let mut projected = core::mem::take(&mut level.projected);
    projected.clear();
    projected.extend(candidates.iter().flat_map(|&p| {
        input
            .point(p)
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != axis)
            .map(|(_, &x)| x)
    }));
    let placed = sub_extremes(d - 1, &projected, faces, level, key)
        .map(|places| extremes.extend(places.iter().map(|&i| candidates[i as usize])));
    level.projected = projected;
    placed?;
    if remember {
        faces
            .extremes
            .insert(key, extremes.iter().map(|&v| global_of(global, v)));
    }
    Ok(())
}

/// The extreme points of the candidates of one face, projected to
/// `projected` (dimension `d`), as places among the candidates, ascending.
///
/// For `d == 2` the chain of strict left turns finds them on the projected
/// candidates as they are, through buffers kept from one face to the next
/// (#530). A whole hull of D = 2 gave the same points and allocated for
/// each face. Its acceptance has nothing to find here: the projection is
/// one to one on the face's plane, so the candidates stay distinct, and
/// the face's basis keeps them from one line.
fn sub_extremes<'f>(
    d: usize,
    projected: &[f64],
    faces: &'f mut FoundFaces,
    level: &mut Level,
    key: &[u32],
) -> Result<&'f [u32], ConvexHullError> {
    if d == 2 {
        #[cfg(test)]
        tests::SUB_HULLS.with(|c| c.set(c.get() + 1));
        let count = (projected.len() / 2) as u32;
        let cycle = faces.chain.strict_cycle(Rows(projected), 0..count)?;
        debug_assert!(cycle.len() >= 3, "a face spans its plane");
        faces.places.clear();
        faces.places.extend_from_slice(cycle);
        faces.places.sort_unstable();
        #[cfg(debug_assertions)]
        {
            let hull = SimplicialHull::build(accept(2, projected)?)?;
            let mut polygon = hull.polygon.clone();
            polygon.sort_unstable();
            debug_assert_eq!(
                faces.places, polygon,
                "the chain gives the sub-hull's vertices"
            );
        }
    } else {
        sub_vertices(d, projected, faces, level, key)?;
    }
    Ok(&faces.places)
}

/// The input index of `v`, an index of a level that `global` maps.
fn global_of(global: Option<&[u32]>, v: u32) -> u32 {
    global.map_or(v, |g| g[v as usize])
}

/// One simplex of the placing triangulation of the extreme points of a
/// facet, ordered so that `q`, a hull vertex off the facet, is on the
/// negative side.
fn oriented(input: &Input<'_>, simplex: &[u32], q: u32) -> Result<Small<u32, 11>, ConvexHullError> {
    let mut vertices: Small<u32, 11> = simplex.iter().copied().collect();
    let points: Small<&[f64], 12> = vertices
        .iter()
        .map(|&v| input.point(v))
        .chain([input.point(q)])
        .collect();
    if orient(&points)? == Sign::Positive && vertices.len() >= 2 {
        vertices.swap(0, 1);
    }
    Ok(vertices)
}

/// A boundary ridge of a placing triangulation, ascending: the key of
/// [`Placing`]'s boundary, on the stack for D up to 11.
type Ridge = Small<u32, 11>;

/// The placing triangulation of the extreme points of one facet or one
/// cospherical group, with its buffers kept from one run to the next
/// (#524): a run allocates nothing per simplex, per ridge, or per point.
///
/// Points are placed one at a time in index order. A point that raises the
/// affine dimension of the points placed so far is joined to every current
/// simplex. Otherwise it lies outside their hull, since it is extreme, and it
/// is joined to every boundary ridge it is beyond: the ridge's other vertex
/// and the point are strictly on opposite sides of the ridge, judged by exact
/// orientation within the current affine span.
///
/// The placing triangulation restricts to the placing triangulation of each
/// face in the same order, so two facets that share a lower face split it
/// the same way, and the boundary complex closes up.
///
/// The boundary ridges of the current complex are kept from one point to
/// the next (#393). A point that does not raise the dimension adds a simplex
/// on each boundary ridge it is beyond, and each ridge of the added
/// simplices then leaves the boundary if it was on it, or joins it: in a
/// triangulation every ridge lies on one simplex or two. A point that
/// raises the dimension cones every simplex, and the boundary is gathered
/// again. Each boundary ridge keeps the side of its opposite vertex, which
/// does not change until the dimension does.
#[derive(Default)]
pub(crate) struct Placing {
    /// The simplices, `len` vertices each, one after another.
    simplices: Vec<u32>,
    len: usize,
    /// Where the simplices are written when the dimension is raised.
    raised: Vec<u32>,
    /// Each boundary ridge, with the vertex opposite it in its one simplex
    /// and, once evaluated, that vertex's side of the ridge.
    boundary: HashMap<Ridge, (u32, Option<Sign>)>,
    /// The simplices one point adds, `len` vertices each, and their order.
    added: Vec<u32>,
    order: Vec<u32>,
    /// Coordinates on which the points placed so far project to an affinely
    /// independent basis; their count is the current affine dimension.
    axes: Vec<usize>,
    basis: Vec<u32>,
    /// The projected rows of one orientation; a `Vec` per row was most of
    /// the time left after the boundary was kept (#393).
    rows: Vec<f64>,
    /// For points that span a plane ([`Self::in_a_plane`]): their two
    /// coordinates, the chain that orders them around their polygon, and
    /// each point's neighbors in that cycle.
    plane: Vec<f64>,
    chain: ChainWork,
    before: Vec<u32>,
    after: Vec<u32>,
    /// For points that span three dimensions or more
    /// ([`Self::on_the_surface`]): the boundary ridges with their neighbors.
    surface: Surface,
}

impl Placing {
    /// The placing triangulation of `extreme` (ascending), coordinates of
    /// dimension `d` given by `point`: the number of vertices of a simplex,
    /// `r + 1` for the affine dimension `r` of the points, and the simplices
    /// one after another, each ascending, unoriented.
    ///
    /// `span` is the affine dimension of the points when the caller knows
    /// it: a facet of a hull spans D - 1 dimensions, and a cospherical
    /// group of Delaunay D. Points that span a plane take
    /// [`Self::in_a_plane`], with the same simplices in the same order.
    pub(crate) fn run<'p>(
        &mut self,
        d: usize,
        span: Option<usize>,
        point: impl Fn(u32) -> &'p [f64],
        extreme: &[u32],
    ) -> Result<(usize, &[u32]), ConvexHullError> {
        if span == Some(2) && extreme.len() > 3 && self.in_a_plane(d, &point, extreme)? {
            // Debug builds compare a face of up to 64 points with the
            // general method, which is quadratic in them.
            #[cfg(debug_assertions)]
            if extreme.len() <= 64 {
                let fast = self.simplices.clone();
                let (len, general) = self.general(d, None, &point, extreme)?;
                debug_assert_eq!(
                    (3, fast.as_slice()),
                    (len, general),
                    "the cyclic order gives the placing triangulation"
                );
                return Ok((len, general));
            }
            return Ok((3, &self.simplices));
        }
        let span = span.filter(|&span| span >= 3);
        // Debug builds compare up to 64 points with the scan of every
        // boundary ridge, which is quadratic in them.
        #[cfg(debug_assertions)]
        if span.is_some() && extreme.len() <= 64 {
            let (len, fast) = self.general(d, span, &point, extreme)?;
            let fast = (len, fast.to_vec());
            let (len, scanned) = self.general(d, None, &point, extreme)?;
            debug_assert_eq!(
                (fast.0, fast.1.as_slice()),
                (len, scanned),
                "the ridges reached through their neighbors are those of the scan"
            );
            return Ok((len, scanned));
        }
        self.general(d, span, &point, extreme)
    }

    /// The placing triangulation of points that span a plane, read from
    /// their cyclic order (#534). `false` when the points are not in
    /// strictly convex position on the two coordinates found, and nothing
    /// is decided: the caller takes [`Self::general`].
    ///
    /// The extreme points of a face are in strictly convex position. The
    /// first three make the first triangle. A point placed after them is
    /// outside the polygon of the points before it and removes none of
    /// them, so it is beyond exactly one boundary ridge: the edge between
    /// its two neighbors, in cyclic order, among the points before it. Its
    /// one simplex is that edge and itself. Removing the points from the
    /// cycle in descending order leaves, at each removal, those two
    /// neighbors.
    ///
    /// The general method tests each point against every boundary ridge,
    /// about k^2 / 2 orientations for k points: the hull of a prism over a
    /// 2,048-gon took 138 ms, nearly all of it there (#533).
    fn in_a_plane<'p>(
        &mut self,
        d: usize,
        point: &impl Fn(u32) -> &'p [f64],
        extreme: &[u32],
    ) -> Result<bool, ConvexHullError> {
        let m = extreme.len();
        // Two coordinates on which the first three points are not on one
        // line: dropping the others is then one to one on the plane.
        let first = [point(extreme[0]), point(extreme[1]), point(extreme[2])];
        let mut axes = None;
        'pairs: for i in 0..d {
            for j in i + 1..d {
                let rows = first.map(|p| [p[i], p[j]]);
                if orient(&[&rows[0][..], &rows[1][..], &rows[2][..]])? != Sign::Zero {
                    axes = Some((i, j));
                    break 'pairs;
                }
            }
        }
        let Some((i, j)) = axes else {
            return Ok(false);
        };
        self.plane.clear();
        self.plane.extend(extreme.iter().flat_map(|&v| {
            let p = point(v);
            [p[i], p[j]]
        }));
        // The points by their place in `extreme`, around their polygon.
        let cycle = self.chain.strict_cycle(Rows(&self.plane), 0..m as u32)?;
        if cycle.len() != m {
            return Ok(false);
        }
        self.before.clear();
        self.before.resize(m, 0);
        self.after.clear();
        self.after.resize(m, 0);
        for (at, &v) in cycle.iter().enumerate() {
            self.before[v as usize] = cycle[(at + m - 1) % m];
            self.after[v as usize] = cycle[(at + 1) % m];
        }
        // Each point's two neighbors among the points before it, found by
        // removing the points in descending order.
        self.added.clear();
        self.added.resize(2 * m, 0);
        for v in (3..m).rev() {
            let (a, b) = (self.before[v], self.after[v]);
            self.added[2 * v] = a.min(b);
            self.added[2 * v + 1] = a.max(b);
            self.after[a as usize] = b;
            self.before[b as usize] = a;
        }
        self.len = 3;
        self.simplices.clear();
        self.simplices.extend_from_slice(&extreme[..3]);
        for v in 3..m {
            self.simplices.extend([
                extreme[self.added[2 * v] as usize],
                extreme[self.added[2 * v + 1] as usize],
                extreme[v],
            ]);
        }
        Ok(true)
    }

    /// [`Self::run`] for any points: each point against every boundary
    /// ridge of the complex built so far. With `span`, the affine dimension
    /// of the points, the points left once the complex has that dimension
    /// go to [`Self::on_the_surface`], when there are enough of them.
    fn general<'p>(
        &mut self,
        d: usize,
        span: Option<usize>,
        point: &impl Fn(u32) -> &'p [f64],
        extreme: &[u32],
    ) -> Result<(usize, &[u32]), ConvexHullError> {
        if let Some(from) = self.scan(d, span, point, extreme)? {
            self.on_the_surface(point, &extreme[from..])?;
        }
        Ok((self.len, &self.simplices))
    }

    /// [`Self::general`] by the scan. `Some` is the place in `extreme` of
    /// the first point left to [`Self::on_the_surface`].
    fn scan<'p>(
        &mut self,
        d: usize,
        span: Option<usize>,
        point: &impl Fn(u32) -> &'p [f64],
        extreme: &[u32],
    ) -> Result<Option<usize>, ConvexHullError> {
        let Self {
            simplices,
            len,
            raised,
            boundary,
            added,
            order,
            axes,
            basis,
            rows,
            ..
        } = self;
        simplices.clear();
        *len = 1;
        axes.clear();
        basis.clear();
        boundary.clear();
        let Some((&first, rest)) = extreme.split_first() else {
            return Ok(None);
        };
        simplices.push(first);
        basis.push(first);
        // The orientation of `vertices` projected onto `axes`.
        let mut projected_sign = |vertices: &mut dyn Iterator<Item = u32>,
                                  axes: &[usize]|
         -> Result<Sign, ConvexHullError> {
            rows.clear();
            for v in vertices {
                let x = point(v);
                rows.extend(axes.iter().map(|&a| x[a]));
            }
            let refs: Small<&[f64], 11> = rows.chunks_exact(axes.len()).collect();
            Ok(orient(&refs)?)
        };
        toggle(boundary, simplices);
        for (at, &p) in rest.iter().enumerate() {
            // Does p raise the affine dimension?
            let mut raise = None;
            for j in (0..d).filter(|j| !axes.contains(j)) {
                let with_j: Small<usize, 11> = axes.iter().copied().chain([j]).collect();
                let mut vertices = basis.iter().copied().chain([p]);
                if projected_sign(&mut vertices, &with_j)? != Sign::Zero {
                    raise = Some(j);
                    break;
                }
            }
            if let Some(j) = raise {
                axes.push(j);
                basis.push(p);
                raised.clear();
                for simplex in simplices.chunks_exact(*len) {
                    raised.extend_from_slice(simplex);
                    raised.push(p);
                }
                core::mem::swap(simplices, raised);
                *len += 1;
                boundary.clear();
                for simplex in simplices.chunks_exact(*len) {
                    toggle(boundary, simplex);
                }
                if span == Some(axes.len()) && rest.len() - at > SURFACE_FROM {
                    return Ok(Some(at + 2));
                }
                continue;
            }
            // Beyond which boundary ridges of the current complex is p?
            added.clear();
            for (ridge, (opposite, known)) in boundary.iter_mut() {
                let sp = projected_sign(&mut ridge.iter().copied().chain([p]), axes)?;
                if sp == Sign::Zero {
                    continue;
                }
                let sa = match *known {
                    Some(sign) => sign,
                    None => {
                        let sign =
                            projected_sign(&mut ridge.iter().copied().chain([*opposite]), axes)?;
                        *known = Some(sign);
                        sign
                    }
                };
                if sa != Sign::Zero && sp != sa {
                    added.extend_from_slice(ridge);
                    added.push(p);
                }
            }
            // The added simplices in lexicographic order, as a sort of a
            // list of them gave.
            let simplex = |i: u32| &added[i as usize * *len..(i as usize + 1) * *len];
            order.clear();
            order.extend(0..(added.len() / *len) as u32);
            order.sort_unstable_by(|&a, &b| simplex(a).cmp(simplex(b)));
            for &i in order.iter() {
                toggle(boundary, simplex(i));
                simplices.extend_from_slice(simplex(i));
            }
        }
        Ok(None)
    }

    /// The rest of [`Self::general`] once the complex has the dimension of
    /// the points: each point of `rest` adds a simplex on each boundary
    /// ridge it is beyond, and those ridges are reached through their
    /// neighbors and not by a scan of the boundary (#540).
    ///
    /// The points are in convex position, so the complex is the hull of
    /// the points placed, and the ridges a point is beyond are connected
    /// through the faces they share. One of them is searched for from a
    /// ridge of the point before, highest first by the height of the point
    /// over a ridge's hyperplane, in units of the depth of a point inside
    /// the complex under it. Over the ridges of a convex surface that
    /// height has no peak but its greatest, which is above 1 exactly when
    /// the point is beyond the ridge. It is computed in `f64` and decides
    /// nothing: every ridge taken is tested exactly, and the search ends
    /// only at a ridge the point is beyond or when no ridge is left. Ridges
    /// in one hyperplane have one height, and a face of many ridges would
    /// be searched whole, so among ridges of one height, to about one part
    /// in a million, the one with the vertex nearest to the point is taken
    /// first. The others
    /// follow from it across shared faces. The simplices of one point are
    /// appended in lexicographic order, as the scan appends them.
    ///
    /// The scan tests each point against every boundary ridge, about n^2
    /// orientations for n points: the hull of a 1,024-gon times a square
    /// took 889 ms, and a cospherical group of 6,630 sites 1.9 s (#531).
    fn on_the_surface<'p>(
        &mut self,
        point: &impl Fn(u32) -> &'p [f64],
        rest: &[u32],
    ) -> Result<(), ConvexHullError> {
        let Self {
            simplices,
            len,
            boundary,
            added,
            order,
            axes,
            basis,
            rows,
            surface,
            ..
        } = self;
        let s = axes.len();
        let Surface {
            vertices,
            opposite,
            side,
            across,
            depth,
            inner,
            matrix,
            queued,
            tried,
            inside,
            free,
            heap,
            stack,
            visible,
            links,
        } = surface;
        // The orientation of the ridge `ridge` with `last`, on `axes`.
        let mut sign = |ridge: &[u32], last: u32| -> Result<Sign, ConvexHullError> {
            rows.clear();
            for &v in ridge.iter().chain([&last]) {
                let x = point(v);
                rows.extend(axes.iter().map(|&a| x[a]));
            }
            let refs: Small<&[f64], 11> = rows.chunks_exact(s).collect();
            Ok(orient(&refs)?)
        };
        // A point inside the complex: the mean of the first simplex.
        inner.clear();
        inner.extend(
            axes.iter()
                .map(|&a| basis.iter().map(|&v| point(v)[a]).sum::<f64>() / basis.len() as f64),
        );
        // The volume, in `f64`, of `ridge` with the point `x` (on `axes`
        // already): its sign is the side of `x`, and its size the height
        // of `x` over the ridge's hyperplane times the ridge's own measure.
        let mut volume = |ridge: &[u32], x: &[f64]| -> f64 {
            matrix.clear();
            for &v in ridge {
                let y = point(v);
                matrix.extend(axes.iter().zip(x).map(|(&a, &c)| y[a] - c));
            }
            determinant(s, matrix)
        };
        // A number of the face of ridge `id` without the vertex at `slot`.
        let mixed = |vertices: &[u32], entry: u32| {
            let (id, slot) = (entry as usize / s, entry as usize % s);
            vertices[id * s..(id + 1) * s]
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != slot)
                .fold(0_u64, |h, (_, &v)| {
                    (h ^ u64::from(v))
                        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
                        .rotate_left(29)
                })
        };

        vertices.clear();
        opposite.clear();
        side.clear();
        free.clear();
        for (ridge, &(vertex, known)) in boundary.iter() {
            vertices.extend_from_slice(ridge);
            opposite.push(vertex);
            side.push(known);
        }
        let count = opposite.len();
        across.clear();
        across.resize(count * s, u32::MAX);
        depth.clear();
        for id in 0..count {
            depth.push(volume(&vertices[id * s..(id + 1) * s], inner));
        }
        for marks in [&mut *queued, &mut *tried, &mut *inside] {
            marks.clear();
            marks.resize(count, 0);
        }
        links.clear();
        links.extend((0..(count * s) as u32).map(|entry| (mixed(vertices, entry), entry)));
        pair_faces(s, vertices, links, across);

        let mut stamp = 0_u32;
        let mut last = 0_u32;
        for &p in rest {
            stamp += 1;
            let x: Small<f64, 11> = axes.iter().map(|&a| point(p)[a]).collect();
            // Highest first: the least volume with the point over the
            // volume with the inner point, negative beyond the ridge.
            let mut near = |vertices: &[u32], depth: &[f64], id: u32| -> Near {
                let at = id as usize * s;
                let low = volume(&vertices[at..at + s], &x) / depth[id as usize];
                let far = vertices[at..at + s]
                    .iter()
                    .map(|&v| {
                        let y = point(v);
                        axes.iter()
                            .zip(x.iter())
                            .map(|(&a, &c)| (y[a] - c) * (y[a] - c))
                            .sum::<f64>()
                    })
                    .fold(f64::INFINITY, f64::min);
                Near {
                    // Saturating, and 0 for a height that is not a number.
                    low: (low * 1_048_576.0).round() as i64,
                    far,
                    ridge: id,
                }
            };
            let mut beyond = |id: u32,
                              side: &mut [Option<Sign>],
                              vertices: &[u32]|
             -> Result<bool, ConvexHullError> {
                let ridge = &vertices[id as usize * s..(id as usize + 1) * s];
                let of_point = sign(ridge, p)?;
                if of_point == Sign::Zero {
                    return Ok(false);
                }
                let of_opposite = match side[id as usize] {
                    Some(known) => known,
                    None => {
                        let found = sign(ridge, opposite[id as usize])?;
                        side[id as usize] = Some(found);
                        found
                    }
                };
                Ok(of_opposite != Sign::Zero && of_point != of_opposite)
            };
            // One ridge the point is beyond.
            heap.clear();
            queued[last as usize] = stamp;
            heap.push(near(vertices, depth, last));
            let mut start = None;
            while let Some(Near { ridge: id, .. }) = heap.pop() {
                tried[id as usize] = stamp;
                if beyond(id, side, vertices)? {
                    start = Some(id);
                    break;
                }
                for &next in &across[id as usize * s..(id as usize + 1) * s] {
                    if queued[next as usize] != stamp {
                        queued[next as usize] = stamp;
                        heap.push(near(vertices, depth, next));
                    }
                }
            }
            // A point beyond no ridge adds nothing, as in the scan.
            let Some(start) = start else {
                continue;
            };
            // The others, across shared faces.
            visible.clear();
            visible.push(start);
            inside[start as usize] = stamp;
            stack.clear();
            stack.push(start);
            while let Some(id) = stack.pop() {
                for slot in 0..s {
                    let next = across[id as usize * s + slot];
                    if tried[next as usize] == stamp {
                        continue;
                    }
                    tried[next as usize] = stamp;
                    if beyond(next, side, vertices)? {
                        inside[next as usize] = stamp;
                        visible.push(next);
                        stack.push(next);
                    }
                }
            }
            // The added simplices in lexicographic order.
            added.clear();
            for &id in visible.iter() {
                added.extend_from_slice(&vertices[id as usize * s..(id as usize + 1) * s]);
                added.push(p);
            }
            let simplex = |i: u32| &added[i as usize * *len..(i as usize + 1) * *len];
            order.clear();
            order.extend(0..visible.len() as u32);
            order.sort_unstable_by(|&a, &b| simplex(a).cmp(simplex(b)));
            for &i in order.iter() {
                simplices.extend_from_slice(simplex(i));
            }
            // The boundary: the ridges the point is beyond leave it, and
            // each face they share with a ridge that stays gives a ridge
            // with the point.
            links.clear();
            for &id in visible.iter() {
                for slot in 0..s {
                    let outer = across[id as usize * s + slot];
                    if inside[outer as usize] == stamp {
                        continue;
                    }
                    let old: Small<u32, 11> = vertices[id as usize * s..(id as usize + 1) * s]
                        .iter()
                        .copied()
                        .collect();
                    let new = match free.pop() {
                        Some(new) => new,
                        None => {
                            let new = opposite.len() as u32;
                            vertices.resize(vertices.len() + s, 0);
                            across.resize(across.len() + s, u32::MAX);
                            depth.push(0.0);
                            opposite.push(0);
                            side.push(None);
                            queued.push(0);
                            tried.push(0);
                            inside.push(0);
                            new
                        }
                    };
                    let at = new as usize * s;
                    let mut next = at;
                    for (i, &v) in old.iter().enumerate() {
                        if i != slot {
                            vertices[next] = v;
                            next += 1;
                        }
                    }
                    vertices[next] = p;
                    opposite[new as usize] = old[slot];
                    side[new as usize] = None;
                    depth[new as usize] = volume(&vertices[at..at + s], inner);
                    // Across the face without the point is the ridge that
                    // stays, which now has the new ridge across that face.
                    across[at + s - 1] = outer;
                    for back in &mut across[outer as usize * s..(outer as usize + 1) * s] {
                        if *back == id {
                            *back = new;
                        }
                    }
                    links.extend((0..s as u32 - 1).map(|j| {
                        let entry = new * s as u32 + j;
                        (mixed(vertices, entry), entry)
                    }));
                    last = new;
                }
            }
            pair_faces(s, vertices, links, across);
            free.extend_from_slice(visible);
        }
        Ok(())
    }
}

/// From how many points left [`Placing::general`] hands them to
/// [`Placing::on_the_surface`].
const SURFACE_FROM: usize = 16;

/// The boundary ridges of [`Placing::on_the_surface`], `s` vertices each,
/// ascending, by number; a number is reused once its ridge has left.
#[derive(Default)]
struct Surface {
    vertices: Vec<u32>,
    /// Per ridge, the vertex opposite it in its one simplex and, once
    /// evaluated, that vertex's side of the ridge.
    opposite: Vec<u32>,
    side: Vec<Option<Sign>>,
    /// `s` per ridge: the ridge across the face without the vertex at that
    /// place.
    across: Vec<u32>,
    /// Per ridge, its volume in `f64` with the inner point, and that point
    /// on the axes; `matrix` is the scratch of one volume.
    depth: Vec<f64>,
    inner: Vec<f64>,
    matrix: Vec<f64>,
    /// Per ridge, the point that last queued it, tested it, and was found
    /// beyond it, as that point's number among the points placed here.
    queued: Vec<u32>,
    tried: Vec<u32>,
    inside: Vec<u32>,
    /// The numbers of ridges that have left.
    free: Vec<u32>,
    heap: BinaryHeap<Near>,
    stack: Vec<u32>,
    /// The ridges the current point is beyond.
    visible: Vec<u32>,
    /// Faces to pair, as (a number of the face's vertices, `ridge * s +
    /// place of the vertex left out`).
    links: Vec<(u64, u32)>,
}

/// The determinant of the `s` rows of `s` entries in `rows`, by
/// elimination with the largest pivot of each column. `rows` is consumed.
fn determinant(s: usize, rows: &mut [f64]) -> f64 {
    let mut product = 1.0;
    for column in 0..s {
        let mut pivot = column;
        for row in column + 1..s {
            if rows[row * s + column].abs() > rows[pivot * s + column].abs() {
                pivot = row;
            }
        }
        if rows[pivot * s + column] == 0.0 {
            return 0.0;
        }
        if pivot != column {
            for entry in column..s {
                rows.swap(pivot * s + entry, column * s + entry);
            }
            product = -product;
        }
        let top = rows[column * s + column];
        product *= top;
        for row in column + 1..s {
            let factor = rows[row * s + column] / top;
            for entry in column + 1..s {
                rows[row * s + entry] -= factor * rows[column * s + entry];
            }
        }
    }
    product
}

/// A ridge in the search of [`Placing::on_the_surface`]: the volume with
/// the point over the volume with the inner point, in parts of 2^-20 (least
/// first: negative is beyond the ridge), and the squared distance from the
/// point to the nearest vertex of the ridge (least first).
struct Near {
    low: i64,
    far: f64,
    ridge: u32,
}

impl PartialEq for Near {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Near {}

impl PartialOrd for Near {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Near {
    /// The greatest is the first taken: the lowest, then the nearest, then
    /// the smallest number.
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        other
            .low
            .cmp(&self.low)
            .then(other.far.total_cmp(&self.far))
            .then(other.ridge.cmp(&self.ridge))
    }
}

/// Pairs the faces in `links` by their vertices: each face is in two
/// ridges, which become each other's ridge across it.
fn pair_faces(s: usize, vertices: &[u32], links: &mut [(u64, u32)], across: &mut [u32]) {
    let face = |entry: u32| {
        let (id, slot) = (entry as usize / s, entry as usize % s);
        vertices[id * s..(id + 1) * s]
            .iter()
            .enumerate()
            .filter(move |&(i, _)| i != slot)
            .map(|(_, &v)| v)
    };
    links.sort_unstable_by(|x, y| x.0.cmp(&y.0).then_with(|| face(x.1).cmp(face(y.1))));
    for pair in links.windows(2) {
        if pair[0].0 == pair[1].0 && face(pair[0].1).eq(face(pair[1].1)) {
            across[pair[0].1 as usize] = pair[1].1 / s as u32;
            across[pair[1].1 as usize] = pair[0].1 / s as u32;
        }
    }
}

/// Adds `simplex` to `boundary`: each of its ridges leaves the boundary
/// when it was on it (it now lies on two simplices), and joins it otherwise.
fn toggle(boundary: &mut HashMap<Ridge, (u32, Option<Sign>)>, simplex: &[u32]) {
    for slot in 0..simplex.len() {
        let mut ridge: Ridge = simplex
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != slot)
            .map(|(_, &v)| v)
            .collect();
        ridge.sort_unstable();
        match boundary.entry(ridge) {
            std::collections::hash_map::Entry::Occupied(on_one) => {
                on_one.remove();
            }
            std::collections::hash_map::Entry::Vacant(free) => {
                free.insert((simplex[slot], None));
            }
        }
    }
}

/// A neighbor slot of a [`ComplexSimplex`] not linked yet.
const UNLINKED: u32 = u32::MAX;

/// Links every [`UNLINKED`] simplex neighbor through its shared ridge.
///
/// The face neighbors are those of the merged groups. Two simplices of
/// different facets meet in a ridge of dimension D - 2 on both facets'
/// planes, so the facets share a (D - 2)-face; and a (D - 2)-face of a
/// polytope lies on exactly two facets, so the simplex across any ridge on
/// it belongs to the other. Groups and faces are therefore adjacent alike,
/// however the faces are triangulated; debug builds check it.
fn link_neighbors(d: usize, simplices: &mut Complex, faces: &Faces) {
    if d == 1 {
        // A segment has no ridge. Each simplex's neighbor list is empty
        // (`unlinked` is 0), so there is no triangulation to compare with
        // the group neighbors. `segment_has_two_groups` checks that those
        // neighbors are empty.
        return;
    }
    // Unlinked ridges as sorted vertex lists packed in one buffer, with
    // their owner. A linked ridge has both of its sides linked.
    let mut keys: Vec<u32> = Vec::new();
    let mut owners: Vec<(u32, usize)> = Vec::new();
    for (s, simplex) in simplices.iter().enumerate() {
        for slot in 0..d {
            if simplex.neighbors[slot] != UNLINKED {
                continue;
            }
            let start = keys.len();
            keys.extend(
                simplex
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v),
            );
            keys[start..].sort_unstable();
            owners.push((s as u32, slot));
        }
    }
    // Every ridge of a closed boundary has two sides.
    for (first, second) in pair_equal_keys(&keys, owners.len(), fingerprint) {
        let (a, slot_a) = owners[first];
        let (b, slot_b) = owners[second];
        simplices.neighbors_mut(a as usize)[slot_a] = b;
        simplices.neighbors_mut(b as usize)[slot_b] = a;
    }
    #[cfg(not(debug_assertions))]
    let _ = faces;
    #[cfg(debug_assertions)]
    {
        let mut across: Vec<Vec<u32>> = vec![Vec::new(); faces.len()];
        for simplex in simplices.iter() {
            across[simplex.face as usize].extend(
                simplex
                    .neighbors
                    .iter()
                    .filter(|&&n| n < simplices.len() as u32)
                    .map(|&n| simplices.get(n as usize).face)
                    .filter(|&n| n != simplex.face),
            );
        }
        for (f, (face, mut neighbors)) in faces.neighbors.iter().zip(across).enumerate() {
            neighbors.sort_unstable();
            neighbors.dedup();
            debug_assert_eq!(
                face,
                &neighbors[..],
                "face {f}: the group neighbors differ from the triangulation's"
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::hull::simplicial::tests::Rng;

    thread_local! {
        /// Whether `face_extremes` reads and keeps the faces it has found;
        /// tests turn it off to compare the result without the memo.
        pub(crate) static FACES_REMEMBERED: core::cell::Cell<bool> = const { core::cell::Cell::new(true) };
        /// The hulls built one level down, counted.
        pub(crate) static SUB_HULLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    }

    /// Every point of `{0..3}^k`, row-major.
    fn cube_grid(k: usize) -> Vec<f64> {
        let mut points = Vec::new();
        for i in 0..4_usize.pow(k as u32) {
            for a in 0..k {
                points.push(((i / 4_usize.pow(a as u32)) % 4) as f64);
            }
        }
        points
    }

    /// The hulls built one level down for the classification of `points`.
    fn sub_hulls(k: usize, points: &[f64], remembered: bool) -> usize {
        FACES_REMEMBERED.with(|c| c.set(remembered));
        SUB_HULLS.with(|c| c.set(0));
        let classified = classify(accept(k, points).unwrap()).unwrap();
        FACES_REMEMBERED.with(|c| c.set(true));
        assert_eq!(classified.vertices.len(), 1 << k, "the cube's vertices");
        SUB_HULLS.with(core::cell::Cell::get)
    }

    #[test]
    fn each_face_is_found_once_on_cube_grids() {
        // The grid {0..3}^k is a k-cube with points on every face. Each of
        // its faces of dimension 2 to k - 1, C(k, j) 2^(k - j) of them, is
        // built once (#394). Without the memo, a face is built again from
        // every facet above it.
        let binomial = |n: usize, r: usize| (0..r).fold(1, |acc, i| acc * (n - i) / (i + 1));
        for (k, without) in [(3, 6), (4, 56), (5, 570)] {
            let points = cube_grid(k);
            let faces: usize = (2..k).map(|j| binomial(k, j) << (k - j)).sum();
            assert_eq!(sub_hulls(k, &points, true), faces, "k = {k}");
            assert_eq!(
                sub_hulls(k, &points, false),
                without,
                "k = {k}, without the memo"
            );
        }
    }

    /// The D6 cases of the two tests above and below, which take minutes in
    /// debug: run in release with
    /// `cargo test --release -- --ignored faces_once_in_d6`.
    #[test]
    #[ignore = "minutes in debug; run in release"]
    fn faces_once_in_d6() {
        // The 6-cube's faces of dimension 5 to 2: 12 + 60 + 160 + 240.
        let points = cube_grid(6);
        assert_eq!(sub_hulls(6, &points, true), 472);
        assert_eq!(sub_hulls(6, &points, false), 12 + 120 + 960 + 5760);
        // The published hull with the memo equals the one without it.
        let mut rng = Rng(406);
        for family in ["grid", "lattice", "cubesurf"] {
            let mut points = Vec::with_capacity(2000 * 6);
            for _ in 0..2000 {
                match family {
                    "grid" => points.extend((0..6).map(|_| (rng.next() % 4) as f64)),
                    "lattice" => points.extend((0..6).map(|_| (rng.next() % 5) as f64 - 2.0)),
                    _ => {
                        let start = points.len();
                        points.extend((0..6).map(|_| (rng.next() % 9) as f64 - 4.0));
                        let axis = rng.next() as usize % 6;
                        points[start + axis] = if rng.next().is_multiple_of(2) {
                            -4.0
                        } else {
                            4.0
                        };
                    }
                }
            }
            let hull = |remembered: bool| {
                FACES_REMEMBERED.with(|c| c.set(remembered));
                let hull = crate::ConvexHullBuilder::new(6, &points).build();
                FACES_REMEMBERED.with(|c| c.set(true));
                hull
            };
            assert!(hull(true).unwrap() == hull(false).unwrap(), "{family} D6");
        }
    }

    #[test]
    fn faces_once_publish_the_same_hull_and_diagram() {
        // Degenerate inputs whose facets hold many points, D = 3 to 5: the
        // hull and the Voronoi diagram published with the memo equal those
        // published without it. D = 6 takes minutes per input in debug; the
        // pull request compares it in release.
        let mut rng = Rng(394);
        let sphere = |k: usize| -> Vec<Vec<f64>> {
            let radius2 = if k <= 3 { 9 } else { 4 };
            let mut out = Vec::new();
            let mut p = vec![-3_i64; k];
            loop {
                if p.iter().map(|x| x * x).sum::<i64>() == radius2 {
                    out.push(p.iter().map(|&x| x as f64).collect());
                }
                let Some(a) = (0..k).rev().find(|&a| p[a] < 3) else {
                    return out;
                };
                p[a] += 1;
                for c in &mut p[a + 1..] {
                    *c = -3;
                }
            }
        };
        let mut hulls = 0;
        for k in 3..=5 {
            let n = [0, 0, 0, 400, 300, 60][k];
            let on_sphere = sphere(k);
            for family in ["grid", "lattice", "cubesurf", "onsphere", "nearsphere"] {
                let mut points = Vec::with_capacity(n * k);
                for _ in 0..n {
                    match family {
                        "grid" => points.extend((0..k).map(|_| (rng.next() % 4) as f64)),
                        "lattice" => points.extend((0..k).map(|_| (rng.next() % 5) as f64 - 2.0)),
                        "cubesurf" => {
                            let start = points.len();
                            points.extend((0..k).map(|_| (rng.next() % 9) as f64 - 4.0));
                            let axis = rng.next() as usize % k;
                            points[start + axis] = if rng.next().is_multiple_of(2) {
                                -4.0
                            } else {
                                4.0
                            };
                        }
                        _ => {
                            let p = &on_sphere[rng.next() as usize % on_sphere.len()];
                            let step = if family == "nearsphere" {
                                1.0 / 256.0
                            } else {
                                0.0
                            };
                            points.extend(
                                p.iter()
                                    .map(|&x| x + step * ((rng.next() % 3) as f64 - 1.0)),
                            );
                        }
                    }
                }
                let hull = |remembered: bool| {
                    FACES_REMEMBERED.with(|c| c.set(remembered));
                    let hull = crate::ConvexHullBuilder::new(k, &points).build();
                    FACES_REMEMBERED.with(|c| c.set(true));
                    hull
                };
                let (with, without) = (hull(true).unwrap(), hull(false).unwrap());
                assert!(with == without, "{family} D{k}: the hull");
                hulls += 1;
                if k <= 4 {
                    let diagram = |remembered: bool| {
                        FACES_REMEMBERED.with(|c| c.set(remembered));
                        let diagram = crate::VoronoiBuilder::new(k, &points).build();
                        FACES_REMEMBERED.with(|c| c.set(true));
                        diagram
                    };
                    assert_eq!(diagram(true), diagram(false), "{family} D{k}: the diagram");
                }
            }
        }
        assert_eq!(hulls, 15);
    }

    /// [`Placing::run`] on `workspace`, as a list of simplices.
    fn placed<'p>(
        workspace: &mut Placing,
        d: usize,
        span: Option<usize>,
        point: impl Fn(u32) -> &'p [f64],
        extreme: &[u32],
    ) -> Vec<Vec<u32>> {
        let (len, simplices) = workspace.run(d, span, point, extreme).unwrap();
        simplices.chunks_exact(len).map(<[u32]>::to_vec).collect()
    }

    /// The placing triangulation before #393: the ridges of every simplex
    /// gathered again for each point, and both sides evaluated for each
    /// boundary ridge. The reference of [`Placing`]'s tests.
    fn placing_by_full_scan<'p>(
        d: usize,
        point: impl Fn(u32) -> &'p [f64],
        extreme: &[u32],
    ) -> Result<Vec<Vec<u32>>, ConvexHullError> {
        let Some((&first, rest)) = extreme.split_first() else {
            return Ok(Vec::new());
        };
        let mut simplices: Vec<Vec<u32>> = vec![vec![first]];
        // Coordinates on which the points placed so far project to an affinely
        // independent basis; their count is the current affine dimension.
        let mut axes: Vec<usize> = Vec::with_capacity(d);
        let mut basis: Vec<u32> = vec![first];
        let project = |v: u32, axes: &[usize], extra: Option<usize>| -> Vec<f64> {
            let x = point(v);
            axes.iter()
                .map(|&a| x[a])
                .chain(extra.map(|j| x[j]))
                .collect()
        };
        for &p in rest {
            // Does p raise the affine dimension?
            let mut raised = None;
            for j in (0..d).filter(|j| !axes.contains(j)) {
                let projected: Vec<Vec<f64>> = basis
                    .iter()
                    .chain(core::iter::once(&p))
                    .map(|&v| project(v, &axes, Some(j)))
                    .collect();
                let refs: Vec<&[f64]> = projected.iter().map(Vec::as_slice).collect();
                if orient(&refs)? != Sign::Zero {
                    raised = Some(j);
                    break;
                }
            }
            if let Some(j) = raised {
                axes.push(j);
                basis.push(p);
                for simplex in &mut simplices {
                    simplex.push(p);
                }
                continue;
            }
            // Beyond which boundary ridges of the current complex is p?
            let mut sides: HashMap<Vec<u32>, (usize, usize, usize)> = HashMap::new();
            for (s, simplex) in simplices.iter().enumerate() {
                for slot in 0..simplex.len() {
                    let mut ridge: Vec<u32> = simplex
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != slot)
                        .map(|(_, &v)| v)
                        .collect();
                    ridge.sort_unstable();
                    let entry = sides.entry(ridge).or_insert((0, s, slot));
                    entry.0 += 1;
                }
            }
            let mut added = Vec::new();
            for (ridge, (count, s, slot)) in sides {
                if count != 1 {
                    continue;
                }
                let a = simplices[s][slot];
                let side_of = |v: u32| -> Result<Sign, ConvexHullError> {
                    let projected: Vec<Vec<f64>> = ridge
                        .iter()
                        .chain(core::iter::once(&v))
                        .map(|&u| project(u, &axes, None))
                        .collect();
                    let refs: Vec<&[f64]> = projected.iter().map(Vec::as_slice).collect();
                    Ok(orient(&refs)?)
                };
                let (sp, sa) = (side_of(p)?, side_of(a)?);
                if sp != Sign::Zero && sa != Sign::Zero && sp != sa {
                    let mut vertices = ridge;
                    vertices.push(p);
                    added.push(vertices);
                }
            }
            added.sort_unstable();
            simplices.extend(added);
        }
        Ok(simplices)
    }

    pub(crate) fn classified(dim: usize, points: &[f64]) -> Classified<'_> {
        let c = classify(accept(dim, points).unwrap()).unwrap();
        check(&c);
        c
    }

    /// Partition, extreme vertex sets, and symmetric neighbors.
    pub(crate) fn check(c: &Classified<'_>) {
        let mut all: Vec<u32> = c
            .vertices
            .iter()
            .chain(&c.coplanar_points)
            .chain(&c.interior_points)
            .copied()
            .collect();
        all.sort_unstable();
        assert_eq!(
            all, c.input.representatives,
            "the three lists partition the representatives"
        );
        let mut face_vertices: Vec<u32> = c
            .faces
            .iter()
            .flat_map(|f| f.vertices.iter().copied())
            .collect();
        face_vertices.sort_unstable();
        face_vertices.dedup();
        assert_eq!(face_vertices, c.vertices);
        for (s, simplex) in c.simplices.iter().enumerate() {
            for (slot, &n) in simplex.neighbors.iter().enumerate() {
                let other = c.simplices.get(n as usize);
                let back = other.neighbors.iter().position(|&b| b == s as u32).unwrap();
                let mut a: Vec<u32> = simplex
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != slot)
                    .map(|(_, &v)| v)
                    .collect();
                let mut b: Vec<u32> = other
                    .vertices
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != back)
                    .map(|(_, &v)| v)
                    .collect();
                a.sort_unstable();
                b.sort_unstable();
                assert_eq!(a, b);
            }
            // The simplex spans a supporting hyperplane: no two
            // representatives lie strictly on opposite sides.
            let signs: Vec<Sign> = c
                .input
                .representatives
                .iter()
                .map(|&p| {
                    let mut points: Vec<&[f64]> =
                        simplex.vertices.iter().map(|&v| c.input.point(v)).collect();
                    points.push(c.input.point(p));
                    orient(&points).unwrap()
                })
                .collect();
            assert!(!(signs.contains(&Sign::Positive) && signs.contains(&Sign::Negative)));
            for v in simplex.vertices {
                assert!(c.faces.get(simplex.face as usize).vertices.contains(v));
            }
        }
        for (f, neighbors) in c.faces.neighbors.iter().enumerate() {
            for &n in neighbors {
                assert!(c.faces.neighbors.get(n as usize).contains(&(f as u32)));
            }
        }
    }

    #[test]
    fn placing_keeps_the_triangulation_of_the_full_scan() {
        // One workspace for every case, of every dimension: what a run
        // leaves behind must not reach the next (#524).
        let mut workspace = Placing::default();
        // Points in convex position within their span, as `placing` takes
        // them: the vertices of a cube (the extreme points of a grid's
        // faces), integer points on a sphere (a cospherical group of
        // Delaunay), and points on the moment curve. Each family is placed
        // whole and as random subsets of at most 24 points, in shuffled index
        // orders, in its own dimension (k = 2 to 6) and lifted into one more
        // by a constant coordinate (a facet of a hull), so up to D = 7. Then
        // the extreme points of the facets of real hulls of grids, lattices,
        // and cube surfaces, as classification passes them. The simplices
        // and their order are those of the reference that gathers every
        // ridge again.
        let mut rng = Rng(393);
        let cube = |k: usize| -> Vec<Vec<f64>> {
            (0..1_u32 << k)
                .map(|i| (0..k).map(|a| f64::from((i >> a) & 1)).collect())
                .collect()
        };
        let sphere = |k: usize| -> Vec<Vec<f64>> {
            // Integer points with |p|^2 = 9 (k = 2, 3) or 4 (k >= 4).
            let radius2 = if k <= 3 { 9 } else { 4 };
            let bound = 3_i64;
            let mut out = Vec::new();
            let mut p = vec![-bound; k];
            loop {
                if p.iter().map(|x| x * x).sum::<i64>() == radius2 {
                    out.push(p.iter().map(|&x| x as f64).collect());
                }
                let Some(a) = (0..k).rev().find(|&a| p[a] < bound) else {
                    return out;
                };
                p[a] += 1;
                for c in &mut p[a + 1..] {
                    *c = -bound;
                }
            }
        };
        let moment = |k: usize| -> Vec<Vec<f64>> {
            (0..k as i32 + 6)
                .map(|t| (1..=k as i32).map(|e| f64::from(t).powi(e)).collect())
                .collect()
        };
        let mut cases = 0;
        let mut beyond = 0;
        let mut planar = 0;
        for k in 2..=6 {
            let families: [(&str, Vec<Vec<f64>>); 3] = [
                ("cube", cube(k)),
                ("sphere", sphere(k)),
                ("moment", moment(k)),
            ];
            for (name, points) in families {
                for lifted in [false, true] {
                    for round in 0..6 {
                        // A shuffled order, then a random subset for later rounds.
                        let mut order: Vec<usize> = (0..points.len()).collect();
                        for i in (1..order.len()).rev() {
                            order.swap(i, rng.next() as usize % (i + 1));
                        }
                        // At most 24 points keep the reference fast in debug.
                        let keep = if round >= 2 {
                            k + 1 + rng.next() as usize % (points.len() - k)
                        } else {
                            points.len()
                        };
                        order.truncate(keep.min(24));
                        let rows: Vec<Vec<f64>> = order
                            .iter()
                            .map(|&i| {
                                let mut row = points[i].clone();
                                if lifted {
                                    row.insert(0, 2.0);
                                }
                                row
                            })
                            .collect();
                        let d = k + usize::from(lifted);
                        let extreme: Vec<u32> = (0..rows.len() as u32).collect();
                        let point = |i: u32| rows[i as usize].as_slice();
                        let expected = placing_by_full_scan(d, point, &extreme).unwrap();
                        let got = placed(&mut workspace, d, None, point, &extreme);
                        // The families are in strictly convex position, so
                        // three points or more of a family of the plane span
                        // it, and take the path of #534.
                        if k == 2 && rows.len() >= 3 {
                            assert_eq!(
                                placed(&mut workspace, d, Some(2), point, &extreme),
                                expected,
                                "{name} in a plane, lifted {lifted}, round {round}"
                            );
                            planar += 1;
                        }
                        assert_eq!(
                            got, expected,
                            "{name} k = {k}, lifted {lifted}, round {round}"
                        );
                        // A subset may span less than k dimensions; every simplex
                        // spans what the points span.
                        assert!(
                            got.iter().all(|s| s.len() == got[0].len()),
                            "{name} k = {k}: simplices of one dimension"
                        );
                        cases += 1;
                        beyond += usize::from(rows.len() > k + 1);
                    }
                }
            }
        }
        assert_eq!(cases, 180);
        assert!(planar > 30, "{planar} cases placed in a plane");
        assert!(beyond > 150, "{beyond} cases place a point beyond a ridge");

        // The facets of hulls of degenerate inputs, each a face that is not
        // a simplex, with the input's coordinates.
        let mut faces = 0;
        for (name, k, n) in [
            ("grid", 3, 300),
            ("grid", 5, 300),
            ("lattice", 3, 300),
            ("lattice", 4, 300),
            ("cubesurf", 3, 200),
            ("cubesurf", 4, 120),
        ] {
            let mut points = Vec::with_capacity(n * k);
            for _ in 0..n {
                match name {
                    "grid" => points.extend((0..k).map(|_| (rng.next() % 4) as f64)),
                    "lattice" => points.extend((0..k).map(|_| (rng.next() % 5) as f64 - 2.0)),
                    _ => {
                        let start = points.len();
                        points.extend((0..k).map(|_| (rng.next() % 9) as f64 - 4.0));
                        let axis = rng.next() as usize % k;
                        points[start + axis] = if rng.next().is_multiple_of(2) {
                            -4.0
                        } else {
                            4.0
                        };
                    }
                }
            }
            let hull = crate::ConvexHullBuilder::new(k, &points).build().unwrap();
            let point = |i: u32| &points[i as usize * k..(i as usize + 1) * k];
            let mut found = 0;
            for facet in hull.facets().iter() {
                let extreme = facet.vertices();
                if extreme.len() <= k {
                    continue;
                }
                let expected = placing_by_full_scan(k, point, extreme).unwrap();
                assert_eq!(
                    placed(&mut workspace, k, Some(k - 1), point, extreme),
                    expected,
                    "{name} D{k}"
                );
                found += 1;
            }
            assert!(found > 0, "{name} D{k}: no facet that is not a simplex");
            faces += found;
        }
        assert!(faces > 30, "{faces} faces of real hulls");
    }

    /// What is found on a hull with the buffers of the hulls before it
    /// (#531), levels and lists, is what is found with none: the groups,
    /// the vertices, the other points and whether each is on the boundary,
    /// and each facet's extreme points. The hulls are of D = 3 to 5, with
    /// faces that hold many points, so that sub-hulls are built at one
    /// level and at two, and in general position. A face is also found
    /// again through the memo.
    #[test]
    fn found_with_the_buffers_of_another_hull_is_found_with_none() {
        fn read(found: &Found) -> impl PartialEq + core::fmt::Debug {
            (
                found
                    .groups
                    .groups
                    .iter()
                    .map(<[FacetId]>::to_vec)
                    .collect::<Vec<_>>(),
                found.groups.vertices.clone(),
                found.groups.neighbors.clone(),
                found.on_complex.clone(),
                found.others.clone(),
                found.on_boundary.clone(),
                (found.extremes.lists.clone(), found.extremes.own.clone()),
                found.is_vertex.clone(),
                found.vertices.clone(),
            )
        }
        let mut rng = Rng(531);
        let mut levels = Vec::new();
        let mut out = Found::default();
        let before = SUB_HULLS.with(core::cell::Cell::get);
        let mut hulls = 0;
        for round in 0..2 {
            for (dim, count, grid) in [
                (4, 500, true),
                (3, 80, false),
                (5, 400, true),
                (3, 700, true),
                (4, 120, false),
                (5, 60, false),
                (4, 40, true),
            ] {
                let points = crate::hull::simplicial::tests::drawn(&mut rng, dim, count, grid);
                let Ok(input) = accept(dim, &points) else {
                    continue;
                };
                let hull = SimplicialHull::build(input).unwrap();
                let fresh = found_vertices(&hull, &mut FoundFaces::default(), None).unwrap();
                let mut faces = FoundFaces {
                    levels: core::mem::take(&mut levels),
                    ..FoundFaces::default()
                };
                found_vertices_into(&hull, &mut faces, None, &mut out).unwrap();
                assert_eq!(
                    read(&out),
                    read(&fresh),
                    "D = {dim}, {count} points, round {round}"
                );
                levels = faces.levels;
                hulls += 1;
            }
        }
        assert!(hulls >= 12, "{hulls} hulls");
        // The cross-polytope of D = 4 with the midpoint of one edge, twice
        // in a row, for each edge. Four facets meet around an edge, so the
        // midpoint is on a facet that is no neighbor of the one it was
        // recorded on, and the walk reaches that facet through marks that
        // the hull before left set for the same point.
        let corners: Vec<f64> = (0..4)
            .flat_map(|i| [-1.0, 1.0].map(|side| (i, side)))
            .flat_map(|(i, side)| (0..4).map(move |a| if a == i { side } else { 0.0 }))
            .collect();
        let mut walked = 0;
        for i in 0..4 {
            for j in i + 1..4 {
                for (si, sj) in [(-0.5, -0.5), (-0.5, 0.5), (0.5, -0.5), (0.5, 0.5)] {
                    let mut cross = corners.clone();
                    cross.extend((0..4).map(|a| match a {
                        a if a == i => si,
                        a if a == j => sj,
                        _ => 0.0,
                    }));
                    for round in 0..2 {
                        let hull = SimplicialHull::build(accept(4, &cross).unwrap()).unwrap();
                        let fresh =
                            found_vertices(&hull, &mut FoundFaces::default(), None).unwrap();
                        assert_eq!(fresh.vertices.len(), 8);
                        assert_eq!(fresh.on_boundary, [true]);
                        walked += usize::from(hull.records.has_any(8));
                        let mut faces = FoundFaces {
                            levels: core::mem::take(&mut levels),
                            ..FoundFaces::default()
                        };
                        found_vertices_into(&hull, &mut faces, None, &mut out).unwrap();
                        assert_eq!(
                            read(&out),
                            read(&fresh),
                            "the cross-polytope, edge {i} {j}, round {round}"
                        );
                        levels = faces.levels;
                    }
                }
            }
        }
        assert!(walked >= 24, "{walked} midpoints were recorded on a plane");
        assert!(
            SUB_HULLS.with(core::cell::Cell::get) - before > 100,
            "sub-hulls were built"
        );
        // The memo: faces stored under keys that share a prefix.
        let mut memo = FaceMemo::default();
        assert_eq!(memo.get(&[1, 2, 3]), None);
        memo.insert(&[1, 2, 3], [1, 3]);
        memo.insert(&[1, 2, 3, 4], [1, 4]);
        memo.insert(&[1, 2], []);
        assert_eq!(memo.get(&[1, 2, 3]), Some(&[1, 3][..]));
        assert_eq!(memo.get(&[1, 2, 3, 4]), Some(&[1, 4][..]));
        assert_eq!(memo.get(&[1, 2]), Some(&[][..]));
        assert_eq!(memo.get(&[2, 3]), None);
    }

    /// Points that span three dimensions or more are placed from the
    /// ridges reached through their neighbors (#540), with the simplices
    /// of the scan in its order. The sets have more points than the debug
    /// comparison of [`Placing::run`] covers: a prism over 150 points of a
    /// parabola, the integer points of a sphere in D = 3 and D = 4, each
    /// also lifted into two more coordinates that are exact sums of the
    /// others, and the sphere of D = 3 with its center last, a point
    /// beyond no ridge. Each runs in three index orders.
    #[test]
    fn the_surface_places_as_the_scan() {
        fn sphere(d: usize, squared: i64) -> Vec<Vec<f64>> {
            let r = (squared as f64).sqrt() as i64 + 1;
            let mut found = Vec::new();
            let mut x = vec![-r; d];
            loop {
                if x.iter().map(|c| c * c).sum::<i64>() == squared {
                    found.push(x.iter().map(|&c| c as f64).collect());
                }
                let Some(axis) = (0..d).find(|&a| x[a] < r) else {
                    return found;
                };
                x[..axis].fill(-r);
                x[axis] += 1;
            }
        }
        let prism: Vec<Vec<f64>> = (0..150)
            .flat_map(|i| {
                let t = f64::from(i) - 75.0;
                [vec![t, t * t, 0.0], vec![t, t * t, 1.0]]
            })
            .collect();
        let mut centered = sphere(3, 101);
        centered.push(vec![0.0; 3]);
        let lifted = |rows: &[Vec<f64>]| -> Vec<Vec<f64>> {
            rows.iter()
                .map(|x| {
                    let mut y = vec![x[0] + x[1], 2.0 * x[0]];
                    y.extend_from_slice(x);
                    y
                })
                .collect()
        };
        let sets = [
            (3, prism.clone()),
            (3, lifted(&prism)),
            (3, sphere(3, 101)),
            (3, lifted(&sphere(3, 101))),
            (3, centered),
            (4, sphere(4, 10)),
            (4, lifted(&sphere(4, 10))),
        ];
        let mut workspace = Placing::default();
        let mut rng = Rng(540);
        for (span, mut rows) in sets {
            assert!(
                rows.len() > 64 + SURFACE_FROM + span + 1,
                "{} points",
                rows.len()
            );
            for round in 0..3 {
                if round > 0 {
                    for i in (1..rows.len()).rev() {
                        rows.swap(i, rng.next() as usize % (i + 1));
                    }
                }
                let d = rows[0].len();
                let point = |i: u32| rows[i as usize].as_slice();
                let extreme: Vec<u32> = (0..rows.len() as u32).collect();
                let scanned = placed(&mut workspace, d, None, point, &extreme);
                assert!(
                    scanned.len() >= rows.len() - span - 1,
                    "a simplex per point or more"
                );
                assert_eq!(
                    placed(&mut workspace, d, Some(span), point, &extreme),
                    scanned,
                    "span {span} in D = {d}, {} points, round {round}",
                    rows.len()
                );
            }
        }
    }

    /// Points that span a plane are placed from their cyclic order (#534),
    /// with the simplices of the general method in its order: 300 points
    /// of a parabola in shuffled index order, in the plane and in the plane
    /// `z = 2 x` of D = 3, where every coordinate is exact. A set that is
    /// not in strictly convex position, a square with the midpoint of an
    /// edge, falls back to the general method.
    #[test]
    fn a_plane_places_as_the_general_method() {
        let mut workspace = Placing::default();
        let mut rng = Rng(534);
        for round in 0..4 {
            let mut ts: Vec<f64> = (0..300).map(|i| f64::from(i) - 150.0).collect();
            for i in (1..ts.len()).rev() {
                ts.swap(i, rng.next() as usize % (i + 1));
            }
            for d in [2, 3] {
                let rows: Vec<Vec<f64>> = ts
                    .iter()
                    .map(|&t| {
                        if d == 2 {
                            vec![t, t * t]
                        } else {
                            vec![t, t * t, 2.0 * t]
                        }
                    })
                    .collect();
                let point = |i: u32| rows[i as usize].as_slice();
                let extreme: Vec<u32> = (0..rows.len() as u32).collect();
                let general = placed(&mut workspace, d, None, point, &extreme);
                assert_eq!(general.len(), 298, "one triangle per point after two");
                assert_eq!(
                    placed(&mut workspace, d, Some(2), point, &extreme),
                    general,
                    "D = {d}, round {round}"
                );
            }
        }
        let square = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [1.0, 0.0]];
        let point = |i: u32| square[i as usize].as_slice();
        let extreme = [0, 1, 2, 3, 4];
        assert!(!workspace.in_a_plane(2, &point, &extreme).unwrap());
        assert_eq!(
            placed(&mut workspace, 2, Some(2), point, &extreme),
            placed(&mut workspace, 2, None, point, &extreme),
            "a point on an edge falls back"
        );
    }

    #[test]
    fn square_with_an_edge_midpoint_and_a_center() {
        let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.5, 0.0, 0.5, 0.5];
        let c = classified(2, &points);
        assert_eq!(c.vertices, vec![0, 1, 2, 3]);
        assert_eq!(c.coplanar_points, vec![4]);
        assert_eq!(c.interior_points, vec![5]);
        assert_eq!(c.faces.len(), 4);
    }

    #[test]
    fn cube_with_face_center_and_edge_midpoint() {
        let mut points = Vec::new();
        for i in 0..8 {
            points.extend([(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64]);
        }
        points.extend([0.5, 0.5, 0.0, 0.5, 0.0, 0.0, 0.5, 0.5, 0.5]);
        let c = classified(3, &points);
        assert_eq!(c.vertices, (0..8).collect::<Vec<u32>>());
        assert_eq!(c.coplanar_points, vec![8, 9]);
        assert_eq!(c.interior_points, vec![10]);
        assert_eq!(c.faces.len(), 6);
        assert!(c.faces.iter().all(|f| f.vertices.len() == 4));
        assert!(c.faces.neighbors.iter().all(|n| n.len() == 4));
    }

    #[test]
    fn duplicate_of_a_corner_stays_a_duplicate() {
        let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0];
        let c = classified(2, &points);
        assert_eq!(c.input.representative, vec![0, 1, 2, 1]);
        assert_eq!(c.vertices, vec![0, 1, 2]);
        assert!(c.coplanar_points.is_empty());
    }

    #[test]
    fn segment_endpoints() {
        let c = classified(1, &[2.0, -1.0, 0.5, 3.0]);
        assert_eq!(c.vertices, vec![1, 3]);
        assert_eq!(c.interior_points, vec![0, 2]);
    }

    #[test]
    fn collinear_vertex_inserted_early_is_demoted() {
        // (1, 0) is extreme for the first triangle, then (2, 0) puts it on
        // the bottom edge.
        let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 2.0, 0.0];
        let c = classified(2, &points);
        assert_eq!(c.vertices, vec![0, 2, 3]);
        assert_eq!(c.coplanar_points, vec![1]);
        assert_eq!(c.faces.len(), 3);
    }

    #[test]
    fn integer_grids_keep_only_corners() {
        for dim in 2..=4 {
            let side = 3_usize;
            let count = side.pow(dim as u32);
            let points: Vec<f64> = (0..count)
                .flat_map(|i| (0..dim).map(move |a| ((i / side.pow(a as u32)) % side) as f64))
                .collect();
            let c = classified(dim, &points);
            assert_eq!(c.vertices.len(), 1 << dim, "dim {dim}");
            assert_eq!(c.faces.len(), 2 * dim, "dim {dim}");
            assert_eq!(
                c.interior_points.len(),
                (side - 2).pow(dim as u32),
                "dim {dim}"
            );
        }
    }

    #[test]
    fn random_inputs_partition() {
        let mut rng = Rng(31);
        for dim in 2..=4 {
            let points: Vec<f64> = (0..40 * dim).map(|_| rng.unit()).collect();
            let c = classified(dim, &points);
            assert!(c.coplanar_points.is_empty());
        }
    }

    #[test]
    fn rounded_grid_on_sphere_shell() {
        // Points of an integer grid inside a ball, many cocircular boundary
        // points, but exact coordinates.
        let mut points = Vec::new();
        for i in -3_i32..=3 {
            for j in -3_i32..=3 {
                for k in -3_i32..=3 {
                    if i * i + j * j + k * k <= 9 {
                        points.extend([f64::from(i), f64::from(j), f64::from(k)]);
                    }
                }
            }
        }
        classified(3, &points);
    }

    /// Andrew's chain, from the exact orientation, as a reference that does
    /// not call the hull.
    fn monotone_cycle(points: &[f64]) -> Vec<u32> {
        let n = points.len() / 2;
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by(|&a, &b| {
            let (pa, pb) = (
                &points[a as usize * 2..a as usize * 2 + 2],
                &points[b as usize * 2..b as usize * 2 + 2],
            );
            pa[0]
                .total_cmp(&pb[0])
                .then(pa[1].total_cmp(&pb[1]))
                .then(a.cmp(&b))
        });
        let p = |i: u32| &points[i as usize * 2..i as usize * 2 + 2];
        let mut chain = Vec::new();
        for pass in [order.clone(), order.iter().rev().copied().collect()] {
            let start = chain.len();
            for &i in &pass {
                while chain.len() >= start + 2 {
                    let sign =
                        orient(&[p(chain[chain.len() - 2]), p(chain[chain.len() - 1]), p(i)])
                            .unwrap();
                    if sign == Sign::Positive {
                        break;
                    }
                    chain.pop();
                }
                chain.push(i);
            }
            chain.pop();
        }
        chain
    }

    /// A filled square: the discarding polygon is the square itself, so its
    /// edge points are not proved inside and stay coplanar, and the points
    /// off the edges are discarded as interior.
    #[test]
    fn filled_square_discards_its_interior_and_keeps_edge_points_coplanar() {
        let side = 32usize;
        let mut points = Vec::with_capacity(side * side * 2);
        for y in 0..side {
            for x in 0..side {
                points.push(x as f64);
                points.push(y as f64);
            }
        }
        let at = |x: usize, y: usize| (y * side + x) as u32;
        let mut vertices = vec![
            at(0, 0),
            at(side - 1, 0),
            at(0, side - 1),
            at(side - 1, side - 1),
        ];
        vertices.sort_unstable();
        let mut coplanar = Vec::new();
        let mut interior = Vec::new();
        for y in 0..side {
            for x in 0..side {
                let edge = x == 0 || y == 0 || x + 1 == side || y + 1 == side;
                let corner = (x == 0 || x + 1 == side) && (y == 0 || y + 1 == side);
                if corner {
                    continue;
                } else if edge {
                    coplanar.push(at(x, y));
                } else {
                    interior.push(at(x, y));
                }
            }
        }
        let cycle = monotone_cycle(&points);
        let mut by_chain = cycle.clone();
        by_chain.sort_unstable();
        assert_eq!(by_chain, vertices, "the square's extremes are its corners");
        {
            let hull = SimplicialHull::build(accept(2, &points).unwrap()).unwrap();
            assert!(hull.strict_edges, "every D = 2 hull is the chain");
            let mut proved = hull.proved_interior.clone();
            proved.sort_unstable();
            assert_eq!(proved, interior, "the points off the edges are discarded");
            let c = classify_built(hull, &mut FoundFaces::default()).unwrap();
            check(&c);
            assert_eq!(c.vertices, vertices);
            assert_eq!(c.coplanar_points, coplanar);
            assert_eq!(c.interior_points, interior);
        }
    }

    /// The farthest points in the eight directions are only the two ends of
    /// a segment, so there is no polygon and nothing is discarded. The point
    /// off the segment is still a vertex, and the one on it is coplanar.
    #[test]
    fn collinear_directional_extremes_discard_nothing() {
        let points = [0.0, 0.0, 1.0, 2.0, 2.0, 4.0, 1.0, 2.25];
        {
            let hull = SimplicialHull::build(accept(2, &points).unwrap()).unwrap();
            assert!(hull.strict_edges);
            assert!(hull.proved_interior.is_empty(), "no polygon, no discard");
            let c = classify_built(hull, &mut FoundFaces::default()).unwrap();
            check(&c);
            assert_eq!(c.vertices, vec![0, 2, 3]);
            assert_eq!(c.coplanar_points, vec![1]);
            assert!(c.interior_points.is_empty());
        }
    }

    /// Sites on a circle with `inside` more strictly inside it, placed
    /// evenly through the input so that a stride meets them.
    fn circle_with_interior(on_circle: usize, inside: usize) -> (Vec<f64>, Vec<u32>) {
        let total = on_circle + inside;
        let every = total / inside.max(1);
        let mut points = Vec::with_capacity(2 * total);
        let mut interior = Vec::new();
        let mut placed = 0;
        for i in 0..total {
            let t = (i as f64) * core::f64::consts::TAU / (total as f64);
            if placed < inside && i % every == every / 2 {
                // Radius 0.25: inside the octagon of any dense circle.
                points.extend([0.25 * t.cos(), 0.25 * t.sin()]);
                interior.push(i as u32);
                placed += 1;
            } else {
                points.extend([t.cos(), t.sin()]);
            }
        }
        assert_eq!(placed, inside);
        (points, interior)
    }

    /// One site in 16 is inside: the sample finds enough of them, and every
    /// interior site is discarded. One in 100 is not enough: none is
    /// discarded, and classification still finds them all.
    #[test]
    fn discarding_runs_only_where_it_pays() {
        for (on_circle, inside, discards) in [(2040, 136, true), (2040, 20, false)] {
            let (points, interior) = circle_with_interior(on_circle, inside);
            {
                let hull = SimplicialHull::build(accept(2, &points).unwrap()).unwrap();
                let mut proved = hull.proved_interior.clone();
                proved.sort_unstable();
                if discards {
                    assert_eq!(proved, interior, "{inside} inside: all are discarded");
                } else {
                    assert!(proved.is_empty(), "{inside} inside: none is discarded");
                }
                let c = classify_built(hull, &mut FoundFaces::default()).unwrap();
                // check is quadratic in the facets; the lists below are
                // what the cutoff could change.
                assert_eq!(c.vertices.len(), on_circle);
                assert_eq!(c.interior_points, interior, "{inside} inside");
                assert!(c.coplanar_points.is_empty());
            }
        }
    }

    /// On a circle every site is extreme, and none is discarded.
    #[test]
    fn large_circle_uses_the_chain() {
        let n = 1100usize;
        let points: Vec<f64> = (0..n)
            .flat_map(|i| {
                let t = (i as f64) * core::f64::consts::TAU / (n as f64);
                [t.cos(), t.sin()]
            })
            .collect();
        let cycle = monotone_cycle(&points);
        let mut vertices = cycle.clone();
        vertices.sort_unstable();
        {
            let hull = SimplicialHull::build(accept(2, &points).unwrap()).unwrap();
            assert!(hull.strict_edges, "every D = 2 hull is the chain");
            assert!(hull.proved_interior.is_empty(), "no site is inside");
            let c = classify_built(hull, &mut FoundFaces::default()).unwrap();
            check(&c);
            assert_eq!(c.vertices, vertices);
            assert!(c.coplanar_points.is_empty());
            assert!(c.interior_points.is_empty());
        }
    }

    /// The partition with the proved points skipped, and without: the
    /// reference scans every representative against every group.
    fn with_and_without_reuse(dim: usize, points: &[f64]) -> (usize, Classified<'_>, bool) {
        let built = || SimplicialHull::build(accept(dim, points).unwrap()).unwrap();
        let sequential = built();
        let chain = sequential.strict_edges;
        let mut proved = sequential.proved_interior.clone();
        proved.sort_unstable();

        let reused = classify_built(sequential, &mut FoundFaces::default()).unwrap();
        let mut reference = built();
        reference.proved_interior.clear();
        let reference = classify_built(reference, &mut FoundFaces::default()).unwrap();
        check(&reused);
        assert_eq!(reused.vertices, reference.vertices);
        assert_eq!(reused.coplanar_points, reference.coplanar_points);
        assert_eq!(reused.interior_points, reference.interior_points);
        for p in &proved {
            assert!(
                reference.interior_points.binary_search(p).is_ok(),
                "{p} is interior"
            );
        }
        (proved.len(), reused, chain)
    }

    #[test]
    fn recorded_planes_change_no_partition() {
        use crate::hull::records::Records;
        use crate::hull::simplicial::tests::RECORDED_SIGNS;

        let mut rng = Rng(306);
        let mut cases: Vec<(String, usize, Vec<f64>)> = Vec::new();
        for dim in 3..=5 {
            // Random points of a small grid: many points on facets, ridges,
            // and edges, so on planes construction records.
            let coarse: Vec<f64> = (0..80 * dim).map(|_| (rng.next() % 4) as f64).collect();
            cases.push((format!("coarse {dim}"), dim, coarse));
        }
        // Points on the surface of a cube, one coordinate at +-4.
        let surface: Vec<f64> = (0..300)
            .flat_map(|_| {
                let mut p: Vec<f64> = (0..3).map(|_| (rng.next() % 9) as f64 - 4.0).collect();
                let axis = (rng.next() % 3) as usize;
                p[axis] = if rng.next().is_multiple_of(2) {
                    -4.0
                } else {
                    4.0
                };
                p
            })
            .collect();
        cases.push(("cube surface".to_string(), 3, surface));
        for (name, dim, points) in &cases {
            let built = || SimplicialHull::build(accept(*dim, points).unwrap()).unwrap();
            RECORDED_SIGNS.with(|c| c.set(0));
            let recorded = built();
            assert!(
                RECORDED_SIGNS.with(core::cell::Cell::get) > 0,
                "{name}: insertion took no sign from a record"
            );
            let groups = crate::hull::merge::merge(&recorded).unwrap();
            let others: Vec<u32> = classify_built(built(), &mut FoundFaces::default())
                .unwrap()
                .coplanar_points;
            let starts = recorded_groups(&recorded, &groups, &others);
            assert!(
                starts.iter().any(Option::is_some),
                "{name}: no boundary point starts from a record"
            );
            let mut cleared = built();
            cleared.records = Records::new(cleared.input.representative.len());
            // A point the record starts inside its facet's polygon is
            // left out of the facet's zero points (#504); every other one
            // is listed as without records.
            let (on_with, zeros_with) = distance_zeros(&recorded, &groups, &others).unwrap();
            let (on_without, zeros_without) = distance_zeros(&cleared, &groups, &others).unwrap();
            assert!(
                zeros_with.iter().all(<[u32]>::is_sorted),
                "{name}: a facet's zero points ascend"
            );
            assert_eq!(on_with, on_without, "{name}");
            let mut left_out = 0;
            for (g, (with, without)) in zeros_with.iter().zip(zeros_without.iter()).enumerate() {
                let mut work = PolygonWork::default();
                let polygon = if *dim == 3 {
                    facet_polygon(&recorded, &groups, g as u32, &mut work).unwrap()
                } else {
                    None
                };
                for &p in without {
                    if with.binary_search(&p).is_err() {
                        left_out += 1;
                        assert!(
                            polygon
                                .as_ref()
                                .is_some_and(|f| f.contains(recorded.input.point(p))),
                            "{name}: point {p} left out of facet {g}"
                        );
                    }
                }
                assert!(
                    with.iter().all(|p| without.binary_search(p).is_ok()),
                    "{name}"
                );
            }
            if name == "cube surface" {
                assert!(
                    left_out > 0,
                    "{name}: no point was decided inside its facet"
                );
            }
            let with = classify_built(recorded, &mut FoundFaces::default()).unwrap();
            let mut reference = built();
            reference.records = Records::new(reference.input.representative.len());
            let reference = classify_built(reference, &mut FoundFaces::default()).unwrap();
            check(&with);
            assert_eq!(with.vertices, reference.vertices, "{name}");
            assert_eq!(with.coplanar_points, reference.coplanar_points, "{name}");
            assert_eq!(with.interior_points, reference.interior_points, "{name}");
            assert_eq!(with.faces.vertices, reference.faces.vertices, "{name}");
            assert_eq!(with.faces.neighbors, reference.faces.neighbors, "{name}");
            let simplices = |c: &Classified<'_>| -> Vec<(Vec<u32>, u32)> {
                c.simplices
                    .iter()
                    .map(|s| (s.vertices.to_vec(), s.face))
                    .collect()
            };
            assert_eq!(simplices(&with), simplices(&reference), "{name}");
        }
    }

    #[test]
    fn reused_interior_proofs_change_no_partition() {
        let mut rng = Rng(73);
        let mut cases: Vec<(String, usize, Vec<f64>)> = Vec::new();
        for dim in 2..=4 {
            // Full integer grids: most points lie on facets, ridges, and
            // edges, where construction sees a zero sign.
            let side = 5_usize;
            let count = side.pow(dim as u32);
            let grid: Vec<f64> = (0..count)
                .flat_map(|i| (0..dim).map(move |a| ((i / side.pow(a as u32)) % side) as f64))
                .collect();
            cases.push((format!("grid {dim}"), dim, grid));
            // Random points of a small grid: duplicates and coplanar sets.
            let coarse: Vec<f64> = (0..60 * dim).map(|_| (rng.next() % 4) as f64).collect();
            cases.push((format!("coarse {dim}"), dim, coarse));
            // General position.
            let general: Vec<f64> = (0..200 * dim).map(|_| rng.unit()).collect();
            cases.push((format!("general {dim}"), dim, general));
        }
        let mut ball = Vec::new();
        for i in -3_i32..=3 {
            for j in -3_i32..=3 {
                for k in -3_i32..=3 {
                    if i * i + j * j + k * k <= 9 {
                        ball.extend([f64::from(i), f64::from(j), f64::from(k)]);
                    }
                }
            }
        }
        cases.push(("ball".to_string(), 3, ball));
        for (name, dim, points) in &cases {
            let (proved, c, chain) = with_and_without_reuse(*dim, points);
            if chain {
                // The chain proves only the points strictly inside its
                // discarding polygon.
                let non_vertices = c.interior_points.len() + c.coplanar_points.len();
                assert!(proved <= non_vertices, "{name}");
                if name.starts_with("general") {
                    assert!(proved > 0, "{name}: no point was discarded");
                }
                continue;
            }
            let non_vertices = c.interior_points.len() + c.coplanar_points.len();
            assert!(proved > 0, "{name}: no point was proved interior");
            if name.starts_with("general") {
                // No zero sign: every non-vertex is proved, including the
                // vertices a later insertion swallowed (design §3).
                assert_eq!(proved, non_vertices, "{name}");
            } else {
                // Zero signs leave points to the scan.
                assert!(proved < non_vertices, "{name}: no point was scanned");
            }
        }
    }
}
#[cfg(test)]
mod grid_regression {
    use super::tests::classified;
    use crate::hull::simplicial::tests::Rng;

    #[test]
    fn random_integer_grids_in_high_dimensions() {
        for dim in 4..=6 {
            // Seed 3 in 6D was the first failure: two facets split a shared
            // lower face differently before every facet used placing.
            for seed in [0, 3] {
                let mut rng = Rng(seed * 31 + dim as u64);
                let points: Vec<f64> = (0..35 * dim).map(|_| (rng.next() % 4) as f64).collect();
                classified(dim, &points);
            }
        }
    }
}
