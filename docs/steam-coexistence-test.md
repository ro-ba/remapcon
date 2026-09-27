# Test using Remapcon while Steam is running

This feature needs hardware testing. When Steam and Remapcon share the physical controller, Steam can send duplicate inputs. Remapcon does not change Steam's settings.

## Preparation

Download the trial ZIP from **Remapcon-Windows-x64** on the GitHub Actions run page and extract it to a separate folder so the released build remains available. Settings are shared with the existing Remapcon installation on the same PC, so export them first if you want an easy backup.

1. In Steam, open **Settings → Controller → Desktop Layout** and select an empty layout. If **Disable Steam Input** is offered for Desktop Layout, that is another option. Note the previous layout so you can restore it. Desktop Layout is separate from per-game layouts.
2. If a Steam button chord opens the on-screen keyboard, review Steam's Guide Button Chord settings too. An empty Desktop Layout may not disable those chords.
3. Leave Steam running and connect the controller. On the desktop, check that the buttons and pads no longer send unintended keys or mouse input.
4. Set your target app in Remapcon and enable **Auto enable in target app**. In Remapcon **Settings**, enable **Use alongside Steam (experimental)**. Manual activation does not use shared access.

Steam may apply a game-specific layout if the target app is launched from its library. First test with a target app launched outside Steam.

## Expected results

1. Bring the target app to the front while Steam is running. Remapcon should show **Auto active** or **Shared access**, and each mapped key or mouse action should occur once. **Shared access** means an exclusive HID claim was unavailable. **Unavailable** means even shared access could not read the controller.
2. Check that Steam's desktop keys, mouse actions, and on-screen keyboard do not also activate in the target app. Test the D-pad, left stick, A button, and Steam button chords. If inputs overlap, the test has failed; turn off the experimental setting.
3. Hold a mapped button and switch away from the target app. The key should be released, and Remapcon should return to **Auto standby**.
4. Launch a Steam game and confirm that its usual Steam Input layout works with the physical controller. Switch back to the target app and repeat steps 1–2.
5. Exit the Steam game, return to the target app, and test again. If you use both a wireless Puck and USB, repeat for each connection.

Shared access depends on Windows HID handle sharing and how the other process opened the device. It is not guaranteed even when Steam is running. Remapcon cannot inspect Steam's Desktop Layout or Guide Button Chord settings.
