#include <Windows.h>
#include <commdlg.h>
#include <ShlObj.h>
#include <shellapi.h>
#include <dwmapi.h>
#include <windowsx.h>
#include <array>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <cwchar>
#include <filesystem>
#include <iterator>
#include <map>
#include <mutex>
#include <string>
#include <thread>
#include <utility>
#include <vector>
#include <algorithm>
#include <limits>
#include "steam/SteamController.h"
#include "TargetIcon.h"
#include "SimpleJson.h"
#include "WebUi.h"
#include "resources/resource.h"
#include "UpdateProtocol.h"
#include "DeviceCycle.h"

namespace {

using Clock = std::chrono::steady_clock;
constexpr UINT WM_CONTROLLER_STATUS = WM_APP + 1;
constexpr UINT WM_CONTROLLER_INPUT = WM_APP + 2;
constexpr UINT WM_STICK_INPUT = WM_APP + 9;
constexpr UINT WM_TRAY_ICON = WM_APP + 3;
constexpr UINT WM_FORCE_EXIT = WM_APP + 4;
constexpr UINT WM_UI_READY = WM_APP + 5;
constexpr UINT WM_ALREADY_RUNNING = WM_APP + 6;
constexpr UINT_PTR ID_TRAY_SHOW = 401;
constexpr UINT_PTR ID_TRAY_EXIT = 402;
constexpr UINT ID_TRAY_ICON = 1;
constexpr UINT_PTR ID_STARTUP_TIMER = 1;
constexpr int ID_TOGGLE = 101;
constexpr int ID_CATEGORY = 102;
constexpr int ID_PRESET = 103;
constexpr int ID_PRESET_NEW = 104;
constexpr int ID_PRESET_COPY = 105;
constexpr int ID_PRESET_RENAME = 106;
constexpr int ID_PRESET_DELETE = 107;
constexpr int ID_LAYER_SELECT = 108;
constexpr int ID_ROLE = 209;
constexpr int ID_ROLE_LAYER = 210;
constexpr int ID_LAYER_RENAME = 211;
constexpr int ID_TURBO = 212;
constexpr int ID_TURBO_INTERVAL = 213;
constexpr int ID_TURBO_DELAY = 214;
constexpr int ID_SEQUENCE_EDIT = 215;
constexpr int ID_SEQUENCE_LIST = 216;
constexpr int ID_SEQUENCE_ADD = 217;
constexpr int ID_SEQUENCE_REMOVE = 218;
constexpr int ID_SEQUENCE_UP = 219;
constexpr int ID_SEQUENCE_DOWN = 220;
constexpr int ID_SEQUENCE_INTERVAL = 221;
constexpr int ID_SEQUENCE_REPEAT = 222;
constexpr int ID_SEQUENCE_OK = 223;
constexpr int ID_SEQUENCE_CANCEL = 224;
constexpr int ID_AUTO_MODE = 225;
constexpr int ID_TARGET_PICK = 226;
constexpr int ID_TARGET_RESET = 227;
constexpr int ID_TARGET_MENU = 228;
constexpr int ID_CONFIG_MENU = 229;
constexpr int ID_CONFIG_IMPORT = 230;
constexpr int ID_CONFIG_EXPORT = 231;
constexpr int ID_ROW_FIRST = 300;
constexpr int ID_CAPTURE = 201;
constexpr int ID_DISABLE = 202;
constexpr int ID_CONFIRM = 203;
constexpr int ID_CANCEL = 204;
constexpr int ID_NAME_EDIT = 205;
constexpr int ID_NAME_OK = 206;
constexpr int ID_NAME_CANCEL = 207;
constexpr int ID_INHERIT = 208;
constexpr uint32_t EXTENDED = 0x10000;
constexpr uint32_t MOUSE_LEFT = 0x20001;
constexpr uint32_t MOUSE_RIGHT = 0x20002;
constexpr uint32_t MOUSE_MIDDLE = 0x20003;
constexpr uint32_t INHERIT = 0xFFFFFFFFu;

enum Button : size_t {
    A, B, X, Y, LB, RB, LT, RT, L3, R3,
    DPadUp, DPadDown, DPadLeft, DPadRight,
    L4, L5, R4, R5,
    LeftPadClick, LeftPadTap, RightPadClick, RightPadTap,
    Menu, View,
    RightStickUp, RightStickDown, RightStickLeft, RightStickRight,
    LeftStickUp, LeftStickDown, LeftStickLeft, LeftStickRight,
    ButtonCount
};

struct ButtonDef {
    const wchar_t* name;
    const wchar_t* setting;
    int category;
};

constexpr std::array<ButtonDef, ButtonCount> BUTTONS{{
    {L"A", L"A", 0}, {L"B", L"B", 0}, {L"X", L"X", 0}, {L"Y", L"Y", 0},
    {L"LB", L"LB", 0}, {L"RB", L"RB", 0}, {L"LT", L"LT", 0},
    {L"RT", L"RT", 0}, {L"L3", L"L3", 0}, {L"R3", L"R3", 0},
    {L"↑", L"DPadUp", 1}, {L"↓", L"DPadDown", 1},
    {L"←", L"DPadLeft", 1}, {L"→", L"DPadRight", 1},
    {L"L4", L"L4", 2}, {L"L5", L"L5", 2},
    {L"R4", L"R4", 2}, {L"R5", L"R5", 2},
    {L"左クリック", L"LeftPadClick", 3}, {L"左タップ", L"LeftPadTap", 3},
    {L"右クリック", L"RightPadClick", 3}, {L"右タップ", L"RightPadTap", 3},
    {L"メニュー", L"Menu", 4}, {L"ビュー", L"View", 4},
    {L"右スティック ↑", L"RightStickUp", 5},
    {L"右スティック ↓", L"RightStickDown", 5},
    {L"右スティック ←", L"RightStickLeft", 5},
    {L"右スティック →", L"RightStickRight", 5},
    {L"左スティック ↑", L"LeftStickUp", 5},
    {L"左スティック ↓", L"LeftStickDown", 5},
    {L"左スティック ←", L"LeftStickLeft", 5},
    {L"左スティック →", L"LeftStickRight", 5},
}};

constexpr std::array<const wchar_t*, 6> CATEGORIES{{
    L"基本ボタン", L"十字キー", L"背面ボタン", L"トラックパッド",
    L"メニュー・ビュー", L"スティック"
}};
constexpr size_t VISIBLE_ROWS = 10;

HWND mainWindow = nullptr;
bool windowWasMaximized = false;
bool initialWindowShown = false;
bool trayIconAdded = false;
int startupShow = SW_SHOWNORMAL;
UINT taskbarCreatedMessage = 0;
HWND statusLabel = nullptr;
HWND toggleButton = nullptr;
HWND autoModeBox = nullptr;
HWND targetButton = nullptr;
HWND configButton = nullptr;
HWND categoryBox = nullptr;
HWND presetBox = nullptr;
HWND layerBox = nullptr;
HWND layerRenameButton = nullptr;
HWND roleBox = nullptr;
HWND roleLayerBox = nullptr;
HWND captureButton = nullptr;
HWND disableButton = nullptr;
HWND inheritButton = nullptr;
HWND turboCheckbox = nullptr;
HWND turboIntervalEdit = nullptr;
HWND turboDelayEdit = nullptr;
HWND turboIntervalLabel = nullptr;
HWND turboDelayLabel = nullptr;
HWND sequenceButton = nullptr;
HWND sequenceDialog = nullptr;
HWND sequenceList = nullptr;
HWND sequenceIntervalEdit = nullptr;
HWND sequenceRepeatBox = nullptr;
HWND sequenceStatusLabel = nullptr;
HWND dialogWindow = nullptr;
HWND capturedLabel = nullptr;
HWND nameDialog = nullptr;
HWND nameEdit = nullptr;
std::wstring suggestedName;
std::array<HWND, VISIBLE_ROWS> rowButtons{};
std::array<size_t, VISIBLE_ROWS> visibleButtons{};
std::array<std::atomic<uint32_t>, ButtonCount> bindings{};
struct TurboSettings {
    bool enabled = false;
    uint32_t intervalMs = 50;
    uint32_t delayMs = 0;
};
struct SequenceSettings {
    bool enabled = false;
    bool repeat = true;
    uint32_t intervalMs = 50;
    std::vector<uint32_t> keys;
};
struct Layer {
    std::wstring name;
    std::array<uint32_t, ButtonCount> mapping{};
    std::array<TurboSettings, ButtonCount> turbo{};
    std::array<SequenceSettings, ButtonCount> sequence{};
    size_t trigger = ButtonCount;

    Layer() { mapping.fill(INHERIT); }
};
std::vector<Layer> liveLayers;
std::array<TurboSettings, ButtonCount> liveTurbo{};
std::array<SequenceSettings, ButtonCount> liveSequence{};
std::mutex bindingMutex;
std::wstring liveTargetExecutable;
uint32_t liveLeftPadMode = 0;
uint32_t liveRightPadMode = 0;
uint32_t liveLeftPadSensitivity = 100;
uint32_t liveRightPadSensitivity = 100;
uint32_t liveLeftStickDeadzone = 12288, liveRightStickDeadzone = 12288;
uint32_t liveLeftStickOverlap = 16000, liveRightStickOverlap = 16000;
std::atomic<uint64_t> targetVersion{0};
uint64_t mappingGeneration = 0;
struct Preset {
    std::wstring name;
    std::wstring folder;
    std::wstring targetExecutable;
    uint32_t leftPadMode = 0;
    uint32_t rightPadMode = 0;
    uint32_t leftPadSensitivity = 100;
    uint32_t rightPadSensitivity = 100;
    uint32_t leftStickDeadzone = 12288, rightStickDeadzone = 12288;
    uint32_t leftStickOverlap = 16000, rightStickOverlap = 16000;
    std::array<uint32_t, ButtonCount> mapping{};
    std::array<TurboSettings, ButtonCount> turbo{};
    std::array<SequenceSettings, ButtonCount> sequence{};
    std::vector<Layer> layers;
};
std::vector<Preset> presets;
std::vector<std::wstring> folders;
std::wstring language = L"ja";
uint32_t closeBehavior = 0; // 0: ask, 1: tray, 2: exit
bool processElevated = false;
WebUi webUi;
bool webReady = false;
bool alreadyRunningNoticePending = false;
std::wstring latestStatus;
std::atomic<bool> previewMode{false};
size_t selectedPreset = 0;
int editedLayer = 0;
bool pendingRoleLayer = false;
bool pendingRoleSequence = false;
size_t pendingTargetLayer = 0;
enum class NameAction { New, Copy, Rename, LayerRename };
NameAction nameAction = NameAction::New;
std::thread controllerThread;
std::atomic<bool> running{true};
std::atomic<bool> requested{false};
std::atomic<bool> autoMode{false};
std::atomic<bool> steamTakeover{false};
size_t selectedButton = A;
uint32_t pendingBinding = 0;
TurboSettings pendingTurbo;
SequenceSettings pendingSequence;
SequenceSettings sequenceBeforeEdit;
bool capturing = false;
bool sequenceCapturing = false;
bool sequenceAccepted = false;
std::wstring settingsPath;
bool updateInProgress = false;

bool IsProcessElevated() {
    HANDLE token = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) return false;
    TOKEN_ELEVATION elevation{};
    DWORD size = 0;
    const BOOL success = GetTokenInformation(token, TokenElevation, &elevation,
                                             sizeof(elevation), &size);
    CloseHandle(token);
    return success && elevation.TokenIsElevated != 0;
}

std::wstring KeyName(uint32_t key) {
    if (key == INHERIT) return L"引き継ぐ";
    if (!key) return L"無効";
    if (key == MOUSE_LEFT) return L"左クリック";
    if (key == MOUSE_RIGHT) return L"右クリック";
    if (key == MOUSE_MIDDLE) return L"中クリック";
    wchar_t name[128]{};
    const LONG parameter = static_cast<LONG>(((key & 0xFFu) << 16) |
        ((key & EXTENDED) ? (1u << 24) : 0u));
    if (GetKeyNameTextW(parameter, name, 128)) return name;
    return L"不明なキー";
}

std::wstring TargetName(const std::wstring& path) {
    if (path.empty()) return L"未設定";
    const size_t slash = path.find_last_of(L"\\/");
    return slash == std::wstring::npos ? path : path.substr(slash + 1);
}

void UpdateTargetButton() {
    if (!targetButton) return;
    std::wstring name = TargetName(presets[selectedPreset].targetExecutable);
    if (name.size() > 18) name = name.substr(0, 17) + L"…";
    SetWindowTextW(targetButton, (L"対象: " + name + L" ▼").c_str());
}

void UpdateRows() {
    if (!categoryBox) return;
    const int category = static_cast<int>(SendMessageW(categoryBox, CB_GETCURSEL, 0, 0));
    size_t row = 0;
    for (size_t i = 0; i < ButtonCount; ++i) {
        if (BUTTONS[i].category != category) continue;
        size_t triggerLayer = presets[selectedPreset].layers.size();
        for (size_t layer = 0; layer < presets[selectedPreset].layers.size(); ++layer) {
            if (presets[selectedPreset].layers[layer].trigger == i) {
                triggerLayer = layer;
                break;
            }
        }
        if (editedLayer != 0 && triggerLayer < presets[selectedPreset].layers.size()) continue;
        visibleButtons[row] = i;
        const uint32_t key = editedLayer == 0 ? bindings[i].load() :
            presets[selectedPreset].layers[editedLayer - 1].mapping[i];
        const TurboSettings turbo = editedLayer == 0 ? presets[selectedPreset].turbo[i] :
            presets[selectedPreset].layers[editedLayer - 1].turbo[i];
        const SequenceSettings& sequence = editedLayer == 0 ?
            presets[selectedPreset].sequence[i] :
            presets[selectedPreset].layers[editedLayer - 1].sequence[i];
        const std::wstring label = std::wstring(BUTTONS[i].name) + L"      →      " +
            (triggerLayer < presets[selectedPreset].layers.size() ?
             (presets[selectedPreset].layers[triggerLayer].name.empty() ?
              L"レイヤー" + std::to_wstring(triggerLayer + 1) :
              presets[selectedPreset].layers[triggerLayer].name) + L"切替" :
             sequence.enabled ? L"シーケンス（" + std::to_wstring(sequence.keys.size()) + L"入力）" :
             KeyName(key) + (key && key != INHERIT && turbo.enabled ?
                 L"（連打 " + std::to_wstring(turbo.intervalMs) + L" ms）" : L""));
        SetWindowTextW(rowButtons[row], label.c_str());
        ShowWindow(rowButtons[row], SW_SHOW);
        ++row;
    }
    while (row < VISIBLE_ROWS) ShowWindow(rowButtons[row++], SW_HIDE);
}

void UpdateLayerControls() {
    if (!layerBox) return;
    if (editedLayer > static_cast<int>(presets[selectedPreset].layers.size())) editedLayer = 0;
    SendMessageW(layerBox, CB_RESETCONTENT, 0, 0);
    SendMessageW(layerBox, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"通常"));
    for (size_t layer = 0; layer < presets[selectedPreset].layers.size(); ++layer) {
        const size_t trigger = presets[selectedPreset].layers[layer].trigger;
        const std::wstring name = presets[selectedPreset].layers[layer].name.empty() ?
            L"レイヤー" + std::to_wstring(layer + 1) : presets[selectedPreset].layers[layer].name;
        const std::wstring label = name +
            (trigger < ButtonCount ? L"（" + std::wstring(BUTTONS[trigger].name) + L"を押している間）" : L"");
        SendMessageW(layerBox, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(label.c_str()));
    }
    SendMessageW(layerBox, CB_SETCURSEL, editedLayer, 0);
    if (layerRenameButton) EnableWindow(layerRenameButton, editedLayer != 0);
    UpdateRows();
}

