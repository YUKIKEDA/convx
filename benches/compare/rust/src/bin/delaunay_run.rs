//! The `delaunay` crate in the comparison: the Delaunay triangulation of
//! D = 2 to 5, with its default options. Its predicates are exact.
//!
//! The default is the PL-manifold guarantee, which validates as it inserts.
//! The lightest options it offers (the pseudomanifold guarantee with no
//! automatic validation) were tried so that the time would be the
//! construction's alone: with them 0.8.2 returned an error, a Delaunay
//! violation after its seven retries, on 1,000 points of `rbox` in D = 2
//! and D = 3 and on the larger files tried. So the default is what is run.

use convx_compare::Job;
use delaunay::prelude::construction::{DelaunayTriangulation, DelaunayTriangulationBuilder};
use delaunay::prelude::geometry::{AdaptiveKernel, ExactPredicates};
use delaunay::prelude::tds::Vertex;

fn run<const D: usize>(job: &Job)
where
    AdaptiveKernel<f64>: ExactPredicates<D>,
{
    job.run(
        |job| {
            let vertices: Vec<Vertex<(), D>> = job
                .points
                .chunks_exact(D)
                .map(|p| {
                    let coords: [f64; D] = p.try_into().expect("a point of D coordinates");
                    Vertex::try_new(coords).expect("a finite point")
                })
                .collect();
            let triangulation: DelaunayTriangulation<AdaptiveKernel<f64>, (), (), D> =
                DelaunayTriangulationBuilder::new(&vertices)
                    .build()
                    .expect("a triangulation");
            triangulation
        },
        |triangulation| {
            (
                triangulation.number_of_vertices(),
                triangulation.number_of_simplices(),
            )
        },
    );
}

fn main() {
    let job = Job::from_args();
    assert!(job.operation == "delaunay", "the delaunay crate: Delaunay");
    match job.dim {
        2 => run::<2>(&job),
        3 => run::<3>(&job),
        4 => run::<4>(&job),
        5 => run::<5>(&job),
        d => panic!("the delaunay crate is run for D = 2 to 5, not {d}"),
    }
}
