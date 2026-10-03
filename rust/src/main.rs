#[cfg(windows)]
use padmux_diag::{
    controller,
    mapping::{Button, Config, Output, PadMotion, Physical, MOUSE_LEFT},
    windows_controller::Controller,
    windows_hid::{enumerate, install_stop_handler, stop_requested, HidDevice},
    windows_output::{send_checked, OutputSession},
};
#[cfg(windows)]
use std::time::{Duration, Instant};

#[cfg(windows)]
#[derive(Debug)]
struct Options {
    list: bool,
    seconds: u64,
    device: Option<usize>,
    raw: bool,
    control: bool,
    haptics: bool,
    reconnect: bool,
    output_test: bool,
    mouse_pad: usize,
    mouse_sensitivity: u32,
    check_config: Option<std::path::PathBuf>,
    save_config: Option<std::path::PathBuf>,
    foreground_config: Option<std::path::PathBuf>,
    steam_handoff: Option<std::path::PathBuf>,
}

#[cfg(windows)]
fn options(mut args: impl Iterator<Item = String>) -> Result<Option<Options>, String> {
    let mut options = Options {
        list: false,
        seconds: 10,
        device: None,
        raw: false,
        control: false,
        haptics: false,
        reconnect: false,
        output_test: false,
        mouse_pad: 0,
        mouse_sensitivity: 100,
        check_config: None,
        save_config: None,
        foreground_config: None,
        steam_handoff: None,
    };
    let mut pad_options = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("padmux-diag [--list] [--device INDEX] [--seconds 1..300] [--raw] [--control [--haptics] [--output-test [--mouse-pad left|right] [--mouse-sensitivity 25..400]]] [--reconnect]\nDefault: read-only. --control requires write-exclusive access and temporarily disables Lizard Mode; normal exit/error/Ctrl+C restores defaults. --haptics tests left tick/click then right tick/click. --output-test sends actual Windows input: A/B/X/Y letters, D-pad arrows, LB/RB/L4 mouse buttons, default left pad mouse/right pad wheel (100%); --mouse-pad selects the mouse side, the other side scrolls at 100%. Focus a blank test editor. These HID modes do not perform Steam takeover/device cycling.\nConfiguration only (no HID/output): --check-config FILE [--save-config DESTINATION]. Existing schema; saving uses atomic replacement.\nForeground observation only (no HID/output/writes): --foreground-config FILE [--seconds 1..300].\nSteam handoff (administrator, device disable/re-enable, selected target only): --steam-handoff FILE [--seconds 1..300]. Requires autoMode/steamTakeover; reads reports without keyboard/mouse output.");
                return Ok(None);
            }
            "--list" => options.list = true,
            "--raw" => options.raw = true,
            "--control" => options.control = true,
            "--haptics" => options.haptics = true,
            "--reconnect" => options.reconnect = true,
            "--output-test" => options.output_test = true,
            "--check-config" | "--save-config" | "--foreground-config" | "--steam-handoff" => {
                let path = args
                    .next()
                    .filter(|p| !p.is_empty())
                    .ok_or("missing config path")?;
                if arg == "--check-config" {
                    options.check_config = Some(path.into());
                } else if arg == "--foreground-config" {
                    options.foreground_config = Some(path.into());
                } else if arg == "--steam-handoff" {
                    options.steam_handoff = Some(path.into());
                } else {
                    options.save_config = Some(path.into());
                }
            }
            "--mouse-pad" => {
                options.mouse_pad = match args.next().as_deref() {
                    Some("left") => 0,
                    Some("right") => 1,
                    _ => return Err("mouse pad must be left or right".into()),
                };
                pad_options = true;
            }
            "--mouse-sensitivity" => {
                options.mouse_sensitivity = args
                    .next()
                    .ok_or("missing mouse sensitivity")?
                    .parse()
                    .map_err(|_| "invalid mouse sensitivity")?;
                if !(25..=400).contains(&options.mouse_sensitivity) {
                    return Err("mouse sensitivity must be 25..400".into());
                }
                pad_options = true;
            }
            "--device" => {
                options.device = Some(
                    args.next()
                        .ok_or("missing device index")?
                        .parse()
                        .map_err(|_| "invalid device index")?,
                )
            }
            "--seconds" => {
                options.seconds = args
                    .next()
                    .ok_or("missing seconds")?
                    .parse()
                    .map_err(|_| "invalid seconds")?;
                if !(1..=300).contains(&options.seconds) {
                    return Err("seconds must be 1..300".into());
                }
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    if options.haptics && !options.control {
        return Err("--haptics requires --control".into());
    }
    if options.save_config.is_some() && options.check_config.is_none() {
        return Err("--save-config requires --check-config".into());
    }
    if options.steam_handoff.is_some()
        && (options.check_config.is_some()
            || options.save_config.is_some()
            || options.foreground_config.is_some()
            || options.list
            || options.raw
            || options.device.is_some()
            || options.control
            || options.haptics
            || options.reconnect
            || options.output_test
            || pad_options)
    {
        return Err("Steam handoff cannot be combined with other diagnostics".into());
    }
    if options.foreground_config.is_some()
        && (options.check_config.is_some()
            || options.save_config.is_some()
            || options.list
            || options.raw
            || options.device.is_some()
            || options.control
            || options.haptics
            || options.reconnect
            || options.output_test
            || pad_options)
    {
        return Err("foreground observation cannot be combined with HID/config writes".into());
    }
    if options.check_config.is_some()
        && (options.list
            || options.raw
            || options.device.is_some()
            || options.control
            || options.haptics
            || options.reconnect
            || options.output_test
            || pad_options
            || options.seconds != 10)
    {
        return Err("config checking cannot be combined with HID diagnostics".into());
    }
    if options.output_test && !options.control {
        return Err("--output-test requires --control".into());
    }
    if pad_options && !options.output_test {
        return Err("mouse pad/sensitivity options require --output-test".into());
    }
    if options.haptics && options.seconds < 2 {
        return Err("--haptics requires at least 2 seconds".into());
    }
    if options.list
        && (options.control || options.haptics || options.reconnect || options.output_test)
    {
        return Err("--list cannot be combined with control/reconnect".into());
    }
    Ok(Some(options))
}