bool IsTargetForeground() {
    const HWND foreground = GetForegroundWindow();
    static HWND lastForeground = nullptr;
    static bool lastResult = false;
    static Clock::time_point lastChecked{};
    static uint64_t lastVersion = 0;
    const auto now = Clock::now();
    const uint64_t version = targetVersion.load();
    if (foreground == lastForeground && version == lastVersion &&
        now - lastChecked < std::chrono::milliseconds(100))
        return lastResult;
    lastForeground = foreground;
    lastChecked = now;
    lastVersion = version;
    std::wstring selectedPath;
    {
        std::lock_guard<std::mutex> lock(bindingMutex);
        selectedPath = liveTargetExecutable;
    }
    if (selectedPath.empty()) return lastResult = false;
    DWORD processId = 0;
    GetWindowThreadProcessId(foreground, &processId);
    if (!processId) return lastResult = false;
    HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, processId);
    if (!process) return lastResult = false;
    wchar_t path[MAX_PATH * 2]{};
    DWORD length = static_cast<DWORD>(std::size(path));
    const bool found = QueryFullProcessImageNameW(process, 0, path, &length) != 0;
    CloseHandle(process);
    if (!found) return lastResult = false;
    const wchar_t* name = wcsrchr(path, L'\\');
    if (selectedPath.find_first_of(L"\\/") != std::wstring::npos)
        return lastResult = _wcsicmp(path, selectedPath.c_str()) == 0;
    return lastResult = _wcsicmp(name ? name + 1 : path, selectedPath.c_str()) == 0;
}

bool ShouldHoldController() {
    return requested.load() || (autoMode.load() && IsTargetForeground()) ||
        (previewMode.load() && GetForegroundWindow() == mainWindow);
}

struct DeviceCycleProcess {
    HANDLE process = nullptr;
    HANDLE downEvent = nullptr;
    void Close() {
        if (process) CloseHandle(process);
        if (downEvent) CloseHandle(downEvent);
        process = downEvent = nullptr;
    }
};

std::wstring DeviceCycleExe() {
    wchar_t executable[MAX_PATH * 2]{};
    if (!GetModuleFileNameW(nullptr, executable, static_cast<DWORD>(std::size(executable))))
        return {};
    return executable;
}

