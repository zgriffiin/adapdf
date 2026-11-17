//! Links, bookmarks, tables of contents, the page colour, and the text of
//! headers and footers. These live in fields (`w:fldChar`, `w:fldSimple`) and
//! in elements around the paragraphs, so they have their own pass over the XML.
//!
//! Field codes are read in document order with a stack, because a field can
//! span several paragraphs (a table of contents does) and fields can nest.

use std::ops::Range;

use roxmltree::Node;

use super::R_NS;
use super::body::{belongs_to, is_alternative_content, scan_run};
use super::props::themed;
use super::theme::{Rgb, Theme};
use super::{Drawing, Paragraph};
use crate::docx::{DOCUMENT_PART, MC_NS, Relationship, WordNode, parse_xml};
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// A web address, file or e-mail address. Empty when the relationship
    /// that `r:id` names is missing.
    External(String),
    /// A bookmark in this document (`w:anchor`, or `HYPERLINK \l`).
    Internal(String),
}

/// Where a link is in `word/document.xml`, for a fix that changes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkSource {
    /// The byte range of the `w:hyperlink` or `w:fldSimple` element.
    Element(Range<usize>),
    /// The byte ranges of the `w:instrText` elements of a complex field, in
    /// order.
    Field(Vec<Range<usize>>),
}

/// A link in the body: a `w:hyperlink` element or a `HYPERLINK` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Index of the paragraph where the link starts.
    pub paragraph: usize,
    /// The visible text of the link.
    pub text: String,
    pub target: LinkTarget,
    /// The ScreenTip as written: `w:tooltip` on a `w:hyperlink`, or the `\o`
    /// switch of a `HYPERLINK` field. `None` when there is none.
    pub screen_tip: Option<String>,
    /// The link is an entry of a table of contents: it is in the result of a
    /// `TOC` field or in a "Table of Contents" content control.
    pub in_toc: bool,
    pub source: LinkSource,
}

impl Link {
    /// The link has a ScreenTip that is not empty and not only spaces.
    pub fn has_screen_tip(&self) -> bool {
        self.screen_tip
            .as_deref()
            .is_some_and(|tip| !tip.trim().is_empty())
    }
}

/// A table of contents: a `TOC` field, or a content control (`w:sdt`) whose
/// building block gallery is "Table of Contents".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toc {
    /// Index of the first paragraph of the table of contents.
    pub first_paragraph: usize,
    /// The field instruction, for example `TOC \o "1-3" \h \z \u`. Empty
    /// when a content control holds no `TOC` field.
    pub instruction: String,
    /// The byte ranges of the `w:instrText` elements that make the
    /// instruction, in order. Empty when `instruction` is.
    pub instruction_ranges: Vec<Range<usize>>,
    /// The field is marked dirty (`w:dirty`): Word has not updated it since it
    /// was changed.
    pub dirty: bool,
    /// The text of each entry paragraph (the TOC 1 to TOC 9 styles), with the
    /// page number and tabs still in it.
    pub entries: Vec<String>,
}

/// Text in a header or footer, with the field it is the result of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFooterText {
    pub text: String,
    /// The keyword of the innermost field that makes this text, upper case,
    /// for example `PAGE`. `None` for text that is typed in.
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderFooter {
    /// The part name, for example `word/header1.xml`.
    pub part: String,
    /// The visible text, in order, in pieces: the runs of one paragraph that
    /// belong to the same field (or to no field) make one piece. Pieces that
    /// are only spaces or tabs are left out.
    pub text: Vec<HeaderFooterText>,
}

/// What the pass over `word/document.xml` finds.
#[derive(Debug, Clone, Default)]
pub struct BodyFields {
    pub links: Vec<Link>,
    /// The `w:name` of every `w:bookmarkStart`.
    pub bookmarks: Vec<String>,
    pub tocs: Vec<Toc>,
    /// The page colour (`w:background`). Word shows it only when the
    /// document setting `displayBackgroundShape` is on; the caller decides.
    pub page_background: Option<Rgb>,
}

/// The first word of a field instruction, upper case: `PAGE`, `TOC`.
pub fn keyword(instruction: &str) -> String {
    instruction
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase()
}

struct Token {
    text: String,
    quoted: bool,
    /// The byte range in the instruction. A quoted word includes its quotes.
    start: usize,
    end: usize,
}

