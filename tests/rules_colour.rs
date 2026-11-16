//! CLR-001 and CLR-002.

mod common;

use adapdf::findings::{Finding, FixKind, RuleId, Severity};
use adapdf::fixtures::{DocxBuilder, FixtureFonts, Para, Run, Table, clean, raw_docx};
use adapdf::rules::colour::{
    LARGE_BOLD_TEXT_HALF_POINTS, LARGE_TEXT_HALF_POINTS, MIN_CONTRAST_LARGE, MIN_CONTRAST_NORMAL,
};
use common::{assert_location, fixture_findings, of, rule_findings, the_failing_finding};

/// One paragraph with one run. `size` is in half points.
fn one_run(color: &str, size: u32, bold: bool) -> DocxBuilder {
    let mut run = Run::new("Sample text").color(color).size(size);
    if bold {
        run = run.bold();
    }
    clean().paragraph(Para::empty().then(run))
}

fn clr001(builder: &DocxBuilder) -> Vec<Finding> {
    rule_findings(builder, RuleId::Clr001)
}

fn clr002(builder: &DocxBuilder) -> Vec<Finding> {
    rule_findings(builder, RuleId::Clr002)
}

/// Findings of one rule for a document written as raw XML.
fn raw(body: &str, styles: &str, theme: bool, rule: RuleId) -> Vec<Finding> {
    adapdf::check_bytes(&raw_docx(body, styles, theme), &FixtureFonts)
        .expect("a valid .docx")
        .into_iter()
        .filter(|f| f.rule_id == rule)
        .collect()
}

fn p(content: &str) -> String {
    format!("<w:p>{content}</w:p>")
}

fn r(rpr: &str, text: &str) -> String {
    format!("<w:r><w:rPr>{rpr}</w:rPr><w:t xml:space=\"preserve\">{text}</w:t></w:r>")
}

// ---- CLR-001: the fixtures and the message --------------------------------

#[test]
fn clr_001_fails_for_light_grey_text() {
    let f = the_failing_finding(RuleId::Clr001);
    assert_eq!(f.severity, Severity::Error);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Light grey text."),
        Some("Test guide"),
    );
    assert!(f.message.contains("4.48:1"), "{}", f.message);
    assert!(f.message.contains("#777777"), "{}", f.message);
    assert!(f.message.contains("#FFFFFF"), "{}", f.message);
    assert!(
        f.message.contains("Normal text needs at least 4.5:1"),
        "{}",
        f.message
    );
}

#[test]
fn clr_001_passes_for_dark_grey_text() {
    assert!(of(&fixture_findings(RuleId::Clr001, false), RuleId::Clr001).is_empty());
}

#[test]
fn clr_001_thresholds_and_large_text_sizes_are_as_in_spec_03() {
    assert_eq!(MIN_CONTRAST_NORMAL, 4.5);
    assert_eq!(MIN_CONTRAST_LARGE, 3.0);
    assert_eq!(LARGE_TEXT_HALF_POINTS, 36, "18 pt");
    assert_eq!(LARGE_BOLD_TEXT_HALF_POINTS, 28, "14 pt");
}

// ---- CLR-001: WCAG values ---------------------------------------------------

#[test]
fn clr_001_black_on_white_passes() {
    assert!(clr001(&one_run("000000", 22, false)).is_empty());
}

#[test]
fn clr_001_777777_on_white_fails_normal_text_but_passes_large() {
    assert_eq!(clr001(&one_run("777777", 22, false)).len(), 1, "4.48:1");
    assert!(clr001(&one_run("777777", 36, false)).is_empty(), "18 pt");
}

#[test]
fn clr_001_767676_on_white_passes() {
    assert!(clr001(&one_run("767676", 22, false)).is_empty(), "4.54:1");
}

#[test]
fn clr_001_large_text_is_18_pt_or_14_pt_bold() {
    assert!(clr001(&one_run("777777", 36, false)).is_empty(), "18 pt");
    assert_eq!(clr001(&one_run("777777", 35, false)).len(), 1, "17.5 pt");
    assert!(
        clr001(&one_run("777777", 28, true)).is_empty(),
        "14 pt bold"
    );
    assert_eq!(
        clr001(&one_run("777777", 28, false)).len(),
        1,
        "14 pt regular"
    );
    assert_eq!(
        clr001(&one_run("777777", 27, true)).len(),
        1,
        "13.5 pt bold"
    );
}

