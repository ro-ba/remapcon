#include "TargetIcon.h"

#include <Windows.h>
#include <TlHelp32.h>
#include <shellapi.h>
#include <wincodec.h>
#include <wrl/client.h>
#include <cstdint>
#include <cstring>
#include <cwctype>
#include <iterator>
#include <vector>

namespace {

constexpr unsigned char pngSignature[] = {0x89, 'P', 'N', 'G', 0x0d, 0x0a, 0x1a, 0x0a};

bool IsPng(const unsigned char* bytes, size_t length) {
    return length >= sizeof(pngSignature) &&
        std::memcmp(bytes, pngSignature, sizeof(pngSignature)) == 0;
}

std::wstring CachedIconPath(const std::wstring& target, const std::wstring& directory) {
    if (directory.empty()) return {};
    std::uint64_t hash = 14695981039346656037ull;
    for (wchar_t ch : target) {
        hash ^= static_cast<std::uint16_t>(std::towlower(ch));
        hash *= 1099511628211ull;
    }
    wchar_t filename[21]{};
    swprintf_s(filename, L"%016llx.png", static_cast<unsigned long long>(hash));
    return directory + L"\\" + filename;
}

bool ReadCachedPng(const std::wstring& path, std::vector<unsigned char>& bytes) {
    if (path.empty()) return false;
    HANDLE file = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    LARGE_INTEGER size{};
    bool okay = GetFileSizeEx(file, &size) && size.QuadPart >= 8 &&
                size.QuadPart <= 1024 * 1024;
    if (okay) {
        bytes.resize(static_cast<size_t>(size.QuadPart));
        DWORD read = 0;
        okay = ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read, nullptr) &&
               read == bytes.size() && IsPng(bytes.data(), bytes.size());
    }
    CloseHandle(file);
    return okay;
}

bool WriteCachedPng(const std::wstring& directory, const std::wstring& path,
                    const unsigned char* bytes, size_t length) {
    if (path.empty() || !IsPng(bytes, length)) return false;
    if (!CreateDirectoryW(directory.c_str(), nullptr) &&
        GetLastError() != ERROR_ALREADY_EXISTS) return false;
    const std::wstring temporary = path + L".tmp";
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr,
                              CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    DWORD written = 0;
    const bool okay = WriteFile(file, bytes, static_cast<DWORD>(length), &written, nullptr) &&
                      written == length && FlushFileBuffers(file);
    CloseHandle(file);
    const bool saved = okay && MoveFileExW(temporary.c_str(), path.c_str(),
                              MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH);
    if (!saved)
        DeleteFileW(temporary.c_str());
    return saved;
}

std::wstring FileFingerprint(const std::wstring& path) {
    WIN32_FILE_ATTRIBUTE_DATA attributes{};
    if (!GetFileAttributesExW(path.c_str(), GetFileExInfoStandard, &attributes)) return {};
    std::wstring result = path;
    for (wchar_t& ch : result) ch = static_cast<wchar_t>(std::towlower(ch));
    return result + L"\n" + std::to_wstring(attributes.ftLastWriteTime.dwHighDateTime) +
        L":" + std::to_wstring(attributes.ftLastWriteTime.dwLowDateTime) + L":" +
        std::to_wstring(attributes.nFileSizeHigh) + L":" +
        std::to_wstring(attributes.nFileSizeLow);
}

bool ReadFingerprint(const std::wstring& path, std::wstring& fingerprint) {
    if (path.empty()) return false;
    HANDLE file = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    LARGE_INTEGER size{};
    bool okay = GetFileSizeEx(file, &size) && size.QuadPart > 0 &&
        size.QuadPart <= 65536 && size.QuadPart % sizeof(wchar_t) == 0;
    if (okay) {
        fingerprint.resize(static_cast<size_t>(size.QuadPart) / sizeof(wchar_t));
        DWORD read = 0;
        okay = ReadFile(file, fingerprint.data(), static_cast<DWORD>(size.QuadPart), &read,
                        nullptr) && read == size.QuadPart;
    }
    CloseHandle(file);
    return okay;
}

void WriteFingerprint(const std::wstring& path, const std::wstring& fingerprint) {
    if (path.empty() || fingerprint.empty()) return;
    const std::wstring temporary = path + L".tmp";
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr,
                              CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return;
    const DWORD size = static_cast<DWORD>(fingerprint.size() * sizeof(wchar_t));
    DWORD written = 0;
    const bool okay = WriteFile(file, fingerprint.data(), size, &written, nullptr) &&
                      written == size && FlushFileBuffers(file);
    CloseHandle(file);
    if (!okay || !MoveFileExW(temporary.c_str(), path.c_str(),
                              MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH))
        DeleteFileW(temporary.c_str());
}

std::wstring ResolveExecutablePath(const std::wstring& target) {
    if (target.find_first_of(L"\\/") != std::wstring::npos)
        return GetFileAttributesW(target.c_str()) != INVALID_FILE_ATTRIBUTES ? target : L"";

    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snapshot != INVALID_HANDLE_VALUE) {
        PROCESSENTRY32W entry{};
        entry.dwSize = sizeof(entry);
        for (BOOL found = Process32FirstW(snapshot, &entry); found;
             found = Process32NextW(snapshot, &entry)) {
            if (_wcsicmp(entry.szExeFile, target.c_str()) != 0) continue;
            HANDLE process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE,
                                         entry.th32ProcessID);
            if (!process) continue;
            wchar_t path[32768]{};
            DWORD length = static_cast<DWORD>(std::size(path));
            const BOOL resolved = QueryFullProcessImageNameW(process, 0, path, &length);
            CloseHandle(process);
            if (resolved && GetFileAttributesW(path) != INVALID_FILE_ATTRIBUTES) {
                CloseHandle(snapshot);
                return path;
            }
        }
        CloseHandle(snapshot);
    }

    wchar_t path[MAX_PATH * 2]{};
    const DWORD length = SearchPathW(nullptr, target.c_str(), nullptr,
                                     static_cast<DWORD>(std::size(path)), path, nullptr);
    return length && length < std::size(path) ? std::wstring(path) : std::wstring();
}

