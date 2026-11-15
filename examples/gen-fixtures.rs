//! Writes every test fixture to `tests/fixtures/<RULE-ID>-pass.docx` and
//! `<RULE-ID>-fail.docx`.
//!
//! Run it with `cargo run --example gen-fixtures` after you change a fixture or
//! the fixture builder. The test `tests/fixtures.rs` fails when the files on
//! disk differ from what this program writes.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    fs::create_dir_all(&dir)?;

    let fixtures = adapdf::fixtures::catalogue();
    let names: HashSet<String> = fixtures.iter().map(|f| f.file_name()).collect();

    // Remove .docx files that are no longer in the catalogue.
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        let stale = path.extension().is_some_and(|e| e == "docx")
            && !path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| names.contains(n));
        if stale {
            fs::remove_file(&path)?;
            println!("removed {}", path.display());
        }
    }

    for fixture in &fixtures {
        let path = dir.join(fixture.file_name());
        fs::write(&path, &fixture.bytes)?;
        println!("wrote {} ({} bytes)", path.display(), fixture.bytes.len());
    }
    Ok(())
}
