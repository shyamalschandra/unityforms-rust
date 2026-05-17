//! Desktop shell integrations for UnityForm-like apps.
//!
//! - **Windows** — HWND message pump, Common Controls-compatible routing (`WM_COMMAND` / ids).
//! - **Everything else here** — portable host via [`winit`](https://docs.rs/winit) +
//!   [`softbuffer`](https://docs.rs/softbuffer) (covers macOS, Linux, *BSD desktops that winit targets).

mod application;
mod form;

#[cfg(windows)]
mod host;
#[cfg(windows)]
mod win32;

#[cfg(not(windows))]
mod portable;

pub use application::{ApplicationExitCode, UnityApplication, WindowsApplication};
pub use form::NativeForm;

#[cfg(windows)]
pub use win32::run_minimal_demo;

#[cfg(not(windows))]
pub use portable::run_minimal_demo;
