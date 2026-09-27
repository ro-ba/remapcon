// Included inside main.cpp's private namespace.
std::wstring ConfigQuote(const std::wstring& input) {
    std::wstring output = L"\"";
    for (wchar_t ch : input) {
        if (ch == L'"' || ch == L'\\') { output += L'\\'; output += ch; }
        else if (ch == L'\n') output += L"\\n";
        else if (ch == L'\r') output += L"\\r";
        else if (ch == L'\t') output += L"\\t";
        else if (ch < 32) {
            wchar_t escaped[7]{};
            swprintf_s(escaped, L"\\u%04x", static_cast<unsigned>(ch));
            output += escaped;
        } else output += ch;
    }
    return output + L'"';
}
std::wstring ConfigBool(bool value) { return value ? L"true" : L"false"; }
std::wstring ConfigTurbo(const TurboSettings& value) {
    return L"{\"enabled\":" + ConfigBool(value.enabled) + L",\"intervalMs\":" +
        std::to_wstring(value.intervalMs) + L",\"delayMs\":" +
        std::to_wstring(value.delayMs) + L"}";
}
std::wstring ConfigSequence(const SequenceSettings& value) {
    std::wstring output = L"{\"enabled\":" + ConfigBool(value.enabled) +
        L",\"repeat\":" + ConfigBool(value.repeat) + L",\"intervalMs\":" +
        std::to_wstring(value.intervalMs) + L",\"keys\":[";
    for (size_t index = 0; index < value.keys.size(); ++index) {
        if (index) output += L',';
        output += std::to_wstring(value.keys[index]);
    }
    return output + L"]}";
}
template <typename Mapping>
void AppendConfigMapping(std::wstring& output, const Mapping& value) {
    output += L"\"mapping\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) output += L',';
        output += std::to_wstring(value.mapping[index]);
    }
    output += L"],\"turbo\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) output += L',';
        output += ConfigTurbo(value.turbo[index]);
    }
    output += L"],\"sequence\":[";
    for (size_t index = 0; index < ButtonCount; ++index) {
        if (index) output += L',';
        output += ConfigSequence(value.sequence[index]);
    }
    output += L']';
}
std::wstring ConfigJson() {
    std::wstring output = L"{\n  \"formatVersion\": 1,\n  \"language\": " + ConfigQuote(language) +
        L",\n  \"closeBehavior\": " + std::to_wstring(closeBehavior) +
        L",\n  \"autoMode\": " + ConfigBool(autoMode.load()) +
        L",\n  \"steamTakeover\": " + ConfigBool(steamTakeover.load()) +
        L",\n  \"selectedPreset\": " + std::to_wstring(selectedPreset) +
        L",\n  \"folders\": [";
    for (size_t index = 0; index < folders.size(); ++index) {
        if (index) output += L',';
        output += ConfigQuote(folders[index]);
    }
    output += L"],\n  \"presets\": [\n";
    for (size_t index = 0; index < presets.size(); ++index) {
        if (index) output += L",\n";
        const Preset& preset = presets[index];
        output += L"    {\"name\":" + ConfigQuote(preset.name) +
            L",\"folder\":" + ConfigQuote(preset.folder) +
            L",\"targetExecutable\":" + ConfigQuote(preset.targetExecutable) +
            L",\"leftPadMode\":" + std::to_wstring(preset.leftPadMode) +
            L",\"rightPadMode\":" + std::to_wstring(preset.rightPadMode) +
            L",\"leftPadSensitivity\":" + std::to_wstring(preset.leftPadSensitivity) +
            L",\"rightPadSensitivity\":" + std::to_wstring(preset.rightPadSensitivity) +
            L",\"leftStickDeadzone\":" + std::to_wstring(preset.leftStickDeadzone) +
            L",\"rightStickDeadzone\":" + std::to_wstring(preset.rightStickDeadzone) +
            L",\"leftStickOverlap\":" + std::to_wstring(preset.leftStickOverlap) +
            L",\"rightStickOverlap\":" + std::to_wstring(preset.rightStickOverlap) + L',';
        AppendConfigMapping(output, preset);
        output += L",\"layers\":[";
        for (size_t layer = 0; layer < preset.layers.size(); ++layer) {
            if (layer) output += L',';
            const Layer& entry = preset.layers[layer];
            output += L"{\"name\":" + ConfigQuote(entry.name) +
                L",\"trigger\":" + std::to_wstring(entry.trigger) + L',';
            AppendConfigMapping(output, entry);
            output += L'}';
        }
        output += L"]}";
    }
    return output + L"\n  ]\n}\n";
}
bool WriteUtf8File(const std::wstring& path, const std::wstring& content) {
    if (path.empty() || content.size() > 8 * 1024 * 1024) return false;
    const int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, content.data(),
        static_cast<int>(content.size()), nullptr, 0, nullptr, nullptr);
    if (size <= 0) return false;
    std::string bytes(static_cast<size_t>(size), '\0');
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, content.data(),
        static_cast<int>(content.size()), bytes.data(), size, nullptr, nullptr) != size) return false;
    const std::wstring temporary = path + L".tmp";
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr,
                              CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    DWORD written = 0;
    const bool okay = WriteFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &written,
                                nullptr) && written == bytes.size() && FlushFileBuffers(file);
    CloseHandle(file);
    if (okay && MoveFileExW(temporary.c_str(), path.c_str(),
                              MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) return true;
    DeleteFileW(temporary.c_str());
    return false;
}
bool ReadUtf8File(const std::wstring& path, std::wstring& content) {
    HANDLE file = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    LARGE_INTEGER length{};
    bool okay = GetFileSizeEx(file, &length) && length.QuadPart > 0 &&
                length.QuadPart <= 8 * 1024 * 1024;
    std::string bytes;
    if (okay) {
        bytes.resize(static_cast<size_t>(length.QuadPart));
        DWORD read = 0;
        okay = ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read,
                        nullptr) && read == bytes.size();
    }
    CloseHandle(file);
    if (!okay) return false;
    if (bytes.size() >= 3 && bytes.compare(0, 3, "\xef\xbb\xbf") == 0) bytes.erase(0, 3);
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes.data(),
                                        static_cast<int>(bytes.size()), nullptr, 0);
    if (size <= 0) return false;
    content.resize(static_cast<size_t>(size));
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, bytes.data(),
                               static_cast<int>(bytes.size()), content.data(), size) == size;
}
bool SavePresets() { return WriteUtf8File(settingsPath, ConfigJson()); }

