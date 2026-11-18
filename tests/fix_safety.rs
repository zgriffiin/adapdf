//! Stage 2 safety (spec 02; AGENTS.md): the original is never written; the fixes
//! go into the accessible copy `Guide - Adapdf\Guide (accessible).docx`, after the
//! lock file and the exclusive open, and the copy is written atomically. Test plan
//! rows S2-01, S2-03, S2-05, S2-06, S2-07, S2-14, S2-34 to S2-36 and the local time
//! of the fix log (S2-07b).

mod fix_support;

use std::any::Any;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use adapdf::findings::RuleId;
use adapdf::fix::{
    ExistingCopy, FileSystem, FixError, FixRequest, RealFs, accessible_copy_path, fixlog,
    lock_file_names,
};
use adapdf::fixtures::clean;
use fix_support::*;

/// `DOC-002-fail.docx` and the one fix that it needs.
fn doc_002() -> (Vec<u8>, Vec<FixRequest>) {
    let bytes = fixture(RuleId::Doc002, true);
    let requests = confirm_one(&bytes, RuleId::Doc002);
    (bytes, requests)
}

/// The folder of the guide, `Guide - Adapdf`, in a test folder.
fn guide_folder(dir: &TempDir) -> PathBuf {
    dir.path().join("Guide - Adapdf")
}

/// Puts `bytes` where an earlier run left the accessible copy of `Guide.docx`.
fn write_copy(dir: &TempDir, bytes: &[u8]) -> PathBuf {
    let folder = guide_folder(dir);
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("Guide (accessible).docx");
    fs::write(&path, bytes).unwrap();
    path
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn modified(path: &Path) -> std::time::SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

// ---------------------------------------------------------------- S2-01

#[test]
fn fix_refuses_when_a_lock_file_exists() {
    let (bytes, requests) = doc_002();
    // The lock file is next to the accessible copy, which is the file that is
    // written. File names of 5, 8, 9 and 20 characters (before the extension) and
    // one with spaces; the copy name has 13 more characters, so Word cuts the
    // start of every name in the same way (see `lock_file_names`, test S2-02).
    for base in [
        "Guide",
        "Guide123",
        "Guide1234",
        "Guide_for_new_staff",
        "My Guide 2026",
    ] {
        let dir = TempDir::new("lock");
        let folder = dir.path().join(format!("{base} - Adapdf"));
        fs::create_dir_all(&folder).unwrap();
        let copy_name = format!("{base} (accessible).docx");
        let copy = folder.join(&copy_name);
        fs::write(&copy, &bytes).unwrap();
        let lock = &lock_file_names(&copy_name)[0];
        fs::write(folder.join(lock), b"owner").unwrap();
        let original = dir.write(&format!("{base}.docx"), &bytes);

        // Working in the copy.
        let error = fix_file(&copy, &requests).expect_err("a lock file stops the fix");
        assert!(matches!(error, FixError::LockFile { .. }), "{error}");
        let message = error.to_string();
        assert!(message.contains(&copy_name), "{message}");
        assert!(
            message.contains("Close the document in Word first, then try again."),
            "{message}"
        );
        assert_eq!(fs::read(&copy).unwrap(), bytes, "{base} copy bytes");
        let mut expected = vec![copy_name.clone(), lock.clone()];
        expected.sort();
        assert_eq!(names_in(&folder), expected, "no temp file");

        // Starting over from the original replaces the copy, so the copy must not
        // be open either.
        let error = fix_file_existing(&RealFs, &original, &requests, ExistingCopy::Replace)
            .expect_err("a lock file on the copy stops a replace");
        assert!(matches!(error, FixError::LockFile { .. }), "{error}");
        assert_eq!(fs::read(&copy).unwrap(), bytes);
        assert_eq!(fs::read(&original).unwrap(), bytes);
        assert_eq!(names_in(&folder), expected, "no temp file");
    }
}

#[test]
fn a_lock_file_of_another_document_does_not_stop_the_fix() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("other-lock");
    let path = dir.write("Guide.docx", &bytes);
    fs::create_dir_all(guide_folder(&dir)).unwrap();
    fs::write(guide_folder(&dir).join("~$Other.docx"), b"owner").unwrap();
    let outcome = fix_file(&path, &requests).expect("the fix runs");
    assert_eq!(outcome.applied.len(), 1);
}

