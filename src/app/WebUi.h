#pragma once

#include <Windows.h>
#include <WebView2.h>
#include <wrl.h>
#include <functional>
#include <string>

class WebUi {
public:
    using MessageHandler = std::function<void(const std::wstring&)>;
    bool Open(HWND window, MessageHandler handler);
    void Resize();
    void SendJson(const std::wstring& json);
    void Close();

private:
    void OnControllerReady(ICoreWebView2Controller* controller);
    HWND window_ = nullptr;
    MessageHandler handler_;
    Microsoft::WRL::ComPtr<ICoreWebView2Environment> environment_;
    Microsoft::WRL::ComPtr<ICoreWebView2Controller> controller_;
    Microsoft::WRL::ComPtr<ICoreWebView2> webview_;
};
