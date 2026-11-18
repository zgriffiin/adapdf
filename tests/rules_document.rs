//! DOC-001 to DOC-006.

mod common;

use adapdf::findings::{FixId, FixKind, RuleId, Severity};
use adapdf::fixtures::{DocxBuilder, Para, Run, clean};
use adapdf::fonts::{FontLookup, FontStatus};
use adapdf::rules::document::{MIN_FOREIGN_SCRIPT_LETTERS, TEMPLATE_FILE_NAME};
use common::{assert_location, fixture_findings, of, rule_findings, the_failing_finding};

// ---- DOC-001 -------------------------------------------------------------

#[test]
fn doc_001_fails_on_an_empty_title() {
    let f = the_failing_finding(RuleId::Doc001);
    assert_eq!(f.severity, Severity::Error);
    assert_location(&f, "docProps/core.xml", None, None, None);
    assert_eq!(f.fix, FixKind::Ask(FixId::SetTitle));
    // The titles to choose from come from the document when the fix is offered.
    assert_eq!(f.suggestion, None);
    assert_eq!(f.location.place, "The document's properties (File > Info)");
}

#[test]
fn doc_001_passes_with_a_title() {
    assert!(of(&fixture_findings(RuleId::Doc001, false), RuleId::Doc001).is_empty());
}

#[test]
fn doc_001_treats_spaces_and_a_missing_title_as_empty() {
    for builder in [clean().title("   "), DocxBuilder::new().heading(1, "x")] {
        assert_eq!(rule_findings(&builder, RuleId::Doc001).len(), 1);
    }
}

#[test]
fn doc_001_carries_no_suggestion_the_fix_offers_them() {
    // The first Title or Heading 1 used to be the suggestion of the finding, which
    // put a heading such as "Pre-Requisites:" in the place of a title. The fix now
    // offers the file name, the Title paragraph and the first Heading 1 as buttons
    // (`fix::title_suggestions`); the finding has none.
    let title_first = DocxBuilder::new()
        .paragraph(Para::new("  Guide to Printing ").style("Title"))
        .heading(1, "Section");
    let f = rule_findings(&title_first, RuleId::Doc001);
    assert_eq!(f[0].suggestion, None);
}

#[test]
fn doc_001_has_no_suggestion_for_a_document_with_no_title_or_heading_1() {
    let f = rule_findings(
        &DocxBuilder::new().heading(2, "Only H2").para("text"),
        RuleId::Doc001,
    );
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].suggestion, None);
}

// ---- DOC-002 -------------------------------------------------------------

#[test]
fn doc_002_fails_without_a_default_language() {
    let f = the_failing_finding(RuleId::Doc002);
    assert_eq!(f.severity, Severity::Error);
    assert_location(&f, "word/styles.xml", None, None, None);
    assert_eq!(f.fix, FixKind::Auto(FixId::SetDefaultLanguage));
}

#[test]
fn doc_002_passes_with_a_language() {
    assert!(of(&fixture_findings(RuleId::Doc002, false), RuleId::Doc002).is_empty());
}

#[test]
fn doc_002_accepts_the_theme_font_language_when_no_other_language_is_set() {
    let builder = DocxBuilder::new().theme_font_lang("en-US");
    assert!(rule_findings(&builder, RuleId::Doc002).is_empty());
}

// ---- DOC-003 -------------------------------------------------------------

#[test]
fn doc_003_fails_on_unmarked_text_in_another_script() {
    let f = the_failing_finding(RuleId::Doc003);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Привет мир"),
        Some("Test guide"),
    );
}

#[test]
fn doc_003_passes_when_the_language_is_marked() {
    assert!(of(&fixture_findings(RuleId::Doc003, false), RuleId::Doc003).is_empty());
}

#[test]
fn doc_003_needs_at_least_the_minimum_number_of_letters() {
    assert_eq!(MIN_FOREIGN_SCRIPT_LETTERS, 3);
    let at_limit = clean().para("При");
    let below = clean().para("Пр µm");
    assert_eq!(rule_findings(&at_limit, RuleId::Doc003).len(), 1);
    assert!(rule_findings(&below, RuleId::Doc003).is_empty());
}

