//! `zxing` CLI binary — thin shim that calls the library entry point.

use std::process::ExitCode;

fn main() -> ExitCode {
    zxing::run()
}