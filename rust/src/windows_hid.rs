//! Windows-only HID boundary translated from Remapcon src/hid/HidDevice.cpp.
//! Derived from SteamlessController 26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2.
//! Copyright (c) 2026 Dylan Deverill; Copyright (c) 2026 ro-ba. MIT: ../../LICENSE.

use crate::controller::{CONTROLLER_USAGE, PRODUCT_IDS, USAGE_PAGE, VALVE_VID};
use std::mem::{offset_of, size_of};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{Error, Result, HRESULT, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Devices::HumanInterfaceDevice::*;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Console::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::IO::*;

pub(crate) struct DeviceList(pub(crate) HDEVINFO);
impl Drop for DeviceList {
    fn drop(&mut self) {
        // SAFETY: this sole owner destroys the live SetupAPI list exactly once.
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

pub(crate) fn own(handle: HANDLE) -> OwnedHandle {
    // SAFETY: callers only pass a newly created valid file/event/snapshot handle on API
    // success. Ownership transfers here; OwnedHandle closes it exactly once.
    unsafe { OwnedHandle::from_raw_handle(handle.0) }
}
pub(crate) fn raw(handle: &OwnedHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle())
}

fn caps(handle: &OwnedHandle) -> Result<HIDP_CAPS> {
    // SAFETY: initialized outputs/live handle; preparsed allocation is freed
    // after querying caps, including the query's failure path.
    unsafe {
        let mut data = PHIDP_PREPARSED_DATA::default();
        if !HidD_GetPreparsedData(raw(handle), &mut data) {
            return Err(Error::from_thread());
        }
        let mut caps = HIDP_CAPS::default();
        let status = HidP_GetCaps(data, &mut caps);
        HidD_FreePreparsedData(data);
        if status != HIDP_STATUS_SUCCESS {
            return Err(Error::new(
                HRESULT(0x80004005u32 as i32),
                "HidP_GetCaps failed",
            ));
        }
        Ok(caps)
    }
}

fn controller_caps(handle: &OwnedHandle) -> Option<(u16, HIDP_CAPS)> {
    let mut attrs = HIDD_ATTRIBUTES {
        Size: size_of::<HIDD_ATTRIBUTES>() as u32,
        ..Default::default()
    };
    // SAFETY: the owned query handle is live; attributes is initialized writable
    // storage with the ABI-required size. The call does not modify the device.
    if !unsafe { HidD_GetAttributes(raw(handle), &mut attrs) }
        || attrs.VendorID != VALVE_VID
        || !PRODUCT_IDS.contains(&attrs.ProductID)
    {
        return None;
    }
    let caps = caps(handle).ok()?;
    (caps.UsagePage == USAGE_PAGE && caps.Usage == CONTROLLER_USAGE)
        .then_some((attrs.ProductID, caps))
}

pub fn is_controller_interface(path: &str) -> bool {
    if path.is_empty() || path.contains('\0') {
        return false;
    }
    let path: Vec<_> = path.encode_utf16().chain([0]).collect();
    // SAFETY: live terminated path; zero-access shared query does not write or
    // acquire controller ownership. Successful handle gets one owner at once.
    let Ok(handle) = (unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }) else {
        return false;
    };
    controller_caps(&own(handle)).is_some()
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub path: String,
    pub pid: u16,
    pub caps: HIDP_CAPS,
}

pub fn enumerate() -> Result<Vec<DeviceInfo>> {
    // SAFETY: cbSize uses the actual ABI, outputs are initialized. List/query
    // handles have sole owners. Each variable-sized detail allocation is u32
    // aligned and bounded by the required byte count returned by SetupAPI.
    unsafe {
        let guid = HidD_GetHidGuid();
        let list = DeviceList(SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )?);
        let mut devices = Vec::new();
        for index in 0.. {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if let Err(error) =
                SetupDiEnumDeviceInterfaces(list.0, None, &guid, index, &mut interface)
            {
                if error.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0) {
                    break;
                }
                return Err(error);
            }
            let mut needed = 0;
            let probe = SetupDiGetDeviceInterfaceDetailW(
                list.0,
                &interface,
                None,
                0,
                Some(&mut needed),
                None,
            );
            if !matches!(probe, Err(ref e) if e.code() == HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0))
            {
                probe?;
                continue;
            }
            if needed < size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 {
                continue;
            }
            let mut storage = vec![0u32; (needed as usize).div_ceil(size_of::<u32>())];
            let detail = storage
                .as_mut_ptr()
                .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            SetupDiGetDeviceInterfaceDetailW(list.0, &interface, Some(detail), needed, None, None)?;
            let offset = offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);
            // DevicePath is u16 aligned, and this slice stays inside initialized
            // storage. Search for termination within the API's byte count.
            let units = std::slice::from_raw_parts(
                storage.as_ptr().cast::<u8>().add(offset).cast::<u16>(),
                (needed as usize - offset) / 2,
            );
            let Some(end) = units.iter().position(|&unit| unit == 0) else {
                continue;
            };
            let path = String::from_utf16(&units[..end]).map_err(|_| {
                Error::new(HRESULT(0x80070057u32 as i32), "invalid UTF-16 HID path")
            })?;
            let Ok(query) = CreateFileW(
                PCWSTR(units.as_ptr()),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) else {
                continue;
            };
            let query = own(query);
            let Some((pid, caps)) = controller_caps(&query) else {
                continue;
            };
            devices.push(DeviceInfo { path, pid, caps });
        }
        // Match EnumerateAll product order, preserving order within each PID.
        devices.sort_by_key(|device| PRODUCT_IDS.iter().position(|&pid| pid == device.pid));
        Ok(devices)
    }
}

