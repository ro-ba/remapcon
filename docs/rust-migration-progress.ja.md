# Rust migration 検証記録

> Phase 1〜10は改名前のRemapconで実施した調査・試験の記録です。旧名称・実行file・TEMP pathは履歴として保持します。改名後の実機試験は2026-10-04の節に記載。現在の製品名はPadMuxです。[名称変更と互換性](padmux-rename.ja.md)。

2026-10-04更新。各Phaseのチェック合格後は次へ進む。実機結果が必要な項目はunit test/build成功とは区別する。

## Phase 1 — 合格

- 独立 `rust/` workspace、当初は外部crateなし。C++/CMake/UIは保持。
- Windows x64、Rust 1.99.0: fmt/clippy/test（当初0件）、debug/release build、console起動成功。
- C++のRemapcon/UpdaterもVS2026、MSVC 19.51、Windows SDK 10.0.26100.0でRelease build成功。
- WSLのrustupはbrewで導入。WindowsのRustは公式rustup、Visual Studioは既存環境を再利用。
- WSL上の出力directoryはWindows file lock/UNC制約があるため、Windows一時directoryでbuild。

## Phase 2 — Puck PoC合格

- Rustで列挙/open/read/parse。VID=28DE、PID=1304、usage=FF00:0001の4 interface。
- caps: input=54、output=64、feature=64。1つがlive slot、他3つは350ms probe timeout。
- 変更していないC++ `HidDevice.cpp` を一時consoleへリンクして同じ条件で比較。C++も4 interfaceをopenし、1つから54byteのstate `0x42` を取得。他3つはtimeout。
- Rustの最初の30秒readでstate 8,063件。ユーザーが操作した60秒readで16,032件、non-state 166件。errorなし、正常終了。
- 実機の累積flagsは `[87, 3f, be, 3b]`。A/B/X、D-pad、LB/RB、L4/L5/R4/R5、左右stick touch、pad touch、grip等を観測。全buttonの実機確認を主張するものではない。
- sampled raw reportのtriggerは左右とも0–32767。左stick X=-32767–32767、Y=-32767–11840、右X=-32767–32767、Y=-32767–24377。
- 左pad X=-12586–21394、Y=-32766–24122、area=0–7279。右X=-11392–20280、Y=-21104–11782、area=0–7171。pressは既存C++のarea>=1800条件に該当。firmware click bitはこの試験では観測されていない。
- parserは10/18/30byte gates、signed LE、0x42/0x45、28個のraw button/touch/grip bitをtest。unknown status等はrawのまま。
- C++の固定vectorも実行し、trigger/stick/pad座標がRust testの期待値と一致。
- Windows fmt/clippy、parser 3件＋CLI 2件のtest、debug/release build合格。wired/Bluetooth/Nereidの実機は未検証。

## Phase 3 — Puck合格

- C++ `DisableLizardMode/EnableLizardMode/SendKeepalive` とtrackpad haptic outputを参照。
- feature reportのcaps長padding/truncation、interrupt output padding、cancel/drainをWindows境界へ追加。
- Lizard offはclear mappings→IMU off→両pad raw。keepaliveは2秒ごと。復元はrumble zero→IMU off→default mappings→default settings。
- 正常終了は復元結果を確認。部分setup/read failure/panic時もDropでbest effort restore。Ctrl+C/Breakはatomic stop要求をmainに渡す。
- haptic側0=左/1=右、tick=1/click=2、movementは50ms以内drop、click優先。CLIで左tick/click→右tick/clickを試せる。
- 単一threadがdeviceを所有。Remapconにはnonzero rumble sourceがないため、未使用のrumble mix/threadやIMU mappingは導入していない。UIで利用するhapticsとrumble停止を対象とする。
- `--reconnect` は期限内で再列挙し1秒ごとretry。read無受信4秒でconnection loss（C++と同じ）。
- Windows fmt/clippy、6件のtest、debug/release build、help表示合格。
- 初回control実機testはwrite-exclusive openでWindows error 32（sharing violation）。SteamとC++版Remapconが起動していた。所有processは未特定。この失敗時はfeature/haptic commandsを送っていない。
- ユーザーによる両アプリの通常終了後、`--control --haptics --seconds 6` は終了code 0。Puckのlive slotでwrite-exclusive open、Lizard off、左tick/click→右tick/click、keepalive 2回、default mappings/settings restoreがすべて成功。state 1,513件、non-state 13件を受信。
- ユーザーの依頼で同じ6秒試験を再実行。終了code 0、state 1,519件、non-state 13件、左右4振動コマンド、keepalive 2回、default mappings/settings restore成功。体感の確認は通信成功と区別して扱う。
- 再試験後、ユーザーが「左左右右」の振動と、終了後の通常のマウス操作を確認した。Puckで左右tick/clickの体感、Lizard Mode復帰が確認済み。
- Ctrl+C試験も合格。Windows一時directoryの検証用harnessが専用consoleで診断を起動し、3秒後にCtrl+Cを送信。10秒の実行期限より前に復元し終了code 0、state 710件、non-state 6件、keepalive 1回を記録。ほかのprocessへsignal送信・強制終了はしていない。signal方式は [GenerateConsoleCtrlEvent](https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent) の仕様を参照。
- 60秒の電源off/on試験も終了code 0。電源offでread error 0x8007001F、1秒retryで再列挙し、空slot probe後にlive slotを再取得。排他open/Lizard off/keepalive/inputを再開し、復元成功。再接続後state 8,369件、non-state 74件、累積flags `[07, 02, b8, 3b]`。
- 最新変更もWindows fmt/clippy、6件のtest、release build合格。PuckのPhase 3 gateを満たしたためPhase 4へ進む。wired/Bluetooth/Nereidの実機検証は引き続き未実施。

## Phase 4 — pure Rust比較合格

- `mapping.rs` に32 button順序、trigger hysteresis、stick deadzone/diagonal overlap、2-report pad press、tap時間/移動判定、pad mouse/wheel、小数remainderを移植。
- layerの後勝ち/inherit/trigger除外、turbo、sequence、same-key参照数、keyboard repeat、foreground無効化、mapping generation変更時の解放、送信失敗時retry/statusを保持。
- 時刻はdriverが単調時刻を渡す。出力はcallbackが成功/失敗を返し、cleanupの20ms waitもdriverへ渡す。OS依存/unsafe/新規crate/共有lockなし。configをcloneせず借用。
- `tests/reference_mapping.py` が既存C++の該当関数を変更せず抽出。時刻とSendKey/SendInputのみ検証用に差し替え、同じchecked-in command列から出力fixtureを生成。実際のOS入力を送らない。
- Windows MSVC `/W4 /WX` のC++参照実行とRustのtraceが一致。境界時刻、短いreport、layer/preset変更、失敗したdown/repeat/up、3回cleanup、fractional motion等を含む。fixtureは `rust/tests/mapping-input.txt` / `mapping-expected.txt`。
- Windows fmt/clippy、8件のtest、release build合格。純粋なmappingとOS出力を分離したため、実際のkey/mouse送信確認はPhase 5で行う。診断exeへのconfig/UI接続は後続Phaseで行う。
- press counterは2-report判定を保ったままsaturatingにし、C++の長時間連続入力によるsigned overflowを避ける。通常入力の判定差はない。

