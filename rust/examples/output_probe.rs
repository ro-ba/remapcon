//! Small native comparison with C++ SendInput, independent of HID/mapping.
#[cfg(windows)]
fn main() {
    use padmux_diag::{mapping::Output, windows_output::send_checked};
    for x in [1, -1] {
        let event = Output::Move { x, y: 0 };
        println!("Rust {event:?}: {:?}", send_checked(event));
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows required");
}
