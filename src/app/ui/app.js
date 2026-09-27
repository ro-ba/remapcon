// @ts-check
(() => {
  'use strict';
  const $ = (id) => document.getElementById(id);
  const bridge = window.chrome && window.chrome.webview;
  const send = (command, ...parts) => {
    if (bridge) bridge.postMessage([command, ...parts].map(value =>
      encodeURIComponent(String(value))).join('\t'));
  };
  const t = {
    ja: {
      inputTest: 'ボタンを探す', inputTestHelp: 'コントローラーのボタンを押すと、対応する設定行が光ります。確認中はキー入力を送信しません。',
      stopInputTest: '確認を終了', enable: '有効にする', disableMode: '停止する',
      foldersPresets: 'フォルダとプリセット', newFolder: '＋ フォルダ', newPreset: '＋ プリセット',
      copy: '複製', rename: '名前変更', delete: '削除', targetApp: 'このプリセットの対象アプリ',
      changeApp: 'アプリを変更', clear: '解除', autoMode: '前面で自動有効', editLayer: '編集するレイヤー',
      inputGroup: '入力の種類', doubleClickHint: '行をダブルクリックして編集', controller: 'コントローラー',
      assignedTo: '割り当て先', type: '設定種別', action: '動作', keyInput: 'キー入力',
      sequence: 'シーケンス', layerSwitch: 'レイヤー切替', disabled: '無効', inherit: '引き継ぐ',
      sentKey: '送信するキー', mouseButton: 'マウスボタン', mouseLeft: '左クリック', mouseRight: '右クリック', mouseMiddle: '中クリック', capture: 'キーを押す', turbo: '連打する', interval: '間隔（ms）',
      delay: '初回遅延（ms）', sequenceKeys: 'キーの順序', addKey: '＋ キーを追加',
      removeLast: '最後を削除', repeatHeld: '押している間繰り返す', destinationLayer: '切り替え先レイヤー',
      cancel: 'キャンセル', save: '保存', name: '名前', folder: 'フォルダ', settings: '設定',
      language: '言語', settingsData: '設定データ', settingsDataHelp: 'すべてのプリセットとフォルダを移動します。',
      import: 'インポート', export: 'エクスポート', resetBinding: '割り当てをリセット', resetBindingTitle: '割り当てをリセット', resetLayerPrompt: 'このボタンに割り当てたレイヤーも削除します。続けますか？', unset: '未設定', normal: '通常',
      all: 'すべて', newLayer: '＋ レイヤー', root: 'フォルダなし', createLayer: '新しいレイヤー',
      layer: 'レイヤー', pressed: '入力を検出', waiting: '接続待機中', capturePrompt: 'キーを押してください…',
      duplicateName: '同じ名前が使われています。', invalidName: '名前を入力してください。',
      deletePresetTitle: 'プリセットを削除', deleteLayerTitle: 'レイヤーを削除',
      deleteFolderTitle: 'フォルダを削除', deletePrompt: '「{name}」を削除しますか？',
      clearTargetTitle: '対象アプリの指定を解除',
      clearTargetPrompt: '「{name}」の指定を解除しますか？ボタンの割り当ては残ります。',
      importTitle: '設定をインポート', importPrompt: '現在の設定をバックアップしてから、選択した設定で置き換えます。',
      nextLayer: '切り替え先', turboLabel: '連打',
      selectedLayer: '編集するレイヤー', selectInput: '入力を選択',
      moveToFolder: 'フォルダへ移動', destinationFolder: '移動先', createFolderAndMove: '新規フォルダを作成して移動',
      parentFolder: '親フォルダ', newFolderName: '新しいフォルダ名', move: '移動',
      padSettings: 'トラックパッドの指移動', padExplanation: '左右の指移動は独立設定です。タップと押し込みは一覧から別に割り当てできます。',
      leftPadMove: '左パッドの指移動', rightPadMove: '右パッドの指移動', padOff: 'オフ', padMouse: 'マウス移動', padScroll: 'スクロール',
      padMotionType: '指移動', folderNotEmpty: '中身があるフォルダは削除できません。',
      leftPadSensitivity: '左の感度（%）', rightPadSensitivity: '右の感度（%）',
      padSensitivityHelp: '100%が標準。マウス移動とスクロールの両方に適用します。',
      invalidSensitivity: '感度は25〜400%で入力してください。',
      stickInfo: '左右スティックの傾きは十字キーと別に設定できます。L3/R3の押し込みは基本ボタンで設定します。',
      stickSettings: 'スティックの反応範囲', stickSettingType: '感度設定',
      stickExplanation: '中心の円がデッドゾーンです。斜めの重なり幅を広げると、2方向を同時に入力しやすくなります。確認中はキーを送信しません。',
      leftStick: '左スティック', rightStick: '右スティック', deadzone: 'デッドゾーン', overlap: '斜めの重なり幅',
      invalidStickSetting: '0〜32767の整数で入力してください。', noDirection: '入力なし',
      closeBehavior: '閉じるボタンの動作', closeAsk: '毎回確認', closeTray: 'トレイに格納',
      steamCoexistenceTitle: 'Steamとの併用（試験的）',
      steamCoexistenceOption: '対象アプリの前面表示中、Steamとの共有アクセスを試す',
      steamCoexistenceHelp: '先にSteamのデスクトップレイアウトを「空」にしてください。設定が残ると二重入力やオンスクリーンキーボードが起きます。Steamゲームのレイアウトは別です。',
      closeExit: '終了', closeTitle: 'アプリを閉じる',
      closeExplanation: 'トレイに格納すると、アプリは動作を続けます。通知領域のアイコンから再表示・終了できます。',
      adminWarning: '管理者として実行されていません。ゲーム内で入力が効かない場合は、管理者としてアプリを再起動してください。',
      dismissWarning: '警告を閉じる',
      alreadyRunningTitle: 'アプリは起動中です', alreadyRunningExplanation: '開いているウィンドウに切り替えました。', ok: 'OK',
      updates: 'アップデート', currentVersion: '現在のバージョン', checkUpdates: '更新を確認',
      viewRelease: 'リリースを見る', updateChecking: '最新版を確認しています…',
      updateAvailable: '新しいバージョン {version} を利用できます。',
      updateCurrent: '最新版を使用しています。', updateAhead: '公開版より新しいビルドを使用しています。',
      updateError: '確認できませんでした。ネット接続を確認して、もう一度お試しください。',
      updateOpenError: 'リリースページを開けませんでした。',
      installUpdate: '更新して再起動', updateConfirmTitle: '更新して再起動',
      updateConfirmText: 'Remapcon {version} をダウンロードして検証します。準備ができたらアプリを終了し、更新後に再起動します。',
      updateInstalling: '更新ファイルをダウンロード・検証しています。完了後に再起動します…',
      updateInstallError: '更新に失敗しました。もう一度お試しいただくか、リリースページから手動で更新してください。',
      updateNetworkError: 'GitHubへの接続に失敗しました。ネットワークを確認してください。（E1）',
      updateDownloadError: '更新用ZIPをダウンロードできませんでした。（E3）',
      updateExtractError: '更新用ZIPを展開できませんでした。（E5）',
      updateStartError: '更新用プログラムを起動できませんでした。RemapconUpdater.exeがアプリと同じフォルダにあるか確認してください。（E6）',
      updateChecksumError: 'ダウンロードしたZIPのSHA-256が一致しません。更新は中止しました。',
      updateReleaseError: '最新版が変わりました。もう一度「更新を確認」を押してください。',
    },
    en: {
      inputTest: 'Find button', inputTestHelp: 'Press a controller button to highlight its settings row. No key input is sent while checking.',
      stopInputTest: 'Stop test', enable: 'Enable', disableMode: 'Stop',
      foldersPresets: 'Folders and presets', newFolder: '+ Folder', newPreset: '+ Preset',
      copy: 'Duplicate', rename: 'Rename', delete: 'Delete', targetApp: 'Target app for this preset',
      changeApp: 'Choose app', clear: 'Clear', autoMode: 'Auto enable in target app', editLayer: 'Layer to edit',
      inputGroup: 'Input group', doubleClickHint: 'Double-click a row to edit', controller: 'Controller',
      assignedTo: 'Assignment', type: 'Type', action: 'Action', keyInput: 'Key input',
      sequence: 'Sequence', layerSwitch: 'Layer switch', disabled: 'Disabled', inherit: 'Inherit',
      sentKey: 'Key to send', mouseButton: 'Mouse button', mouseLeft: 'Left click', mouseRight: 'Right click', mouseMiddle: 'Middle click', capture: 'Press a key', turbo: 'Turbo', interval: 'Interval (ms)',
      delay: 'Initial delay (ms)', sequenceKeys: 'Key sequence', addKey: '+ Add key',
      removeLast: 'Remove last', repeatHeld: 'Repeat while held', destinationLayer: 'Destination layer',
      cancel: 'Cancel', save: 'Save', name: 'Name', folder: 'Folder', settings: 'Settings',
      language: 'Language', settingsData: 'Settings data', settingsDataHelp: 'Transfer every preset and folder.',
      import: 'Import', export: 'Export', resetBinding: 'Reset binding', resetBindingTitle: 'Reset binding', resetLayerPrompt: 'This also deletes the layer assigned to this button. Continue?', unset: 'Not set', normal: 'Normal',
      all: 'All', newLayer: '+ Layer', root: 'No folder', createLayer: 'New layer',
      layer: 'Layer', pressed: 'Input detected', waiting: 'Waiting for controller', capturePrompt: 'Press a key…',
      duplicateName: 'This name is already used.', invalidName: 'Enter a name.',
      deletePresetTitle: 'Delete preset', deleteLayerTitle: 'Delete layer',
      deleteFolderTitle: 'Delete folder', deletePrompt: 'Delete “{name}”?',
      clearTargetTitle: 'Clear target app',
      clearTargetPrompt: 'Clear “{name}” as the target app? Button mappings will remain.',
      importTitle: 'Import settings', importPrompt: 'The current settings will be backed up, then replaced with the selected settings.',
      nextLayer: 'Switch to', turboLabel: 'Turbo',
      selectedLayer: 'Layer to edit', selectInput: 'Select an input',
      moveToFolder: 'Move to folder', destinationFolder: 'Destination', createFolderAndMove: 'Create a folder and move',
      parentFolder: 'Parent folder', newFolderName: 'New folder name', move: 'Move',
      padSettings: 'Trackpad finger movement', padExplanation: 'Left and right movement are independent. Set tap and press actions separately in the input list.',
      leftPadMove: 'Left pad movement', rightPadMove: 'Right pad movement', padOff: 'Off', padMouse: 'Move mouse', padScroll: 'Scroll',
      padMotionType: 'Finger movement', folderNotEmpty: 'The folder must be empty before deletion.',
      leftPadSensitivity: 'Left sensitivity (%)', rightPadSensitivity: 'Right sensitivity (%)',
      padSensitivityHelp: '100% is standard. Applies to mouse movement and scrolling.',
      invalidSensitivity: 'Enter sensitivity from 25 to 400%.',
      stickInfo: 'Stick directions have separate bindings from the D-pad. L3/R3 clicks are basic buttons.',
      stickSettings: 'Stick response', stickSettingType: 'Response settings',
      stickExplanation: 'The center circle is the deadzone. A wider diagonal overlap makes two directions activate together more easily. No keys are sent while testing.',
      leftStick: 'Left stick', rightStick: 'Right stick', deadzone: 'Deadzone', overlap: 'Diagonal overlap',
      invalidStickSetting: 'Enter an integer from 0 to 32767.', noDirection: 'No input',
      closeBehavior: 'Close button action', closeAsk: 'Ask every time', closeTray: 'Minimize to tray',
      steamCoexistenceTitle: 'Use alongside Steam (experimental)',
      steamCoexistenceOption: 'Try shared access while the target app is in front',
      steamCoexistenceHelp: 'First set Steam’s Desktop Layout to Empty. Otherwise Steam may send duplicate input or open its on-screen keyboard. Steam game layouts are separate.',
      closeExit: 'Exit', closeTitle: 'Close the app',
      closeExplanation: 'The app keeps running in the tray. Use its notification icon to reopen or exit.',
      adminWarning: 'This app is not running as administrator. If inputs do not work in your game, restart the app as administrator.',
      dismissWarning: 'Dismiss warning',
      alreadyRunningTitle: 'Already running', alreadyRunningExplanation: 'Switched to the open window.', ok: 'OK',
      updates: 'Updates', currentVersion: 'Current version', checkUpdates: 'Check for updates',
      viewRelease: 'View release', updateChecking: 'Checking for updates…',
      updateAvailable: 'Version {version} is available.',
      updateCurrent: 'You are using the latest version.', updateAhead: 'This build is newer than the public release.',
      updateError: 'Could not check for updates. Check your connection and try again.',
      updateOpenError: 'Could not open the release page.',
      installUpdate: 'Update and restart', updateConfirmTitle: 'Update and restart',
      updateConfirmText: 'Download and verify Remapcon {version}. When ready, the app will close, update, and restart.',
      updateInstalling: 'Downloading and verifying the update. The app will restart when ready…',
      updateInstallError: 'Update failed. Try again or update manually from the release page.',
      updateNetworkError: 'Could not connect to GitHub. Check your network connection. (E1)',
      updateDownloadError: 'Could not download the update ZIP. (E3)',
      updateExtractError: 'Could not extract the update ZIP. (E5)',
      updateStartError: 'Could not start the updater. Check that RemapconUpdater.exe is in the app folder. (E6)',
      updateChecksumError: 'The ZIP SHA-256 did not match. The update was stopped.',
      updateReleaseError: 'The latest release changed. Check for updates again.',
    },
  };
  let state = null;
  let category = 0;
  let editingButton = -1;
  let naming = null;
  let captureTarget = '';
  let capturedKey = 0;
  let sequenceKeys = [];
  let collapsed = false;
  let toastTimer = 0;
  let lastInputSignature = "";
  let movingPreset = -1;
  let pendingDelete = null;
  let treeSignature = '';
  let startupReadySent = false;
  let warningDismissed = false;
  let updateStatus = 'idle';
  let latestVersion = '';
  let updateFailure = '';
  let stickPreviewOwned = false;
  const stickPosition = {left:{x:0,y:0}, right:{x:0,y:0}};
  const stickHeld = {left:false, right:false};
  const stickDiagonal = {left:false, right:false};
  const openFolders = new Set();
  const targetIcons = new Map();
  const requestedIcons = new Set();
  const targetIconCheckAt = new Map();
  const lang = () => state && state.language === 'en' ? 'en' : 'ja';
  const tr = key => t[lang()][key] || key;
  const parseVersion = value => {
    const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(value);
    return match ? match.slice(1).map(Number) : null;
  };
  const compareVersions = (current, latest) => {
    const a = parseVersion(current), b = parseVersion(latest);
    if (!a || !b || [...a, ...b].some(n => !Number.isSafeInteger(n))) return null;
    for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] < b[i] ? -1 : 1;
    return 0;
  };
  function renderUpdateStatus() {
    const status = $('update-status');
    status.dataset.state = updateStatus;
    status.textContent = updateStatus === 'idle' ? '' :
      updateStatus === 'installError' ? tr(updateFailure || 'updateInstallError') :
      tr('update' + updateStatus[0].toUpperCase() + updateStatus.slice(1))
        .replace('{version}', latestVersion);
    $('check-updates').disabled = updateStatus === 'checking' || updateStatus === 'installing';
    const available = updateStatus === 'available' ||
      (updateStatus === 'installError' && updateFailure !== 'updateReleaseError');
    $('open-release').hidden = !available;
    $('install-update').hidden = !available;
  }
  async function checkUpdates() {
    if (updateStatus === 'checking' || updateStatus === 'installing') return;
    updateStatus = 'checking'; renderUpdateStatus();
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);
    try {
      const response = await fetch('https://api.github.com/repos/ro-ba/remapcon/releases/latest', {
        headers: {Accept: 'application/vnd.github+json'}, cache: 'no-store', signal: controller.signal
      });
      if (!response.ok) throw new Error('GitHub response: ' + response.status);
      const release = await response.json();
      const version = release.tag_name;
      if (typeof version !== 'string') throw new Error('Missing release version');
      const result = compareVersions($('app-version').textContent.trim(), version);
      if (result === null) throw new Error('Invalid release version');
      latestVersion = version;
      updateStatus = result < 0 ? 'available' : result > 0 ? 'ahead' : 'current';
    } catch (_) {
      updateStatus = 'error';
    } finally {
      clearTimeout(timeout);
      renderUpdateStatus();
    }
  }
  const esc = value => String(value).replace(/[&<>"']/g, ch => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[ch]));
  const targetName = path => String(path).split(/[\\/]/).pop();
  const englishButtons = {18:'Left pad click',19:'Left pad tap',20:'Right pad click',21:'Right pad tap',22:'Menu',23:'View',24:'Right stick ↑',25:'Right stick ↓',26:'Right stick ←',27:'Right stick →',28:'Left stick ↑',29:'Left stick ↓',30:'Left stick ←',31:'Left stick →'};
  const buttonName = index => lang() === 'en' && englishButtons[index] ? englishButtons[index] : state.buttons[index].name;
  const englishStatus = {
    '停止中：Steam Inputを使用できます':'Stopped: Steam Input is available',
    '自動待機中：対象アプリを前面にすると有効になります':'Waiting for the target app',
    'コントローラー待機中：接続を確認してください':'Waiting for the controller',
    '取得できません：Steamなどが使用中です':'Controller is in use by Steam or another app',
    '共有アクセス中：Steamなどの入力が重複する可能性があります':'Shared access: Steam or another app may send duplicate input',
    'Lizard Modeを無効化できませんでした':'Could not disable Lizard Mode',
    '自動有効：対象アプリへ入力します':'Auto mode active in target app',
    '有効：ボタン入力待ちです':'Active: waiting for controller input',
    '入力確認中：ボタンを押してください':'Input test: press a controller button',
    '通信が切れました：再接続します':'Disconnected: reconnecting',
    '予期しないエラー：アプリを再起動してください':'Unexpected error: restart the app',
    'ボタン入力を検出：設定した入力をWindowsに送信しました':'Sent mapped input to Windows',
    '入力送信に失敗：対象アプリの権限を確認してください':'Could not send input: check app permissions',
  };
  const statusText = value => lang() === 'en' ? (englishStatus[value] || value) : value;
  const showSharedStatus = value => $('connection').classList.toggle('shared', value.startsWith('共有アクセス中'));
  const compactStatus = value => {
    const translated = statusText(value);
    const prefix = value.split('：')[0];
    const labels = lang() === 'en' ? {
      '停止中':'Stopped','自動待機中':'Auto standby','コントローラー待機中':'Waiting',
      '取得できません':'Unavailable','自動有効':'Auto active','有効':'Active',
      '共有アクセス中':'Shared access',
      '入力確認中':'Input test','通信が切れました':'Disconnected',
      '入力送信に失敗':'Send failed','ボタン入力を検出':'Input sent'
    } : {
      '停止中':'停止中','自動待機中':'自動待機中','コントローラー待機中':'接続待ち',
      '取得できません':'取得不可','自動有効':'自動有効','有効':'有効',
      '共有アクセス中':'共有中',
      '入力確認中':'入力確認中','通信が切れました':'切断',
      '入力送信に失敗':'送信失敗','ボタン入力を検出':'入力送信済み'
    };
    return labels[prefix] || translated;
  };
  const current = () => state && state.presets[state.selectedPreset];
  const layerData = () => state.editedLayer === 0 ? current() : current().layers[state.editedLayer - 1];
  const displayKey = code => {
    if (code === 0) return tr('disabled');
    if (code === 0xffffffff) return tr('inherit');
    if (code >= 0x20001 && code <= 0x20003) return tr(['mouseLeft','mouseRight','mouseMiddle'][code - 0x20001]);
    const japanese = lang() === 'ja';
    const names = {0x1c:'Enter',0x1001c:japanese ? 'テンキー Enter' : 'Numpad Enter',
      0x39:'Space',0x01:'Esc',0x0f:'Tab',0x0e:'Backspace',
      0x1004b:'←',0x1004d:'→',0x10048:'↑',0x10050:'↓',
      0x10053:'Delete',0x10052:'Insert',0x10047:'Home',0x1004f:'End',
      0x10049:'Page Up',0x10051:'Page Down',
      0x2a:japanese ? '左Shift' : 'Left Shift',0x36:japanese ? '右Shift' : 'Right Shift',
      0x1d:japanese ? '左Ctrl' : 'Left Ctrl',0x1001d:japanese ? '右Ctrl' : 'Right Ctrl',
      0x38:japanese ? '左Alt' : 'Left Alt',0x10038:japanese ? '右Alt' : 'Right Alt',
      0x0c:'-',0x0d:'=',0x1a:'[',0x1b:']',0x27:';',0x28:"'",0x29:'`',
      0x2b:'\\',0x33:',',0x34:'.',0x35:'/',0x56:'Intl Backslash',
      0x53:japanese ? 'テンキー .' : 'Numpad .',
      0x10035:japanese ? 'テンキー /' : 'Numpad /',
      0x37:japanese ? 'テンキー *' : 'Numpad *',
      0x4a:japanese ? 'テンキー -' : 'Numpad -',
      0x4e:japanese ? 'テンキー +' : 'Numpad +'};
    if (names[code]) return names[code];
    const chars = {0x1e:'A',0x30:'B',0x2e:'C',0x20:'D',0x12:'E',0x21:'F',0x22:'G',0x23:'H',
      0x17:'I',0x24:'J',0x25:'K',0x26:'L',0x32:'M',0x31:'N',0x18:'O',0x19:'P',0x10:'Q',
      0x13:'R',0x1f:'S',0x14:'T',0x16:'U',0x2f:'V',0x11:'W',0x2d:'X',0x15:'Y',0x2c:'Z'};
    if (chars[code]) return chars[code];
    if (code >= 0x02 && code <= 0x0b) return String((code - 1) % 10);
    const numpad = {0x52:0,0x4f:1,0x50:2,0x51:3,0x4b:4,
      0x4c:5,0x4d:6,0x47:7,0x48:8,0x49:9};
    if (Object.prototype.hasOwnProperty.call(numpad, code))
      return (japanese ? 'テンキー ' : 'Numpad ') + numpad[code];
    if (code >= 0x3b && code <= 0x44) return 'F' + (code - 0x3a);
    if (code === 0x57 || code === 0x58) return 'F' + (code - 0x4c);
    return 'Scan ' + code.toString(16).toUpperCase();
  };
  const scan = event => {
    const letters = {KeyA:0x1e,KeyB:0x30,KeyC:0x2e,KeyD:0x20,KeyE:0x12,KeyF:0x21,KeyG:0x22,
      KeyH:0x23,KeyI:0x17,KeyJ:0x24,KeyK:0x25,KeyL:0x26,KeyM:0x32,KeyN:0x31,KeyO:0x18,
      KeyP:0x19,KeyQ:0x10,KeyR:0x13,KeyS:0x1f,KeyT:0x14,KeyU:0x16,KeyV:0x2f,KeyW:0x11,
      KeyX:0x2d,KeyY:0x15,KeyZ:0x2c};
    const other = {Enter:0x1c,NumpadEnter:0x1001c,Space:0x39,Escape:0x01,Tab:0x0f,
      Backspace:0x0e,ArrowLeft:0x1004b,ArrowRight:0x1004d,ArrowUp:0x10048,ArrowDown:0x10050,
      Delete:0x10053,Insert:0x10052,Home:0x10047,End:0x1004f,PageUp:0x10049,PageDown:0x10051,
      Digit1:0x02,Digit2:0x03,Digit3:0x04,Digit4:0x05,Digit5:0x06,
      Digit6:0x07,Digit7:0x08,Digit8:0x09,Digit9:0x0a,Digit0:0x0b,
      Minus:0x0c,Equal:0x0d,BracketLeft:0x1a,BracketRight:0x1b,
      Semicolon:0x27,Quote:0x28,Backquote:0x29,Backslash:0x2b,
      Comma:0x33,Period:0x34,Slash:0x35,IntlBackslash:0x56,
      Numpad0:0x52,Numpad1:0x4f,Numpad2:0x50,Numpad3:0x51,
      Numpad4:0x4b,Numpad5:0x4c,Numpad6:0x4d,Numpad7:0x47,
      Numpad8:0x48,Numpad9:0x49,NumpadDecimal:0x53,
      NumpadDivide:0x10035,NumpadMultiply:0x37,NumpadSubtract:0x4a,NumpadAdd:0x4e,
      ShiftLeft:0x2a,ShiftRight:0x36,ControlLeft:0x1d,ControlRight:0x1001d,
      AltLeft:0x38,AltRight:0x10038,F1:0x3b,F2:0x3c,F3:0x3d,F4:0x3e,F5:0x3f,F6:0x40,
      F7:0x41,F8:0x42,F9:0x43,F10:0x44,F11:0x57,F12:0x58};
    return letters[event.code] || other[event.code] || 0;
  };
  function toast(message) { const node = $('alert'); node.textContent = message; node.hidden = false;
    clearTimeout(toastTimer); toastTimer = setTimeout(() => { node.hidden = true; }, 3500); }
  function translate() {
    document.documentElement.lang = lang();
    document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = tr(el.dataset.i18n); });
  }
  function folderChildren(parent) {
    return state.folders.filter(path => path !== parent &&
      (parent ? path.startsWith(parent + '/') : true) &&
      path.slice(parent ? parent.length + 1 : 0).indexOf('/') < 0);
  }
  function hideContext() { $('context-menu').hidden = true; }
  function showContext(event, items) {
    event.preventDefault(); event.stopPropagation();
    const menu = $('context-menu'); menu.innerHTML = '';
    items.forEach(item => {
      const button = document.createElement('button'); button.type = 'button';
      button.textContent = item.label; button.className = item.danger ? 'danger' : '';
      button.disabled = !!item.disabled; button.setAttribute('role','menuitem');
      button.onclick = () => { hideContext(); item.action(); };
      menu.append(button);
    });
    menu.hidden = false;
    const bounds = menu.getBoundingClientRect();
    menu.style.left = Math.max(8, Math.min(event.clientX, innerWidth - bounds.width - 8)) + 'px';
    menu.style.top = Math.max(8, Math.min(event.clientY, innerHeight - bounds.height - 8)) + 'px';
    menu.querySelector('button:not(:disabled)')?.focus();
  }
  function confirmDeletePreset(index) {
    openDeleteDialog('deletePreset', index, state.presets[index].name);
  }
  function confirmDeleteLayer(index) {
    openDeleteDialog('layer-delete', index, current().layers[index].name);
  }
  function confirmDeleteFolder(path) {
    const occupied = state.folders.some(folder => folder.startsWith(path + '/')) ||
      state.presets.some(preset => preset.folder === path);
    if (occupied) { toast(tr('folderNotEmpty')); return; }
    openDeleteDialog('deleteFolder', path, path);
  }
  function requestBindingReset(index) {
    if (!state || index < 0 || index >= state.buttons.length) return;
    const trigger = state.editedLayer === 0 &&
      current().layers.some(layer => layer.trigger === index);
    if (trigger) openDeleteDialog('resetBinding', index, buttonName(index));
    else send('resetBinding', index, state.editedLayer, state.selectedPreset);
  }
  function openDeleteDialog(command, target, name) {
    pendingDelete = {command, target, name, presetIndex: state.selectedPreset, layerIndex: state.editedLayer};
    const title = command === 'deletePreset' ? 'deletePresetTitle' :
      command === 'layer-delete' ? 'deleteLayerTitle' :
      command === 'resetBinding' ? 'resetBindingTitle' :
      command === 'clearTarget' ? 'clearTargetTitle' : 'deleteFolderTitle';
    $('delete-title').textContent = tr(title);
    $('delete-explanation').textContent = command === 'resetBinding' ? tr('resetLayerPrompt') :
      tr(command === 'clearTarget' ? 'clearTargetPrompt' : 'deletePrompt').replace('{name}', name);
    $('delete-confirm').textContent = command === 'resetBinding' ? tr('resetBinding') :
      command === 'clearTarget' ? tr('clear') : tr('delete');
    $('delete-confirm').classList.toggle('danger-confirm', command === 'clearTarget');
    $('delete-dialog').hidden = false;
    $('delete-cancel').focus();
  }
  function openMove(index) {
    movingPreset = index;
    const fill = (id) => {
      const select = $(id); select.innerHTML = '';
      const root = document.createElement('option'); root.value = ''; root.textContent = tr('root'); select.append(root);
      state.folders.forEach(path => { const option = document.createElement('option');
        option.value = path; option.textContent = path; select.append(option); });
    };
    fill('move-folder'); fill('move-parent');
    $('move-folder').value = state.presets[index].folder;
    $('move-folder').disabled = false;
    $('move-parent').value = state.presets[index].folder;
    $('move-create-new').checked = false; $('move-new-fields').hidden = true;
    $('move-new-name').value = '';
    $('move-dialog').hidden = false; $('move-folder').focus();
  }
  function dropPreset(event, destination) {
    event.preventDefault(); event.stopPropagation();
    document.querySelectorAll('.drop-target').forEach(node => node.classList.remove('drop-target'));
    const raw = event.dataTransfer.getData('text/plain');
    if (!/^\d+$/.test(raw)) return;
    const index = Number(raw);
    if (index >= 0 && index < state.presets.length && state.presets[index].folder !== destination)
      send('movePreset', index, destination);
  }
  function makeDropTarget(node, destination) {
    node.ondragover = event => { if (!event.dataTransfer.types.includes('text/plain')) return;
      event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = 'move';
      node.classList.add('drop-target'); };
    node.ondragleave = event => { if (!node.contains(event.relatedTarget)) node.classList.remove('drop-target'); };
    node.ondrop = event => dropPreset(event, destination);
  }
  function renderTree() {
    const tree = $('preset-tree'); tree.innerHTML = '';
    const makePreset = index => {
      const preset = state.presets[index];
      const button = document.createElement('button'); button.type = 'button';
      button.className = 'preset-row' + (index === state.selectedPreset ? ' active' : '');
      button.dataset.preset = String(index);
      button.innerHTML = '<strong>' + esc(preset.name) + '</strong><small>' + esc(preset.target ? targetName(preset.target) : tr('unset')) + '</small>';
      button.title = preset.target || '';
      button.onclick = () => {
        if (index === state.selectedPreset) return;
        state.selectedPreset = index; state.editedLayer = 0;
        updateTreeSelection(); renderDetails();
        send('selectPreset', index);
      };
      button.oncontextmenu = event => showContext(event, [
        {label:tr('rename'), action:()=>openName('preset-rename', String(index))},
        {label:tr('copy'), action:()=>openName('preset-copy', String(index))},
        {label:tr('moveToFolder') + '…', action:()=>openMove(index)},
        {label:tr('delete'), danger:true, disabled:state.presets.length <= 1,
          action:()=>confirmDeletePreset(index)},
      ]);
      button.draggable = true;
      button.ondragstart = event => { hideContext(); event.dataTransfer.setData('text/plain', String(index));
        event.dataTransfer.effectAllowed = 'move'; };
      button.ondragend = () => document.querySelectorAll('.drop-target').forEach(node => node.classList.remove('drop-target'));
      makeDropTarget(button, preset.folder);
      return button;
    };
    const appendFolder = (path, host) => {
      const wrapper = document.createElement('div'); wrapper.className = 'tree-folder';
      const row = document.createElement('div'); row.className = 'folder-row';
      const name = path.split('/').pop();
      const expanded = !openFolders.has('!' + path);
      const toggle = document.createElement('button'); toggle.type = 'button'; toggle.className = 'folder-toggle';
      toggle.setAttribute('aria-expanded', String(expanded)); toggle.textContent = (expanded ? '▾ ' : '▸ ') + name;
      toggle.onclick = () => {
        const next = toggle.getAttribute('aria-expanded') !== 'true';
        if (next) openFolders.delete('!' + path); else openFolders.add('!' + path);
        toggle.setAttribute('aria-expanded', String(next));
        toggle.textContent = (next ? '▾ ' : '▸ ') + name;
        children.hidden = !next;
      };
      row.append(toggle);
      row.oncontextmenu = event => showContext(event, [
        {label:tr('newFolder'), action:()=>openName('folder-new', path)},
        {label:tr('rename'), action:()=>openName('folder-rename', path)},
        {label:tr('delete'), danger:true, action:()=>confirmDeleteFolder(path)},
      ]);
      makeDropTarget(row, path);
      const actions = document.createElement('span'); actions.className = 'folder-actions';
      const addChild = document.createElement('button'); addChild.type = 'button'; addChild.textContent = '+';
      addChild.setAttribute('aria-label', tr('newFolder')); addChild.onclick = () => openName('folder-new', path);
      actions.append(addChild); row.append(actions); wrapper.append(row);
      const children = document.createElement('div'); children.className = 'folder-children'; children.hidden = !expanded;
      folderChildren(path).forEach(child => appendFolder(child, children));
      state.presets.forEach((preset, index) => { if (preset.folder === path) children.append(makePreset(index)); });
      wrapper.append(children); host.append(wrapper);
    };
    folderChildren('').forEach(folder => appendFolder(folder, tree));
    const root = document.createElement('div'); root.className = 'root-drop'; root.textContent = tr('root');
    makeDropTarget(root, ''); tree.append(root);
    state.presets.forEach((preset, index) => { if (!preset.folder) tree.append(makePreset(index)); });
  }
  function updateTreeSelection() {
    document.querySelectorAll('.preset-row').forEach(row =>
      row.classList.toggle('active', Number(row.dataset.preset) === state.selectedPreset));
  }
  function renderLayers() {
    const host = $('layer-tabs'); host.innerHTML = '';
    const add = (label, index) => {
      const button = document.createElement('button'); button.type = 'button';
      button.textContent = label; button.className = state.editedLayer === index ? 'active' : '';
      button.setAttribute('role','tab'); button.setAttribute('aria-selected', String(index === state.editedLayer));
      button.onclick = () => send('selectLayer', index);
      if (index) {
        const layerIndex = index - 1;
        button.ondblclick = () => openName('layer-rename', String(layerIndex));
        button.oncontextmenu = event => showContext(event, [
          {label:tr('rename'), action:()=>openName('layer-rename', String(layerIndex))},
          {label:tr('delete'), danger:true, action:()=>confirmDeleteLayer(layerIndex)},
        ]);
      }
      host.append(button);
    };
    add(tr('normal'), 0);
    current().layers.forEach((layer,index) => add((layer.name || tr('layer') + (index + 1)) +
      (layer.trigger < state.buttons.length ? ' (' + buttonName(layer.trigger) + ')' : ''), index + 1));
    const plus = document.createElement('button'); plus.type = 'button'; plus.textContent = tr('newLayer');
    plus.onclick = () => openName('layer-new',''); host.append(plus);
  }
  function renderCategory() {
    const select = $('category'); const old = String(category); select.innerHTML = '';
    const all = document.createElement('option'); all.value = '-1'; all.textContent = tr('all'); select.append(all);
    const englishCategories = ['Basic buttons','D-pad','Back buttons','Trackpads','Menu and View','Sticks'];
    state.categories.forEach((name,index) => { const option = document.createElement('option'); option.value = String(index); option.textContent = lang() === 'en' ? englishCategories[index] : name; select.append(option); });
    select.value = old;
  }
  function padModeName(mode) {
    return tr(mode === 1 ? 'padMouse' : mode === 2 ? 'padScroll' : 'padOff');
  }
  function paintSensitivityRange(range) {
    range.style.setProperty('--range-progress', ((Number(range.value) - 25) / 375 * 100) + '%');
  }
  function openPadDialog() {
    $('left-pad-mode').value = String(current().leftPadMode);
    $('right-pad-mode').value = String(current().rightPadMode);
    $('left-pad-sensitivity').value = String(current().leftPadSensitivity || 100);
    $('right-pad-sensitivity').value = String(current().rightPadSensitivity || 100);
    $('left-pad-sensitivity-range').value = $('left-pad-sensitivity').value;
    $('right-pad-sensitivity-range').value = $('right-pad-sensitivity').value;
    paintSensitivityRange($('left-pad-sensitivity-range'));
    paintSensitivityRange($('right-pad-sensitivity-range'));
    $('pad-dialog').hidden = false; $('left-pad-mode').focus();
  }
  function renderStick(side) {
    if ($('stick-dialog').hidden) return;
    const {x,y} = stickPosition[side];
    const deadzone = Math.max(0, Math.min(32767, Number($(`${side}-stick-deadzone`).value) || 0));
    const overlap = Math.max(0, Math.min(32767, Number($(`${side}-stick-overlap`).value) || 0));
    const ax = Math.abs(x), ay = Math.abs(y);
    const threshold = stickHeld[side] ? Math.max(0, deadzone - 1500) : deadzone;
    stickHeld[side] = (ax || ay) && ax * ax + ay * ay >= threshold * threshold;
    const diagonal = stickHeld[side] && Math.min(ax, ay) >= Math.max(1500, Math.floor(Math.max(ax, ay) / 10)) &&
      Math.abs(ax - ay) <= overlap + (stickDiagonal[side] ? 700 : 0);
    stickDiagonal[side] = diagonal;
    const directions = [];
    if (stickHeld[side]) {
      if (ay >= ax || diagonal) directions.push(y > 0 ? '↑' : '↓');
      if (ax >= ay || diagonal) directions.push(x > 0 ? '→' : '←');
    }
    $(`${side}-stick-direction`).textContent = directions.length ? directions.join(' + ') : tr('noDirection');
    $(`${side}-stick-dot`).style.left = (50 + Math.max(-1, Math.min(1, x / 32767)) * 46) + '%';
    $(`${side}-stick-dot`).style.top = (50 - Math.max(-1, Math.min(1, y / 32767)) * 46) + '%';
    $(`${side}-stick-dot`).classList.toggle('active', !!directions.length);
    $(`${side}-stick-zone`).style.width = $(`${side}-stick-zone`).style.height = (deadzone / 32767 * 92) + '%';
  }
  function openStickDialog() {
    for (const side of ['left','right']) {
      for (const setting of ['deadzone','overlap']) {
        const value = current()[`${side}Stick${setting[0].toUpperCase()}${setting.slice(1)}`];
        $(`${side}-stick-${setting}`).value = String(value);
        $(`${side}-stick-${setting}-range`).value = String(value);
        paintStickRange($(`${side}-stick-${setting}-range`));
      }
      stickPosition[side] = {x:0,y:0}; stickHeld[side] = false; stickDiagonal[side] = false;
    }
    $('stick-dialog').hidden = false;
    for (const side of ['left','right']) renderStick(side);
    stickPreviewOwned = !state.preview;
    if (stickPreviewOwned) send('setPreview', 1);
    $('left-stick-deadzone-range').focus();
  }
  function paintStickRange(range) { range.style.setProperty('--range-progress', (Number(range.value) / 32767 * 100) + '%'); }
  function renderRows() {
    const host = $('rows'); host.innerHTML = '';
    if (category === -1 || category === 5) {
      const row = document.createElement('button'); row.type = 'button'; row.className = 'input-row pad-mode-row';
      row.innerHTML = '<strong>' + esc(tr('stickSettings')) + '</strong><span class="subtle">' +
        esc(tr('deadzone')) + ' · ' + esc(tr('overlap')) + '</span><span class="kind">' + esc(tr('stickSettingType')) + '</span>';
      row.ondblclick = openStickDialog;
      row.onkeydown = event => { if (event.key === 'Enter') { event.preventDefault(); openStickDialog(); } };
      host.append(row);
    }
    if (category === -1 || category === 3) {
      for (const side of ['left','right']) {
        const row = document.createElement('button'); row.type = 'button'; row.className = 'input-row pad-mode-row';
        const mode = side === 'left' ? current().leftPadMode : current().rightPadMode;
        const sensitivity = side === 'left' ? current().leftPadSensitivity : current().rightPadSensitivity;
        const description = padModeName(mode) + (mode ? ' · ' + (sensitivity || 100) + '%' : '');
        row.innerHTML = '<strong>' + esc(tr(side === 'left' ? 'leftPadMove' : 'rightPadMove')) +
          '</strong><span><span class="keycap">' + esc(description) +
          '</span></span><span class="kind">' + esc(tr('padMotionType')) + '</span>';
        row.ondblclick = openPadDialog;
        row.onkeydown = event => { if (event.key === 'Enter') { event.preventDefault(); openPadDialog(); } };
        host.append(row);
      }
    }
    $('stick-info').hidden = category !== 5;
    const layer = layerData();
    const buttonOrder = state.buttons.map((_, index) => index);
    if (buttonOrder.length >= 32) buttonOrder.splice(24, 8, 28, 29, 30, 31, 24, 25, 26, 27);
    buttonOrder.forEach(index => {
      const button = state.buttons[index];
      if (category >= 0 && button.category !== category) return;
      const triggerLayer = current().layers.findIndex(item => item.trigger === index);
      if (state.editedLayer && triggerLayer >= 0) return;
      const sequence = layer.sequence[index]; const turbo = layer.turbo[index];
      let value = layer.mapping[index]; let kind = tr('keyInput'); let assignment = displayKey(value);
      if (triggerLayer >= 0) { kind = tr('layerSwitch'); assignment = current().layers[triggerLayer].name || tr('layer') + (triggerLayer + 1); }
      else if (sequence.enabled) { kind = tr('sequence'); assignment = sequence.keys.map(displayKey).join(' → '); }
      else if (value === 0 || value === 0xffffffff) kind = value === 0 ? tr('disabled') : tr('inherit');
      else if (value >= 0x20001 && value <= 0x20003) kind = tr('mouseButton');
      else if (turbo.enabled) kind = tr('turboLabel');
      const row = document.createElement('button'); row.type = 'button'; row.className = 'input-row'; row.dataset.button = String(index);
      row.innerHTML = '<strong>' + esc(buttonName(index)) + '</strong><span><span class="keycap">' + esc(assignment) + '</span></span><span class="kind">' + esc(kind) + '</span>';
      row.ondblclick = () => openEditor(index);
      row.oncontextmenu = event => showContext(event, [
        {label:tr('resetBinding'), danger:true, action:()=>requestBindingReset(index)},
      ]);
      row.onkeydown = event => { if (event.key === 'Enter') { event.preventDefault(); openEditor(index); } };
      host.append(row);
    });
  }
  function clearHighlight() {
    document.querySelectorAll('.input-row.pressed').forEach(row => row.classList.remove('pressed'));
    lastInputSignature = '';
  }
  function renderTargetIcon() {
    const path = current().target;
    const icon = targetIcons.get(path);
    $('target-icon').hidden = !icon;
    if (icon) {
      if ($('target-icon').src !== icon) $('target-icon').src = icon;
    } else $('target-icon').removeAttribute('src');
    if (path && !requestedIcons.has(path) &&
        Date.now() >= (targetIconCheckAt.get(path) || 0)) {
      requestedIcons.add(path);
      send('getTargetIcon', path);
    }
  }
  function renderDetails() {
    renderLayers(); renderRows();
    $('preset-title').textContent = current().name;
    $('preset-path').textContent = current().folder;
    $('target-name').textContent = current().target ? targetName(current().target) : tr('unset');
    $('target-name').title = current().target || '';
    $('target-clear').disabled = !current().target;
    renderTargetIcon();
    $('auto').checked = state.autoMode;
    clearHighlight();
    $('mode').hidden = state.autoMode;
    $('mode').disabled = !current().target;
    $('mode').textContent = state.requested ? tr('disableMode') : tr('enable');
    $('preview').textContent = state.preview ? tr('stopInputTest') : tr('inputTest');
    $('preview').classList.toggle('primary', state.preview);
    $('language').value = lang();
    $('close-behavior').value = String(state.closeBehavior ?? 0);
    $('steam-shared-mode').checked = !!state.steamSharedMode;
    $('connection').textContent = compactStatus(state.status || tr('waiting'));
    showSharedStatus(state.status || '');
    $('connection').title = statusText(state.status || tr('waiting'));
    $('status').textContent = statusText(state.status || '');
    document.body.classList.add('ui-ready');
  }
  function render() {
    if (!state) return;
    translate();
    renderUpdateStatus();
    $('admin-warning').hidden = state.elevated !== false || warningDismissed;
    $('admin-warning-close').setAttribute('aria-label', tr('dismissWarning'));
    const nextSignature = JSON.stringify([lang(), state.folders,
      state.presets.map(preset => [preset.name, preset.folder, preset.target])]);
    if (nextSignature !== treeSignature) { renderTree(); treeSignature = nextSignature; }
    else updateTreeSelection();
    renderCategory(); renderDetails();
  }
  function close(id) {
    $(id).hidden = true; captureTarget = '';
    if (id === 'delete-dialog') pendingDelete = null;
    if (id === 'stick-dialog' && stickPreviewOwned) { stickPreviewOwned = false; send('setPreview', 0); }
  }
  function openName(mode, target) {
    naming = {mode,target};
    const source = mode.startsWith('preset') && mode !== 'preset-new' ?
      state.presets[Number(target)] : current();
    $('name-title').textContent = mode.includes('folder') ? tr('folder') : mode.includes('layer') ? tr('layer') : tr('name');
    $('name-input').value = mode === 'preset-copy' ? source.name + ' - Copy' :
      mode === 'preset-rename' ? source.name : mode === 'folder-rename' ? target.split('/').pop() :
      mode === 'layer-rename' ? source.layers[Number(target)].name : '';
    const selector = $('folder-selector'); selector.innerHTML = '';
    const option = (path,label) => { const node = document.createElement('option'); node.value = path; node.textContent = label; selector.append(node); };
    option('',tr('root')); state.folders.forEach(path => option(path,path));
    selector.value = mode === 'preset-new' || mode === 'preset-copy' || mode === 'preset-rename' ? source.folder : '';
    $('folder-selector-row').hidden = !mode.startsWith('preset');
    $('name-dialog').hidden = false; $('name-input').focus(); $('name-input').select();
  }
  function openEditor(index) {
    editingButton = index;
    const preset = current(); const layer = layerData();
    const trigger = state.editedLayer === 0 ? preset.layers.findIndex(entry => entry.trigger === index) : -1;
    const sequence = layer.sequence[index]; const turbo = layer.turbo[index];
    const value = layer.mapping[index];
    $('editor-title').textContent = buttonName(index);
    $('editor-layer').textContent = state.editedLayer === 0 ? tr('normal') : preset.layers[state.editedLayer - 1].name;
    $('edit-role').querySelector('[value=layer]').hidden = state.editedLayer !== 0;
    $('edit-role').querySelector('[value=inherit]').hidden = state.editedLayer === 0;
    $('edit-role').querySelector('[value=mouse]').hidden = index < 18 || index > 21;
    $('edit-role').value = trigger >= 0 ? 'layer' : sequence.enabled ? 'sequence' : value === 0 ? 'off' : value === 0xffffffff ? 'inherit' : value >= 0x20001 && value <= 0x20003 ? 'mouse' : 'key';
    capturedKey = value === 0xffffffff || (value >= 0x20001 && value <= 0x20003) ? 0 : value;
    $('edit-mouse-button').value = String(value >= 0x20001 && value <= 0x20003 ? value : 0x20001);
    $('edit-key').value = displayKey(capturedKey);
    $('edit-turbo').checked = turbo.enabled;
    $('edit-interval').value = String(turbo.intervalMs);
    $('edit-delay').value = String(turbo.delayMs);
    sequenceKeys = [...sequence.keys]; $('sequence-interval').value = String(sequence.intervalMs);
    $('sequence-repeat').checked = sequence.repeat;
    const target = $('edit-layer-target'); target.innerHTML = '';
    preset.layers.forEach((entry,i) => { const option = document.createElement('option'); option.value = String(i); option.textContent = entry.name || tr('layer') + (i+1); target.append(option); });
    const newOption = document.createElement('option'); newOption.value = String(preset.layers.length); newOption.textContent = tr('createLayer'); target.append(newOption);
    target.value = String(trigger >= 0 ? trigger : preset.layers.length);
    updateEditorRole(); renderSequence(); $('editor').hidden = false; $('edit-role').focus();
  }
  function updateEditorRole() {
    const role = $('edit-role').value;
    $('edit-key-group').hidden = role !== 'key';
    $('edit-mouse-group').hidden = role !== 'mouse';
    $('edit-sequence-group').hidden = role !== 'sequence';
    $('edit-layer-group').hidden = role !== 'layer';
    $('turbo-fields').hidden = !$('edit-turbo').checked;
  }
  function renderSequence() { $('sequence-keys').innerHTML = sequenceKeys.map(key => '<span>' + esc(displayKey(key)) + '</span>').join(''); }
  $('sidebar-toggle').onclick = () => { collapsed = !collapsed; $('layout').classList.toggle('collapsed', collapsed); };
  $('titlebar').addEventListener('mousedown', event => {
    if (event.button !== 0 || event.target.closest('button')) return;
    send('windowDrag');
  });
  $('win-min').onclick = () => send('windowMinimize');
  $('win-max').onclick = () => send('windowToggleMaximize');
  $('win-close').onclick = () => send('windowClose');
  $('admin-warning-close').onclick = () => { warningDismissed = true; $('admin-warning').hidden = true; };
  $('close-to-tray').onclick = () => { close('close-dialog'); send('closeToTray'); };
  $('close-exit').onclick = () => send('quit');
  $('delete-confirm').onclick = () => {
    if (!pendingDelete || !state) return;
    const {command, target, name, presetIndex, layerIndex} = pendingDelete;
    const stillMatches = command === 'resetBinding' ?
      state.selectedPreset === presetIndex && state.editedLayer === pendingDelete.layerIndex &&
      target >= 0 && target < state.buttons.length :
      command === 'clearTarget' ? state.selectedPreset === presetIndex && current().target === target :
      command === 'deletePreset' ? state.presets[target]?.name === name :
      command === 'layer-delete' ? state.selectedPreset === presetIndex && current()?.layers[target]?.name === name :
      state.folders.includes(target) && !state.folders.some(folder => folder.startsWith(target + '/')) &&
        !state.presets.some(preset => preset.folder === target);
    close('delete-dialog');
    if (stillMatches) {
      if (command === 'resetBinding') send(command, target, layerIndex, presetIndex);
      else if (command === 'clearTarget') send(command);
      else send(command, target);
    }
  };
  $('import-confirm').onclick = () => { close('import-dialog'); close('settings'); send('import'); };
  $('preset-new').onclick = () => openName('preset-new','');
  $('folder-new').onclick = () => openName('folder-new','');
  $('target-pick').onclick = () => send('pickTarget');
  $('target-clear').onclick = () => {
    if (state && current().target) openDeleteDialog('clearTarget', current().target, targetName(current().target));
  };
  $('auto').onchange = event => send('setAuto', event.target.checked ? 1 : 0);
  $('mode').onclick = () => send('toggleMode');
  $('preview').onclick = () => { if (state.preview) clearHighlight(); send('setPreview', state.preview ? 0 : 1); };
  setInterval(() => { if (state && document.visibilityState === 'visible') renderTargetIcon(); }, 30000);
  $('category').onchange = event => { category = Number(event.target.value); renderRows(); };
  $('settings-open').onclick = () => { $('settings').hidden = false; $('language').focus(); };
  $('check-updates').onclick = checkUpdates;
  $('open-release').onclick = () => send('openReleases');
  $('install-update').onclick = () => {
    if (updateStatus !== 'available' && updateStatus !== 'installError') return;
    $('update-confirm-text').textContent = tr('updateConfirmText').replace('{version}', latestVersion);
    $('update-dialog').hidden = false;
    $('update-cancel').focus();
  };
  $('update-confirm').onclick = () => {
    if (!latestVersion) return;
    close('update-dialog');
    updateStatus = 'installing'; renderUpdateStatus();
    send('installUpdate', latestVersion);
  };
  $('move-create-new').onchange = event => { $('move-new-fields').hidden = !event.target.checked;
    $('move-folder').disabled = event.target.checked;
    if (event.target.checked) $('move-new-name').focus(); };
  $('move-save').onclick = () => {
    if (movingPreset < 0) return;
    if ($('move-create-new').checked) {
      const name = $('move-new-name').value.trim(); if (!name) { toast(tr('invalidName')); return; }
      send('movePresetNewFolder', movingPreset, $('move-parent').value, name);
    } else send('movePreset', movingPreset, $('move-folder').value);
    close('move-dialog'); movingPreset = -1;
  };
  $('pad-save').onclick = () => {
    const left = Number($('left-pad-sensitivity').value);
    const right = Number($('right-pad-sensitivity').value);
    if (!Number.isInteger(left) || !Number.isInteger(right) || left < 25 || left > 400 ||
        right < 25 || right > 400) { toast(tr('invalidSensitivity')); return; }
    send('setPadConfig', $('left-pad-mode').value, $('right-pad-mode').value, left, right);
    close('pad-dialog');
  };
  $('stick-save').onclick = () => {
    const fields = ['left-stick-deadzone','right-stick-deadzone','left-stick-overlap','right-stick-overlap'];
    const values = fields.map(id => Number($(id).value));
    if (fields.some(id => $(id).value.trim() === '') ||
        values.some(value => !Number.isInteger(value) || value < 0 || value > 32767)) {
      toast(tr('invalidStickSetting')); return;
    }
    send('setStickConfig', ...values); close('stick-dialog');
  };
  for (const side of ['left','right']) for (const setting of ['deadzone','overlap']) {
    const range = $(`${side}-stick-${setting}-range`), number = $(`${side}-stick-${setting}`);
    range.oninput = () => { number.value = range.value; paintStickRange(range); renderStick(side); };
    number.oninput = () => {
      const value = Number(number.value);
      if (number.value !== '' && Number.isFinite(value)) {
        range.value = String(Math.max(0, Math.min(32767, value)));
        paintStickRange(range); renderStick(side);
      }
    };
  }
  for (const side of ['left','right']) {
    const range = $(`${side}-pad-sensitivity-range`);
    const number = $(`${side}-pad-sensitivity`);
    range.oninput = () => { number.value = range.value; paintSensitivityRange(range); };
    number.oninput = () => {
      const value = Number(number.value);
      if (number.value !== '' && Number.isFinite(value)) {
        range.value = String(Math.max(25, Math.min(400, value)));
        paintSensitivityRange(range);
      }
    };
  }
  document.addEventListener('click', event => { if (!$('context-menu').contains(event.target)) hideContext(); });
  window.addEventListener('blur', clearHighlight);
  $('language').onchange = event => send('setLanguage', event.target.value);
  $('close-behavior').onchange = event => send('setCloseBehavior', event.target.value);
  $('steam-shared-mode').onchange = event => send('setSteamSharedMode', event.target.checked ? 1 : 0);
  $('import').onclick = () => { $('import-dialog').hidden = false; $('import-confirm').focus(); };
  $('export').onclick = () => { close('settings'); send('export'); };
  document.querySelectorAll('[data-close]').forEach(button => button.onclick = () => close(button.dataset.close));
  document.querySelectorAll('.scrim').forEach(scrim => scrim.onclick = event => { if (event.target === scrim) close(scrim.id); });
  $('edit-role').onchange = updateEditorRole;
  $('edit-turbo').onchange = updateEditorRole;
  $('capture-key').onclick = () => { captureTarget = 'key'; $('edit-key').value = tr('capturePrompt'); $('capture-key').focus(); };
  $('sequence-add').onclick = () => { if (sequenceKeys.length >= 16) return; captureTarget = 'sequence'; $('sequence-add').textContent = tr('capturePrompt'); $('sequence-add').focus(); };
  $('sequence-remove').onclick = () => { sequenceKeys.pop(); renderSequence(); };
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && !$('context-menu').hidden) { hideContext(); event.preventDefault(); return; }
    if (captureTarget) {
      event.preventDefault(); event.stopPropagation();
      const key = scan(event);
      if (!key) return;
      if (captureTarget === 'key') { capturedKey = key; $('edit-key').value = displayKey(key); }
      else { sequenceKeys.push(key); renderSequence(); $('sequence-add').textContent = tr('addKey'); }
      captureTarget = '';
      return;
    }
    if (event.key === 'Escape') {
      for (const id of ['already-running-dialog','update-dialog','delete-dialog','import-dialog','close-dialog','editor','name-dialog','move-dialog','pad-dialog','stick-dialog','settings']) if (!$(id).hidden) {
        close(id); event.preventDefault(); return;
      }
    }
  }, true);
  $('name-save').onclick = () => {
    const name = $('name-input').value.trim(); if (!name) { toast(tr('invalidName')); return; }
    const folder = $('folder-selector').value;
    send(naming.mode, naming.target, name, folder); close('name-dialog');
  };
  $('name-input').onkeydown = event => { if (event.key === 'Enter') $('name-save').click(); };
  $('editor-reset').onclick = () => {
    const button = editingButton; close('editor'); requestBindingReset(button);
  };
  $('editor-save').onclick = () => {
    const role = $('edit-role').value;
    const interval = Number($('edit-interval').value);
    const delay = Number($('edit-delay').value);
    const seqInterval = Number($('sequence-interval').value);
    if (role === 'key' && (!capturedKey || capturedKey === 0xffffffff)) { toast(tr('capturePrompt')); return; }
    if (role === 'sequence' && !sequenceKeys.length) { toast(tr('addKey')); return; }
    if (interval < 20 || interval > 2000 || delay < 0 || delay > 5000 || seqInterval < 20 || seqInterval > 2000) return;
    send('saveBinding', editingButton, state.editedLayer, role,
      role === 'mouse' ? $('edit-mouse-button').value : capturedKey,
      role === 'key' && $('edit-turbo').checked ? 1 : 0, interval, delay, sequenceKeys.join(','), seqInterval,
      $('sequence-repeat').checked ? 1 : 0, $('edit-layer-target').value);
    close('editor');
  };
  if (bridge) bridge.addEventListener('message', event => {
    const message = event.data;
    if (message.type === 'state') {
      state = message; render();
      if (!startupReadySent) { startupReadySent = true; send('uiReady'); }
    }
    else if (message.type === 'closePrompt') { $('close-dialog').hidden = false; $('close-to-tray').focus(); }
    else if (message.type === 'alreadyRunning') { $('already-running-dialog').hidden = false; $('already-running-ok').focus(); }
    else if (message.type === 'updateOpenError') toast(tr('updateOpenError'));
    else if (message.type === 'updateInstallError') {
      updateFailure = ({1:'updateNetworkError',2:'updateReleaseError',3:'updateDownloadError',
        4:'updateChecksumError',5:'updateExtractError',6:'updateStartError'})[message.code] || 'updateInstallError';
      updateStatus = 'installError'; renderUpdateStatus();
    }
    else if (message.type === 'targetIcon' && state) {
      requestedIcons.delete(message.path);
      const icon = typeof message.data === 'string' &&
        message.data.startsWith('data:image/png;base64,') ? message.data : '';
      targetIcons.set(message.path, icon);
      targetIconCheckAt.set(message.path, Date.now() + (icon ? 60000 : 30000));
      if (current().target === message.path) renderTargetIcon();
    }
    else if (message.type === 'selection' && state) {
      state.selectedPreset = message.selectedPreset;
      state.editedLayer = message.editedLayer;
      state.requested = message.requested;
      updateTreeSelection(); renderDetails();
    }
    else if (message.type === 'window') { $('win-max').textContent = message.maximized ? '❐' : '□'; $('win-max').title = message.maximized ? '元のサイズに戻す' : '最大化'; }
    else if (message.type === 'status') { $('status').textContent = statusText(message.text); $('connection').textContent = compactStatus(message.text); $('connection').title = statusText(message.text); showSharedStatus(message.text); }
    else if (message.type === 'stickInput' && !$('stick-dialog').hidden) {
      const side = message.side === 0 ? 'left' : 'right';
      stickPosition[side] = {x:message.x, y:message.y}; renderStick(side);
    }
    else if (message.type === 'input') {
      if (!state || !state.preview) { clearHighlight(); return; }
      if (!$('stick-dialog').hidden) return;
      const active = new Set(message.buttons);
      const signature = message.buttons.join(',');
      if (signature !== lastInputSignature && message.buttons.length) {
        const first = message.buttons[0];
        if (category >= 0 && state.buttons[first].category !== category) {
          category = state.buttons[first].category; renderCategory(); renderRows();
        }
        if (state.editedLayer && current().layers.some(layer => layer.trigger === first)) {
          send('selectLayer', 0);
        }
        const row = document.querySelector('.input-row[data-button="' + first + '"]');
        if (row) row.scrollIntoView({block:'nearest'});
      }
      document.querySelectorAll('.input-row').forEach(row => row.classList.toggle('pressed', active.has(Number(row.dataset.button))));
      lastInputSignature = signature;
    } else if (message.type === 'error') toast(lang() === 'en' ? 'Could not save the setting. Check the value and try again.' : message.text);
  });
  send('ready');
})();
