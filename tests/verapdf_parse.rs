//! Stage 5: veraPDF. Everything here runs without Java or veraPDF: the JSON
//! samples in `tests/data/verapdf/` were recorded from the real veraPDF 1.28.2
//! on PDFs made by `tests/pdf_support` (rows S5-01 to S5-08 of the test plan).
//! The tests that run the real veraPDF are in `tests/verapdf_real.rs`.

mod pdf_support;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use adapdf::findings::{RuleId, Severity};
use adapdf::verapdf::rules::{self, TABLE};
use adapdf::verapdf::{
    Invocation, ProcessRunner, Profile, RunOutput, Runner, Tool, VeraPdfError, WcagOutcome, parse,
    validate_with,
};
use pdf_support::TempDir;
use serde_json::{Value, json};

const UA1_PASS: &str = include_str!("data/verapdf/ua1-pass.json");
const UA1_METADATA_MISSING: &str = include_str!("data/verapdf/ua1-metadata-missing.json");
const UA1_UNTAGGED_TEXT: &str = include_str!("data/verapdf/ua1-untagged-text.json");
const UA1_FIGURE_WITHOUT_ALT: &str = include_str!("data/verapdf/ua1-figure-without-alt.json");
const UA1_FIGURE_WITH_ALT: &str = include_str!("data/verapdf/ua1-figure-with-alt.json");
const UA1_NOT_A_PDF: &str = include_str!("data/verapdf/ua1-not-a-pdf.json");
const WCAG_METADATA_MISSING: &str = include_str!("data/verapdf/wcag-metadata-missing.json");

/// A fake that gives the answers in order and records every command.
struct Fake {
    answers: RefCell<VecDeque<Result<RunOutput, VeraPdfError>>>,
    calls: RefCell<Vec<Invocation>>,
}

impl Fake {
    fn new(answers: Vec<Result<RunOutput, VeraPdfError>>) -> Fake {
        Fake {
            answers: RefCell::new(answers.into()),
            calls: RefCell::new(Vec::new()),
        }
    }

    fn json(outputs: &[&str]) -> Fake {
        Fake::new(outputs.iter().map(|o| Ok(output(o, 0))).collect())
    }
}

impl Runner for Fake {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput, VeraPdfError> {
        self.calls.borrow_mut().push(invocation.clone());
        self.answers
            .borrow_mut()
            .pop_front()
            .expect("the fake has an answer for every command")
    }
}

fn output(stdout: &str, exit_code: i32) -> RunOutput {
    RunOutput {
        exit_code: Some(exit_code),
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

/// A `vendor` folder with empty stand-ins for Java and veraPDF.
fn fake_vendor(dir: &TempDir, wcag_profile: bool) -> Tool {
    let tool = Tool::in_vendor(dir.path());
    fs::create_dir_all(tool.java.parent().unwrap()).unwrap();
    fs::write(&tool.java, b"").unwrap();
    fs::create_dir_all(tool.verapdf.join("bin")).unwrap();
    if wcag_profile {
        let folder = tool.verapdf.join("profiles").join("rel").join("PDF_UA");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("WCAG-2-2-Machine.xml"), b"<profile/>").unwrap();
    }
    tool
}

fn pdf() -> &'static Path {
    Path::new("C:/docs/Guide.pdf")
}

const TIMEOUT: Duration = Duration::from_secs(5);

// S5-01
#[test]
fn verapdf_runs_the_ua1_profile_command() {
    let dir = TempDir::new("command");
    let tool = fake_vendor(&dir, false);
    let fake = Fake::json(&[UA1_PASS]);
    validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();

    let calls = fake.calls.borrow();
    assert_eq!(calls.len(), 1, "no WCAG profile: one run");
    let call = &calls[0];
    // The bundled Java runs it, not one on the PATH.
    assert_eq!(call.program, tool.java);
    assert!(call.program.starts_with(dir.path()));
    // The arguments of veraPDF are exactly these.
    let args: Vec<&OsString> = call.verapdf_args().iter().collect();
    assert_eq!(
        args,
        ["--flavour", "ua1", "--format", "json", "C:/docs/Guide.pdf"]
            .iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
            .iter()
            .collect::<Vec<_>>()
    );
    // The bundled jar files, and the main class of veraPDF's command line.
    let text: Vec<String> = call
        .args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(text[0], "-classpath");
    assert!(
        Path::new(&text[1]).starts_with(tool.verapdf.join("bin")),
        "{}",
        text[1]
    );
    assert!(text.contains(&"org.verapdf.apps.GreenfieldCliWrapper".to_string()));
    assert!(text.contains(&"-Dstdout.encoding=UTF-8".to_string()));
    assert_eq!(call.timeout, TIMEOUT);
}

