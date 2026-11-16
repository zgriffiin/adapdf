//! Driving Microsoft Word through COM (stage 3 of the pipeline, spec 02).
//!
//! This is the base that the export builds on. It has two safe types:
//!
//! - [`WordSession`] starts the tool's own hidden Word and always quits it.
//! - [`WordDocument`] is a document that the session opened (read-only, or for
//!   editing in tests), and closes it without saving.
//!
//! Rules from AGENTS.md and spec 02 that this module keeps:
//!
//! - The tool starts its own Word. It never attaches to a running one (there is
//!   no `GetActiveObject`), and it never touches a window or a document that
//!   the user has open.
//! - Word is quit after success, after an error and after a panic (`Drop`).
//! - A file that Word cannot open, or would only open after a repair, is an
//!   error. No dialog is shown.
//!
//! Windows only. All `unsafe` code is in `com.rs`, in small safe functions.

mod com;

use std::marker::PhantomData;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

use windows::core::BSTR;

use crate::model::theme::Rgb;
use com::{ComApartment, Dispatch};

pub use com::ComError;

/// The program ID of Word. COM turns it into the class that starts Word.
const WORD_PROG_ID: &str = "Word.Application";
/// The image name of Word's process.
const WORD_EXE: &str = "WINWORD.EXE";
/// `wdDoNotSaveChanges`.
const WD_DO_NOT_SAVE_CHANGES: i32 = 0;
/// `wdAlertsNone`.
const WD_ALERTS_NONE: i32 = 0;
/// How long [`WordSession`] waits for Word's process to end after `Quit`.
const EXIT_WAIT: Duration = Duration::from_secs(10);
/// How often the process list is read while waiting.
const EXIT_POLL: Duration = Duration::from_millis(50);
/// Word asks for a password in a dialog when a document needs one and none is
/// given. Any text makes Word report a wrong password as an error instead.
const NO_PASSWORD: &str = "adapdf-no-password";

/// What can go wrong when Word is used. The messages are for people.
#[derive(Debug, thiserror::Error)]
pub enum WordError {
    #[error("Microsoft Word is not installed on this computer, so the tool cannot start it.")]
    NotInstalled,

    #[error("The tool could not start Microsoft Word: {0}")]
    Start(ComError),

    #[error(
        "The tool asked for a new copy of Word, but Windows gave it a Word that was already open. The tool left that Word alone and stopped."
    )]
    SharedInstance,

    #[error("The tool could not tell which Word process it started, so it stopped: {0}")]
    UnknownProcess(String),

    #[error("The file \"{}\" does not exist.", .0.display())]
    FileNotFound(PathBuf),

    #[error(
        "Word could not open \"{}\": {reason}. The file may be damaged, protected with a password, or not a Word document.",
        .path.display()
    )]
    Open { path: PathBuf, reason: String },

    #[error("Word reported an error: {0}")]
    Word(#[from] ComError),
}

pub type Result<T> = std::result::Result<T, WordError>;

/// The tool's own hidden Word.
///
/// Starting it makes a new Word process. Dropping it quits that process (with
/// "do not save") and waits a short time for it to end. A session must be used
/// on the thread that made it (it is not `Send`).
///
/// # The process id
///
/// [`pid`](WordSession::pid) is the process that this session started. Word's
/// `Application` object has no process id, so it is found like this:
///
/// 1. Before Word is started, the ids of all running `WINWORD.EXE` processes
///    are read and kept.
/// 2. COM starts Word (`CoCreateInstance` with `CLSCTX_LOCAL_SERVER`, never
///    `GetActiveObject`). The ids are read again. If no new process is there,
///    COM gave the session a Word that was already running (the user's). The
///    session then does nothing to it except release its reference, and
///    [`start`](WordSession::start) fails with [`WordError::SharedInstance`].
/// 3. Otherwise the session adds one hidden, empty document to its new Word,
///    asks that document's window for its handle, and asks Windows which
///    process owns the handle. It closes the document again. That process is
///    the pid, and it must be one of the new processes from step 2, or
///    `start` fails with [`WordError::SharedInstance`]. (A new process alone
///    would not prove which Word the object lives in: two tools that start
///    Word at the same moment would each see the other's process.)
///
/// So the pid belongs to the Word that this session's own object lives in, and
/// that process did not exist before the session started. Visibility, alerts
/// and `Quit` are only used after this check. Only that process is ever quit
/// or (by the export, later) ended.
pub struct WordSession {
    /// `None` only while `drop` has taken it to release it.
    app: Option<Dispatch>,
    pid: u32,
    /// Last, so that COM stays started until the Word object is released.
    _com: ComApartment,
}

