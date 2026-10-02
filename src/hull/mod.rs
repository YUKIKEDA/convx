//! The convex hull: input acceptance, construction, and (later) the
//! published surface.

// The public builder (P2-4, #11) is the first caller outside tests.
#![cfg_attr(not(test), allow(dead_code))]

mod error;
pub(crate) mod input;
pub(crate) mod simplicial;

pub use error::ConvexHullError;
