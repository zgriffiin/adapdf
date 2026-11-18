//! Writing the fixed .docx: every entry that did not change is copied raw
//! (same compressed bytes, same method, same time stamp), and only the changed
//! parts are written again. A replacement for a name that is not in the file
//! is added at the end (the fix log adds `docProps/custom.xml`).

use std::collections::BTreeMap;
use std::io::Cursor;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{Error, Result};

/// The zip file with the parts in `replacements` (name to new bytes) in place
/// of the old ones. The entries keep their order and names. A name that the
/// file does not have is added after the last entry.
pub fn rewrite(original: &[u8], replacements: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    let bad = |e: zip::result::ZipError| Error::NotDocx(e.to_string());
    let mut archive = ZipArchive::new(Cursor::new(original)).map_err(bad)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::with_capacity(original.len())));
    writer
        .set_raw_comment(archive.comment().to_vec().into_boxed_slice())
        .map_err(bad)?;
    let mut seen: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).map_err(bad)?;
        let name = entry.name().map_err(bad)?.into_owned();
        seen.push(name.clone());
        let Some(bytes) = replacements.get(&name) else {
            writer.raw_copy_file(entry).map_err(bad)?;
            continue;
        };
        let mut options = SimpleFileOptions::default().compression_method(
            if entry.compression() == CompressionMethod::Stored {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            },
        );
        if let Some(time) = entry.last_modified() {
            options = options.last_modified_time(time);
        }
        if let Some(mode) = entry.unix_mode() {
            options = options.unix_permissions(mode);
        }
        drop(entry);
        writer.start_file(name, options).map_err(bad)?;
        std::io::Write::write_all(&mut writer, bytes)?;
    }
    for (name, bytes) in replacements {
        if !seen.contains(name) {
            writer
                .start_file(name.as_str(), SimpleFileOptions::default())
                .map_err(bad)?;
            std::io::Write::write_all(&mut writer, bytes)?;
        }
    }
    Ok(writer.finish().map_err(bad)?.into_inner())
}
