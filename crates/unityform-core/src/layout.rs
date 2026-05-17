//! Layout primitives shared by every backend (Win32 pixel alignment, portable hit testing, etc.).

use crate::geometry::Rectangle;

/// Symmetric insets from a container’s edges (WinForms “padding” / margin on a child).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Margin {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Margin {
    pub const fn uniform(px: i32) -> Self {
        Self {
            left: px,
            top: px,
            right: px,
            bottom: px,
        }
    }

    pub const fn from_ltrb(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// WinForms-style anchor flags (future use with constraint solvers).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Anchors {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

impl Anchors {
    pub const TOP_LEFT: Self = Self {
        left: true,
        top: true,
        right: false,
        bottom: false,
    };
}

/// Docking mode (WinForms `Dock`); backends apply this when computing `SetWindowPos` / draw rects.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Dock {
    None,
    Fill,
    Top,
    Bottom,
    Left,
    Right,
}

/// Shrink `rect` by `m` on each side; negative or excessive margins clamp to zero area.
#[inline]
pub fn inset_rect(rect: Rectangle, m: Margin) -> Rectangle {
    Rectangle::from_xywh(
        rect.x + m.left,
        rect.y + m.top,
        (rect.width - m.left - m.right).max(0),
        (rect.height - m.top - m.bottom).max(0),
    )
}

/// Stack fixed-height horizontal bands from the **top** of `client`, returning each band’s
/// rectangle plus the **remaining** client area (typically used for a multiline editor).
pub fn dock_top_stack(client: Rectangle, band_heights: &[i32]) -> (Vec<Rectangle>, Rectangle) {
    let mut out = Vec::with_capacity(band_heights.len());
    let mut rest = client;
    for &h_raw in band_heights {
        let h = h_raw.max(0).min(rest.height);
        out.push(Rectangle::from_xywh(rest.x, rest.y, rest.width, h));
        rest.y += h;
        rest.height = rest.height.saturating_sub(h);
    }
    (out, rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_splits_remainder() {
        let client = Rectangle::from_xywh(0, 0, 200, 100);
        let (bands, rest) = dock_top_stack(client, &[10, 20]);
        assert_eq!(bands.len(), 2);
        assert_eq!(bands[0], Rectangle::from_xywh(0, 0, 200, 10));
        assert_eq!(bands[1], Rectangle::from_xywh(0, 10, 200, 20));
        assert_eq!(rest, Rectangle::from_xywh(0, 30, 200, 70));
    }
}
