//! STA WebView2 boundary translated from Remapcon WebUi.cpp (MIT).
//! Copyright (c) 2026 ro-ba; retain ../../LICENSE / THIRD_PARTY_NOTICES.md.
use std::os::windows::ffi::OsStrExt;
use std::{marker::PhantomData, path::Path, rc::Rc, sync::mpsc};
use webview2_com::{
    CoTaskMemPWSTR, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, ExecuteScriptCompletedHandler,
    Microsoft::Web::WebView2::Win32::*, WebMessageReceivedEventHandler,
};
use windows::{
    core::{Interface, PCWSTR, PWSTR},
    Win32::{
        Foundation::{E_POINTER, HWND, RECT},
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::WindowsAndMessaging::GetClientRect,
    },
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub(crate) struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    pub(crate) fn new() -> windows::core::Result<Self> {
        // SAFETY: request the reference's STA; success (including S_FALSE)
        // creates exactly one !Send/!Sync guard for balanced CoUninitialize.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
        Ok(Self(PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: created after one successful CoInitializeEx; !Send/!Sync keeps
        // its matching CoUninitialize on this thread, after COM fields release.
        unsafe { CoUninitialize() };
    }
}

pub struct WebUi {
    window: HWND,
    webview: Option<ICoreWebView2>,
    controller: Option<ICoreWebView2Controller>,
    environment: Option<ICoreWebView2Environment>,
    message_token: i64,
    _apartment: Apartment,
}
impl WebUi {
    pub fn open(
        window: HWND,
        profile: &Path,
        html: &str,
        handler: impl Fn(String) + 'static,
    ) -> Result<Self> {
        let apartment = Apartment::new()?;
        let mut version = PWSTR::null();
        // SAFETY: initialized owned output; WebView2 allocates with CoTaskMem.
        unsafe { GetAvailableCoreWebView2BrowserVersionString(PCWSTR::null(), &mut version) }?;
        let version = CoTaskMemPWSTR::from(version);
        if version.to_string().is_empty() {
            return Err("WebView2 Runtime is unavailable".into());
        }
        let profile: Vec<_> = profile.as_os_str().encode_wide().collect();
        if profile.contains(&0) {
            return Err("NUL in WebView2 profile path".into());
        }
        let profile: Vec<_> = profile.into_iter().chain([0]).collect();
        let (sender, receiver) = mpsc::channel();
        CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
            Box::new(move |callback| {
                // SAFETY: terminated profile is owned by this closure and live
                // for the call. SDK copies the path; it owns the COM callback.
                unsafe {
                    CreateCoreWebView2EnvironmentWithOptions(
                        PCWSTR::null(),
                        PCWSTR(profile.as_ptr()),
                        None,
                        &callback,
                    )
                }
                .map_err(Into::into)
            }),
            Box::new(move |status, environment| {
                let _ = sender.send(status.and_then(|()| {
                    environment.ok_or_else(|| windows::core::Error::from(E_POINTER))
                }));
                Ok(())
            }),
        )?;
        let environment = receiver.recv()??;
        let (sender, receiver) = mpsc::channel();
        let creating = environment.clone(); // COM AddRef retains async creation ownership.
        CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
            Box::new(move |callback| {
                // SAFETY: HWND is an OS-validated borrowed handle; the STA COM
                // environment stays live in this owned callback closure.
                unsafe { creating.CreateCoreWebView2Controller(window, &callback) }
                    .map_err(Into::into)
            }),
            Box::new(move |status, controller| {
                let _ = sender.send(status.and_then(|()| {
                    controller.ok_or_else(|| windows::core::Error::from(E_POINTER))
                }));
                Ok(())
            }),
        )?;
        let controller = receiver.recv()??;
        // SAFETY: these COM interfaces were created in the guarded STA, on this
        // thread. Every returned interface is owned and releases before the guard.
        let mut ui = Self {
            window,
            webview: None,
            controller: Some(controller),
            environment: Some(environment),
            message_token: 0,
            _apartment: apartment,
        };
        // SAFETY: guarded STA controller. Construct its Close guard before this
        // fallible query so initialization failures also release browser resources.
        ui.webview = Some(unsafe { ui.controller.as_ref().unwrap().CoreWebView2() }?);
        // SAFETY: all interfaces belong to this STA. Optional color/settings
        // behave like C++: unavailable interfaces do not prevent opening.
        unsafe {
            if let Ok(color) = ui
                .controller
                .as_ref()
                .unwrap()
                .cast::<ICoreWebView2Controller2>()
            {
                let _ = color.SetDefaultBackgroundColor(COREWEBVIEW2_COLOR {
                    A: 255,
                    R: 27,
                    G: 25,
                    B: 34,
                });
            }
            let webview = ui.webview.as_ref().unwrap();
            if let Ok(settings) = webview.Settings() {
                let _ = settings.SetAreDefaultContextMenusEnabled(false);
                let _ = settings.SetIsStatusBarEnabled(false);
                if !cfg!(debug_assertions) {
                    let _ = settings.SetAreDevToolsEnabled(false);
                }
            }
            webview.add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                    if let Some(args) = args {
                        let mut message = PWSTR::null();
                        // The event provides a live interface; successful strings
                        // transfer CoTaskMem ownership to the crate's RAII guard.
                        if args.TryGetWebMessageAsString(&mut message).is_ok() && !message.is_null()
                        {
                            handler(CoTaskMemPWSTR::from(message).to_string());
                        }
                    }
                    Ok(())
                })),
                &mut ui.message_token,
            )?;
            ui.controller.as_ref().unwrap().SetIsVisible(true)?;
        }
        ui.resize()?;
        let html: Vec<_> = html.encode_utf16().chain([0]).collect();
        // SAFETY: live terminated HTML; NavigateToString copies it synchronously.
        unsafe {
            ui.webview
                .as_ref()
                .unwrap()
                .NavigateToString(PCWSTR(html.as_ptr()))
        }?;
        Ok(ui)
    }
    pub fn resize(&self) -> windows::core::Result<()> {
        let Some(controller) = &self.controller else {
            return Ok(());
        };
        let mut bounds = RECT::default();
        // SAFETY: writable initialized RECT; OS validates HWND. Controller lives
        // on this object's !Send STA, and SetBounds copies the RECT value.
        unsafe {
            GetClientRect(self.window, &mut bounds)?;
            controller.SetBounds(bounds)
        }
    }
    pub fn bounds(&self) -> windows::core::Result<RECT> {
        let controller = self
            .controller
            .as_ref()
            .ok_or_else(|| windows::core::Error::from(E_POINTER))?;
        let mut bounds = RECT::default();
        // SAFETY: initialized writable RECT; controller belongs to this STA and
        // writes synchronously. Used to compare actual browser/native geometry.
        unsafe { controller.Bounds(&mut bounds) }?;
        Ok(bounds)
    }
    pub fn execute_script(&self, script: &str) -> Result<()> {
        let webview = self
            .webview
            .as_ref()
            .ok_or_else(|| windows::core::Error::from(E_POINTER))?
            .clone();
        // COM AddRef retains the interface while the SDK pumps this STA.
        let script: Vec<_> = script.encode_utf16().chain([0]).collect();
        ExecuteScriptCompletedHandler::wait_for_async_operation(
            Box::new(move |callback| {
                // SAFETY: same guarded STA, owned interface and terminated JS;
                // the SDK copies script synchronously and retains the callback.
                unsafe { webview.ExecuteScript(PCWSTR(script.as_ptr()), &callback) }
                    .map_err(Into::into)
            }),
            Box::new(|status, _| status),
        )?;
        Ok(())
    }
    pub fn send_json(&self, json: &str) -> windows::core::Result<()> {
        let Some(webview) = &self.webview else {
            return Ok(());
        };
        let json: Vec<_> = json.encode_utf16().chain([0]).collect();
        // SAFETY: live terminated JSON, copied during this guarded STA call.
        unsafe { webview.PostWebMessageAsJson(PCWSTR(json.as_ptr())) }
    }
    pub fn close(&mut self) {
        // SAFETY: interfaces belong to this STA. Remove the handler before
        // Close, then release webview/controller/environment before CoUninitialize.
        unsafe {
            if let Some(webview) = &self.webview {
                let _ = webview.remove_WebMessageReceived(self.message_token);
            }
            if let Some(controller) = &self.controller {
                let _ = controller.Close();
            }
        }
        self.webview = None;
        self.controller = None;
        self.environment = None;
    }
}
impl Drop for WebUi {
    fn drop(&mut self) {
        self.close();
    }
}
