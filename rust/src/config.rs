//! Existing SettingsJson.inc / SimpleJson.h schema, translated from Remapcon.
//! Retain ../../LICENSE and ../../THIRD_PARTY_NOTICES.md (MIT, ro-ba/Dylan Deverill).
#![forbid(unsafe_code)]

use crate::mapping::{self, Button, Sequence, Turbo, BUTTON_COUNT, INHERIT, MOUSE_LEFT};
use serde::{de, Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    pub format_version: u32,
    pub language: String,
    pub close_behavior: u32,
    pub auto_mode: bool,
    #[serde(default)]
    pub steam_takeover: bool,
    pub selected_preset: usize,
    pub folders: Vec<String>,
    pub presets: Vec<Preset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub name: String,
    pub folder: String,
    pub target_executable: String,
    pub left_pad_mode: u32,
    pub right_pad_mode: u32,
    pub left_pad_sensitivity: u32,
    pub right_pad_sensitivity: u32,
    #[serde(default = "deadzone")]
    pub left_stick_deadzone: u32,
    #[serde(default = "deadzone")]
    pub right_stick_deadzone: u32,
    #[serde(default = "overlap")]
    pub left_stick_overlap: u32,
    #[serde(default = "overlap")]
    pub right_stick_overlap: u32,
    #[serde(flatten)]
    pub bindings: Bindings,
    pub layers: Vec<Layer>,
}
fn deadzone() -> u32 {
    12288
}
fn overlap() -> u32 {
    16000
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub trigger: usize,
    #[serde(flatten)]
    pub bindings: Bindings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Bindings {
    pub mapping: [u32; BUTTON_COUNT],
    pub turbo: [Turbo; BUTTON_COUNT],
    pub sequence: [Sequence; BUTTON_COUNT],
}
impl<'de> Deserialize<'de> for Bindings {
    fn deserialize<D: de::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Arrays {
            mapping: Vec<u32>,
            turbo: Vec<Turbo>,
            sequence: Vec<Sequence>,
        }
        let mut arrays = Arrays::deserialize(deserializer)?;
        let count = arrays.mapping.len();
        if ![28, BUTTON_COUNT].contains(&count)
            || arrays.turbo.len() != count
            || arrays.sequence.len() != count
        {
            return Err(de::Error::custom(
                "mapping/turbo/sequence must all contain 28 or 32 entries",
            ));
        }
        if count == 28 {
            // The four appended left-stick directions mirror the old D-pad
            // entries, including their owned sequence keys, exactly as C++.
            for index in Button::DPadUp as usize..=Button::DPadRight as usize {
                arrays.mapping.push(arrays.mapping[index]);
                arrays.turbo.push(arrays.turbo[index]);
                arrays.sequence.push(arrays.sequence[index].clone());
            }
        }
        Ok(Self {
            mapping: arrays
                .mapping
                .try_into()
                .map_err(|_| de::Error::custom("mapping size"))?,
            turbo: arrays
                .turbo
                .try_into()
                .map_err(|_| de::Error::custom("turbo size"))?,
            sequence: arrays
                .sequence
                .try_into()
                .map_err(|_| de::Error::custom("sequence size"))?,
        })
    }
}
impl Bindings {
    pub fn into_engine(self, trigger: usize) -> mapping::Layer {
        mapping::Layer {
            mapping: self.mapping,
            turbo: self.turbo,
            sequence: self.sequence,
            trigger,
        }
    }
    fn validate(&self, inherit: bool) -> Result<()> {
        for ((&key, turbo), sequence) in self.mapping.iter().zip(&self.turbo).zip(&self.sequence) {
            if !valid_key(key, inherit)
                || !(20..=2000).contains(&turbo.interval_ms)
                || turbo.delay_ms > 5000
                || !(20..=2000).contains(&sequence.interval_ms)
                || sequence.keys.len() > 16
                || (sequence.enabled && sequence.keys.is_empty())
                || sequence
                    .keys
                    .iter()
                    .any(|&key| key == 0 || !valid_key(key, false))
            {
                return Err("invalid mapping/turbo/sequence".into());
            }
        }
        Ok(())
    }
}
fn valid_key(key: u32, inherit: bool) -> bool {
    (inherit && key == INHERIT)
        || key == 0
        || (MOUSE_LEFT..=MOUSE_LEFT + 2).contains(&key)
        || (key & 0xff != 0 && key & !0x100ff == 0)
}
fn string_fits(text: &str, maximum: usize) -> bool {
    // C++ wchar_t lengths are UTF-16 code units, not UTF-8 bytes or scalar count.
    text.encode_utf16().count() <= maximum
}
impl Preset {
    pub fn into_engine(self, generation: u64) -> mapping::Config {
        mapping::Config {
            base: self.bindings.into_engine(BUTTON_COUNT),
            layers: self
                .layers
                .into_iter()
                .map(|layer| layer.bindings.into_engine(layer.trigger))
                .collect(),
            generation,
        }
    }
}
pub(crate) fn empty_bindings(inherit: bool) -> Bindings {
    Bindings {
        mapping: [if inherit { INHERIT } else { 0 }; BUTTON_COUNT],
        turbo: [Turbo::default(); BUTTON_COUNT],
        sequence: std::array::from_fn(|_| Sequence::default()),
    }
}
pub(crate) fn empty_preset(name: String, folder: String) -> Preset {
    Preset {
        name,
        folder,
        target_executable: String::new(),
        left_pad_mode: 0,
        right_pad_mode: 0,
        left_pad_sensitivity: 100,
        right_pad_sensitivity: 100,
        left_stick_deadzone: 12288,
        right_stick_deadzone: 12288,
        left_stick_overlap: 16000,
        right_stick_overlap: 16000,
        bindings: empty_bindings(false),
        layers: Vec::new(),
    }
}
impl Configuration {
    /// Defaults of C++ LoadSettings when neither current nor legacy files exist.
    pub fn initial() -> Self {
        Self {
            format_version: 1,
            language: "ja".into(),
            close_behavior: 0,
            auto_mode: false,
            steam_takeover: false,
            selected_preset: 0,
            folders: Vec::new(),
            presets: vec![empty_preset("標準".into(), String::new())],
        }
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err("config file must be 1..8 MiB".into());
        }
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        let value = parse_json(bytes)?;
        let config: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        config.validate()?;
        Ok(config)
    }
    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if bytes.len() > MAX_BYTES {
            return Err("config exceeds 8 MiB".into());
        }
        Ok(bytes)
    }
    fn validate(&self) -> Result<()> {
        if self.format_version != 1
            || !["ja", "en"].contains(&self.language.as_str())
            || self.close_behavior > 2
            || self.presets.is_empty()
            || self.presets.len() > 100
            || self.selected_preset > 99
            || self.selected_preset >= self.presets.len()
            || self.folders.len() > 100
            || self
                .folders
                .iter()
                .any(|s| s.is_empty() || !string_fits(s, 512))
        {
            return Err("invalid top-level configuration".into());
        }
        for preset in &self.presets {
            if preset.name.is_empty()
                || !string_fits(&preset.name, 128)
                || !string_fits(&preset.folder, 512)
                || !string_fits(&preset.target_executable, 520)
                || preset.left_pad_mode > 2
                || preset.right_pad_mode > 2
                || !(25..=400).contains(&preset.left_pad_sensitivity)
                || !(25..=400).contains(&preset.right_pad_sensitivity)
                || [
                    preset.left_stick_deadzone,
                    preset.right_stick_deadzone,
                    preset.left_stick_overlap,
                    preset.right_stick_overlap,
                ]
                .iter()
                .any(|&n| n > 32767)
                || preset.layers.len() > BUTTON_COUNT
            {
                return Err("invalid preset".into());
            }
            preset.bindings.validate(false)?;
            for layer in &preset.layers {
                if !string_fits(&layer.name, 64) || layer.trigger > BUTTON_COUNT {
                    return Err("invalid layer".into());
                }
                layer.bindings.validate(true)?;
            }
        }
        Ok(())
    }
}

