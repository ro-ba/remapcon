pub mod app;
pub mod config;
pub mod controller;
pub mod editor;
pub mod mapping;
pub mod update;
pub mod webview;
#[cfg(windows)]
#[allow(unsafe_code)] // Foreground/picker APIs and reference CRT comparison.
pub mod windows_app;
#[cfg(windows)]
#[allow(unsafe_code)] // Atomic replacement, backup copy and imported target attributes.
pub mod windows_config;
#[cfg(windows)]
pub mod windows_controller;
#[cfg(windows)]
#[allow(unsafe_code)] // HWND/WndProc host; shared production and diagnostic GUI.
pub mod windows_gui;
#[cfg(windows)]
#[allow(unsafe_code)] // Native Win32 boundary; safe modules forbid unsafe code.
pub mod windows_hid;
#[cfg(windows)]
#[allow(unsafe_code)] // Shell/WIC icon encoding and native cache identity boundary.
pub mod windows_icons;
#[cfg(windows)]
#[allow(unsafe_code)] // SendInput/SystemParametersInfoW boundary only.
pub mod windows_output;
#[cfg(windows)]
pub mod windows_runtime;
#[cfg(windows)]
#[allow(unsafe_code)] // Notification icon, native menu and window placement APIs.
pub mod windows_shell;
#[cfg(windows)]
#[allow(unsafe_code)] // SetupAPI/registry/helper process boundary only.
pub mod windows_steam;
#[cfg(windows)]
#[allow(unsafe_code)] // WinHTTP/BCrypt/update notification/native process boundary.
pub mod windows_update;
#[cfg(windows)]
#[allow(unsafe_code)] // STA WebView2 COM and HWND boundary only.
pub mod windows_webview;
