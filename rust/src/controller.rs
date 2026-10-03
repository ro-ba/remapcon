//! Report layout/transport translated from Remapcon src/steam/SteamController.{h,cpp}.
//! Derived from SteamlessController 26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2.
//! Copyright (c) 2026 Dylan Deverill; Copyright (c) 2026 ro-ba. MIT: ../../LICENSE.
#![forbid(unsafe_code)]

pub const VALVE_VID: u16 = 0x28de;
pub const PRODUCT_IDS: [u16; 4] = [0x1302, 0x1303, 0x1304, 0x1305];
pub const USAGE_PAGE: u16 = 0xff00;
pub const CONTROLLER_USAGE: u16 = 1;

/// C++ BuildCmd: feature report 0x01; no speculative 0x02 fallback.
pub fn command(id: u8, payload: &[u8]) -> Result<[u8; 64], &'static str> {
    if payload.len() > 61 {
        return Err("feature command payload exceeds 61 bytes");
    }
    let mut report = [0u8; 64];
    report[..3].copy_from_slice(&[1, id, payload.len() as u8]);
    report[3..3 + payload.len()].copy_from_slice(payload);
    Ok(report)
}

#[derive(Default)]
pub struct HapticLimiter {
    last: [Option<std::time::Duration>; 2],
}
impl HapticLimiter {
    pub fn pulse(&mut self, left: bool, click: bool, now: std::time::Duration) -> [u8; 4] {
        let side = usize::from(!left);
        self.last[side] = Some(now);
        [0x82, side as u8, if click { 2 } else { 1 }, 0]
    }

    pub fn movement(&mut self, left: bool, now: std::time::Duration) -> Option<[u8; 4]> {
        let side = usize::from(!left);
        if self.last[side]
            .is_some_and(|last| now.saturating_sub(last) < std::time::Duration::from_millis(50))
        {
            return None;
        }
        Some(self.pulse(left, false, now))
    }
}

pub fn transport(path: &str) -> &'static str {
    let path = path.to_ascii_lowercase();
    if [
        "{00001812-0000-1000-8000-00805f9b34fb}",
        "bthledevice",
        "bthenum",
    ]
    .iter()
    .any(|token| path.contains(token))
    {
        "bluetooth"
    } else if path.contains("pid_1304") || path.contains("pid_1305") {
        "dongle"
    } else if path.contains("pid_1302") || path.contains("pid_1303") {
        "wired"
    } else {
        "unknown"
    }
}

const BUTTON_BITS: [(usize, u8, &str); 28] = [
    (0, 0x01, "A"),
    (0, 0x02, "B"),
    (0, 0x04, "X"),
    (0, 0x08, "Y"),
    (0, 0x20, "R3"),
    (0, 0x40, "Menu"),
    (0, 0x80, "R4"),
    (1, 0x01, "R5"),
    (1, 0x02, "RB"),
    (1, 0x04, "DPadDown"),
    (1, 0x08, "DPadRight"),
    (1, 0x10, "DPadLeft"),
    (1, 0x20, "DPadUp"),
    (1, 0x40, "View"),
    (1, 0x80, "L3"),
    (2, 0x01, "Steam"),
    (2, 0x02, "L4"),
    (2, 0x04, "L5"),
    (2, 0x08, "LB"),
    (2, 0x10, "RightStickTouch"),
    (2, 0x20, "RightPadTouch"),
    (2, 0x40, "RightPadClick"),
    (2, 0x80, "RightTriggerFull"),
    (3, 0x01, "LeftStickTouch"),
    (3, 0x02, "LeftPadTouch"),
    (3, 0x04, "LeftPadClick"),
    (3, 0x10, "RightGrip"),
    (3, 0x20, "LeftGrip"),
];

#[derive(Debug, PartialEq)]
pub struct Pad {
    pub x: i16,
    pub y: i16,
    pub area: u16,
}

#[derive(Debug, PartialEq)]
pub struct State {
    pub report_id: u8,
    pub sequence: u8,
    pub flags: [u8; 4],
    pub buttons: Vec<&'static str>,
    pub triggers: [i16; 2],
    pub sticks: Option<[[i16; 2]; 2]>,
    pub pads: Option<[Pad; 2]>,
}

