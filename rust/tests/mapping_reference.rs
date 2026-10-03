use padmux_diag::{controller, mapping::*};
use std::time::Duration;

fn emit(trace: &mut String, now: u64, event: Output, fail: &mut (u32, bool, u32)) -> bool {
    use std::fmt::Write;
    match event {
        Output::Key { key, down } => {
            let ok = !(fail.0 == key && fail.1 == down && fail.2 > 0);
            if !ok {
                fail.2 -= 1;
            }
            writeln!(trace, "K {now} {key} {} {}", u8::from(down), u8::from(ok)).unwrap();
            ok
        }
        Output::Status(status) => {
            writeln!(trace, "S {now} {}", u8::from(status == Status::Sent)).unwrap();
            true
        }
        Output::Move { x, y } => {
            writeln!(trace, "M {now} {x} {y}").unwrap();
            true
        }
        Output::Wheel { amount, horizontal } => {
            writeln!(trace, "W {now} {amount} {}", u8::from(horizontal)).unwrap();
            true
        }
        Output::Wait(_) => true, // Reference really sleeps; trace records only output.
    }
}

#[test]
fn same_input_and_time_produce_cpp_reference_trace() {
    use std::fmt::Write;
    let mut config = Config::default();
    let mut engine = Engine::new(Duration::from_millis(250), Duration::from_millis(40));
    let mut physical = Physical::default();
    let mut motion = PadMotion::default();
    let mut deadzone = [12288; 2];
    let mut overlap = [16000; 2];
    let mut fail = (0, false, 0);
    let mut trace = String::new();
    for (line_number, line) in include_str!("mapping-input.txt").lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let command = fields.next().unwrap();
        if command == "reset" {
            config = Config::default();
            engine = Engine::new(Duration::from_millis(250), Duration::from_millis(40));
            physical = Physical::default();
            motion = PadMotion::default();
            deadzone = [12288; 2];
            overlap = [16000; 2];
            fail.2 = 0;
            continue;
        }
        if command == "physical" {
            let now = fields.next().unwrap().parse::<u64>().unwrap();
            let hex = fields.next().unwrap();
            let report: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let state = controller::parse(&report).unwrap().unwrap();
            physical.update(&state, deadzone, overlap, Duration::from_millis(now));
            let bits = physical
                .buttons
                .iter()
                .enumerate()
                .fold(0u32, |bits, (i, &pressed)| bits | (u32::from(pressed) << i));
            writeln!(trace, "P {now} {bits}").unwrap();
            continue;
        }
        // Fixtures are trusted checked-in test inputs; assertions identify a bad line.
        let numbers: Vec<i64> = fields.map(|v| v.parse().unwrap()).collect();
        let n = |i: usize| numbers[i] as u32;
        match command {
            "map" | "turbo" | "seq" => {
                let layer = if n(0) == 0 {
                    &mut config.base
                } else {
                    &mut config.layers[n(0) as usize - 1]
                };
                let index = n(1) as usize;
                match command {
                    "map" => layer.mapping[index] = n(2),
                    "turbo" => {
                        layer.turbo[index] = Turbo {
                            enabled: n(2) != 0,
                            interval_ms: n(3),
                            delay_ms: n(4),
                        }
                    }
                    "seq" => {
                        layer.sequence[index] = Sequence {
                            enabled: n(2) != 0,
                            repeat: n(3) != 0,
                            interval_ms: n(4),
                            keys: numbers[5..].iter().map(|&key| key as u32).collect(),
                        }
                    }
                    _ => unreachable!(),
                }
            }
            "layer" => config.layers.push(Layer {
                trigger: n(0) as usize,
                ..Layer::default()
            }),
            "gen" => config.generation = n(0) as u64,
            "fail" => fail = (n(0), n(1) != 0, n(2)),
            "tick" => {
                let now = n(0) as u64;
                let buttons = std::array::from_fn(|i| n(1) & (1 << i) != 0);
                engine
                    .update(
                        &buttons,
                        n(2) != 0,
                        &config,
                        Duration::from_millis(now),
                        &mut |event| emit(&mut trace, now, event, &mut fail),
                    )
                    .unwrap();
            }
            "release" => {
                let now = n(0) as u64;
                engine.release_all(&mut |event| emit(&mut trace, now, event, &mut fail));
            }
            "stick" => {
                deadzone[n(0) as usize] = n(1);
                overlap[n(0) as usize] = n(2);
            }
            "pulse" => {
                let now = n(0) as u64;
                physical.update_tap_pulses(Duration::from_millis(now));
                let bits = physical
                    .buttons
                    .iter()
                    .enumerate()
                    .fold(0u32, |bits, (i, &pressed)| bits | (u32::from(pressed) << i));
                writeln!(trace, "P {now} {bits}").unwrap();
            }
            "motion" => {
                let now = n(0) as u64;
                let (events, tick) = motion.update(
                    n(1),
                    n(2),
                    n(3) != 0,
                    [numbers[4] as i16, numbers[5] as i16],
                );
                for event in events {
                    emit(&mut trace, now, event, &mut fail);
                }
                writeln!(trace, "H {now} {}", u8::from(tick)).unwrap();
            }
            _ => panic!("unknown fixture command on line {}", line_number + 1),
        }
    }
    assert_eq!(
        trace,
        include_str!("mapping-expected.txt").replace("\r\n", "\n")
    );
}

#[test]
fn timing_validation_and_cleanup_waits_preserve_contract() {
    let mut engine = Engine::new(Duration::from_millis(250), Duration::from_millis(40));
    let mut config = Config::default();
    config.base.mapping[0] = 30;
    let mut buttons = [false; BUTTON_COUNT];
    buttons[0] = true;
    let mut events = Vec::new();
    engine
        .update(&buttons, true, &config, Duration::ZERO, &mut |e| {
            events.push(e);
            true
        })
        .unwrap();
    assert!(!engine.needs_fast_read());
    events.clear();
    engine.release_all(&mut |e| {
        events.push(e);
        false
    });
    assert_eq!(
        events,
        [
            Output::Key {
                key: 30,
                down: false
            },
            Output::Wait(Duration::from_millis(20)),
            Output::Key {
                key: 30,
                down: false
            },
            Output::Wait(Duration::from_millis(20)),
            Output::Key {
                key: 30,
                down: false
            },
            Output::Wait(Duration::from_millis(20))
        ]
    );
    config.base.turbo[0].interval_ms = 0;
    assert!(engine
        .update(&buttons, true, &config, Duration::ZERO, &mut |_| true)
        .is_err());
    config.base.turbo[0] = Turbo {
        enabled: true,
        ..Turbo::default()
    };
    engine
        .update(&buttons, true, &config, Duration::ZERO, &mut |_| true)
        .unwrap();
    assert!(engine.needs_fast_read());
    buttons[0] = false;
    engine
        .update(&buttons, true, &config, Duration::ZERO, &mut |_| true)
        .unwrap();
    assert!(!engine.needs_fast_read());
}