// S5-02
#[test]
fn verapdf_json_failed_rules_become_findings() {
    // A recorded report with 3 failed rules.
    let result = parse(UA1_METADATA_MISSING, Profile::Ua1).unwrap();
    assert_eq!(result.findings.len(), 3);
    assert_eq!((result.failed_rules, result.failed_checks), (3, 3));
    assert_eq!(result.profile_name, "PDF/UA-1 validation profile");
    assert!(!result.passed());

    let ids: Vec<(&str, u32)> = result
        .findings
        .iter()
        .map(|f| (f.clause.as_str(), f.test_number))
        .collect();
    // In a fixed order: by clause, then by test number.
    assert_eq!(ids, vec![("5", 1), ("7.1", 9), ("7.1", 10)]);

    for finding in &result.findings {
        assert_eq!(finding.profile, Profile::Ua1);
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(finding.specification, "ISO 14289-1:2014");
        assert_eq!(finding.failed_checks, 1);
        assert!(!finding.message.is_empty());
        assert!(!finding.how_to_fix.is_empty());
        assert!(!finding.description.is_empty());
        assert!(
            finding
                .context
                .as_deref()
                .is_some_and(|c| c.starts_with("root"))
        );
        assert!(finding.error_message.is_some());
    }
    let title = &result.findings[1];
    assert!(title.context.as_deref().unwrap().contains("XMPPackage"));
    assert_eq!(title.pre_check, Some(RuleId::Doc001));
    assert!(title.message.contains("title"));
}

#[test]
fn a_report_with_seven_failed_rules_gives_seven_findings() {
    let result = parse(UA1_UNTAGGED_TEXT, Profile::Ua1).unwrap();
    assert_eq!(result.findings.len(), 7);
    assert_eq!(result.failed_rules, 7);
    let ids: Vec<(String, u32)> = result
        .findings
        .iter()
        .map(|f| (f.clause.clone(), f.test_number))
        .collect();
    let expected: Vec<(String, u32)> = [
        ("6.2", 1),
        ("7.1", 3),
        ("7.1", 8),
        ("7.1", 10),
        ("7.1", 11),
        ("7.2", 34),
        ("7.21.4.1", 1),
    ]
    .iter()
    .map(|(c, t)| (c.to_string(), *t))
    .collect();
    assert_eq!(ids, expected);
    assert!(result.findings.iter().all(|f| f.mapped));
}

/// A recorded report whose first failed rule is changed to the given rule.
fn with_rule(clause: &str, test_number: u32, description: &str) -> String {
    let mut value: Value = serde_json::from_str(UA1_FIGURE_WITHOUT_ALT).unwrap();
    let rule =
        &mut value["report"]["jobs"][0]["validationResult"][0]["details"]["ruleSummaries"][0];
    rule["clause"] = json!(clause);
    rule["testNumber"] = json!(test_number);
    rule["description"] = json!(description);
    value.to_string()
}

