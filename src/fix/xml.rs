//! Editing XML text with byte-range splices.
//!
//! A fix never writes a part again from a tree. It lists splices (a byte
//! range of the original text and the text that replaces it), and
//! [`Splices::apply`] puts them in. Bytes outside the ranges stay the same.
//! The ranges come from `roxmltree`, which knows where every node is in the
//! source text.

use std::ops::Range;

use roxmltree::Node;

use crate::docx::{MC_NS, W_NS, WordNode};

/// One edit: the bytes in `range` become `text`. An empty range inserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    pub range: Range<usize>,
    pub text: String,
}

/// Two fixes changed the same bytes in different ways.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub first: Range<usize>,
    pub second: Range<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct Splices(Vec<Splice>);

impl Splices {
    pub fn insert(&mut self, at: usize, text: impl Into<String>) {
        self.replace(at..at, text);
    }

    pub fn delete(&mut self, range: Range<usize>) {
        self.replace(range, "");
    }

    pub fn replace(&mut self, range: Range<usize>, text: impl Into<String>) {
        self.0.push(Splice {
            range,
            text: text.into(),
        });
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The splices in text order, ready to apply. A splice inside a deletion is
    /// dropped (the deletion wins), and an identical splice that was added
    /// twice is kept once. Other overlaps are a [`Conflict`].
    pub fn resolve(&self) -> Result<Vec<Splice>, Conflict> {
        let mut items = self.0.clone();
        items.sort_by(|a, b| {
            (a.range.start, a.range.end, &a.text).cmp(&(b.range.start, b.range.end, &b.text))
        });
        items.dedup();
        let deletions: Vec<Range<usize>> = items
            .iter()
            .filter(|s| s.text.is_empty() && !s.range.is_empty())
            .map(|s| s.range.clone())
            .collect();
        let inside = |s: &Splice, d: &Range<usize>| {
            if s.range.is_empty() {
                d.start < s.range.start && s.range.start < d.end
            } else {
                *d != s.range && d.start <= s.range.start && s.range.end <= d.end
            }
        };
        items.retain(|s| !deletions.iter().any(|d| inside(s, d)));
        for pair in items.windows(2) {
            if pair[0].range.end > pair[1].range.start {
                return Err(Conflict {
                    first: pair[0].range.clone(),
                    second: pair[1].range.clone(),
                });
            }
        }
        Ok(items)
    }

    /// The text with every splice applied.
    pub fn apply(&self, original: &str) -> Result<String, Conflict> {
        Ok(apply_resolved(original, &self.resolve()?))
    }
}

/// Applies splices that are in text order and do not overlap.
pub fn apply_resolved(original: &str, splices: &[Splice]) -> String {
    let mut out = String::with_capacity(original.len());
    let mut at = 0;
    for s in splices {
        out.push_str(&original[at..s.range.start]);
        out.push_str(&s.text);
        at = s.range.end;
    }
    out.push_str(&original[at..]);
    out
}

/// Escapes text for element content.
pub fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Escapes text for an attribute value in double quotes. Tabs and line breaks
/// are written as character references, because a raw one would be read back
/// as a space.
pub fn escape_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
    out
}

/// Where the tags of an element are in the source text.
#[derive(Debug, Clone, Copy)]
pub struct Tag {
    /// Start of the element (`<`).
    pub start: usize,
    /// The end of the qualified name in the start tag.
    pub name_end: usize,
    /// The end of the last attribute (or of the name): where a new attribute
    /// goes.
    pub attrs_end: usize,
    /// The first byte after the start tag.
    pub open_end: usize,
    /// For `<a/>`: the position of the `/`.
    pub slash: Option<usize>,
    /// For `<a>...</a>`: the position of `</`.
    pub close_start: Option<usize>,
    /// The end of the element.
    pub end: usize,
}

impl Tag {
    pub fn self_closing(&self) -> bool {
        self.slash.is_some()
    }
}

/// Reads the tags of an element from the source text.
pub fn tag(text: &str, node: Node) -> Tag {
    let range = node.range();
    let bytes = text.as_bytes();
    let mut i = range.start + 1;
    while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/') {
        i += 1;
    }
    let name_end = i;
    let mut attrs_end = name_end;
    let (open_end, slash) = loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        match bytes.get(i) {
            Some(b'>') => break (i + 1, None),
            Some(b'/') => break (i + 2, Some(i)),
            Some(_) => {
                while i < bytes.len() && bytes[i] != b'=' {
                    i += 1;
                }
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                i += 1;
                attrs_end = i;
            }
            None => break (bytes.len(), None),
        }
    };
    let close_start = if slash.is_some() {
        None
    } else {
        text[..range.end].rfind("</")
    };
    Tag {
        start: range.start,
        name_end,
        attrs_end,
        open_end,
        slash,
        close_start,
        end: range.end,
    }
}

