//! Configuration commands from Remapcon WebBridge.inc / main.cpp (MIT).
//! Copyright (c) 2026 ro-ba; retain ../../LICENSE / THIRD_PARTY_NOTICES.md.
#![forbid(unsafe_code)]
use crate::{
    config::{empty_bindings, empty_preset, Bindings, Configuration, Layer, Preset},
    mapping::{Sequence, Turbo, BUTTON_COUNT, INHERIT, MOUSE_LEFT},
    webview::{self, Runtime},
};
use serde_json::json;

// Windows wcstoul is 32-bit: overflow returns ULONG_MAX, embedded NUL ends
// parsing, and a minus after leading whitespace is accepted by ParseNumber.
// Preserve these reference quirks rather than substituting Rust's strict parse.
pub fn parse_number(text: &str) -> Option<u32> {
    if text.is_empty() || text.starts_with('-') {
        return None;
    }
    let prefix = text.split('\0').next()?;
    if prefix.is_empty() {
        return Some(0);
    }
    let digits = prefix.trim_start_matches(crate::webview::crt_space);
    let negative = digits.starts_with('-');
    let digits = digits.strip_prefix(['+', '-']).unwrap_or(digits);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let parsed = digits.bytes().try_fold(0u32, |n, b| {
        n.checked_mul(10)?.checked_add(u32::from(b - b'0'))
    });
    Some(match parsed {
        Some(n) if negative => n.wrapping_neg(),
        Some(n) => n,
        None => u32::MAX,
    })
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.encode_utf16().count() <= 40
        && !name.contains(['/', '\\', '\r', '\n', '[', ']', '='])
}
fn empty_layer(name: String) -> Layer {
    Layer {
        name,
        trigger: BUTTON_COUNT,
        bindings: empty_bindings(true),
    }
}
struct History {
    config: Configuration,
    layer: usize,
}
enum Clipboard {
    Binding(u32, Turbo, Sequence),
    Layer(Box<Layer>),
    Pad(u32, u32),
    Stick([u32; 4]),
}
pub struct Editor {
    pub config: Configuration,
    pub edited_layer: usize,
    pub requested: bool,
    pub preview: bool,
    pub generation: u64,
    undo: Vec<History>,
    redo: Vec<History>,
    clipboard: Option<Clipboard>,
}
impl Editor {
    pub fn new(config: Configuration) -> Self {
        Self {
            config,
            edited_layer: 0,
            requested: false,
            preview: false,
            generation: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            clipboard: None,
        }
    }
    fn snapshot(&self) -> History {
        // Undo/rollback require an independent owned configuration, as in C++.
        History {
            config: self.config.clone(),
            layer: self.edited_layer,
        }
    }
    /// Apply a native picker result; cancellation leaves configuration/history intact.
    pub fn pick_target(
        &mut self,
        path: Option<String>,
        mut save: impl FnMut(&Configuration) -> bool,
    ) -> bool {
        let Some(path) = path else { return true };
        let before = self.snapshot();
        self.preset_mut().target_executable = path;
        if !save(&self.config) {
            self.config = before.config;
            return false;
        }
        self.generation += 1;
        if self.config != before.config {
            Self::push_history(&mut self.undo, before);
            self.redo.clear();
        }
        true
    }
    /// Native host validates input and backs up the current file before this call.
    pub fn import(
        &mut self,
        config: Configuration,
        mut save: impl FnMut(&Configuration) -> bool,
    ) -> bool {
        let previous = std::mem::replace(&mut self.config, config);
        let requested = std::mem::replace(&mut self.requested, false);
        self.generation += 1; // C++ ApplyConfig publishes even on rollback.
        if !save(&self.config) {
            self.config = previous;
            self.requested = requested;
            self.generation += 1;
            return false;
        }
        self.edited_layer = 0;
        if self.config != previous {
            self.undo.clear();
            self.redo.clear();
        }
        true
    }
    fn push_history(stack: &mut Vec<History>, value: History) {
        if stack.len() == 30 {
            stack.remove(0);
        }
        stack.push(value);
    }
    fn preset(&self) -> &Preset {
        &self.config.presets[self.config.selected_preset]
    }
    fn preset_mut(&mut self) -> &mut Preset {
        &mut self.config.presets[self.config.selected_preset]
    }
    fn bindings(&self) -> &Bindings {
        if self.edited_layer == 0 {
            &self.preset().bindings
        } else {
            &self.preset().layers[self.edited_layer - 1].bindings
        }
    }
    fn bindings_mut(&mut self) -> &mut Bindings {
        let layer = self.edited_layer;
        if layer == 0 {
            &mut self.preset_mut().bindings
        } else {
            &mut self.preset_mut().layers[layer - 1].bindings
        }
    }
    fn folder_exists(&self, name: &str) -> bool {
        name.is_empty() || self.config.folders.iter().any(|folder| folder == name)
    }
    fn name_available(
        &self,
        name: &str,
        folder: &str,
        except: Option<usize>,
        equal: &impl Fn(&str, &str) -> bool,
    ) -> bool {
        self.config
            .presets
            .iter()
            .enumerate()
            .all(|(index, preset)| {
                Some(index) == except || preset.folder != folder || !equal(&preset.name, name)
            })
    }
    fn set_binding(&mut self, button: usize, key: u32, turbo: Turbo, sequence: Sequence) {
        let bindings = self.bindings_mut();
        bindings.mapping[button] = key;
        bindings.turbo[button] = turbo;
        bindings.sequence[button] = sequence;
    }
    fn set_role(
        &mut self,
        button: usize,
        target: Option<usize>,
        key: u32,
        turbo: Turbo,
        sequence: Sequence,
    ) -> bool {
        let preset = self.preset_mut();
        let previous = preset
            .layers
            .iter()
            .position(|layer| layer.trigger == button);
        if let Some(mut target) = target {
            if target > preset.layers.len()
                || (target == preset.layers.len() && target >= BUTTON_COUNT)
            {
                return false;
            }
            if let Some(previous) = previous.filter(|&p| p != target) {
                preset.layers.remove(previous);
                if previous < target {
                    target -= 1;
                }
            }
            if target == preset.layers.len() {
                preset.layers.push(empty_layer(String::new()));
            }
            preset.layers[target].trigger = button;
        } else {
            if let Some(previous) = previous {
                preset.layers.remove(previous);
            }
            preset.bindings.mapping[button] = key;
            preset.bindings.turbo[button] = turbo;
            preset.bindings.sequence[button] = sequence;
        }
        true
    }
    fn restore_history(
        &mut self,
        undo: bool,
        save: &mut impl FnMut(&Configuration) -> bool,
    ) -> (bool, bool) {
        let previous = if undo {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        let Some(previous) = previous else {
            return (false, true);
        };
        let current = History {
            config: std::mem::replace(&mut self.config, previous.config),
            layer: self.edited_layer,
        };
        let requested = self.requested;
        self.edited_layer = if previous.layer <= self.preset().layers.len() {
            previous.layer
        } else {
            0
        };
        self.generation += 1;
        if self.config.auto_mode {
            self.requested = false;
        }
        if !save(&self.config) {
            let failed = History {
                config: std::mem::replace(&mut self.config, current.config),
                layer: self.edited_layer,
            };
            self.edited_layer = current.layer;
            self.requested = requested;
            self.generation += 1;
            if undo {
                self.undo.push(failed);
            } else {
                self.redo.push(failed);
            }
            return (false, false);
        }
        Self::push_history(if undo { &mut self.redo } else { &mut self.undo }, current);
        (true, false)
    }
    pub fn state(&self, elevated: bool, status: &str) -> serde_json::Result<String> {
        webview::state_json(
            &self.config,
            Runtime {
                edited_layer: self.edited_layer,
                requested: self.requested,
                preview: self.preview,
                elevated,
                status,
            },
        )
    }
    /// None delegates window/dialog/icon/update commands to the native host.
    /// Save and CRT name comparison are supplied at the platform boundary.
    pub fn handle(
        &mut self,
        raw: &str,
        mut save: impl FnMut(&Configuration) -> bool,
        equal: impl Fn(&str, &str) -> bool,
        elevated: bool,
        status: &str,
    ) -> Option<Vec<String>> {
        let args = webview::decode_message(raw);
        let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
        let number = |i: usize| args.get(i).and_then(|value| parse_number(value.as_str()));
        let command = arg(0);
        let mut responses = Vec::new();
        if matches!(command, "undo" | "redo") {
            let (okay, empty) = self.restore_history(command == "undo", &mut save);
            responses.push(
                json!({"type":"history", "action":command,"okay":okay,"empty":empty}).to_string(),
            );
            if okay {
                responses.push(self.state(elevated, status).unwrap());
            }
            return Some(responses);
        }
        if matches!(
            command,
            "ready"
                | "uiReady"
                | "windowDrag"
                | "windowMinimize"
                | "windowToggleMaximize"
                | "windowClose"
                | "closeToTray"
                | "quit"
                | "getTargetIcon"
                | "openReleases"
                | "installUpdate"
                | "pickTarget"
                | "import"
                | "export"
        ) {
            return None;
        }
        if matches!(command, "copyRow" | "pasteRow") {
            let before = self.snapshot();
            let selected = number(3) == Some(self.edited_layer as u32)
                && number(4) == Some(self.config.selected_preset as u32);
            let row = number(2).map(|n| n as usize);
            let result = match row.filter(|_| selected) {
                Some(row) if command == "copyRow" => self.copy_row(arg(1), row),
                Some(row) => self.paste_row(arg(1), row, &mut save),
                None => "incompatible",
            };
            if result == "pasted" {
                if self.config != before.config {
                    Self::push_history(&mut self.undo, before);
                    self.redo.clear();
                }
                responses.push(self.state(elevated, status).unwrap());
            }
            responses.push(json!({"type":"rowClipboard","result":result}).to_string());
            return Some(responses);
        }
        let before = self.snapshot();
        let mut persist = true;
        let mut publish = false;
        let mut rollback_layer = false;
        let selected = self.config.selected_preset;
        let okay = match command {
            "selectPreset" => {
                if let Some(index) = number(1)
                    .map(|n| n as usize)
                    .filter(|&n| n < self.config.presets.len())
                {
                    self.config.selected_preset = index;
                    persist = index != selected;
                    publish = persist;
                    true
                } else {
                    false
                }
            }
            "selectLayer" => {
                persist = false;
                if let Some(layer) = number(1)
                    .map(|n| n as usize)
                    .filter(|&n| n <= self.preset().layers.len())
                {
                    self.edited_layer = layer;
                    true
                } else {
                    false
                }
            }
            "toggleMode" => {
                persist = false;
                if !self.config.auto_mode
                    && (self.requested || !self.preset().target_executable.is_empty())
                {
                    self.requested = !self.requested;
                    true
                } else {
                    false
                }
            }
            "setAuto" => {
                self.config.auto_mode = arg(1) == "1";
                true
            }
            "setPreview" => {
                persist = false;
                self.preview = arg(1) == "1";
                true
            }
            "setSteamTakeover" => {
                self.config.steam_takeover = arg(1) == "1";
                true
            }
            "setLanguage" => {
                self.config.language = if arg(1) == "en" { "en" } else { "ja" }.into();
                true
            }
            "setCloseBehavior" => {
                if let Some(value) = number(1).filter(|&n| n <= 2) {
                    self.config.close_behavior = value;
                    true
                } else {
                    false
                }
            }
            "setPadConfig" => {
                if let (Some(l), Some(r), Some(ls), Some(rs)) =
                    (number(1), number(2), number(3), number(4))
                {
                    if l <= 2 && r <= 2 && (25..=400).contains(&ls) && (25..=400).contains(&rs) {
                        let preset = self.preset_mut();
                        preset.left_pad_mode = l;
                        preset.right_pad_mode = r;
                        preset.left_pad_sensitivity = ls;
                        preset.right_pad_sensitivity = rs;
                        publish = true;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "setStickConfig" => {
                if let (Some(l), Some(r), Some(lo), Some(ro)) =
                    (number(1), number(2), number(3), number(4))
                {
                    if [l, r, lo, ro].iter().all(|&n| n <= 32767) {
                        let preset = self.preset_mut();
                        preset.left_stick_deadzone = l;
                        preset.right_stick_deadzone = r;
                        preset.left_stick_overlap = lo;
                        preset.right_stick_overlap = ro;
                        publish = true;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "setPadModes" => {
                if let (Some(l), Some(r)) = (number(1), number(2)) {
                    if l <= 2 && r <= 2 {
                        let preset = self.preset_mut();
                        preset.left_pad_mode = l;
                        preset.right_pad_mode = r;
                        publish = true;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "setPadMode" => {
                if let Some(mode) = number(2).filter(|&n| n <= 2) {
                    match arg(1) {
                        "left" => {
                            self.preset_mut().left_pad_mode = mode;
                            publish = true;
                            true
                        }
                        "right" => {
                            self.preset_mut().right_pad_mode = mode;
                            publish = true;
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            "clearTarget" => {
                self.preset_mut().target_executable.clear();
                publish = true;
                true
            }
            "preset-new" | "preset-copy" | "preset-rename" => {
                let source = number(1).map(|n| n as usize);
                let name = arg(2);
                let folder = arg(3);
                if valid_name(name)
                    && self.folder_exists(folder)
                    && (command == "preset-new"
                        || source.is_some_and(|n| n < self.config.presets.len()))
                    && (command == "preset-rename" || self.config.presets.len() < 100)
                    && self.name_available(
                        name,
                        folder,
                        if command == "preset-rename" {
                            source
                        } else {
                            None
                        },
                        &equal,
                    )
                {
                    if command == "preset-rename" {
                        let preset = &mut self.config.presets[source.unwrap()];
                        preset.name = name.into();
                        preset.folder = folder.into();
                    } else {
                        let mut preset = if command == "preset-copy" {
                            self.config.presets[source.unwrap()].clone()
                        } else {
                            empty_preset(name.into(), folder.into())
                        };
                        preset.name = name.into();
                        preset.folder = folder.into();
                        self.config.presets.push(preset);
                        self.config.selected_preset = self.config.presets.len() - 1;
                        self.edited_layer = 0;
                    }
                    publish = true;
                    true
                } else {
                    false
                }
            }
            "deletePreset" => {
                if let Some(index) = number(1)
                    .map(|n| n as usize)
                    .filter(|&n| n < self.config.presets.len() && self.config.presets.len() > 1)
                {
                    self.config.presets.remove(index);
                    if selected > index {
                        self.config.selected_preset -= 1;
                    } else if selected >= self.config.presets.len() {
                        self.config.selected_preset = self.config.presets.len() - 1;
                    }
                    self.edited_layer = 0;
                    publish = true;
                    true
                } else {
                    false
                }
            }
            "movePreset" | "movePresetNewFolder" => {
                if let Some(index) = number(1)
                    .map(|n| n as usize)
                    .filter(|&n| n < self.config.presets.len())
                {
                    let mut destination = arg(2).to_owned();
                    if command == "movePresetNewFolder" {
                        let name = arg(3);
                        if valid_name(name)
                            && self.folder_exists(arg(2))
                            && self.config.folders.len() < 100
                        {
                            destination = if arg(2).is_empty() {
                                name.into()
                            } else {
                                format!("{}/{name}", arg(2))
                            };
                            if !self.folder_exists(&destination) {
                                self.config.folders.push(destination.clone());
                            } else {
                                destination.clear();
                            }
                        } else {
                            destination.clear();
                        }
                    }
                    if self.folder_exists(&destination)
                        && (command == "movePreset" || !destination.is_empty())
                        && self.name_available(
                            &self.config.presets[index].name,
                            &destination,
                            Some(index),
                            &equal,
                        )
                    {
                        self.config.presets[index].folder = destination;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "folder-new" | "folder-rename" => {
                let target = arg(1);
                let name = arg(2);
                let path = if target.is_empty() {
                    name.into()
                } else if command == "folder-new" {
                    format!("{target}/{name}")
                } else {
                    format!("{}{name}", target.rfind('/').map_or("", |i| &target[..=i]))
                };
                if valid_name(name)
                    && self.folder_exists(target)
                    && !self.folder_exists(&path)
                    && self.config.folders.len() < 100
                {
                    if command == "folder-new" {
                        self.config.folders.push(path);
                    } else {
                        let descendant = format!("{target}/");
                        for folder in &mut self.config.folders {
                            if folder == target || folder.starts_with(&descendant) {
                                *folder = format!("{path}{}", &folder[target.len()..]);
                            }
                        }
                        for preset in &mut self.config.presets {
                            if preset.folder == target || preset.folder.starts_with(&descendant) {
                                preset.folder = format!("{path}{}", &preset.folder[target.len()..]);
                            }
                        }
                    }
                    true
                } else {
                    false
                }
            }
            "deleteFolder" => {
                let target = arg(1);
                let descendant = format!("{target}/");
                if !target.is_empty()
                    && self.folder_exists(target)
                    && !self
                        .config
                        .folders
                        .iter()
                        .any(|n| n.starts_with(&descendant))
                    && !self.config.presets.iter().any(|p| p.folder == target)
                {
                    self.config.folders.retain(|n| n != target);
                    true
                } else {
                    false
                }
            }
            "layer-new" | "layer-rename" | "layer-delete" => {
                let index = number(1).map(|n| n as usize);
                let name = arg(2);
                rollback_layer = true;
                if (command == "layer-delete" || valid_name(name))
                    && (command == "layer-new" || index.is_some())
                {
                    if command == "layer-new" && self.preset().layers.len() < BUTTON_COUNT {
                        self.preset_mut().layers.push(empty_layer(name.into()));
                        self.edited_layer = self.preset().layers.len();
                        publish = true;
                        true
                    } else if command == "layer-rename"
                        && index.unwrap() < self.preset().layers.len()
                    {
                        self.preset_mut().layers[index.unwrap()].name = name.into();
                        publish = true;
                        true
                    } else if command == "layer-delete"
                        && index.unwrap() < self.preset().layers.len()
                    {
                        let index = index.unwrap();
                        self.preset_mut().layers.remove(index);
                        if self.edited_layer == index + 1 {
                            self.edited_layer = 0;
                        } else if self.edited_layer > index + 1 {
                            self.edited_layer -= 1;
                        }
                        publish = true;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "resetBinding" => {
                if let (Some(button), Some(layer), Some(preset)) = (number(1), number(2), number(3))
                {
                    let button = button as usize;
                    let layer = layer as usize;
                    if preset as usize == selected
                        && button < BUTTON_COUNT
                        && layer <= self.preset().layers.len()
                        && layer == self.edited_layer
                    {
                        if layer == 0 && self.preset().layers.iter().any(|l| l.trigger == button) {
                            self.set_role(button, None, 0, Turbo::default(), Sequence::default());
                        } else {
                            self.set_binding(
                                button,
                                if layer == 0 { 0 } else { INHERIT },
                                Turbo::default(),
                                Sequence::default(),
                            );
                        }
                        publish = true;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "saveBinding" => {
                let okay = self.save_binding(&args);
                publish = okay;
                okay
            }
            _ => {
                persist = false;
                false
            }
        };
        let okay = okay && (!persist || save(&self.config));
        if !okay {
            self.config = before.config;
            if rollback_layer {
                self.edited_layer = before.layer;
            }
        } else {
            if publish {
                self.generation += 1;
            }
            if command == "selectPreset" {
                self.edited_layer = 0;
                if selected != self.config.selected_preset
                    && self.preset().target_executable.is_empty()
                {
                    self.requested = false;
                }
            }
            if (command == "setAuto" && self.config.auto_mode) || command == "clearTarget" {
                self.requested = false;
            }
            if !matches!(
                command,
                "selectPreset" | "selectLayer" | "toggleMode" | "setPreview"
            ) && self.config != before.config
            {
                Self::push_history(&mut self.undo, before);
                self.redo.clear();
            }
        }
        if matches!(command, "selectPreset" | "selectLayer") && okay {
            responses.push(json!({"type":"selection","selectedPreset":self.config.selected_preset,"editedLayer":self.edited_layer,"requested":self.requested}).to_string());
        } else {
            if !okay {
                responses.push(json!({"type":"error","text":if matches!(command,"selectPreset"|"selectLayer"){"選択を変更できませんでした。"}else{"設定を保存できませんでした。入力内容を確認してください。"}}).to_string());
            }
            responses.push(self.state(elevated, status).unwrap());
        }
        Some(responses)
    }
    fn save_binding(&mut self, args: &[String]) -> bool {
        let arg = |i| args.get(i).map(String::as_str).unwrap_or("");
        let number = |i: usize| args.get(i).and_then(|s| parse_number(s.as_str()));
        let (
            Some(button),
            Some(layer),
            Some(mut key),
            Some(interval),
            Some(delay),
            Some(seq_interval),
            Some(target),
        ) = (
            number(1),
            number(2),
            number(4),
            number(6),
            number(7),
            number(9),
            number(11),
        )
        else {
            return false;
        };
        let button = button as usize;
        let layer = layer as usize;
        let role = arg(3);
        if button >= BUTTON_COUNT
            || layer > self.preset().layers.len()
            || !(20..=2000).contains(&interval)
            || delay > 5000
            || !(20..=2000).contains(&seq_interval)
        {
            return false;
        }
        let valid_key =
            |key: u32| key != 0 && key != INHERIT && (key & 0xff != 0 && key & !0x100ff == 0);
        if (role == "mouse"
            && (!(18..=21).contains(&button) || !(MOUSE_LEFT..=MOUSE_LEFT + 2).contains(&key)))
            || (role == "key" && (!valid_key(key) || key >= MOUSE_LEFT))
        {
            return false;
        }
        let turbo = Turbo {
            enabled: arg(5) == "1",
            interval_ms: interval,
            delay_ms: delay,
        };
        let mut sequence = Sequence {
            enabled: role == "sequence",
            repeat: arg(10) == "1",
            interval_ms: seq_interval,
            keys: Vec::new(),
        };
        if sequence.enabled {
            for item in arg(8).split(',').take(16) {
                let Some(item) = parse_number(item).filter(|&n| valid_key(n) && n < MOUSE_LEFT)
                else {
                    return false;
                };
                sequence.keys.push(item);
            }
            if sequence.keys.is_empty() {
                return false;
            }
        }
        self.edited_layer = layer;
        if role == "off" || role == "sequence" {
            key = 0;
        }
        if role == "inherit" && layer > 0 {
            key = INHERIT;
        }
        let okay = if role == "layer" && layer == 0 {
            self.set_role(
                button,
                Some(target as usize),
                0,
                Turbo::default(),
                Sequence::default(),
            )
        } else if matches!(role, "key" | "mouse" | "off" | "inherit" | "sequence") {
            if layer == 0 && self.preset().layers.iter().any(|l| l.trigger == button) {
                self.set_role(button, None, key, turbo, sequence)
            } else {
                self.set_binding(button, key, turbo, sequence);
                true
            }
        } else {
            false
        };
        if self.edited_layer > self.preset().layers.len() {
            self.edited_layer = 0;
        }
        okay
    }
    fn copy_row(&mut self, kind: &str, index: usize) -> &'static str {
        if self.edited_layer > self.preset().layers.len() {
            return "incompatible";
        }
        let clipboard = match kind {
            "binding" if index < BUTTON_COUNT => {
                if let Some(layer) = self
                    .preset()
                    .layers
                    .iter()
                    .find(|l| self.edited_layer == 0 && l.trigger == index)
                {
                    Clipboard::Layer(Box::new(layer.clone()))
                } else {
                    let binding = self.bindings();
                    Clipboard::Binding(
                        binding.mapping[index],
                        binding.turbo[index],
                        binding.sequence[index].clone(),
                    )
                }
            }
            "pad" if index <= 1 => {
                let p = self.preset();
                if index == 0 {
                    Clipboard::Pad(p.left_pad_mode, p.left_pad_sensitivity)
                } else {
                    Clipboard::Pad(p.right_pad_mode, p.right_pad_sensitivity)
                }
            }
            "stick" if index == 0 => {
                let p = self.preset();
                Clipboard::Stick([
                    p.left_stick_deadzone,
                    p.right_stick_deadzone,
                    p.left_stick_overlap,
                    p.right_stick_overlap,
                ])
            }
            _ => return "incompatible",
        };
        self.clipboard = Some(clipboard);
        "copied"
    }
    fn paste_row(
        &mut self,
        kind: &str,
        index: usize,
        save: &mut impl FnMut(&Configuration) -> bool,
    ) -> &'static str {
        let Some(clipboard) = self.clipboard.take() else {
            return "empty";
        };
        let previous = self.preset().clone();
        let compatible = match (&clipboard, kind) {
            (Clipboard::Layer(layer), "binding")
                if index < BUTTON_COUNT && self.edited_layer == 0 =>
            {
                let mut layer = (**layer).clone();
                layer.trigger = index;
                let preset = self.preset_mut();
                if let Some(destination) = preset.layers.iter_mut().find(|l| l.trigger == index) {
                    *destination = layer;
                    true
                } else if preset.layers.len() < BUTTON_COUNT {
                    preset.layers.push(layer);
                    true
                } else {
                    false
                }
            }
            (Clipboard::Binding(key, turbo, sequence), "binding")
                if index < BUTTON_COUNT
                    && self.edited_layer <= self.preset().layers.len()
                    && !(*key == INHERIT && self.edited_layer == 0)
                    && !((MOUSE_LEFT..=MOUSE_LEFT + 2).contains(key)
                        && !(18..=21).contains(&index)) =>
            {
                if self.edited_layer == 0 {
                    if let Some(destination) =
                        self.preset().layers.iter().position(|l| l.trigger == index)
                    {
                        self.preset_mut().layers.remove(destination);
                    }
                }
                self.set_binding(index, *key, *turbo, sequence.clone());
                true
            }
            (Clipboard::Pad(mode, sensitivity), "pad") if index <= 1 => {
                let p = self.preset_mut();
                if index == 0 {
                    p.left_pad_mode = *mode;
                    p.left_pad_sensitivity = *sensitivity;
                } else {
                    p.right_pad_mode = *mode;
                    p.right_pad_sensitivity = *sensitivity;
                }
                true
            }
            (Clipboard::Stick([l, r, lo, ro]), "stick") if index == 0 => {
                let p = self.preset_mut();
                p.left_stick_deadzone = *l;
                p.right_stick_deadzone = *r;
                p.left_stick_overlap = *lo;
                p.right_stick_overlap = *ro;
                true
            }
            _ => false,
        };
        self.clipboard = Some(clipboard);
        if !compatible {
            return "incompatible";
        }
        if !save(&self.config) {
            *self.preset_mut() = previous;
            return "saveFailed";
        }
        self.generation += 1;
        "pasted"
    }
}
