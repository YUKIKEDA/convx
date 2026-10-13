//! Expectations come from each case's statement or from an independent
//! brute-force reference in `i128`, never from the library's own output.

use super::*;

thread_local! {
    /// Makes D = 2 and D = 3 insert with the shape of any dimension
    /// ([`insert::Any`]) on this thread, to compare the shapes.
    pub(super) static GENERIC_INSERTION: core::cell::Cell<bool> =
        const { core::cell::Cell::new(false) };
    /// Makes the insertion store the sites in the input's order on this
    /// thread, to compare with the order of insertion (#366).
    pub(super) static STORED_IN_INPUT_ORDER: core::cell::Cell<bool> =
        const { core::cell::Cell::new(false) };
    /// Makes the pass after the insertion test every face left unknown by
    /// the lifted orientation on this thread, to compare with the
    /// orientation that decides most of them (#451).
    pub(super) static EVERY_UNKNOWN_FACE_LIFTED: core::cell::Cell<bool> =
        const { core::cell::Cell::new(false) };
    /// Turns the table of translates off on this thread, to compare a
    /// build with it and without (#637).
    pub(super) static TRANSLATES_OFF: core::cell::Cell<bool> =
        const { core::cell::Cell::new(false) };
    /// How many groups took the cells of a group placed before, on this
    /// thread.
    pub(super) static TRANSLATES_FOUND: core::cell::Cell<usize> =
        const { core::cell::Cell::new(0) };
}

/// The triangulation of `points` with the table of translates and without
/// it, and how many groups took the cells of another.
fn with_and_without_translates(
    dim: usize,
    points: &[f64],
) -> (DelaunayTriangulation, DelaunayTriangulation, usize) {
    TRANSLATES_FOUND.with(|c| c.set(0));
    let with = DelaunayBuilder::new(dim, points).build().unwrap();
    let found = TRANSLATES_FOUND.with(core::cell::Cell::get);
    TRANSLATES_OFF.with(|c| c.set(true));
    let without = DelaunayBuilder::new(dim, points).build();
    TRANSLATES_OFF.with(|c| c.set(false));
    (with, without.unwrap(), found)
}

/// Every point of the integer lattice of `side` points a side in `dim`
/// dimensions, sheared by `shear` along the first coordinate, times `scale`,
/// plus `shift`.
fn lattice_points(dim: usize, side: usize, shear: f64, scale: f64, shift: f64) -> Vec<f64> {
    let mut points = Vec::new();
    for i in 0..side.pow(dim as u32) {
        let mut cell: Vec<f64> = (0..dim)
            .map(|a| ((i / side.pow(a as u32)) % side) as f64)
            .collect();
        cell[0] += shear * cell[1];
        points.extend(cell.iter().map(|x| x * scale + shift));
    }
    points
}

/// A group that is an exact translate of one placed before takes its cells
/// (#637), and the triangulation published is the one of placing every
/// group. Lattices of D = 3 and 4, plain, sheared, scaled by a power of two
/// and moved; a lattice times 0.1 and one moved by 0.3, whose differences
/// round and whose groups are placed; a lattice among random sites; boxes
/// of forty different shapes, more than the table holds; and in D = 2
/// copies of the eight integer points of a circle, groups of more than four
/// sites. On a lattice of exact coordinates every group but the first is
/// found.
#[test]
fn a_translate_of_a_placed_group_takes_its_cells() {
    let mut state = 637_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1_u64 << 53) as f64
    };
    // The name, the dimension, the points, and the fewest and the most
    // groups that take the cells of another. A sheared lattice has groups
    // of other shapes than its unit cells; a difference of two neighboring
    // values is often exact where the values are not round, so some groups
    // of a lattice times 0.1 are found as well, rightly: with exact
    // differences they are translates.
    // A lattice of s points a side has (s - 1)^D unit cells, one group each,
    // and all but the first placed are found.
    let mut cases: Vec<(&str, usize, Vec<f64>, usize, usize)> = vec![(
        "lattice D3",
        3,
        lattice_points(3, 6, 0.0, 1.0, 0.0),
        124,
        124,
    )];
    cases.push((
        "sheared D3",
        3,
        lattice_points(3, 5, 1.0, 1.0, 0.0),
        1,
        usize::MAX,
    ));
    cases.push((
        "scaled and moved D3",
        3,
        lattice_points(3, 5, 0.0, 0.25, 1024.0),
        63,
        63,
    ));
    cases.push(("lattice D4", 4, lattice_points(4, 4, 0.0, 1.0, 0.0), 80, 80));
    // Differences that round: 0.1 has no finite binary form, and 0.3 moves
    // every site off the lattice by a rounding of its own.
    cases.push((
        "times 0.1 D3",
        3,
        lattice_points(3, 5, 0.0, 0.1, 0.0),
        0,
        63,
    ));
    cases.push((
        "moved by 0.3 D3",
        3,
        lattice_points(3, 5, 0.0, 1.0, 0.3),
        0,
        63,
    ));
    let mut mixed = lattice_points(3, 5, 0.0, 1.0, 0.0);
    mixed.extend((0..120).map(|_| 6.0 + 4.0 * next()));
    cases.push(("lattice and random sites", 3, mixed, 40, 63));
    // Boxes of forty shapes, far apart: the corners of each are one group.
    let mut boxes = Vec::new();
    for j in 0..40 {
        let height = 1.0 + j as f64;
        for corner in 0..8 {
            boxes.extend([
                1000.0 * j as f64 + f64::from(corner & 1),
                f64::from(corner >> 1 & 1) * 2.0,
                f64::from(corner >> 2 & 1) * height,
            ]);
        }
    }
    cases.push(("forty boxes", 3, boxes, 0, 40));
    // The eight integer points of the circle of radius 5 through (5, 0)
    // and (3, 4), copied along a row, far enough apart to stay groups.
    let octagon = [
        [5.0, 0.0],
        [3.0, 4.0],
        [-3.0, 4.0],
        [-5.0, 0.0],
        [-3.0, -4.0],
        [3.0, -4.0],
        [0.0, 5.0],
        [0.0, -5.0],
    ];
    let mut circles = Vec::new();
    for copy in 0..12 {
        for corner in octagon {
            circles.extend([corner[0] + 64.0 * f64::from(copy), corner[1]]);
        }
    }
    cases.push(("circles D2", 2, circles, 11, 11));
    for (name, dim, points, fewest, most) in cases {
        let (with, without, found) = with_and_without_translates(dim, &points);
        assert!(with == without, "{name}");
        assert!(
            (fewest..=most).contains(&found),
            "{name}: {found} groups found"
        );
    }
}

