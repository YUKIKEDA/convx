//! `geo` in the comparison: the convex hull of D = 2, as a polygon. Its
//! orientation tests are robust (the `robust` crate). The vertices and the
//! edges of a polygon are as many, so both counts are its vertices.

use convx_compare::Job;
use geo::{ConvexHull, MultiPoint, Point};

fn main() {
    let job = Job::from_args();
    assert!(
        job.operation == "hull" && job.dim == 2,
        "geo: the hull of D = 2"
    );
    job.run(
        |job| {
            let points: MultiPoint<f64> = job
                .points
                .chunks_exact(2)
                .map(|p| Point::new(p[0], p[1]))
                .collect::<Vec<_>>()
                .into();
            points.convex_hull()
        },
        |polygon| {
            // A closed ring repeats its first point.
            let vertices = polygon.exterior().0.len().saturating_sub(1);
            (vertices, vertices)
        },
    );
}
