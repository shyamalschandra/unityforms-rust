use crate::Rectangle;

/// Stable identity for a component in the live object graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(u64);

impl ComponentId {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Non-visual or visual object participating in the component model.
pub trait Component {
    fn id(&self) -> ComponentId;

    fn name(&self) -> Option<&str> {
        None
    }
}

/// Interactive surface that occupies bounds and participates in layout.
pub trait Control: Component {
    fn bounds(&self) -> Rectangle;

    fn set_bounds(&mut self, value: Rectangle);
}