/// The table of translates by itself: a group is found when its
/// differences are those of a group kept, in the same order; a group whose
/// difference rounds to them is not; the table keeps sixteen groups and is
/// given up at the seventeenth it does not hold.
#[test]
fn the_table_of_translates_takes_exact_differences_and_a_fixed_number_of_groups() {
    let points: Vec<[f64; 2]> = vec![
        [0.0, 0.0],
        [1e16, 3.0],
        [5.0, 1.0],
        // The same three moved by (2, 2): exact differences.
        [2.0, 2.0],
        [1e16 + 2.0, 5.0],
        [7.0, 3.0],
        // From (1, 0) the first difference is 1e16 - 1, which rounds to
        // 1e16, and the others are those of the first group.
        [1.0, 0.0],
        [1e16, 3.0],
        [6.0, 1.0],
        // The first group with its last two sites exchanged.
        [0.0, 0.0],
        [5.0, 1.0],
        [1e16, 3.0],
    ];
    let point = |i: u32| &points[i as usize][..];
    let cells = [0_u32, 1, 2];
    let mut table = Translates::default();
    assert_eq!(table.find(point, &[0, 1, 2]), None);
    table.keep(&[0, 1, 2], &cells, &[true]);
    assert_eq!(table.find(point, &[3, 4, 5]), Some(0));
    let kept: Vec<(Vec<u32>, bool)> = table.cells(0, 3).map(|(c, n)| (c.to_vec(), n)).collect();
    assert_eq!(kept, [(vec![0, 1, 2], true)]);
    assert_eq!(table.find(point, &[6, 7, 8]), None);
    // Not kept either: its key was not taken.
    table.keep(&[6, 7, 8], &[6, 7, 8], &[false]);
    assert_eq!(table.key_ends.len(), 1);
    assert_eq!(table.find(point, &[9, 10, 11]), None);
    // Sixteen groups, each of another shape, then a seventeenth.
    let shapes: Vec<[f64; 2]> = (0..40)
        .flat_map(|j| [[0.0, 0.0], [1.0, f64::from(j)]])
        .collect();
    let shape = |i: u32| &shapes[i as usize][..];
    let mut table = Translates::default();
    for j in 0..16_u32 {
        assert_eq!(table.find(shape, &[2 * j, 2 * j + 1]), None);
        table.keep(&[2 * j, 2 * j + 1], &[2 * j, 2 * j + 1], &[false]);
    }
    assert_eq!(table.key_ends.len(), TRANSLATES);
    assert_eq!(table.find(shape, &[10, 11]), Some(5));
    assert!(!table.given_up);
    assert_eq!(table.find(shape, &[32, 33]), None);
    assert!(table.given_up);
    table.keep(&[32, 33], &[32, 33], &[false]);
    assert_eq!(table.key_ends.len(), TRANSLATES);
    assert_eq!(table.find(shape, &[10, 11]), None);
}

