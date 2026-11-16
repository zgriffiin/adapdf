//! Stage 4: PDF metadata fixes (rows S4-01 to S4-09 and S4-12 of the test plan).

mod pdf_support;

use std::collections::BTreeSet;

use adapdf::findings::Severity;
use adapdf::pdfmeta::{self, Change, Field, IssueKind, Outcome, PdfMetaError, SourceMetadata};
use lopdf::{Document, Object};
use pdf_support::{PART_1, PART_2, Pdf, TITLE, TempDir, layouts, seen, xmp_packet, xmp_title};

fn source() -> SourceMetadata {
    SourceMetadata {
        title: Some(TITLE.to_string()),
        language: Some("en-US".to_string()),
    }
}

fn run(pdf: &[u8]) -> Outcome {
    pdfmeta::check_bytes(pdf, &source()).expect("stage 4 runs")
}

fn fields(outcome: &Outcome) -> Vec<Field> {
    outcome.report.changes.iter().map(|c| c.field).collect()
}

fn startxref_offset(bytes: &[u8]) -> i64 {
    let text = String::from_utf8_lossy(bytes);
    let tail = &text[text.rfind("startxref").expect("startxref")..];
    tail.split_whitespace()
        .nth(1)
        .expect("offset")
        .parse()
        .expect("number")
}

// S4-01
#[test]
fn pdfmeta_output_starts_with_the_original_bytes() {
    for (name, original) in layouts() {
        let outcome = run(&original);
        assert!(!outcome.appended.is_empty(), "{name}: something is missing");
        let output = outcome.apply(&original);
        assert!(output.starts_with(&original), "{name}: prefix");
        assert!(output.len() > original.len(), "{name}: longer");
        assert_eq!(&output[original.len()..], &outcome.appended[..], "{name}");

        // The new tail has its own cross-reference section, and /Prev points to
        // the section the file had before.
        let before = Document::load_mem(&original).unwrap();
        let after = Document::load_mem(&output).expect(name);
        assert_eq!(
            after.xref_section_count,
            before.xref_section_count + 1,
            "{name}: one more cross-reference section"
        );
        assert!(
            after.xref_recovery.is_none(),
            "{name}: no repair was needed"
        );
        let tail = String::from_utf8_lossy(&outcome.appended).into_owned();
        let prev = tail.split("/Prev").nth(1).expect("the tail has /Prev");
        let prev: i64 = prev
            .split(|c: char| !c.is_ascii_digit() && c != ' ')
            .next()
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            prev,
            startxref_offset(&original),
            "{name}: /Prev is the old startxref"
        );
        // The old section is where /Prev says: a table or a stream.
        let old_section = &original[prev as usize..];
        assert!(
            old_section.starts_with(b"xref") || old_section.iter().take(24).any(|b| *b == b'o'),
            "{name}"
        );
        // The new startxref points into the appended bytes.
        assert!(
            startxref_offset(&output) as usize >= original.len(),
            "{name}"
        );
        assert!(output.ends_with(b"%%EOF\n"), "{name}");
    }
}

#[test]
fn a_second_run_after_a_fix_changes_nothing() {
    for (name, original) in layouts() {
        let output = run(&original).apply(&original);
        let again = run(&output);
        assert!(again.appended.is_empty(), "{name}: nothing is left to fix");
        assert!(again.report.changes.is_empty(), "{name}");
        assert!(!again.report.has_errors(), "{name}");
    }
}

#[test]
fn a_file_without_a_final_line_break_still_gets_a_clean_update() {
    let mut original = Pdf::all_missing().build();
    while original.last().is_some_and(u8::is_ascii_whitespace) {
        original.pop();
    }
    assert!(original.ends_with(b"%%EOF"));
    let outcome = run(&original);
    let output = outcome.apply(&original);
    assert!(output.starts_with(&original));
    assert_eq!(output[original.len()], b'\n');
    Document::load_mem(&output).unwrap();
}