## Phase 5 — Puck実入力合格

- C++ `SendKey` のscan code低8bit/extended bit、keyboard up/down、mouse 3 button flags、relative move、signed wheel dataを維持。`windows` の既存依存へUI/InputとWindowsAndMessaging featuresのみ追加。新規crateなし。
- SPI_GETKEYBOARDDELAY/SPEEDのdefault値・clamp・repeat計算も同じ。Windows境界のunsafeに必要理由とABI/buffer前提を記載。
- `OutputSession` がmappingのheld keysを所有し、正常終了/read error/setup error時のDropで3-attempt key-up cleanupを実行する。controller復元より先に解放。
- 診断の `--control --output-test` で固定mappingと左右pad mouse/wheelを実行可能。read無受信500msでphysical/gestureをresetし解放するC++の条件を維持。
- Windows fmt/clippy、9件のtest、release build合格。INPUTの各union arm/scan code/mouse flags/負wheelの検証を含む。
- 初回60秒の実入力試験は終了code 0、state 16,056件、non-state 135件、終了時restore成功。ただし累積flagsは `[00, 00, 00, 20]` のみ。ユーザーが操作に間に合わなかったことを確認した。Windows出力の実機gateは未合格。3分間の試験で再確認する。
- 再試験180秒は終了code 0、state 48,181件、non-state 411件、累積flags `[0f, ba, f8, 3f]`。Windows出力status Sent 54件、Failed 0件を記録し、キー解放後のcontroller restore成功。
- ユーザーが文字入力を確認した。mouseに問題があるとの回答だが、空のエディタでスクロール対象がなかった可能性があり、移動/クリック/scrollのどこに問題があるかは未確定。100行のWindows一時試験文書を用意し、native move/wheel成功・失敗件数も診断に追加して再確認する。
- 100行文書での180秒試験は終了code 0、state 48,232件、non-state 408件。ユーザーがLBクリックと右padの上下scrollを確認したが、左padのcursor移動は動かない。move: 成功0/失敗2,850、wheel: 成功3,455/失敗0、key/button status Sent 21/Failed 0。最後のcontroller restoreは成功。
- 原因切り分け: C++ `INPUT{}` / `MOUSEEVENTF_MOVE` / `SendInput(1,...,sizeof(INPUT))` の最小probeもreturn 0 / Windows error 87。C++本体と同じDPI初期化、別console起動、Explorer経由通常起動でも再現。現在位置へのabsolute moveもerror 87。GetCursorPos/SetCursorPosは成功、mouse present=1、pointer設定取得成功。設定変更はしていない。
- Rustにも即時GetLastError捕捉を追加し、HID/mappingを使わない `cargo run --release --example output_probe --target x86_64-pc-windows-msvc` で±1px相対moveがともに `Err(87)`。Win32 INPUT field/ABIのunit testは合格。errorだけからUIPI等の原因を断定しない（[SendInput公式仕様](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)）。
- C++のGUI probeでもDPI/STA COM初期化、window/message pump、worker threadからの送信を比較したが、moveはerror 87のまま。本体の実行条件との差はまだ未確定。
- ユーザーによると、普段のC++版では右padでmouse移動できていた（今回の新しい実機試験ではない）。既存JSONを読み取り専用で確認すると、選択presetは右pad move75%/左pad scroll100%。診断だけに `--mouse-pad right --mouse-sensitivity 75` を追加し、同じpad/感度条件で比較可能にした。既存設定は変更していない。move/wheel送信件数はpad側ではなくevent種別で集計する。
- 右pad move75%/左pad scroll100%の180秒再試験は終了code 0、state 48,350件、non-state 408件、累積flags `[00, 38, 38, 33]`。move成功0/失敗1,150（error 87）、wheel成功550/失敗0、最後のdefault mappings/settings restore成功。ユーザーは診断稼働の合図後に右pad移動だけ不可、左pad scrollとLB clickは正常と確認した。
- この再試験の起動前、PowerShellのnative stderr扱いで最初の起動が中断し、別process起動も短時間で終了した。短時間起動にはrestore logがなく、その後の通常cursor動作をRust出力の成功として扱わない。最終試験は直接起動・stderr分離・Continueで180秒間keepaliveを確認し、正常復元した。
- 最新のfmt/clippy・9件のtest・release buildも合格。現在のC++本体とのfresh比較と、普段のC++版の起動権限の確認が必要。mouse move gate未合格のためPhase 6以降の実装へはまだ進めない。相対入力を別API/absolute方式へ置換して成功扱いにしない。
- ユーザーが今回のC++版で左pad move/LB left click/右pad scrollが正常動作すると確認した。診断defaultもこの条件に合わせる（途中の右pad/RT操作説明に合わせた診断クリック変更は戻した）。
- 起動条件の読み取り比較で、動作中C++版はelevated=1/integrity=0x3000、通常probeはelevated=0/integrity=0x2000。同じsession=1。権限差は確認済みだが、error 87の原因と結論づけるには同じ権限でのprobe比較が必要。ユーザーがUAC承認を含む最小probeを許可。Rust probeをelevated=1/integrity=0x3000で実行したが、左右1pxともerror 87。C++ GUI/worker threadの1px probeも昇格後に同じ失敗。権限差だけでは解消しない。
- 正常動作するC++は `C:\Tools\Remapcon-Windows-x64\Remapcon.exe`、x64/version 1.7.0。installed SHA256 `5A90AD1B0B843A05B0527608CDD1D97F7C39AF5610013619EF176794970BF79D`。importとdisassemblyでmouse moveはINPUT size40/type0/flags1/time0/extra0のSendInput(1,...)を確認し、別APIへのfallbackは見つからない。参照ビルドとのhash差は未特定（ビルド条件によってhashは変わり得る）。
- 同じRust probeをWindows一時directory内でRemapcon.exeへコピーして通常権限で実行してもerror 87。Windows 25H2/build26200.9457。コード・ABI・DPI/GUI/worker thread・権限・実行ファイル名の比較では原因未確定。ユーザーが同じRust probeのcmdをエクスプローラーから直接起動しても、elevated=0/integrity=0x2000で左右1pxともerror 87。手動の通常起動でも失敗を確認した。
- C++版で観測する移動の経路を区別するため、一時directoryにWH_MOUSE_LL observerを作成。WM_MOUSEMOVEのLLMHF_INJECTED/LOWER_IL flag件数だけを記録し、全イベントをCallNextHookExへ渡す。座標、キー入力、対象window、文書等は記録しない。既存C++本体/HID/configを変更せず、60秒でhookを解放する。MSVC /W4 /WXビルドと不正時間引数の拒否を確認済み。初回60秒は記録0件で正常終了（hook_removed=1）。ユーザーは左padを操作したが、現在の左padはscrollだったため、WM_MOUSEMOVEのみの記録では移動経路の判断に使えない。設定はselectedPreset=0/autoMode=true/左scroll2/右move1/両感度100%/base LB=0/RT=0。userの前のpad説明だけで比較条件を決めた点を訂正。observerにwheel件数も追加してビルド確認。ユーザーは右padが動いた際のC++版の状態が「自動待機中／停止中」と確認した。この状態ではShouldHoldController=falseでC++ mapping出力を行わないため、右padの通常cursor動作をC++ SendInput成功の確認として使った比較条件を訂正した。Steam/Lizard/HID等のどの標準入力かは未特定。ユーザーが対象アプリの条件でもC++版のmouse moveが動かず、scrollも動かないと報告した。昨日までは使えていた認識で、昨日以降の変更に心当たりはない。Rust固有の原因と扱う根拠は得られていない。C++ `canMove` は対象exeがforegroundかつpreview=falseの場合に限る（単にC++版が起動していてもpad mappingの有効性は確定しない）。flagの意味は[MSLLHOOKSTRUCT公式仕様](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-msllhookstruct)を参照。
- 前面exeと保存targetをC++と同じfull path/filename・case-insensitive比較で読み取り確認し、一致=true、autoMode=true、左mode2/右mode1だった。query成功でアプリ名/パスは出力していない。対象アプリ判定のずれはこの時点では見つからない。
- Windows bootは2026-09-30 22:42:11、最近のhotfixは09/26 KB5129195・09/21 KB5126052・09/09 KB5124007。過去3日の更新成功eventは見つからない。present mouse devicesは全てOK。mouse class upper filter=mouclass、lowerなし。一部Razer deviceにRzDev_00c0/0244 upper filter、PowerToys/KeyboardManagerEngine等は稼働中だが、これらが原因という根拠はない。設定/driver/常駐processを変更・停止していない。
- Native一時probeでtime=0、GetTickCount明示、MOVE_NOCOALESCEも比較し、左右1px全てreturn0/error87。製品のtimestampやflagsは変更していない。ユーザーによる対象アプリの通常終了後も、Rustの左右1pxは両方Err(87)、C++のreference/明示現在時刻/NOCOALESCEも全てreturn0/error87。対象アプリの起動中だけの影響では説明できない。C++版本体とRustの両方で発生する共有の入力経路の問題として調査中。製品のAPI/flags/時間を変えて成功扱いにはしない。ユーザーがWindowsを再起動して確認すると回答した。再起動後は対象アプリ起動前にWin+Rから `%TEMP%\remapcon-phase5-manual-probe.cmd` を実行し、ビルド済みRust probeの左右1pxがOk(())かErr(87)か比較する。全診断processは終了済み（0件）、hookは解放済み。再起動後、ユーザーが通常権限（elevated=0/integrity=0x2000）のRust probeで左右1pxともOk(())と報告。boot=2026-10-02 20:04:40を確認し、C++ reference/明示現在時刻/NOCOALESCEの全6送信もreturn1/error0。製品コードのAPI変更なしで共通の失敗は解消した。再起動で解消したという観測に留め、根本原因は未特定。右pad移動100%/左pad scroll100%/LB clickの実機再試験を準備。Phase 5のcontroller実入力gateは結果待ち。
- 診断defaultは左pad移動/LB click/右pad scroll。最新Windows fmt/clippy・9件のtest・release buildは合格。
- 再起動後の180秒実機試験は正常終了code0。state47,712件/non-state408件、move成功1,309/失敗0、wheel成功864/失敗0、キー解放後のLizard defaults restore成功。ユーザーも右pad移動/LB click/左pad scrollが全て動いたと確認した。既存keyboard実機確認・native ABI testsと合わせPhase 5のPuck gate合格。API/flagsを変更する回避策は導入していない。Phase 6へ進む。
- 診断固定mappingは既存presetを保存/変更しない。既存JSONとforeground/UIの接続は後続Phase。

