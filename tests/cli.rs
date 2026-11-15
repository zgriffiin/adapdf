//! The `adapdf` command line: exit codes and output.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use adapdf::findings::RuleId;
use adapdf::fixtures::{DocxBuilder, catalogue, clean, zip_stored};
use serde_json::Value;

/// Writes a file in the test temp folder and returns its path.
fn write(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cli");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

fn fixture(rule: RuleId, fails: bool) -> PathBuf {
    let f = catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .unwrap();
    write(&f.file_name(), &f.bytes)
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(args)
        .output()
        .expect("the adapdf binary runs")
}

fn check(path: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["check", path.to_str().unwrap()];
    args.extend(extra);
    run(&args)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn exit_code_0_when_there_are_no_findings() {
    let path = write("clean.docx", &clean().build());
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("No problems found."),
        "{}",
        stdout(&out)
    );
}

#[test]
fn exit_code_0_when_there_are_only_warnings() {
    let path = fixture(RuleId::Hdg006, true);
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("WARNINGS (1"), "{text}");
    assert!(!text.contains("ERRORS"), "{text}");
    assert!(text.contains("[HDG-006]"), "{text}");
}

#[test]
fn exit_code_1_when_there_are_errors() {
    let path = fixture(RuleId::Hdg002, true);
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("ERRORS (1"), "{text}");
    assert!(text.contains("[HDG-002]"), "{text}");
    assert!(text.contains("How to fix:"), "{text}");
    assert!(text.contains("Where: paragraph 2 \"Three\""), "{text}");
}

#[test]
fn errors_are_listed_before_warnings() {
    let builder = DocxBuilder::new().heading(2, "Starts at two"); // HDG-002 error, DOC warnings
    let path = write("mixed.docx", &builder.build());
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(
        text.find("ERRORS").unwrap() < text.find("WARNINGS").unwrap(),
        "{text}"
    );
}

#[test]
fn exit_code_2_for_a_file_that_is_not_a_docx() {
    let path = write("not-a-docx.docx", b"This is plain text, not a zip file.");
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("not a .docx"), "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "no report on a tool failure");
}

#[test]
fn exit_code_2_for_a_zip_without_a_word_document() {
    let path = write(
        "no-document.docx",
        &zip_stored(&[("hello.txt", b"hi".as_slice())]),
    );
    let out = check(&path, &["--json"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("word/document.xml"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).is_empty());
}

#[test]
fn exit_code_2_for_damaged_xml() {
    let path = write(
        "damaged.docx",
        &zip_stored(&[("word/document.xml", b"<w:document>".as_slice())]),
    );
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("word/document.xml"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn exit_code_2_for_a_missing_file() {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("cli")
        .join("does-not-exist.docx");
    let out = check(&path, &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("cannot be read"), "{}", stderr(&out));
}

#[test]
fn exit_code_2_for_wrong_usage() {
    assert_eq!(run(&[]).status.code(), Some(2));
    assert_eq!(run(&["check"]).status.code(), Some(2));
    assert_eq!(
        run(&["export", "a.docx"]).status.code(),
        Some(2),
        "export is not in this version"
    );
}

#[test]
fn json_for_a_clean_document() {
    let path = write("clean-json.docx", &clean().build());
    let out = check(&path, &["--json"]);
    assert_eq!(out.status.code(), Some(0));
    let v = json(&out);
    assert_eq!(v["errors"], 0);
    assert_eq!(v["warnings"], 0);
    assert_eq!(v["findings"], Value::Array(vec![]));
    assert!(v["file"].as_str().unwrap().ends_with("clean-json.docx"));
}

#[test]
fn json_shape_of_a_finding() {
    let path = fixture(RuleId::Hdg002, true);
    let out = check(&path, &["--json"]);
    assert_eq!(out.status.code(), Some(1));
    let v = json(&out);
    assert_eq!(v["errors"], 1);
    assert_eq!(v["warnings"], 0);
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f["rule_id"], "HDG-002");
    assert_eq!(f["severity"], "error");
    assert!(f["message"].as_str().unwrap().contains("Heading 3"));
    assert!(!f["how_to_fix"].as_str().unwrap().is_empty());
    assert_eq!(f["fix"], serde_json::json!({"kind": "manual"}));
    assert_eq!(f["suggestion"], Value::Null);
    assert_eq!(
        f["location"],
        serde_json::json!({
            "heading": "One",
            "snippet": "Three",
            "paragraph_index": 1,
            "part": "word/document.xml"
        })
    );
}

#[test]
fn json_carries_the_suggested_title_and_the_fix_kind() {
    let path = fixture(RuleId::Doc001, true);
    let out = check(&path, &["--json"]);
    let v = json(&out);
    let f = &v["findings"][0];
    assert_eq!(f["rule_id"], "DOC-001");
    assert_eq!(
        f["fix"],
        serde_json::json!({"kind": "ask", "id": "set-title"})
    );
    assert_eq!(f["suggestion"], "Test guide");
    assert_eq!(f["location"]["paragraph_index"], Value::Null);
    assert_eq!(f["location"]["part"], "docProps/core.xml");
}

#[test]
fn json_counts_errors_and_warnings_apart() {
    let path = fixture(RuleId::Lst002, true); // one LST-001 error per typed item, one LST-002 warning
    let v = json(&check(&path, &["--json"]));
    assert_eq!(v["errors"], 1);
    assert_eq!(v["warnings"], 1);
    let rules: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap())
        .collect();
    assert_eq!(rules, ["LST-001", "LST-002"]);
}