// S4-02
#[test]
fn pdfmeta_adds_pdfuaid_part_1() {
    let original = Pdf::all_missing().build();
    let outcome = run(&original);
    assert!(fields(&outcome).contains(&Field::XmpPdfUaPart));
    let xmp = seen(&outcome.apply(&original)).xmp.unwrap();
    assert!(xmp.contains("<pdfuaid:part>1</pdfuaid:part>"));
    // Other XMP content is kept.
    assert!(xmp.contains("<xmp:CreatorTool>Adapdf test maker</xmp:CreatorTool>"));
    assert!(xmp.starts_with("<?xpacket begin="));
    assert!(xmp.ends_with("<?xpacket end=\"w\"?>"));

    // Part 1 is already there: no change for it.
    let has_part = Pdf {
        xmp: Some(xmp_packet(PART_1)),
        ..Pdf::all_missing()
    }
    .build();
    assert!(!fields(&run(&has_part)).contains(&Field::XmpPdfUaPart));

    // Part 2: an error, and the value is not changed.
    let part_2 = Pdf {
        xmp: Some(xmp_packet(PART_2)),
        ..Pdf::all_missing()
    }
    .build();
    let outcome = run(&part_2);
    assert!(!fields(&outcome).contains(&Field::XmpPdfUaPart));
    assert!(outcome.report.has_errors());
    assert!(
        outcome
            .report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::WrongPdfUaPart && i.severity == Severity::Error)
    );
    let xmp = seen(&outcome.apply(&part_2)).xmp.unwrap();
    assert!(xmp.contains("<pdfuaid:part>2</pdfuaid:part>"));
    assert!(!xmp.contains("<pdfuaid:part>1</pdfuaid:part>"));
}

#[test]
fn a_missing_xmp_stream_is_made() {
    let original = Pdf {
        info_without_title: true,
        ..Pdf::default()
    }
    .build();
    let outcome = run(&original);
    let output = outcome.apply(&original);
    let xmp = seen(&output).xmp.expect("a /Metadata stream was added");
    assert!(xmp.contains("<pdfuaid:part>1</pdfuaid:part>"));
    assert!(xmp.contains(TITLE));
    // The catalog and the metadata stream are new, and no other object is.
    assert_eq!(outcome.written_objects.len(), 3); // metadata, Info, catalog
}

// S4-03
#[test]
fn pdfmeta_copies_the_title() {
    let original = Pdf::all_missing().build();
    let outcome = run(&original);
    let output = outcome.apply(&original);
    let now = seen(&output);
    assert_eq!(now.info_title.as_deref(), Some(TITLE));
    assert!(now.xmp.unwrap().contains(TITLE));
    assert!(fields(&outcome).contains(&Field::XmpTitle));
    assert!(fields(&outcome).contains(&Field::InfoTitle));

    // Only one of the two is missing: only that one is added.
    let only_info_missing = Pdf {
        xmp: Some(xmp_packet(&format!(
            "{PART_1}{}",
            xmp_title("Existing title")
        ))),
        info_without_title: true,
        ..Pdf::default()
    }
    .build();
    let outcome = run(&only_info_missing);
    assert!(fields(&outcome).contains(&Field::InfoTitle));
    assert!(!fields(&outcome).contains(&Field::XmpTitle));
    let now = seen(&outcome.apply(&only_info_missing));
    assert_eq!(now.info_title.as_deref(), Some(TITLE));
    assert!(now.xmp.unwrap().contains("Existing title"));

    let only_xmp_missing = Pdf {
        xmp: Some(xmp_packet(PART_1)),
        info_title: Some("Existing info title".to_string()),
        ..Pdf::default()
    }
    .build();
    let outcome = run(&only_xmp_missing);
    assert!(fields(&outcome).contains(&Field::XmpTitle));
    assert!(!fields(&outcome).contains(&Field::InfoTitle));
    let now = seen(&outcome.apply(&only_xmp_missing));
    assert_eq!(now.info_title.as_deref(), Some("Existing info title"));
    assert!(now.xmp.unwrap().contains(TITLE));
}

#[test]
fn an_empty_title_in_the_pdf_counts_as_missing() {
    let original = Pdf {
        xmp: Some(xmp_packet(&format!("{PART_1}{}", xmp_title("  ")))),
        info_title: Some(String::new()),
        ..Pdf::default()
    }
    .build();
    let outcome = run(&original);
    assert!(fields(&outcome).contains(&Field::XmpTitle));
    assert!(fields(&outcome).contains(&Field::InfoTitle));
    let xmp = seen(&outcome.apply(&original)).xmp.unwrap();
    assert_eq!(xmp.matches("<dc:title").count(), 1);
}

