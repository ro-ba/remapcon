//! TargetIcon.cpp translation (Remapcon MIT, copyright (c) 2026 ro-ba).
//! Preserve ../../LICENSE and THIRD_PARTY_NOTICES.md.
use crate::{windows_app, windows_config, windows_hid};
use std::{
    fs::OpenOptions,
    io::Read,
    mem::size_of,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{E_FAIL, HGLOBAL},
        Graphics::Imaging::*,
        Security::Cryptography::{
            CryptBinaryToStringW, CRYPT_STRING, CRYPT_STRING_BASE64, CRYPT_STRING_NOCRLF,
        },
        Storage::FileSystem::{
            GetFileAttributesExW, GetFileAttributesW, GetFileExInfoStandard, SearchPathW,
            INVALID_FILE_ATTRIBUTES, WIN32_FILE_ATTRIBUTE_DATA,
        },
        System::{
            Com::{
                CoCreateInstance, StructuredStorage::CreateStreamOnHGlobal, CLSCTX_INPROC_SERVER,
                STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET,
            },
            Diagnostics::ToolHelp::*,
        },
        UI::{
            Shell::{ExtractIconExW, SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON},
            WindowsAndMessaging::{DestroyIcon, HICON},
        },
    },
};
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
unsafe extern "C" {
    fn towlower(ch: u16) -> u16;
}
fn lower(ch: u16) -> u16 {
    // SAFETY: Microsoft CRT wint_t is u16; value-only FFI uses the same default
    // locale and UTF-16 code-unit mapping as std::towlower in the reference.
    unsafe { towlower(ch) }
}
fn cache_path(target: &str, directory: &Path) -> PathBuf {
    let hash = target
        .encode_utf16()
        .fold(14695981039346656037u64, |hash, ch| {
            (hash ^ u64::from(lower(ch))).wrapping_mul(1099511628211)
        });
    directory.join(format!("{hash:016x}.png"))
}
fn metadata_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".meta");
    PathBuf::from(name)
}
fn read_file(path: &Path, min: u64, max: u64) -> Option<Vec<u8>> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .ok()?;
    let size = file.metadata().ok()?.len();
    if !(min..=max).contains(&size) {
        return None;
    }
    let mut bytes = vec![0; size as usize];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}