#[test]
fn clr_001_large_text_needs_3_to_1() {
    // #999999 on white is 2.85:1.
    let f = clr001(&one_run("999999", 36, false));
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("2.85:1"), "{}", f[0].message);
    assert!(
        f[0].message.contains("Large text needs at least 3:1"),
        "{}",
        f[0].message
    );
}

// ---- CLR-001: the background ------------------------------------------------

#[test]
fn clr_001_paragraph_shading_is_the_background() {
    let white_on_yellow = clean().paragraph(
        Para::empty()
            .then(Run::new("Sample text").color("FFFFFF"))
            .shading("FFFF00"),
    );
    let f = clr001(&white_on_yellow);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("on #FFFF00"), "{}", f[0].message);
    let white_on_black = clean().paragraph(
        Para::empty()
            .then(Run::new("Sample text").color("FFFFFF"))
            .shading("000000"),
    );
    assert!(clr001(&white_on_black).is_empty());
}

#[test]
fn clr_001_run_shading_is_above_paragraph_shading() {
    let build = |run_fill: &str| {
        clean().paragraph(
            Para::empty()
                .then(Run::new("Sample text").color("FFFFFF").shading(run_fill))
                .shading("000000"),
        )
    };
    assert!(clr001(&build("000000")).is_empty());
    assert_eq!(clr001(&build("FFFFFF")).len(), 1, "white on white");
}

#[test]
fn clr_001_highlight_is_above_run_shading() {
    let build = |highlight: &str| {
        clean().paragraph(
            Para::empty().then(
                Run::new("Sample text")
                    .color("FFFFFF")
                    .shading("000000")
                    .highlight(highlight),
            ),
        )
    };
    let f = clr001(&build("yellow"));
    assert_eq!(f.len(), 1, "white on yellow");
    assert!(f[0].message.contains("on #FFFF00"), "{}", f[0].message);
    assert!(clr001(&build("darkBlue")).is_empty(), "white on dark blue");
}

#[test]
fn clr_001_highlight_names_are_read() {
    // The text has the same colour as the highlight, so it always fails, and
    // the message names the background that was read.
    for (name, hex) in [
        ("yellow", "FFFF00"),
        ("cyan", "00FFFF"),
        ("lightGray", "C0C0C0"),
        ("darkYellow", "808000"),
        ("darkGray", "808080"),
        ("black", "000000"),
    ] {
        let builder = clean()
            .paragraph(Para::empty().then(Run::new("Sample text").color(hex).highlight(name)));
        let found = clr001(&builder);
        assert_eq!(found.len(), 1, "{name}");
        assert!(
            found[0].message.contains(&format!("on #{hex}")),
            "{name}: {}",
            found[0].message
        );
    }
}

#[test]
fn clr_001_table_cell_shading_is_the_background() {
    let cell = |color: &str| {
        clean().paragraph(
            Para::empty()
                .then(Run::new("Sample text").color(color))
                .cell_shading("000000"),
        )
    };
    assert!(clr001(&cell("FFFFFF")).is_empty());
    assert_eq!(clr001(&cell("333333")).len(), 1);
}

#[test]
fn clr_001_paragraph_shading_is_above_cell_shading() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Sample text").color("FFFFFF"))
            .cell_shading("000000")
            .shading("FFFFFF"),
    );
    assert_eq!(clr001(&builder).len(), 1, "white on the white paragraph");
}

