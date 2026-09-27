#pragma once

#include <string>

// Returns an embedded PNG. A cache keeps the icon available when the app is closed.
std::wstring TargetIconDataUri(const std::wstring& path,
                               const std::wstring& cacheDirectory = {}, bool refresh = false);