/// Splits a field instruction into words. A quoted string is one word. In a
/// quoted string, `\"` is a quote mark and `\\` is a backslash; any other
/// backslash stays as it is. The flag is true when a quoted string is not
/// closed.
fn scan(instruction: &str) -> (Vec<Token>, bool) {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut chars = instruction.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if in_quotes && c == '\\' && matches!(chars.peek(), Some((_, '"' | '\\'))) {
            current.extend(chars.next().map(|(_, escaped)| escaped));
        } else if c == '"' {
            if in_quotes {
                out.push(Token {
                    text: std::mem::take(&mut current),
                    quoted: true,
                    start,
                    end: i + 1,
                });
            } else if !current.is_empty() {
                out.push(Token {
                    text: std::mem::take(&mut current),
                    quoted: false,
                    start,
                    end: i,
                });
            }
            in_quotes = !in_quotes;
            if in_quotes {
                start = i;
            }
        } else if c.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                out.push(Token {
                    text: std::mem::take(&mut current),
                    quoted: false,
                    start,
                    end: i,
                });
            }
        } else {
            if current.is_empty() && !in_quotes {
                start = i;
            }
            current.push(c);
        }
    }
    if !current.is_empty() {
        out.push(Token {
            text: current,
            quoted: in_quotes,
            start,
            end: instruction.len(),
        });
    }
    (out, in_quotes)
}

fn tokens(instruction: &str) -> Vec<Token> {
    scan(instruction).0
}

/// True when the instruction ends inside a quoted string that is not closed.
pub fn has_open_quote(instruction: &str) -> bool {
    scan(instruction).1
}

