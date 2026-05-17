# UnityForm (Rust)

Experimental **Windows Forms–inspired desktop shell** implemented in Rust, with:

- **Windows** — Native **Win32** message pump (`GetMessage`), `HWND` host, **`WM_COMMAND` / `BN_CLICKED`** routing to closures (similar in spirit to WinForms `Click`).
- **macOS, Linux, and typical BSD desktops** — Portable **`winit`** window + **`softbuffer`** framebuffer demo (drawn chrome + mouse hit-testing), gated behind `cfg(not(windows))`.

> This is **not** a full port of [dotnet/winforms](https://github.com/dotnet/winforms). It targets the same conceptual stack (HWND graph + notify routing on Windows; portable loop elsewhere) so the project can grow toward richer controls.

## Repository layout

| Path | Crate | Purpose |
|------|--------|---------|
| `crates/unityform-core` | `unityform-core` | Shared geometry (`Point`, `Rectangle`, …) and `Component` / `Control` traits. |
| `crates/unityform-platform` | `unityform-platform` | Cross-platform **`UnityApplication::run()`**; Win32 backend + portable backend. |
| `crates/unityform-windows` | `unityform-windows` | Thin re-export of `unityform-platform` for older `Cargo.toml` paths. |
| `examples/minimal_window` | binary | Minimal sample (same entry point on every OS). |

## Requirements

- **Rust** 1.78+ (`rust-version` in workspace `Cargo.toml`).
- **Windows**: MSVC or GNU Windows target for the native Win32 path.
- **Other OSes**: A display stack supported by **winit 0.29** and **softbuffer 0.4** (e.g. AppKit, X11, Wayland).

## Build and run

```bash
cargo build --workspace
cargo run -p minimal_window
```

On **Windows**, the sample uses the **native** `BUTTON` control and console output on click. On **other platforms**, it opens a **framebuffer** window with a clickable header region.

## License

This project is licensed under the **MIT License** — see [`LICENSE`](LICENSE).