#[test]
fn doc_003_uses_the_language_attribute_that_fits_the_script() {
    let japanese = "日本語のテキスト";
    let unmarked = clean().paragraph(Para::empty().then(Run::new(japanese)));
    let still_english =
        clean().paragraph(Para::empty().then(Run::new(japanese).lang_east_asia("en-US")));
    let marked = clean().paragraph(Para::empty().then(Run::new(japanese).lang_east_asia("ja-JP")));
    // `lang` (the Latin attribute) does not mark East Asian text.
    let wrong_attribute = clean().paragraph(Para::empty().then(Run::new(japanese).lang("ja-JP")));
    assert_eq!(rule_findings(&unmarked, RuleId::Doc003).len(), 1);
    assert_eq!(rule_findings(&still_english, RuleId::Doc003).len(), 1);
    assert!(rule_findings(&marked, RuleId::Doc003).is_empty());
    assert_eq!(rule_findings(&wrong_attribute, RuleId::Doc003).len(), 1);

    let arabic = "مرحبا بالعالم";
    let unmarked = clean().para(arabic);
    let marked = clean().paragraph(Para::empty().then(Run::new(arabic).lang_bidi("ar-SA")));
    assert_eq!(rule_findings(&unmarked, RuleId::Doc003).len(), 1);
    assert!(rule_findings(&marked, RuleId::Doc003).is_empty());
}

#[test]
fn doc_003_compares_the_main_language_only() {
    // The default is en-US. Text marked en-GB is still English, so Cyrillic marked en-GB is unmarked.
    let builder = clean().paragraph(Para::new("Привет").lang("en-GB"));
    assert_eq!(rule_findings(&builder, RuleId::Doc003).len(), 1);
}

#[test]
fn doc_003_runs_only_for_an_english_default_language() {
    let russian = DocxBuilder::new().language("ru-RU").para("Привет мир");
    let french = DocxBuilder::new().language("fr-FR").para("Привет мир");
    assert!(rule_findings(&russian, RuleId::Doc003).is_empty());
    assert!(rule_findings(&french, RuleId::Doc003).is_empty());
    let british = DocxBuilder::new().language("en-GB").para("Привет мир");
    assert_eq!(rule_findings(&british, RuleId::Doc003).len(), 1);
}

#[test]
fn doc_003_reports_one_finding_per_paragraph() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Привет"))
            .then(Run::new(" and "))
            .then(Run::new("Здравствуйте")),
    );
    assert_eq!(rule_findings(&builder, RuleId::Doc003).len(), 1);
}

#[test]
fn doc_003_does_not_flag_latin_script_text_or_a_missing_default_language() {
    let spanish = clean().para("Hola, esto es una guía de impresión.");
    assert!(rule_findings(&spanish, RuleId::Doc003).is_empty());
    let no_default = DocxBuilder::new().para("Привет мир");
    assert!(rule_findings(&no_default, RuleId::Doc003).is_empty());
}

// ---- DOC-004 -------------------------------------------------------------

#[test]
fn doc_004_fails_without_the_template() {
    let f = the_failing_finding(RuleId::Doc004);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(&f, "word/settings.xml", None, None, None);
}

#[test]
fn doc_004_passes_with_the_template() {
    assert!(of(&fixture_findings(RuleId::Doc004, false), RuleId::Doc004).is_empty());
}

#[test]
fn doc_004_names_another_template_and_ignores_case() {
    let other = rule_findings(&clean().template("Normal.dotm"), RuleId::Doc004);
    assert_eq!(other.len(), 1);
    assert!(
        other[0].message.contains("Normal.dotm"),
        "{}",
        other[0].message
    );

    let shouting = clean().template(&TEMPLATE_FILE_NAME.to_uppercase());
    assert!(rule_findings(&shouting, RuleId::Doc004).is_empty());
}

#[test]
fn doc_004_needs_the_exact_file_name() {
    let similar = clean().template("Adapdf Training Guide v2.dotx");
    assert_eq!(rule_findings(&similar, RuleId::Doc004).len(), 1);
}

// ---- DOC-005 -------------------------------------------------------------

#[test]
fn doc_005_fails_on_tracked_changes() {
    let f = the_failing_finding(RuleId::Doc005);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    // A problem with the document as a whole: no paragraph, no snippet.
    assert_location(&f, "word/document.xml", None, None, None);
    assert!(f.message.contains("tracked changes"), "{}", f.message);
    assert!(!f.message.contains("comments"), "{}", f.message);
}

#[test]
fn doc_005_passes_without_markup() {
    assert!(of(&fixture_findings(RuleId::Doc005, false), RuleId::Doc005).is_empty());
}