impl WordSession {
    /// Starts a new hidden Word: not visible, no alerts.
    pub fn start() -> Result<WordSession> {
        let com = ComApartment::enter()?;
        let before = com::process_ids(WORD_EXE)?;
        let app = Dispatch::create(WORD_PROG_ID).map_err(|e| {
            if e.is_class_not_registered() {
                WordError::NotInstalled
            } else {
                WordError::Start(e)
            }
        })?;
        let pid = match own_process_id(&app, &before) {
            Ok(pid) => pid,
            // Not a new Word: let it go without touching it.
            Err(error @ WordError::SharedInstance) => return Err(error),
            // A new process did start and could not be asked more. It is
            // almost certainly this object's, so end it rather than leak it.
            Err(error) => {
                quit(&app);
                return Err(error);
            }
        };
        let session = WordSession {
            app: Some(app),
            pid,
            _com: com,
        };
        // If this fails the session is dropped, and that quits Word.
        session.app().put("Visible", false)?;
        session.app().put("DisplayAlerts", WD_ALERTS_NONE)?;
        Ok(session)
    }

    /// The id of the Word process that this session started.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Word's build, for example `16.0.19127.20154`.
    pub fn build(&self) -> Result<String> {
        Ok(self.app().get_string("Build")?)
    }

    /// Opens a document read-only and hidden. It is not added to the recent
    /// files, no file conversion or repair is offered, and no dialog can wait
    /// for an answer. A file that Word cannot open is an [`WordError::Open`].
    pub fn open_read_only(&self, path: &Path) -> Result<WordDocument<'_>> {
        self.open(path, true)
    }

    /// Opens a document for editing, hidden, in the same way as
    /// [`open_read_only`](WordSession::open_read_only). Word then writes its
    /// owner file (`~$...`) next to the document and holds the file. The tool
    /// never saves: the document is closed with "do not save". This exists for
    /// tests that need a document that Word has open the way a person has it.
    pub fn open_for_editing(&self, path: &Path) -> Result<WordDocument<'_>> {
        self.open(path, false)
    }

    fn open(&self, path: &Path, read_only: bool) -> Result<WordDocument<'_>> {
        if !path.is_file() {
            return Err(WordError::FileNotFound(path.to_path_buf()));
        }
        let full_path =
            std::path::absolute(path).map_err(|_| WordError::FileNotFound(path.to_path_buf()))?;
        let file_name = BSTR::from_wide(&full_path.as_os_str().encode_wide().collect::<Vec<u16>>());
        let args = vec![
            ("FileName", file_name.into()),
            ("ConfirmConversions", false.into()),
            ("ReadOnly", read_only.into()),
            ("AddToRecentFiles", false.into()),
            ("PasswordDocument", NO_PASSWORD.into()),
            ("WritePasswordDocument", NO_PASSWORD.into()),
            ("Visible", false.into()),
            ("OpenAndRepair", false.into()),
            ("NoEncodingDialog", true.into()),
        ];
        let document = self
            .app()
            .get_object("Documents")
            .and_then(|documents| documents.call_object("Open", args))
            .map_err(|e| WordError::Open {
                path: path.to_path_buf(),
                reason: e.message.trim_end_matches('.').to_string(),
            })?;
        Ok(WordDocument {
            document: Some(document),
            session: PhantomData,
        })
    }

    fn app(&self) -> &Dispatch {
        self.app
            .as_ref()
            .expect("the Word object is there until drop")
    }
}

/// The process that the new Word object lives in. Fails with
/// [`WordError::SharedInstance`] when it is not a process that started after
/// `running_before` was read. See "The process id" on [`WordSession`].
fn own_process_id(app: &Dispatch, running_before: &[u32]) -> Result<u32> {
    let started: Vec<u32> = com::process_ids(WORD_EXE)?
        .into_iter()
        .filter(|pid| !running_before.contains(pid))
        .collect();
    if started.is_empty() {
        return Err(WordError::SharedInstance);
    }
    let probe = app
        .get_object("Documents")?
        .call_object("Add", vec![("Visible", false.into())])?;
    let window = probe.get_object("ActiveWindow")?.get_i32("Hwnd");
    // The probe is closed whatever the answer; the answer is the error to report.
    let closed = close_without_saving(&probe);
    let window = window?;
    closed?;
    let pid = com::window_process_id(window).ok_or_else(|| {
        WordError::UnknownProcess(format!("Word's window ({window}) has no process"))
    })?;
    if started.contains(&pid) {
        Ok(pid)
    } else {
        Err(WordError::SharedInstance)
    }
}

