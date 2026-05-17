//! Win32 façade: `GWLP_USERDATA` host attachment, HWND children, modal message pump.

#![cfg(windows)]

use core::ffi::c_void;

use unityform_core::{ComponentId, Rectangle};

use windows::Win32::{
    Foundation::{ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, PCWSTR, RECT, WPARAM},
    Graphics::Gdi::{BeginPaint, EndPaint, FillRect, PAINTSTRUCT},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};
use windows::core::{w, Result as WinResult};

use crate::application::ApplicationExitCode;
use crate::host::{brush_white_stock, HostState};
use crate::NativeForm;

unsafe extern "system" fn dispatch_window_message(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCCREATE => unsafe {
            let cs = *(lparam.0 as *const CREATESTRUCTW);
            let hp = cs.lpCreateParams.cast::<HostState>() as isize;
            if hp != 0 {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, hp);
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        },
        WM_DESTROY => unsafe {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HostState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);

            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }

            PostQuitMessage(0);
            LRESULT(0)
        },
        WM_ERASEBKGND => LRESULT(1),
        WM_COMMAND => unsafe {
            if let Some(raw) = hwnd_host(hwnd) {
                let handled =
                    (&mut *raw).dispatch_wm_command(wparam, lparam);
                if handled {
                    return LRESULT(0);
                }
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        },
        WM_PAINT => unsafe {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let brush = brush_white_stock();
            FillRect(hdc, &ps.rcPaint, brush);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        },
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

unsafe fn hwnd_host(hwnd: HWND) -> Option<*mut HostState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HostState;
    (!ptr.is_null()).then_some(ptr)
}

fn register_primitive_window_class(instance: HINSTANCE) -> WinResult<()> {
    let class_name = w!("UnityForm.Minimal.Window");

    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(dispatch_window_message),
        hInstance: instance,
        lpszClassName: PCWSTR(class_name.as_ptr()),
        style: CS_HREDRAW | CS_VREDRAW,
        ..Default::default()
    };

    unsafe {
        let atom = RegisterClassExW(&wnd_class);
        if atom == 0 {
            let err = windows::Win32::Foundation::GetLastError();
            if err != ERROR_CLASS_ALREADY_EXISTS {
                return Err(windows::core::Error::from_win32());
            }
        }
    }

    Ok(())
}

fn position_form_window(hwnd: HWND) -> WinResult<()> {
    unsafe {
        SetWindowPos(
            hwnd,
            HWND::default(),
            96,
            96,
            960,
            600,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        InvalidateRect(hwnd, None, true);
    }
    Ok(())
}

fn populate_demo(host: &mut HostState, form_hwnd: HWND) -> WinResult<()> {
    unsafe {
        let mut client = RECT::default();
        if !GetClientRect(form_hwnd, &mut client).as_bool() {
            return Err(windows::core::Error::from_win32());
        }

        let pad = 16i32;

        host.create_push_button(
            form_hwnd,
            pad,
            pad,
            (client.right - client.left).saturating_sub(pad.saturating_mul(2)),
            34,
            w!("Rust / Win32 unityform-platform — click handler"),
            || {
                println!("BN_CLICKED: WinForms-equivalent routed from WM_COMMAND.");
            },
        )?;
    }
    Ok(())
}

fn create_primitive_host_window(instance: HINSTANCE, host: *mut HostState) -> WinResult<HWND> {
    let style = WS_OVERLAPPEDWINDOW;
    let ex_style = WINDOW_EX_STYLE::default();

    unsafe {
        CreateWindowExW(
            ex_style,
            w!("UnityForm.Minimal.Window"),
            w!("UnityForm prototype — HWND host + BUTTON child"),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND::default(),
            HMENU::default(),
            instance,
            Some(host as *const c_void),
        )
    }
}

/// Runs the UI thread modal loop WinForms binds to `Application.Run`.
pub fn pump_foreground_loop() -> ApplicationExitCode {
    if let Err(err) = run_message_pump_inner() {
        eprintln!("unityform-platform (Win32) message pump failed: {err:?}");
    }
    ApplicationExitCode::SUCCESS
}

pub fn run_minimal_demo() -> Result<ApplicationExitCode, String> {
    pump_foreground_loop();
    Ok(ApplicationExitCode::SUCCESS)
}

fn run_message_pump_inner() -> WinResult<()> {
    unsafe {
        let instance = GetModuleHandleW(PCWSTR::null())?;
        register_primitive_window_class(instance)?;

        let boxed = Box::new(HostState::new(instance));
        let host_raw = Box::into_raw(boxed);

        let hwnd = match create_primitive_host_window(instance, host_raw) {
            Ok(h) => h,
            Err(e) => {
                drop(Box::from_raw(host_raw));
                return Err(e);
            }
        };

        if let Err(e) = position_form_window(hwnd) {
            let _ = DestroyWindow(hwnd);
            return Err(e);
        }

        {
            let host = &mut *host_raw;
            if let Err(e) = populate_demo(host, hwnd) {
                let _ = DestroyWindow(hwnd);
                return Err(e);
            }

            let mut nf = NativeForm::new(ComponentId::from_raw(1), Rectangle::ZERO);
            nf.set_bounds(Rectangle::from_xywh(100, 100, 960, 600));
            nf.set_hwnd(hwnd);
            let _mirror_form_handle = nf;
        }

        let _ = ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);

        loop {
            let mut msg = MSG::default();
            let status = GetMessageW(&mut msg, HWND::default(), 0, 0);

            match status {
                Ok(exit_flag) if exit_flag.0 == 0 => break,
                Ok(_) => {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                Err(_) => break,
            }
        }
    }

    Ok(())
}