## Phase 6 — 設定JSON（合格）

- `config.rs` に既存schemaを移植。serde 1.0.229 / serde_json 1.0.151を追加前に公開metadata・license・MSRVを確認し、Windows x64でビルドした。全依存にMIT選択肢があり、GPL/AGPL依存はない。
- C++ `SettingsJson.inc` / `SimpleJson.h` を変更せず抽出してMSVCでreference executableを作成。公開fixture、旧28ボタン形式、optional既定値、UTF-16文字数、mapping/layer/turbo/sequence境界、重複key、負数・小数、u64、BOM、UTF-8、depth/member/fileサイズの受理・拒否を比較した。
- Windows保存は同directoryの一時fileへwrite/flush後にMoveFileExWで置換。保存先lock・別processが保持する一時file・不正設定について、元file保全と自分の一時fileだけをcleanupするtestが合格。
- 実ユーザー設定3 presets/1 folderを読み取り、Windows TEMP内だけでC++正規化→Rust→C++ round-tripを実施。全JSON値が一致し、元設定のhashは不変。ユーザー設定はfixtureへ転記していない。
- Windows fmt/clippy（warnings禁止）、13 tests（C++ differential比較を含む）、release build合格。CIにもC++ referenceのビルドと比較を追加したが、GitHubでの実行結果は未確認。
- JSON出力の空白・項目順は異なるがschemaと値は互換。C++ writerはUTF-16文字数を制限し、readerはUTF-8 byte数を8MiBに制限するため、Rust writerはreaderが読めない8MiB超のUTF-8出力を保存前に拒否する。旧INI移行・初回既定preset・icon cache移行は後続の起動処理統合で接続する。
- Phase 5内の未合格・保留の記述は再起動前の調査経過。現在のPuck実機gateは合格。

## Phase 7 — 前面アプリ判定（合格）