#[test]
fn a_title_with_other_letters_is_written_as_utf16() {
    let source = SourceMetadata {
        title: Some("Guía – Año 2026 & <más>".to_string()),
        language: Some("es-US".to_string()),
    };
    let original = Pdf::all_missing().build();
    let outcome = pdfmeta::check_bytes(&original, &source).unwrap();
    let output = outcome.apply(&original);
    let doc = Document::load_mem(&output).unwrap();
    let info = doc
        .get_dictionary(doc.trailer.get(b"Info").unwrap().as_reference().unwrap())
        .unwrap();
    let bytes = info.get(b"Title").unwrap().as_str().unwrap();
    assert_eq!(&bytes[..2], &[0xFE, 0xFF]);
    let units: Vec<u16> = bytes[2..]
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    assert_eq!(
        String::from_utf16(&units).unwrap(),
        "Guía – Año 2026 & <más>"
    );
    // The XMP is still well-formed XML and has the title with its markup escaped.
    let xmp = seen(&output).xmp.unwrap();
    roxmltree::Document::parse(&xmp).expect("the XMP is XML");
    assert!(xmp.contains("Guía – Año 2026 &amp; &lt;más&gt;"));
}

#[test]
fn no_title_in_the_docx_is_an_error_and_nothing_is_invented() {
    let original = Pdf::all_missing().build();
    let source = SourceMetadata {
        title: Some("   ".to_string()),
        language: Some("en-US".to_string()),
    };
    let outcome = pdfmeta::check_bytes(&original, &source).unwrap();
    assert!(!fields(&outcome).contains(&Field::XmpTitle));
    assert!(!fields(&outcome).contains(&Field::InfoTitle));
    assert!(
        outcome
            .report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::NoTitleToCopy && i.severity == Severity::Error)
    );
    assert!(outcome.report.has_errors());
    let now = seen(&outcome.apply(&original));
    assert_eq!(now.info_title, None);
    assert!(!now.xmp.unwrap().contains("dc:title"));
}

// S4-04
#[test]
fn pdfmeta_sets_display_doc_title() {
    // No ViewerPreferences: a direct dictionary is added to the catalog.
    let original = Pdf::all_missing().build();
    let outcome = run(&original);
    assert!(fields(&outcome).contains(&Field::DisplayDocTitle));
    assert_eq!(
        seen(&outcome.apply(&original)).display_doc_title,
        Some(true)
    );

    // False, with another key and an indirect object: the other key is kept.
    let original = Pdf {
        display_doc_title: Some(false),
        prefs_indirect: true,
        prefs_other_key: true,
        ..Pdf::all_missing()
    }
    .build();
    let outcome = run(&original);
    let change = outcome
        .report
        .changes
        .iter()
        .find(|c| c.field == Field::DisplayDocTitle)
        .unwrap();
    assert_eq!(change.old.as_deref(), Some("false"));
    assert_eq!(change.new, "true");
    let output = outcome.apply(&original);
    assert_eq!(seen(&output).display_doc_title, Some(true));
    let doc = Document::load_mem(&output).unwrap();
    let prefs = doc
        .catalog()
        .unwrap()
        .get_deref(b"ViewerPreferences", &doc)
        .unwrap()
        .as_dict()
        .unwrap();
    assert!(prefs.get(b"HideToolbar").unwrap().as_bool().unwrap());

    // Direct dictionary with another key.
    let original = Pdf {
        display_doc_title: Some(false),
        prefs_other_key: true,
        ..Pdf::all_missing()
    }
    .build();
    let output = run(&original).apply(&original);
    let doc = Document::load_mem(&output).unwrap();
    let prefs = doc
        .catalog()
        .unwrap()
        .get_deref(b"ViewerPreferences", &doc)
        .unwrap()
        .as_dict()
        .unwrap();
    assert!(prefs.has(b"HideToolbar"));
    assert!(prefs.get(b"DisplayDocTitle").unwrap().as_bool().unwrap());

    // Already true: no change.
    let original = Pdf {
        display_doc_title: Some(true),
        ..Pdf::all_missing()
    }
    .build();
    assert!(!fields(&run(&original)).contains(&Field::DisplayDocTitle));
}

