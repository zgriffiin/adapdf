//! Stage 2 safety (spec 02; AGENTS.md): the lock file, the exclusive open, the
//! backup and the atomic write. Test plan rows S2-01, S2-03, S2-05, S2-06,
//! S2-07 and S2-14.

mod fix_support;

use std::any::Any;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;

use adapdf::findings::RuleId;
use adapdf::fix::{FileSystem, FixError, RealFs, lock_file_names};
use fix_support::*;

/// `DOC-002-fail.docx` and the one fix that it needs.
fn doc_002() -> (Vec<u8>, Vec<adapdf::fix::FixRequest>) {
    let bytes = fixture(RuleId::Doc002, true);
    let requests = confirm_one(&bytes, RuleId::Doc002);
    (bytes, requests)
}

#[test]
fn fix_refuses_when_a_lock_file_exists() {
    let (bytes, requests) = doc_002();
    // File names of 5, 8, 9 and 20 characters (before the extension), and one
    // with spaces. Both forms of the lock file name are tried (see
    // `lock_file_names`).
    for base in [
        "Guide",
        "Guide123",
        "Guide1234",
        "Guide_for_new_staff",
        "My Guide 2026",
    ] {
        let file_name = format!("{base}.docx");
        for lock in lock_file_names(&file_name) {
            let dir = TempDir::new("lock");
            let path = dir.write(&file_name, &bytes);
            dir.write(&lock, b"owner");
            let error = fix_file(&path, &requests).expect_err("a lock file stops the fix");
            assert!(matches!(error, FixError::LockFile { .. }), "{error}");
            let message = error.to_string();
            assert!(message.contains(&file_name), "{message}");
            assert!(message.contains("Close the document in Word"), "{message}");
            assert_eq!(fs::read(&path).unwrap(), bytes, "{file_name} bytes");
            let mut expected = vec![file_name.clone(), lock.clone()];
            expected.sort();
            assert_eq!(dir.names(), expected, "no backup and no temp file");
        }
    }
}

#[test]
fn a_lock_file_of_another_document_does_not_stop_the_fix() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("other-lock");
    let path = dir.write("Guide.docx", &bytes);
    dir.write("~$Other.docx", b"owner");
    let outcome = fix_file(&path, &requests).expect("the fix runs");
    assert_eq!(outcome.applied.len(), 1);
}

#[test]
fn lock_file_names_cover_both_forms_of_the_word_rule() {
    // Until the Word test S2-02 shows the real names, both are checked.
    assert_eq!(
        lock_file_names("Guide.docx"),
        ["~$Guide.docx", "~$ide.docx"]
    );
}

#[cfg(windows)]
#[test]
fn fix_refuses_when_the_file_cannot_be_opened_for_writing() {
    use std::os::windows::fs::OpenOptionsExt;

    let (bytes, requests) = doc_002();

    // Held open by another handle with no sharing.
    let dir = TempDir::new("held");
    let path = dir.write("Guide.docx", &bytes);
    let holder = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let error = fix_file(&path, &requests).expect_err("a held file stops the fix");
    assert!(matches!(error, FixError::CannotWrite { .. }), "{error}");
    assert!(error.to_string().contains("Guide.docx"), "{error}");
    drop(holder);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(dir.names(), ["Guide.docx"], "no backup and no temp file");

    read_only_copy(&bytes, &requests);
}

#[cfg(not(windows))]
#[test]
fn fix_refuses_when_the_file_cannot_be_opened_for_writing() {
    let (bytes, requests) = doc_002();
    read_only_copy(&bytes, &requests);
}

fn read_only_copy(bytes: &[u8], requests: &[adapdf::fix::FixRequest]) {
    let dir = TempDir::new("read-only");
    let path = dir.write("Guide.docx", bytes);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    let error = fix_file(&path, requests).expect_err("a read-only file stops the fix");
    assert!(matches!(error, FixError::CannotWrite { .. }), "{error}");
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(dir.names(), ["Guide.docx"], "no backup and no temp file");
}

/// A file system that does what [`RealFs`] does and writes down each call.
#[derive(Default)]
struct Recording {
    calls: RefCell<Vec<String>>,
}

impl Recording {
    fn log(&self, call: &str, path: &Path) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        self.calls.borrow_mut().push(format!("{call} {name}"));
    }
}

impl FileSystem for Recording {
    fn exists(&self, path: &Path) -> bool {
        RealFs.exists(path)
    }
    fn open_exclusive(&self, path: &Path) -> io::Result<(Box<dyn Any>, Vec<u8>)> {
        self.log("open_exclusive", path);
        RealFs.open_exclusive(path)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.log("read", path);
        RealFs.read(path)
    }
    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.log("write_new", path);
        RealFs.write_new(path, bytes)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.log(
            &format!("rename {} to", from.file_name().unwrap().to_string_lossy()),
            to,
        );
        RealFs.rename(from, to)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.log("remove_file", path);
        RealFs.remove_file(path)
    }
}

#[test]
fn fix_writes_the_backup_before_the_first_write() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("order");
    let path = dir.write("Guide.docx", &bytes);
    let recording = Recording::default();
    fix_file_with(&recording, &path, &requests).expect("the fix runs");
    let calls = recording.calls.borrow().clone();
    let backup = "Guide.backup-20260305-101530.docx";
    assert_eq!(
        calls,
        [
            "open_exclusive Guide.docx".to_string(),
            format!("write_new {backup}"),
            format!("read {backup}"),
            "write_new Guide.docx.adapdf-0.tmp".to_string(),
            "rename Guide.docx.adapdf-0.tmp to Guide.docx".to_string(),
        ],
        "the backup is written and read back before the first write to the target"
    );
    assert_eq!(fs::read(dir.path().join(backup)).unwrap(), bytes);
}

