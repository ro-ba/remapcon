//! PadMux UI/native host derived from Remapcon, with TEMP diagnostic checks.
//! MIT, copyright 2026 ro-ba; preserve ../../LICENSE / THIRD_PARTY_NOTICES.md.
use crate::{
    editor::Editor,
    webview, windows_app, windows_config, windows_icons,
    windows_runtime::{Controls, Worker},
    windows_shell::{self, Shell},
    windows_steam, windows_update,
    windows_webview::WebUi,
};
struct EditDirectory(std::path::PathBuf);
impl Drop for EditDirectory {
    fn drop(&mut self) {
        // This directory was created exclusively by this process. It never
        // points at the source settings or the shared WebView profile.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn after_render(
    ui: &WebUi,
    condition: &str,
    action: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Owned diagnostic scripts operate only on this temporary editor's DOM.
    // Poll rendered state instead of assuming WebView IPC/render timing.
    ui.execute_script(&format!(r#"(() => {{ let attempts=0; const timer=setInterval(() => {{
        if ({condition}) {{ clearInterval(timer); try {{ {action} }} catch (_) {{ chrome.webview.postMessage('__selfTestFailed'); }} }}
        else if (++attempts>=500) {{ clearInterval(timer); chrome.webview.postMessage('__selfTestFailed'); }}
    }},10); }})();"#))
}
use std::{cell::Cell, mem::size_of, os::windows::ffi::OsStrExt, path::Path, sync::mpsc};
use windows::{
    core::{w, BOOL, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        System::{LibraryLoader::GetModuleHandleW, WindowsProgramming::*},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};
struct Background(HBRUSH);
impl Drop for Background {
    fn drop(&mut self) {
        // SAFETY: uniquely owns the successful CreateSolidBrush allocation;
        // all windows/classes release it before this guard is dropped.
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}
struct WindowBinding<'a> {
    window: HWND,
    _context: &'a WindowContext<'a>,
}
struct WindowContext<'a> {
    ui: &'a WebUi,
    worker: Option<&'a Worker>,
    ready: Cell<bool>,
    ui_ready: Cell<bool>,
    already_running_pending: Cell<bool>,
    update_in_progress: Cell<bool>,
    closing: Cell<bool>,
    shell: Option<&'a Shell>,
    events: mpsc::Sender<(u32, isize)>,
}
impl WindowContext<'_> {
    fn enqueue(&self, window: HWND, event: u32, payload: isize) {
        let _ = self.events.send((event, payload));
        // SAFETY: data stays owned by the channel; the native message only
        // wakes this HWND's thread. COM/modal pumps cannot lose the event.
        let _ = unsafe { PostMessageW(Some(window), WM_NULL, WPARAM(0), LPARAM(0)) };
    }
    fn force_exit(&self) {
        if let Some(shell) = self.shell {
            if let Err(error) = shell.save() {
                eprintln!("window placement save: {error}");
            }
            shell.remove();
        }
        if let Some(worker) = self.worker {
            worker.request_stop();
        }
        self.ready.set(false);
        self.closing.set(true);
        // SAFETY: !Send WebUi keeps this context on its owning GUI thread.
        // The closing flag also handles WM_QUIT consumed by a COM pump.
        unsafe {
            PostQuitMessage(0);
        }
    }
}
fn publish(editor: &Editor, worker: Option<&Worker>, generation: &mut u64, flags: &mut Controls) {
    if let Some(worker) = worker {
        if editor.generation != *generation {
            // The worker owns an independent snapshot, like C++ PublishSelectedPreset.
            worker.preset(
                editor.config.presets[editor.config.selected_preset].clone(),
                editor.generation,
            );
            *generation = editor.generation;
        }
        let current = controls(editor);
        if current != *flags {
            worker.controls(current);
            *flags = current;
        }
    }
}
fn hide_to_tray(
    context: &WindowContext<'_>,
    editor: &mut Editor,
    elevated: bool,
    status: &str,
) -> windows::core::Result<()> {
    if let Some(shell) = context.shell {
        if shell.hide(editor.config.language == "en") {
            editor.preview = false;
            if let Some(worker) = context.worker {
                worker.controls(controls(editor));
            }
            if context.ready.get() {
                context.ui.send_json(
                    &editor
                        .state(elevated, status)
                        .map_err(|_| windows::core::Error::from(E_FAIL))?,
                )?;
            }
            println!("tray: hidden");
        }
    }
    Ok(())
}
fn controls(editor: &Editor) -> Controls {
    Controls {
        requested: editor.requested,
        auto_mode: editor.config.auto_mode,
        preview: editor.preview,
        steam_takeover: editor.config.steam_takeover,
    }
}
fn settings_command(
    window: HWND,
    export: bool,
    settings: &Path,
    editor: &mut Editor,
) -> Option<(&'static str, &'static str, MESSAGEBOX_STYLE)> {
    let selected = match windows_app::choose_settings(window, export) {
        Ok(Some(selected)) => selected,
        Ok(None) => return None,
        Err(error) => {
            eprintln!("settings picker: {error}");
            return Some((
                "ファイル選択画面を開けませんでした。",
                "ファイル選択エラー",
                MB_ICONERROR,
            ));
        }
    };
    if export {
        let result = windows_config::save(settings, &editor.config).and_then(|()| {
            if windows_app::equal_paths(&selected, settings) {
                Ok(())
            } else {
                windows_config::copy(settings, &selected)
            }
        });
        return Some(if result.is_ok() {
            ("設定をエクスポートしました。", "完了", MB_ICONINFORMATION)
        } else {
            (
                "設定をエクスポートできませんでした。",
                "保存エラー",
                MB_ICONERROR,
            )
        });
    }
    let mut imported = match windows_config::load(&selected) {
        Ok(imported) => imported,
        Err(error) => {
            eprintln!("settings import validation: {error}");
            return Some((
                "このアプリからエクスポートしたJSONファイルを選択してください。",
                "設定形式が違います",
                MB_ICONWARNING,
            ));
        }
    };
    let missing = windows_config::remove_missing_targets(&mut imported);
    let mut backup = settings.as_os_str().to_os_string();
    backup.push(".before-import.json");
    if windows_config::save(settings, &editor.config)
        .and_then(|()| windows_config::copy(settings, Path::new(&backup)))
        .is_err()
    {
        return Some((
            "現在の設定をバックアップできませんでした。",
            "インポート中止",
            MB_ICONERROR,
        ));
    }
    if !editor.import(imported, |config| {
        windows_config::save(settings, config).is_ok()
    }) {
        return Some((
            "設定を保存できませんでした。",
            "インポートエラー",
            MB_ICONERROR,
        ));
    }
    Some((
        if missing {
            "設定を読み込みました。このPCにない対象アプリの指定を解除し、自動モードをオフにしました。対象アプリを選び直してください。"
        } else {
            "設定を読み込みました。"
        },
        "完了",
        MB_ICONINFORMATION,
    ))
}
impl Drop for WindowBinding<'_> {
    fn drop(&mut self) {
        // SAFETY: clear the borrowed pointer before its WebUi can drop.
        // The guard's reference keeps WebUi at its address on this UI thread.
        unsafe {
            SetWindowLongPtrW(self.window, GWLP_USERDATA, 0);
        }
    }
}
fn check_resize(window: HWND, ui: &WebUi) -> Result<(), Box<dyn std::error::Error>> {
    let check = || -> Result<(), Box<dyn std::error::Error>> {
        let mut client = RECT::default();
        // SAFETY: live HWND and initialized writable output on the UI thread.
        unsafe { GetClientRect(window, &mut client) }?;
        if ui.bounds()? != client {
            return Err("browser bounds differ from native client after WM_SIZE".into());
        }
        Ok(())
    };
    let mut original = RECT::default();
    // SAFETY: OS validates the borrowed HWND; operations synchronously
    // deliver WM_SIZE through the real host callback, including maximize.
    unsafe {
        let maximized = IsZoomed(window).as_bool();
        let _ = ShowWindow(window, SW_RESTORE);
        GetWindowRect(window, &mut original)?;
        SetWindowPos(
            window,
            None,
            0,
            0,
            original.right - original.left + 37,
            original.bottom - original.top + 29,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
        check()?;
        let _ = ShowWindow(window, SW_MAXIMIZE);
        check()?;
        let _ = ShowWindow(window, SW_RESTORE);
        check()?;
        SetWindowPos(
            window,
            None,
            original.left,
            original.top,
            original.right - original.left,
            original.bottom - original.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
        if maximized {
            let _ = ShowWindow(window, SW_MAXIMIZE);
        }
        check()?;
    }
    println!("resize/maximize/restore: browser bounds match native client");
    Ok(())
}
fn initial_layout(
    window: HWND,
    path: &Path,
    work: RECT,
) -> Result<SHOW_WINDOW_CMD, windows::core::Error> {
    let path: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
    let mut saved = [0u16; 128];
    // SAFETY: Win32 writes only into the declared UTF-16 buffer.
    unsafe {
        GetPrivateProfileStringW(
            w!("Window"),
            w!("Placement"),
            w!(""),
            Some(&mut saved),
            PCWSTR(path.as_ptr()),
        );
    };
    let saved = String::from_utf16_lossy(&saved);
    let parsed = webview::window_placement(&saved);
    let [left, top, right, bottom, maximized] = parsed.unwrap_or_default();
    let rectangle = RECT {
        left,
        top,
        right,
        bottom,
    };
    let width = i64::from(rectangle.right) - i64::from(rectangle.left);
    let height = i64::from(rectangle.bottom) - i64::from(rectangle.top);
    // SAFETY: live borrowed HWND and initialized native structs. Placement
    // APIs copy their inputs. The saved INI is only read, never updated.
    unsafe {
        if parsed.is_some() && (400..=10000).contains(&width) && (300..=10000).contains(&height) {
            let mut placement = WINDOWPLACEMENT {
                length: size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            GetWindowPlacement(window, &mut placement)?;
            placement.rcNormalPosition = rectangle;
            placement.showCmd = SW_HIDE.0 as u32;
            placement.flags = WINDOWPLACEMENT_FLAGS(0);
            SetWindowPlacement(window, &placement)?;
            let mut actual = RECT::default();
            GetWindowRect(window, &mut actual)?;
            let mut monitor = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(
                MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
            .as_bool()
            {
                let work = monitor.rcWork;
                let width = (actual.right - actual.left).min(work.right - work.left);
                let height = (actual.bottom - actual.top).min(work.bottom - work.top);
                if width != actual.right - actual.left || height != actual.bottom - actual.top {
                    SetWindowPos(
                        window,
                        None,
                        work.left + (work.right - work.left - width) / 2,
                        work.top + (work.bottom - work.top - height) / 2,
                        width,
                        height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )?;
                }
            }
            return Ok(if maximized != 0 {
                SW_MAXIMIZE
            } else {
                SW_SHOWNORMAL
            });
        }
        let dpi = GetDpiForWindow(window);
        let dpi = if dpi == 0 { 96 } else { dpi } as i32;
        let width = MulDiv(1440, dpi, 96).min((work.right - work.left) * 82 / 100);
        let height = MulDiv(900, dpi, 96).min((work.bottom - work.top) * 82 / 100);
        SetWindowPos(
            window,
            None,
            work.left + (work.right - work.left - width) / 2,
            work.top + (work.bottom - work.top - height) / 2,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(SW_SHOWNORMAL)
}
fn check_saved_placement(
    window: HWND,
    shell: &Shell,
    path: &Path,
    work: RECT,
) -> Result<(), Box<dyn std::error::Error>> {
    // SAFETY: this test owns the live window and TEMP INI. Native operations
    // deliver real WM_SIZE messages through its guarded window procedure.
    unsafe {
        let _ = ShowWindow(window, SW_RESTORE);
        let mut rect = RECT::default();
        GetWindowRect(window, &mut rect)?;
        SetWindowPos(
            window,
            None,
            rect.left + 11,
            rect.top + 13,
            rect.right - rect.left + 23,
            rect.bottom - rect.top + 21,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
        let mut before = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(window, &mut before)?;
        shell.save()?;
        let show = initial_layout(window, path, work)?;
        let _ = ShowWindow(window, show);
        let mut restored = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(window, &mut restored)?;
        if restored.rcNormalPosition != before.rcNormalPosition || IsZoomed(window).as_bool() {
            return Err("saved normal position/size did not restore".into());
        }
        let _ = ShowWindow(window, SW_MAXIMIZE);
        let _ = ShowWindow(window, SW_MINIMIZE);
        shell.save()?;
        let show = initial_layout(window, path, work)?;
        if show != SW_MAXIMIZE {
            return Err("minimizing a maximized window lost its saved flag".into());
        }
        let _ = ShowWindow(window, show);
        if !IsZoomed(window).as_bool() {
            return Err("saved maximized state did not restore".into());
        }
    }
    println!("window placement self-test passed: normal position/size and maximized-after-minimize restore");
    Ok(())
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: Win32 supplies valid pointers for the matching system messages.
    // HWND operations are validated by the OS; these callbacks run on the UI
    // thread and never retain message pointers after returning.
    unsafe {
        if let Some(context) =
            (GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>).as_ref()
        {
            if let Some(shell) = context.shell {
                if message == shell.taskbar_created {
                    shell.taskbar_recreated();
                    return LRESULT(0);
                }
            }
        }
        match message {
            windows_update::READY => {
                if let Some(context) = (GetWindowLongPtrW(window, GWLP_USERDATA)
                    as *const WindowContext<'_>)
                    .as_ref()
                    .filter(|c| c.update_in_progress.get())
                {
                    context.force_exit();
                    // Preserve C++'s synchronous controller cleanup before
                    // acknowledging updater readiness (15-second IPC timeout).
                    if let Some(worker) = context.worker {
                        worker.shutdown();
                    }
                    return LRESULT(1);
                }
                return LRESULT(0);
            }
            windows_update::FAILED => {
                if let Some(context) =
                    (GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>).as_ref()
                {
                    context.update_in_progress.set(false);
                    context.enqueue(window, windows_update::FAILED, wparam.0 as isize);
                }
                return LRESULT(0);
            }
            windows_shell::ALREADY_RUNNING => {
                if let Some(context) =
                    (GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>).as_ref()
                {
                    context.enqueue(window, windows_shell::ALREADY_RUNNING, 0);
                    return LRESULT(1);
                }
                return LRESULT(0);
            }
            WM_CREATE => {
                let dark = BOOL(1);
                let caption = COLORREF(36 | (33 << 8) | (45 << 16));
                let text = COLORREF(240 | (237 << 8) | (245 << 16));
                // DWM copies these correctly sized native values. Optional
                // attributes match C++ and do not prevent UI startup.
                let _ = DwmSetWindowAttribute(
                    window,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &dark as *const _ as _,
                    size_of::<BOOL>() as u32,
                );
                let _ = DwmSetWindowAttribute(
                    window,
                    DWMWA_CAPTION_COLOR,
                    &caption as *const _ as _,
                    size_of::<COLORREF>() as u32,
                );
                let _ = DwmSetWindowAttribute(
                    window,
                    DWMWA_TEXT_COLOR,
                    &text as *const _ as _,
                    size_of::<COLORREF>() as u32,
                );
                return LRESULT(0);
            }
            WM_CLOSE | windows_shell::FORCE_EXIT => {
                if let Some(context) =
                    (GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>).as_ref()
                {
                    if message == WM_CLOSE
                        && context.shell.is_some()
                        && context.ready.get()
                        && IsWindowVisible(window).as_bool()
                    {
                        context.enqueue(window, windows_shell::CLOSE_REQUEST, 0);
                        return LRESULT(0);
                    }
                    context.force_exit();
                    return LRESULT(0);
                }
                let _ = DestroyWindow(window);
                return LRESULT(0);
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            windows_shell::TRAY => {
                if let Some(context) =
                    (GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>).as_ref()
                {
                    context.enqueue(window, windows_shell::TRAY, lparam.0);
                }
                return LRESULT(0);
            }
            WM_TIMER => {
                let _ = PostMessageW(
                    Some(window),
                    windows_shell::FORCE_EXIT,
                    WPARAM(0),
                    LPARAM(0),
                );
                return LRESULT(0);
            }
            WM_QUERYENDSESSION => return LRESULT(1),
            WM_ENDSESSION if wparam.0 != 0 => {
                if let Some(context) =
                    (GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>).as_ref()
                {
                    context.force_exit();
                }
                return LRESULT(0);
            }
            WM_SIZE => {
                // WM_SIZE is commonly delivered by SendMessage during
                // sizing, including nested modal drag/maximize loops. Update
                // the shared WebUi boundary here, as the reference does.
                let pointer = GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowContext<'_>;
                if let Some(context) = pointer.as_ref() {
                    if wparam.0 != SIZE_MINIMIZED as usize {
                        if let Some(shell) = context.shell {
                            shell.maximized.set(wparam.0 == SIZE_MAXIMIZED as usize);
                        }
                    }
                    // WindowBinding keeps this shared borrow live and clears
                    // it before WebUi drops, including error/close paths.
                    let _ = context.ui.resize();
                    if context.ready.get() {
                        let _ = context.ui.send_json(if IsZoomed(window).as_bool() {
                            "{\"type\":\"window\",\"maximized\":true}"
                        } else {
                            "{\"type\":\"window\",\"maximized\":false}"
                        });
                    }
                }
                return LRESULT(0);
            }
            WM_ACTIVATE if wparam.0 & 0xffff == WA_INACTIVE as usize => {
                if let Some(context) = (GetWindowLongPtrW(window, GWLP_USERDATA)
                    as *const WindowContext<'_>)
                    .as_ref()
                    .filter(|context| context.ready.get())
                {
                    let _ = context.ui.send_json("{\"type\":\"input\",\"buttons\":[]}");
                    let _ = context
                        .ui
                        .send_json("{\"type\":\"stickInput\",\"side\":0,\"x\":0,\"y\":0}");
                    let _ = context
                        .ui
                        .send_json("{\"type\":\"stickInput\",\"side\":1,\"x\":0,\"y\":0}");
                }
                return LRESULT(0);
            }
            WM_GETMINMAXINFO => {
                let dpi = GetDpiForWindow(window);
                let dpi = if dpi == 0 { 96 } else { dpi } as i32;
                let limits = &mut *(lparam.0 as *mut MINMAXINFO);
                limits.ptMinTrackSize.x = (800 * dpi + 48) / 96;
                limits.ptMinTrackSize.y = (560 * dpi + 48) / 96;
                return LRESULT(0);
            }
            WM_DPICHANGED => {
                let rect = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(
                    window,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                return LRESULT(0);
            }
            WM_NCCALCSIZE if wparam.0 != 0 && !IsZoomed(window).as_bool() => {
                let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
                let top = params.rgrc[0].top;
                let result = DefWindowProcW(window, message, wparam, lparam);
                if result.0 == 0 {
                    params.rgrc[0].top = top;
                }
                return result;
            }
            WM_NCHITTEST => {
                let hit = DefWindowProcW(window, message, wparam, lparam);
                if IsZoomed(window).as_bool() {
                    return hit;
                }
                let x = lparam.0 as i16 as i32;
                let y = (lparam.0 >> 16) as i16 as i32;
                let mut bounds = RECT::default();
                if GetWindowRect(window, &mut bounds).is_ok() {
                    let dpi = GetDpiForWindow(window);
                    let grip = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi)
                        + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
                    if y < bounds.top + grip {
                        return LRESULT(if x < bounds.left + grip {
                            HTTOPLEFT
                        } else if x >= bounds.right - grip {
                            HTTOPRIGHT
                        } else {
                            HTTOP
                        } as isize);
                    }
                }
                return hit;
            }
            _ => {}
        }
        DefWindowProcW(window, message, wparam, lparam)
    }
}
pub fn run(diagnostic: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments: Vec<_> = std::env::args().skip(1).collect();
    if diagnostic && arguments.first().is_some_and(|s| s == "--diagnostic") {
        arguments.remove(0);
    }
    if let Some(code) = windows_steam::helper_command(&arguments) {
        std::process::exit(i32::from(code));
    }
    // Exercise the normal host/worker/resources against an isolated settings
    // directory. Ordinary startup still uses exactly C++'s native AppData path.
    let data_dir = if !diagnostic && arguments.len() == 2 && arguments[0] == "--data-dir" {
        let path = std::path::PathBuf::from(&arguments[1]);
        if !path.is_absolute() {
            return Err("--data-dir requires an absolute directory".into());
        }
        arguments.clear();
        Some(path)
    } else {
        None
    };
    let mut seconds = 60;
    let mut edit_test = false;
    let mut self_test = false;
    let mut controller_test = !diagnostic;
    let mut shell_test = !diagnostic;
    let mut instance_test = !diagnostic;
    let mut output_test = false;
    let mut startup_test = false;
    let mut update_test = false;
    let mut update_test_stage = 0;
    let mut seconds_set = false;
    if !diagnostic && !arguments.is_empty() {
        return Err("PadMux [--data-dir ABSOLUTE_DIRECTORY] or --diagnostic FLAGS".into());
    }
    let mut args = arguments.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seconds" if !seconds_set => {
                seconds = args
                    .next()
                    .ok_or("--seconds needs a value")?
                    .parse::<u32>()?;
                seconds_set = true;
            }
            "--edit-test" if !edit_test => edit_test = true,
            "--self-test" if !self_test => {
                self_test = true;
                edit_test = true;
            }
            "--controller-test" if !controller_test => {controller_test=true;edit_test=true;}
            "--shell-test" if !shell_test => {shell_test=true;edit_test=true;}
            "--single-instance-test" if !instance_test => {instance_test=true;shell_test=true;edit_test=true;}
            "--output-test" if !output_test => {output_test=true;controller_test=true;edit_test=true;}
            "--startup-test" if !startup_test => {startup_test=true;edit_test=true;}
            "--update-test" if !update_test => {update_test=true;edit_test=true;}
            _ => {
                return Err(
                    "usage: webview_probe [--seconds 1..3600] [--edit-test | --self-test | --controller-test | --shell-test | --single-instance-test | --output-test | --startup-test | --update-test]"
                        .into(),
                )
            }
        }
    }
    if !(1..=3600).contains(&seconds) {
        return Err("seconds must be 1..3600".into());
    }
    if update_test && (controller_test || self_test || startup_test) {
        return Err("update IPC diagnostic runs separately without controller acquisition".into());
    }
    if self_test && controller_test {
        return Err("automated DOM tests cannot run with controller output".into());
    }
    if startup_test && controller_test {
        return Err("startup migration diagnostic does not acquire the controller".into());
    }
    // Legacy production mutex/class prevent overlap with old Remapcon.
    let _single_instance = if instance_test {
        match windows_shell::single_instance(
            if diagnostic {
                "PadMux_Phase9_SingleInstance_Test"
            } else {
                "Remapcon_SingleInstance"
            },
            if diagnostic {
                "PadMuxRustWebviewProbe"
            } else {
                "RemapconMainWindow"
            },
        )? {
            Some(handle) => Some(handle),
            None => {
                println!("single instance: notified existing window; no second UI/worker created");
                return Ok(());
            }
        }
    } else {
        None
    };
    if controller_test && !windows_steam::recover_interrupted() {
        if !diagnostic {
            // SAFETY: static recovery warning from C++, before host creation.
            unsafe {
                MessageBoxW(None,w!("前回の切り替えでコントローラーデバイスが無効のまま残った可能性があります。Windowsのデバイスマネージャーでコントローラーを有効にしてください。"),w!("コントローラーの復旧が必要です"),MB_OK|MB_ICONERROR);
            }
        }
        return Err("pending device cycle recovery failed".into());
    }
    let startup_sandbox = if startup_test {
        let root = std::env::temp_dir().join(format!(
            "padmux-phase9-startup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir(&root)?;
        let owned = EditDirectory(root);
        let legacy = owned.0.join("SteamlessController");
        std::fs::create_dir(&legacy)?;
        let ini = "[MapleStory]\r\nConfigFormat=ControllerSettings-v2\r\nPresetCount=1\r\n[Preset0]\r\nName=Startup migration test\r\nTargetExecutable=notepad.exe\r\nLeftPadMode=2\r\nRightPadMode=1\r\nA=30\r\n";
        let bytes: Vec<_> = [0xfeff]
            .into_iter()
            .chain(ini.encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        std::fs::write(legacy.join("MapleStoryController.ini"), bytes)?;
        Some(owned)
    } else {
        None
    };
    let source = if let Some(sandbox) = &startup_sandbox {
        sandbox.0.join("PadMux/ControllerSettings.json")
    } else if let Some(directory) = data_dir {
        directory.join("ControllerSettings.json")
    } else if !diagnostic {
        windows_config::settings_file()?
    } else {
        std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA missing")?)
            .join("PadMux/ControllerSettings.json")
    };
    let mut config = if update_test {
        // IPC diagnostics need no installed settings or device access.
        crate::config::Configuration::initial()
    } else if startup_test || !diagnostic {
        let config = windows_config::load_startup(&source).inspect_err(|_error| {
            if !diagnostic {
                // SAFETY: static C++ error text and no owner before host creation.
                unsafe { MessageBoxW(None,w!("設定ファイルを読み込めませんでした。ファイルを修復するか、バックアップを復元してください。"),w!("設定エラー"),MB_OK|MB_ICONERROR); }
            }
        })?;
        if startup_test {
            if windows_config::load_startup(&source)? != config {
                return Err("migrated startup JSON differs after reopening".into());
            }
            println!("startup: TEMP legacy INI migrated; current JSON reload matches");
        }
        config
    } else {
        windows_config::load(&source)?
    };
    if output_test {
        use crate::mapping::{Button, Config, MOUSE_LEFT};
        // Diagnostic-only preset in the existing TEMP sandbox. The real
        // GUI/controller/mapping/output path is unchanged; only Notepad is
        // eligible for output, and Steam gets control back on focus loss.
        config.selected_preset = 0;
        config.auto_mode = true;
        config.steam_takeover = true;
        config.close_behavior = 2;
        config.folders.clear();
        config.presets.truncate(1);
        let preset = &mut config.presets[0];
        preset.name = "Rust GUI output test".into();
        preset.folder.clear();
        preset.target_executable = "notepad.exe".into();
        preset.left_pad_mode = 2;
        preset.right_pad_mode = 1;
        preset.left_pad_sensitivity = 100;
        preset.right_pad_sensitivity = 100;
        preset.layers.clear();
        let base = Config::default().base;
        preset.bindings.mapping = [0; crate::mapping::BUTTON_COUNT];
        preset.bindings.turbo = base.turbo;
        preset.bindings.sequence = base.sequence;
        for (button, key) in [
            (Button::A, 0x1e),
            (Button::B, 0x30),
            (Button::X, 0x2d),
            (Button::Y, 0x15),
            (Button::LB, MOUSE_LEFT),
        ] {
            preset.bindings.mapping[button as usize] = key;
        }
    }
    let mut editor = Editor::new(config);
    let sandbox = if edit_test {
        let directory = std::env::temp_dir().join(format!(
            "padmux-phase9-edit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir(&directory)?;
        let sandbox = EditDirectory(directory);
        windows_config::save(&sandbox.0.join("ControllerSettings.json"), &editor.config)?;
        Some(sandbox)
    } else {
        None
    };
    let edit_path = if !diagnostic {
        Some(source.clone())
    } else {
        sandbox
            .as_ref()
            .map(|sandbox| sandbox.0.join("ControllerSettings.json"))
    };
    let mut self_test_stage = 0;
    let mut status = String::from("停止中：Steam Inputを使用できます");
    let elevated = windows_steam::elevated();
    let mut published_generation = editor.generation;
    let mut published_controls = controls(&editor);
    let mut observed_buttons = 0u32;
    let mut observed_sticks = [false; 2];
    let profile = if diagnostic {
        std::env::temp_dir().join("padmux-phase9-webview-probe-profile")
    } else {
        source.with_file_name("WebView2")
    };
    let shell_directory = if self_test {
        sandbox
            .as_ref()
            .ok_or("self-test requires TEMP config")?
            .0
            .join("Shell")
    } else {
        std::env::temp_dir().join("padmux-phase9-shell-test")
    };
    let placement = if shell_test && diagnostic {
        std::fs::create_dir_all(&shell_directory)?;
        let placement = shell_directory.join("WindowPlacement.ini");
        if !placement.exists() && source.with_file_name("WindowPlacement.ini").exists() {
            windows_config::copy(&source.with_file_name("WindowPlacement.ini"), &placement)?;
        }
        placement
    } else {
        source.with_file_name("WindowPlacement.ini")
    };
    let icons = sandbox.as_ref().map_or_else(
        || {
            if diagnostic {
                profile.join("TargetIcons")
            } else {
                source.with_file_name("TargetIcons")
            }
        },
        |sandbox| sandbox.0.join("TargetIcons"),
    );
    // SAFETY: DPI setting takes a system-defined constant; instance/cursor
    // handles are borrowed system resources used only on this UI thread.
    let (instance, cursor, work) = unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let mut cursor_position = POINT::default();
        GetCursorPos(&mut cursor_position)?;
        let mut monitor = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(
            MonitorFromPoint(cursor_position, MONITOR_DEFAULTTOPRIMARY),
            &mut monitor,
        )
        .as_bool()
        {
            return Err(windows::core::Error::from_thread().into());
        }
        (
            HINSTANCE(GetModuleHandleW(None)?.0),
            LoadCursorW(None, IDC_ARROW)?,
            monitor.rcWork,
        )
    };
    // SAFETY: CreateSolidBrush takes a COLORREF value. Its unique GDI object
    // stays owned until the window is destroyed and class unregistered.
    let background = Background(unsafe { CreateSolidBrush(COLORREF(27 | (25 << 8) | (34 << 16))) });
    if background.0.is_invalid() {
        return Err(windows::core::Error::from_thread().into());
    }
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: cursor,
        hbrBackground: background.0,
        lpszClassName: if diagnostic {
            w!("PadMuxRustWebviewProbe")
        } else {
            w!("RemapconMainWindow")
        },
        // SAFETY: resource 101 is embedded by the existing app.rc. LoadIcon's
        // shared module icon is borrowed and is never passed to DestroyIcon.
        hIcon: if diagnostic {
            HICON::default()
        } else {
            unsafe { LoadIconW(Some(instance), PCWSTR(101usize as *const u16)) }?
        },
        ..Default::default()
    };
    // SAFETY: class/callback have the correct ABI and a static name. Window
    // creation copies strings and retains the class until unregister below.
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(windows::core::Error::from_thread().into());
    }
    let window = unsafe {
        CreateWindowExW(
            Default::default(),
            class.lpszClassName,
            w!("PadMux — Steam Controller 2026 compatible"),
            WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX,
            work.left + 20,
            work.top + 20,
            1000.min(work.right - work.left),
            700.min(work.bottom - work.top),
            None,
            None,
            Some(instance),
            None,
        )
    };
    let window = match window {
        Ok(window) => window,
        Err(error) => {
            // SAFETY: registration succeeded and no window was created.
            unsafe { UnregisterClassW(class.lpszClassName, Some(instance)) }?;
            return Err(error.into());
        }
    };
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        // SAFETY: allow only the C++ activation message for this live HWND,
        // including non-elevated launch of an existing elevated process.
        unsafe {
            ChangeWindowMessageFilterEx(window, windows_shell::ALREADY_RUNNING, MSGFLT_ALLOW, None)
        }?;
        let show = initial_layout(window, &placement, work)?;
        let (sender, receiver) = mpsc::channel();
        let ui = WebUi::open(
            window,
            &profile,
            &webview::embedded_html("1.7.0"),
            move |message| {
                let _ = sender.send(message);
            },
        )?;
        // SAFETY: live owned window; timer is bounded and all messages are
        // dispatched on its UI thread; controller access requires its flag.
        unsafe {
            if diagnostic && SetTimer(Some(window), 1, seconds * 1000, None) == 0 {
                return Err(windows::core::Error::from_thread().into());
            }
            let _ = ShowWindow(window, show);
            let _ = SetForegroundWindow(window);
            println!(
                "diagnostic window visible={}",
                IsWindowVisible(window).as_bool()
            );
        }
        let worker = if controller_test {
            Some(Worker::start(
                window.0 as usize,
                editor.config.presets[editor.config.selected_preset].clone(),
                editor.generation,
                published_controls,
            )?)
        } else {
            None
        };
        let shell = if shell_test {
            let icon = if diagnostic {
                let path = sandbox
                    .as_ref()
                    .ok_or("shell test requires temporary config")?
                    .0
                    .join("padmux.ico");
                std::fs::write(&path, include_bytes!("../../src/app/resources/padmux.ico"))?;
                Some(path)
            } else {
                None
            };
            Some(Shell::new(window, placement.clone(), icon.as_deref())?)
        } else {
            None
        };
        let (native_events, native_receiver) = mpsc::channel();
        let context = WindowContext {
            ui: &ui,
            worker: worker.as_ref(),
            ready: Cell::new(false),
            ui_ready: Cell::new(false),
            already_running_pending: Cell::new(false),
            update_in_progress: Cell::new(false),
            closing: Cell::new(false),
            shell: shell.as_ref(),
            events: native_events,
        };
        let binding = WindowBinding {
            window,
            _context: &context,
        };
        // SAFETY: stable references remain borrowed until binding clears
        // user data, before UI/worker release, including error return paths.
        unsafe {
            SetWindowLongPtrW(
                window,
                GWLP_USERDATA,
                &context as *const WindowContext<'_> as isize,
            );
        }
        ui.resize()?;
        let mut message = MSG::default();
        loop {
            if context.closing.get() {
                break;
            }
            for (event, payload) in native_receiver.try_iter() {
                if event == windows_update::FAILED {
                    if context.ready.get() {
                        ui.send_json(&serde_json::json!({"type":"updateInstallError","code":payload as usize}).to_string())?;
                    }
                    continue;
                }
                if let Some(shell) = context.shell {
                    match event {
                        windows_shell::ALREADY_RUNNING => {
                            context.already_running_pending.set(true);
                            if context.ui_ready.get() {
                                shell.restore();
                                ui.send_json(&editor.state(elevated, &status)?)?;
                                ui.send_json("{\"type\":\"alreadyRunning\"}")?;
                                context.already_running_pending.set(false);
                                println!(
                                    "single instance: existing window restored and notice sent"
                                );
                            }
                        }
                        windows_shell::CLOSE_REQUEST => match editor.config.close_behavior {
                            0 => {
                                ui.send_json("{\"type\":\"closePrompt\"}")?;
                                if self_test && self_test_stage == 5 {
                                    after_render(
                                        &ui,
                                        "!document.getElementById('close-dialog').hidden",
                                        "document.getElementById('close-to-tray').click();",
                                    )?;
                                }
                            }
                            1 => hide_to_tray(&context, &mut editor, elevated, &status)?,
                            _ => context.force_exit(),
                        },
                        windows_shell::TRAY => match payload as u32 {
                            WM_LBUTTONDBLCLK => {
                                shell.restore();
                                ui.send_json(&editor.state(elevated, &status)?)?;
                                println!("tray: restored");
                                if self_test && self_test_stage == 6 {
                                    // SAFETY: query-only access to this diagnostic HWND.
                                    if !unsafe { IsWindowVisible(window) }.as_bool() {
                                        return Err("tray restore did not show window".into());
                                    }
                                    check_saved_placement(window, shell, &placement, work)?;
                                    self_test_stage = 7;
                                    println!("shell self-test passed: close prompt, tray hide/restore, placement save/load");
                                    context.force_exit();
                                }
                            }
                            WM_RBUTTONUP | WM_CONTEXTMENU => {
                                match shell.menu(editor.config.language == "en")? {
                                    windows_shell::EXIT => context.force_exit(),
                                    windows_shell::SHOW => {
                                        ui.send_json(&editor.state(elevated, &status)?)?
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            if context.closing.get() {
                break;
            }
            // SAFETY: initialized writable MSG; default filter uses this
            // thread's message queue. -1 is checked separately from WM_QUIT.
            let fetched = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
            if fetched == -1 {
                return Err(windows::core::Error::from_thread().into());
            }
            if fetched == 0 {
                break;
            }
            // SAFETY: MSG was populated by GetMessageW and dispatched while
            // the window/class/callback and WebUi remain live.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if context.closing.get() {
                break;
            }
            if let Some(worker) = &worker {
                for event in worker.events.try_iter() {
                    let value: serde_json::Value = serde_json::from_str(&event)?;
                    if value["type"] == "status" {
                        status = value["text"].as_str().unwrap_or("").to_owned();
                        println!("controller status: {status}");
                    } else if value["type"] == "input" {
                        if let Some(buttons) = value["buttons"].as_array() {
                            for button in buttons.iter().filter_map(serde_json::Value::as_u64) {
                                if button < 32 && observed_buttons & (1 << button) == 0 {
                                    observed_buttons |= 1 << button;
                                    println!("preview observed button={button}");
                                }
                            }
                        }
                    } else if value["type"] == "stickInput" {
                        if let Some(side) = value["side"].as_u64().filter(|&side| side < 2) {
                            if !observed_sticks[side as usize]
                                && (value["x"] != 0 || value["y"] != 0)
                            {
                                observed_sticks[side as usize] = true;
                                println!("preview observed stick side={side}");
                            }
                        }
                    }
                    if value["type"] == "runtimeStopped" {
                        editor.requested = false;
                        continue;
                    }
                    if context.ready.get() {
                        ui.send_json(&event)?;
                    }
                }
            }
            for raw in receiver.try_iter() {
                let fields = webview::decode_message(&raw);
                let command = fields.first().map(String::as_str).unwrap_or("");
                match command {
                    "ready" => {
                        context.ready.set(true);
                        ui.send_json(&editor.state(elevated, &status)?)?;
                        // SAFETY: query-only access to the live native HWND.
                        ui.send_json(if unsafe { IsZoomed(window) }.as_bool() {
                            "{\"type\":\"window\",\"maximized\":true}"
                        } else {
                            "{\"type\":\"window\",\"maximized\":false}"
                        })?;
                        println!("ready: existing configuration state sent");
                    }
                    "uiReady" => {
                        context.ui_ready.set(true);
                        println!("uiReady: existing UI rendered");
                        if diagnostic { check_resize(window, &ui)?; }
                        if let Some(shell) = context.shell {
                            shell.add();
                        }
                        if context.already_running_pending.get() {
                            context.enqueue(window, windows_shell::ALREADY_RUNNING, 0);
                        }
                        if update_test {
                            ui.execute_script(r#"try { chrome.webview.addEventListener('message', event => {
                                if (event.data.type === 'updateInstallError') {
                                    chrome.webview.postMessage(event.data.code === 6 ? '__updateRefused' : event.data.code === 3 ? '__updateFailure' : '__updateUnexpected');
                                }
                            });chrome.webview.postMessage('__updateListenerReady');chrome.webview.postMessage('installUpdate\tv1.7.0'); } catch (_) { chrome.webview.postMessage('__updateUnexpected'); }"#)?;
                        }
                        if self_test {
                            let create = "document.getElementById('preset-new').click();document.getElementById('name-input').value='Rust UI bridge test';document.getElementById('folder-selector').value='';document.getElementById('name-save').click();";
                            if startup_test {
                                after_render(&ui, "!document.getElementById('target-icon').hidden && document.getElementById('target-icon').complete && document.getElementById('target-icon').naturalWidth>0", &format!("chrome.webview.postMessage('__startupIconRendered');{create}"))?;
                            } else { ui.execute_script(create)?; }
                        }
                    }
                    "__startupIconRendered" if self_test && startup_test => println!("startup icon self-test passed: native PNG decoded and displayed by unchanged UI"),
                    "windowClose" => {
                        // SAFETY: HWND is borrowed; PostMessage validates it.
                        unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) }?;
                    }
                    "quit" => context.force_exit(),
                    "closeToTray" => {
                        hide_to_tray(&context, &mut editor, elevated, &status)?;
                        if self_test && self_test_stage == 5 {
                            // SAFETY: query live window and simulate only this
                            // process's native notification callback (no OS input).
                            unsafe {
                                if IsWindowVisible(window).as_bool() {
                                    return Err("tray hide did not hide window".into());
                                }
                                self_test_stage = 6;
                                PostMessageW(
                                    Some(window),
                                    windows_shell::TRAY,
                                    WPARAM(0),
                                    LPARAM(WM_LBUTTONDBLCLK as isize),
                                )?;
                            }
                        }
                    }
                    "getTargetIcon" => {
                        if let Some(target) = fields.get(1).filter(|target| {
                            !target.is_empty()
                                && editor
                                    .config
                                    .presets
                                    .iter()
                                    .any(|preset| preset.target_executable == **target)
                        }) {
                            let data = windows_icons::icon_data(target, &icons, false);
                            ui.send_json(&serde_json::json!({"type":"targetIcon","path":target,"data":data}).to_string())?;
                        }
                    }
                    "windowMinimize" => {
                        // SAFETY: live borrowed HWND, system-defined show flag.
                        unsafe {
                            let _ = ShowWindow(window, SW_MINIMIZE);
                        }
                    }
                    "installUpdate" => {
                        let tag=fields.get(1).map_or("",String::as_str);
                        if diagnostic || context.update_in_progress.get() || !windows_update::start(window,tag) {
                            ui.send_json("{\"type\":\"updateInstallError\",\"code\":6}")?;
                        } else {context.update_in_progress.set(true);}
                    }
                    "__updateListenerReady" if update_test => println!("updater diagnostic listener ready"),
                    "__updateRefused" if update_test => {
                        println!("updater diagnostic launch refused");
                        if update_test_stage != 0 || context.update_in_progress.get() {
                            return Err("diagnostic update launch was not refused".into());
                        }
                        // SAFETY: integer-only synchronous IPC to our own live
                        // diagnostic HWND; no updater process or download runs.
                        let rejected = unsafe { SendMessageW(window, windows_update::READY, None, None) };
                        if rejected.0 != 0 || context.closing.get() {
                            return Err("unsolicited updater READY was accepted".into());
                        }
                        update_test_stage = 1;
                        context.update_in_progress.set(true);
                        // SAFETY: queues a failure code to this diagnostic's
                        // own HWND, exercising the unchanged production bridge.
                        unsafe { PostMessageW(Some(window), windows_update::FAILED, WPARAM(3), LPARAM(0)) }?;
                    }
                    "__updateFailure" if update_test => {
                        println!("updater diagnostic failure delivered");
                        if update_test_stage != 1 || context.update_in_progress.get() {
                            return Err("updater failure did not reach UI/reset state".into());
                        }
                        context.update_in_progress.set(true);
                        // SAFETY: synchronous integer IPC only to our own live
                        // HWND. This diagnostic has no HID/output worker.
                        let accepted = unsafe { SendMessageW(window, windows_update::READY, None, None) };
                        if accepted.0 != 1 || !context.closing.get() {
                            return Err("active updater READY did not acknowledge shutdown".into());
                        }
                        update_test_stage = 2;
                        println!("updater IPC self-test passed: launch refusal, unsolicited READY rejection, FAILED delivered to UI, active READY shutdown acknowledgment");
                    }
                    "__updateUnexpected" if update_test => return Err("unexpected updater failure code".into()),
                    "openReleases" => {
                        // SAFETY: live borrowed HWND and static owned release
                        // URL; Shell launches a browser, no untrusted URI input.
                        let result=unsafe {windows::Win32::UI::Shell::ShellExecuteW(Some(window),w!("open"),w!("https://github.com/ro-ba/padmux/releases/latest"),None,None,SW_SHOWNORMAL)};
                        if result.0 as isize<=32 {ui.send_json("{\"type\":\"updateOpenError\"}")?;}
                    }
                    "windowToggleMaximize" => {
                        // SAFETY: live HWND queries/operations; no raw buffers.
                        unsafe {
                            let flag = if IsZoomed(window).as_bool() {
                                SW_RESTORE
                            } else {
                                SW_MAXIMIZE
                            };
                            let _ = ShowWindow(window, flag);
                            ui.send_json(if IsZoomed(window).as_bool() {
                                "{\"type\":\"window\",\"maximized\":true}"
                            } else {
                                "{\"type\":\"window\",\"maximized\":false}"
                            })?;
                        }
                    }
                    "windowDrag" => {
                        // SAFETY: same borrowed window; native drag command
                        // matches the reference and carries no data pointers.
                        unsafe {
                            let _ =
                                windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                            PostMessageW(
                                Some(window),
                                WM_SYSCOMMAND,
                                WPARAM((SC_MOVE | HTCAPTION) as usize),
                                LPARAM(0),
                            )?;
                        }
                    }
                    _ => {
                        if self_test && command == "__selfTestFailed" {
                            return Err(
                                "UI self-test failed or timed out waiting for rendered state"
                                    .into(),
                            );
                        }
                        if self_test && command == "__selfTestDone" {
                            if self_test_stage != 4 {
                                return Err("unexpected UI self-test completion".into());
                            }
                            self_test_stage = 5;
                            println!("UI self-test passed: preset create, key capture/save, undo, redo, atomic persisted configuration");
                            if shell_test {
                                ui.execute_script("document.getElementById('close-behavior').value='0';document.getElementById('close-behavior').dispatchEvent(new Event('change'));document.getElementById('win-close').click();")?;
                                continue;
                            }
                            // SAFETY: closes only this diagnostic's live window.
                            unsafe {
                                PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0))
                            }?;
                            continue;
                        }
                        if let Some(path) = &edit_path {
                            let mut notice = None;
                            let responses = if command == "pickTarget" {
                                let result = windows_app::choose_target(window);
                                match result {
                                    Ok(selected) => {
                                        let refresh = selected.is_some();
                                        if !editor.pick_target(selected, |config| {
                                            windows_config::save(path, config).is_ok()
                                        }) {
                                            // SAFETY: owned live HWND and static terminated UTF-16 strings;
                                            // synchronous native alert matches the C++ save failure.
                                            unsafe {
                                                MessageBoxW(
                                                    Some(window),
                                                    w!("対象アプリを保存できませんでした。"),
                                                    w!("保存エラー"),
                                                    MB_OK | MB_ICONERROR,
                                                );
                                            }
                                        } else if refresh {
                                            let _ = windows_icons::icon_data(
                                                &editor.config.presets
                                                    [editor.config.selected_preset]
                                                    .target_executable,
                                                &icons,
                                                true,
                                            );
                                        }
                                    }
                                    Err(error) => {
                                        eprintln!("target picker: {error}");
                                        let (text, title, icon) =
                                            if error.starts_with("file dialog error") {
                                                (
                                                    w!("ファイル選択画面を開けませんでした。"),
                                                    w!("ファイル選択エラー"),
                                                    MB_ICONERROR,
                                                )
                                            } else {
                                                (
                                                    w!(".exe ファイルを選択してください。"),
                                                    w!("対象アプリを確認"),
                                                    MB_ICONWARNING,
                                                )
                                            };
                                        // SAFETY: live HWND; static text remains valid through this dialog.
                                        unsafe {
                                            MessageBoxW(
                                                Some(window),
                                                text,
                                                title,
                                                MB_OK | icon,
                                            );
                                        }
                                    }
                                }
                                Some(vec![editor.state(elevated, &status)?])
                            } else if matches!(command, "import" | "export") {
                                notice = settings_command(
                                    window,
                                    command == "export",
                                    path,
                                    &mut editor,
                                );
                                Some(vec![editor.state(elevated, &status)?])
                            } else {
                                editor.handle(
                                    &raw,
                                    |config| {
                                        let result = windows_config::save(path, config);
                                        println!(
                                            "temporary configuration save: success={}",
                                            result.is_ok()
                                        );
                                        result.is_ok()
                                    },
                                    windows_app::equal_text,
                                    elevated,
                                    &status,
                                )
                            };
                            if let Some(responses) = responses {
                                publish(
                                    &editor,
                                    worker.as_ref(),
                                    &mut published_generation,
                                    &mut published_controls,
                                );
                                if let Some((text, title, icon)) = notice {
                                    let text: Vec<_> = text.encode_utf16().chain([0]).collect();
                                    let title: Vec<_> =
                                        title.encode_utf16().chain([0]).collect();
                                    // SAFETY: live HWND and owned terminated strings remain
                                    // valid through the synchronous native dialog. Publish
                                    // controller settings before the alert, as C++ does.
                                    unsafe {
                                        MessageBoxW(
                                            Some(window),
                                            PCWSTR(text.as_ptr()),
                                            PCWSTR(title.as_ptr()),
                                            MB_OK | icon,
                                        );
                                    }
                                }
                                for response in responses {
                                    ui.send_json(&response)?;
                                }
                                if self_test {
                                    let preset =
                                        &editor.config.presets[editor.config.selected_preset];
                                    match (self_test_stage, command) {
                                        (0, "preset-new") => {
                                            if preset.name != "Rust UI bridge test" {
                                                return Err("UI preset creation failed".into());
                                            }
                                            self_test_stage = 1;
                                            after_render(&ui,"document.querySelector('.preset-row.active')?.textContent.includes('Rust UI bridge test')",
                                                "const row=document.querySelector('.input-row[data-button=\"0\"]');window.__diagEmptyKeycap=row.querySelector('.keycap').textContent;row.dispatchEvent(new MouseEvent('dblclick',{bubbles:true}));const role=document.getElementById('edit-role');role.value='key';role.dispatchEvent(new Event('change'));document.getElementById('capture-key').click();document.dispatchEvent(new KeyboardEvent('keydown',{key:'a',code:'KeyA',bubbles:true}));document.getElementById('editor-save').click();")?;
                                        }
                                        (1, "saveBinding") => {
                                            if preset.bindings.mapping[0] != 30 {
                                                return Err("UI key capture/save failed".into());
                                            }
                                            self_test_stage = 2;
                                            after_render(&ui,"document.querySelector('.input-row[data-button=\"0\"] .keycap')?.textContent==='A'",
                                                "document.getElementById('preset-new').focus();document.dispatchEvent(new KeyboardEvent('keydown',{key:'z',ctrlKey:true,bubbles:true}));")?;
                                        }
                                        (2, "undo") => {
                                            if preset.bindings.mapping[0] != 0 {
                                                return Err("UI undo failed".into());
                                            }
                                            self_test_stage = 3;
                                            after_render(&ui,"document.querySelector('.input-row[data-button=\"0\"] .keycap')?.textContent===window.__diagEmptyKeycap",
                                                "document.getElementById('preset-new').focus();document.dispatchEvent(new KeyboardEvent('keydown',{key:'y',ctrlKey:true,bubbles:true}));")?;
                                        }
                                        (3, "redo") => {
                                            if preset.bindings.mapping[0] != 30
                                                || windows_config::load(path)? != editor.config
                                            {
                                                return Err(
                                                    "UI redo/persisted configuration mismatch"
                                                        .into(),
                                                );
                                            }
                                            self_test_stage = 4;
                                            after_render(&ui,"document.querySelector('.input-row[data-button=\"0\"] .keycap')?.textContent==='A'",
                                                "chrome.webview.postMessage('__selfTestDone');")?;
                                        }
                                        _ => {}
                                    }
                                }
                            } else {
                                println!(
                                    "native dialog/icon/update command is not yet connected"
                                );
                            }
                        } else {
                            println!("display diagnostic received configuration command (not applied)");
                        }
                    }
                }
            }
        }
        drop(binding);
        if let Some(worker) = &worker {
            worker.shutdown();
        }
        drop(ui);
        drop(shell);
        if self_test && self_test_stage != if shell_test { 7 } else { 5 } {
            return Err("UI self-test ended before completing all steps".into());
        }
        if update_test && update_test_stage != 2 {
            return Err(
                "updater IPC self-test ended before completing all steps (see stage log)".into(),
            );
        }
        println!("PadMux host complete");
        Ok(())
    })();
    // SAFETY: clean up the registered class after the message loop/WebUi
    // release. On errors, destroy any remaining window on its owning thread.
    unsafe {
        if IsWindow(Some(window)).as_bool() {
            let _ = DestroyWindow(window);
        }
        let _ = UnregisterClassW(class.lpszClassName, Some(instance));
    }
    result
}
