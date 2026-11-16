//! The Word calls that stage 3 (the export) needs, on top of [`WordSession`]
//! and [`WordDocument`]: update the fields, call `ExportAsFixedFormat`, and end
//! a Word process that does not answer.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use serde::Serialize;
use windows::core::BSTR;

use super::{Result, WORD_EXE, WordDocument, WordSession, com};

/// The values that [`WordDocument::export_as_fixed_format`] passes to
/// `Document.ExportAsFixedFormat`: the table in spec 02, stage 3. The output
/// file name is the other parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportParameters {
    /// `wdExportFormatPDF`.
    pub export_format: i32,
    pub open_after_export: bool,
    /// `wdExportOptimizeForPrint`.
    pub optimize_for: i32,
    /// `wdExportDocumentContent`: no comments or markup.
    pub item: i32,
    pub include_doc_props: bool,
    /// `wdExportCreateHeadingBookmarks`.
    pub create_bookmarks: i32,
    /// The tags. Required.
    pub doc_structure_tags: bool,
    pub bitmap_missing_fonts: bool,
    /// PDF/A off. It is not needed for PDF/UA.
    pub use_iso19005_1: bool,
}

impl ExportParameters {
    /// The values of spec 02, stage 3.
    pub const SPEC: ExportParameters = ExportParameters {
        export_format: 17,
        open_after_export: false,
        optimize_for: 0,
        item: 0,
        include_doc_props: true,
        create_bookmarks: 1,
        doc_structure_tags: true,
        bitmap_missing_fonts: true,
        use_iso19005_1: false,
    };
}

/// What updating the fields of a document did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FieldUpdate {
    /// How many fields the main text has (a table of contents is one field).
    pub fields: u32,
    /// How many tables of contents the document has.
    pub tables_of_contents: u32,
    /// The number (from 1) of the first field that Word could not update.
    pub first_failed: Option<u32>,
}

impl WordDocument<'_> {
    /// Updates all fields of the main text (`Fields.Update`) and then every
    /// table of contents (`TablesOfContents(i).Update`, which also brings the
    /// page numbers up to date). The document is in memory only: it is never
    /// saved, and a read-only document can be updated.
    pub fn update_fields(&self) -> Result<FieldUpdate> {
        let fields = self.document().get_object("Fields")?;
        let count = fields.get_i32("Count")?;
        let failed = fields.call_i32("Update", Vec::new())?;
        let tables = self.document().get_object("TablesOfContents")?;
        let tables_count = tables.get_i32("Count")?;
        for n in 0..usize::try_from(tables_count).unwrap_or(0) {
            super::item(&tables, n)?.call("Update", Vec::new())?;
        }
        Ok(FieldUpdate {
            fields: u32::try_from(count).unwrap_or(0),
            tables_of_contents: u32::try_from(tables_count).unwrap_or(0),
            // `Update` gives 0 when every field was updated, else the number
            // of the first field that failed.
            first_failed: u32::try_from(failed).ok().filter(|n| *n != 0),
        })
    }

    /// Writes the document as a PDF with `Document.ExportAsFixedFormat`. Every
    /// parameter that the spec names is passed by name; the others keep Word's
    /// default (the whole document).
    pub fn export_as_fixed_format(
        &self,
        output: &Path,
        parameters: &ExportParameters,
    ) -> Result<()> {
        let output = std::path::absolute(output).unwrap_or_else(|_| output.to_path_buf());
        let output = BSTR::from_wide(&output.as_os_str().encode_wide().collect::<Vec<u16>>());
        let args = vec![
            ("OutputFileName", output.into()),
            ("ExportFormat", parameters.export_format.into()),
            ("OpenAfterExport", parameters.open_after_export.into()),
            ("OptimizeFor", parameters.optimize_for.into()),
            ("Item", parameters.item.into()),
            ("IncludeDocProps", parameters.include_doc_props.into()),
            ("CreateBookmarks", parameters.create_bookmarks.into()),
            ("DocStructureTags", parameters.doc_structure_tags.into()),
            ("BitmapMissingFonts", parameters.bitmap_missing_fonts.into()),
            ("UseISO19005_1", parameters.use_iso19005_1.into()),
        ];
        self.document().call("ExportAsFixedFormat", args)?;
        Ok(())
    }
}

/// Reads and edits that the export tests use to stand in for a person's Word:
/// a Word with a document that has an unsaved edit.
impl WordSession {
    /// Whether Word's window is visible. The tool's own Word never is.
    pub fn is_visible(&self) -> Result<bool> {
        Ok(self.app().get_i32("Visible")? != 0)
    }

    /// Word's `DisplayAlerts` (`wdAlertsNone` is 0). The tool's Word has no alerts.
    pub fn display_alerts(&self) -> Result<i32> {
        Ok(self.app().get_i32("DisplayAlerts")?)
    }

    /// How many documents this Word has open.
    pub fn document_count(&self) -> Result<u32> {
        let count = self.app().get_object("Documents")?.get_i32("Count")?;
        Ok(u32::try_from(count).unwrap_or(0))
    }
}

impl WordDocument<'_> {
    /// Whether the document has no unsaved change (Word's `Saved`).
    pub fn is_saved(&self) -> Result<bool> {
        Ok(self.document().get_i32("Saved")? != 0)
    }

    /// Adds text at the end of the document, in memory. The document is not
    /// saved. For tests that need a document with an unsaved edit.
    pub fn append_text(&self, text: &str) -> Result<()> {
        self.document()
            .get_object("Content")?
            .call("InsertAfter", vec![("Text", BSTR::from(text).into())])?;
        Ok(())
    }
}

/// Ends the process `pid` if it is a `WINWORD.EXE`, and gives `Ok(false)` if
/// it has already ended. A process with another image name is not ended and is
/// an error. The caller passes only the id that [`WordSession::pid`] gave: the
/// process that the tool started itself.
pub fn end_word_process(pid: u32) -> Result<bool> {
    Ok(com::end_process_named(pid, WORD_EXE)?)
}