/// Quits a Word that the session started, without saving. Errors are ignored:
/// the usual cause is a Word that has already ended.
fn quit(app: &Dispatch) {
    let _ = app.call("Quit", vec![("SaveChanges", WD_DO_NOT_SAVE_CHANGES.into())]);
}

impl Drop for WordSession {
    /// Quits Word without saving, releases it, and waits for its process to
    /// end (for at most ten seconds). Errors cannot be reported from here; a
    /// Word that is already gone is the usual cause.
    fn drop(&mut self) {
        if let Some(app) = self.app.take() {
            quit(&app);
            // Release the object before waiting: Word ends when its last user lets go.
            drop(app);
            wait_for_exit(self.pid);
        }
    }
}

/// Waits until the process is gone, for at most [`EXIT_WAIT`].
fn wait_for_exit(pid: u32) {
    let deadline = Instant::now() + EXIT_WAIT;
    while Instant::now() < deadline {
        match com::process_ids(WORD_EXE) {
            Ok(ids) if !ids.contains(&pid) => return,
            Ok(_) => sleep(EXIT_POLL),
            Err(_) => return,
        }
    }
}

/// The colours Word shows in a cell. `None` is "automatic".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellColours {
    /// The fill behind the text.
    pub fill: Option<Rgb>,
    /// The text colour. Word does not turn "automatic" into black or white here.
    pub text: Option<Rgb>,
}

/// A Word colour (`WdColor`, blue in the high byte) as a colour, or `None` for
/// "automatic" (`wdColorAutomatic` is -16777216) and for the other special values.
fn word_colour(value: i32) -> Option<Rgb> {
    let value = u32::try_from(value).ok().filter(|v| *v <= 0x00FF_FFFF)?;
    Some(Rgb {
        r: (value & 0xFF) as u8,
        g: ((value >> 8) & 0xFF) as u8,
        b: ((value >> 16) & 0xFF) as u8,
    })
}

/// A document that a [`WordSession`] opened. It is closed without
/// saving when [`close`](WordDocument::close) is called or when it is dropped.
/// It cannot outlive its session.
pub struct WordDocument<'session> {
    /// `None` after it was closed.
    document: Option<Dispatch>,
    session: PhantomData<&'session WordSession>,
}