bool RecoverInterruptedDeviceCycle() {
    if (!processElevated) return true;
    const std::wstring helper = DeviceCycleExe();
    if (helper.empty() || GetFileAttributesW(helper.c_str()) == INVALID_FILE_ATTRIBUTES)
        return true;
    std::wstring command = L"\"" + helper + L"\" --recover-device-cycle";
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_HIDE;
    if (!CreateProcessW(helper.c_str(), command.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) return false;
    CloseHandle(process.hThread);
    const DWORD wait = WaitForSingleObject(process.hProcess, 12000);
    DWORD code = 1;
    if (wait == WAIT_OBJECT_0) GetExitCodeProcess(process.hProcess, &code);
    CloseHandle(process.hProcess);
    return wait == WAIT_OBJECT_0 && code == 0;
}

bool StartDeviceCycle(const std::wstring& path, DeviceCycleProcess& cycle) {
    if (!processElevated || path.empty() || path.find(L'"') != std::wstring::npos)
        return false;
    const std::wstring helper = DeviceCycleExe();
    if (helper.empty() || GetFileAttributesW(helper.c_str()) == INVALID_FILE_ATTRIBUTES)
        return false;
    SECURITY_ATTRIBUTES security{sizeof(security), nullptr, TRUE};
    cycle.downEvent = CreateEventW(&security, TRUE, FALSE, nullptr);
    if (!cycle.downEvent) return false;
    std::wstring command = L"\"" + helper + L"\" --device-cycle \"" + path +
        L"\" " + std::to_wstring(reinterpret_cast<ULONG_PTR>(cycle.downEvent));
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_HIDE;
    if (!CreateProcessW(helper.c_str(), command.data(), nullptr, nullptr, TRUE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
        cycle.Close();
        return false;
    }
    CloseHandle(process.hThread);
    cycle.process = process.hProcess;
    return true;
}

bool FinishDeviceCycle(DeviceCycleProcess& cycle, DWORD timeoutMs) {
    const DWORD wait = WaitForSingleObject(cycle.process, timeoutMs);
    DWORD code = 1;
    if (wait == WAIT_OBJECT_0) GetExitCodeProcess(cycle.process, &code);
    cycle.Close();
    return wait == WAIT_OBJECT_0 && code == 0;
}

bool TakeControllerFromSteam(SteamController& controller, const std::wstring& path) {
    DeviceCycleProcess cycle;
    if (!StartDeviceCycle(path, cycle)) return false;
    HANDLE waitHandles[] = {cycle.downEvent, cycle.process};
    const DWORD down = WaitForMultipleObjects(2, waitHandles, FALSE, 6000);
    if (down != WAIT_OBJECT_0) {
        FinishDeviceCycle(cycle, 6000);
        return false;
    }
    const auto deadline = Clock::now() + std::chrono::seconds(8);
    bool claimed = false;
    while (running && ShouldHoldController() && steamTakeover.load() &&
           Clock::now() < deadline) {
        if (controller.OpenExclusive(path)) {
            if (controller.WaitForStateReport(350)) { claimed = true; break; }
            controller.Close();
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    const bool cycled = FinishDeviceCycle(cycle, 6000);
    if (!claimed || !cycled || !ShouldHoldController()) {
        controller.Close();
        return false;
    }
    return true;
}

bool ReturnControllerToSteam(const std::wstring& path) {
    DeviceCycleProcess cycle;
    if (!StartDeviceCycle(path, cycle)) return false;
    return FinishDeviceCycle(cycle, 12000);
}

bool SendKey(uint32_t key, bool down) {
    INPUT input{};
    if (key >= MOUSE_LEFT && key <= MOUSE_MIDDLE) {
        input.type = INPUT_MOUSE;
        const DWORD flags[3][2] = {
            {MOUSEEVENTF_LEFTUP, MOUSEEVENTF_LEFTDOWN},
            {MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_RIGHTDOWN},
            {MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MIDDLEDOWN},
        };
        input.mi.dwFlags = flags[key - MOUSE_LEFT][down ? 1 : 0];
        return SendInput(1, &input, sizeof(input)) == 1;
    }
    input.type = INPUT_KEYBOARD;
    input.ki.wScan = static_cast<WORD>(key & 0xFFu);
    input.ki.dwFlags = KEYEVENTF_SCANCODE |
        ((key & EXTENDED) ? KEYEVENTF_EXTENDEDKEY : 0) |
        (down ? 0 : KEYEVENTF_KEYUP);
    return SendInput(1, &input, sizeof(input)) == 1;
}

void SetStatus(const wchar_t* value) {
    if (mainWindow) PostMessageW(mainWindow, WM_CONTROLLER_STATUS, 0,
                                 reinterpret_cast<LPARAM>(value));
}

int16_t ReadInt16(const uint8_t* bytes) {
    int16_t value = 0;
    std::memcpy(&value, bytes, sizeof(value));
    return value;
}

uint16_t ReadUInt16(const uint8_t* bytes) {
    uint16_t value = 0;
    std::memcpy(&value, bytes, sizeof(value));
    return value;
}

struct PadPressState {
    bool pressed = false;
    int above = 0;
    int below = 0;

    bool Update(uint16_t area, bool clickBit) {
        // Match the upstream press threshold; the firmware click bit can miss a press.
        const bool raw = area >= 1800 || clickBit;
        if (raw) { ++above; below = 0; }
        else { ++below; above = 0; }
        if (!pressed && above >= 2) pressed = true;
        else if (pressed && below >= 2) pressed = false;
        return pressed;
    }
};

struct TapState {
    bool touching = false;
    bool clicked = false;
    int16_t startX = 0;
    int16_t startY = 0;
    int64_t maxTravelSquared = 0;
    Clock::time_point started{};
    Clock::time_point pulseUntil{};

    void Update(bool contact, bool click, int16_t x, int16_t y, Clock::time_point now) {
        if (contact && !touching) {
            started = now;
            startX = x;
            startY = y;
            maxTravelSquared = 0;
            clicked = false;
        }
        if (contact) {
            clicked = clicked || click;
            const int64_t dx = static_cast<int64_t>(x) - startX;
            const int64_t dy = static_cast<int64_t>(y) - startY;
            const int64_t distance = dx * dx + dy * dy;
            if (distance > maxTravelSquared) maxTravelSquared = distance;
        } else if (touching && !clicked && now - started <= std::chrono::milliseconds(200) &&
                   maxTravelSquared <= 3000LL * 3000LL) {
            pulseUntil = now + std::chrono::milliseconds(50);
        }
        touching = contact;
    }

    bool Pulsing(Clock::time_point now) const { return now < pulseUntil; }
};

struct StickDirections {
    bool up = false, down = false, left = false, right = false;
    bool active = false;
    void Update(int16_t x, int16_t y, uint32_t deadzone, uint32_t overlap) {
        const int32_t ax = std::abs(static_cast<int32_t>(x));
        const int32_t ay = std::abs(static_cast<int32_t>(y));
        const int64_t radiusSquared = static_cast<int64_t>(ax) * ax + static_cast<int64_t>(ay) * ay;
        const int32_t release = (std::max)(0, static_cast<int32_t>(deadzone) - 1500);
        const int32_t threshold = active ? release : static_cast<int32_t>(deadzone);
        active = radiusSquared >= static_cast<int64_t>(threshold) * threshold && (ax || ay);
        if (!active) { up = down = left = right = false; return; }
        const bool wasDiagonal = (up || down) && (left || right);
        const bool diagonal = (std::min)(ax, ay) >= (std::max)(1500, (std::max)(ax, ay) / 10) &&
            std::abs(ax - ay) <= static_cast<int32_t>(overlap) + (wasDiagonal ? 700 : 0);
        const bool horizontal = ax >= ay || diagonal;
        const bool vertical = ay >= ax || diagonal;
        up = vertical && y > 0;
        down = vertical && y < 0;
        left = horizontal && x < 0;
        right = horizontal && x > 0;
    }
};

void UpdatePhysical(const uint8_t* report, size_t size,
                    std::array<bool, ButtonCount>& physical,
                    PadPressState& leftPress, PadPressState& rightPress,
                    TapState& leftTap, TapState& rightTap,
                    StickDirections& leftStick, StickDirections& rightStick,
                    uint32_t leftDeadzone, uint32_t rightDeadzone,
                    uint32_t leftOverlap, uint32_t rightOverlap,
                    Clock::time_point now) {
    if (size < 10) return;
    const uint8_t b0 = report[2], b1 = report[3], b2 = report[4], b3 = report[5];
    physical[A] = (b0 & SteamController::BTN_A) != 0;
    physical[B] = (b0 & SteamController::BTN_B) != 0;
    physical[X] = (b0 & SteamController::BTN_X) != 0;
    physical[Y] = (b0 & SteamController::BTN_Y) != 0;
    physical[LB] = (b2 & SteamController::BTN_LB) != 0;
    physical[RB] = (b1 & SteamController::BTN_RB) != 0;
    physical[LT] = ReadInt16(report + 6) > (physical[LT] ? 0x1800 : 0x2000);
    physical[RT] = ReadInt16(report + 8) > (physical[RT] ? 0x1800 : 0x2000);
    physical[L3] = (b1 & SteamController::BTN_LS) != 0;
    physical[R3] = (b0 & SteamController::BTN_RS) != 0;
    physical[DPadUp] = (b1 & SteamController::BTN_DPAD_UP) != 0;
    physical[DPadDown] = (b1 & SteamController::BTN_DPAD_DN) != 0;
    physical[DPadLeft] = (b1 & SteamController::BTN_DPAD_LT) != 0;
    physical[DPadRight] = (b1 & SteamController::BTN_DPAD_RT) != 0;
    if (size >= 18) {
        const int16_t x = ReadInt16(report + 10);
        const int16_t y = ReadInt16(report + 12);
        leftStick.Update(x, y, leftDeadzone, leftOverlap);
        physical[LeftStickUp] = leftStick.up;
        physical[LeftStickDown] = leftStick.down;
        physical[LeftStickLeft] = leftStick.left;
        physical[LeftStickRight] = leftStick.right;
        rightStick.Update(ReadInt16(report + 14), ReadInt16(report + 16), rightDeadzone, rightOverlap);
        physical[RightStickUp] = rightStick.up;
        physical[RightStickDown] = rightStick.down;
        physical[RightStickLeft] = rightStick.left;
        physical[RightStickRight] = rightStick.right;
    } else {
        leftStick = {}; rightStick = {};
        physical[LeftStickUp] = physical[LeftStickDown] = false;
        physical[LeftStickLeft] = physical[LeftStickRight] = false;
        physical[RightStickUp] = physical[RightStickDown] = false;
        physical[RightStickLeft] = physical[RightStickRight] = false;
    }
    physical[L4] = (b2 & SteamController::BTN_L4) != 0;
    physical[L5] = (b2 & SteamController::BTN_L5) != 0;
    physical[R4] = (b0 & SteamController::BTN_R4) != 0;
    physical[R5] = (b1 & SteamController::BTN_R5) != 0;
    if (size >= 30) {
        physical[LeftPadClick] = leftPress.Update(ReadUInt16(report + 22),
            (b3 & SteamController::BTN_TP_LT_CLICK) != 0);
        physical[RightPadClick] = rightPress.Update(ReadUInt16(report + 28),
            (b2 & SteamController::BTN_TP_RT_CLICK) != 0);
    } else {
        physical[LeftPadClick] = (b3 & SteamController::BTN_TP_LT_CLICK) != 0;
        physical[RightPadClick] = (b2 & SteamController::BTN_TP_RT_CLICK) != 0;
    }
    physical[Menu] = (b0 & SteamController::BTN_MENU) != 0;
    physical[View] = (b1 & SteamController::BTN_VIEW) != 0;
    if (size >= 30) {
        leftTap.Update((b3 & SteamController::BTN_TP_LT) != 0,
                       physical[LeftPadClick], ReadInt16(report + 18),
                       ReadInt16(report + 20), now);
        rightTap.Update((b2 & SteamController::BTN_TP_RT) != 0,
                        physical[RightPadClick], ReadInt16(report + 24),
                        ReadInt16(report + 26), now);
    }
    physical[LeftPadTap] = leftTap.Pulsing(now);
    physical[RightPadTap] = rightTap.Pulsing(now);
}

struct HeldKey {
    size_t references = 0;
    bool sent = false;
    bool failureReported = false;
    Clock::time_point nextRepeat{};
};

using HeldKeys = std::map<uint32_t, HeldKey>;
struct TurboState {
    bool armed = false;
    uint32_t key = 0;
    TurboSettings settings;
    Clock::time_point started{};
};
struct SequenceState {
    bool armed = false;
    size_t sourceLayer = 0;
    Clock::time_point started{};
};

void ReleaseAll(HeldKeys& held, std::array<uint32_t, ButtonCount>& active) {
    for (int attempt = 0; attempt < 3; ++attempt) {
        bool failed = false;
        for (const auto& [key, state] : held) {
            if (state.sent && !SendKey(key, false)) failed = true;
        }
        if (!failed) break;
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }
    held.clear();
    active.fill(0);
}

void ApplyMappings(const std::array<bool, ButtonCount>& physical, bool gameForeground,
                   HeldKeys& held, std::array<uint32_t, ButtonCount>& active,
                   std::array<TurboState, ButtonCount>& turboStates,
                   std::array<SequenceState, ButtonCount>& sequenceStates,
                   uint64_t& appliedGeneration,
                   std::chrono::milliseconds repeatDelay,
                   std::chrono::milliseconds repeatInterval) {
    std::array<uint32_t, ButtonCount> configured{};
    std::array<TurboSettings, ButtonCount> baseTurbo{};
    std::array<SequenceSettings, ButtonCount> baseSequence{};
    std::vector<Layer> layered;
    uint64_t generation = 0;
    {
        std::lock_guard<std::mutex> lock(bindingMutex);
        for (size_t i = 0; i < ButtonCount; ++i) configured[i] = bindings[i].load();
        baseTurbo = liveTurbo;
        baseSequence = liveSequence;
        layered = liveLayers;
        generation = mappingGeneration;
    }
    if (generation != appliedGeneration) {
        ReleaseAll(held, active);
        turboStates = {};
        sequenceStates = {};
        appliedGeneration = generation;
    }
    std::array<uint32_t, ButtonCount> wanted{};
    std::array<bool, ButtonCount> turboActive{};
    std::array<bool, ButtonCount> sequenceActive{};
    std::map<uint32_t, size_t> continuousRefs;
    const auto now = Clock::now();
    if (gameForeground) {
        std::vector<bool> layerActive(layered.size(), false);
        for (size_t layer = 0; layer < layered.size(); ++layer) {
            const size_t trigger = layered[layer].trigger;
            layerActive[layer] = trigger < ButtonCount && physical[trigger];
        }
        for (size_t i = 0; i < ButtonCount; ++i) {
            if (!physical[i]) continue;
            bool isTrigger = false;
            for (size_t layer = 0; layer < layered.size(); ++layer)
                if (layered[layer].trigger == i) isTrigger = true;
            if (isTrigger) continue;
            uint32_t key = configured[i];
            TurboSettings turbo = baseTurbo[i];
            SequenceSettings sequence = baseSequence[i];
            size_t sourceLayer = 0;
            for (size_t layer = 0; layer < layered.size(); ++layer) {
                if (layerActive[layer] &&
                    (layered[layer].mapping[i] != INHERIT || layered[layer].sequence[i].enabled)) {
                    key = layered[layer].mapping[i];
                    turbo = layered[layer].turbo[i];
                    sequence = layered[layer].sequence[i];
                    sourceLayer = layer + 1;
                }
            }
            if (key == INHERIT) key = 0;
            if (sequence.enabled && !sequence.keys.empty()) {
                sequenceActive[i] = true;
                auto& state = sequenceStates[i];
                if (!state.armed || state.sourceLayer != sourceLayer) {
                    state.armed = true;
                    state.sourceLayer = sourceLayer;
                    state.started = now;
                }
                const uint64_t elapsed = static_cast<uint64_t>(
                    std::chrono::duration_cast<std::chrono::milliseconds>(now - state.started).count());
                uint64_t step = elapsed / sequence.intervalMs;
                if (sequence.repeat) step %= sequence.keys.size();
                if (step < sequence.keys.size() && elapsed % sequence.intervalMs <
                    sequence.intervalMs / 2) wanted[i] = sequence.keys[static_cast<size_t>(step)];
            } else if (key && turbo.enabled) {
                turboActive[i] = true;
                auto& state = turboStates[i];
                if (!state.armed || state.key != key ||
                    state.settings.intervalMs != turbo.intervalMs ||
                    state.settings.delayMs != turbo.delayMs) {
                    state.armed = true;
                    state.key = key;
                    state.settings = turbo;
                    state.started = now;
                }
                const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                    now - state.started).count();
                if (elapsed >= static_cast<int64_t>(turbo.delayMs)) {
                    const uint32_t phase = static_cast<uint32_t>(elapsed - turbo.delayMs) %
                        turbo.intervalMs;
                    if (phase < turbo.intervalMs / 2) wanted[i] = key;
                }
            } else {
                turboStates[i] = {};
                wanted[i] = key;
                if (key) ++continuousRefs[key];
            }
        }
    }
    for (size_t i = 0; i < ButtonCount; ++i)
        if (!turboActive[i]) turboStates[i] = {};
    for (size_t i = 0; i < ButtonCount; ++i)
        if (!sequenceActive[i]) sequenceStates[i] = {};
    for (size_t i = 0; i < ButtonCount; ++i) {
        if (active[i] && active[i] != wanted[i]) {
            auto it = held.find(active[i]);
            if (it != held.end() && it->second.references) --it->second.references;
            active[i] = 0;
        }
    }
    for (size_t i = 0; i < ButtonCount; ++i) {
        if (!active[i] && wanted[i]) {
            ++held[wanted[i]].references;
            active[i] = wanted[i];
        }
    }
    // Release obsolete keys before pressing replacements when a layer changes.
    for (auto it = held.begin(); it != held.end();) {
        if (it->second.references) { ++it; continue; }
        if (!it->second.sent || SendKey(it->first, false)) it = held.erase(it);
        else ++it;
    }
    for (auto it = held.begin(); it != held.end();) {
        auto& state = it->second;
        if (!state.references) {
            if (!state.sent || SendKey(it->first, false)) {
                it = held.erase(it);
                continue;
            }
        } else if (!state.sent) {
            if (SendKey(it->first, true)) {
                state.sent = true;
                state.failureReported = false;
                state.nextRepeat = now + repeatDelay;
                SetStatus(L"ボタン入力を検出：設定した入力をWindowsに送信しました");
            } else if (!state.failureReported) {
                state.failureReported = true;
                SetStatus(L"入力送信に失敗：対象アプリの権限を確認してください");
            }
        } else if (it->first < MOUSE_LEFT && continuousRefs[it->first] &&
                   now >= state.nextRepeat) {
            if (SendKey(it->first, true)) {
                if (state.failureReported) {
                    state.failureReported = false;
                    SetStatus(L"ボタン入力を検出：設定した入力をWindowsに送信しました");
                }
            } else if (!state.failureReported) {
                state.failureReported = true;
                SetStatus(L"入力送信に失敗：対象アプリの権限を確認してください");
            }
            state.nextRepeat = now + repeatInterval;
        }
        ++it;
    }
}

struct PadMotionState {
    uint32_t previousMode = 0;
    bool touching = false;
    int16_t x = 0;
    int16_t y = 0;
    float remainderX = 0;
    float remainderY = 0;
    int hapticTravel = 0;

    bool Update(uint32_t mode, uint32_t sensitivity, bool contact,
                int16_t nextX, int16_t nextY) {
        if (mode != previousMode) {
            touching = false; remainderX = remainderY = 0; hapticTravel = 0;
            previousMode = mode;
        }
        if (!mode || !contact) {
            touching = false; remainderX = remainderY = 0; hapticTravel = 0;
            return false;
        }
        bool hapticTick = false;
        if (touching) {
            const int dx = static_cast<int>(nextX) - x;
            const int dy = static_cast<int>(y) - nextY;
            // A discontinuity can occur after a missed report; start a new gesture.
            if (std::abs(dx) < 12000 && std::abs(dy) < 12000) {
                hapticTravel += std::abs(dx) + std::abs(dy);
                if (hapticTravel >= 1800) {
                    hapticTick = true;
                    hapticTravel = 0;
                }
                if (mode == 1) {
                    const float scale = 0.01125f * static_cast<float>(sensitivity) / 100.0f;
                    const float px = dx * scale + remainderX;
                    const float py = dy * scale + remainderY;
                    const LONG moveX = static_cast<LONG>(px);
                    const LONG moveY = static_cast<LONG>(py);
                    remainderX = px - moveX;
                    remainderY = py - moveY;
                    if (moveX || moveY) {
                        INPUT input{};
                        input.type = INPUT_MOUSE;
                        input.mi.dwFlags = MOUSEEVENTF_MOVE;
                        input.mi.dx = moveX;
                        input.mi.dy = moveY;
                        SendInput(1, &input, sizeof(input));
                    }
                } else if (mode == 2) {
                    const float scale = 0.02f * static_cast<float>(sensitivity) / 100.0f;
                    const float sx = dx * scale + remainderX;
                    const float sy = -dy * scale + remainderY;
                    const LONG wheelX = static_cast<LONG>(sx);
                    const LONG wheelY = static_cast<LONG>(sy);
                    remainderX = sx - wheelX;
                    remainderY = sy - wheelY;
                    if (wheelY) {
                        INPUT input{};
                        input.type = INPUT_MOUSE;
                        input.mi.dwFlags = MOUSEEVENTF_WHEEL;
                        input.mi.mouseData = static_cast<DWORD>(wheelY);
                        SendInput(1, &input, sizeof(input));
                    }
                    if (wheelX) {
                        INPUT input{};
                        input.type = INPUT_MOUSE;
                        input.mi.dwFlags = MOUSEEVENTF_HWHEEL;
                        input.mi.mouseData = static_cast<DWORD>(wheelX);
                        SendInput(1, &input, sizeof(input));
                    }
                }
            } else hapticTravel = 0;
        }
        x = nextX; y = nextY; touching = true;
        return hapticTick;
    }
};

void ControllerLoop() {
    SteamController controller;
    std::array<bool, ButtonCount> physical{};
    std::array<uint32_t, ButtonCount> active{};
    std::array<TurboState, ButtonCount> turboStates{};
    std::array<SequenceState, ButtonCount> sequenceStates{};
    HeldKeys held;
    uint64_t appliedGeneration = 0;
    PadPressState leftPress, rightPress;
    TapState leftTap, rightTap;
    StickDirections leftStick, rightStick;
    PadMotionState leftMotion, rightMotion;
    DWORD delaySetting = 1, speedSetting = 20;
    SystemParametersInfoW(SPI_GETKEYBOARDDELAY, 0, &delaySetting, 0);
    SystemParametersInfoW(SPI_GETKEYBOARDSPEED, 0, &speedSetting, 0);
    if (delaySetting > 3) delaySetting = 3;
    if (speedSetting > 31) speedSetting = 31;
    const auto repeatDelay = std::chrono::milliseconds(250 * (delaySetting + 1));
    const auto repeatInterval = std::chrono::milliseconds(62000 / (155 + 55 * speedSetting));
    try {
        bool announcedStopped = false;
        bool lastIdleWasAuto = false;
        Clock::time_point retryTakeoverAfter{};
        while (running) {
            if (!ShouldHoldController()) {
                const bool idleAuto = autoMode.load();
                if (!announcedStopped || idleAuto != lastIdleWasAuto)
                    SetStatus(idleAuto ?
                        L"自動待機中：対象アプリを前面にすると有効になります" :
                        L"停止中：Steam Inputを使用できます");
                announcedStopped = true;
                lastIdleWasAuto = idleAuto;
                std::this_thread::sleep_for(std::chrono::milliseconds(50));
                continue;
            }
            announcedStopped = false;
            bool opened = false;
            bool tookFromSteam = false;
            std::wstring controllerPath;
            for (const auto& path : SteamController::EnumerateAll()) {
                if (!running || !ShouldHoldController()) break;
                if (!controller.Open(path)) continue;
                if (controller.WaitForStateReport(350)) {
                    opened = true;
                    controllerPath = path;
                    break;
                }
                controller.Close();
            }
            if (!opened) {
                if (ShouldHoldController())
                    SetStatus(L"コントローラー待機中：接続を確認してください");
                std::this_thread::sleep_for(std::chrono::seconds(1));
                continue;
            }
            if (!ShouldHoldController()) { controller.Close(); continue; }
            const auto claim = controller.ClaimGameModeAccess();
            if (claim != SteamController::AccessClaim::Exclusive &&
                steamTakeover.load() && autoMode.load() && processElevated &&
                IsTargetForeground() && Clock::now() >= retryTakeoverAfter) {
                controller.Close();
                SetStatus(L"Steamからコントローラーを切り替え中です");
                tookFromSteam = TakeControllerFromSteam(controller, controllerPath);
                if (!tookFromSteam)
                    retryTakeoverAfter = Clock::now() + std::chrono::seconds(20);
            }
            if (claim != SteamController::AccessClaim::Exclusive && !tookFromSteam) {
                SetStatus(L"取得できません：Steamなどが使用中です");
                controller.Close();
                std::this_thread::sleep_for(std::chrono::seconds(1));
                continue;
            }
            if (!controller.DisableLizardMode()) {
                controller.EnableLizardMode();
                controller.Close();
                if (tookFromSteam) ReturnControllerToSteam(controllerPath);
                SetStatus(L"Lizard Modeを無効化できませんでした");
                std::this_thread::sleep_for(std::chrono::seconds(1));
                continue;
            }
            SetStatus(previewMode.load() && GetForegroundWindow() == mainWindow ?
                L"入力確認中：ボタンを押してください" :
                (autoMode ? L"自動有効：対象アプリへ入力します" :
                            L"有効：ボタン入力待ちです"));
            physical.fill(false);
            turboStates = {};
            sequenceStates = {};
            leftPress = {};
            rightPress = {};
            leftTap = {};
            rightTap = {};
            int16_t previewStickX[2]{}, previewStickY[2]{};
            leftStick = {}; rightStick = {};
            leftMotion = {}; rightMotion = {};
            bool leftClickHaptic = false, rightClickHaptic = false;
            auto lastPreview = Clock::now();
            auto lastReport = Clock::now();
            auto lastKeepalive = lastReport;
            bool healthy = true;
            while (running && ShouldHoldController() && healthy) {
                const auto now = Clock::now();
                if (now - lastKeepalive >= std::chrono::seconds(2)) {
                    healthy = controller.SendKeepalive();
                    lastKeepalive = now;
                }
                const auto beforeRead = Clock::now();
                uint8_t report[64]{};
                uint32_t readTimeoutMs = 32;
                for (size_t i = 0; i < ButtonCount; ++i) {
                    if (turboStates[i].armed || sequenceStates[i].armed) {
                        readTimeoutMs = 8;
                        break;
                    }
                }
                const size_t size = controller.ReadReport(report, sizeof(report), readTimeoutMs);
                if (!size && Clock::now() - beforeRead < std::chrono::milliseconds(2)) {
                    std::this_thread::sleep_for(std::chrono::milliseconds(16));
                }
                if (size >= 10 && SteamController::IsStateReportId(report[0])) {
                    if (size >= 18) {
                        previewStickX[0] = ReadInt16(report + 10); previewStickY[0] = ReadInt16(report + 12);
                        previewStickX[1] = ReadInt16(report + 14); previewStickY[1] = ReadInt16(report + 16);
                    } else {
                        previewStickX[0] = previewStickX[1] = previewStickY[0] = previewStickY[1] = 0;
                    }
                    uint32_t leftMode, rightMode, leftSensitivity, rightSensitivity;
                    uint32_t leftDeadzone, rightDeadzone, leftOverlap, rightOverlap;
                    {
                        std::lock_guard<std::mutex> lock(bindingMutex);
                        leftMode = liveLeftPadMode; rightMode = liveRightPadMode;
                        leftSensitivity = liveLeftPadSensitivity; rightSensitivity = liveRightPadSensitivity;
                        leftDeadzone = liveLeftStickDeadzone; rightDeadzone = liveRightStickDeadzone;
                        leftOverlap = liveLeftStickOverlap; rightOverlap = liveRightStickOverlap;
                    }
                    UpdatePhysical(report, size, physical, leftPress, rightPress,
                                   leftTap, rightTap, leftStick, rightStick,
                                   leftDeadzone, rightDeadzone, leftOverlap, rightOverlap, Clock::now());
                    lastReport = Clock::now();
                    if (size >= 30) {
                        const bool canMove = IsTargetForeground() && !previewMode.load();
                        if (canMove && physical[LeftPadClick] && !leftClickHaptic)
                            controller.PulseTrackpadHaptic(true, true);
                        if (canMove && physical[RightPadClick] && !rightClickHaptic)
                            controller.PulseTrackpadHaptic(false, true);
                        leftClickHaptic = physical[LeftPadClick];
                        rightClickHaptic = physical[RightPadClick];
                        const bool leftTick = leftMotion.Update(canMove ? leftMode : 0, leftSensitivity,
                            (report[5] & SteamController::BTN_TP_LT) != 0,
                            ReadInt16(report + 18), ReadInt16(report + 20));
                        const bool rightTick = rightMotion.Update(canMove ? rightMode : 0, rightSensitivity,
                            (report[4] & SteamController::BTN_TP_RT) != 0,
                            ReadInt16(report + 24), ReadInt16(report + 26));
                        if (leftTick) controller.TickTrackpadMovement(true);
                        if (rightTick) controller.TickTrackpadMovement(false);
                    } else {
                        leftMotion = {}; rightMotion = {};
                        leftClickHaptic = rightClickHaptic = false;
                    }
                }
                const auto inputNow = Clock::now();
                if (inputNow - lastReport > std::chrono::milliseconds(500)) {
                    physical.fill(false);
                    leftPress = {};
                    rightPress = {};
                    leftTap = {};
                    rightTap = {};
                    leftStick = {}; rightStick = {};
                    previewStickX[0] = previewStickX[1] = previewStickY[0] = previewStickY[1] = 0;
                    leftMotion = {}; rightMotion = {};
                    leftClickHaptic = rightClickHaptic = false;
                } else {
                    physical[LeftPadTap] = leftTap.Pulsing(inputNow);
                    physical[RightPadTap] = rightTap.Pulsing(inputNow);
                }
                if (previewMode.load() && GetForegroundWindow() == mainWindow &&
                    inputNow - lastPreview >= std::chrono::milliseconds(40)) {
                    WPARAM mask = 0;
                    for (size_t index = 0; index < ButtonCount; ++index)
                        if (physical[index]) mask |= static_cast<WPARAM>(1) << index;
                    if (mainWindow) PostMessageW(mainWindow, WM_CONTROLLER_INPUT, mask, 0);
                    if (mainWindow) {
                        for (WPARAM side = 0; side < 2; ++side) {
                            const uint32_t packed = static_cast<uint16_t>(previewStickX[side]) |
                                (static_cast<uint32_t>(static_cast<uint16_t>(previewStickY[side])) << 16);
                            PostMessageW(mainWindow, WM_STICK_INPUT, side, static_cast<LPARAM>(packed));
                        }
                    }
                    lastPreview = inputNow;
                }
                ApplyMappings(physical, IsTargetForeground() && !previewMode.load(), held, active, turboStates,
                              sequenceStates,
                              appliedGeneration,
                              repeatDelay, repeatInterval);
                if (inputNow - lastReport > std::chrono::seconds(4)) healthy = false;
            }
            ReleaseAll(held, active);
            turboStates = {};
            sequenceStates = {};
            controller.EnableLizardMode();
            controller.Close();
            if (tookFromSteam) {
                SetStatus(L"Steamへコントローラーを返しています");
                if (!ReturnControllerToSteam(controllerPath))
                    SetStatus(L"Steamへの切り替えを確認できませんでした");
            }
            if (mainWindow) PostMessageW(mainWindow, WM_CONTROLLER_INPUT, 0, 0);
            if (mainWindow) { PostMessageW(mainWindow, WM_STICK_INPUT, 0, 0); PostMessageW(mainWindow, WM_STICK_INPUT, 1, 0); }
            if (ShouldHoldController() && running)
                SetStatus(L"通信が切れました：再接続します");
        }
    } catch (...) {
        ReleaseAll(held, active);
        if (controller.IsOpen()) controller.EmergencyLizardRestore();
        controller.Close();
        SetStatus(L"予期しないエラー：アプリを再起動してください");
        requested = false;
    }
}

std::wstring SettingsFile() {
    wchar_t directory[MAX_PATH]{};
    if (FAILED(SHGetFolderPathW(nullptr, CSIDL_LOCAL_APPDATA, nullptr,
                                SHGFP_TYPE_CURRENT, directory))) return L"";
    std::wstring folder = std::wstring(directory) + L"\\Remapcon";
    CreateDirectoryW(folder.c_str(), nullptr);
    return folder + L"\\ControllerSettings.json";
}

std::wstring WindowSettingsFile() {
    const size_t separator = settingsPath.find_last_of(L"\\/");
    return separator == std::wstring::npos ? L"" :
        settingsPath.substr(0, separator + 1) + L"WindowPlacement.ini";
}

void SaveWindowPlacement(HWND window) {
    const std::wstring path = WindowSettingsFile();
    WINDOWPLACEMENT placement{sizeof(placement)};
    if (path.empty() || !GetWindowPlacement(window, &placement)) return;
    const RECT& rect = placement.rcNormalPosition;
    const bool maximized = windowWasMaximized || placement.showCmd == SW_SHOWMAXIMIZED;
    const std::wstring value = std::to_wstring(rect.left) + L"," +
        std::to_wstring(rect.top) + L"," + std::to_wstring(rect.right) + L"," +
        std::to_wstring(rect.bottom) + L"," + (maximized ? L"1" : L"0");
    WritePrivateProfileStringW(L"Window", L"Placement", value.c_str(), path.c_str());
}

bool RestoreWindowPlacement(HWND window) {
    const std::wstring path = WindowSettingsFile();
    if (path.empty()) return false;
    wchar_t value[128]{};
    GetPrivateProfileStringW(L"Window", L"Placement", L"", value,
                             static_cast<DWORD>(std::size(value)), path.c_str());
    int left = 0, top = 0, right = 0, bottom = 0, maximized = 0;
    if (swscanf_s(value, L"%d,%d,%d,%d,%d", &left, &top, &right, &bottom,
                  &maximized) != 5 || right - left < 400 || bottom - top < 300 ||
        right - left > 10000 || bottom - top > 10000) return false;
    WINDOWPLACEMENT placement{sizeof(placement)};
    if (!GetWindowPlacement(window, &placement)) return false;
    placement.rcNormalPosition = {left, top, right, bottom};
    placement.showCmd = SW_HIDE;
    placement.flags = 0;
    if (!SetWindowPlacement(window, &placement)) return false;
    RECT actual{};
    GetWindowRect(window, &actual);
    MONITORINFO monitor{sizeof(monitor)};
    if (GetMonitorInfoW(MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST), &monitor)) {
        const RECT& work = monitor.rcWork;
        const int workWidth = work.right - work.left;
        const int workHeight = work.bottom - work.top;
        if (actual.right - actual.left > workWidth || actual.bottom - actual.top > workHeight) {
            const int width = (std::min)(static_cast<int>(actual.right - actual.left), workWidth);
            const int height = (std::min)(static_cast<int>(actual.bottom - actual.top), workHeight);
            SetWindowPos(window, nullptr, work.left + (workWidth - width) / 2,
                         work.top + (workHeight - height) / 2, width, height,
                         SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
    startupShow = maximized ? SW_MAXIMIZE : SW_SHOWNORMAL;
    return true;
}

void SizeNewWindow(HWND window, const RECT& work) {
    const int workWidth = work.right - work.left;
    const int workHeight = work.bottom - work.top;
    const UINT dpi = GetDpiForWindow(window);
    const int width = (std::min)(MulDiv(1440, dpi ? dpi : 96, 96),
                                 workWidth * 82 / 100);
    const int height = (std::min)(MulDiv(900, dpi ? dpi : 96, 96),
                                  workHeight * 82 / 100);
    SetWindowPos(window, nullptr, work.left + (workWidth - width) / 2,
                 work.top + (workHeight - height) / 2, width, height,
                 SWP_NOZORDER | SWP_NOACTIVATE);
}

uint32_t ReadBinding(const wchar_t* section, const wchar_t* setting) {
    const uint32_t value = GetPrivateProfileIntW(section, setting, 0, settingsPath.c_str());
    return value == 0 || ((value & 0xFFu) != 0 && (value & ~0x100FFu) == 0)
        ? value : 0;
}

uint32_t ReadLayerBinding(const wchar_t* section, const std::wstring& setting) {
    const uint32_t value = GetPrivateProfileIntW(section, setting.c_str(), INHERIT,
                                                 settingsPath.c_str());
    if (value == INHERIT || value == 0 ||
        ((value & 0xFFu) != 0 && (value & ~0x100FFu) == 0)) return value;
    return INHERIT;
}

TurboSettings ReadTurbo(const wchar_t* section, const std::wstring& prefix,
                        const wchar_t* button) {
    TurboSettings value;
    value.enabled = GetPrivateProfileIntW(section, (prefix + L"Turbo_" + button).c_str(),
                                          0, settingsPath.c_str()) == 1;
    value.intervalMs = GetPrivateProfileIntW(section,
        (prefix + L"TurboInterval_" + button).c_str(), 50, settingsPath.c_str());
    value.delayMs = GetPrivateProfileIntW(section,
        (prefix + L"TurboDelay_" + button).c_str(), 0, settingsPath.c_str());
    if (value.intervalMs < 20 || value.intervalMs > 2000) value.intervalMs = 50;
    if (value.delayMs > 5000) value.delayMs = 0;
    return value;
}

SequenceSettings ReadSequence(const wchar_t* section, const std::wstring& prefix,
                              const wchar_t* button) {
    SequenceSettings value;
    value.enabled = GetPrivateProfileIntW(section,
        (prefix + L"SequenceEnabled_" + button).c_str(), 0, settingsPath.c_str()) == 1;
    value.intervalMs = GetPrivateProfileIntW(section,
        (prefix + L"SequenceInterval_" + button).c_str(), 50, settingsPath.c_str());
    value.repeat = GetPrivateProfileIntW(section,
        (prefix + L"SequenceRepeat_" + button).c_str(), 1, settingsPath.c_str()) != 0;
    if (value.intervalMs < 20 || value.intervalMs > 2000) value.intervalMs = 50;
    wchar_t buffer[256]{};
    GetPrivateProfileStringW(section, (prefix + L"Sequence_" + button).c_str(), L"",
        buffer, static_cast<DWORD>(std::size(buffer)), settingsPath.c_str());
    const std::wstring list = buffer;
    size_t start = 0;
    while (start < list.size() && value.keys.size() < 16) {
        const size_t comma = list.find(L',', start);
        const std::wstring item = list.substr(start, comma == std::wstring::npos ?
            std::wstring::npos : comma - start);
        wchar_t* end = nullptr;
        const unsigned long key = std::wcstoul(item.c_str(), &end, 10);
        if (item.empty() || !end || *end || (key & 0xFFu) == 0 ||
            (key & ~0x100FFu) != 0) {
            value.keys.clear();
            break;
        }
        value.keys.push_back(static_cast<uint32_t>(key));
        if (comma == std::wstring::npos) break;
        start = comma + 1;
    }
    if (value.keys.empty()) value.enabled = false;
    return value;
}

void PublishSelectedPreset() {
    std::lock_guard<std::mutex> lock(bindingMutex);
    for (size_t i = 0; i < ButtonCount; ++i)
        bindings[i] = presets[selectedPreset].mapping[i];
    liveTurbo = presets[selectedPreset].turbo;
    liveSequence = presets[selectedPreset].sequence;
    liveLayers = presets[selectedPreset].layers;
    liveTargetExecutable = presets[selectedPreset].targetExecutable;
    liveLeftPadMode = presets[selectedPreset].leftPadMode;
    liveRightPadMode = presets[selectedPreset].rightPadMode;
    liveLeftPadSensitivity = presets[selectedPreset].leftPadSensitivity;
    liveRightPadSensitivity = presets[selectedPreset].rightPadSensitivity;
    liveLeftStickDeadzone = presets[selectedPreset].leftStickDeadzone;
    liveRightStickDeadzone = presets[selectedPreset].rightStickDeadzone;
    liveLeftStickOverlap = presets[selectedPreset].leftStickOverlap;
    liveRightStickOverlap = presets[selectedPreset].rightStickOverlap;
    ++targetVersion;
    ++mappingGeneration;
}

void LoadLegacyPresets() {
    bool legacySettings = false;
    std::wstring legacyTarget;
    if (!settingsPath.empty()) {
        wchar_t savedLanguage[8]{};
        GetPrivateProfileStringW(L"MapleStory", L"Language", L"ja", savedLanguage,
            static_cast<DWORD>(std::size(savedLanguage)), settingsPath.c_str());
        language = wcscmp(savedLanguage, L"en") == 0 ? L"en" : L"ja";
        closeBehavior = GetPrivateProfileIntW(L"MapleStory", L"CloseBehavior", 0,
                                              settingsPath.c_str());
        if (closeBehavior > 2) closeBehavior = 0;
        folders.clear();
        const UINT folderCount = GetPrivateProfileIntW(L"MapleStory", L"FolderCount", 0,
                                                       settingsPath.c_str());
        for (UINT index = 0; index < folderCount && index < 100; ++index) {
            wchar_t value[512]{};
            GetPrivateProfileStringW(L"MapleStory",
                (L"Folder" + std::to_wstring(index)).c_str(), L"", value,
                static_cast<DWORD>(std::size(value)), settingsPath.c_str());
            if (value[0]) folders.emplace_back(value);
        }
        autoMode = GetPrivateProfileIntW(L"MapleStory", L"AutoMode", 0,
                                         settingsPath.c_str()) == 1;
        wchar_t format[64]{};
        GetPrivateProfileStringW(L"MapleStory", L"ConfigFormat", L"", format,
            static_cast<DWORD>(std::size(format)), settingsPath.c_str());
        legacySettings = wcscmp(format, L"ControllerSettings-v2") != 0 &&
            GetFileAttributesW(settingsPath.c_str()) != INVALID_FILE_ATTRIBUTES;
        wchar_t path[MAX_PATH * 2]{};
        GetPrivateProfileStringW(L"MapleStory", L"TargetExecutable", L"", path,
            static_cast<DWORD>(std::size(path)), settingsPath.c_str());
        legacyTarget = path[0] ? path : L"MapleStory.exe";
    }
    const uint32_t count = settingsPath.empty() ? 0 :
        GetPrivateProfileIntW(L"MapleStory", L"PresetCount", 0, settingsPath.c_str());
    if (count == 0 || count > 100) {
        Preset initial;
        initial.name = L"標準";
        if (legacySettings) initial.targetExecutable = legacyTarget;
        if (!settingsPath.empty()) {
            for (size_t i = 0; i < ButtonCount; ++i) {
                initial.mapping[i] = ReadBinding(L"MapleStory", BUTTONS[i].setting);
                initial.turbo[i] = ReadTurbo(L"MapleStory", L"", BUTTONS[i].setting);
                initial.sequence[i] = ReadSequence(L"MapleStory", L"", BUTTONS[i].setting);
            }
        }
        presets.push_back(std::move(initial));
    } else {
        for (uint32_t index = 0; index < count; ++index) {
            Preset preset;
            const std::wstring section = L"Preset" + std::to_wstring(index);
            wchar_t name[128]{};
            GetPrivateProfileStringW(section.c_str(), L"Name", L"プリセット", name,
                                     static_cast<DWORD>(std::size(name)), settingsPath.c_str());
            preset.name = name;
            wchar_t folder[512]{};
            GetPrivateProfileStringW(section.c_str(), L"Folder", L"", folder,
                static_cast<DWORD>(std::size(folder)), settingsPath.c_str());
            preset.folder = folder;
            const UINT oldLeft = GetPrivateProfileIntW(section.c_str(), L"LeftPadMouse", 0,
                                                       settingsPath.c_str());
            const UINT oldRight = GetPrivateProfileIntW(section.c_str(), L"RightPadMouse", 0,
                                                        settingsPath.c_str());
            preset.leftPadMode = GetPrivateProfileIntW(section.c_str(), L"LeftPadMode", oldLeft,
                                                       settingsPath.c_str());
            preset.rightPadMode = GetPrivateProfileIntW(section.c_str(), L"RightPadMode", oldRight,
                                                        settingsPath.c_str());
            if (preset.leftPadMode > 2) preset.leftPadMode = 0;
            if (preset.rightPadMode > 2) preset.rightPadMode = 0;
            preset.leftPadSensitivity = GetPrivateProfileIntW(section.c_str(),
                L"LeftPadSensitivity", 100, settingsPath.c_str());
            preset.rightPadSensitivity = GetPrivateProfileIntW(section.c_str(),
                L"RightPadSensitivity", 100, settingsPath.c_str());
            if (preset.leftPadSensitivity < 25 || preset.leftPadSensitivity > 400)
                preset.leftPadSensitivity = 100;
            if (preset.rightPadSensitivity < 25 || preset.rightPadSensitivity > 400)
                preset.rightPadSensitivity = 100;
            if (legacySettings) preset.targetExecutable = legacyTarget;
            else {
                wchar_t path[MAX_PATH * 2]{};
                GetPrivateProfileStringW(section.c_str(), L"TargetExecutable", L"", path,
                    static_cast<DWORD>(std::size(path)), settingsPath.c_str());
                preset.targetExecutable = path;
            }
            for (size_t i = 0; i < ButtonCount; ++i) {
                preset.mapping[i] = ReadBinding(section.c_str(), BUTTONS[i].setting);
                preset.turbo[i] = ReadTurbo(section.c_str(), L"", BUTTONS[i].setting);
                preset.sequence[i] = ReadSequence(section.c_str(), L"", BUTTONS[i].setting);
            }
            wchar_t countText[32]{};
            const DWORD countLength = GetPrivateProfileStringW(section.c_str(),
                L"LayerCount", L"", countText, static_cast<DWORD>(std::size(countText)),
                settingsPath.c_str());
            wchar_t oldTrigger[32]{};
            const DWORD oldLength = GetPrivateProfileStringW(section.c_str(),
                L"Layer1Trigger", L"", oldTrigger, static_cast<DWORD>(std::size(oldTrigger)),
                settingsPath.c_str());
            const size_t savedCount = countLength ?
                GetPrivateProfileIntW(section.c_str(), L"LayerCount", 0,
                                      settingsPath.c_str()) : (oldLength ? 2 : 0);
            const size_t layerCount = savedCount <= ButtonCount ? savedCount : ButtonCount;
            for (size_t layer = 0; layer < layerCount; ++layer) {
                Layer entry;
                const std::wstring prefix = L"Layer" + std::to_wstring(layer + 1);
                wchar_t layerName[64]{};
                GetPrivateProfileStringW(section.c_str(), (prefix + L"Name").c_str(), L"",
                    layerName, static_cast<DWORD>(std::size(layerName)), settingsPath.c_str());
                entry.name = layerName;
                const uint32_t trigger = GetPrivateProfileIntW(section.c_str(),
                    (prefix + L"Trigger").c_str(),
                    static_cast<UINT>(layer == 0 ? L4 : layer == 1 ? R4 : ButtonCount),
                    settingsPath.c_str());
                entry.trigger = trigger < ButtonCount ? trigger : ButtonCount;
                for (size_t i = 0; i < ButtonCount; ++i) {
                    entry.mapping[i] = ReadLayerBinding(section.c_str(),
                        prefix + L"_" + BUTTONS[i].setting);
                    entry.turbo[i] = ReadTurbo(section.c_str(), prefix + L"_",
                                                BUTTONS[i].setting);
                    entry.sequence[i] = ReadSequence(section.c_str(), prefix + L"_",
                                                      BUTTONS[i].setting);
                }
                preset.layers.push_back(std::move(entry));
            }
            presets.push_back(std::move(preset));
        }
        selectedPreset = GetPrivateProfileIntW(L"MapleStory", L"SelectedPreset", 0,
                                                settingsPath.c_str());
        if (selectedPreset >= presets.size()) selectedPreset = 0;
    }
    PublishSelectedPreset();
}

#include "SettingsJson.inc"

bool SetBinding(size_t button, uint32_t value, const TurboSettings& turbo,
                const SequenceSettings& sequence) {
    uint32_t& destination = editedLayer == 0 ? presets[selectedPreset].mapping[button] :
        presets[selectedPreset].layers[editedLayer - 1].mapping[button];
    TurboSettings& turboDestination = editedLayer == 0 ? presets[selectedPreset].turbo[button] :
        presets[selectedPreset].layers[editedLayer - 1].turbo[button];
    SequenceSettings& sequenceDestination = editedLayer == 0 ?
        presets[selectedPreset].sequence[button] :
        presets[selectedPreset].layers[editedLayer - 1].sequence[button];
    const uint32_t old = destination;
    const TurboSettings oldTurbo = turboDestination;
    const SequenceSettings oldSequence = sequenceDestination;
    destination = value;
    turboDestination = turbo;
    sequenceDestination = sequence;
    if (!SavePresets()) {
        destination = old;
        turboDestination = oldTurbo;
        sequenceDestination = oldSequence;
        return false;
    }
    PublishSelectedPreset();
    return true;
}

void UpdatePresetBox() {
    if (!presetBox) return;
    SendMessageW(presetBox, CB_RESETCONTENT, 0, 0);
    for (const auto& preset : presets)
        SendMessageW(presetBox, CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(preset.name.c_str()));
    SendMessageW(presetBox, CB_SETCURSEL, selectedPreset, 0);
}

bool SelectPreset(size_t index) {
    if (index >= presets.size()) return false;
    if (index == selectedPreset) return true;
    const size_t old = selectedPreset;
    selectedPreset = index;
    if (!SavePresets()) {
        selectedPreset = old;
        return false;
    }
    PublishSelectedPreset();
    UpdatePresetBox();
    UpdateLayerControls();
    UpdateTargetButton();
    if (requested && presets[selectedPreset].targetExecutable.empty()) {
        requested = false;
        if (toggleButton) SetWindowTextW(toggleButton, L"Steamless Mode を有効にする");
    }
    return true;
}

bool SetButtonRole(size_t button, bool asLayer, size_t targetLayer, uint32_t key,
                   const TurboSettings& turbo, const SequenceSettings& sequence) {
    Preset previous = presets[selectedPreset];
    auto& preset = presets[selectedPreset];
    size_t previousLayer = preset.layers.size();
    for (size_t layer = 0; layer < preset.layers.size(); ++layer) {
        if (preset.layers[layer].trigger == button) {
            previousLayer = layer;
            break;
        }
    }
    if (asLayer) {
        if (targetLayer > preset.layers.size() ||
            (targetLayer == preset.layers.size() && preset.layers.size() >= ButtonCount))
            return false;
        if (previousLayer < preset.layers.size() && previousLayer != targetLayer) {
            preset.layers.erase(preset.layers.begin() + static_cast<std::ptrdiff_t>(previousLayer));
            if (previousLayer < targetLayer) --targetLayer;
        }
        if (targetLayer == preset.layers.size()) preset.layers.emplace_back();
        preset.layers[targetLayer].trigger = button;
    } else {
        if (previousLayer < preset.layers.size())
            preset.layers.erase(preset.layers.begin() + static_cast<std::ptrdiff_t>(previousLayer));
        preset.mapping[button] = key;
        preset.turbo[button] = turbo;
        preset.sequence[button] = sequence;
    }
    if (!SavePresets()) {
        presets[selectedPreset] = std::move(previous);
        return false;
    }
    PublishSelectedPreset();
    UpdateLayerControls();
    return true;
}

void UpdateRoleControls() {
    if (!dialogWindow) return;
    const bool showKey = !pendingRoleLayer && !pendingRoleSequence;
    ShowWindow(roleBox, SW_SHOW);
    ShowWindow(roleLayerBox, pendingRoleLayer ? SW_SHOW : SW_HIDE);
    ShowWindow(captureButton, showKey ? SW_SHOW : SW_HIDE);
    ShowWindow(disableButton, showKey ? SW_SHOW : SW_HIDE);
    ShowWindow(inheritButton, editedLayer != 0 && showKey ? SW_SHOW : SW_HIDE);
    ShowWindow(sequenceButton, pendingRoleSequence ? SW_SHOW : SW_HIDE);
    const bool showTurbo = showKey && pendingBinding != 0 && pendingBinding != INHERIT;
    ShowWindow(turboCheckbox, showTurbo ? SW_SHOW : SW_HIDE);
    ShowWindow(turboIntervalLabel, showTurbo ? SW_SHOW : SW_HIDE);
    ShowWindow(turboIntervalEdit, showTurbo ? SW_SHOW : SW_HIDE);
    ShowWindow(turboDelayLabel, showTurbo ? SW_SHOW : SW_HIDE);
    ShowWindow(turboDelayEdit, showTurbo ? SW_SHOW : SW_HIDE);
    EnableWindow(turboIntervalEdit, showTurbo && pendingTurbo.enabled);
    EnableWindow(turboDelayEdit, showTurbo && pendingTurbo.enabled);
    if (pendingRoleLayer)
        SetWindowTextW(capturedLabel, L"押している間だけレイヤーを有効にします");
    else if (pendingRoleSequence)
        SetWindowTextW(capturedLabel,
            (L"シーケンス：" + std::to_wstring(pendingSequence.keys.size()) + L"入力").c_str());
    else
        SetWindowTextW(capturedLabel, (L"現在：" + KeyName(pendingBinding)).c_str());
}

void ShowBindingDialog(HWND parent, size_t button) {
    if (dialogWindow) { SetForegroundWindow(dialogWindow); return; }
    selectedButton = button;
    pendingBinding = editedLayer == 0 ? bindings[button].load() :
        presets[selectedPreset].layers[editedLayer - 1].mapping[button];
    pendingTurbo = editedLayer == 0 ? presets[selectedPreset].turbo[button] :
        presets[selectedPreset].layers[editedLayer - 1].turbo[button];
    pendingSequence = editedLayer == 0 ? presets[selectedPreset].sequence[button] :
        presets[selectedPreset].layers[editedLayer - 1].sequence[button];
    pendingRoleLayer = false;
    pendingRoleSequence = pendingSequence.enabled;
    pendingTargetLayer = presets[selectedPreset].layers.size();
    if (editedLayer == 0) {
        for (size_t layer = 0; layer < presets[selectedPreset].layers.size(); ++layer) {
            if (presets[selectedPreset].layers[layer].trigger == button) {
                pendingRoleLayer = true;
                pendingRoleSequence = false;
                pendingTargetLayer = layer;
                break;
            }
        }
    }
    capturing = false;
    const std::wstring title = std::wstring(BUTTONS[button].name) + L" ボタンの設定";
    dialogWindow = CreateWindowExW(WS_EX_DLGMODALFRAME, L"MapleStoryBinding",
        title.c_str(), WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        CW_USEDEFAULT, CW_USEDEFAULT, 390, 435, parent, nullptr,
        GetModuleHandleW(nullptr), nullptr);
    if (!dialogWindow) return;
    EnableWindow(parent, FALSE);
    roleBox = CreateWindowW(L"COMBOBOX", L"", WS_CHILD | WS_TABSTOP | CBS_DROPDOWNLIST,
        20, 18, 340, 110, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_ROLE)), nullptr, nullptr);
    SendMessageW(roleBox, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"キー入力として使う"));
    SendMessageW(roleBox, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"シーケンスとして使う"));
    if (editedLayer == 0)
        SendMessageW(roleBox, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(L"レイヤー切替ボタンにする"));
    SendMessageW(roleBox, CB_SETCURSEL,
                 pendingRoleLayer ? 2 : pendingRoleSequence ? 1 : 0, 0);
    roleLayerBox = CreateWindowW(L"COMBOBOX", L"", WS_CHILD | WS_TABSTOP | CBS_DROPDOWNLIST,
        20, 56, 340, 160, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_ROLE_LAYER)), nullptr, nullptr);
    for (size_t layer = 0; layer < presets[selectedPreset].layers.size(); ++layer) {
        const std::wstring label =
            (presets[selectedPreset].layers[layer].name.empty() ?
             L"レイヤー" + std::to_wstring(layer + 1) :
             presets[selectedPreset].layers[layer].name) + L"に割り当てる";
        SendMessageW(roleLayerBox, CB_ADDSTRING, 0, reinterpret_cast<LPARAM>(label.c_str()));
    }
    SendMessageW(roleLayerBox, CB_ADDSTRING, 0,
                 reinterpret_cast<LPARAM>(L"新しいレイヤーを作る"));
    SendMessageW(roleLayerBox, CB_SETCURSEL, pendingTargetLayer, 0);
    capturedLabel = CreateWindowW(L"STATIC", L"", WS_CHILD | WS_VISIBLE,
        20, 100, 340, 26, dialogWindow, nullptr, nullptr, nullptr);
    captureButton = CreateWindowW(L"BUTTON", L"キーを入力して設定", WS_CHILD | WS_TABSTOP,
        20, 135, 340, 34, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_CAPTURE)), nullptr, nullptr);
    sequenceButton = CreateWindowW(L"BUTTON", L"シーケンスを編集...", WS_CHILD | WS_TABSTOP,
        20, 135, 340, 34, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_EDIT)), nullptr, nullptr);
    disableButton = CreateWindowW(L"BUTTON", L"無効にする", WS_CHILD | WS_TABSTOP,
        20, 180, 160, 34, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_DISABLE)), nullptr, nullptr);
    inheritButton = CreateWindowW(L"BUTTON", L"引き継ぐ", WS_CHILD | WS_TABSTOP,
        200, 180, 160, 34, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_INHERIT)), nullptr, nullptr);
    turboCheckbox = CreateWindowW(L"BUTTON", L"連打を有効にする",
        WS_CHILD | WS_TABSTOP | BS_AUTOCHECKBOX, 20, 230, 340, 27,
        dialogWindow, reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_TURBO)), nullptr, nullptr);
    SendMessageW(turboCheckbox, BM_SETCHECK, pendingTurbo.enabled ? BST_CHECKED : BST_UNCHECKED, 0);
    turboIntervalLabel = CreateWindowW(L"STATIC", L"連打間隔 (ms)", WS_CHILD,
        20, 266, 170, 27, dialogWindow, nullptr, nullptr, nullptr);
    turboIntervalEdit = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT",
        std::to_wstring(pendingTurbo.intervalMs).c_str(),
        WS_CHILD | WS_TABSTOP | ES_NUMBER, 205, 263, 155, 28,
        dialogWindow, reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_TURBO_INTERVAL)), nullptr, nullptr);
    SendMessageW(turboIntervalEdit, EM_LIMITTEXT, 4, 0);
    turboDelayLabel = CreateWindowW(L"STATIC", L"初回遅延 (ms)", WS_CHILD,
        20, 304, 170, 27, dialogWindow, nullptr, nullptr, nullptr);
    turboDelayEdit = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT",
        std::to_wstring(pendingTurbo.delayMs).c_str(),
        WS_CHILD | WS_TABSTOP | ES_NUMBER, 205, 301, 155, 28,
        dialogWindow, reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_TURBO_DELAY)), nullptr, nullptr);
    SendMessageW(turboDelayEdit, EM_LIMITTEXT, 4, 0);
    CreateWindowW(L"BUTTON", L"決定", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        20, 350, 160, 34, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_CONFIRM)), nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"キャンセル", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        200, 350, 160, 34, dialogWindow,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_CANCEL)), nullptr, nullptr);
    UpdateRoleControls();
}

