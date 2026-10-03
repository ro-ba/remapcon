#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    #[cfg(windows)]
    {
        let diagnostic = std::env::args_os()
            .nth(1)
            .is_some_and(|s| s == "--diagnostic");
        if let Err(error) = padmux_diag::windows_gui::run(diagnostic) {
            eprintln!("PadMux startup failed: {error}");
            std::process::exit(1);
        }
    }
    #[cfg(not(windows))]
    eprintln!("PadMux GUI requires Windows");
}
