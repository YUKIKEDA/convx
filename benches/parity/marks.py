import sys

root = sys.argv[1]


def edit(p, old, new):
    s = open(root + '/' + p, encoding='utf-8').read()
    assert s.count(old) == 1, (p, old[:50], s.count(old))
    open(root + '/' + p, 'w', encoding='utf-8').write(s.replace(old, new))


edit('src/lib.rs', '#![forbid(unsafe_code)]\n', '''#![forbid(unsafe_code)]

/// Scratch phase timer: prints the seconds since the first mark.
pub(crate) fn mark(name: &str) {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = *START.get_or_init(std::time::Instant::now);
    eprintln!("mark {name} {:.6}", start.elapsed().as_secs_f64());
}
''')
edit('src/hull/classify.rs', '''    let built = SimplicialHull::build(input)?;
    classify_built(built, &mut FoundFaces::default())
''', '''    crate::mark("construct_start");
    let built = SimplicialHull::build(input)?;
    crate::mark("construct_end");
    classify_built(built, &mut FoundFaces::default())
''')
edit('src/hull/publish.rs', '''        let mut hull = publish(classify(input)?);
''', '''        let classified = classify(input)?;
        crate::mark("publish_start");
        let mut hull = publish(classified);
        crate::mark("publish_end");
''')
edit('src/delaunay/mod.rs', '''    let order = insert::brio(sites, &rest);
''', '''    crate::mark("construct_start");
    let order = insert::brio(sites, &rest);
''')
edit('src/delaunay/mod.rs', '''    let mesh = insert::Mesh::build(shape, &local, &local_first, &local_order)?;
''', '''    let mesh = insert::Mesh::build(shape, &local, &local_first, &local_order)?;
    crate::mark("construct_end");
''')
edit('src/delaunay/mod.rs', '''        Ok(publish(complex(
            self.dim,
            scaled.as_deref().unwrap_or(self.points),
        )?))
''', '''        let complex = complex(self.dim, scaled.as_deref().unwrap_or(self.points))?;
        crate::mark("publish_start");
        let out = publish(complex);
        crate::mark("publish_end");
        Ok(out)
''')
