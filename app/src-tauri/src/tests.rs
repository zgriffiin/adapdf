//! Tests of the commands of the app: rows A-07 and A-08 of the test plan. They
//! need no window, no Node, and (except the last test, behind the features
//! `windows_word` and `verapdf`) no Word or Java.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use adapdf::findings::{FixId, FixKind, RuleId};
use adapdf::fix::interactive::CopyChoice;
use adapdf::fix::{Environment, FixOptions, RealFs, Timestamp};
use adapdf::fixtures::{FixtureFonts, catalogue};
use adapdf::render;
use adapdf::report::{self, Input, Report, html};
use serde_json::Value;

use crate::commands::{
    self, Availability, Choice, Input as Answer, check, fix_offers, read_report, report_paths,
};
use crate::signoff::{PacInput, Signoff, save_signoff};

// ------------------------------------------------------------ helpers

/// A folder under `target/tmp` that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp/app-tests")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    /// The names of the files, sorted.
    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .expect("the catalogue has this fixture")
        .bytes
}

fn clock() -> Timestamp {
    Timestamp {
        year: 2026,
        month: 3,
        day: 5,
        hour: 10,
        minute: 15,
        second: 30,
    }
}

fn environment() -> Environment<'static> {
    Environment {
        fs: &RealFs,
        now: clock(),
        fonts: &FixtureFonts,
    }
}

// --------------------------------------------------------------- A-07

/// The same findings and the same exit result as `adapdf check` (which calls
/// `check_file` and prints `render::to_json`), on a fixture with only a warning,
/// one with errors, a passing one, and a file that cannot be read.
#[test]
fn app_commands_match_the_cli_check() {
    let dir = TempDir::new("a07-check");
    for (rule, fails, errors, warnings, exit) in [
        (RuleId::Lst002, true, 1, 1, 1),
        (RuleId::Txb004, true, 0, 1, 0),
        (RuleId::Doc001, true, 1, 0, 1),
        (RuleId::Doc001, false, 0, 0, 0),
    ] {
        let path = dir.write("Guide.docx", &fixture(rule, fails));
        let app = check(&path, &FixtureFonts).unwrap();
        let cli = adapdf::check_file(&path, &FixtureFonts).unwrap();
        assert_eq!(app.findings, cli, "{rule} {fails}");
        // The JSON of `adapdf check --json` has the same findings, counts and file.
        let json: Value =
            serde_json::from_str(&render::to_json(&path.display().to_string(), &cli)).unwrap();
        assert_eq!(
            serde_json::to_value(&app.findings).unwrap(),
            json["findings"]
        );
        assert_eq!(app.errors, json["errors"]);
        assert_eq!(app.warnings, json["warnings"]);
        assert_eq!(app.file, json["file"]);
        assert_eq!((app.errors, app.warnings), (errors, warnings), "{rule}");
        assert_eq!(app.exit_code, exit, "{rule} {fails}");
        assert_eq!(app.exit_code == 1, render::has_errors(&cli));
    }
    // The CLI exits with 2 and prints a message. The app gives an error.
    let missing = dir.path().join("Missing.docx");
    assert!(adapdf::check_file(&missing, &FixtureFonts).is_err());
    let error = check(&missing, &FixtureFonts).unwrap_err();
    assert!(
        error
            .message
            .starts_with("Missing.docx: the file cannot be read"),
        "{error}"
    );
    let not_docx = dir.write("Text.docx", b"this is not a zip file");
    assert!(
        check(&not_docx, &FixtureFonts)
            .unwrap_err()
            .message
            .contains("not a .docx")
    );
}

