# UnityForm (Rust)

Experimental **Windows Forms–inspired desktop shell** implemented in Rust, with:

- **Windows** — Native **Win32** message pump (`GetMessage`), `HWND` host, **`WM_COMMAND`** routing (**`BN_CLICKED`** on toolbar buttons), and **`EN_CHANGE`** tracing on a docked multiline **`EDIT`** child. **`WM_SIZE`** triggers **`dock_top_stack`–style relayout**: top bands plus a margin-inset fill region for the editor.
- **macOS, Linux, and typical BSD desktops** — Portable **`winit`** window + **`softbuffer`** demo: **`ThemePalette`**-driven fills, **`font8x8`** glyph rendering, focus + hit-test (toolbar vs editor), **`KeyboardInput`** (`KeyEvent::state`), and **`Ime::Commit`** for IME text.

Shared **`unityform-core`** adds **dock layout helpers** (`Margin`, `Anchors`, `Dock`, `dock_top_stack`, `inset_rect`), a **`ThemePalette`** preset (`LIGHT`), and **accessibility taxonomy stubs** (`AccessibleRole`, `AccessibleAnnouncement`) — not wired to OS screen readers yet.

> This is **not** a full port of [dotnet/winforms](https://github.com/dotnet/winforms). It targets the same conceptual stack (HWND graph + notify routing on Windows; portable loop elsewhere) so the project can grow toward richer controls, tab order, and real accessibility bridges.

## Repository layout

| Path | Crate | Purpose |
|------|--------|---------|
| `crates/unityform-core` | `unityform-core` | Shared geometry, `Component` / `Control` traits, layout, theme, accessibility stubs. |
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

On **Windows**, the sample shows a docked **`BUTTON`** + caption **`STATIC`**, a **`WM_SIZE`**-aware **`EDIT`** area, and console traces for clicks / edit changes. On **other platforms**, it opens a **framebuffer** window with toolbar, caption strip, multiline pseudo-editor (**Enter** newline, IME commit), and theme-aware chrome.

## License

This project is licensed under the **MIT License** — see [`LICENSE`](LICENSE).
