# PadMuxへの名称変更

2026-10-03。製品名は **PadMux**、GitHub repositoryは **padmux**。Pad + Multiplexer（Mux）の略で、controller入力を選択・振り分け、Keyboard／Mouseなどへ変換する役割を表す。将来の出力方式を限定する名称ではない。

## 変更した範囲

- README（日英）・名称の由来・UIタイトル／ヘッダー・tray tooltip／menu・更新表示・export名（padmux-settings.json）・log・version resource・branding asset名・日英の架空データ画面例。
- CMake projectはPadMux、target／exeは `padmux` と `padmux-updater`。Rust package／libraryは `padmux-diag`／`padmux_diag`、通常GUIも同じexe名。移行中の両実装は保持。
- C++配布は `padmux-windows-x64.zip`、Rust候補は `padmux-rust-windows-x64.zip`。ZIP内のfolderは `padmux-windows-x64`。このprojectにinstallerはなく、portable ZIP配布を維持。
- UI／native updaterのGitHub URL、asset名、staging suffix、TEMP prefix、CI／Release workflowとartifact名を更新。
- GitHub repositoryを `ro-ba/remapcon` から `ro-ba/padmux` へ改名し、旧URLの転送とlocal originを確認。コードのcommit／pushや新releaseの公開は行っていない。

## データ互換性

新しい保存先は `%LOCALAPPDATA%\PadMux\ControllerSettings.json`。ファイル名とJSON schemaは元のまま。

1. PadMux JSONがあればその内容を使用する。不正でも別データで上書きしない。
2. PadMux JSONがなければ、旧 `%LOCALAPPDATA%\Remapcon\ControllerSettings.json` を優先する。不正ならerrorにし、SteamlessControllerや初期値へfallbackしない。
3. Remapcon JSONがなければ、従来どおりSteamlessControllerのJSON／旧INI／初期設定の順で使用する。
4. 新JSONの保存成功後にだけ、旧Remapconの配置・TargetIconsをcopy-if-absentする。古いSteamlessControllerからの移行も保持。既存の配置・iconは上書きしない。

旧JSON・配置・iconは削除・変更しない。WebView2 profileは新しいPadMux folderに作るが、旧profileを削除しない。UIはlocalStorage等に設定を保存しておらず、preset等はJSONに保持される。

旧Remapcon updaterは旧exe／ZIP名を要求する。初回はPadMux ZIPを手動downloadし、旧版を通常終了してから新しいexeを起動する。過去releaseのassetを改名・削除していない。

## 旧名称を残す理由

repository全体でcase-insensitiveの `remapcon`、`REMAPCON` と派生identifier／file名を再検索した。現在のUI、branding、exe／package、CMake／Cargo、CI／Releaseのactive名称に旧名称は残していない。以下は意図して保持する。

| 残すもの | 場所・理由 |
| --- | --- |
| `Remapcon` data folder | C++ SettingsJson.inc、Rust windows_config、移行test。既存ユーザーのJSON・配置・iconを探すため |
| `Remapcon_SingleInstance`、`RemapconMainWindow` | C++ main、Rust GUI／shell。旧版との二重起動を防ぎ、同じcontrollerを並行取得しないため |
| `SOFTWARE\Remapcon\PendingDeviceCycle`、`Local\RemapconDeviceCycle` | C++ DeviceCycle、Rust windows_steam。旧版の中断記録を復旧し、device disable処理を直列化するため |
| Rust moduleのRemapcon reference comment | 翻訳元・MIT由来の記録。名称変更でprovenanceを消さないため |
| UI testの旧名称不在assert | 旧製品名がUIへ戻らないことを確認するため |
| THIRD_PARTY_NOTICESのformerly Remapcon | プロジェクトの由来を明示するため |
| READMEのRemapcon移行説明 | 既存ユーザーが設定場所と初回更新手順を把握するため |
| Phase 0／各Phaseの検証記録、Steam coexistence調査 | 改名前に実施した試験の履歴。旧exe／TEMP path／hashを新名に書き換えると記録が事実と異なるため |

SteamlessController／MapleStory由来のlicense、INI section／format、生成UI header等は別の由来を持つ識別子であり、不要なrefactorを避けて保持する。root MIT LICENSEのDylan Deverill／ro-ba copyright、SteamlessController固定commit attribution、MicrosoftとRust依存のlicense全文は保持。

## 検証

- Windows x64でC++ GUI／Updater Release buildとCPack ZIP生成に成功。既存Visual Studio／SDK／WebView2を再利用。TEMP buildに対する既知のMSBuild incremental警告はある。
- Rust fmt／Clippy（warnings拒否）／25 tests／debug・release all-targetsに成功。新dependencyなし。
- 設定移行は旧INI 6ケースと起動12ケースを変更後のC++参照と比較。旧Remapconの優先、不正JSON、既存PadMuxの優先、配置／iconの保持、保存失敗でも旧fileと別writerのTMP保持を確認。
- HTML／JS／CSSのC++・Rust一致とPadMux表示、新GitHub URL、UI旧名称不在をtestで確認。UI操作・設定schema・mapping logicは維持。
- GUI自動診断のstartup／編集・undo・redo／tray・配置、updater拒否／FAILED／READYがexit 0・error log空で合格。設定のない環境でもupdater診断を実行可能にした。
- Rust ZIPを展開した通常GUIを一時設定で2回起動し、通常終了・最大化状態の再起動復元を確認。両native GUI resourceのProductName／OriginalFilenameはPadMux／padmux.exe、versionは1.7.0を維持。
- C++／Rust両updaterが両ZIPを受理し、誤digestと不完全tagを拒否。ZIPの必要file、MIT両copyrightとthird-party notice同梱を確認。Rust candidateはstatic CRTのDLL検査にも合格。
- 普段のRemapcon JSON／WindowPlacement.iniのSHA-256は試験前後で不変。試験終了後、PadMux／Remapcon processは残っていない。

実機確認と実更新、GitHub上のCI実行は未実施。ユーザー外出中のため、以前の実機gateを引き続き保留し、C++実装を削除しない。

2026-10-04に実機確認を再開。改名後のRustでUSB列挙／入力／Lizard制御／振動／通常マウス復帰が合格し、変更なしのC++ HID参照とtransport・caps・report ID／sizeが一致。詳しい記録と残るgateは [移行進捗](rust-migration-progress.ja.md) を参照。実更新・GitHub CI・残りの実機gateが完了するまでC++実装を保持する。

2026-10-04追加確認：USBのkeyboard／mouse出力、Steam共存／再接続、Puckの通常配布候補GUIとlayers／turbo／sequencesが実機合格。旧設定JSON／配置hashは不変。最新公開releaseは旧C++ assetのみであり、Rustの実更新とGitHub CIは未確認のまま。
