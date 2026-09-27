# Third-party notices

Remapcon is an independent application derived in part from
[SteamlessController](https://github.com/ddeverill/SteamlessController) by Dylan Deverill.
The reused source was based on commit `26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2`.
The reused HID and Steam Controller communication code is distributed under the
MIT license in [LICENSE](LICENSE). This repository is not a GitHub fork of that project.

The Windows build downloads Microsoft.Web.WebView2 1.0.2849.39 and statically
links its WebView2Loader library. Its distribution terms and notices are kept
in [LICENSES/WebView2-LICENSE.txt](LICENSES/WebView2-LICENSE.txt) and
[LICENSES/WebView2-NOTICE.txt](LICENSES/WebView2-NOTICE.txt). Include these files
with binary releases.

Remapcon does not include ViGEmClient or the ViGEmBus driver. The WebView2
Runtime is installed separately by the user or their system.
