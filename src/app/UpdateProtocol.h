#pragma once

#include <Windows.h>
#include <string>

inline bool ValidUpdateTag(const std::wstring& tag) {
    if (tag.size() < 6 || tag.size() > 32 || tag[0] != L'v') return false;
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

constexpr UINT WM_REMAPCON_UPDATE_READY = WM_APP + 7;
constexpr UINT WM_REMAPCON_UPDATE_FAILED = WM_APP + 8;

enum class UpdateFailure : WPARAM {
    Network = 1,
    Release = 2,
    Download = 3,
    Checksum = 4,
    Extract = 5,
    Restart = 6,
};