/// The original is only read, so Word having it open (its lock file is next to it)
/// does not stop a fix: the fix goes into the copy, and the original is as it was.
#[test]
fn a_lock_file_next_to_the_original_does_not_stop_a_fix_into_the_copy() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("original-open");
    let original = dir.write("Guide.docx", &bytes);
    dir.write(&lock_file_names("Guide.docx")[0], b"owner");
    let outcome = fix_file(&original, &requests).expect("the fix runs");
    assert!(outcome.copy.is_some());
    assert_eq!(fs::read(&original).unwrap(), bytes);
}

#[test]
fn lock_file_names_follow_the_rule_that_word_uses() {
    // The names that Word wrote in test S2-02 (`tests/fix_word.rs`).
    assert_eq!(lock_file_names("Guide.docx"), ["~$Guide.docx"]);
    assert_eq!(lock_file_names("Guide12.docx"), ["~$uide12.docx"]);
    assert_eq!(lock_file_names("Guide123.docx"), ["~$ide123.docx"]);
}

// ---------------------------------------------------------------- S2-03

#[cfg(windows)]
#[test]
fn fix_refuses_when_the_file_cannot_be_opened_for_writing() {
    use std::os::windows::fs::OpenOptionsExt;

    let (bytes, requests) = doc_002();

    // The copy is held open by another handle with no sharing.
    let dir = TempDir::new("held");
    let copy = write_copy(&dir, &bytes);
    let holder = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&copy)
        .unwrap();
    let error = fix_file(&copy, &requests).expect_err("a held file stops the fix");
    assert!(matches!(error, FixError::CannotWrite { .. }), "{error}");
    assert!(
        error.to_string().contains("Guide (accessible).docx"),
        "{error}"
    );
    assert!(
        error
            .to_string()
            .contains("Close the document in Word first, then try again."),
        "{error}"
    );
    drop(holder);
    assert_eq!(fs::read(&copy).unwrap(), bytes);
    assert_eq!(names_in(&guide_folder(&dir)), ["Guide (accessible).docx"]);

    read_only_copy(&bytes, &requests);
}

#[cfg(not(windows))]
#[test]
fn fix_refuses_when_the_file_cannot_be_opened_for_writing() {
    let (bytes, requests) = doc_002();
    read_only_copy(&bytes, &requests);
}

fn read_only_copy(bytes: &[u8], requests: &[FixRequest]) {
    let dir = TempDir::new("read-only");
    let copy = write_copy(&dir, bytes);
    let mut permissions = fs::metadata(&copy).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&copy, permissions).unwrap();
    let error = fix_file(&copy, requests).expect_err("a read-only file stops the fix");
    assert!(matches!(error, FixError::CannotWrite { .. }), "{error}");
    assert_eq!(fs::read(&copy).unwrap(), bytes);
    assert_eq!(names_in(&guide_folder(&dir)), ["Guide (accessible).docx"]);
}

// -------------------------------------------------- the calls, and the order

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
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.log("create_dir_all", path);
        RealFs.create_dir_all(path)
    }
}

/// S2-05: the first fix creates the copy: the original is read and nothing is
/// written to it, and the copy comes into place with one move of a temporary file.
#[test]
fn fix_creates_the_copy_atomically_and_only_reads_the_original() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("order");
    let original = dir.write("Guide.docx", &bytes);
    let recording = Recording::default();
    fix_file_with(&recording, &original, &requests).expect("the fix runs");
    let calls = recording.calls.borrow().clone();
    assert_eq!(
        calls,
        [
            "read Guide.docx".to_string(),
            "create_dir_all Guide - Adapdf".to_string(),
            "write_new Guide (accessible).docx.adapdf-0.tmp".to_string(),
            "rename Guide (accessible).docx.adapdf-0.tmp to Guide (accessible).docx".to_string(),
        ],
        "the original is read and never opened for writing or moved"
    );
    assert_eq!(names_in(&guide_folder(&dir)), ["Guide (accessible).docx"]);
    assert_eq!(fs::read(&original).unwrap(), bytes);
}

