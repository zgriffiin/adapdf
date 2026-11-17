//! The fixes for LNK-004 (a ScreenTip for a link, `AddScreenTip`) and LNK-005
//! (a table of contents without links, `RemoveTocLinks`). Test plan rows S2-32
//! and S2-33, and the rows C-09 and C-10 of `adapdf fix` for them.

mod fix_support;

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use adapdf::findings::RuleId;
use adapdf::fix::{self, Availability, FixRequest};
use adapdf::fixtures::{
    DocxBuilder, FixtureFonts, Para, TOC_INSTRUCTION, TOC_INSTRUCTION_NO_LINKS, clean, raw_docx,
};
use adapdf::model::Document;
use adapdf::model::fields::LinkTarget;
use fix_support::*;

const URL: &str = "https://example.org/parking";
const DOCUMENT: &str = "word/document.xml";

/// The ScreenTips of the links of a .docx, in order (`None`: no ScreenTip).
fn screen_tips(bytes: &[u8]) -> Vec<Option<String>> {
    Document::from_bytes(bytes)
        .unwrap()
        .links
        .iter()
        .map(|l| l.screen_tip.clone())
        .collect()
}

/// Fixes every LNK-004 finding of the document.
fn fix_all_links(bytes: &[u8]) -> fix::Applied {
    let found = findings(bytes);
    let requests = requests_for(&found, RuleId::Lnk004, "");
    assert!(!requests.is_empty());
    apply(bytes, &requests)
}

/// The same with a document whose body is this XML, in a raw package.
fn raw(body: &str) -> Vec<u8> {
    raw_docx(body, "", false)
}

/// The text that all splices put in, for example a new attribute.
fn inserted(applied: &fix::Applied) -> Vec<&str> {
    applied.splices[DOCUMENT]
        .iter()
        .map(|s| s.text.as_str())
        .collect()
}

// ---- LNK-004: a w:hyperlink element ----------------------------------------

// S2-32
#[test]
fn fix_add_screen_tip_to_a_hyperlink_element() {
    let original = fixture(RuleId::Lnk004, true);
    assert_eq!(screen_tips(&original), [None]);
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_eq!(
        applied.applied[0].summary,
        "Added the ScreenTip \"parking policy\" to the link in paragraph 3."
    );
    assert_only_the_splices_changed(&original, &applied);
    assert_eq!(
        inserted(&applied),
        [r#" w:tooltip="parking policy""#],
        "only the attribute is written"
    );
    let after = fixed(&applied);
    assert_eq!(screen_tips(after), [Some("parking policy".to_string())]);
    // The target and the text of the link are as they were.
    let model = Document::from_bytes(after).unwrap();
    assert_eq!(model.links[0].text, "parking policy");
    assert_eq!(
        model.links[0].target,
        LinkTarget::External(URL_OF_FIXTURE.to_string())
    );
    assert!(of(&findings(after), RuleId::Lnk004).is_empty());
}

const URL_OF_FIXTURE: &str = "https://example.org/parking-policy";

#[test]
fn fix_add_screen_tip_to_an_internal_link_and_replaces_an_empty_one() {
    let original = build(
        &clean()
            .paragraph(Para::new("Setup").style("Heading2").bookmark("Setup"))
            .paragraph(
                Para::empty()
                    .internal_link("See the setup section", "Setup")
                    .internal_link_tip("Back to the top", "_top", "   "),
            ),
    );
    assert_eq!(findings(&original).len(), 2, "{:#?}", findings(&original));
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty());
    assert_only_the_splices_changed(&original, &applied);
    assert_eq!(
        screen_tips(fixed(&applied)),
        [
            Some("See the setup section".to_string()),
            Some("Back to the top".to_string())
        ]
    );
    assert!(findings(fixed(&applied)).is_empty());
}

// ---- LNK-004: fields -------------------------------------------------------

