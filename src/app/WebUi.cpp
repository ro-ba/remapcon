#include "WebUi.h"
#include "maple_ui_embedded.h"
#include <ShlObj.h>
#include <wrl.h>
#include <utility>

bool WebUi::Open(HWND window, MessageHandler handler) {
    wchar_t* version = nullptr;
    if (FAILED(GetAvailableCoreWebView2BrowserVersionString(nullptr, &version)) || !version)
        return false;
    CoTaskMemFree(version);
    window_ = window;
    handler_ = std::move(handler);
    wchar_t localApp[MAX_PATH]{};
    if (FAILED(SHGetFolderPathW(nullptr, CSIDL_LOCAL_APPDATA, nullptr,
                                SHGFP_TYPE_CURRENT, localApp))) return false;
    const std::wstring dataDir = std::wstring(localApp) +
        L"\\PadMux\\WebView2";
    using namespace Microsoft::WRL;
    const HRESULT result = CreateCoreWebView2EnvironmentWithOptions(
        nullptr, dataDir.c_str(), nullptr,
        Callback<ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler>(
            [this](HRESULT status, ICoreWebView2Environment* environment) -> HRESULT {
                if (!window_) return S_OK;
                if (FAILED(status) || !environment) {
                    MessageBoxW(window_, L"WebView2の起動に失敗しました。", L"画面を開けません", MB_ICONERROR);
                    PostMessageW(window_, WM_CLOSE, 0, 0);
                    return S_OK;
                }
                environment_ = environment;
                environment_->CreateCoreWebView2Controller(window_,
                    Callback<ICoreWebView2CreateCoreWebView2ControllerCompletedHandler>(
                        [this](HRESULT controllerStatus, ICoreWebView2Controller* controller) -> HRESULT {
                            if (!window_) return S_OK;
                            if (FAILED(controllerStatus) || !controller) {
                                MessageBoxW(window_, L"WebView2の画面を作成できませんでした。", L"画面を開けません", MB_ICONERROR);
                                PostMessageW(window_, WM_CLOSE, 0, 0);
                                return S_OK;
                            }
                            OnControllerReady(controller);
                            return S_OK;
                        }).Get());
                return S_OK;
            }).Get());
    return SUCCEEDED(result);
}

void WebUi::OnControllerReady(ICoreWebView2Controller* controller) {
    controller_ = controller;
    Microsoft::WRL::ComPtr<ICoreWebView2Controller2> colorController;
    if (SUCCEEDED(controller_.As(&colorController)) && colorController)
        colorController->put_DefaultBackgroundColor(COREWEBVIEW2_COLOR{255, 27, 25, 34});
    controller_->get_CoreWebView2(&webview_);
    if (!webview_) return;
    Microsoft::WRL::ComPtr<ICoreWebView2Settings> settings;
    webview_->get_Settings(&settings);
    if (settings) {
        settings->put_AreDefaultContextMenusEnabled(FALSE);
        settings->put_IsStatusBarEnabled(FALSE);
#ifndef _DEBUG
        settings->put_AreDevToolsEnabled(FALSE);
#endif
    }
    Resize();
    controller_->put_IsVisible(TRUE);
    EventRegistrationToken token{};
    webview_->add_WebMessageReceived(
        Microsoft::WRL::Callback<ICoreWebView2WebMessageReceivedEventHandler>(
            [this](ICoreWebView2*, ICoreWebView2WebMessageReceivedEventArgs* args) -> HRESULT {
                LPWSTR message = nullptr;
                if (SUCCEEDED(args->TryGetWebMessageAsString(&message)) && message) {
                    if (handler_) handler_(message);
                    CoTaskMemFree(message);
                }
                return S_OK;
            }).Get(), &token);
    std::wstring html;
    for (const auto* part : kMapleUiHtmlParts) html += part;
    webview_->NavigateToString(html.c_str());
}

void WebUi::Resize() {
    if (!controller_ || !window_) return;
    RECT bounds{};
    GetClientRect(window_, &bounds);
    controller_->put_Bounds(bounds);
}

void WebUi::SendJson(const std::wstring& json) {
    if (webview_) webview_->PostWebMessageAsJson(json.c_str());
}

void WebUi::Close() {
    if (controller_) controller_->Close();
    webview_.Reset();
    controller_.Reset();
    environment_.Reset();
    handler_ = nullptr;
    window_ = nullptr;
}
