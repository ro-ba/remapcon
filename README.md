# Remapcon

Launching an app through another launcher, only to find that your Steam Input layout did not switch? Getting unintended input from the Desktop Layout? Remapcon helps you use the 2026 Steam Controller with the mapping you want for the app in front of you.

It reads the controller directly and maps buttons, sticks, and trackpads to keyboard and mouse input. A preset can activate automatically while its target app is in the foreground. Puck connections are supported.

[日本語の説明](README.ja.md)

## Why I built it

Steam Input is usually enough. In my setup, it could not reliably track an app launched through a separate launcher, and Desktop Layout or Lizard Mode input could overlap with the intended mapping. I built Remapcon so I could bring that app to the foreground and use the keyboard and mouse mapping I intended.

It is also useful for apps you want to control with keyboard and mouse bindings rather than gamepad input. Remapcon does not create a virtual Xbox controller.

## Features

- Per-app presets organized in folders
- Independent D-pad and left/right stick direction mappings; trackpad movement, tap, and press actions with movement and press haptics
- Adjustable deadzones and diagonal overlap for each stick, with a live response preview
- Layers, turbo, and key sequences
- Automatic activation while the target app is in the foreground
- JSON import and export

## Screenshot

![Remapcon with fictional English demo data](docs/images/remapcon-demo-en.png)

## Getting started

Download `Remapcon-Windows-x64.zip` from [Releases](https://github.com/ro-ba/remapcon/releases), extract it, and run `Remapcon.exe` from the extracted folder. There is no installer.

You need Windows 10/11 and a 2026 Steam Controller / Puck. Install the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) and [Visual C++ Redistributable (x64)](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) if your PC does not already have them.

1. Close Steam if it is using the controller.
2. Create a preset and use **Choose app** to select the target executable.
3. Double-click a mapping row to edit it. Turn on **Auto enable in target app** to activate the preset while that app is in the foreground.

If the target app runs as administrator, run Remapcon as administrator too.

## Build from source

Install Visual Studio with Desktop development with C++, the Windows SDK, and CMake 3.20 or newer. CMake downloads the WebView2 SDK on first configure.

```bat
cmake -S . -B build -G "Visual Studio 18 2026" -A x64
cmake --build build --config Release --target Remapcon RemapconUpdater
```

The executable is `build\Release\Remapcon.exe`. To package it with license files, run `cpack --config build\CPackConfig.cmake -C Release -G ZIP`.

## License

Remapcon uses HID and controller communication code from [SteamlessController](https://github.com/ddeverill/SteamlessController). The original code and Remapcon are MIT-licensed; see [LICENSE](LICENSE) and [third-party notices](THIRD_PARTY_NOTICES.md).

Remapcon is an unofficial app independent of Valve Corporation. Steam and Steam Controller are trademarks of Valve Corporation.
