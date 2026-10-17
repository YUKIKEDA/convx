"""Writes stage marks for the Delaunay build into a copy of the crate (never into src).

usage: marks_d2.py <copy root>

Each mark adds the time since the previous mark to its stage, in a thread-local
table; `convx::stages_take()` returns and clears the table. The stages:
0 scale, 1 accept, 2 sites, 3 flat, 4 order, 5 copy, 6 insert, 7 group, 8 publish.
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
    static STAGES: std::cell::RefCell<([f64; 9], Option<std::time::Instant>)> =
        const { std::cell::RefCell::new(([0.0; 9], None)) };
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
pub fn stages_take() -> [f64; 9] {
    STAGES.with(|s| std::mem::replace(&mut s.borrow_mut().0, [0.0; 9]))
}
''')

edit('src/delaunay/mod.rs', '''        let scaled = crate::scaling::scaled_copy(self.points, crate::scaling::SITES_FROM);
        Ok(publish(complex(
            self.dim,
            scaled.as_deref().unwrap_or(self.points),
        )?))
''', '''        crate::stage_start();
        let scaled = crate::scaling::scaled_copy(self.points, crate::scaling::SITES_FROM);
        crate::stage(0);
        let complex = complex(self.dim, scaled.as_deref().unwrap_or(self.points))?;
        let out = publish(complex);
        crate::stage(8);
        Ok(out)
''')
edit('src/delaunay/mod.rs', '''    let input = accept_sites(dim, points)?;
    let sites = insert::Sites::of(&input);
    let complex = if flat(&input, &sites)? {
''', '''    let input = accept_sites(dim, points)?;
    crate::stage(1);
    let sites = insert::Sites::of(&input);
    crate::stage(2);
    let is_flat = flat(&input, &sites)?;
    crate::stage(3);
    let complex = if is_flat {
''')
edit('src/delaunay/mod.rs', '''    let order = insert::brio(sites, &rest);
''', '''    let order = insert::brio(sites, &rest);
    crate::stage(4);
''')
edit('src/delaunay/mod.rs', '''    let mesh = insert::Mesh::build(shape, &local, &local_first, &local_order)?;
''', '''    crate::stage(5);
    let mesh = insert::Mesh::build(shape, &local, &local_first, &local_order)?;
    crate::stage(6);
''')
edit('src/delaunay/mod.rs', '''    drop(mesh);
    Ok(draft.finish(d, input.representative.clone()))
''', '''    drop(mesh);
    let finished = draft.finish(d, input.representative.clone());
    crate::stage(7);
    Ok(finished)
''')
