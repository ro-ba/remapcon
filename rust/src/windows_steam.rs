//! Steam handoff and crash recovery translated from Remapcon DeviceCycle.cpp
//! and src/app/main.cpp. MIT, copyright (c) 2026 ro-ba / Dylan Deverill.
//! Keep ../../LICENSE and ../../THIRD_PARTY_NOTICES.md with distributions.
use crate::{
    windows_controller::Controller,
    windows_hid::{self, own, raw, DeviceList, HidDevice},
};
use std::{
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::OwnedHandle},
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Devices::DeviceAndDriverInstallation::*,
        Foundation::*,
        Security::{
            GetTokenInformation, TokenElevation, SECURITY_ATTRIBUTES, TOKEN_ELEVATION, TOKEN_QUERY,
        },
        System::{Registry::*, Threading::*},
        UI::WindowsAndMessaging::SW_HIDE,
    },
};

// Legacy registry/mutex identity is shared with Remapcon so an interrupted
// device disable can still be recovered after upgrading to PadMux.
const PENDING: PCWSTR = w!("SOFTWARE\\Remapcon\\PendingDeviceCycle");
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

pub fn elevated() -> bool {
    let mut token = HANDLE::default();
    // SAFETY: pseudo process handle is borrowed; output token immediately
    // transfers to OwnedHandle. Initialized elevation storage has its ABI size.
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let token = own(token);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut bytes = 0;
        GetTokenInformation(
            raw(&token),
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut bytes,
        )
        .is_ok()
            && elevation.TokenIsElevated != 0
    }
}

