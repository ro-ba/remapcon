//! Mapping behavior translated from Remapcon src/app/main.cpp.
//! Pad press threshold derives from SteamlessController; retain ../../LICENSE
//! and ../../THIRD_PARTY_NOTICES.md (Dylan Deverill / ro-ba, MIT).
#![forbid(unsafe_code)]

use crate::controller::State;
use std::{collections::BTreeMap, time::Duration};

pub const BUTTON_COUNT: usize = 32;
pub const INHERIT: u32 = u32::MAX;
pub const MOUSE_LEFT: u32 = 0x20001;

// Indices are the existing JSON/UI button order, including the four appended
// left-stick directions. Do not reorder them during migration.
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Button {
    A,
    B,
    X,
    Y,
    LB,
    RB,
    LT,
    RT,
    L3,
    R3,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    L4,
    L5,
    R4,
    R5,
    LeftPadClick,
    LeftPadTap,
    RightPadClick,
    RightPadTap,
    Menu,
    View,
    RightStickUp,
    RightStickDown,
    RightStickLeft,
    RightStickRight,
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
}

#[derive(Default)]
struct PadPress {
    pressed: bool,
    above: u8,
    below: u8,
}
impl PadPress {
    fn update(&mut self, area: u16, click: bool) -> bool {
        if area >= 1800 || click {
            // Two consecutive reports suffice; saturate instead of overflowing
            // C++'s counter during long, uninterrupted sessions.
            self.above = self.above.saturating_add(1);
            self.below = 0;
        } else {
            self.below = self.below.saturating_add(1);
            self.above = 0;
        }
        if !self.pressed && self.above >= 2 {
            self.pressed = true;
        } else if self.pressed && self.below >= 2 {
            self.pressed = false;
        }
        self.pressed
    }
}

#[derive(Default)]
struct Tap {
    touching: bool,
    clicked: bool,
    start: [i16; 2],
    max_travel_squared: i64,
    started: Duration,
    pulse_until: Duration,
}
impl Tap {
    fn update(&mut self, contact: bool, click: bool, xy: [i16; 2], now: Duration) {
        if contact && !self.touching {
            self.started = now;
            self.start = xy;
            self.max_travel_squared = 0;
            self.clicked = false;
        }
        if contact {
            self.clicked |= click;
            let dx = i64::from(xy[0]) - i64::from(self.start[0]);
            let dy = i64::from(xy[1]) - i64::from(self.start[1]);
            self.max_travel_squared = self.max_travel_squared.max(dx * dx + dy * dy);
        } else if self.touching
            && !self.clicked
            && now.saturating_sub(self.started) <= Duration::from_millis(200)
            && self.max_travel_squared <= 3000 * 3000
        {
            self.pulse_until = now + Duration::from_millis(50);
        }
        self.touching = contact;
    }
}

#[derive(Default)]
struct Stick {
    directions: [bool; 4], // up, down, left, right
    active: bool,
}
impl Stick {
    fn update(&mut self, [x, y]: [i16; 2], deadzone: u32, overlap: u32) {
        let ax = i32::from(x).abs();
        let ay = i32::from(y).abs();
        let threshold = if self.active {
            deadzone.saturating_sub(1500)
        } else {
            deadzone
        };
        self.active = i64::from(ax) * i64::from(ax) + i64::from(ay) * i64::from(ay)
            >= i64::from(threshold) * i64::from(threshold)
            && (ax != 0 || ay != 0);
        if !self.active {
            self.directions = [false; 4];
            return;
        }
        let [up, down, left, right] = self.directions;
        let was_diagonal = (up || down) && (left || right);
        let diagonal = ax.min(ay) >= 1500.max(ax.max(ay) / 10)
            && (ax - ay).abs() <= overlap as i32 + if was_diagonal { 700 } else { 0 };
        let horizontal = ax >= ay || diagonal;
        let vertical = ay >= ax || diagonal;
        self.directions = [
            vertical && y > 0,
            vertical && y < 0,
            horizontal && x < 0,
            horizontal && x > 0,
        ];
    }
}