#[cfg(windows)]
struct TestMapping {
    config: Config,
    physical: Physical,
    motion: [PadMotion; 2],
    output: OutputSession,
    last_state: Duration,
    clicks: [bool; 2],
    mouse_successes: [u64; 2],
    mouse_failures: [u64; 2],
    mouse_pad: usize,
    mouse_sensitivity: u32,
}
#[cfg(windows)]
impl TestMapping {
    fn new(mouse_pad: usize, mouse_sensitivity: u32) -> Self {
        use Button::*;
        let mut config = Config::default();
        config.base.mapping.fill(0);
        for (button, key) in [
            (A, 0x1e),
            (B, 0x30),
            (X, 0x2d),
            (Y, 0x15),
            (DPadUp, 0x10048),
            (DPadDown, 0x10050),
            (DPadLeft, 0x1004b),
            (DPadRight, 0x1004d),
            (LB, MOUSE_LEFT),
            (RB, MOUSE_LEFT + 1),
            (L4, MOUSE_LEFT + 2),
            (LeftPadClick, MOUSE_LEFT),
            (RightPadClick, MOUSE_LEFT + 1),
        ] {
            config.base.mapping[button as usize] = key;
        }
        Self {
            config,
            physical: Physical::default(),
            motion: std::array::from_fn(|_| PadMotion::default()),
            output: OutputSession::default(),
            last_state: Duration::ZERO,
            clicks: [false; 2],
            mouse_successes: [0; 2],
            mouse_failures: [0; 2],
            mouse_pad,
            mouse_sensitivity,
        }
    }
    fn state(
        &mut self,
        state: &controller::State,
        now: Duration,
        controller: &mut Controller,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.last_state = now;
        self.physical.update(state, [12288; 2], [16000; 2], now);
        if let Some(pads) = &state.pads {
            for (side, pad) in pads.iter().enumerate() {
                let click = self.physical.buttons[if side == 0 {
                    Button::LeftPadClick
                } else {
                    Button::RightPadClick
                } as usize];
                if click && !self.clicks[side] {
                    controller.pulse(side == 0, true)?;
                }
                self.clicks[side] = click;
                let contact = if side == 0 {
                    state.flags[3] & 2 != 0
                } else {
                    state.flags[2] & 0x20 != 0
                };
                let (events, tick) = self.motion[side].update(
                    if side == self.mouse_pad { 1 } else { 2 },
                    if side == self.mouse_pad {
                        self.mouse_sensitivity
                    } else {
                        100
                    },
                    contact,
                    [pad.x, pad.y],
                );
                for event in events {
                    let kind = usize::from(matches!(event, Output::Wheel { .. }));
                    match send_checked(event) {
                        Ok(()) => self.mouse_successes[kind] += 1,
                        Err(code) => {
                            if self.mouse_failures[kind] == 0 {
                                eprintln!("SendInput {event:?} failed: Windows error {code}");
                            }
                            self.mouse_failures[kind] += 1;
                        }
                    }
                }
                if tick {
                    controller.movement_tick(side == 0)?;
                }
            }
        } else {
            self.motion = std::array::from_fn(|_| PadMotion::default());
            self.clicks = [false; 2];
        }
        Ok(())
    }
    fn tick(&mut self, now: Duration) -> Result<(), &'static str> {
        if now.saturating_sub(self.last_state) > Duration::from_millis(500) {
            self.physical = Physical::default();
            self.motion = std::array::from_fn(|_| PadMotion::default());
            self.clicks = [false; 2];
        } else {
            self.physical.update_tap_pulses(now);
        }
        self.output.update(
            &self.physical.buttons,
            true,
            &self.config,
            now,
            &mut |status| println!("Windows output status: {status:?}"),
        )
    }
}