- C++の `IsTargetForeground`、`ShouldHoldController`、`ShouldTakeControllerFromSteam` と全callerを確認。既存のautoModeは選択中presetのtargetだけを判定し、別presetの自動選択は行わない。
- `app.rs` に100ms cache・target version/window変更による再query・manual/auto/previewの取得/出力条件を移植。`windows_app.rs` にquery-only process access、520 UTF-16 buffer、失敗時false、フルパス/filename照合を移植。
- 比較には既存と同じCRT `_wcsicmp` を使用。Rust lowercaseやWin32 ordinal compareへの置換は行わない。unsafeはCRTとWindows API境界だけ。
- 既存C++ foreground関数もreference executableへ変更せず抽出。実際の同一foreground windowで、TEMP内設定によるfull-path一致true / filename一致true / 別target falseをC++・Rustで比較し、全て一致。元設定hash不変、TEMP試験fileをcleanup済み。
- `.exe` file pickerは同じGetOpenFileNameW/flags/buffer/extension確認を実装。UI接続と選択/cancel/errorの手動確認はPhase 9で行う。
- 新crateなし。既存windows crateへCommon Dialog featureを追加。Windows fmt/clippy・15 tests・debug/release build合格。

## Phase 8 — Steam引き渡し（Puck実機gate合格）

- C++ `DeviceCycle.cpp` 全体とControllerLoopの取得/返却/例外処理を確認。`windows_steam.rs` にpending registry・SetupAPI/Configuration Manager・同一named mutex・独立helper process・inherited down eventを移植。既存HID属性/caps確認とhandle/list所有者を再利用した。
- pending Child/ParentをHKLMへwrite/flushしてからdisable。helperがglobal/config-specific enableを最大3回試し、両node復旧後だけpending keyを削除する。timeout後もhelperをkillせず復旧を継続する。startup recoveryもhelperのmutex内で行う。
- C++のwait/claim/retry時間を維持。350ms state probeは完全なparseではなくreport ID 0x42/0x45判定に合わせる。SteamSessionはsetup失敗/panicを含めrestore→HID close→return cycleを順序保証する。
- `--steam-handoff FILE --seconds 180` で選択targetのforeground出入りを試験可能。診断は入力read/keepaliveのみでkeyboard/mouse出力やconfig保存を行わない。通常権限でadministrator拒否となり、pending key不在を確認した。この診断の事前確認時点ではSteam/C++は0 processだった（実機試験時はSteamを通常起動）。
- Windows fmt/clippy（warnings禁止）・16 tests・debug/release build合格。testsはdevice state変更を行わず、helper argumentとcontroller identity guardを確認する。実際のPnP cycle、Steamから取得/返却、Steam起動中reconnectは未確認。
- 境界での差異: malformed pending REG_SZの奇数byte数はbuffer overflowを避けて拒否する。実際に保存する値は常に偶数byte数のterminated UTF-16。helper numeric handleはusizeとして厳密にparseする。
- Rust CIのPowerShell syntaxとMSVC commandをlocal Windowsで確認。local WindowsにPythonが無いため、生成済みC++ sourceでcommandを検証した。CIは[公式windows-2022 imageに同梱されたPython](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)を使い、追加installerを導入しない。GitHub CI本番実行は未確認。
- 次のgateはSteam起動中・管理者権限でのPuck試験。取得後state、targetを離れた際の復元/返却、Steam側入力の復帰、再入場/再接続とpending key cleanupを確認してからPhase 9へ進む。
- 2026-10-03: ユーザーの継続指示後にSteamを通常起動し、UAC承認を経て180秒診断を開始。Notepad targetへの2回の入場で `taken_from_steam=true`、離脱時のstate件数10,760 / 3,228、両回 `restore_and_return_ok=true` を記録。pending keyは削除済み、present Puck HID device 13件は全てstatus OK。ユーザーはメモ帳では操作できず、Steam等へfocusを戻すと通常操作が復帰すると確認した。診断は意図的にmapping出力を送らないため、この挙動は予定どおり。Steam起動中の再接続試験が残る。
- 同日、準備返答後に2回目の180秒試験を実施。最初の取得で6,618 state、電源OFF中のkeepalive/restoreで0x8007001F（device unavailable）を記録。診断はHIDを閉じて復旧を試み、終了せず再列挙へ戻った。電源ON後に再取得して2,001 state、追加の入場でも1,076 stateを記録し、両離脱でrestore/return成功。ユーザーも電源再投入後のSteam通常操作の復帰を確認した。
- 両試験はcomplete markerまで正常に終了。再接続前の切断中restore失敗を隠さず記録し、再接続後の正常経路と区別した。終了後のdiagnostic process 0・pending keyなし・Puck HID status OKを確認し、Puckの取得/返却/再入場/再接続gate合格。wired/BT/Nereidの実機互換性は未確認のためC++削除条件を満たさない。

## Phase 9 — WebView2／bridge（Puck実機・UI診断合格）

