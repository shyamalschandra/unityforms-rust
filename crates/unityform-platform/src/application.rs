use std::process::ExitCode;

/// Process exit status after [`UnityApplication::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationExitCode(pub u32);

impl ApplicationExitCode {
    pub const SUCCESS: Self = Self(0);
    pub const FAILURE: Self = Self(1);
}

impl From<ApplicationExitCode> for ExitCode {
    fn from(value: ApplicationExitCode) -> Self {
        if value.0 == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

/// Cross-platform top-level runner (maps to Win32 `Application.Run` on Windows, or the portable pump elsewhere).
pub struct UnityApplication;

impl UnityApplication {
    /// Owns the primary event/message loop until the shell window closes.
    pub fn run() -> ApplicationExitCode {
        #[cfg(windows)]
        {
            crate::win32::pump_foreground_loop()
        }
        #[cfg(not(windows))]
        {
            crate::portable::pump_foreground_loop()
        }
    }
}

/// Historical name retained for early experiments that targeted Win32-only.
pub type WindowsApplication = UnityApplication;
