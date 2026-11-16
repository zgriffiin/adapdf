//! What the tool does before and while it writes a .docx (spec 02, stage 2):
//!
//! 1. refuse when Word has the file open (the `~$` owner file, then an
//!    exclusive open of the file),
//! 2. write the backup `Guide.backup-<yyyymmdd-hhmmss>.docx` and read it back,
//! 3. write the new file to a temporary file in the same folder, then replace
//!    the original with it.
//!
//! If anything fails, the original file is as it was. All file calls go
//! through [`FileSystem`], so a test can record them or make one fail.

use std::any::Any;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::FixError;

/// How many backups and temporary files with the same name are tried before
/// the tool gives up.
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
}

/// A date and time, for the name of the backup. [`Timestamp::now`] is the
/// clock; a test gives a fixed one.
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

    /// `yyyymmdd-hhmmss`.
    pub fn backup_stamp(&self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
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
    handle: Box<dyn Any>,
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
    Ok(Opened { bytes, handle })
}

/// `Guide.docx` in the same folder as `path` with `.backup-<stamp>` added.
fn backup_path(path: &Path, stamp: &str, attempt: u32) -> PathBuf {
    let stem = path.file_stem().map_or_else(
        || "document".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let suffix = if attempt == 0 {
        String::new()
    } else {
        format!("-{attempt}")
    };
    path.with_file_name(format!("{stem}.backup-{stamp}{suffix}.docx"))
}

/// Writes the backup (and reads it back), then replaces the file with `new`.
/// Returns the path of the backup.
///
/// The original is not changed unless the last step, the move of the
/// temporary file over it, succeeds.
pub fn write_with_backup(
    fs: &dyn FileSystem,
    path: &Path,
    opened: Opened,
    new: &[u8],
    now: Timestamp,
) -> Result<PathBuf, FixError> {
    let original = &opened.bytes;
    let stamp = now.backup_stamp();
    let backup = write_backup(fs, path, original, &stamp)?;

    let temp = write_temp(fs, path, new)?;
    drop(opened.handle); // let go of the file so it can be replaced
    if let Err(source) = fs.rename(&temp, path) {
        let _ = fs.remove_file(&temp);
        return Err(FixError::Write {
            file: path.to_path_buf(),
            source,
        });
    }
    Ok(backup)
}

fn write_backup(
    fs: &dyn FileSystem,
    path: &Path,
    original: &[u8],
    stamp: &str,
) -> Result<PathBuf, FixError> {
    for attempt in 0..MAX_NAME_TRIES {
        let backup = backup_path(path, stamp, attempt);
        let failed = |source: io::Error| FixError::Backup {
            backup: backup.clone(),
            source,
        };
        match fs.write_new(&backup, original) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                let _ = fs.remove_file(&backup);
                return Err(failed(e));
            }
        }
        match fs.read(&backup) {
            Ok(copy) if copy == original => return Ok(backup),
            Ok(_) => {
                let _ = fs.remove_file(&backup);
                return Err(failed(io::Error::other(
                    "the copy is not the same as the file",
                )));
            }
            Err(e) => return Err(failed(e)),
        }
    }
    Err(FixError::Backup {
        backup: backup_path(path, stamp, 0),
        source: io::Error::other("every backup name is already used"),
    })
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
        assert_eq!(t.backup_stamp(), "19700101-000000");
        // 2026-03-05 10:15:30 UTC
        let t = Timestamp::from_unix_seconds(1_772_705_730);
        assert_eq!(t.backup_stamp(), "20260305-101530");
        // 2024-02-29 23:59:59 UTC (a leap day)
        let t = Timestamp::from_unix_seconds(1_709_251_199);
        assert_eq!(t.backup_stamp(), "20240229-235959");
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
    fn backup_names() {
        let p = Path::new("C:/guides/Guide.docx");
        assert_eq!(
            backup_path(p, "20260305-101530", 0),
            Path::new("C:/guides/Guide.backup-20260305-101530.docx")
        );
        assert_eq!(
            backup_path(p, "20260305-101530", 2),
            Path::new("C:/guides/Guide.backup-20260305-101530-2.docx")
        );
    }
}