#[derive(Default)]
pub struct Physical {
    pub buttons: [bool; BUTTON_COUNT],
    presses: [PadPress; 2],
    taps: [Tap; 2],
    sticks: [Stick; 2],
}
impl Physical {
    /// Same short-report retention/reset behavior as UpdatePhysical.
    /// Settings come from the existing validated 0..=32767 config range.
    pub fn update(&mut self, state: &State, deadzone: [u32; 2], overlap: [u32; 2], now: Duration) {
        use Button::*;
        for (button, byte, mask) in [
            (A, 0, 1),
            (B, 0, 2),
            (X, 0, 4),
            (Y, 0, 8),
            (LB, 2, 8),
            (RB, 1, 2),
            (L3, 1, 0x80),
            (R3, 0, 0x20),
            (DPadUp, 1, 0x20),
            (DPadDown, 1, 4),
            (DPadLeft, 1, 0x10),
            (DPadRight, 1, 8),
            (L4, 2, 2),
            (L5, 2, 4),
            (R4, 0, 0x80),
            (R5, 1, 1),
            (Menu, 0, 0x40),
            (View, 1, 0x40),
        ] {
            self.buttons[button as usize] = state.flags[byte] & mask != 0;
        }
        for (side, button) in [LT, RT].into_iter().enumerate() {
            let index = button as usize;
            self.buttons[index] =
                state.triggers[side] > if self.buttons[index] { 0x1800 } else { 0x2000 };
        }
        for side in 0..2 {
            let start = if side == 0 { LeftStickUp } else { RightStickUp } as usize;
            if let Some(sticks) = state.sticks {
                self.sticks[side].update(sticks[side], deadzone[side], overlap[side]);
            } else {
                self.sticks[side] = Stick::default();
            }
            self.buttons[start..start + 4].copy_from_slice(&self.sticks[side].directions);
            let click_index = if side == 0 {
                LeftPadClick
            } else {
                RightPadClick
            } as usize;
            let tap_index = if side == 0 { LeftPadTap } else { RightPadTap } as usize;
            let (byte, click_mask, touch_mask) = if side == 0 {
                (3, 4, 2)
            } else {
                (2, 0x40, 0x20)
            };
            let click_bit = state.flags[byte] & click_mask != 0;
            self.buttons[click_index] = if let Some(pads) = &state.pads {
                self.presses[side].update(pads[side].area, click_bit)
            } else {
                click_bit
            };
            if let Some(pads) = &state.pads {
                self.taps[side].update(
                    state.flags[byte] & touch_mask != 0,
                    self.buttons[click_index],
                    [pads[side].x, pads[side].y],
                    now,
                );
            }
            self.buttons[tap_index] = now < self.taps[side].pulse_until;
        }
    }

    pub fn update_tap_pulses(&mut self, now: Duration) {
        self.buttons[Button::LeftPadTap as usize] = now < self.taps[0].pulse_until;
        self.buttons[Button::RightPadTap as usize] = now < self.taps[1].pulse_until;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Turbo {
    pub enabled: bool,
    pub interval_ms: u32,
    pub delay_ms: u32,
}
impl Default for Turbo {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: 50,
            delay_ms: 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub enabled: bool,
    pub repeat: bool,
    pub interval_ms: u32,
    pub keys: Vec<u32>,
}
impl Default for Sequence {
    fn default() -> Self {
        Self {
            enabled: false,
            repeat: true,
            interval_ms: 50,
            keys: Vec::new(),
        }
    }
}
pub struct Layer {
    pub mapping: [u32; BUTTON_COUNT],
    pub turbo: [Turbo; BUTTON_COUNT],
    pub sequence: [Sequence; BUTTON_COUNT],
    pub trigger: usize,
}
impl Default for Layer {
    fn default() -> Self {
        Self {
            mapping: [INHERIT; BUTTON_COUNT],
            turbo: [Turbo::default(); BUTTON_COUNT],
            sequence: std::array::from_fn(|_| Sequence::default()),
            trigger: BUTTON_COUNT,
        }
    }
}
#[derive(Default)]
pub struct Config {
    pub base: Layer,
    pub layers: Vec<Layer>,
    /// Changing the selected preset or mapping increments this, as in C++.
    pub generation: u64,
}
impl Config {
    fn validate(&self) -> Result<(), &'static str> {
        for layer in std::iter::once(&self.base).chain(&self.layers) {
            for (turbo, sequence) in layer.turbo.iter().zip(&layer.sequence) {
                if !(20..=2000).contains(&turbo.interval_ms)
                    || turbo.delay_ms > 5000
                    || !(20..=2000).contains(&sequence.interval_ms)
                {
                    return Err("mapping timing is outside the existing config range");
                }
            }
        }
        Ok(())
    }
}

/// The driver executes these and reports SendInput success for Key. Waiting is
/// used only for the existing three-attempt ReleaseAll cleanup; no OS code here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Key { key: u32, down: bool },
    Move { x: i32, y: i32 },
    Wheel { amount: i32, horizontal: bool },
    Wait(Duration),
    Status(Status),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Sent,
    Failed,
}

