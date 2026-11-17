//! N-05: the web UI loads nothing from the network (spec 01, constraints), and
//! the window is configured to refuse it. Also the parts of `tauri.conf.json` and
//! the capability file that the test plan fixes (rows I-02 and I-08, and the
//! minimal permissions of the app).
//!
//! These tests read the built page in `app/dist`. Run `pnpm build` in `app/`
//! first (the test says so when `dist` is missing). They need no Node
//! themselves.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn app_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn dist() -> PathBuf {
    let dist = app_dir().join("dist");
    assert!(
        dist.join("index.html").is_file(),
        "{} has no index.html. Run `pnpm build` in the app folder first.",
        dist.display()
    );
    dist
}

fn files_in(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_in(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// Addresses that are in the built files and are never fetched. Each entry is a
/// prefix, and the reason it is allowed.
const ALLOWED: &[(&str, &str)] = &[
    (
        "http://www.w3.org/",
        "XML namespace names (SVG, XLink, XML, MathML). They name a namespace; no program fetches them.",
    ),
    (
        "https://react.dev/errors/",
        "React's text for a minified error message. It is shown in an error message, never loaded.",
    ),
    (
        "http://fb.me/use-check-prop-types",
        "A warning text of the `prop-types` package (a Carbon dependency). It is text, never loaded.",
    ),
    (
        "http://ipc.localhost",
        "Tauri's IPC origin: a local address inside WebView2.",
    ),
];

/// The addresses (`http://` or `https://`) in a text that are not allowed, and
/// protocol-relative addresses in `url()`, `src` and `href`.
fn remote_urls(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for scheme in ["http://", "https://"] {
        let mut rest = text;
        while let Some(start) = rest.find(scheme) {
            let tail = &rest[start..];
            let end = tail
                .find(|c: char| {
                    c.is_whitespace()
                        || matches!(c, '"' | '\'' | '`' | ')' | '>' | '<' | '\\' | ',')
                })
                .unwrap_or(tail.len());
            let url = &tail[..end];
            if !ALLOWED.iter().any(|(prefix, _)| url.starts_with(prefix)) {
                found.push(url.to_string());
            }
            rest = &tail[scheme.len()..];
        }
    }
    for opener in [
        "url(//",
        "url('//",
        "url(\"//",
        "src=\"//",
        "href=\"//",
        "@import",
    ] {
        if text.contains(opener) {
            found.push(format!("{opener}..."));
        }
    }
    found
}

#[test]
fn frontend_bundle_has_no_remote_urls() {
    let dist = dist();
    let mut files = Vec::new();
    files_in(&dist, &mut files);
    let mut text_files = 0;
    let mut problems = Vec::new();
    for file in &files {
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "html" | "js" | "css" | "json" | "svg" | "map" | "txt") {
            continue;
        }
        text_files += 1;
        let text = fs::read_to_string(file).unwrap();
        for url in remote_urls(&text) {
            problems.push(format!(
                "{}: {url}",
                file.strip_prefix(&dist).unwrap().display()
            ));
        }
    }
    assert!(
        text_files >= 3,
        "the built page was read ({text_files} text files)"
    );
    assert!(
        problems.is_empty(),
        "the built page names remote addresses: {problems:#?}"
    );

    // The one page loads one script and one style sheet, both from its own folder.
    let index = fs::read_to_string(dist.join("index.html")).unwrap();
    for attribute in ["src=\"", "href=\""] {
        for (n, part) in index.split(attribute).enumerate().skip(1) {
            let value = part.split('"').next().unwrap();
            assert!(
                value.starts_with("/assets/"),
                "index.html: {attribute}{value} (item {n}) is not in the app's own assets folder"
            );
        }
    }
    assert!(!index.contains("<link rel=\"preconnect\"") && !index.contains("dns-prefetch"));
}

/// The scanner itself: it finds an address that is not allowed, and it lets the
/// allowed ones through.
#[test]
fn the_remote_url_scan_finds_an_address() {
    assert_eq!(
        remote_urls(
            "src:url(https://1.www.s81c.com/common/carbon/plex/fonts/IBMPlexSans-Regular.woff2)"
        ),
        ["https://1.www.s81c.com/common/carbon/plex/fonts/IBMPlexSans-Regular.woff2"]
    );
    assert_eq!(remote_urls("@import url(\"a.css\")"), ["@import..."]);
    assert_eq!(
        remote_urls("src:url(//cdn.example.com/a.woff2)"),
        ["url(//..."]
    );
    assert_eq!(
        remote_urls("fetch(\"http://example.com\")"),
        ["http://example.com"]
    );
    assert!(remote_urls("createElementNS(`http://www.w3.org/2000/svg`,`svg`)").is_empty());
    assert!(remote_urls("`https://react.dev/errors/`+e").is_empty());
    assert!(remote_urls("connect-src http://ipc.localhost").is_empty());
}

/// The fonts are files of the app: the style sheet names IBM Plex Sans, every font
/// file it names is in `dist/assets`, and none is a remote address (the scan above
/// finds those too, but this says what the page uses).
#[test]
fn the_fonts_are_bundled() {
    let dist = dist();
    let mut files = Vec::new();
    files_in(&dist.join("assets"), &mut files);
    let css_files: Vec<&PathBuf> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "css"))
        .collect();
    assert_eq!(css_files.len(), 1, "one style sheet");
    let css = fs::read_to_string(css_files[0]).unwrap();
    assert!(css.contains("IBM Plex Sans"));
    let fonts: Vec<String> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "woff2"))
        .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(fonts.len() >= 3, "the Plex files are in assets: {fonts:?}");
    for font in &fonts {
        assert!(font.starts_with("IBMPlexSans-"), "{font}");
        assert!(
            css.contains(&format!("url(/assets/{font})")),
            "{font} is named in the style sheet"
        );
    }
    // Every `src:url(...)` of a font face is one of those files.
    for part in css.split("@font-face{").skip(1) {
        let url = part
            .split("url(")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        assert!(
            url.starts_with("/assets/IBMPlexSans-") && url.ends_with(".woff2"),
            "{url}"
        );
    }
}