/// A file system that fails some calls.
struct Failing {
    /// A `write_new` for a path that contains this fails.
    fail_write: Option<&'static str>,
    /// Write half of the bytes before the failure.
    half: bool,
    fail_rename: bool,
}

impl FileSystem for Failing {
    fn exists(&self, path: &Path) -> bool {
        RealFs.exists(path)
    }
    fn open_exclusive(&self, path: &Path) -> io::Result<(Box<dyn Any>, Vec<u8>)> {
        RealFs.open_exclusive(path)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        RealFs.read(path)
    }
    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let name = path.file_name().unwrap().to_string_lossy();
        if self.fail_write.is_some_and(|part| name.contains(part)) {
            if self.half {
                RealFs.write_new(path, &bytes[..bytes.len() / 2])?;
            }
            return Err(io::Error::other("the disk is full"));
        }
        RealFs.write_new(path, bytes)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if self.fail_rename {
            return Err(io::Error::other("the file is in use"));
        }
        RealFs.rename(from, to)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        RealFs.remove_file(path)
    }
}

#[test]
fn fix_stops_when_the_backup_cannot_be_written() {
    let (bytes, requests) = doc_002();
    for half in [false, true] {
        let dir = TempDir::new("no-backup");
        let path = dir.write("Guide.docx", &bytes);
        let failing = Failing {
            fail_write: Some(".backup-"),
            half,
            fail_rename: false,
        };
        let error = fix_file_with(&failing, &path, &requests).expect_err("no backup, no write");
        assert!(matches!(error, FixError::Backup { .. }), "{error}");
        assert_eq!(fs::read(&path).unwrap(), bytes, "target bytes");
        assert_eq!(
            dir.names(),
            ["Guide.docx"],
            "no partial backup and no temp file"
        );
    }
}

#[test]
fn fix_backup_name_and_bytes() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("backup");
    let path = dir.write("Guide.docx", &bytes);
    let outcome = fix_file(&path, &requests).expect("the fix runs");
    let backup = dir.path().join("Guide.backup-20260305-101530.docx");
    assert_eq!(outcome.backup.as_deref(), Some(backup.as_path()));
    assert_eq!(
        fs::read(&backup).unwrap(),
        bytes,
        "backup bytes equal the original"
    );
    assert_ne!(fs::read(&path).unwrap(), bytes, "the file was fixed");

    // A second run in the same second does not overwrite the backup.
    fs::write(&path, &bytes).unwrap();
    let outcome = fix_file(&path, &requests).expect("the second fix runs");
    let second = dir.path().join("Guide.backup-20260305-101530-1.docx");
    assert_eq!(outcome.backup.as_deref(), Some(second.as_path()));
    assert_eq!(fs::read(&backup).unwrap(), bytes);
    assert_eq!(fs::read(&second).unwrap(), bytes);
}

/// The real-file-system form of S2-06. On Windows a new file cannot take the
/// name of a folder, and the error is "access denied", not "already exists", so
/// the tool does not try another name: it stops.
#[cfg(windows)]
#[test]
fn a_folder_with_the_name_of_the_backup_stops_the_fix() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("backup-folder");
    let path = dir.write("Guide.docx", &bytes);
    fs::create_dir(dir.path().join("Guide.backup-20260305-101530.docx")).unwrap();
    let error = fix_file(&path, &requests).expect_err("no backup, no write");
    assert!(matches!(error, FixError::Backup { .. }), "{error}");
    assert_eq!(fs::read(&path).unwrap(), bytes, "target bytes");
    assert_eq!(
        dir.names(),
        ["Guide.backup-20260305-101530.docx", "Guide.docx"],
        "the folder is untouched and nothing else was written"
    );
}

#[test]
fn fix_write_is_atomic() {
    let (bytes, requests) = doc_002();

    // The writer fails after half of the bytes of the new file.
    let dir = TempDir::new("atomic");
    let path = dir.write("Guide.docx", &bytes);
    let failing = Failing {
        fail_write: Some(".tmp"),
        half: true,
        fail_rename: false,
    };
    let error = fix_file_with(&failing, &path, &requests).expect_err("the write fails");
    assert!(matches!(error, FixError::Write { .. }), "{error}");
    assert_eq!(fs::read(&path).unwrap(), bytes, "the original is intact");
    assert_eq!(
        dir.names(),
        ["Guide.backup-20260305-101530.docx", "Guide.docx"],
        "no partial file stays; the backup is a whole copy"
    );

    // The move over the original fails.
    let dir = TempDir::new("atomic-rename");
    let path = dir.write("Guide.docx", &bytes);
    let failing = Failing {
        fail_write: None,
        half: false,
        fail_rename: true,
    };
    let error = fix_file_with(&failing, &path, &requests).expect_err("the move fails");
    assert!(matches!(error, FixError::Write { .. }), "{error}");
    assert_eq!(fs::read(&path).unwrap(), bytes, "the original is intact");
    assert_eq!(
        dir.names(),
        ["Guide.backup-20260305-101530.docx", "Guide.docx"],
        "the temporary file is removed"
    );
}

#[test]
fn nothing_to_write_means_no_backup() {
    let bytes = fixture(RuleId::Doc002, true);
    let dir = TempDir::new("nothing");
    let path = dir.write("Guide.docx", &bytes);
    let outcome = fix_file(&path, &[]).expect("an empty run works");
    assert!(outcome.backup.is_none());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(dir.names(), ["Guide.docx"]);
    assert!(outcome.findings.iter().any(|f| f.rule_id == RuleId::Doc002));
}
