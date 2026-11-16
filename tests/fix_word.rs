//! Stage 2 tests that need Word (level `windows_word` in the test plan): S2-02
//! (the lock file name Word writes), S2-04 (never write while Word has the file
//! open), S2-15 (Word opens every fixed file) and the part of S2-23 that Word
//! can answer.
//!
//! Run them with `cargo test --features windows_word --test fix_word`. Every
//! test starts its own hidden Word, works in its own folder under the temp
//! folder, and never touches a Word that the user has open. Word never saves.
#![cfg(all(windows, feature = "windows_word"))]

mod fix_support;
mod word_support;

use std::any::Any;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use adapdf::findings::{FixKind, RuleId};
use adapdf::fix::{self, FileSystem, FixError, FixRequest, RealFs, lock_file_names};
use adapdf::fixtures::{Image, Para, catalogue, clean};
use adapdf::model::Document;
use adapdf::word::{WordDocument, WordSession};
use fix_support::{
    confirm_one, findings, fix_file, fix_file_with, fixture, of, replace_part, requests_for,
};
use word_support::{TempDir, exclusive, wait_for_word_pids, within, word_pids};

/// The names of the files in a folder, sorted.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The `~$` files in a folder.
fn lock_files(folder: &Path) -> Vec<String> {
    names(folder)
        .into_iter()
        .filter(|name| name.starts_with("~$"))
        .collect()
}

/// The `~$` files in a folder after Word has closed a document. Word deletes
/// its owner file when it closes the document; this waits up to 5 seconds.
fn lock_files_after_close(folder: &Path) -> Vec<String> {
    for _ in 0..50 {
        if lock_files(folder).is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    lock_files(folder)
}

// ---------- S2-02 ----------

/// What Word did with one document that it had open for editing.
struct LockObservation {
    file: String,
    locks: Vec<String>,
    refused_with_a_lock_error: bool,
    bytes_unchanged: bool,
    locks_after_close: Vec<String>,
}

#[test]
fn fix_detects_the_lock_file_word_writes() {
    let _lock = exclusive();
    let before = word_pids();
    let bytes = fixture(RuleId::Doc002, true);
    let requests = confirm_one(&bytes, RuleId::Doc002);
    let dir = TempDir::new("s2-02");
    let folder = dir.path().to_path_buf();

    // Base names of 1 to 12 characters (the length decides the rule), 20 and 35
    // characters, a name with a space, a name with a letter that is not ASCII,
    // and a capital extension.
    let alphabet = "abcdefghijklmnopqrstuvwxyz0123456789";
    let of_length = |n: usize| -> String { alphabet.chars().cycle().take(n).collect() };
    let mut files: Vec<String> = (1..=12)
        .chain([20, 35])
        .map(|n| format!("{}.docx", of_length(n)))
        .collect();
    files.extend(["My Guide.docx", "Guía de uso.docx", "Guide.DOCX"].map(str::to_string));
    let paths: Vec<PathBuf> = files.iter().map(|name| dir.write(name, &bytes)).collect();
    let original = bytes.clone();

    let seen = within(Duration::from_secs(180), move |session: &WordSession| {
        paths
            .iter()
            .map(|path| {
                let document = session.open_for_editing(path).expect("Word opens the file");
                let locks = lock_files(&folder);
                let refused_with_a_lock_error =
                    matches!(fix_file(path, &requests), Err(FixError::LockFile { .. }));
                let bytes_unchanged = fs::read(path).unwrap() == original;
                document.close().expect("Word closes the file");
                LockObservation {
                    file: path.file_name().unwrap().to_string_lossy().into_owned(),
                    locks,
                    refused_with_a_lock_error,
                    bytes_unchanged,
                    locks_after_close: lock_files_after_close(&folder),
                }
            })
            .collect::<Vec<_>>()
    });

    assert_eq!(seen.len(), files.len());
    for observation in &seen {
        println!("{} -> {:?}", observation.file, observation.locks);
        assert_eq!(
            observation.locks,
            lock_file_names(&observation.file),
            "the lock file that Word wrote for {}",
            observation.file
        );
        assert!(
            observation.refused_with_a_lock_error,
            "{}: the fix must stop on the lock file",
            observation.file
        );
        assert!(observation.bytes_unchanged, "{}", observation.file);
        assert!(
            observation.locks_after_close.is_empty(),
            "{}: Word left {:?}",
            observation.file,
            observation.locks_after_close
        );
    }
    assert_eq!(wait_for_word_pids(&before), before);
}

// ---------- S2-04 ----------

/// What happened when `fix` ran on a file while Word had it (or another file) open.
struct Attempt {
    library: Result<fix::FixOutcome, FixError>,
    exit_code: Option<i32>,
    stdout_empty: bool,
    stderr: String,
    names: Vec<String>,
    /// The file has the same bytes as `original` after the attempt.
    bytes_unchanged: bool,
}

/// A file system that does not see `~$` files, so the check for Word's owner
/// file cannot stop the fix. Only the exclusive open of the file is left to
/// notice that Word has the file open.
struct BlindToLockFiles;

impl FileSystem for BlindToLockFiles {
    fn exists(&self, path: &Path) -> bool {
        let lock = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("~$"));
        !lock && RealFs.exists(path)
    }
    fn open_exclusive(&self, path: &Path) -> io::Result<(Box<dyn Any>, Vec<u8>)> {
        RealFs.open_exclusive(path)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        RealFs.read(path)
    }
    fn write_new(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        RealFs.write_new(path, bytes)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        RealFs.rename(from, to)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        RealFs.remove_file(path)
    }
}

