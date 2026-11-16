//! The `adapdf fix` command: questions, answers, exit codes and output. Test
//! plan rows C-01 to C-04. The questions go to stderr and the answers come
//! from stdin; the result goes to stdout.

mod fix_support;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use adapdf::findings::RuleId;
use adapdf::fixtures::{DocxBuilder, Image, Para, Table, clean};
use fix_support::*;

/// Runs `adapdf fix <file>` with these lines as the answers.
fn fix(path: &Path, answers: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(["fix", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the adapdf binary runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(answers.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn check(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(["check", path.to_str().unwrap()])
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The backup files in the folder of the document.
fn backups(dir: &TempDir) -> Vec<String> {
    dir.names()
        .into_iter()
        .filter(|n| n.starts_with("Guide.backup-") && n.ends_with(".docx"))
        .collect()
}

fn path_of(dir: &TempDir, bytes: &[u8]) -> PathBuf {
    dir.write("Guide.docx", bytes)
}

#[test]
fn cli_fix_exit_0_when_no_error_remains() {
    let original = fixture(RuleId::Doc002, true);
    let dir = TempDir::new("cli-exit0");
    let path = path_of(&dir, &original);
    let out = fix(&path, "y\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    let backups = backups(&dir);
    assert_eq!(backups.len(), 1, "{:?}", dir.names());
    assert_eq!(fs::read(dir.path().join(&backups[0])).unwrap(), original);
    let text = stdout(&out);
    assert!(text.contains(&format!("Backup: {}", backups[0])), "{text}");
    assert!(text.contains("Fixes applied (1):"), "{text}");
    assert!(
        text.contains("[DOC-002] Set the document language to en-US."),
        "{text}"
    );
    assert!(
        text.contains("No problems found."),
        "the new results: {text}"
    );
    // Re-running check gives exit 0.
    assert_eq!(check(&path).status.code(), Some(0));
    assert_ne!(fs::read(&path).unwrap(), original);
}

#[test]
fn cli_fix_exit_0_when_there_is_nothing_to_fix() {
    let original = clean().build();
    let dir = TempDir::new("cli-clean");
    let path = path_of(&dir, &original);
    let out = fix(&path, "");
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("No problems found."));
    assert!(stdout(&out).contains("No backup was made."));
    assert_eq!(dir.names(), ["Guide.docx"]);
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn cli_fix_exit_1_when_errors_remain() {
    // A manual error only: nothing to fix, exit 1, the error is listed.
    let original = fixture(RuleId::Hdg002, true);
    let dir = TempDir::new("cli-exit1-manual");
    let path = path_of(&dir, &original);
    let out = fix(&path, "");
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stdout(&out).contains("[HDG-002]"), "{}", stdout(&out));
    assert!(backups(&dir).is_empty());

    // One auto error (no language) and one manual error (a skipped heading).
    let original = clean().no_language().heading(3, "Skipped a level").build();
    let dir = TempDir::new("cli-exit1-mixed");
    let path = path_of(&dir, &original);
    let out = fix(&path, "y\n");
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("[DOC-002] Set the document language"),
        "the auto fix was applied: {text}"
    );
    assert!(
        text.contains("[HDG-002]"),
        "the manual error is listed: {text}"
    );
    assert!(!text.contains("[DOC-002] The document language"), "{text}");
    assert_eq!(backups(&dir).len(), 1);
    let after = String::from_utf8_lossy(&check(&path).stdout).into_owned();
    assert!(!after.contains("DOC-002"), "{after}");
}

#[test]
fn cli_fix_exit_2_on_tool_failure() {
    let original = fixture(RuleId::Doc002, true);

    // A locked file: Word's lock file is next to it.
    let dir = TempDir::new("cli-exit2-lock");
    let path = path_of(&dir, &original);
    dir.write("~$Guide.docx", b"owner");
    let out = fix(&path, "y\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("open in Word"), "{}", stderr(&out));
    assert!(stderr(&out).contains("Guide.docx"));
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide.docx", "~$Guide.docx"], "no backup");

    // A file that is not a .docx.
    let dir = TempDir::new("cli-exit2-not-docx");
    let path = dir.write("Guide.docx", b"This is plain text.");
    let out = fix(&path, "y\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("not a .docx"), "{}", stderr(&out));
    assert!(stdout(&out).is_empty());
    assert_eq!(fs::read(&path).unwrap(), b"This is plain text.");
    assert_eq!(dir.names(), ["Guide.docx"]);

    // A missing file.
    let dir = TempDir::new("cli-exit2-missing");
    let out = fix(&dir.path().join("Missing.docx"), "y\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(!stderr(&out).is_empty());
    assert!(stdout(&out).is_empty());

    // Wrong arguments.
    let out = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .arg("fix")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout(&out).is_empty());
}

/// A backup that cannot be written (decided by the unit test
/// `fix_stops_when_the_backup_cannot_be_written`) gives exit 2 with nothing on
/// stdout. Here: the name of the backup is a folder, for every second in a
/// short span of local time (the name of a backup uses local time), so that the
/// real clock hits one of them.
#[cfg(windows)]
#[test]
fn cli_fix_exit_2_when_the_backup_cannot_be_written() {
    let original = fixture(RuleId::Doc002, true);
    let dir = TempDir::new("cli-exit2-backup");
    let path = path_of(&dir, &original);
    let seconds = u64::try_from(wall_clock_seconds(&adapdf::fix::Timestamp::now())).unwrap();
    for offset in 0..30 {
        let t = adapdf::fix::Timestamp::from_unix_seconds(seconds + offset);
        fs::create_dir(
            dir.path()
                .join(format!("Guide.backup-{}.docx", t.backup_stamp())),
        )
        .unwrap();
    }
    let out = fix(&path, "y\n");
    assert_eq!(
        out.status.code(),
        Some(2),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(stderr(&out).contains("backup"), "{}", stderr(&out));
    assert!(stdout(&out).is_empty());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn cli_fix_confirms_each_fix() {
    // Three auto problems, in the order of the report: DOC-002, TBL-006, TXT-002.
    let original = clean()
        .no_language()
        .table(Table::new(&[&["A", "B"], &["C", "D"]]).rows_can_split())
        .paragraph(Para::new("A justified paragraph.").justify())
        .build();
    let found = findings(&original);
    let order: Vec<RuleId> = found.iter().map(|f| f.rule_id).collect();
    assert_eq!(order, [RuleId::Doc002, RuleId::Tbl006, RuleId::Txt002]);

    let dir = TempDir::new("cli-confirm");
    let path = path_of(&dir, &original);
    let out = fix(&path, "y\nn\ny\n");
    let text = stdout(&out);
    assert!(text.contains("Fixes applied (2):"), "{text}");
    assert!(
        text.contains("[DOC-002]") && text.contains("[TXT-002]"),
        "{text}"
    );
    let after = rules(&findings(&fs::read(&path).unwrap()));
    assert_eq!(
        after,
        [RuleId::Tbl006],
        "the first and the third are applied, the second is not"
    );
    assert_eq!(
        stderr(&out).matches("Apply this fix? [y/n]").count(),
        3,
        "{}",
        stderr(&out)
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn cli_fix_applies_nothing_when_every_answer_is_no_or_the_input_ends() {
    let original = fixture(RuleId::Txt002, true);
    for answers in ["n\n", "no\n", "\n", ""] {
        let dir = TempDir::new("cli-no");
        let path = path_of(&dir, &original);
        let out = fix(&path, answers);
        assert_eq!(out.status.code(), Some(0), "{answers:?}: {}", stderr(&out));
        assert!(stdout(&out).contains("No backup was made."), "{answers:?}");
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(dir.names(), ["Guide.docx"]);
    }
}

#[test]
fn cli_fix_asks_again_for_an_answer_it_does_not_understand() {
    let original = fixture(RuleId::Txt002, true);
    let dir = TempDir::new("cli-reask");
    let path = path_of(&dir, &original);
    let out = fix(&path, "maybe\nyes\n");
    assert!(
        stderr(&out).contains("Please answer y or n."),
        "{}",
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("Fixes applied (1)"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn cli_fix_shows_the_suggestion_but_never_fills_it_in() {
    let original = fixture(RuleId::Doc001, true);

    // The user types nothing: the suggestion is not used.
    let dir = TempDir::new("cli-title-empty");
    let path = path_of(&dir, &original);
    let out = fix(&path, "\n");
    assert_eq!(out.status.code(), Some(1), "DOC-001 is still an error");
    assert!(
        stderr(&out).contains("Suggestion (not applied): Test guide"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).contains("No backup was made."));
    assert_eq!(fs::read(&path).unwrap(), original);

    // The user types a title: exactly that text is written.
    let dir = TempDir::new("cli-title");
    let path = path_of(&dir, &original);
    let out = fix(&path, "  Printer & scanner guide  \n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    let core = part(&fs::read(&path).unwrap(), "docProps/core.xml");
    assert!(
        core.contains("<dc:title>Printer &amp; scanner guide</dc:title>"),
        "{core}"
    );
}

#[test]
fn cli_fix_asks_for_alt_text_or_decorative() {
    let original = fixture(RuleId::Img001, true);

    // Not decorative, then the alt text.
    let dir = TempDir::new("cli-alt");
    let path = path_of(&dir, &original);
    let out = fix(&path, "n\nThe printer front panel\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    let xml = part(&fs::read(&path).unwrap(), "word/document.xml");
    assert!(xml.contains(r#"descr="The printer front panel""#), "{xml}");
    assert!(
        stderr(&out).contains("Mark it as decorative"),
        "{}",
        stderr(&out)
    );

    // Decorative.
    let dir = TempDir::new("cli-decorative");
    let path = path_of(&dir, &original);
    let out = fix(&path, "y\n");
    assert_eq!(out.status.code(), Some(0));
    assert!(part(&fs::read(&path).unwrap(), "word/document.xml").contains("adec:decorative"));

    // Neither: nothing is written.
    let dir = TempDir::new("cli-alt-skip");
    let path = path_of(&dir, &original);
    let out = fix(&path, "n\n\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide.docx"]);
}

#[test]
fn cli_fix_asks_before_marking_the_arrows_decorative() {
    let original = fixture(RuleId::Img007, true);
    let dir = TempDir::new("cli-arrows");
    let path = path_of(&dir, &original);
    let out = fix(&path, "n\n");
    assert!(
        stderr(&out).contains("describe what the arrows and boxes show"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        original,
        "no confirmation, no write"
    );
    let out = fix(&path, "y\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("[IMG-007] Marked 3 arrow(s) or box(es)"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn cli_fix_says_what_it_cannot_do_and_what_word_does() {
    // A floating group (the tool does not make it inline) and a table of
    // contents that is out of date (Word updates it at export).
    let original = fixture(RuleId::Txt004, true);
    let dir = TempDir::new("cli-at-export");
    let path = path_of(&dir, &original);
    let out = fix(&path, "y\n");
    assert!(
        stderr(&out).contains("Word does this when the PDF is exported"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).contains("No backup was made."));
    assert_eq!(fs::read(&path).unwrap(), original);

    let original = DocxBuilder::new()
        .title("T")
        .language("en-US")
        .paragraph(Para::empty().raw_run(&Image::group().alt("A flowchart").floating().xml()))
        .build();
    let dir = TempDir::new("cli-group");
    let path = path_of(&dir, &original);
    let out = fix(&path, "y\n");
    assert!(
        stderr(&out).contains("The tool cannot fix this now"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("In Line with Text"));
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn cli_fix_writes_the_text_for_the_picture_it_was_asked_about() {
    // Two pictures with the same finding: the user skips the first and types
    // the alt text of the second.
    let original = clean()
        .paragraph(
            Para::empty()
                .raw_run(&Image::picture().xml())
                .raw_run(&Image::picture().xml()),
        )
        .build();
    let dir = TempDir::new("cli-two-pictures");
    let path = path_of(&dir, &original);
    let out = fix(&path, "n\n\nn\nThe second picture\n");
    assert_eq!(
        out.status.code(),
        Some(1),
        "the first picture still has no alt text"
    );
    let xml = part(&fs::read(&path).unwrap(), "word/document.xml");
    let first = xml.find("<wp:docPr").unwrap();
    let second = first + 1 + xml[first + 1..].find("<wp:docPr").unwrap();
    assert!(!xml[first..second].contains("descr="), "{xml}");
    assert!(
        xml[second..].contains(r#"descr="The second picture""#),
        "{xml}"
    );
}
