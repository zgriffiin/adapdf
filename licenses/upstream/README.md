# Licence texts from upstream

These crates publish no licence file in their package on crates.io, so
`scripts/collect-licenses.mjs` takes the text from this folder instead.

| Crates | Licence | Source of the text |
|---|---|---|
| `alloc-stdlib` 0.2.4 and 0.3.0 | BSD-3-Clause | `LICENSE` of <https://github.com/dropbox/rust-alloc-no-stdlib> (commit 0a81fd6928ea), the crate's repository |
| `webview2-com` and `webview2-com-sys` 0.39.1, `webview2-com-macros` 0.8.1 | MIT | `LICENSE` of <https://github.com/wravery/webview2-rs> (commit edc2caf88617), the crates' repository |
| `selectors` 0.38.0 | MPL-2.0 | The standard MPL-2.0 text. The crate's repository (<https://github.com/servo/stylo>) has no licence file; the text is the one in `cssparser` 0.38.0, from the same Servo project |

`webview2-com-sys` 0.39.1 also links Microsoft's `WebView2LoaderStatic.lib` into the
program. Its licence and notice are `LICENSE-WebView2-SDK.txt` and
`NOTICE-WebView2-SDK.txt` in `webview2-com-sys-0.39.1/`, copied unchanged from the
`Microsoft.Web.WebView2` 1.0.3800.47 NuGet package (the SDK version set by
<https://github.com/wravery/webview2-rs> commit 12be1a54bf4d, 2026-03-11, which
`webview2-com-sys` 0.39 uses).
