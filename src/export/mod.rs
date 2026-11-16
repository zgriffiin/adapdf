//! Stage 3 of the pipeline (spec 02): export a .docx to a tagged PDF with Word.
//!
//! [`export_pdf`] does this, in this order:
//!
//! 1. Stage 1. Any error finding refuses the export ([`ExportError::Blocked`]);
//!    that includes DOC-005 (tracked changes or comments). Word is not started
//!    and no PDF is written. The error holds the findings for the report.
//! 2. The output is checked: an existing PDF must not be locked by another
//!    program, and the folder must accept a new file.
//! 3. The tool's own hidden Word opens the document **read-only**, updates the
//!    fields (the table of contents, cross-references), calls
//!    `ExportAsFixedFormat` with the values of spec 02 and closes the document
//!    without saving. The .docx is never written.
//! 4. The PDF is moved into place. Word writes it to a temporary file in the
//!    target folder, so a failed export never leaves a half PDF and never
//!    deletes the old one. An existing `Guide.pdf` is replaced.
//!
//! # Threads and the timeout
//!
//! Word is driven through COM in a single-threaded apartment, and a
//! [`WordSession`](crate::word::WordSession) is not `Send`. So the whole Word
//! part (start, open, update, export, close, quit) runs on one worker thread
//! that makes its own session. The thread that called [`export_pdf`] only waits
//! on a channel with a deadline (`ExportOptions::timeout`, 5 minutes by
//! default). The worker sends two messages: the id of the Word process, as soon
//! as the session knows it, and at the end the result (after the session is
//! dropped, so Word is already quit).
//!
//! When the deadline passes, the caller ends **only** that process: the id in
//! the message is [`WordSession::pid`](crate::word::WordSession::pid), a
//! process that did not exist before the session started and that owns the
//! session's own window (see the doc comment of `WordSession`). The ending code
//! opens the process once, reads its image name from the open handle and ends it
//! only if it is `WINWORD.EXE`, so an id that was reused by another program is
//! not touched ([`end_word_process`](crate::word::end_word_process)). If the
//! deadline passes before Word has started, the caller waits for the id (for at
//! most [`START_GRACE`]) and then ends it. If the id never comes, nothing is
//! ended and the error says so. After the end, the worker's calls fail by
//! themselves and it finishes; the caller waits for it for at most [`END_GRACE`]
//! and removes the temporary PDF.

mod engine;
#[cfg(test)]
mod tests;

use std::fmt;
use std::fs;
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::findings::{Finding, Severity};
use crate::fonts::FontLookup;
use crate::model::Document;
use crate::rules;
use crate::word::WordError;

use engine::{EndProcess, EndWordProcess, StartWord, start_real_word};

pub use crate::word::{ExportParameters, FieldUpdate};

/// The time Word has for one document (spec 02: 5 minutes).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
/// After the deadline, how long the caller waits for the id of a Word that was
/// still starting.
const START_GRACE: Duration = Duration::from_secs(60);
/// After the end of Word, how long the caller waits for the worker thread.
const END_GRACE: Duration = Duration::from_secs(15);

/// A function that runs on the Word thread with the id of the tool's Word.
type Hook = Arc<dyn Fn(u32) + Send + Sync>;

/// What the caller can choose.
#[derive(Clone)]
pub struct ExportOptions {
    /// How long Word has for the document. The default is [`DEFAULT_TIMEOUT`].
    pub timeout: Duration,
    /// For tests. It runs on the Word thread when the document is open, before
    /// the fields are updated, with the id of the tool's Word process. A hook
    /// that does not return makes the export hang, which is how a test makes a
    /// timeout happen at a known place.
    pub after_open: Option<Hook>,
}

impl Default for ExportOptions {
    fn default() -> ExportOptions {
        ExportOptions {
            timeout: DEFAULT_TIMEOUT,
            after_open: None,
        }
    }
}

impl fmt::Debug for ExportOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportOptions")
            .field("timeout", &self.timeout)
            .field("after_open", &self.after_open.is_some())
            .finish()
    }
}

/// What the export did, for the report.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportResult {
    /// The PDF that was written.
    pub pdf: PathBuf,
    /// The findings of stage 1. There is no error among them (an error refuses
    /// the export), so these are the warnings.
    pub findings: Vec<Finding>,
    /// The fields that Word updated before the export.
    pub fields: FieldUpdate,
    /// The values that were passed to `ExportAsFixedFormat`.
    pub parameters: ExportParameters,
    /// Word's build, for example `16.0.20527.20000`.
    pub word_build: String,
    /// The time from the start of Word to the PDF in place.
    pub duration: Duration,
}