/// A group of two triangles of D = 2 is written as the placing writes it
/// (#633): the same four sites, the same two cells in the same order, and
/// for each cell the sign of its ascending order, which the exact
/// orientation gives. Every four of the twelve integer points of the circle
/// of radius 5, under every assignment of indices to them, with the mesh's
/// diagonal either way and its triangles in either order and from every
/// corner. Both answers occur: the mesh's diagonal kept, and the other.
#[test]
fn a_group_of_two_triangles_is_the_placing_triangulation() {
    // Counterclockwise from (5, 0).
    let circle: [[f64; 2]; 12] = [
        [5.0, 0.0],
        [4.0, 3.0],
        [3.0, 4.0],
        [0.0, 5.0],
        [-3.0, 4.0],
        [-4.0, 3.0],
        [-5.0, 0.0],
        [-4.0, -3.0],
        [-3.0, -4.0],
        [0.0, -5.0],
        [3.0, -4.0],
        [4.0, -3.0],
    ];
    let mut placing = Placing::default();
    let (mut kept, mut other, mut cases) = (0, 0, 0);
    for mask in 0_u32..1 << 12 {
        if mask.count_ones() != 4 {
            continue;
        }
        let quad: Vec<usize> = (0..12).filter(|i| mask >> i & 1 == 1).collect();
        // The index of each corner, counterclockwise: every order of 0..4.
        for order in 0..24 {
            let mut left = vec![0_u32, 1, 2, 3];
            let mut index = [0_u32; 4];
            let mut code = order;
            for slot in &mut index {
                *slot = left.remove(code % left.len());
                code /= left.len() + 1;
            }
            let mut points = [[0.0; 2]; 4];
            for corner in 0..4 {
                points[index[corner] as usize] = circle[quad[corner]];
            }
            let point = |i: u32| &points[i as usize][..];
            let (len, split) = placing.run(2, Some(2), point, &[0, 1, 2, 3]).unwrap();
            assert_eq!(len, 3);
            let expected: Vec<[u32; 3]> = split.as_chunks::<3>().0.to_vec();
            assert_eq!(expected.len(), 2);
            let q = index;
            for diagonal in 0..2 {
                // The two triangles of the diagonal through corner
                // `diagonal`, counterclockwise.
                let d = diagonal;
                let a = [q[d], q[d + 1], q[d + 2]];
                let b = [q[d], q[d + 2], q[(d + 3) % 4]];
                for turn in 0..3 {
                    let turned =
                        |t: [u32; 3], by: usize| [t[by % 3], t[(by + 1) % 3], t[(by + 2) % 3]];
                    for (first, second) in [
                        (turned(a, turn), b),
                        (b, turned(a, turn)),
                        (a, turned(b, turn)),
                    ] {
                        let (sites, cells) = quadrilateral(first, second);
                        assert_eq!(sites, [0, 1, 2, 3]);
                        for (cell, expected) in cells.iter().zip(&expected) {
                            let (sorted, _, negative) = ascending_fixed(*cell);
                            assert_eq!(&sorted, expected, "{quad:?} {index:?}");
                            let exact =
                                orient(&[point(sorted[0]), point(sorted[1]), point(sorted[2])])
                                    .unwrap();
                            assert_eq!(negative, exact == Sign::Negative, "{quad:?} {index:?}");
                            assert_ne!(exact, Sign::Zero);
                        }
                        let same =
                            |x: [u32; 3], y: [u32; 3]| ascending_fixed(x).0 == ascending_fixed(y).0;
                        if same(cells[0], first) || same(cells[0], second) {
                            kept += 1;
                        } else {
                            other += 1;
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 495 * 24 * 2 * 3 * 3);
    assert!(kept > 10_000 && other > 10_000, "{kept} {other}");
}

/// The faces the insertion leaves unknown are decided by an orientation,
/// and by the lifted orientation only when that is zero (design §7, #451).
/// The simplices published, cospherical groups included, and their
/// neighbors are those of testing every such face by the lifted
/// orientation. Lattices of D = 2 to 5,
/// where most faces left unknown are cospherical and many of their sites
/// are affinely dependent; integer grids with duplicates; sites near and
/// on one sphere; and general position.
#[test]
fn faces_left_unknown_need_the_lifted_test_only_when_dependent() {
    let mut state = 451_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for dim in 2..=5_usize {
        let side: u32 = [0, 0, 12, 6, 4, 3][dim];
        let lattice: Vec<f64> = (0..side.pow(dim as u32))
            .flat_map(|i| (0..dim).map(move |a| f64::from(i / side.pow(a as u32) % side)))
            .collect();
        let count = [0, 0, 300, 150, 60, 30][dim];
        let grid: Vec<f64> = (0..dim * count).map(|_| (next() % 5) as f64).collect();
        let unit = |next: &mut dyn FnMut() -> u64| {
            2.0 * ((next() >> 11) as f64 / (1_u64 << 53) as f64) - 1.0
        };
        let general: Vec<f64> = (0..dim * count).map(|_| unit(&mut next)).collect();
        let mut near_sphere = Vec::with_capacity(dim * count);
        for _ in 0..count {
            let v: Vec<f64> = (0..dim).map(|_| unit(&mut next)).collect();
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            near_sphere.extend(v.iter().map(|x| x / norm));
        }
        // The integer points of [-2, 2]^D on one sphere (squared radius 5
        // for D = 2, 8 points; 4 above: 6 points in D = 3, 24 in D = 4, 90 in
        // D = 5), with its center, so the lift is not flat.
        let radius = if dim == 2 { 5.0 } else { 4.0 };
        let mut on_sphere: Vec<f64> = vec![0.0; dim];
        for i in 0..5_u32.pow(dim as u32) {
            let p: Vec<f64> = (0..dim)
                .map(|a| f64::from(i / 5_u32.pow(a as u32) % 5) - 2.0)
                .collect();
            if p.iter().map(|x| x * x).sum::<f64>() == radius {
                on_sphere.extend(p);
            }
        }
        for (family, points) in [
            ("lattice", &lattice),
            ("grid", &grid),
            ("general", &general),
            ("near sphere", &near_sphere),
            ("on sphere", &on_sphere),
        ] {
            assert!(!is_flat(dim, points), "D = {dim}, {family}: inserted");
            let gated = DelaunayBuilder::new(dim, points).build().unwrap();
            EVERY_UNKNOWN_FACE_LIFTED.with(|c| c.set(true));
            let lifted = DelaunayBuilder::new(dim, points).build();
            EVERY_UNKNOWN_FACE_LIFTED.with(|c| c.set(false));
            let lifted = lifted.unwrap();
            // The merged groups are numbered by the order their faces were
            // joined, which the skipped tests change, so the simplices are
            // compared as sets (design §7 fixes no order across builds that
            // decide differently).
            assert_eq!(rows(&gated), rows(&lifted), "D = {dim}, {family}");
            check(&gated, points);
        }
    }
}

/// The triangulation of `points` by the shape of its dimension, and by the
/// shape of any dimension.
fn by_both_shapes(dim: usize, points: &[f64]) -> [DelaunayTriangulation; 2] {
    let specialized = DelaunayBuilder::new(dim, points).build().unwrap();
    GENERIC_INSERTION.with(|c| c.set(true));
    let generic = DelaunayBuilder::new(dim, points).build();
    GENERIC_INSERTION.with(|c| c.set(false));
    [specialized, generic.unwrap()]
}

/// The insertion specialized for D = 2 and for D = 3 publishes what the
/// insertion of any dimension publishes (design §7, ADR 0005): the same
/// simplices, diagonals among cospherical sites included, with the same
/// neighbors. D = 2 inserts by edge flips (ADR 0007), so its simplices are
/// compared as sets; D = 3 also in their order. General position, integer
/// grids with many cospherical groups, lattices with duplicates, sites near
/// and on one sphere, and a flat lift, which does not insert.
#[test]
fn the_shapes_publish_the_same_triangulation() {
    let mut state = 294_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let unit = |next: &mut dyn FnMut() -> u64| (next() >> 11) as f64 / (1_u64 << 53) as f64;
    for dim in [2_usize, 3] {
        for (family, count) in [
            ("general", 400),
            ("grid", 300),
            ("lattice", 200),
            ("near sphere", 150),
            ("on sphere", 12),
        ] {
            let mut points = Vec::with_capacity(dim * count);
            for _ in 0..count {
                let mut v: Vec<f64> = (0..dim).map(|_| 2.0 * unit(&mut next) - 1.0).collect();
                match family {
                    // Integer points of [0, 7]^D: many cospherical groups.
                    "grid" => v.iter_mut().for_each(|x| *x = (next() % 8) as f64),
                    // Integer points of [-2, 2]^D, with duplicates.
                    "lattice" => v.iter_mut().for_each(|x| *x = (next() % 5) as f64 - 2.0),
                    // On the unit sphere up to the rounding of a division.
                    "near sphere" => {
                        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
                        v.iter_mut().for_each(|x| *x /= norm);
                    }
                    // Exactly on one sphere: signs of unit axes.
                    "on sphere" => {
                        let axis = (next() % dim as u64) as usize;
                        let sign = if next() % 2 == 0 { 1.0 } else { -1.0 };
                        v.iter_mut().for_each(|x| *x = 0.0);
                        v[axis] = sign;
                    }
                    _ => {}
                }
                points.extend(v);
            }
            if family == "on sphere" {
                // Both signs of every axis, so the sites span the space.
                points.clear();
                for axis in 0..dim {
                    for sign in [1.0, -1.0] {
                        let mut v = vec![0.0; dim];
                        v[axis] = sign;
                        points.extend(v);
                    }
                }
                assert!(is_flat(dim, &points), "D = {dim}: the lift is flat");
            } else {
                assert!(!is_flat(dim, &points), "D = {dim}, {family}: inserted");
            }
            let [specialized, generic] = by_both_shapes(dim, &points);
            // D = 2 inserts by edge flips (ADR 0007), which create the
            // simplices, and so publish them, in another order; D = 3 runs
            // the same steps as the generic insertion.
            if dim == 3 {
                assert_eq!(specialized, generic, "D = {dim}, {family}");
            }
            assert_eq!(rows(&specialized), rows(&generic), "D = {dim}, {family}");
            check(&specialized, &points);
        }
    }
}

/// The published order is the construction's (design §7): deterministic,
/// so two builds of one input are equal under `==`, order included, on
/// general, grid, and cospherical inputs, and on a flat lift.
#[test]
fn one_input_publishes_one_order() {
    let mut state = 335_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for dim in 1..=4 {
        let general: Vec<f64> = (0..dim * 60)
            .map(|_| (next() % 1_000_003) as f64 / 7.0)
            .collect();
        let grid: Vec<f64> = (0..dim * 60).map(|_| (next() % 4) as f64).collect();
        let corners: Vec<f64> = (0..1_u32 << dim)
            .flat_map(|i| (0..dim).map(move |a| f64::from((i >> a) & 1)))
            .collect();
        for points in [&general, &grid, &corners] {
            let a = DelaunayBuilder::new(dim, points).build().unwrap();
            let b = DelaunayBuilder::new(dim, points).build().unwrap();
            assert!(a == b, "D = {dim}");
            assert!(a
                .simplices()
                .iter()
                .zip(b.simplices().iter())
                .all(|(x, y)| x.vertices() == y.vertices() && x.neighbors() == y.neighbors()));
        }
    }
}

/// The override bites: with it, D = 2 runs the code of any dimension. The
/// two shapes are told apart by which predicate entry they call, which a
/// result cannot show, so this test checks the dispatch itself.
#[test]
fn the_override_selects_the_shape_of_any_dimension() {
    assert!(!generic_insertion());
    GENERIC_INSERTION.with(|c| c.set(true));
    let forced = generic_insertion();
    GENERIC_INSERTION.with(|c| c.set(false));
    assert!(forced);
    assert!(!generic_insertion());
}

/// Simplices with their neighbors named by vertex lists ([`named`]).
type Named = Vec<(Vec<u32>, Vec<Option<Vec<u32>>>)>;

/// Every simplex as its (vertices, neighbors), the neighbors named by their
/// vertex lists and the simplices sorted ([`named`]).
fn rows(t: &DelaunayTriangulation) -> Named {
    named(
        t.simplices()
            .iter()
            .map(|s| (s.vertices().to_vec(), s.neighbors().to_vec()))
            .collect(),
    )
}

/// `rows`, each neighbor named by its ascending vertex list instead of its
/// number, sorted by the vertices: the published order of the simplices is
/// the construction's (design §7), which these tests do not fix.
fn named(rows: Vec<(Vec<u32>, Vec<u32>)>) -> Named {
    let ascending = |v: &[u32]| {
        let mut v = v.to_vec();
        v.sort_unstable();
        v
    };
    let mut named: Named = rows
        .iter()
        .map(|(vertices, neighbors)| {
            let across = neighbors
                .iter()
                .map(|&n| (n != NO_NEIGHBOR).then(|| ascending(&rows[n as usize].0)))
                .collect();
            (vertices.clone(), across)
        })
        .collect();
    named.sort_by_key(|a| ascending(&a.0));
    named
}

/// Whether the lift of `points` is flat (every site on one sphere).
fn is_flat(dim: usize, points: &[f64]) -> bool {
    let input = accept_sites(dim, points).unwrap();
    let sites = insert::Sites::of(&input);
    flat(&input, &sites).unwrap()
}

/// The triangulation built by insertion; panics on a flat lift.
fn triangulate(dim: usize, points: &[f64]) -> DelaunayTriangulation {
    assert!(!is_flat(dim, points), "unexpected flat lift");
    DelaunayBuilder::new(dim, points).build().unwrap()
}

/// Ascending vertex lists of the simplices, sorted.
fn cells(t: &DelaunayTriangulation) -> Vec<Vec<u32>> {
    let mut cells: Vec<Vec<u32>> = t
        .simplices()
        .iter()
        .map(|s| {
            let mut v = s.vertices().to_vec();
            v.sort_unstable();
            v
        })
        .collect();
    cells.sort();
    cells
}

/// Structural checks from §7: distinct simplices, orientation, and
/// neighbor symmetry across the shared face. The order is the
/// construction's and is not checked.
fn check(t: &DelaunayTriangulation, points: &[f64]) {
    let d = t.dim();
    let point = |i: u32| &points[i as usize * d..(i as usize + 1) * d];
    let mut sorted: Vec<Vec<u32>> = t
        .simplices()
        .iter()
        .map(|s| {
            let mut v = s.vertices().to_vec();
            v.sort_unstable();
            v
        })
        .collect();
    sorted.sort();
    assert!(
        sorted.windows(2).all(|w| w[0] < w[1]),
        "every simplex is published once"
    );
    for (i, s) in t.simplices().iter().enumerate() {
        assert_eq!(s.vertices().len(), d + 1);
        let refs: Vec<&[f64]> = s.vertices().iter().map(|&v| point(v)).collect();
        assert_eq!(orient(&refs).unwrap(), Sign::Positive, "simplex {i}");
        for (slot, &n) in s.neighbors().iter().enumerate() {
            if n == NO_NEIGHBOR {
                continue;
            }
            let other = &t.simplices().get((n as usize) as u32).unwrap();
            let back = other
                .neighbors()
                .iter()
                .position(|&b| b == i as u32)
                .unwrap();
            let face = |v: &[u32], skip: usize| -> Vec<u32> {
                let mut f: Vec<u32> = v
                    .iter()
                    .enumerate()
                    .filter(|&(k, _)| k != skip)
                    .map(|(_, &x)| x)
                    .collect();
                f.sort_unstable();
                f
            };
            assert_eq!(face(s.vertices(), slot), face(other.vertices(), back));
        }
    }
}

#[test]
fn a_triangle_with_an_inner_site() {
    // (0,0), (1,0), (0,1), (0.1,0.1): the inner site joins every edge of
    // the triangle.
    let points = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.1, 0.1];
    let t = triangulate(2, &points);
    assert_eq!(cells(&t), vec![vec![0, 1, 3], vec![0, 2, 3], vec![1, 2, 3]]);
    check(&t, &points);
}

#[test]
fn partly_cocircular_square() {
    // The square's corners are cocircular, and the circle is empty, so the
    // square is one lower facet of four sites, split by placing in index
    // order: 0, 1, 2, then 3 beyond the ridge {0, 2}. Point 4 = (1, -3) lies
    // outside that circle and joins edge {0, 1}.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, -3.0];
    let t = triangulate(2, &points);
    let m = NO_NEIGHBOR;
    // Numbered here in the lexicographic order of the vertex lists; `named`
    // compares them by vertices.
    assert_eq!(
        rows(&t),
        named(vec![
            (vec![0, 1, 2], vec![m, 2, 1]),
            // (0,0), (2,0), (1,-3) is clockwise, so the last two swap.
            (vec![0, 4, 1], vec![m, 0, m]),
            (vec![0, 2, 3], vec![m, m, 0]),
        ])
    );
    check(&t, &points);
}

#[test]
fn every_site_on_one_circle_is_flat() {
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
    assert!(
        is_flat(2, &points),
        "four cocircular sites have a flat lift"
    );
    // One site off the circle makes the lift span three dimensions.
    let points = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, 1.0];
    assert!(!is_flat(2, &points));
}

#[test]
fn degenerate_sites_report_original_indices() {
    // 1 duplicates 0; all on the line y = x.
    let points = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0];
    assert_eq!(
        complex(2, &points).err(),
        Some(ConvexHullError::DegenerateDimension {
            actual_dim: 1,
            spanning_points: vec![0, 2],
        })
    );
}

#[test]
fn duplicates_map_to_their_representative() {
    let points = [0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 4.0, 0.0, 1.0, 1.0];
    let t = triangulate(2, &points);
    assert_eq!(t.representative(), vec![0, 1, 2, 1, 4]);
    assert!(t.simplices().iter().all(|s| !s.vertices().contains(&3)));
}

// Brute-force reference: a (D+1)-subset of sites in general position is a
// Delaunay simplex exactly when its circumsphere has no site strictly
// inside, decided by the lifted determinant over i128.

fn det(m: &[Vec<i128>]) -> i128 {
    if m.len() == 1 {
        return m[0][0];
    }
    (0..m.len())
        .map(|j| {
            let minor: Vec<Vec<i128>> = m[1..]
                .iter()
                .map(|r| {
                    r.iter()
                        .enumerate()
                        .filter(|&(c, _)| c != j)
                        .map(|(_, &x)| x)
                        .collect()
                })
                .collect();
            let t = m[0][j] * det(&minor);
            if j.is_multiple_of(2) {
                t
            } else {
                -t
            }
        })
        .sum()
}

fn plain(points: &[Vec<i64>], s: &[usize]) -> i128 {
    let o = &points[s[0]];
    let rows: Vec<Vec<i128>> = s[1..]
        .iter()
        .map(|&i| {
            points[i]
                .iter()
                .zip(o)
                .map(|(&x, &y)| i128::from(x - y))
                .collect()
        })
        .collect();
    det(&rows)
}

fn lifted(points: &[Vec<i64>], s: &[usize]) -> i128 {
    let norm = |p: &[i64]| {
        p.iter()
            .map(|&x| i128::from(x) * i128::from(x))
            .sum::<i128>()
    };
    let o = &points[s[0]];
    let rows: Vec<Vec<i128>> = s[1..]
        .iter()
        .map(|&i| {
            let p = &points[i];
            p.iter()
                .zip(o)
                .map(|(&x, &y)| i128::from(x - y))
                .chain(core::iter::once(norm(p) - norm(o)))
                .collect()
        })
        .collect();
    det(&rows)
}

fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current = (0..k).collect::<Vec<usize>>();
    loop {
        out.push(current.clone());
        let Some(i) = (0..k).rev().find(|&i| current[i] < n - k + i) else {
            return out;
        };
        current[i] += 1;
        for j in i + 1..k {
            current[j] = current[j - 1] + 1;
        }
    }
}

/// The Delaunay simplices by brute force, or `None` when the sites are not
/// in general position (D + 1 on a hyperplane, or D + 2 on a sphere).
fn brute_force(points: &[Vec<i64>]) -> Option<Vec<Vec<u32>>> {
    let n = points.len();
    let d = points[0].len();
    for s in subsets(n, d + 1) {
        if plain(points, &s) == 0 {
            return None;
        }
    }
    for s in subsets(n, d + 2) {
        if lifted(points, &s) == 0 {
            return None;
        }
    }
    let mut cells = Vec::new();
    for s in subsets(n, d + 1) {
        let orientation = plain(points, &s).signum();
        let empty = (0..n).filter(|q| !s.contains(q)).all(|q| {
            let mut with = s.clone();
            with.push(q);
            // q strictly inside the circumsphere: the lifted orientation
            // has the opposite sign of the simplex's own orientation.
            lifted(points, &with).signum() != -orientation
        });
        if empty {
            cells.push(s.iter().map(|&i| i as u32).collect());
        }
    }
    Some(cells)
}

fn random_sites(dim: usize, count: usize, seed: u64, range: u64) -> Vec<Vec<i64>> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            (0..dim)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state % range) as i64
                })
                .collect()
        })
        .collect()
}

