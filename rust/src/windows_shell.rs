//! Tray/window persistence from Remapcon main.cpp (MIT, copyright 2026 ro-ba).
//! Preserve ../../LICENSE and THIRD_PARTY_NOTICES.md.
use crate::windows_hid::own;
use crate::windows_icons::Icon;
use std::{
    cell::Cell,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    os::windows::io::OwnedHandle,
    path::{Path, PathBuf},
    time::Duration,
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, POINT},
        System::{Threading::CreateMutexW, WindowsProgramming::WritePrivateProfileStringW},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};
pub const TRAY: u32 = WM_APP + 3;
pub const FORCE_EXIT: u32 = WM_APP + 4;
pub const CLOSE_REQUEST: u32 = WM_APP + 42;
pub const ALREADY_RUNNING: u32 = WM_APP + 6;
pub const SHOW: usize = 401;
pub const EXIT: usize = 402;
/// Hold the C++ named mutex until shutdown, or notify the existing window.
/// Diagnostic callers use separate names; production keeps Remapcon's names for cross-version exclusion.
pub fn single_instance(name: &str, class: &str) -> windows::core::Result<Option<OwnedHandle>> {
    if name.contains('\0') || class.contains('\0') {
        return Err(windows::Win32::Foundation::E_INVALIDARG.into());
    }
    let name: Vec<_> = name.encode_utf16().chain([0]).collect();
    let class: Vec<_> = class.encode_utf16().chain([0]).collect();
    // SAFETY: both names are owned, terminated buffers. Successful CreateMutex
    // transfers its sole handle owner immediately; last error is sampled before
    // any other native call. HWND from FindWindow is borrowed, never destroyed.
    unsafe {
        let handle = CreateMutexW(None, true, PCWSTR(name.as_ptr()));
        let exists = GetLastError() == ERROR_ALREADY_EXISTS;
        let handle = handle.ok().map(own);
        if handle.is_some() && !exists {
            return Ok(handle);
        }
        let mut existing = None;
        for _ in 0..10 {
            existing = FindWindowW(PCWSTR(class.as_ptr()), None).ok();
            if existing.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if let Some(window) = existing {
            let mut accepted = 0;
            let sent = SendMessageTimeoutW(
                window,
                ALREADY_RUNNING,
                Default::default(),
                Default::default(),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                1500,
                Some(&mut accepted),
            );
            if sent.0 == 0 || accepted != 1 {
                let _ = ShowWindow(window, SW_RESTORE);
            }
            let _ = SetForegroundWindow(window);
        }
        Ok(None)
    }
}
fn wide(path: &Path) -> windows::core::Result<Vec<u16>> {
    let units: Vec<_> = path.as_os_str().encode_wide().collect();
    if units.contains(&0) {
        return Err(windows::Win32::Foundation::E_INVALIDARG.into());
    }
    Ok(units.into_iter().chain([0]).collect())
}
pub struct Shell {
    window: HWND,
    icon: Icon,
    added: Cell<bool>,
    pub maximized: Cell<bool>,
    pub taskbar_created: u32,
    placement: PathBuf,
}
impl Shell {
    pub fn new(
        window: HWND,
        placement: PathBuf,
        icon_path: Option<&Path>,
    ) -> windows::core::Result<Self> {
        let path = icon_path.map(wide).transpose()?;
        // SAFETY: a live terminated diagnostic file or the embedded native
        // resource ID 101 (MAKEINTRESOURCE, never dereferenced as a Rust pointer).
        // No LR_SHARED: both sources give an owned HICON for the existing guard.
        let icon = unsafe {
            let instance = if path.is_none() {
                Some(windows::Win32::Foundation::HINSTANCE(
                    windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.0,
                ))
            } else {
                None
            };
            LoadImageW(
                instance,
                path.as_ref()
                    .map_or(PCWSTR(101usize as *const u16), |path| PCWSTR(path.as_ptr())),
                IMAGE_ICON,
                GetSystemMetrics(SM_CXSMICON),
                GetSystemMetrics(SM_CYSMICON),
                if path.is_some() {
                    LR_LOADFROMFILE
                } else {
                    Default::default()
                },
            )
        }?;
        let icon = Icon(HICON(icon.0));
        // SAFETY: static terminated message name; query-only HWND access.
        let (taskbar_created, maximized) = unsafe {
            (
                RegisterWindowMessageW(w!("TaskbarCreated")),
                IsZoomed(window).as_bool(),
            )
        };
        if taskbar_created == 0 {
            return Err(windows::core::Error::from_thread());
        }
        Ok(Self {
            window,
            icon,
            added: Cell::new(false),
            maximized: Cell::new(maximized),
            taskbar_created,
            placement,
        })
    }
    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.window,
            uID: 1,
            ..Default::default()
        }
    }
    pub fn add(&self) -> bool {
        if self.added.get() {
            return true;
        }
        let mut data = self.data();
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY;
        data.hIcon = self.icon.0;
        let tip: Vec<_> = "PadMux — Steam Controller 2026 compatible"
            .encode_utf16()
            .collect();
        data.szTip[..tip.len()].copy_from_slice(&tip);
        // SAFETY: initialized correctly sized structure and live borrowed HWND/
        // owned HICON. The notification area copies the tooltip/icon values.
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool();
        self.added.set(added);
        added
    }
    pub fn taskbar_recreated(&self) {
        self.added.set(false);
        self.add();
    }
    pub fn remove(&self) {
        if self.added.replace(false) {
            // SAFETY: borrowed live HWND and this process's notification ID;
            // remove before the uniquely owned icon or host can be destroyed.
            let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &self.data()) };
        }
    }
    pub fn save(&self) -> windows::core::Result<()> {
        let mut placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        // SAFETY: live borrowed HWND and writable native placement structure.
        unsafe { GetWindowPlacement(self.window, &mut placement) }?;
        let rect = placement.rcNormalPosition;
        let maximized = self.maximized.get() || placement.showCmd == SW_SHOWMAXIMIZED.0 as u32;
        let value: Vec<_> = format!(
            "{},{},{},{},{}",
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            u8::from(maximized)
        )
        .encode_utf16()
        .chain([0])
        .collect();
        let path = wide(&self.placement)?;
        // SAFETY: static key names and owned terminated value/path stay live
        // until the synchronous native INI write returns; exact C++ format.
        unsafe {
            WritePrivateProfileStringW(
                w!("Window"),
                w!("Placement"),
                PCWSTR(value.as_ptr()),
                PCWSTR(path.as_ptr()),
            )
        }
    }
    pub fn hide(&self, english: bool) -> bool {
        if !self.add() {
            let (text, title) = if english {
                (
                    w!("Could not add an icon to the notification area."),
                    w!("Could not minimize to tray"),
                )
            } else {
                (
                    w!("通知領域にアイコンを作成できませんでした。"),
                    w!("トレイに格納できません"),
                )
            };
            // SAFETY: live HWND and static terminated strings for a native alert.
            unsafe {
                MessageBoxW(Some(self.window), text, title, MB_ICONERROR);
            }
            return false;
        }
        if let Err(error) = self.save() {
            eprintln!("window placement save: {error}");
        }
        // SAFETY: borrowed live HWND, native system-defined visibility flag.
        unsafe {
            let _ = ShowWindow(self.window, SW_HIDE);
        }
        true
    }
    pub fn restore(&self) {
        // SAFETY: query/show the borrowed live HWND on its owning GUI thread.
        unsafe {
            let _ = ShowWindow(
                self.window,
                if IsIconic(self.window).as_bool() {
                    SW_RESTORE
                } else {
                    SW_SHOW
                },
            );
            let _ = SetForegroundWindow(self.window);
        }
    }
    /// Native right-click menu; zero means canceled, otherwise a C++ command ID.
    pub fn menu(&self, english: bool) -> windows::core::Result<usize> {
        struct Menu(HMENU);
        impl Drop for Menu {
            fn drop(&mut self) {
                // SAFETY: owns this newly created menu exactly once.
                let _ = unsafe { DestroyMenu(self.0) };
            }
        }
        // SAFETY: native menu is uniquely guarded; labels are static terminated
        // strings. The OS validates HWND and copies menu labels synchronously.
        unsafe {
            let menu = Menu(CreatePopupMenu()?);
            AppendMenuW(
                menu.0,
                MF_STRING,
                SHOW,
                if english {
                    w!("Open PadMux")
                } else {
                    w!("PadMuxを開く")
                },
            )?;
            AppendMenuW(
                menu.0,
                MF_STRING,
                EXIT,
                if english { w!("Exit") } else { w!("終了") },
            )?;
            let mut cursor = POINT::default();
            GetCursorPos(&mut cursor)?;
            let _ = SetForegroundWindow(self.window);
            let command = TrackPopupMenu(
                menu.0,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                cursor.x,
                cursor.y,
                None,
                self.window,
                None,
            )
            .0 as usize;
            drop(menu);
            let _ = PostMessageW(
                Some(self.window),
                WM_NULL,
                Default::default(),
                Default::default(),
            );
            if command == SHOW {
                self.restore();
            }
            Ok(command)
        }
    }
}
impl Drop for Shell {
    fn drop(&mut self) {
        self.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mutex_blocks_duplicate_and_releases_at_shutdown() {
        let name = format!("PadMux_SingleInstance_Unit_{}", std::process::id());
        let owner = single_instance(&name, &name).unwrap().unwrap();
        assert!(single_instance(&name, &name).unwrap().is_none());
        drop(owner);
        assert!(single_instance(&name, &name).unwrap().is_some());
        assert!(single_instance("bad\0name", &name).is_err());
        assert!(single_instance(&name, "bad\0class").is_err());
    }
}