pub struct HidDevice {
    file: OwnedHandle,
    event: OwnedHandle,
    output_len: usize,
    feature_len: usize,
}
impl HidDevice {
    pub fn open(path: &str) -> Result<Self> {
        Self::open_with_share(path, FILE_SHARE_READ | FILE_SHARE_WRITE)
    }

    pub fn open_exclusive(path: &str) -> Result<Self> {
        Self::open_with_share(path, FILE_SHARE_READ)
    }

    fn open_with_share(path: &str, share: FILE_SHARE_MODE) -> Result<Self> {
        if path.contains('\0') {
            return Err(Error::new(HRESULT(0x80070057u32 as i32), "NUL in HID path"));
        }
        let path: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        // SAFETY: terminated path lives through CreateFileW; successful handles
        // immediately get sole owners. Event failure also closes the file.
        unsafe {
            let file = own(CreateFileW(
                PCWSTR(path.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                share,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )?);
            let event = own(CreateEventW(None, true, false, PCWSTR::null())?);
            let caps = caps(&file)?;
            Ok(Self {
                file,
                event,
                output_len: if caps.OutputReportByteLength == 0 {
                    64
                } else {
                    caps.OutputReportByteLength as usize
                },
                feature_len: if caps.FeatureReportByteLength == 0 {
                    64
                } else {
                    caps.FeatureReportByteLength as usize
                },
            })
        }
    }

    pub fn feature(&mut self, report: &[u8]) -> Result<()> {
        let mut buffer = vec![0u8; self.feature_len];
        let count = report.len().min(buffer.len());
        buffer[..count].copy_from_slice(&report[..count]);
        // SAFETY: owned live handle; initialized buffer has the exact feature
        // report length and stays live throughout synchronous HidD_SetFeature.
        unsafe {
            if HidD_SetFeature(raw(&self.file), buffer.as_ptr().cast(), buffer.len() as u32) {
                Ok(())
            } else {
                Err(Error::from_thread())
            }
        }
    }

    pub fn output(&mut self, report: &[u8]) -> Result<()> {
        let mut buffer = vec![0u8; self.output_len.max(report.len())];
        buffer[..report.len()].copy_from_slice(report);
        // SAFETY: &mut self serializes all I/O. Buffer/event/OVERLAPPED stay live
        // until completion, including cancellation/drain on every wait failure.
        unsafe {
            ResetEvent(raw(&self.event))?;
            let mut overlapped = OVERLAPPED {
                hEvent: raw(&self.event),
                ..Default::default()
            };
            let mut bytes = 0;
            if let Err(error) = WriteFile(
                raw(&self.file),
                Some(&buffer),
                Some(&mut bytes),
                Some(&mut overlapped),
            ) {
                if error.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
                    return Err(error);
                }
                let wait = WaitForSingleObject(raw(&self.event), 1000);
                if wait != WAIT_OBJECT_0 {
                    let _ = CancelIoEx(raw(&self.file), Some(&overlapped));
                    GetOverlappedResult(raw(&self.file), &overlapped, &mut bytes, true)?;
                } else {
                    GetOverlappedResult(raw(&self.file), &overlapped, &mut bytes, false)?;
                }
            }
            if bytes as usize != buffer.len() {
                return Err(Error::new(
                    HRESULT(0x80004005u32 as i32),
                    "partial HID output write",
                ));
            }
            Ok(())
        }
    }

    /// Zero means timeout. Actual read errors remain errors for the diagnostic.
    pub fn read(&mut self, buffer: &mut [u8], timeout_ms: u32) -> Result<usize> {
        // SAFETY: &mut self prevents concurrent read/close/reopen. Buffer/event/
        // OVERLAPPED stay live and unmoved until completion or cancellation drains.
        // No return/panic path after a pending read skips that drain.
        unsafe {
            ResetEvent(raw(&self.event))?;
            let mut overlapped = OVERLAPPED {
                hEvent: raw(&self.event),
                ..Default::default()
            };
            let mut bytes = 0;
            if let Err(error) = ReadFile(
                raw(&self.file),
                Some(buffer),
                Some(&mut bytes),
                Some(&mut overlapped),
            ) {
                if error.code() != HRESULT::from_win32(ERROR_IO_PENDING.0) {
                    return Err(error);
                }
                let wait = WaitForSingleObject(raw(&self.event), timeout_ms);
                if wait != WAIT_OBJECT_0 {
                    let wait_error = (wait != WAIT_TIMEOUT).then(Error::from_thread);
                    // Cancel only this read; ERROR_NOT_FOUND can mean completion
                    // won the race. Always wait before OVERLAPPED leaves scope.
                    let _ = CancelIoEx(raw(&self.file), Some(&overlapped));
                    let _ = GetOverlappedResult(raw(&self.file), &overlapped, &mut bytes, true);
                    return match wait_error {
                        Some(error) => Err(error),
                        None => Ok(0),
                    };
                }
                GetOverlappedResult(raw(&self.file), &overlapped, &mut bytes, false)?;
            }
            Ok(bytes as usize)
        }
    }
}

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

// SAFETY: Windows requires an extern-system callback. It has process lifetime,
// touches only an atomic, and cannot unwind or access device/buffer memory.
unsafe extern "system" fn stop_handler(event: u32) -> windows::core::BOOL {
    if event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT {
        STOP_REQUESTED.store(true, Ordering::Relaxed);
        true.into()
    } else {
        false.into()
    }
}

pub fn install_stop_handler() -> Result<()> {
    // SAFETY: callback is a static function with the required ABI and no captures.
    unsafe { SetConsoleCtrlHandler(Some(stop_handler), true) }
}

pub fn stop_requested() -> bool {
    STOP_REQUESTED.load(Ordering::Relaxed)
}