/// The offers are those of `adapdf fix`, and a fix chosen in the app gives the
/// same accessible copy, the same folder, the same findings and the same exit
/// result as the answers `y` (or the typed text) given to `adapdf fix`.
#[test]
fn app_commands_match_the_cli() {
    // The fixes of fixtures, with what the person gives in the app for a fix that
    // asks (`ask`) and what is typed at the command line for the same fix.
    let text = |text: &str| Answer::Text {
        text: text.to_string(),
    };
    for (rule, ask, cli) in [
        (RuleId::Txb004, Answer::Confirm, "y"),
        (RuleId::Lst002, Answer::Confirm, "y"),
        (RuleId::Tbl001, Answer::Confirm, "y"),
        (
            RuleId::Doc001,
            text("A title typed by a person"),
            "A title typed by a person",
        ),
        (RuleId::Doc002, Answer::Confirm, "y"),
        // A picture: "Mark as decorative", and then alt text typed instead.
        (RuleId::Img001, Answer::Decorative, "y"),
        (
            RuleId::Img001,
            text("A red arrow"),
            "n
A red arrow",
        ),
        // The question about arrows and boxes: yes.
        (RuleId::Img007, Answer::Confirm, "y"),
    ] {
        let bytes = fixture(rule, true);
        let a = TempDir::new("a07-app");
        let b = TempDir::new("a07-cli");
        let app_path = a.write("Guide.docx", &bytes);
        let cli_path = b.write("Guide.docx", &bytes);
        let options = FixOptions::default();
        let env = environment();

        let offers = fix_offers(&app_path, &env, &options).unwrap();
        let library = adapdf::fix::offers(&bytes, &options, &FixtureFonts).unwrap();
        assert_eq!(offers.len(), library.len(), "{rule}");
        for (view, offer) in offers.iter().zip(&library) {
            assert_eq!(view.finding, offer.finding, "{rule}");
            assert_eq!(view.position, offer.position);
            assert_eq!(view.fix, offer.fix);
            assert_eq!(view.ask, offer.ask);
            assert_eq!(view.description, adapdf::fix::describe(offer.fix, &options));
        }

        // The app: choose every fix that is ready.
        let choices: Vec<Choice> = offers
            .iter()
            .filter(|o| o.availability == Availability::Ready)
            .map(|o| Choice {
                rule_id: o.finding.rule_id,
                position: o.position,
                input: if o.ask { ask.clone() } else { Answer::Confirm },
            })
            .collect();
        assert!(!choices.is_empty(), "{rule}");
        let result = commands::apply_fixes(&app_path, &choices, false, &env, &options).unwrap();

        // The command line: one answer for each offer, in order.
        let answers: String = offers
            .iter()
            .map(|o| match o.availability {
                Availability::Ready => format!("{cli}\n"),
                _ => String::new(),
            })
            .collect();
        let mut input = answers.as_bytes();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = adapdf::fix::interactive::run(
            &cli_path,
            CopyChoice::Ask,
            &mut input,
            &mut out,
            &mut err,
            &env,
        );

        // The original is not changed by either. The copy is the same file, in
        // the folder of the guide.
        let (app_copy, cli_copy) = (
            adapdf::fix::accessible_copy_path(&app_path),
            adapdf::fix::accessible_copy_path(&cli_path),
        );
        assert_eq!(fs::read(&app_path).unwrap(), bytes, "{rule}");
        assert_eq!(fs::read(&cli_path).unwrap(), bytes, "{rule}");
        assert_eq!(
            fs::read(&app_copy).unwrap(),
            fs::read(&cli_copy).unwrap(),
            "{rule}"
        );
        assert_eq!(a.names(), b.names(), "{rule}: the same files");
        assert_eq!(a.names(), ["Guide - Adapdf", "Guide.docx"], "{rule}");
        assert_eq!(
            result.copy.as_deref(),
            Some(app_copy.display().to_string().as_str())
        );
        assert_eq!(result.working_file, app_copy.display().to_string());
        assert_eq!(result.original, app_path.display().to_string());
        assert!(result.refused.is_empty(), "{rule}: {:?}", result.refused);
        assert_eq!(result.exit_code, code, "{rule}");
        assert_eq!(
            result.findings,
            adapdf::check_file(&cli_copy, &FixtureFonts).unwrap(),
            "{rule}"
        );
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains(&render::to_text(
                &cli_copy.display().to_string(),
                &result.findings
            )),
            "{text}"
        );
        for applied in &result.applied {
            assert!(
                text.contains(&format!("[{}] {}", applied.rule_id, applied.summary)),
                "{text}"
            );
        }
    }
}

/// A fixture with no fix: no offers, nothing written, the same result as the CLI.
#[test]
fn app_commands_match_the_cli_when_there_is_nothing_to_fix() {
    let dir = TempDir::new("a07-none");
    let path = dir.write("Guide.docx", &fixture(RuleId::Doc003, true));
    let before = fs::read(&path).unwrap();
    let options = FixOptions::default();
    let env = environment();
    assert!(fix_offers(&path, &env, &options).unwrap().is_empty());
    let result = commands::apply_fixes(&path, &[], false, &env, &options).unwrap();
    assert_eq!(result.copy, None);
    assert_eq!(result.working_file, path.display().to_string());
    assert!(result.applied.is_empty() && result.refused.is_empty());
    assert_eq!(result.exit_code, 0);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(dir.names(), ["Guide.docx"]);
}

/// The text of an ask fix comes from the person. An empty text is refused (as the
/// CLI refuses it), the finding that the page names must still be in the file,
/// and a file that Word has open is not written.
#[test]
fn apply_fixes_refuses_what_the_cli_refuses() {
    let dir = TempDir::new("a07-refuse");
    let path = dir.write("Guide.docx", &fixture(RuleId::Doc001, true));
    let before = fs::read(&path).unwrap();
    let options = FixOptions::default();
    let env = environment();
    let offers = fix_offers(&path, &env, &options).unwrap();
    assert_eq!(offers.len(), 1);
    assert!(offers[0].ask);
    assert_eq!(offers[0].fix, FixId::SetTitle);
    assert_eq!(offers[0].finding.fix, FixKind::Ask(FixId::SetTitle));

    let blank = Choice {
        rule_id: RuleId::Doc001,
        position: 0,
        input: Answer::Text {
            text: "   ".to_string(),
        },
    };
    let gone = Choice {
        rule_id: RuleId::Img001,
        position: 3,
        input: Answer::Decorative,
    };
    let result = commands::apply_fixes(&path, &[blank, gone], false, &env, &options).unwrap();
    assert!(result.applied.is_empty());
    assert_eq!(result.refused.len(), 2);
    assert!(
        result
            .refused
            .iter()
            .any(|r| r.reason.contains("No text was typed"))
    );
    assert!(
        result
            .refused
            .iter()
            .any(|r| r.reason.contains("changed since the check"))
    );
    assert_eq!(result.copy, None);
    assert_eq!(fs::read(&path).unwrap(), before, "nothing was written");
    assert_eq!(dir.names(), ["Guide.docx"], "no copy and no folder");

    // Word has the accessible copy open: its lock file is next to it. The message
    // says what to do.
    let copy = dir.write("Guide (accessible).docx", &before);
    let lock = &adapdf::fix::lock_file_names("Guide (accessible).docx")[0];
    fs::write(dir.path().join(lock), b"lock").unwrap();
    for error in [
        fix_offers(&copy, &env, &options).unwrap_err(),
        commands::apply_fixes(&copy, &[], false, &env, &options).unwrap_err(),
    ] {
        assert!(error.message.contains("open in Word"), "{error}");
        assert!(
            error
                .message
                .contains("Close the document in Word first, then try again."),
            "{error}"
        );
    }
}

