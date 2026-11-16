//! Stage 5 of the pipeline (spec 02): run veraPDF on the PDF.
//!
//! The tool runs the bundled veraPDF with the bundled Java runtime, both from
//! the folder `vendor/` (see `scripts/fetch-vendor.ps1`). It never uses a Java
//! or a veraPDF that is installed on the machine.
//!
//! The command is what `verapdf.bat` runs, with the Java of the bundle:
//!
//! ```text
//! <vendor>/jre/bin/java.exe -classpath "<vendor>/verapdf/bin/*" ...
//!     org.verapdf.apps.GreenfieldCliWrapper --flavour ua1 --format json <pdf>
//! ```
//!
//! veraPDF's command line has no network access: the check for new versions
//! is in the GUI jar, which the command line does not call, and the built-in
//! PDF/UA-1 profile is part of the jar. So there is no offline switch to set.
//!
//! If the bundle has a WCAG 2.2 machine-check profile
//! (`profiles/*/PDF_UA/WCAG-2-2-Machine.xml`), it is run as well, and its
//! results are warnings. They do not change pass or fail.

mod report;
pub mod rules;

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

pub use report::{Profile, ProfileResult, VeraFinding, parse};

/// Stage 5 could not run, or it did not give a result. These are tool
/// failures (exit code 2). A failed PDF/UA check is not an error; it is in the
/// [`Validation`].
#[derive(Debug, thiserror::Error)]
pub enum VeraPdfError {
    #[error(
        "the Java runtime of the tool is missing: {} was not found. \
         Run scripts/fetch-vendor.ps1, or install the tool again.",
        .0.display()
    )]
    JavaMissing(PathBuf),

    #[error(
        "veraPDF is missing: {} was not found. \
         Run scripts/fetch-vendor.ps1, or install the tool again.",
        .0.display()
    )]
    VeraPdfMissing(PathBuf),

    #[error("veraPDF could not be started: {0}")]
    Launch(io::Error),

    #[error("veraPDF did not finish within {0} seconds and was stopped")]
    TimedOut(u64),

    #[error("veraPDF stopped with an error (exit code {code}){detail}", code = fmt_code(.exit_code), detail = fmt_detail(.stderr))]
    Crashed {
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("veraPDF gave no usable result: {0}")]
    BadOutput(String),
}

fn fmt_code(code: &Option<i32>) -> String {
    code.map_or_else(|| "none".to_string(), |c| c.to_string())
}

fn fmt_detail(stderr: &str) -> String {
    let line = stderr.lines().find(|l| !l.trim().is_empty());
    line.map_or_else(String::new, |l| format!(": {}", l.trim()))
}

/// Where veraPDF and its Java runtime are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The `java` program of the bundled runtime.
    pub java: PathBuf,
    /// The veraPDF folder: it has `bin/` with the jar files.
    pub verapdf: PathBuf,
}

const JAVA_PROGRAM: &str = if cfg!(windows) { "java.exe" } else { "java" };

impl Tool {
    pub fn new(java: impl Into<PathBuf>, verapdf: impl Into<PathBuf>) -> Tool {
        Tool {
            java: java.into(),
            verapdf: verapdf.into(),
        }
    }

    /// The tools in a `vendor` folder: `jre/bin/java.exe` and `verapdf/`.
    pub fn in_vendor(vendor: &Path) -> Tool {
        Tool::new(
            vendor.join("jre").join("bin").join(JAVA_PROGRAM),
            vendor.join("verapdf"),
        )
    }