void RefreshSequenceList(size_t selection) {
    if (!sequenceList) return;
    SendMessageW(sequenceList, LB_RESETCONTENT, 0, 0);
    for (size_t index = 0; index < pendingSequence.keys.size(); ++index) {
        const std::wstring label = std::to_wstring(index + 1) + L". " +
            KeyName(pendingSequence.keys[index]);
        SendMessageW(sequenceList, LB_ADDSTRING, 0, reinterpret_cast<LPARAM>(label.c_str()));
    }
    if (!pendingSequence.keys.empty()) {
        if (selection >= pendingSequence.keys.size()) selection = pendingSequence.keys.size() - 1;
        SendMessageW(sequenceList, LB_SETCURSEL, selection, 0);
    }
}

LRESULT CALLBACK SequenceProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_COMMAND:
        switch (LOWORD(wParam)) {
        case ID_SEQUENCE_ADD:
            if (pendingSequence.keys.size() >= 16) {
                MessageBoxW(window, L"入力は最大16件です。", L"追加できません", MB_OK | MB_ICONINFORMATION);
                return 0;
            }
            sequenceCapturing = true;
            SetWindowTextW(sequenceStatusLabel, L"追加するキーを押してください");
            SetFocus(window);
            return 0;
        case ID_SEQUENCE_REMOVE:
        case ID_SEQUENCE_UP:
        case ID_SEQUENCE_DOWN: {
            const LRESULT selected = SendMessageW(sequenceList, LB_GETCURSEL, 0, 0);
            if (selected == LB_ERR) return 0;
            const size_t index = static_cast<size_t>(selected);
            if (LOWORD(wParam) == ID_SEQUENCE_REMOVE) {
                pendingSequence.keys.erase(pendingSequence.keys.begin() +
                    static_cast<std::ptrdiff_t>(index));
                RefreshSequenceList(index);
            } else if (LOWORD(wParam) == ID_SEQUENCE_UP && index > 0) {
                std::swap(pendingSequence.keys[index], pendingSequence.keys[index - 1]);
                RefreshSequenceList(index - 1);
            } else if (LOWORD(wParam) == ID_SEQUENCE_DOWN &&
                       index + 1 < pendingSequence.keys.size()) {
                std::swap(pendingSequence.keys[index], pendingSequence.keys[index + 1]);
                RefreshSequenceList(index + 1);
            }
            return 0;
        }
        case ID_SEQUENCE_OK: {
            BOOL valid = FALSE;
            const UINT interval = GetDlgItemInt(window, ID_SEQUENCE_INTERVAL, &valid, FALSE);
            if (!valid || interval < 20 || interval > 2000 || pendingSequence.keys.empty()) {
                MessageBoxW(window, L"1件以上のキーと20〜2000 msの間隔を設定してください。",
                            L"シーケンス設定を確認", MB_OK | MB_ICONWARNING);
                return 0;
            }
            pendingSequence.intervalMs = interval;
            pendingSequence.repeat =
                SendMessageW(sequenceRepeatBox, BM_GETCHECK, 0, 0) == BST_CHECKED;
            pendingSequence.enabled = true;
            sequenceAccepted = true;
            DestroyWindow(window);
            return 0;
        }
        case ID_SEQUENCE_CANCEL: DestroyWindow(window); return 0;
        }
        break;
    case WM_KEYDOWN:
    case WM_SYSKEYDOWN:
        if (sequenceCapturing) {
            const uint32_t scan = (static_cast<uint32_t>(lParam) >> 16) & 0xFFu;
            if (scan) {
                pendingSequence.keys.push_back(scan | ((lParam & (1u << 24)) ? EXTENDED : 0));
                RefreshSequenceList(pendingSequence.keys.size() - 1);
                sequenceCapturing = false;
                SetWindowTextW(sequenceStatusLabel, L"＋入力追加から次のキーを設定できます");
            }
            return 0;
        }
        if (wParam == VK_ESCAPE) { DestroyWindow(window); return 0; }
        break;
    case WM_CLOSE: DestroyWindow(window); return 0;
    case WM_DESTROY:
        if (!sequenceAccepted) pendingSequence = sequenceBeforeEdit;
        sequenceDialog = nullptr;
        sequenceList = nullptr;
        sequenceIntervalEdit = nullptr;
        sequenceRepeatBox = nullptr;
        sequenceStatusLabel = nullptr;
        sequenceCapturing = false;
        EnableWindow(dialogWindow, TRUE);
        SetForegroundWindow(dialogWindow);
        UpdateRoleControls();
        return 0;
    }
    return DefWindowProcW(window, message, wParam, lParam);
}

