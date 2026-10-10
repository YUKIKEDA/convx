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

use std::collections::HashMap;

use super::input::{accept, Input};
use super::merge::{merge, LogicalFacets};
use super::ridge::{fingerprint, pair_equal_keys};
use super::simplicial::SimplicialHull;
use super::ConvexHullError;
use crate::arena::{FacetId, SlotMarks};
use crate::lists::Lists;
use crate::predicates::{orient, orient_direction, Sign};
use crate::small::Small;

/// A simplex of the boundary complex.
#[derive(Clone)]
pub(crate) struct ComplexSimplex {
    /// D vertices.
    pub(crate) vertices: Small<u32, 8>,
    /// The face that contains this simplex.
    pub(crate) face: u32,
    /// `neighbors[i]` is the simplex across the ridge opposite
    /// `vertices[i]`. Empty for D = 1.
    pub(crate) neighbors: Small<u32, 8>,
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
    pub(crate) simplices: Vec<ComplexSimplex>,
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
    extremes: HashMap<Vec<u32>, Vec<u32>>,
}

/// The vertices of the hull of an accepted input, for a face one level
/// down: nothing else of its classification is read. `global` maps its
/// indices to the input's.
fn sub_vertices(
    input: Input<'_>,
    faces: &mut FoundFaces,
    global: &[u32],
) -> Result<Vec<u32>, ConvexHullError> {
    #[cfg(test)]
    tests::SUB_HULLS.with(|c| c.set(c.get() + 1));
    let hull = SimplicialHull::build(input)?;
    if hull.strict_edges {
        // The cycle vertices of a strict polygon are its extreme points.
        let mut vertices = hull.polygon.clone();
        vertices.sort_unstable();
        return Ok(vertices);
    }
    Ok(found_vertices(&hull, faces, Some(global))?.vertices)
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
    let mut on_cycle = vec![false; hull.input.representative.len()];
    for &v in &hull.polygon {
        on_cycle[v as usize] = true;
    }
    let others: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| !on_cycle[p as usize])
        .collect();
    let mut proved = vec![false; hull.input.representative.len()];
    for &p in &hull.proved_interior {
        debug_assert!(!on_cycle[p as usize], "a discarded point is not a vertex");
        proved[p as usize] = true;
    }
    let mut on_boundary = vec![false; others.len()];
    for (k, &point) in others.iter().enumerate() {
        if proved[point as usize] {
            debug_assert!(
                polygon_edge(&hull.input, &hull.polygon, point)?.is_none(),
                "point {point} was discarded and is not strictly inside"
            );
            continue;
        }
        if polygon_edge(&hull.input, &hull.polygon, point)?.is_some() {
            on_boundary[k] = true;
        }
    }

    let mut vertices = hull.polygon.clone();
    vertices.sort_unstable();
    // `others` is ascending, so both lists are too.
    let coplanar_points: Vec<u32> = others
        .iter()
        .zip(&on_boundary)
        .filter(|&(_, &b)| b)
        .map(|(&p, _)| p)
        .collect();
    let interior_points: Vec<u32> = others
        .iter()
        .zip(&on_boundary)
        .filter(|&(_, &b)| !b)
        .map(|(&p, _)| p)
        .collect();

    // The public order (design §5): edge `i` joins `cycle[i]` and
    // `cycle[i + 1]`, where `cycle` is the counterclockwise polygon from
    // its smallest vertex. Its neighbors are edges `i - 1` and `i + 1`. One
    // pass writes every list in that order; nothing is sorted.
    let first = (0..n).min_by_key(|&i| hull.polygon[i]).unwrap_or(0);
    let cycle = |i: usize| hull.polygon[(first + i) % n];
    let mut faces = Faces {
        vertices: Lists::with_capacity(n, 2 * n),
        neighbors: Lists::with_capacity(n, 2 * n),
    };
    let mut simplices = Vec::with_capacity(n);
    for i in 0..n {
        let (start, end) = (cycle(i), cycle(i + 1));
        let prev = (if i == 0 { n - 1 } else { i - 1 }) as u32;
        let next = ((i + 1) % n) as u32;
        faces.vertices.push(&[start.min(end), start.max(end)]);
        faces.neighbors.push(&[prev.min(next), prev.max(next)]);
        // `neighbors[i]` is the simplex across the ridge opposite `vertices[i]`.
        // The tip is first, so slot 0 faces the previous edge and slot 1 the next.
        simplices.push(ComplexSimplex {
            vertices: [end, start].as_slice().into(),
            face: i as u32,
            neighbors: [prev, next].as_slice().into(),
        });
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
struct Found {
    groups: LogicalFacets,
    on_complex: Vec<bool>,
    others: Vec<u32>,
    on_boundary: Vec<bool>,
    extremes: Vec<Option<Vec<u32>>>,
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
    let groups = merge(hull)?;
    // A group's member simplices, in outward order.
    let members = |g: usize| {
        groups.groups[g]
            .simplices
            .iter()
            .filter_map(|id| hull.facets.get(*id))
            .map(|s| s.vertices())
    };

    // Simplicial vertices.
    let mut on_complex = vec![false; hull.input.representative.len()];
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
    let mut skipped = vec![false; hull.input.representative.len()];
    for &p in &hull.proved_interior {
        debug_assert!(!on_complex[p as usize], "a dropped point is not a vertex");
        // A point leaves every outside set when it is proved interior, and
        // a lost vertex is in none, so no later plan meets it again.
        debug_assert!(!skipped[p as usize], "point {p} was proved interior twice");
        skipped[p as usize] = true;
    }
    let others: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| !on_complex[p as usize] && !skipped[p as usize])
        .collect();
    let (on_boundary, zero_points) = distance_zeros(hull, &groups, &others)?;

    // Extreme points of each group; `None` when they are the group's
    // vertices, a single simplex with no other point on its plane.
    let mut extremes: Vec<Option<Vec<u32>>> = Vec::with_capacity(groups.groups.len());
    for (g, group) in groups.groups.iter().enumerate() {
        if group.simplices.len() == 1 && zero_points[g].is_empty() {
            extremes.push(None);
            continue;
        }
        let mut candidates = groups.vertices.get(g).to_vec();
        candidates.extend_from_slice(&zero_points[g]);
        candidates.sort_unstable();
        candidates.dedup();
        let plane = members(g).next().unwrap_or(&[]);
        extremes.push(Some(face_extremes(
            &hull.input,
            plane,
            &candidates,
            faces,
            global,
        )?));
    }

    let mut is_vertex = vec![false; hull.input.representative.len()];
    for (g, e) in extremes.iter().enumerate() {
        for &v in e.as_deref().unwrap_or(groups.vertices.get(g)) {
            is_vertex[v as usize] = true;
        }
    }
    let vertices: Vec<u32> = hull
        .input
        .representatives
        .iter()
        .copied()
        .filter(|&p| is_vertex[p as usize])
        .collect();
    Ok(Found {
        groups,
        on_complex,
        others,
        on_boundary,
        extremes,
        is_vertex,
        vertices,
    })
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
    let mut simplices: Vec<ComplexSimplex> = Vec::with_capacity(extremes.len());
    let mut faces = Faces {
        vertices: Lists::with_capacity(extremes.len(), extremes.len() * d),
        neighbors: Lists::with_capacity(extremes.len(), extremes.len() * d),
    };
    // Simplices kept as built, by their index here and their construction id.
    let mut kept: Vec<(u32, FacetId)> = Vec::with_capacity(extremes.len());
    let unlinked = if d == 1 { 0 } else { d };
    for (g, extreme) in extremes.into_iter().enumerate() {
        let group = &groups.groups[g];
        let group_vertices = groups.vertices.get(g);
        let first = simplices.len() as u32;
        let mut push = |vertices: Small<u32, 8>| {
            simplices.push(ComplexSimplex {
                vertices,
                face: g as u32,
                neighbors: core::iter::repeat_n(UNLINKED, unlinked).collect(),
            });
        };
        // A single simplex whose vertices are all extreme is kept; every
        // other facet is re-triangulated by placing, so facets that share a
        // lower face split it the same way.
        let single = group.simplices.len() == 1;
        match extreme {
            Some(extreme) if !(single && extreme == group_vertices) => {
                let q = vertices
                    .iter()
                    .copied()
                    .find(|v| extreme.binary_search(v).is_err())
                    .unwrap_or(extreme[0]);
                for members in place(&hull.input, &extreme, q)? {
                    push(members.into());
                }
                faces.vertices.push(&extreme);
            }
            _ => {
                if let Some(&id) = group.simplices.first() {
                    if let Some(s) = hull.facets.get(id) {
                        kept.push((first, id));
                        push(s.vertices().into());
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
                    simplices[s as usize].neighbors[slot] = other;
                }
            }
        }
    }
    #[cfg(debug_assertions)]
    let mut paired_all = (simplices.clone(), faces.clone());
    link_neighbors(d, &mut simplices, &faces);
    #[cfg(debug_assertions)]
    {
        for simplex in &mut paired_all.0 {
            simplex.neighbors.fill(UNLINKED);
        }
        link_neighbors(d, &mut paired_all.0, &paired_all.1);
        debug_assert!(
            simplices
                .iter()
                .zip(&paired_all.0)
                .all(|(a, b)| a.neighbors == b.neighbors),
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
/// points at distance zero, each list unordered.
type Zeros = (Vec<bool>, Vec<Vec<u32>>);

/// The exact distance-zero signs of `others` against every group. Points
/// construction proved strictly inside are not among `others`.
fn distance_zeros(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    others: &[u32],
) -> Result<Zeros, ConvexHullError> {
    let mut on_boundary = vec![false; others.len()];
    let mut zero_points: Vec<Vec<u32>> = vec![Vec::new(); groups.groups.len()];
    // The group of a recorded number of each other point, if a simplex of
    // the final complex carries one, and the points with none, which are
    // scanned against every group.
    let starts = recorded_groups(hull, groups, others);
    let scanned: Vec<u32> = others
        .iter()
        .zip(&starts)
        .filter(|&(_, start)| start.is_none())
        .map(|(&p, _)| p)
        .collect();
    let scanned_at: Vec<usize> = (0..others.len()).filter(|&k| starts[k].is_none()).collect();
    if !scanned.is_empty() {
        let mut sides = vec![None; scanned.len()];
        for (g, group) in groups.groups.iter().enumerate() {
            let Some(simplex) = group.simplices.first().and_then(|id| hull.facets.get(*id)) else {
                continue;
            };
            sides.fill(None);
            if let Some(cull) = simplex.cull() {
                let (rows, stride) = hull.input.rows();
                let origin = hull.input.point(simplex.vertices()[0]);
                cull.mark_sides(origin, rows, stride, &scanned, &mut sides);
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
                    zero_points[g].push(p);
                }
            }
        }
    }
    // A recorded point is on its start group's plane. The groups at
    // distance zero from it are connected through neighbors, and every
    // other group is strictly negative (design §3).
    let mut tested = vec![usize::MAX; groups.groups.len()];
    let mut stack: Vec<u32> = Vec::new();
    for (k, (&p, start)) in others.iter().zip(&starts).enumerate() {
        let Some(start) = *start else {
            continue;
        };
        let side_of = |g: u32| -> Result<Sign, ConvexHullError> {
            match groups.groups[g as usize]
                .simplices
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
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            side_of(start)?,
            Sign::Zero,
            "point {p} is recorded on a plane it is off"
        );
        on_boundary[k] = true;
        zero_points[start as usize].push(p);
        tested[start as usize] = k;
        stack.clear();
        stack.push(start);
        while let Some(g) = stack.pop() {
            for &n in groups.neighbors.get(g as usize) {
                if tested[n as usize] == k {
                    continue;
                }
                tested[n as usize] = k;
                let side = side_of(n)?;
                debug_assert!(
                    side != Sign::Positive,
                    "a point is outside the finished hull"
                );
                if side == Sign::Zero {
                    zero_points[n as usize].push(p);
                    stack.push(n);
                }
            }
        }
        // Debug builds check the walk against every group.
        #[cfg(debug_assertions)]
        for g in 0..groups.groups.len() as u32 {
            if tested[g as usize] != k {
                debug_assert_eq!(
                    side_of(g)?,
                    Sign::Negative,
                    "the walk from a record missed a facet through point {p}"
                );
            }
        }
    }
    Ok((on_boundary, zero_points))
}

/// Per point of `others`, the group of a simplex of the final complex whose
/// plane number construction recorded for that point, if any.
fn recorded_groups(
    hull: &SimplicialHull<'_>,
    groups: &LogicalFacets,
    others: &[u32],
) -> Vec<Option<u32>> {
    let mut starts = vec![None; others.len()];
    if hull.records.is_empty() {
        return starts;
    }
    // Simplices with one number lie in one plane, so in one logical facet.
    let mut numbers: Vec<(u32, u32)> = groups
        .groups
        .iter()
        .enumerate()
        .flat_map(|(g, group)| {
            group
                .simplices
                .iter()
                .filter_map(|id| hull.facets.get(*id))
                .map(move |s| (s.number(), g as u32))
        })
        .collect();
    numbers.sort_unstable();
    for (start, &p) in starts.iter_mut().zip(others) {
        *start = hull.records.numbers(p).find_map(|n| {
            numbers
                .binary_search_by_key(&n, |&(number, _)| number)
                .ok()
                .map(|i| numbers[i].1)
        });
    }
    starts
}

/// Extreme points of `candidates`, which lie on the hyperplane through the
/// D affinely independent points `plane`.
fn face_extremes(
    input: &Input<'_>,
    plane: &[u32],
    candidates: &[u32],
    faces: &mut FoundFaces,
    global: Option<&[u32]>,
) -> Result<Vec<u32>, ConvexHullError> {
    let d = input.dim();
    if d == 1 {
        return Ok(candidates.to_vec());
    }
    // The candidates as input indices. Each level's candidates ascend and
    // map to ascending indices, so the key ascends too.
    let key: Vec<u32> = match global {
        Some(global) => candidates.iter().map(|&c| global[c as usize]).collect(),
        None => candidates.to_vec(),
    };
    debug_assert!(key.is_sorted(), "a face's key ascends");
    // Tests compare the published result with and without the memo.
    #[cfg(test)]
    let forget = !tests::FACES_REMEMBERED.with(core::cell::Cell::get);
    #[cfg(not(test))]
    let forget = false;
    if !forget {
        if let Some(found) = faces.extremes.get(&key) {
            // Back to this level's indices, through the key's order.
            return Ok(found
                .iter()
                .map(|v| candidates[key.binary_search(v).unwrap_or_default()])
                .collect());
        }
    }
    // An axis along which the hyperplane is not vertical: its cofactor is
    // nonzero, so dropping that coordinate is a bijection of the hyperplane.
    let plane_points: Vec<&[f64]> = plane.iter().map(|&v| input.point(v)).collect();
    let mut unit = vec![0.0; d];
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
        return Ok(candidates.to_vec());
    };
    let projected: Vec<f64> = candidates
        .iter()
        .flat_map(|&p| {
            input
                .point(p)
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != axis)
                .map(|(_, &x)| x)
        })
        .collect();
    let sub = sub_vertices(accept(d - 1, &projected)?, faces, &key)?;
    let extremes: Vec<u32> = sub.iter().map(|&i| candidates[i as usize]).collect();
    faces.extremes.insert(
        key,
        sub.iter()
            .map(|&i| global_of(global, candidates[i as usize]))
            .collect(),
    );
    Ok(extremes)
}

/// The input index of `v`, an index of a level that `global` maps.
fn global_of(global: Option<&[u32]>, v: u32) -> u32 {
    global.map_or(v, |g| g[v as usize])
}

/// Placing triangulation of the extreme points `extreme` (ascending) of one
/// facet, oriented so that `q`, a hull vertex off the facet, is inside.
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
fn place(input: &Input<'_>, extreme: &[u32], q: u32) -> Result<Vec<Vec<u32>>, ConvexHullError> {
    let d = input.dim();
    let simplices = placing(d, |i| input.point(i), extreme)?;
    debug_assert!(
        simplices.iter().all(|s| s.len() == d),
        "a facet spans D - 1 dimensions"
    );
    simplices
        .into_iter()
        .map(|s| oriented(input, s, q))
        .collect()
}

/// The placing triangulation of `points` (ascending), coordinates of
/// dimension `d` given by `point`: simplices of `r + 1` vertices for the
/// affine dimension `r` of the points, unoriented. See [`place`].
///
/// The boundary ridges of the current complex are kept from one point to
/// the next (#393). A point that does not raise the dimension adds a simplex
/// on each boundary ridge it is beyond, and each ridge of the added
/// simplices then leaves the boundary if it was on it, or joins it: in a
/// triangulation every ridge lies on one simplex or two. A point that
/// raises the dimension cones every simplex, and the boundary is gathered
/// again. Each boundary ridge keeps the side of its opposite vertex, which
/// does not change until the dimension does.
pub(crate) fn placing<'p>(
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
    // The orientation of `vertices` projected onto `axes`. The projected
    // rows go into one buffer reused across calls; a `Vec` per row was most
    // of the time left after the boundary was kept (#393).
    let mut rows: Vec<f64> = Vec::new();
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
    let mut boundary = Boundary::of(&simplices);
    for &p in rest {
        // Does p raise the affine dimension?
        let mut raised = None;
        for j in (0..d).filter(|j| !axes.contains(j)) {
            let with_j: Small<usize, 11> = axes.iter().copied().chain([j]).collect();
            let mut vertices = basis.iter().copied().chain([p]);
            if projected_sign(&mut vertices, &with_j)? != Sign::Zero {
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
            boundary = Boundary::of(&simplices);
            continue;
        }
        // Beyond which boundary ridges of the current complex is p?
        let mut added = Vec::new();
        for (ridge, (opposite, known)) in &mut boundary.ridges {
            let sp = projected_sign(&mut ridge.iter().copied().chain([p]), &axes)?;
            if sp == Sign::Zero {
                continue;
            }
            let sa = match *known {
                Some(sign) => sign,
                None => {
                    let sign =
                        projected_sign(&mut ridge.iter().copied().chain([*opposite]), &axes)?;
                    *known = Some(sign);
                    sign
                }
            };
            if sa != Sign::Zero && sp != sa {
                let mut vertices = ridge.clone();
                vertices.push(p);
                added.push(vertices);
            }
        }
        added.sort_unstable();
        for simplex in &added {
            boundary.toggle(simplex);
        }
        simplices.extend(added);
    }
    Ok(simplices)
}

/// The boundary ridges of a placing triangulation: each ridge (ascending)
/// that lies on one simplex, with that simplex's vertex opposite it and,
/// once evaluated, that vertex's side of the ridge.
struct Boundary {
    ridges: HashMap<Vec<u32>, (u32, Option<Sign>)>,
}

impl Boundary {
    /// The boundary of `simplices`.
    fn of(simplices: &[Vec<u32>]) -> Self {
        let mut boundary = Self {
            ridges: HashMap::new(),
        };
        for simplex in simplices {
            boundary.toggle(simplex);
        }
        boundary
    }

    /// Adds `simplex`: each of its ridges leaves the boundary when it was
    /// on it (it now lies on two simplices), and joins it otherwise.
    fn toggle(&mut self, simplex: &[u32]) {
        for slot in 0..simplex.len() {
            let mut ridge: Vec<u32> = simplex
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != slot)
                .map(|(_, &v)| v)
                .collect();
            ridge.sort_unstable();
            match self.ridges.entry(ridge) {
                std::collections::hash_map::Entry::Occupied(on_one) => {
                    on_one.remove();
                }
                std::collections::hash_map::Entry::Vacant(free) => {
                    free.insert((simplex[slot], None));
                }
            }
        }
    }
}

