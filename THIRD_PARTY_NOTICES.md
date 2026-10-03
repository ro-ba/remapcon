# Third-party notices

PadMux (formerly Remapcon) is an independent application derived in part from
[SteamlessController](https://github.com/ddeverill/SteamlessController) by Dylan Deverill.
The reused source was based on commit `26c5b4ab6eee8aaf57eb9c99383eed3dfe475df2`.
The reused HID and Steam Controller communication code is distributed under the
MIT license in [LICENSE](LICENSE). This repository is not a GitHub fork of that project.

The Windows build downloads Microsoft.Web.WebView2 1.0.2849.39 and statically
links its WebView2Loader library. Its distribution terms and notices are kept
in [LICENSES/WebView2-LICENSE.txt](LICENSES/WebView2-LICENSE.txt) and
[LICENSES/WebView2-NOTICE.txt](LICENSES/WebView2-NOTICE.txt). Include these files
with binary releases.

PadMux does not include ViGEmClient or the ViGEmBus driver. The WebView2
Runtime is installed separately by the user or their system.

## Rust migration diagnostic

The independent diagnostic in `rust/` translates HID access and controller
report parsing from the same SteamlessController reference commit above.
Its source retains Dylan Deverill's and ro-ba's copyright notices and MIT
attribution; migration does not remove the original notices.

The Rust workspace uses Microsoft `windows` 0.62.2, `serde` 1.0.229,
`serde_json` 1.0.151, MIT-licensed `webview2-com` 0.39.1, and the dependencies pinned
in `rust/Cargo.lock`. Dual-licensed MIT/Apache-2.0 dependencies are used under
the MIT option. `unicode-ident` additionally requires its Unicode-3.0 notice.
The dependency license texts, copyright notices, and version inventory are
preserved in [rust/THIRD_PARTY_LICENSES.txt](rust/THIRD_PARTY_LICENSES.txt).
Include that file with distributions of the Rust diagnostic. These crates
are not linked into the existing C++ executables.

The Rust WebView2 adapter uses the statically linked Microsoft WebView2Loader
1.0.3800.47 bundled by `webview2-com-sys` 0.39.1. Its Microsoft SDK license and
notices were compared with the official NuGet 1.0.3800.47 package. The terms
are the same as the existing files in `LICENSES/` (license whitespace differs).
Include both WebView2 license/notice files as well as the Rust dependency
license inventory with Rust application releases. The WebView2 Runtime
continues to be installed separately.
