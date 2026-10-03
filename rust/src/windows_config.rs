//! Win32 settings file sharing and durable replacement from SettingsJson.inc.
use crate::{
    config::{empty_bindings, empty_preset, Configuration, Layer, MAX_BYTES},
    mapping::{Button, Sequence, Turbo, BUTTON_COUNT, INHERIT},
};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::Path,
};
use windows::{
    core::PCWSTR,
    Win32::{
        Storage::FileSystem::{
            CopyFileW, GetFileAttributesW, MoveFileExW, FILE_ATTRIBUTE_DIRECTORY, FILE_SHARE_READ,
            INVALID_FILE_ATTRIBUTES, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        },
        System::WindowsProgramming::{GetPrivateProfileIntW, GetPrivateProfileStringW},
    },
};
pub fn copy(source: &Path, destination: &Path) -> Result<(), String> {
    copy_file(source, destination, false)
}
pub(crate) fn copy_file(
    source: &Path,
    destination: &Path,
    fail_if_exists: bool,
) -> Result<(), String> {
    let source = wide(source).map_err(|e| e.to_string())?;
    let destination = wide(destination).map_err(|e| e.to_string())?;
    // SAFETY: both owned paths are terminated and live through the synchronous
    // copy; the flag preserves each C++ call's overwrite/sharing semantics.
    unsafe {
        CopyFileW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            fail_if_exists,
        )
    }
    .map_err(|e| e.to_string())
}
pub fn remove_missing_targets(config: &mut Configuration) -> bool {
    let mut missing = false;
    for preset in &mut config.presets {
        if !preset.target_executable.contains(['\\', '/']) {
            continue;
        }
        // Embedded NUL terminates a C++ target too; preserve that behavior here.
        let path: Vec<_> = preset.target_executable.encode_utf16().chain([0]).collect();
        // SAFETY: readable, terminated UTF-16 remains live; Win32 only queries
        // attributes. A directory or failed query invalidates an imported target.
        let attributes = unsafe { GetFileAttributesW(PCWSTR(path.as_ptr())) };
        if attributes == INVALID_FILE_ATTRIBUTES || attributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
            preset.target_executable.clear();
            missing = true;
        }
    }
    if missing {
        config.auto_mode = false;
    }
    missing
}

