//! Finding out whether a font can be embedded in a PDF (rule DOC-006).
//!
//! A font file says what its licence allows in the `fsType` field of its OS/2
//! table. The rule asks a [`FontLookup`]. [`SystemFonts`] reads the fonts
//! installed on this computer. Tests give the rule a fixed lookup, so they do
//! not depend on the machine.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ttf_parser::{Face, Permissions, name_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStatus {
    /// The font is installed and its licence allows embedding.
    Embeddable,
    /// The font is installed and its licence does not allow embedding.
    Restricted,
    /// No installed font file has this family name.
    NotInstalled,
}

pub trait FontLookup {
    /// `family` is the font name as Word stores it, for example `Arial`.
    fn status(&self, family: &str) -> FontStatus;
}

/// Font files bigger than this are skipped. The largest fonts Windows ships
/// are far smaller.
const MAX_FONT_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// How deep to look in sub-folders (Word's cloud fonts are in sub-folders).
const MAX_FOLDER_DEPTH: usize = 2;
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc", "otc"];

/// The fonts installed on this computer. The font files are read on the first
/// lookup, and the result is kept.
pub struct SystemFonts {
    folders: Vec<PathBuf>,
    index: OnceLock<HashMap<String, FontStatus>>,
}

impl SystemFonts {
    pub fn new(folders: Vec<PathBuf>) -> SystemFonts {
        SystemFonts {
            folders,
            index: OnceLock::new(),
        }
    }

    /// The folders Windows and Microsoft 365 install fonts in: the system
    /// folder, the per-user folder, and Word's folder for cloud fonts such
    /// as Aptos.
    pub fn from_environment() -> SystemFonts {
        let mut folders = Vec::new();
        let windir =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from("C:/Windows"), PathBuf::from);
        folders.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = PathBuf::from(local);
            folders.push(local.join("Microsoft/Windows/Fonts"));
            folders.push(local.join("Microsoft/FontCache/4/CloudFonts"));
        }
        SystemFonts::new(folders)
    }

    fn index(&self) -> &HashMap<String, FontStatus> {
        self.index.get_or_init(|| {
            let mut index = HashMap::new();
            for folder in &self.folders {
                scan_folder(folder, 0, &mut index);
            }
            index
        })
    }
}

impl FontLookup for SystemFonts {
    fn status(&self, family: &str) -> FontStatus {
        self.index()
            .get(&family.trim().to_lowercase())
            .copied()
            .unwrap_or(FontStatus::NotInstalled)
    }
}

fn scan_folder(folder: &Path, depth: usize, index: &mut HashMap<String, FontStatus>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            if depth < MAX_FOLDER_DEPTH {
                scan_folder(&path, depth + 1, index);
            }
        } else if is_font_file(&path) {
            read_font_file(&path, index);
        }
    }
}

fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| FONT_EXTENSIONS.iter().any(|f| e.eq_ignore_ascii_case(f)))
}

fn read_font_file(path: &Path, index: &mut HashMap<String, FontStatus>) {
    if fs::metadata(path).is_ok_and(|m| m.len() > MAX_FONT_FILE_BYTES) {
        return;
    }
    let Ok(data) = fs::read(path) else {
        return;
    };
    for face_index in 0..ttf_parser::fonts_in_collection(&data).unwrap_or(1) {
        if let Ok(face) = Face::parse(&data, face_index) {
            add_face(&face, index);
        }
    }
}

