//! `adapdf export`, the real program, in the cases that need neither Word nor
//! veraPDF: rows C-06 (stage 1 errors), C-07 (tool failures that come before
//! Word starts), S6-09 and S3-12 and S3-13 at the command. The cases that need
//! Word and veraPDF are in `tests/export_verapdf.rs`.
#![cfg(windows)]

mod export_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use adapdf::findings::RuleId;
use adapdf::fixtures::{catalogue, clean};
use export_support::names;
use serde_json::Value;

/// A folder of its own for one test, so that tests never share a path.
fn folder(name: &str) -> PathBuf {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("cli-export")
        .join(format!("{name}-{call}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture_bytes(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .unwrap()
        .bytes
}

fn export(args: &[&Path]) -> Output {
    export_with(args, &[])
}

fn export_with(args: &[&Path], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_adapdf"));
    command.arg("export").args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the adapdf binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// C-06 and decided point 7
#[test]
fn cli_export_exit_1_on_errors() {
    let dir = folder("errors");
    let docx = dir.join("Guide.docx");
    fs::write(&docx, fixture_bytes(RuleId::Hdg002, true)).unwrap();
    let output = export(&[&docx]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stdout(&output).starts_with("Guide.docx: Fail"),
        "{}",
        stdout(&output)
    );
    assert!(stdout(&output).contains("[HDG-002]"));
    // The report is written, and no PDF.
    assert_eq!(
        names(&dir),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.docx"
        ]
    );
    let html = fs::read_to_string(dir.join("Guide.accessibility.html")).unwrap();
    assert!(html.contains("<html lang=\"en\">"));
    assert!(html.contains("Result: <strong>Fail</strong>"));
    let json: Value =
        serde_json::from_str(&fs::read_to_string(dir.join("Guide.accessibility.json")).unwrap())
            .unwrap();
    assert_eq!(json["summary"]["verdict"], "fail");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["precheck"][0]["rule_id"], "HDG-002");
}

// S3-12, S3-13 at the command: DOC-005 gives exit 1 without Word, and each
// error rule does the same.
#[test]
fn cli_export_blocks_every_error_rule_and_doc_005() {
    for fixture in catalogue()
        .into_iter()
        .filter(|f| f.fails && f.rule.severity() == adapdf::findings::Severity::Error)
    {
        let dir = folder(&fixture.rule.as_str().to_lowercase());
        let docx = dir.join("Guide.docx");
        fs::write(&docx, &fixture.bytes).unwrap();
        let output = export(&[&docx]);
        let name = fixture.file_name();
        assert_eq!(output.status.code(), Some(1), "{name}: {}", stderr(&output));
        assert!(!dir.join("Guide.pdf").exists(), "{name}");
        assert!(dir.join("Guide.accessibility.html").exists(), "{name}");
        assert!(stdout(&output).contains(fixture.rule.as_str()), "{name}");
    }
}

// C-07: stdout is empty, stderr says why, and no report is written.
#[test]
fn cli_export_exit_2_on_tool_failure() {
    let dir = folder("failures");

    // A file that is not there, and a file that is not a .docx.
    let missing = dir.join("Missing.docx");
    let output = export(&[&missing]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output).contains("Missing.docx"),
        "{}",
        stderr(&output)
    );

    let text = dir.join("Text.docx");
    fs::write(&text, "plain text").unwrap();
    let output = export(&[&text]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output).contains("not a .docx"),
        "{}",
        stderr(&output)
    );

    // Wrong usage.
    let output = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .arg("export")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());

    // S5-06: the folder of the tools does not exist, or has no Java. A document
    // that passes stage 1 needs the tools, and the check comes before Word.
    let docx = dir.join("Guide.docx");
    fs::write(&docx, clean().build()).unwrap();
    let nowhere = dir.join("no-vendor");
    let output = export_with(&[&docx], &[("ADAPDF_VENDOR", nowhere.to_str().unwrap())]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("java.exe"), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("Guide.docx"),
        "{}",
        stderr(&output)
    );

    let no_java = dir.join("vendor-without-java");
    fs::create_dir_all(no_java.join("verapdf").join("bin")).unwrap();
    let output = export_with(&[&docx], &[("ADAPDF_VENDOR", no_java.to_str().unwrap())]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("Java"), "{}", stderr(&output));

    let no_verapdf = dir.join("vendor-without-verapdf");
    fs::create_dir_all(no_verapdf.join("jre").join("bin")).unwrap();
    fs::write(no_verapdf.join("jre").join("bin").join("java.exe"), b"").unwrap();
    let output = export_with(&[&docx], &[("ADAPDF_VENDOR", no_verapdf.to_str().unwrap())]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output).contains("veraPDF is missing"),
        "{}",
        stderr(&output)
    );

    // None of these wrote a PDF or a report.
    assert!(!dir.join("Guide.pdf").exists());
    assert!(!dir.join("Guide.accessibility.html").exists());
    assert!(!dir.join("Missing.accessibility.html").exists());
}
