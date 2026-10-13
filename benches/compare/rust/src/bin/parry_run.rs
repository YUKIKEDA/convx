//! `parry3d-f64` in the comparison: the convex hull of D = 3, as vertices
//! and triangles. Its predicates are not exact. A facet of more than three
//! points comes out as several triangles, so its second count is the
//! triangles of the surface, not convx's facets.

use convx_compare::Job;
use parry3d_f64::math::Vector3;
use parry3d_f64::transformation::convex_hull;

fn main() {
    let job = Job::from_args();
    assert!(
        job.operation == "hull" && job.dim == 3,
        "parry3d: the hull of D = 3"
    );
    job.run(
        |job| {
            let points: Vec<Vector3> = job
                .points
                .chunks_exact(3)
                .map(|p| Vector3::new(p[0], p[1], p[2]))
                .collect();
            convex_hull(&points)
        },
        |(vertices, triangles)| (vertices.len(), triangles.len()),
    );
}