/// Records the status of a face under its family names. If one face of a
/// family is restricted, the whole family counts as restricted.
fn add_face(face: &Face, index: &mut HashMap<String, FontStatus>) {
    let permissions = face.permissions();
    let outline_blocked = permissions.is_some() && !face.is_outline_embedding_allowed();
    let status = if permissions == Some(Permissions::Restricted) || outline_blocked {
        FontStatus::Restricted
    } else {
        FontStatus::Embeddable
    };
    for name in face.names() {
        let wanted = matches!(
            name.name_id,
            name_id::FAMILY | name_id::TYPOGRAPHIC_FAMILY | name_id::FULL_NAME
        );
        if let (true, Some(text)) = (wanted, name.to_string()) {
            let entry = index.entry(text.trim().to_lowercase()).or_insert(status);
            if status == FontStatus::Restricted {
                *entry = FontStatus::Restricted;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a tiny but valid TrueType file with one family name and an
    /// OS/2 `fsType` value. It has no glyph outlines.
    fn tiny_font(family: &str, fs_type: u16) -> Vec<u8> {
        let name = {
            let utf16: Vec<u8> = family.encode_utf16().flat_map(u16::to_be_bytes).collect();
            let mut t = Vec::new();
            t.extend([0, 0]); // format
            t.extend(1u16.to_be_bytes()); // count
            t.extend(18u16.to_be_bytes()); // string offset: 6 + 12
            t.extend(3u16.to_be_bytes()); // platform: Windows
            t.extend(1u16.to_be_bytes()); // encoding: Unicode BMP
            t.extend(0x409u16.to_be_bytes()); // language: en-US
            t.extend(1u16.to_be_bytes()); // name id: family
            t.extend((utf16.len() as u16).to_be_bytes());
            t.extend(0u16.to_be_bytes()); // offset
            t.extend(utf16);
            t
        };
        let head = {
            let mut t = vec![0u8; 54];
            t[0..4].copy_from_slice(&[0, 1, 0, 0]); // version 1.0
            t[12..16].copy_from_slice(&0x5F0F3CF5u32.to_be_bytes()); // magic
            t[18..20].copy_from_slice(&1000u16.to_be_bytes()); // units per em
            t
        };
        let hhea = {
            let mut t = vec![0u8; 36];
            t[0..4].copy_from_slice(&[0, 1, 0, 0]);
            t[34..36].copy_from_slice(&1u16.to_be_bytes()); // number of metrics
            t
        };
        let maxp = {
            let mut t = vec![0u8; 6];
            t[0..4].copy_from_slice(&0x0000_5000u32.to_be_bytes());
            t[4..6].copy_from_slice(&1u16.to_be_bytes());
            t
        };
        let os2 = {
            let mut t = vec![0u8; 96];
            t[0..2].copy_from_slice(&2u16.to_be_bytes()); // OS/2 version 2
            t[8..10].copy_from_slice(&fs_type.to_be_bytes());
            t
        };
        let tables: [(&[u8; 4], Vec<u8>); 5] = [
            (b"OS/2", os2),
            (b"head", head),
            (b"hhea", hhea),
            (b"maxp", maxp),
            (b"name", name),
        ];
        let mut file = Vec::new();
        file.extend([0, 1, 0, 0]);
        file.extend((tables.len() as u16).to_be_bytes());
        file.extend([0u8; 6]); // search range, entry selector, range shift
        let mut offset = 12 + 16 * tables.len();
        let mut body = Vec::new();
        for (tag, data) in &tables {
            file.extend(*tag);
            file.extend([0u8; 4]); // checksum (not checked)
            file.extend((offset as u32).to_be_bytes());
            file.extend((data.len() as u32).to_be_bytes());
            body.extend(data);
            while body.len() % 4 != 0 {
                body.push(0);
            }
            offset = 12 + 16 * tables.len() + body.len();
        }
        file.extend(body);
        file
    }

    /// A folder in the temp folder that is deleted when the value is dropped.
    struct TempFolder(PathBuf);

    impl TempFolder {
        fn new(name: &str, fonts: &[(&str, &str, u16)]) -> TempFolder {
            let dir = std::env::temp_dir().join("adapdf-tests").join(name);
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("sub")).unwrap();
            for (file, family, fs_type) in fonts {
                fs::write(dir.join(file), tiny_font(family, *fs_type)).unwrap();
            }
            TempFolder(dir)
        }

        fn fonts(&self) -> SystemFonts {
            SystemFonts::new(vec![self.0.clone()])
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reads_licence_flags_from_font_files() {
        let dir = TempFolder::new(
            "fonts-flags",
            &[
                ("a.ttf", "Open Test", 0),        // installable
                ("b.ttf", "Print Test", 0x0004),  // preview and print
                ("c.ttf", "Edit Test", 0x0008),   // editable
                ("d.ttf", "Locked Test", 0x0002), // restricted licence
                ("e.ttf", "Bitmap Test", 0x0200), // bitmap embedding only
            ],
        );
        let fonts = dir.fonts();
        assert_eq!(fonts.status("Open Test"), FontStatus::Embeddable);
        assert_eq!(fonts.status("Print Test"), FontStatus::Embeddable);
        assert_eq!(fonts.status("Edit Test"), FontStatus::Embeddable);
        assert_eq!(fonts.status("Locked Test"), FontStatus::Restricted);
        assert_eq!(fonts.status("Bitmap Test"), FontStatus::Restricted);
    }

    #[test]
    fn matches_names_without_regard_to_case_and_reports_missing_fonts() {
        let dir = TempFolder::new("fonts-names", &[("a.ttf", "Open Test", 0)]);
        let fonts = dir.fonts();
        assert_eq!(fonts.status("  open TEST "), FontStatus::Embeddable);
        assert_eq!(fonts.status("Not Here"), FontStatus::NotInstalled);
    }

    #[test]
    fn looks_in_sub_folders_and_skips_other_files() {
        let dir = TempFolder::new("fonts-sub", &[]);
        fs::write(dir.0.join("sub/deep.otf"), tiny_font("Deep Test", 0)).unwrap();
        fs::write(dir.0.join("notes.txt"), tiny_font("Text Test", 0)).unwrap();
        fs::write(dir.0.join("broken.ttf"), b"not a font").unwrap();
        let fonts = dir.fonts();
        assert_eq!(fonts.status("Deep Test"), FontStatus::Embeddable);
        assert_eq!(fonts.status("Text Test"), FontStatus::NotInstalled);
    }

    #[test]
    fn a_missing_folder_is_not_an_error() {
        let fonts = SystemFonts::new(vec![PathBuf::from("Z:/does/not/exist")]);
        assert_eq!(fonts.status("Arial"), FontStatus::NotInstalled);
    }

    #[test]
    fn one_restricted_face_makes_the_family_restricted() {
        let dir = TempFolder::new(
            "fonts-mixed",
            &[("a.ttf", "Mixed Test", 0), ("b.ttf", "Mixed Test", 0x0002)],
        );
        let fonts = dir.fonts();
        assert_eq!(fonts.status("Mixed Test"), FontStatus::Restricted);
    }
}
