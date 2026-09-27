# Remapcon

Remapconは、2026年版Steam Controllerの入力をキーボード・マウス操作へ割り当てる
非公式のWindowsアプリです。対象アプリごとのプリセットを作り、対象アプリが前面に
あるときに自動で有効化できます。画面はWebView2、入力処理はC++で動作します。

**Valve Corporationによる公式アプリではなく、Valveとの提携・承認関係はありません。**
SteamおよびSteam ControllerはValve Corporationの商標です。アイコンは独自の
簡略化した図案で、Valveの公式ロゴは使用していません。

## 主な機能

- アプリごとのプリセットとフォルダ、JSON形式のインポート・エクスポート
- ボタン・右スティックの方向、レイヤー、連打、キーシーケンスの割り当て
- 左右トラックパッドそれぞれのマウス移動またはスクロール
- 停止、フォーカス移動、切断時の送信キー解除

## 動作・ビルド

Windows 10/11、2026年版Steam Controller、[Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)が必要です。
配布版のEXEには[Microsoft Visual C++ 再頒布可能パッケージ（x64）](https://learn.microsoft.com/ja-jp/cpp/windows/latest-supported-vc-redist)も必要です。
対象アプリが管理者権限の場合はRemapconも管理者として実行する必要があります。
Steamがコントローラーを使用中なら、Steamを終了してからSteamless Modeを有効にしてください。
仮想ゲームパッドドライバーは使用しません。

ビルドにはVisual Studioの「C++によるデスクトップ開発」、Windows SDK、CMake 3.20以上が必要です。
初回のCMake設定時にWebView2 SDKをNuGetから取得します。Visual Studio 2026の場合:

```bat
cmake -S . -B build -G "Visual Studio 18 2026" -A x64
cmake --build build --config Release --target Remapcon
```

Visual Studio 2022ではジェネレーターを`Visual Studio 17 2022`に変更します。
実行ファイルは`build\Release\Remapcon.exe`です。
ライセンス表示を含む配布用ZIPは、ビルド後に次のコマンドで作れます。

```bat
cpack --config build\CPackConfig.cmake -C Release -G ZIP
```

生成された`Remapcon-Windows-x64.zip`を展開して`Remapcon.exe`を起動します。
ZIPにはMicrosoftのランタイムインストーラーを同梱していません。必要な場合は
上記のリンクから入手してください。
設定は`%LOCALAPPDATA%\Remapcon\ControllerSettings.json`に保存します。
以前の`%LOCALAPPDATA%\SteamlessController`に設定がある場合は初回起動時に
読み取り、元のファイルは残します。

## 出典・ライセンス

HID通信とSteam Controller制御には、Dylan Deverill氏の
[SteamlessController](https://github.com/ddeverill/SteamlessController)
（基点コミット`26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2`）のコードを利用しています。
元コードとRemapconの追加コードはMITライセンスで公開します。全文は[LICENSE](LICENSE)、
WebView2などの権利表示は[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)を参照してください。
ソースやEXEを配布するときはライセンス・権利表示ファイルも同梱してください。
