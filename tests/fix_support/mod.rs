//! Helpers for the stage 2 (fix) tests.
#![allow(dead_code)] // each test file uses a different part of this module

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use adapdf::findings::{Finding, FixKind, RuleId};
use adapdf::fix::{self, Applied, FixInput, FixOptions, FixRequest, RealFs, Splice, Timestamp};
use adapdf::fixtures::{DocxBuilder, FixtureFonts, catalogue, zip_stored};

pub const BOM: &[u8] = b"\xEF\xBB\xBF";

/// The typed text that the escaping tests use.
pub const AWKWARD_TEXT: &str = "A & B <C> \"D\" é 日本";

/// The 2026-03-05 10:15:30 clock of the backup tests.
pub fn fixed_clock() -> Timestamp {
    Timestamp {
        year: 2026,
        month: 3,
        day: 5,
        hour: 10,
        minute: 15,
        second: 30,
    }
}

/// The seconds from 1970-01-01 00:00:00 to the date and time on a clock (the
/// reading is taken as if it were UTC). It is the reverse of
/// `Timestamp::from_unix_seconds`, and it lets a test compare two clock readings.
pub fn wall_clock_seconds(t: &Timestamp) -> i64 {
    let year = t.year - i64::from(t.month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = i64::from(if t.month > 2 {
        t.month - 3
    } else {
        t.month + 9
    });
    let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(t.day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    days * 86_400 + i64::from(t.hour) * 3600 + i64::from(t.minute) * 60 + i64::from(t.second)
}

/// The bytes of the failing (`fails`) or passing fixture of a rule.
pub fn fixture(rule: RuleId, fails: bool) -> Vec<u8> {
    catalogue()
        .into_iter()
        .find(|f| f.rule == rule && f.fails == fails)
        .expect("the catalogue has this fixture")
        .bytes
}

pub fn findings(bytes: &[u8]) -> Vec<Finding> {
    adapdf::check_bytes(bytes, &FixtureFonts).expect("a valid .docx")
}

pub fn of(findings: &[Finding], rule: RuleId) -> Vec<Finding> {
    findings
        .iter()
        .filter(|f| f.rule_id == rule)
        .cloned()
        .collect()
}

/// The rules of the findings, one entry for each finding, in order.
pub fn rules(findings: &[Finding]) -> Vec<RuleId> {
    findings.iter().map(|f| f.rule_id).collect()
}

/// The names and bytes of the entries of a zip file, in order.
pub fn parts(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("a zip file");
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            (entry.name().unwrap().into_owned(), data)
        })
        .collect()
}

/// The text of a part.
pub fn part(bytes: &[u8], name: &str) -> String {
    let (_, data) = parts(bytes)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no part {name}"));
    String::from_utf8(data).expect("a UTF-8 part")
}

/// The same document with the text of one part changed.
pub fn replace_part(bytes: &[u8], name: &str, change: impl Fn(&str) -> String) -> Vec<u8> {
    let all = parts(bytes);
    let changed: Vec<(String, Vec<u8>)> = all
        .into_iter()
        .map(|(n, data)| {
            if n == name {
                let text = String::from_utf8(data).unwrap();
                let new = change(&text);
                assert_ne!(new, text, "the change did nothing to {name}");
                (n, new.into_bytes())
            } else {
                (n, data)
            }
        })
        .collect();
    let entries: Vec<(&str, &[u8])> = changed
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    zip_stored(&entries)
}

pub fn build(builder: &DocxBuilder) -> Vec<u8> {
    builder.build()
}

pub fn options() -> FixOptions {
    FixOptions::default()
}

/// Applies the requests in memory. Panics when the fix run fails.
pub fn apply(bytes: &[u8], requests: &[FixRequest]) -> Applied {
    fix::apply(bytes, requests, &options()).expect("the fix run works")
}

/// A request for every finding of a rule: a yes for auto fixes, `typed` for
/// the fixes that take text, and a yes for IMG-007.
pub fn requests_for(findings: &[Finding], rule: RuleId, typed: &str) -> Vec<FixRequest> {
    of(findings, rule)
        .iter()
        .enumerate()
        .map(|(position, f)| match f.fix {
            FixKind::Auto(_) => FixRequest::confirm(f).at(position),
            FixKind::Ask(adapdf::findings::FixId::MarkAnnotationsDecorative) => {
                FixRequest::confirm(f).at(position)
            }
            FixKind::Ask(_) => FixRequest::text(f, typed).at(position),
            FixKind::Manual => panic!("{rule} has no fix"),
        })
        .collect()
}

