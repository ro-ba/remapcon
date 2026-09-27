#include <Windows.h>
#include <bcrypt.h>
#include <shellapi.h>
#include <winhttp.h>
#include <array>
#include <climits>
#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>
#include "SimpleJson.h"
#include "UpdateProtocol.h"

namespace {

namespace fs = std::filesystem;
constexpr size_t MaxReleaseJson = 1024 * 1024;
constexpr size_t MaxPackage = 100 * 1024 * 1024;

struct InternetHandle {
    HINTERNET value = nullptr;
    explicit InternetHandle(HINTERNET handle) : value(handle) {}
    ~InternetHandle() { if (value) WinHttpCloseHandle(value); }
    InternetHandle(const InternetHandle&) = delete;
    InternetHandle& operator=(const InternetHandle&) = delete;
};

bool Fetch(const std::wstring& host, const std::wstring& path, size_t limit,
           std::vector<unsigned char>& output) {
    InternetHandle session(WinHttpOpen(L"Remapcon-Updater/1.0",
        WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_NO_PROXY_NAME,
        WINHTTP_NO_PROXY_BYPASS, 0));
    if (!session.value || !WinHttpSetTimeouts(session.value, 5000, 5000, 15000, 15000))
        return false;
    InternetHandle connection(WinHttpConnect(session.value, host.c_str(),
                                              INTERNET_DEFAULT_HTTPS_PORT, 0));
    if (!connection.value) return false;
    InternetHandle request(WinHttpOpenRequest(connection.value, L"GET", path.c_str(),
        nullptr, WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES,
        WINHTTP_FLAG_SECURE));
    if (!request.value) return false;
    const wchar_t* headers = L"Accept: application/vnd.github+json\r\n";
    if (!WinHttpSendRequest(request.value, headers, static_cast<DWORD>(-1),
                            WINHTTP_NO_REQUEST_DATA, 0, 0, 0) ||
        !WinHttpReceiveResponse(request.value, nullptr)) return false;
    DWORD status = 0, length = sizeof(status);
    if (!WinHttpQueryHeaders(request.value,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        WINHTTP_HEADER_NAME_BY_INDEX, &status, &length, WINHTTP_NO_HEADER_INDEX) ||
        status != 200) return false;
    std::array<unsigned char, 64 * 1024> chunk{};
    for (;;) {
        DWORD read = 0;
        if (!WinHttpReadData(request.value, chunk.data(),
                             static_cast<DWORD>(chunk.size()), &read)) return false;
        if (read == 0) return true;
        if (read > limit || output.size() > limit - read) return false;
        output.insert(output.end(), chunk.data(), chunk.data() + read);
    }
}

bool Utf8ToWide(const std::vector<unsigned char>& bytes, std::wstring& output) {
    if (bytes.empty() || bytes.size() > INT_MAX) return false;
    const int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
        reinterpret_cast<const char*>(bytes.data()), static_cast<int>(bytes.size()),
        nullptr, 0);
    if (count <= 0) return false;
    output.resize(static_cast<size_t>(count));
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
        reinterpret_cast<const char*>(bytes.data()), static_cast<int>(bytes.size()),
        output.data(), count) == count;
}

bool ValidTag(const std::wstring& tag) {
    if (tag.size() < 7 || tag.size() > 32 || tag[0] != L'v') return false;
    int dots = 0;
    bool digit = false;
    for (size_t i = 1; i < tag.size(); ++i) {
        if (tag[i] == L'.') {
            if (!digit || ++dots > 2) return false;
            digit = false;
        } else if (tag[i] >= L'0' && tag[i] <= L'9') digit = true;
        else return false;
    }
    return dots == 2 && digit;
}

bool ReleaseDigest(const std::vector<unsigned char>& bytes,
                   const std::wstring& expectedTag, std::string& digest) {
    std::wstring text;
    if (!Utf8ToWide(bytes, text)) return false;
    simple_json::Value root;
    if (!simple_json::Parser(text).Parse(root)) return false;
    const auto* tag = root.Get(L"tag_name");
    const auto* assets = root.Get(L"assets");
    if (!tag || tag->type != simple_json::Value::Type::String ||
        tag->string != expectedTag || !assets ||
        assets->type != simple_json::Value::Type::Array) return false;
    for (const auto& asset : assets->array) {
        const auto* name = asset.Get(L"name");
        const auto* hash = asset.Get(L"digest");
        if (!name || name->type != simple_json::Value::Type::String ||
            name->string != L"Remapcon-Windows-x64.zip") continue;
        if (!hash || hash->type != simple_json::Value::Type::String ||
            hash->string.size() != 71 || hash->string.substr(0, 7) != L"sha256:")
            return false;
        digest.clear();
        for (wchar_t ch : hash->string.substr(7)) {
            if (!((ch >= L'0' && ch <= L'9') || (ch >= L'a' && ch <= L'f') ||
                  (ch >= L'A' && ch <= L'F'))) return false;
            digest.push_back(static_cast<char>(ch >= L'A' && ch <= L'F' ? ch + 32 : ch));
        }
        return true;
    }
    return false;
}

