// Included inside main.cpp's private namespace. The bridge never executes code from the page.
std::wstring JsonString(const std::wstring& value) {
    std::wstring result = L"\"";
    for (wchar_t ch : value) {
        switch (ch) {
        case L'"': result += L"\\\""; break;
        case L'\\': result += L"\\\\"; break;
        case L'\n': result += L"\\n"; break;
        case L'\r': result += L"\\r"; break;
        case L'\t': result += L"\\t"; break;
        default:
            if (ch < 32) {
                wchar_t escaped[7]{};
                swprintf_s(escaped, L"\\u%04x", static_cast<unsigned>(ch));
                result += escaped;
            } else result += ch;
        }
    }
    return result + L"\"";
}

std::wstring JsonBool(bool value) { return value ? L"true" : L"false"; }
std::wstring JsonTurbo(const TurboSettings& value) {
    return L"{\"enabled\":" + JsonBool(value.enabled) + L",\"intervalMs\":" +
        std::to_wstring(value.intervalMs) + L",\"delayMs\":" +
        std::to_wstring(value.delayMs) + L"}";
}
std::wstring JsonSequence(const SequenceSettings& value) {
    std::wstring json = L"{\"enabled\":" + JsonBool(value.enabled) + L",\"repeat\":" +
        JsonBool(value.repeat) + L",\"intervalMs\":" + std::to_wstring(value.intervalMs) +
        L",\"keys\":[";
    for (size_t index = 0; index < value.keys.size(); ++index) {
        if (index) json += L",";
        json += std::to_wstring(value.keys[index]);
    }
    return json + L"]}";
}
template <typename Mapping>
void AppendMappingJson(std::wstring& json, const Mapping& value) {
    json += L"\"mapping\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) json += L",";
        json += std::to_wstring(value.mapping[index]);
    }
    json += L"],\"turbo\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) json += L",";
        json += JsonTurbo(value.turbo[index]);
    }
    json += L"],\"sequence\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) json += L",";
        json += JsonSequence(value.sequence[index]);
    }
    json += L"]";
}
void SendUiState() {
    if (!webReady) return;
    std::wstring json = L"{\"type\":\"state\",\"presets\":[";
    for (size_t index = 0; index < presets.size(); ++index) {
        if (index) json += L",";
        const auto& preset = presets[index];
        json += L"{\"name\":" + JsonString(preset.name) + L",\"folder\":" +
            JsonString(preset.folder) + L",\"target\":" + JsonString(preset.targetExecutable) +
            L",\"leftPadMode\":" + std::to_wstring(preset.leftPadMode) +
            L",\"rightPadMode\":" + std::to_wstring(preset.rightPadMode) +
            L",\"leftPadSensitivity\":" + std::to_wstring(preset.leftPadSensitivity) +
            L",\"rightPadSensitivity\":" + std::to_wstring(preset.rightPadSensitivity) + L",";
        AppendMappingJson(json, preset);
        json += L",\"layers\":[";
        for (size_t layer = 0; layer < preset.layers.size(); ++layer) {
            if (layer) json += L",";
            const auto& entry = preset.layers[layer];
            json += L"{\"name\":" + JsonString(entry.name) + L",\"trigger\":" +
                std::to_wstring(entry.trigger) + L",";
            AppendMappingJson(json, entry);
            json += L"}";
        }
        json += L"]}";
    }
    json += L"],\"folders\":[";
    for (size_t index = 0; index < folders.size(); ++index) {
        if (index) json += L",";
        json += JsonString(folders[index]);
    }
    json += L"],\"selectedPreset\":" + std::to_wstring(selectedPreset) +
        L",\"editedLayer\":" + std::to_wstring(editedLayer) +
        L",\"autoMode\":" + JsonBool(autoMode.load()) +
        L",\"requested\":" + JsonBool(requested.load()) +
        L",\"preview\":" + JsonBool(previewMode.load()) +
        L",\"elevated\":" + JsonBool(processElevated) +
        L",\"language\":" + JsonString(language) +
        L",\"closeBehavior\":" + std::to_wstring(closeBehavior) +
        L",\"status\":" + JsonString(latestStatus) + L",\"buttons\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) json += L",";
        json += L"{\"name\":" + JsonString(BUTTONS[index].name) +
            L",\"setting\":" + JsonString(BUTTONS[index].setting) +
            L",\"category\":" + std::to_wstring(BUTTONS[index].category) + L"}";
    }
    json += L"],\"categories\":[";
    for (size_t index = 0; index < CATEGORIES.size(); ++index) {
        if (index) json += L",";
        json += JsonString(CATEGORIES[index]);
    }
    json += L"]}";
    webUi.SendJson(json);
}
void SendUiSelection() {
    webUi.SendJson(L"{\"type\":\"selection\",\"selectedPreset\":" +
                   std::to_wstring(selectedPreset) + L",\"editedLayer\":" +
                   std::to_wstring(editedLayer) + L",\"requested\":" +
                   JsonBool(requested.load()) + L"}");
}
void UiError(const std::wstring& value) {
    webUi.SendJson(L"{\"type\":\"error\",\"text\":" + JsonString(value) + L"}");
}
std::wstring DecodeField(const std::wstring& encoded) {
    std::string bytes;
    for (size_t index = 0; index < encoded.size(); ++index) {
        wchar_t ch = encoded[index];
        if (ch == L'%' && index + 2 < encoded.size()) {
            auto hex = [](wchar_t c) -> int {
                if (c >= L'0' && c <= L'9') return c - L'0';
                if (c >= L'A' && c <= L'F') return c - L'A' + 10;
                if (c >= L'a' && c <= L'f') return c - L'a' + 10;
                return -1;
            };
            const int high = hex(encoded[index + 1]);
            const int low = hex(encoded[index + 2]);
            if (high >= 0 && low >= 0) {
                bytes.push_back(static_cast<char>((high << 4) | low));
                index += 2;
                continue;
            }
        }
        if (ch <= 127) bytes.push_back(static_cast<char>(ch));
    }
    if (bytes.empty()) return L"";
    const int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes.data(),
                                         static_cast<int>(bytes.size()), nullptr, 0);
    if (count <= 0) return L"";
    std::wstring result(static_cast<size_t>(count), L'\0');
    MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes.data(),
                        static_cast<int>(bytes.size()), result.data(), count);
    return result;
}
std::vector<std::wstring> DecodeMessage(const std::wstring& message) {
    std::vector<std::wstring> parts;
    size_t start = 0;
    while (start <= message.size()) {
        const size_t end = message.find(L'\t', start);
        parts.push_back(DecodeField(message.substr(start, end == std::wstring::npos ?
                                                    end : end - start)));
        if (end == std::wstring::npos) break;
        start = end + 1;
    }
    return parts;
}
bool ParseNumber(const std::wstring& value, uint32_t& output) {
    if (value.empty() || value[0] == L'-') return false;
    wchar_t* end = nullptr;
    const unsigned long parsed = std::wcstoul(value.c_str(), &end, 10);
    if (!end || *end || parsed > (std::numeric_limits<uint32_t>::max)()) return false;
    output = static_cast<uint32_t>(parsed);
    return true;
}
bool ValidName(const std::wstring& name) {
    return !name.empty() && name.size() <= 40 &&
        name.find_first_of(L"/\\\r\n[]=") == std::wstring::npos;
}
bool FolderExists(const std::wstring& path) {
    return path.empty() || std::find(folders.begin(), folders.end(), path) != folders.end();
}
bool NameAvailable(const std::wstring& name, const std::wstring& folder, size_t except = SIZE_MAX) {
    for (size_t index = 0; index < presets.size(); ++index)
        if (index != except && presets[index].folder == folder &&
            _wcsicmp(presets[index].name.c_str(), name.c_str()) == 0) return false;
    return true;
}
void OnWebMessage(const std::wstring& raw) {
    const auto args = DecodeMessage(raw);
    if (args.empty()) return;
    auto arg = [&](size_t index) -> std::wstring { return index < args.size() ? args[index] : L""; };
    auto number = [&](size_t index, uint32_t& value) -> bool {
        return index < args.size() && ParseNumber(args[index], value);
    };
    const std::wstring& command = args[0];
    if (command == L"ready") {
        webReady = true; SendUiState();
        webUi.SendJson(std::wstring(L"{\"type\":\"window\",\"maximized\":") +
                       (IsZoomed(mainWindow) ? L"true}" : L"false}"));
        return;
    }
    if (command == L"uiReady") {
        PostMessageW(mainWindow, WM_UI_READY, 0, 0);
        return;
    }
    if (command == L"windowDrag") {
        ReleaseCapture();
        PostMessageW(mainWindow, WM_SYSCOMMAND, SC_MOVE | HTCAPTION, 0);
        return;
    }
    if (command == L"windowMinimize") { ShowWindow(mainWindow, SW_MINIMIZE); return; }
    if (command == L"windowToggleMaximize") {
        ShowWindow(mainWindow, IsZoomed(mainWindow) ? SW_RESTORE : SW_MAXIMIZE);
        return;
    }
    if (command == L"windowClose") { PostMessageW(mainWindow, WM_CLOSE, 0, 0); return; }
    if (command == L"closeToTray") { HideToTray(mainWindow); return; }
    if (command == L"quit") { PostMessageW(mainWindow, WM_FORCE_EXIT, 0, 0); return; }
    if (command == L"getTargetIcon") {
        const std::wstring path = arg(1);
        if (path.empty() || std::none_of(presets.begin(), presets.end(),
            [&](const Preset& preset) { return preset.targetExecutable == path; })) return;
        webUi.SendJson(L"{\"type\":\"targetIcon\",\"path\":" + JsonString(path) +
                       L",\"data\":" +
                       JsonString(TargetIconDataUri(path, TargetIconCacheDirectory())) + L"}");
        return;
    }
    bool okay = false;
    if (command == L"selectPreset") {
        uint32_t index = 0;
        okay = number(1, index) && SelectPreset(index);
        if (okay) editedLayer = 0;
    } else if (command == L"selectLayer") {
        uint32_t index = 0;
        okay = number(1, index) && index <= presets[selectedPreset].layers.size();
        if (okay) editedLayer = static_cast<int>(index);
    } else if (command == L"toggleMode") {
        okay = !autoMode && (requested || !presets[selectedPreset].targetExecutable.empty());
        if (okay) requested = !requested.load();
    } else if (command == L"setAuto") {
        const bool old = autoMode.load();
        autoMode = arg(1) == L"1";
        okay = SavePresets();
        if (!okay) autoMode = old;
        if (okay && autoMode) requested = false;
    } else if (command == L"setPreview") {
        previewMode = arg(1) == L"1";
        okay = true;
    } else if (command == L"setLanguage") {
        const auto old = language;
        language = arg(1) == L"en" ? L"en" : L"ja";
        okay = SavePresets();
        if (!okay) language = old;
    } else if (command == L"setCloseBehavior") {
        uint32_t value = 0;
        if (number(1, value) && value <= 2) {
            const auto old = closeBehavior;
            closeBehavior = value;
            okay = SavePresets();
            if (!okay) closeBehavior = old;
        }
    } else if (command == L"setPadConfig") {
        uint32_t leftMode = 0, rightMode = 0, leftSensitivity = 0, rightSensitivity = 0;
        if (number(1, leftMode) && number(2, rightMode) &&
            number(3, leftSensitivity) && number(4, rightSensitivity) &&
            leftMode <= 2 && rightMode <= 2 &&
            leftSensitivity >= 25 && leftSensitivity <= 400 &&
            rightSensitivity >= 25 && rightSensitivity <= 400) {
            const auto old = presets[selectedPreset];
            auto& preset = presets[selectedPreset];
            preset.leftPadMode = leftMode;
            preset.rightPadMode = rightMode;
            preset.leftPadSensitivity = leftSensitivity;
            preset.rightPadSensitivity = rightSensitivity;
            okay = SavePresets();
            if (!okay) preset = old;
            else PublishSelectedPreset();
        }
    } else if (command == L"setPadModes") {
        uint32_t left = 0, right = 0;
        if (number(1, left) && number(2, right) && left <= 2 && right <= 2) {
            const auto old = presets[selectedPreset];
            presets[selectedPreset].leftPadMode = left;
            presets[selectedPreset].rightPadMode = right;
            okay = SavePresets();
            if (!okay) presets[selectedPreset] = old;
            else PublishSelectedPreset();
        }
    } else if (command == L"setPadMode") {
        const auto old = presets[selectedPreset];
        uint32_t mode = 0;
        if ((arg(1) == L"left" || arg(1) == L"right") && number(2, mode) && mode <= 2) {
            if (arg(1) == L"left") presets[selectedPreset].leftPadMode = mode;
            else presets[selectedPreset].rightPadMode = mode;
            okay = SavePresets();
            if (!okay) presets[selectedPreset] = old;
            else PublishSelectedPreset();
        }
    } else if (command == L"pickTarget") {
        PickTargetExecutable(mainWindow);
        okay = true;
    } else if (command == L"clearTarget") {
        okay = SetTargetExecutable(L"");
        if (okay) requested = false;
    } else if (command == L"import") {
        ImportSettings(mainWindow);
        okay = true;
    } else if (command == L"export") {
        ExportSettings(mainWindow);
        okay = true;
    } else if (command == L"preset-new" || command == L"preset-copy" || command == L"preset-rename") {
        const auto name = arg(2), folder = arg(3);
        uint32_t source = 0;
        const bool needsSource = command != L"preset-new";
        if (ValidName(name) && FolderExists(folder) &&
            (!needsSource || (number(1, source) && source < presets.size())) &&
            (command == L"preset-rename" || presets.size() < 100) &&
            NameAvailable(name, folder, command == L"preset-rename" ? source : SIZE_MAX)) {
            auto oldPresets = presets;
            const size_t oldSelected = selectedPreset;
            if (command == L"preset-rename") {
                presets[source].name = name;
                presets[source].folder = folder;
            } else {
                Preset preset;
                if (command == L"preset-copy") preset = presets[source];
                preset.name = name;
                preset.folder = folder;
                presets.push_back(std::move(preset));
                selectedPreset = presets.size() - 1;
                editedLayer = 0;
            }
            okay = SavePresets();
            if (!okay) { presets = std::move(oldPresets); selectedPreset = oldSelected; }
            else PublishSelectedPreset();
        }
    } else if (command == L"deletePreset") {
        uint32_t index = 0;
        if (number(1, index) && index < presets.size() && presets.size() > 1) {
            const auto oldPresets = presets;
            const size_t oldSelected = selectedPreset;
            presets.erase(presets.begin() + static_cast<std::ptrdiff_t>(index));
            if (selectedPreset > index) --selectedPreset;
            else if (selectedPreset >= presets.size()) selectedPreset = presets.size() - 1;
            editedLayer = 0;
            okay = SavePresets();
            if (!okay) { presets = oldPresets; selectedPreset = oldSelected; }
            else PublishSelectedPreset();
        }
    } else if (command == L"movePreset" || command == L"movePresetNewFolder") {
        uint32_t index = 0;
        if (number(1, index) && index < presets.size()) {
            auto oldPresets = presets;
            auto oldFolders = folders;
            std::wstring destination;
            if (command == L"movePreset") destination = arg(2);
            else {
                const auto parent = arg(2), name = arg(3);
                if (ValidName(name) && FolderExists(parent) && folders.size() < 100) {
                    destination = parent.empty() ? name : parent + L"/" + name;
                    if (!FolderExists(destination)) folders.push_back(destination);
                    else destination.clear();
                }
            }
            if (FolderExists(destination) &&
                (command == L"movePreset" || !destination.empty()) &&
                NameAvailable(presets[index].name, destination, index)) {
                presets[index].folder = destination;
                okay = SavePresets();
            }
            if (!okay) { presets = std::move(oldPresets); folders = std::move(oldFolders); }
        }
    } else if (command == L"folder-new" || command == L"folder-rename") {
        const auto target = arg(1), name = arg(2);
        const auto path = target.empty() ? name :
            (command == L"folder-new" ? target + L"/" + name :
             target.substr(0, target.find_last_of(L'/') == std::wstring::npos ? 0 :
                               target.find_last_of(L'/') + 1) + name);
        const bool parentValid = command == L"folder-new" ? FolderExists(target) : FolderExists(target);
        if (ValidName(name) && parentValid && !FolderExists(path) && folders.size() < 100) {
            const auto oldFolders = folders;
            const auto oldPresets = presets;
            if (command == L"folder-new") folders.push_back(path);
            else {
                for (auto& folder : folders) if (folder == target || folder.starts_with(target + L"/"))
                    folder = path + folder.substr(target.size());
                for (auto& preset : presets) if (preset.folder == target ||
                    preset.folder.starts_with(target + L"/"))
                    preset.folder = path + preset.folder.substr(target.size());
            }
            okay = SavePresets();
            if (!okay) { folders = oldFolders; presets = oldPresets; }
        }
    } else if (command == L"deleteFolder") {
        const auto target = arg(1);
        if (FolderExists(target) && !target.empty()) {
            bool nonempty = false;
            for (const auto& folder : folders) if (folder.starts_with(target + L"/")) nonempty = true;
            for (const auto& preset : presets) if (preset.folder == target) nonempty = true;
            if (!nonempty) {
                const auto old = folders;
                folders.erase(std::remove(folders.begin(), folders.end(), target), folders.end());
                okay = SavePresets();
                if (!okay) folders = old;
            }
        }
    } else if (command == L"layer-new" || command == L"layer-rename" ||
               command == L"layer-delete") {
        const auto name = arg(2);
        uint32_t index = 0;
        if ((command == L"layer-delete" || ValidName(name)) &&
            (command == L"layer-new" || number(1, index))) {
            Preset old = presets[selectedPreset];
            const int oldEdited = editedLayer;
            auto& layers = presets[selectedPreset].layers;
            if (command == L"layer-new" && layers.size() < ButtonCount) {
                Layer entry; entry.name = name; layers.push_back(entry);
                editedLayer = static_cast<int>(layers.size());
                okay = true;
            } else if (command == L"layer-rename" && index < layers.size()) {
                layers[index].name = name; okay = true;
            } else if (command == L"layer-delete" && index < layers.size()) {
                layers.erase(layers.begin() + static_cast<std::ptrdiff_t>(index));
                if (editedLayer == static_cast<int>(index + 1)) editedLayer = 0;
                else if (editedLayer > static_cast<int>(index + 1)) --editedLayer;
                okay = true;
            }
            if (okay) okay = SavePresets();
            if (!okay) { presets[selectedPreset] = std::move(old); editedLayer = oldEdited; }
            else PublishSelectedPreset();
        }
    } else if (command == L"resetBinding") {
        uint32_t button = 0, layer = 0, preset = 0;
        if (number(1, button) && number(2, layer) && number(3, preset) &&
            preset == selectedPreset && button < ButtonCount &&
            layer <= presets[selectedPreset].layers.size() &&
            layer == static_cast<uint32_t>(editedLayer)) {
            if (layer == 0) {
                bool trigger = false;
                for (const auto& entry : presets[selectedPreset].layers)
                    if (entry.trigger == button) trigger = true;
                okay = trigger ? SetButtonRole(button, false, 0, 0, {}, {}) :
                                 SetBinding(button, 0, {}, {});
            } else okay = SetBinding(button, INHERIT, {}, {});
        }
    } else if (command == L"saveBinding") {
        uint32_t button = 0, layer = 0, key = 0, interval = 0, delay = 0, seqInterval = 0,
                 target = 0;
        bool valid = number(1, button) && number(2, layer) && number(4, key) &&
            number(6, interval) && number(7, delay) && number(9, seqInterval) &&
            number(11, target) && button < ButtonCount &&
            layer <= presets[selectedPreset].layers.size() &&
            interval >= 20 && interval <= 2000 && delay <= 5000 &&
            seqInterval >= 20 && seqInterval <= 2000;
        TurboSettings turbo{arg(5) == L"1", interval, delay};
        SequenceSettings sequence;
        sequence.enabled = arg(3) == L"sequence";
        sequence.intervalMs = seqInterval;
        sequence.repeat = arg(10) == L"1";
        if (sequence.enabled) {
            const auto keys = arg(8);
            size_t start = 0;
            while (start <= keys.size() && sequence.keys.size() < 16) {
                const size_t end = keys.find(L',', start);
                uint32_t item = 0;
                if (!ParseNumber(keys.substr(start, end == std::wstring::npos ? end : end - start), item) ||
                    !item || item == INHERIT) { valid = false; break; }
                sequence.keys.push_back(item);
                if (end == std::wstring::npos) break;
                start = end + 1;
            }
            valid = valid && !sequence.keys.empty();
        }
        if (valid) {
            editedLayer = static_cast<int>(layer);
            const auto role = arg(3);
            if (role == L"off") key = 0;
            if (role == L"inherit" && layer > 0) key = INHERIT;
            if (role == L"sequence") key = 0;
            if (role == L"layer" && layer == 0) {
                okay = SetButtonRole(button, true, target, 0, {}, {});
            } else if (role == L"key" || role == L"off" || role == L"inherit" ||
                       role == L"sequence") {
                if (layer == 0) {
                    bool trigger = false;
                    for (const auto& entry : presets[selectedPreset].layers)
                        if (entry.trigger == button) trigger = true;
                    okay = trigger ? SetButtonRole(button, false, 0, key, turbo, sequence) :
                                     SetBinding(button, key, turbo, sequence);
                } else okay = SetBinding(button, key, turbo, sequence);
            }
            if (editedLayer > static_cast<int>(presets[selectedPreset].layers.size())) editedLayer = 0;
        }
    }
    if (command == L"selectPreset" || command == L"selectLayer") {
        if (okay) SendUiSelection();
        else {
            UiError(L"選択を変更できませんでした。");
            SendUiState();
        }
        return;
    }
    if (!okay) UiError(L"設定を保存できませんでした。入力内容を確認してください。");
    SendUiState();
}