/// Why the export did not give a PDF. The messages are for people.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("The document cannot be checked before the export: {0}")]
    Check(#[from] crate::error::Error),

    /// Stage 1 found errors. Nothing was exported. `findings` has all findings
    /// of stage 1, for the report.
    #[error("{}", blocked_message(.findings))]
    Blocked { findings: Vec<Finding> },

    #[error(
        "The tool cannot write the PDF in the folder \"{}\" ({source}). Check that the folder exists and that you may save files in it.",
        folder.display()
    )]
    OutputFolder { folder: PathBuf, source: io::Error },

    #[error(
        "The PDF \"{}\" cannot be replaced ({source}). Another program may have it open, or it may be read-only. Close it or change it, then run the export again. The old PDF was not changed.",
        file.display()
    )]
    PdfLocked { file: PathBuf, source: io::Error },

    #[error("The PDF \"{}\" cannot be written ({source}).", file.display())]
    WritePdf { file: PathBuf, source: io::Error },

    #[error("Word finished, but it did not write a PDF. No PDF was written.")]
    NoPdf,

    #[error("{0}")]
    Word(#[from] WordError),

    #[error("{}", timeout_message(*.limit, *.pid, *.ended))]
    Timeout {
        limit: Duration,
        /// The Word process that the tool started, if it was known.
        pid: Option<u32>,
        /// Whether that process is gone.
        ended: bool,
    },

    #[error("The part of the tool that drives Word stopped without a result. No PDF was written.")]
    WorkerStopped,
}

fn blocked_message(findings: &[Finding]) -> String {
    let errors: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    let mut text = format!(
        "The export was not started because the document has {} error{} that must be fixed first. No PDF was written.",
        errors.len(),
        if errors.len() == 1 { "" } else { "s" }
    );
    for finding in errors {
        text.push_str(&format!("\n- {}: {}", finding.rule_id, finding.message));
    }
    text
}

fn timeout_message(limit: Duration, pid: Option<u32>, ended: bool) -> String {
    let minutes = limit.as_secs() / 60;
    let time = if minutes >= 1 && limit == Duration::from_secs(minutes * 60) {
        format!("{minutes} minute{}", if minutes == 1 { "" } else { "s" })
    } else {
        format!("{:.1} seconds", limit.as_secs_f64())
    };
    let word = match (pid, ended) {
        (Some(_), true) => "The tool ended the copy of Word that it had started.".to_string(),
        (Some(pid), false) => format!(
            "The tool could not end the copy of Word that it had started (process {pid}). End it in Task Manager."
        ),
        (None, _) => "The tool could not tell which copy of Word it had started, so it ended none."
            .to_string(),
    };
    format!("Word did not finish the export within {time}. {word} No PDF was written.")
}

/// Exports `docx` to a tagged PDF at `pdf` with the tool's own hidden Word. See
/// the module documentation for the steps, the threads and the timeout. `fonts`
/// is for the font rule of stage 1.
pub fn export_pdf(
    docx: &Path,
    pdf: &Path,
    fonts: &dyn FontLookup,
    options: &ExportOptions,
) -> Result<ExportResult, ExportError> {
    run(
        docx,
        pdf,
        fonts,
        options,
        start_real_word(),
        &EndWordProcess,
    )
}

/// [`export_pdf`] with Word and the ending of a process given, for tests.
fn run(
    docx: &Path,
    pdf: &Path,
    fonts: &dyn FontLookup,
    options: &ExportOptions,
    start: StartWord,
    ender: &dyn EndProcess,
) -> Result<ExportResult, ExportError> {
    let document = Document::open(docx)?;
    let findings = rules::check(&document, fonts);
    if findings.iter().any(|f| f.severity == Severity::Error) {
        return Err(ExportError::Blocked { findings });
    }

    ensure_not_locked(pdf)?;
    let temp = TempPdf::create(pdf)?;

    let started = Instant::now();
    let work = run_word(docx, &temp.path, options, start, ender)?;
    temp.check_is_pdf()?;
    temp.move_over(pdf)?;

    Ok(ExportResult {
        pdf: pdf.to_path_buf(),
        findings,
        fields: work.fields,
        parameters: ExportParameters::SPEC,
        word_build: work.build,
        duration: started.elapsed(),
    })
}

/// What the Word thread found out.
struct WordWork {
    build: String,
    fields: FieldUpdate,
}

enum Message {
    /// The id of the Word process that the thread started.
    Started(u32),
    /// The end. Word is already quit.
    Finished(Result<WordWork, WordError>),
}