// Match SimpleJson's 32-deep value limit, 10,000 members per container,
// unsigned u64 integer syntax and duplicate-key rejection using serde's parser.
struct StrictValue(u32);
impl<'de> de::DeserializeSeed<'de> for StrictValue {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(
        self,
        parser: D,
    ) -> std::result::Result<Value, D::Error> {
        if self.0 > 32 {
            return Err(de::Error::custom("JSON depth exceeds 32"));
        }
        parser.deserialize_any(self)
    }
}
impl<'de> de::Visitor<'de> for StrictValue {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("C++ compatible JSON value")
    }
    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Value, E> {
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: de::SeqAccess<'de>>(
        self,
        mut input: A,
    ) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = input.next_element_seed(StrictValue(self.0 + 1))? {
            if values.len() == 10000 {
                return Err(de::Error::custom("too many array entries"));
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: de::MapAccess<'de>>(
        self,
        mut input: A,
    ) -> std::result::Result<Value, A::Error> {
        let mut values = serde_json::Map::new();
        while let Some(key) = input.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON key"));
            }
            if values.len() == 10000 {
                return Err(de::Error::custom("too many object members"));
            }
            values.insert(key, input.next_value_seed(StrictValue(self.0 + 1))?);
        }
        Ok(Value::Object(values))
    }
}

// Shared exact SimpleJson grammar for configuration and updater release metadata.
pub(crate) fn parse_json(bytes: &[u8]) -> Result<Value> {
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    // Value's normal deserializer accepts duplicate keys and floating-point
    // numbers. C++ rejects these even in ignored fields; retain that boundary.
    let value =
        de::DeserializeSeed::deserialize(StrictValue(0), &mut parser).map_err(|e| e.to_string())?;
    parser.end().map_err(|e| e.to_string())?;
    Ok(value)
}