bool Sha256(const std::vector<unsigned char>& bytes, std::string& result) {
    BCRYPT_ALG_HANDLE algorithm = nullptr;
    BCRYPT_HASH_HANDLE hash = nullptr;
    bool success = false;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0)
        return false;
    DWORD objectLength = 0, size = 0;
    if (BCryptGetProperty(algorithm, BCRYPT_OBJECT_LENGTH,
        reinterpret_cast<PUCHAR>(&objectLength), sizeof(objectLength), &size, 0) >= 0) {
        std::vector<unsigned char> object(objectLength);
        std::array<unsigned char, 32> value{};
        if (BCryptCreateHash(algorithm, &hash, object.data(), objectLength,
                             nullptr, 0, 0) >= 0 &&
            BCryptHashData(hash, const_cast<PUCHAR>(bytes.data()),
                           static_cast<ULONG>(bytes.size()), 0) >= 0 &&
            BCryptFinishHash(hash, value.data(), static_cast<ULONG>(value.size()), 0) >= 0) {
            static constexpr char Hex[] = "0123456789abcdef";
            result.clear();
            for (unsigned char byte : value) {
                result.push_back(Hex[byte >> 4]);
                result.push_back(Hex[byte & 15]);
            }
            success = true;
        }
    }
    if (hash) BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    return success;
}