/// Unknown IDs stay raw; battery/status layout is not guessed.
pub fn parse(report: &[u8]) -> Result<Option<State>, &'static str> {
    let Some(&id) = report.first() else {
        return Err("empty report");
    };
    if id != 0x42 && id != 0x45 {
        return Ok(None);
    }
    if report.len() < 10 {
        return Err("state report is shorter than 10 bytes");
    }
    // Offsets are covered by the same 10/18/30-byte gates as the C++ consumer.
    let signed = |offset| i16::from_le_bytes([report[offset], report[offset + 1]]);
    let flags = [report[2], report[3], report[4], report[5]];
    let buttons = BUTTON_BITS
        .iter()
        .filter_map(|&(byte, mask, name)| (flags[byte] & mask != 0).then_some(name))
        .collect();
    let sticks = (report.len() >= 18).then(|| [[signed(10), signed(12)], [signed(14), signed(16)]]);
    let pad = |offset| Pad {
        x: signed(offset),
        y: signed(offset + 2),
        area: u16::from_le_bytes([report[offset + 4], report[offset + 5]]),
    };
    Ok(Some(State {
        report_id: id,
        sequence: report[1],
        flags,
        buttons,
        triggers: [signed(6), signed(8)],
        sticks,
        pads: (report.len() >= 30).then(|| [pad(18), pad(24)]),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_layout_and_haptic_sides_rate_limit_match_reference() {
        let clear = command(0x81, &[]).unwrap();
        assert_eq!(&clear[..4], &[1, 0x81, 0, 0]);
        let settings = command(0x87, &[8, 0, 0, 7, 0, 0]).unwrap();
        assert_eq!(&settings[..9], &[1, 0x87, 6, 8, 0, 0, 7, 0, 0]);
        assert!(settings[9..].iter().all(|&byte| byte == 0));
        assert!(command(0x87, &[0; 62]).is_err());
        let mut haptics = HapticLimiter::default();
        let ms = std::time::Duration::from_millis;
        assert_eq!(haptics.movement(true, ms(0)), Some([0x82, 0, 1, 0]));
        assert_eq!(haptics.movement(true, ms(49)), None);
        assert_eq!(haptics.movement(false, ms(49)), Some([0x82, 1, 1, 0]));
        assert_eq!(haptics.movement(true, ms(50)), Some([0x82, 0, 1, 0]));
        assert_eq!(haptics.pulse(true, true, ms(51)), [0x82, 0, 2, 0]);
        assert_eq!(haptics.movement(true, ms(100)), None);
        assert_eq!(haptics.movement(true, ms(101)), Some([0x82, 0, 1, 0]));
    }

    #[test]
    fn reference_layout_signed_values_and_button_bits() {
        let mut report = [0u8; 64];
        report[0] = 0x45;
        report[1] = 255;
        // Fixed reference vector, independently checked against C++ offsets/masks.
        report[2..6].copy_from_slice(&[0x81, 0x21, 0x66, 0x36]);
        report[6..30].copy_from_slice(&[
            0xff, 0x7f, 0x00, 0x20, 0x00, 0x80, 0xff, 0x7f, 0xff, 0xff, 0x00, 0x00, 0x34, 0x12,
            0xfe, 0xff, 0x08, 0x07, 0x00, 0x80, 0x01, 0x00, 0xff, 0xff,
        ]);
        let state = parse(&report).unwrap().unwrap();
        assert_eq!(state.sequence, 255);
        assert_eq!(state.triggers, [32767, 8192]);
        assert_eq!(state.sticks, Some([[-32768, 32767], [-1, 0]]));
        assert_eq!(
            state.pads,
            Some([
                Pad {
                    x: 0x1234,
                    y: -2,
                    area: 1800
                },
                Pad {
                    x: -32768,
                    y: 1,
                    area: 65535
                },
            ])
        );
        assert_eq!(
            state.buttons,
            [
                "A",
                "R4",
                "R5",
                "DPadUp",
                "L4",
                "L5",
                "RightPadTouch",
                "RightPadClick",
                "LeftPadTouch",
                "LeftPadClick",
                "RightGrip",
                "LeftGrip"
            ]
        );
        for &(byte, mask, name) in &BUTTON_BITS {
            let mut single = [0u8; 30];
            single[0] = 0x42;
            single[2 + byte] = mask;
            assert_eq!(parse(&single).unwrap().unwrap().buttons, [name]);
        }
    }

    #[test]
    fn both_report_ids_and_all_truncation_boundaries() {
        assert!(parse(&[]).is_err());
        assert_eq!(parse(&[0x43]).unwrap(), None);
        for id in [0x42, 0x45] {
            let mut report = [0u8; 64];
            report[0] = id;
            for len in 1..=64 {
                let result = parse(&report[..len]);
                if len < 10 {
                    assert!(result.is_err());
                    continue;
                }
                let state = result.unwrap().unwrap();
                assert_eq!(state.report_id, id);
                assert_eq!(state.sticks.is_some(), len >= 18);
                assert_eq!(state.pads.is_some(), len >= 30);
            }
        }
    }

    #[test]
    fn bluetooth_path_takes_priority_over_product_id() {
        assert_eq!(transport(r"\\?\HID#VID_28DE&PID_1304"), "dongle");
        assert_eq!(transport("pid_1305"), "dongle");
        assert_eq!(transport("pid_1302"), "wired");
        assert_eq!(transport("pid_1303"), "wired");
        assert_eq!(transport("bthledevice#pid_1303"), "bluetooth");
        assert_eq!(transport("bthenum#pid_1304"), "bluetooth");
        assert_eq!(
            transport("pid_1303#{00001812-0000-1000-8000-00805f9b34fb}"),
            "bluetooth"
        );
        assert_eq!(transport("unrecognized"), "unknown");
    }
}