void ShowSequenceDialog() {
    if (sequenceDialog) { SetForegroundWindow(sequenceDialog); return; }
    sequenceBeforeEdit = pendingSequence;
    sequenceAccepted = false;
    sequenceCapturing = false;
    sequenceDialog = CreateWindowExW(WS_EX_DLGMODALFRAME, L"MapleStorySequence",
        L"シーケンス設定", WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
        CW_USEDEFAULT, CW_USEDEFAULT, 430, 490, dialogWindow, nullptr,
        GetModuleHandleW(nullptr), nullptr);
    if (!sequenceDialog) return;
    EnableWindow(dialogWindow, FALSE);
    sequenceList = CreateWindowW(L"LISTBOX", L"", WS_CHILD | WS_VISIBLE |
        WS_TABSTOP | WS_VSCROLL | WS_BORDER | LBS_NOTIFY,
        20, 20, 385, 190, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_LIST)), nullptr, nullptr);
    RefreshSequenceList(0);
    sequenceStatusLabel = CreateWindowW(L"STATIC", L"＋入力追加からキーを設定できます",
        WS_CHILD | WS_VISIBLE, 20, 215, 385, 26,
        sequenceDialog, nullptr, nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"＋入力追加", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        20, 250, 180, 32, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_ADD)), nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"選択を削除", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        225, 250, 180, 32, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_REMOVE)), nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"上へ", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        20, 290, 180, 30, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_UP)), nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"下へ", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        225, 290, 180, 30, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_DOWN)), nullptr, nullptr);
    CreateWindowW(L"STATIC", L"間隔 (ms)", WS_CHILD | WS_VISIBLE,
        20, 334, 170, 27, sequenceDialog, nullptr, nullptr, nullptr);
    sequenceIntervalEdit = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT",
        std::to_wstring(pendingSequence.intervalMs).c_str(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_NUMBER,
        205, 331, 200, 28, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_INTERVAL)), nullptr, nullptr);
    SendMessageW(sequenceIntervalEdit, EM_LIMITTEXT, 4, 0);
    sequenceRepeatBox = CreateWindowW(L"BUTTON", L"押している間繰り返す",
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTOCHECKBOX,
        20, 370, 385, 27, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_REPEAT)), nullptr, nullptr);
    SendMessageW(sequenceRepeatBox, BM_SETCHECK,
                 pendingSequence.repeat ? BST_CHECKED : BST_UNCHECKED, 0);
    CreateWindowW(L"BUTTON", L"決定", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        20, 410, 180, 34, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_OK)), nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"キャンセル", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        225, 410, 180, 34, sequenceDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_SEQUENCE_CANCEL)), nullptr, nullptr);
}