- HTML/CSS/JavaScript/SVGは既存ファイルをそのままincludeし、CMakeと同じplaceholder／version置換を行う。既存UIファイルは変更していない。
- `webview.rs` のTAB／percent／UTF-8デコードとstate JSONを、C++ `WebBridge.inc` の抽出実行と比較した。raw non-ASCII・不正UTF-8・空field・control characterの扱い、全32 button metadata／categories／preset値が一致。
- `editor.rs` にpreset／folder／layer／binding／pad／stick／runtime flag／row clipboard／undo・redoを移植。181操作について、保存失敗時のrollback、返答JSON、mapping generation、30件のhistory上限をC++と比較して一致。必要な独立history／clipboardだけをcloneする。名前照合は既存CRT比較を再利用。
- 数字のbridge解釈はC++の32-bit wcstoulに合わせ、overflow→ULONG_MAX、leading whitespace/sign、embedded NULも保持。CRT iswspaceの全UTF-16範囲を抽出比較し、Unicode空白とU+180Eを受理する点も保持。ウィンドウ配置の5整数はsafe Rustでparseし、C++ scanfと比較。malformed signed overflowだけは拒否して未定義動作を避ける。
- Windows fmt／Clippy（warnings禁止）／18 tests（181操作のC++比較含む）／debug・release all-targets build合格。GitHub CI本番は未実行。
- WebView2接続には `webview2-com 0.39.1` を追加。公開2026-03-11／MSRV1.82／MIT／Windows x64／windows 0.62 ABIを確認。関連macros 0.8.1とthiserror 2.0.21を含め、lockfile内28依存のlicense全文を同梱。crateが含むMicrosoft WebView2 loader 1.0.3800.47のlicense／NOTICEをNuGet原本と照合し、既存LICENSESと同じ条件であることを確認した。
- `windows_webview.rs` にSTA初期化／非同期COM作成／message callback／JSON送信／resize／Close／releaseを局所化。STA guardは!Send/!Syncで移動を防ぎ、成功したCoInitializeExだけをbalancedに解放する。safe protocol／editorへunsafeを追加していない。
- `webview_probe` は実ユーザー設定・WindowPlacement.iniを読むだけの3分表示診断。TEMP専用WebView profileを使い、controller出力・設定保存を行わない。ready／uiReadyを受信し、既存UI描画を確認した。
- 初回表示診断でユーザーが初期サイズの小ささとdrag／最大化後の描画欠落を発見。WM_SIZEはSendMessage／modal resize loop経由でも届くため、GetMessageの戻り値から検出する実装を修正。guard付きのborrowed WebUi pointerをwindow procedureへ渡し、C++と同じ場所でresizeする。guardはWebUi解放前にpointerをclearする。
- 初期表示はC++と同じ既存配置の復元／DPIと作業領域82%によるfallback計算へ変更。配置fileを書き換えない。top edge resize hit testもC++と同じ判定。
- 修正後、native resize／maximize／restoreの実操作で、WebView2の実際のboundsとGetClientRectが一致するregression checkが合格。ユーザーも拡大／最大化が動くと確認。最大化時の一瞬の白黒ちらつきは、C++と同じGDI dark background／DWM attributesを追加し、ユーザーが「表示崩れは気にならなくなった」と確認した。
- `webview_probe --self-test` で既存DOMの新規preset・名前入力・ボタン編集・KeyA捕捉・保存・Ctrl+Z／Ctrl+Yを自動操作。Windows全体へのSendInputやUIファイルの変更を使わず、native bridge／atomic save／読戻し／再描画まで確認して合格。描画完了をpollし、失敗・5秒timeout・未完了終了を成功扱いにしない。
- `webview_probe --edit-test` で設定command processorを既存GUIへ接続。専用TEMP directoryの設定コピーだけにatomic saveし、終了時にそのdirectoryだけcleanupする。5分の手動試験は変更記録なしで期限終了し、ユーザーは画面が見えないと回答。15分へ延長して前面表示で再開し、25回のTEMP保存成功を記録。ユーザーがCtrl+Y／Ctrl+Z／Ctrl+Shift+Zの全てが動くと確認した。正常completeまで終了し、元設定・配置hash不変も確認済み。`windows_runtime.rs` でcontroller loopを接続。OS境界は既存moduleを再利用し、thread間は設定の所有snapshotとchannel・単一stop AtomicBoolで連携。新crate／unsafe／Arc<Mutex>は追加していない。500ms stale reset／4秒disconnect／32・8ms read／40ms preview／2秒keepalive／20秒retryを保持し、key-up→controller restore→HID close→Steam returnの順序を維持。この時点で未完了だったGUI通常出力、icons、single instance／起動時legacy移行、tray／close prompt／配置保存は後述の試験で確認した。updater／Rust release packagingはPhase 10に記載。

- GUI runtimeをWindows x64で接続後、fmt／Clippy／19 tests／debug・release all-targets buildが合格。最新版DOM self-testも完了markerまで合格。
- Puck・Steam起動中・管理者権限の15分GUI診断で、「ボタンを探す」から取得して28種類のbutton indexと両stickの非zero入力を受信。ユーザーが画面の反応と、preview停止後のSteam通常操作の復帰を確認。返却status→自動待機の順序を記録した。元設定／配置を保存しない診断のため、次回表示は既存WindowPlacement.iniの位置・サイズへ戻る。この点は未接続の配置保存と区別する。
- 既存.exe pickerをGUI編集診断へ接続中。選択／同じpathの再選択／キャンセル／保存失敗／undo・redoをC++ command処理と比較する6操作を追加（計187操作）。選択はTEMP設定へ保存し、保存失敗のnative messageとrollbackを維持する。ユーザーが.exe選択／キャンセル／Ctrl+Z・Ctrl+Yの全てが動くと確認した。

- 手動picker診断で×後に画面は消えてもprocessが残る終了不具合を検出。WebView2のnested message pumpでWM_QUITだけに依存しないよう、borrowed contextのclosing flagでouter loopを抜け、worker停止→WebView Close→host HWND破棄の順序へ修正。旧診断は所有processのUI threadへWM_QUITを送り通常completeまで終了させた。修正版はDOM self-test／2秒timer終了がcompleteまで合格し、残存processなし。
- import／exportをGUIのTEMP編集診断へ接続。C++と同じJSON native dialog／overwrite prompt／default filename、CopyFileWによるexportと.before-import.json backup、存在しないfull-path target解除＋auto off、保存失敗時config／requested復元、成功時edited layer初期化、変更時だけhistory消去を保持。C++ ImportSettingsを変更せずreferenceへ抽出し、cancel／不正JSON／backup失敗／最終save失敗／同一config／historyの18操作を追加（計205操作）、全stepのJSON・generation・flagが一致。backup copyのlock失敗で元fileが不変、targetのfile／directory／不存在／filename差異もWindowsのtestで確認。この変更後のfmt／Clippy／19 tests／debug・release all-targets buildが合格。ユーザーが再開した診断でimport／exportが動くと確認。正常completeまで終了し、export JSONは選択中presetのみ試験操作による変更があり、それ以外の全値が元設定と一致、元設定／配置hash不変を確認した。

- `windows_icons.rs` にTargetIcon.cppを移植。Windows Shell／WIC／CryptBinaryToStringWを使い、追加crateなし。CRT towlowerによるUTF-16 FNV名、PNG signature／1MiB、UTF-16 fingerprint／64KiB、process検索32768／SearchPath520、PNG encoder format／cache失敗時fallbackを保持。既存OwnedHandle／atomic write／STA guardを再利用し、diagnosticのicon cacheはTEMPだけへ保存する。
- C++ TargetIcon.cppをそのままreference executableへinclude。System32 NotepadのPNG全byte・data URI・cache名・fingerprintがRustと一致。既存cacheの再使用・case比較・不存在targetでのcache fallback・base64 padding・不正PNG・cache directoryなしも確認。Windows fmt／Clippy／20 tests／debug all-targetsが合格。手動試験中の旧release診断がexeを保持するため、release all-targetsも別TEMP target directoryで合格。GitHub CIのreference linkerへole32／windowscodecsを追加。既存C++ release workflowは変更なし。

