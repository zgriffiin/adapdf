//! What the tool does before and while it writes a .docx (spec 02, stage 2).
//!
//! The user's original is never written. Everything the tool makes goes into
//! one folder next to it, `Guide - Adapdf`. Fixes go into the accessible copy
//! `Guide - Adapdfccessible.docx`:
//!
//! 0. refuse when the full path of the copy is too long for Word to open
//!    ([`WORD_PATH_LIMIT`]),
//! 1. refuse when Word has the file that will be written open (the `~$` owner
//!    file, then an exclusive open of the file),
//! 2. write the new file to a temporary file in the same folder, then move it
//!    over the copy (or into place, when the copy is new).
//!
//! If anything fails, the copy is as it was (or not there) and the original is
//! untouched. All file calls go through [`FileSystem`], so a test can record
//! them or make one fail.

use std::any::Any;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::FixError;

/// How many temporary files with the same name are tried before the tool
/// gives up.
const MAX_NAME_TRIES: u32 = 100;

/// The file calls of stage 2.
pub trait FileSystem {
    /// True when the path exists (a file or a folder).
    fn exists(&self, path: &Path) -> bool;
    /// Opens the file for writing with no sharing, and reads it. The first
    /// value is the open handle: the file stays locked until it is dropped.
    fn open_exclusive(&self, path: &Path) -> io::Result<(Box<dyn Any>, Vec<u8>)>;
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    /// Writes a new file and closes it. Fails with `AlreadyExists` when the
    /// path exists.
    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    /// Moves a file over another one.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    /// Makes a folder and the folders above it. It is not an error when the
    /// folder exists.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFs;

impl FileSystem for RealFs {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn open_exclusive(&self, path: &Path) -> io::Result<(Box<dyn Any>, Vec<u8>)> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0);
        }
        let mut file = options.open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok((Box::new(file), bytes))
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }
}

/// A date and time, for the report. [`Timestamp::now`] is the clock; a test
/// gives a fixed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl Timestamp {
    /// The current local time, which is the time a person sees on the clock.
    /// On Windows it comes from the system (`GetLocalTime`). The standard
    /// library has no local time zone, so on other systems this is UTC.
    pub fn now() -> Timestamp {
        #[cfg(windows)]
        {
            super::local_time::now()
        }
        #[cfg(not(windows))]
        Timestamp::now_utc()
    }

    /// The current time in UTC.
    pub fn now_utc() -> Timestamp {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Timestamp::from_unix_seconds(seconds)
    }

    pub fn from_unix_seconds(seconds: u64) -> Timestamp {
        let days = (seconds / 86_400) as i64;
        let rest = (seconds % 86_400) as u32;
        // Civil date from a day count (H. Hinnant's algorithm).
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let mp = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Timestamp {
            year,
            month,
            day,
            hour: rest / 3600,
            minute: rest % 3600 / 60,
            second: rest % 60,
        }
    }

    /// `yyyy-mm-dd hh:mm:ss`, for the report.
    pub fn display(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// The name Word gives the owner (lock) file of a document, for the file name
/// `Guide.docx`. It is a list with one name.
///
/// Word writes the owner file next to the document while it has the document
/// open for editing. The rule was read from a real Word (Microsoft 365, 16.0;
/// test S2-02): the name starts with `~$`, and for a long name the `~$`
/// replaces its first characters. By the length of the name before the
/// extension:
///
/// - a name of 6 characters or fewer (before the extension) keeps all of them
///   after `~$`: `Guide.docx` gives `~$Guide.docx`;
/// - a name of 7 characters loses its first character: `Guide12.docx` gives
///   `~$uide12.docx`;
/// - a name of 8 characters or more loses its first 2 characters:
///   `Guide123.docx` gives `~$ide123.docx`.
///
/// Characters are counted as Unicode characters. Names with characters outside
/// the Basic Multilingual Plane were not tried. A document that Word has open
/// read-only gets no owner file; the exclusive open of the file catches that.
pub fn lock_file_names(file_name: &str) -> Vec<String> {
    let base = file_name
        .rfind('.')
        .map_or(file_name, |dot| &file_name[..dot]);
    let dropped = base.chars().count().saturating_sub(6).min(2);
    let rest: String = file_name.chars().skip(dropped).collect();
    vec![format!("~${rest}")]
}

/// The owner file of Word that exists next to the document, if any.
pub fn find_lock_file(fs: &dyn FileSystem, path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let folder = path.parent().unwrap_or(Path::new(""));
    lock_file_names(&file_name)
        .into_iter()
        .map(|name| folder.join(name))
        .find(|candidate| fs.exists(candidate))
}

/// A held lock on the document, with the bytes read through it. Drop it to
/// let go of the file.
pub struct Opened {
    pub bytes: Vec<u8>,
    /// Held only to keep the file locked until the value is dropped.
    _handle: Box<dyn Any>,
}

/// Checks that Word does not have the file open, and opens it for writing with
/// no sharing. Nothing is written.
pub fn open_for_fixing(fs: &dyn FileSystem, path: &Path) -> Result<Opened, FixError> {
    if let Some(lock) = find_lock_file(fs, path) {
        return Err(FixError::LockFile {
            file: path.to_path_buf(),
            lock,
        });
    }
    let (handle, bytes) = fs
        .open_exclusive(path)
        .map_err(|source| FixError::CannotWrite {
            file: path.to_path_buf(),
            source,
        })?;
    Ok(Opened {
        bytes,
        _handle: handle,
    })
}

/// The file name of the working copy. It is short on purpose: Word cannot open
/// a file whose full path is longer than [`WORD_PATH_LIMIT`], and the guide's
/// name is already in the name of the folder.
pub const COPY_NAME: &str = "accessible.docx";

/// The ending of the folder that holds everything made for a guide.
pub const FOLDER_SUFFIX: &str = " - Adapdf";

/// The name of the HTML report in the folder of a guide.
pub const REPORT_HTML: &str = "report.html";

/// The name of the JSON report in the folder of a guide.
pub const REPORT_JSON: &str = "report.json";

/// The longest full path, in characters, that Word opens: 259. Measured on
/// 2026-11-18 with the tool's own hidden Word (Microsoft 365, build 16.0.20527),
/// on documents opened read-only and for editing, in a folder with a long name:
/// a file at every length from 240 to 259 characters opens, and a file at 260,
/// 261 or 262 does not ("Word could not open ..."). This is the Windows limit
/// `MAX_PATH` (260), which counts the end of the string. The test
/// `word_opens_the_longest_path_and_no_more` (`tests/word_path_limit.rs`) checks
/// it again.
pub const WORD_PATH_LIMIT: usize = 259;

fn stem_of(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_string_lossy().into_owned())
}

