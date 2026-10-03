use padmux_diag::{
    config::Configuration,
    webview::{self, Runtime},
};
use serde_json::{json, Value};

#[test]
fn existing_bridge_encoding_and_assets_match_reference() {
    let config = Configuration::from_json(include_bytes!("config-reference.json")).unwrap();
    let state: Value =
        serde_json::from_str(&webview::state_json(&config, Runtime::default()).unwrap()).unwrap();
    assert_eq!(
        state["presets"][0]["target"],
        config.presets[0].target_executable
    );
    assert!(state["presets"][0].get("targetExecutable").is_none());
    let html = webview::embedded_html("1.7.0");
    assert!(html.contains(include_str!("../../src/app/ui/app.js")));
    assert!(html.contains(include_str!("../../src/app/ui/style.css")));
    assert!(!html.contains("<!-- SCRIPT -->") && !html.contains("__PADMUX_VERSION__"));
    assert!(html.contains("<title>PadMux") && html.contains("class=\"app-name\">PadMux"));
    assert!(html.contains("https://api.github.com/repos/ro-ba/padmux/releases/latest"));
    assert!(!html.contains("Remapcon") && !html.contains("remapcon"));
    let cases = [
        ("", json!([""])),
        ("ready", json!(["ready"])),
        (
            "preset-new\t0\t%E6%97%A5%E6%9C%AC%F0%9F%98%80\t",
            json!(["preset-new", "0", "日本😀", ""]),
        ),
        ("%00%09%25%2B%22%5C", json!(["\0\t%+\"\\"])),
        ("%FF\t%G0\t%\t%2\t日本A", json!(["", "%G0", "%", "%2", "A"])),
    ];
    for (message, expected) in &cases {
        assert_eq!(json!(webview::decode_message(message)), *expected);
    }
    if let Some(reference) = std::env::var_os("PADMUX_CONFIG_REFERENCE") {
        let directory = std::env::temp_dir().join(format!(
            "padmux-ui-reference-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let input = directory.join("config.json");
        let output = directory.join("output.json");
        std::fs::write(&input, include_bytes!("config-reference.json")).unwrap();
        let result = std::process::Command::new(&reference)
            .arg("--ui-state")
            .arg(input)
            .arg(&output)
            .output()
            .unwrap();
        assert!(result.status.success(), "{result:?}");
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(&output).unwrap()).unwrap(),
            state
        );
        for (message, expected) in cases {
            let result = std::process::Command::new(&reference)
                .arg("--decode")
                .arg(message)
                .arg(&output)
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(&output).unwrap()).unwrap(),
                expected
            );
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