#[test]
fn clr_001_table_shading_is_used_when_the_cell_has_none() {
    let table = |tbl_pr: &str, tc_pr: &str| {
        format!(
            r#"<w:tbl><w:tblPr>{tbl_pr}</w:tblPr><w:tblGrid><w:gridCol w:w="9000"/></w:tblGrid><w:tr><w:tc><w:tcPr>{tc_pr}</w:tcPr>{}</w:tc></w:tr></w:tbl><w:p/>"#,
            p(&r(r#"<w:color w:val="333333"/>"#, "Sample text"))
        )
    };
    let dark = r#"<w:shd w:val="clear" w:color="auto" w:fill="000000"/>"#;
    let light = r#"<w:shd w:val="clear" w:color="auto" w:fill="FFFFFF"/>"#;
    assert_eq!(raw(&table(dark, ""), "", false, RuleId::Clr001).len(), 1);
    assert!(raw(&table("", ""), "", false, RuleId::Clr001).is_empty());
    // The cell is above the table.
    assert!(raw(&table(dark, light), "", false, RuleId::Clr001).is_empty());
}

#[test]
fn clr_001_page_colour_counts_only_when_it_is_shown() {
    let build = |shown: bool| {
        clean()
            .page_background("000000", shown)
            .paragraph(Para::empty().then(Run::new("Sample text").color("333333")))
    };
    assert_eq!(clr001(&build(true)).len(), 1, "dark grey on a black page");
    assert!(clr001(&build(false)).is_empty(), "the page colour is off");
}

#[test]
fn clr_001_paragraph_shading_is_above_the_page_colour() {
    let builder = clean().page_background("000000", true).paragraph(
        Para::empty()
            .then(Run::new("Sample text").color("333333"))
            .shading("FFFFFF"),
    );
    assert!(clr001(&builder).is_empty());
}

// ---- CLR-001: automatic colour ----------------------------------------------

#[test]
fn clr_001_automatic_text_is_white_on_a_dark_background() {
    let build =
        |fill: &str| clean().paragraph(Para::empty().then(Run::new("Sample text")).shading(fill));
    assert!(clr001(&build("1F3864")).is_empty(), "white on navy");
    assert!(clr001(&build("000000")).is_empty());
    assert!(clr001(&build("FFFFFF")).is_empty(), "black on white");
    assert!(clr001(&build("FFFF00")).is_empty(), "black on yellow");
    assert!(
        clr001(&build("808080")).is_empty(),
        "black on mid grey: 5.3:1"
    );
}

#[test]
fn clr_001_automatic_text_on_a_dark_page_colour() {
    let builder = clean()
        .page_background("1F3864", true)
        .paragraph(Para::new("Sample text"));
    assert!(clr001(&builder).is_empty());
}

#[test]
fn clr_001_the_message_says_when_the_colour_is_automatic() {
    // White on #7F7F7F is 4.0:1.
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Sample text"))
            .shading("7F7F7F"),
    );
    let f = clr001(&builder);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].message.contains("#FFFFFF (automatic) on #7F7F7F"),
        "{}",
        f[0].message
    );
    assert!(f[0].message.contains("4.0"), "{}", f[0].message);
}

#[test]
fn clr_001_black_text_on_a_dark_background_fails() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Sample text").color("000000"))
            .shading("1F3864"),
    );
    assert_eq!(clr001(&builder).len(), 1);
}

// ---- CLR-001: styles and theme colours --------------------------------------

const GREY_STYLES: &str = r#"
  <w:style w:type="paragraph" w:styleId="Grey"><w:name w:val="Grey"/><w:basedOn w:val="Normal"/><w:rPr><w:color w:val="777777"/></w:rPr></w:style>
  <w:style w:type="paragraph" w:styleId="Grey2"><w:name w:val="Grey 2"/><w:basedOn w:val="Grey"/></w:style>
  <w:style w:type="paragraph" w:styleId="NavyBox"><w:name w:val="Navy Box"/><w:basedOn w:val="Normal"/><w:pPr><w:shd w:val="clear" w:color="auto" w:fill="1F3864"/></w:pPr></w:style>
  <w:style w:type="character" w:styleId="Pale"><w:name w:val="Pale"/><w:rPr><w:color w:val="777777"/></w:rPr></w:style>"#;

fn styled(style: &str, run_rpr: &str) -> String {
    p(&format!(
        r#"<w:pPr><w:pStyle w:val="{style}"/></w:pPr>{}"#,
        r(run_rpr, "Sample text")
    ))
}

#[test]
fn clr_001_uses_the_colour_of_the_paragraph_style_and_its_base_styles() {
    for style in ["Grey", "Grey2"] {
        let found = raw(&styled(style, ""), GREY_STYLES, false, RuleId::Clr001);
        assert_eq!(found.len(), 1, "{style}");
        assert!(found[0].message.contains("#777777"));
    }
}

