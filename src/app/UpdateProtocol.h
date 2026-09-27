#pragma once

#include <Windows.h>

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