#[cfg(windows)]
impl Drop for TestMapping {
    fn drop(&mut self) {
        println!(
            "mouse outputs: move_success={}, move_failure={}, wheel_success={}, wheel_failure={}",
            self.mouse_successes[0],
            self.mouse_failures[0],
            self.mouse_successes[1],
            self.mouse_failures[1]
        );
    }
}

#[cfg(windows)]
fn diagnose(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = &options.check_config {
        let config = padmux_diag::windows_config::load(path)?;
        println!(
            "config valid: presets={}, folders={}, selected_preset={}",
            config.presets.len(),
            config.folders.len(),
            config.selected_preset
        );
        if let Some(destination) = &options.save_config {
            padmux_diag::windows_config::save(destination, &config)?;
            println!("config saved with atomic replacement");
        }
        return Ok(());
    }
    install_stop_handler()?;
    if let Some(path) = &options.steam_handoff {
        return handoff_diagnostic(path, options.seconds);
    }
    if let Some(path) = &options.foreground_config {
        let config = padmux_diag::windows_config::load(path)?;
        let target = &config.presets[config.selected_preset].target_executable;
        let mut monitor = padmux_diag::windows_app::ForegroundMonitor::default();
        let start = Instant::now();
        let mut previous = None;
        println!("foreground observation only: no HID, input output, or configuration writes");
        while start.elapsed() < Duration::from_secs(options.seconds) && !stop_requested() {
            let matched = monitor.check(target, 0, start.elapsed());
            if previous != Some(matched) {
                println!(
                    "selected_target_foreground={matched}, automatic_active={}",
                    config.auto_mode && matched
                );
                previous = Some(matched);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        return Ok(());
    }
    let deadline = Instant::now() + Duration::from_secs(options.seconds);
    loop {
        let result = session(&options, deadline);
        if result.is_ok() || !options.reconnect || Instant::now() >= deadline || stop_requested() {
            return result;
        }
        eprintln!(
            "{error}; retrying enumeration in 1 second",
            error = result.unwrap_err()
        );
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(windows)]
fn handoff_diagnostic(
    path: &std::path::Path,
    seconds: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use padmux_diag::{
        windows_app::ForegroundMonitor,
        windows_steam::{self, SteamSession},
    };
    let config = padmux_diag::windows_config::load(path)?;
    let target = &config.presets[config.selected_preset].target_executable;
    if !config.auto_mode || !config.steam_takeover || target.is_empty() {
        return Err("handoff diagnostic requires autoMode, steamTakeover and a selected target; configuration is not modified".into());
    }
    if !windows_steam::elevated() {
        return Err("Steam handoff requires administrator execution".into());
    }
    if !windows_steam::recover_interrupted() {
        return Err("pending device recovery failed; handoff aborted".into());
    }
    let start = Instant::now();
    let mut monitor = ForegroundMonitor::default();
    let mut retry_after = Duration::ZERO;
    let mut previous_match = None;
    while start.elapsed() < Duration::from_secs(seconds) && !stop_requested() {
        let matched = monitor.check(target, 0, start.elapsed());
        if previous_match != Some(matched) {
            println!("selected_target_foreground={matched}");
            previous_match = Some(matched);
        }
        if !matched {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let mut active = None;
        for device in enumerate()? {
            if stop_requested() || !monitor.check(target, 0, start.elapsed()) {
                break;
            }
            if let Ok(mut shared) = HidDevice::open(&device.path) {
                if windows_steam::wait_for_state(&mut shared, 350) {
                    active = Some(device);
                    break;
                }
            }
        }
        let Some(device) = active else {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };
        if stop_requested() || !monitor.check(target, 0, start.elapsed()) {
            continue;
        }
        let allow_cycle = start.elapsed() >= retry_after;
        let mut attempted_takeover = false;
        let result = SteamSession::claim(&device.path, || {
            let allowed = allow_cycle
                && !stop_requested()
                && start.elapsed() < Duration::from_secs(seconds)
                && monitor.check(target, 0, start.elapsed());
            attempted_takeover |= allowed;
            allowed
        });
        let mut session = match result {
            Ok(session) => session,
            Err(error) => {
                eprintln!("exclusive acquisition failed: {error}");
                if attempted_takeover {
                    retry_after = start.elapsed() + Duration::from_secs(20);
                }
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        };
        println!(
            "controller active: taken_from_steam={}",
            session.taken_from_steam()
        );
        let mut last_report = Instant::now();
        let mut last_keepalive = Instant::now();
        let mut states = 0u64;
        let mut report = [0u8; 64];
        while start.elapsed() < Duration::from_secs(seconds)
            && !stop_requested()
            && monitor.check(target, 0, start.elapsed())
        {
            if last_keepalive.elapsed() >= Duration::from_secs(2) {
                if let Err(error) = session.controller().keepalive() {
                    eprintln!("keepalive: {error}");
                    break;
                }
                last_keepalive = Instant::now();
            }
            match session.controller().read(&mut report, 32) {
                Ok(size) => {
                    if let Ok(Some(_)) = controller::parse(&report[..size]) {
                        states += 1;
                        last_report = Instant::now();
                    }
                }
                Err(error) => {
                    eprintln!("controller disconnected: {error}");
                    break;
                }
            }
            if last_report.elapsed() > Duration::from_secs(4) {
                break;
            }
        }
        let taken = session.taken_from_steam();
        let closed = session.close();
        if let Err(error) = &closed {
            eprintln!("controller recovery: {error}");
        }
        println!("controller released: states={states}, taken_from_steam={taken}, restore_and_return_ok={}", closed.is_ok());
    }
    println!("Steam handoff diagnostic complete; no keyboard/mouse output or configuration writes");
    Ok(())
}

#[cfg(windows)]
fn session(options: &Options, deadline: Instant) -> Result<(), Box<dyn std::error::Error>> {
    let devices = enumerate()?;
    for (index, info) in devices.iter().enumerate() {
        println!("[{index}] VID=28de PID={:04x} transport={} usage={:04x}:{:04x} input={} output={} feature={}\n  {}",
            info.pid, controller::transport(&info.path), info.caps.UsagePage, info.caps.Usage,
            info.caps.InputReportByteLength, info.caps.OutputReportByteLength, info.caps.FeatureReportByteLength, info.path);
    }
    if devices.is_empty() {
        return Err("No matching Steam Controller HID interfaces found".into());
    }
    if let Some(index) = options.device {
        if index >= devices.len() {
            return Err("device index out of range".into());
        }
    }
    if options.list {
        return Ok(());
    }
    for (index, info) in devices.iter().enumerate() {
        if options.device.is_some_and(|selected| selected != index) {
            continue;
        }
        let mut device = match HidDevice::open(&info.path) {
            Ok(device) => device,
            Err(error) => {
                eprintln!("[{index}] open failed: {error}");
                continue;
            }
        };
        // Same 64-byte buffer and 350 ms active-slot probe as ControllerLoop.
        let mut report = [0u8; 64];
        let probe_deadline = Instant::now() + Duration::from_millis(350);
        let mut first = None;
        while Instant::now() < probe_deadline {
            let remaining = probe_deadline.saturating_duration_since(Instant::now());
            match device.read(&mut report, remaining.as_millis().max(1) as u32) {
                Ok(0) => break,
                Ok(size) => match controller::parse(&report[..size]) {
                    Ok(Some(state)) => {
                        first = Some((state, size));
                        break;
                    }
                    Ok(None) => println!("[{index}] non-state report: {:02x?}", &report[..size]),
                    Err(error) => eprintln!("[{index}] malformed report: {error}"),
                },
                Err(error) => {
                    eprintln!("[{index}] read failed: {error}");
                    break;
                }
            }
        }
        let Some((first, first_size)) = first else {
            eprintln!("[{index}] no live state report within 350ms");
            continue;
        };
        println!("[{index}] active slot: {first:?}");
        if options.raw {
            println!("raw({first_size}): {:02x?}", &report[..first_size]);
        }
        if stop_requested() {
            return Ok(());
        }
        if options.control {
            // Close the shared probe before requesting C++'s FILE_SHARE_READ mode.
            drop(device);
            device = HidDevice::open_exclusive(&info.path)?;
        }
        let mut controller = Controller::new(device);
        if options.control {
            controller.disable_lizard_mode()?;
            println!("Lizard Mode disabled; defaults will be restored on exit");
        }
        let started = Instant::now();
        let mut mapping = options
            .output_test
            .then(|| TestMapping::new(options.mouse_pad, options.mouse_sensitivity));
        if let Some(mapping) = &mut mapping {
            mapping.state(&first, Duration::ZERO, &mut controller)?;
            println!("Windows output test active; focus a blank test editor");
        }
        let mut last_keepalive = started;
        let mut last_state = started;
        let mut haptic_step = 0;
        let mut last_print = Instant::now();
        let mut states = 1u64;
        let mut others = 0u64;
        let mut seen_flags = first.flags;
        let mut last_flags = first.flags;
        while Instant::now() < deadline && !stop_requested() {
            if options.control && last_keepalive.elapsed() >= Duration::from_secs(2) {
                controller.keepalive()?;
                println!("keepalive sent");
                last_keepalive = Instant::now();
            }
            if options.haptics
                && haptic_step < 4
                && started.elapsed() >= Duration::from_millis(500 * haptic_step)
            {
                if haptic_step == 0 {
                    if !controller.movement_tick(true)? {
                        return Err("first movement haptic was unexpectedly rate limited".into());
                    }
                } else {
                    controller.pulse(haptic_step < 2, haptic_step % 2 == 1)?;
                }
                println!(
                    "haptic {} {}",
                    if haptic_step < 2 { "left" } else { "right" },
                    if haptic_step % 2 == 1 {
                        "click"
                    } else {
                        "tick"
                    }
                );
                haptic_step += 1;
            }
            if last_state.elapsed() > Duration::from_secs(4) {
                return Err("no state report for 4 seconds; connection lost".into());
            }
            let fast = mapping.as_ref().is_some_and(|m| m.output.needs_fast_read());
            let size = controller.read(&mut report, if fast { 8 } else { 32 })?;
            if size == 0 {
                if let Some(mapping) = &mut mapping {
                    mapping.tick(started.elapsed())?;
                }
                continue;
            }
            match controller::parse(&report[..size]) {
                Ok(Some(state)) => {
                    last_state = Instant::now();
                    states += 1;
                    if let Some(mapping) = &mut mapping {
                        mapping.state(&state, started.elapsed(), &mut controller)?;
                    }
                    for (seen, flag) in seen_flags.iter_mut().zip(state.flags) {
                        *seen |= flag;
                    }
                    if state.flags != last_flags
                        || last_print.elapsed() >= Duration::from_millis(100)
                    {
                        println!("{state:?}");
                        if options.raw {
                            println!("raw({size}): {:02x?}", &report[..size]);
                        }
                        last_print = Instant::now();
                        last_flags = state.flags;
                    }
                }
                Ok(None) => {
                    others += 1;
                    if options.raw {
                        println!("other({size}): {:02x?}", &report[..size]);
                    }
                }
                Err(error) => return Err(error.into()),
            }
            if let Some(mapping) = &mut mapping {
                mapping.tick(started.elapsed())?;
            }
        }
        drop(mapping); // Release mapped keys before restoring controller defaults.
        controller.restore_lizard_mode()?;
        if options.control {
            println!("Lizard Mode default mappings/settings restored");
        }
        println!("[{index}] read complete: states={states}, other_reports={others}, seen_flags={seen_flags:02x?}");
        if states < 2 {
            return Err("active slot stopped producing state reports".into());
        }
        return Ok(());
    }
    Err("No active slot could be opened/read. Check connection and Steam/shared access.".into())
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(code) = padmux_diag::windows_steam::helper_command(&args) {
        return std::process::ExitCode::from(code);
    }
    match options(args.into_iter()) {
        Ok(None) => std::process::ExitCode::SUCCESS,
        Ok(Some(options)) => match diagnose(options) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                std::process::ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}; use --help");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_arguments_reject_invalid_and_missing_values() {
        for args in [
            vec!["--seconds"],
            vec!["--seconds", "0"],
            vec!["--seconds", "301"],
            vec!["--seconds", "-1"],
            vec!["--device"],
            vec!["--device", "bad"],
            vec!["--unknown"],
            vec!["--haptics"],
            vec!["--output-test"],
            vec!["--check-config"],
            vec!["--check-config", ""],
            vec!["--save-config", "test.json"],
            vec!["--check-config", "test.json", "--control"],
            vec!["--mouse-pad"],
            vec!["--mouse-pad", "right"],
            vec!["--control", "--output-test", "--mouse-pad", "bad"],
            vec!["--control", "--output-test", "--mouse-sensitivity"],
            vec!["--control", "--output-test", "--mouse-sensitivity", "24"],
            vec!["--control", "--output-test", "--mouse-sensitivity", "401"],
            vec!["--control", "--haptics", "--seconds", "1"],
            vec!["--list", "--control"],
            vec!["--foreground-config"],
            vec!["--foreground-config", "config.json", "--control"],
            vec!["--steam-handoff"],
            vec!["--steam-handoff", "config.json", "--control"],
            vec![
                "--steam-handoff",
                "config.json",
                "--foreground-config",
                "other.json",
            ],
            vec![
                "--foreground-config",
                "config.json",
                "--check-config",
                "other.json",
            ],
        ] {
            assert!(options(args.into_iter().map(String::from)).is_err());
        }
    }

    #[test]
    fn diagnostic_defaults_and_explicit_options() {
        let defaults = options(std::iter::empty()).unwrap().unwrap();
        assert_eq!(defaults.seconds, 10);
        assert_eq!(defaults.device, None);
        assert_eq!(defaults.mouse_pad, 0);
        assert_eq!(defaults.mouse_sensitivity, 100);
        let foreground = options(
            ["--foreground-config", "config.json", "--seconds", "1"]
                .into_iter()
                .map(String::from),
        )
        .unwrap()
        .unwrap();
        assert!(foreground.foreground_config.is_some());
        let right = options(
            [
                "--control",
                "--output-test",
                "--mouse-pad",
                "right",
                "--mouse-sensitivity",
                "75",
            ]
            .into_iter()
            .map(String::from),
        )
        .unwrap()
        .unwrap();
        assert_eq!(right.mouse_pad, 1);
        assert_eq!(right.mouse_sensitivity, 75);
        let check = options(
            [
                "--check-config",
                "input.json",
                "--save-config",
                "output.json",
            ]
            .into_iter()
            .map(String::from),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            check.check_config.unwrap(),
            std::path::PathBuf::from("input.json")
        );
        assert_eq!(
            check.save_config.unwrap(),
            std::path::PathBuf::from("output.json")
        );
        let parsed = options(
            ["--device", "2", "--seconds", "30", "--raw", "--list"]
                .into_iter()
                .map(String::from),
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.device, Some(2));
        assert_eq!(parsed.seconds, 30);
        assert!(parsed.raw && parsed.list);
        assert!(options(["--help"].into_iter().map(String::from))
            .unwrap()
            .is_none());
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("HID diagnostics require Windows x64; protocol tests are platform-independent.");
    std::process::ExitCode::FAILURE
}