#[derive(Default)]
struct Held {
    references: usize,
    sent: bool,
    failure_reported: bool,
    next_repeat: Duration,
}
#[derive(Default, Clone, Copy)]
struct TurboState {
    armed: bool,
    key: u32,
    settings: Turbo,
    started: Duration,
}
#[derive(Default, Clone, Copy)]
struct SequenceState {
    armed: bool,
    source_layer: usize,
    started: Duration,
}
pub struct Engine {
    held: BTreeMap<u32, Held>,
    active: [u32; BUTTON_COUNT],
    turbo: [TurboState; BUTTON_COUNT],
    sequence: [SequenceState; BUTTON_COUNT],
    generation: u64,
    repeat_delay: Duration,
    repeat_interval: Duration,
}
impl Engine {
    /// Repeat values are queried from Windows by the driver, just as in C++.
    pub fn new(repeat_delay: Duration, repeat_interval: Duration) -> Self {
        Self {
            held: BTreeMap::new(),
            active: [0; BUTTON_COUNT],
            turbo: [TurboState::default(); BUTTON_COUNT],
            sequence: [SequenceState::default(); BUTTON_COUNT],
            generation: 0,
            repeat_delay,
            repeat_interval,
        }
    }
    pub fn needs_fast_read(&self) -> bool {
        self.turbo.iter().any(|s| s.armed) || self.sequence.iter().any(|s| s.armed)
    }
    pub fn release_all(&mut self, send: &mut impl FnMut(Output) -> bool) {
        for _ in 0..3 {
            let mut failed = false;
            for (&key, state) in &self.held {
                if state.sent && !send(Output::Key { key, down: false }) {
                    failed = true;
                }
            }
            if !failed {
                break;
            }
            send(Output::Wait(Duration::from_millis(20)));
        }
        self.held.clear();
        self.active.fill(0);
    }
    pub fn reset(&mut self, send: &mut impl FnMut(Output) -> bool) {
        self.release_all(send);
        self.turbo.fill(TurboState::default());
        self.sequence.fill(SequenceState::default());
    }
    pub fn update(
        &mut self,
        physical: &[bool; BUTTON_COUNT],
        enabled: bool,
        config: &Config,
        now: Duration,
        send: &mut impl FnMut(Output) -> bool,
    ) -> Result<(), &'static str> {
        config.validate()?;
        if config.generation != self.generation {
            self.reset(send);
            self.generation = config.generation;
        }
        let mut wanted = [0; BUTTON_COUNT];
        let mut turbo_active = [false; BUTTON_COUNT];
        let mut sequence_active = [false; BUTTON_COUNT];
        let mut continuous = BTreeMap::<u32, usize>::new();
        if enabled {
            for i in 0..BUTTON_COUNT {
                if !physical[i] || config.layers.iter().any(|layer| layer.trigger == i) {
                    continue;
                }
                let mut selected = &config.base;
                let mut source_layer = 0;
                for (index, layer) in config.layers.iter().enumerate() {
                    if physical.get(layer.trigger).copied().unwrap_or(false)
                        && (layer.mapping[i] != INHERIT || layer.sequence[i].enabled)
                    {
                        selected = layer;
                        source_layer = index + 1;
                    }
                }
                let key = if selected.mapping[i] == INHERIT {
                    0
                } else {
                    selected.mapping[i]
                };
                let turbo = selected.turbo[i];
                let sequence = &selected.sequence[i];
                if sequence.enabled && !sequence.keys.is_empty() {
                    sequence_active[i] = true;
                    let state = &mut self.sequence[i];
                    if !state.armed || state.source_layer != source_layer {
                        *state = SequenceState {
                            armed: true,
                            source_layer,
                            started: now,
                        };
                    }
                    let elapsed = now.saturating_sub(state.started).as_millis() as u64;
                    let mut step = elapsed / u64::from(sequence.interval_ms);
                    if sequence.repeat {
                        step %= sequence.keys.len() as u64;
                    }
                    if step < sequence.keys.len() as u64
                        && elapsed % u64::from(sequence.interval_ms)
                            < u64::from(sequence.interval_ms / 2)
                    {
                        wanted[i] = sequence.keys[step as usize];
                    }
                } else if key != 0 && turbo.enabled {
                    turbo_active[i] = true;
                    let state = &mut self.turbo[i];
                    if !state.armed
                        || state.key != key
                        || state.settings.interval_ms != turbo.interval_ms
                        || state.settings.delay_ms != turbo.delay_ms
                    {
                        *state = TurboState {
                            armed: true,
                            key,
                            settings: turbo,
                            started: now,
                        };
                    }
                    let elapsed = now.saturating_sub(state.started).as_millis() as u64;
                    if elapsed >= u64::from(turbo.delay_ms) {
                        // Preserve C++'s uint32_t cast before the modulo.
                        let phase =
                            (elapsed - u64::from(turbo.delay_ms)) as u32 % turbo.interval_ms;
                        if phase < turbo.interval_ms / 2 {
                            wanted[i] = key;
                        }
                    }
                } else {
                    self.turbo[i] = TurboState::default();
                    wanted[i] = key;
                    if key != 0 {
                        *continuous.entry(key).or_default() += 1;
                    }
                }
            }
        }
        for i in 0..BUTTON_COUNT {
            if !turbo_active[i] {
                self.turbo[i] = TurboState::default();
            }
            if !sequence_active[i] {
                self.sequence[i] = SequenceState::default();
            }
            if self.active[i] != 0 && self.active[i] != wanted[i] {
                if let Some(held) = self.held.get_mut(&self.active[i]) {
                    held.references = held.references.saturating_sub(1);
                }
                self.active[i] = 0;
            }
        }
        for (active, wanted) in self.active.iter_mut().zip(wanted) {
            if *active == 0 && wanted != 0 {
                self.held.entry(wanted).or_default().references += 1;
                *active = wanted;
            }
        }
        // Preserve both C++ release passes, including a retry in the second
        // pass if the first key-up failed. All releases precede replacements.
        self.held.retain(|&key, state| {
            state.references != 0 || (state.sent && !send(Output::Key { key, down: false }))
        });
        self.held.retain(|&key, state| {
            if state.references == 0 {
                if !state.sent || send(Output::Key { key, down: false }) {
                    return false;
                }
            } else if !state.sent {
                if send(Output::Key { key, down: true }) {
                    state.sent = true;
                    state.failure_reported = false;
                    state.next_repeat = now + self.repeat_delay;
                    send(Output::Status(Status::Sent));
                } else if !state.failure_reported {
                    state.failure_reported = true;
                    send(Output::Status(Status::Failed));
                }
            } else if key < MOUSE_LEFT && continuous.contains_key(&key) && now >= state.next_repeat
            {
                if send(Output::Key { key, down: true }) {
                    if state.failure_reported {
                        state.failure_reported = false;
                        send(Output::Status(Status::Sent));
                    }
                } else if !state.failure_reported {
                    state.failure_reported = true;
                    send(Output::Status(Status::Failed));
                }
                state.next_repeat = now + self.repeat_interval;
            }
            true
        });
        Ok(())
    }
}

