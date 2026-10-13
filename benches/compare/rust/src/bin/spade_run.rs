//! `spade` in the comparison: the Delaunay triangulation of D = 2, loaded
//! in bulk. Its predicates are exact. A triangulation of spade holds
//! vertices, edges, and faces with their adjacency.

use convx_compare::Job;
use spade::{DelaunayTriangulation, Point2, Triangulation};

fn main() {
    let job = Job::from_args();
    assert!(
        job.operation == "delaunay" && job.dim == 2,
        "spade: Delaunay of D = 2"
    );
    job.run(
        |job| {
            let points: Vec<Point2<f64>> = job
                .points
                .chunks_exact(2)
                .map(|p| Point2::new(p[0], p[1]))
                .collect();
            DelaunayTriangulation::<Point2<f64>>::bulk_load(points).expect("a triangulation")
        },
        |triangulation| {
            (
                triangulation.num_vertices(),
                triangulation.num_inner_faces(),
            )
        },
    );
}