- `windows_shell.rs` と `--shell-test` で既存close prompt／トレイ格納・復帰／右click menu／taskbar再生成／WindowPlacement.ini保存を接続。元ICOをincludeし、TEMPへ展開してnative LoadImageで同じsmall sizeを選択。新crateなし。通知iconを消してからHICON／HWNDを解放し、close／tray eventはchannelに保持してCOM・modal pumpでの消失を避ける。設定0=確認／1=トレイ／2=終了、preview解除、最大化状態をminimizeで消さない点をC++に合わせた。
- 配置保存先はTEMPremapcon-phase9-shell-test内のINIのみ。初回は既存INIのコピーから開始し、再起動では診断中の配置を復元する。`--self-test --shell-test` は既存DOMのclose prompt→tray hide→native doubleclick callback→restore→normal位置・サイズsave/load→最大化のままminimizeしてsave/loadを実行し、全stage／正常completeまで合格。元設定／配置hash不変。fmt／Clippy／20 tests／debug・release build合格。右click menu cancel時にstate再描画を送らないC++挙動も保持する。ユーザーがトレイ格納／復帰／再起動後のモニター・位置・サイズ復元を全て確認。2回目の×は元設定どおりトレイへ格納され、右click→終了で正常completeまで終了した。元設定／配置hash不変、残存diagnostic processなし。


- 多重起動は既存CreateMutexW／10回100msのFindWindow待機／SendMessageTimeout1500ms／WM_APP+6通知／UIPI message filterへ移植。handleは既存OwnedHandleで保持し、二重起動はUI・worker・設定sandbox作成前に終了する。UI-ready前の通知をpendingとして保持し、既存alreadyRunningメッセージと画面復帰を行う。`--single-instance-test` は専用mutex／window classを使い、C++と競合しない。mutex duplicate／shutdown後の再取得／NUL拒否のtestを追加。Windows fmt／Clippy／21 tests／debug・release all-targets合格。実際に2回起動し、非表示の既存画面の復帰・notice送信・processが1個だけ・正常終了を確認。最初のPowerShell確認はnull titleが空文字へ変換されたためwindow検索に失敗したが、C#側でnullを渡す確認へ修正して合格。
- `--output-test` は既存TEMP sandboxにメモ帳限定presetを設定して、GUIの通常worker／mapping／SendInput経路を接続する。右pad=mouse／左pad=scroll／LB=left click／A・B・X・Y文字入力、autoとSteam takeoverを有効にする。通常設定は変更しない。ユーザーが文字入力／カーソル／LBクリック／スクロール／Steam通常操作の復帰を全て確認。native送信成功statusと返却を記録、error log空、終了コード0、残存processなし、元設定／配置hash不変。

- 起動時移行を `windows_config.rs` に追加。GetPrivateProfileStringW／IntWとC++と同じUTF-16 bufferで旧INIを読む。binding／turbo／sequence／layer count・旧2layer fallback／target共通指定・v2指定／各fallback値を保持。現行JSON→旧JSON→旧INI→初期presetの優先順位、不正JSONを上書きしないこと、保存成功後だけ配置・PNG/meta cacheをcopy-if-absentする順序を保持。初期preset／bindings constructorはeditorからconfigへ移動して再利用。INIの空preset名など、C++が保存して次回のstrict JSON loadで拒否する値も勝手に修復しない。
- C++ LoadLegacyPresets／LoadSettingsを変更せずreferenceへ抽出し、旧INI 6ケース・startup 7ケースについて全JSON値と配置／cacheコピー結果が一致。TMP競合保存失敗でも元fileを保持。Windows fmt／Clippy／22 tests／debug・release all-targets合格。`--startup-test --self-test --shell-test` はTEMPの旧INI→新JSON→再起動load→native PNGを既存UIのimgでdecode表示→編集／undo・redo／tray／配置復元／正常終了まで自動確認して合格。

## Phase 10 — Updater／packaging（実装・ローカル検証済み、実更新未確認）

