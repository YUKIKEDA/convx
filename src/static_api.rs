//! The static API (design §9): stack arrays `&[[f64; D]]`, passed to the
//! dynamic core with `as_flattened()`.
//!
//! `build` exists only for the dimensions §9 names, emitted by a macro since
//! a single `impl` cannot carry a constant bound on stable Rust. The static
//! types run the sequential build; parallelism belongs to the dynamic
//! builders.
//!
//! A dimension outside the range has no `build`:
//!
//! ```compile_fail
//! use convx::StaticConvexHull;
//!
//! let _ = StaticConvexHull::<9>::build(&[[0.0; 9]; 10]);
//! ```
//!
//! ```compile_fail
//! use convx::StaticDelaunay;
//!
//! let _ = StaticDelaunay::<8>::build(&[[0.0; 8]; 9]);
//! ```
//!
//! ```compile_fail
//! use convx::StaticVoronoi;
//!
//! let _ = StaticVoronoi::<8>::build(&[[0.0; 8]; 9]);
//! ```
//!
//! Nor does `D = 0`:
//!
//! ```compile_fail
//! use convx::StaticConvexHull;
//!
//! let _ = StaticConvexHull::<0>::build(&[[0.0; 0]; 1]);
//! ```
//!
//! ```compile_fail
//! use convx::StaticDelaunay;
//!
//! let _ = StaticDelaunay::<0>::build(&[[0.0; 0]; 1]);
//! ```
//!
//! ```compile_fail
//! use convx::StaticVoronoi;
//!
//! let _ = StaticVoronoi::<0>::build(&[[0.0; 0]; 1]);
//! ```

use crate::{
    ConvexHull, ConvexHullBuilder, ConvexHullError, DelaunayBuilder, DelaunayTriangulation,
    VoronoiBuilder, VoronoiDiagram,
};

/// The convex hull of stack arrays, for `1 <= D <= 8`.
///
/// ```
/// use convx::StaticConvexHull;
///
/// let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.5, 0.5]];
/// let hull = StaticConvexHull::<2>::build(&square)?;
/// assert_eq!(hull.vertices, vec![0, 1, 2, 3]);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StaticConvexHull<const D: usize>;

/// The Delaunay triangulation of stack arrays, for `1 <= D <= 7`.
///
/// ```
/// use convx::StaticDelaunay;
///
/// let triangle = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
/// let delaunay = StaticDelaunay::<2>::build(&triangle)?;
/// assert_eq!(delaunay.simplices.len(), 1);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StaticDelaunay<const D: usize>;

/// The Voronoi diagram of stack arrays, for `1 <= D <= 7`.
///
/// ```
/// use convx::StaticVoronoi;
///
/// let segment = [[0.0], [2.0]];
/// let voronoi = StaticVoronoi::<1>::build(&segment)?;
/// assert_eq!(voronoi.vertices.len(), 1);
/// # Ok::<(), convx::ConvexHullError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StaticVoronoi<const D: usize>;

macro_rules! static_build {
    ($ty:ident, $builder:ident, $out:ty, $what:literal, [$($d:literal),*]) => {
        $(
            impl $ty<$d> {
                #[doc = concat!("Builds the ", $what, " of `points` in dimension ", stringify!($d), ".")]
                ///
                /// # Errors
                ///
                /// The same as the dynamic builder's `build`.
                pub fn build(points: &[[f64; $d]]) -> Result<$out, ConvexHullError> {
                    $builder::new($d, points.as_flattened()).build()
                }
            }
        )*
    };
}

static_build!(
    StaticConvexHull,
    ConvexHullBuilder,
    ConvexHull,
    "convex hull",
    [1, 2, 3, 4, 5, 6, 7, 8]
);
static_build!(
    StaticDelaunay,
    DelaunayBuilder,
    DelaunayTriangulation,
    "Delaunay triangulation",
    [1, 2, 3, 4, 5, 6, 7]
);
static_build!(
    StaticVoronoi,
    VoronoiBuilder,
    VoronoiDiagram,
    "Voronoi diagram",
    [1, 2, 3, 4, 5, 6, 7]
);

#[cfg(test)]
mod tests {
    use super::*;

    /// `count` points of dimension `D` with small integer coordinates.
    fn sample<const D: usize>(count: usize, seed: u64) -> Vec<[f64; D]> {
        let mut state = seed;
        (0..count)
            .map(|_| {
                core::array::from_fn(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state % 1000) as f64
                })
            })
            .collect()
    }

    macro_rules! agree {
        ($name:ident, $d:literal, hull) => {
            #[test]
            fn $name() {
                let points = sample::<$d>($d + 4, 0x9e37_79b9 + $d);
                assert_eq!(
                    StaticConvexHull::<$d>::build(&points),
                    ConvexHullBuilder::new($d, points.as_flattened()).build()
                );
            }
        };
        ($name:ident, $d:literal, all) => {
            #[test]
            fn $name() {
                let points = sample::<$d>($d + 4, 0x9e37_79b9 + $d);
                let flat = points.as_flattened();
                assert_eq!(
                    StaticConvexHull::<$d>::build(&points),
                    ConvexHullBuilder::new($d, flat).build()
                );
                assert_eq!(
                    StaticDelaunay::<$d>::build(&points),
                    DelaunayBuilder::new($d, flat).build()
                );
                assert_eq!(
                    StaticVoronoi::<$d>::build(&points),
                    VoronoiBuilder::new($d, flat).build()
                );
            }
        };
    }

    agree!(d1, 1, all);
    agree!(d2, 2, all);
    agree!(d3, 3, all);
    agree!(d4, 4, all);
    agree!(d5, 5, all);
    agree!(d6, 6, all);
    agree!(d7, 7, all);
    agree!(d8, 8, hull);
}