#[test]
fn clr_001_direct_colour_wins_over_the_style() {
    let body = styled("Grey", r#"<w:color w:val="000000"/>"#);
    assert!(raw(&body, GREY_STYLES, false, RuleId::Clr001).is_empty());
}

#[test]
fn clr_001_uses_the_colour_of_a_character_style() {
    let body = p(&r(r#"<w:rStyle w:val="Pale"/>"#, "Sample text"));
    assert_eq!(raw(&body, GREY_STYLES, false, RuleId::Clr001).len(), 1);
}

#[test]
fn clr_001_uses_the_shading_of_the_paragraph_style() {
    let auto = styled("NavyBox", "");
    assert!(raw(&auto, GREY_STYLES, false, RuleId::Clr001).is_empty());
    let dark = styled("NavyBox", r#"<w:color w:val="333333"/>"#);
    let found = raw(&dark, GREY_STYLES, false, RuleId::Clr001);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].message.contains("on #1F3864"),
        "{}",
        found[0].message
    );
}

#[test]
fn clr_001_resolves_theme_colours_with_tint_and_shade() {
    let theme_run = |attrs: &str| {
        p(&r(
            &format!(r#"<w:color w:val="FF0000" {attrs}/>"#),
            "Sample text",
        ))
    };
    // Accent 1 is 4472C4. Tint 99 gives 8FAADC (Word's "Lighter 40%"): 2.3:1.
    let tinted = raw(
        &theme_run(r#"w:themeColor="accent1" w:themeTint="99""#),
        "",
        true,
        RuleId::Clr001,
    );
    assert_eq!(tinted.len(), 1);
    assert!(
        tinted[0].message.contains("#8FAADC"),
        "{}",
        tinted[0].message
    );
    // Shade BF gives 2F5597 (Word's "Darker 25%"): 7.1:1.
    let shaded = theme_run(r#"w:themeColor="accent1" w:themeShade="BF""#);
    assert!(raw(&shaded, "", true, RuleId::Clr001).is_empty());
    // The theme colour wins over w:val. Accent 1 itself is 4.7:1 on white.
    let plain = theme_run(r#"w:themeColor="accent1""#);
    assert!(raw(&plain, "", true, RuleId::Clr001).is_empty());
    let text1 = theme_run(r#"w:themeColor="text1""#);
    assert!(raw(&text1, "", true, RuleId::Clr001).is_empty());
    // Without a theme the colour falls back to w:val: red on white is 4.0:1.
    let no_theme = raw(&plain, "", false, RuleId::Clr001);
    assert_eq!(no_theme.len(), 1);
    assert!(no_theme[0].message.contains("#FF0000"));
}

#[test]
fn clr_001_resolves_theme_fills() {
    // Black on accent 1 (#4472C4) is 4.4:1. White on it is 4.7:1.
    let body = |colour: &str| {
        p(&format!(
            r#"<w:pPr><w:shd w:val="clear" w:color="auto" w:fill="FFFFFF" w:themeFill="accent1"/></w:pPr>{}"#,
            r(colour, "Sample text")
        ))
    };
    let black = raw(
        &body(r#"<w:color w:val="000000"/>"#),
        "",
        true,
        RuleId::Clr001,
    );
    assert_eq!(black.len(), 1);
    assert!(
        black[0].message.contains("on #4472C4"),
        "{}",
        black[0].message
    );
    assert!(
        raw(&body(""), "", true, RuleId::Clr001).is_empty(),
        "automatic is white"
    );
    assert!(
        raw(
            &body(r#"<w:color w:val="FFFFFF"/>"#),
            "",
            true,
            RuleId::Clr001
        )
        .is_empty()
    );
}

// ---- CLR-001: which runs are checked ----------------------------------------

#[test]
fn clr_001_skips_runs_with_no_visible_text() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("   ").color("FFFFFF"))
            .then(Run::new("\t").color("FFFFFF"))
            .then(Run::new("").color("FFFFFF")),
    );
    assert!(clr001(&builder).is_empty());
}

#[test]
fn clr_001_reports_each_pair_of_colours_once_in_a_paragraph() {
    let same = clean().paragraph(
        Para::empty()
            .then(Run::new("One").color("777777"))
            .then(Run::new(" two").color("777777")),
    );
    assert_eq!(clr001(&same).len(), 1);
    let different = clean().paragraph(
        Para::empty()
            .then(Run::new("One").color("777777"))
            .then(Run::new(" two").color("888888")),
    );
    assert_eq!(clr001(&different).len(), 2);
    let sizes = clean().paragraph(
        Para::empty()
            .then(Run::new("One").color("999999"))
            .then(Run::new(" two").color("999999").size(36)),
    );
    assert_eq!(clr001(&sizes).len(), 2, "normal and large text differ");
}

#[test]
fn clr_001_checks_text_in_table_cells() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Sample text").color("777777"))
            .in_table(),
    );
    assert_eq!(clr001(&builder).len(), 1);
}

#[test]
fn clr_001_passing_text_in_a_failing_paragraph_is_not_reported() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("Fine").color("000000"))
            .then(Run::new(" not fine").color("777777")),
    );
    let f = clr001(&builder);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("not fine"), "{}", f[0].message);
}