fn config() -> Value {
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The directives of the content security policy of the window.
fn csp() -> BTreeMap<String, Vec<String>> {
    let config = config();
    let policy = config["app"]["security"]["csp"]
        .as_str()
        .expect("the window has a content security policy");
    policy
        .split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|directive| {
            let mut words = directive.split_whitespace();
            let name = words.next().unwrap().to_string();
            (name, words.map(str::to_string).collect())
        })
        .collect()
}

#[test]
fn the_csp_allows_only_the_app_and_its_ipc() {
    let csp = csp();
    // Only these sources, in the directives that load things. Tauri's IPC is `ipc:`
    // and `http://ipc.localhost` (a local address in WebView2).
    let allowed = ["'self'", "ipc:", "http://ipc.localhost"];
    for directive in [
        "default-src",
        "connect-src",
        "font-src",
        "img-src",
        "script-src",
        "style-src",
    ] {
        let sources = csp
            .get(directive)
            .unwrap_or_else(|| panic!("the policy has no {directive}"));
        assert!(!sources.is_empty(), "{directive}");
        for source in sources {
            assert!(
                allowed.contains(&source.as_str()),
                "{directive} allows {source}"
            );
        }
        assert!(
            sources.iter().any(|s| s == "'self'") || directive == "default-src",
            "{directive} allows the app itself"
        );
    }
    // Only the connect directive may name the IPC, and the others only the app.
    for directive in [
        "default-src",
        "font-src",
        "img-src",
        "script-src",
        "style-src",
    ] {
        assert_eq!(csp[directive], ["'self'"], "{directive}");
    }
    assert_eq!(
        csp["connect-src"],
        ["'self'", "ipc:", "http://ipc.localhost"]
    );
    // No inline script or style, no `eval`, and nothing that embeds or redirects.
    let policy = config()["app"]["security"]["csp"]
        .as_str()
        .unwrap()
        .to_string();
    for word in [
        "unsafe-inline",
        "unsafe-eval",
        "data:",
        "blob:",
        "https:",
        "http:*",
        " *",
    ] {
        assert!(!policy.contains(word), "the policy has {word}");
    }
    assert_eq!(csp["object-src"], ["'none'"]);
    assert_eq!(csp["base-uri"], ["'self'"]);
}

/// The page may ask for a file to be chosen and hear about a dropped file, and
/// nothing else of Tauri: no shell, no HTTP, no file system plugin, and no
/// opener (the report is opened by a command of the app that checks the path).
#[test]
fn the_capabilities_are_minimal() {
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json"))
            .unwrap();
    let capability: Value = serde_json::from_str(&text).unwrap();
    let permissions: Vec<&str> = capability["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().expect("a permission is a name"))
        .collect();
    assert_eq!(
        permissions,
        [
            "core:event:allow-listen",
            "core:event:allow-unlisten",
            "dialog:allow-open"
        ]
    );
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    assert!(
        capability.get("remote").is_none(),
        "no remote page gets these"
    );
    // The plugins that are not wanted are not in the Rust code or the front end.
    let manifest =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    for plugin in [
        "tauri-plugin-shell",
        "tauri-plugin-http",
        "tauri-plugin-fs",
        "tauri-plugin-process",
    ] {
        assert!(!manifest.contains(plugin), "{plugin}");
    }
    let package = fs::read_to_string(app_dir().join("package.json")).unwrap();
    for plugin in ["plugin-shell", "plugin-http", "plugin-fs", "plugin-opener"] {
        assert!(!package.contains(plugin), "{plugin}");
    }
}

/// Rows I-02 and I-08 (the settings only; M5b builds the installer): a per-user
/// install, and no download of WebView2.
#[test]
fn the_installer_settings_are_per_user_and_download_nothing() {
    let config = config();
    assert_eq!(
        config["bundle"]["windows"]["webviewInstallMode"],
        serde_json::json!({ "type": "skip" })
    );
    assert_eq!(
        config["bundle"]["windows"]["nsis"]["installMode"],
        "currentUser"
    );
    assert_eq!(config["bundle"]["targets"], serde_json::json!(["nsis"]));
    // The window accepts a dropped file (a Tauri drag and drop event).
    assert_eq!(config["app"]["windows"][0]["dragDropEnabled"], true);
}
