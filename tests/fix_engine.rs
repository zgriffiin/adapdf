//! The fix engine as a whole: only the spliced bytes change, the result is a
//! valid .docx, stage 1 runs again, the user picks the fixes, and ask fixes write
//! only what the user typed. Test plan rows S2-08 to S2-13 and S2-31.

mod fix_support;

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;

use adapdf::findings::{FixId, FixKind, RuleId};
use adapdf::fix::{self, FixInput, FixRequest};
use adapdf::fixtures::{DocxBuilder, Image, Para, Table, clean};
use adapdf::model::Document;
use fix_support::*;

/// Every rule that has a fix that writes, with the fixture that shows it.
const FIXED_RULES: &[RuleId] = &[
    RuleId::Doc001,
    RuleId::Doc002,
    RuleId::Hdg003,
    RuleId::Hdg004,
    RuleId::Lst001,
    RuleId::Img001,
    RuleId::Img002,
    RuleId::Img004,
    RuleId::Img007,
    RuleId::Txb004,
    RuleId::Tbl001,
    RuleId::Tbl005,
    RuleId::Tbl006,
    RuleId::Txt002,
    RuleId::Lnk004,
    RuleId::Lnk005,
];

const DOCUMENT: &str = "word/document.xml";

const DECORATIVE: &str = r#"<a:extLst><a:ext uri="{C183D7F6-B498-43B3-948B-1728B52AA6E4}"><adec:decorative xmlns:adec="http://schemas.microsoft.com/office/drawing/2017/decorative" val="1"/></a:ext></a:extLst>"#;

