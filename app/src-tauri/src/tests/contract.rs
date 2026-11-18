//! The contract between the Rust commands and the page (row A-09).
//!
//! `app/src/types.ts` is written by hand, and a page test that uses made-up values
//! cannot see that Rust sends something else. So this file serialises real values
//! of every type that the page receives, from the same functions that the Tauri
//! commands call, into the files in `app/src/test/contract/`. The page tests
//! (`app/src/test/contract.test.tsx`) read those files, check their shape against
//! the TypeScript types, and render the screens from them.
//!
//! * `the_samples_for_the_page_are_what_rust_serialises` fails when a file differs
//!   from what Rust gives now. Change a type, then write the files again:
//!   `ADAPDF_REGENERATE_CONTRACT=1 cargo test -p adapdf-app --lib contract`, and
//!   look at the diff.
//! * `calls.json` is the other way: the arguments that the page sends. The page
//!   test checks that the page sends exactly these, and
//!   `the_arguments_of_the_page_are_the_parameters_of_the_commands` checks that they
//!   are the parameters of the Tauri commands in `shell.rs` and that Rust reads the
//!   values.

use std::collections::BTreeMap;
use std::env;

use adapdf::fixtures::{Para, clean};
use adapdf::pdfmeta::{self, Change, Field, TableScopeChange, UnscopedHeader};
use adapdf::report::{ExportInfo, Parameter};
use adapdf::verapdf::{Profile, Validation, WcagOutcome, parse};
use serde::Serialize;

use super::export::NoStages;
use super::*;
use crate::commands::{ApplyResult, CheckResult, ExportResult, OfferView, ReportView, WorkingInfo};

const REGENERATE: &str = "ADAPDF_REGENERATE_CONTRACT";
const UA1_PASS: &str = include_str!("../../../../tests/data/verapdf/ua1-pass.json");
const UA1_FAIL: &str = include_str!("../../../../tests/data/verapdf/ua1-figure-without-alt.json");

/// Where the page tests read the samples. `calls.json` is written by hand.
fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/test/contract")
}

/// The JSON that Rust gives for each value, by file name.
struct Samples {
    /// The folders of the runs. A path in them has the process number in it, so it
    /// is written as `C:\Docs`, which is what the page tests show.
    folders: Vec<TempDir>,
    files: BTreeMap<String, String>,
}

impl Samples {
    /// A new folder with `Guide.docx` in it. The path of the file.
    fn docx(&mut self, bytes: &[u8]) -> PathBuf {
        let folder = TempDir::new("contract");
        let path = folder.write("Guide.docx", bytes);
        self.folders.push(folder);
        path
    }

    fn add(&mut self, name: &str, value: &impl Serialize) {
        self.add_text(name, &serde_json::to_string_pretty(value).unwrap());
    }