// S4-05
#[test]
fn pdfmeta_copies_the_language() {
    let original = Pdf::all_missing().build();
    let outcome = run(&original);
    assert!(fields(&outcome).contains(&Field::Lang));
    assert_eq!(
        seen(&outcome.apply(&original)).lang.as_deref(),
        Some("en-US")
    );

    // An existing value is not changed, even when it differs.
    let original = Pdf {
        lang: Some("fr-CA".to_string()),
        ..Pdf::all_missing()
    }
    .build();
    let outcome = run(&original);
    assert!(!fields(&outcome).contains(&Field::Lang));
    assert_eq!(
        seen(&outcome.apply(&original)).lang.as_deref(),
        Some("fr-CA")
    );
}

#[test]
fn no_language_in_the_docx_is_an_error_and_nothing_is_invented() {
    let original = Pdf::all_missing().build();
    let source = SourceMetadata {
        title: Some(TITLE.to_string()),
        language: None,
    };
    let outcome = pdfmeta::check_bytes(&original, &source).unwrap();
    assert!(
        outcome
            .report
            .issues
            .iter()
            .any(|i| i.kind == IssueKind::NoLanguageToCopy)
    );
    assert_eq!(seen(&outcome.apply(&original)).lang, None);
}

// S4-06
#[test]
fn pdfmeta_reports_missing_markinfo_and_structtreeroot() {
    for (name, pdf, kinds) in [
        (
            "no-markinfo",
            Pdf {
                marked: false,
                ..Pdf::all_missing()
            },
            vec![IssueKind::NotMarked],
        ),
        (
            "no-structtreeroot",
            Pdf {
                struct_tree: false,
                ..Pdf::all_missing()
            },
            vec![IssueKind::NoStructTreeRoot],
        ),
        (
            "untagged",
            Pdf {
                marked: false,
                struct_tree: false,
                ..Pdf::all_missing()
            },
            vec![IssueKind::NotMarked, IssueKind::NoStructTreeRoot],
        ),
    ] {
        let original = pdf.build();
        let outcome = run(&original);
        let found: Vec<IssueKind> = outcome.report.issues.iter().map(|i| i.kind).collect();
        assert_eq!(found, kinds, "{name}");
        assert!(
            outcome
                .report
                .issues
                .iter()
                .all(|i| i.severity == Severity::Error
                    && !i.message.is_empty()
                    && !i.how_to_fix.is_empty()),
            "{name}"
        );
        assert!(outcome.report.has_errors(), "{name}");
        // The export failed: nothing is appended, so no /MarkInfo or
        // /StructTreeRoot object is written, and no other fix is either.
        assert!(outcome.appended.is_empty(), "{name}");
        assert!(outcome.report.changes.is_empty(), "{name}");
        assert_eq!(outcome.apply(&original), original, "{name}");
    }
}

#[test]
fn marked_false_is_not_marked() {
    use lopdf::dictionary;
    let original = Pdf::all_missing().build();
    let mut doc = Document::load_mem(&original).unwrap();
    let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    if let Ok(Object::Dictionary(catalog)) = doc.get_object_mut(catalog_id) {
        catalog.set("MarkInfo", dictionary! { "Marked" => false });
    }
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let outcome = run(&bytes);
    assert_eq!(outcome.report.issues[0].kind, IssueKind::NotMarked);
}

