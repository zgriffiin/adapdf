//! The `adapdf fix` command: questions, answers, exit codes and output. Test
//! plan rows C-01 to C-04 and C-12 to C-14. The questions go to stderr and the
//! answers come from stdin; the result goes to stdout. The original is never
//! changed: the fixes go into `Guide - Adapdf\Guide (accessible).docx`.

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
    fix_with(path, &[], answers)
}

/// The same, with flags (`--continue`, `--start-over`).
fn fix_with(path: &Path, flags: &[&str], answers: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(["fix", path.to_str().unwrap()])
        .args(flags)
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

/// The accessible copy of `Guide.docx` in the test folder.
fn copy_of(dir: &TempDir) -> PathBuf {
    dir.path()
        .join("Guide - Adapdf")
        .join("Guide (accessible).docx")
}

/// The fix log of a .docx.
fn log_of(path: &Path) -> Vec<adapdf::fix::FixLogEntry> {
    let package = adapdf::docx::Package::open(path).unwrap();
    adapdf::fix::fixlog::read(&package).unwrap()
}

const NOTHING_WRITTEN: &str = "No fixes were written. No file was changed.";

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
    // The original is not changed. The fixes are in the accessible copy, in the
    // folder of the guide, and nothing else is next to the original.
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide - Adapdf", "Guide.docx"]);
    let copy = copy_of(&dir);
    assert!(copy.is_file());
    let text = stdout(&out);
    assert!(
        text.contains(
            "Saved your fixes to Guide (accessible).docx. Your original Guide.docx was not changed."
        ),
        "{text}"
    );
    assert!(text.contains("Folder: "), "{text}");
    assert!(!text.to_lowercase().contains("backup"), "{text}");
    assert!(text.contains("Fixes applied (1):"), "{text}");
    assert!(
        text.contains("[DOC-002] Set the document language to en-US."),
        "{text}"
    );
    assert!(
        text.contains("No problems found."),
        "the new results: {text}"
    );
    // Re-running check on the copy gives exit 0, and the log is in the copy.
    assert_eq!(check(&copy).status.code(), Some(0));
    assert_ne!(fs::read(&copy).unwrap(), original);
    assert_eq!(log_of(&copy).len(), 1);
    assert_eq!(log_of(&copy)[0].rule, RuleId::Doc002);
}

#[test]
fn cli_fix_exit_0_when_there_is_nothing_to_fix() {
    let original = clean().build();
    let dir = TempDir::new("cli-clean");
    let path = path_of(&dir, &original);
    let out = fix(&path, "");
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("No problems found."));
    assert!(stdout(&out).contains(NOTHING_WRITTEN));
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
    assert!(!copy_of(&dir).exists());

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
    assert!(copy_of(&dir).is_file());
    let after = String::from_utf8_lossy(&check(&copy_of(&dir)).stdout).into_owned();
    assert!(!after.contains("DOC-002"), "{after}");
}

#[test]
fn cli_fix_exit_2_on_tool_failure() {
    let original = fixture(RuleId::Doc002, true);

    // A locked file: Word has the accessible copy open (its lock file is next to
    // it). Working in the copy, given as the original with --continue and given
    // as the copy itself, and starting over (which replaces the copy).
    let lock = &adapdf::fix::lock_file_names("Guide (accessible).docx")[0];
    for (given, flags) in [
        ("original", vec!["--continue"]),
        ("original", vec!["--start-over"]),
        ("copy", vec![]),
    ] {
        let dir = TempDir::new("cli-exit2-lock");
        let path = path_of(&dir, &original);
        let folder = dir.path().join("Guide - Adapdf");
        fs::create_dir_all(&folder).unwrap();
        let copy = copy_of(&dir);
        fs::write(&copy, &original).unwrap();
        fs::write(folder.join(lock), b"owner").unwrap();
        let target = if given == "copy" { &copy } else { &path };
        let out = fix_with(target, &flags, "y\n");
        assert_eq!(out.status.code(), Some(2), "{given} {flags:?}");
        assert!(stderr(&out).contains("open in Word"), "{}", stderr(&out));
        assert!(
            stderr(&out).contains("Close the document in Word first, then try again."),
            "{}",
            stderr(&out)
        );
        assert!(stderr(&out).contains("Guide (accessible).docx"));
        assert!(stdout(&out).is_empty(), "{}", stdout(&out));
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(&copy).unwrap(), original);
        assert_eq!(
            names_in(&folder),
            ["Guide (accessible).docx".to_string(), lock.clone()],
            "nothing was written"
        );
    }

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