    fn add_text(&mut self, name: &str, json: &str) {
        // A path in a JSON string has its backslashes doubled.
        let escaped = |text: &str| text.replace('\\', "\\\\");
        let mut json = json.to_string();
        for folder in &self.folders {
            // With the separator, so that `...-1` is not found in `...-10`.
            let folder = format!("{}\\", folder.path().display());
            json = json.replace(&escaped(&folder), &escaped(r"C:\Docs\"));
            // The folder itself, at the end of a string (`chosen_folder`).
            let bare = format!("{}\"", escaped(folder.trim_end_matches('\\')));
            json = json.replace(&bare, &format!("{}\"", escaped(r"C:\Docs")));
        }
        assert!(
            !json.contains("app-tests"),
            "{name} has a path of this machine"
        );
        let file = format!("{name}.json");
        assert!(
            self.files
                .insert(
                    file,
                    format!(
                        "{json}
"
                    )
                )
                .is_none(),
            "{name} is made twice"
        );
    }
}

/// A document that is checked and offered fixes, as the page does when a person
/// chooses it.
struct Document {
    name: &'static str,
    bytes: Vec<u8>,
}

fn documents() -> Vec<Document> {
    let from_catalogue = |name, rule, fails| Document {
        name,
        bytes: fixture(rule, fails),
    };
    vec![
        from_catalogue("lst002-fail", RuleId::Lst002, true),
        from_catalogue("doc001-fail", RuleId::Doc001, true),
        from_catalogue("doc001-pass", RuleId::Doc001, false),
        from_catalogue("img001-fail", RuleId::Img001, true),
        from_catalogue("img007-fail", RuleId::Img007, true),
        from_catalogue("txt004-fail", RuleId::Txt004, true),
        // A link with no text: the tool will not give it a ScreenTip.
        Document {
            name: "lnk004-no-text-fail",
            bytes: clean()
                .paragraph(Para::empty().hyperlink("", "https://example.com/"))
                .build(),
        },
    ]
}

fn answer(offer: &OfferView) -> Answer {
    match (offer.ask, offer.fix) {
        (true, FixId::SetAltText) => Answer::Decorative,
        (true, FixId::SetTitle) => Answer::Text {
            text: "A title typed by a person".to_string(),
        },
        _ => Answer::Confirm,
    }
}

fn choice(offer: &OfferView, input: Answer) -> Choice {
    Choice {
        rule_id: offer.finding.rule_id,
        position: offer.position,
        input,
    }
}

/// Every value that the page receives, as the commands make them.
fn samples() -> Samples {
    let mut samples = Samples {
        folders: Vec::new(),
        files: BTreeMap::new(),
    };
    let options = FixOptions::default();
    let env = environment();

    // `check` and `fix_offers`, and `apply_fixes` with what the page can send.
    for document in documents() {
        let path = samples.docx(&document.bytes);
        let checked: CheckResult = check(&path, &FixtureFonts).unwrap();
        let offers: Vec<OfferView> = fix_offers(&path, &env, &options).unwrap();
        samples.add(&format!("check-{}", document.name), &checked);
        samples.add(&format!("offers-{}", document.name), &offers);

        // A fix that Word does at export is not offered to choose, but a command
        // that is sent one must say so.
        if let Some(at_export) = offers
            .iter()
            .find(|o| o.availability == Availability::AtExport)
        {
            let path = samples.docx(&document.bytes);
            let choices = [choice(at_export, Answer::Confirm)];
            let result = commands::apply_fixes(&path, &choices, false, &env, &options).unwrap();
            assert!(!result.at_export.is_empty());
            samples.add(&format!("apply-at-export-{}", document.name), &result);
        }

        // The page chooses every fix that is ready, with the person's input.
        let choices: Vec<Choice> = offers
            .iter()
            .filter(|o| o.availability == Availability::Ready)
            .map(|o| choice(o, answer(o)))
            .collect();
        let applied: ApplyResult =
            commands::apply_fixes(&path, &choices, false, &env, &options).unwrap();
        samples.add(&format!("apply-{}", document.name), &applied);
    }

    // A fix that the tool refuses: text of only spaces, and a finding that is gone.
    let path = samples.docx(&fixture(RuleId::Doc001, true));
    let offers = fix_offers(&path, &env, &options).unwrap();
    let blank = Answer::Text {
        text: "   ".to_string(),
    };
    let gone = Choice {
        rule_id: RuleId::Img001,
        position: 3,
        input: Answer::Decorative,
    };
    let choices = [choice(&offers[0], blank), gone];
    let refused = commands::apply_fixes(&path, &choices, false, &env, &options).unwrap();
    assert_eq!(refused.refused.len(), 2);
    samples.add("apply-refused", &refused);

    // The files of a guide: an original with no copy, the copy that is made (with
    // no fix, so that the person can open it in Word), and the original once the
    // copy exists (the page then asks which to use).
    let path = samples.docx(&fixture(RuleId::Doc001, true));
    let before: WorkingInfo = commands::working_info(&path, &env);
    assert!(!before.copy_exists && !before.is_copy);
    samples.add("working-info-original", &before);
    let made: WorkingInfo = commands::create_copy(&path, false, &env).unwrap();
    assert!(made.is_copy && made.copy_exists);
    samples.add("working-info-copy", &made);
    let with_copy: WorkingInfo = commands::working_info(&path, &env);
    assert!(with_copy.copy_exists && !with_copy.is_copy);
    samples.add("working-info-original-with-copy", &with_copy);

    // The reports on disk: unsigned, as `export` writes it, and signed.
    let saved = saved_report();
    let json = saved.json.clone();
    samples.folders.push(saved.dir);
    samples.add("report-view-unsigned", &read_report(&json).unwrap());
    samples.add_text(
        "report-file-lst002-fail",
        fs::read_to_string(&json).unwrap().trim_end(),
    );
    let signed = save_signoff(&json, &five_of_seven("A. Reviewer", "2026-03-06")).unwrap();
    samples.add("report-view-signed", &signed);

    exports(&mut samples);
    samples
}

/// The export: stage 1 finds errors (no Word is started), the tool fails, and the
/// two results with a PDF, built from the report library as `export_docx` builds
/// them (the library tests run the whole pipeline with the real Word and veraPDF).
fn exports(samples: &mut Samples) {
    // A Fail: the document has an error, so no PDF is made.
    let path = samples.docx(&fixture(RuleId::Doc001, true));
    let blocked: ExportResult = commands::export(&path, &NoStages, &FixtureFonts, clock()).unwrap();
    assert_eq!(blocked.exit_code, 1);
    samples.add("export-fail-blocked", &blocked);

    // A tool failure is an error with a message.
    let missing = path.with_file_name("Missing.docx");
    let error = commands::export(&missing, &NoStages, &FixtureFonts, clock()).unwrap_err();
    samples.add("command-error-export", &error);
    let error = check(&missing, &FixtureFonts).unwrap_err();
    samples.add("command-error-check", &error);

    // A Pass and a Fail with a PDF.
    let json = Path::new(r"C:\Docs\Guide - Adapdf\Guide.accessibility.json");
    let validation = |text: &str| Validation {
        ua1: parse(text, Profile::Ua1).unwrap(),
        wcag: WcagOutcome::NotAvailable,
    };
    let metadata = |issues| pdfmeta::Report {
        changes: vec![Change {
            field: Field::InfoTitle,
            what: Field::InfoTitle.describe().to_string(),
            old: None,
            new: "Guide".to_string(),
        }],
        table_scopes: vec![TableScopeChange {
            table: 1,
            column: 2,
            row: 3,
        }],
        unscoped_headers: vec![UnscopedHeader {
            table: 2,
            row: 3,
            cell: 2,
        }],
        issues,
    };
    let word = ExportInfo {
        word_build: "16.0.20527.20000".to_string(),
        parameters: vec![Parameter {
            name: "ExportFormat".to_string(),
            value: "wdExportFormatPDF".to_string(),
        }],
        fields_updated: 2,
        tables_of_contents_updated: 1,
        first_failed_field: None,
        seconds: 1.5,
    };
    for (name, verapdf, issues) in [
        ("export-pass", UA1_PASS, Vec::new()),
        (
            "export-fail-verapdf",
            UA1_FAIL,
            vec![pdfmeta::Issue {
                kind: pdfmeta::IssueKind::NoTitleToCopy,
                severity: adapdf::findings::Severity::Error,
                message: "The PDF has no title, and the Word document has none to copy."
                    .to_string(),
                how_to_fix: "Type a title in the document properties in Word.".to_string(),
            }],
        ),
    ] {
        let report = report::build(Input {
            document: "Guide.docx".to_string(),
            generated_local: "2026-03-05 10:15:30".to_string(),
            findings: Vec::new(),
            applied_fixes: Vec::new(),
            original: None,
            pdf: Some("Guide.pdf".to_string()),
            export: Some(word.clone()),
            metadata: Some(metadata(issues)),
            verapdf: Some(validation(verapdf)),
        });
        let result = ExportResult {
            exit_code: u8::from(report.summary.verdict == report::Verdict::Fail),
            view: ReportView::new(report, json).unwrap(),
            console: "Guide.docx: result\n".to_string(),
            files: vec![
                r"C:\Docs\Guide - Adapdf\Guide.pdf".to_string(),
                r"C:\Docs\Guide - Adapdf\Guide.accessibility.html".to_string(),
                r"C:\Docs\Guide - Adapdf\Guide.accessibility.json".to_string(),
            ],
        };
        samples.add(name, &result);
    }
}

#[test]
fn the_samples_for_the_page_are_what_rust_serialises() {
    let samples = samples();
    let dir = contract_dir();
    if env::var_os(REGENERATE).is_some() {
        fs::create_dir_all(&dir).unwrap();
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            if entry.file_name() != "calls.json" {
                fs::remove_file(entry.path()).unwrap();
            }
        }
        for (name, text) in &samples.files {
            fs::write(dir.join(name), text).unwrap();
        }
        return;
    }
    let mut on_disk: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}. {REGENERATE}=1 writes the samples.", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "calls.json")
        .collect();
    on_disk.sort();
    let made: Vec<&String> = samples.files.keys().collect();
    assert_eq!(
        on_disk.iter().collect::<Vec<_>>(),
        made,
        "the files in {} are not the samples that Rust makes. Run `{REGENERATE}=1 cargo test -p adapdf-app --lib contract`.",
        dir.display()
    );
    for (name, text) in &samples.files {
        // A checkout may change the line ends of a text file.
        let on_disk = fs::read_to_string(dir.join(name))
            .unwrap()
            .replace("\r\n", "\n");
        assert!(
            &on_disk == text,
            "{name} is not what Rust serialises now. Run `{REGENERATE}=1 cargo test -p adapdf-app --lib contract` and look at the diff.\n--- Rust now ---\n{text}\n--- file ---\n{on_disk}"
        );
    }
}

