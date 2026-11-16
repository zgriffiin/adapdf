//! Word as the export sees it: two small traits, so that a test can use a fake
//! Word that records the calls, blocks or panics. The real implementation is
//! [`WordSession`] and [`WordDocument`].

use std::path::Path;

use crate::word::{
    ExportParameters, FieldUpdate, Result, WordDocument, WordSession, end_word_process,
};

/// A started Word. It lives on the thread that started it (it is not `Send`).
/// Dropping it quits that Word.
pub(super) trait WordApp {
    /// The id of the Word process that this object started.
    fn pid(&self) -> u32;
    fn build(&self) -> Result<String>;
    /// Opens the document read-only and hidden.
    fn open_read_only(&self, path: &Path) -> Result<Box<dyn WordDoc + '_>>;
}

pub(super) trait WordDoc {
    fn update_fields(&self) -> Result<FieldUpdate>;
    fn export(&self, output: &Path, parameters: &ExportParameters) -> Result<()>;
    /// Closes the document without saving.
    fn close(self: Box<Self>) -> Result<()>;
}

/// Starts a Word. It runs on the Word thread.
pub(super) type StartWord = Box<dyn FnOnce() -> Result<Box<dyn WordApp>> + Send>;

/// Ends a process that did not answer. Gives `Ok(false)` when it has already
/// ended. It is only ever called with the id that [`WordApp::pid`] gave.
pub(super) trait EndProcess {
    fn end(&self, pid: u32) -> Result<bool>;
}

/// The start of the real Word.
pub(super) fn start_real_word() -> StartWord {
    Box::new(|| Ok(Box::new(RealApp(WordSession::start()?)) as Box<dyn WordApp>))
}

/// Ends a real `WINWORD.EXE` (and checks its image name first).
pub(super) struct EndWordProcess;

impl EndProcess for EndWordProcess {
    fn end(&self, pid: u32) -> Result<bool> {
        end_word_process(pid)
    }
}

struct RealApp(WordSession);

impl WordApp for RealApp {
    fn pid(&self) -> u32 {
        self.0.pid()
    }

    fn build(&self) -> Result<String> {
        self.0.build()
    }

    fn open_read_only(&self, path: &Path) -> Result<Box<dyn WordDoc + '_>> {
        Ok(Box::new(RealDoc(self.0.open_read_only(path)?)))
    }
}

struct RealDoc<'session>(WordDocument<'session>);

impl WordDoc for RealDoc<'_> {
    fn update_fields(&self) -> Result<FieldUpdate> {
        self.0.update_fields()
    }

    fn export(&self, output: &Path, parameters: &ExportParameters) -> Result<()> {
        self.0.export_as_fixed_format(output, parameters)
    }

    fn close(self: Box<Self>) -> Result<()> {
        self.0.close()
    }
}