// ---- CLR-002 --------------------------------------------------------------

#[test]
fn clr_002_fails_for_text_that_only_differs_in_colour() {
    let f = the_failing_finding(RuleId::Clr002);
    assert_eq!(f.severity, Severity::Warning);
    assert_eq!(f.fix, FixKind::Manual);
    assert_location(
        &f,
        "word/document.xml",
        Some(2),
        Some("Warning: save your work first."),
        Some("Test guide"),
    );
    assert!(f.message.contains("#C00000"), "{}", f.message);
    assert!(f.message.contains("Warning:"), "{}", f.message);
}

#[test]
fn clr_002_passes_when_the_text_is_also_bold() {
    assert!(of(&fixture_findings(RuleId::Clr002, false), RuleId::Clr002).is_empty());
}

fn red(run: Run) -> DocxBuilder {
    clean()
        .char_style(
            "Hyperlink",
            "Hyperlink",
            r#"<w:color w:val="0563C1"/><w:u w:val="single"/>"#,
        )
        .char_style("Strong", "Strong", "<w:b/>")
        .paragraph(
            Para::empty()
                .then(run)
                .then(Run::new(" then the rest of the line.")),
        )
}

#[test]
fn clr_002_each_other_kind_of_emphasis_is_enough() {
    let base = Run::new("Red text").color("C00000");
    assert_eq!(clr002(&red(base.clone())).len(), 1);
    for (name, run) in [
        ("bold", base.clone().bold()),
        ("italic", base.clone().italic()),
        ("underline", base.clone().underline()),
        ("highlight", base.clone().highlight("yellow")),
        ("shading", base.clone().shading("FFE0E0")),
        ("character style", base.clone().char_style("Strong")),
    ] {
        assert!(clr002(&red(run)).is_empty(), "{name}");
    }
}

#[test]
fn clr_002_hyperlinks_are_not_reported() {
    let styled = clean()
        .char_style(
            "Hyperlink",
            "Hyperlink",
            r#"<w:color w:val="0563C1"/><w:u w:val="single"/>"#,
        )
        .paragraph(Para::empty().then(Run::new("A link").char_style("Hyperlink")));
    assert!(clr002(&styled).is_empty());
    // A coloured run inside a hyperlink element, with no character style.
    let body = p(
        r#"<w:hyperlink w:anchor="_top"><w:r><w:rPr><w:color w:val="0563C1"/></w:rPr><w:t>A link</w:t></w:r></w:hyperlink>"#,
    );
    assert!(raw(&body, "", false, RuleId::Clr002).is_empty());
}