fn general_position_matches(dim: usize, count: usize, trials: u64, range: u64) -> usize {
    let mut checked = 0;
    for seed in 1..=trials {
        let sites = random_sites(dim, count, seed * 7919, range);
        let Some(expected) = brute_force(&sites) else {
            continue;
        };
        let points: Vec<f64> = sites.iter().flatten().map(|&x| x as f64).collect();
        let t = triangulate(dim, &points);
        assert_eq!(cells(&t), expected, "D = {dim}, seed {seed}");
        check(&t, &points);
        // Scaling by 2^600 overflows every square in f64; the exact lift
        // keeps the same triangulation.
        let huge: Vec<f64> = points.iter().map(|x| x * 2f64.powi(600)).collect();
        assert_eq!(
            cells(&triangulate(dim, &huge)),
            expected,
            "D = {dim}, seed {seed}, huge"
        );
        checked += 1;
    }
    checked
}

#[test]
fn general_position_in_two_dimensions() {
    assert!(general_position_matches(2, 12, 40, 1000) >= 20);
}

#[test]
fn general_position_in_three_dimensions() {
    assert!(general_position_matches(3, 9, 20, 1000) >= 10);
}

/// The Delaunay conditions of `t` over the integer `sites`, by the `i128`
/// reference: every simplex is positive and has no site strictly inside its
/// circumsphere, every face without a neighbor has no site strictly beyond
/// it, and every representative is a vertex. Returns the number of
/// boundary faces with a site on their plane (a degenerate hull) and of
/// simplices with a cospherical site (a degenerate interior).
fn check_delaunay(t: &DelaunayTriangulation, sites: &[Vec<i64>]) -> (usize, usize) {
    let (mut flat_boundary, mut cospherical) = (0, 0);
    let mut used = vec![false; sites.len()];
    for s in t.simplices().iter() {
        let cell: Vec<usize> = s.vertices().iter().map(|&v| v as usize).collect();
        let orientation = plain(sites, &cell);
        assert!(orientation > 0, "{cell:?} is positive");
        let mut on_sphere = false;
        for q in 0..sites.len() {
            if t.representative()[q] != q as u32 || cell.contains(&q) {
                continue;
            }
            let mut with = cell.clone();
            with.push(q);
            let l = lifted(sites, &with);
            // q strictly inside: the lifted sign is the opposite of the
            // simplex's own (see `brute_force`).
            assert!(l.signum() != -orientation.signum(), "{q} inside {cell:?}");
            on_sphere |= l == 0;
        }
        cospherical += usize::from(on_sphere);
        for (slot, &n) in s.neighbors().iter().enumerate() {
            if n != NO_NEIGHBOR {
                continue;
            }
            // The face opposite `slot`, with the opposite vertex replaced by
            // each site: positive means beyond the face, away from it.
            let mut on_plane = false;
            for q in 0..sites.len() {
                if t.representative()[q] != q as u32 || cell.contains(&q) {
                    continue;
                }
                let mut with = cell.clone();
                with[slot] = q;
                let side = plain(sites, &with);
                assert!(side >= 0, "{q} beyond the boundary face of {cell:?}");
                on_plane |= side == 0;
            }
            flat_boundary += usize::from(on_plane);
        }
        for &v in &cell {
            used[v] = true;
        }
    }
    for (q, &is_vertex) in used.iter().enumerate() {
        assert!(
            is_vertex || t.representative()[q] != q as u32,
            "site {q} is a vertex"
        );
    }
    (flat_boundary, cospherical)
}

