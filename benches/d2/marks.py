"""Writes stage marks for the Delaunay build into a copy of the crate (never into src).

usage: marks_d2.py <copy root>

Each mark adds the time since the previous mark to its stage, in a thread-local
table; `convx::stages_take()` returns and clears the table. The stages:
0 scale, 1 accept, 2 flat, 3 order, 4 sites (the rows in insertion order),
5 insert (either insertion: the mesh, or the half-edges of D = 2), 6 group (the
groups merged after the insertion), 7 publish (the cells written, from the
triangles directly or through the draft, and published).
"""
import sys

root = sys.argv[1]


def edit(p, old, new):
    s = open(root + '/' + p, encoding='utf-8').read()
    s = s.replace('\r\n', '\n')
    assert s.count(old) == 1, (p, old[:60], s.count(old))
    open(root + '/' + p, 'w', encoding='utf-8', newline='\n').write(s.replace(old, new))


edit('src/lib.rs', '#![deny(missing_docs)]\n', '''#![deny(missing_docs)]

thread_local! {
    static STAGES: std::cell::RefCell<([f64; 8], Option<std::time::Instant>)> =
        const { std::cell::RefCell::new(([0.0; 8], None)) };
}

/// Starts the clock of the stage marks.
pub(crate) fn stage_start() {
    STAGES.with(|s| s.borrow_mut().1 = Some(std::time::Instant::now()));
}

/// Adds the time since the previous mark to stage `i`.
pub(crate) fn stage(i: usize) {
    STAGES.with(|s| {
        let mut s = s.borrow_mut();
        let now = std::time::Instant::now();
        if let Some(t) = s.1 {
            s.0[i] += (now - t).as_secs_f64();
        }
        s.1 = Some(now);
    });
}

/// The seconds of each stage since the last call, and clears them.
pub fn stages_take() -> [f64; 8] {
    STAGES.with(|s| std::mem::replace(&mut s.borrow_mut().0, [0.0; 8]))
}
''')

edit('src/delaunay/mod.rs', '''        let scaled = crate::scaling::scaled_copy(self.points, crate::scaling::SITES_FROM);
        triangulation(self.dim, scaled.as_deref().unwrap_or(self.points))
''', '''        crate::stage_start();
        let scaled = crate::scaling::scaled_copy(self.points, crate::scaling::SITES_FROM);
        crate::stage(0);
        let out = triangulation(self.dim, scaled.as_deref().unwrap_or(self.points));
        crate::stage(7);
        out
''')
edit('src/delaunay/mod.rs', '''    let input = accept_sites(dim, points)?;
    if flat(&input)? {
''', '''    let input = accept_sites(dim, points)?;
    crate::stage(1);
    let is_flat = flat(&input)?;
    crate::stage(2);
    if is_flat {
''')
edit('src/delaunay/mod.rs', '''    let inserted_order = insert::brio(input, first, &input.representatives);
''', '''    let inserted_order = insert::brio(input, first, &input.representatives);
    crate::stage(3);
''')
edit('src/delaunay/mod.rs', '''    let local = insert::Sites::of(input, inserted_order);
''', '''    let local = insert::Sites::of(input, inserted_order);
    crate::stage(4);
''')
edit('src/delaunay/mod.rs', '''        let mesh = plane::PlaneMesh::build(local, first, order)?;
''', '''        let mesh = plane::PlaneMesh::build(local, first, order)?;
        crate::stage(5);
''')
edit('src/delaunay/mod.rs', '''    let mesh = insert::Mesh::build(shape, local, first, order)?;
''', '''    let mesh = insert::Mesh::build(shape, local, first, order)?;
    crate::stage(5);
''')
edit('src/delaunay/mod.rs', '''    // D = 2 by flips, no group merged, and only the triangulation wanted:
''', '''    crate::stage(6);
    // D = 2 by flips, no group merged, and only the triangulation wanted:
''')