/// The name of the guide (`Guide`) when `path` is a working copy: a file named
/// [`COPY_NAME`] inside a folder named `Guide - Adapdf`. The check ignores
/// case, as Windows does.
pub fn original_stem(path: &Path) -> Option<String> {
    if !path
        .file_name()?
        .to_string_lossy()
        .eq_ignore_ascii_case(COPY_NAME)
    {
        return None;
    }
    let folder = path.parent()?.file_name()?.to_string_lossy().into_owned();
    let cut = folder.len().checked_sub(FOLDER_SUFFIX.len())?;
    let (start, end) = (folder.get(..cut)?, folder.get(cut..)?);
    (end.eq_ignore_ascii_case(FOLDER_SUFFIX) && !start.trim().is_empty()).then(|| start.to_string())
}

/// True when the file is the working copy: `accessible.docx` in `<name> - Adapdf`.
pub fn is_accessible_copy(path: &Path) -> bool {
    original_stem(path).is_some()
}

/// The folder for the original `Guide.docx`: `Guide - Adapdf`, next to it.
pub fn adapdf_folder(original: &Path) -> PathBuf {
    let stem = stem_of(original).unwrap_or_else(|| "document".to_string());
    original.with_file_name(format!("{stem}{FOLDER_SUFFIX}"))
}

/// The accessible copy that fixes go into: `accessible.docx` in the folder
/// `Guide - Adapdf` next to `Guide.docx`. For a file that is already a copy,
/// the file itself.
pub fn accessible_copy_path(path: &Path) -> PathBuf {
    if is_accessible_copy(path) {
        return path.to_path_buf();
    }
    adapdf_folder(path).join(COPY_NAME)
}

/// The original of a copy: `Guide.docx` for `Guide - Adapdfccessible.docx`,
/// next to the folder. For a file that is not a copy, the file itself.
pub fn original_path(path: &Path) -> PathBuf {
    let (Some(stem), Some(folder)) = (original_stem(path), path.parent()) else {
        return path.to_path_buf();
    };
    folder.with_file_name(format!("{stem}.docx"))
}

/// The folder where the PDF and the reports of `docx` go: the folder of the
/// file when it is an accessible copy, and `Guide - Adapdf` for an original.
pub fn output_folder(docx: &Path) -> PathBuf {
    if is_accessible_copy(docx) {
        match docx.parent() {
            Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
            _ => PathBuf::from("."),
        }
    } else {
        adapdf_folder(docx)
    }
}

/// The name that the PDF has, without the extension: the name of the original,
/// so `Guide` for `Guide - Adapdfccessible.docx`.
pub fn output_stem(docx: &Path) -> String {
    original_stem(docx)
        .or_else(|| stem_of(docx))
        .unwrap_or_else(|| "document".to_string())
}