/// Applies the fix of every finding of `rule` in the document.
pub fn fix_rule(bytes: &[u8], rule: RuleId) -> Applied {
    let found = findings(bytes);
    let requests = requests_for(&found, rule, "Typed text");
    assert!(!requests.is_empty(), "{rule} has no finding to fix");
    apply(bytes, &requests)
}

/// The new .docx of a fix run. Panics when nothing was changed.
pub fn fixed(applied: &Applied) -> &[u8] {
    applied.bytes.as_deref().expect("the fix changed the file")
}

/// Applies splices by hand, from the last to the first.
pub fn by_hand(original: &str, splices: &[Splice]) -> String {
    let mut text = original.to_string();
    let mut sorted: Vec<&Splice> = splices.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse((s.range.start, s.range.end)));
    for s in sorted {
        text.replace_range(s.range.clone(), &s.text);
    }
    text
}

fn without_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(BOM).unwrap_or(bytes)
}

/// Spec 02, stage 2: only the spliced bytes change. Every entry has the same
/// name and place; every part with no splice has the same bytes; every part
/// with splices equals the original with exactly those splices applied by hand,
/// and the splices do not overlap.
pub fn assert_only_the_splices_changed(original: &[u8], applied: &Applied) {
    let new = fixed(applied);
    let before = parts(original);
    let after = parts(new);
    let names = |p: &[(String, Vec<u8>)]| p.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    assert_eq!(names(&before), names(&after), "entries and their order");
    let known: BTreeSet<&str> = before.iter().map(|(n, _)| n.as_str()).collect();
    for name in applied.splices.keys() {
        assert!(known.contains(name.as_str()), "splices for {name}");
    }
    for ((name, old), (_, now)) in before.iter().zip(&after) {
        match applied.splices.get(name) {
            None => assert!(old == now, "{name} must be byte-identical"),
            Some(splices) => {
                assert!(!splices.is_empty());
                for pair in splices.windows(2) {
                    assert!(
                        pair[0].range.end <= pair[1].range.start,
                        "overlap in {name}"
                    );
                }
                let old_text = std::str::from_utf8(without_bom(old)).unwrap();
                let expected = by_hand(old_text, splices);
                assert_eq!(
                    std::str::from_utf8(without_bom(now)).unwrap(),
                    expected,
                    "{name} is the original with the splices applied"
                );
                assert_eq!(
                    old.starts_with(BOM),
                    now.starts_with(BOM),
                    "the byte order mark of {name}"
                );
                assert_ne!(old, now, "{name} changed");
            }
        }
    }
}

/// A folder for one test. It is removed when the value is dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        let call = CALLS.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("fix")
            .join(format!("{name}-{}-{call}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Writes a file in the folder.
    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    /// The names of the files in the folder, sorted.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if let Ok(entries) = fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    let mut permissions = meta.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    permissions.set_readonly(false);
                    let _ = fs::set_permissions(entry.path(), permissions);
                }
            }
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs the fixes on a file with the real file system and the fixed clock.
pub fn fix_file(path: &Path, requests: &[FixRequest]) -> Result<fix::FixOutcome, fix::FixError> {
    fix_file_with(&RealFs, path, requests)
}

pub fn fix_file_with(
    fs: &dyn fix::FileSystem,
    path: &Path,
    requests: &[FixRequest],
) -> Result<fix::FixOutcome, fix::FixError> {
    let env = fix::Environment {
        fs,
        now: fixed_clock(),
        fonts: &FixtureFonts,
    };
    fix::fix_file(path, requests, &options(), &env)
}

/// The request for the one finding of a rule in a document, with a yes.
pub fn confirm_one(bytes: &[u8], rule: RuleId) -> Vec<FixRequest> {
    let found = of(&findings(bytes), rule);
    assert_eq!(found.len(), 1, "{rule}: {found:#?}");
    vec![FixRequest::confirm(&found[0])]
}

pub fn text_input(text: &str) -> FixInput {
    FixInput::Text(text.to_string())
}