/// Runs the fix with the library and, when the library refuses it, with the
/// command too (the command would ask questions and write if it were let in).
fn attempt(path: &Path, requests: &[FixRequest], original: &[u8]) -> Attempt {
    attempt_with(&RealFs, path, requests, original)
}

/// The same with the file calls of `files` for the library. The command always
/// uses the real file system.
fn attempt_with(
    files: &dyn FileSystem,
    path: &Path,
    requests: &[FixRequest],
    original: &[u8],
) -> Attempt {
    let library = fix_file_with(files, path, requests);
    let (exit_code, stdout_empty, stderr) = if library.is_err() {
        let out = Command::new(env!("CARGO_BIN_EXE_adapdf"))
            .arg("fix")
            .arg(path)
            .stdin(Stdio::null())
            .output()
            .expect("run adapdf");
        (
            out.status.code(),
            out.stdout.is_empty(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    } else {
        (None, true, String::new())
    };
    Attempt {
        library,
        exit_code,
        stdout_empty,
        stderr,
        names: names(path.parent().unwrap()),
        bytes_unchanged: fs::read(path).unwrap() == original,
    }
}

#[test]
fn fix_never_writes_while_word_has_the_file_open() {
    let _lock = exclusive();
    let before = word_pids();
    let bytes = fixture(RuleId::Doc002, true);
    let requests = confirm_one(&bytes, RuleId::Doc002);
    let dir = TempDir::new("s2-04");
    let guide = dir.write("Guide.docx", &bytes);
    let other = dir.write("Other.docx", &bytes);
    let (guide_in_thread, other_in_thread) = (guide.clone(), other.clone());

    let original = bytes.clone();
    let (open_for_editing, open_read_only, blind_to_lock, other_open) =
        within(Duration::from_secs(180), move |session: &WordSession| {
            let guide = guide_in_thread;
            let editing = session.open_for_editing(&guide).expect("Word opens Guide");
            let first = attempt(&guide, &requests, &original);
            editing.close().expect("close");
            let read_only = session.open_read_only(&guide).expect("Word opens Guide");
            let second = attempt(&guide, &requests, &original);
            read_only.close().expect("close");
            // Open for editing again, but the check for the owner file cannot
            // see it: the exclusive open must stop the fix on its own.
            let editing = session.open_for_editing(&guide).expect("Word opens Guide");
            let blind = attempt_with(&BlindToLockFiles, &guide, &requests, &original);
            editing.close().expect("close");
            let elsewhere = session
                .open_for_editing(&other_in_thread)
                .expect("Word opens Other");
            let third = attempt(&guide, &requests, &original);
            elsewhere.close().expect("close");
            (first, second, blind, third)
        });

    // Open for reading and writing: the owner file is there, and the fix stops.
    assert!(
        matches!(open_for_editing.library, Err(FixError::LockFile { .. })),
        "{:?}",
        open_for_editing.library.as_ref().err()
    );
    // Open read-only: Word writes no owner file, but it holds the file. The
    // exclusive open fails, and the fix stops.
    assert!(
        matches!(open_read_only.library, Err(FixError::CannotWrite { .. })),
        "{:?}",
        open_read_only.library.as_ref().err()
    );
    // The exclusive open alone stops the fix when Word has the file open for
    // editing and the owner file is not seen (no backup, no temporary file).
    assert!(
        matches!(blind_to_lock.library, Err(FixError::CannotWrite { .. })),
        "{:?}",
        blind_to_lock.library.as_ref().err()
    );
    assert_eq!(
        blind_to_lock.names,
        ["Guide.docx", "Other.docx", "~$Guide.docx"]
    );
    for (what, attempt) in [
        ("open for editing", &open_for_editing),
        ("open read-only", &open_read_only),
    ] {
        assert_eq!(attempt.exit_code, Some(2), "{what}: {}", attempt.stderr);
        assert!(attempt.stdout_empty, "{what}");
        assert!(attempt.bytes_unchanged, "{what}: the bytes changed");
        assert!(
            attempt.stderr.contains("Guide.docx"),
            "{what}: {}",
            attempt.stderr
        );
    }
    // No backup and no temporary file in either case.
    assert_eq!(
        open_for_editing.names,
        ["Guide.docx", "Other.docx", "~$Guide.docx"]
    );
    assert_eq!(open_read_only.names, ["Guide.docx", "Other.docx"]);
    assert!(
        blind_to_lock.bytes_unchanged,
        "the bytes changed (owner file hidden)"
    );

    // Word has a different file open: the fix runs, with a backup.
    let outcome = other_open.library.expect("the fix runs");
    assert_eq!(outcome.applied.len(), 1);
    let backup = outcome.backup.expect("a backup was written");
    assert_eq!(fs::read(backup).unwrap(), bytes);
    assert_ne!(fs::read(&guide).unwrap(), bytes, "Guide.docx was fixed");
    assert_eq!(
        fs::read(&other).unwrap(),
        bytes,
        "Other.docx was not touched"
    );

    assert_eq!(wait_for_word_pids(&before), before);
}

// ---------- S2-15 ----------

type Check = fn(&WordDocument, &WordDocument) -> Result<(), Box<dyn std::error::Error>>;

/// A document, the fixes to apply, and how Word shows the result.
struct Case {
    name: String,
    original: Vec<u8>,
    requests: Vec<FixRequest>,
    /// Compares what Word reads in the original and in the fixed file. `None`
    /// when the case only has to open.
    check: Option<Check>,
    /// Word counts the end of each table row as a paragraph, so the paragraph
    /// counts of the model and Word are compared only for documents with no table.
    has_table: bool,
}

fn same<T: PartialEq + std::fmt::Debug>(
    what: &str,
    found: T,
    expected: T,
) -> Result<(), Box<dyn std::error::Error>> {
    if found == expected {
        Ok(())
    } else {
        Err(format!("{what}: Word gives {found:?}, expected {expected:?}").into())
    }
}

/// `WdParagraphAlignment` values.
const LEFT: i32 = 0;
const JUSTIFIED: i32 = 3;

/// How Word shows the result of the fix for a rule, for the fixtures of the
/// catalogue (the paragraph numbers are those of the fixtures).
fn word_check_for(rule: RuleId) -> Option<Check> {
    let check: Check = match rule {
        RuleId::Doc001 => |_, fixed| same("title", fixed.title()?, "Typed text".to_string()),
        RuleId::Hdg003 => |_, fixed| {
            same(
                "text",
                fixed.paragraph_text(2)?.starts_with("Setting up"),
                true,
            )?;
            same("style", fixed.paragraph_style(2)?, "Heading 2".to_string())
        },
        RuleId::Lst001 => |original, fixed| {
            same(
                "text before",
                original.paragraph_text(2)?,
                "1. Open the file.\r".to_string(),
            )?;
            same(
                "text after",
                fixed.paragraph_text(2)?,
                "Open the file.\r".to_string(),
            )?;
            same(
                "style",
                fixed.paragraph_style(2)?,
                "List Number".to_string(),
            )?;
            same(
                "style",
                fixed.paragraph_style(3)?,
                "List Number".to_string(),
            )
        },
        RuleId::Img001 | RuleId::Img002 => |_, fixed| {
            same(
                "alt text",
                fixed.inline_shape_alt_text(0)?,
                "Typed text".to_string(),
            )
        },
        RuleId::Img004 => |original, fixed| {
            same("floating before", original.floating_shape_count()?, 1)?;
            same("inline before", original.inline_shape_count()?, 0)?;
            same("floating after", fixed.floating_shape_count()?, 0)?;
            same("inline after", fixed.inline_shape_count()?, 1)
        },
        RuleId::Img007 => |original, fixed| {
            same(
                "shapes",
                fixed.floating_shape_count()?,
                original.floating_shape_count()?,
            )
        },
        RuleId::Tbl001 => |original, fixed| {
            same("header row before", original.row_is_header(0, 0)?, false)?;
            same("header row after", fixed.row_is_header(0, 0)?, true)
        },
        RuleId::Tbl005 => |original, fixed| {
            same("rows before", original.row_count(0)?, 4)?;
            same("rows after", fixed.row_count(0)?, 3)
        },
        RuleId::Tbl006 => |original, fixed| {
            for row in 0..original.row_count(0)? as usize {
                same(
                    "row can split before",
                    original.row_can_split(0, row)?,
                    true,
                )?;
                same("row can split after", fixed.row_can_split(0, row)?, false)?;
            }
            Ok(())
        },
        RuleId::Txt002 => |original, fixed| {
            same(
                "alignment before",
                original.paragraph_alignment(2)?,
                JUSTIFIED,
            )?;
            same("alignment after", fixed.paragraph_alignment(2)?, LEFT)
        },
        _ => return None,
    };
    Some(check)
}

/// The document with a paragraph style removed from `word/styles.xml`.
fn removing_style(bytes: &[u8], id: &str) -> Vec<u8> {
    replace_part(bytes, "word/styles.xml", |t| {
        let start = t
            .find(&format!(r#"<w:style w:type="paragraph" w:styleId="{id}">"#))
            .expect("the style is in the document");
        let end = start + t[start..].find("</w:style>").unwrap() + "</w:style>".len();
        format!("{}{}", &t[..start], &t[end..])
    })
}

fn with_image(image: &Image) -> Vec<u8> {
    clean()
        .paragraph(Para::empty().raw_run(&image.xml()))
        .build()
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    // Every failing fixture that has an auto or ask fix. `UpdateFields`
    // (TXT-004) writes nothing, so TXT-004 is not here.
    for fixture in catalogue().into_iter().filter(|f| f.fails) {
        let found = findings(&fixture.bytes);
        let Some(first) = of(&found, fixture.rule).into_iter().next() else {
            continue;
        };
        if matches!(first.fix, FixKind::Manual) || fixture.rule == RuleId::Txt004 {
            continue;
        }
        cases.push(Case {
            name: fixture.rule.to_string(),
            requests: requests_for(&found, fixture.rule, "Typed text"),
            check: word_check_for(fixture.rule),
            has_table: fixture.rule.to_string().starts_with("TBL"),
            original: fixture.bytes,
        });
    }

    // A heading style that the document does not define is added (decided
    // point 4). Word must read the added definition as Heading 2.
    let undefined = removing_style(&fixture(RuleId::Hdg003, true), "Heading2");
    cases.push(Case {
        name: "HDG-003 with Heading 2 not defined".to_string(),
        requests: requests_for(&findings(&undefined), RuleId::Hdg003, ""),
        original: undefined,
        check: word_check_for(RuleId::Hdg003),
        has_table: false,
    });

    // Alt text on the kinds of drawing that Word opens: the text goes into
    // `descr` and into the VML copy when there is one. The in-memory chart and
    // SmartArt images are not here: their parts are stand-ins, and Word cannot
    // open even the original file (see "Known gaps in stage 2" in the test plan).
    for (name, image) in [
        ("picture", Image::picture()),
        ("shape", Image::shape()),
        ("group", Image::group()),
        ("canvas", Image::canvas()),
        ("vml picture", Image::vml_picture()),
        ("vml shape", Image::vml_shape()),
        ("vml group", Image::vml_group()),
    ] {
        let original = with_image(&image);
        cases.push(Case {
            name: format!("IMG-001 alt text on a {name}"),
            requests: requests_for(&findings(&original), RuleId::Img001, "Typed text"),
            original,
            check: None,
            has_table: false,
        });
    }

    // "Mark as decorative".
    let original = fixture(RuleId::Img001, true);
    let found = of(&findings(&original), RuleId::Img001);
    cases.push(Case {
        name: "IMG-001 marked as decorative".to_string(),
        requests: vec![FixRequest::decorative(&found[0]).at(0)],
        original,
        check: None,
        has_table: false,
    });
    cases
}

/// What one case found in Word, as lines of text (empty when all is well).
fn word_problems(
    session: &WordSession,
    case: &Case,
    original: &Path,
    fixed: &Path,
    model_delta: i64,
) -> Vec<String> {
    let mut problems = Vec::new();
    let fail =
        |what: &str, error: &dyn std::fmt::Display| format!("{}: {what}: {error}", case.name);
    let before = match session.open_read_only(original) {
        Ok(document) => document,
        Err(error) => return vec![fail("Word does not open the ORIGINAL", &error)],
    };
    let after = match session.open_read_only(fixed) {
        Ok(document) => document,
        Err(error) => return vec![fail("Word does not open the FIXED file", &error)],
    };
    if !case.has_table {
        match (before.paragraph_count(), after.paragraph_count()) {
            (Ok(b), Ok(a)) => {
                let word_delta = i64::from(a) - i64::from(b);
                if word_delta != model_delta {
                    problems.push(fail(
                        "paragraph count",
                        &format!("Word changed by {word_delta}, the model by {model_delta}"),
                    ));
                }
            }
            (b, a) => problems.push(fail("paragraph count", &format!("{b:?} {a:?}"))),
        }
    }
    if let Some(check) = case.check
        && let Err(error) = check(&before, &after)
    {
        problems.push(fail("Word reads the fix differently", &error));
    }
    let _ = before.close();
    let _ = after.close();
    problems
}

#[test]
fn fix_result_opens_in_word() {
    let _lock = exclusive();
    let before_pids = word_pids();
    let dir = TempDir::new("s2-15");
    let model_paragraphs = |path: &Path| {
        Document::from_bytes(&fs::read(path).unwrap())
            .unwrap()
            .paragraphs
            .len() as i64
    };
    let mut work = Vec::new();
    for (number, case) in cases().into_iter().enumerate() {
        assert!(!case.requests.is_empty(), "{}: no fix to apply", case.name);
        // Two copies: the one the fix changes, and the one that stays as it was.
        let original = dir.write(&format!("original-{number}.docx"), &case.original);
        let fixed = dir.write(&format!("fixed-{number}.docx"), &case.original);
        let outcome = fix_file(&fixed, &case.requests)
            .unwrap_or_else(|error| panic!("{}: the fix run failed: {error}", case.name));
        assert!(
            outcome.refused.is_empty() && !outcome.applied.is_empty(),
            "{}: applied {}, refused {:?}",
            case.name,
            outcome.applied.len(),
            outcome.refused
        );
        let model_delta = model_paragraphs(&fixed) - model_paragraphs(&original);
        work.push((case, original, fixed, model_delta));
    }
    println!("{} cases", work.len());

    let problems = within(Duration::from_secs(600), move |session: &WordSession| {
        work.iter()
            .flat_map(|(case, original, fixed, delta)| {
                word_problems(session, case, original, fixed, *delta)
            })
            .collect::<Vec<_>>()
    });
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(wait_for_word_pids(&before_pids), before_pids);
}

// ---------- S2-23 ----------

/// The part of S2-23 that Word can answer: Word sees a group, a canvas and a
/// shape that the document makes floating as floating shapes, and not as
/// objects in the text line. This is the reason the fix is not offered for them
/// (`fix_does_not_offer_inline_for_groups_canvases_and_shapes`). Whether the
/// layout survives "In Line with Text" is for a person to look at.
#[test]
fn fix_make_inline_for_groups_canvases_and_shapes() {
    let _lock = exclusive();
    let before = word_pids();
    let dir = TempDir::new("s2-23");
    let documents: Vec<(&str, PathBuf)> = [
        ("group", Image::group()),
        ("canvas", Image::canvas()),
        ("shape", Image::shape()),
    ]
    .into_iter()
    .map(|(name, image)| {
        let bytes = with_image(&image.alt("A drawing").floating());
        (name, dir.write(&format!("{name}.docx"), &bytes))
    })
    .collect();

    let seen = within(Duration::from_secs(120), move |session: &WordSession| {
        documents
            .iter()
            .map(|(name, path)| {
                let document = session.open_read_only(path).expect("Word opens the file");
                (
                    *name,
                    document.floating_shape_count().unwrap(),
                    document.inline_shape_count().unwrap(),
                )
            })
            .collect::<Vec<_>>()
    });
    for (name, floating, inline) in seen {
        println!("{name}: {floating} floating, {inline} inline");
        assert_eq!(inline, 0, "{name}: Word shows it in the text line");
        assert!(floating >= 1, "{name}: Word sees no floating shape");
    }
    assert_eq!(wait_for_word_pids(&before), before);
}