#[test]
fn doc_005_fails_on_comments_alone_and_on_both() {
    let comments = rule_findings(
        &clean().paragraph(Para::new("Text").comment("Check this")),
        RuleId::Doc005,
    );
    assert_eq!(comments.len(), 1);
    assert!(
        comments[0].message.contains("has comments"),
        "{}",
        comments[0].message
    );

    let both = rule_findings(
        &clean()
            .paragraph(Para::new("Text").comment("Check this"))
            .paragraph(Para::empty().then(Run::new("More").inserted())),
        RuleId::Doc005,
    );
    assert_eq!(both.len(), 1, "one finding for both kinds");
    assert!(
        both[0].message.contains("tracked changes and comments"),
        "{}",
        both[0].message
    );
}

#[test]
fn doc_005_finds_deletions_and_formatting_changes() {
    let deletion = clean().paragraph(Para::empty().raw_run(
        r#"<w:del w:id="9" w:author="A" w:date="2020-01-01T00:00:00Z"><w:r><w:delText>old</w:delText></w:r></w:del>"#,
    ));
    let formatting = clean().paragraph(Para::empty().raw_run(
        r#"<w:r><w:rPr><w:b/><w:rPrChange w:id="9" w:author="A" w:date="2020-01-01T00:00:00Z"><w:rPr/></w:rPrChange></w:rPr><w:t>x</w:t></w:r>"#,
    ));
    assert_eq!(rule_findings(&deletion, RuleId::Doc005).len(), 1);
    assert_eq!(rule_findings(&formatting, RuleId::Doc005).len(), 1);
}

// ---- DOC-006 -------------------------------------------------------------

#[test]
fn doc_006_fails_on_a_font_that_cannot_be_embedded() {
    let f = the_failing_finding(RuleId::Doc006);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Text in a locked font."),
        Some("Test guide"),
    );
    assert!(f.message.contains("Restricted Test Font"), "{}", f.message);
    assert!(
        f.message.contains("does not allow embedding"),
        "{}",
        f.message
    );
}

#[test]
fn doc_006_passes_with_an_embeddable_font() {
    assert!(of(&fixture_findings(RuleId::Doc006, false), RuleId::Doc006).is_empty());
}

#[test]
fn doc_006_reports_a_font_that_is_not_installed_as_an_error() {
    let f = rule_findings(
        &clean().paragraph(Para::new("Text").font("Missing Test Font")),
        RuleId::Doc006,
    );
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].severity, Severity::Error);
    assert!(f[0].message.contains("not installed"), "{}", f[0].message);
    assert!(
        f[0].message.contains("Missing Test Font"),
        "{}",
        f[0].message
    );
}

#[test]
fn doc_006_checks_the_font_of_the_document_defaults() {
    let builder = DocxBuilder::new()
        .font("Restricted Test Font")
        .para("One")
        .para("Two");
    let f = rule_findings(&builder, RuleId::Doc006);
    assert_eq!(f.len(), 1, "one finding per font, not per paragraph");
    assert_eq!(
        f[0].location.paragraph_index,
        Some(0),
        "at the first paragraph that uses it"
    );
}

#[test]
fn doc_006_ignores_fonts_that_no_text_uses() {
    let no_text = DocxBuilder::new()
        .font("Restricted Test Font")
        .paragraph(Para::empty());
    assert!(rule_findings(&no_text, RuleId::Doc006).is_empty());
    let blank_run = DocxBuilder::new().paragraph(Para::new("   ").font("Restricted Test Font"));
    assert!(rule_findings(&blank_run, RuleId::Doc006).is_empty());
}

#[test]
fn doc_006_reports_each_bad_font_once() {
    let builder = DocxBuilder::new()
        .paragraph(Para::new("a").font("Restricted Test Font"))
        .paragraph(Para::new("b").font("Missing Test Font"))
        .paragraph(Para::new("c").font("Restricted Test Font"))
        .paragraph(Para::new("d").font("Arial"));
    let f = rule_findings(&builder, RuleId::Doc006);
    let places: Vec<_> = f.iter().map(|f| f.location.paragraph_index).collect();
    assert_eq!(places, [Some(0), Some(1)]);
}

#[test]
fn doc_006_uses_the_lookup_it_is_given() {
    struct Everything(FontStatus);
    impl FontLookup for Everything {
        fn status(&self, _family: &str) -> FontStatus {
            self.0
        }
    }
    let bytes = DocxBuilder::new()
        .paragraph(Para::new("x").font("Any Font"))
        .build();
    let check = |status| adapdf::check_bytes(&bytes, &Everything(status)).unwrap();
    assert!(of(&check(FontStatus::Embeddable), RuleId::Doc006).is_empty());
    assert_eq!(of(&check(FontStatus::Restricted), RuleId::Doc006).len(), 1);
    assert_eq!(
        of(&check(FontStatus::NotInstalled), RuleId::Doc006).len(),
        1
    );
}