// S5-03
#[test]
fn verapdf_every_mapped_rule_has_a_message() {
    for entry in TABLE {
        let text = with_rule(entry.clause, entry.test_number, "veraPDF's own words");
        let result = parse(&text, Profile::Ua1).unwrap();
        let finding = &result.findings[0];
        let id = format!("{}-{}", entry.clause, entry.test_number);
        assert!(finding.mapped, "{id}");
        assert_eq!(finding.message, entry.message, "{id}");
        assert_eq!(finding.how_to_fix, entry.how_to_fix, "{id}");
        assert_eq!(finding.pre_check, entry.pre_check, "{id}");
        assert!(
            !entry.message.is_empty() && !entry.how_to_fix.is_empty(),
            "{id}"
        );
        if let Some(rule) = entry.pre_check {
            assert!(RuleId::ALL.contains(&rule), "{id}: {rule}");
        }
        // The message does not repeat the rule text; it is plain language.
        assert!(!finding.message.contains("veraPDF's own words"), "{id}");
        assert_eq!(finding.description, "veraPDF's own words", "{id}");
    }
}

/// The IDs in the table. A new entry must be added here as well, and so it
/// gets a place in the test above.
#[test]
fn the_mapped_rule_ids_are_these() {
    let ids: Vec<String> = TABLE
        .iter()
        .map(|e| format!("{}-{}", e.clause, e.test_number))
        .collect();
    let expected = [
        "5-1",
        "5-2",
        "6.2-1",
        "7.1-3",
        "7.1-4",
        "7.1-8",
        "7.1-9",
        "7.1-10",
        "7.1-11",
        "7.2-17",
        "7.2-18",
        "7.2-19",
        "7.2-20",
        "7.2-22",
        "7.2-29",
        "7.2-34",
        "7.2-41",
        "7.2-42",
        "7.2-43",
        "7.3-1",
        "7.4.2-1",
        "7.5-1",
        "7.5-2",
        "7.18.5-2",
        "7.21.4.1-1",
        "7.21.4.1-2",
        "7.21.7-1",
        "7.21.8-1",
    ];
    assert_eq!(ids, expected);
    let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "no ID is twice in the table");
}

#[test]
fn figure_without_alt_text_is_img_001() {
    let result = parse(UA1_FIGURE_WITHOUT_ALT, Profile::Ua1).unwrap();
    assert_eq!(result.findings.len(), 1);
    let finding = &result.findings[0];
    assert_eq!((finding.clause.as_str(), finding.test_number), ("7.3", 1));
    assert_eq!(finding.pre_check, Some(RuleId::Img001));
    assert!(finding.message.contains("alternative text"));
    assert!(finding.how_to_fix.contains("Alt Text"));
    assert!(
        finding.context.as_deref().unwrap().contains("Figure"),
        "{:?}",
        finding.context
    );
}

// S5-04
#[test]
fn verapdf_unmapped_rule_keeps_the_original_text() {
    assert!(rules::lookup("7.9", 1).is_none());
    let description = "Note tag shall have ID entry";
    let result = parse(&with_rule("7.9", 1, description), Profile::Ua1).unwrap();
    assert_eq!(result.findings.len(), 1, "it is not dropped");
    let finding = &result.findings[0];
    assert!(!finding.mapped);
    assert_eq!(finding.pre_check, None);
    assert_eq!(finding.description, description);
    assert!(finding.message.contains(description));
    assert!(finding.message.contains("clause 7.9"));
    assert!(finding.message.contains("test 1"));
    assert!(finding.how_to_fix.contains("ISO 14289-1:2014"));
    assert!(!result.passed());
}

// S5-05
#[test]
fn verapdf_wcag_profile_results_are_warnings() {
    // A veraPDF that has the profile: a second run, with `--profile`.
    let dir = TempDir::new("wcag");
    let tool = fake_vendor(&dir, true);
    let fake = Fake::json(&[UA1_PASS, WCAG_METADATA_MISSING]);
    let validation = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();
    let calls = fake.calls.borrow();
    assert_eq!(calls.len(), 2);
    let second: Vec<String> = calls[1]
        .verapdf_args()
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(second[0], "--profile");
    assert!(second[1].ends_with("WCAG-2-2-Machine.xml"));
    assert_eq!(&second[2..], ["--format", "json", "C:/docs/Guide.pdf"]);

    // The UA-1 run passed; the WCAG results are warnings and do not change that.
    assert!(validation.passed());
    let WcagOutcome::Ran(wcag) = &validation.wcag else {
        panic!("the WCAG profile ran: {:?}", validation.wcag);
    };
    assert_eq!(wcag.profile_name, "WCAG 2.2 machine validation profile");
    assert_eq!(wcag.findings.len(), 2);
    assert!(
        wcag.findings
            .iter()
            .all(|f| f.severity == Severity::Warning)
    );
    assert!(wcag.findings.iter().all(|f| f.profile == Profile::Wcag));
    assert_eq!(validation.findings().len(), 2);
}