/// The title has up to three one-click texts from the document, and the app never
/// fills text in: the offer has the texts, and no input of its own.
#[test]
fn the_title_offer_carries_the_texts_from_the_document_and_no_input() {
    let dir = TempDir::new("a03-offers");
    let path = dir.write("Parks_Guide.docx", &fixture(RuleId::Doc001, true));
    let offers = fix_offers(&path, &environment(), &FixOptions::default()).unwrap();
    let json = serde_json::to_value(&offers[0]).unwrap();
    assert!(json.get("input").is_none() && json.get("text").is_none());
    assert_eq!(json["ask"], true);
    assert_eq!(json["availability"]["status"], "ready");
    assert_eq!(json["finding"]["suggestion"], serde_json::Value::Null);
    // The file name, cleaned, and the first Heading 1 of the fixture.
    assert_eq!(
        json["suggestions"],
        serde_json::json!([
            { "source": "file-name", "label": "Use the file name", "text": "Parks Guide" },
            { "source": "first-heading1", "label": "Use the first Heading 1", "text": "Test guide" }
        ])
    );
    // The name of the accessible copy gives the same title (no `(accessible)`).
    let copy = dir.write(
        "Parks_Guide (accessible).docx",
        &fixture(RuleId::Doc001, true),
    );
    let offers = fix_offers(&copy, &environment(), &FixOptions::default()).unwrap();
    assert_eq!(offers[0].suggestions[0].text, "Parks Guide");
    // Other fixes have no suggestion; alt text has none (IMG-005 would report a
    // caption that repeats it).
    let path = dir.write("Pictures.docx", &fixture(RuleId::Img001, true));
    let offers = fix_offers(&path, &environment(), &FixOptions::default()).unwrap();
    assert!(offers.iter().all(|o| o.suggestions.is_empty()));
}

// The export of the app is the export of the command: stage 1 comes first, so
// a document with an error gets a Fail report and no PDF, and a missing file is
// a tool failure. Neither starts Word.
#[cfg(windows)]
mod export {
    use super::*;
    use adapdf::export::{ExportError, ExportResult};
    use adapdf::pipeline::{Exit, Stages, export_docx, output_paths};

    #[cfg(all(feature = "windows_word", feature = "verapdf"))]
    fn name_of_file(path: &Path) -> String {
        path.file_name().unwrap().to_string_lossy().into_owned()
    }
    use adapdf::verapdf::{Validation, VeraPdfError};

    /// Stages that fail the test when they are used: Word and veraPDF must not
    /// be asked for.
    pub(super) struct NoStages;

    impl Stages for NoStages {
        fn check_tools(&self) -> Result<(), VeraPdfError> {
            panic!("the tools must not be checked");
        }
        fn export(&self, _: &Path, _: &Path) -> Result<ExportResult, ExportError> {
            panic!("Word must not be started");
        }
        fn validate(&self, _: &Path) -> Result<Validation, VeraPdfError> {
            panic!("veraPDF must not be run");
        }
    }