#[test]
fn clr_002_the_colour_of_the_paragraph_style_is_not_a_difference() {
    let in_style = styled("Grey", "");
    assert!(raw(&in_style, GREY_STYLES, false, RuleId::Clr002).is_empty());
    let same = styled("Grey", r#"<w:color w:val="777777"/>"#);
    assert!(raw(&same, GREY_STYLES, false, RuleId::Clr002).is_empty());
    let different = styled("Grey", r#"<w:color w:val="C00000"/>"#);
    assert_eq!(raw(&different, GREY_STYLES, false, RuleId::Clr002).len(), 1);
    let inherited = styled("Grey2", r#"<w:color w:val="C00000"/>"#);
    assert_eq!(raw(&inherited, GREY_STYLES, false, RuleId::Clr002).len(), 1);
}

#[test]
fn clr_002_black_is_the_same_as_automatic() {
    let builder = clean().paragraph(Para::empty().then(Run::new("Black text").color("000000")));
    assert!(clr002(&builder).is_empty());
}

#[test]
fn clr_002_emphasis_is_compared_with_the_paragraph_style() {
    // Heading 1 is bold, so a coloured run in it is not "also bold".
    let heading = clean().paragraph(
        Para::empty()
            .then(Run::new("Red heading").color("C00000"))
            .style("Heading2"),
    );
    assert_eq!(clr002(&heading).len(), 1);
}

#[test]
fn clr_002_reports_each_colour_once_in_a_paragraph() {
    let builder = clean().paragraph(
        Para::empty()
            .then(Run::new("One").color("C00000"))
            .then(Run::new(" two").color("C00000"))
            .then(Run::new(" three").color("0000FF")),
    );
    assert_eq!(clr002(&builder).len(), 2);
}

#[test]
fn clr_002_skips_runs_with_no_visible_text() {
    let builder = clean().paragraph(Para::empty().then(Run::new("  ").color("C00000")));
    assert!(clr002(&builder).is_empty());
}

// ---- Table styles ----------------------------------------------------------
//
// A text colour or a fill that comes only from a table style counts. The
// colour model of table styles is tested in `model::table_styles::tests`.

fn tc_shd(fill: &str) -> String {
    format!(r#"<w:tcPr><w:shd w:val="clear" w:color="auto" w:fill="{fill}"/></w:tcPr>"#)
}

fn tc_colour(hex: &str) -> String {
    format!(r#"<w:rPr><w:color w:val="{hex}"/></w:rPr>"#)
}

/// A conditional format with a text colour and a fill (`None`: not set).
fn conditional(kind: &str, text: Option<&str>, fill: Option<&str>) -> String {
    format!(
        r#"<w:tblStylePr w:type="{kind}">{}{}</w:tblStylePr>"#,
        text.map(tc_colour).unwrap_or_default(),
        fill.map(tc_shd).unwrap_or_default()
    )
}

const BAND_SIZES: &str =
    r#"<w:tblPr><w:tblStyleRowBandSize w:val="1"/><w:tblStyleColBandSize w:val="1"/></w:tblPr>"#;
/// The first row and the row bands.
const LOOK_HEADER_AND_BANDS: &str = r#"<w:tblLook w:firstRow="1" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="0" w:noVBand="1"/>"#;
/// The first row, and no bands.
const LOOK_HEADER_ONLY: &str = r#"<w:tblLook w:firstRow="1" w:lastRow="0" w:firstColumn="0" w:lastColumn="0" w:noHBand="1" w:noVBand="1"/>"#;

/// A table of `rows` x 2 cells that uses the table style `Adapdf` with this
/// inner XML, after the heading and the paragraph of `clean()`. The cells are
/// paragraphs 2, 3, ... in reading order.
fn styled_table(style_inner: &str, rows: usize, look: &str) -> DocxBuilder {
    let text: Vec<Vec<String>> = (0..rows)
        .map(|r| (0..2).map(|c| format!("r{r}c{c}")).collect())
        .collect();
    let refs: Vec<Vec<&str>> = text
        .iter()
        .map(|row| row.iter().map(String::as_str).collect())
        .collect();
    let slices: Vec<&[&str]> = refs.iter().map(Vec::as_slice).collect();
    clean()
        .table_style("Adapdf", "Adapdf Table", None, style_inner)
        .table(Table::new(&slices).style("Adapdf").look(look))
}

/// The paragraph indexes (from 0) of the findings, in order.
fn at(findings: &[Finding]) -> Vec<Option<usize>> {
    findings
        .iter()
        .map(|f| f.location.paragraph_index)
        .collect()
}

#[test]
fn clr_001_white_header_text_on_a_dark_fill_from_the_table_style_passes() {
    // Without the table style the text would be white on white.
    let style = conditional("firstRow", Some("FFFFFF"), Some("1F3864"));
    let builder = styled_table(&style, 3, LOOK_HEADER_ONLY);
    assert!(clr001(&builder).is_empty());
}

#[test]
fn clr_001_fails_for_header_text_that_is_only_too_light_because_of_the_table_style() {
    // The paragraphs and runs set no colour. Only the table style does.
    let style = conditional("firstRow", Some("FFFFFF"), Some("FFFF00"));
    let found = clr001(&styled_table(&style, 3, LOOK_HEADER_ONLY));
    assert_eq!(at(&found), [Some(2), Some(3)], "the two header cells");
    assert!(found[0].message.contains("#FFFFFF"), "{}", found[0].message);
    assert!(
        found[0].message.contains("on #FFFF00"),
        "{}",
        found[0].message
    );
    // The same table with no style on it has no finding.
    let plain = clean().table(Table::new(&[&["r0c0", "r0c1"], &["r1c0", "r1c1"]]));
    assert!(clr001(&plain).is_empty());
}

#[test]
fn clr_001_banded_rows_that_fail_come_from_the_band_fill() {
    // Grey text (4.54:1 on white) on the band fill CCCCCC is 2.83:1.
    let style = format!(
        "{}{}{}{}",
        tc_colour("767676"),
        BAND_SIZES,
        conditional("band1Horz", None, Some("CCCCCC")),
        conditional("firstRow", Some("000000"), None),
    );
    let found = clr001(&styled_table(&style, 5, LOOK_HEADER_AND_BANDS));
    // Row 0 is the header, rows 1 and 3 are the first band, rows 2 and 4 have
    // no fill. Row r has paragraphs 2 + 2r and 3 + 2r.
    assert_eq!(
        at(&found),
        [Some(4), Some(5), Some(8), Some(9)],
        "rows 1 and 3"
    );
    assert!(found[0].message.contains("#767676"), "{}", found[0].message);
    assert!(
        found[0].message.contains("on #CCCCCC"),
        "{}",
        found[0].message
    );
}

#[test]
fn clr_001_tbl_look_can_switch_the_band_off() {
    let style = format!(
        "{}{}{}",
        tc_colour("767676"),
        BAND_SIZES,
        conditional("band1Horz", None, Some("CCCCCC")),
    );
    let banded = |look: &str| clr001(&styled_table(&style, 3, look)).len();
    assert_eq!(banded(LOOK_HEADER_AND_BANDS), 2, "row 1 is in the band");
    assert_eq!(banded(LOOK_HEADER_ONLY), 0, "noHBand: no bands");
    // The same in the older form: 0x0020 first row, 0x0200 no row bands.
    assert_eq!(banded(r#"<w:tblLook w:val="0020"/>"#), 2);
    assert_eq!(banded(r#"<w:tblLook w:val="0220"/>"#), 0);
}

#[test]
fn clr_001_a_fill_set_in_the_cell_beats_the_table_style() {
    // The style makes the header white on navy: it passes.
    let style = conditional("firstRow", Some("FFFFFF"), Some("1F3864"));
    let table = |fill: Option<(usize, &str)>| {
        let mut table = Table::new(&[&["r0c0", "r0c1"], &["r1c0", "r1c1"]])
            .style("Adapdf")
            .look(LOOK_HEADER_ONLY);
        if let Some((cell, hex)) = fill {
            table = table.cell_fill(0, cell, hex);
        }
        clean()
            .table_style("Adapdf", "Adapdf Table", None, &style)
            .table(table)
    };
    assert!(clr001(&table(None)).is_empty());
    // White text on a white cell fill fails, in that cell only.
    let found = clr001(&table(Some((1, "FFFFFF"))));
    assert_eq!(at(&found), [Some(3)]);
}

#[test]
fn clr_001_a_fill_set_in_the_cell_can_also_make_the_style_pass() {
    // The style makes the header white on yellow: it fails. A navy fill in
    // the second cell fixes that cell.
    let style = conditional("firstRow", Some("FFFFFF"), Some("FFFF00"));
    let table = Table::new(&[&["r0c0", "r0c1"], &["r1c0", "r1c1"]])
        .style("Adapdf")
        .look(LOOK_HEADER_ONLY)
        .cell_fill(0, 1, "1F3864");
    let builder = clean()
        .table_style("Adapdf", "Adapdf Table", None, &style)
        .table(table);
    assert_eq!(at(&clr001(&builder)), [Some(2)]);
}

#[test]
fn clr_001_follows_the_based_on_chain_of_a_table_style() {
    let base = format!(
        "{}{}",
        tc_colour("767676"),
        conditional("firstRow", Some("FFFFFF"), Some("1F3864"))
    );
    // The derived style changes only the fill of the first row, to yellow.
    let derived = conditional("firstRow", None, Some("FFFF00"));
    let build = |style: &str| {
        let table = Table::new(&[&["r0c0", "r0c1"], &["r1c0", "r1c1"]])
            .style(style)
            .look(LOOK_HEADER_ONLY);
        clean()
            .table_style("Base", "Base", None, &base)
            .table_style("Derived", "Derived", Some("Base"), &derived)
            .table(table)
    };
    assert!(clr001(&build("Base")).is_empty());
    let found = clr001(&build("Derived"));
    assert_eq!(
        at(&found),
        [Some(2), Some(3)],
        "white (from Base) on yellow"
    );
}

#[test]
fn clr_001_a_paragraph_style_colour_beats_the_table_style_colour() {
    let styles = r#"<w:style w:type="paragraph" w:styleId="Pale"><w:name w:val="Pale"/><w:rPr><w:color w:val="999999"/></w:rPr></w:style>
<w:style w:type="table" w:styleId="TS"><w:name w:val="TS"/><w:rPr><w:color w:val="000000"/></w:rPr></w:style>"#;
    let cell = |style: &str| {
        let ppr = if style.is_empty() {
            String::new()
        } else {
            format!(r#"<w:pPr><w:pStyle w:val="{style}"/></w:pPr>"#)
        };
        format!(
            "<w:tc>{}</w:tc>",
            p(&format!(r#"{ppr}<w:r><w:t>text</w:t></w:r>"#))
        )
    };
    let body = format!(
        r#"<w:tbl><w:tblPr><w:tblStyle w:val="TS"/></w:tblPr><w:tblGrid><w:gridCol w:w="4000"/><w:gridCol w:w="4000"/></w:tblGrid><w:tr>{}{}</w:tr></w:tbl><w:p/>"#,
        cell("Pale"),
        cell("")
    );
    let found = raw(&body, styles, false, RuleId::Clr001);
    assert_eq!(
        found.len(),
        1,
        "only the cell with the pale paragraph style"
    );
    assert!(found[0].message.contains("#999999"), "{}", found[0].message);
}

#[test]
fn clr_002_the_table_style_colour_is_the_colour_of_the_text_around() {
    // The table style makes the text navy. A run in the same navy is not a
    // different colour; a red run is, and nothing else marks it.
    let style = tc_colour("1F3864");
    let navy = r#"<w:p><w:r><w:rPr><w:color w:val="1F3864"/></w:rPr><w:t>navy</w:t></w:r></w:p>"#;
    let red = r#"<w:p><w:r><w:rPr><w:color w:val="C00000"/></w:rPr><w:t>red</w:t></w:r></w:p>"#;
    let table = Table::new(&[&["a", "b"]])
        .style("Adapdf")
        .raw_cell(0, 0, navy)
        .raw_cell(0, 1, red);
    let builder = clean()
        .table_style("Adapdf", "Adapdf Table", None, &style)
        .table(table);
    let found = clr002(&builder);
    assert_eq!(at(&found), [Some(3)], "the red run only");
    assert!(found[0].message.contains("#C00000"), "{}", found[0].message);
}

#[test]
fn clr_002_white_header_text_from_the_table_style_is_not_reported() {
    let style = conditional("firstRow", Some("FFFFFF"), Some("1F3864"));
    let builder = styled_table(&style, 2, LOOK_HEADER_ONLY);
    assert!(clr002(&builder).is_empty());
}