LRESULT CALLBACK BindingProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_COMMAND:
        if (LOWORD(wParam) == ID_TURBO && HIWORD(wParam) == BN_CLICKED) {
            pendingTurbo.enabled = SendMessageW(turboCheckbox, BM_GETCHECK, 0, 0) == BST_CHECKED;
            UpdateRoleControls();
            return 0;
        }
        if (LOWORD(wParam) == ID_ROLE && HIWORD(wParam) == CBN_SELCHANGE) {
            const LRESULT choice = SendMessageW(roleBox, CB_GETCURSEL, 0, 0);
            pendingRoleSequence = choice == 1;
            pendingRoleLayer = editedLayer == 0 && choice == 2;
            UpdateRoleControls();
            return 0;
        }
        if (LOWORD(wParam) == ID_ROLE_LAYER && HIWORD(wParam) == CBN_SELCHANGE) {
            pendingTargetLayer = static_cast<size_t>(SendMessageW(roleLayerBox, CB_GETCURSEL, 0, 0));
            return 0;
        }
        switch (LOWORD(wParam)) {
        case ID_SEQUENCE_EDIT:
            ShowSequenceDialog();
            return 0;
        case ID_CAPTURE:
            capturing = true;
            SetWindowTextW(capturedLabel, L"設定するキーを押してください...");
            SetFocus(window);
            return 0;
        case ID_DISABLE:
            pendingBinding = 0;
            pendingRoleSequence = false;
            SendMessageW(roleBox, CB_SETCURSEL, 0, 0);
            capturing = false;
            UpdateRoleControls();
            return 0;
        case ID_INHERIT:
            pendingBinding = INHERIT;
            pendingRoleSequence = false;
            SendMessageW(roleBox, CB_SETCURSEL, 0, 0);
            capturing = false;
            UpdateRoleControls();
            return 0;
        case ID_CONFIRM: {
            if (pendingRoleSequence && pendingSequence.keys.empty()) {
                MessageBoxW(window, L"シーケンスにキーを追加してください。",
                            L"入力列が未設定です", MB_OK | MB_ICONWARNING);
                return 0;
            }
            pendingSequence.enabled = pendingRoleSequence;
            if (pendingBinding == 0 || pendingBinding == INHERIT ||
                pendingRoleLayer || pendingRoleSequence)
                pendingTurbo.enabled = false;
            if (pendingTurbo.enabled) {
                BOOL intervalValid = FALSE, delayValid = FALSE;
                const UINT interval = GetDlgItemInt(window, ID_TURBO_INTERVAL, &intervalValid, FALSE);
                const UINT delay = GetDlgItemInt(window, ID_TURBO_DELAY, &delayValid, FALSE);
                if (!intervalValid || !delayValid || interval < 20 || interval > 2000 || delay > 5000) {
                    MessageBoxW(window, L"連打間隔は20〜2000 ms、初回遅延は0〜5000 msで入力してください。",
                                L"連打設定を確認", MB_OK | MB_ICONWARNING);
                    return 0;
                }
                pendingTurbo.intervalMs = interval;
                pendingTurbo.delayMs = delay;
            }
            bool removesLayer = false;
            if (editedLayer == 0) {
                for (size_t layer = 0; layer < presets[selectedPreset].layers.size(); ++layer)
                    if (presets[selectedPreset].layers[layer].trigger == selectedButton &&
                        (!pendingRoleLayer || pendingTargetLayer != layer)) removesLayer = true;
            }
            if (removesLayer && MessageBoxW(window,
                L"元のレイヤーとそのキー設定を削除します。続けますか？",
                L"レイヤーの削除", MB_YESNO | MB_ICONQUESTION) != IDYES) return 0;
            const bool saved = editedLayer == 0 ?
                SetButtonRole(selectedButton, pendingRoleLayer, pendingTargetLayer, pendingBinding,
                              pendingTurbo, pendingSequence) :
                SetBinding(selectedButton, pendingBinding, pendingTurbo, pendingSequence);
            if (!saved) {
                MessageBoxW(window, L"設定を保存できませんでした。", L"保存エラー", MB_OK | MB_ICONERROR);
                return 0;
            }
            UpdateLayerControls();
            DestroyWindow(window);
            return 0;
        }
        case ID_CANCEL: DestroyWindow(window); return 0;
        }
        break;
    case WM_KEYDOWN:
    case WM_SYSKEYDOWN:
        if (capturing) {
            const uint32_t scan = (static_cast<uint32_t>(lParam) >> 16) & 0xFFu;
            if (scan) {
                pendingBinding = scan | ((lParam & (1u << 24)) ? EXTENDED : 0);
                const std::wstring label = L"設定するキー：" + KeyName(pendingBinding);
                SetWindowTextW(capturedLabel, label.c_str());
                capturing = false;
                UpdateRoleControls();
            }
            return 0;
        }
        if (wParam == VK_ESCAPE) { DestroyWindow(window); return 0; }
        break;
    case WM_CLOSE: DestroyWindow(window); return 0;
    case WM_DESTROY:
        dialogWindow = nullptr;
        capturedLabel = nullptr;
        roleBox = nullptr;
        roleLayerBox = nullptr;
        captureButton = nullptr;
        disableButton = nullptr;
        inheritButton = nullptr;
        turboCheckbox = nullptr;
        turboIntervalEdit = nullptr;
        turboDelayEdit = nullptr;
        turboIntervalLabel = nullptr;
        turboDelayLabel = nullptr;
        sequenceButton = nullptr;
        capturing = false;
        EnableWindow(mainWindow, TRUE);
        SetForegroundWindow(mainWindow);
        return 0;
    }
    return DefWindowProcW(window, message, wParam, lParam);
}