/// S2-05: a fix in the copy (an ask fix on a document that has a title problem).
#[test]
fn a_second_fix_goes_into_the_copy_with_one_move() {
    let bytes = clean().no_language().title("").build();
    let found = findings(&bytes);
    let dir = TempDir::new("two-fixes");
    let original = dir.write("Guide.docx", &bytes);
    let language = FixRequest::confirm(&of(&found, RuleId::Doc002)[0]);
    let first = fix_file(&original, &[language]).expect("the first fix runs");
    let copy = first.copy.expect("a copy");

    let after_first = fs::read(&copy).unwrap();
    let title = of(&findings(&after_first), RuleId::Doc001);
    assert_eq!(title.len(), 1);
    let recording = Recording::default();
    let second = fix_file_with(
        &recording,
        &copy,
        &[FixRequest::text(&title[0], "Parks Training Guide")],
    )
    .expect("the second fix runs");
    assert_eq!(second.copy.as_deref(), Some(copy.as_path()));
    assert_eq!(
        recording.calls.borrow().clone(),
        [
            "open_exclusive Guide (accessible).docx".to_string(),
            "write_new Guide (accessible).docx.adapdf-0.tmp".to_string(),
            "rename Guide (accessible).docx.adapdf-0.tmp to Guide (accessible).docx".to_string(),
        ]
    );
    // Both fixes are in the copy and in its log, and the original has neither.
    let rest = rules(&findings(&fs::read(&copy).unwrap()));
    assert!(!rest.contains(&RuleId::Doc001) && !rest.contains(&RuleId::Doc002));
    let log = log_of(&fs::read(&copy).unwrap());
    assert_eq!(
        log.iter().map(|e| e.rule).collect::<Vec<_>>(),
        [RuleId::Doc002, RuleId::Doc001]
    );
    assert_eq!(log[1].text.as_deref(), Some("Parks Training Guide"));
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(names_in(&guide_folder(&dir)), ["Guide (accessible).docx"]);
}

fn log_of(bytes: &[u8]) -> Vec<fixlog::FixLogEntry> {
    let package = adapdf::docx::Package::from_bytes(bytes).unwrap();
    fixlog::read(&package).unwrap()
}

// ------------------------------------------------------------- failures

/// What `write_new` and `rename` do in [`Failing`].
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
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        RealFs.create_dir_all(path)
    }
}

/// S2-14: a failed write of the new copy leaves no copy, no partial file and no
/// temporary file, and the original is untouched.
#[test]
fn a_failed_write_leaves_no_partial_copy_and_the_original_intact() {
    let (bytes, requests) = doc_002();
    for (half, fail_rename, fail_write) in [
        (true, false, Some(".tmp")),
        (false, false, Some(".tmp")),
        (false, true, None),
    ] {
        let dir = TempDir::new("atomic");
        let original = dir.write("Guide.docx", &bytes);
        let before = modified(&original);
        let failing = Failing {
            fail_write,
            half,
            fail_rename,
        };
        let error = fix_file_with(&failing, &original, &requests).expect_err("the write fails");
        assert!(matches!(error, FixError::Write { .. }), "{error}");
        assert_eq!(
            fs::read(&original).unwrap(),
            bytes,
            "the original is intact"
        );
        assert_eq!(modified(&original), before);
        assert!(
            !accessible_copy_path(&original).exists(),
            "no copy, no partial copy"
        );
        let leftovers = if guide_folder(&dir).exists() {
            names_in(&guide_folder(&dir))
        } else {
            Vec::new()
        };
        assert!(leftovers.is_empty(), "no temporary file: {leftovers:?}");
        assert_eq!(
            dir.names().iter().filter(|n| n.ends_with(".docx")).count(),
            1
        );
    }
}

/// S2-14: a failed rewrite of the copy leaves the copy as it was.
#[test]
fn a_failed_rewrite_leaves_the_copy_as_it_was() {
    let (bytes, requests) = doc_002();
    for (half, fail_rename, fail_write) in [(true, false, Some(".tmp")), (false, true, None)] {
        let dir = TempDir::new("atomic-copy");
        let copy = write_copy(&dir, &bytes);
        let failing = Failing {
            fail_write,
            half,
            fail_rename,
        };
        let error = fix_file_with(&failing, &copy, &requests).expect_err("the write fails");
        assert!(matches!(error, FixError::Write { .. }), "{error}");
        assert_eq!(fs::read(&copy).unwrap(), bytes, "the copy is intact");
        assert_eq!(
            names_in(&guide_folder(&dir)),
            ["Guide (accessible).docx"],
            "the temporary file is removed"
        );
    }
}

// ------------------------------------------------------- the original

/// S2-06: whatever the fix does, the original keeps its bytes and its modified
/// time.
#[test]
fn the_original_keeps_its_bytes_and_its_modified_time() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("untouched");
    let original = dir.write("Guide.docx", &bytes);
    let before = modified(&original);
    std::thread::sleep(std::time::Duration::from_millis(30));
    let outcome = fix_file(&original, &requests).expect("the fix runs");
    let copy = outcome.copy.expect("a copy");
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(modified(&original), before);
    assert_ne!(fs::read(&copy).unwrap(), bytes, "the copy was fixed");
    // A second round: replace the copy, and fix the copy again.
    fix_file_existing(&RealFs, &original, &requests, ExistingCopy::Replace).expect("replace");
    fix_file(&copy, &[]).expect("a run in the copy");
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(modified(&original), before);
    // Only the original and the folder of the guide are in the folder.
    assert_eq!(dir.names(), ["Guide - Adapdf", "Guide.docx"]);
}