#[test]
fn wcag_results_that_repeat_a_ua1_failure_are_not_shown_twice() {
    let dir = TempDir::new("wcag-dedupe");
    let tool = fake_vendor(&dir, true);
    // The same missing metadata is reported by both profiles. The WCAG report
    // has 7.1-9 and 7.1-10, and so has the UA-1 report.
    let fake = Fake::json(&[UA1_METADATA_MISSING, WCAG_METADATA_MISSING]);
    let validation = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();
    assert!(!validation.passed());
    let WcagOutcome::Ran(wcag) = &validation.wcag else {
        panic!("the WCAG profile ran");
    };
    assert!(wcag.findings.is_empty(), "{:?}", wcag.findings);
    assert_eq!(validation.ua1.findings.len(), 3);
    assert_eq!(validation.findings().len(), 3);
}

#[test]
fn without_the_wcag_profile_there_is_no_second_run() {
    let dir = TempDir::new("no-wcag");
    let tool = fake_vendor(&dir, false);
    let fake = Fake::json(&[UA1_PASS]);
    let validation = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();
    assert_eq!(fake.calls.borrow().len(), 1);
    assert_eq!(validation.wcag, WcagOutcome::NotAvailable);
    assert!(validation.passed());
}

#[test]
fn a_wcag_run_that_fails_does_not_change_the_result() {
    let dir = TempDir::new("wcag-fails");
    let tool = fake_vendor(&dir, true);
    let fake = Fake::new(vec![
        Ok(output(UA1_PASS, 0)),
        Err(VeraPdfError::TimedOut(5)),
    ]);
    let validation = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();
    assert!(validation.passed());
    let WcagOutcome::Failed(reason) = &validation.wcag else {
        panic!("{:?}", validation.wcag);
    };
    assert!(reason.contains("5 seconds"));

    // And a WCAG failure with a failed UA-1 run: still not a pass.
    let fake = Fake::json(&[UA1_FIGURE_WITHOUT_ALT, "not json"]);
    let validation = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();
    assert!(!validation.passed());
    assert!(matches!(validation.wcag, WcagOutcome::Failed(_)));
}

// S5-06
#[test]
fn verapdf_missing_gives_a_typed_error() {
    let dir = TempDir::new("missing");

    // No vendor folder at all: Java is named first.
    let tool = Tool::in_vendor(&dir.path().join("nothing-here"));
    let fake = Fake::json(&[]);
    let error = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap_err();
    assert!(
        matches!(error, VeraPdfError::JavaMissing(ref p) if *p == tool.java),
        "{error}"
    );
    assert!(error.to_string().contains("Java runtime"));
    assert!(error.to_string().contains("nothing-here"));
    assert!(fake.calls.borrow().is_empty(), "nothing was started");

    // A veraPDF folder without the Java runtime.
    let only_vera = Tool::in_vendor(&dir.path().join("only-vera"));
    fs::create_dir_all(only_vera.verapdf.join("bin")).unwrap();
    let error = validate_with(&only_vera, pdf(), &fake, TIMEOUT).unwrap_err();
    assert!(matches!(error, VeraPdfError::JavaMissing(_)), "{error}");

    // Java is there; veraPDF is not.
    let only_java = Tool::in_vendor(&dir.path().join("only-java"));
    fs::create_dir_all(only_java.java.parent().unwrap()).unwrap();
    fs::write(&only_java.java, b"").unwrap();
    let error = validate_with(&only_java, pdf(), &fake, TIMEOUT).unwrap_err();
    assert!(
        matches!(error, VeraPdfError::VeraPdfMissing(ref p) if *p == only_java.verapdf),
        "{error}"
    );
    assert!(error.to_string().starts_with("veraPDF is missing"));
    assert!(fake.calls.borrow().is_empty());
}

