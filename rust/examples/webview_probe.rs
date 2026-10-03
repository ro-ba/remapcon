//! Existing UI diagnostic; all host code is shared with the Rust GUI binary.
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    padmux_diag::windows_gui::run(true)
}
#[cfg(not(windows))]
fn main() {
    eprintln!("WebView2 diagnostic requires Windows");
}