bool WriteFileBytes(const fs::path& path, const std::vector<unsigned char>& bytes) {
    HANDLE file = CreateFileW(path.c_str(), GENERIC_WRITE, 0, nullptr,
                              CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    DWORD written = 0;
    const bool okay = WriteFile(file, bytes.data(), static_cast<DWORD>(bytes.size()),
                                &written, nullptr) && written == bytes.size();
    CloseHandle(file);
    return okay;
}

bool Extract(const fs::path& archive, const fs::path& destination) {
    wchar_t system[MAX_PATH]{};
    if (!GetSystemDirectoryW(system, MAX_PATH)) return false;
    const fs::path tar = fs::path(system) / L"tar.exe";
    std::wstring command = L"\"" + tar.wstring() + L"\" -xf \"" +
        archive.wstring() + L"\" -C \"" + destination.wstring() + L"\"";
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    startup.dwFlags = STARTF_USESHOWWINDOW;
    startup.wShowWindow = SW_HIDE;
    if (!CreateProcessW(tar.c_str(), command.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, destination.c_str(),
                        &startup, &process)) return false;
    CloseHandle(process.hThread);
    const DWORD wait = WaitForSingleObject(process.hProcess, 30000);
    DWORD exitCode = 1;
    if (wait == WAIT_OBJECT_0) GetExitCodeProcess(process.hProcess, &exitCode);
    else TerminateProcess(process.hProcess, 1);
    CloseHandle(process.hProcess);
    return wait == WAIT_OBJECT_0 && exitCode == 0;
}

bool ApplyPackage(const fs::path& package, const fs::path& install,
                  const fs::path& temporary) {
    constexpr std::array<const wchar_t*, 8> Files = {
        L"Remapcon.exe", L"RemapconUpdater.exe", L"LICENSE", L"README.md",
        L"README.ja.md", L"THIRD_PARTY_NOTICES.md", L"LICENSES/WebView2-LICENSE.txt",
        L"LICENSES/WebView2-NOTICE.txt"
    };
    const fs::path backup = temporary / L"previous.exe";
    if (!CopyFileW((install / L"Remapcon.exe").c_str(), backup.c_str(), TRUE))
        return false;
    std::vector<fs::path> staged;
    for (const wchar_t* name : Files) {
        const fs::path source = package / name;
        const fs::path target = install / name;
        const fs::path next = target.wstring() + L".remapcon-new";
        std::error_code error;
        fs::create_directories(target.parent_path(), error);
        DeleteFileW(next.c_str());
        if (error || !fs::is_regular_file(source, error) || error ||
            !CopyFileW(source.c_str(), next.c_str(), FALSE)) {
            for (const auto& file : staged) DeleteFileW(file.c_str());
            return false;
        }
        staged.push_back(next);
    }
    for (size_t i = 0; i < staged.size(); ++i) {
        const fs::path target = install / Files[i];
        if (!MoveFileExW(staged[i].c_str(), target.c_str(), MOVEFILE_REPLACE_EXISTING)) {
            for (size_t j = i; j < staged.size(); ++j) DeleteFileW(staged[j].c_str());
            CopyFileW(backup.c_str(), (install / L"Remapcon.exe").c_str(), FALSE);
            return false;
        }
    }
    return true;
}

void NotifyFailure(HWND window, UpdateFailure reason) {
    if (IsWindow(window)) PostMessageW(window, WM_REMAPCON_UPDATE_FAILED,
                                       static_cast<WPARAM>(reason), 0);
}

} // namespace

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int) {
    int count = 0;
    LPWSTR* arguments = CommandLineToArgvW(GetCommandLineW(), &count);
    if (!arguments || count != 6) {
        if (arguments) LocalFree(arguments);
        return 1;
    }
    const DWORD parentId = wcstoul(arguments[1], nullptr, 10);
    const auto window = reinterpret_cast<HWND>(static_cast<uintptr_t>(
        _wcstoui64(arguments[2], nullptr, 10)));
    const fs::path install(arguments[3]);
    const fs::path temporary(arguments[4]);
    const std::wstring tag(arguments[5]);
    LocalFree(arguments);
    DWORD windowProcess = 0;
    GetWindowThreadProcessId(window, &windowProcess);
    if (!ValidTag(tag) || !parentId || windowProcess != parentId) return 1;
    HANDLE parent = OpenProcess(SYNCHRONIZE, FALSE, parentId);
    if (!parent) return 1;

    std::vector<unsigned char> release;
    if (!Fetch(L"api.github.com", L"/repos/ro-ba/remapcon/releases/latest",
               MaxReleaseJson, release)) {
        NotifyFailure(window, UpdateFailure::Network); CloseHandle(parent); return 1;
    }
    std::string expected;
    if (!ReleaseDigest(release, tag, expected)) {
        NotifyFailure(window, UpdateFailure::Release); CloseHandle(parent); return 1;
    }
    std::vector<unsigned char> package;
    if (!Fetch(L"github.com", L"/ro-ba/remapcon/releases/download/" + tag +
               L"/Remapcon-Windows-x64.zip", MaxPackage, package)) {
        NotifyFailure(window, UpdateFailure::Download); CloseHandle(parent); return 1;
    }
    std::string actual;
    if (!Sha256(package, actual) || actual != expected) {
        NotifyFailure(window, UpdateFailure::Checksum); CloseHandle(parent); return 1;
    }
    std::error_code error;
    fs::create_directories(temporary / L"extracted", error);
    const fs::path archive = temporary / L"release.zip";
    if (error || !WriteFileBytes(archive, package) ||
        !Extract(archive, temporary / L"extracted")) {
        NotifyFailure(window, UpdateFailure::Extract); CloseHandle(parent); return 1;
    }
    const fs::path extracted = temporary / L"extracted" / L"Remapcon-Windows-x64";
    if (!fs::is_regular_file(extracted / L"Remapcon.exe") ||
        !fs::is_regular_file(extracted / L"RemapconUpdater.exe")) {
        NotifyFailure(window, UpdateFailure::Extract); CloseHandle(parent); return 1;
    }
    DWORD_PTR acknowledged = 0;
    if (!SendMessageTimeoutW(window, WM_REMAPCON_UPDATE_READY, 0, 0,
        SMTO_ABORTIFHUNG | SMTO_BLOCK, 15000, &acknowledged) || acknowledged != 1 ||
        WaitForSingleObject(parent, 30000) != WAIT_OBJECT_0) {
        CloseHandle(parent); return 1;
    }
    CloseHandle(parent);
    if (!ApplyPackage(extracted, install, temporary)) {
        MessageBoxW(nullptr, L"更新ファイルを配置できませんでした。Remapcon.exeを手動で起動してください。",
            L"Remapconの更新に失敗", MB_OK | MB_ICONERROR);
        return 1;
    }
    const fs::path executable = install / L"Remapcon.exe";
    const auto result = ShellExecuteW(nullptr, L"open", executable.c_str(),
                                      nullptr, install.c_str(), SW_SHOWNORMAL);
    if (reinterpret_cast<INT_PTR>(result) <= 32) {
        MessageBoxW(nullptr, L"更新は完了しましたが、再起動できませんでした。Remapcon.exeを手動で起動してください。",
            L"Remapconを起動できません", MB_OK | MB_ICONWARNING);
        return 1;
    }
    return 0;
}
