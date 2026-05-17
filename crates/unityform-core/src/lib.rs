//! Core geometry, component identity, and the abstract control graph.
//!
//! This crate is intentionally free of Win32 or other platform imports so it
//! can be unit-tested anywhere and reused by alternate backends.

mod accessibility;
mod component;
mod geometry;
mod layout;
mod theme;

pub use accessibility::{AccessibleAnnouncement, AccessibleRole};
pub use component::{Component, ComponentId, Control};
pub use geometry::{Point, Rectangle, Size};
pub use layout::{inset_rect, dock_top_stack, Anchors, Dock, Margin};
pub use theme::ThemePalette;
