//! Shared WebBridge.inc wire format/web assets derived from Remapcon (MIT).
//! Copyright (c) 2026 ro-ba; retain ../../LICENSE / THIRD_PARTY_NOTICES.md.
#![forbid(unsafe_code)]
use crate::config::Configuration;
use serde::Serialize;
use serde_json::json;

pub(crate) fn crt_space(ch: char) -> bool {
    // Microsoft CRT iswspace in the reference's default locale: Unicode White
    // Space plus U+180E. Compare the entire UTF-16 table in the native test.
    ch.is_whitespace() || ch == '\u{180e}'
}

pub fn window_placement(text: &str) -> Option<[i32; 5]> {
    // Match the reference's five %d conversions and literal commas. Whitespace
    // before a number and trailing text after the fifth number are accepted.
    // Reject integer overflow rather than inheriting scanf's undefined result.
    let mut remaining = text.split('\0').next()?;
    let mut result = [0; 5];
    for (index, output) in result.iter_mut().enumerate() {
        remaining = remaining.trim_start_matches(crt_space);
        let start = usize::from(remaining.starts_with(['+', '-']));
        let count = remaining.as_bytes()[start..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        if count == 0 {
            return None;
        }
        *output = remaining[..start + count].parse().ok()?;
        remaining = &remaining[start + count..];
        if index < 4 {
            remaining = remaining.strip_prefix(',')?;
        }
    }
    Some(result)
}

pub fn embedded_html(version: &str) -> String {
    include_str!("../../src/app/ui/index.html")
        .replace(
            "<!-- STYLE -->",
            &format!(
                "<style>{}</style>",
                include_str!("../../src/app/ui/style.css")
            ),
        )
        .replace(
            "<!-- SCRIPT -->",
            &format!(
                "<script>{}</script>",
                include_str!("../../src/app/ui/app.js")
            ),
        )
        .replace(
            "<!-- BRAND_ICON -->",
            include_str!("../../src/app/resources/padmux.svg"),
        )
        .replace("__PADMUX_VERSION__", version)
}

pub fn decode_message(message: &str) -> Vec<String> {
    message
        .split('\t')
        .map(|field| {
            let units = field.as_bytes();
            let mut bytes = Vec::new();
            let mut index = 0;
            while index < units.len() {
                let ch = units[index];
                if ch == b'%' && index + 2 < units.len() {
                    let hex = |c: u8| char::from(c).to_digit(16);
                    if let (Some(high), Some(low)) = (hex(units[index + 1]), hex(units[index + 2]))
                    {
                        bytes.push((high * 16 + low) as u8);
                        index += 3;
                        continue;
                    }
                }
                if ch <= 127 {
                    bytes.push(ch);
                }
                index += 1;
            }
            String::from_utf8(bytes).unwrap_or_default()
        })
        .collect()
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Runtime<'a> {
    pub edited_layer: usize,
    pub requested: bool,
    pub preview: bool,
    pub elevated: bool,
    pub status: &'a str,
}
pub const BUTTONS: [(&str, &str, u8); 32] = [
    ("A", "A", 0),
    ("B", "B", 0),
    ("X", "X", 0),
    ("Y", "Y", 0),
    ("LB", "LB", 0),
    ("RB", "RB", 0),
    ("LT", "LT", 0),
    ("RT", "RT", 0),
    ("L3", "L3", 0),
    ("R3", "R3", 0),
    ("↑", "DPadUp", 1),
    ("↓", "DPadDown", 1),
    ("←", "DPadLeft", 1),
    ("→", "DPadRight", 1),
    ("L4", "L4", 2),
    ("L5", "L5", 2),
    ("R4", "R4", 2),
    ("R5", "R5", 2),
    ("左クリック", "LeftPadClick", 3),
    ("左タップ", "LeftPadTap", 3),
    ("右クリック", "RightPadClick", 3),
    ("右タップ", "RightPadTap", 3),
    ("メニュー", "Menu", 4),
    ("ビュー", "View", 4),
    ("右スティック ↑", "RightStickUp", 5),
    ("右スティック ↓", "RightStickDown", 5),
    ("右スティック ←", "RightStickLeft", 5),
    ("右スティック →", "RightStickRight", 5),
    ("左スティック ↑", "LeftStickUp", 5),
    ("左スティック ↓", "LeftStickDown", 5),
    ("左スティック ←", "LeftStickLeft", 5),
    ("左スティック →", "LeftStickRight", 5),
];
pub fn state_json(config: &Configuration, runtime: Runtime<'_>) -> serde_json::Result<String> {
    // Reuse the config serializer, changing only the bridge's existing field
    // names. No additional copy of the owned configuration is retained.
    let mut value = serde_json::to_value(config)?;
    let state = value
        .as_object_mut()
        .expect("Configuration serializes as an object");
    state.remove("formatVersion");
    for preset in state.get_mut("presets").unwrap().as_array_mut().unwrap() {
        let preset = preset.as_object_mut().unwrap();
        let target = preset.remove("targetExecutable").unwrap();
        preset.insert("target".into(), target);
    }
    if let serde_json::Value::Object(fields) = serde_json::to_value(runtime)? {
        state.extend(fields);
    }
    state.insert("type".into(), json!("state"));
    state.insert("buttons".into(), BUTTONS.iter().map(|(name, setting, category)| json!({"name":name,"setting":setting,"category":category})).collect());
    state.insert(
        "categories".into(),
        json!([
            "基本ボタン",
            "十字キー",
            "背面ボタン",
            "トラックパッド",
            "メニュー・ビュー",
            "スティック"
        ]),
    );
    serde_json::to_string(&value)
}