/// The length of the full path, as Word counts it: UTF-16 code units of the
/// absolute path.
pub fn word_path_length(path: &Path) -> usize {
    let full = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    full.to_string_lossy().encode_utf16().count()
}

/// Stops when Word cannot open the copy at `copy` because its full path is too
/// long ([`WORD_PATH_LIMIT`]). Nothing is written.
pub fn check_word_path(copy: &Path) -> Result<(), FixError> {
    let length = word_path_length(copy);
    if length > WORD_PATH_LIMIT {
        return Err(FixError::PathTooLong {
            length,
            limit: WORD_PATH_LIMIT,
        });
    }
    Ok(())
}

/// What to do when the accessible copy of an original already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistingCopy {
    /// Stop with [`FixError::CopyExists`]. Nothing is written.
    Refuse,
    /// Replace the copy, with everything that was changed in it since.
    Replace,
}

/// What a write to the copy needs: the bytes to start from, the copy, and the
/// hold on the copy while it exists.
pub struct Prepared {
    /// The bytes that the fixes start from: the original, or the copy.
    pub bytes: Vec<u8>,
    /// The file that is written.
    pub copy: PathBuf,
    held: Option<Opened>,
}

/// Checks everything that can stop a write and reads the file that the fixes
/// start from. Nothing is written.
///
/// - `path` is a copy: Word must not have it open (the lock file, then an
///   exclusive open), and it is both the start and the target.
/// - `path` is an original: it is only read. If its copy exists, `existing`
///   says what to do, and a copy that is going to be replaced must not be open
///   in Word either.
pub fn prepare(
    fs: &dyn FileSystem,
    path: &Path,
    existing: ExistingCopy,
) -> Result<Prepared, FixError> {
    let copy = accessible_copy_path(path);
    check_word_path(&copy)?;
    if is_accessible_copy(path) {
        let opened = open_for_fixing(fs, path)?;
        return Ok(Prepared {
            bytes: opened.bytes.clone(),
            copy,
            held: Some(opened),
        });
    }
    let held = if fs.exists(&copy) {
        if existing == ExistingCopy::Refuse {
            return Err(FixError::CopyExists {
                original: path.to_path_buf(),
                copy,
            });
        }
        Some(open_for_fixing(fs, &copy)?)
    } else {
        None
    };
    let bytes = fs.read(path).map_err(|source| FixError::CannotRead {
        file: path.to_path_buf(),
        source,
    })?;
    Ok(Prepared { bytes, copy, held })
}

impl Prepared {
    /// Writes `new` to the copy: a temporary file in the same folder first,
    /// then a move over the copy. When this fails, the copy is as it was, and a
    /// folder that this write made is removed again if it is still empty.
    pub fn write(self, fs: &dyn FileSystem, new: &[u8]) -> Result<PathBuf, FixError> {
        let folder = self.copy.parent().filter(|f| !f.as_os_str().is_empty());
        let made = folder.filter(|f| !fs.exists(f));
        if let Some(folder) = made {
            fs.create_dir_all(folder)
                .map_err(|source| FixError::Write {
                    file: self.copy.clone(),
                    source,
                })?;
        }
        // Fails when a file is in the folder, which is what is wanted.
        let undo_folder = || {
            if let Some(folder) = made {
                let _ = std::fs::remove_dir(folder);
            }
        };
        let temp = match write_temp(fs, &self.copy, new) {
            Ok(temp) => temp,
            Err(error) => {
                undo_folder();
                return Err(error);
            }
        };
        drop(self.held); // let go of the copy so it can be replaced
        if let Err(source) = fs.rename(&temp, &self.copy) {
            let _ = fs.remove_file(&temp);
            undo_folder();
            return Err(FixError::Write {
                file: self.copy,
                source,
            });
        }
        Ok(self.copy)
    }
}