std::wstring Base64Png(const unsigned char* bytes, size_t length) {
    constexpr wchar_t alphabet[] = L"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::wstring result = L"data:image/png;base64,";
    result.reserve(result.size() + ((length + 2) / 3) * 4);
    for (size_t offset = 0; offset < length; offset += 3) {
        const unsigned int value = (static_cast<unsigned int>(bytes[offset]) << 16) |
            (offset + 1 < length ? static_cast<unsigned int>(bytes[offset + 1]) << 8 : 0) |
            (offset + 2 < length ? bytes[offset + 2] : 0);
        result += alphabet[(value >> 18) & 63];
        result += alphabet[(value >> 12) & 63];
        result += offset + 1 < length ? alphabet[(value >> 6) & 63] : L'=';
        result += offset + 2 < length ? alphabet[value & 63] : L'=';
    }
    return result;
}

}  // namespace

std::wstring TargetIconDataUri(const std::wstring& path,
                               const std::wstring& cacheDirectory, bool refresh) {
    if (path.empty()) return {};
    const std::wstring cachedPath = CachedIconPath(path, cacheDirectory);
    const std::wstring executable = ResolveExecutablePath(path);
    const std::wstring fingerprint = executable.empty() ? L"" : FileFingerprint(executable);
    std::vector<unsigned char> cachedBytes;
    const bool hasCache = ReadCachedPng(cachedPath, cachedBytes);
    std::wstring previousFingerprint;
    const auto cachedResult = [&]() -> std::wstring {
        return hasCache ? Base64Png(cachedBytes.data(), cachedBytes.size()) : L"";
    };
    if (!refresh && hasCache && (executable.empty() ||
        (!fingerprint.empty() && ReadFingerprint(cachedPath + L".meta", previousFingerprint) &&
         fingerprint == previousFingerprint)))
        return Base64Png(cachedBytes.data(), cachedBytes.size());
    if (executable.empty())
        return cachedResult();

    HICON largeIcon = nullptr;
    HICON smallIcon = nullptr;
    const UINT count = ExtractIconExW(executable.c_str(), 0, &largeIcon, &smallIcon, 1);
    if (smallIcon) DestroyIcon(smallIcon);
    if (count == 0 || count == static_cast<UINT>(-1) || !largeIcon) {
        if (largeIcon) DestroyIcon(largeIcon);
        SHFILEINFOW fileInfo{};
        if (!SHGetFileInfoW(executable.c_str(), 0, &fileInfo, sizeof(fileInfo),
                            SHGFI_ICON | SHGFI_LARGEICON) || !fileInfo.hIcon) return cachedResult();
        largeIcon = fileInfo.hIcon;
    }

    using Microsoft::WRL::ComPtr;
    ComPtr<IWICImagingFactory> factory;
    ComPtr<IWICBitmap> bitmap;
    const HRESULT factoryResult = CoCreateInstance(CLSID_WICImagingFactory, nullptr,
        CLSCTX_INPROC_SERVER, IID_PPV_ARGS(factory.GetAddressOf()));
    const HRESULT bitmapResult = SUCCEEDED(factoryResult) ?
        factory->CreateBitmapFromHICON(largeIcon, bitmap.GetAddressOf()) : E_FAIL;
    DestroyIcon(largeIcon);
    if (FAILED(bitmapResult)) return cachedResult();

    ComPtr<IStream> stream;
    ComPtr<IWICBitmapEncoder> encoder;
    ComPtr<IWICBitmapFrameEncode> frame;
    ComPtr<IPropertyBag2> options;
    UINT width = 0, height = 0;
    WICPixelFormatGUID format = GUID_WICPixelFormat32bppBGRA;
    if (FAILED(CreateStreamOnHGlobal(nullptr, TRUE, stream.GetAddressOf())) ||
        FAILED(factory->CreateEncoder(GUID_ContainerFormatPng, nullptr, encoder.GetAddressOf())) ||
        FAILED(encoder->Initialize(stream.Get(), WICBitmapEncoderNoCache)) ||
        FAILED(encoder->CreateNewFrame(frame.GetAddressOf(), options.GetAddressOf())) ||
        FAILED(frame->Initialize(options.Get())) ||
        FAILED(bitmap->GetSize(&width, &height)) ||
        FAILED(frame->SetSize(width, height)) ||
        FAILED(frame->SetPixelFormat(&format)) ||
        FAILED(frame->WriteSource(bitmap.Get(), nullptr)) ||
        FAILED(frame->Commit()) || FAILED(encoder->Commit())) return cachedResult();

    STATSTG stats{};
    HGLOBAL memory = nullptr;
    if (FAILED(stream->Stat(&stats, STATFLAG_NONAME)) || stats.cbSize.QuadPart <= 0 ||
        stats.cbSize.QuadPart > 1024 * 1024 ||
        FAILED(GetHGlobalFromStream(stream.Get(), &memory)) || !memory) return cachedResult();
    const auto* bytes = static_cast<const unsigned char*>(GlobalLock(memory));
    if (!bytes) return cachedResult();
    const size_t length = static_cast<size_t>(stats.cbSize.QuadPart);
    if (WriteCachedPng(cacheDirectory, cachedPath, bytes, length))
        WriteFingerprint(cachedPath + L".meta", fingerprint);
    const std::wstring result = Base64Png(bytes, length);
    GlobalUnlock(memory);
    return result;
}