bool CommitPresetName(HWND window) {
    wchar_t buffer[128]{};
    GetWindowTextW(nameEdit, buffer, static_cast<int>(std::size(buffer)));
    std::wstring name = buffer;
    const size_t first = name.find_first_not_of(L" \t\r\n");
    if (first == std::wstring::npos) {
        MessageBoxW(window, L"プリセット名を入力してください。", L"名前が必要です", MB_OK | MB_ICONWARNING);
        return false;
    }
    name = name.substr(first, name.find_last_not_of(L" \t\r\n") - first + 1);
    if (nameAction == NameAction::LayerRename) {
        for (size_t layer = 0; layer < presets[selectedPreset].layers.size(); ++layer) {
            if (layer != static_cast<size_t>(editedLayer - 1) &&
                _wcsicmp(name.c_str(), presets[selectedPreset].layers[layer].name.c_str()) == 0) {
                MessageBoxW(window, L"同じ名前のレイヤーがあります。", L"名前を変更してください",
                            MB_OK | MB_ICONWARNING);
                return false;
            }
        }
        auto& current = presets[selectedPreset].layers[editedLayer - 1].name;
        const std::wstring old = current;
        current = name;
        if (!SavePresets()) {
            current = old;
            MessageBoxW(window, L"名前を保存できませんでした。", L"保存エラー", MB_OK | MB_ICONERROR);
            return false;
        }
        UpdateLayerControls();
        return true;
    }
    for (size_t i = 0; i < presets.size(); ++i) {
        if (nameAction == NameAction::Rename && i == selectedPreset) continue;
        if (_wcsicmp(name.c_str(), presets[i].name.c_str()) == 0) {
            MessageBoxW(window, L"同じ名前のプリセットがあります。", L"名前を変更してください",
                        MB_OK | MB_ICONWARNING);
            return false;
        }
    }
    if (nameAction == NameAction::Rename) {
        const std::wstring old = presets[selectedPreset].name;
        presets[selectedPreset].name = name;
        if (!SavePresets()) {
            presets[selectedPreset].name = old;
            MessageBoxW(window, L"名前を保存できませんでした。", L"保存エラー", MB_OK | MB_ICONERROR);
            return false;
        }
        UpdatePresetBox();
    } else {
        Preset preset;
        preset.name = name;
        if (nameAction == NameAction::Copy) {
            preset.mapping = presets[selectedPreset].mapping;
            preset.turbo = presets[selectedPreset].turbo;
            preset.sequence = presets[selectedPreset].sequence;
            preset.layers = presets[selectedPreset].layers;
            preset.targetExecutable = presets[selectedPreset].targetExecutable;
        }
        presets.push_back(std::move(preset));
        if (!SelectPreset(presets.size() - 1)) {
            presets.pop_back();
            MessageBoxW(window, L"プリセットを保存できませんでした。", L"保存エラー",
                        MB_OK | MB_ICONERROR);
            return false;
        }
    }
    return true;
}

LRESULT CALLBACK NameProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam) {
    switch (message) {
    case WM_COMMAND:
        if (LOWORD(wParam) == ID_NAME_OK) {
            if (CommitPresetName(window)) DestroyWindow(window);
            return 0;
        }
        if (LOWORD(wParam) == ID_NAME_CANCEL) { DestroyWindow(window); return 0; }
        break;
    case WM_CLOSE: DestroyWindow(window); return 0;
    case WM_DESTROY:
        nameDialog = nullptr;
        nameEdit = nullptr;
        EnableWindow(mainWindow, TRUE);
        SetForegroundWindow(mainWindow);
        return 0;
    }
    return DefWindowProcW(window, message, wParam, lParam);
}

void ShowNameDialog(NameAction action) {
    if (nameDialog) { SetForegroundWindow(nameDialog); return; }
    nameAction = action;
    suggestedName = action == NameAction::LayerRename ?
        (presets[selectedPreset].layers[editedLayer - 1].name.empty() ?
         L"レイヤー" + std::to_wstring(editedLayer) :
         presets[selectedPreset].layers[editedLayer - 1].name) :
        action == NameAction::New ? L"新しいプリセット" :
        action == NameAction::Copy ? presets[selectedPreset].name + L" のコピー" :
        presets[selectedPreset].name;
    const wchar_t* title = action == NameAction::New ? L"プリセットを新規作成" :
        action == NameAction::Copy ? L"プリセットを複製" :
        action == NameAction::LayerRename ? L"レイヤー名を変更" : L"プリセット名を変更";
    nameDialog = CreateWindowExW(WS_EX_DLGMODALFRAME, L"MapleStoryPresetName", title,
        WS_CAPTION | WS_SYSMENU | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT,
        390, 185, mainWindow, nullptr, GetModuleHandleW(nullptr), nullptr);
    if (!nameDialog) return;
    EnableWindow(mainWindow, FALSE);
    CreateWindowW(L"STATIC", action == NameAction::LayerRename ? L"レイヤー名" : L"プリセット名",
        WS_CHILD | WS_VISIBLE,
        20, 18, 340, 24, nameDialog, nullptr, nullptr, nullptr);
    nameEdit = CreateWindowExW(WS_EX_CLIENTEDGE, L"EDIT", suggestedName.c_str(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
        20, 45, 340, 27, nameDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_NAME_EDIT)), nullptr, nullptr);
    SendMessageW(nameEdit, EM_LIMITTEXT, 40, 0);
    CreateWindowW(L"BUTTON", L"決定", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        195, 90, 75, 32, nameDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_NAME_OK)), nullptr, nullptr);
    CreateWindowW(L"BUTTON", L"キャンセル", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        280, 90, 80, 32, nameDialog,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(ID_NAME_CANCEL)), nullptr, nullptr);
    SetFocus(nameEdit);
    SendMessageW(nameEdit, EM_SETSEL, 0, -1);
}

void DeleteCurrentPreset() {
    if (presets.size() == 1) {
        MessageBoxW(mainWindow, L"最後のプリセットは削除できません。", L"削除できません",
                    MB_OK | MB_ICONINFORMATION);
        return;
    }
    if (MessageBoxW(mainWindow, L"選択中のプリセットを削除しますか？", L"プリセットを削除",
                    MB_YESNO | MB_ICONQUESTION) != IDYES) return;
    const auto previous = presets;
    const size_t previousIndex = selectedPreset;
    presets.erase(presets.begin() + static_cast<std::ptrdiff_t>(selectedPreset));
    if (selectedPreset >= presets.size()) selectedPreset = presets.size() - 1;
    if (!SelectPreset(selectedPreset)) {
        presets = previous;
        selectedPreset = previousIndex;
        UpdatePresetBox();
        MessageBoxW(mainWindow, L"削除結果を保存できませんでした。", L"保存エラー",
                    MB_OK | MB_ICONERROR);
    }
}

bool ChooseFile(HWND owner, bool save, const wchar_t* title, const wchar_t* filter,
                wchar_t (&path)[MAX_PATH * 2]) {
    OPENFILENAMEW dialog{};
    dialog.lStructSize = sizeof(dialog);
    dialog.hwndOwner = owner;
    dialog.lpstrFilter = filter;
    dialog.lpstrFile = path;
    dialog.nMaxFile = static_cast<DWORD>(std::size(path));
    dialog.lpstrTitle = title;
    dialog.lpstrDefExt = save ? L"json" : nullptr;
    dialog.Flags = OFN_EXPLORER | OFN_NOCHANGEDIR |
        (save ? OFN_OVERWRITEPROMPT : OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST);
    const bool chosen = save ? GetSaveFileNameW(&dialog) != 0 : GetOpenFileNameW(&dialog) != 0;
    if (!chosen && CommDlgExtendedError())
        MessageBoxW(owner, L"ファイル選択画面を開けませんでした。", L"ファイル選択エラー",
                    MB_OK | MB_ICONERROR);
    return chosen;
}

bool SetTargetExecutable(const std::wstring& path) {
    std::wstring& target = presets[selectedPreset].targetExecutable;
    const std::wstring previous = target;
    target = path;
    if (!SavePresets()) {
        target = previous;
        return false;
    }
    PublishSelectedPreset();
    UpdateTargetButton();
    return true;
}

std::wstring TargetIconCacheDirectory() {
    const size_t separator = settingsPath.find_last_of(L"\\/");
    return separator == std::wstring::npos ? L"" :
        settingsPath.substr(0, separator) + L"\\TargetIcons";
}

void PickTargetExecutable(HWND window) {
    wchar_t path[MAX_PATH * 2]{};
    if (!ChooseFile(window, false, L"自動有効化するアプリの実行ファイルを選択",
                    L"実行ファイル (*.exe)\0*.exe\0\0", path)) return;
    const wchar_t* extension = wcsrchr(path, L'.');
    if (!extension || _wcsicmp(extension, L".exe") != 0) {
        MessageBoxW(window, L".exe ファイルを選択してください。", L"対象アプリを確認",
                    MB_OK | MB_ICONWARNING);
        return;
    }
    if (!SetTargetExecutable(path))
        MessageBoxW(window, L"対象アプリを保存できませんでした。", L"保存エラー",
                    MB_OK | MB_ICONERROR);
    else TargetIconDataUri(path, TargetIconCacheDirectory(), true);
}

void ExportSettings(HWND window) {
    wchar_t path[MAX_PATH * 2] = L"Remapcon-settings.json";
    if (!ChooseFile(window, true, L"設定をエクスポート",
                    L"JSON 設定 (*.json)\0*.json\0\0", path)) return;
    if (!SavePresets() ||
        (_wcsicmp(path, settingsPath.c_str()) != 0 &&
         !CopyFileW(settingsPath.c_str(), path, FALSE))) {
        MessageBoxW(window, L"設定をエクスポートできませんでした。", L"保存エラー",
                    MB_OK | MB_ICONERROR);
        return;
    }
    MessageBoxW(window, L"設定をエクスポートしました。", L"完了", MB_OK | MB_ICONINFORMATION);
}

void ImportSettings(HWND window) {
    wchar_t path[MAX_PATH * 2]{};
    if (!ChooseFile(window, false, L"設定をインポート",
                    L"JSON 設定 (*.json)\0*.json\0\0", path)) return;
    ConfigSnapshot imported;
    if (!ReadConfig(path, imported)) {
        MessageBoxW(window, L"このアプリからエクスポートしたJSONファイルを選択してください。",
                    L"設定形式が違います", MB_OK | MB_ICONWARNING);
        return;
    }
    bool targetMissing = false;
    for (auto& preset : imported.presets) {
        if (preset.targetExecutable.empty() ||
            preset.targetExecutable.find_first_of(L"\\/") == std::wstring::npos) continue;
        const DWORD attributes = GetFileAttributesW(preset.targetExecutable.c_str());
        if (attributes == INVALID_FILE_ATTRIBUTES || (attributes & FILE_ATTRIBUTE_DIRECTORY)) {
            preset.targetExecutable.clear();
            targetMissing = true;
        }
    }
    if (targetMissing) imported.autoMode = false;
    if (!SavePresets() || !CopyFileW(settingsPath.c_str(),
                   (settingsPath + L".before-import.json").c_str(), FALSE)) {
        MessageBoxW(window, L"現在の設定をバックアップできませんでした。", L"インポート中止",
                    MB_OK | MB_ICONERROR);
        return;
    }
    ConfigSnapshot previous{language, closeBehavior, autoMode.load(), steamTakeover.load(), selectedPreset,
                            folders, presets};
    const bool previousRequested = requested.exchange(false);
    ApplyConfig(std::move(imported));
    if (!SavePresets()) {
        ApplyConfig(std::move(previous));
        requested = previousRequested;
        MessageBoxW(window, L"設定を保存できませんでした。", L"インポートエラー",
                    MB_OK | MB_ICONERROR);
        return;
    }
    editedLayer = 0;
    UpdatePresetBox();
    UpdateLayerControls();
    UpdateTargetButton();
    SendMessageW(autoModeBox, BM_SETCHECK, autoMode ? BST_CHECKED : BST_UNCHECKED, 0);
    EnableWindow(toggleButton, !autoMode);
    SetWindowTextW(toggleButton, L"Steamless Mode を有効にする");
    MessageBoxW(window, targetMissing ?
        L"設定を読み込みました。このPCにない対象アプリの指定を解除し、自動モードをオフにしました。対象アプリを選び直してください。" :
        L"設定を読み込みました。", L"完了", MB_OK | MB_ICONINFORMATION);
}

void ShowPopup(HWND window, HWND button, bool targetMenu) {
    HMENU menu = CreatePopupMenu();
    if (!menu) return;
    if (targetMenu) {
        AppendMenuW(menu, MF_STRING, ID_TARGET_PICK, L"実行ファイルを選ぶ...");
        AppendMenuW(menu, MF_STRING, ID_TARGET_RESET, L"対象アプリの指定を解除");
    } else {
        AppendMenuW(menu, MF_STRING, ID_CONFIG_IMPORT, L"設定をインポート...");
        AppendMenuW(menu, MF_STRING, ID_CONFIG_EXPORT, L"設定をエクスポート...");
    }
    RECT rectangle{};
    GetWindowRect(button, &rectangle);
    const UINT selected = TrackPopupMenu(menu, TPM_LEFTALIGN | TPM_TOPALIGN |
        TPM_RETURNCMD | TPM_NONOTIFY, rectangle.left, rectangle.bottom, 0, window, nullptr);
    DestroyMenu(menu);
    if (selected) SendMessageW(window, WM_COMMAND, MAKEWPARAM(selected, 0), 0);
}

bool HideToTray(HWND window);
void ForceExit(HWND window);
bool StartUpdate(const std::wstring& tag);

#include "WebBridge.inc"

bool CanWriteDirectory(const std::filesystem::path& directory) {
    const auto probe = directory / (L".remapcon-update-" + std::to_wstring(GetCurrentProcessId()));
    HANDLE file = CreateFileW(probe.c_str(), GENERIC_WRITE | DELETE, 0, nullptr, CREATE_NEW,
        FILE_ATTRIBUTE_TEMPORARY | FILE_FLAG_DELETE_ON_CLOSE, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    CloseHandle(file);
    return true;
}

bool StartUpdate(const std::wstring& tag) {
    if (updateInProgress || !ValidUpdateTag(tag)) return false;
    wchar_t executable[MAX_PATH]{};
    const DWORD size = GetModuleFileNameW(nullptr, executable, MAX_PATH);
    if (!size || size >= MAX_PATH) return false;
    const std::filesystem::path install = std::filesystem::path(executable).parent_path();
    wchar_t tempRoot[MAX_PATH]{};
    const DWORD tempLength = GetTempPathW(MAX_PATH, tempRoot);
    if (!tempLength || tempLength >= MAX_PATH) return false;
    wchar_t tempName[MAX_PATH]{};
    if (!GetTempFileNameW(tempRoot, L"rmu", 0, tempName) ||
        !DeleteFileW(tempName) || !CreateDirectoryW(tempName, nullptr)) return false;
    const std::filesystem::path temporary(tempName);
    const auto updater = temporary / L"RemapconUpdater.exe";
    if (!CopyFileW((install / L"RemapconUpdater.exe").c_str(), updater.c_str(), TRUE)) {
        RemoveDirectoryW(temporary.c_str());
        return false;
    }
    const std::wstring parameters = std::to_wstring(GetCurrentProcessId()) + L" " +
        std::to_wstring(reinterpret_cast<uintptr_t>(mainWindow)) + L" \"" +
        install.wstring() + L"\" \"" + temporary.wstring() + L"\" " + tag;
    SHELLEXECUTEINFOW launch{sizeof(launch)};
    launch.fMask = SEE_MASK_NOCLOSEPROCESS;
    launch.hwnd = mainWindow;
    launch.lpVerb = CanWriteDirectory(install) ? L"open" : L"runas";
    launch.lpFile = updater.c_str();
    launch.lpParameters = parameters.c_str();
    launch.lpDirectory = temporary.c_str();
    launch.nShow = SW_HIDE;
    if (!ShellExecuteExW(&launch)) {
        DeleteFileW(updater.c_str());
        RemoveDirectoryW(temporary.c_str());
        return false;
    }
    if (launch.hProcess) CloseHandle(launch.hProcess);
    updateInProgress = true;
    return true;
}

NOTIFYICONDATAW TrayIconData(HWND window) {
    NOTIFYICONDATAW data{};
    data.cbSize = sizeof(data);
    data.hWnd = window;
    data.uID = ID_TRAY_ICON;
    return data;
}

bool AddTrayIcon(HWND window) {
    if (trayIconAdded) return true;
    NOTIFYICONDATAW data = TrayIconData(window);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY_ICON;
    data.hIcon = static_cast<HICON>(LoadImageW(GetModuleHandleW(nullptr),
        MAKEINTRESOURCEW(IDI_REMAPCON), IMAGE_ICON,
        GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_SHARED));
    wcscpy_s(data.szTip, L"Remapcon — Steam Controller 2026 compatible");
    trayIconAdded = Shell_NotifyIconW(NIM_ADD, &data) != FALSE;
    return trayIconAdded;
}

void RemoveTrayIcon(HWND window) {
    if (!trayIconAdded) return;
    NOTIFYICONDATAW data = TrayIconData(window);
    Shell_NotifyIconW(NIM_DELETE, &data);
    trayIconAdded = false;
}

void RestoreFromTray(HWND window) {
    ShowWindow(window, IsIconic(window) ? SW_RESTORE : SW_SHOW);
    SetForegroundWindow(window);
    if (webReady) SendUiState();
}

void ShowAlreadyRunningNotice(HWND window) {
    if (!initialWindowShown || !webReady) return;
    RestoreFromTray(window);
    webUi.SendJson(L"{\"type\":\"alreadyRunning\"}");
    alreadyRunningNoticePending = false;
}

bool HideToTray(HWND window) {
    if (!AddTrayIcon(window)) {
        MessageBoxW(window,
            language == L"en" ? L"Could not add an icon to the notification area." :
                                L"通知領域にアイコンを作成できませんでした。",
            language == L"en" ? L"Could not minimize to tray" : L"トレイに格納できません",
            MB_ICONERROR);
        return false;
    }
    SaveWindowPlacement(window);
    previewMode = false;
    ShowWindow(window, SW_HIDE);
    if (webReady) SendUiState();
    return true;
}

void ForceExit(HWND window) {
    if (initialWindowShown) SaveWindowPlacement(window);
    RemoveTrayIcon(window);
    previewMode = false;
    requested = false;
    autoMode = false;
    running = false;
    if (controllerThread.joinable()) controllerThread.join();
    DestroyWindow(window);
}

void ShowTrayMenu(HWND window) {
    HMENU menu = CreatePopupMenu();
    if (!menu) return;
    AppendMenuW(menu, MF_STRING, ID_TRAY_SHOW,
                language == L"en" ? L"Open Remapcon" :
                                    L"Remapconを開く");
    AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, language == L"en" ? L"Exit" : L"終了");
    POINT cursor{};
    GetCursorPos(&cursor);
    SetForegroundWindow(window);
    const UINT command = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                                        cursor.x, cursor.y, 0, window, nullptr);
    DestroyMenu(menu);
    PostMessageW(window, WM_NULL, 0, 0);
    if (command == ID_TRAY_SHOW) RestoreFromTray(window);
    else if (command == ID_TRAY_EXIT) PostMessageW(window, WM_FORCE_EXIT, 0, 0);
}