/// The name of an element as written in the source, such as `w:p`.
pub fn source_name<'t>(text: &'t str, node: Node) -> &'t str {
    let t = tag(text, node);
    &text[t.start + 1..t.name_end]
}

/// Adds `xml` as the last child of `node`. An element written as `<a/>` is
/// opened first.
pub fn append_child(sp: &mut Splices, text: &str, node: Node, xml: &str) {
    let t = tag(text, node);
    match (t.slash, t.close_start) {
        (Some(slash), _) => sp.replace(
            slash..t.end,
            format!(">{xml}</{}>", &text[t.start + 1..t.name_end]),
        ),
        (None, Some(close)) => sp.insert(close, xml),
        (None, None) => {}
    }
}

/// Adds `xml` as the first child of `node`.
pub fn prepend_child(sp: &mut Splices, text: &str, node: Node, xml: &str) {
    let t = tag(text, node);
    if t.self_closing() {
        append_child(sp, text, node, xml);
    } else {
        sp.insert(t.open_end, xml);
    }
}

/// The prefix that `node` uses for a namespace, such as `w`. `None` when the
/// namespace is the default namespace or is not declared.
pub fn prefix_of<'a>(node: Node<'a, '_>, ns: &str) -> Option<&'a str> {
    node.lookup_prefix(ns).filter(|p| !p.is_empty())
}

/// `prefix:local` for an element or attribute in a namespace, using the prefix
/// that is in scope at `node`.
pub fn qname(node: Node, ns: &str, local: &str) -> String {
    match prefix_of(node, ns) {
        Some(prefix) => format!("{prefix}:{local}"),
        None => local.to_string(),
    }
}

/// `w:local` as the document at `node` writes it.
pub fn w_name(node: Node, local: &str) -> String {
    match prefix_of(node, W_NS) {
        Some(prefix) => format!("{prefix}:{local}"),
        None => format!("w:{local}"),
    }
}

