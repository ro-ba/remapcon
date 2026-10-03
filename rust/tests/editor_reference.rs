use padmux_diag::{
    config::Configuration,
    editor::{parse_number, Editor},
    webview,
};
use serde_json::{json, Value};

#[test]
fn bridge_edits_clipboard_history_and_failed_saves_match_cpp() {
    let commands = [
        "selectLayer\t1",
        "copyRow\tbinding\t2\t1\t0",
        "pasteRow\tbinding\t1\t1\t0",
        "pasteRow\tbinding\t1\t1\t1",
        "selectLayer\t0",
        "setAuto\t0",
        "PICK\tC:\\Games\\Picked.exe",
        "PICK\tC:\\Games\\Picked.exe",
        "PICK\t-",
        "FAIL\tPICK\tC:\\Games\\Failed.exe",
        "undo",
        "redo",
        "toggleMode",
        "setPreview\t1",
        "setLanguage\ten",
        "FAIL\tsetLanguage\tja",
        "setCloseBehavior\t2",
        "setSteamTakeover\t0",
        "setPadConfig\t1\t2\t25\t400",
        "FAIL\tsetPadConfig\t2\t1\t100\t100",
        "setStickConfig\t0\t32767\t32767\t0",
        "setPadModes\t2\t1",
        "setPadMode\tright\t2",
        "copyRow\tpad\t0\t0\t0",
        "pasteRow\tpad\t1\t0\t0",
        "FAIL\tpasteRow\tpad\t0\t0\t0",
        "pasteRow\tbinding\t0\t0\t0",
        "copyRow\tstick\t0\t0\t0",
        "pasteRow\tstick\t0\t0\t0",
        "saveBinding\t0\t0\tkey\t30\t1\t20\t5000\t\t50\t1\t0",
        "saveBinding\t0\t0\tlayer\t0\t0\t50\t0\t\t50\t1\t0",
        "copyRow\tbinding\t0\t0\t0",
        "pasteRow\tbinding\t1\t0\t0",
        "selectLayer\t1",
        "pasteRow\tbinding\t2\t1\t0",
        "selectLayer\t0",
        "saveBinding\t0\t0\tlayer\t0\t0\t50\t0\t\t50\t1\t2",
        "saveBinding\t1\t0\tsequence\t0\t1\t20\t5000\t30,65608\t2000\t0\t0",
        "saveBinding\t18\t0\tmouse\t131073\t0\t50\t0\t\t50\t1\t0",
        "copyRow\tbinding\t18\t0\t0",
        "pasteRow\tbinding\t0\t0\t0",
        "pasteRow\tbinding\t20\t0\t0",
        "FAIL\tresetBinding\t1\t0\t0",
        "resetBinding\t1\t0\t0",
        "resetBinding\t0\t0\t0",
        "layer-new\t0\tExtra",
        "FAIL\tlayer-new\t0\tFailed",
        "layer-rename\t0\tRenamed",
        "saveBinding\t2\t1\tinherit\t0\t0\t50\t0\t\t50\t1\t0",
        "resetBinding\t2\t1\t0",
        "copyRow\tbinding\t2\t1\t0",
        "selectLayer\t0",
        "pasteRow\tbinding\t2\t0\t0",
        "layer-delete\t0",
        "undo",
        "FAIL\tredo",
        "redo",
        "undo",
        "redo",
        "folder-new\t\tGroup",
        "folder-new\tGroup\tChild",
        "folder-rename\tGroup\tRenamed",
        "preset-copy\t0\tCopy\tRenamed/Child",
        "toggleMode",
        "clearTarget",
        "toggleMode",
        "preset-rename\t1\tcopy\tRenamed/Child",
        "preset-new\t0\tCOPY\tRenamed/Child",
        "preset-new\t0\tNew\t",
        "FAIL\tselectPreset\t0",
        "selectPreset\t0",
        "selectLayer\t1",
        "FAIL\tpreset-new\t0\tFailed\t",
        "selectLayer\t0",
        "movePreset\t1\tRenamed",
        "movePresetNewFolder\t1\tRenamed\tOther",
        "deleteFolder\tRenamed",
        "deleteFolder\tRenamed/Child",
        "FAIL\tdeletePreset\t1",
        "deletePreset\t1",
        "deletePreset\t99",
        "selectPreset\t99",
        "setPadConfig\t3\t0\t100\t100",
        "setStickConfig\t32768\t0\t0\t0",
        "saveBinding\t0\t0\tmouse\t131073\t0\t50\t0\t\t50\t1\t0",
        "saveBinding\t0\t0\tkey\t0\t0\t50\t0\t\t50\t1\t0",
        "unknown",
        "setAuto\t1",
    ];
    let mut commands: Vec<String> = commands.into_iter().map(str::to_owned).collect();
    // Exercise the actual 30-entry history ceiling, including empty undo/redo.
    for i in 0..35 {
        commands.push(format!(
            "setLanguage\t{}",
            if i % 2 == 0 { "ja" } else { "en" }
        ));
    }
    for _ in 0..32 {
        commands.push("undo".into());
    }
    for _ in 0..32 {
        commands.push("redo".into());
    }
    commands.extend(
        [
            "IMPORT\t-",
            "IMPORT\tbad.json",
            "FAIL\tIMPORT\tconfig-import.json",
            "IMPORTFAIL\tconfig-import.json",
            "IMPORT\tconfig-import.json",
            "undo",
            "setLanguage\tja",
            "setLanguage\ten",
            "IMPORT\tconfig-import.json",
            "undo",
            "redo",
            "PICK\tpadmux-reference.exe",
            "toggleMode",
            "selectLayer\t1",
            "IMPORTFAIL\tconfig-import.json",
            "FAIL\tIMPORT\tconfig-import.json",
            "IMPORT\tconfig-import.json",
            "undo",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    let mut imported = Configuration::from_json(include_bytes!("config-reference.json")).unwrap();
    imported.language = "en".into();
    imported.presets[0].name = "Imported preset".into();
    imported.presets[0].target_executable = "C:\\PadMuxReference\\DoesNotExist.exe".into();
    imported.auto_mode = true;
    let import_bytes = imported.to_json().unwrap();
    imported.presets[0].target_executable.clear();
    imported.auto_mode = false;
    let mut editor =
        Editor::new(Configuration::from_json(include_bytes!("config-reference.json")).unwrap());
    let mut trace = Vec::new();
    for command in &commands {
        let (command, okay) = command
            .strip_prefix("FAIL\t")
            .map_or((command.as_str(), true), |s| (s, false));
        let messages = if let Some(path) = command.strip_prefix("PICK\t") {
            editor.pick_target((path != "-").then(|| path.to_owned()), |_| okay);
            vec![editor.state(false, "").unwrap()]
        } else if command.starts_with("IMPORT\t") || command.starts_with("IMPORTFAIL\t") {
            if okay && command.ends_with("config-import.json") {
                editor.import(imported.clone(), |_| !command.starts_with("IMPORTFAIL\t"));
            }
            vec![editor.state(false, "").unwrap()]
        } else {
            editor
                .handle(
                    command,
                    |_| okay,
                    |a, b| {
                        #[cfg(windows)]
                        {
                            padmux_diag::windows_app::equal_text(a, b)
                        }
                        #[cfg(not(windows))]
                        {
                            a.eq_ignore_ascii_case(b)
                        }
                    },
                    false,
                    "",
                )
                .unwrap()
        };
        let messages: Vec<Value> = messages
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        trace.push(json!({"config":editor.config,"editedLayer":editor.edited_layer,"requested":editor.requested,"preview":editor.preview,"generation":editor.generation,"messages":messages}));
    }
    assert!(trace.iter().any(|step| step["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["result"] == "saveFailed")));
    assert!(trace.iter().any(|step| step["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["type"] == "history" && m["empty"] == true)));
    let numbers = [
        "",
        "0",
        "01",
        "+1",
        " 1",
        "-1",
        " -1",
        "4294967295",
        "4294967296",
        "99999999999999999999999999",
        "1x",
        "1 ",
        "+",
        " ",
        "0x10",
        "\u{00a0}1",
        "\u{2003}1",
        "\u{3000}1",
        " -4294967295",
        " -4294967296",
    ];
    assert_eq!(parse_number("\0trailing"), Some(0));
    assert_eq!(parse_number("1\0trailing"), Some(1));
    let placements = [
        "0,0,1440,900,0",
        "-100, +20,1000,700,1tail",
        "0 ,0,1000,700,0",
        "0,0,1000,700",
        "0,0,1000,700,-1",
        "+0,0,1000,700,0",
    ];
    assert_eq!(webview::window_placement("0,0,2147483648,700,0"), None);
    if let Some(reference) = std::env::var_os("PADMUX_CONFIG_REFERENCE") {
        let directory = std::env::temp_dir().join(format!(
            "padmux-editor-reference-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let input = directory.join("config.json");
        let script = directory.join("commands.txt");
        let output = directory.join("scalar.json");
        let result = std::process::Command::new(&reference)
            .arg("--number-spaces")
            .arg(&output)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        let spaces: Vec<u32> = serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
        let accepted: Vec<u32> = (1..65536)
            .filter(|&code| {
                char::from_u32(code).is_some_and(|ch| {
                    parse_number(&format!("{ch}1")) == Some(1) && !matches!(ch, '0' | '+')
                })
            })
            .collect();
        assert_eq!(accepted, spaces);
        std::fs::write(&input, include_bytes!("config-reference.json")).unwrap();
        std::fs::write(directory.join("config-import.json"), import_bytes).unwrap();
        std::fs::write(directory.join("bad.json"), b"{}").unwrap();
        std::fs::write(&script, commands.join("\n")).unwrap();
        let result = std::process::Command::new(&reference)
            .arg("--ui-commands")
            .arg(input)
            .arg(&script)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        let expected: Vec<Value> = serde_json::from_slice(
            &std::fs::read(directory.join("commands.txt.output.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(expected.len(), trace.len());
        for (index, (actual, expected)) in trace.iter().zip(expected).enumerate() {
            assert_eq!(actual, &expected, "command {index}: {}", commands[index]);
        }
        for number in numbers {
            let result = std::process::Command::new(&reference)
                .arg("--number")
                .arg(number)
                .arg(&output)
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            let expected: Value = serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
            assert_eq!(json!(parse_number(number)), expected, "number {number:?}");
        }
        for placement in placements {
            let result = std::process::Command::new(&reference)
                .arg("--placement")
                .arg(placement)
                .arg(&output)
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            let expected: Value = serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
            assert_eq!(
                json!(webview::window_placement(placement)),
                expected,
                "placement {placement:?}"
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
