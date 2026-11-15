//! Errors that stop the tool (exit code 2). Problems found in a document are
//! not errors; they are [`crate::findings::Finding`] values.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the file cannot be read: {0}")]
    Io(#[from] io::Error),

    #[error("this is not a .docx file: {0}")]
    NotDocx(String),

    #[error("the document is damaged: {part} is not valid XML ({reason})")]
    Xml { part: String, reason: String },

    #[error("the document is too large to check: {part} is bigger than {limit_mib} MiB")]
    PartTooLarge { part: String, limit_mib: u64 },
}

pub type Result<T> = std::result::Result<T, Error>;