/// S2-07: the copy is the original with the fix and the fix log, and nothing else.
#[test]
fn the_copy_is_the_original_with_the_fix_and_the_log() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("copy-bytes");
    let original = dir.write("Guide.docx", &bytes);
    let outcome = fix_file(&original, &requests).expect("the fix runs");
    let copy_bytes = fs::read(outcome.copy.as_ref().unwrap()).unwrap();
    assert_eq!(
        outcome.copy.as_deref(),
        Some(guide_folder(&dir).join("Guide (accessible).docx").as_path())
    );
    let old = parts(&bytes);
    let new = parts(&copy_bytes);
    let mut changed: Vec<&str> = new
        .iter()
        .filter(|(name, data)| {
            old.iter()
                .find(|(n, _)| n == name)
                .is_none_or(|(_, d)| d != data)
        })
        .map(|(name, _)| name.as_str())
        .collect();
    changed.sort_unstable();
    assert_eq!(
        changed,
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "docProps/custom.xml",
            "word/styles.xml"
        ],
        "the fix, and the part, the content type and the relationship of the log"
    );
    // The log has the fix, with the clock of the test.
    let log = log_of(&copy_bytes);
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].time, "2026-03-05 10:15:30");
    assert_eq!(log[0].rule, RuleId::Doc002);
    assert_eq!(log[0].place, "The whole document (its default language)");
    assert!(log[0].text.is_none());
}

/// S2-34: an accessible copy that exists is not replaced unless the caller says so.
#[test]
fn an_existing_copy_is_refused_unless_it_is_to_be_replaced() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("existing");
    let original = dir.write("Guide.docx", &bytes);
    let copy = write_copy(&dir, b"the copy that a person edited in Word");
    let error = fix_file(&original, &requests).expect_err("no choice, no write");
    assert!(matches!(error, FixError::CopyExists { .. }), "{error}");
    assert!(
        error.to_string().contains("Guide (accessible).docx"),
        "{error}"
    );
    assert_eq!(
        fs::read(&copy).unwrap(),
        b"the copy that a person edited in Word"
    );
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(names_in(&guide_folder(&dir)), ["Guide (accessible).docx"]);

    // Start over: the copy is a new one, made from the original with the fix.
    let outcome =
        fix_file_existing(&RealFs, &original, &requests, ExistingCopy::Replace).expect("a replace");
    assert_eq!(outcome.copy.as_deref(), Some(copy.as_path()));
    let new = fs::read(&copy).unwrap();
    assert_ne!(new, b"the copy that a person edited in Word");
    assert!(!rules(&findings(&new)).contains(&RuleId::Doc002));
    assert_eq!(
        log_of(&new).len(),
        1,
        "the log of the old copy is gone with it"
    );
    assert_eq!(fs::read(&original).unwrap(), bytes);
}

/// S2-35: continuing means that the copy is the file that is given. Its fixes and
/// its log stay, and the new fixes go in.
#[test]
fn working_in_the_copy_keeps_what_is_in_it() {
    let bytes = clean().no_language().title("").build();
    let dir = TempDir::new("continue");
    let original = dir.write("Guide.docx", &bytes);
    let language = FixRequest::confirm(&of(&findings(&bytes), RuleId::Doc002)[0]);
    let copy = fix_file(&original, &[language]).unwrap().copy.unwrap();
    // The person adds a paragraph in Word and saves (a change of the copy).
    let edited = replace_part(&fs::read(&copy).unwrap(), "word/document.xml", |t| {
        t.replacen(
            "<w:body>",
            "<w:body><w:p><w:r><w:t>Added in Word</w:t></w:r></w:p>",
            1,
        )
    });
    fs::write(&copy, &edited).unwrap();
    let title = of(&findings(&edited), RuleId::Doc001);
    fix_file(&copy, &[FixRequest::text(&title[0], "A title")]).unwrap();
    let new = fs::read(&copy).unwrap();
    assert!(part(&new, "word/document.xml").contains("Added in Word"));
    assert_eq!(log_of(&new).len(), 2);
    assert_eq!(fs::read(&original).unwrap(), bytes);
}

