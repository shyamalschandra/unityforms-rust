//! Portable desktop host backed by [`winit`] + [`softbuffer`] (covers macOS, Linux/Wayland or X11, and BSDs that [`winit`] supports).

use std::num::NonZeroU32;
use std::ops::DerefMut;
use std::rc::Rc;

use raw_window_handle::{DisplayHandle as RwhDisplayHandle, HandleError, HasDisplayHandle};
use softbuffer::{Context, SoftBufferError, Surface};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::application::ApplicationExitCode;

/// `Rc<Window>` does not implement `HasDisplayHandle` in winit, but [`Window`] does.
/// Carry the `Rc` so `softbuffer::Context<D>` owns a `'static` display source.
#[derive(Clone)]
struct DisplayCarrier(Rc<Window>);

impl HasDisplayHandle for DisplayCarrier {
    fn display_handle(&self) -> Result<RwhDisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

#[inline(always)]
fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    u32::from(b) | (u32::from(g) << 8) | (u32::from(r) << 16)
}

#[derive(Clone)]
struct HeaderButton {
    x: u32,
    y: u32,
    w: NonZeroU32,
    h: NonZeroU32,
}

impl HeaderButton {
    fn for_width(inner_w: u32) -> Self {
        let pad = 16_u32;
        let w_inner = inner_w.saturating_sub(pad * 2).max(1);
        Self {
            x: pad,
            y: pad,
            w: NonZeroU32::new(w_inner).unwrap_or(NonZeroU32::new(1).unwrap()),
            h: NonZeroU32::new(40).expect("constant"),
        }
    }

    fn contains_px(&self, px: u32, py: u32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x.saturating_add(self.w.get())
            && py < self.y.saturating_add(self.h.get())
    }

    fn paint_into(&self, pixels: &mut [u32], stride: usize, height_px: usize) {
        let face = pack_rgb(0xe8, 0xe8, 0xe8);
        let border_hi = pack_rgb(0xff, 0xff, 0xff);
        let border_lo = pack_rgb(0x6f, 0x6f, 0x6f);

        let x0 = self.x as usize;
        let y0 = self.y as usize;
        let x1 = x0.saturating_add(self.w.get() as usize);
        let y1 = y0.saturating_add(self.h.get() as usize);

        rect_fill_clamped(pixels, stride, height_px, x0, y0, x1, y1, face);

        if y0 < height_px && x1 > 1 {
            rect_fill_row(pixels, stride, height_px, y0, x0, x1, border_hi);
        }
        if y1 > 1 && x1 > 1 {
            let hb = y1.saturating_sub(1);
            rect_fill_row(pixels, stride, height_px, hb, x0, x1, border_lo);
        }
        if x0 < stride && y1 > y0 {
            rect_fill_col(pixels, stride, height_px, x0, y0, y1.min(height_px), border_hi);
        }
        if x1 > 1 {
            let xb = x1.saturating_sub(1);
            if xb < stride {
                rect_fill_col(pixels, stride, height_px, xb, y0, y1.min(height_px), border_lo);
            }
        }
    }
}

fn rect_fill_clamped(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    color: u32,
) {
    let x_lo = x0.min(stride);
    let x_hi = x1.min(stride).max(x_lo);

    for row in y0..y1.min(height_px) {
        let base = row * stride;
        for col in x_lo..x_hi {
            pixels[base + col] = color;
        }
    }
}

fn rect_fill_row(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    row: usize,
    x0: usize,
    x1: usize,
    color: u32,
) {
    if row >= height_px {
        return;
    }
    let base = row * stride;
    let x_lo = x0.min(stride);
    let x_hi = x1.min(stride).max(x_lo);
    for col in x_lo..x_hi {
        pixels[base + col] = color;
    }
}

fn rect_fill_col(
    pixels: &mut [u32],
    stride: usize,
    height_px: usize,
    col: usize,
    y0: usize,
    y1: usize,
    color: u32,
) {
    if col >= stride {
        return;
    }
    for row in y0..y1.min(height_px) {
        pixels[row * stride + col] = color;
    }
}

fn redraw(surf: &mut Surface<DisplayCarrier, Rc<Window>>, window: &Rc<Window>) -> Result<(), SoftBufferError> {
    let size = window.inner_size();
    let w = NonZeroU32::new(size.width.max(1)).expect("max(1) positive");
    let h = NonZeroU32::new(size.height.max(1)).expect("max(1) positive");

    surf.resize(w, h)?;

    let mut buffer = surf.buffer_mut()?;
    let stride = buffer.width().get() as usize;
    let height_px = buffer.height().get() as usize;
    debug_assert_eq!(stride * height_px, buffer.len());

    let bg = pack_rgb(0xfe, 0xfe, 0xfe);
    let btn = HeaderButton::for_width(size.width.max(1));

    {
        let pix = buffer.deref_mut();
        pix.fill(bg);
        btn.paint_into(pix, stride, height_px);
    }

    buffer.present()?;
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
    let event_loop =
        EventLoop::new().map_err(|e| format!("EventLoop::new() failed: {e}"))?;

    let window = Rc::new(
        WindowBuilder::new()
            .with_title("UnityForm prototype (portable)")
            .build(&event_loop)
            .map_err(|e| format!("building window failed: {e}"))?,
    );

    let carrier = DisplayCarrier(Rc::clone(&window));

    let context =
        Context::new(carrier).map_err(|e| format!("softbuffer Context failed: {e}"))?;

    let mut surface = Surface::new(&context, Rc::clone(&window)).map_err(|e| {
        format!("softbuffer Surface failed: {e}")
    })?;

    let wind_run = Rc::clone(&window);

    wind_run.request_redraw();

    let mut cursor: Option<PhysicalPosition<f64>> = None;

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
                        if let Err(e) = redraw(&mut surface, &wind_run) {
                            eprintln!("unityform-platform: redraw failed: {e}");
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor = Some(position);
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
                        let btn = HeaderButton::for_width(inner.width.max(1));
                        if btn.contains_px(px, py) {
                            println!(
                                "Portable click: framebuffer button hit-test (winit MouseInput)."
                            );
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        })
        .map_err(|e| format!("event loop exited with error: {e}"))?;

    Ok(())
}