#[test]
fn fix_add_screen_tip_to_a_simple_field_edits_only_the_instruction() {
    let original =
        build(&clean().paragraph(
            Para::empty().simple_field(&format!(r#"HYPERLINK "{URL}""#), "Parking policy"),
        ));
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_exactly_these_edits(
        &original,
        fixed(&applied),
        DOCUMENT,
        &[edit(
            &format!(r#"w:instr=" HYPERLINK &quot;{URL}&quot; ">"#),
            &format!(r#"w:instr=" HYPERLINK &quot;{URL}&quot; \o &quot;Parking policy&quot; ">"#),
        )],
    );
    assert_only_the_splices_changed(&original, &applied);
    assert_eq!(
        screen_tips(fixed(&applied)),
        [Some("Parking policy".to_string())]
    );
    assert!(of(&findings(fixed(&applied)), RuleId::Lnk004).is_empty());
}

#[test]
fn fix_add_screen_tip_to_a_simple_field_in_single_quotes() {
    // Word can write the attribute in single quotes, so that the instruction
    // can hold double quotes as they are.
    let original = raw(
        r#"<w:p><w:fldSimple w:instr=' HYPERLINK \l "Top" '><w:r><w:t>Back to the top</w:t></w:r></w:fldSimple></w:p>"#,
    );
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_exactly_these_edits(
        &original,
        fixed(&applied),
        DOCUMENT,
        &[edit(
            r#"w:instr=' HYPERLINK \l "Top" '>"#,
            r#"w:instr=' HYPERLINK \l "Top" \o &quot;Back to the top&quot; '>"#,
        )],
    );
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    assert_eq!(
        model.links[0].screen_tip.as_deref(),
        Some("Back to the top")
    );
    assert_eq!(
        model.links[0].target,
        LinkTarget::Internal("Top".to_string())
    );
}

#[test]
fn fix_add_screen_tip_to_a_complex_field_edits_only_the_instruction() {
    let original = build(&clean().paragraph(
        Para::new("See ").field(&format!(r#"HYPERLINK "{URL}""#), "the parking policy"),
    ));
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_exactly_these_edits(
        &original,
        fixed(&applied),
        DOCUMENT,
        &[edit(
            &format!(r#"HYPERLINK &quot;{URL}&quot; </w:instrText>"#),
            &format!(r#"HYPERLINK &quot;{URL}&quot; \o "the parking policy" </w:instrText>"#),
        )],
    );
    assert_only_the_splices_changed(&original, &applied);
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    assert_eq!(
        model.links[0].screen_tip.as_deref(),
        Some("the parking policy")
    );
    assert_eq!(model.links[0].target, LinkTarget::External(URL.to_string()));
    assert_eq!(model.links[0].text, "the parking policy");
    assert!(of(&findings(fixed(&applied)), RuleId::Lnk004).is_empty());
}

#[test]
fn fix_add_screen_tip_to_a_complex_field_with_its_instruction_in_pieces() {
    // Word can split an instruction over several runs. The switch goes in the
    // last piece that has text, before the spaces at the end.
    let body = format!(
        r#"<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> HYPERLINK </w:instrText></w:r><w:r><w:instrText xml:space="preserve">"{URL}"</w:instrText></w:r><w:r><w:instrText xml:space="preserve">  </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>Parking</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#
    );
    let original = raw(&body);
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_exactly_these_edits(
        &original,
        fixed(&applied),
        DOCUMENT,
        &[edit(
            &format!(r#""{URL}"</w:instrText>"#),
            &format!(r#""{URL}" \o "Parking"</w:instrText>"#),
        )],
    );
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    assert_eq!(model.links[0].screen_tip.as_deref(), Some("Parking"));
    assert_eq!(model.links[0].target, LinkTarget::External(URL.to_string()));
}

#[test]
fn fix_add_screen_tip_gives_the_instruction_xml_space_preserve() {
    let body = format!(
        r#"<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> HYPERLINK "{URL}" </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>Parking</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#
    );
    let original = raw(&body);
    let applied = fix_all_links(&original);
    assert_exactly_these_edits(
        &original,
        fixed(&applied),
        DOCUMENT,
        &[edit(
            &format!(r#"<w:instrText> HYPERLINK "{URL}" </w:instrText>"#),
            &format!(
                r#"<w:instrText xml:space="preserve"> HYPERLINK "{URL}" \o "Parking" </w:instrText>"#
            ),
        )],
    );
    assert_eq!(screen_tips(fixed(&applied)), [Some("Parking".to_string())]);
}

#[test]
fn fix_add_screen_tip_keeps_the_other_switches_of_a_field() {
    let original = build(&clean().paragraph(Para::empty().field(
        r#"HYPERLINK "other.docx" \l "Intro" \t "_blank""#,
        "Other file",
    )));
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    assert_eq!(model.links[0].screen_tip.as_deref(), Some("Other file"));
    assert_eq!(
        model.links[0].target,
        LinkTarget::External("other.docx#Intro".to_string())
    );
}

// ---- LNK-004: escaping -----------------------------------------------------

/// The text that the tests of escaping use as the text of a link.
const AWKWARD_LINK: &str = r#"Say "stop" & <go> C:\temp é 日本"#;

#[test]
fn fix_add_screen_tip_escapes_quotes_backslashes_and_xml_characters() {
    let documents = [
        (
            "w:hyperlink",
            build(&clean().paragraph(Para::empty().hyperlink(AWKWARD_LINK, URL))),
        ),
        (
            "complex field",
            build(
                &clean()
                    .paragraph(Para::empty().field(&format!(r#"HYPERLINK "{URL}""#), AWKWARD_LINK)),
            ),
        ),
        (
            "simple field",
            build(&clean().paragraph(
                Para::empty().simple_field(&format!(r#"HYPERLINK "{URL}""#), AWKWARD_LINK),
            )),
        ),
    ];
    for (name, original) in documents {
        let applied = fix_all_links(&original);
        assert!(applied.refused.is_empty(), "{name}: {:?}", applied.refused);
        assert_only_the_splices_changed(&original, &applied);
        let model = Document::from_bytes(fixed(&applied)).unwrap();
        assert_eq!(
            model.links[0].screen_tip.as_deref(),
            Some(AWKWARD_LINK),
            "{name}: the ScreenTip is the link text"
        );
        assert_eq!(model.links[0].text, AWKWARD_LINK, "{name}");
        // A quote mark or a backslash in the text must not change the target.
        assert_eq!(
            model.links[0].target,
            LinkTarget::External(URL.to_string()),
            "{name}"
        );
        assert!(
            of(&findings(fixed(&applied)), RuleId::Lnk004).is_empty(),
            "{name}"
        );
    }
}

#[test]
fn fix_add_screen_tip_writes_the_field_escapes_and_the_xml_escapes() {
    let complex = build(
        &clean().paragraph(Para::empty().field(&format!(r#"HYPERLINK "{URL}""#), AWKWARD_LINK)),
    );
    // In a field instruction: \" and \\. In XML text: &amp; and &lt;, and a
    // quote mark as it is.
    assert_eq!(
        inserted(&fix_all_links(&complex)),
        [r#" \o "Say \"stop\" &amp; &lt;go&gt; C:\\temp é 日本""#]
    );
    // In the attribute of a simple field: the quote marks are &quot; as well.
    let simple = build(
        &clean()
            .paragraph(Para::empty().simple_field(&format!(r#"HYPERLINK "{URL}""#), AWKWARD_LINK)),
    );
    assert_eq!(
        inserted(&fix_all_links(&simple)),
        [r#" \o &quot;Say \&quot;stop\&quot; &amp; &lt;go&gt; C:\\temp é 日本&quot;"#]
    );
    // In the attribute of a w:hyperlink: no field escapes, and XML escapes.
    let element = build(&clean().paragraph(Para::empty().hyperlink(AWKWARD_LINK, URL)));
    assert_eq!(
        inserted(&fix_all_links(&element)),
        [r#" w:tooltip="Say &quot;stop&quot; &amp; &lt;go&gt; C:\temp é 日本""#]
    );
}

#[test]
fn fix_add_screen_tip_folds_spaces_tabs_and_line_breaks_in_the_text() {
    let original = raw(
        r#"<w:p><w:hyperlink r:id="rId9"><w:r><w:t xml:space="preserve">  Parking </w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>policy</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>page  </w:t></w:r></w:hyperlink></w:p>"#,
    );
    let applied = fix_all_links(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_eq!(
        screen_tips(fixed(&applied)),
        [Some("Parking policy page".to_string())]
    );
}

// ---- LNK-004: what is refused, and what is left alone ------------------------

#[test]
fn fix_add_screen_tip_refuses_a_link_with_no_text() {
    for text in ["", "   ", " \t "] {
        let original = build(&clean().paragraph(Para::empty().hyperlink(text, URL)));
        let found = findings(&original);
        let finding = of(&found, RuleId::Lnk004).remove(0);
        let applied = apply(&original, &[FixRequest::confirm(&finding)]);
        assert!(applied.bytes.is_none(), "{text:?}");
        assert!(applied.applied.is_empty());
        assert_eq!(applied.refused.len(), 1, "{text:?}");
        let reason = &applied.refused[0].reason;
        assert!(reason.contains("no text"), "{reason}");
        assert!(reason.contains("ScreenTip"), "{reason}");
        assert!(reason.contains("does not make one up"), "{reason}");

        // `adapdf fix` shows it as a fix the tool cannot do now.
        let offers = fix::offers(&original, &options(), &FixtureFonts).unwrap();
        let offer = offers
            .iter()
            .find(|o| o.finding.rule_id == RuleId::Lnk004)
            .unwrap();
        assert!(
            matches!(&offer.availability, Availability::Unavailable(r) if r.contains("no text")),
            "{:?}",
            offer.availability
        );
    }
}

#[test]
fn fix_add_screen_tip_refuses_a_field_that_it_cannot_extend() {
    // An empty \o switch is not changed; an open quote is not completed.
    for instruction in [
        format!(r#"HYPERLINK "{URL}" \o """#),
        format!(r#"HYPERLINK "{URL}" \o "   ""#),
        format!(r#"HYPERLINK "{URL}"#),
    ] {
        for complex in [true, false] {
            let paragraph = if complex {
                Para::empty().field(&instruction, "Parking")
            } else {
                Para::empty().simple_field(&instruction, "Parking")
            };
            let original = build(&clean().paragraph(paragraph));
            let found = findings(&original);
            let finding = of(&found, RuleId::Lnk004).remove(0);
            let applied = apply(&original, &[FixRequest::confirm(&finding)]);
            assert!(applied.bytes.is_none(), "{instruction} {complex}");
            assert_eq!(applied.refused.len(), 1, "{instruction} {complex}");
            assert!(
                applied.refused[0].reason.contains("ScreenTip"),
                "{}",
                applied.refused[0].reason
            );
        }
    }
}

#[test]
fn fix_add_screen_tip_changes_only_the_link_that_was_chosen() {
    let original = build(
        &clean().paragraph(
            Para::new("A ")
                .hyperlink("first", URL)
                .hyperlink("second", URL)
                .hyperlink("third", URL),
        ),
    );
    let found = findings(&original);
    assert_eq!(of(&found, RuleId::Lnk004).len(), 3);
    let second = of(&found, RuleId::Lnk004).remove(1);
    let applied = apply(&original, &[FixRequest::confirm(&second).at(1)]);
    assert_only_the_splices_changed(&original, &applied);
    assert_eq!(
        screen_tips(fixed(&applied)),
        [None, Some("second".to_string()), None]
    );
    assert_eq!(
        of(&findings(fixed(&applied)), RuleId::Lnk004).len(),
        2,
        "the other two are still reported"
    );
}

#[test]
fn fix_add_screen_tip_leaves_the_entries_of_a_table_of_contents_alone() {
    let original = build(
        &DocxBuilder::new()
            .toc(&[("Test guide", 1), ("Setup", 2)], false)
            .paragraph(Para::new("Test guide").style("Heading1").bookmark("_Toc1"))
            .paragraph(Para::new("Setup").style("Heading2").bookmark("_Toc2"))
            .paragraph(Para::new("See ").hyperlink("the policy", URL)),
    );
    let found = findings(&original);
    assert_eq!(
        of(&found, RuleId::Lnk004).len(),
        1,
        "only the link in the text"
    );
    let applied = fix_all_links(&original);
    assert_only_the_splices_changed(&original, &applied);
    assert_eq!(
        screen_tips(fixed(&applied)),
        [None, None, Some("the policy".to_string())]
    );
}

// ---- LNK-005 ---------------------------------------------------------------

/// A document with a table of contents of this instruction.
fn toc_document(instruction: &str) -> Vec<u8> {
    build(
        &DocxBuilder::new()
            .toc_with(instruction, &[("Test guide", 1), ("Setup", 2)], false)
            .paragraph(Para::new("Test guide").style("Heading1").bookmark("_Toc1"))
            .paragraph(Para::new("Setup").style("Heading2").bookmark("_Toc2")),
    )
}

/// The instruction of the one table of contents of a .docx.
fn toc_instruction(bytes: &[u8]) -> String {
    let model = Document::from_bytes(bytes).unwrap();
    assert_eq!(model.tocs.len(), 1);
    model.tocs[0].instruction.clone()
}

fn fix_toc(bytes: &[u8]) -> fix::Applied {
    let found = findings(bytes);
    let requests = requests_for(&found, RuleId::Lnk005, "");
    assert!(!requests.is_empty());
    apply(bytes, &requests)
}

// S2-33
#[test]
fn fix_remove_toc_links_removes_the_h_switch_and_nothing_else() {
    let original = fixture(RuleId::Lnk005, true);
    let applied = fix_toc(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_only_the_splices_changed(&original, &applied);
    assert_eq!(
        applied.splices[DOCUMENT]
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>(),
        [""],
        "one deletion"
    );
    assert_eq!(
        toc_instruction(fixed(&applied)).trim(),
        r#"TOC \o "1-3" \z \u"#
    );
    assert!(applied.applied[0].summary.contains("\\h"));
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Lnk005).is_empty());
    assert!(after.is_empty(), "{after:#?}");
}

#[test]
fn fix_remove_toc_links_for_every_place_of_the_switch() {
    let cases = [
        (r#"TOC \o "1-3" \h \z \u"#, r#"TOC \o "1-3" \z \u"#),
        (r#"TOC \h \o "1-3""#, r#"TOC \o "1-3""#),
        (r#"TOC \o "1-3" \h"#, r#"TOC \o "1-3""#),
        (r"TOC \h", "TOC"),
        (r#"toc \o "1-3" \H \z"#, r#"toc \o "1-3" \z"#),
        (
            r#"TOC \o "1-3" \t "Callout \h,2" \h"#,
            r#"TOC \o "1-3" \t "Callout \h,2""#,
        ),
    ];
    for (before, after) in cases {
        let original = toc_document(before);
        assert_eq!(
            of(&findings(&original), RuleId::Lnk005).len(),
            1,
            "{before}"
        );
        let applied = fix_toc(&original);
        assert!(
            applied.refused.is_empty(),
            "{before}: {:?}",
            applied.refused
        );
        assert_only_the_splices_changed(&original, &applied);
        assert_eq!(toc_instruction(fixed(&applied)).trim(), after, "{before}");
        assert!(
            of(&findings(fixed(&applied)), RuleId::Lnk005).is_empty(),
            "{before}"
        );
    }
}

#[test]
fn fix_remove_toc_links_in_a_content_control() {
    let original = build(
        &DocxBuilder::new()
            .toc_content_control(&[("Test guide", 1)], false)
            .paragraph(Para::new("Test guide").style("Heading1").bookmark("_Toc1")),
    );
    let applied = fix_toc(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_only_the_splices_changed(&original, &applied);
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    assert_eq!(model.tocs.len(), 1);
    assert_eq!(model.tocs[0].instruction.trim(), r#"TOC \o "1-3" \z \u"#);
    assert!(of(&findings(fixed(&applied)), RuleId::Lnk005).is_empty());
}

#[test]
fn fix_remove_toc_links_changes_only_the_table_that_was_chosen() {
    let original = build(
        &DocxBuilder::new()
            .toc_with(r#"TOC \o "1-3" \h"#, &[("One", 1)], false)
            .toc_with(r#"TOC \o "1-2" \h"#, &[("One", 1)], false),
    );
    let found = findings(&original);
    let second = of(&found, RuleId::Lnk005).remove(1);
    let applied = apply(&original, &[FixRequest::confirm(&second).at(1)]);
    assert_only_the_splices_changed(&original, &applied);
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    let instructions: Vec<&str> = model.tocs.iter().map(|t| t.instruction.trim()).collect();
    assert_eq!(instructions, [r#"TOC \o "1-3" \h"#, r#"TOC \o "1-2""#]);
}

#[test]
fn fix_remove_toc_links_when_the_instruction_is_in_pieces() {
    let piece = |text: &str| {
        format!(r#"<w:r><w:instrText xml:space="preserve">{text}</w:instrText></w:r>"#)
    };
    let field = |first: &str, second: &str| {
        format!(
            r#"<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r>{}{}<w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>Entry</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#,
            piece(first),
            piece(second)
        )
    };
    // The switch is whole in the second piece.
    let original = raw(&field(r#" TOC \o &quot;1-3&quot; "#, r#"\h \z"#));
    let applied = fix_toc(&original);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_exactly_these_edits(
        &original,
        fixed(&applied),
        DOCUMENT,
        &[edit(
            r#"<w:instrText xml:space="preserve">\h \z</w:instrText>"#,
            // The space before the switch was in the piece before it.
            r#"<w:instrText xml:space="preserve"> \z</w:instrText>"#,
        )],
    );
    // The switch is split between two pieces: the tool does not guess.
    let split = raw(&field(r#" TOC \o &quot;1-3&quot; \"#, r#"h \z"#));
    assert_eq!(of(&findings(&split), RuleId::Lnk005).len(), 1);
    let applied = fix_toc(&split);
    assert!(applied.bytes.is_none());
    assert_eq!(applied.refused.len(), 1);
    assert!(
        applied.refused[0]
            .reason
            .contains("Use hyperlinks instead of page numbers"),
        "{}",
        applied.refused[0].reason
    );
}

#[test]
fn fix_remove_toc_links_keeps_the_old_entries_and_does_not_make_a_new_finding() {
    // The entries of the table are still links until Word updates the table at
    // export. They are not LNK-004 findings, before or after the fix.
    let original = toc_document(TOC_INSTRUCTION);
    assert!(
        fix::apply(&original, &[], &options())
            .unwrap()
            .bytes
            .is_none(),
        "no request, no change"
    );
    let before = findings(&original);
    let applied = fix_toc(&original);
    let after = findings(fixed(&applied));
    assert_eq!(new_rules(&before, &after), []);
    assert!(Document::from_bytes(fixed(&applied)).unwrap().links.len() == 2);
    assert!(of(&after, RuleId::Lnk004).is_empty());
    // A table that was made without links has nothing to fix.
    assert!(
        of(
            &findings(&toc_document(TOC_INSTRUCTION_NO_LINKS)),
            RuleId::Lnk005
        )
        .is_empty()
    );
}

/// The rules of the findings that are in `after` and not in `before`.
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

// ---- both fixes in one run, and `adapdf fix` -------------------------------

fn guide_with_both_problems() -> Vec<u8> {
    build(
        &clean_with_toc()
            .paragraph(Para::new("Read the ").hyperlink("parking policy", URL))
            .paragraph(Para::empty().field(&format!(r#"HYPERLINK "{URL}""#), "the same policy"))
            .paragraph(Para::empty().simple_field(&format!(r#"HYPERLINK "{URL}""#), "again")),
    )
}

/// A guide that has a table of contents with one heading.
fn clean_with_toc() -> DocxBuilder {
    DocxBuilder::new()
        .title("Adapdf test guide")
        .language("en-US")
        .template(adapdf::rules::document::TEMPLATE_FILE_NAME)
        .toc(&[("Test guide", 1)], false)
        .paragraph(Para::new("Test guide").style("Heading1").bookmark("_Toc1"))
}

#[test]
fn fix_both_link_fixes_in_one_run_clear_both_findings() {
    let original = guide_with_both_problems();
    let before = findings(&original);
    assert_eq!(of(&before, RuleId::Lnk004).len(), 3, "{before:#?}");
    assert_eq!(of(&before, RuleId::Lnk005).len(), 1);
    let mut requests = requests_for(&before, RuleId::Lnk004, "");
    requests.extend(requests_for(&before, RuleId::Lnk005, ""));
    let applied = apply(&original, &requests);
    assert!(applied.refused.is_empty(), "{:?}", applied.refused);
    assert_eq!(applied.applied.len(), 4);
    assert_only_the_splices_changed(&original, &applied);
    let after = findings(fixed(&applied));
    assert!(of(&after, RuleId::Lnk004).is_empty(), "{after:#?}");
    assert!(of(&after, RuleId::Lnk005).is_empty(), "{after:#?}");
    assert_eq!(new_rules(&before, &after), []);
    let model = Document::from_bytes(fixed(&applied)).unwrap();
    assert_eq!(model.tocs[0].instruction.trim(), r#"TOC \o "1-3" \z \u"#);
}

#[test]
fn fix_the_link_fixes_write_a_backup_and_the_new_file() {
    let original = guide_with_both_problems();
    let dir = TempDir::new("link-fixes");
    let path = dir.write("Guide.docx", &original);
    let found = findings(&original);
    let mut requests = requests_for(&found, RuleId::Lnk004, "");
    requests.extend(requests_for(&found, RuleId::Lnk005, ""));
    let outcome = fix_file(&path, &requests).expect("the run works");
    assert!(outcome.refused.is_empty());
    let backup = outcome.backup.expect("a backup");
    assert_eq!(fs::read(&backup).unwrap(), original);
    assert_ne!(fs::read(&path).unwrap(), original);
    assert!(of(&outcome.findings, RuleId::Lnk004).is_empty());
    assert!(of(&outcome.findings, RuleId::Lnk005).is_empty());
}

// C-09
#[test]
fn cli_fix_offers_both_link_fixes_and_applies_them() {
    let original = guide_with_both_problems();
    let dir = TempDir::new("cli-link-fixes");
    let path = dir.write("Guide.docx", &original);
    let mut child = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(["fix", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the adapdf binary runs");
    // The findings come in the order of the rules: LNK-004 three times, then
    // LNK-005. Answer yes to all four.
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"y\ny\ny\ny\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stderr.contains("4 problem(s) the tool can fix"), "{stderr}");
    assert!(
        stderr.contains("Copy the link text into the ScreenTip"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Make the table of contents without links"),
        "{stderr}"
    );
    assert!(stdout.contains("Fixes applied (4):"), "{stdout}");
    assert!(
        stdout.contains("[LNK-004] Added the ScreenTip \"parking policy\""),
        "{stdout}"
    );
    assert!(
        stdout.contains("[LNK-005] Removed the \\h switch"),
        "{stdout}"
    );
    assert!(stdout.contains("No problems found."), "{stdout}");
    let model = Document::open(&path).unwrap();
    assert!(model.links.iter().all(|l| l.has_screen_tip() || l.in_toc));
}

// C-10
#[test]
fn cli_fix_says_when_a_link_fix_cannot_be_done() {
    let original = build(&clean().paragraph(Para::empty().hyperlink("   ", URL)));
    let dir = TempDir::new("cli-link-refused");
    let path = dir.write("Guide.docx", &original);
    let mut child = Command::new(env!("CARGO_BIN_EXE_adapdf"))
        .args(["fix", path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("The tool cannot fix this now: The link has no text"),
        "{stderr}"
    );
    assert_eq!(fs::read(&path).unwrap(), original, "nothing was written");
    assert_eq!(dir.names(), ["Guide.docx"]);
}
