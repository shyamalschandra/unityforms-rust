//! Portable desktop shell: [`winit`] window + [`softbuffer`] + bitmap font glyphs.

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::ops::DerefMut;
use std::rc::Rc;

use font8x8::{BASIC_FONTS, UnicodeFonts};
use raw_window_handle::{DisplayHandle as RwhDisplayHandle, HandleError, HasDisplayHandle};
use softbuffer::{Context, SoftBufferError, Surface};
use unityform_core::{dock_top_stack, inset_rect, Margin, Rectangle, ThemePalette};
use winit::dpi::PhysicalPosition;
use winit::event::{
    ElementState, Event, Ime, KeyEvent, MouseButton, TouchPhase, WindowEvent,
};
use winit::keyboard::{Key, NamedKey};
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::application::ApplicationExitCode;

#[derive(Clone)]
struct DisplayCarrier(Rc<Window>);

impl HasDisplayHandle for DisplayCarrier {
    fn display_handle(&self) -> Result<RwhDisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

#[inline(always)]
fn pack_rgb(rgb: [u8; 3]) -> u32 {
    let [r, g, b] = rgb;
    u32::from(b) | (u32::from(g) << 8) | (u32::from(r) << 16)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Focus {
    Toolbar,
    Editor,
}

struct ShellState {
    theme: ThemePalette,
    focus: Focus,
    text: String,
    caret_byte: usize,
}

impl ShellState {
    fn new() -> Self {
        Self {
            theme: ThemePalette::LIGHT,
            focus: Focus::Toolbar,
            text: String::new(),
            caret_byte: 0,
        }
    }

    fn clamp_caret(&mut self) {
        self.caret_byte = self.caret_byte.min(self.text.len());
        while self.caret_byte > 0 && !self.text.is_char_boundary(self.caret_byte) {
            self.caret_byte -= 1;
        }
    }

    fn prev_char_boundary(&self, idx: usize) -> usize {
        if idx == 0 {
            return 0;
        }
        let prefix = &self.text[..idx];
        prefix
            .char_indices()
            .next_back()
            .map(|(b, _)| b)
            .unwrap_or(0)
    }

    fn backspace(&mut self) {
        if self.caret_byte == 0 {
            return;
        }
        let prev = self.prev_char_boundary(self.caret_byte);
        self.text.drain(prev..self.caret_byte);
        self.caret_byte = prev;
        self.clamp_caret();
    }

    fn insert_char(&mut self, ch: char) {
        if ch == '\r' {
            return;
        }
        if ch == '\u{8}' {
            self.backspace();
            return;
        }
        if ch.is_control() && ch != '\n' {
            return;
        }
        self.clamp_caret();
        self.text.insert(self.caret_byte, ch);
        self.caret_byte += ch.len_utf8();
        self.clamp_caret();
    }

    fn handle_key_pressed(&mut self, event: &KeyEvent) {
        if self.focus != Focus::Editor {
            return;
        }

        match &event.logical_key {
            Key::Named(NamedKey::Backspace) => {
                self.backspace();
            }
            Key::Named(NamedKey::Enter) => {
                self.insert_char('\n');
            }
            _ => {}
        }

        if let Some(t) = &event.text {
            if matches!(
                event.logical_key,
                Key::Named(NamedKey::Enter)
            ) && (t.as_str() == "\r" || t.as_str() == "\r\n" || t.as_str() == "\n")
            {
                return;
            }
            for ch in t.chars() {
                self.insert_char(ch);
            }
        }
    }

    fn append_ime(&mut self, s: &str) {
        if self.focus != Focus::Editor {
            return;
        }
        for ch in s.chars() {
            self.insert_char(ch);
        }
    }

    fn measure_chars_px(chars_to_left: usize, scale: u32) -> u32 {
        (chars_to_left as u32).saturating_mul(8).saturating_mul(scale)
    }
}

struct UiFrame {
    btn: Rectangle,
    caption: Rectangle,
    editor_outer: Rectangle,
}

impl UiFrame {
    fn compute(inner_w: u32, inner_h: u32) -> Self {
        let cw = inner_w.max(1) as i32;
        let ch = inner_h.max(1) as i32;
        let root = Rectangle::from_xywh(0, 0, cw, ch);
        let (bands, remainder) = dock_top_stack(root, &[48, 26]);
        let btn = bands.get(0).copied().unwrap_or(root);
        let caption = bands.get(1).copied().unwrap_or(root);
        Self {
            btn,
            caption,
            editor_outer: inset_rect(remainder, Margin::uniform(8)),
        }
    }

    fn to_u32_xy(r: Rectangle) -> (u32, u32, u32, u32) {
        (
            r.x.max(0) as u32,
            r.y.max(0) as u32,
            r.width.max(0) as u32,
            r.height.max(0) as u32,
        )
    }
}

fn rect_fill_clamped(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: u32,
) {
    let x_lo = x0.min(stride as u32) as usize;
    let x_hi = x1.min(stride as u32).max(x_lo as u32) as usize;
    for row in y0 as usize..(y1 as usize).min(height_px) {
        let base = row * stride;
        for col in x_lo..x_hi {
            pixels[base + col] = color;
        }
    }
}

fn blit_glyph_scaled(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    origin_x: u32,
    origin_y: u32,
    glyph: &[u8; 8],
    scale: u32,
    fg: u32,
    bg_opt: Option<u32>,
) {
    for row in 0u32..8u32 {
        let bits = glyph[row as usize];
        for col in 0u32..8 {
            let on = bits & (1 << (7 - col)) != 0;
            if !on && bg_opt.is_none() {
                continue;
            }
            let pix = if on { fg } else { bg_opt.unwrap() };
            for dy in 0..scale {
                for dx in 0..scale {
                    let px = origin_x.saturating_add(col.saturating_mul(scale).saturating_add(dx));
                    let py =
                        origin_y.saturating_add(row.saturating_mul(scale).saturating_add(dy));
                    if (px as usize) < stride && (py as usize) < height_px {
                        pixels[py as usize * stride + px as usize] = pix;
                    }
                }
            }
        }
    }
}

fn draw_text_block(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    mut x: u32,
    y: u32,
    text: &str,
    fg: u32,
    bg: u32,
    scale: u32,
) -> u32 {
    for ch in text.chars() {
        let glyph = BASIC_FONTS.get(ch).unwrap_or_else(|| BASIC_FONTS.get(' ').unwrap_or([0u8; 8]));
        blit_glyph_scaled(pixels, stride, height_px, x, y, &glyph, scale, fg, Some(bg));
        x = x.saturating_add(8 * scale);
        if x as usize >= stride {
            break;
        }
    }
    x
}

fn draw_soft_border(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    light: u32,
    dark: u32,
    bg: u32,
) {
    rect_fill_clamped(
        pixels,
        stride,
        height_px,
        x,
        y,
        x.saturating_add(w),
        y.saturating_add(h),
        bg,
    );
    if h > 0 && w > 0 {
        rect_fill_clamped(pixels, stride, height_px, x, y, x.saturating_add(w), y.saturating_add(1), light);
        rect_fill_clamped(pixels, stride, height_px, x, y, x.saturating_add(1), y.saturating_add(h), light);
        let xb = x.saturating_add(w).saturating_sub(1);
        let yb = y.saturating_add(h).saturating_sub(1);
        rect_fill_clamped(pixels, stride, height_px, x, yb, x.saturating_add(w), y.saturating_add(h), dark);
        rect_fill_clamped(pixels, stride, height_px, xb, y, x.saturating_add(w), y.saturating_add(h), dark);
    }
}

fn visual_lines(text: &str) -> Vec<&str> {
    let ls: Vec<&str> = text.split('\n').collect();
    if ls.is_empty() {
        vec![""]
    } else {
        ls
    }
}

fn caret_line_col(state: &ShellState) -> (usize, usize) {
    let upto = state.caret_byte.min(state.text.len());
    let safe = if upto == 0 || state.text.is_char_boundary(upto) {
        upto
    } else {
        state.text[..upto].char_indices().last().map(|(b, _)| b).unwrap_or(0)
    };
    let before = &state.text[..safe];
    let line_idx = before.matches('\n').count();
    let col = before
        .rsplit('\n')
        .next()
        .unwrap_or("")
        .chars()
        .count();
    (line_idx, col)
}

fn draw_editor(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    outer: Rectangle,
    state: &ShellState,
    scale: u32,
) {
    let (bx, by, bw, bh) = UiFrame::to_u32_xy(outer);
    let theme = state.theme;
    let face = pack_rgb(theme.editor_bg_rgb);
    let light = pack_rgb(theme.control_edge_light_rgb);
    let dark = pack_rgb(theme.control_edge_dark_rgb);
    let fg = pack_rgb(theme.text_primary_rgb);

    draw_soft_border(pixels, stride, height_px, bx, by, bw, bh, light, dark, face);

    let pad = 8u32;
    let inner_x = bx.saturating_add(pad);
    let inner_y = by.saturating_add(pad);
    let line_h = (8 * scale).saturating_add(scale.saturating_mul(2));

    let lines = visual_lines(&state.text);
    for (i, line) in lines.iter().enumerate() {
        let y = inner_y.saturating_add((i as u32).saturating_mul(line_h));
        if y as usize >= height_px {
            break;
        }
        draw_text_block(pixels, stride, height_px, inner_x, y, line, fg, face, scale);
    }

    if matches!(state.focus, Focus::Editor) {
        let (line_idx, col) = caret_line_col(state);
        let caret_x = inner_x.saturating_add(ShellState::measure_chars_px(col, scale));
        let caret_y = inner_y.saturating_add((line_idx as u32).saturating_mul(line_h));
        rect_fill_clamped(
            pixels,
            stride,
            height_px,
            caret_x,
            caret_y,
            caret_x.saturating_add(2),
            caret_y.saturating_add(line_h.saturating_sub(scale)),
            fg,
        );
    }
}

fn redraw_all(
    surf: &mut Surface<DisplayCarrier, Rc<Window>>,
    window: &Rc<Window>,
    state: &ShellState,
) -> Result<(), SoftBufferError> {
    let size = window.inner_size();
    let wnz = NonZeroU32::new(size.width.max(1)).unwrap();
    let hnz = NonZeroU32::new(size.height.max(1)).unwrap();
    surf.resize(wnz, hnz)?;

    let mut buf = surf.buffer_mut()?;
    let stride = buf.width().get() as usize;
    let height_px = buf.height().get() as usize;
    let frame = UiFrame::compute(size.width.max(1), size.height.max(1));

    let theme = state.theme;
    let win_bg = pack_rgb(theme.window_bg_rgb);
    let face = pack_rgb(theme.control_face_rgb);
    let lite = pack_rgb(theme.control_edge_light_rgb);
    let dk = pack_rgb(theme.control_edge_dark_rgb);
    let fg = pack_rgb(theme.text_primary_rgb);
    let cap_bg = face;

    {
        let pix = buf.deref_mut();
        pix.fill(win_bg);

        let (bx, by, bw, bh) = UiFrame::to_u32_xy(frame.btn);
        draw_soft_border(pix, stride, height_px, bx, by, bw, bh, lite, dk, face);
        let label = "Run demo (portable hit-test)";
        let tx = bx.saturating_add(12);
        let ty = by.saturating_add((bh.saturating_sub(8 * 3)) / 2);
        draw_text_block(pix, stride, height_px, tx, ty, label, fg, face, 3);

        let (cx, cy, cw, ch) = UiFrame::to_u32_xy(frame.caption);
        rect_fill_clamped(pix, stride, height_px, cx, cy, cx.saturating_add(cw), cy.saturating_add(ch), cap_bg);
        draw_text_block(
            pix,
            stride,
            height_px,
            cx.saturating_add(8),
            cy.saturating_add(4),
            "Notes — click the editor to type (font8x8 + softbuffer).",
            fg,
            cap_bg,
            2,
        );

        draw_editor(pix, stride, height_px, frame.editor_outer, state, 2);
    }

    buf.present()?;
    Ok(())
}

pub(crate) fn pump_foreground_loop() -> ApplicationExitCode {
    match run_portable() {
        Ok(()) => ApplicationExitCode::SUCCESS,
        Err(message) => {
            eprintln!("unityform-platform (portable backend): {message}");
            ApplicationExitCode::FAILURE
        }
    }
}

pub fn run_minimal_demo() -> Result<ApplicationExitCode, String> {
    run_portable().map(|_| ApplicationExitCode::SUCCESS)
}

fn run_portable() -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("EventLoop::new() failed: {e}"))?;