LRESULT CALLBACK MainProc(HWND window, UINT message, WPARAM wParam, LPARAM lParam) {
    if (message == taskbarCreatedMessage) {
        trayIconAdded = false;
        AddTrayIcon(window);
        return 0;
    }
    switch (message) {
    case WM_CREATE: {
        const BOOL dark = TRUE;
        const COLORREF caption = RGB(36, 33, 45);
        const COLORREF captionText = RGB(240, 237, 245);
        DwmSetWindowAttribute(window, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark, sizeof(dark));
        DwmSetWindowAttribute(window, DWMWA_CAPTION_COLOR, &caption, sizeof(caption));
        DwmSetWindowAttribute(window, DWMWA_TEXT_COLOR, &captionText, sizeof(captionText));
        if (!webUi.Open(window, OnWebMessage)) {
            MessageBoxW(window, L"WebView2 Runtimeをインストールしてください。", L"画面を開けません", MB_ICONERROR);
            return -1;
        }
        return 0;
    }
    case WM_SIZE:
        if (wParam != SIZE_MINIMIZED) windowWasMaximized = wParam == SIZE_MAXIMIZED;
        webUi.Resize();
        if (webReady) webUi.SendJson(std::wstring(L"{\"type\":\"window\",\"maximized\":") +
                                     (IsZoomed(window) ? L"true}" : L"false}"));
        return 0;
    case WM_GETMINMAXINFO: {
        const UINT dpi = GetDpiForWindow(window);
        auto* limits = reinterpret_cast<MINMAXINFO*>(lParam);
        limits->ptMinTrackSize.x = MulDiv(800, dpi ? dpi : 96, 96);
        limits->ptMinTrackSize.y = MulDiv(560, dpi ? dpi : 96, 96);
        return 0;
    }
    case WM_DPICHANGED: {
        const auto* rect = reinterpret_cast<const RECT*>(lParam);
        SetWindowPos(window, nullptr, rect->left, rect->top,
                     rect->right - rect->left, rect->bottom - rect->top,
                     SWP_NOZORDER | SWP_NOACTIVATE);
        return 0;
    }
    case WM_NCCALCSIZE: {
        if (!wParam || IsZoomed(window)) break;
        auto* params = reinterpret_cast<NCCALCSIZE_PARAMS*>(lParam);
        const LONG originalTop = params->rgrc[0].top;
        const LRESULT result = DefWindowProcW(window, message, wParam, lParam);
        if (result != 0) return result;
        params->rgrc[0].top = originalTop;
        return 0;
    }
    case WM_NCHITTEST: {
        const LRESULT hit = DefWindowProcW(window, message, wParam, lParam);
        if (IsZoomed(window)) return hit;
        const POINT point{GET_X_LPARAM(lParam), GET_Y_LPARAM(lParam)};
        RECT bounds{};
        GetWindowRect(window, &bounds);
        const UINT dpi = GetDpiForWindow(window);
        const int grip = GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) +
                         GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
        if (point.y < bounds.top + grip) {
            if (point.x < bounds.left + grip) return HTTOPLEFT;
            if (point.x >= bounds.right - grip) return HTTOPRIGHT;
            return HTTOP;
        }
        return hit;
    }
    case WM_ACTIVATE:
        if (LOWORD(wParam) == WA_INACTIVE) {
            PostMessageW(window, WM_CONTROLLER_INPUT, 0, 0);
            PostMessageW(window, WM_STICK_INPUT, 0, 0);
            PostMessageW(window, WM_STICK_INPUT, 1, 0);
        }
        return 0;
    case WM_CONTROLLER_STATUS:
        latestStatus = reinterpret_cast<const wchar_t*>(lParam);
        webUi.SendJson(L"{\"type\":\"status\",\"text\":" +
                       JsonString(reinterpret_cast<const wchar_t*>(lParam)) + L"}");
        return 0;
    case WM_REMAPCON_UPDATE_FAILED:
        updateInProgress = false;
        if (webReady) webUi.SendJson(L"{\"type\":\"updateInstallError\",\"code\":" +
                                   std::to_wstring(wParam) + L"}");
        return 0;
    case WM_REMAPCON_UPDATE_READY:
        if (!updateInProgress) return 0;
        ForceExit(window);
        return 1;
    case WM_CONTROLLER_INPUT:
        if (webReady) {
            std::wstring json = L"{\"type\":\"input\",\"buttons\":[";
            bool first = true;
            for (size_t index = 0; index < ButtonCount; ++index) {
                if (!(wParam & (static_cast<WPARAM>(1) << index))) continue;
                if (!first) json += L",";
                first = false;
                json += std::to_wstring(index);
            }
            webUi.SendJson(json + L"]}");
        }
        return 0;
    case WM_STICK_INPUT:
        if (webReady) {
            const uint32_t packed = static_cast<uint32_t>(lParam);
            const int16_t x = static_cast<int16_t>(packed & 0xffff);
            const int16_t y = static_cast<int16_t>(packed >> 16);
            webUi.SendJson(L"{\"type\":\"stickInput\",\"side\":" + std::to_wstring(wParam) +
                L",\"x\":" + std::to_wstring(x) + L",\"y\":" + std::to_wstring(y) + L"}");
        }
        return 0;
    case WM_TRAY_ICON:
        if (lParam == WM_LBUTTONDBLCLK) RestoreFromTray(window);
        else if (lParam == WM_RBUTTONUP || lParam == WM_CONTEXTMENU) ShowTrayMenu(window);
        return 0;
    case WM_FORCE_EXIT:
        ForceExit(window);
        return 0;
    case WM_ALREADY_RUNNING:
        alreadyRunningNoticePending = true;
        ShowAlreadyRunningNotice(window);
        return 1;
    case WM_UI_READY:
        KillTimer(window, ID_STARTUP_TIMER);
        if (!trayIconAdded) AddTrayIcon(window);
        if (!initialWindowShown) {
            initialWindowShown = true;
            ShowWindow(window, startupShow);
            UpdateWindow(window);
        }
        if (alreadyRunningNoticePending) ShowAlreadyRunningNotice(window);
        return 0;
    case WM_TIMER:
        if (wParam == ID_STARTUP_TIMER && !initialWindowShown) {
            KillTimer(window, ID_STARTUP_TIMER);
            if (!trayIconAdded) AddTrayIcon(window);
            initialWindowShown = true;
            ShowWindow(window, startupShow);
            UpdateWindow(window);
            return 0;
        }
        break;
    case WM_QUERYENDSESSION:
        return TRUE;
    case WM_ENDSESSION:
        if (wParam) ForceExit(window);
        return 0;
    case WM_CLOSE:
        if (initialWindowShown && IsWindowVisible(window)) {
            if (closeBehavior == 0 && webReady) {
                webUi.SendJson(L"{\"type\":\"closePrompt\"}");
                return 0;
            }
            if (closeBehavior == 1 && HideToTray(window)) return 0;
            if (closeBehavior == 1) return 0;
        }
        ForceExit(window);
        return 0;
    case WM_DESTROY:
        RemoveTrayIcon(window);
        webReady = false;
        webUi.Close();
        mainWindow = nullptr;
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(window, message, wParam, lParam);
}

}  // namespace

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR, int show) {
    const int deviceCycleResult = RunDeviceCycleCommand();
    if (deviceCycleResult >= 0) return deviceCycleResult;
    processElevated = IsProcessElevated();
    HANDLE singleInstance = CreateMutexW(nullptr, TRUE, L"Remapcon_SingleInstance");
    if (!singleInstance || GetLastError() == ERROR_ALREADY_EXISTS) {
        HWND existing = nullptr;
        for (int attempt = 0; attempt < 10 && !existing; ++attempt) {
            existing = FindWindowW(L"RemapconMainWindow", nullptr);
            if (!existing) Sleep(100);
        }
        DWORD_PTR accepted = 0;
        if (existing && SendMessageTimeoutW(existing, WM_ALREADY_RUNNING, 0, 0,
                SMTO_ABORTIFHUNG | SMTO_BLOCK, 1500, &accepted) && accepted == 1) {
            SetForegroundWindow(existing);
        } else if (existing) {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
        }
        if (singleInstance) CloseHandle(singleInstance);
        return 0;
    }
    const bool cycleRecovered = RecoverInterruptedDeviceCycle();
    if (!cycleRecovered) {
        MessageBoxW(nullptr,
            L"前回の切り替えでコントローラーデバイスが無効のまま残った可能性があります。"
            L"Windowsのデバイスマネージャーでコントローラーを有効にしてください。",
            L"コントローラーの復旧が必要です", MB_OK | MB_ICONERROR);
        CloseHandle(singleInstance);
        return 1;
    }
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    const HRESULT apartment = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    settingsPath = SettingsFile();
    if (!LoadSettings()) {
        MessageBoxW(nullptr, L"設定ファイルを読み込めませんでした。ファイルを修復するか、バックアップを復元してください。",
                    L"設定エラー", MB_OK | MB_ICONERROR);
        CoUninitialize();
        if (singleInstance) CloseHandle(singleInstance);
        return 1;
    }
    taskbarCreatedMessage = RegisterWindowMessageW(L"TaskbarCreated");

    WNDCLASSW bindingClass{};
    bindingClass.lpfnWndProc = BindingProc;
    bindingClass.hInstance = instance;
    bindingClass.lpszClassName = L"MapleStoryBinding";
    bindingClass.hCursor = LoadCursorW(nullptr, MAKEINTRESOURCEW(32512));
    RegisterClassW(&bindingClass);

    WNDCLASSW sequenceClass{};
    sequenceClass.lpfnWndProc = SequenceProc;
    sequenceClass.hInstance = instance;
    sequenceClass.lpszClassName = L"MapleStorySequence";
    sequenceClass.hCursor = LoadCursorW(nullptr, MAKEINTRESOURCEW(32512));
    sequenceClass.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
    RegisterClassW(&sequenceClass);

    WNDCLASSW nameClass{};
    nameClass.lpfnWndProc = NameProc;
    nameClass.hInstance = instance;
    nameClass.lpszClassName = L"MapleStoryPresetName";
    nameClass.hCursor = LoadCursorW(nullptr, MAKEINTRESOURCEW(32512));
    RegisterClassW(&nameClass);

    WNDCLASSEXW mainClass{};
    mainClass.cbSize = sizeof(mainClass);
    mainClass.lpfnWndProc = MainProc;
    mainClass.hInstance = instance;
    mainClass.lpszClassName = L"RemapconMainWindow";
    mainClass.hCursor = LoadCursorW(nullptr, MAKEINTRESOURCEW(32512));
    mainClass.hIcon = LoadIconW(instance, MAKEINTRESOURCEW(IDI_REMAPCON));
    mainClass.hIconSm = static_cast<HICON>(LoadImageW(instance,
        MAKEINTRESOURCEW(IDI_REMAPCON), IMAGE_ICON,
        GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_SHARED));
    const HBRUSH darkBackground = CreateSolidBrush(RGB(27, 25, 34));
    mainClass.hbrBackground = darkBackground;
    RegisterClassExW(&mainClass);

    POINT cursor{};
    GetCursorPos(&cursor);
    MONITORINFO startMonitor{sizeof(startMonitor)};
    GetMonitorInfoW(MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY), &startMonitor);
    const RECT& work = startMonitor.rcWork;
    mainWindow = CreateWindowW(L"RemapconMainWindow",
        L"Remapcon — Steam Controller 2026 compatible",
        WS_POPUP | WS_THICKFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX,
        work.left + 20, work.top + 20,
        (std::min)(1000L, work.right - work.left),
        (std::min)(700L, work.bottom - work.top),
        nullptr, nullptr, instance, nullptr);
    if (!mainWindow) {
        DeleteObject(darkBackground);
        if (SUCCEEDED(apartment)) CoUninitialize();
        CloseHandle(singleInstance);
        return 1;
    }
    ChangeWindowMessageFilterEx(mainWindow, WM_ALREADY_RUNNING, MSGFLT_ALLOW, nullptr);
    if (!RestoreWindowPlacement(mainWindow)) {
        SizeNewWindow(mainWindow, work);
        startupShow = show == SW_MAXIMIZE || show == SW_SHOWMAXIMIZED ? SW_MAXIMIZE : SW_SHOWNORMAL;
    }
    controllerThread = std::thread(ControllerLoop);
    AddTrayIcon(mainWindow);
    SetTimer(mainWindow, ID_STARTUP_TIMER, 1800, nullptr);

    MSG message{};
    while (GetMessageW(&message, nullptr, 0, 0) > 0) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    if (SUCCEEDED(apartment)) CoUninitialize();
    DeleteObject(darkBackground);
    CloseHandle(singleInstance);
    return static_cast<int>(message.wParam);
}