- Updater.cpp／UpdateProtocol.h／既存Windows release workflowを参照。タグ・release digestは `update.rs`、既存SimpleJson互換parserはconfigと共用。WinHTTP／BCrypt／system tar／stage-copy→rename→失敗時exe復旧を `windows_update.rs` へ移植。新crateなし。既存windows crateのfeatureだけ追加。従来8fileとRust依存全文 `THIRD_PARTY_LICENSES.txt` を更新する。
- SHA-256はnative [BCryptHash](https://learn.microsoft.com/en-us/windows/win32/api/bcrypt/nf-bcrypt-bcrypthash)（Windows 10+）。既知digestとC++結果に一致。TEMP限定のZIP検証、digest不一致、破損・不足package、配置失敗とexe復旧を確認。GUIにinstallUpdate／openReleases／READY・FAILED通知を接続し、READY応答前にworker終了・controller返却を待つ。実インストール先への更新・GitHubからの実ダウンロードは未確認。
- GUI hostを `windows_gui.rs` に共用し、通常 `Remapcon.exe` と `RemapconUpdater.exe` を追加。既存app.rc／ICO／version resourceを使用し、Version 1.7.0を確認。通常起動は既存設定・profile・配置を使用する。比較用 `--data-dir ABSOLUTE_DIRECTORY` で一時設定も使用可能。C++版と同じmutex／window classで二重起動を防ぐ。
- Windows x64 fmt／Clippy（warnings拒否）／25 tests／debug・release all-targetsが合格。CRTを静的リンクし、両GUI exeのx64／GUI subsystem／DLL一覧を確認。Visual C++ Redistributableへの依存なし。WebView2 Runtimeは引き続き必要。
- `rust/scripts/package.ps1` がRust専用候補ZIPとSHA-256を作成。LICENSE、Dylan Deverill／ro-ba・SteamlessController由来表示、THIRD_PARTY_NOTICES、Rust全文license、WebView2 license／noticeの同梱を確認。Rust updaterと変更なしのC++ updaterが候補ZIPを受理し、不一致digestを拒否した。
- 配布候補 `Remapcon.exe --diagnostic --startup-test --self-test --shell-test --seconds 30` で旧設定移行・native icon表示・編集／undo・redo／tray／配置保存を自動確認しexit 0。候補ZIPをTEMPへ展開し、HID取得しない一時設定で通常起動→最大化・最小化→終了→再起動も確認。最大化状態が復元され、2回ともerror log空・exit 0。PowerShellで非同期GUI processのExitCodeがnullとなる最初の確認は、process handleを保持するharnessへ修正して再試験した。
- 追加の `--update-test` はTEMP設定・controller未取得で、診断中のinstallUpdate拒否code 6→既存JSへの到達、未開始READY拒否、FAILED code 3→既存JSへの到達・進行state解除、開始済みREADY→終了ack 1を自動確認して合格。FAILED受信処理がtray有効時に限られていた点を共通bridgeで修正。試行したWebView wake追加は不要と分かったため取り除いた。新dependencyなし、UI assets変更なし。
- ApplyPackageを変更なしのC++参照へ追加。配置成功・LICENSE不足・Updater.exeの置換lockによる失敗について、従来8file／exe復旧／backup／残存stageの結果がRustと一致。Rust追加notice fileだけは意図した差異。
- `rust/scripts/check-gui.ps1` でstartup／編集・履歴／tray・配置とupdater IPCを再実行可能。2つともexit 0・error log空。実ダウンロード、実install先への更新、controllerを保持した更新終了は未確認。
- Rust CIにpackage・artifact生成を追加。既存C++ production release workflowは変更していない。GitHub CIの実行と実際の更新／再起動は未確認のため、production releaseの切替は行わない。

## 実機確認の保留一覧（外出中は実施しない）

ユーザーの2026-10-03の指示により、実機操作が必要な試験を保留。自動確認の成功で実機合格へ置き換えない。帰宅後は準備の返答と開始の合図を揃えて1項目ずつ実施する。

| 未確認項目 | 再開時の確認内容 |
| --- | --- |
| USB wired | C++とRustの列挙条件、read／全入力、Lizard off／restore、haptics、抜き差し、mapping出力 |
| Bluetooth／Nereid | 機材とC++対応範囲を確認後、transport別report／feature／再接続を照合。非対応を推測実装しない |
| 通常配布GUIの実機 | 専用設定でPuck入力・mapping・layers／turbo／sequence・Steamへの返却を確認 |
| updater実更新 | テスト配布元・専用install先でダウンロード→digest→終了通知→置換→再起動、権限必要時のUACも確認 |

GitHub上のRust CI／artifact確認は別の未完了項目。現行productionはC++ ZIPのため、Rust依存noticeを含む配布物と切替手順を確認してからRust updaterの実更新を試験する。

## 2026-10-04 — PadMux改名後のUSB実機確認

ユーザー帰宅後、Puckを外し、Steamと旧C++版を通常終了してUSBデータケーブルで接続。以下は改名前のPuck試験とは別の記録。

- Rustが1 interfaceを列挙。VID `28de`、PID `1302`、Usage `ff00:0001`、wired、input 54／output 64／feature 64 bytes。
- 変更なしのC++ HidDevice／SteamControllerを使用するread-only参照probeも1 interface、wired、output／feature 64、state 54 bytes／report ID `0x42`、1秒で247 states。Rustの列挙・caps・受信reportと一致。参照probeはfeature writeを行わない。
- Rust read-only 180秒で44,671 states／52 other reports、累積flags `[af,bf,fe,3f]`、exit 0／stderr空。A/B/X/Y、D-pad、L3/R3、L4/L5/R4/R5、LB/RB、grip、両pad touch／click、両stick touchを記録。追加操作後、両stickのXは±32767、左Yは−31401〜25201、右Yは±32767を確認。
- control／haptics 30秒で7,438 states、exit 0／stderr空。Lizard disable、左tick／click→右tick／click、2秒間隔keepalive 14回、正常終了時default mappings／settings restoreを記録。Menu／View／Steamと両triggerの0〜32767も受信。
- ユーザーが「振動を感じ、終了後のマウス操作も戻った」と確認。USBの入力read／制御／振動／通常復帰は合格。抜き差し・mapping出力・Steam handoffは別途確認する。

USB reconnect 180秒も合格。ユーザーがケーブル抜き差しと操作を確認。切断時の `0x8007048F` とdevice不在を記録し、1秒間隔で再列挙後、同じwired interfaceを再取得してLizard disableを再実行。再接続後20,641 states／24 other reports、十字キー・両pad等の入力、keepalive再開、終了時default restore、exit 0を確認。stderrは意図した切断／再試行の記録のみ。ログはTEMP内 `padmux-wired-reconnect-65cbd7cf-ce9b-4595-a709-311f359bbb10`。mapping出力・Steam handoffは未確認。

USB keyboard／mouse出力180秒も合格。ユーザーがA/B/X/Y文字入力、右padカーソル、LB左クリック、左padスクロールの4項目を確認。SendInputのmove成功590／失敗0、wheel成功717／失敗0、key／button送信status成功、44,671 states／52 other reports、Lizard default restore、exit 0／stderr空。ログはTEMP内 `padmux-wired-output-f5036de4-4ae0-4873-ae56-32fc1aced12d`。専用console診断のmappingであり、通常配布GUI／Steam起動中のwired handoffは別gate。

Steam起動中のwired handoff 180秒も合格。管理者診断でメモ帳→Steamを2回切り替え、各回 `taken_from_steam=true`、2,073／744 states、`restore_and_return_ok=true` を記録。ユーザーが2回ともSteam通常操作の復帰を確認。診断はOS入力・設定保存を行わず、正常complete／exit 0、PendingDeviceCycle記録なし、旧設定JSON／配置SHA-256不変。ログはTEMP内 `padmux-wired-handoff-7dfc520e-ace2-46a2-99e6-ae2b5bf7829e`。Steam起動中のwired再接続は別途確認する。

Steam起動中のwired再接続180秒も合格。取得中にUSBを抜き、1,945 statesの旧sessionはdevice disconnected `0x8007048F`、切断中のrestore／Steam返却失敗を明示。再接続後に再取得（`taken_from_steam=false`）し2,210 statesを受信、`restore_and_return_ok=true`、normal complete／exit 0。ユーザーがSteam通常操作の復帰を確認。実装が使用するHKLMの `SOFTWARE\Remapcon\PendingDeviceCycle` が存在しないことも確認し、エラー文の「pending recovery record is retained」を実際の残存証拠として扱わない。ログはTEMP内 `padmux-wired-steam-reconnect-38da1cb7-276d-497c-8a27-eb5795bc788f`。

ユーザーに確認した機材はUSB／Puckのみ。Bluetooth／Nereidは実機未確認として保持し、非対応・互換を推測しない。

通常配布候補ZIPから展開した `padmux.exe --data-dir TEMP\data` のPuck実機試験も合格。専用Notepad presetでユーザーがA/B/X/Y文字、右padカーソル、LB左クリック、左padスクロールとSteam通常操作復帰を確認し、×で正常終了。通常host／workerでSteam取得→自動有効→入力送信→返却→自動待機、host complete／exit 0、PadMux残存processなし、HKLM PendingDeviceCycleなしを記録。試験設定／WebView profileはTEMP内 `padmux-packaged-hardware-915d085e-f24b-4f41-b025-9c47c5269d68` のみ。基本試験logと設定は `basic-gui.log`／`basic-settings.json` に保存し、追加のlayer／turbo／sequence試験と区別する。

同じ通常配布候補でPuckのlayer／turbo／sequence追加試験も合格。専用設定でA→a、RB+A→c／RB release+A→a、B長押し→200ms周期のb連射／release停止、X長押し→300ms間隔abcを1回／再押下で再実行をユーザーが確認。Steam通常操作復帰と×での終了も確認。Windows送信成功status、Steam返却→自動待機、host complete／exit 0、残存GUIなし、HKLM PendingDeviceCycleなし、旧設定JSON／配置hash不変。記録は同じTEMP directoryの `mapping-gui.log`／`mapping-settings.json`。

ログはユーザーTEMP内の `padmux-wired-read-e08e5937-4d5b-4fec-870f-fcd101d2311b`、`padmux-wired-control-2f8ba6f6-cc98-4d66-94d4-b6f0e14365c9`、`padmux-wired-cpp-reference` に保持。設定変更・新crate追加・C++実装削除は行っていない。

## 2026-10-04 — 残る配布／更新gate

- 手元のUSB／Puckについて、入力・制御・出力・Steam取得／返却・再接続、および通常配布候補GUIのlayers／turbo／sequencesを上記の通り確認済み。Bluetooth／Nereidは機材がなく未確認。
- GitHub最新releaseをread-onlyで確認。`ro-ba/padmux` の `v1.7.0` は旧C++ `Remapcon-Windows-x64.zip` のみ（公開2026-09-30、SHA-256 `fb0e5aaaeaf2837b488087b7fdb22ebcee2e53c984bc31d79a82bda2cb5ef960`）。新updaterが要求する `padmux-windows-x64.zip` は存在しない。
- そのためGitHub経由のRust package実download→install→restartを合格としない。旧assetを改名してRust packageと見せかけることも行わない。テスト配布元・候補releaseを確定してから実更新する。既存のZIP検証／配置・復旧／updater IPCの合格記録はPhase 10を参照。
- Rust workflow／ローカル候補ZIPは作成済みだが、未commit／未pushのためGitHub上で今回の変更を実行していない。GitHub CI／artifactと実更新の確認前にproduction releaseをRustへ切り替えず、C++実装を削除しない。

## 2026-10-04 — GitHub CI／artifact確認

- 作業branch `ro-ba/rewright-rust-project` にmigration／改名をcommitしpush。初回pushはOAuthのworkflow scope不足で拒否されたが、ユーザーが追加認証を承認して解消。mainへのmerge／release公開は行っていない。
- 初回Rust CIはPythonのcp1252既定decodeで参照C++生成に失敗。`reference_config.py`／`reference_mapping.py` のread／writeをUTF-8へ明示した `7296ead` で修正。非UTF-8既定を再現したチェックが合格し、生成C++ bytesは従来のUTF-8環境と同一。
- [Rust CI 37138317362](https://github.com/ro-ba/padmux/actions/runs/37138317362) 成功。fmt、変更なしのC++参照build、Clippy（warnings拒否）、25 tests、debug／release build、help、package／static CRT／licenses／ZIP／digest検証、artifact uploadが合格。
- 同じcommitの [C++ CI 37138534212](https://github.com/ro-ba/padmux/actions/runs/37138534212) をbranch指定のworkflow_dispatchで実行し成功。C++ GUI／updater Release build、CPack、tag／digest／ZIP内容検証、artifact uploadが合格。tag release公開stepはbranch実行のためskip。
- 実際のCI artifactをTEMPへdownload。Rust ZIP SHA-256 `7b80b3ff82cc8be25eb7c6acb8e3ed846f0474a5e08b142734ea2ed2566018e6`、C++ ZIP SHA-256 `d020e1e0b1a6508d39dbb404ff22c426d70092d3ddb7f6e2b154690853cf49a8`。両MIT copyright／SteamlessController notice、Rust依存全文、PadMux native resourceを確認。
- CI製C++／Rust双方のupdaterが両ZIPを受理し、全組合せで誤digestを拒否。CI製Rust exeを `check-gui.ps1` でstartup／編集・履歴／tray・配置、updater IPCまで実行。両ケースexit 0／stderr空。HID取得／OS入力／本来の設定保存は行わない。ログはTEMP `padmux-gui-check-6e523534-e106-441a-8d5f-0af0660a370b`。
- 実GitHub download→controller返却／GUI終了→exe配置→再起動の一連の実更新は引き続き未確認。GitHub最新releaseは旧C++ ZIPのみのため、CI成功を実更新成功と置き換えない。テスト候補の公開と試験方法を確定してから進める。

## Phase 11 — C++削除前の未完了gate

- PuckのHID／再接続／制御／出力／Steam handoff、設定互換、mapping trace、WebView UIの確認は上記の通り合格。
- wiredの列挙／入力／制御／出力／再接続／Steam handoffは2026-10-04の上記実機試験で合格。通常配布GUIのPuck入力・layer／turbo／sequence・Steam返却も合格。GitHub上のC++／Rust build／packagingとCI artifact GUI検証も合格。updater実更新が残る。Bluetooth／Nereidは機材がなく未確認。C++ source／CMakeを保持し、現時点では削除しない。

## 差異・license

- protocolや設定schema、UIは変更していない。HID readはC++のtimeout/error=0をdiagnosticで区別する。
- cancelはC++のthread単位 `CancelIo` に代わり、該当OVERLAPPED指定の `CancelIoEx`。完了drain後にbufferを破棄する前提は同じ。
- CAPS取得失敗はRust openではerror、partial output writeもerrorとして扱う。復元の失敗時も両default commandを試す。正常経路のreport bytes/順序はC++に合わせる。
- unsafeは `windows_hid.rs` / `windows_output.rs` / `windows_config.rs` / `windows_app.rs` / `windows_steam.rs` / `windows_webview.rs` / `windows_icons.rs` / `windows_shell.rs`  / `windows_gui.rs` / `windows_update.rs` のnative境界だけ。parser/command/haptic limiter/controller session/mapping/config/app policyはunsafe禁止。
- `windows 0.62.2`、serde/serde_jsonとlockfile内28依存を確認。全crateでMIT選択可能、`unicode-ident` の追加Unicode-3.0条件も保持。各copyright/license文は `rust/THIRD_PARTY_LICENSES.txt`。
- SteamlessController固定commit、Dylan Deverill / ro-ba、MITを翻訳moduleへ記載。root LICENSE/attributionを維持し、THIRD_PARTY_NOTICESにRust依存の表示を追記。
- C++ source/CMake/UI/既存release workflowは変更していない。GitHub上のRust CI実行と全transportの実機検証は未実施。
