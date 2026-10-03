//! Controller control translated from Remapcon src/steam/SteamController.cpp.
//! Derived from SteamlessController 26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2.
//! Copyright (c) 2026 Dylan Deverill; Copyright (c) 2026 ro-ba. MIT: ../../LICENSE.
#![forbid(unsafe_code)]

use crate::{
    controller::{command, HapticLimiter},
    windows_hid::HidDevice,
};
use std::time::Instant;
use windows::core::{Error, Result, HRESULT};

pub struct Controller {
    device: HidDevice,
    active: bool,
    started: Instant,
    haptics: HapticLimiter,
}

impl Controller {
    pub fn new(device: HidDevice) -> Self {
        Self {
            device,
            active: false,
            started: Instant::now(),
            haptics: HapticLimiter::default(),
        }
    }

    fn feature(&mut self, id: u8, payload: &[u8]) -> Result<()> {
        let report = command(id, payload)
            .map_err(|error| Error::new(HRESULT(0x80070057u32 as i32), error))?;
        self.device.feature(&report)
    }

    pub fn disable_lizard_mode(&mut self) -> Result<()> {
        // Arm restoration before the first command, so partial setup also restores.
        self.active = true;
        self.feature(0x81, &[])?;
        self.feature(0x87, &[0x30, 0, 0])?;
        self.feature(0x87, &[8, 0, 0, 7, 0, 0])
    }

    pub fn keepalive(&mut self) -> Result<()> {
        self.feature(0x81, &[])?;
        self.feature(0x87, &[8, 0, 0, 7, 0, 0])
    }

    pub fn read(&mut self, report: &mut [u8], timeout_ms: u32) -> Result<usize> {
        self.device.read(report, timeout_ms)
    }

    pub fn pulse(&mut self, left: bool, click: bool) -> Result<()> {
        self.device
            .output(&self.haptics.pulse(left, click, self.started.elapsed()))
    }

    pub fn movement_tick(&mut self, left: bool) -> Result<bool> {
        let Some(report) = self.haptics.movement(left, self.started.elapsed()) else {
            return Ok(false);
        };
        self.device.output(&report)?;
        Ok(true)
    }

    pub fn restore_lizard_mode(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        // Attempt both firmware defaults even if an earlier cleanup command fails.
        // Successful-path order matches EnableLizardMode, with no rumble source.
        let rumble = self.device.output(&[0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let imu = self.feature(0x87, &[0x30, 0, 0]);
        let mapping = self.feature(0x85, &[]);
        let settings = self.feature(0x8e, &[]);
        let result = rumble.and(imu).and(mapping).and(settings);
        if result.is_ok() {
            self.active = false;
        }
        result
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        // Best effort on read/setup errors or panic; normal exit checks the result.
        let _ = self.restore_lizard_mode();
    }
}
