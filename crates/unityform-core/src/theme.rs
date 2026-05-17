//! Logical palette for backends that rasterize chrome in software (`softbuffer`) or emulate Win32 brushes.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ThemePalette {
    /// Main window background (COLOR_WINDOW analogue).
    pub window_bg_rgb: [u8; 3],
    pub control_face_rgb: [u8; 3],
    pub control_edge_light_rgb: [u8; 3],
    pub control_edge_dark_rgb: [u8; 3],
    pub editor_bg_rgb: [u8; 3],
    pub editor_outline_rgb: [u8; 3],
    pub text_primary_rgb: [u8; 3],
}

impl ThemePalette {
    pub const LIGHT: Self = Self {
        window_bg_rgb: [252, 252, 252],
        control_face_rgb: [228, 228, 228],
        control_edge_light_rgb: [255, 255, 255],
        control_edge_dark_rgb: [120, 120, 120],
        editor_bg_rgb: [254, 254, 254],
        editor_outline_rgb: [164, 164, 164],
        text_primary_rgb: [34, 34, 34],
    };
}