/// A copy that cannot be written gives exit 2 with nothing on stdout, and the
/// original is as it was. Here: a file has the name of the folder of the guide,
/// so the folder cannot be made.
#[test]
fn cli_fix_exit_2_when_the_copy_cannot_be_written() {
    let original = fixture(RuleId::Doc002, true);
    let dir = TempDir::new("cli-exit2-copy");
    let path = path_of(&dir, &original);
    dir.write("Guide - Adapdf", b"a file with the name of the folder");
    let out = fix(&path, "y\n");
    assert_eq!(
        out.status.code(),
        Some(2),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("cannot be written"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).is_empty());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide - Adapdf", "Guide.docx"]);
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
    assert_eq!(fs::read(&path).unwrap(), original);
    let after = rules(&findings(&fs::read(copy_of(&dir)).unwrap()));
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
        assert!(stdout(&out).contains(NOTHING_WRITTEN), "{answers:?}");
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
fn cli_fix_shows_the_suggestions_but_never_fills_one_in() {
    let original = fixture(RuleId::Doc001, true);

    // The user types nothing: no suggestion is used.
    let dir = TempDir::new("cli-title-empty");
    let path = path_of(&dir, &original);
    let out = fix(&path, "\n");
    assert_eq!(out.status.code(), Some(1), "DOC-001 is still an error");
    // The texts come from the document: the file name and the first Heading 1.
    assert!(
        stderr(&out).contains("Suggestion (not applied), the file name: Guide"),
        "{}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("Suggestion (not applied), the first Heading 1: Test guide"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).contains(NOTHING_WRITTEN));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide.docx"]);

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
    let core = part(&fs::read(copy_of(&dir)).unwrap(), "docProps/core.xml");
    assert!(
        core.contains("<dc:title>Printer &amp; scanner guide</dc:title>"),
        "{core}"
    );
    // The text that was typed is in the fix log.
    assert_eq!(
        log_of(&copy_of(&dir))[0].text.as_deref(),
        Some("Printer & scanner guide")
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
    let xml = part(&fs::read(copy_of(&dir)).unwrap(), "word/document.xml");
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
    assert!(
        part(&fs::read(copy_of(&dir)).unwrap(), "word/document.xml").contains("adec:decorative")
    );

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
    assert!(copy_of(&dir).is_file());
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
    assert!(stdout(&out).contains(NOTHING_WRITTEN));
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
    let xml = part(&fs::read(copy_of(&dir)).unwrap(), "word/document.xml");
    let first = xml.find("<wp:docPr").unwrap();
    let second = first + 1 + xml[first + 1..].find("<wp:docPr").unwrap();
    assert!(!xml[first..second].contains("descr="), "{xml}");
    assert!(
        xml[second..].contains(r#"descr="The second picture""#),
        "{xml}"
    );
}

// ------------------------------------------------- the copy, and the choice

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A document with two fixable problems: no title (ask) and no language (auto).
fn two_problems() -> Vec<u8> {
    clean().no_language().title("").build()
}

// C-12
#[test]
fn cli_fix_refuses_an_existing_copy_unless_told_what_to_do() {
    let original = two_problems();
    let dir = TempDir::new("cli-existing");
    let path = path_of(&dir, &original);
    // The first run makes the copy (the title only).
    let out = fix(&path, "My title\nn\n");
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let copy = copy_of(&dir);
    let first = fs::read(&copy).unwrap();

    // The second run, with no choice: exit 2, nothing on stdout, nothing changed,
    // and the message says how to choose.
    let out = fix(&path, "y\n");
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
    let message = stderr(&out);
    for part in [
        "Guide (accessible).docx already exists",
        "--continue",
        "--start-over",
        "also in Word",
    ] {
        assert!(message.contains(part), "{part}: {message}");
    }
    assert_eq!(fs::read(&copy).unwrap(), first);
    assert_eq!(fs::read(&path).unwrap(), original);
}

// C-13
#[test]
fn cli_fix_continue_works_in_the_copy_and_keeps_what_is_in_it() {
    let original = two_problems();
    let dir = TempDir::new("cli-continue");
    let path = path_of(&dir, &original);
    fix(&path, "My title\nn\n");
    let copy = copy_of(&dir);
    // The person edits the copy in Word.
    let edited = replace_part(&fs::read(&copy).unwrap(), "word/document.xml", |t| {
        t.replacen(
            "<w:body>",
            "<w:body><w:p><w:r><w:t>Added in Word</w:t></w:r></w:p>",
            1,
        )
    });
    fs::write(&copy, &edited).unwrap();

    let out = fix_with(&path, &["--continue"], "y\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    // The questions are about the copy: only the language is left to fix.
    assert!(
        stderr(&out).contains("1 problem(s) the tool can fix"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).contains("Saved your fixes to Guide (accessible).docx."));
    let new = fs::read(&copy).unwrap();
    assert!(part(&new, "word/document.xml").contains("Added in Word"));
    assert!(part(&new, "docProps/core.xml").contains("My title"));
    let rules_in_log: Vec<RuleId> = log_of(&copy).iter().map(|e| e.rule).collect();
    assert_eq!(rules_in_log, [RuleId::Doc001, RuleId::Doc002]);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        names_in(&dir.path().join("Guide - Adapdf")),
        ["Guide (accessible).docx"]
    );

    // `--continue` with no copy is a mistake that the message names.
    let dir = TempDir::new("cli-continue-none");
    let path = path_of(&dir, &original);
    let out = fix_with(&path, &["--continue"], "y\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("there is no accessible copy"),
        "{}",
        stderr(&out)
    );
    assert!(stdout(&out).is_empty());
    assert_eq!(dir.names(), ["Guide.docx"]);
}

// C-14
#[test]
fn cli_fix_start_over_replaces_the_copy() {
    let original = two_problems();
    let dir = TempDir::new("cli-start-over");
    let path = path_of(&dir, &original);
    fix(&path, "My title\nn\n");
    let copy = copy_of(&dir);
    fs::write(&copy, b"edited in Word, and changed beyond use").unwrap();

    let out = fix_with(&path, &["--start-over"], "Another title\ny\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    // The questions are about the original: both problems.
    assert!(
        stderr(&out).contains("2 problem(s) the tool can fix"),
        "{}",
        stderr(&out)
    );
    let new = fs::read(&copy).unwrap();
    assert!(part(&new, "docProps/core.xml").contains("Another title"));
    assert!(!part(&new, "docProps/core.xml").contains("My title"));
    let rules_in_log: Vec<RuleId> = log_of(&copy).iter().map(|e| e.rule).collect();
    assert_eq!(rules_in_log, [RuleId::Doc001, RuleId::Doc002], "a new log");
    assert_eq!(fs::read(&path).unwrap(), original);

    // `--start-over` with no copy is the first run.
    let dir = TempDir::new("cli-start-over-none");
    let path = path_of(&dir, &original);
    let out = fix_with(&path, &["--start-over"], "T\ny\n");
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(copy_of(&dir).is_file());

    // The two flags together are a mistake.
    let out = fix_with(&path, &["--continue", "--start-over"], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(stdout(&out).is_empty());
}

// C-14
#[test]
fn cli_fix_works_on_the_copy_when_the_copy_is_given() {
    let original = two_problems();
    let dir = TempDir::new("cli-given-copy");
    let path = path_of(&dir, &original);
    fix(&path, "My title\nn\n");
    let copy = copy_of(&dir);
    let out = fix(&copy, "y\n");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(log_of(&copy).len(), 2);
    assert_eq!(fs::read(&path).unwrap(), original);
    // A copy cannot be started over: the original is.
    let out = fix_with(&copy, &["--start-over"], "y\n");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("is an accessible copy"),
        "{}",
        stderr(&out)
    );

    // A copy that is somewhere else is fixed in place, and no folder is made.
    let dir = TempDir::new("cli-given-copy-elsewhere");
    let path = dir.write("Guide (accessible).docx", &original);
    let out = fix(&path, "y\n");
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert_eq!(dir.names(), ["Guide (accessible).docx"]);
    assert_eq!(log_of(&path).len(), 1);
}