/// The text as one quoted word of a field instruction, with `\` and `"`
/// written as `\\` and `\"`. [`switch_arg`] reads it back as the same text.
pub fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if matches!(c, '"' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// The byte range to delete to remove the first switch of that name: the switch
/// and the spaces before it. For `TOC \o "1-3" \h \z` and `h` it is the range of
/// ` \h`. `None` when the instruction has no such switch.
pub fn switch_span(instruction: &str, name: &str) -> Option<Range<usize>> {
    let all = tokens(instruction);
    let token = all.iter().find(|t| is_switch(t, name))?;
    let before = instruction[..token.start].trim_end_matches(char::is_whitespace);
    Some(before.len()..token.end)
}

fn is_switch(token: &Token, name: &str) -> bool {
    !token.quoted
        && token
            .text
            .strip_prefix('\\')
            .is_some_and(|s| s.eq_ignore_ascii_case(name))
}

/// True when the instruction has the switch, for example `has_switch(i, "u")`
/// for `\u`.
pub fn has_switch(instruction: &str, name: &str) -> bool {
    tokens(instruction).iter().any(|t| is_switch(t, name))
}

/// The word after a switch, for example `1-3` for `\o "1-3"`.
pub fn switch_arg(instruction: &str, name: &str) -> Option<String> {
    let all = tokens(instruction);
    let at = all.iter().position(|t| is_switch(t, name))?;
    let next = all.get(at + 1)?;
    (next.quoted || !next.text.starts_with('\\')).then(|| next.text.clone())
}

/// The target of a `HYPERLINK` field instruction.
fn hyperlink_target(instruction: &str) -> Option<LinkTarget> {
    let all = tokens(instruction);
    let mut file = None;
    let mut anchor = None;
    let mut i = 1; // the first word is HYPERLINK
    while i < all.len() {
        let t = &all[i];
        if !t.quoted && t.text.starts_with('\\') {
            // \l \o \t take a value. The others (\m \n) do not.
            let takes_value = ["l", "o", "t"].iter().any(|s| is_switch(t, s));
            if is_switch(t, "l") {
                anchor = all.get(i + 1).map(|v| v.text.clone());
            }
            i += if takes_value { 2 } else { 1 };
        } else {
            file.get_or_insert_with(|| t.text.clone());
            i += 1;
        }
    }
    match (file, anchor) {
        (Some(file), Some(anchor)) => Some(LinkTarget::External(format!("{file}#{anchor}"))),
        (Some(file), None) => Some(LinkTarget::External(file)),
        (None, Some(anchor)) => Some(LinkTarget::Internal(anchor)),
        (None, None) => None,
    }
}

#[derive(Default)]
struct OpenField {
    instruction: String,
    instruction_ranges: Vec<Range<usize>>,
    dirty: bool,
    in_result: bool,
    text: String,
    start_paragraph: usize,
}

struct ClosedField {
    instruction: String,
    instruction_ranges: Vec<Range<usize>>,
    dirty: bool,
    text: String,
    start_paragraph: usize,
    end_paragraph: usize,
}

/// The complex fields (`fldChar` begin, separate, end) that are open now.
#[derive(Default)]
struct FieldStack {
    open: Vec<OpenField>,
}

impl FieldStack {
    fn begin(&mut self, dirty: bool, paragraph: usize) {
        self.open.push(OpenField {
            dirty,
            start_paragraph: paragraph,
            ..OpenField::default()
        });
    }

    /// Adds the text of a `w:instrText` element, and its byte range.
    fn instruction(&mut self, text: &str, range: Range<usize>) {
        if let Some(field) = self.open.last_mut().filter(|f| !f.in_result) {
            field.instruction.push_str(text);
            field.instruction_ranges.push(range);
        }
    }

    fn separate(&mut self) {
        if let Some(field) = self.open.last_mut() {
            field.in_result = true;
        }
    }

    fn end(&mut self, paragraph: usize) -> Option<ClosedField> {
        let field = self.open.pop()?;
        Some(ClosedField {
            instruction: field.instruction,
            instruction_ranges: field.instruction_ranges,
            dirty: field.dirty,
            text: field.text,
            start_paragraph: field.start_paragraph,
            end_paragraph: paragraph,
        })
    }

    /// Text that shows. It is part of the result of every open field that is
    /// past its instruction.
    fn visible(&mut self, text: &str) {
        for field in self.open.iter_mut().filter(|f| f.in_result) {
            field.text.push_str(text);
        }
    }

    fn innermost_keyword(&self) -> Option<String> {
        self.open.last().map(|f| keyword(&f.instruction))
    }
}

fn is_on(value: Option<&str>) -> bool {
    matches!(value, Some("1" | "true" | "on"))
}

/// Reads one run: field characters, field instructions and text. `emit` gets
/// every piece of visible text, with the stack as it is at that point.
/// Returns the fields that this run closes.
fn feed_run(
    run: Node,
    paragraph: usize,
    stack: &mut FieldStack,
    emit: &mut dyn FnMut(&str, &FieldStack),
) -> Vec<ClosedField> {
    let mut closed = Vec::new();
    let has_field_parts = run
        .children()
        .any(|c| c.is_w("fldChar") || c.is_w("instrText"));
    if !has_field_parts {
        let mut text = String::new();
        scan_run(run, &mut text, &mut Vec::<Drawing>::new());
        if !text.is_empty() {
            stack.visible(&text);
            emit(&text, stack);
        }
        return closed;
    }
    // Word writes a field character in a run of its own. A run that mixes
    // them with text is read child by child; only `w:t` text is kept.
    for child in run.children().filter(Node::is_element) {
        if child.is_w("fldChar") {
            match child.w_attr("fldCharType") {
                Some("begin") => stack.begin(is_on(child.w_attr("dirty")), paragraph),
                Some("separate") => stack.separate(),
                Some("end") => closed.extend(stack.end(paragraph)),
                _ => {}
            }
        } else if child.is_w("instrText") {
            stack.instruction(child.text().unwrap_or(""), child.range());
        } else if child.is_w("t") {
            let text = child.text().unwrap_or("");
            stack.visible(text);
            emit(text, stack);
        }
    }
    closed
}

/// The text of the runs under `node` that belong to paragraph `p`.
fn visible_text(node: Node, p: Node) -> String {
    let mut text = String::new();
    let mut drawings = Vec::new();
    for run in node
        .descendants()
        .filter(|n| n.is_w("r") && belongs_to(*n, p) && !is_alternative_content(*n))
    {
        scan_run(run, &mut text, &mut drawings);
    }
    text
}

fn is_toc_content_control(sdt: Node) -> bool {
    sdt.w_child("sdtPr")
        .and_then(|n| n.w_child("docPartObj"))
        .and_then(|n| n.w_child("docPartGallery"))
        .and_then(|n| n.w_attr("val"))
        .is_some_and(|v| v.eq_ignore_ascii_case("Table of Contents"))
}

/// True when `node` is an entry of a table of contents: it is in a "Table of
/// Contents" content control, or in the result of a `TOC` field that is open.
fn in_toc(node: Node, stack: &FieldStack) -> bool {
    stack
        .open
        .iter()
        .any(|f| f.in_result && keyword(&f.instruction) == "TOC")
        || node
            .ancestors()
            .any(|a| a.is_w("sdt") && is_toc_content_control(a))
}

/// True for the built-in styles TOC 1 to TOC 9 (not TOC Heading).
fn is_toc_entry_style(name: &str) -> bool {
    name.to_ascii_lowercase()
        .strip_prefix("toc ")
        .is_some_and(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit())
}

fn make_toc(
    paragraphs: &[Paragraph],
    first: usize,
    last: usize,
    instruction: String,
    instruction_ranges: Vec<Range<usize>>,
    dirty: bool,
) -> Toc {
    let entries = paragraphs[first..=last.min(paragraphs.len() - 1)]
        .iter()
        .filter(|p| p.style_name.as_deref().is_some_and(is_toc_entry_style) && !p.is_blank())
        .map(|p| p.text.clone())
        .collect();
    Toc {
        first_paragraph: first,
        instruction,
        instruction_ranges,
        dirty,
        entries,
    }
}

/// Reads links, bookmarks, tables of contents and the page colour from
/// `word/document.xml`. `paragraphs` are the paragraphs that the body parser
/// found in the same text; the paragraph indexes in the result refer to them.
pub(super) fn scan_body(
    text: &str,
    paragraphs: &[Paragraph],
    theme: &Theme,
    relationships: &[Relationship],
) -> Result<BodyFields> {
    let doc = parse_xml(DOCUMENT_PART, text)?;
    let root = doc.root_element();
    let mut found = BodyFields {
        page_background: root.w_child("background").and_then(|n| {
            themed(&n, theme, "themeColor", "themeShade", "themeTint")
                .or_else(|| n.w_attr("color").and_then(Rgb::from_hex))
        }),
        bookmarks: doc
            .descendants()
            .filter(|n| n.is_w("bookmarkStart"))
            .filter_map(|n| n.w_attr("name"))
            .map(str::to_string)
            .collect(),
        ..BodyFields::default()
    };
    let Some(body) = root.w_child("body") else {
        return Ok(found);
    };

    // The same paragraphs, in the same order, as the body parser.
    let nodes: Vec<Node> = body
        .descendants()
        .filter(|n| n.is_w("p") && !is_alternative_content(*n))
        .collect();
    debug_assert_eq!(nodes.len(), paragraphs.len());

    let mut stack = FieldStack::default();
    let mut field_tocs = Vec::new();
    for (index, &p) in nodes.iter().enumerate() {
        for node in p.descendants().filter(|n| belongs_to(*n, p)) {
            if node.is_w("hyperlink") {
                let target = if let Some(id) = node.attribute((R_NS, "id")) {
                    let target = relationships.iter().find(|r| r.id == id);
                    Some(LinkTarget::External(
                        target.map(|r| r.target.clone()).unwrap_or_default(),
                    ))
                } else {
                    node.w_attr("anchor")
                        .map(|a| LinkTarget::Internal(a.to_string()))
                };
                if let Some(target) = target {
                    found.links.push(Link {
                        paragraph: index,
                        text: visible_text(node, p),
                        target,
                        screen_tip: node.w_attr("tooltip").map(str::to_string),
                        in_toc: in_toc(node, &stack),
                        source: LinkSource::Element(node.range()),
                    });
                }
            } else if node.is_w("fldSimple") {
                let instruction = node.w_attr("instr").unwrap_or("");
                if keyword(instruction) == "HYPERLINK"
                    && let Some(target) = hyperlink_target(instruction)
                {
                    found.links.push(Link {
                        paragraph: index,
                        text: visible_text(node, p),
                        target,
                        screen_tip: switch_arg(instruction, "o"),
                        in_toc: in_toc(node, &stack),
                        source: LinkSource::Element(node.range()),
                    });
                }
            } else if node.is_w("r") && !is_alternative_content(node) {
                for field in feed_run(node, index, &mut stack, &mut |_, _| {}) {
                    match keyword(&field.instruction).as_str() {
                        "HYPERLINK" => {
                            if let Some(target) = hyperlink_target(&field.instruction) {
                                found.links.push(Link {
                                    paragraph: field.start_paragraph,
                                    text: field.text,
                                    target,
                                    screen_tip: switch_arg(&field.instruction, "o"),
                                    in_toc: in_toc(node, &stack),
                                    source: LinkSource::Field(field.instruction_ranges),
                                });
                            }
                        }
                        "TOC" => field_tocs.push(field),
                        _ => {}
                    }
                }
            }
        }
    }

    // A content control can hold a TOC field. The control gives the range of
    // paragraphs; the field gives the switches. Report them once.
    let mut covered = Vec::new();
    for sdt in body
        .descendants()
        .filter(|n| n.is_w("sdt") && is_toc_content_control(*n))
    {
        let inside: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.ancestors().any(|a| a == sdt))
            .map(|(i, _)| i)
            .collect();
        let (Some(&first), Some(&last)) = (inside.first(), inside.last()) else {
            continue;
        };
        let field = field_tocs
            .iter()
            .find(|f| (first..=last).contains(&f.start_paragraph));
        let marked_dirty = sdt
            .descendants()
            .any(|n| (n.is_w("fldChar") || n.is_w("fldSimple")) && is_on(n.w_attr("dirty")));
        found.tocs.push(make_toc(
            paragraphs,
            first,
            last,
            field.map(|f| f.instruction.clone()).unwrap_or_default(),
            field
                .map(|f| f.instruction_ranges.clone())
                .unwrap_or_default(),
            marked_dirty || field.is_some_and(|f| f.dirty),
        ));
        covered.push(first..=last);
    }
    for field in field_tocs {
        if !covered.iter().any(|r| r.contains(&field.start_paragraph)) {
            found.tocs.push(make_toc(
                paragraphs,
                field.start_paragraph,
                field.end_paragraph,
                field.instruction,
                field.instruction_ranges,
                field.dirty,
            ));
        }
    }
    found.tocs.sort_by_key(|t| t.first_paragraph);
    Ok(found)
}

