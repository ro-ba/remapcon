//! Native controller/UI driver from Remapcon main.cpp (MIT).
//! Copyright (c) 2026 ro-ba; controller modules retain SteamlessController
//! attribution. Preserve ../../LICENSE / THIRD_PARTY_NOTICES.md.
#![forbid(unsafe_code)]
use crate::{
    app::{self, Activation},
    config::Preset,
    controller,
    mapping::{self, Button, PadMotion, Physical, Status, BUTTON_COUNT},
    windows_app::{self, ForegroundMonitor},
    windows_hid::{self, HidDevice},
    windows_output::{self, OutputSession},
    windows_steam::{self, SteamSession},
};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub const WAKE: u32 = 0x8000 + 39;
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Controls {
    pub requested: bool,
    pub auto_mode: bool,
    pub preview: bool,
    pub steam_takeover: bool,
}
struct Settings {
    target: String,
    modes: [u32; 2],
    sensitivities: [u32; 2],
    deadzone: [u32; 2],
    overlap: [u32; 2],
    mapping: mapping::Config,
}
impl Settings {
    fn new(mut preset: Preset, generation: u64) -> Self {
        Self {
            target: std::mem::take(&mut preset.target_executable),
            modes: [preset.left_pad_mode, preset.right_pad_mode],
            sensitivities: [preset.left_pad_sensitivity, preset.right_pad_sensitivity],
            deadzone: [preset.left_stick_deadzone, preset.right_stick_deadzone],
            overlap: [preset.left_stick_overlap, preset.right_stick_overlap],
            mapping: preset.into_engine(generation),
        }
    }
}
enum Command {
    Settings(Box<Settings>),
    Controls(Controls),
}
pub struct Worker {
    commands: mpsc::Sender<Command>,
    pub events: mpsc::Receiver<String>,
    stop: Arc<AtomicBool>,
    // GUI-thread ownership; take permits synchronous shutdown from WndProc's
    // shared context without a mutex or a second join-handle owner.
    thread: std::cell::Cell<Option<thread::JoinHandle<()>>>,
}
impl Worker {
    pub fn start(
        window: usize,
        preset: Preset,
        generation: u64,
        controls: Controls,
    ) -> std::io::Result<Self> {
        let (commands, receiver) = mpsc::channel();
        let (sender, events) = mpsc::channel();
        // A single shared cancellation bit also covers blocking acquisition;
        // configuration and telemetry use ownership transfers, without a mutex.
        let stop = Arc::new(AtomicBool::new(false));
        let cancellation = Arc::clone(&stop);
        let settings = Settings::new(preset, generation);
        let thread = thread::Builder::new()
            .name("padmux-controller".into())
            .spawn(move || {
                let mut driver = Driver {
                    window,
                    settings,
                    controls,
                    commands: receiver,
                    events: sender,
                    stop: cancellation,
                    foreground: ForegroundMonitor::default(),
                    started: Instant::now(),
                    elevated: windows_steam::elevated(),
                };
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| driver.run())).is_err()
                {
                    driver.status("予期しないエラー：アプリを再起動してください");
                    driver.emit(json!({"type":"runtimeStopped"}).to_string());
                }
            })?;
        Ok(Self {
            commands,
            events,
            stop,
            thread: std::cell::Cell::new(Some(thread)),
        })
    }
    pub fn preset(&self, preset: Preset, generation: u64) {
        let _ = self.commands.send(Command::Settings(Box::new(Settings::new(
            preset, generation,
        ))));
    }
    pub fn controls(&self, controls: Controls) {
        let _ = self.commands.send(Command::Controls(controls));
    }
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
    pub fn shutdown(&self) {
        self.request_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Driver {
    window: usize,
    settings: Settings,
    controls: Controls,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<String>,
    stop: Arc<AtomicBool>,
    foreground: ForegroundMonitor,
    started: Instant,
    elevated: bool,
}
impl Driver {
    fn refresh(&mut self) {
        for command in self.commands.try_iter() {
            match command {
                Command::Settings(settings) => self.settings = *settings,
                Command::Controls(controls) => self.controls = controls,
            }
        }
    }
    fn running(&self) -> bool {
        !self.stop.load(Ordering::Relaxed)
    }
    fn activation(&mut self) -> Activation {
        self.refresh();
        let target = self.foreground.check(
            &self.settings.target,
            self.settings.mapping.generation,
            self.started.elapsed(),
        );
        app::activation(
            self.controls.requested,
            self.controls.auto_mode,
            self.controls.preview,
            target,
            windows_app::main_foreground(self.window),
            self.controls.steam_takeover,
            self.elevated,
        )
    }
    fn emit(&self, event: String) {
        if self.running() && self.events.send(event).is_ok() {
            windows_app::wake_ui(self.window, WAKE);
        }
    }
    fn status(&self, text: &str) {
        self.emit(json!({"type":"status","text":text}).to_string());
    }
    fn preview(&self, physical: &Physical, sticks: [[i16; 2]; 2]) {
        let buttons: Vec<_> = (0..BUTTON_COUNT).filter(|&i| physical.buttons[i]).collect();
        self.emit(json!({"type":"input","buttons":buttons}).to_string());
        for (side, [x, y]) in sticks.into_iter().enumerate() {
            self.emit(json!({"type":"stickInput","side":side,"x":x,"y":y}).to_string());
        }
    }
    fn run(&mut self) {
        let repeat = windows_output::repeat_settings();
        let mut idle = None;
        let mut retry_after = Duration::ZERO;
        while self.running() {
            if !self.activation().hold_controller {
                if idle != Some(self.controls.auto_mode) {
                    self.status(if self.controls.auto_mode {
                        "自動待機中：対象アプリを前面にすると有効になります"
                    } else {
                        "停止中：Steam Inputを使用できます"
                    });
                    idle = Some(self.controls.auto_mode);
                }
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            idle = None;
            let mut active = None;
            if let Ok(devices) = windows_hid::enumerate() {
                for device in devices {
                    if !self.running() || !self.activation().hold_controller {
                        break;
                    }
                    if let Ok(mut shared) = HidDevice::open(&device.path) {
                        if windows_steam::wait_for_state(&mut shared, 350) {
                            active = Some(device);
                            break;
                        }
                    }
                }
            }
            let Some(device) = active else {
                if self.activation().hold_controller {
                    self.status("コントローラー待機中：接続を確認してください");
                }
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            if !self.running() || !self.activation().hold_controller {
                continue;
            }
            let mut attempted = false;
            let claimed = SteamSession::claim(&device.path, || {
                let allowed = self.running()
                    && self.started.elapsed() >= retry_after
                    && self.activation().take_from_steam;
                if allowed && !attempted {
                    self.status("Steamからコントローラーを切り替え中です");
                }
                attempted |= allowed;
                allowed
            });
            let mut session = match claimed {
                Ok(session) => session,
                Err(error) => {
                    eprintln!("controller acquisition: {error}");
                    let disable_failed =
                        matches!(error, windows_steam::ClaimError::DisableLizard(_));
                    if attempted && !disable_failed {
                        retry_after = self.started.elapsed() + Duration::from_secs(20);
                    }
                    self.status(if disable_failed {
                        "Lizard Modeを無効化できませんでした"
                    } else if attempted {
                        "排他取得に失敗しました：20秒後に再試行します"
                    } else if self.controls.preview
                        && windows_app::main_foreground(self.window)
                        && !self.controls.steam_takeover
                    {
                        "ボタンを探すには、Steam起動中の排他取得をオンにしてください"
                    } else {
                        "取得できません：Steamなどが使用中です"
                    });
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
            };
            self.status(
                if self.controls.preview && windows_app::main_foreground(self.window) {
                    "入力確認中：ボタンを押してください"
                } else if self.controls.auto_mode {
                    "自動有効：対象アプリへ入力します"
                } else {
                    "有効：ボタン入力待ちです"
                },
            );
            // Declare output after the controller guard: key-up runs before HID
            // restore/return, on normal exit and during unwinding alike.
            let mut output = OutputSession::new(repeat.0, repeat.1);
            let mut physical = Physical::default();
            let mut motion: [PadMotion; 2] = std::array::from_fn(|_| PadMotion::default());
            let mut clicks = [false; 2];
            let mut sticks = [[0; 2]; 2];
            let mut last_report = Instant::now();
            let mut keepalive = last_report;
            let mut last_preview = last_report;
            let mut healthy = true;
            let mut read_error_reported = false;
            while self.running()
                && self.activation().hold_controller
                && healthy
                && (!session.taken_from_steam() || self.controls.steam_takeover)
            {
                if keepalive.elapsed() >= Duration::from_secs(2) {
                    if let Err(error) = session.controller().keepalive() {
                        eprintln!("keepalive: {error}");
                        healthy = false;
                    }
                    keepalive = Instant::now();
                }
                let before = Instant::now();
                let mut report = [0u8; 64];
                let size = match session
                    .controller()
                    .read(&mut report, if output.needs_fast_read() { 8 } else { 32 })
                {
                    Ok(size) => size,
                    Err(error) => {
                        // C++ ReadReport converts read failures to zero, then
                        // retains/reset physical input at the 500ms/4s gates.
                        if !read_error_reported {
                            eprintln!("controller read: {error}");
                            read_error_reported = true;
                        }
                        0
                    }
                };
                if size == 0 && before.elapsed() < Duration::from_millis(2) {
                    thread::sleep(Duration::from_millis(16));
                }
                if let Ok(Some(state)) = controller::parse(&report[..size]) {
                    read_error_reported = false;
                    sticks = state.sticks.unwrap_or([[0; 2]; 2]);
                    let now = self.started.elapsed();
                    physical.update(&state, self.settings.deadzone, self.settings.overlap, now);
                    last_report = Instant::now();
                    if let Some(pads) = state.pads {
                        let enabled = self.activation().output;
                        for (side, pad) in pads.into_iter().enumerate() {
                            let click = physical.buttons[if side == 0 {
                                Button::LeftPadClick
                            } else {
                                Button::RightPadClick
                            } as usize];
                            if enabled && click && !clicks[side] {
                                let _ = session.controller().pulse(side == 0, true);
                            }
                            clicks[side] = click;
                            let contact = if side == 0 {
                                state.flags[3] & 2 != 0
                            } else {
                                state.flags[2] & 0x20 != 0
                            };
                            let (events, tick) = motion[side].update(
                                if enabled {
                                    self.settings.modes[side]
                                } else {
                                    0
                                },
                                self.settings.sensitivities[side],
                                contact,
                                [pad.x, pad.y],
                            );
                            for event in events {
                                windows_output::send(event);
                            }
                            if tick {
                                let _ = session.controller().movement_tick(side == 0);
                            }
                        }
                    } else {
                        motion = std::array::from_fn(|_| PadMotion::default());
                        clicks = [false; 2];
                    }
                }
                let now = self.started.elapsed();
                if last_report.elapsed() > Duration::from_millis(500) {
                    physical = Physical::default();
                    motion = std::array::from_fn(|_| PadMotion::default());
                    clicks = [false; 2];
                    sticks = [[0; 2]; 2];
                } else {
                    physical.update_tap_pulses(now);
                }
                if self.controls.preview
                    && windows_app::main_foreground(self.window)
                    && last_preview.elapsed() >= Duration::from_millis(40)
                {
                    self.preview(&physical, sticks);
                    last_preview = Instant::now();
                }
                let enabled = self.activation().output;
                if let Err(error) = output.update(
                    &physical.buttons,
                    enabled,
                    &self.settings.mapping,
                    now,
                    &mut |status| {
                        self.status(match status {
                            Status::Sent => "ボタン入力を検出：設定した入力をWindowsに送信しました",
                            Status::Failed => "入力送信に失敗：対象アプリの権限を確認してください",
                        })
                    },
                ) {
                    eprintln!("mapping validation: {error}");
                    break;
                }
                if last_report.elapsed() > Duration::from_secs(4) {
                    break;
                }
            }
            drop(output);
            if let Err(error) = session.close_before_return(|| {
                self.status("Steamへコントローラーを返しています");
            }) {
                eprintln!("controller restore/return: {error}");
                if matches!(error, windows_steam::CloseError::Return(_)) {
                    self.status("Steamへの切り替えを確認できませんでした");
                }
            }
            self.preview(&Physical::default(), [[0; 2]; 2]);
            if self.running() && self.activation().hold_controller {
                self.status("通信が切れました：再接続します");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_worker_stops_without_controller_access_or_output() {
        let config = crate::config::Configuration::from_json(include_bytes!(
            "../tests/config-reference.json"
        ))
        .unwrap();
        let worker = Worker::start(
            0,
            config.presets.into_iter().next().unwrap(),
            0,
            Controls {
                requested: false,
                auto_mode: false,
                preview: false,
                steam_takeover: false,
            },
        )
        .unwrap();
        worker.controls(Controls {
            requested: false,
            auto_mode: false,
            preview: false,
            steam_takeover: false,
        });
        worker.shutdown();
        assert!(worker.thread.take().is_none());
    }
}