#[test]
fn degenerate_grids_are_delaunay() {
    // Integer sites with many on one line or plane of the hull boundary and
    // many cospherical (#189): a grid of 0..=3 in D = 2 and D = 3, the
    // boundary of a square, and lattice points on and inside a circle. Each
    // case must reach both degeneracies, so the outside-simplex rule for a
    // site on a hull plane and the cospherical merge both run.
    let mut cases: Vec<(usize, Vec<Vec<i64>>)> = Vec::new();
    cases.push((2, random_sites(2, 40, 3, 4)));
    cases.push((3, random_sites(3, 30, 5, 4)));
    let mut square: Vec<Vec<i64>> = Vec::new();
    for i in 0..=6 {
        square.extend([vec![i, 0], vec![6, i], vec![6 - i, 6], vec![0, 6 - i]]);
    }
    square.extend([vec![3, 3], vec![2, 4], vec![4, 1]]);
    cases.push((2, square));
    let mut circle: Vec<Vec<i64>> = [
        (5, 0),
        (4, 3),
        (3, 4),
        (0, 5),
        (-3, 4),
        (-4, 3),
        (-5, 0),
        (-4, -3),
        (-3, -4),
        (0, -5),
        (3, -4),
        (4, -3),
    ]
    .iter()
    .map(|&(x, y)| vec![x, y])
    .collect();
    // (5, 5) and (5, -5) put (5, 0) on a hull edge.
    circle.extend([vec![0, 0], vec![1, 2], vec![-2, 1], vec![5, 5], vec![5, -5]]);
    cases.push((2, circle));
    for (i, (dim, sites)) in cases.iter().enumerate() {
        let points: Vec<f64> = sites.iter().flatten().map(|&x| x as f64).collect();
        let t = triangulate(*dim, &points);
        check(&t, &points);
        let (flat_boundary, cospherical) = check_delaunay(&t, sites);
        assert!(
            flat_boundary > 0 && cospherical > 0,
            "case {i}: boundary sites on a plane {flat_boundary}, cospherical {cospherical}"
        );
    }
}