    #[test]
    fn app_export_matches_the_cli_when_stage_1_finds_errors() {
        let bytes = fixture(RuleId::Doc001, true);
        let a = TempDir::new("a07-export-app");
        let b = TempDir::new("a07-export-cli");
        let app_path = a.write("Guide.docx", &bytes);
        let cli_path = b.write("Guide.docx", &bytes);

        let app = commands::export(&app_path, &NoStages, &FixtureFonts, clock()).unwrap();
        let cli = export_docx(&cli_path, &NoStages, &FixtureFonts, clock());

        assert_eq!(cli.exit, Exit::ErrorsFound);
        assert_eq!(app.exit_code, cli.exit.code());
        assert_eq!(app.view.report, cli.report.clone().unwrap());
        let in_dir =
            |text: &str, dir: &TempDir| text.replace(&dir.path().display().to_string(), "<dir>");
        assert_eq!(in_dir(&app.console, &a), in_dir(&cli.stdout, &b));
        // The same files, written in the folder of the guide: nothing else is next
        // to the original.
        assert_eq!(a.names(), b.names());
        assert_eq!(a.names(), ["Guide - Adapdf", "Guide.docx"]);
        let folder = |dir: &TempDir| {
            let mut names: Vec<String> = fs::read_dir(dir.path().join("Guide - Adapdf"))
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        assert_eq!(
            folder(&a),
            ["Guide.accessibility.html", "Guide.accessibility.json"]
        );
        assert_eq!(folder(&a), folder(&b));
        let (_, html_path, json_path) = output_paths(&app_path);
        assert_eq!(app.view.report_html, html_path.display().to_string());
        assert_eq!(app.view.report_json, json_path.display().to_string());
        assert_eq!(app.view.pdf, None, "no PDF for a document with errors");
        assert_eq!(app.files.len(), 2);
        assert_eq!(app.view.report.summary.verdict, report::Verdict::Fail);
        // What the app reads back is what was written.
        assert_eq!(read_report(&json_path).unwrap(), app.view);
    }

    #[test]
    fn app_export_gives_an_error_for_what_the_cli_exits_2() {
        let dir = TempDir::new("a07-export-fail");
        let missing = dir.path().join("Guide.docx");
        let cli = export_docx(&missing, &NoStages, &FixtureFonts, clock());
        assert_eq!(cli.exit, Exit::ToolFailure);
        let error = commands::export(&missing, &NoStages, &FixtureFonts, clock()).unwrap_err();
        assert!(!error.message.starts_with("adapdf:"), "{error}");
        assert!(
            cli.stderr.contains(&error.message),
            "{error}: {}",
            cli.stderr
        );
        assert!(error.message.contains("cannot be read"), "{error}");
        assert!(dir.names().is_empty(), "no report for a tool failure");
    }

    /// The real Word and the real veraPDF. The app and `adapdf export` (its code,
    /// `pipeline::export_docx`, which `main.rs` calls with the same stages) run the
    /// same pass fixture, each in its own folder: the same exit code, the same
    /// files, the same text and the same report, except the values that depend on
    /// the clock (the time of the run and the seconds Word took). Needs desktop
    /// Word and the bundled veraPDF (set `ADAPDF_VENDOR` when `vendor/` is not in
    /// the source tree).
    #[cfg(all(feature = "windows_word", feature = "verapdf"))]
    #[test]
    fn app_export_with_word_matches_the_cli_export() {
        use adapdf::fonts::SystemFonts;
        use adapdf::pipeline::RealStages;

        let a = TempDir::new("a07-word-app");
        let b = TempDir::new("a07-word-cli");
        let bytes = fixture(RuleId::Doc001, false);
        let app_docx = a.write("Guide.docx", &bytes);
        let cli_docx = b.write("Guide.docx", &bytes);

        let app = commands::export_with_word(&app_docx, &SystemFonts::from_environment()).unwrap();
        let fonts = SystemFonts::from_environment();
        let stages = RealStages {
            fonts: &fonts,
            tool: adapdf::verapdf::Tool::locate(),
            options: adapdf::export::ExportOptions::default(),
        };
        let cli = export_docx(&cli_docx, &stages, &fonts, Timestamp::now());

        // The exit code, the folder and the files that the run wrote.
        assert_eq!(cli.exit, Exit::Pass, "{}{}", cli.stdout, cli.stderr);
        assert_eq!(app.exit_code, cli.exit.code(), "{}", app.console);
        let expected = [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.pdf",
        ];
        let adapdf_folder = |dir: &TempDir| {
            let mut names: Vec<String> = fs::read_dir(dir.path().join("Guide - Adapdf"))
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        assert_eq!(a.names(), ["Guide - Adapdf", "Guide.docx"]);
        assert_eq!(b.names(), ["Guide - Adapdf", "Guide.docx"]);
        assert_eq!(adapdf_folder(&a), expected);
        assert_eq!(adapdf_folder(&b), expected);
        let names = |files: &[String]| -> Vec<String> {
            files
                .iter()
                .map(|f| name_of_file(Path::new(f)))
                .collect::<Vec<_>>()
        };
        let cli_files: Vec<String> = cli.files.iter().map(|f| f.display().to_string()).collect();
        assert_eq!(names(&app.files), names(&cli_files));
        assert_eq!(app.files.len(), 3);

        // The report, without what depends on the clock.
        let clockless = |report: &Report| -> Report {
            let mut report = report.clone();
            report.summary.generated_local = String::new();
            if let Some(export) = report.export.as_mut() {
                export.seconds = 0.0;
            }
            report
        };
        let cli_report = cli.report.clone().unwrap();
        assert_eq!(clockless(&app.view.report), clockless(&cli_report));
        assert_eq!(app.view.report.summary.verdict, report::Verdict::Pass);
        assert_eq!(app.view.report.summary.pdf.as_deref(), Some("Guide.pdf"));
        assert_eq!(
            app.view.pdf,
            Some(
                a.path()
                    .join("Guide - Adapdf")
                    .join("Guide.pdf")
                    .display()
                    .to_string()
            )
        );

        // The text that the command prints (with the folder and the time left out).
        let plain = |text: &str, dir: &TempDir, report: &Report| {
            text.replace(&dir.path().display().to_string(), "<dir>")
                .replace(&report.summary.generated_local, "<time>")
        };
        assert_eq!(
            plain(&app.console, &a, &app.view.report),
            plain(&cli.stdout, &b, &cli_report)
        );
        assert!(
            app.console
                .starts_with("Guide.docx: Pass (0 errors, 0 warnings).")
        );

        // The report that was written is the report that came back.
        let json = a
            .path()
            .join("Guide - Adapdf")
            .join("Guide.accessibility.json");
        assert_eq!(read_report(&json).unwrap(), app.view);
    }
}

// --------------------------------------------------------- the report

#[test]
fn only_the_html_of_a_report_can_be_opened() {
    let dir = TempDir::new("a06-open");
    let html = dir.write("Guide.accessibility.html", b"<html></html>");
    assert_eq!(commands::check_report_html(&html).unwrap(), html);
    for bad in [
        "Guide.pdf",
        "Guide.docx",
        "Guide.html",
        ".accessibility.html",
        "Guide.accessibility.json",
    ] {
        let path = dir.write(bad, b"x");
        assert!(commands::check_report_html(&path).is_err(), "{bad}");
    }
    // Not there, not absolute, a web address, and a network share.
    assert!(commands::check_report_html(&dir.path().join("Other.accessibility.html")).is_err());
    assert!(commands::check_report_html(Path::new("Guide.accessibility.html")).is_err());
    for address in [
        "https://example.com/Guide.accessibility.html",
        r"\\server\share\Guide.accessibility.html",
    ] {
        assert!(
            commands::check_report_html(Path::new(address)).is_err(),
            "{address}"
        );
    }
}

#[test]
fn report_paths_need_the_name_of_a_report() {
    let (json, html) = report_paths(Path::new("C:/g/Guide.accessibility.json")).unwrap();
    assert_eq!(json, Path::new("C:/g/Guide.accessibility.json"));
    assert_eq!(html, Path::new("C:/g/Guide.accessibility.html"));
    for bad in [
        "C:/g/Guide.json",
        "C:/g/.accessibility.json",
        "C:/g/Guide.accessibility.html",
    ] {
        assert!(report_paths(Path::new(bad)).is_err(), "{bad}");
    }
}

// --------------------------------------------------------------- A-08

/// An unsigned report from `LST-002-fail.docx`, as `adapdf export` writes it, with
/// a PDF next to it.
struct Saved {
    dir: TempDir,
    json: PathBuf,
    html: PathBuf,
    pdf: PathBuf,
    report: Report,
}

fn saved_report() -> Saved {
    let dir = TempDir::new("a08");
    let findings = adapdf::check_bytes(&fixture(RuleId::Lst002, true), &FixtureFonts).unwrap();
    let report = report::build(Input {
        document: "Guide.docx".to_string(),
        generated_local: "2026-03-05 10:15:30".to_string(),
        findings,
        applied_fixes: Vec::new(),
        original: None,
        pdf: Some("Guide.pdf".to_string()),
        export: None,
        metadata: None,
        verapdf: None,
    });
    let json = dir.write("Guide.accessibility.json", report.to_json().as_bytes());
    let html = dir.write("Guide.accessibility.html", html::render(&report).as_bytes());
    let pdf = dir.write("Guide.pdf", b"%PDF-1.7 pretend bytes \x00\xff");
    Saved {
        dir,
        json,
        html,
        pdf,
        report,
    }
}

fn five_of_seven(name: &str, date: &str) -> Signoff {
    Signoff {
        name: name.to_string(),
        date: date.to_string(),
        items: vec![true, true, true, true, true, false, false],
        pac: no_pac(),
    }
}

fn no_pac() -> PacInput {
    PacInput {
        result: "none".to_string(),
        version: String::new(),
        report_file: None,
    }
}

fn json_without_checklist(path: &Path) -> Value {
    let mut json: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("checklist");
    json
}

/// Tick 5 of the 7 items, type a name, pick a date, save: the JSON and the HTML
/// have them, and opening the report again shows the same.
#[test]
fn app_saves_the_signoff_in_the_report() {
    let saved = saved_report();
    assert_eq!(saved.report.checklist.items.len(), 7);
    assert!(saved.report.checklist.name.is_none());
    let json_before = json_without_checklist(&saved.json);
    let summary_before = saved.report.summary.clone();

    let view = save_signoff(&saved.json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap();

    // The JSON.
    let json: Value = serde_json::from_str(&fs::read_to_string(&saved.json).unwrap()).unwrap();
    assert_eq!(json["checklist"]["name"], "A. Reviewer");
    assert_eq!(json["checklist"]["date"], "2026-03-06");
    let done: Vec<bool> = json["checklist"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["done"].as_bool().unwrap())
        .collect();
    assert_eq!(done, [true, true, true, true, true, false, false]);
    // The text of the items did not change.
    for (item, expected) in json["checklist"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .zip(report::CHECKLIST)
    {
        assert_eq!(item["text"], expected);
    }
    // Nothing else in the JSON changed: not the verdict, the findings or the rest.
    assert_eq!(json_without_checklist(&saved.json), json_before);
    assert_eq!(view.report.summary, summary_before);

    // The HTML is the page of the signed report. It has no control: it shows
    // the sign-off as text.
    let html_text = fs::read_to_string(&saved.html).unwrap();
    assert_eq!(html_text, html::render(&view.report));
    assert!(!html_text.contains("<input") && !html_text.contains("<select"));
    assert_eq!(html_text.matches("<td>Checked</td>").count(), 5);
    assert_eq!(html_text.matches("<td>Not checked yet</td>").count(), 2);
    assert!(
        html_text
            .contains("Sign-off: Signed by A. Reviewer on 2026-03-06. 5 of 7 checks are checked.")
    );

    // Opening it again shows the same name, date and ticks.
    let again = read_report(&saved.json).unwrap();
    assert_eq!(again, view);
    assert_eq!(again.report.checklist.name.as_deref(), Some("A. Reviewer"));
    assert_eq!(again.report.checklist.date.as_deref(), Some("2026-03-06"));
    assert_eq!(again.pdf, Some(saved.pdf.display().to_string()));

    // Signing again replaces the sign-off.
    let second = Signoff {
        name: "B. Second".to_string(),
        date: String::new(),
        items: vec![true; 7],
        pac: no_pac(),
    };
    let view = save_signoff(&saved.json, &second).unwrap();
    assert_eq!(view.report.checklist.name.as_deref(), Some("B. Second"));
    assert_eq!(view.report.checklist.date, None);
    assert!(view.report.checklist.items.iter().all(|i| i.done));
    assert_eq!(read_report(&saved.json).unwrap(), view);
    assert_eq!(
        fs::read_to_string(&saved.html).unwrap(),
        html::render(&view.report)
    );
}

#[test]
fn the_signoff_never_touches_the_pdf_and_leaves_no_temporary_file() {
    let saved = saved_report();
    let pdf_before = fs::read(&saved.pdf).unwrap();
    let modified = fs::metadata(&saved.pdf).unwrap().modified().unwrap();
    save_signoff(&saved.json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap();
    assert_eq!(fs::read(&saved.pdf).unwrap(), pdf_before);
    assert_eq!(
        fs::metadata(&saved.pdf).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(
        saved.dir.names(),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.pdf"
        ]
    );
}

#[test]
fn a_date_that_was_not_given_is_not_filled_in() {
    let saved = saved_report();
    let view = save_signoff(&saved.json, &five_of_seven("  A. Reviewer  ", "")).unwrap();
    assert_eq!(
        view.report.checklist.name.as_deref(),
        Some("A. Reviewer"),
        "trimmed"
    );
    assert_eq!(view.report.checklist.date, None);
    let json: Value = serde_json::from_str(&fs::read_to_string(&saved.json).unwrap()).unwrap();
    assert_eq!(json["checklist"]["date"], Value::Null);
    assert!(
        fs::read_to_string(&saved.html)
            .unwrap()
            .contains("Signed by A. Reviewer (no date was given)")
    );
}

#[test]
fn an_empty_name_or_a_bad_request_is_refused_and_nothing_changes() {
    let saved = saved_report();
    let json_before = fs::read(&saved.json).unwrap();
    let html_before = fs::read(&saved.html).unwrap();
    let mut short = five_of_seven("A. Reviewer", "2026-03-06");
    short.items.pop();
    let cases = [
        (five_of_seven("", "2026-03-06"), "Type your name"),
        (five_of_seven("   \t ", "2026-03-06"), "Type your name"),
        (five_of_seven("A. Reviewer", "2026-02-30"), "not a date"),
        (five_of_seven("A. Reviewer", "yesterday"), "not a date"),
        (short, "7 items"),
    ];
    for (signoff, message) in cases {
        let error = save_signoff(&saved.json, &signoff).unwrap_err();
        assert!(error.message.contains(message), "{error}");
        assert_eq!(fs::read(&saved.json).unwrap(), json_before);
        assert_eq!(fs::read(&saved.html).unwrap(), html_before);
    }
    assert_eq!(saved.dir.names().len(), 3);
}

/// The files are written under temporary names and then moved, the JSON first.
/// When the first move fails, the old files are as they were and no temporary file
/// is left. Here the JSON cannot be replaced because it is read-only.
#[test]
fn a_failed_move_leaves_the_old_report_and_no_temporary_file() {
    let saved = saved_report();
    let json_before = fs::read(&saved.json).unwrap();
    let html_before = fs::read(&saved.html).unwrap();
    let writable = fs::metadata(&saved.json).unwrap().permissions();
    let mut read_only = writable.clone();
    read_only.set_readonly(true);
    fs::set_permissions(&saved.json, read_only).unwrap();
    let error = save_signoff(&saved.json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap_err();
    fs::set_permissions(&saved.json, writable).unwrap();
    assert!(
        error.message.contains("Guide.accessibility.json"),
        "{error}"
    );
    assert!(error.message.contains("not saved"), "{error}");
    assert_eq!(fs::read(&saved.json).unwrap(), json_before);
    assert_eq!(fs::read(&saved.html).unwrap(), html_before);
    assert_eq!(
        saved.dir.names(),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.pdf"
        ]
    );
}

/// The second move fails (here a folder has the name of the page): the JSON, the
/// record, has the sign-off, the page is as it was, no temporary file is left,
/// and the message says what was saved and what was not.
#[test]
fn a_half_saved_signoff_says_what_was_saved() {
    let saved = saved_report();
    fs::remove_file(&saved.html).unwrap();
    fs::create_dir(&saved.html).unwrap();
    let error = save_signoff(&saved.json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap_err();
    let message = &error.message;
    assert!(
        message.contains("sign-off is saved in")
            && message.contains("Guide.accessibility.json")
            && message.contains("Guide.accessibility.html")
            && message.contains("does not show the sign-off yet")
            && message.contains("save the sign-off again"),
        "{error}"
    );
    // The record has it, and the app reads it back.
    let again = read_report(&saved.json).unwrap();
    assert_eq!(again.report.checklist.name.as_deref(), Some("A. Reviewer"));
    assert_eq!(again.report.checklist.date.as_deref(), Some("2026-03-06"));
    assert!(saved.html.is_dir(), "the page was not replaced");
    assert_eq!(
        saved.dir.names(),
        [
            "Guide.accessibility.html",
            "Guide.accessibility.json",
            "Guide.pdf"
        ]
    );
    // The person closes what held the page and saves again: both files agree.
    fs::remove_dir(&saved.html).unwrap();
    let view = save_signoff(&saved.json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap();
    assert_eq!(
        fs::read_to_string(&saved.html).unwrap(),
        html::render(&view.report)
    );
}

#[test]
fn a_file_that_is_not_a_report_of_this_tool_is_refused() {
    let saved = saved_report();
    let not_ours = saved
        .dir
        .write("Other.accessibility.json", b"{\"hello\": 1}");
    let error = save_signoff(&not_ours, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap_err();
    assert!(
        error.message.contains("not a report that this tool wrote"),
        "{error}"
    );
    let wrong_name = saved
        .dir
        .write("Guide.json", saved.report.to_json().as_bytes());
    assert!(save_signoff(&wrong_name, &five_of_seven("A. Reviewer", "2026-03-06")).is_err());
    // A later layout of the report is not changed by an older tool.
    let mut json: Value = serde_json::from_str(&saved.report.to_json()).unwrap();
    json["schema_version"] = Value::from(4);
    let later = saved
        .dir
        .write("Later.accessibility.json", json.to_string().as_bytes());
    let before = fs::read(&later).unwrap();
    let error = save_signoff(&later, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap_err();
    assert!(error.message.contains("layout version 4"), "{error}");
    assert_eq!(fs::read(&later).unwrap(), before);
    assert!(!saved.dir.path().join("Later.accessibility.html").exists());
}

/// A report that came from a real run has a checklist of 7 items, and the
/// verdict of a report is not changed by a sign-off.
#[test]
fn a_signoff_does_not_change_the_verdict() {
    let saved = saved_report();
    assert_eq!(saved.report.summary.verdict, report::Verdict::Fail);
    let view = save_signoff(&saved.json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap();
    assert_eq!(view.report.summary.verdict, report::Verdict::Fail);
    assert_eq!(view.report.precheck, saved.report.precheck);
    assert_eq!(
        (view.report.summary.errors, view.report.summary.warnings),
        (saved.report.summary.errors, saved.report.summary.warnings)
    );
}

// ------------------------------------------------- the files of a guide

/// The page asks for the files of a guide when a document is chosen: the original,
/// the accessible copy, the folder, and whether the copy is there.
#[test]
fn working_info_names_the_original_the_copy_and_the_folder() {
    let dir = TempDir::new("w-info");
    let env = environment();
    let original = dir.write("Guide.docx", &fixture(RuleId::Doc002, true));
    let folder = dir.path().join("Guide - Adapdf");
    let copy = folder.join("Guide (accessible).docx");

    let info = commands::working_info(&original, &env);
    assert_eq!(info.chosen, original.display().to_string());
    assert_eq!(info.original, original.display().to_string());
    assert_eq!(info.copy, copy.display().to_string());
    assert_eq!(info.folder, folder.display().to_string());
    assert_eq!(info.chosen_folder, dir.path().display().to_string());
    assert!(!info.is_copy && !info.copy_exists);

    // After the copy is made, the same file says that the copy is there.
    fs::create_dir_all(&folder).unwrap();
    fs::write(&copy, b"x").unwrap();
    assert!(commands::working_info(&original, &env).copy_exists);

    // The copy itself: the original is named from it.
    let info = commands::working_info(&copy, &env);
    assert!(info.is_copy && info.copy_exists);
    assert_eq!(info.original, original.display().to_string());
    assert_eq!(info.copy, copy.display().to_string());
    assert_eq!(info.chosen_folder, folder.display().to_string());

    // A copy that is somewhere else: its original is named next to it, the files
    // of the guide are next to it, and nothing is made.
    let loose = dir.write("Other (accessible).docx", b"x");
    let info = commands::working_info(&loose, &env);
    assert!(info.is_copy && info.copy_exists);
    assert_eq!(
        info.original,
        dir.path().join("Other.docx").display().to_string()
    );
    assert_eq!(info.folder, dir.path().display().to_string());
}

#[test]
fn create_copy_makes_the_copy_with_no_fix_and_does_not_replace_one_unasked() {
    let dir = TempDir::new("w-copy");
    let env = environment();
    let bytes = fixture(RuleId::Doc002, true);
    let original = dir.write("Guide.docx", &bytes);
    let info = commands::create_copy(&original, false, &env).unwrap();
    assert!(info.is_copy && info.copy_exists);
    assert_eq!(
        info.chosen,
        dir.path()
            .join("Guide - Adapdf")
            .join("Guide (accessible).docx")
            .display()
            .to_string()
    );
    assert_eq!(
        fs::read(&info.chosen).unwrap(),
        bytes,
        "a copy, with no fix"
    );
    assert_eq!(fs::read(&original).unwrap(), bytes);

    // A second time, with no choice, it does not replace the copy.
    fs::write(&info.chosen, b"edited in Word").unwrap();
    let error = commands::create_copy(&original, false, &env).unwrap_err();
    assert!(error.message.contains("already exists"), "{error}");
    assert_eq!(fs::read(&info.chosen).unwrap(), b"edited in Word");
    // Starting over replaces it.
    commands::create_copy(&original, true, &env).unwrap();
    assert_eq!(fs::read(&info.chosen).unwrap(), bytes);
    // The copy is its own copy.
    let again = commands::create_copy(Path::new(&info.chosen), false, &env).unwrap();
    assert_eq!(again.chosen, info.chosen);
}

#[test]
fn apply_fixes_refuses_an_existing_copy_unless_the_person_started_over() {
    let dir = TempDir::new("w-apply");
    let env = environment();
    let options = FixOptions::default();
    let bytes = fixture(RuleId::Doc002, true);
    let original = dir.write("Guide.docx", &bytes);
    let offers = fix_offers(&original, &env, &options).unwrap();
    let choices = [Choice {
        rule_id: RuleId::Doc002,
        position: 0,
        input: Answer::Confirm,
    }];
    let first = commands::apply_fixes(&original, &choices, false, &env, &options).unwrap();
    let copy = first.copy.clone().expect("a copy");
    assert_eq!(first.working_file, copy);
    let after_first = fs::read(&copy).unwrap();
    assert_eq!(offers.len(), 1);

    // The copy exists: with no choice, nothing is written.
    let error = commands::apply_fixes(&original, &choices, false, &env, &options).unwrap_err();
    assert!(error.message.contains("already exists"), "{error}");
    assert_eq!(fs::read(&copy).unwrap(), after_first);
    // Continuing is working in the copy: it has no DOC-002 left, so there is
    // nothing to choose, and the file is as it was.
    let copy_path = PathBuf::from(&copy);
    assert!(fix_offers(&copy_path, &env, &options).unwrap().is_empty());
    // Starting over replaces it, with a new log of one fix.
    fs::write(&copy, b"edited in Word").unwrap();
    let again = commands::apply_fixes(&original, &choices, true, &env, &options).unwrap();
    assert_eq!(again.copy.as_deref(), Some(copy.as_str()));
    assert_eq!(
        fs::read(&copy).unwrap(),
        after_first,
        "the same fix, the same file"
    );
    assert_eq!(fs::read(&original).unwrap(), bytes);
}

/// The app opens only the accessible copy of a guide in Word. The original is
/// never opened by the app, so that the person edits the copy. The check is made
/// before the opener is called (the tests never start Word).
#[test]
fn only_the_accessible_copy_is_opened_in_word() {
    let dir = TempDir::new("w-open");
    let copy = dir.write("Guide (accessible).docx", b"x");
    assert_eq!(commands::check_open_in_word(&copy).unwrap(), copy);
    let original = dir.write("Guide.docx", b"x");
    let error = commands::check_open_in_word(&original).unwrap_err();
    assert!(error.message.contains("not an accessible copy"), "{error}");
    for bad in [
        "Guide (accessible).doc",
        "Guide (accessible).pdf",
        "Guide (accessible).docx.exe",
        "Guide.accessibility.html",
    ] {
        let path = dir.write(bad, b"x");
        assert!(commands::check_open_in_word(&path).is_err(), "{bad}");
    }
    // Not there, not absolute, a web address, and a network share.
    assert!(commands::check_open_in_word(&dir.path().join("Other (accessible).docx")).is_err());
    assert!(commands::check_open_in_word(Path::new("Guide (accessible).docx")).is_err());
    for address in [
        "https://example.com/Guide (accessible).docx",
        r"\\server\share\Guide (accessible).docx",
    ] {
        assert!(
            commands::check_open_in_word(Path::new(address)).is_err(),
            "{address}"
        );
    }
}

#[test]
fn only_the_folder_of_a_docx_is_opened() {
    let dir = TempDir::new("w-folder");
    let file = dir.write("Guide.docx", b"x");
    assert_eq!(
        commands::check_open_folder(&file).unwrap(),
        dir.path().to_path_buf()
    );
    // The folder of a report is opened too (the result screen has no .docx).
    let report = dir.write("Guide.accessibility.json", b"{}");
    assert_eq!(
        commands::check_open_folder(&report).unwrap(),
        dir.path().to_path_buf()
    );
    for bad in ["Guide.pdf", "Guide.exe", ".docx", ".accessibility.json"] {
        let path = dir.write(bad, b"x");
        assert!(commands::check_open_folder(&path).is_err(), "{bad}");
    }
    // A folder is not a document, and a path that is not there is not opened.
    assert!(commands::check_open_folder(dir.path()).is_err());
    assert!(commands::check_open_folder(&dir.path().join("Missing.docx")).is_err());
    assert!(commands::check_open_folder(Path::new(r"\\server\share\Guide.docx")).is_err());
    assert!(commands::check_open_folder(Path::new("Guide.docx")).is_err());
}

// --------------------------------------------------- PAC in the sign-off

fn with_pac(result: &str, version: &str, file: Option<&str>) -> Signoff {
    let mut signoff = five_of_seven("A. Reviewer", "2026-03-06");
    signoff.pac = PacInput {
        result: result.to_string(),
        version: version.to_string(),
        report_file: file.map(str::to_string),
    };
    signoff
}

/// The PAC result, the version and the report file are saved with the sign-off, in
/// the JSON and in the page, and read back. The file is saved as a path relative
/// to the folder of the report.
#[test]
fn the_pac_result_is_saved_with_the_signoff() {
    let saved = saved_report();
    let folder = saved.json.parent().unwrap().to_path_buf();
    fs::create_dir_all(folder.join("PAC")).unwrap();
    fs::write(folder.join("PAC").join("Guide-pac.pdf"), b"%PDF").unwrap();

    // A file that the file chooser gave (an absolute path) is saved relative.
    let absolute = folder.join("PAC").join("Guide-pac.pdf");
    let view = save_signoff(
        &saved.json,
        &with_pac("pass", "  2024.1 ", Some(absolute.to_str().unwrap())),
    )
    .unwrap();
    let pac = &view.report.pac;
    assert_eq!(pac.result.as_deref(), Some("pass"));
    assert_eq!(pac.pac_version.as_deref(), Some("2024.1"));
    assert_eq!(pac.report_file.as_deref(), Some("PAC/Guide-pac.pdf"));
    let json: Value = serde_json::from_str(&fs::read_to_string(&saved.json).unwrap()).unwrap();
    assert_eq!(json["pac"]["result"], "pass");
    assert_eq!(json["pac"]["report_file"], "PAC/Guide-pac.pdf");
    let page = fs::read_to_string(&saved.html).unwrap();
    assert!(
        page.contains(
            "PAC: Pass, report PAC/Guide-pac.pdf, PAC 2024.1, recorded by A. Reviewer on 2026-03-06"
        ),
        "{page}"
    );
    assert_eq!(read_report(&saved.json).unwrap(), view);

    // A relative file that is there is accepted as it is. A fail without a file or
    // a version is a fail.
    let view = save_signoff(
        &saved.json,
        &with_pac("fail", "", Some("PAC/Guide-pac.pdf")),
    )
    .unwrap();
    assert_eq!(view.report.pac.result.as_deref(), Some("fail"));
    assert_eq!(view.report.pac.pac_version, None);
    let view = save_signoff(&saved.json, &with_pac("fail", "", None)).unwrap();
    assert_eq!(view.report.pac.report_file, None);
    // "Not checked yet" clears everything.
    let view = save_signoff(
        &saved.json,
        &with_pac("none", "2024.1", Some("PAC/Guide-pac.pdf")),
    )
    .unwrap();
    assert_eq!(view.report.pac, adapdf::report::PacSection::default());
    assert!(
        fs::read_to_string(&saved.html)
            .unwrap()
            .contains("PAC: not checked yet")
    );
}

#[test]
fn a_pac_file_that_is_not_there_or_not_with_the_report_is_refused() {
    let saved = saved_report();
    let folder = saved.json.parent().unwrap().to_path_buf();
    let json_before = fs::read(&saved.json).unwrap();
    let html_before = fs::read(&saved.html).unwrap();
    let outside = TempDir::new("pac-outside");
    let elsewhere = outside.write("pac.pdf", b"%PDF");
    fs::write(folder.join("real.pdf"), b"%PDF").unwrap();
    fs::create_dir(folder.join("a-folder")).unwrap();
    for (file, message) in [
        ("missing.pdf".to_string(), "is not there"),
        (elsewhere.display().to_string(), "in the folder"),
        ("../pac.pdf".to_string(), "not a file in the folder"),
        ("a-folder".to_string(), "is not there"),
        (String::new(), ""),
    ] {
        let signoff = with_pac("pass", "1", Some(&file));
        if file.is_empty() {
            // An empty name is no file: the sign-off is saved with none.
            assert_eq!(
                save_signoff(&saved.json, &signoff)
                    .unwrap()
                    .report
                    .pac
                    .report_file,
                None
            );
            fs::write(&saved.json, &json_before).unwrap();
            fs::write(&saved.html, &html_before).unwrap();
            continue;
        }
        let error = save_signoff(&saved.json, &signoff).unwrap_err();
        assert!(error.message.contains(message), "{file}: {error}");
        assert_eq!(fs::read(&saved.json).unwrap(), json_before, "{file}");
        assert_eq!(fs::read(&saved.html).unwrap(), html_before, "{file}");
    }
    let error = save_signoff(&saved.json, &with_pac("maybe", "", None)).unwrap_err();
    assert!(error.message.contains("none, pass or fail"), "{error}");
    assert_eq!(fs::read(&saved.json).unwrap(), json_before);
}

mod contract;
