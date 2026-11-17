//! N-01: the program has no HTTP client (spec 01, constraints; AGENTS.md: no
//! network calls).

use std::collections::BTreeSet;
use std::process::Command;

/// Crates that make or carry network connections for a program.
const FORBIDDEN: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "curl",
    "curl-sys",
    "attohttpc",
    "isahc",
    "surf",
    "h2",
    "native-tls",
    "rustls",
    "openssl",
    "openssl-sys",
    "tokio-native-tls",
    "tokio-rustls",
    "hyper-util",
    "http-client",
];

/// `windows` crate features that give access to the network.
const FORBIDDEN_WINDOWS_FEATURES: &[&str] = &["Win32_Networking", "Win32_NetworkManagement"];

fn manifest_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

fn forbidden_in(packages: &BTreeSet<String>) -> Vec<&'static str> {
    FORBIDDEN
        .iter()
        .copied()
        .filter(|name| packages.contains(*name))
        .collect()
}

#[test]
fn no_http_client_in_the_dependency_graph() {
    // The lock file lists every package of the graph (read as text).
    let lock = std::fs::read_to_string(manifest_path("Cargo.lock")).unwrap();
    let packages: BTreeSet<String> = lock
        .lines()
        .filter_map(|line| line.strip_prefix("name = \""))
        .filter_map(|rest| rest.strip_suffix('"'))
        .map(str::to_string)
        .collect();
    assert!(packages.contains("adapdf"), "the lock file was read");
    assert!(packages.contains("lopdf"), "the lock file is up to date");
    assert_eq!(
        forbidden_in(&packages),
        Vec::<&str>::new(),
        "an HTTP or TLS crate is in Cargo.lock"
    );

    // No `windows` feature for networking.
    let manifest = std::fs::read_to_string(manifest_path("Cargo.toml")).unwrap();
    for feature in FORBIDDEN_WINDOWS_FEATURES {
        assert!(
            !manifest.contains(feature),
            "Cargo.toml turns on the windows feature {feature}"
        );
    }
}

/// The same check on what is built for the program: `cargo tree -e normal`.
#[test]
fn no_http_client_in_the_normal_dependency_tree() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .args([
            "tree",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--locked",
            "--offline",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree can run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let packages: BTreeSet<String> = text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect();
    assert!(packages.contains("adapdf"));
    assert!(packages.contains("lopdf"));
    assert_eq!(
        forbidden_in(&packages),
        Vec::<&str>::new(),
        "an HTTP or TLS crate is in the normal dependency tree"
    );
}

#[test]
fn the_check_finds_a_client_when_there_is_one() {
    let packages: BTreeSet<String> = ["adapdf", "reqwest", "serde"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(forbidden_in(&packages), vec!["reqwest"]);
}

/// The program makes no socket itself: no `std::net` type is used anywhere in
/// `src/` (a connection made with the standard library would not be in the
/// dependency graph, so the two tests above cannot see it).
#[test]
fn the_source_makes_no_socket() {
    const NAMES: &[&str] = &[
        "std::net",
        "core::net",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "ToSocketAddrs",
    ];
    fn rust_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }
    let mut files = Vec::new();
    rust_files(&manifest_path("src"), &mut files);
    assert!(
        files.len() > 40,
        "the source tree was read: {}",
        files.len()
    );
    let mut hits = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        for (number, line) in text.lines().enumerate() {
            for name in NAMES {
                if line.contains(name) {
                    hits.push(format!("{}:{}: {name}", file.display(), number + 1));
                }
            }
        }
    }
    assert!(hits.is_empty(), "the source uses a socket type: {hits:#?}");
}