pub fn load(path: &Path) -> Result<Configuration, String> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(path)
        .map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    if length == 0 || length > MAX_BYTES as u64 {
        return Err("config file must be 1..8 MiB".into());
    }
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Configuration::from_json(&bytes)
}
pub fn settings_file() -> Result<std::path::PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::UI::Shell::{SHGetFolderPathW, CSIDL_LOCAL_APPDATA, SHGFP_TYPE_CURRENT};
    let mut directory = [0u16; 260];
    // SAFETY: exact native MAX_PATH array and current-user query. Preserve C++
    // folder API instead of trusting a mutable LOCALAPPDATA environment value.
    unsafe {
        SHGetFolderPathW(
            None,
            CSIDL_LOCAL_APPDATA as i32,
            None,
            SHGFP_TYPE_CURRENT.0 as u32,
            &mut directory,
        )
    }
    .map_err(|e| e.to_string())?;
    let length = directory
        .iter()
        .position(|&u| u == 0)
        .ok_or("unterminated appdata path")?;
    let folder = std::path::PathBuf::from(std::ffi::OsString::from_wide(&directory[..length]))
        .join("PadMux");
    let _ = fs::create_dir(&folder);
    Ok(folder.join("ControllerSettings.json"))
}
pub(crate) fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains NUL",
        ));
    }
    value.push(0);
    Ok(value)
}
pub fn save(path: &Path, config: &Configuration) -> Result<(), String> {
    let bytes = config.to_json()?;
    save_bytes(path, &bytes)
}
pub(crate) fn save_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("empty settings path".into());
    }
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    let temporary = Path::new(&name);
    let destination = wide(path).map_err(|e| e.to_string())?;
    let source = wide(temporary).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .share_mode(0)
        .open(temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        // SAFETY: both paths are NUL-terminated UTF-16 without interior NULs
        // and live until the synchronous call returns. Win32 is necessary for
        // C++'s REPLACE_EXISTING | WRITE_THROUGH metadata durability semantics.
        unsafe {
            MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(destination.as_ptr()),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
        .map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn attributes(path: &Path) -> Result<u32, String> {
    let path = wide(path).map_err(|e| e.to_string())?;
    // SAFETY: owned terminated path is live for this query-only native call.
    Ok(unsafe { GetFileAttributesW(PCWSTR(path.as_ptr())) })
}
struct Ini(Vec<u16>);
impl Ini {
    fn int(&self, section: &str, key: &str, default: u32) -> u32 {
        let section: Vec<_> = section.encode_utf16().chain([0]).collect();
        let key: Vec<_> = key.encode_utf16().chain([0]).collect();
        // SAFETY: terminated section/key/path remain live; native INI integer
        // parsing preserves Win32's unsigned/sign/hex/default quirks verbatim.
        unsafe {
            GetPrivateProfileIntW(
                PCWSTR(section.as_ptr()),
                PCWSTR(key.as_ptr()),
                default as i32,
                PCWSTR(self.0.as_ptr()),
            ) as u32
        }
    }
    fn string(
        &self,
        section: &str,
        key: &str,
        default: &str,
        capacity: usize,
    ) -> Result<String, String> {
        let section: Vec<_> = section.encode_utf16().chain([0]).collect();
        let key: Vec<_> = key.encode_utf16().chain([0]).collect();
        let default: Vec<_> = default.encode_utf16().chain([0]).collect();
        let mut buffer = vec![0; capacity];
        // SAFETY: all inputs are owned terminated buffers and native API gets
        // the actual writable output length, including its terminating NUL.
        let length = unsafe {
            GetPrivateProfileStringW(
                PCWSTR(section.as_ptr()),
                PCWSTR(key.as_ptr()),
                PCWSTR(default.as_ptr()),
                Some(&mut buffer),
                PCWSTR(self.0.as_ptr()),
            )
        } as usize;
        String::from_utf16(&buffer[..length]).map_err(|e| e.to_string())
    }
    fn bindings(
        &self,
        section: &str,
        prefix: &str,
        inherit: bool,
    ) -> Result<crate::config::Bindings, String> {
        let mut bindings = empty_bindings(inherit);
        for (i, (_, setting, _)) in crate::webview::BUTTONS.iter().enumerate() {
            let default = if inherit { INHERIT } else { 0 };
            let value = self.int(section, &format!("{prefix}{setting}"), default);
            let key_valid = |key: u32| key & 0xff != 0 && key & !0x100ff == 0;
            bindings.mapping[i] = if value == 0 || (inherit && value == INHERIT) || key_valid(value)
            {
                value
            } else {
                default
            };
            let interval = |value| {
                if (20..=2000).contains(&value) {
                    value
                } else {
                    50
                }
            };
            let delay = self.int(section, &format!("{prefix}TurboDelay_{setting}"), 0);
            bindings.turbo[i] = Turbo {
                enabled: self.int(section, &format!("{prefix}Turbo_{setting}"), 0) == 1,
                interval_ms: interval(self.int(
                    section,
                    &format!("{prefix}TurboInterval_{setting}"),
                    50,
                )),
                delay_ms: if delay <= 5000 { delay } else { 0 },
            };
            let mut sequence = Sequence {
                enabled: self.int(section, &format!("{prefix}SequenceEnabled_{setting}"), 0) == 1,
                interval_ms: interval(self.int(
                    section,
                    &format!("{prefix}SequenceInterval_{setting}"),
                    50,
                )),
                repeat: self.int(section, &format!("{prefix}SequenceRepeat_{setting}"), 1) != 0,
                keys: Vec::new(),
            };
            let list = self.string(section, &format!("{prefix}Sequence_{setting}"), "", 256)?;
            // split_terminator accepts one final comma, as the C++ start<length
            // loop does, but still rejects an empty item between two commas.
            for item in list.split_terminator(',').take(16) {
                // The bridge's wcstoul helper excludes an initial minus; INI's
                // wcstoul does not. A leading space preserves that distinction.
                let signed;
                let text = if item.starts_with('-') {
                    signed = format!(" {item}");
                    &signed
                } else {
                    item
                };
                match crate::editor::parse_number(text).filter(|&key| key_valid(key)) {
                    Some(key) if !item.is_empty() => sequence.keys.push(key),
                    _ => {
                        sequence.keys.clear();
                        break;
                    }
                }
            }
            if sequence.keys.is_empty() {
                sequence.enabled = false;
            }
            bindings.sequence[i] = sequence;
        }
        Ok(bindings)
    }
}
/// Read the old INI through the same Windows APIs and buffers as main.cpp.
pub fn load_legacy(path: &Path) -> Result<Configuration, String> {
    let ini = Ini(wide(path).map_err(|e| e.to_string())?);
    let mut config = Configuration::initial();
    config.language = if ini.string("MapleStory", "Language", "ja", 8)? == "en" {
        "en"
    } else {
        "ja"
    }
    .into();
    config.close_behavior = ini.int("MapleStory", "CloseBehavior", 0);
    if config.close_behavior > 2 {
        config.close_behavior = 0;
    }
    config.auto_mode = ini.int("MapleStory", "AutoMode", 0) == 1;
    for index in 0..ini.int("MapleStory", "FolderCount", 0).min(100) {
        let folder = ini.string("MapleStory", &format!("Folder{index}"), "", 512)?;
        if !folder.is_empty() {
            config.folders.push(folder);
        }
    }
    let legacy = ini.string("MapleStory", "ConfigFormat", "", 64)? != "ControllerSettings-v2"
        && attributes(path)? != INVALID_FILE_ATTRIBUTES;
    let target = ini.string("MapleStory", "TargetExecutable", "", 520)?;
    let target = if target.is_empty() {
        "MapleStory.exe".into()
    } else {
        target
    };
    let count = ini.int("MapleStory", "PresetCount", 0);
    config.presets.clear();
    if count == 0 || count > 100 {
        let mut preset = empty_preset("標準".into(), String::new());
        if legacy {
            preset.target_executable = target;
        }
        preset.bindings = ini.bindings("MapleStory", "", false)?;
        config.presets.push(preset);
    } else {
        for index in 0..count {
            let section = format!("Preset{index}");
            let mut preset = empty_preset(
                ini.string(&section, "Name", "プリセット", 128)?,
                ini.string(&section, "Folder", "", 512)?,
            );
            preset.target_executable = if legacy {
                target.clone()
            } else {
                ini.string(&section, "TargetExecutable", "", 520)?
            };
            preset.left_pad_mode = ini.int(
                &section,
                "LeftPadMode",
                ini.int(&section, "LeftPadMouse", 0),
            );
            preset.right_pad_mode = ini.int(
                &section,
                "RightPadMode",
                ini.int(&section, "RightPadMouse", 0),
            );
            if preset.left_pad_mode > 2 {
                preset.left_pad_mode = 0;
            }
            if preset.right_pad_mode > 2 {
                preset.right_pad_mode = 0;
            }
            preset.left_pad_sensitivity = ini.int(&section, "LeftPadSensitivity", 100);
            preset.right_pad_sensitivity = ini.int(&section, "RightPadSensitivity", 100);
            if !(25..=400).contains(&preset.left_pad_sensitivity) {
                preset.left_pad_sensitivity = 100;
            }
            if !(25..=400).contains(&preset.right_pad_sensitivity) {
                preset.right_pad_sensitivity = 100;
            }
            preset.bindings = ini.bindings(&section, "", false)?;
            let saved_count = if !ini.string(&section, "LayerCount", "", 32)?.is_empty() {
                ini.int(&section, "LayerCount", 0)
            } else if !ini.string(&section, "Layer1Trigger", "", 32)?.is_empty() {
                2
            } else {
                0
            };
            for index in 0..saved_count.min(BUTTON_COUNT as u32) {
                let prefix = format!("Layer{}", index + 1);
                let default = match index {
                    0 => Button::L4 as usize,
                    1 => Button::R4 as usize,
                    _ => BUTTON_COUNT,
                };
                preset.layers.push(Layer {
                    name: ini.string(&section, &format!("{prefix}Name"), "", 64)?,
                    trigger: (ini.int(&section, &format!("{prefix}Trigger"), default as u32)
                        as usize)
                        .min(BUTTON_COUNT),
                    bindings: ini.bindings(&section, &format!("{prefix}_"), true)?,
                });
            }
            config.presets.push(preset);
        }
        config.selected_preset = ini.int("MapleStory", "SelectedPreset", 0) as usize;
        if config.selected_preset >= config.presets.len() {
            config.selected_preset = 0;
        }
    }
    Ok(config)
}
/// C++ startup priority. The caller supplies a real settings path or a TEMP
/// sandbox; existing or malformed JSON is never overwritten by a fallback.
pub fn load_startup(path: &Path) -> Result<Configuration, String> {
    if path.as_os_str().is_empty() {
        return Err("empty settings path".into());
    }
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or("settings folder missing")?;
    let _ = fs::create_dir(folder); // C++ SettingsFile also tolerates existing/failing mkdir.
    if attributes(path)? != INVALID_FILE_ATTRIBUTES {
        return load(path);
    }
    let parent = folder.parent().ok_or("settings parent missing")?;
    // Previous product data wins over the older fork's data. Invalid old JSON
    // is an error, never an excuse to silently replace the user's presets.
    let previous = parent.join("Remapcon");
    let steamless = parent.join("SteamlessController");
    let legacy =
        if attributes(&previous.join("ControllerSettings.json"))? != INVALID_FILE_ATTRIBUTES {
            &previous
        } else {
            &steamless
        };
    let json = legacy.join("ControllerSettings.json");
    let ini = legacy.join("MapleStoryController.ini");
    let config = if attributes(&json)? != INVALID_FILE_ATTRIBUTES {
        load(&json)?
    } else if attributes(&ini)? != INVALID_FILE_ATTRIBUTES {
        load_legacy(&ini)?
    } else {
        Configuration::initial()
    };
    // Legacy C++ saves what INI APIs read, including an empty preset name that
    // its strict JSON loader would reject on the next run. Do not silently
    // "repair" historical values during migration; retain that behavior.
    let mut bytes = serde_json::to_vec(&config).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    if bytes.len() > MAX_BYTES {
        return Err("config exceeds 8 MiB".into());
    }
    save_bytes(path, &bytes)?;
    let copy_new = |source: &Path, destination: &Path| {
        if let (Ok(source), Ok(destination)) = (wide(source), wide(destination)) {
            // SAFETY: terminated owned paths live through CopyFileW; TRUE
            // preserves C++ best-effort migration without overwriting files.
            let _ =
                unsafe { CopyFileW(PCWSTR(source.as_ptr()), PCWSTR(destination.as_ptr()), true) };
        }
    };
    // Copy-if-absent keeps newer placement/icons and leaves both old folders intact.
    for source_folder in [&previous, legacy] {
        copy_new(
            &source_folder.join("WindowPlacement.ini"),
            &folder.join("WindowPlacement.ini"),
        );
        if let Ok(entries) = fs::read_dir(source_folder.join("TargetIcons")) {
            let destination = folder.join("TargetIcons");
            let _ = fs::create_dir(&destination);
            for entry in entries.flatten() {
                let name = entry.file_name();
                let text = name.to_string_lossy();
                if (text.ends_with(".png") || text.ends_with(".png.meta"))
                    && attributes(&entry.path()).is_ok_and(|a| {
                        a != INVALID_FILE_ATTRIBUTES && a & FILE_ATTRIBUTE_DIRECTORY.0 == 0
                    })
                {
                    copy_new(&entry.path(), &destination.join(name));
                }
            }
        }
    }
    Ok(config)
}