/// The samples have every kind of value that the page has a screen for.
#[test]
fn the_samples_have_every_kind_of_value() {
    let samples = samples();
    let value = |name: &str| -> serde_json::Value {
        serde_json::from_str(&samples.files[&format!("{name}.json")]).unwrap()
    };
    let all = |prefix: &str| -> Vec<serde_json::Value> {
        samples
            .files
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(_, text)| serde_json::from_str(text).unwrap())
            .collect()
    };
    let offers: Vec<serde_json::Value> = all("offers-")
        .into_iter()
        .flat_map(|v| v.as_array().unwrap().clone())
        .collect();
    let has = |f: &dyn Fn(&serde_json::Value) -> bool| offers.iter().any(f);
    assert!(has(
        &|o| o["availability"]["status"] == "ready" && o["ask"] == false
    ));
    assert!(has(
        &|o| o["availability"]["status"] == "ready" && o["ask"] == true
    ));
    assert!(has(&|o| o["availability"]["status"] == "at-export"));
    assert!(has(&|o| o["availability"]["status"] == "unavailable"
        && o["availability"]["reason"].is_string()));
    assert!(has(&|o| o["fix"] == "set-alt-text"));
    assert!(has(&|o| o["fix"] == "mark-annotations-decorative"));
    assert_eq!(
        value("check-doc001-pass")["findings"],
        serde_json::json!([])
    );
    assert_eq!(value("check-lst002-fail")["exit_code"], 1);
    assert_eq!(value("check-txt004-fail")["exit_code"], 0);
    let applied = value("apply-lst002-fail");
    assert!(applied["copy"].is_string() && !applied["applied"].as_array().unwrap().is_empty());
    assert!(
        applied["working_file"]
            .as_str()
            .unwrap()
            .ends_with("(accessible).docx")
    );
    assert!(
        applied["original"]
            .as_str()
            .unwrap()
            .ends_with("Guide.docx")
    );
    assert!(value("apply-refused")["copy"].is_null());
    assert!(
        value("apply-refused")["working_file"]
            .as_str()
            .unwrap()
            .ends_with("Guide.docx")
    );
    assert_eq!(value("working-info-original")["copy_exists"], false);
    assert_eq!(value("working-info-copy")["is_copy"], true);
    assert_eq!(
        value("working-info-original-with-copy")["copy_exists"],
        true
    );
    assert_eq!(value("working-info-original-with-copy")["is_copy"], false);
    // The title offer has its one-click texts.
    assert!(has(
        &|o| o["fix"] == "set-title" && o["suggestions"].as_array().unwrap().len() == 2
    ));
    assert!(
        !value("apply-refused")["refused"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !value("apply-at-export-txt004-fail")["at_export"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        value("export-pass")["view"]["report"]["summary"]["verdict"],
        "pass"
    );
    assert_eq!(
        value("export-fail-verapdf")["view"]["report"]["summary"]["verdict"],
        "fail"
    );
    let blocked = value("export-fail-blocked");
    assert_eq!(blocked["view"]["report"]["summary"]["verdict"], "fail");
    assert!(blocked["view"]["pdf"].is_null());
    assert_eq!(blocked["view"]["report"]["verapdf"]["status"], "not-run");
    assert!(value("command-error-export")["message"].is_string());
    assert!(value("report-view-unsigned")["report"]["checklist"]["name"].is_null());
    assert_eq!(
        value("report-view-signed")["report"]["checklist"]["name"],
        "A. Reviewer"
    );
}

// ------------------------------------------------------------ arguments

/// The parameters of each `#[tauri::command]` in `shell.rs` that the page sets
/// (not those that Tauri fills in, such as the `AppHandle`), and the names in
/// `generate_handler!`.
fn commands_of_the_shell() -> (BTreeMap<String, Vec<String>>, Vec<String>) {
    let source =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell.rs")).unwrap();
    let mut commands = BTreeMap::new();
    for part in source.split("#[tauri::command]").skip(1) {
        let signature = part.split("async fn ").nth(1).expect("a command is a fn");
        let (name, rest) = signature.split_once('(').unwrap();
        let (parameters, _) = rest.split_once(") ->").unwrap();
        let mut names = Vec::new();
        let (mut depth, mut start) = (0, 0);
        let mut items = Vec::new();
        for (i, c) in parameters.char_indices() {
            match c {
                '<' | '(' => depth += 1,
                '>' | ')' => depth -= 1,
                ',' if depth == 0 => {
                    items.push(&parameters[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        items.push(&parameters[start..]);
        for item in items.into_iter().map(str::trim).filter(|i| !i.is_empty()) {
            let (parameter, kind) = item.split_once(':').unwrap();
            if !kind.trim().starts_with("tauri::") {
                names.push(parameter.trim().to_string());
            }
        }
        commands.insert(name.trim().to_string(), names);
    }
    let handler = source
        .split("generate_handler![")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .unwrap();
    let mut handled: Vec<String> = handler
        .split(',')
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    handled.sort();
    (commands, handled)
}

/// Tauri turns the `snake_case` name of a parameter into a `camelCase` key.
fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn the_arguments_of_the_page_are_the_parameters_of_the_commands() {
    let (commands, handled) = commands_of_the_shell();
    assert_eq!(handled, commands.keys().cloned().collect::<Vec<_>>());
    assert_eq!(commands.len(), 11);

    let text = fs::read_to_string(contract_dir().join("calls.json")).unwrap();
    let calls: BTreeMap<String, BTreeMap<String, serde_json::Value>> =
        serde_json::from_str(&text).unwrap();
    assert_eq!(
        calls.keys().collect::<Vec<_>>(),
        commands.keys().collect::<Vec<_>>(),
        "the page calls the commands of the shell, and no other"
    );
    for (command, parameters) in &commands {
        let mut expected: Vec<String> = parameters.iter().map(|p| camel(p)).collect();
        expected.sort();
        let sent: Vec<&String> = calls[command].keys().collect();
        assert_eq!(
            sent.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            expected,
            "the page sends these arguments to `{command}`, and the command has these parameters"
        );
    }

    // Rust reads what the page sends: the choices and the sign-off.
    let choices: Vec<Choice> =
        serde_json::from_value(calls["apply_fixes"]["choices"].clone()).unwrap();
    assert!(choices.iter().any(|c| c.input == Answer::Confirm));
    assert!(
        choices
            .iter()
            .any(|c| matches!(c.input, Answer::Text { .. }))
    );
    assert!(choices.iter().any(|c| c.input == Answer::Decorative));
    let signoff: Signoff =
        serde_json::from_value(calls["save_signoff"]["signoff"].clone()).unwrap();
    assert_eq!(signoff, five_of_seven("A. Reviewer", "2026-03-06"));
}

/// The check of the parameters sees a rename (it is only worth something if it
/// does).
#[test]
fn the_parameter_names_are_camel_case_keys() {
    assert_eq!(camel("report_json"), "reportJson");
    assert_eq!(camel("path"), "path");
    assert_eq!(camel("a_b_c"), "aBC");
}
