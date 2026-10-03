//! Windows output translated from Remapcon src/app/main.cpp; ../../LICENSE (MIT).
use crate::mapping::{Config, Engine, Output, Status, BUTTON_COUNT, MOUSE_LEFT};
use std::{mem::size_of, time::Duration};
use windows::Win32::Foundation::{GetLastError, SetLastError, WIN32_ERROR};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn input(event: Output) -> Option<INPUT> {
    let mouse = |flags, x, y, data| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: x,
                dy: y,
                mouseData: data,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    Some(match event {
        Output::Key { key, down } if (MOUSE_LEFT..=MOUSE_LEFT + 2).contains(&key) => {
            let flags = [
                [MOUSEEVENTF_LEFTUP, MOUSEEVENTF_LEFTDOWN],
                [MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_RIGHTDOWN],
                [MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MIDDLEDOWN],
            ];
            mouse(
                flags[(key - MOUSE_LEFT) as usize][usize::from(down)],
                0,
                0,
                0,
            )
        }
        Output::Key { key, down } => INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wScan: (key & 0xff) as u16,
                    dwFlags: KEYEVENTF_SCANCODE
                        | if key & 0x10000 != 0 {
                            KEYEVENTF_EXTENDEDKEY
                        } else {
                            KEYBD_EVENT_FLAGS(0)
                        }
                        | if down {
                            KEYBD_EVENT_FLAGS(0)
                        } else {
                            KEYEVENTF_KEYUP
                        },
                    ..Default::default()
                },
            },
        },
        Output::Move { x, y } => mouse(MOUSEEVENTF_MOVE, x, y, 0),
        Output::Wheel { amount, horizontal } => mouse(
            if horizontal {
                MOUSEEVENTF_HWHEEL
            } else {
                MOUSEEVENTF_WHEEL
            },
            0,
            0,
            amount as u32,
        ),
        Output::Wait(_) | Output::Status(_) => return None,
    })
}

pub fn send(event: Output) -> bool {
    send_checked(event).is_ok()
}

/// Preserve the return count and immediate Windows error for diagnostics.
/// Error 0 can occur on blocked input; it does not identify the cause.
pub fn send_checked(event: Output) -> Result<(), u32> {
    if let Output::Wait(duration) = event {
        std::thread::sleep(duration);
        return Ok(());
    }
    let Some(input) = input(event) else {
        return Ok(());
    };
    // SAFETY: input constructs and initializes the union arm matching r#type.
    // The one-element slice remains live throughout the synchronous call, and
    // cbSize is the actual Windows ABI size. Match C++'s success count check.
    // Clear/capture this thread's error immediately so earlier HID calls cannot
    // contaminate a failure diagnostic. This does not change input behavior.
    unsafe {
        SetLastError(WIN32_ERROR(0));
        if SendInput(&[input], size_of::<INPUT>() as i32) == 1 {
            Ok(())
        } else {
            Err(GetLastError().0)
        }
    }
}

pub fn repeat_settings() -> (Duration, Duration) {
    let (mut delay, mut speed) = (1u32, 20u32);
    // SAFETY: these queries write a DWORD into each live initialized local.
    // No setting is changed. Failed queries retain C++'s default values.
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETKEYBOARDDELAY,
            0,
            Some((&mut delay as *mut u32).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let _ = SystemParametersInfoW(
            SPI_GETKEYBOARDSPEED,
            0,
            Some((&mut speed as *mut u32).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    (
        Duration::from_millis(u64::from(250 * (delay.min(3) + 1))),
        Duration::from_millis(u64::from(62000 / (155 + 55 * speed.min(31)))),
    )
}

/// Owns mapping-held keys so read/setup errors also attempt key-up cleanup.
pub struct OutputSession {
    engine: Engine,
}
impl Default for OutputSession {
    fn default() -> Self {
        let (delay, interval) = repeat_settings();
        Self::new(delay, interval)
    }
}
impl OutputSession {
    pub fn new(delay: Duration, interval: Duration) -> Self {
        Self {
            engine: Engine::new(delay, interval),
        }
    }
    pub fn update(
        &mut self,
        physical: &[bool; BUTTON_COUNT],
        enabled: bool,
        config: &Config,
        now: Duration,
        status: &mut impl FnMut(Status),
    ) -> Result<(), &'static str> {
        self.engine
            .update(physical, enabled, config, now, &mut |event| {
                if let Output::Status(value) = event {
                    status(value);
                    true
                } else {
                    send(event)
                }
            })
    }
    pub fn needs_fast_read(&self) -> bool {
        self.engine.needs_fast_read()
    }
}
impl Drop for OutputSession {
    fn drop(&mut self) {
        self.engine.reset(&mut send);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_layout_matches_cpp_scancodes_buttons_motion_and_signed_wheels() {
        // SAFETY: input() initializes the arm selected by r#type; assertions
        // check that discriminant before reading the corresponding union arm.
        unsafe {
            for (key, scan, extended) in [(0x1e, 0x1e, false), (0x10048, 0x48, true)] {
                for down in [false, true] {
                    let i = input(Output::Key { key, down }).unwrap();
                    assert_eq!(i.r#type, INPUT_KEYBOARD);
                    let ki = i.Anonymous.ki;
                    assert_eq!(ki.wVk.0, 0);
                    assert_eq!(ki.wScan, scan);
                    assert_eq!(
                        ki.dwFlags,
                        KEYEVENTF_SCANCODE
                            | if extended {
                                KEYEVENTF_EXTENDEDKEY
                            } else {
                                KEYBD_EVENT_FLAGS(0)
                            }
                            | if down {
                                KEYBD_EVENT_FLAGS(0)
                            } else {
                                KEYEVENTF_KEYUP
                            }
                    );
                    assert_eq!(ki.time, 0);
                    assert_eq!(ki.dwExtraInfo, 0);
                }
            }
            for (index, pair) in [
                [MOUSEEVENTF_LEFTUP, MOUSEEVENTF_LEFTDOWN],
                [MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_RIGHTDOWN],
                [MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MIDDLEDOWN],
            ]
            .into_iter()
            .enumerate()
            {
                for down in [false, true] {
                    let i = input(Output::Key {
                        key: MOUSE_LEFT + index as u32,
                        down,
                    })
                    .unwrap();
                    assert_eq!(i.r#type, INPUT_MOUSE);
                    assert_eq!(i.Anonymous.mi.dwFlags, pair[usize::from(down)]);
                }
            }
            let i = input(Output::Move { x: -12, y: 34 }).unwrap();
            assert_eq!(i.r#type, INPUT_MOUSE);
            let m = i.Anonymous.mi;
            assert_eq!((m.dx, m.dy, m.dwFlags), (-12, 34, MOUSEEVENTF_MOVE));
            for horizontal in [false, true] {
                let i = input(Output::Wheel {
                    amount: -17,
                    horizontal,
                })
                .unwrap();
                assert_eq!(i.r#type, INPUT_MOUSE);
                assert_eq!(i.Anonymous.mi.mouseData, (-17_i32) as u32);
                assert_eq!(
                    i.Anonymous.mi.dwFlags,
                    if horizontal {
                        MOUSEEVENTF_HWHEEL
                    } else {
                        MOUSEEVENTF_WHEEL
                    }
                );
            }
        }
        assert!(input(Output::Wait(Duration::ZERO)).is_none());
        assert!(input(Output::Status(crate::mapping::Status::Sent)).is_none());
    }
}
