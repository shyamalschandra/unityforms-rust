//! Per-top-level-window host: Win32 child graph, `WM_COMMAND` routing, and docked layout.

#![cfg(windows)]

use core::ffi::c_void;
use std::collections::HashMap;

use unityform_core::{dock_top_stack, inset_rect, Margin, Rectangle as ClientRect};

use windows::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{
        CreateWindowExW, GetClientRect, MoveWindow, WINDOW_EX_STYLE, WINDOW_STYLE,
        HMENU,
        WS_CHILD, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
    },
};
use windows::Win32::{
    Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH},
};
use windows::core::{w, PCWSTR};
use windows::core::Result as WinResult;

pub struct HostState {
    pub instance: HINSTANCE,
    seq: u32,
    button_handlers: HashMap<u32, Box<dyn FnMut()>>,
    dock_top: Vec<(HWND, i32)>,
    fill_edit: Option<(HWND, u32)>,
}

impl HostState {
    pub fn new(instance: HINSTANCE) -> Self {
        Self {
            instance,
            seq: 1000,
            button_handlers: HashMap::new(),
            dock_top: Vec::new(),
            fill_edit: None,
        }
    }

    fn next_id(&mut self) -> u32 {
        self.seq = self.seq.saturating_add(1);
        self.seq
    }

    unsafe fn menu_id(id: u32) -> HMENU {
        HMENU(id as *mut c_void)
    }

    pub unsafe fn relayout(&mut self, parent: HWND) -> WinResult<()> {
        let mut rr = windows::Win32::Foundation::RECT::default();
        GetClientRect(parent, &mut rr).ok()?;
        let cw = rr.right - rr.left;
        let ch = rr.bottom - rr.top;
        let client = ClientRect::from_xywh(0, 0, cw, ch);
        let band_heights: Vec<i32> = self.dock_top.iter().map(|(_, h)| *h).collect();
        let (bands, remainder) = dock_top_stack(client, &band_heights);

        for (i, (hwnd, _)) in self.dock_top.iter().enumerate() {
            if let Some(r) = bands.get(i) {
                MoveWindow(*hwnd, r.x, r.y, r.width, r.height, true)?;
            }
        }

        if let Some((edit_hwnd, _)) = self.fill_edit {
            let editor = inset_rect(remainder, Margin::uniform(8));
            MoveWindow(
                edit_hwnd,
                editor.x,
                editor.y,
                editor.width,
                editor.height,
                true,
            )?;
        }
        Ok(())
    }

    pub unsafe fn create_docked_toolbar_button(
        &mut self,
        parent: HWND,
        caption: PCWSTR,
        band_height: i32,
        on_click: impl FnMut() + 'static,
    ) -> WinResult<()> {
        let id = self.next_id();
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            caption,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(0),
            0,
            0,
            10,
            band_height.max(24),
            parent,
            Self::menu_id(id),
            self.instance,
            None,
        )?;
        self.dock_top.push((hwnd, band_height));
        self.button_handlers.insert(id, Box::new(on_click));
        Ok(())
    }

    pub unsafe fn create_docked_caption_strip(
        &mut self,
        parent: HWND,
        caption: PCWSTR,
        band_height: i32,
    ) -> WinResult<()> {
        let id = self.next_id();
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            caption,
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0),
            0,
            0,
            10,
            band_height.max(18),
            parent,
            Self::menu_id(id),
            self.instance,
            None,
        )?;
        self.dock_top.push((hwnd, band_height));
        Ok(())
    }

    pub unsafe fn attach_fill_multiline_editor(
        &mut self,
        parent: HWND,
        placeholder: PCWSTR,
    ) -> WinResult<()> {
        let edit_id = self.next_id();

        let style = WS_CHILD
            | WS_VISIBLE
            | WS_TABSTOP
            | WS_BORDER
            | WS_VSCROLL
            | WINDOW_STYLE(
                (ES_LEFT
                    | ES_MULTILINE
                    | ES_AUTOVSCROLL
                    | ES_WANTRETURN) as u32,
            );

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            placeholder,
            style,
            0,
            0,
            120,
            120,
            parent,
            Self::menu_id(edit_id),
            self.instance,
            None,
        )?;
        self.fill_edit = Some((hwnd, edit_id));
        Ok(())
    }

    pub fn dispatch_wm_command(&mut self, wparam: WPARAM, _lparam: LPARAM) -> bool {
        const BN_CLICKED: u16 = 0;
        const EN_CHANGE: u16 = 0x0300;

        let notify = hiword_wp(wparam);
        let ctrl_id = loword_wp(wparam) as u32;

        if notify == BN_CLICKED {
            if let Some(h) = self.button_handlers.get_mut(&ctrl_id) {
                (**h)();
                return true;
            }
        } else if notify == EN_CHANGE {
            if let Some((_, edit_id)) = self.fill_edit {
                if edit_id == ctrl_id {
                    println!("TRACE: EDIT EN_CHANGE for control id={ctrl_id}");
                    return true;
                }
            }
        }
        false
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
