//! Core geometry, component identity, and the abstract control graph.
//!
//! This crate is intentionally free of Win32 or other platform imports so it
//! can be unit-tested anywhere and reused by alternate backends.

mod component;
mod geometry;

pub use component::{Component, ComponentId, Control};
pub use geometry::{Point, Rectangle, Size};
