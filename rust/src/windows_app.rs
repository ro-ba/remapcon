//! Windows foreground query and executable picker from Remapcon (MIT).
//! Preserve ../../LICENSE: Dylan Deverill and ro-ba; see THIRD_PARTY_NOTICES.md.
use crate::app::ForegroundCache;
use std::{
    mem::size_of,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    time::Duration,
};
use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, HWND},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::{
            Controls::Dialogs::{
                CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_EXPLORER,
                OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
                OPENFILENAMEW,
            },
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    },
};

// Use the same CRT comparison as C++, including its locale and embedded-NUL
// behavior. Unicode lowercase/Win32 ordinal comparison is not equivalent.
unsafe extern "C" {
    fn _wcsicmp(left: *const u16, right: *const u16) -> i32;
}
pub(crate) fn equal(left: &[u16], right: &[u16]) -> bool {
    debug_assert!(left.contains(&0) && right.contains(&0));
    // SAFETY: private callers provide live, NUL-terminated UTF-16 arrays;
    // _wcsicmp only reads through their terminator. FFI retains C++ semantics.
    unsafe { _wcsicmp(left.as_ptr(), right.as_ptr()) == 0 }
}
pub(crate) fn terminated(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}
pub fn equal_text(left: &str, right: &str) -> bool {
    equal(&terminated(left), &terminated(right))
}
pub fn equal_paths(left: &Path, right: &Path) -> bool {
    let left: Vec<_> = left.as_os_str().encode_wide().chain([0]).collect();
    let right: Vec<_> = right.as_os_str().encode_wide().chain([0]).collect();
    equal(&left, &right)
}
pub fn main_foreground(window: usize) -> bool {
    // SAFETY: query takes no pointers; HWND is compared as an opaque value.
    window != 0 && unsafe { GetForegroundWindow() }.0 as usize == window
}
pub fn wake_ui(window: usize, message: u32) {
    // SAFETY: OS validates the borrowed window handle. No pointers are carried
    // in this wake notification; owned data is delivered via the Rust channel.
    let _ = unsafe {
        windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(HWND(window as *mut _)),
            message,
            Default::default(),
            Default::default(),
        )
    };
}
pub fn target_matches(selected: &str, process_path: &[u16]) -> bool {
    if selected.is_empty() || process_path.contains(&0) {
        return false;
    }
    let name = if selected.contains(['\\', '/']) {
        process_path
    } else {
        &process_path[process_path
            .iter()
            .rposition(|&c| c == b'\\' as u16)
            .map_or(0, |i| i + 1)..]
    };
    let path: Vec<_> = name.iter().copied().chain([0]).collect();
    equal(&path, &terminated(selected))
}
pub(crate) fn process_path(process_id: u32, capacity: usize) -> Option<Vec<u16>> {
    if process_id == 0 {
        return None;
    }
    // SAFETY: process ID is a value and no pointers are passed. Query-only
    // access matches C++; failed/inaccessible processes mean no match.
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }.ok()?;
    let mut path = vec![0u16; capacity];
    let mut length = path.len() as u32;
    // SAFETY: length describes the writable array; the owned handle remains
    // open until the synchronous query ends and is closed on both outcomes.
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        )
    };
    // SAFETY: this is the unique handle obtained above, closed exactly once.
    let _ = unsafe { CloseHandle(process) };
    result.ok()?;
    path.truncate(length as usize);
    Some(path)
}

#[derive(Default)]
pub struct ForegroundMonitor {
    cache: ForegroundCache,
}
impl ForegroundMonitor {
    pub fn check(&mut self, target: &str, target_version: u64, now: Duration) -> bool {
        // SAFETY: the foreground query takes no pointers or owned resources.
        let window = unsafe { GetForegroundWindow() };
        self.cache
            .check(window.0 as usize, target_version, now, || {
                if target.is_empty() {
                    return false;
                }
                let mut id = 0;
                // SAFETY: id is valid output storage; the OS validates the HWND,
                // including windows that disappear between these calls.
                unsafe { GetWindowThreadProcessId(window, Some(&mut id)) };
                process_path(id, 520).is_some_and(|path| target_matches(target, &path))
            })
    }
}

pub fn choose_target(owner: HWND) -> Result<Option<String>, String> {
    let Some(path) = choose_file(
        owner,
        false,
        "自動有効化するアプリの実行ファイルを選択",
        "実行ファイル (*.exe)\0*.exe\0",
        "",
    )?
    else {
        return Ok(None);
    };
    let extension = path
        .iter()
        .rposition(|&c| c == b'.' as u16)
        .ok_or("select an .exe file")?;
    if !equal(&path[extension..], &terminated(".exe")) {
        return Err("select an .exe file".into());
    }
    String::from_utf16(&path[..path.len() - 1])
        .map(Some)
        .map_err(|e| e.to_string())
}
pub fn choose_settings(owner: HWND, save: bool) -> Result<Option<PathBuf>, String> {
    let path = choose_file(
        owner,
        save,
        if save {
            "設定をエクスポート"
        } else {
            "設定をインポート"
        },
        "JSON 設定 (*.json)\0*.json\0",
        if save { "padmux-settings.json" } else { "" },
    )?;
    Ok(path.map(|path| PathBuf::from(std::ffi::OsString::from_wide(&path[..path.len() - 1]))))
}
fn choose_file(
    owner: HWND,
    save: bool,
    title: &str,
    filter: &str,
    initial: &str,
) -> Result<Option<Vec<u16>>, String> {
    let filter = terminated(filter);
    let title = terminated(title);
    let mut path = [0u16; 520];
    let initial = terminated(initial);
    if initial.len() > path.len() {
        return Err("file dialog initial name is too long".into());
    }
    path[..initial.len()].copy_from_slice(&initial);
    let mut dialog = OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(path.as_mut_ptr()),
        nMaxFile: path.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        lpstrDefExt: if save { w!("json") } else { PCWSTR::null() },
        Flags: OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST
            },
        ..Default::default()
    };
    // SAFETY: all buffers and the dialog structure remain live during this
    // synchronous native dialog; nMaxFile bounds the writable path. No hooks.
    if !unsafe {
        if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        }
    }
    .as_bool()
    {
        // SAFETY: this returns thread-local status without pointer arguments.
        let error = unsafe { CommDlgExtendedError() }.0;
        return if error == 0 {
            Ok(None)
        } else {
            Err(format!("file dialog error {error}"))
        };
    }
    let length = path
        .iter()
        .position(|&c| c == 0)
        .ok_or("unterminated selected path")?;
    Ok(Some(path[..=length].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_query_and_cpp_filename_fullpath_comparison() {
        // SAFETY: ID query has no pointer arguments or side effects.
        let id = unsafe { windows::Win32::System::Threading::GetCurrentProcessId() };
        let path = process_path(id, 520).unwrap();
        assert!(target_matches(&String::from_utf16(&path).unwrap(), &path));
        let path: Vec<_> = "C:\\Games\\Example.EXE".encode_utf16().collect();
        assert!(target_matches("example.exe", &path));
        assert!(target_matches("c:\\games\\example.exe", &path));
        assert!(!target_matches("C:/Games/Example.EXE", &path));
        assert!(!target_matches("Other.exe", &path));
        assert!(!target_matches("", &path));
        assert!(process_path(0, 520).is_none());
    }
}
