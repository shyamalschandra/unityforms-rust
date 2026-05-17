//! Per-top-level-window host: binds `HWND` notifies to closures (WinForms-style `Click`).

#![cfg(windows)]

use core::ffi::c_void;
use std::collections::HashMap;

use core::ffi::c_void;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::{
    Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH},
    UI::WindowsAndMessaging::{
        CreateWindowExW, HMENU, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD, WS_VISIBLE,
    },
};
use windows::core::{Result as WinResult, w, PCWSTR};

/// Tracks child control ids ↔ handlers for `WM_COMMAND` just like WinForms routes `BN_CLICKED`.
pub struct HostState {
    handlers: HashMap<u32, Box<dyn FnMut()>>,
    seq: u32,
    pub instance: HINSTANCE,
}

impl HostState {
    pub fn new(instance: HINSTANCE) -> Self {
        Self {
            handlers: HashMap::new(),
            seq: 1000,
            instance,
        }
    }

    /// Native push button (`CreateWindowExW` + `"BUTTON"`). Invokes `on_click` on `BN_CLICKED`.
    pub unsafe fn create_push_button(
        &mut self,
        parent: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        caption: PCWSTR,
        on_click: impl FnMut() + 'static,
    ) -> WinResult<()> {
        let cmd_id = self.seq.saturating_add(1);

        match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            caption,
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0),
            x,
            y,
            width,
            height,
            parent,
            HMENU(cmd_id as *mut c_void),
            self.instance,
            None,
        ) {
            Ok(_) => {
                self.seq = cmd_id;
                self.handlers.insert(cmd_id, Box::new(on_click));
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// Returns `true` when the notify code was dispatched to a Rust handler.
    pub fn dispatch_wm_command(&mut self, wparam: WPARAM, _lparam: LPARAM) -> bool {
        const BN_CLICKED: u16 = 0;

        let notify = hiword_wp(wparam);
        if notify != BN_CLICKED {
            return false;
        }

        let id = loword_wp(wparam) as u32;
        let Some(handler) = self.handlers.get_mut(&id) else {
            return false;
        };
        (**handler)();
        true
    }
}

#[inline(always)]
pub fn brush_white_stock() -> HBRUSH {
    unsafe { GetStockObject(WHITE_BRUSH) }.cast()
}

#[inline(always)]
pub fn hiword_wp(wparam: WPARAM) -> u16 {
    ((wparam.0 >> 16) & 0xFFFF) as u16
}

#[inline(always)]
pub fn loword_wp(wparam: WPARAM) -> u16 {
    (wparam.0 & 0xFFFF) as u16
}
