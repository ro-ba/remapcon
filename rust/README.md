# PadMux Rust migration workspace

既存のC++版と共存する独立workspaceです。`padmux-diag` はWindows x64でSteam Controllerを列挙し、HID reportを読み取り、入力を表示します。通常GUIのRust版 `padmux.exe` と `padmux-updater.exe` もビルドします。移行候補であり、既存C++版とproduction releaseを維持しています。名称表示と更新URLをPadMuxへ変更しました。UIの操作と設定schemaは維持しています。

`windows 0.62.2` をWindows境界、`serde` / `serde_json` を既存JSONの読み書きに使います。protocol/mapping/config/controller sessionはsafe Rust、unsafeは `windows_hid.rs`、`windows_output.rs`、`windows_config.rs`、`windows_app.rs`、`windows_steam.rs`、`windows_webview.rs` 、`windows_icons.rs`、`windows_shell.rs`、`windows_gui.rs`、`windows_update.rs` のWindows境界に局所化しています。既存C++をreferenceにしたtestを追加しています。

Windows x64でRust stable（MSVC）、Visual Studio C++ Build Tools、Windows SDKを用意し、このdirectoryで実行します。

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
cargo test --workspace --locked --target x86_64-pc-windows-msvc
cargo build --workspace --locked --target x86_64-pc-windows-msvc
cargo run --locked --target x86_64-pc-windows-msvc -- --help
```

出力は `target/x86_64-pc-windows-msvc/debug/padmux-diag.exe`。C++版はrepository rootから従来のCMake手順でビルドします。Rust CIは `.github/workflows/rust.yml`、既存releaseのbuild/packageは従来のworkflowです。

ライセンスはrootの [LICENSE](../LICENSE)（MIT、Dylan Deverill / ro-baのcopyright）を参照します。[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) のSteamlessController attributionも維持します。翻訳moduleにもreference commitと両copyrightを記載しています。Rust依存の権利表示は [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) に同梱しています。

Phase 0は [調査報告](../docs/rust-migration-phase0.ja.md)、最新の各Phaseの合否・実機結果・C++との差は [検証記録](../docs/rust-migration-progress.ja.md)。チェックが合格したら次Phaseへ進みます。

## 開発環境と検証

WSL/Linuxはbrewを優先します。今回 `brew install rustup` で導入しました。brew版はkeg-onlyなので、使用するshellでPATHを追加し、toolchainを用意します。

```bash
export PATH="$(brew --prefix rustup)/bin:$PATH"
rustup toolchain install stable --profile minimal --component rustfmt --component clippy
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --locked --offline -- -D warnings
```

brewはWindows非対応のため、Windows側は公式rustupと既存Visual Studioを使用しました。WSL上のcheckoutをWindowsのCargoからビルドすると出力directoryのfile lockが失敗するため、その場合はPowerShellで次を設定してから上記Windows用コマンドを実行します。

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP "padmux-phase1-rust-target"
```

Phase 1時点では2026-10-02、Rust 1.99.0 / `x86_64-pc-windows-msvc` でfmt・clippy・test（0件）・debug build・console起動・release buildが成功しました。外部crateを解決しないためbuildはofflineで確認しています。Windows CIの実行結果は未確認です。
WSL側のfmt/clippyは成功しましたが、Linux linker `cc` が無くtest/build/runは失敗しました。本Phaseの対象であるWindows側でtest/build/runを確認しており、Linux用compilerは追加していません。

既存C++版もソースをWindows一時directoryへコピーし、Visual Studio 18 2026 / MSVC 19.51 / Windows SDK 10.0.26100.0で `PadMux` と `padmux-updater` のReleaseビルドが成功しました。一時directoryに対するMSBuildのincremental build警告はありました。C++・CMake・UI・既存release workflowは維持します。Phase 2以降の結果は上記検証記録を参照してください。

## 診断

