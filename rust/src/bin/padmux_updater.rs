#![cfg_attr(windows, windows_subsystem = "windows")]
fn main() {
    #[cfg(windows)]
    {
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        std::process::exit(padmux_diag::windows_update::run(&arguments));
    }
    #[cfg(not(windows))]
    eprintln!("PadMux updater requires Windows");
}