#[test]
fn cospherical_groups_sharing_a_face_split_it_alike() {
    // The cube's eight corners are cospherical, and so are its bottom four
    // with the apex (1, 1, -5) below. The two lower facets meet in the bottom
    // square; placing in index order splits it the same way in both, so
    // every interior face lies in exactly two tetrahedra. The boundary is the
    // hull surface: top 2, sides 8, pyramid 4 triangles.
    let mut points: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    points.extend([1.0, 1.0, -5.0]);
    let t = triangulate(3, &points);
    check(&t, &points);
    let boundary: usize = t
        .simplices()
        .iter()
        .map(|s| s.neighbors().iter().filter(|&&n| n == NO_NEIGHBOR).count())
        .sum();
    assert_eq!(boundary, 14);
    // Volumes of integer tetrahedra are exact sixths: cube 8, pyramid 20/3.
    let point = |i: u32| &points[i as usize * 3..i as usize * 3 + 3];
    let six_volume: f64 = t
        .simplices()
        .iter()
        .map(|s| {
            let o = point(s.vertices()[0]);
            let r: Vec<Vec<f64>> = s.vertices()[1..]
                .iter()
                .map(|&v| point(v).iter().zip(o).map(|(x, y)| x - y).collect())
                .collect();
            r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
                - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
                + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0])
        })
        .sum();
    assert_eq!(six_volume, 6.0 * 8.0 + 40.0);
}

// Flat lift: the pulling triangulation (P4-2).

fn built(dim: usize, points: &[f64]) -> DelaunayTriangulation {
    let t = DelaunayBuilder::new(dim, points).build().unwrap();
    check(&t, points);
    t
}

#[test]
fn square_is_pulled_from_its_smallest_site() {
    // Facets of the square without site 0: edges {1, 2} and {2, 3}.
    let points = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    let t = built(2, &points);
    let m = NO_NEIGHBOR;
    assert_eq!(
        rows(&t),
        named(vec![
            (vec![0, 1, 2], vec![m, 1, m]),
            (vec![0, 2, 3], vec![m, m, 0]),
        ])
    );
}

#[test]
fn cocircular_pentagon_is_a_fan_from_site_0() {
    // Five sites on the circle of radius 5 with exact rational coordinates
    // (a regular pentagon is not representable exactly). The edges without
    // site 0 are {1, 2}, {2, 3}, {3, 4}.
    let points = [5.0, 0.0, 3.0, 4.0, -3.0, 4.0, -5.0, 0.0, 0.0, -5.0];
    let t = built(2, &points);
    assert_eq!(cells(&t), vec![vec![0, 1, 2], vec![0, 2, 3], vec![0, 3, 4]]);
}