fn status(id: &[u16], disabled: bool) -> bool {
    if id.first() == Some(&0) {
        return !disabled;
    }
    let mut node = 0;
    let mut status = CM_DEVNODE_STATUS_FLAGS::default();
    let mut problem = CM_PROB::default();
    // SAFETY: internal IDs are terminated, outputs are live initialized storage;
    // Configuration Manager only reads/querys device state here.
    unsafe {
        if CM_Locate_DevNodeW(
            &mut node,
            PCWSTR(id.as_ptr()),
            if disabled {
                CM_LOCATE_DEVNODE_PHANTOM
            } else {
                CM_LOCATE_DEVNODE_NORMAL
            },
        ) != CR_SUCCESS
        {
            return false;
        }
        if CM_Get_DevNode_Status(&mut status, &mut problem, node, 0) != CR_SUCCESS {
            return false;
        }
    }
    if disabled {
        status & DN_HAS_PROBLEM != CM_DEVNODE_STATUS_FLAGS(0) && problem == CM_PROB_DISABLED
    } else {
        status & DN_HAS_PROBLEM == CM_DEVNODE_STATUS_FLAGS(0)
    }
}
fn id_of(node: u32) -> Option<Vec<u16>> {
    let mut id = [0u16; MAX_DEVICE_ID_LEN as usize];
    // SAFETY: the binding passes the real writable slice length to the API.
    if unsafe { CM_Get_Device_IDW(node, &mut id, 0) } != CR_SUCCESS {
        return None;
    }
    let end = id.iter().position(|&c| c == 0)?;
    Some(id[..=end].to_vec())
}
fn change(
    list: &DeviceList,
    device: &SP_DEVINFO_DATA,
    state: SETUP_DI_STATE_CHANGE,
    scope: SETUP_DI_PROPERTY_CHANGE_SCOPE,
) -> bool {
    let params = SP_PROPCHANGE_PARAMS {
        ClassInstallHeader: SP_CLASSINSTALL_HEADER {
            cbSize: size_of::<SP_CLASSINSTALL_HEADER>() as u32,
            InstallFunction: DIF_PROPERTYCHANGE,
        },
        StateChange: state,
        Scope: scope,
        HwProfile: 0,
    };
    // SAFETY: list owns a live SetupAPI set; device belongs to that set; params
    // begins with the declared header and its complete struct size is supplied.
    unsafe {
        SetupDiSetClassInstallParamsW(
            list.0,
            Some(device),
            Some(&params.ClassInstallHeader),
            size_of::<SP_PROPCHANGE_PARAMS>() as u32,
        )
        .is_ok()
            && SetupDiCallClassInstaller(DIF_PROPERTYCHANGE, list.0, Some(device)).is_ok()
    }
}
fn enable(id: &[u16]) -> bool {
    if status(id, false) {
        return true;
    }
    // SAFETY: no raw input buffers; successful list has a single RAII owner.
    let Ok(list) = (unsafe { SetupDiCreateDeviceInfoList(None, None) }) else {
        return false;
    };
    let list = DeviceList(list);
    let mut data = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    // SAFETY: terminated ID and initialized output live for the call; the
    // resulting device descriptor remains associated with the live list.
    if unsafe { SetupDiOpenDeviceInfoW(list.0, PCWSTR(id.as_ptr()), None, 0, Some(&mut data)) }
        .is_err()
    {
        return false;
    }
    for _ in 0..3 {
        if status(id, false) {
            break;
        }
        change(&list, &data, DICS_ENABLE, DICS_FLAG_GLOBAL);
        if !status(id, false) {
            change(&list, &data, DICS_ENABLE, DICS_FLAG_CONFIGSPECIFIC);
        }
        if !status(id, false) {
            thread::sleep(Duration::from_millis(250));
        }
    }
    status(id, false)
}

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: this sole owner closes an opened registry key once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}
fn save_pending(child: &[u16], parent: &[u16]) -> bool {
    let mut key = HKEY::default();
    // SAFETY: fixed terminated key name, initialized output; new key has a
    // sole owner on success. Persist recovery data before any device disable.
    if unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            PENDING,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    } != ERROR_SUCCESS
    {
        return false;
    }
    let key = Key(key);
    for (name, value) in [(w!("Child"), child), (w!("Parent"), parent)] {
        let bytes: Vec<_> = value.iter().flat_map(|c| c.to_ne_bytes()).collect();
        // SAFETY: name is a static terminated string; the binding supplies the
        // initialized byte slice length. UTF-16 includes its terminator.
        if unsafe { RegSetValueExW(key.0, name, None, REG_SZ, Some(&bytes)) } != ERROR_SUCCESS {
            return false;
        }
    }
    // SAFETY: the owned registry handle stays open until flushing finishes.
    unsafe { RegFlushKey(key.0) == ERROR_SUCCESS }
}
fn read_pending(key: &Key, name: PCWSTR) -> Vec<u16> {
    let mut kind = REG_VALUE_TYPE::default();
    let mut bytes = 0;
    // SAFETY: live key/static terminated name and initialized size/type outputs;
    // the first call queries size without a buffer.
    if unsafe { RegQueryValueExW(key.0, name, None, Some(&mut kind), None, Some(&mut bytes)) }
        != ERROR_SUCCESS
        || kind != REG_SZ
        || !(2..=4096).contains(&bytes)
        || bytes % 2 != 0
    {
        return vec![0];
    }
    let mut data = vec![0u8; bytes as usize];
    // SAFETY: allocated buffer has exactly the reported size; a racing growth
    // returns ERROR_MORE_DATA rather than overrunning it. Reject malformed UTF-16
    // byte lengths (the C++ buffer must never be emulated with an odd byte count).
    if unsafe {
        RegQueryValueExW(
            key.0,
            name,
            None,
            Some(&mut kind),
            Some(data.as_mut_ptr()),
            Some(&mut bytes),
        )
    } != ERROR_SUCCESS
        || bytes as usize > data.len()
        || bytes % 2 != 0
    {
        return vec![0];
    }
    let mut value: Vec<_> = data[..bytes as usize]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_ne_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    value.push(0);
    value
}
fn recover_pending() -> bool {
    let mut key = HKEY::default();
    // SAFETY: fixed key name and initialized output. Read-only open; all actual
    // restoration is limited to IDs recorded before the original disable.
    let opened =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, PENDING, None, KEY_QUERY_VALUE, &mut key) };
    if opened == ERROR_FILE_NOT_FOUND {
        return true;
    }
    if opened != ERROR_SUCCESS {
        return false;
    }
    let key = Key(key);
    let parent = read_pending(&key, w!("Parent"));
    let child = read_pending(&key, w!("Child"));
    drop(key);
    if child == [0] {
        return false;
    }
    let restored = enable(&parent) && enable(&child);
    if restored {
        // SAFETY: delete only the fixed recovery key after both nodes are enabled.
        let _ = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, PENDING) };
    }
    restored
}

