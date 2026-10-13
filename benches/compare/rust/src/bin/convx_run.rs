//! convx in the comparison: the hull and the Delaunay triangulation as
//! `build()` returns them.

use convx::{ConvexHullBuilder, DelaunayBuilder};
use convx_compare::Job;

fn main() {
    let job = Job::from_args();
    match job.operation.as_str() {
        "hull" => job.run(
            |job| {
                ConvexHullBuilder::new(job.dim, &job.points)
                    .build()
                    .expect("a hull")
            },
            |hull| (hull.vertices().len(), hull.facets().len()),
        ),
        "delaunay" => job.run(
            |job| {
                DelaunayBuilder::new(job.dim, &job.points)
                    .build()
                    .expect("a triangulation")
            },
            |triangulation| {
                let sites = triangulation
                    .representative()
                    .iter()
                    .enumerate()
                    .filter(|&(i, &r)| r as usize == i)
                    .count();
                (sites, triangulation.simplices().len())
            },
        ),
        other => panic!("unknown operation {other}"),
    }
}