    /// Looks for `vendor/` next to the program, and then at the root of the
    /// source tree (for development). When neither has the tools, the result is
    /// the folder next to the program, so that [`Tool::check`] names it.
    ///
    /// The environment variable `ADAPDF_VENDOR` names another `vendor` folder.
    /// When it is set, it is the only place that is used, so that a wrong path
    /// is an error that names it and not a silent use of another copy.
    pub fn locate() -> Tool {
        if let Some(folder) = std::env::var_os("ADAPDF_VENDOR").filter(|v| !v.is_empty()) {
            return Tool::in_vendor(Path::new(&folder));
        }
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
        {
            candidates.push(dir.join("vendor"));
        }
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor"));
        let tools: Vec<Tool> = candidates.iter().map(|v| Tool::in_vendor(v)).collect();
        tools
            .iter()
            .find(|t| t.check().is_ok())
            .or(tools.first())
            .cloned()
            .unwrap_or_else(|| Tool::in_vendor(Path::new("vendor")))
    }

    /// Checks that Java and veraPDF are there.
    pub fn check(&self) -> Result<(), VeraPdfError> {
        if !self.java.is_file() {
            return Err(VeraPdfError::JavaMissing(self.java.clone()));
        }
        if !self.verapdf.join("bin").is_dir() {
            return Err(VeraPdfError::VeraPdfMissing(self.verapdf.clone()));
        }
        Ok(())
    }

    /// The WCAG 2.2 machine-check profile of this veraPDF, if it has one.
    pub fn wcag_profile(&self) -> Option<PathBuf> {
        let profiles = std::fs::read_dir(self.verapdf.join("profiles")).ok()?;
        profiles
            .flatten()
            .map(|entry| entry.path().join("PDF_UA").join("WCAG-2-2-Machine.xml"))
            .find(|path| path.is_file())
    }

    /// The command that runs veraPDF with these arguments.
    pub fn invocation(&self, verapdf_args: &[OsString], timeout: Duration) -> Invocation {
        let mut args: Vec<OsString> = vec![
            "-classpath".into(),
            self.verapdf.join("bin").join("*").into_os_string(),
            // veraPDF writes UTF-8. Java 19 and later use the Windows code page
            // for a pipe unless it is told otherwise.
            "-Dfile.encoding=UTF8".into(),
            "-Dstdout.encoding=UTF-8".into(),
            "-Dstderr.encoding=UTF-8".into(),
            OsString::from(format!("-Dapp.home={}", self.verapdf.display())),
            OsString::from(format!("-Dbasedir={}", self.verapdf.display())),
            "org.verapdf.apps.GreenfieldCliWrapper".into(),
        ];
        args.extend_from_slice(verapdf_args);
        Invocation {
            program: self.java.clone(),
            args,
            timeout,
        }
    }
}

/// A program to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub timeout: Duration,
}

impl Invocation {
    /// The arguments that veraPDF itself gets (those after the main class).
    pub fn verapdf_args(&self) -> &[OsString] {
        let main = self
            .args
            .iter()
            .position(|a| a == "org.verapdf.apps.GreenfieldCliWrapper");
        main.map_or(&self.args[..], |i| &self.args[i + 1..])
    }
}

/// What a program wrote and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Runs programs. A test can give a fake one.
pub trait Runner {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput, VeraPdfError>;
}

/// Starts the program as a process. It is stopped if it runs longer than the
/// timeout.
pub struct ProcessRunner;

impl Runner for ProcessRunner {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput, VeraPdfError> {
        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // No console window, when the tool runs from the app.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command.spawn().map_err(VeraPdfError::Launch)?;
        let stdout = read_in_thread(child.stdout.take());
        let stderr = read_in_thread(child.stderr.take());

        let started = Instant::now();
        let status = loop {
            match child.try_wait().map_err(VeraPdfError::Launch)? {
                Some(status) => break status,
                None if started.elapsed() >= invocation.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(VeraPdfError::TimedOut(invocation.timeout.as_secs()));
                }
                None => thread::sleep(Duration::from_millis(20)),
            }
        };
        Ok(RunOutput {
            exit_code: status.code(),
            stdout: String::from_utf8_lossy(&stdout.join().unwrap_or_default()).into_owned(),
            stderr: String::from_utf8_lossy(&stderr.join().unwrap_or_default()).into_owned(),
        })
    }
}

fn read_in_thread<R: Read + Send + 'static>(pipe: Option<R>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// How long a run may take before it is stopped.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(180);

