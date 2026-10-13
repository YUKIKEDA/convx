//! `delaunator` in the comparison: the Delaunay triangulation of D = 2 as
//! triangles and half-edges over the input indices. Its orientation tests
//! are robust (the `robust` crate); it does not merge equal points, so the
//! first count is the number of points given.

use convx_compare::Job;
use delaunator::{triangulate, Point};

fn main() {
    let job = Job::from_args();
    assert!(
        job.operation == "delaunay" && job.dim == 2,
        "delaunator: Delaunay of D = 2"
    );
    job.run(
        |job| {
            let points: Vec<Point> = job
                .points
                .chunks_exact(2)
                .map(|p| Point { x: p[0], y: p[1] })
                .collect();
            let triangulation = triangulate(&points);
            (points.len(), triangulation)
        },
        |(points, triangulation)| (*points, triangulation.triangles.len() / 3),
    );
}