```powershell
# 列挙のみ / defaultはshared readで命令を送らない
cargo run --locked --target x86_64-pc-windows-msvc -- --list
cargo run --locked --target x86_64-pc-windows-msvc -- --seconds 30 --raw
# 特定interfaceを選択（indexは列挙順）
cargo run --locked --target x86_64-pc-windows-msvc -- --device 1 --seconds 30
# 制御試験: write-exclusive取得後だけLizard off、keepalive、左右haptics、終了時restore
cargo run --locked --target x86_64-pc-windows-msvc -- --control --haptics --seconds 10
# 期限内の切断/reconnect試験
cargo run --locked --target x86_64-pc-windows-msvc -- --control --reconnect --seconds 60
# 実際のWindows出力試験（空のエディタを前面にし、操作準備をしてから開始）
cargo run --locked --target x86_64-pc-windows-msvc -- --control --output-test --seconds 60
# 右padでmouse移動75%、左padでscroll100%を比較
cargo run --locked --target x86_64-pc-windows-msvc -- --control --output-test --mouse-pad right --mouse-sensitivity 75 --seconds 180
```

通常のreadではLizard Modeを変更しません。Puckには空slotがあるため、350msでstate reportが出るslotをprobeします。`--raw` は表示sampleのhex、`seen_flags` は全state reportの累積flagsです。stateは0x42/0x45、その他のreportの意味は推測しません。

control/hapticsの試験前はSteamとC++版PadMuxを通常終了してください。exclusive取得に失敗したらcommandを送らずerror終了します。このcontrol診断はPnP cycleを行いません。Steam取得は別の `--steam-handoff` 診断で確認します。正常終了/error/Ctrl+Cでrestoreし、強制kill/console close時の復旧は保証できません。wired/Bluetooth/Nereidは未検証です。

`--output-test` は診断用の固定mappingです。A/B/X/Y→a/b/x/y、D-pad→arrow keys、LB/RB/L4→左/右/中mouse button、pad press→対応する左右mouse button、左pad→mouse move、右pad→wheel。`--mouse-pad left|right` で移動する側、`--mouse-sensitivity 25..400` で移動感度（default 100%）を選べます。反対側はscroll100%です。既存設定のload/saveやforeground判定は未接続です。終了/error時は保持したキーを解放してからcontrollerを復元します。

mapping比較fixtureは `tests/mapping-input.txt`。`tests/reference_mapping.py` で既存C++の関数を抽出し、MSVCで実行した記録が `tests/mapping-expected.txt` です。Rustのtestで同じ操作/時刻/失敗条件を再生します。参照の再生成手順はscript冒頭を参照してください。実際のSendInputを使わず、入力判定・mapping・mouse/wheel出力列を比較できます。

mouse送信失敗は最初のWindows errorとmove/wheelの成功・失敗件数を表示します。HIDから独立した小さな比較は `cargo run --locked --target x86_64-pc-windows-msvc --example output_probe`。これは左右1pxの相対moveを実際に送ります。C++/Rust共通のerror 87はWindows再起動後に解消し、Puck実機で移動・クリック・スクロールを確認しました。原因は未特定です。

## 設定JSON互換性

```powershell
cargo run --locked -- --check-config "$env:LOCALAPPDATA\PadMux\ControllerSettings.json"
# 保存先は明示する。比較時は元の設定を上書きせず、一時directoryを使う。
cargo run --locked -- --check-config "$env:LOCALAPPDATA\PadMux\ControllerSettings.json" --save-config "$env:TEMP\padmux-config-comparison.json"
```

この操作はHIDや入力送信を開始しません。既存schema、旧28ボタン形式、optional項目の既定値、UTF-16文字数制限、UTF-8 BOMに対応します。保存は同directoryの `.tmp` へ書き込み・flush後にWindows APIで置換し、失敗時は元ファイルを保持します。読めない8MiB超の出力は保存前に拒否します。旧INIからの起動時移行は後続のアプリ統合で接続します。