impl WordDocument<'_> {
    /// How many paragraphs Word counts in the main text. This is Word's own
    /// count (it includes the end of each table row), not the model's.
    pub fn paragraph_count(&self) -> Result<u32> {
        let count = self.document().get_object("Paragraphs")?.get_i32("Count")?;
        Ok(u32::try_from(count).unwrap_or(0))
    }

    /// What Word shows for one cell of one table: the fill behind the text and
    /// the text colour, with the table style, the paragraph style and direct
    /// formatting all applied. `table`, `row` and `column` count from 0; `column`
    /// counts the cells of the row (a cell that spans columns is one). For a
    /// check of the model against Word.
    pub fn cell_colours(&self, table: usize, row: usize, column: usize) -> Result<CellColours> {
        let index = |n: usize| i32::try_from(n + 1).unwrap_or(i32::MAX);
        let cell = self
            .document()
            .get_object("Tables")?
            .call_object("Item", vec![("Index", index(table).into())])?
            .call_object(
                "Cell",
                vec![("Row", index(row).into()), ("Column", index(column).into())],
            )?;
        let fill = cell
            .get_object("Shading")?
            .get_i32("BackgroundPatternColor")?;
        let text = cell
            .get_object("Range")?
            .get_object("Font")?
            .get_i32("Color")?;
        Ok(CellColours {
            fill: word_colour(fill),
            text: word_colour(text),
        })
    }

    /// The title in the document's properties (File > Info > Title).
    pub fn title(&self) -> Result<String> {
        Ok(self
            .document()
            .get_object("BuiltInDocumentProperties")?
            .get_object_with("Item", vec![("Index", BSTR::from("Title").into())])?
            .get_string("Value")?)
    }

    /// The text of paragraph `n` of the main text, counting from 0, with the
    /// paragraph mark. Word's own paragraph numbers (see
    /// [`paragraph_count`](WordDocument::paragraph_count)).
    pub fn paragraph_text(&self, n: usize) -> Result<String> {
        Ok(self.paragraph(n)?.get_object("Range")?.get_string("Text")?)
    }

    /// The name of the style of paragraph `n` (for example `Heading 2`), in
    /// Word's language.
    pub fn paragraph_style(&self, n: usize) -> Result<String> {
        Ok(self
            .paragraph(n)?
            .get_object("Style")?
            .get_string("NameLocal")?)
    }

    /// The alignment of paragraph `n` (`WdParagraphAlignment`: 0 left, 1
    /// centre, 2 right, 3 justified).
    pub fn paragraph_alignment(&self, n: usize) -> Result<i32> {
        Ok(self.paragraph(n)?.get_i32("Alignment")?)
    }

    /// How many pictures and other objects are in the text line (inline).
    pub fn inline_shape_count(&self) -> Result<u32> {
        self.count_of("InlineShapes")
    }

    /// How many floating shapes and pictures the document has.
    pub fn floating_shape_count(&self) -> Result<u32> {
        self.count_of("Shapes")
    }

    /// The alt text of inline object `n`, counting from 0. Word fails to give
    /// the alt text (error 0x80004005) of a document that is not its active
    /// one, so the document is made active first. The session is hidden, so
    /// nothing shows.
    pub fn inline_shape_alt_text(&self, n: usize) -> Result<String> {
        self.document().call("Activate", Vec::new())?;
        Ok(item(&self.document().get_object("InlineShapes")?, n)?.get_string("AlternativeText")?)
    }

    /// How many rows table `table` has (counting from 0).
    pub fn row_count(&self, table: usize) -> Result<u32> {
        let count = self.table(table)?.get_object("Rows")?.get_i32("Count")?;
        Ok(u32::try_from(count).unwrap_or(0))
    }

    /// Whether row `row` of table `table` repeats at the top of each page (it
    /// is a header row).
    pub fn row_is_header(&self, table: usize, row: usize) -> Result<bool> {
        Ok(self.row(table, row)?.get_i32("HeadingFormat")? != 0)
    }

    /// Whether Word may split row `row` of table `table` across two pages.
    pub fn row_can_split(&self, table: usize, row: usize) -> Result<bool> {
        Ok(self.row(table, row)?.get_i32("AllowBreakAcrossPages")? != 0)
    }

    fn count_of(&self, collection: &str) -> Result<u32> {
        let count = self.document().get_object(collection)?.get_i32("Count")?;
        Ok(u32::try_from(count).unwrap_or(0))
    }

    fn paragraph(&self, n: usize) -> std::result::Result<Dispatch, ComError> {
        item(&self.document().get_object("Paragraphs")?, n)
    }

    fn table(&self, table: usize) -> std::result::Result<Dispatch, ComError> {
        item(&self.document().get_object("Tables")?, table)
    }

    fn row(&self, table: usize, row: usize) -> std::result::Result<Dispatch, ComError> {
        item(&self.table(table)?.get_object("Rows")?, row)
    }

    /// Closes the document without saving.
    pub fn close(mut self) -> Result<()> {
        match self.document.take() {
            Some(document) => Ok(close_without_saving(&document)?),
            None => Ok(()),
        }
    }

    fn document(&self) -> &Dispatch {
        self.document
            .as_ref()
            .expect("the document is open until it is closed")
    }
}

impl Drop for WordDocument<'_> {
    fn drop(&mut self) {
        if let Some(document) = self.document.take() {
            let _ = close_without_saving(&document);
        }
    }
}

/// Item `n` of a Word collection. Word counts from 1; `n` counts from 0.
fn item(collection: &Dispatch, n: usize) -> std::result::Result<Dispatch, ComError> {
    let index = i32::try_from(n + 1).unwrap_or(i32::MAX);
    collection.call_object("Item", vec![("Index", index.into())])
}

fn close_without_saving(document: &Dispatch) -> std::result::Result<(), ComError> {
    document
        .call(
            "Close",
            vec![("SaveChanges", WD_DO_NOT_SAVE_CHANGES.into())],
        )
        .map(drop)
}