// S4-07
#[test]
fn pdfmeta_does_not_touch_content_or_tags() {
    for (name, original) in layouts() {
        let before = Document::load_mem(&original).unwrap();
        let outcome = run(&original);
        let output = outcome.apply(&original);
        let after = Document::load_mem(&output).unwrap();

        // The objects in the update are only the catalog, the Info dictionary,
        // the metadata stream and (when it is indirect) ViewerPreferences.
        let catalog_id = before.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let info_id = before.trailer.get(b"Info").unwrap().as_reference().unwrap();
        let metadata_id = before
            .catalog()
            .unwrap()
            .get(b"Metadata")
            .unwrap()
            .as_reference()
            .unwrap();
        let allowed: BTreeSet<_> = [catalog_id, info_id, metadata_id].into_iter().collect();
        let written: BTreeSet<_> = outcome.written_objects.iter().copied().collect();
        assert!(written.is_subset(&allowed), "{name}: {written:?}");

        // Every other object is the same as before, and the catalog still
        // points to the same pages and structure tree.
        for (id, object) in &before.objects {
            if !allowed.contains(id) {
                assert_eq!(after.objects.get(id), Some(object), "{name}: {id:?}");
            }
        }
        for key in [&b"Pages"[..], b"StructTreeRoot", b"MarkInfo"] {
            assert_eq!(
                before.catalog().unwrap().get(key).unwrap(),
                after.catalog().unwrap().get(key).unwrap(),
                "{name}: {}",
                String::from_utf8_lossy(key)
            );
        }
        // The appended bytes do not hold any content stream, page, font or
        // structure element.
        let tail = String::from_utf8_lossy(&outcome.appended);
        for word in [
            "/Kids",
            "/MediaBox",
            "/StructElem",
            "/Font",
            "BT",
            "/Contents",
        ] {
            assert!(!tail.contains(word), "{name}: the update has {word}");
        }
    }
}

// S4-08
#[test]
fn pdfmeta_no_change_when_nothing_is_missing() {
    for modern in [false, true] {
        let original = Pdf {
            modern,
            ..Pdf::complete()
        }
        .build();
        let outcome = run(&original);
        assert!(outcome.appended.is_empty());
        assert!(outcome.written_objects.is_empty());
        assert!(outcome.report.changes.is_empty());
        assert!(outcome.report.issues.is_empty());
        assert_eq!(outcome.apply(&original), original);
    }
}

#[test]
fn fix_file_does_not_write_when_nothing_is_missing() {
    let dir = TempDir::new("complete");
    let original = Pdf::complete().build();
    let path = dir.write("Guide.pdf", &original);
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let report = pdfmeta::fix_file(&path, &source()).unwrap();
    assert!(report.changes.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
    assert_eq!(dir.names(), vec!["Guide.pdf"]);
}

#[test]
fn fix_file_writes_through_a_temporary_file() {
    let dir = TempDir::new("write");
    let original = Pdf::all_missing().build();
    let path = dir.write("Guide.pdf", &original);
    let report = pdfmeta::fix_file(&path, &source()).unwrap();
    assert_eq!(report.changes.len(), 5);
    let written = std::fs::read(&path).unwrap();
    assert!(written.starts_with(&original));
    assert!(written.len() > original.len());
    // The temporary file is gone.
    assert_eq!(dir.names(), vec!["Guide.pdf"]);
    // The next run finds nothing to do.
    let report = pdfmeta::fix_file(&path, &source()).unwrap();
    assert!(report.changes.is_empty());
}

// S4-09
#[test]
fn pdfmeta_changes_are_listed() {
    let original = Pdf::all_missing().build();
    let outcome = run(&original);
    let expected = vec![
        Field::XmpPdfUaPart,
        Field::XmpTitle,
        Field::InfoTitle,
        Field::DisplayDocTitle,
        Field::Lang,
    ];
    assert_eq!(fields(&outcome), expected);
    for change in &outcome.report.changes {
        assert!(!change.what.is_empty());
        assert_eq!(change.old, None, "{:?}", change.field);
    }
    let by_field = |f: Field| -> &Change {
        outcome
            .report
            .changes
            .iter()
            .find(|c| c.field == f)
            .unwrap()
    };
    assert_eq!(by_field(Field::XmpPdfUaPart).new, "1");
    assert_eq!(by_field(Field::XmpTitle).new, TITLE);
    assert_eq!(by_field(Field::InfoTitle).new, TITLE);
    assert_eq!(by_field(Field::DisplayDocTitle).new, "true");
    assert_eq!(by_field(Field::Lang).new, "en-US");

    // The list matches the differences between the two files: each value
    // before and after.
    let before = seen(&original);
    let after = seen(&outcome.apply(&original));
    assert_eq!(before.info_title, None);
    assert_eq!(after.info_title.as_deref(), Some(TITLE));
    assert_eq!(before.lang, None);
    assert_eq!(after.lang.as_deref(), Some("en-US"));
    assert_eq!(before.display_doc_title, None);
    assert_eq!(after.display_doc_title, Some(true));
    assert!(!before.xmp.unwrap().contains("pdfuaid"));
    assert!(after.xmp.unwrap().contains("pdfuaid"));

    // Complete: no changes. Missing one value: one change.
    let one_missing = Pdf {
        lang: None,
        ..Pdf::complete()
    }
    .build();
    assert_eq!(fields(&run(&one_missing)), vec![Field::Lang]);
}

#[test]
fn the_report_serialises_with_stable_names() {
    let original = Pdf::all_missing().build();
    let json = serde_json::to_value(&run(&original).report).unwrap();
    assert_eq!(json["changes"][0]["field"], "xmp-pdfuaid-part");
    assert_eq!(json["changes"][0]["new"], "1");
    assert!(json["issues"].as_array().unwrap().is_empty());
}

// S4-12
#[test]
fn pdfmeta_reports_a_pdf_it_cannot_read() {
    let original = Pdf::all_missing().build();
    let dir = TempDir::new("unreadable");

    // A truncated PDF.
    let truncated = original[..original.len() / 2].to_vec();
    let path = dir.write("truncated.pdf", &truncated);
    let error = pdfmeta::fix_file(&path, &source()).unwrap_err();
    assert!(matches!(error, PdfMetaError::Unreadable(_)), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), truncated);

    // Not a PDF at all.
    let path = dir.write("text.pdf", b"This is not a PDF.");
    assert!(pdfmeta::fix_file(&path, &source()).is_err());

    // A missing file.
    let error = pdfmeta::fix_file(&dir.path().join("missing.pdf"), &source()).unwrap_err();
    assert!(matches!(error, PdfMetaError::Io(_)));
}

