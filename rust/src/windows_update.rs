//! Updater.cpp Windows boundary (MIT, copyright 2026 ro-ba).
//! Preserve ../../LICENSE / THIRD_PARTY_NOTICES.md.
use crate::{update, windows_config};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, process::CommandExt},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};
use windows::Win32::{
    Storage::FileSystem::*,
    System::LibraryLoader::GetModuleFileNameW,
    UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Networking::WinHttp::*,
        Security::Cryptography::*,
        System::SystemInformation::GetSystemDirectoryW,
        System::Threading::*,
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::*},
    },
};
pub const READY: u32 = WM_APP + 7;
pub const FAILED: u32 = WM_APP + 8;
type Result<T> = std::result::Result<T, String>;
pub const FILES: [&str; 9] = [
    "padmux.exe",
    "padmux-updater.exe",
    "LICENSE",
    "README.md",
    "README.ja.md",
    "THIRD_PARTY_NOTICES.md",
    "LICENSES/WebView2-LICENSE.txt",
    "LICENSES/WebView2-NOTICE.txt",
    "THIRD_PARTY_LICENSES.txt",
];
pub fn start(window: HWND, tag: &str) -> bool {
    if !update::valid_tag(tag) {
        return false;
    }
    use std::os::windows::ffi::OsStringExt;
    let mut executable = [0u16; 260];
    let mut temp_root = [0u16; 260];
    let mut temp_name = [0u16; 260];
    // SAFETY: bounded native outputs, return lengths checked before indexing.
    // GetTempFileName reserves a name; preserve C++ delete→mkdir transition.
    let (size, temp_length) = unsafe {
        (
            GetModuleFileNameW(None, &mut executable),
            GetTempPathW(Some(&mut temp_root)),
        )
    };
    if size == 0
        || size as usize >= executable.len()
        || temp_length == 0
        || temp_length as usize >= temp_root.len()
    {
        return false;
    }
    let executable =
        std::path::PathBuf::from(std::ffi::OsString::from_wide(&executable[..size as usize]));
    let Some(install) = executable.parent() else {
        return false;
    };
    // SAFETY: live terminated temp path/prefix and writable exact native array.
    unsafe {
        if GetTempFileNameW(PCWSTR(temp_root.as_ptr()), w!("pmu"), 0, &mut temp_name) == 0
            || DeleteFileW(PCWSTR(temp_name.as_ptr())).is_err()
            || CreateDirectoryW(PCWSTR(temp_name.as_ptr()), None).is_err()
        {
            return false;
        }
    }
    let length = temp_name
        .iter()
        .position(|&u| u == 0)
        .unwrap_or(temp_name.len());
    let temporary = std::path::PathBuf::from(std::ffi::OsString::from_wide(&temp_name[..length]));
    let updater = temporary.join("padmux-updater.exe");
    if windows_config::copy_file(&install.join("padmux-updater.exe"), &updater, true).is_err() {
        let _ = fs::remove_dir(&temporary);
        return false;
    }
    let mut parameters: Vec<_> = format!("{} {} \"", std::process::id(), window.0 as usize)
        .encode_utf16()
        .collect();
    parameters.extend(install.as_os_str().encode_wide());
    parameters.extend("\" \"".encode_utf16());
    parameters.extend(temporary.as_os_str().encode_wide());
    parameters.extend("\" ".encode_utf16());
    parameters.extend(tag.encode_utf16().chain([0]));
    let file = match windows_config::wide(&updater) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let directory = match windows_config::wide(&temporary) {
        Ok(path) => path,
        Err(_) => return false,
    };
    let probe = install.join(format!(".padmux-update-{}", std::process::id()));
    let writable = OpenOptions::new()
        .write(true)
        .access_mode(GENERIC_WRITE.0 | DELETE.0)
        .create_new(true)
        .share_mode(0)
        .attributes(FILE_ATTRIBUTE_TEMPORARY.0)
        .custom_flags(FILE_FLAG_DELETE_ON_CLOSE.0)
        .open(probe)
        .is_ok();
    let mut launch = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        hwnd: window,
        lpVerb: if writable { w!("open") } else { w!("runas") },
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        lpDirectory: PCWSTR(directory.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: all native strings remain alive during launch; use Windows' UAC
    // verb only when the original delete-on-close write probe fails. Transfer
    // the optional returned process handle to its sole owner immediately.
    if unsafe { ShellExecuteExW(&mut launch) }.is_err() {
        let _ = fs::remove_file(updater);
        let _ = fs::remove_dir(temporary);
        return false;
    }
    if !launch.hProcess.is_invalid() {
        drop(crate::windows_hid::own(launch.hProcess));
    }
    true
}
struct Internet(*mut std::ffi::c_void);
impl Internet {
    fn new(handle: *mut std::ffi::c_void) -> Result<Self> {
        if handle.is_null() {
            Err(windows::core::Error::from_thread().to_string())
        } else {
            Ok(Self(handle))
        }
    }
}
impl Drop for Internet {
    fn drop(&mut self) {
        // SAFETY: sole owner of a successful WinHTTP allocation. Reverse local
        // drop order releases request/connection before their session.
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}
fn fetch(host: &str, path: &str, limit: usize) -> Result<Vec<u8>> {
    if host.contains('\0') || path.contains('\0') {
        return Err("NUL in URL".into());
    }
    let host: Vec<_> = host.encode_utf16().chain([0]).collect();
    let path: Vec<_> = path.encode_utf16().chain([0]).collect();
    // SAFETY: static/owned terminated names, default synchronous session. Every
    // successful handle immediately acquires one owner; no async callback or
    // retained Rust buffer is involved. Preserve C++ proxy/timeouts/HTTPS flags.
    unsafe {
        let session = Internet::new(WinHttpOpen(
            w!("PadMux-Updater/1.0"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            None,
            None,
            0,
        ))?;
        WinHttpSetTimeouts(session.0, 5000, 5000, 15000, 15000).map_err(|e| e.to_string())?;
        let connection = Internet::new(WinHttpConnect(
            session.0,
            PCWSTR(host.as_ptr()),
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ))?;
        let request = Internet::new(WinHttpOpenRequest(
            connection.0,
            w!("GET"),
            PCWSTR(path.as_ptr()),
            None,
            None,
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        let headers: Vec<_> = "Accept: application/vnd.github+json\r\n"
            .encode_utf16()
            .collect();
        WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0).map_err(|e| e.to_string())?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(|e| e.to_string())?;
        let mut status = 0u32;
        let mut length = std::mem::size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            None,
            Some((&mut status as *mut u32).cast()),
            &mut length,
            std::ptr::null_mut(),
        )
        .map_err(|e| e.to_string())?;
        if status != 200 {
            return Err(format!("HTTP {status}"));
        }
        let mut output = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let mut read = 0u32;
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut read,
            )
            .map_err(|e| e.to_string())?;
            let read = read as usize;
            if read == 0 {
                return Ok(output);
            }
            if read > chunk.len() || read > limit || output.len() > limit - read {
                return Err("download exceeds limit".into());
            }
            output.extend_from_slice(&chunk[..read]);
        }
    }
}
struct Algorithm(BCRYPT_ALG_HANDLE);
impl Drop for Algorithm {
    fn drop(&mut self) {
        // SAFETY: unique successful provider allocation; synchronous hash has
        // already finished before its provider is released.
        let _ = unsafe { BCryptCloseAlgorithmProvider(self.0, 0) };
    }
}
pub fn sha256(bytes: &[u8]) -> Result<String> {
    if bytes.len() > u32::MAX as usize {
        return Err("hash input exceeds DWORD".into());
    }
    let mut algorithm = BCRYPT_ALG_HANDLE::default();
    let mut value = [0u8; 32];
    // SAFETY: writable native handle output; guard owns successful allocation.
    // BCryptHash takes bounded Rust slices and copies the 32-byte SHA-256
    // output synchronously. Its native convenience call replaces the same
    // CreateHash/HashData/FinishHash/DestroyHash sequence on Windows 10+.
    unsafe {
        BCryptOpenAlgorithmProvider(
            &mut algorithm,
            BCRYPT_SHA256_ALGORITHM,
            None,
            Default::default(),
        )
        .ok()
        .map_err(|e| e.to_string())?;
        let algorithm = Algorithm(algorithm);
        BCryptHash(algorithm.0, None, bytes, &mut value)
            .ok()
            .map_err(|e| e.to_string())?;
    }
    Ok(value.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn extract(archive: &Path, destination: &Path) -> Result<()> {
    let mut system = [0u16; 260];
    // SAFETY: bounded writable UTF-16 buffer. Return length is checked before
    // constructing a path; never resolve tar through PATH or the working dir.
    let length = unsafe { GetSystemDirectoryW(Some(&mut system)) } as usize;
    if length == 0 || length >= system.len() {
        return Err("system directory unavailable".into());
    }
    use std::os::windows::ffi::OsStringExt;
    let tar =
        std::path::PathBuf::from(std::ffi::OsString::from_wide(&system[..length])).join("tar.exe");
    let mut child = Command::new(tar)
        .args(["-xf"])
        .arg(archive)
        .arg("-C")
        .arg(destination)
        .current_dir(destination)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("tar exit {status}"))
                }
            }
            Ok(None) if start.elapsed() < Duration::from_secs(30) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            _ => {
                // C++ terminates only this owned extraction child on timeout.
                let _ = child.kill();
                let _ = child.wait();
                return Err("tar timeout or wait failure".into());
            }
        }
    }
}
fn read_package(archive: &Path) -> Result<Vec<u8>> {
    let mut input = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let length = input.metadata().map_err(|e| e.to_string())?.len();
    if length == 0 || length > update::MAX_PACKAGE as u64 {
        return Err("archive must be 1..100 MiB".into());
    }
    let mut bytes = vec![0; length as usize];
    input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}
