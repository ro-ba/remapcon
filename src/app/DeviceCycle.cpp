#define WIN32_LEAN_AND_MEAN
#include <Windows.h>
#include <SetupAPI.h>
#include <cfgmgr32.h>
#include <hidsdi.h>
#include <hidpi.h>
#include <shellapi.h>
#include <string>
#include <vector>
#include <cstdlib>

namespace {

constexpr wchar_t PendingKey[] = L"SOFTWARE\\Remapcon\\PendingDeviceCycle";

bool ChangeState(HDEVINFO devices, SP_DEVINFO_DATA& device, DWORD change, DWORD scope) {
    SP_PROPCHANGE_PARAMS parameters{};
    parameters.ClassInstallHeader.cbSize = sizeof(SP_CLASSINSTALL_HEADER);
    parameters.ClassInstallHeader.InstallFunction = DIF_PROPERTYCHANGE;
    parameters.StateChange = change;
    parameters.Scope = scope;
    return SetupDiSetClassInstallParamsW(devices, &device, &parameters.ClassInstallHeader,
                                        sizeof(parameters)) &&
           SetupDiCallClassInstaller(DIF_PROPERTYCHANGE, devices, &device);
}

bool IsDisabled(const std::wstring& id) {
    if (id.empty()) return false;
    DEVINST node = 0;
    if (CM_Locate_DevNodeW(&node, const_cast<DEVINSTID_W>(id.c_str()),
                           CM_LOCATE_DEVNODE_PHANTOM) != CR_SUCCESS) return false;
    ULONG status = 0, problem = 0;
    return CM_Get_DevNode_Status(&status, &problem, node, 0) == CR_SUCCESS &&
           (status & DN_HAS_PROBLEM) && problem == CM_PROB_DISABLED;
}

bool IsEnabled(const std::wstring& id) {
    if (id.empty()) return true;
    DEVINST node = 0;
    if (CM_Locate_DevNodeW(&node, const_cast<DEVINSTID_W>(id.c_str()),
                           CM_LOCATE_DEVNODE_NORMAL) != CR_SUCCESS) return false;
    ULONG status = 0, problem = 0;
    return CM_Get_DevNode_Status(&status, &problem, node, 0) == CR_SUCCESS &&
           (status & DN_HAS_PROBLEM) == 0;
}

std::wstring IdOf(DEVINST node) {
    wchar_t id[MAX_DEVICE_ID_LEN]{};
    if (CM_Get_Device_IDW(node, id, MAX_DEVICE_ID_LEN, 0) != CR_SUCCESS) return {};
    return id;
}

bool SavePending(const std::wstring& child, const std::wstring& parent) {
    HKEY key = nullptr;
    if (RegCreateKeyExW(HKEY_LOCAL_MACHINE, PendingKey, 0, nullptr, 0,
                        KEY_SET_VALUE, nullptr, &key, nullptr) != ERROR_SUCCESS) return false;
    const auto write = [key](const wchar_t* name, const std::wstring& value) {
        return RegSetValueExW(key, name, 0, REG_SZ,
            reinterpret_cast<const BYTE*>(value.c_str()),
            static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t))) == ERROR_SUCCESS;
    };
    const bool ok = write(L"Child", child) && write(L"Parent", parent) &&
                    RegFlushKey(key) == ERROR_SUCCESS;
    RegCloseKey(key);
    return ok;
}

std::wstring ReadPending(HKEY key, const wchar_t* name) {
    DWORD type = 0, bytes = 0;
    if (RegQueryValueExW(key, name, nullptr, &type, nullptr, &bytes) != ERROR_SUCCESS ||
        type != REG_SZ || bytes < sizeof(wchar_t) || bytes > 4096) return {};
    std::wstring value(bytes / sizeof(wchar_t), L'\0');
    if (RegQueryValueExW(key, name, nullptr, &type,
                         reinterpret_cast<BYTE*>(value.data()), &bytes) != ERROR_SUCCESS) return {};
    value.resize(wcsnlen_s(value.c_str(), value.size()));
    return value;
}

bool EnableNode(const std::wstring& id) {
    if (IsEnabled(id)) return true;
    HDEVINFO devices = SetupDiCreateDeviceInfoList(nullptr, nullptr);
    if (devices == INVALID_HANDLE_VALUE) return false;
    SP_DEVINFO_DATA data{};
    data.cbSize = sizeof(data);
    bool opened = SetupDiOpenDeviceInfoW(devices, id.c_str(), nullptr, 0, &data) != 0;
    for (int attempt = 0; opened && attempt < 3 && !IsEnabled(id); ++attempt) {
        ChangeState(devices, data, DICS_ENABLE, DICS_FLAG_GLOBAL);
        if (!IsEnabled(id)) ChangeState(devices, data, DICS_ENABLE, DICS_FLAG_CONFIGSPECIFIC);
        if (!IsEnabled(id)) Sleep(250);
    }
    SetupDiDestroyDeviceInfoList(devices);
    return opened && IsEnabled(id);
}