#[test]
fn encrypted_pdfs_are_refused() {
    use lopdf::{EncryptionState, EncryptionVersion, Permissions};
    let dir = TempDir::new("encrypted");
    for (name, user_password) in [("open", ""), ("locked", "secret")] {
        let original = Pdf::all_missing().build();
        let mut doc = Document::load_mem(&original).unwrap();
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &doc,
            owner_password: "owner",
            user_password,
            key_length: 128,
            permissions: Permissions::all(),
        })
        .unwrap();
        doc.encrypt(&state).unwrap();
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        let path = dir.write(&format!("{name}.pdf"), &bytes);
        let error = pdfmeta::fix_file(&path, &source()).unwrap_err();
        assert!(matches!(error, PdfMetaError::Encrypted), "{name}: {error}");
        assert!(error.to_string().contains("encrypted"));
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "{name}: unchanged");
    }
}

#[test]
fn a_damaged_cross_reference_table_is_not_repaired_silently() {
    let original = Pdf::all_missing().build();
    let mut damaged = original.clone();
    let at = damaged.windows(9).rposition(|w| w == b"startxref").unwrap();
    // Point startxref to the wrong place.
    let text = String::from_utf8_lossy(&damaged[at..]).into_owned();
    let new_tail = text.replacen(&startxref_offset(&original).to_string(), "5", 1);
    damaged.truncate(at);
    damaged.extend_from_slice(new_tail.as_bytes());
    let error = pdfmeta::check_bytes(&damaged, &source()).unwrap_err();
    assert!(matches!(error, PdfMetaError::Unreadable(_)), "{error}");
}

/// The title and the language come from the .docx model, as `export` will pass
/// them.
#[test]
fn the_source_metadata_comes_from_the_docx() {
    use adapdf::fixtures::DocxBuilder;
    use adapdf::model::Document as Docx;

    let bytes = DocxBuilder::new()
        .title("Guide to parking permits")
        .language("es-US")
        .build();
    let docx = Docx::from_bytes(&bytes).unwrap();
    assert_eq!(
        SourceMetadata::from_document(&docx),
        SourceMetadata {
            title: Some("Guide to parking permits".to_string()),
            language: Some("es-US".to_string()),
        }
    );

    let bytes = DocxBuilder::new().no_language().build();
    let docx = Docx::from_bytes(&bytes).unwrap();
    let source = SourceMetadata::from_document(&docx);
    assert_eq!(source.title, None);
    assert_eq!(source.language, None);
}
