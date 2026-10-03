use padmux_diag::{
    config::Configuration,
    mapping::{Button, INHERIT},
};
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn fixture() -> Value {
    serde_json::from_slice(include_bytes!("config-reference.json")).unwrap()
}
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
fn temporary() -> PathBuf {
    std::env::temp_dir().join(format!(
        "padmux-config-test-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ))
}
fn compare(bytes: &[u8], expected: bool) {
    assert_eq!(
        Configuration::from_json(bytes).is_ok(),
        expected,
        "Rust acceptance mismatch"
    );
    if let Some(exe) = std::env::var_os("PADMUX_CONFIG_REFERENCE") {
        let path = temporary();
        fs::write(&path, bytes).unwrap();
        let result = std::process::Command::new(exe)
            .arg("--check")
            .arg(&path)
            .output()
            .unwrap();
        fs::remove_file(path).unwrap();
        assert!(
            [Some(0), Some(1)].contains(&result.status.code()),
            "reference failed: {:?}",
            result
        );
        assert_eq!(result.status.success(), expected, "C++ acceptance mismatch");
    }
}
fn check(value: &Value, expected: bool) {
    compare(&serde_json::to_vec(value).unwrap(), expected);
}

#[test]
fn cpp_fixture_roundtrips_and_old_28_button_format_keeps_sequences() {
    let bytes = include_bytes!("config-reference.json");
    compare(bytes, true);
    let config = Configuration::from_json(bytes).unwrap();
    assert!(config.presets[0].name.contains('😀'));
    assert_eq!(config.presets[0].layers[0].bindings.mapping[0], INHERIT);
    let encoded = config.to_json().unwrap();
    compare(&encoded, true);
    assert_eq!(Configuration::from_json(&encoded).unwrap(), config);
    let mut old = fixture();
    for name in ["mapping", "turbo", "sequence"] {
        old["presets"][0][name].as_array_mut().unwrap().truncate(28);
        old["presets"][0]["layers"][0][name]
            .as_array_mut()
            .unwrap()
            .truncate(28);
    }
    old.as_object_mut().unwrap().remove("steamTakeover");
    for name in [
        "leftStickDeadzone",
        "rightStickDeadzone",
        "leftStickOverlap",
        "rightStickOverlap",
    ] {
        old["presets"][0].as_object_mut().unwrap().remove(name);
    }
    check(&old, true);
    let legacy = Configuration::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(!legacy.steam_takeover);
    assert_eq!(legacy.presets[0].left_stick_deadzone, 12288);
    assert_eq!(legacy.presets[0].right_stick_overlap, 16000);
    let bindings = &legacy.presets[0].bindings;
    for i in 0..4 {
        assert_eq!(bindings.mapping[28 + i], bindings.mapping[10 + i]);
        assert_eq!(bindings.turbo[28 + i], bindings.turbo[10 + i]);
        assert_eq!(bindings.sequence[28 + i], bindings.sequence[10 + i]);
    }
    let encoded = legacy.to_json().unwrap();
    compare(&encoded, true);
    assert_eq!(Configuration::from_json(&encoded).unwrap(), legacy);
    let engine = legacy.presets.into_iter().next().unwrap().into_engine(7);
    assert_eq!(engine.generation, 7);
    assert_eq!(engine.base.mapping[Button::LB as usize], 0x20001);
    assert_eq!(engine.layers[0].trigger, 32);
}

#[test]
fn schema_boundaries_match_cpp_including_utf16_lengths() {
    for (field, value) in [
        ("formatVersion", json!(0)),
        ("language", json!("JA")),
        ("closeBehavior", json!(3)),
        ("selectedPreset", json!(1)),
        ("autoMode", json!("true")),
        ("steamTakeover", Value::Null),
        ("folders", json!([""])),
        ("presets", json!([])),
    ] {
        let mut value_json = fixture();
        value_json[field] = value;
        check(&value_json, false);
    }
    for (field, valid, invalid) in [
        ("name", json!("😀".repeat(64)), json!("😀".repeat(65))),
        ("folder", json!("😀".repeat(256)), json!("😀".repeat(257))),
        (
            "targetExecutable",
            json!("😀".repeat(260)),
            json!("😀".repeat(261)),
        ),
        ("leftPadMode", json!(2), json!(3)),
        ("rightPadSensitivity", json!(25), json!(24)),
        ("leftPadSensitivity", json!(400), json!(401)),
        ("leftStickDeadzone", json!(32767), json!(32768)),
        ("leftStickOverlap", json!(0), json!(32768)),
    ] {
        let mut data = fixture();
        data["presets"][0][field] = valid;
        check(&data, true);
        data["presets"][0][field] = invalid;
        check(&data, false);
    }
    for (key, valid) in [
        (0, true),
        (0x10048, true),
        (0x20003, true),
        (0x10000, false),
        (0x20004, false),
        (INHERIT, false),
    ] {
        let mut data = fixture();
        data["presets"][0]["mapping"][0] = json!(key);
        check(&data, valid);
    }
    let mut data = fixture();
    data["presets"][0]["layers"][0]["trigger"] = json!(33);
    check(&data, false);
    data["presets"][0]["layers"][0]["trigger"] = json!(32);
    check(&data, true);
    data["presets"][0]["layers"][0]["name"] = json!("😀".repeat(33));
    check(&data, false);
    data["presets"][0]["layers"][0]["name"] = json!("");
    check(&data, true);
    data["presets"][0]["turbo"][0]["intervalMs"] = json!(19);
    check(&data, false);
    data["presets"][0]["turbo"][0]["intervalMs"] = json!(2000);
    check(&data, true);
    data["presets"][0]["turbo"][0]["delayMs"] = json!(5001);
    check(&data, false);
    data["presets"][0]["turbo"][0]["delayMs"] = json!(5000);
    data["presets"][0]["sequence"][1]["keys"] = json!([]);
    check(&data, false);
    data["presets"][0]["sequence"][1]["enabled"] = json!(false);
    check(&data, true);
    data["presets"][0]["sequence"][1]["keys"] = json!([0]);
    check(&data, false);
    data["presets"][0]["sequence"][1]["keys"] = json!([INHERIT]);
    check(&data, false);
    data["presets"][0]["sequence"][1]["keys"] = json!(vec![30; 16]);
    check(&data, true);
    data["presets"][0]["sequence"][1]["keys"] = json!(vec![30; 17]);
    check(&data, false);
    data = fixture();
    data["presets"][0]["mapping"].as_array_mut().unwrap().pop();
    check(&data, false);
    data = fixture();
    let layer = data["presets"][0]["layers"][0].clone();
    data["presets"][0]["layers"] = json!(vec![layer.clone(); 32]);
    check(&data, true);
    data["presets"][0]["layers"] = json!(vec![layer; 33]);
    check(&data, false);
}

#[test]
fn ignored_fields_still_obey_cpp_json_parser_and_file_limits() {
    let mut data = fixture();
    for (value, valid) in [
        (json!(u64::MAX), true),
        (json!(-1), false),
        (json!(1.5), false),
        (Value::Null, true),
    ] {
        data["ignored"] = value;
        check(&data, valid);
    }
    let bytes = include_bytes!("config-reference.json");
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(bytes);
    compare(&bom, true);
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    compare(
        text.replacen('{', "{\"ignored\":1,\"ignored\":2,", 1)
            .as_bytes(),
        false,
    );
    compare(
        text.replacen('{', "{\"ignored\":{\"a\":1,\"\\u0061\":2},", 1)
            .as_bytes(),
        false,
    );
    compare(
        text.replacen('{', "{\"ignored\":18446744073709551616,", 1)
            .as_bytes(),
        false,
    );
    compare(text.replacen('{', "{\"ignored\":-0,", 1).as_bytes(), false);
    compare(text.replacen('{', "{\"ignored\":1e0,", 1).as_bytes(), false);
    for depth in [31, 32] {
        let nested = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        let input = text.replacen('{', &format!("{{\"ignored\":{nested},"), 1);
        compare(input.as_bytes(), depth == 31);
    }
    for count in [10000, 10001] {
        data["ignored"] = json!(vec![Value::Null; count]);
        check(&data, count == 10000);
    }
    compare(text.replacen("テスト", "\\ud800", 1).as_bytes(), false);
    compare(text.replacen("テスト", "\\udc00", 1).as_bytes(), false);
    let mut invalid = text.into_bytes();
    let index = invalid.iter().position(|&b| b > 127).unwrap();
    invalid[index] = 0xff;
    compare(&invalid, false);
    compare(&[], false);
    compare(&vec![b' '; padmux_diag::config::MAX_BYTES + 1], false);
}

#[cfg(windows)]
#[test]
fn atomic_save_preserves_destination_and_foreign_temporary_on_failures() {
    use padmux_diag::windows_config::{copy, load, remove_missing_targets, save};
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
    let directory = temporary();
    fs::create_dir(&directory).unwrap();
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    let path = directory.join("設定😀.json");
    let config = Configuration::from_json(include_bytes!("config-reference.json")).unwrap();
    save(&path, &config).unwrap();
    assert_eq!(load(&path).unwrap(), config);
    let original = fs::read(&path).unwrap();
    let backup = directory.join("before-import.json");
    copy(&path, &backup).unwrap();
    assert_eq!(fs::read(&backup).unwrap(), original);
    let backup_lock = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&backup)
        .unwrap();
    assert!(copy(&path, &backup).is_err());
    assert_eq!(fs::read(&backup).unwrap(), original);
    assert_eq!(fs::read(&path).unwrap(), original);
    drop(backup_lock);
    let mut targets = config.clone();
    targets.auto_mode = true;
    targets.presets[0].target_executable = path.to_string_lossy().into_owned();
    assert!(!remove_missing_targets(&mut targets));
    assert!(targets.auto_mode);
    for target in [
        "reference.exe".to_owned(),
        directory.join("missing.exe").to_string_lossy().into_owned(),
        directory.to_string_lossy().into_owned(),
    ] {
        let mut preset = targets.presets[0].clone();
        preset.target_executable = target;
        targets.presets.push(preset);
    }
    assert!(remove_missing_targets(&mut targets));
    assert!(!targets.auto_mode);
    assert_eq!(targets.presets[0].target_executable, path.to_string_lossy());
    assert_eq!(targets.presets[1].target_executable, "reference.exe");
    assert!(targets.presets[2].target_executable.is_empty());
    assert!(targets.presets[3].target_executable.is_empty());
    let mut changed = Configuration::from_json(&original).unwrap();
    changed.auto_mode = !changed.auto_mode;
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    let temp = PathBuf::from(name);
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    assert!(save(&path, &changed).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(!temp.exists());
    drop(lock);
    fs::write(&temp, b"owned by another writer").unwrap();
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&temp)
        .unwrap();
    assert!(save(&path, &changed).is_err());
    drop(lock);
    assert_eq!(fs::read(&temp).unwrap(), b"owned by another writer");
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::remove_file(&temp).unwrap();
    save(&path, &changed).unwrap();
    assert_eq!(load(&path).unwrap(), changed);
    assert!(!temp.exists());
    changed.language.clear();
    assert!(save(&path, &changed).is_err());
    assert_eq!(
        fs::read(&path).unwrap(),
        Configuration::from_json(&original)
            .map(|mut c| {
                c.auto_mode = !c.auto_mode;
                c.to_json().unwrap()
            })
            .unwrap()
    );
}

#[cfg(windows)]
#[test]
fn startup_and_legacy_ini_match_cpp_without_touching_user_settings() {
    use padmux_diag::windows_config::{load, load_legacy, load_startup};
    use std::{io::Write, os::windows::fs::OpenOptionsExt};
    let root = temporary();
    fs::create_dir(&root).unwrap();
    let ini = root.join("legacy.ini");
    let output = root.join("cpp-legacy.json");
    let reference = std::env::var_os("PADMUX_CONFIG_REFERENCE");
    let cases = [
        "",
        "[MapleStory]\nA=30\nB=131073\nTurbo_A=1\nTurboInterval_A=19\nTurboDelay_A=5001\nSequenceEnabled_X=1\nSequence_X=30,655...\n",
        "[MapleStory]\nLanguage=en\nCloseBehavior=3\nAutoMode=2\nFolderCount=3\nFolder0=Games\nFolder2=Second\nPresetCount=2\nSelectedPreset=9\nTargetExecutable=old.exe\n[Preset0]\nName=First\nLeftPadMouse=1\nRightPadMode=2\nLayer1Trigger=14\nLayer1Name=shift\nLayer1_A=30\nLayer2_B=131073\n[Preset1]\nName=Second\nLeftPadSensitivity=24\nRightPadSensitivity=401\n",
        "[MapleStory]\nConfigFormat=ControllerSettings-v2\nLanguage=english\nCloseBehavior=2\nAutoMode=1\nPresetCount=1\n[Preset0]\nName=\nTargetExecutable=notepad.exe\nLeftPadMode=3\nRightPadMouse=2\nLayerCount=2\nLayer1Trigger=4294967295\nLayer2Trigger=\nA=-1\nB=0x1e\nX=65536\nY=65566\nLB=131073\nSequenceEnabled_A=1\nSequence_A=30,\nSequenceEnabled_B=1\nSequence_B=30,,\nSequenceEnabled_X=1\nSequence_X=-4294967266\nSequenceEnabled_Y=1\nSequence_Y=+30, 65566\nSequenceRepeat_Y=0\nSequenceInterval_Y=2001\nLayer1_Turbo_A=1\nLayer1_SequenceEnabled_A=1\nLayer1_Sequence_A=30,65566\n",
        "[MapleStory]\nPresetCount=101\nSelectedPreset=100\nConfigFormat=ControllerSettings-v2\nA=30\n",
        "[MapleStory]\nPresetCount=1\n[Preset0]\nLayerCount=100\nLayer1_A=\nLayer1_B=0\nLayer1_X=0x1001e\nLayer1_Y=0x20001\n",
    ];
    for (i, case) in cases.iter().enumerate() {
        let mut text = case.to_string();
        if i == 2 {
            text = text.replace(
                "Name=First",
                &format!("Name={}\nFolder={}", "名".repeat(150), "a".repeat(600)),
            );
        }
        if i == 3 {
            text.push_str(&format!(
                "SequenceEnabled_LB=1\nSequence_LB={}\n",
                ["30"; 20].join(",")
            ));
        }
        // Native INI uses Unicode only with the UTF-16 BOM. Reuse its actual
        // API conversion/limits rather than inventing an INI parser in tests.
        let bytes: Vec<_> = [0xfeff]
            .into_iter()
            .chain(text.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        fs::write(&ini, bytes).unwrap();
        let rust = serde_json::to_value(load_legacy(&ini).unwrap()).unwrap();
        if let Some(exe) = &reference {
            let status = std::process::Command::new(exe)
                .args(["--legacy"])
                .arg(&ini)
                .arg(&output)
                .status()
                .unwrap();
            assert!(status.success(), "C++ legacy case {i} failed");
            let cpp: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
            assert_eq!(rust, cpp, "legacy case {i}");
        }
    }
    // Startup priority/copy behavior, actual C++ LoadSettings on a second root.
    for scenario in 0..12 {
        let rust_root = root.join(format!("rust-{scenario}"));
        let cpp_root = root.join(format!("cpp-{scenario}"));
        for base in [&rust_root, &cpp_root] {
            fs::create_dir(base).unwrap();
            let old = base.join("SteamlessController");
            fs::create_dir(&old).unwrap();
            fs::create_dir(old.join("TargetIcons")).unwrap();
            fs::write(old.join("WindowPlacement.ini"), b"legacy placement").unwrap();
            for name in ["keep.png", "keep.png.meta", "skip.PNG", "skip.txt"] {
                fs::write(old.join("TargetIcons").join(name), name).unwrap();
            }
            fs::create_dir(old.join("TargetIcons/sub.png")).unwrap();
            let new = base.join("PadMux");
            fs::create_dir(&new).unwrap();
            if scenario != 0 {
                fs::write(
                    old.join("MapleStoryController.ini"),
                    b"[MapleStory]\nA=30\n",
                )
                .unwrap();
            }
            if scenario >= 2 {
                fs::write(
                    old.join("ControllerSettings.json"),
                    include_bytes!("config-reference.json"),
                )
                .unwrap();
            }
            if scenario == 3 {
                fs::write(old.join("ControllerSettings.json"), b"bad JSON").unwrap();
            }
            if scenario == 4 {
                fs::write(
                    new.join("ControllerSettings.json"),
                    include_bytes!("config-reference.json"),
                )
                .unwrap();
            }
            if scenario == 5 {
                fs::write(new.join("ControllerSettings.json"), b"bad JSON").unwrap();
            }
            if scenario == 6 || scenario == 11 {
                fs::write(new.join("WindowPlacement.ini"), b"existing placement").unwrap();
                fs::create_dir(new.join("TargetIcons")).unwrap();
                fs::write(new.join("TargetIcons/keep.png"), b"existing icon").unwrap();
            }
        }
        if scenario >= 7 {
            for base in [&rust_root, &cpp_root] {
                let previous = base.join("Remapcon");
                fs::create_dir(&previous).unwrap();
                fs::create_dir(previous.join("TargetIcons")).unwrap();
                fs::write(
                    previous.join("WindowPlacement.ini"),
                    b"previous product placement",
                )
                .unwrap();
                fs::write(
                    previous.join("TargetIcons/keep.png"),
                    b"previous product icon",
                )
                .unwrap();
                let mut config: Value =
                    serde_json::from_slice(include_bytes!("config-reference.json")).unwrap();
                config["presets"][0]["name"] = serde_json::json!("Old product user preset");
                let bytes = if [8, 9].contains(&scenario) {
                    b"bad previous JSON".to_vec()
                } else {
                    serde_json::to_vec(&config).unwrap()
                };
                fs::write(previous.join("ControllerSettings.json"), bytes).unwrap();
                if scenario == 9 {
                    fs::write(
                        base.join("PadMux/ControllerSettings.json"),
                        include_bytes!("config-reference.json"),
                    )
                    .unwrap();
                }
                if scenario == 10 {
                    fs::write(
                        base.join("PadMux/ControllerSettings.json"),
                        b"bad current JSON",
                    )
                    .unwrap();
                }
            }
        }
        let previous_path = rust_root.join("Remapcon/ControllerSettings.json");
        let previous_bytes = fs::read(&previous_path).ok();
        let rust_path = rust_root.join("PadMux/ControllerSettings.json");
        let result = load_startup(&rust_path);
        assert_eq!(result.is_ok(), ![3, 5, 8, 10].contains(&scenario));
        assert_eq!(
            fs::read(&previous_path).ok(),
            previous_bytes,
            "old settings must remain unchanged"
        );
        if [7, 11].contains(&scenario) {
            assert_eq!(
                result.as_ref().unwrap().presets[0].name,
                "Old product user preset"
            );
            assert_eq!(
                fs::read(rust_root.join("Remapcon/WindowPlacement.ini")).unwrap(),
                b"previous product placement"
            );
            assert_eq!(
                fs::read(rust_root.join("Remapcon/TargetIcons/keep.png")).unwrap(),
                b"previous product icon"
            );
            assert_eq!(
                fs::read(rust_path.parent().unwrap().join("WindowPlacement.ini")).unwrap(),
                if scenario == 11 {
                    b"existing placement".as_slice()
                } else {
                    b"previous product placement".as_slice()
                }
            );
            assert_eq!(
                fs::read(rust_path.parent().unwrap().join("TargetIcons/keep.png")).unwrap(),
                if scenario == 11 {
                    b"existing icon".as_slice()
                } else {
                    b"previous product icon".as_slice()
                }
            );
        }
        if let Some(exe) = &reference {
            let cpp_path = cpp_root.join("PadMux/ControllerSettings.json");
            let status = std::process::Command::new(exe)
                .arg("--startup")
                .arg(&cpp_path)
                .status()
                .unwrap();
            assert_eq!(status.success(), result.is_ok(), "startup case {scenario}");
            for name in [
                "ControllerSettings.json",
                "WindowPlacement.ini",
                "TargetIcons/keep.png",
                "TargetIcons/keep.png.meta",
                "TargetIcons/skip.PNG",
                "TargetIcons/skip.txt",
            ] {
                let a = fs::read(rust_path.parent().unwrap().join(name));
                let b = fs::read(cpp_path.parent().unwrap().join(name));
                match (a, b) {
                    (Ok(a), Ok(b)) if name == "ControllerSettings.json" && result.is_ok() => {
                        assert_eq!(
                            serde_json::from_slice::<Value>(&a).unwrap(),
                            serde_json::from_slice::<Value>(&b).unwrap()
                        )
                    }
                    (Ok(a), Ok(b)) => assert_eq!(a, b, "startup file {name} case {scenario}"),
                    (Err(_), Err(_)) => {}
                    _ => panic!("copy mismatch {name} case {scenario}"),
                }
            }
        }
    }
    let locked_root = root.join("locked");
    fs::create_dir(&locked_root).unwrap();
    fs::create_dir(locked_root.join("PadMux")).unwrap();
    fs::create_dir(locked_root.join("Remapcon")).unwrap();
    let locked_previous = locked_root.join("Remapcon/ControllerSettings.json");
    fs::write(&locked_previous, include_bytes!("config-reference.json")).unwrap();
    let destination = locked_root.join("PadMux/ControllerSettings.json");
    let tmp_path = destination.with_file_name("ControllerSettings.json.tmp");
    let mut lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(&tmp_path)
        .unwrap();
    lock.write_all(b"another writer").unwrap();
    assert!(load_startup(&destination).is_err());
    assert!(!destination.exists());
    assert!(!destination.with_file_name("WindowPlacement.ini").exists());
    drop(lock);
    assert_eq!(fs::read(&tmp_path).unwrap(), b"another writer");
    assert_eq!(
        fs::read(locked_previous).unwrap(),
        include_bytes!("config-reference.json")
    );
    fs::remove_dir_all(&root).unwrap();
    // New JSON loads with the existing strict schema after initial creation.
    let fresh = temporary();
    fs::create_dir(&fresh).unwrap();
    let path = fresh.join("PadMux/ControllerSettings.json");
    assert_eq!(load_startup(&path).unwrap(), Configuration::initial());
    assert_eq!(load(&path).unwrap(), Configuration::initial());
    fs::remove_dir_all(fresh).unwrap();
}