bool RecoverPending() {
    HKEY key = nullptr;
    const LONG opened = RegOpenKeyExW(HKEY_LOCAL_MACHINE, PendingKey, 0, KEY_QUERY_VALUE, &key);
    if (opened == ERROR_FILE_NOT_FOUND) return true;
    if (opened != ERROR_SUCCESS) return false;
    const std::wstring parent = ReadPending(key, L"Parent");
    const std::wstring child = ReadPending(key, L"Child");
    RegCloseKey(key);
    if (child.empty()) return false;
    const bool restored = EnableNode(parent) && EnableNode(child);
    if (restored) RegDeleteTreeW(HKEY_LOCAL_MACHINE, PendingKey);
    return restored;
}

bool IsControllerInterface(const std::wstring& path) {
    HANDLE handle = CreateFileW(path.c_str(), 0, FILE_SHARE_READ | FILE_SHARE_WRITE,
                                nullptr, OPEN_EXISTING, 0, nullptr);
    if (handle == INVALID_HANDLE_VALUE) return false;
    HIDD_ATTRIBUTES attributes{};
    attributes.Size = sizeof(attributes);
    bool valid = HidD_GetAttributes(handle, &attributes) && attributes.VendorID == 0x28DE &&
        (attributes.ProductID == 0x1302 || attributes.ProductID == 0x1303 ||
         attributes.ProductID == 0x1304 || attributes.ProductID == 0x1305);
    PHIDP_PREPARSED_DATA preparsed = nullptr;
    if (valid && HidD_GetPreparsedData(handle, &preparsed)) {
        HIDP_CAPS caps{};
        valid = HidP_GetCaps(preparsed, &caps) == HIDP_STATUS_SUCCESS &&
                caps.UsagePage == 0xFF00 && caps.Usage == 1;
        HidD_FreePreparsedData(preparsed);
    } else valid = false;
    CloseHandle(handle);
    return valid;
}

int Cycle(const std::wstring& path, HANDLE downEvent) {
    if (!RecoverPending() || !IsControllerInterface(path)) return 2;
    HDEVINFO devices = SetupDiCreateDeviceInfoList(nullptr, nullptr);
    if (devices == INVALID_HANDLE_VALUE) return 3;
    SP_DEVICE_INTERFACE_DATA interfaceData{};
    interfaceData.cbSize = sizeof(interfaceData);
    SP_DEVINFO_DATA data{};
    data.cbSize = sizeof(data);
    const bool opened = SetupDiOpenDeviceInterfaceW(devices, path.c_str(), 0, &interfaceData) &&
        (SetupDiGetDeviceInterfaceDetailW(devices, &interfaceData, nullptr, 0, nullptr, &data) ||
         GetLastError() == ERROR_INSUFFICIENT_BUFFER) && data.DevInst != 0;
    if (!opened) {
        SetupDiDestroyDeviceInfoList(devices);
        return 3;
    }
    const std::wstring child = IdOf(data.DevInst);
    DEVINST parentNode = 0;
    const std::wstring parent = CM_Get_Parent(&parentNode, data.DevInst, 0) == CR_SUCCESS
        ? IdOf(parentNode) : std::wstring();
    if (child.empty() || !SavePending(child, parent)) {
        SetupDiDestroyDeviceInfoList(devices);
        return 4;
    }
    if (!ChangeState(devices, data, DICS_DISABLE, DICS_FLAG_GLOBAL))
        ChangeState(devices, data, DICS_DISABLE, DICS_FLAG_CONFIGSPECIFIC);
    bool down = false;
    for (int i = 0; i < 20 && !down; ++i) {
        down = IsDisabled(child) || IsDisabled(parent);
        if (!down) Sleep(50);
    }
    SetupDiDestroyDeviceInfoList(devices);
    if (down && downEvent) SetEvent(downEvent);
    if (down) Sleep(500);
    const bool recovered = RecoverPending();
    return !recovered ? 5 : down ? 0 : 6;
}

} // namespace

int RunDeviceCycleCommand() {
    int count = 0;
    LPWSTR* args = CommandLineToArgvW(GetCommandLineW(), &count);
    if (!args) return 1;
    const bool helper = count >= 2 &&
        (wcscmp(args[1], L"--recover-device-cycle") == 0 ||
         wcscmp(args[1], L"--device-cycle") == 0);
    if (!helper) {
        LocalFree(args);
        return -1;
    }
    HANDLE mutex = CreateMutexW(nullptr, FALSE, L"Local\\RemapconDeviceCycle");
    if (!mutex) {
        LocalFree(args);
        return 7;
    }
    const DWORD lock = WaitForSingleObject(mutex, 15000);
    if (lock != WAIT_OBJECT_0 && lock != WAIT_ABANDONED) {
        CloseHandle(mutex);
        LocalFree(args);
        return 7;
    }
    int result = 1;
    if (count == 2 && wcscmp(args[1], L"--recover-device-cycle") == 0) {
        result = RecoverPending() ? 0 : 5;
    } else if (count == 4 && wcscmp(args[1], L"--device-cycle") == 0) {
        wchar_t* end = nullptr;
        const unsigned long long value = wcstoull(args[3], &end, 10);
        if (end && !*end && value != 0)
            result = Cycle(args[2], reinterpret_cast<HANDLE>(static_cast<ULONG_PTR>(value)));
        else result = 1;
    }
    ReleaseMutex(mutex);
    CloseHandle(mutex);
    LocalFree(args);
    return result;
}