#[test]
fn cube_corners_are_six_tetrahedra_from_corner_0() {
    // The three squares away from corner 0 are each split from their own
    // smallest corner, and 0 joins every triangle: 6 tetrahedra, volume 8,
    // 12 boundary triangles.
    let points: Vec<f64> = (0..8)
        .flat_map(|i: u32| (0..3).map(move |a| f64::from((i >> a) & 1) * 2.0))
        .collect();
    let t = built(3, &points);
    assert_eq!(t.simplices().len(), 6);
    assert!(t.simplices().iter().all(|s| s.vertices().contains(&0)));
    let boundary: usize = t
        .simplices()
        .iter()
        .map(|s| s.neighbors().iter().filter(|&&n| n == NO_NEIGHBOR).count())
        .sum();
    assert_eq!(boundary, 12);
    // Squares x = 2 (corners 1, 3, 5, 7), y = 2 (2, 3, 6, 7), z = 2 (4, 5,
    // 6, 7), each pulled from its smallest corner.
    assert_eq!(
        cells(&t),
        vec![
            vec![0, 1, 3, 7],
            vec![0, 1, 5, 7],
            vec![0, 2, 3, 7],
            vec![0, 2, 6, 7],
            vec![0, 4, 5, 7],
            vec![0, 4, 6, 7],
        ]
    );
}

#[test]
fn d_plus_one_sites_are_one_simplex() {
    let triangle = [0.0, 0.0, 3.0, 0.0, 0.0, 2.0];
    let t = built(2, &triangle);
    assert_eq!(cells(&t), vec![vec![0, 1, 2]]);
    assert_eq!(
        t.simplices().get(0_u32).unwrap().neighbors(),
        vec![NO_NEIGHBOR; 3]
    );
    let tetrahedron = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(cells(&built(3, &tetrahedron)), vec![vec![0, 1, 2, 3]]);
}

#[test]
fn one_dimension() {
    // Two sites: a flat lift. More sites: the lower hull of the parabola,
    // one segment between consecutive sites.
    let two = [3.0, -1.0];
    let t = built(1, &two);
    // Ascending [0, 1] runs from 3 to -1, negative, so the two swap.
    assert_eq!(t.simplices().get(0_u32).unwrap().vertices(), vec![1, 0]);
    let line = [0.0, 3.0, 1.0, 7.0];
    assert_eq!(
        cells(&built(1, &line)),
        vec![vec![0, 2], vec![1, 2], vec![1, 3]]
    );
}

#[test]
fn pulled_simplices_have_empty_circumspheres() {
    // Every pulled simplex of a cospherical set has the common sphere as its
    // circumsphere, with every site on it and none inside: the lifted
    // orientation of each simplex with any other site is zero.
    let points = [
        5.0, 0.0, 3.0, 4.0, -3.0, 4.0, -5.0, 0.0, 0.0, -5.0, 4.0, -3.0,
    ];
    let t = built(2, &points);
    for s in t.simplices().iter() {
        for q in 0..6_u32 {
            let mut p: Vec<&[f64]> = s
                .vertices()
                .iter()
                .map(|&v| &points[v as usize * 2..v as usize * 2 + 2])
                .collect();
            p.push(&points[q as usize * 2..q as usize * 2 + 2]);
            assert_eq!(crate::predicates::orient_lifted(&p).unwrap(), Sign::Zero);
        }
    }
    assert_eq!(t.simplices().len(), 4);
}

/// The pulling triangulation as its rule reads (design §7), with every
/// face intersected with every facet of the hull: the cells in the order
/// the faces leave the stack.
fn pulled_by_every_facet<'p>(
    d: usize,
    sites: &[u32],
    facets: &Lists<u32>,
    point: impl Fn(u32) -> &'p [f64] + Copy,
) -> Vec<Vec<u32>> {
    let mut rows = Vec::new();
    let mut stack: Vec<(Vec<u32>, usize, Vec<u32>)> = vec![(sites.to_vec(), d, Vec::new())];
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
        let mut sub: Vec<Vec<u32>> = Vec::new();
        for facet in facets.iter() {
            let meet: Vec<u32> = face
                .iter()
                .copied()
                .filter(|x| facet.binary_search(x).is_ok())
                .collect();
            if meet.len() < k || meet.contains(&v) || sub.contains(&meet) {
                continue;
            }
            let basis = minimum_basis_with(d, &meet, point, &mut rows).unwrap();
            if basis.len() == k {
                sub.push(meet);
            }
        }
        for facet in sub {
            let mut next = pulled.clone();
            next.push(v);
            stack.push((facet, k - 1, next));
        }
    }
    cells
}

/// [`pulled_cells`] finds the facets of a face from the facets at its
/// vertices (#539). It gives the cells of the rule read with every facet,
/// in the same order, on sets whose faces are not simplices at one level
/// or at several: the corners of a cube of D = 2 to 5, the integer points
/// of spheres in D = 2 to 4 (those of squared radius 4 in D = 4 are the
/// 24-cell, with octahedra for facets), each in shuffled index orders.
///
/// The last set is 28 of the 40 integer points of squared radius 2 in
/// D = 5, from the review of #542. Two facets of its hull meet a face of
/// dimension k in k points that span less than k - 1 dimensions, which the
/// other sets do not have: keeping such a meet changes the cells.
#[test]
fn pulled_cells_are_those_of_the_rule_read_with_every_facet() {
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
    let mut sets: Vec<(usize, Vec<Vec<f64>>)> = Vec::new();
    for d in 2..=5 {
        let corners = (0..1_u32 << d)
            .map(|i| (0..d).map(|a| f64::from((i >> a) & 1) * 2.0).collect())
            .collect();
        sets.push((d, corners));
    }
    for (d, squared) in [
        (2, 25),
        (2, 325),
        (3, 9),
        (3, 26),
        (3, 41),
        (3, 101),
        (4, 4),
        (4, 10),
    ] {
        sets.push((d, sphere(d, squared)));
    }
    #[rustfmt::skip]
    let subset: [[i8; 5]; 28] = [
        [0, -1, -1, 0, 0], [0, 0, 1, 0, -1], [0, 1, 0, -1, 0], [0, -1, 0, -1, 0],
        [-1, 0, 1, 0, 0], [-1, 0, 0, 0, -1], [0, 1, 0, 0, 1], [-1, 0, -1, 0, 0],
        [-1, 0, 0, -1, 0], [0, -1, 0, 0, 1], [-1, -1, 0, 0, 0], [0, 0, -1, 1, 0],
        [0, 0, 1, -1, 0], [0, 0, -1, 0, -1], [1, 0, 0, -1, 0], [1, 0, 0, 0, 1],
        [0, 1, 0, 0, -1], [0, -1, 0, 1, 0], [1, -1, 0, 0, 0], [0, 0, 0, 1, 1],
        [0, 0, -1, 0, 1], [-1, 0, 0, 1, 0], [0, -1, 1, 0, 0], [1, 0, -1, 0, 0],
        [-1, 0, 0, 0, 1], [0, 0, 1, 1, 0], [0, 0, 0, 1, -1], [0, 1, -1, 0, 0],
    ];
    let subset = subset
        .iter()
        .map(|row| row.iter().map(|&c| f64::from(c)).collect())
        .collect();
    sets.push((5, subset));
    let mut state = 539_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let (mut not_simplices, mut deep) = (0, 0);
    for (d, mut rows) in sets {
        for round in 0..3 {
            if round > 0 {
                for i in (1..rows.len()).rev() {
                    rows.swap(i, next() as usize % (i + 1));
                }
            }
            let flat: Vec<f64> = rows.iter().flatten().copied().collect();
            let hull = SimplicialHull::build(accept_sites(d, &flat).unwrap()).unwrap();
            let facets = merge(&hull).unwrap().vertices;
            let input = &hull.input;
            let point = |i: u32| input.point(i);
            let sites = &input.representatives;
            assert_eq!(sites.len(), rows.len(), "every site is its own");
            let expected = pulled_by_every_facet(d, sites, &facets, point);
            let got = pulled_cells(d, sites, &facets, point).unwrap();
            let got: Vec<Vec<u32>> = got.chunks_exact(d + 1).map(<[u32]>::to_vec).collect();
            assert_eq!(
                got,
                expected,
                "D = {d}, {} sites, round {round}",
                rows.len()
            );
            not_simplices += facets.iter().filter(|f| f.len() > d).count();
            deep += usize::from(d >= 4 && facets.iter().any(|f| f.len() > d));
        }
    }
    assert!(not_simplices > 100, "facets that are not simplices");
    assert!(deep >= 9, "faces split at two levels or more");
}