#[test]
fn the_tool_is_looked_for_in_a_vendor_folder() {
    // Whether or not `vendor/` exists, the answer names the bundled Java and the
    // veraPDF folder, so that a missing one can be reported by name.
    let tool = Tool::locate();
    let java = tool.java.to_string_lossy().replace('\\', "/");
    assert!(java.contains("vendor/jre/bin/java"), "{java}");
    let home = tool.verapdf.to_string_lossy().replace('\\', "/");
    assert!(home.ends_with("vendor/verapdf"), "{home}");
}

#[test]
fn the_tool_can_be_given_other_paths() {
    let dir = TempDir::new("injected");
    let java = dir.write("my-java.exe", b"");
    let home = dir.path().join("my-verapdf");
    fs::create_dir_all(home.join("bin")).unwrap();
    let tool = Tool::new(&java, &home);
    tool.check().unwrap();
    let fake = Fake::json(&[UA1_PASS]);
    validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap();
    assert_eq!(fake.calls.borrow()[0].program, java);
}

// S5-07
#[test]
fn verapdf_crash_or_bad_output_is_not_a_pass() {
    let dir = TempDir::new("crash");
    let tool = fake_vendor(&dir, false);
    let cases: Vec<(&str, Result<RunOutput, VeraPdfError>)> = vec![
        (
            "exits with an error and writes nothing",
            Ok(RunOutput {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "Exception in thread main: out of memory\n".to_string(),
            }),
        ),
        (
            "is killed",
            Ok(RunOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
            }),
        ),
        ("writes nothing and exits 0", Ok(output("", 0))),
        ("writes only spaces", Ok(output("  \n ", 0))),
        ("writes text that is not JSON", Ok(output("Hello", 0))),
        ("writes half a JSON text", Ok(output(&UA1_PASS[..300], 0))),
        ("writes JSON of another shape", Ok(output("{\"a\": 1}", 0))),
        (
            "writes a report with no jobs",
            Ok(output("{\"report\": {\"jobs\": []}}", 0)),
        ),
        ("could not read the PDF", Ok(output(UA1_NOT_A_PDF, 7))),
        (
            "cannot be started",
            Err(VeraPdfError::Launch(std::io::Error::other("no"))),
        ),
        ("runs too long", Err(VeraPdfError::TimedOut(3))),
    ];
    for (name, answer) in cases {
        let fake = Fake::new(vec![answer]);
        let result = validate_with(&tool, pdf(), &fake, TIMEOUT);
        let error = result.expect_err(name);
        assert!(!error.to_string().is_empty(), "{name}");
    }
}

#[test]
fn the_error_for_a_pdf_verapdf_cannot_read_says_why() {
    let error = parse(UA1_NOT_A_PDF, Profile::Ua1).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("could not read the PDF"), "{text}");
    assert!(text.contains("End of file is reached"), "{text}");

    let dir = TempDir::new("unreadable");
    let tool = fake_vendor(&dir, false);
    // A non-zero exit code does not hide the reason.
    let fake = Fake::new(vec![Ok(output(UA1_NOT_A_PDF, 7))]);
    let error = validate_with(&tool, pdf(), &fake, TIMEOUT).unwrap_err();
    assert!(
        error.to_string().contains("could not read the PDF"),
        "{error}"
    );

    let crash = Fake::new(vec![Ok(RunOutput {
        exit_code: Some(3),
        stdout: String::new(),
        stderr: "\nboom\nmore\n".to_string(),
    })]);
    let error = validate_with(&tool, pdf(), &crash, TIMEOUT).unwrap_err();
    assert_eq!(
        error.to_string(),
        "veraPDF stopped with an error (exit code 3): boom"
    );
}