/// Sets an attribute (`ns` is `None` for one with no prefix). An existing value
/// is replaced; a new attribute is written after the last one.
pub fn set_attr(
    sp: &mut Splices,
    text: &str,
    node: Node,
    ns: Option<&str>,
    local: &str,
    new_name: &str,
    value: &str,
) {
    let value = escape_attr(value);
    match node
        .attributes()
        .find(|a| a.namespace() == ns && a.name() == local)
    {
        Some(attr) => sp.replace(attr.range_value(), value),
        None => sp.insert(
            tag(text, node).attrs_end,
            format!(r#" {new_name}="{value}""#),
        ),
    }
}

/// Sets `w:<local>` on a WordprocessingML element.
pub fn set_w_attr(sp: &mut Splices, text: &str, node: Node, local: &str, value: &str) {
    set_attr(
        sp,
        text,
        node,
        Some(W_NS),
        local,
        &w_name(node, local),
        value,
    );
}

/// Adds new children to an element that has a fixed child order. Each child
/// goes before the first existing child that must come after it, or at the end.
/// `order` lists the local names in schema order. Children whose local names are
/// not in `order` are treated as coming last.
///
/// `new` is `(local name, XML)`. Children for the same place are written in
/// schema order.
pub fn insert_children_in_order(
    sp: &mut Splices,
    text: &str,
    parent: Node,
    order: &[&str],
    new: &[(&str, String)],
) {
    if new.is_empty() {
        return;
    }
    let rank = |local: &str| {
        order
            .iter()
            .position(|o| *o == local)
            .unwrap_or(order.len())
    };
    let mut new: Vec<&(&str, String)> = new.iter().collect();
    new.sort_by_key(|(local, _)| rank(local));
    if tag(text, parent).self_closing() {
        let xml: String = new.iter().map(|(_, xml)| xml.as_str()).collect();
        append_child(sp, text, parent, &xml);
        return;
    }
    let close = tag(text, parent).close_start.unwrap_or(parent.range().end);
    let existing: Vec<(usize, usize)> = parent
        .children()
        .filter(Node::is_element)
        .map(|c| (rank(c.tag_name().name()), c.range().start))
        .collect();
    let mut by_place: Vec<(usize, String)> = Vec::new();
    for (local, xml) in new {
        let at = existing
            .iter()
            .filter(|(r, _)| *r > rank(local))
            .map(|(_, start)| *start)
            .min()
            .unwrap_or(close);
        match by_place.iter_mut().find(|(place, _)| *place == at) {
            Some((_, joined)) => joined.push_str(xml),
            None => by_place.push((at, xml.clone())),
        }
    }
    for (at, xml) in by_place {
        sp.insert(at, xml);
    }
}

/// True for a toggle element (`w:cantSplit`, `w:tblHeader`) that is on.
pub fn toggle_on(node: Node) -> bool {
    !matches!(node.w_attr("val"), Some("0" | "false" | "off" | "none"))
}

/// True for a paragraph or table in a text box or in the fallback copy of
/// `mc:AlternateContent`. The model does not read these.
pub fn in_alternative_content(node: Node) -> bool {
    node.ancestors().any(|a| {
        a.is_w("txbxContent")
            || (a.is_element()
                && a.tag_name().namespace() == Some(MC_NS)
                && a.tag_name().name() == "Fallback")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splices(items: &[(Range<usize>, &str)]) -> Splices {
        let mut s = Splices::default();
        for (range, text) in items {
            s.replace(range.clone(), *text);
        }
        s
    }

    #[test]
    fn applies_in_text_order() {
        let s = splices(&[(6..7, "W"), (0..0, ">>"), (3..3, "-")]);
        assert_eq!(s.apply("hello world").unwrap(), ">>hel-lo World");
    }

    #[test]
    fn a_splice_inside_a_deletion_is_dropped() {
        let s = splices(&[(2..8, ""), (4..4, "X"), (5..6, "Y")]);
        assert_eq!(s.apply("0123456789").unwrap(), "0189");
    }

    #[test]
    fn an_insert_at_the_edge_of_a_deletion_stays() {
        let s = splices(&[(2..8, ""), (2..2, "X"), (8..8, "Z")]);
        assert_eq!(s.apply("0123456789").unwrap(), "01XZ89");
    }

    #[test]
    fn the_same_splice_twice_counts_once() {
        let s = splices(&[(2..4, ""), (2..4, "")]);
        assert_eq!(s.apply("0123456789").unwrap(), "01456789");
    }

    #[test]
    fn different_overlapping_splices_conflict() {
        let s = splices(&[(2..6, "A"), (4..8, "B")]);
        assert!(s.apply("0123456789").is_err());
    }

    #[test]
    fn escapes() {
        assert_eq!(escape_text("a<b>&c\"d"), "a&lt;b&gt;&amp;c\"d");
        assert_eq!(escape_attr("a\"<&\n\té"), "a&quot;&lt;&amp;&#10;&#9;é");
    }

    #[test]
    fn finds_the_tags_of_an_element() {
        let text = r#"<r xmlns:w="x"><w:p a="1>2" b='3'>hi</w:p><w:q /></r>"#;
        let doc = roxmltree::Document::parse(text).unwrap();
        let p = doc.root_element().first_element_child().unwrap();
        let t = tag(text, p);
        assert_eq!(&text[t.start..t.name_end], "<w:p");
        assert_eq!(&text[t.open_end - 1..t.open_end], ">");
        assert_eq!(&text[t.close_start.unwrap()..t.end], "</w:p>");
        let q = doc.root_element().last_element_child().unwrap();
        let t = tag(text, q);
        assert_eq!(t.slash.map(|s| &text[s..t.end]), Some("/>"));
    }

    #[test]
    fn opens_and_appends_to_a_self_closing_element() {
        let text = r#"<a xmlns:w="x"><w:b/></a>"#;
        let doc = roxmltree::Document::parse(text).unwrap();
        let b = doc.root_element().first_element_child().unwrap();
        let mut s = Splices::default();
        append_child(&mut s, text, b, "<c/>");
        assert_eq!(
            s.apply(text).unwrap(),
            r#"<a xmlns:w="x"><w:b><c/></w:b></a>"#
        );
    }

    #[test]
    fn children_go_in_schema_order() {
        let text = r#"<p><b/><d/></p>"#;
        let doc = roxmltree::Document::parse(text).unwrap();
        let p = doc.root_element();
        let mut s = Splices::default();
        insert_children_in_order(
            &mut s,
            text,
            p,
            &["a", "b", "c", "d", "e"],
            &[
                ("e", "<e/>".into()),
                ("c", "<c/>".into()),
                ("a", "<a/>".into()),
            ],
        );
        assert_eq!(s.apply(text).unwrap(), "<p><a/><b/><c/><d/><e/></p>");
    }
}