/// For each fix, the part that changes and every change in it, written by hand
/// from the failing fixture: the exact original text and the exact new text.
/// `fix_rule` types "Typed text" where a fix takes text. These do not come from
/// the splice list of the engine, so they can tell a fix that writes more than
/// it says.
fn expected_edits() -> Vec<(RuleId, &'static str, Vec<Edit>)> {
    let decorative = |id: u32| {
        let open = format!(r#"<wp:docPr id="{id}" name="Object 1">"#);
        edit(
            &format!("{open}</wp:docPr>"),
            &format!("{open}{DECORATIVE}</wp:docPr>"),
        )
    };
    let list_style = |before: &str, after: &str, text: &str| {
        edit(
            &format!(r#"<w:p><w:r><w:t xml:space="preserve">{before}{text}</w:t>"#),
            &format!(
                r#"<w:p><w:pPr><w:pStyle w:val="ListNumber"/></w:pPr><w:r><w:t xml:space="preserve">{after}{text}</w:t>"#
            ),
        )
    };
    vec![
        (
            RuleId::Doc001,
            "docProps/core.xml",
            vec![edit(
                "<dc:title></dc:title>",
                "<dc:title>Typed text</dc:title>",
            )],
        ),
        (
            RuleId::Doc002,
            "word/styles.xml",
            vec![edit(
                r#"<w:sz w:val="22"/></w:rPr></w:rPrDefault>"#,
                r#"<w:sz w:val="22"/><w:lang w:val="en-US"/></w:rPr></w:rPrDefault>"#,
            )],
        ),
        (
            RuleId::Hdg003,
            DOCUMENT,
            vec![edit(
                r#"body text.</w:t></w:r></w:p><w:p><w:r><w:rPr><w:b/>"#,
                r#"body text.</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:rPr><w:b/>"#,
            )],
        ),
        (
            RuleId::Hdg004,
            DOCUMENT,
            vec![edit(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t xml:space="preserve"></w:t></w:r></w:p>"#,
                "",
            )],
        ),
        (
            RuleId::Lst001,
            DOCUMENT,
            vec![
                list_style("1. ", "", "Open the file."),
                list_style("2. ", "", "Save the file."),
            ],
        ),
        (
            RuleId::Img001,
            DOCUMENT,
            vec![edit(
                r#"name="Object 1"></wp:docPr>"#,
                r#"name="Object 1" descr="Typed text"></wp:docPr>"#,
            )],
        ),
        (
            RuleId::Img002,
            DOCUMENT,
            vec![edit(r#"descr="IMG_1234.jpg""#, r#"descr="Typed text""#)],
        ),
        (
            RuleId::Img004,
            DOCUMENT,
            vec![
                edit(
                    r#"<wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="1" behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1">"#,
                    r#"<wp:inline distT="0" distB="0" distL="0" distR="0">"#,
                ),
                edit(r#"<wp:simplePos x="0" y="0"/>"#, ""),
                edit(
                    r#"<wp:positionH relativeFrom="column"><wp:posOffset>0</wp:posOffset></wp:positionH>"#,
                    "",
                ),
                edit(
                    r#"<wp:positionV relativeFrom="paragraph"><wp:posOffset>0</wp:posOffset></wp:positionV>"#,
                    "",
                ),
                edit(r#"<wp:wrapSquare wrapText="bothSides"/>"#, ""),
                edit("</wp:anchor>", "</wp:inline>"),
            ],
        ),
        (
            RuleId::Img007,
            DOCUMENT,
            vec![decorative(2), decorative(3), decorative(4)],
        ),
        (
            RuleId::Txb004,
            DOCUMENT,
            vec![edit(
                "First paragraph.</w:t></w:r></w:p><w:p></w:p>",
                "First paragraph.</w:t></w:r></w:p>",
            )],
        ),
        (
            RuleId::Tbl001,
            DOCUMENT,
            vec![edit(
                "</w:tblGrid><w:tr><w:trPr><w:cantSplit/>",
                "</w:tblGrid><w:tr><w:trPr><w:cantSplit/><w:tblHeader/>",
            )],
        ),
        (
            RuleId::Tbl005,
            DOCUMENT,
            vec![edit(
                concat!(
                    r#"A4</w:t></w:r></w:p></w:tc></w:tr>"#,
                    r#"<w:tr><w:trPr><w:cantSplit/></w:trPr><w:tc><w:tcPr><w:tcW w:w="4680" w:type="dxa"/></w:tcPr><w:p/></w:tc>"#,
                    r#"<w:tc><w:tcPr><w:tcW w:w="4680" w:type="dxa"/></w:tcPr><w:p/></w:tc></w:tr>"#
                ),
                "A4</w:t></w:r></w:p></w:tc></w:tr>",
            )],
        ),
        (
            RuleId::Tbl006,
            DOCUMENT,
            vec![
                edit(
                    "</w:tblGrid><w:tr><w:trPr><w:tblHeader/>",
                    "</w:tblGrid><w:tr><w:trPr><w:cantSplit/><w:tblHeader/>",
                ),
                edit(
                    "Value</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc>",
                    "Value</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:trPr><w:cantSplit/></w:trPr><w:tc>",
                ),
                edit(
                    "A4</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc>",
                    "A4</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:trPr><w:cantSplit/></w:trPr><w:tc>",
                ),
            ],
        ),
        (
            RuleId::Txt002,
            DOCUMENT,
            vec![edit(r#"<w:jc w:val="both"/>"#, r#"<w:jc w:val="left"/>"#)],
        ),
        (
            // The ScreenTip is the text of the link, and nothing else changes.
            RuleId::Lnk004,
            DOCUMENT,
            vec![edit(
                r#""><w:r><w:t xml:space="preserve">parking policy</w:t>"#,
                r#"" w:tooltip="parking policy"><w:r><w:t xml:space="preserve">parking policy</w:t>"#,
            )],
        ),
        (
            // Only ` \h` goes. The quote marks of the field keep the way the
            // file wrote them (`&quot;`).
            RuleId::Lnk005,
            DOCUMENT,
            vec![edit(
                r#"\o &quot;1-3&quot; \h \z \u </w:instrText>"#,
                r#"\o &quot;1-3&quot; \z \u </w:instrText>"#,
            )],
        ),
    ]
}

#[test]
fn fix_changes_only_the_spliced_bytes() {
    let table = expected_edits();
    let table_rules: Vec<RuleId> = table.iter().map(|(rule, ..)| *rule).collect();
    assert_eq!(table_rules, FIXED_RULES, "one row of edits for every fix");
    for (rule, part_name, edits) in &table {
        let original = fixture(*rule, true);
        let applied = fix_rule(&original, *rule);
        assert!(applied.refused.is_empty(), "{rule}: {:?}", applied.refused);
        assert!(!applied.applied.is_empty(), "{rule}");
        // The new part is the original with exactly the hand-written edits.
        assert_exactly_these_edits(&original, fixed(&applied), part_name, edits);
        // The splices the engine reports agree with that.
        assert_only_the_splices_changed(&original, &applied);
    }
}

#[test]
fn fix_changes_only_the_target_part() {
    let expected: BTreeMap<RuleId, &[&str]> = BTreeMap::from([
        (RuleId::Doc001, &["docProps/core.xml"][..]),
        (RuleId::Doc002, &["word/styles.xml"][..]),
        (RuleId::Hdg004, &["word/document.xml"][..]),
        (RuleId::Img001, &["word/document.xml"][..]),
        (RuleId::Txb004, &["word/document.xml"][..]),
        (RuleId::Tbl001, &["word/document.xml"][..]),
    ]);
    for (rule, parts) in expected {
        let applied = fix_rule(&fixture(rule, true), rule);
        let changed: Vec<&str> = applied.splices.keys().map(String::as_str).collect();
        assert_eq!(changed, parts, "{rule}");
    }
}

#[test]
fn fix_result_is_a_valid_docx() {
    for &rule in FIXED_RULES {
        let applied = fix_rule(&fixture(rule, true), rule);
        let bytes = fixed(&applied);
        for (name, data) in parts(bytes) {
            if name.ends_with(".xml") || name.ends_with(".rels") {
                roxmltree::Document::parse(std::str::from_utf8(&data).unwrap())
                    .unwrap_or_else(|e| panic!("{rule}: {name} is not XML: {e}"));
            }
        }
        Document::from_bytes(bytes).unwrap_or_else(|e| panic!("{rule}: {e}"));
    }
}

/// The rules of the findings that appear after a fix and not before.
fn new_rules(
    before: &[adapdf::findings::Finding],
    after: &[adapdf::findings::Finding],
) -> Vec<RuleId> {
    let mut remaining = rules(before);
    let mut added = Vec::new();
    for rule in rules(after) {
        match remaining.iter().position(|r| *r == rule) {
            Some(i) => {
                remaining.remove(i);
            }
            None => added.push(rule),
        }
    }
    added
}

#[test]
fn fix_reruns_stage_1_and_the_fixed_finding_is_gone() {
    for &rule in FIXED_RULES {
        let original = fixture(rule, true);
        let before = findings(&original);
        let applied = fix_rule(&original, rule);
        let after = findings(fixed(&applied));
        assert!(
            of(&after, rule).is_empty(),
            "{rule} is still there: {:#?}",
            of(&after, rule)
        );
        // No new finding appears. The rows of the plan that allow one are in
        // the tests of the fix itself.
        assert_eq!(new_rules(&before, &after), [], "{rule}: new findings");
    }
}

/// One document with every problem that has an auto fix.
fn every_auto_problem() -> DocxBuilder {
    clean()
        .no_language()
        .heading(2, "Overview")
        .paragraph(Para::new("Setting up the printer").bold())
        .paragraph(Para::new("1. Open the lid."))
        .paragraph(Para::new("2. Remove the tape."))
        .paragraph(Para::new("A justified paragraph.").justify())
        .paragraph(Para::new("See ").hyperlink("the parking policy", "https://example.org/p"))
        .heading(2, "")
        .paragraph(Para::empty())
        .paragraph(Para::empty())
        .paragraph(Para::empty())
        .paragraph(Para::new("Text after the gap."))
        .paragraph(Para::empty().raw_run(&Image::picture().alt("A printer").floating().xml()))
        .table(
            Table::new(&[
                &["Setting", "Value"],
                &["Paper", "A4"],
                &["", ""],
                &["Ink", "Black"],
            ])
            .no_header_row()
            .rows_can_split(),
        )
}

#[test]
fn fix_reruns_stage_1_with_every_auto_problem_in_one_document() {
    let original = build(&every_auto_problem());
    let before = findings(&original);
    let auto_rules: Vec<RuleId> = before
        .iter()
        .filter(|f| matches!(f.fix, FixKind::Auto(_)))
        .map(|f| f.rule_id)
        .collect();
    for rule in [
        RuleId::Doc002,
        RuleId::Hdg003,
        RuleId::Hdg004,
        RuleId::Lst001,
        RuleId::Img004,
        RuleId::Txb004,
        RuleId::Tbl001,
        RuleId::Tbl005,
        RuleId::Tbl006,
        RuleId::Txt002,
        RuleId::Txt004,
        RuleId::Lnk004,
    ] {
        // TXT-004 needs a table of contents. The others are in this document.
        if rule != RuleId::Txt004 {
            assert!(auto_rules.contains(&rule), "the document has no {rule}");
        }
    }
    let requests: Vec<FixRequest> = before
        .iter()
        .filter(|f| matches!(f.fix, FixKind::Auto(_)))
        .map(FixRequest::confirm)
        .collect();
    let applied = apply(&original, &requests);
    assert!(applied.refused.is_empty(), "{:#?}", applied.refused);
    assert_only_the_splices_changed(&original, &applied);
    let after = findings(fixed(&applied));
    let still_auto: Vec<RuleId> = after
        .iter()
        .filter(|f| matches!(f.fix, FixKind::Auto(_)))
        .map(|f| f.rule_id)
        .collect();
    assert_eq!(still_auto, [], "no auto finding is left");
    // The manual findings are the same rules and counts as before.
    let manual = |f: &[adapdf::findings::Finding]| {
        f.iter()
            .filter(|f| matches!(f.fix, FixKind::Manual))
            .map(|f| f.rule_id)
            .collect::<Vec<_>>()
    };
    let mut expected = manual(&before);
    // TBL-004 may appear when a table is left with one row (not the case here).
    expected.sort();
    let mut now = manual(&after);
    now.sort();
    assert_eq!(now, expected, "manual findings");
}

#[test]
fn fix_applies_only_the_fixes_the_user_picked() {
    let original = build(
        &clean()
            .paragraph(Para::new("A justified paragraph.").justify())
            .paragraph(Para::new("1. Open the lid."))
            .table(Table::new(&[&["A", "B"], &["C", "D"]]).rows_can_split()),
    );
    let before = findings(&original);
    for rule in [RuleId::Txt002, RuleId::Lst001, RuleId::Tbl006] {
        assert_eq!(of(&before, rule).len(), 1, "{rule}");
    }
    let applied = apply(&original, &requests_for(&before, RuleId::Lst001, ""));
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Lst001).is_empty());
    assert_eq!(
        of(&after, RuleId::Txt002).len(),
        1,
        "not picked, still there"
    );
    assert_eq!(
        of(&after, RuleId::Tbl006).len(),
        1,
        "not picked, still there"
    );
    assert_only_the_splices_changed(&original, &applied);
    // The paragraph with the justified text and the table are byte-equal.
    let (old, new) = (
        part(&original, "word/document.xml"),
        part(fixed(&applied), "word/document.xml"),
    );
    for (start, end) in [("<w:p><w:pPr><w:jc", "</w:p>"), ("<w:tbl>", "</w:tbl>")] {
        let from = old.find(start).unwrap();
        let to = from + old[from..].find(end).unwrap() + end.len();
        assert!(new.contains(&old[from..to]), "{start} is byte-equal");
    }
}

#[test]
fn ask_fix_without_text_writes_nothing() {
    let original = fixture(RuleId::Doc001, true);
    let found = findings(&original);
    let title = of(&found, RuleId::Doc001).remove(0);
    assert_eq!(title.suggestion.as_deref(), Some("Test guide"));
    let picture =
        build(&DocxBuilder::new().paragraph(Para::empty().raw_run(&Image::picture().xml())));
    let alt = of(&findings(&picture), RuleId::Img001).remove(0);
    let annotated = fixture(RuleId::Img007, true);
    let marks = of(&findings(&annotated), RuleId::Img007).remove(0);

    let cases = [
        (&original, &title, FixInput::Text(String::new())),
        (&original, &title, FixInput::Text("   \t ".to_string())),
        (&original, &title, FixInput::Confirm),
        (&picture, &alt, FixInput::Text(String::new())),
        (&picture, &alt, FixInput::Text("  ".to_string())),
        (&picture, &alt, FixInput::Confirm),
        (&annotated, &marks, FixInput::Text(String::new())),
        (&annotated, &marks, FixInput::Decorative),
    ];
    for (bytes, finding, input) in cases {
        let request = FixRequest {
            finding: finding.clone(),
            input: input.clone(),
            position: None,
        };
        let applied = apply(bytes, &[request]);
        assert!(
            applied.bytes.is_none(),
            "{} with {input:?} wrote a file",
            finding.rule_id
        );
        assert!(applied.applied.is_empty());
        assert_eq!(
            applied.refused.len(),
            1,
            "{} with {input:?}",
            finding.rule_id
        );
        assert!(applied.splices.is_empty());
    }

    // On disk: no write and no backup.
    let dir = TempDir::new("ask");
    let path = dir.write("Guide.docx", &original);
    let outcome = fix_file(&path, &[FixRequest::text(&title, " ")]).expect("the run works");
    assert!(outcome.backup.is_none());
    assert_eq!(outcome.refused.len(), 1);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide.docx"]);
    assert!(
        outcome.findings.iter().any(|f| f.rule_id == RuleId::Doc001),
        "the finding remains"
    );
}

#[test]
fn ask_fix_writes_exactly_the_typed_text() {
    // The title.
    let original = fixture(RuleId::Doc001, true);
    let title = of(&findings(&original), RuleId::Doc001).remove(0);
    let applied = apply(&original, &[FixRequest::text(&title, AWKWARD_TEXT)]);
    let core = part(fixed(&applied), "docProps/core.xml");
    assert!(
        core.contains("<dc:title>A &amp; B &lt;C&gt; \"D\" é 日本</dc:title>"),
        "{core}"
    );
    assert_eq!(
        Document::from_bytes(fixed(&applied))
            .unwrap()
            .core
            .title
            .as_deref(),
        Some(AWKWARD_TEXT)
    );
    let inserted: Vec<&str> = applied.splices["docProps/core.xml"]
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(
        inserted,
        ["A &amp; B &lt;C&gt; \"D\" é 日本"],
        "nothing else is added"
    );

    // The alt text.
    let picture = fixture(RuleId::Img001, true);
    let alt = of(&findings(&picture), RuleId::Img001).remove(0);
    let applied = apply(&picture, &[FixRequest::text(&alt, AWKWARD_TEXT)]);
    let xml = part(fixed(&applied), "word/document.xml");
    assert!(
        xml.contains(r#"descr="A &amp; B &lt;C&gt; &quot;D&quot; é 日本""#),
        "{xml}"
    );
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    let drawing = model
        .paragraphs
        .iter()
        .flat_map(|p| &p.drawings)
        .next()
        .unwrap();
    assert_eq!(drawing.description.as_deref(), Some(AWKWARD_TEXT));
    let inserted: Vec<&str> = applied.splices["word/document.xml"]
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(
        inserted,
        [r#" descr="A &amp; B &lt;C&gt; &quot;D&quot; é 日本""#]
    );

    // Line breaks and tabs in an attribute survive.
    let applied = apply(
        &picture,
        &[FixRequest::text(&alt, "Line one\nLine two\tend")],
    );
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    let drawing = model
        .paragraphs
        .iter()
        .flat_map(|p| &p.drawings)
        .next()
        .unwrap();
    assert_eq!(
        drawing.description.as_deref(),
        Some("Line one\nLine two\tend")
    );
}

#[test]
fn fix_update_fields_is_done_at_export() {
    let original = fixture(RuleId::Txt004, true);
    let found = of(&findings(&original), RuleId::Txt004);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fix, FixKind::Auto(FixId::UpdateFields));
    let applied = apply(&original, &[FixRequest::confirm(&found[0])]);
    assert!(
        applied.bytes.is_none(),
        "the fix writes nothing to the .docx"
    );
    assert!(applied.applied.is_empty());
    assert_eq!(applied.at_export, found, "it is listed as done at export");
    assert!(applied.refused.is_empty());

    let dir = TempDir::new("update-fields");
    let path = dir.write("Guide.docx", &original);
    let outcome = fix_file(&path, &[FixRequest::confirm(&found[0])]).expect("the run works");
    assert!(outcome.backup.is_none(), "no backup is made");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(dir.names(), ["Guide.docx"]);

    // The offer says "done at export".
    let offers = fix::offers(&original, &options(), &adapdf::fixtures::FixtureFonts).unwrap();
    let offer = offers
        .iter()
        .find(|o| o.fix == FixId::UpdateFields)
        .unwrap();
    assert_eq!(offer.availability, fix::Availability::AtExport);
}

#[test]
fn the_same_text_cannot_be_changed_by_two_different_fixes() {
    // An empty heading that is also in a row of empty paragraphs: both fixes
    // delete the same paragraph. The same deletion counts once.
    let original = build(
        &clean()
            .paragraph(Para::new("Before."))
            .heading(2, "")
            .paragraph(Para::empty())
            .paragraph(Para::empty())
            .paragraph(Para::new("After.")),
    );
    let found = findings(&original);
    let mut requests = requests_for(&found, RuleId::Hdg004, "");
    requests.extend(requests_for(&found, RuleId::Txb004, ""));
    assert_eq!(requests.len(), 2, "{found:#?}");
    let applied = apply(&original, &requests);
    assert_only_the_splices_changed(&original, &applied);
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Hdg004).is_empty());
    assert!(of(&after, RuleId::Txb004).is_empty());
}

#[test]
fn a_finding_from_another_version_of_the_file_is_refused() {
    let original = fixture(RuleId::Txt002, true);
    let found = of(&findings(&original), RuleId::Txt002).remove(0);
    let other = fixture(RuleId::Txt002, false);
    let applied = apply(&other, &[FixRequest::confirm(&found)]);
    assert!(applied.bytes.is_none());
    assert_eq!(applied.refused.len(), 1);
    assert!(
        applied.refused[0].reason.contains("changed"),
        "{}",
        applied.refused[0].reason
    );
}

#[test]
fn a_manual_finding_has_no_fix() {
    let original = fixture(RuleId::Hdg002, true);
    let found = of(&findings(&original), RuleId::Hdg002).remove(0);
    let applied = apply(&original, &[FixRequest::confirm(&found)]);
    assert!(applied.bytes.is_none());
    assert!(applied.refused[0].reason.contains("Fix it in Word"));
}

/// Entries that did not change are copied raw: the compressed bytes, the
/// method and the CRC stay the same.
#[test]
fn untouched_entries_are_copied_raw() {
    let original = fixture(RuleId::Txt002, true);
    // The same document with every entry compressed.
    let compressed = {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in parts(&original) {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer.start_file(name, options).unwrap();
            std::io::Write::write_all(&mut writer, &data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    };
    let applied = fix_rule(&compressed, RuleId::Txt002);
    let new = fixed(&applied);
    let raw = |bytes: &[u8]| -> Vec<(String, zip::CompressionMethod, u32, Vec<u8>)> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|i| {
                let mut entry = archive.by_index_raw(i).unwrap();
                let mut data = Vec::new();
                entry.read_to_end(&mut data).unwrap();
                (
                    entry.name().unwrap().into_owned(),
                    entry.compression(),
                    entry.crc32(),
                    data,
                )
            })
            .collect()
    };
    let (before, after) = (raw(&compressed), raw(new));
    assert_eq!(before.len(), after.len());
    for (old, now) in before.iter().zip(&after) {
        assert_eq!(old.0, now.0);
        assert_eq!(old.1, now.1, "{} compression method", old.0);
        if old.0 == "word/document.xml" {
            assert_ne!(old.3, now.3);
        } else {
            assert_eq!(old, now, "{} must be copied raw", old.0);
        }
    }
    assert_only_the_splices_changed(&compressed, &applied);
}

#[test]
fn a_byte_order_mark_is_kept() {
    let original = replace_part(&fixture(RuleId::Txt002, true), "word/document.xml", |t| {
        format!("\u{FEFF}{t}")
    });
    let applied = fix_rule(&original, RuleId::Txt002);
    assert_only_the_splices_changed(&original, &applied);
    assert!(
        parts(fixed(&applied))
            .iter()
            .find(|(n, _)| n == "word/document.xml")
            .unwrap()
            .1
            .starts_with(BOM)
    );
}