/// Orders `vertices` so that `q` is on the negative side.
fn oriented(
    input: &Input<'_>,
    mut vertices: Vec<u32>,
    q: u32,
) -> Result<Vec<u32>, ConvexHullError> {
    let mut points: Vec<&[f64]> = vertices.iter().map(|&v| input.point(v)).collect();
    points.push(input.point(q));
    if orient(&points)? == Sign::Positive && vertices.len() >= 2 {
        vertices.swap(0, 1);
    }
    Ok(vertices)
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
fn link_neighbors(d: usize, simplices: &mut [ComplexSimplex], faces: &Faces) {
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
        simplices[a as usize].neighbors[slot_a] = b;
        simplices[b as usize].neighbors[slot_b] = a;
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
                    .filter_map(|&n| simplices.get(n as usize).map(|t| t.face))
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

    /// The placing triangulation before #393: the ridges of every simplex
    /// gathered again for each point, and both sides evaluated for each
    /// boundary ridge. The reference of [`placing`]'s tests.
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
                let other = &c.simplices[n as usize];
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
            for v in &simplex.vertices {
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
                        let got = placing(d, point, &extreme).unwrap();
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
                assert_eq!(placing(k, point, extreme).unwrap(), expected, "{name} D{k}");
                found += 1;
            }
            assert!(found > 0, "{name} D{k}: no facet that is not a simplex");
            faces += found;
        }
        assert!(faces > 30, "{faces} faces of real hulls");
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
            let groups = merge(&recorded).unwrap();
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
            let sorted = |(on, mut zeros): Zeros| {
                zeros.iter_mut().for_each(|z| z.sort_unstable());
                (on, zeros)
            };
            assert_eq!(
                sorted(distance_zeros(&recorded, &groups, &others).unwrap()),
                sorted(distance_zeros(&cleared, &groups, &others).unwrap()),
                "{name}"
            );
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
