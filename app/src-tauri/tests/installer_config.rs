//! The installer settings (test plan rows I-02, I-08 and I-11): a per-user NSIS
//! installer that downloads nothing, and that puts `vendor/` next to the program.
//!
//! The settings are in two files. `tauri.conf.json` has what every build needs.
//! `tauri.bundle.conf.json` is added by `scripts/build-installer.ps1` only
//! (`pnpm tauri build --config ...`): its resources are in the folder
//! `bundle-staging/`, which the script makes, and every build would need it
//! otherwise (also `cargo test`). These tests need neither the built page nor
//! `vendor/`.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

fn read(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// Every string in a JSON value.
fn strings<'a>(value: &'a Value, found: &mut Vec<&'a str>) {
    match value {
        Value::String(s) => found.push(s),
        Value::Array(items) => items.iter().for_each(|v| strings(v, found)),
        Value::Object(map) => {
            for (key, v) in map {
                found.push(key);
                strings(v, found);
            }
        }
        _ => {}
    }
}

/// I-02: a per-user NSIS installer, so no admin prompt and `%LOCALAPPDATA%`.
#[test]
fn installer_config_is_per_user() {
    let config = read("tauri.conf.json");
    assert_eq!(config["bundle"]["targets"], json!(["nsis"]));
    assert_eq!(
        config["bundle"]["windows"]["nsis"]["installMode"],
        "currentUser"
    );
}

/// I-08: the installer does not download WebView2.
#[test]
fn installer_skips_webview2_download() {
    let config = read("tauri.conf.json");
    assert_eq!(
        config["bundle"]["windows"]["webviewInstallMode"],
        json!({ "type": "skip" })
    );
}

/// I-11: `vendor/` goes next to the program (Tauri puts a resource at its target
/// path under the install folder), and so do the licence texts.
#[test]
fn installer_places_vendor_next_to_the_exe() {
    let bundle = read("tauri.bundle.conf.json");
    // The build merges this file over `tauri.conf.json`, so any other key here could
    // override the per-user install mode, the targets, the WebView2 mode and so on.
    let mut top: Vec<&String> = bundle.as_object().unwrap().keys().collect();
    top.sort();
    assert_eq!(
        top,
        ["$schema", "bundle"],
        "tauri.bundle.conf.json has another top-level key"
    );
    let keys: Vec<&String> = bundle["bundle"].as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        ["resources"],
        "tauri.bundle.conf.json must only add resources"
    );
    let resources = &bundle["bundle"]["resources"];
    assert_eq!(resources["bundle-staging/vendor/"], "vendor/");
    assert_eq!(resources["../../THIRD-PARTY.md"], "THIRD-PARTY.md");
    assert_eq!(
        resources["../../THIRD-PARTY-LICENSES.txt"],
        "licenses/THIRD-PARTY-LICENSES.txt"
    );
    assert_eq!(resources["../../LICENSE"], "licenses/Adapdf-LICENSE.txt");
    assert!(
        resources
            .as_object()
            .unwrap()
            .iter()
            .any(
                |(source, target)| source.ends_with("@ibm/plex-sans/LICENSE.txt")
                    && target == "licenses/IBM-Plex-OFL-1.1.txt"
            ),
        "the font licence is not in the resources"
    );
    // Not in the file that every build reads (see the top of this file).
    assert!(read("tauri.conf.json")["bundle"]["resources"].is_null());
    // No licence page in the installer: THIRD-PARTY.md is plain notes with links,
    // and a person would have to "agree" to it. The file is in the install folder.
    assert!(read("tauri.conf.json")["bundle"]["licenseFile"].is_null());
}

/// The product name and publisher are neutral, and no file has a path of the
/// machine that built it.
#[test]
fn installer_config_has_a_neutral_name_and_no_machine_paths() {
    let config = read("tauri.conf.json");
    assert_eq!(config["productName"], "Adapdf");
    assert_eq!(config["bundle"]["publisher"], "Adapdf");
    for name in ["tauri.conf.json", "tauri.bundle.conf.json"] {
        let mut all = Vec::new();
        let value = read(name);
        strings(&value, &mut all);
        for s in all {
            let bytes = s.as_bytes();
            let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
            assert!(
                !drive && !s.starts_with('/') && !s.contains('\\') && !s.contains("Users"),
                "{name} has a machine path: {s}"
            );
        }
    }
}

/// N-03: the window turns off WebView2's background networking. Tauri's default
/// browser arguments (`--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`,
/// from wry 0.57) are dropped when this setting exists, so they are repeated here.
#[test]
fn window_turns_off_webview2_background_networking() {
    let config = read("tauri.conf.json");
    let windows = config["app"]["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 1, "the test reads the only window");
    let args = windows[0]["additionalBrowserArgs"]
        .as_str()
        .expect("the window has no additionalBrowserArgs");
    let words: Vec<&str> = args.split_whitespace().collect();
    // Chromium takes the last --disable-features, so there must be one only.
    let lists: Vec<&&str> = words
        .iter()
        .filter(|w| w.starts_with("--disable-features="))
        .collect();
    assert_eq!(
        lists.len(),
        1,
        "need exactly one --disable-features: {args}"
    );
    let mut features: Vec<&str> = lists[0]["--disable-features=".len()..].split(',').collect();
    features.sort_unstable();
    assert_eq!(
        features,
        [
            "msEdgeEnhanceSecurityMode",
            "msPdfOOUI",
            "msSmartScreenProtection",
            "msWebOOUI"
        ],
        "Tauri's defaults (msWebOOUI, msPdfOOUI, msSmartScreenProtection) and the network features"
    );
    for flag in [
        "--disable-background-networking",
        "--disable-component-update",
        "--disable-domain-reliability",
    ] {
        assert!(words.contains(&flag), "{flag} is missing from: {args}");
    }
}
