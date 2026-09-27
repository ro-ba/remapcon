# Remapcon

2026年版 Steam Controller / Puck のボタンやトラックパッドを、Windowsのキー・マウス操作に割り当てるアプリです。

## 作った理由

通常はSteam Inputのコントローラー設定で十分です。ただ、別のランチャーから起動するアプリをSteam Inputがうまく追跡できず、Desktop LayoutやLizard Modeの入力が設定と重なることがありました。Remapconはコントローラーの入力を直接読み取り、指定したアプリに合わせてキーやマウス操作へ変換します。

## できること

- 対象アプリごとにプリセットを作り、フォルダで整理する
- ボタン、右スティックの方向、左右トラックパッドを割り当てる
- レイヤー切替、連打、キーシーケンスを設定する
- 対象アプリが前面にあるとき自動で有効にする
- プリセットをJSON形式でインポート・エクスポートする

## 画面イメージ

実際のUIに架空のプリセットと対象アプリを表示したデモ画面です。

![架空データを使ったRemapconの画面例](docs/images/remapcon-demo.png)

## 使い始める

[Releases](https://github.com/ro-ba/remapcon/releases)から`Remapcon-Windows-x64.zip`をダウンロードして展開し、フォルダ内の`Remapcon.exe`を起動します。インストーラーはありません。

Windows 10/11と2026年版 Steam Controller / Puck が必要です。PCにない場合は、[Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)と[Visual C++ 再頒布可能パッケージ（x64）](https://learn.microsoft.com/ja-jp/cpp/windows/latest-supported-vc-redist)をインストールしてください。

1. Steamがコントローラーを使用中なら、Steamを終了します。
2. Remapconでプリセットを作成し、「アプリを変更」から対象アプリを指定します。
3. 割り当てを変えたい行をダブルクリックして設定します。「前面で自動有効」をオンにすると、対象アプリが前面にある間だけ有効になります。

対象アプリを管理者として実行している場合は、Remapconも管理者として実行してください。

## ソースからビルド

Visual Studioの「C++によるデスクトップ開発」、Windows SDK、CMake 3.20以上が必要です。初回の設定時にWebView2 SDKを取得します。

```bat
cmake -S . -B build -G "Visual Studio 18 2026" -A x64
cmake --build build --config Release --target Remapcon
```

生成物は`build\Release\Remapcon.exe`です。ライセンス表示を含む配布用ZIPは`cpack --config build\CPackConfig.cmake -C Release -G ZIP`で作成できます。

## ライセンス

[SteamlessController](https://github.com/ddeverill/SteamlessController)のHID通信・コントローラー制御コードを利用しています。元コードとRemapconはMITライセンスです。詳しくは[LICENSE](LICENSE)と[第三者の権利表示](THIRD_PARTY_NOTICES.md)を参照してください。

RemapconはValve Corporationとは無関係の非公式アプリです。SteamとSteam ControllerはValve Corporationの商標です。