/// Runs the Word part on its own thread and waits for it, with a deadline.
fn run_word(
    docx: &Path,
    temp: &Path,
    options: &ExportOptions,
    start: StartWord,
    ender: &dyn EndProcess,
) -> Result<WordWork, ExportError> {
    let (sender, receiver) = mpsc::channel();
    let docx = docx.to_path_buf();
    let temp = temp.to_path_buf();
    let hook = options.after_open.clone();
    let worker = thread::Builder::new()
        .name("adapdf-word".to_string())
        .spawn(move || {
            // The app is made in here, so it never leaves this thread. It is
            // dropped (and Word quits) when the closure ends, before `Finished`.
            let outcome = (|| -> Result<WordWork, WordError> {
                let app = start()?;
                let _ = sender.send(Message::Started(app.pid()));
                let build = app.build()?;
                let document = app.open_read_only(&docx)?;
                if let Some(hook) = &hook {
                    hook(app.pid());
                }
                let fields = document.update_fields()?;
                document.export(&temp, &ExportParameters::SPEC)?;
                document.close()?;
                Ok(WordWork { build, fields })
            })();
            let _ = sender.send(Message::Finished(outcome));
        })
        .map_err(|e| ExportError::Word(WordError::UnknownProcess(e.to_string())))?;

    let outcome = wait_for_word(&receiver, options.timeout, ender);
    if !matches!(outcome, Err(ExportError::Timeout { .. })) {
        // The thread ends right after its last message.
        let _ = worker.join();
    }
    outcome
}

/// Waits for the worker's result. On the deadline, ends the Word that the
/// worker started (see the module documentation).
fn wait_for_word(
    receiver: &Receiver<Message>,
    limit: Duration,
    ender: &dyn EndProcess,
) -> Result<WordWork, ExportError> {
    let deadline = Instant::now() + limit;
    let mut pid = None;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(left) {
            Ok(Message::Started(id)) => pid = Some(id),
            Ok(Message::Finished(result)) => return Ok(result?),
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => return Err(ExportError::WorkerStopped),
        }
    }

    // The deadline has passed. Word may still be starting: wait for its id.
    while pid.is_none() {
        match receiver.recv_timeout(START_GRACE) {
            Ok(Message::Started(id)) => pid = Some(id),
            Ok(Message::Finished(result)) => return Ok(result?),
            Err(_) => break,
        }
    }
    // Look once more, so that a result that has just come is not lost.
    while let Ok(message) = receiver.try_recv() {
        if let Message::Finished(result) = message {
            return Ok(result?);
        }
    }

    let ended = pid.is_some_and(|pid| ender.end(pid).is_ok());
    // Word is gone, so the worker's calls fail and it finishes.
    let grace = Instant::now() + END_GRACE;
    while let Ok(message) = receiver.recv_timeout(grace.saturating_duration_since(Instant::now())) {
        if matches!(message, Message::Finished(_)) {
            break;
        }
    }
    Err(ExportError::Timeout { limit, pid, ended })
}

/// An existing PDF must be free to replace: nobody else has it open.
fn ensure_not_locked(pdf: &Path) -> Result<(), ExportError> {
    if !pdf.exists() {
        return Ok(());
    }
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(pdf)
        .map(drop)
        .map_err(|source| ExportError::PdfLocked {
            file: pdf.to_path_buf(),
            source,
        })
}

/// The temporary file that Word writes to, in the folder of the PDF. It is
/// removed when the value is dropped, unless it was moved into place.
struct TempPdf {
    path: PathBuf,
}

static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

impl TempPdf {
    /// Makes an empty file, which also shows that the folder takes new files.
    fn create(pdf: &Path) -> Result<TempPdf, ExportError> {
        let folder = match pdf.parent() {
            Some(folder) if !folder.as_os_str().is_empty() => folder,
            _ => Path::new("."),
        };
        let stem = pdf
            .file_stem()
            .map_or_else(|| "export".into(), |s| s.to_string_lossy());
        loop {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = folder.join(format!("{stem}.adapdf-{}-{n}.pdf", std::process::id()));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(TempPdf { path }),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(ExportError::OutputFolder {
                        folder: folder.to_path_buf(),
                        source,
                    });
                }
            }
        }
    }

    /// Word wrote a PDF to the file.
    fn check_is_pdf(&self) -> Result<(), ExportError> {
        let mut head = [0_u8; 5];
        let is_pdf = fs::File::open(&self.path)
            .and_then(|mut file| io::Read::read_exact(&mut file, &mut head))
            .is_ok()
            && &head == b"%PDF-";
        if is_pdf {
            Ok(())
        } else {
            Err(ExportError::NoPdf)
        }
    }

    /// Replaces `pdf` with the file. If this fails, `pdf` is as it was.
    fn move_over(&self, pdf: &Path) -> Result<(), ExportError> {
        fs::rename(&self.path, pdf).map_err(|source| {
            let file = pdf.to_path_buf();
            if pdf.exists() {
                ExportError::PdfLocked { file, source }
            } else {
                ExportError::WritePdf { file, source }
            }
        })
    }
}

impl Drop for TempPdf {
    /// Word's process can need a moment to let go of the file after it ended.
    fn drop(&mut self) {
        for _ in 0..20 {
            if !self.path.exists() || fs::remove_file(&self.path).is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}