// S5-08
#[test]
fn verapdf_any_failure_means_not_pass() {
    let dir = TempDir::new("pass");
    let tool = fake_vendor(&dir, false);

    let pass = validate_with(&tool, pdf(), &Fake::json(&[UA1_PASS]), TIMEOUT).unwrap();
    assert!(pass.passed());
    assert_eq!(pass.ua1.failed_checks, 0);
    assert!(pass.ua1.findings.is_empty());
    assert_eq!(pass.ua1.passed_rules, 106);

    // One failed check, in one rule.
    let fail = validate_with(
        &tool,
        pdf(),
        &Fake::json(&[UA1_FIGURE_WITHOUT_ALT]),
        TIMEOUT,
    )
    .unwrap();
    assert!(!fail.passed());
    assert_eq!(fail.ua1.failed_checks, 1);
    assert_eq!(fail.ua1.findings.len(), 1);

    // The same PDF with alternative text passes.
    let with_alt =
        validate_with(&tool, pdf(), &Fake::json(&[UA1_FIGURE_WITH_ALT]), TIMEOUT).unwrap();
    assert!(with_alt.passed());
}

#[test]
fn a_report_that_says_not_compliant_is_never_a_pass() {
    // No rule list, no counts, but "compliant": false.
    let text = json!({"report": {"jobs": [{"validationResult": [
        {"details": {}, "compliant": false, "profileName": "PDF/UA-1 validation profile"}
    ]}]}})
    .to_string();
    let result = parse(&text, Profile::Ua1).unwrap();
    assert!(!result.passed());

    // Failed checks without a rule list are not a pass either.
    let text = json!({"report": {"jobs": [{"validationResult": [
        {"details": {"failedChecks": 2, "failedRules": 1}, "compliant": false}
    ]}]}})
    .to_string();
    assert!(!parse(&text, Profile::Ua1).unwrap().passed());

    // A job that did not end normally gives an error, not a pass.
    let text = json!({"report": {"jobs": [{"validationResult": [
        {"details": {}, "compliant": true, "jobEndStatus": "timeout"}
    ]}]}})
    .to_string();
    assert!(parse(&text, Profile::Ua1).is_err());
}

#[test]
fn the_result_serialises_for_the_report() {
    let dir = TempDir::new("serialise");
    let tool = fake_vendor(&dir, false);
    let validation = validate_with(
        &tool,
        pdf(),
        &Fake::json(&[UA1_FIGURE_WITHOUT_ALT]),
        TIMEOUT,
    )
    .unwrap();
    let json = serde_json::to_value(&validation).unwrap();
    assert_eq!(json["wcag"]["status"], "not-available");
    assert_eq!(json["ua1"]["profile"], "ua1");
    let finding = &json["ua1"]["findings"][0];
    assert_eq!(finding["pre_check"], "IMG-001");
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["clause"], "7.3");
    assert_eq!(finding["test_number"], 1);
}

// The process runner, on a harmless program that every Windows has.
#[cfg(windows)]
mod process {
    use super::*;

    fn cmd(script: &str, timeout: Duration) -> Invocation {
        Invocation {
            program: "cmd".into(),
            args: vec!["/C".into(), script.into()],
            timeout,
        }
    }

    #[test]
    fn the_process_runner_captures_output_and_the_exit_code() {
        let out = ProcessRunner
            .run(&cmd(
                "echo out& echo err 1>&2& exit 3",
                Duration::from_secs(20),
            ))
            .unwrap();
        assert_eq!(out.exit_code, Some(3));
        assert_eq!(out.stdout.trim(), "out");
        assert_eq!(out.stderr.trim(), "err");
    }

    #[test]
    fn verapdf_timeout_stops_the_process() {
        let started = Instant::now();
        // About 10 seconds of work, and a limit of half a second.
        let error = ProcessRunner
            .run(&cmd(
                "ping -n 11 127.0.0.1 >nul",
                Duration::from_millis(500),
            ))
            .unwrap_err();
        assert!(matches!(error, VeraPdfError::TimedOut(0)), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_program_that_does_not_exist_is_a_launch_error() {
        let invocation = Invocation {
            program: "C:/no/such/program.exe".into(),
            args: vec![],
            timeout: Duration::from_secs(5),
        };
        assert!(matches!(
            ProcessRunner.run(&invocation),
            Err(VeraPdfError::Launch(_))
        ));
    }
}
