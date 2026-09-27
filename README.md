# Remapcon

Remapcon is an unofficial Windows key and mouse mapper for the 2026 Steam Controller.
It reads the controller in Steamless Mode and applies presets when a selected app
is in the foreground. The interface uses WebView2; input handling is native C++.

**This project is independent of Valve and is not endorsed by Valve.**
Steam and Steam Controller are trademarks of Valve Corporation.
The icon is original, stylized artwork; it does not contain a Valve logo.

[日本語の説明](README.ja.md)

## Screenshot

The actual UI is shown with fictional presets and target applications.
No personal settings or real application data are included.
The [demo data generator](docs/demo/generate_preview.py) is included for reproducibility.

![Remapcon with fictional demo data](docs/images/remapcon-demo.png)

## Features

- Per-app presets and folders, with import and export as JSON.
- Button and right-stick direction mappings, layers, turbo, and key sequences.
- Independent mouse or scroll movement for each touchpad.
- Releases all synthesized keys on stop, focus loss, and controller disconnect.

## Requirements

- Windows 10 or 11 and a 2026 Steam Controller.
- [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
- [Microsoft Visual C++ Redistributable (x64)](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) for the prebuilt executable.
- Administrator mode may be required for an elevated target app to receive input.

Close Steam before enabling Steamless Mode if it is using the controller.
The app does not install a virtual gamepad driver.

## Build

Install Visual Studio with Desktop development with C++, CMake 3.20 or newer,
and the Windows SDK. CMake downloads the pinned WebView2 SDK from NuGet on its
first configure. For Visual Studio 2026:

```bat
cmake -S . -B build -G "Visual Studio 18 2026" -A x64
cmake --build build --config Release --target Remapcon
```

For Visual Studio 2022, use `Visual Studio 17 2022` as the generator. The
executable is `build\Release\Remapcon.exe`.
After building, create a portable ZIP containing the executable and required
license files with:

```bat
cpack --config build\CPackConfig.cmake -C Release -G ZIP
```

Extract `Remapcon-Windows-x64.zip` and run `Remapcon.exe`. The ZIP does not
bundle Microsoft runtime installers; install missing prerequisites using the
links above.

Settings are stored in `%LOCALAPPDATA%\Remapcon\ControllerSettings.json`.
On first launch, the app copies existing settings from
`%LOCALAPPDATA%\SteamlessController` if present, leaving the originals intact.

## Origin and licenses

Remapcon reuses and adapts the HID and Steam Controller communication code from
[SteamlessController](https://github.com/ddeverill/SteamlessController)
(commit `26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2`) by Dylan Deverill.
The original code is MIT-licensed. Remapcon's additions are also MIT-licensed;
see [LICENSE](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
for WebView2 notices and release packaging requirements.

A source or binary release should include the license and notice files.
The original SteamlessController installer, SignPath signing workflow,
ViGEmClient, and ViGEmBus installer are not part of this repository.