/// S2-36: a file that is already named `<name> (accessible).docx` is the working
/// copy, wherever it is: the fixes go into it, and no `(accessible) (accessible)`
/// file and no folder is made.
#[test]
fn a_file_named_accessible_is_fixed_in_place() {
    let (bytes, requests) = doc_002();
    let dir = TempDir::new("in-place");
    let path = dir.write("Guide (accessible).docx", &bytes);
    let outcome = fix_file(&path, &requests).expect("the fix runs");
    assert_eq!(outcome.copy.as_deref(), Some(path.as_path()));
    assert_ne!(fs::read(&path).unwrap(), bytes);
    assert_eq!(dir.names(), ["Guide (accessible).docx"]);
    assert_eq!(log_of(&fs::read(&path).unwrap()).len(), 1);
}

#[test]
fn nothing_to_write_means_no_copy() {
    let bytes = fixture(RuleId::Doc002, true);
    let dir = TempDir::new("nothing");
    let original = dir.write("Guide.docx", &bytes);
    let outcome = fix_file(&original, &[]).expect("an empty run works");
    assert!(outcome.copy.is_none());
    assert_eq!(fs::read(&original).unwrap(), bytes);
    assert_eq!(dir.names(), ["Guide.docx"], "no folder and no copy");
    assert!(outcome.findings.iter().any(|f| f.rule_id == RuleId::Doc002));
}

/// A copy of the original with no fix: the same bytes, written the same way.
#[test]
fn make_copy_writes_the_original_bytes_and_leaves_a_copy_alone() {
    use adapdf::fix::{Environment, make_copy};
    let bytes = fixture(RuleId::Doc002, true);
    let dir = TempDir::new("make-copy");
    let original = dir.write("Guide.docx", &bytes);
    let env = Environment {
        fs: &RealFs,
        now: fixed_clock(),
        fonts: &adapdf::fixtures::FixtureFonts,
    };
    let copy = make_copy(&original, &env, ExistingCopy::Refuse).expect("a copy");
    assert_eq!(copy, accessible_copy_path(&original));
    assert_eq!(fs::read(&copy).unwrap(), bytes);
    assert_eq!(fs::read(&original).unwrap(), bytes);
    // A second call does not replace it, unless it is asked to.
    let error = make_copy(&original, &env, ExistingCopy::Refuse).expect_err("it exists");
    assert!(matches!(error, FixError::CopyExists { .. }), "{error}");
    fs::write(&copy, b"edited in Word").unwrap();
    assert_eq!(
        make_copy(&copy, &env, ExistingCopy::Refuse).unwrap(),
        copy,
        "a copy is its own copy"
    );
    assert_eq!(fs::read(&copy).unwrap(), b"edited in Word");
    make_copy(&original, &env, ExistingCopy::Replace).expect("replace");
    assert_eq!(fs::read(&copy).unwrap(), bytes);
}

/// S2-07b: the time of a fix in the log is local time, which is what a person
/// sees on the clock. The test reads the local time from PowerShell (`Get-Date`),
/// which does not use the code under test, and checks that the production clock
/// is within 2 seconds of it. (On a computer set to UTC, local time and UTC are
/// the same, and the test still holds.)
#[cfg(windows)]
#[test]
fn the_production_clock_is_local_time() {
    use adapdf::fix::Timestamp;

    let before = wall_clock_seconds(&Timestamp::now());
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Date -Format yyyyMMdd-HHmmss",
        ])
        .output()
        .expect("run PowerShell");
    let after = wall_clock_seconds(&Timestamp::now());
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let number = |range: std::ops::Range<usize>| -> u32 {
        text.get(range)
            .and_then(|digits| digits.parse().ok())
            .unwrap_or_else(|| panic!("PowerShell gave {text:?}, not yyyyMMdd-HHmmss"))
    };
    let shown = Timestamp {
        year: i64::from(number(0..4)),
        month: number(4..6),
        day: number(6..8),
        hour: number(9..11),
        minute: number(11..13),
        second: number(13..15),
    };
    let shown = wall_clock_seconds(&shown);
    assert!(
        before - 2 <= shown && shown <= after + 2,
        "the production clock gave {before}..{after}, PowerShell gave {shown} ({text})"
    );
    // The reading is a real date: the helper and `from_unix_seconds` agree.
    let now = Timestamp::now();
    assert_eq!(
        Timestamp::from_unix_seconds(wall_clock_seconds(&now).try_into().unwrap()),
        now
    );
}