fn resolve(target: &str) -> Option<Vec<u16>> {
    let target = windows_app::terminated(target);
    // SAFETY: owned terminated strings/bounded outputs remain live; snapshots
    // transfer to the existing OwnedHandle guard before fallible operations.
    unsafe {
        if target.contains(&(b'\\' as u16)) || target.contains(&(b'/' as u16)) {
            return (GetFileAttributesW(PCWSTR(target.as_ptr())) != INVALID_FILE_ATTRIBUTES)
                .then_some(target);
        }
        if let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let snapshot = windows_hid::own(snapshot);
            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut found = Process32FirstW(windows_hid::raw(&snapshot), &mut entry).is_ok();
            while found {
                // The OS supplies a terminated MAX_PATH executable name.
                if windows_app::equal(&entry.szExeFile, &target) {
                    if let Some(mut path) = windows_app::process_path(entry.th32ProcessID, 32768) {
                        path.push(0);
                        if GetFileAttributesW(PCWSTR(path.as_ptr())) != INVALID_FILE_ATTRIBUTES {
                            return Some(path);
                        }
                    }
                }
                found = Process32NextW(windows_hid::raw(&snapshot), &mut entry).is_ok();
            }
        }
        let mut path = [0u16; 520];
        let length = SearchPathW(
            PCWSTR::null(),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            Some(&mut path),
            None,
        ) as usize;
        (length > 0 && length < path.len()).then(|| path[..=length].to_vec())
    }
}
fn fingerprint(executable: &[u16]) -> Option<Vec<u8>> {
    let mut attributes = WIN32_FILE_ATTRIBUTE_DATA::default();
    // SAFETY: terminated owned path; writable structure has the exact Win32 layout.
    unsafe {
        GetFileAttributesExW(
            PCWSTR(executable.as_ptr()),
            GetFileExInfoStandard,
            (&mut attributes as *mut WIN32_FILE_ATTRIBUTE_DATA).cast(),
        )
    }
    .ok()?;
    let suffix = format!(
        "\n{}:{}:{}:{}",
        attributes.ftLastWriteTime.dwHighDateTime,
        attributes.ftLastWriteTime.dwLowDateTime,
        attributes.nFileSizeHigh,
        attributes.nFileSizeLow
    );
    Some(
        executable[..executable.len() - 1]
            .iter()
            .copied()
            .map(lower)
            .chain(suffix.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect(),
    )
}
pub(crate) struct Icon(pub(crate) HICON);
impl Drop for Icon {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: uniquely owns an extracted/Shell/non-shared LoadImage
            // icon; no shared LoadIcon handle enters here. WIC copies pixels.
            let _ = unsafe { DestroyIcon(self.0) };
        }
    }
}
fn encode_icon(executable: &[u16]) -> windows::core::Result<Vec<u8>> {
    // SAFETY: live terminated path, initialized outputs and uniquely owned
    // HICONs. COM interfaces are created/used/released on this thread; absent
    // COM initialization returns an error. Every WIC buffer is SDK-owned and
    // Read is bounded by the checked STATSTG length of our in-memory stream.
    unsafe {
        let mut large = HICON::default();
        let mut small = HICON::default();
        let count = ExtractIconExW(
            PCWSTR(executable.as_ptr()),
            0,
            Some(&mut large),
            Some(&mut small),
            1,
        );
        drop(Icon(small));
        let mut icon = Icon(large);
        if count == 0 || count == u32::MAX || icon.0.is_invalid() {
            drop(icon);
            let mut info = SHFILEINFOW::default();
            if SHGetFileInfoW(
                PCWSTR(executable.as_ptr()),
                Default::default(),
                Some(&mut info),
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            ) == 0
                || info.hIcon.is_invalid()
            {
                drop(Icon(info.hIcon));
                return Err(E_FAIL.into());
            }
            icon = Icon(info.hIcon);
        }
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let bitmap = factory.CreateBitmapFromHICON(icon.0)?;
        drop(icon);
        let stream = CreateStreamOnHGlobal(HGLOBAL::default(), true)?;
        let encoder = factory.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        let mut options = None;
        encoder.CreateNewFrame(&mut frame, &mut options)?;
        let frame = frame.ok_or_else(|| windows::core::Error::from(E_FAIL))?;
        frame.Initialize(options.as_ref())?;
        let mut width = 0;
        let mut height = 0;
        bitmap.GetSize(&mut width, &mut height)?;
        frame.SetSize(width, height)?;
        let mut format = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut format)?;
        frame.WriteSource(&bitmap, std::ptr::null())?;
        frame.Commit()?;
        encoder.Commit()?;
        let mut stats = STATSTG::default();
        stream.Stat(&mut stats, STATFLAG_NONAME)?;
        if !(1..=1024 * 1024).contains(&stats.cbSize) {
            return Err(E_FAIL.into());
        }
        let mut bytes = vec![0u8; stats.cbSize as usize];
        let mut read = 0;
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        stream
            .Read(
                bytes.as_mut_ptr().cast(),
                bytes.len() as u32,
                Some(&mut read),
            )
            .ok()?;
        if read as usize != bytes.len() {
            return Err(E_FAIL.into());
        }
        Ok(bytes)
    }
}
fn base64(bytes: &[u8]) -> String {
    let mut length = 0;
    // SAFETY: input slice and exact-size UTF-16 output are live, lengths are
    // API-queried. Native NOCRLF base64 matches C++'s unwrapped RFC4648 output.
    unsafe {
        let flags = CRYPT_STRING(CRYPT_STRING_BASE64.0 | CRYPT_STRING_NOCRLF);
        if !CryptBinaryToStringW(bytes, flags, None, &mut length).as_bool() {
            return String::new();
        }
        let mut text = vec![0u16; length as usize];
        if !CryptBinaryToStringW(bytes, flags, Some(PWSTR(text.as_mut_ptr())), &mut length)
            .as_bool()
        {
            return String::new();
        }
        let end = text.iter().position(|&ch| ch == 0).unwrap_or(text.len());
        format!(
            "data:image/png;base64,{}",
            String::from_utf16_lossy(&text[..end])
        )
    }
}
pub fn icon_data(target: &str, directory: &Path, refresh: bool) -> String {
    if target.is_empty() {
        return String::new();
    }
    let cache = cache_path(target, directory);
    let caching = !directory.as_os_str().is_empty();
    let meta = metadata_path(&cache);
    let cached = caching
        .then(|| read_file(&cache, 8, 1024 * 1024))
        .flatten()
        .filter(|bytes| bytes.starts_with(PNG));
    let executable = resolve(target);
    let fingerprint = executable.as_deref().and_then(fingerprint);
    let previous = caching
        .then(|| read_file(&meta, 1, 65536))
        .flatten()
        .filter(|bytes| bytes.len() % 2 == 0);
    if !refresh
        && cached.is_some()
        && (executable.is_none() || fingerprint.is_some() && fingerprint == previous)
    {
        return base64(cached.as_deref().unwrap());
    }
    if let Some(bytes) = executable
        .as_deref()
        .and_then(|path| encode_icon(path).ok())
    {
        if caching && bytes.starts_with(PNG) {
            let _ = std::fs::create_dir(directory);
            if windows_config::save_bytes(&cache, &bytes).is_ok() {
                if let Some(fingerprint) = fingerprint {
                    let _ = windows_config::save_bytes(&meta, &fingerprint);
                }
            }
        }
        return base64(&bytes);
    }
    cached.as_deref().map(base64).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_png_cache_fingerprint_and_base64_match_cpp() {
        let _apartment = crate::windows_webview::Apartment::new().unwrap();
        let directory = std::env::temp_dir().join(format!(
            "padmux-icon-reference-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(directory.clone());
        let cache = directory.join("rust-cache");
        let target = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/notepad.exe")
            .to_string_lossy()
            .into_owned();
        let data = icon_data(&target, &cache, false);
        assert!(data.starts_with("data:image/png;base64,iVBORw0KGgo"));
        assert_eq!(icon_data(&target, &cache, false), data);
        assert_eq!(icon_data(&target.to_uppercase(), &cache, false), data);
        let bytes = std::fs::read(cache_path(&target, &cache)).unwrap();
        let meta = std::fs::read(metadata_path(&cache_path(&target, &cache))).unwrap();
        assert_eq!(fingerprint(&resolve(&target).unwrap()).unwrap(), meta);
        assert!(!meta.is_empty() && meta.len().is_multiple_of(2));
        let reference = std::env::var_os("PADMUX_CONFIG_REFERENCE");
        if std::env::var_os("CI").is_some() {
            assert!(reference.is_some(), "C++ reference is required in CI");
        }
        if let Some(reference) = reference {
            let cpp_cache = directory.join("cpp-cache");
            let output = directory.join("icon.txt");
            let result = std::process::Command::new(&reference)
                .args(["--icon", &target])
                .arg(&cpp_cache)
                .arg(&output)
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            assert_eq!(std::fs::read_to_string(&output).unwrap(), data);
            assert_eq!(
                std::fs::read(cache_path(&target, &cpp_cache)).unwrap(),
                bytes
            );
            assert_eq!(
                std::fs::read(metadata_path(&cache_path(&target, &cpp_cache))).unwrap(),
                meta
            );
            let ghost = "C:\\PadMuxReference\\NO-icon-😀.exe";
            // Cached header acceptance is deliberately the reference's 8-byte
            // signature rule. Vary padding to compare all base64 tail lengths.
            for extra in 0..3 {
                let mut cached = PNG.to_vec();
                cached.extend_from_slice(&[0xff, 0x80][..extra]);
                std::fs::write(cache_path(ghost, &cache), &cached).unwrap();
                let result = std::process::Command::new(&reference)
                    .args(["--icon", ghost])
                    .arg(&cache)
                    .arg(&output)
                    .output()
                    .unwrap();
                assert!(result.status.success(), "{result:?}");
                assert_eq!(
                    std::fs::read_to_string(&output).unwrap(),
                    icon_data(ghost, &cache, false)
                );
            }
            std::fs::write(cache_path(ghost, &cache), b"not a PNG").unwrap();
            assert!(icon_data(ghost, &cache, false).is_empty());
        }
        assert_eq!(icon_data(&target, Path::new(""), false), data);
        assert!(icon_data("", &cache, false).is_empty());
    }
}
