use unityform_core::{Component, ComponentId, Control, Rectangle};

/// A top-level Win32 surface (`HWND`) exposed through the shared control trait.
///
/// This is the first building block toward a `Form`-like type: bounds map to
/// `GetWindowRect`/`SetWindowPos` and the handle bridges into the native stack.
pub struct NativeForm {
    id: ComponentId,
    bounds: Rectangle,
    #[cfg(windows)]
    hwnd: ::windows::Win32::Foundation::HWND,
}

impl NativeForm {
    pub fn new(id: ComponentId, bounds: Rectangle) -> Self {
        Self {
            id,
            bounds,
            #[cfg(windows)]
            hwnd: ::windows::Win32::Foundation::HWND::default(),
        }
    }

    #[cfg(windows)]
    pub fn hwnd(&self) -> ::windows::Win32::Foundation::HWND {
        self.hwnd
    }

    #[cfg(windows)]
    pub(crate) fn set_hwnd(&mut self, hwnd: ::windows::Win32::Foundation::HWND) {
        self.hwnd = hwnd;
    }
}

impl Component for NativeForm {
    fn id(&self) -> ComponentId {
        self.id
    }
}

impl Control for NativeForm {
    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn set_bounds(&mut self, value: Rectangle) {
        self.bounds = value;
    }
}
