import base64
import json
import struct
import sys
import zlib
from pathlib import Path

if len(sys.argv) != 3 or sys.argv[1] not in ('ja', 'en'):
    raise SystemExit('Usage: python docs/demo/generate_preview.py ja|en OUTPUT.html')
language, output_path = sys.argv[1:]

# The preview uses fictional data and never reads the user's settings.
root = Path(__file__).resolve().parents[2]
button_specs = [
    ('A', 'A', 0), ('B', 'B', 0), ('X', 'X', 0), ('Y', 'Y', 0),
    ('LB', 'LB', 0), ('RB', 'RB', 0), ('LT', 'LT', 0), ('RT', 'RT', 0), ('L3', 'L3', 0), ('R3', 'R3', 0),
    ('↑', 'DPadUp', 1), ('↓', 'DPadDown', 1), ('←', 'DPadLeft', 1), ('→', 'DPadRight', 1),
    ('L4', 'L4', 2), ('L5', 'L5', 2), ('R4', 'R4', 2), ('R5', 'R5', 2),
    ('左クリック', 'LeftPadClick', 3), ('左タップ', 'LeftPadTap', 3),
    ('右クリック', 'RightPadClick', 3), ('右タップ', 'RightPadTap', 3),
    ('メニュー', 'Menu', 4), ('ビュー', 'View', 4),
    ('右スティック ↑', 'RightStickUp', 5), ('右スティック ↓', 'RightStickDown', 5),
    ('右スティック ←', 'RightStickLeft', 5), ('右スティック →', 'RightStickRight', 5),
]
buttons = [dict(name=name, setting=setting, category=category) for name, setting, category in button_specs]

def mapping(items=None):
    assignments = [0] * len(buttons)
    for key, value in (items or {}).items():
        assignments[key] = value
    return {
        'mapping': assignments,
        'turbo': [dict(enabled=False, intervalMs=100, delayMs=0) for _ in buttons],
        'sequence': [dict(enabled=False, repeat=False, intervalMs=100, keys=[]) for _ in buttons],
    }

def preset(name, folder, target, assignments, layers=None):
    return dict(name=name, folder=folder, target=target,
                leftPadMode=2, rightPadMode=1,
                leftPadSensitivity=100, rightPadSensitivity=115,
                layers=layers or [], **mapping(assignments))

brush_layer = dict(name='ブラシ', trigger=14,
                   **mapping({0:0x02, 1:0x03, 2:0x04, 3:0x05, 4:0x1d, 5:0x2a}))
tool_layer = dict(name='ツール', trigger=16,
                  **mapping({0:0x10, 1:0x11, 2:0x12, 3:0x13}))
state = dict(type='state',
    presets=[
        preset('イラスト制作', '作業用/画像', 'DemoCanvas.exe',
               {0:0x39, 1:0x01, 2:0x12, 3:0x13, 4:0x10, 5:0x11,
                6:0x0f, 7:0x21, 8:0x1e, 9:0x30,
                10:0x10048, 11:0x10050, 12:0x1004b, 13:0x1004d},
               [brush_layer, tool_layer]),
        preset('写真整理', '作業用/画像', 'DemoPhotos.exe', {0:0x1c, 1:0x01}),
        preset('動画編集', '作業用/動画', 'DemoVideo.exe', {0:0x39, 1:0x01}),
        preset('プレゼン操作', '会議用', 'DemoSlides.exe', {0:0x1004d, 1:0x1004b}),
    ],
    folders=['作業用', '作業用/画像', '作業用/動画', '会議用'],
    selectedPreset=0, editedLayer=0, autoMode=True, requested=False,
    preview=False, elevated=True, language='ja', closeBehavior=0,
    status='自動待機中：対象アプリを前面にすると有効になります',
    buttons=buttons,
    categories=['基本ボタン', '十字キー', '背面ボタン', 'トラックパッド', 'メニュー・ビュー', 'スティック'],
)
if language == 'en':
    state['language'] = 'en'
    state['folders'] = ['Projects', 'Projects/Art', 'Projects/Video', 'Meetings']
    for item, name, folder in zip(state['presets'],
                                  ['Illustration', 'Photo library', 'Video editing', 'Presentation'],
                                  ['Projects/Art', 'Projects/Art', 'Projects/Video', 'Meetings']):
        item['name'] = name
        item['folder'] = folder
    state['presets'][0]['layers'][0]['name'] = 'Brush'
    state['presets'][0]['layers'][1]['name'] = 'Tools'

# A tiny original placeholder icon for the fictitious demo executable.
size = 32
pixels = bytearray()
for y in range(size):
    pixels.append(0)
    for x in range(size):
        r, g, b, a = (110 + x // 3, 73 + y // 4, 153 + x // 5, 255)
        if (x < 3 or y < 3 or x > 28 or y > 28) and (x < 5 and y < 5 or x > 26 and y < 5 or x < 5 and y > 26 or x > 26 and y > 26):
            a = 0
        if abs(x-y) <= 2 and 7 <= x <= 23:
            r, g, b = 240, 225, 255
        pixels.extend((r, g, b, a))
def chunk(tag, data):
    return struct.pack('!I',len(data)) + tag + data + struct.pack('!I',zlib.crc32(tag+data)&0xffffffff)
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR',struct.pack('!2I5B',size,size,8,6,0,0,0)) + chunk(b'IDAT',zlib.compress(bytes(pixels))) + chunk(b'IEND',b'')
icon_data = 'data:image/png;base64,' + base64.b64encode(png).decode()

html = (root/'src/app/ui/index.html').read_text()
html = html.replace('<!-- STYLE -->','<style>'+(root/'src/app/ui/style.css').read_text()+'</style>')
html = html.replace('<!-- BRAND_ICON -->',(root/'src/app/resources/padmux.svg').read_text())
bridge = '''<script>
window.chrome = window.chrome || {};
const demoState = DATA;
const demoIcon = ICON;
let demoListener = null;
window.chrome.webview = {
  addEventListener(_type, listener) { demoListener = listener; },
  postMessage(message) {
    const command = decodeURIComponent(message.split('\\t')[0]);
    if (command === 'ready') setTimeout(() => demoListener({data: demoState}), 30);
    if (command === 'getTargetIcon') setTimeout(() => demoListener({data: {type:'targetIcon', path:demoState.presets[0].target, data:demoIcon}}), 30);
  }
};
</script>'''.replace('DATA', json.dumps(state,ensure_ascii=False)).replace('ICON',json.dumps(icon_data))
html = html.replace('<!-- SCRIPT -->',bridge+'<script>'+(root/'src/app/ui/app.js').read_text()+'</script>')
Path(output_path).write_text(html, encoding='utf-8')
print(f'Wrote {output_path}')