`tests/reference_config.py` が既存C++のreader/writerを抽出します。MSVCでビルドしたexeを環境変数 `PADMUX_CONFIG_REFERENCE` に指定すると、config testはRustとC++両方の受理・拒否結果を比較します。Windows CIもこの比較を行います。公開fixtureにはユーザー設定を含めません。

## 前面アプリとSteam引き渡し

```powershell
# 観測のみ。HID・入力送信・設定変更を行わない。
cargo run --locked -- --foreground-config "$env:LOCALAPPDATA\PadMux\ControllerSettings.json" --seconds 30
# 管理者権限での実機試験。対象exeが前面の間だけcontrollerを取得する。
padmux-diag.exe --steam-handoff test-settings.json --seconds 180
```

前面アプリは既存C++と同じquery権限・520 UTF-16 buffer・フルパス/ファイル名・CRT `_wcsicmp` 比較・100ms cacheを使います。自動有効化するのは選択中presetだけで、別presetを自動選択しません。対象アプリの `.exe` 選択dialogも既存flags/文字数で実装し、UIへの接続・手動確認はPhase 9で行います。

`--steam-handoff` はautoMode/steamTakeover/target設定が必要です。Steam起動中のexclusive取得に失敗した場合、C++と同じPnP disable/re-enableを行います。HKLMの同じpending keyにchild/parent IDをflushしてからdisableし、別helper processが復旧します。既存C++との並行cycleを防ぐ同じnamed mutex、12秒down wait・8秒claim・6秒finish・20秒retry、終了時のrestore→HID close→Steam returnを維持します。起動時pending復旧もmutex内のhelperで行います。

この試験はcontroller inputを読むだけでkeyboard/mouseを送信せず、設定を書き換えません。通常権限はdevice write前に拒否します。2026-10-03、Steam起動中のPuck実機で取得/返却/再入場/電源OFF→ON後の再取得とSteam側通常操作の復帰を確認しました。wired/BT/Nereidは未検証です。強制終了時は次回管理者起動のpending復旧が必要になる場合があります。

## WebView2表示とbridge比較（Phase 9進行中）

```powershell
cargo run --release --locked --target x86_64-pc-windows-msvc --example webview_probe -- --seconds 180
# 既存DOM→native bridge→TEMP保存→再描画の自動編集確認（OS全体へキーを送らない）
cargo run --release --locked --target x86_64-pc-windows-msvc --example webview_probe -- --self-test --seconds 30
# 手動編集確認（.exe選択も含む）。元設定は変更せず、15分後または×で終了
cargo run --release --locked --target x86_64-pc-windows-msvc --example webview_probe -- --edit-test --seconds 900
# GUIから実機入力表示／mappingを確認。Steam排他取得を使う場合は管理者権限で実行
cargo run --release --locked --target x86_64-pc-windows-msvc --example webview_probe -- --controller-test --seconds 900
```

既存HTML/CSS/JavaScriptをそのまま使用し、設定と最後のWindowPlacement.iniを読み取ります。TEMP専用WebView profileを使用し、既存設定・配置を保存せず、コントローラーも取得しません。初回描画時にnative resize／maximize／restore後のbrowser boundsを自動比較します。`--edit-test` を追加すると設定編集をTEMPの独立コピーへ保存し、終了時にコピーをcleanupします。普段の設定／配置は維持します。ウィンドウ配置の変更はこの診断では保存しないため、次回表示は元の位置・サイズへ戻ります。`--controller-test` だけは既存のcontroller loop／mapping／Windows出力／Steam取得・返却をGUIへ接続します。Puckで入力確認表示・左右stick・Steam通常操作の復帰を実機確認済みです。import／exportは同じnative JSON dialogを使い、TEMP設定に対してbackup／rollbackを維持します。import／exportは手動確認も合格しました。GUI通常出力・Steam返却はPuckで実機確認も合格しました。アイコンの手動確認と、updaterのGUI統合は未完了です。トレイ／閉じる確認／再起動後の配置復元は手動確認も合格しました。