/// What the WCAG machine-check profile gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "result", rename_all = "kebab-case")]
pub enum WcagOutcome {
    /// This veraPDF has no WCAG machine-check profile, so it was not run.
    NotAvailable,
    /// It ran. Its findings are warnings.
    Ran(ProfileResult),
    /// It was started and did not give a result. This does not change pass or
    /// fail.
    Failed(String),
}

/// The result of stage 5 for one PDF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Validation {
    /// The PDF/UA-1 profile. Any failure here means the PDF does not pass.
    pub ua1: ProfileResult,
    /// The WCAG profile, as warnings.
    pub wcag: WcagOutcome,
}

impl Validation {
    /// The version of veraPDF that gave the result, if its report says it.
    pub fn verapdf_version(&self) -> Option<&str> {
        self.ua1.verapdf_version.as_deref()
    }

    /// `true` when the PDF passes: no PDF/UA-1 rule or check failed. WCAG
    /// warnings do not change it.
    pub fn passed(&self) -> bool {
        self.ua1.passed()
    }

    /// The failed PDF/UA-1 rules, then the WCAG warnings that are not a repeat of
    /// a PDF/UA-1 failure.
    pub fn findings(&self) -> Vec<&VeraFinding> {
        let mut all: Vec<&VeraFinding> = self.ua1.findings.iter().collect();
        if let WcagOutcome::Ran(wcag) = &self.wcag {
            all.extend(wcag.findings.iter());
        }
        all
    }
}

/// Runs veraPDF on a PDF with the real process.
pub fn validate(tool: &Tool, pdf: &Path) -> Result<Validation, VeraPdfError> {
    validate_with(tool, pdf, &ProcessRunner, DEFAULT_TIMEOUT)
}

/// Runs veraPDF on a PDF with a runner and a timeout.
pub fn validate_with(
    tool: &Tool,
    pdf: &Path,
    runner: &dyn Runner,
    timeout: Duration,
) -> Result<Validation, VeraPdfError> {
    tool.check()?;

    let ua1_args: Vec<OsString> = vec![
        "--flavour".into(),
        "ua1".into(),
        "--format".into(),
        "json".into(),
        pdf.as_os_str().to_owned(),
    ];
    let ua1 = run_profile(tool, runner, &ua1_args, timeout, Profile::Ua1)?;

    let wcag = match tool.wcag_profile() {
        None => WcagOutcome::NotAvailable,
        Some(profile_file) => {
            let args: Vec<OsString> = vec![
                "--profile".into(),
                profile_file.into_os_string(),
                "--format".into(),
                "json".into(),
                pdf.as_os_str().to_owned(),
            ];
            match run_profile(tool, runner, &args, timeout, Profile::Wcag) {
                Ok(mut result) => {
                    // The WCAG profile repeats some rules of PDF/UA-1. The user sees
                    // those as errors already.
                    result.findings.retain(|w| {
                        !ua1.findings.iter().any(|u| {
                            u.specification == w.specification
                                && u.clause == w.clause
                                && u.test_number == w.test_number
                        })
                    });
                    WcagOutcome::Ran(result)
                }
                Err(error) => WcagOutcome::Failed(error.to_string()),
            }
        }
    };
    Ok(Validation { ua1, wcag })
}

fn run_profile(
    tool: &Tool,
    runner: &dyn Runner,
    args: &[OsString],
    timeout: Duration,
    profile: Profile,
) -> Result<ProfileResult, VeraPdfError> {
    let output = runner.run(&tool.invocation(args, timeout))?;
    // veraPDF's exit code is 0 when the PDF passes, 1 when it does not, and
    // another number when it cannot read the PDF. The JSON says which, so the
    // code only matters when there is no JSON.
    if output.stdout.trim().is_empty() && output.exit_code != Some(0) {
        return Err(VeraPdfError::Crashed {
            exit_code: output.exit_code,
            stderr: output.stderr,
        });
    }
    parse(&output.stdout, profile)
}
