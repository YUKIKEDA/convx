//! What every adapter of the comparison shares: reading a point file and
//! printing a build.
//!
//! An adapter is a program `<name> <hull|delaunay> <file> <reps>`. It reads
//! the file (a dimension, a count, and the points), builds `reps` times, and
//! prints one line per build: `build <seconds> <a> <b>`. The seconds are
//! those of the construction of the output alone; reading the file and
//! counting the output are outside them. For a hull `a` and `b` are the
//! vertices and the facets, and for a triangulation the distinct sites and
//! the simplices.

use std::time::Instant;

/// The arguments of an adapter and the points of its file, row-major.
pub struct Job {
    pub operation: String,
    pub dim: usize,
    pub points: Vec<f64>,
    pub reps: usize,
}

impl Job {
    /// Reads the command line and the file.
    ///
    /// # Panics
    ///
    /// On a missing argument or a file that is not a point file.
    #[must_use]
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        assert!(
            args.len() == 4,
            "usage: {} <hull|delaunay> <file> <reps>",
            args[0]
        );
        let text = std::fs::read_to_string(&args[2]).expect("the point file");
        let mut words = text.split_ascii_whitespace();
        let dim: usize = words
            .next()
            .expect("a dimension")
            .parse()
            .expect("a dimension");
        let count: usize = words.next().expect("a count").parse().expect("a count");
        let points: Vec<f64> = words.map(|w| w.parse().expect("a coordinate")).collect();
        assert_eq!(
            points.len(),
            dim * count,
            "the file holds its count of points"
        );
        Self {
            operation: args[1].clone(),
            dim,
            points,
            reps: args[3].parse().expect("a number of builds"),
        }
    }

    /// Runs `build` `reps` times and prints each: the seconds of `build`
    /// alone, and the two counts `counts` reads from what it returned,
    /// after the clock has stopped.
    pub fn run<T>(&self, mut build: impl FnMut(&Self) -> T, counts: impl Fn(&T) -> (usize, usize)) {
        for _ in 0..self.reps {
            let start = Instant::now();
            let built = build(self);
            let seconds = start.elapsed().as_secs_f64();
            let (a, b) = counts(&built);
            println!("build {seconds:.6} {a} {b}");
        }
    }
}