const simple_json::Value* ConfigField(const simple_json::Value& value, const wchar_t* key,
                                       simple_json::Value::Type type) {
    const auto* field = value.Get(key);
    return field && field->type == type ? field : nullptr;
}
bool ConfigNumber(const simple_json::Value& object, const wchar_t* key, uint32_t& result,
                  uint32_t maximum = (std::numeric_limits<uint32_t>::max)()) {
    const auto* value = ConfigField(object, key, simple_json::Value::Type::Number);
    if (!value || value->number > maximum) return false;
    result = static_cast<uint32_t>(value->number);
    return true;
}
bool ConfigString(const simple_json::Value& object, const wchar_t* key,
                  std::wstring& result, size_t maximum) {
    const auto* value = ConfigField(object, key, simple_json::Value::Type::String);
    if (!value || value->string.size() > maximum) return false;
    result = value->string;
    return true;
}
bool ConfigBoolean(const simple_json::Value& object, const wchar_t* key, bool& result) {
    const auto* value = ConfigField(object, key, simple_json::Value::Type::Boolean);
    if (!value) return false;
    result = value->boolean;
    return true;
}
bool ValidConfigKey(uint32_t key, bool inherit) {
    return (inherit && key == INHERIT) || key == 0 ||
        (key >= MOUSE_LEFT && key <= MOUSE_MIDDLE) ||
        ((key & 0xffu) != 0 && (key & ~0x100ffu) == 0);
}
template <typename Mapping>
bool ReadConfigMapping(const simple_json::Value& source, Mapping& destination,
                       bool inherit) {
    using Type = simple_json::Value::Type;
    const auto* mapping = ConfigField(source, L"mapping", Type::Array);
    const auto* turbo = ConfigField(source, L"turbo", Type::Array);
    const auto* sequence = ConfigField(source, L"sequence", Type::Array);
    constexpr size_t OldButtonCount = RightStickRight + 1;
    if (!mapping || !turbo || !sequence ||
        (mapping->array.size() != ButtonCount && mapping->array.size() != OldButtonCount) ||
        turbo->array.size() != mapping->array.size() ||
        sequence->array.size() != mapping->array.size()) return false;
    for (size_t index = 0; index < ButtonCount; ++index) {
        const size_t sourceIndex = index < mapping->array.size() ? index :
            DPadUp + (index - LeftStickUp);
        const auto& code = mapping->array[sourceIndex];
        if (code.type != Type::Number || code.number > (std::numeric_limits<uint32_t>::max)() ||
            !ValidConfigKey(static_cast<uint32_t>(code.number), inherit)) return false;
        destination.mapping[index] = static_cast<uint32_t>(code.number);
        auto& rapid = destination.turbo[index];
        const auto& rapidSource = turbo->array[sourceIndex];
        if (!ConfigBoolean(rapidSource, L"enabled", rapid.enabled) ||
            !ConfigNumber(rapidSource, L"intervalMs", rapid.intervalMs, 2000) ||
            rapid.intervalMs < 20 ||
            !ConfigNumber(rapidSource, L"delayMs", rapid.delayMs, 5000)) return false;
        auto& steps = destination.sequence[index];
        const auto& stepSource = sequence->array[sourceIndex];
        if (!ConfigBoolean(stepSource, L"enabled", steps.enabled) ||
            !ConfigBoolean(stepSource, L"repeat", steps.repeat) ||
            !ConfigNumber(stepSource, L"intervalMs", steps.intervalMs, 2000) ||
            steps.intervalMs < 20) return false;
        const auto* keys = ConfigField(stepSource, L"keys", Type::Array);
        if (!keys || keys->array.size() > 16 || (steps.enabled && keys->array.empty())) return false;
        for (const auto& key : keys->array) {
            if (key.type != Type::Number || key.number > (std::numeric_limits<uint32_t>::max)() ||
                !ValidConfigKey(static_cast<uint32_t>(key.number), false) || key.number == 0)
                return false;
            steps.keys.push_back(static_cast<uint32_t>(key.number));
        }
    }
    return true;
}
struct ConfigSnapshot {
    std::wstring language;
    uint32_t closeBehavior = 0;
    bool autoMode = false;
    bool steamTakeover = false;
    size_t selectedPreset = 0;
    std::vector<std::wstring> folders;
    std::vector<Preset> presets;
};
bool ReadConfig(const std::wstring& path, ConfigSnapshot& result) {
    std::wstring content;
    if (!ReadUtf8File(path, content)) return false;
    simple_json::Value root;
    if (!simple_json::Parser(content).Parse(root)) return false;
    using Type = simple_json::Value::Type;
    uint32_t version = 0, selected = 0;
    if (!ConfigNumber(root, L"formatVersion", version, 1) || version != 1 ||
        !ConfigString(root, L"language", result.language, 8) ||
        (result.language != L"ja" && result.language != L"en") ||
        !ConfigNumber(root, L"closeBehavior", result.closeBehavior, 2) ||
        !ConfigBoolean(root, L"autoMode", result.autoMode) ||
        !ConfigNumber(root, L"selectedPreset", selected, 99)) return false;
    if (root.Get(L"steamTakeover") &&
        !ConfigBoolean(root, L"steamTakeover", result.steamTakeover)) return false;
    const auto* folderArray = ConfigField(root, L"folders", Type::Array);
    const auto* presetArray = ConfigField(root, L"presets", Type::Array);
    if (!folderArray || folderArray->array.size() > 100 || !presetArray ||
        presetArray->array.empty() || presetArray->array.size() > 100 ||
        selected >= presetArray->array.size()) return false;
    for (const auto& item : folderArray->array) {
        if (item.type != Type::String || item.string.empty() || item.string.size() > 512) return false;
        result.folders.push_back(item.string);
    }
    for (const auto& item : presetArray->array) {
        Preset preset;
        if (!ConfigString(item, L"name", preset.name, 128) || preset.name.empty() ||
            !ConfigString(item, L"folder", preset.folder, 512) ||
            !ConfigString(item, L"targetExecutable", preset.targetExecutable, MAX_PATH * 2) ||
            !ConfigNumber(item, L"leftPadMode", preset.leftPadMode, 2) ||
            !ConfigNumber(item, L"rightPadMode", preset.rightPadMode, 2) ||
            !ConfigNumber(item, L"leftPadSensitivity", preset.leftPadSensitivity, 400) ||
            preset.leftPadSensitivity < 25 ||
            !ConfigNumber(item, L"rightPadSensitivity", preset.rightPadSensitivity, 400) ||
            preset.rightPadSensitivity < 25 || !ReadConfigMapping(item, preset, false)) return false;
        if ((item.Get(L"leftStickDeadzone") && !ConfigNumber(item, L"leftStickDeadzone", preset.leftStickDeadzone, 32767)) ||
            (item.Get(L"rightStickDeadzone") && !ConfigNumber(item, L"rightStickDeadzone", preset.rightStickDeadzone, 32767)) ||
            (item.Get(L"leftStickOverlap") && !ConfigNumber(item, L"leftStickOverlap", preset.leftStickOverlap, 32767)) ||
            (item.Get(L"rightStickOverlap") && !ConfigNumber(item, L"rightStickOverlap", preset.rightStickOverlap, 32767))) return false;
        const auto* layerArray = ConfigField(item, L"layers", Type::Array);
        if (!layerArray || layerArray->array.size() > ButtonCount) return false;
        for (const auto& layerItem : layerArray->array) {
            Layer layer;
            uint32_t trigger = 0;
            if (!ConfigString(layerItem, L"name", layer.name, 64) ||
                !ConfigNumber(layerItem, L"trigger", trigger, static_cast<uint32_t>(ButtonCount)) ||
                !ReadConfigMapping(layerItem, layer, true)) return false;
            layer.trigger = trigger;
            preset.layers.push_back(std::move(layer));
        }
        result.presets.push_back(std::move(preset));
    }
    result.selectedPreset = selected;
    return true;
}
void ApplyConfig(ConfigSnapshot&& value) {
    language = std::move(value.language);
    closeBehavior = value.closeBehavior;
    autoMode = value.autoMode;
    steamTakeover = value.steamTakeover;
    selectedPreset = value.selectedPreset;
    folders = std::move(value.folders);
    presets = std::move(value.presets);
    PublishSelectedPreset();
}
void CopyLegacyIconCache(const std::wstring& legacyFolder,
                         const std::wstring& newFolder) {
    const std::wstring source = legacyFolder + L"\\TargetIcons";
    const std::wstring destination = newFolder + L"\\TargetIcons";
    WIN32_FIND_DATAW entry{};
    HANDLE search = FindFirstFileW((source + L"\\*").c_str(), &entry);
    if (search == INVALID_HANDLE_VALUE) return;
    CreateDirectoryW(destination.c_str(), nullptr);
    do {
        if (entry.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) continue;
        const std::wstring name = entry.cFileName;
        if (!name.ends_with(L".png") && !name.ends_with(L".png.meta")) continue;
        CopyFileW((source + L"\\" + name).c_str(),
                  (destination + L"\\" + name).c_str(), TRUE);
    } while (FindNextFileW(search, &entry));
    FindClose(search);
}
bool LoadSettings() {
    if (settingsPath.empty()) return false;
    if (GetFileAttributesW(settingsPath.c_str()) != INVALID_FILE_ATTRIBUTES) {
        ConfigSnapshot loaded;
        if (!ReadConfig(settingsPath, loaded)) return false;
        ApplyConfig(std::move(loaded));
        return true;
    }
    const std::wstring jsonPath = settingsPath;
    const std::wstring newFolder = settingsPath.substr(0, settingsPath.find_last_of(L"\\/"));
    const std::wstring legacyFolder =
        newFolder.substr(0, newFolder.find_last_of(L"\\/")) + L"\\SteamlessController";
    const std::wstring legacyJson = legacyFolder + L"\\ControllerSettings.json";
    const std::wstring legacyIni = legacyFolder + L"\\MapleStoryController.ini";
    if (GetFileAttributesW(legacyJson.c_str()) != INVALID_FILE_ATTRIBUTES) {
        ConfigSnapshot loaded;
        if (!ReadConfig(legacyJson, loaded)) return false;
        ApplyConfig(std::move(loaded));
    } else if (GetFileAttributesW(legacyIni.c_str()) != INVALID_FILE_ATTRIBUTES) {
        settingsPath = legacyIni;
        LoadLegacyPresets();
        settingsPath = jsonPath;
    } else {
        Preset initial; initial.name = L"標準";
        presets.push_back(std::move(initial));
        PublishSelectedPreset();
    }
    if (!SavePresets()) return false;
    CopyFileW((legacyFolder + L"\\WindowPlacement.ini").c_str(),
              (newFolder + L"\\WindowPlacement.ini").c_str(), TRUE);
    CopyLegacyIconCache(legacyFolder, newFolder);
    return true;
}