    let window = Rc::new(
        WindowBuilder::new()
            .with_title("UnityForm — portable shell (winit + softbuffer)")
            .build(&event_loop)
            .map_err(|e| format!("building window failed: {e}"))?,
    );

    let carrier = DisplayCarrier(Rc::clone(&window));
    let context = Context::new(carrier).map_err(|e| format!("softbuffer Context failed: {e}"))?;
    let mut surface = Surface::new(&context, Rc::clone(&window))
        .map_err(|e| format!("softbuffer Surface failed: {e}"))?;

    let wind_run = Rc::clone(&window);
    let shell = Rc::new(RefCell::new(ShellState::new()));
    let mut cursor: Option<PhysicalPosition<f64>> = None;

    wind_run.request_redraw();

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                Event::Resumed => wind_run.request_redraw(),
                Event::WindowEvent {
                    window_id,
                    event,
                } if window_id == wind_run.id() => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(_) => wind_run.request_redraw(),
                    WindowEvent::RedrawRequested => {
                        let st = shell.borrow();
                        if let Err(e) = redraw_all(&mut surface, &wind_run, &st) {
                            eprintln!("unityform-platform: redraw failed: {e}");
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor = Some(position);
                    }
                    WindowEvent::Touch(touch) if touch.phase == TouchPhase::Ended => {
                        let inner = wind_run.inner_size();
                        let px = touch.location.x.clamp(0.0, f64::from(inner.width.max(1))) as u32;
                        let py = touch.location.y.clamp(0.0, f64::from(inner.height.max(1))) as u32;
                        apply_hit_test(&shell, &wind_run, px, py);
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    } => {
                        let Some(cursor) = cursor else {
                            return;
                        };
                        let inner = wind_run.inner_size();
                        let px = cursor.x.round().clamp(0.0, f64::from(inner.width.max(1))) as u32;
                        let py = cursor.y.round().clamp(0.0, f64::from(inner.height.max(1))) as u32;
                        apply_hit_test(&shell, &wind_run, px, py);
                    }
                    WindowEvent::KeyboardInput {
                        event,
                        is_synthetic,
                        ..
                    } => {
                        if event.state == ElementState::Pressed && !is_synthetic {
                            shell.borrow_mut().handle_key_pressed(&event);
                            wind_run.request_redraw();
                        }
                    }
                    WindowEvent::Ime(Ime::Commit(s)) => {
                        shell.borrow_mut().append_ime(s.as_str());
                        wind_run.request_redraw();
                    }
                    _ => (),
                },
                _ => (),
            }
        })
        .map_err(|e| format!("event loop exited with error: {e}"))?;

    Ok(())
}

fn apply_hit_test(shell: &Rc<RefCell<ShellState>>, window: &Rc<Window>, px: u32, py: u32) {
    let inner = window.inner_size();
    let frame = UiFrame::compute(inner.width.max(1), inner.height.max(1));
    let (bx, by, bw, bh) = UiFrame::to_u32_xy(frame.btn);
    let (ex, ey, ew, eh) = UiFrame::to_u32_xy(frame.editor_outer);

    let mut st = shell.borrow_mut();
    if px >= bx && py >= by && px < bx.saturating_add(bw) && py < by.saturating_add(bh) {
        st.focus = Focus::Toolbar;
        println!("Portable: toolbar button region clicked (focus → toolbar).");
    } else if px >= ex && py >= ey && px < ex.saturating_add(ew) && py < ey.saturating_add(eh) {
        st.focus = Focus::Editor;
        let _ = window.set_ime_allowed(true);
        println!("Portable: editor focused — keyboard + IME enabled.");
    } else {
        st.focus = Focus::Toolbar;
        let _ = window.set_ime_allowed(false);
    }
    window.request_redraw();
}