fn write_temp(fs: &dyn FileSystem, path: &Path, new: &[u8]) -> Result<PathBuf, FixError> {
    let name = path.file_name().map_or_else(
        || "document".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut last = None;
    for attempt in 0..MAX_NAME_TRIES {
        let temp = path.with_file_name(format!("{name}.adapdf-{attempt}.tmp"));
        match fs.write_new(&temp, new) {
            Ok(()) => return Ok(temp),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(e),
            Err(source) => {
                let _ = fs.remove_file(&temp);
                return Err(FixError::Write {
                    file: path.to_path_buf(),
                    source,
                });
            }
        }
    }
    Err(FixError::Write {
        file: path.to_path_buf(),
        source: last.unwrap_or_else(|| io::Error::other("no temporary file name is free")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_from_unix_seconds() {
        let t = Timestamp::from_unix_seconds(0);
        assert_eq!(t.display(), "1970-01-01 00:00:00");
        // 2026-03-05 10:15:30 UTC
        let t = Timestamp::from_unix_seconds(1_772_705_730);
        assert_eq!(t.display(), "2026-03-05 10:15:30");
        // 2024-02-29 23:59:59 UTC (a leap day)
        let t = Timestamp::from_unix_seconds(1_709_251_199);
        assert_eq!(t.display(), "2024-02-29 23:59:59");
    }

    #[test]
    fn lock_file_names_for_short_and_long_names() {
        // The names that Word wrote in test S2-02.
        for (file_name, lock) in [
            ("a.docx", "~$a.docx"),
            ("Guide.docx", "~$Guide.docx"),
            ("abcdef.docx", "~$abcdef.docx"),
            ("abcdefg.docx", "~$bcdefg.docx"),
            ("abcdefgh.docx", "~$cdefgh.docx"),
            ("Document1.docx", "~$cument1.docx"),
            ("My Guide.docx", "~$ Guide.docx"),
            ("Guide.DOCX", "~$Guide.DOCX"),
        ] {
            assert_eq!(lock_file_names(file_name), [lock], "{file_name}");
        }
    }

    #[test]
    fn copy_names() {
        let original = Path::new("C:/guides/Guide.docx");
        let copy = Path::new("C:/guides/Guide - Adapdf/accessible.docx");
        assert_eq!(
            adapdf_folder(original),
            Path::new("C:/guides/Guide - Adapdf")
        );
        assert_eq!(accessible_copy_path(original), copy);
        assert!(!is_accessible_copy(original));
        assert!(is_accessible_copy(copy));
        // A copy is its own copy.
        assert_eq!(accessible_copy_path(copy), copy);
        assert_eq!(original_path(copy), original);
        assert_eq!(original_path(original), original);
        // The outputs: in the folder of the guide, named after the original.
        assert_eq!(
            output_folder(original),
            Path::new("C:/guides/Guide - Adapdf")
        );
        assert_eq!(output_folder(copy), Path::new("C:/guides/Guide - Adapdf"));
        assert_eq!(output_stem(original), "Guide");
        assert_eq!(output_stem(copy), "Guide");
        // The name and the folder are found whatever their case, as Windows
        // sees them.
        assert!(is_accessible_copy(Path::new(
            "a/Guide - ADAPDF/Accessible.DOCX"
        )));
        // Not a copy: another name, a folder without the ending, no name before
        // the ending, or no folder.
        for other in [
            "Guide - Adapdf/accessible.doc",
            "Guide - Adapdf/accessible 2.docx",
            "Guide - Adapdf/Guide (accessible).docx",
            "Guide - Adapdf/Guide.docx",
            "Guide/accessible.docx",
            "Guide -Adapdf/accessible.docx",
            " - Adapdf/accessible.docx",
            "accessible.docx",
            "D:/accessible.docx",
        ] {
            assert!(!is_accessible_copy(Path::new(other)), "{other}");
        }
        // The copy of a name that holds dots, spaces and non-ASCII letters.
        let odd = Path::new("x/2026.11.18 - Guía v2.1 - Adapdf/accessible.docx");
        assert_eq!(
            accessible_copy_path(Path::new("x/2026.11.18 - Guía v2.1.docx")),
            odd
        );
        assert_eq!(output_stem(odd), "2026.11.18 - Guía v2.1");
        assert_eq!(
            original_path(odd),
            Path::new("x/2026.11.18 - Guía v2.1.docx")
        );
        // A relative name.
        assert_eq!(
            accessible_copy_path(Path::new("Guide.docx")),
            Path::new("Guide - Adapdf/accessible.docx")
        );
        assert_eq!(
            original_path(Path::new("Guide - Adapdf/accessible.docx")),
            Path::new("Guide.docx")
        );
    }

    #[test]
    fn the_path_limit_is_checked_on_the_full_path() {
        // A path of exactly `n` characters.
        let of_length = |n: usize| {
            let head = "C:/g/";
            PathBuf::from(format!("{head}{}", "a".repeat(n - head.len())))
        };
        assert_eq!(word_path_length(&of_length(100)), 100);
        assert!(check_word_path(&of_length(WORD_PATH_LIMIT - 1)).is_ok());
        assert!(check_word_path(&of_length(WORD_PATH_LIMIT)).is_ok());
        let error = check_word_path(&of_length(WORD_PATH_LIMIT + 1)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "The path of the Word copy would be 260 characters, and Word cannot open files with paths over 259 characters. Move the guide to a folder with a shorter path (for example a folder nearer the top of OneDrive), then try again."
        );
        // The count is in UTF-16 units, as Windows counts.
        let wide = PathBuf::from(format!("C:/g/{}", "\u{1F600}".repeat(130)));
        assert_eq!(word_path_length(&wide), 5 + 260);
        assert!(check_word_path(&wide).is_err());
    }
}