`tests/editor_reference.rs` は205操作と数字／配置の境界をC++ referenceと比較します。保存失敗、row clipboard、layer／preset、30件のundo・redoを含み、実ユーザー設定は使いません。WebView2 COMは `webview2-com 0.39.1`（MIT）を利用し、loaderのMicrosoft権利表示も維持します。

対象アイコンはC++と同じShell／WICのPNGとキャッシュ形式を使います。診断はTEMP cacheだけに書き込み、普段のTargetIconsを変更しません。Windows native icon testはC++のPNG／base64／fingerprintと全byte比較します。

`--shell-test` は一時設定で既存の閉じる確認／トレイicon・menuを接続し、TEMP\padmux-phase9-shell-test\WindowPlacement.iniだけに位置・サイズ・最大化状態を保存します。このmodeの再起動では配置を復元し、普段のWindowPlacement.iniを変更しません。`--self-test --shell-test --seconds 30` は既存DOMの確認画面→トレイ格納→notification callbackから復帰→配置保存・復元→最大化／最小化後の復元を自動確認します。OS全体へkeyboard／mouse入力は送りません。

`--single-instance-test` は診断専用mutexで二重起動を確認します。2回目は既存画面を復帰し、既存UIのalreadyRunning通知を送り終了します。`--output-test` は一時presetでメモ帳だけを対象にGUIの実機出力を確認します（管理者権限・Steam起動中）。A/B/X/Y=文字、右pad=カーソル、LB=クリック、左pad=スクロールです。メモ帳から離れるとSteamへ返却します。

`--startup-test --self-test --shell-test --seconds 30` はTEMPの旧INIから設定を移行して再読込し、対象アイコンのnative PNGが既存UIで表示されること、編集・トレイ・配置保存まで自動確認します。旧INI／startupの互換性は `config_reference` でC++関数の抽出実行と比較します。

## 通常GUIと配布候補

```powershell
cargo build --locked --release --all-targets --target x86_64-pc-windows-msvc
# 比較では専用directoryの設定コピーを使用する。指定なしは通常ユーザー設定を使用する。
& ./target/x86_64-pc-windows-msvc/release/padmux.exe --data-dir "C:\absolute\test-settings"
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/package.ps1
# CARGO_TARGET_DIRを変更した場合は -BuildDirectory でrelease directoryを指定する。
```

候補は `dist/padmux-rust-windows-x64.zip` と `.sha256`。Windows 10以降とWebView2 Runtimeが必要です。CRTは静的リンクされ、候補exeはVisual C++ Redistributableへ依存しません。package scriptはx64／GUI subsystem／DLL依存／license同梱とupdaterによるZIP受理・不一致digest拒否を検査します。既存C++ releaseを公開するworkflowは保持しています。

通常GUIは同じユーザー設定と配置を保存します。C++との並行比較には `--data-dir` の専用directoryを使用してください。C++とRustの通常GUIは旧Remapconとも同じsingle-instance mutexを使用します。実更新と未確認transportの実機試験が完了するまで、C++を削除しません。

GUIの自動確認は実機操作なしで実行できます（Windowsの対話desktopとWebView2 Runtimeが必要）。TEMP設定・profileのみ使用し、controller取得・SendInput・実更新は行いません。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/check-gui.ps1
# CARGO_TARGET_DIRを変更した場合は -BuildDirectory でrelease directoryを指定する。
```

startup／旧設定移行・編集・undo／redo・tray・配置復元と、updater拒否／失敗通知／終了ackを検査し、TEMPにlogを保持します。実機確認の保留一覧は検証記録を参照してください。

PadMuxの初回起動では旧RemapconのJSON・配置・target iconをコピーし、元データを保持します。移行優先順位と残す旧識別子は [名称変更の記録](../docs/padmux-rename.ja.md) を参照してください。
