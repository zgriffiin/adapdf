//! The sign-off (row A-08). A static HTML report cannot save what a person types
//! or ticks without a script, so the app saves the human checklist and the
//! sign-off into the report: the name, the date and the state of each of the
//! seven items, in the JSON and in the HTML.
//!
//! The JSON is read back into a [`Report`], the checklist is changed, and both
//! files are written again from the same value (the HTML with
//! `adapdf::report::html::render`). Nothing else in the report changes, and the
//! PDF is never opened. Both files are written to temporary files next to the
//! report first, and then moved over the old ones (the JSON first). A failure of
//! the first move leaves the old files as they were. A failure of the second leaves
//! the JSON signed and the page as it was, and the message says so.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};

use adapdf::report::html;
use serde::Deserialize;

use crate::commands::{CommandError, CommandResult, ReportView, read_report, report_paths};

/// What the person typed and ticked.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Signoff {
    /// The name of the person who signs. It must not be empty.
    pub name: String,
    /// `yyyy-mm-dd`, or empty when the person gave none. The app does not make
    /// one up.
    pub date: String,
    /// The state of each checklist item, in the order of the report.
    pub items: Vec<bool>,
}

/// Whether the text is a real date written `yyyy-mm-dd` (the value of an HTML
/// date field).
fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = text.get(range)?;
        if part.bytes().all(|b| b.is_ascii_digit()) {
            part.parse().ok()
        } else {
            None
        }
    };
    let (Some(year), Some(month), Some(day)) = (number(0..4), number(5..7), number(8..10)) else {
        return false;
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year >= 1 && (1..=days).contains(&day)
}

static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A file written under a temporary name next to its final name. It is removed
/// when the value is dropped, unless it was moved into place.
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn write(target: &Path, content: &str) -> io::Result<TempFile> {
        let name = target
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        loop {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                target.with_file_name(format!("{name}.adapdf-signoff-{}-{n}.tmp", process::id()));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    // From here the value removes the file, also after a failed write.
                    let temp = TempFile { path };
                    file.write_all(content.as_bytes())?;
                    file.sync_all()?;
                    return Ok(temp);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
    }

    /// Replaces `target` with the file. If this fails, `target` is as it was.
    fn move_to(&self, target: &Path) -> io::Result<()> {
        fs::rename(&self.path, target)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Saves the sign-off into the report whose JSON is at `report_json`, and
/// returns the report as saved.
pub fn save_signoff(report_json: &Path, signoff: &Signoff) -> CommandResult<ReportView> {
    let name = signoff.name.trim();
    if name.is_empty() {
        return Err(CommandError::new(
            "Type your name to sign off. The app does not fill it in for you.",
        ));
    }
    let date = signoff.date.trim();
    if !date.is_empty() && !is_date(date) {
        return Err(CommandError::new(
            "The date is not a date. Pick it from the date field, or leave it empty.",
        ));
    }
    let (json_path, html_path) = report_paths(report_json)?;
    let mut view = read_report(&json_path)?;
    let checklist = &mut view.report.checklist;
    if signoff.items.len() != checklist.items.len() {
        return Err(CommandError::new(format!(
            "The checklist has {} items and the sign-off has {}. Open the report again.",
            checklist.items.len(),
            signoff.items.len()
        )));
    }
    for (item, done) in checklist.items.iter_mut().zip(&signoff.items) {
        item.done = *done;
    }
    checklist.name = Some(name.to_string());
    checklist.date = (!date.is_empty()).then(|| date.to_string());

    let write_error = |path: &Path, what: &str, e: io::Error| {
        CommandError::new(format!(
            "The report {} cannot be {what} ({e}). The sign-off was not saved.",
            path.display()
        ))
    };
    let temp_json = TempFile::write(&json_path, &view.report.to_json())
        .map_err(|e| write_error(&json_path, "written", e))?;
    let temp_html = TempFile::write(&html_path, &html::render(&view.report))
        .map_err(|e| write_error(&html_path, "written", e))?;
    // The JSON is moved first. It is the record: `read_report` and the app read it.
    // When the first move fails, nothing was replaced. When the second move fails
    // (the page is open in a program that holds it), the record has the sign-off
    // and the page does not, and the message says so: a page that is behind
    // understates the sign-off, and a page that is ahead of the record would claim
    // a sign-off that the record does not have.
    temp_json
        .move_to(&json_path)
        .map_err(|e| write_error(&json_path, "replaced", e))?;
    temp_html.move_to(&html_path).map_err(|e| {
        CommandError::new(format!(
            "The sign-off is saved in {}, but the page {} cannot be replaced ({e}), so it does not show the sign-off yet. Close the page if it is open, then save the sign-off again.",
            json_path.display(),
            html_path.display()
        ))
    })?;
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_real_dates_in_the_form_of_an_html_date_field() {
        for good in ["2026-11-17", "2024-02-29", "2000-02-29", "0001-01-01"] {
            assert!(is_date(good), "{good}");
        }
        for bad in [
            "",
            "2026-13-01",
            "2026-00-10",
            "2026-02-29",
            "1900-02-29",
            "2026-04-31",
            "2026-4-1",
            "26-04-01",
            "2026/04/01",
            "2026-04-0x",
            "0000-01-01",
            "+026-04-01",
            "2026-04-011",
        ] {
            assert!(!is_date(bad), "{bad}");
        }
    }
}