#[derive(Default)]
pub struct PadMotion {
    previous_mode: u32,
    touching: bool,
    xy: [i16; 2],
    remainder: [f32; 2],
    haptic_travel: i32,
}
impl PadMotion {
    pub fn update(
        &mut self,
        mode: u32,
        sensitivity: u32,
        contact: bool,
        xy: [i16; 2],
    ) -> (Vec<Output>, bool) {
        if mode != self.previous_mode {
            self.touching = false;
            self.remainder = [0.0; 2];
            self.haptic_travel = 0;
            self.previous_mode = mode;
        }
        if mode == 0 || !contact {
            self.touching = false;
            self.remainder = [0.0; 2];
            self.haptic_travel = 0;
            return (Vec::new(), false);
        }
        let mut events = Vec::new();
        let mut tick = false;
        if self.touching {
            let dx = i32::from(xy[0]) - i32::from(self.xy[0]);
            let dy = i32::from(self.xy[1]) - i32::from(xy[1]);
            if dx.abs() < 12000 && dy.abs() < 12000 {
                self.haptic_travel += dx.abs() + dy.abs();
                if self.haptic_travel >= 1800 {
                    tick = true;
                    self.haptic_travel = 0;
                }
                if mode == 1 || mode == 2 {
                    let scale =
                        if mode == 1 { 0.01125_f32 } else { 0.02_f32 } * sensitivity as f32 / 100.0;
                    let fx = dx as f32 * scale + self.remainder[0];
                    let fy = (if mode == 1 { dy } else { -dy }) as f32 * scale + self.remainder[1];
                    let x = fx as i32;
                    let y = fy as i32;
                    self.remainder = [fx - x as f32, fy - y as f32];
                    if mode == 1 && (x != 0 || y != 0) {
                        events.push(Output::Move { x, y });
                    }
                    if mode == 2 {
                        if y != 0 {
                            events.push(Output::Wheel {
                                amount: y,
                                horizontal: false,
                            });
                        }
                        if x != 0 {
                            events.push(Output::Wheel {
                                amount: x,
                                horizontal: true,
                            });
                        }
                    }
                }
            } else {
                self.haptic_travel = 0;
            }
        }
        self.xy = xy;
        self.touching = true;
        (events, tick)
    }
}