fn cycle(path: &str, down_event: HANDLE) -> u8 {
    if !recover_pending() || !windows_hid::is_controller_interface(path) {
        return 2;
    }
    // SAFETY: no raw input; successful device list transfers to its RAII owner.
    let Ok(list) = (unsafe { SetupDiCreateDeviceInfoList(None, None) }) else {
        return 3;
    };
    let list = DeviceList(list);
    let path = wide(path);
    let mut interface = SP_DEVICE_INTERFACE_DATA {
        cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
        ..Default::default()
    };
    let mut data = SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    // SAFETY: initialized ABI-sized descriptors, terminated validated HID path.
    // Detail query has no output buffer; only devnode metadata is requested.
    let opened = unsafe {
        SetupDiOpenDeviceInterfaceW(list.0, PCWSTR(path.as_ptr()), 0, Some(&mut interface)).is_ok()
            && (SetupDiGetDeviceInterfaceDetailW(
                list.0,
                &interface,
                None,
                0,
                None,
                Some(&mut data),
            )
            .is_ok()
                || GetLastError() == ERROR_INSUFFICIENT_BUFFER)
            && data.DevInst != 0
    };
    if !opened {
        return 3;
    }
    let Some(child) = id_of(data.DevInst) else {
        return 4;
    };
    let mut parent_node = 0;
    // SAFETY: initialized output, devnode supplied by SetupAPI.
    let parent = if unsafe { CM_Get_Parent(&mut parent_node, data.DevInst, 0) } == CR_SUCCESS {
        id_of(parent_node).unwrap_or_else(|| vec![0])
    } else {
        vec![0]
    };
    if child == [0] || !save_pending(&child, &parent) {
        return 4;
    }
    if !change(&list, &data, DICS_DISABLE, DICS_FLAG_GLOBAL) {
        change(&list, &data, DICS_DISABLE, DICS_FLAG_CONFIGSPECIFIC);
    }
    let mut down = false;
    for _ in 0..20 {
        down = status(&child, true) || status(&parent, true);
        if down {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    drop(list);
    if down {
        // SAFETY: the helper borrows the inherited event. SetEvent validates the
        // kernel handle; it never treats its numeric value as a memory pointer.
        let _ = unsafe { SetEvent(down_event) };
        thread::sleep(Duration::from_millis(500));
    }
    if !recover_pending() {
        5
    } else if down {
        0
    } else {
        6
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Helper<'a> {
    Recover,
    Cycle(&'a str, usize),
    Invalid,
}
fn parse_helper(args: &[String]) -> Option<Helper<'_>> {
    match args.first().map(String::as_str) {
        Some("--recover-device-cycle") if args.len() == 1 => Some(Helper::Recover),
        Some("--device-cycle") if args.len() == 3 => Some(match args[2].parse::<usize>() {
            Ok(handle) if handle != 0 => Helper::Cycle(&args[1], handle),
            _ => Helper::Invalid,
        }),
        Some("--recover-device-cycle" | "--device-cycle") => Some(Helper::Invalid),
        _ => None,
    }
}
pub fn helper_command(args: &[String]) -> Option<u8> {
    let helper = parse_helper(args)?;
    // SAFETY: static mutex name; success transfers its handle to one owner.
    let Ok(mutex) = (unsafe { CreateMutexW(None, false, w!("Local\\RemapconDeviceCycle")) }) else {
        return Some(7);
    };
    let mutex = own(mutex);
    // SAFETY: the live mutex is owned for the entire wait and operation.
    let lock = unsafe { WaitForSingleObject(raw(&mutex), 15000) };
    if lock != WAIT_OBJECT_0 && lock != WAIT_ABANDONED {
        return Some(7);
    }
    let result = match helper {
        Helper::Recover => {
            if recover_pending() {
                0
            } else {
                5
            }
        }
        Helper::Cycle(path, handle) => cycle(path, HANDLE(handle as *mut _)),
        Helper::Invalid => 1,
    };
    // SAFETY: the successful/abandoned wait acquired this mutex on this thread.
    let _ = unsafe { ReleaseMutex(raw(&mutex)) };
    Some(result)
}

pub struct DeviceCycle {
    process: OwnedHandle,
    event: OwnedHandle,
}
impl DeviceCycle {
    pub fn start(path: &str) -> Result<Self, String> {
        if !elevated() || path.is_empty() || path.contains(['"', '\0']) {
            return Err("device cycle requires elevation and a valid HID path".into());
        }
        let security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            bInheritHandle: true.into(),
            lpSecurityDescriptor: std::ptr::null_mut(),
        };
        // SAFETY: initialized security attributes; new inheritable event has one
        // parent owner. Child receives a borrowed inherited kernel handle.
        let event = own(
            unsafe { CreateEventW(Some(&security), true, false, PCWSTR::null()) }
                .map_err(|e| e.to_string())?,
        );
        let process_handle = launch_helper(
            &format!("--device-cycle \"{}\" {}", path, raw(&event).0 as usize),
            true,
        )?;
        Ok(Self {
            process: process_handle,
            event,
        })
    }
    pub fn wait_down(&self) -> bool {
        // SAFETY: both owned handles remain live for the bounded wait.
        unsafe {
            WaitForMultipleObjects(&[raw(&self.event), raw(&self.process)], false, 12000)
                == WAIT_OBJECT_0
        }
    }
    pub fn finish(self, timeout_ms: u32) -> bool {
        finish_process(&self.process, timeout_ms)
    }
}

fn launch_helper(arguments: &str, inherit: bool) -> Result<OwnedHandle, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let application: Vec<_> = executable.as_os_str().encode_wide().chain([0]).collect();
    let mut command = wide(&format!("\"{}\" {arguments}", executable.display()));
    let startup = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        dwFlags: STARTF_USESHOWWINDOW,
        wShowWindow: SW_HIDE.0 as u16,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // SAFETY: application/command are terminated/live, command is writable;
    // startup/output have ABI sizes. Inheritance matches the C++ helper IPC.
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(command.as_mut_ptr())),
            None,
            None,
            inherit,
            CREATE_NO_WINDOW,
            None,
            PCWSTR::null(),
            &startup,
            &mut process,
        )
    }
    .map_err(|e| e.to_string())?;
    let handle = own(process.hProcess);
    drop(own(process.hThread));
    Ok(handle)
}
fn finish_process(process: &OwnedHandle, timeout_ms: u32) -> bool {
    let mut code = 1;
    // SAFETY: owned process handle is live during wait and exit-code query.
    // On timeout only handles close; the helper continues restoring nodes.
    unsafe {
        WaitForSingleObject(raw(process), timeout_ms) == WAIT_OBJECT_0
            && GetExitCodeProcess(raw(process), &mut code).is_ok()
            && code == 0
    }
}
pub fn recover_interrupted() -> bool {
    if !elevated() {
        return true;
    }
    // Run recovery in the same cross-process named mutex as C++, not on this
    // thread outside the mutex. A timed-out helper keeps restoring independently.
    launch_helper("--recover-device-cycle", false)
        .is_ok_and(|process| finish_process(&process, 12000))
}