pub fn verify_archive(archive: &Path, expected: &str) -> i32 {
    if expected.len() != 64 {
        return 2;
    }
    let bytes = match read_package(archive) {
        Ok(bytes) => bytes,
        Err(_) => return 2,
    };
    if sha256(&bytes).as_deref() != Ok(expected) {
        return 2;
    }
    let Some(parent) = archive.parent() else {
        return 3;
    };
    let destination = parent.join("padmux-verify-extracted");
    if fs::create_dir_all(&destination).is_err() {
        return 3;
    }
    let extracted = extract(archive, &destination).is_ok();
    let package = destination.join("padmux-windows-x64");
    let okay = extracted
        && package.join("padmux.exe").is_file()
        && package.join("padmux-updater.exe").is_file();
    let _ = fs::remove_dir_all(destination);
    if okay {
        0
    } else if extracted {
        4
    } else {
        3
    }
}
pub fn apply_package(package: &Path, install: &Path, temporary: &Path) -> Result<()> {
    let backup = temporary.join("previous.exe");
    windows_config::copy_file(&install.join("padmux.exe"), &backup, true)?;
    let mut staged = Vec::new();
    for name in FILES {
        let source = package.join(name);
        let target = install.join(name);
        let mut next = target.as_os_str().to_os_string();
        next.push(".padmux-new");
        let next = std::path::PathBuf::from(next);
        let _ = fs::remove_file(&next);
        let result = fs::create_dir_all(target.parent().ok_or("install folder missing")?)
            .map_err(|e| e.to_string())
            .and_then(|()| {
                if !source.is_file() {
                    return Err(format!("missing package file: {name}"));
                }
                windows_config::copy(&source, &next)
            });
        if let Err(error) = result {
            for path in staged {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
        staged.push(next);
    }
    for (i, next) in staged.iter().enumerate() {
        let target = install.join(FILES[i]);
        let source = windows_config::wide(next).map_err(|e| e.to_string())?;
        let destination = windows_config::wide(&target).map_err(|e| e.to_string())?;
        // SAFETY: terminated paths stay live for the synchronous rename. Exact
        // C++ replace semantics; files staged before any installation change.
        if let Err(error) = unsafe {
            windows::Win32::Storage::FileSystem::MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(destination.as_ptr()),
                windows::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING,
            )
        } {
            for path in &staged[i..] {
                let _ = fs::remove_file(path);
            }
            let _ = windows_config::copy(&backup, &install.join("padmux.exe"));
            return Err(error.to_string());
        }
    }
    Ok(())
}
fn write_package(archive: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(archive)
        .map_err(|e| e.to_string())?;
    if file.write(bytes).map_err(|e| e.to_string())? != bytes.len() {
        return Err("partial package write".into());
    }
    Ok(())
}
fn notify(window: HWND, failure: update::Failure) {
    // SAFETY: native window validity is checked; only integer payload/message
    // IDs cross process boundaries, never borrowed pointers or Rust data.
    unsafe {
        if IsWindow(Some(window)).as_bool() {
            let _ = PostMessageW(Some(window), FAILED, WPARAM(failure as usize), LPARAM(0));
        }
    }
}
fn perform(parent_id: u32, window: HWND, install: &Path, temporary: &Path, tag: &str) -> i32 {
    let mut window_process = 0;
    // SAFETY: output DWORD lives during query of a borrowed native HWND. Native
    // creator PID validation is the original updater trust boundary.
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut window_process));
    }
    if !update::valid_tag(tag) || parent_id == 0 || window_process != parent_id {
        return 1;
    }
    // SAFETY: synchronization-only query, no inheritance. The existing native
    // owner closes the successful process handle on every failure/success path.
    let parent = match unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, parent_id) } {
        Ok(handle) => crate::windows_hid::own(handle),
        Err(_) => return 1,
    };
    let release = match fetch(
        "api.github.com",
        "/repos/ro-ba/padmux/releases/latest",
        update::MAX_RELEASE_JSON,
    ) {
        Ok(bytes) => bytes,
        Err(_) => {
            notify(window, update::Failure::Network);
            return 1;
        }
    };
    let expected = match update::release_digest(&release, tag) {
        Some(digest) => digest,
        None => {
            notify(window, update::Failure::Release);
            return 1;
        }
    };
    let package = match fetch(
        "github.com",
        &format!("/ro-ba/padmux/releases/download/{tag}/{}", update::ASSET),
        update::MAX_PACKAGE,
    ) {
        Ok(bytes) => bytes,
        Err(_) => {
            notify(window, update::Failure::Download);
            return 1;
        }
    };
    if sha256(&package).as_deref() != Ok(expected.as_str()) {
        notify(window, update::Failure::Checksum);
        return 1;
    }
    let archive = temporary.join("release.zip");
    let extracted = temporary.join("extracted");
    if fs::create_dir_all(&extracted).is_err()
        || write_package(&archive, &package).is_err()
        || extract(&archive, &extracted).is_err()
    {
        notify(window, update::Failure::Extract);
        return 1;
    }
    let extracted = extracted.join("padmux-windows-x64");
    if !extracted.join("padmux.exe").is_file() || !extracted.join("padmux-updater.exe").is_file() {
        notify(window, update::Failure::Extract);
        return 1;
    }
    let mut acknowledged = 0;
    // SAFETY: integer IPC only, synchronous timeout. Keep the owned parent
    // handle until its actual exit, before touching the installed executable.
    unsafe {
        if SendMessageTimeoutW(
            window,
            READY,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            15000,
            Some(&mut acknowledged),
        )
        .0 == 0
            || acknowledged != 1
            || WaitForSingleObject(crate::windows_hid::raw(&parent), 30000) != WAIT_OBJECT_0
        {
            return 1;
        }
    }
    drop(parent);
    if apply_package(&extracted, install, temporary).is_err() {
        // SAFETY: static terminated strings, no owner window after parent exits.
        unsafe {
            MessageBoxW(
                None,
                w!("更新ファイルを配置できませんでした。padmux.exeを手動で起動してください。"),
                w!("PadMuxの更新に失敗"),
                MB_OK | MB_ICONERROR,
            );
        }
        return 1;
    }
    let executable = match windows_config::wide(&install.join("padmux.exe")) {
        Ok(path) => path,
        Err(_) => return 1,
    };
    let directory = match windows_config::wide(install) {
        Ok(path) => path,
        Err(_) => return 1,
    };
    // SAFETY: terminated paths live through native launch; result is Shell's
    // status/HINSTANCE sentinel, not an owned process handle to close.
    let launched = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(executable.as_ptr()),
            None,
            PCWSTR(directory.as_ptr()),
            SW_SHOWNORMAL,
        )
    };
    if launched.0 as isize <= 32 {
        // SAFETY: static terminated text and native unowned alert, as above.
        unsafe {
            MessageBoxW(None,w!("更新は完了しましたが、再起動できませんでした。padmux.exeを手動で起動してください。"),w!("PadMuxを起動できません"),MB_OK|MB_ICONWARNING);
        }
        return 1;
    }
    0
}
// Like windows_app/windows_icons, resolve CRT symbols through Rust's chosen
// runtime. Forcing ucrt.lib would mix dynamic imports with crt-static.
unsafe extern "C" {
    fn wcstoul(text: *const u16, end: *mut *mut u16, radix: i32) -> u32;
    fn _wcstoui64(text: *const u16, end: *mut *mut u16, radix: i32) -> u64;
}
pub fn run(arguments: &[std::ffi::OsString]) -> i32 {
    if arguments.len() == 2 && arguments[0] == "--validate-tag" {
        return i32::from(!arguments[1].to_str().is_some_and(update::valid_tag));
    }
    if arguments.len() == 3 && arguments[0] == "--verify-archive" {
        let Some(digest) = arguments[2].to_str() else {
            return 1;
        };
        if !digest.chars().all(|c| ('0'..='f').contains(&c)) {
            return 1;
        }
        return verify_archive(Path::new(&arguments[1]), digest);
    }
    if arguments.len() != 5 {
        return 1;
    }
    let Some(tag) = arguments[4].to_str() else {
        return 1;
    };
    let parent: Vec<_> = arguments[0].encode_wide().chain([0]).collect();
    let window: Vec<_> = arguments[1].encode_wide().chain([0]).collect();
    // SAFETY: terminated buffers stay owned during CRT conversion, end pointer
    // intentionally unused. Match C++ leading whitespace/sign, prefix parsing
    // and distinct DWORD/u64 overflow instead of changing argv acceptance.
    let (parent, window) = unsafe {
        (
            wcstoul(parent.as_ptr(), std::ptr::null_mut(), 10),
            _wcstoui64(window.as_ptr(), std::ptr::null_mut(), 10),
        )
    };
    perform(
        parent,
        HWND(window as usize as *mut _),
        Path::new(&arguments[2]),
        Path::new(&arguments[3]),
        tag,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "padmux-updater-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn native_hash_and_archive_verification_preserve_cpp_contract() {
        assert_eq!(
            sha256(b"").unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let root = directory();
        let input = root.join("input.bin");
        let output = root.join("hash.txt");
        let bytes: Vec<_> = (0..100_000).map(|i| (i % 256) as u8).collect();
        fs::write(&input, &bytes).unwrap();
        if let Some(exe) = std::env::var_os("PADMUX_CONFIG_REFERENCE") {
            assert!(Command::new(exe)
                .arg("--sha256")
                .arg(&input)
                .arg(&output)
                .status()
                .unwrap()
                .success());
            assert_eq!(sha256(&bytes).unwrap(), fs::read_to_string(output).unwrap());
        }
        let package = root.join("padmux-windows-x64");
        fs::create_dir(&package).unwrap();
        fs::write(
            package.join("padmux.exe"),
            b"public test executable placeholder",
        )
        .unwrap();
        fs::write(
            package.join("padmux-updater.exe"),
            b"public test updater placeholder",
        )
        .unwrap();
        let archive = root.join("release.zip");
        let tar =
            std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap()).join("System32/tar.exe");
        let make_zip = |archive: &Path| {
            let output = Command::new(&tar)
                .args(["-a", "-cf"])
                .arg(archive)
                .arg("-C")
                .arg(&root)
                .arg("padmux-windows-x64")
                .creation_flags(0x08000000)
                .output()
                .unwrap();
            assert!(output.status.success(), "tar ZIP creation: {output:?}");
        };
        make_zip(&archive);
        let expected = sha256(&read_package(&archive).unwrap()).unwrap();
        assert_eq!(verify_archive(&archive, &expected), 0);
        assert!(!root.join("padmux-verify-extracted").exists());
        assert_eq!(verify_archive(&archive, &"0".repeat(64)), 2);
        assert_eq!(verify_archive(&archive, "bad"), 2);
        fs::remove_file(package.join("padmux-updater.exe")).unwrap();
        let incomplete = root.join("incomplete.zip");
        make_zip(&incomplete);
        assert_eq!(
            verify_archive(
                &incomplete,
                &sha256(&read_package(&incomplete).unwrap()).unwrap()
            ),
            4
        );
        let broken = root.join("broken.zip");
        write_package(&broken, b"invalid archive").unwrap();
        assert!(write_package(&broken, b"replacement").is_err());
        assert_eq!(fs::read(&broken).unwrap(), b"invalid archive");
        assert_eq!(
            verify_archive(&broken, &sha256(b"invalid archive").unwrap()),
            3
        );
        assert_eq!(run(&[]), 1);
        assert_eq!(run(&["--validate-tag".into(), "v1.0".into()]), 1);
        assert_eq!(run(&["--validate-tag".into(), "v1.0.0".into()]), 0);
        assert_eq!(
            run(&[
                "0".into(),
                "0".into(),
                "unused".into(),
                "unused".into(),
                "v1.0.0".into()
            ]),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn staged_install_and_failure_restore_only_touch_owned_temp_files() {
        let root = directory();
        let package = root.join("package");
        for name in FILES {
            let file = package.join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, format!("new {name}")).unwrap();
        }
        for scenario in 0..3 {
            let install = root.join(format!("install{scenario}"));
            let temporary = root.join(format!("temp{scenario}"));
            fs::create_dir(&install).unwrap();
            fs::create_dir(&temporary).unwrap();
            for name in FILES {
                let file = install.join(name);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(file, format!("old {name}")).unwrap();
            }
            let lock = if scenario == 2 {
                Some(
                    OpenOptions::new()
                        .read(true)
                        .share_mode(1)
                        .open(install.join("padmux-updater.exe"))
                        .unwrap(),
                )
            } else {
                None
            };
            if scenario == 1 {
                fs::remove_file(package.join("LICENSE")).unwrap();
            }
            let result = apply_package(&package, &install, &temporary);
            assert_eq!(result.is_ok(), scenario == 0);
            assert_eq!(
                fs::read_to_string(install.join("padmux.exe")).unwrap(),
                if scenario == 0 {
                    "new padmux.exe"
                } else {
                    "old padmux.exe"
                }
            );
            for name in FILES {
                assert_eq!(
                    fs::read_to_string(install.join(name)).unwrap(),
                    format!("{} {name}", if scenario == 0 { "new" } else { "old" })
                );
                let mut next = install.join(name).as_os_str().to_os_string();
                next.push(".padmux-new");
                assert!(!Path::new(&next).exists());
            }
            assert_eq!(
                fs::read_to_string(temporary.join("previous.exe")).unwrap(),
                "old padmux.exe"
            );
            drop(lock);
            if let Some(reference) = std::env::var_os("PADMUX_CONFIG_REFERENCE") {
                let cpp_install = root.join(format!("cpp-install{scenario}"));
                let cpp_temp = root.join(format!("cpp-temp{scenario}"));
                fs::create_dir(&cpp_install).unwrap();
                fs::create_dir(&cpp_temp).unwrap();
                for name in FILES {
                    let file = cpp_install.join(name);
                    fs::create_dir_all(file.parent().unwrap()).unwrap();
                    fs::write(file, format!("old {name}")).unwrap();
                }
                let cpp_lock = if scenario == 2 {
                    Some(
                        OpenOptions::new()
                            .read(true)
                            .share_mode(1)
                            .open(cpp_install.join("padmux-updater.exe"))
                            .unwrap(),
                    )
                } else {
                    None
                };
                let status = std::process::Command::new(reference)
                    .arg("--apply-package")
                    .arg(&package)
                    .arg(&cpp_install)
                    .arg(&cpp_temp)
                    .status()
                    .unwrap();
                assert!([Some(0), Some(1)].contains(&status.code()));
                assert_eq!(
                    status.success(),
                    result.is_ok(),
                    "C++ apply scenario {scenario}"
                );
                for name in FILES
                    .iter()
                    .filter(|name| **name != "THIRD_PARTY_LICENSES.txt")
                {
                    assert_eq!(
                        fs::read(cpp_install.join(name)).unwrap(),
                        fs::read(install.join(name)).unwrap()
                    );
                    let mut next = cpp_install.join(name).into_os_string();
                    next.push(".padmux-new");
                    assert!(!Path::new(&next).exists());
                }
                assert_eq!(
                    fs::read(cpp_temp.join("previous.exe")).unwrap(),
                    fs::read(temporary.join("previous.exe")).unwrap()
                );
                drop(cpp_lock);
            }
            if scenario == 1 {
                fs::write(package.join("LICENSE"), "new LICENSE").unwrap();
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
}