fn in_fallback(node: Node) -> bool {
    node.ancestors().any(|a| {
        a.is_element()
            && a.tag_name().namespace() == Some(MC_NS)
            && a.tag_name().name() == "Fallback"
    })
}

/// Reads the visible text of a header or footer part, with the field that
/// each piece of text comes from.
pub(super) fn scan_header_footer(part: &str, text: &str) -> Result<HeaderFooter> {
    let doc = parse_xml(part, text)?;
    let mut stack = FieldStack::default();
    let mut pieces: Vec<HeaderFooterText> = Vec::new();
    let mut last_paragraph = None;
    for run in doc
        .descendants()
        .filter(|n| n.is_w("r") && !in_fallback(*n))
    {
        let paragraph = run.ancestors().find(|a| a.is_w("p")).map(|n| n.id());
        let simple = run
            .ancestors()
            .find(|a| a.is_w("fldSimple"))
            .and_then(|n| n.w_attr("instr"))
            .map(keyword);
        feed_run(run, 0, &mut stack, &mut |text, stack| {
            // Runs of one paragraph and one field make one piece, because
            // Word can split a word over several runs.
            let field = simple.clone().or_else(|| stack.innermost_keyword());
            match pieces.last_mut() {
                Some(last) if last_paragraph == paragraph && last.field == field => {
                    last.text.push_str(text);
                }
                _ => pieces.push(HeaderFooterText {
                    text: text.to_string(),
                    field,
                }),
            }
            last_paragraph = paragraph;
        });
    }
    pieces.retain(|p| !p.text.trim().is_empty());
    Ok(HeaderFooter {
        part: part.to_string(),
        text: pieces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::zip_stored;
    use crate::model::Document;

    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

    fn load(body: &str, rels: &str) -> Document {
        let document = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
        let rels = format!(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
        );
        Document::from_bytes(&zip_stored(&[
            ("word/document.xml", document.as_bytes()),
            ("word/_rels/document.xml.rels", rels.as_bytes()),
        ]))
        .unwrap()
    }

    fn run(text: &str) -> String {
        format!("<w:r><w:t>{text}</w:t></w:r>")
    }

    fn fld(kind: &str) -> String {
        format!(r#"<w:r><w:fldChar w:fldCharType="{kind}"/></w:r>"#)
    }

    fn instr(text: &str) -> String {
        format!(r#"<w:r><w:instrText xml:space="preserve"> {text} </w:instrText></w:r>"#)
    }

    #[test]
    fn instruction_words_and_switches() {
        let i = r#"TOC \o "1-3" \h \z \u"#;
        assert_eq!(keyword(i), "TOC");
        assert_eq!(keyword("  page \\* MERGEFORMAT"), "PAGE");
        assert_eq!(switch_arg(i, "o").as_deref(), Some("1-3"));
        assert_eq!(switch_arg(i, "h"), None, "the next word is a switch");
        assert!(has_switch(i, "u"));
        assert!(!has_switch(i, "t"));
        assert!(has_switch(r"TOC \U", "u"), "switch names ignore case");
    }

    #[test]
    fn hyperlink_instructions() {
        use LinkTarget::*;
        let ext = |s: &str| Some(External(s.to_string()));
        assert_eq!(
            hyperlink_target(r#"HYPERLINK "https://example.org/a""#),
            ext("https://example.org/a")
        );
        assert_eq!(
            hyperlink_target(r#"HYPERLINK \l "Intro""#),
            Some(Internal("Intro".to_string()))
        );
        assert_eq!(
            hyperlink_target(r#"HYPERLINK "other.docx" \l "Intro""#),
            ext("other.docx#Intro")
        );
        assert_eq!(
            hyperlink_target(r#"HYPERLINK "C:\dir\f.docx" \o "tip" \t "_blank""#),
            ext(r"C:\dir\f.docx")
        );
        assert_eq!(hyperlink_target("HYPERLINK"), None);
    }

    #[test]
    fn hyperlink_elements_with_relationship_and_anchor() {
        let body = format!(
            r#"<w:p><w:hyperlink r:id="rId5">{}</w:hyperlink>{}<w:hyperlink w:anchor="Intro">{}</w:hyperlink></w:p><w:p><w:bookmarkStart w:id="0" w:name="Intro"/>{}<w:bookmarkEnd w:id="0"/></w:p>"#,
            run("Parking"),
            run(" and "),
            run("see "),
            run("Intro"),
        );
        let doc = load(
            &body,
            r#"<Relationship Id="rId5" Type="x/hyperlink" Target="https://example.org/p" TargetMode="External"/>"#,
        );
        assert_eq!(doc.links.len(), 2);
        assert_eq!(doc.links[0].text, "Parking");
        assert_eq!(
            doc.links[0].target,
            LinkTarget::External("https://example.org/p".to_string())
        );
        assert_eq!(doc.links[1].text, "see ");
        assert_eq!(
            doc.links[1].target,
            LinkTarget::Internal("Intro".to_string())
        );
        assert_eq!(doc.bookmarks, ["Intro"]);
        assert!(doc.paragraphs[0].runs[0].in_hyperlink);
        assert!(!doc.paragraphs[0].runs[1].in_hyperlink);
        assert!(doc.paragraphs[0].runs[2].in_hyperlink);
    }

    #[test]
    fn a_missing_relationship_gives_an_empty_target() {
        let doc = load(
            &format!(
                r#"<w:p><w:hyperlink r:id="rId9">{}</w:hyperlink></w:p>"#,
                run("x")
            ),
            "",
        );
        assert_eq!(doc.links[0].target, LinkTarget::External(String::new()));
    }

    #[test]
    fn simple_and_complex_hyperlink_fields() {
        let body = format!(
            r#"<w:p><w:fldSimple w:instr=' HYPERLINK \l "Top" '>{}</w:fldSimple></w:p><w:p>{}{}{}{}{}</w:p>"#,
            run("Go up"),
            fld("begin"),
            instr(r#"HYPERLINK "https://example.org""#),
            fld("separate"),
            run("Example"),
            fld("end"),
        );
        let doc = load(&body, "");
        assert_eq!(doc.links.len(), 2);
        assert_eq!(doc.links[0].text, "Go up");
        assert_eq!(doc.links[0].target, LinkTarget::Internal("Top".to_string()));
        assert_eq!(doc.links[1].text, "Example");
        assert_eq!(
            doc.links[1].target,
            LinkTarget::External("https://example.org".to_string())
        );
        assert_eq!(doc.links[1].paragraph, 1);
    }

    #[test]
    fn a_field_that_is_not_a_hyperlink_is_not_a_link() {
        let doc = load(
            &format!(
                "<w:p>{}{}{}{}{}</w:p>",
                fld("begin"),
                instr("PAGE"),
                fld("separate"),
                run("3"),
                fld("end"),
            ),
            "",
        );
        assert!(doc.links.is_empty());
        assert!(doc.tocs.is_empty());
    }

    #[test]
    fn a_toc_field_spans_paragraphs_and_keeps_its_entries() {
        let style = |n: u8| format!(r#"<w:pPr><w:pStyle w:val="TOC{n}"/></w:pPr>"#);
        let styles = format!(
            r#"<w:styles {NS}><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
            <w:style w:type="paragraph" w:styleId="TOC1"><w:name w:val="toc 1"/></w:style>
            <w:style w:type="paragraph" w:styleId="TOCHeading"><w:name w:val="TOC Heading"/></w:style></w:styles>"#
        );
        let body = format!(
            r#"<w:p><w:pPr><w:pStyle w:val="TOCHeading"/></w:pPr>{}</w:p>"#,
            run("Contents")
        ) + &format!(
            r#"<w:p>{}{}{}{}{}{}</w:p>"#,
            style(1),
            r#"<w:r><w:fldChar w:fldCharType="begin" w:dirty="true"/></w:r>"#,
            instr(r#"TOC \o "1-2" \h"#),
            fld("separate"),
            run("Setup"),
            "<w:r><w:tab/></w:r>",
        ) + &format!(r#"<w:p>{}{}</w:p>"#, style(1), run("Use"))
            + &format!("<w:p>{}</w:p>", fld("end"));
        let document = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
        let doc = Document::from_bytes(&zip_stored(&[
            ("word/document.xml", document.as_bytes()),
            ("word/styles.xml", styles.as_bytes()),
        ]))
        .unwrap();
        assert_eq!(doc.tocs.len(), 1);
        let toc = &doc.tocs[0];
        assert_eq!(toc.first_paragraph, 1);
        assert!(toc.dirty);
        assert_eq!(toc.instruction.trim(), r#"TOC \o "1-2" \h"#);
        assert_eq!(toc.entries, ["Setup\t", "Use"]);
    }

    #[test]
    fn a_content_control_with_a_toc_field_is_one_toc() {
        let body = format!(
            r#"<w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/></w:docPartObj></w:sdtPr><w:sdtContent>
            <w:p>{}{}{}{}{}</w:p><w:p>{}</w:p></w:sdtContent></w:sdt>"#,
            fld("begin"),
            instr("TOC"),
            fld("separate"),
            run("One"),
            "",
            fld("end"),
        );
        let doc = load(&body, "");
        assert_eq!(doc.tocs.len(), 1, "{:#?}", doc.tocs);
        assert_eq!(doc.tocs[0].first_paragraph, 0);
        assert_eq!(doc.tocs[0].instruction.trim(), "TOC");
        assert!(!doc.tocs[0].dirty);
    }

    #[test]
    fn a_page_colour_is_read_with_its_theme_tint() {
        let doc = load(
            r#"<w:p/>"#,
            "", // no theme: a theme colour is unknown, so w:color is used
        );
        assert_eq!(doc.page_background, None);
        let xml = format!(
            r#"<w:document {NS}><w:background w:color="FFFF99"/><w:body><w:p/></w:body></w:document>"#
        );
        let doc =
            Document::from_bytes(&zip_stored(&[("word/document.xml", xml.as_bytes())])).unwrap();
        assert_eq!(doc.page_background, Rgb::from_hex("FFFF99"));
    }

    #[test]
    fn header_text_keeps_the_field_it_comes_from() {
        let part = format!(
            r#"<w:hdr {NS}><w:p>{}<w:fldSimple w:instr=" DATE \@ &quot;d MMMM&quot; ">{}</w:fldSimple>{}{}{}{}{}</w:p></w:hdr>"#,
            run("Guide "),
            run("5 May"),
            fld("begin"),
            instr("PAGE"),
            fld("separate"),
            run("2"),
            fld("end"),
        );
        let hf = scan_header_footer("word/header1.xml", &part).unwrap();
        let got: Vec<(&str, Option<&str>)> = hf
            .text
            .iter()
            .map(|t| (t.text.as_str(), t.field.as_deref()))
            .collect();
        assert_eq!(
            got,
            [
                ("Guide ", None),
                ("5 May", Some("DATE")),
                ("2", Some("PAGE"))
            ]
        );
    }

    #[test]
    fn a_run_that_mixes_field_characters_and_text_is_read_in_order() {
        let part = format!(
            r#"<w:hdr {NS}><w:p><w:r><w:fldChar w:fldCharType="begin"/><w:instrText>PAGE</w:instrText><w:fldChar w:fldCharType="separate"/><w:t>7</w:t><w:fldChar w:fldCharType="end"/><w:t>after</w:t></w:r></w:p></w:hdr>"#
        );
        let hf = scan_header_footer("word/footer1.xml", &part).unwrap();
        assert_eq!(hf.text[0].field.as_deref(), Some("PAGE"));
        assert_eq!(hf.text[1].field, None);
    }

    #[test]
    fn toc_entry_style_names() {
        assert!(is_toc_entry_style("toc 1"));
        assert!(is_toc_entry_style("TOC 9"));
        assert!(!is_toc_entry_style("TOC Heading"));
        assert!(!is_toc_entry_style("toc 10"));
        assert!(!is_toc_entry_style("Normal"));
    }

    #[test]
    fn quoted_words_read_escaped_quotes_and_backslashes() {
        let i = r#"HYPERLINK \l "Top" \o "The \"top\" of C:\\dir""#;
        assert_eq!(
            switch_arg(i, "o").as_deref(),
            Some(r#"The "top" of C:\dir"#)
        );
        // The target is still the bookmark, not a word of the ScreenTip.
        assert_eq!(
            hyperlink_target(i),
            Some(LinkTarget::Internal("Top".to_string()))
        );
        // A backslash before any other character stays (a path).
        assert_eq!(
            hyperlink_target(r#"HYPERLINK "C:\dir\f.docx" \o "tip""#),
            Some(LinkTarget::External(r"C:\dir\f.docx".to_string()))
        );
        assert!(!has_open_quote(i));
        assert!(has_open_quote(r#"HYPERLINK "https://example.org"#));
        assert!(has_open_quote(r#"HYPERLINK \o "a \" b"#));
    }

    #[test]
    fn quoted_text_is_read_back_as_the_same_text() {
        for text in [
            "Parking policy",
            r#"He said "stop" & left <now>"#,
            r"C:\temp\file.docx",
            r#"ends with a backslash\"#,
            r#"\"#,
            "é 日本 \u{1F600}",
            "",
        ] {
            let instruction = format!(r#"HYPERLINK \l "Top" \o {}"#, quoted(text));
            assert_eq!(
                switch_arg(&instruction, "o").as_deref(),
                Some(text),
                "{instruction}"
            );
            assert!(!has_open_quote(&instruction), "{instruction}");
            assert_eq!(
                hyperlink_target(&instruction),
                Some(LinkTarget::Internal("Top".to_string())),
                "{instruction}"
            );
        }
    }

    #[test]
    fn a_switch_span_covers_the_switch_and_the_spaces_before_it() {
        let cut = |i: &str, name: &str| {
            switch_span(i, name).map(|s| format!("{}{}", &i[..s.start], &i[s.end..]))
        };
        let i = r#" TOC \o "1-3" \h \z \u "#;
        assert_eq!(cut(i, "h").as_deref(), Some(r#" TOC \o "1-3" \z \u "#));
        assert_eq!(cut(r"TOC \h", "h").as_deref(), Some("TOC"));
        assert_eq!(cut(r"TOC   \h   \z", "H").as_deref(), Some(r"TOC   \z"));
        assert_eq!(cut(i, "n"), None);
        // Only a switch counts: not a word in quotes, not a longer switch.
        assert_eq!(cut(r#"TOC \t "a \h b""#, "h"), None);
        assert_eq!(cut(r"TOC \hz", "h"), None);
        // The first one only.
        assert_eq!(cut(r"TOC \h \h", "h").as_deref(), Some(r"TOC \h"));
    }

    #[test]
    fn links_keep_their_screen_tip_source_and_place_in_a_toc() {
        let body = format!(
            r#"<w:p><w:hyperlink r:id="rId5" w:tooltip="Parking page">{}</w:hyperlink></w:p><w:p><w:hyperlink w:anchor="Intro">{}</w:hyperlink></w:p><w:p><w:fldSimple w:instr=' HYPERLINK \l "Top" \o "Back" '>{}</w:fldSimple></w:p><w:p>{}{}{}{}{}</w:p>"#,
            run("Parking"),
            run("Intro"),
            run("Up"),
            fld("begin"),
            instr(r#"HYPERLINK "https://example.org""#),
            fld("separate"),
            run("Example"),
            fld("end"),
        );
        let rels = r#"<Relationship Id="rId5" Type="x/hyperlink" Target="https://example.org/p" TargetMode="External"/>"#;
        let document = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
        let doc = load(&body, rels);
        let tips: Vec<Option<&str>> = doc.links.iter().map(|l| l.screen_tip.as_deref()).collect();
        assert_eq!(tips, [Some("Parking page"), None, Some("Back"), None]);
        let has: Vec<bool> = doc.links.iter().map(Link::has_screen_tip).collect();
        assert_eq!(has, [true, false, true, false]);
        assert!(doc.links.iter().all(|l| !l.in_toc));
        // The ranges point at the elements in the text of the part.
        let text_of = |range: &Range<usize>| &document[range.clone()];
        match &doc.links[0].source {
            LinkSource::Element(r) => assert!(text_of(r).starts_with("<w:hyperlink r:id=")),
            other => panic!("{other:?}"),
        }
        match &doc.links[2].source {
            LinkSource::Element(r) => assert!(text_of(r).starts_with("<w:fldSimple")),
            other => panic!("{other:?}"),
        }
        match &doc.links[3].source {
            LinkSource::Field(ranges) => {
                assert_eq!(ranges.len(), 1);
                assert!(text_of(&ranges[0]).starts_with("<w:instrText"));
                assert!(text_of(&ranges[0]).contains("HYPERLINK"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_blank_screen_tip_is_not_a_screen_tip() {
        let body = format!(
            r#"<w:p><w:hyperlink w:anchor="a" w:tooltip="">{}</w:hyperlink><w:hyperlink w:anchor="b" w:tooltip="  ">{}</w:hyperlink></w:p>"#,
            run("A"),
            run("B")
        );
        let doc = load(&body, "");
        assert_eq!(doc.links[0].screen_tip.as_deref(), Some(""));
        assert!(doc.links.iter().all(|l| !l.has_screen_tip()));
    }

    #[test]
    fn links_in_a_toc_field_result_or_content_control_are_toc_entries() {
        let toc_begin = format!(
            "{}{}{}",
            fld("begin"),
            instr(r#"TOC \o "1-3" \h"#),
            fld("separate")
        );
        let body = format!(
            r#"<w:p>{toc_begin}<w:hyperlink w:anchor="_Toc1">{}</w:hyperlink></w:p><w:p>{}{}{}{}{}{}</w:p><w:p>{}</w:p><w:p><w:hyperlink w:anchor="Later">{}</w:hyperlink></w:p><w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/></w:docPartObj></w:sdtPr><w:sdtContent><w:p><w:hyperlink w:anchor="_Toc2">{}</w:hyperlink></w:p></w:sdtContent></w:sdt>"#,
            run("One"),
            fld("begin"),
            instr(r#"HYPERLINK \l "_Toc2""#),
            fld("separate"),
            run("Two"),
            fld("end"),
            "",
            fld("end"),
            run("After"),
            run("Three"),
        );
        let doc = load(&body, "");
        let seen: Vec<(&str, bool)> = doc
            .links
            .iter()
            .map(|l| (l.text.as_str(), l.in_toc))
            .collect();
        assert_eq!(
            seen,
            [
                ("One", true),
                ("Two", true),
                ("After", false),
                ("Three", true)
            ]
        );
        // The field, and the content control that holds no field.
        assert_eq!(doc.tocs.len(), 2);
        assert_eq!(doc.tocs[0].instruction_ranges.len(), 1);
        assert!(doc.tocs[1].instruction_ranges.is_empty());
    }

    #[test]
    fn a_toc_keeps_the_ranges_of_its_instruction() {
        let body = format!(
            "<w:p>{}{}{}{}{}</w:p>",
            fld("begin"),
            r#"<w:r><w:instrText xml:space="preserve"> TOC \o "1-3" </w:instrText></w:r>"#,
            r#"<w:r><w:instrText xml:space="preserve">\h \z</w:instrText></w:r>"#,
            fld("separate"),
            fld("end"),
        );
        let document = format!("<w:document {NS}><w:body>{body}</w:body></w:document>");
        let doc = load(&body, "");
        assert_eq!(doc.tocs[0].instruction, r#" TOC \o "1-3" \h \z"#);
        let pieces: Vec<&str> = doc.tocs[0]
            .instruction_ranges
            .iter()
            .map(|r| &document[r.clone()])
            .collect();
        assert_eq!(pieces.len(), 2);
        assert!(pieces[0].ends_with(r#"</w:instrText>"#) && pieces[0].contains(r#""1-3""#));
        assert!(pieces[1].contains(r"\h \z"));
    }
}