pub fn take_controller(
    path: &str,
    mut should_take: impl FnMut() -> bool,
) -> Result<Controller, String> {
    let cycle = DeviceCycle::start(path)?;
    if !cycle.wait_down() {
        cycle.finish(6000);
        return Err("device cycle did not signal disabled state".into());
    }
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut claimed = None;
    while should_take() && Instant::now() < deadline {
        if let Ok(mut device) = HidDevice::open_exclusive(path) {
            if wait_for_state(&mut device, 350) {
                claimed = Some(Controller::new(device));
            }
        }
        if claimed.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let restored = cycle.finish(6000);
    if !restored || !should_take() {
        return Err("device cycle recovery or activation check failed".into());
    }
    claimed.ok_or_else(|| "Steam controller exclusive acquisition failed".into())
}
pub fn return_controller(path: &str) -> bool {
    DeviceCycle::start(path).is_ok_and(|cycle| cycle.finish(12000))
}

pub fn wait_for_state(device: &mut HidDevice, timeout_ms: u32) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.into());
    let mut report = [0u8; 64];
    while Instant::now() < deadline {
        let before = Instant::now();
        let remaining = deadline
            .saturating_duration_since(before)
            .as_millis()
            .max(1) as u32;
        match device.read(&mut report, remaining) {
            Ok(size) if size > 0 && matches!(report[0], 0x42 | 0x45) => return true,
            Ok(0) if before.elapsed() < Duration::from_millis(2) => return false,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

/// Close HID before the return cycle, including setup failure and panic paths.
pub struct SteamSession<'a> {
    path: &'a str,
    controller: Option<Controller>,
    taken: bool,
}
#[derive(Debug)]
pub enum ClaimError {
    Access(String),
    DisableLizard(String),
}
impl std::fmt::Display for ClaimError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(message) | Self::DisableLizard(message) => formatter.write_str(message),
        }
    }
}
impl std::error::Error for ClaimError {}
#[derive(Debug)]
pub enum CloseError {
    Restore(String),
    Return(Option<String>),
}
impl std::fmt::Display for CloseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Restore(message) => formatter.write_str(message),
            Self::Return(restore) => {
                if let Some(message) = restore {
                    write!(formatter, "Lizard restore: {message}; ")?;
                }
                formatter
                    .write_str("Steam return cycle failed; pending recovery record is retained")
            }
        }
    }
}
impl std::error::Error for CloseError {}
fn close_result(restored: Result<(), String>, returned: bool) -> Result<(), CloseError> {
    if returned {
        restored.map_err(CloseError::Restore)
    } else {
        Err(CloseError::Return(restored.err()))
    }
}
impl<'a> SteamSession<'a> {
    pub fn claim(path: &'a str, mut should_take: impl FnMut() -> bool) -> Result<Self, ClaimError> {
        let (controller, taken) = match HidDevice::open_exclusive(path) {
            Ok(device) => (Controller::new(device), false),
            Err(error) => {
                if !should_take() {
                    return Err(ClaimError::Access(error.to_string()));
                }
                (
                    take_controller(path, should_take).map_err(ClaimError::Access)?,
                    true,
                )
            }
        };
        let mut session = Self {
            path,
            controller: Some(controller),
            taken,
        };
        session
            .controller()
            .disable_lizard_mode()
            .map_err(|e| ClaimError::DisableLizard(e.to_string()))?;
        Ok(session)
    }
    pub fn controller(&mut self) -> &mut Controller {
        self.controller
            .as_mut()
            .expect("SteamSession used after close")
    }
    pub fn taken_from_steam(&self) -> bool {
        self.taken
    }
    pub fn close(&mut self) -> Result<(), CloseError> {
        self.close_before_return(|| {})
    }
    /// Notify after restoring/closing HID, before Steam's return cycle (C++ order).
    pub fn close_before_return(&mut self, before_return: impl FnOnce()) -> Result<(), CloseError> {
        let restored = if let Some(mut controller) = self.controller.take() {
            let restored = controller.restore_lizard_mode().map_err(|e| e.to_string());
            drop(controller); // Release the exclusive HID before cycling back.
            restored
        } else {
            Ok(())
        };
        if self.taken {
            before_return();
        }
        let taken = std::mem::take(&mut self.taken);
        close_result(restored, !taken || return_controller(self.path))
    }
}
impl Drop for SteamSession<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("controller cleanup: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helper_arguments_and_controller_guard_do_not_modify_devices() {
        assert!(close_result(Ok(()), true).is_ok());
        assert!(matches!(
            close_result(Err("unplugged".into()), true),
            Err(CloseError::Restore(_))
        ));
        for restored in [Ok(()), Err("unplugged".into())] {
            assert!(matches!(
                close_result(restored, false),
                Err(CloseError::Return(_))
            ));
        }
        assert_eq!(parse_helper(&[]), None);
        assert_eq!(parse_helper(&["--list".into()]), None);
        assert_eq!(
            parse_helper(&["--recover-device-cycle".into()]),
            Some(Helper::Recover)
        );
        for value in ["0", "invalid", "18446744073709551616"] {
            assert_eq!(
                parse_helper(&["--device-cycle".into(), "path".into(), value.into()]),
                Some(Helper::Invalid)
            );
        }
        assert_eq!(
            parse_helper(&["--device-cycle".into(), "path".into(), "123".into()]),
            Some(Helper::Cycle("path", 123))
        );
        assert!(!windows_hid::is_controller_interface(""));
        assert!(!windows_hid::is_controller_interface("invalid\0path"));
        assert!(!windows_hid::is_controller_interface(
            "padmux-nonexistent-interface"
        ));
    }
}