/// Every permutation of `k` distinct vertices, for k = 2 to 6: the sorted
/// vertices ascend, each keeps its slot, and the parity is that of the
/// permutation's inversions, counted pairwise here. Three and four vertices
/// go through `ascending_fixed`, which the cells of D = 2 and 3 take.
#[test]
fn ascending_sorts_and_gives_the_parity_of_every_permutation() {
    fn permutations(k: usize) -> Vec<Vec<u32>> {
        if k == 0 {
            return vec![Vec::new()];
        }
        let mut out = Vec::new();
        for rest in permutations(k - 1) {
            for at in 0..=rest.len() {
                let mut p = rest.clone();
                p.insert(at, 10 * (k as u32 - 1) + 7);
                out.push(p);
            }
        }
        out
    }
    let check = |row: &[u32]| {
        let k = row.len();
        let (sorted, negative) = ascending(row);
        assert_eq!(sorted.len(), k, "{row:?}");
        assert!(sorted.windows(2).all(|w| w[0].0 < w[1].0), "{row:?}");
        assert!(sorted.iter().all(|&(v, slot)| row[slot] == v), "{row:?}");
        let inversions = (0..k)
            .flat_map(|i| (i + 1..k).map(move |j| (i, j)))
            .filter(|&(i, j)| row[i] > row[j])
            .count();
        assert_eq!(negative, inversions % 2 == 1, "{row:?}");
    };
    for k in 2..=6 {
        let rows = permutations(k);
        assert_eq!(rows.len(), (1..=k).product::<usize>());
        for row in rows {
            check(&row);
        }
    }
    // Lengths at and above the inline capacity of 11 (D = 10 to 12): the
    // reversal, rotations, and a fixed shuffle of each.
    for k in 11..=13_u32 {
        let ascending_row: Vec<u32> = (0..k).map(|v| 7 * v + 3).collect();
        let mut rows = vec![ascending_row.iter().rev().copied().collect::<Vec<u32>>()];
        for r in 0..k as usize {
            let mut row = ascending_row.clone();
            row.rotate_left(r);
            rows.push(row);
        }
        let mut shuffled = ascending_row.clone();
        for i in (1..shuffled.len()).rev() {
            shuffled.swap(i, (i * 5 + 3) % (i + 1));
        }
        rows.push(shuffled);
        for row in rows {
            check(&row);
        }
    }
}

/// A build in D = 11 and 12, where a simplex has more vertices than the
/// inline capacity of the published rows' sort (#365 review).
#[test]
fn dimensions_above_the_inline_capacity_build() {
    for dim in [11, 12] {
        let mut state = 0x2545_f491_4f6c_dd1d_u64 ^ dim as u64;
        let points: Vec<f64> = (0..(dim + 4) * dim)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state % 1000) as f64
            })
            .collect();
        let t = DelaunayBuilder::new(dim, &points).build().unwrap();
        assert!(!t.simplices().is_empty(), "D = {dim}");
    }
}

/// The order the sites are stored in for the insertion changes nothing
/// published: the same simplices in the same order, with the same
/// neighbors (#366). The walk is seeded by the input index, so it takes the
/// same steps either way. General position, integer grids with cospherical
/// groups, a lattice with duplicates, and sites near one sphere, D = 2 to 4.
#[test]
fn the_storage_order_of_the_sites_changes_nothing_published() {
    let mut state = 366_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for dim in 2..=4_usize {
        for family in ["general", "grid", "lattice", "near sphere"] {
            let count = if dim == 4 { 120 } else { 400 };
            let mut points = Vec::with_capacity(dim * count);
            for _ in 0..count {
                let mut v: Vec<f64> = (0..dim)
                    .map(|_| 2.0 * ((next() >> 11) as f64 / (1_u64 << 53) as f64) - 1.0)
                    .collect();
                match family {
                    "grid" => v.iter_mut().for_each(|x| *x = (next() % 8) as f64),
                    "lattice" => v.iter_mut().for_each(|x| *x = (next() % 5) as f64 - 2.0),
                    "near sphere" => {
                        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
                        v.iter_mut().for_each(|x| *x /= norm);
                    }
                    _ => {}
                }
                points.extend(v);
            }
            let inserted = DelaunayBuilder::new(dim, &points).build().unwrap();
            STORED_IN_INPUT_ORDER.with(|c| c.set(true));
            let stored = DelaunayBuilder::new(dim, &points).build();
            STORED_IN_INPUT_ORDER.with(|c| c.set(false));
            let stored = stored.unwrap();
            let rows = |t: &DelaunayTriangulation| -> Vec<(Vec<u32>, Vec<u32>)> {
                t.simplices()
                    .iter()
                    .map(|s| (s.vertices().to_vec(), s.neighbors().to_vec()))
                    .collect()
            };
            assert!(!rows(&inserted).is_empty(), "D = {dim}, {family}");
            assert_eq!(rows(&inserted), rows(&stored), "D = {dim}, {family}");
            assert_eq!(inserted.representative(), stored.representative());
        }
    }
}
