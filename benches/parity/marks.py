import sys

root = sys.argv[1]


def edit(p, old, new):
    s = open(root + '/' + p, encoding='utf-8').read()
    assert s.count(old) == 1, (p, old[:50], s.count(old))
    open(root + '/' + p, 'w', encoding='utf-8').write(s.replace(old, new))


# After the last inner attribute: an item before one is not allowed.
edit('src/lib.rs', '#![deny(missing_docs)]\n', '''#![deny(missing_docs)]

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
edit('src/delaunay/mod.rs', '''    let inserted_order = insert::brio(input, first, &input.representatives);
''', '''    crate::mark("construct_start");
    let inserted_order = insert::brio(input, first, &input.representatives);
''')
# Either insertion ends the construction: the half-edges of D = 2, or the mesh.
edit('src/delaunay/mod.rs', '''        let mesh = plane::PlaneMesh::build(local, first, order)?;
''', '''        let mesh = plane::PlaneMesh::build(local, first, order)?;
        crate::mark("construct_end");
''')
edit('src/delaunay/mod.rs', '''    let mesh = insert::Mesh::build(shape, local, first, order)?;
''', '''    let mesh = insert::Mesh::build(shape, local, first, order)?;
    crate::mark("construct_end");
''')
# Publication starts where the cells are written, from the half-edges directly
# or through the draft (#672), and ends when the triangulation is returned.
edit('src/delaunay/mod.rs', '''    // D = 2 by flips, no group merged, and only the triangulation wanted:
''', '''    crate::mark("publish_start");
    // D = 2 by flips, no group merged, and only the triangulation wanted:
''')
edit('src/delaunay/mod.rs', '''        triangulation(self.dim, scaled.as_deref().unwrap_or(self.points))
''', '''        let out = triangulation(self.dim, scaled.as_deref().unwrap_or(self.points));
        crate::mark("publish_end");
        out
''')
