use std::process::ExitCode;

use unityform_platform::UnityApplication;

fn main() -> ExitCode {
    UnityApplication::run().into()
}
