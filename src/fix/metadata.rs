//! Fixes in the document properties and in `word/styles.xml`: the title
//! (DOC-001), the default language (DOC-002), and the heading styles that
//! HDG-003 applies.

use crate::docx::{CORE_PART, DC_NS, STYLES_PART, W_NS, WordNode};

use super::xml::{
    append_child, escape_attr, escape_text, insert_children_in_order, prefix_of, prepend_child,
    set_w_attr, tag, w_name,
};
use super::{Engine, Reason};

/// The children of `w:rPr` in schema order (ECMA-376, `CT_RPr`).
pub(super) const RPR_ORDER: &[&str] = &[
    "rStyle",
    "rFonts",
    "b",
    "bCs",
    "i",
    "iCs",
    "caps",
    "smallCaps",
    "strike",
    "dstrike",
    "outline",
    "shadow",
    "emboss",
    "imprint",
    "noProof",
    "snapToGrid",
    "vanish",
    "webHidden",
    "color",
    "spacing",
    "w",
    "kern",
    "position",
    "sz",
    "szCs",
    "highlight",
    "u",
    "effect",
    "bdr",
    "shd",
    "fitText",
    "vertAlign",
    "rtl",
    "cs",
    "em",
    "lang",
    "eastAsianLayout",
    "specVanish",
    "oMath",
    "rPrChange",
];

/// Word's definition of the built-in styles Heading 1 to Heading 4 (the
/// Microsoft 365 default template), without the link to a character style.
/// `{base}` is the paragraph style they are based on.
fn heading_style_body(level: u8) -> Option<&'static str> {
    Some(match level {
        1 => {
            r#"<w:uiPriority w:val="9"/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="240" w:after="0"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:rFonts w:asciiTheme="majorHAnsi" w:eastAsiaTheme="majorEastAsia" w:hAnsiTheme="majorHAnsi" w:cstheme="majorBidi"/><w:color w:val="2F5496" w:themeColor="accent1" w:themeShade="BF"/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr>"#
        }
        2 => {
            r#"<w:uiPriority w:val="9"/><w:unhideWhenUsed/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="40" w:after="0"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:rFonts w:asciiTheme="majorHAnsi" w:eastAsiaTheme="majorEastAsia" w:hAnsiTheme="majorHAnsi" w:cstheme="majorBidi"/><w:color w:val="2F5496" w:themeColor="accent1" w:themeShade="BF"/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr>"#
        }
        3 => {
            r#"<w:uiPriority w:val="9"/><w:unhideWhenUsed/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="40" w:after="0"/><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:rFonts w:asciiTheme="majorHAnsi" w:eastAsiaTheme="majorEastAsia" w:hAnsiTheme="majorHAnsi" w:cstheme="majorBidi"/><w:color w:val="1F3763" w:themeColor="accent1" w:themeShade="7F"/><w:sz w:val="24"/><w:szCs w:val="24"/></w:rPr>"#
        }
        4 => {
            r#"<w:uiPriority w:val="9"/><w:unhideWhenUsed/><w:qFormat/><w:pPr><w:keepNext/><w:keepLines/><w:spacing w:before="40" w:after="0"/><w:outlineLvl w:val="3"/></w:pPr><w:rPr><w:rFonts w:asciiTheme="majorHAnsi" w:eastAsiaTheme="majorEastAsia" w:hAnsiTheme="majorHAnsi" w:cstheme="majorBidi"/><w:i/><w:iCs/><w:color w:val="2F5496" w:themeColor="accent1" w:themeShade="BF"/></w:rPr>"#
        }
        _ => return None,
    })
}

/// True for a language tag such as `en-US`: letters, digits and hyphens, with
/// a language of 2 or 3 letters first.
fn is_language_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let language = parts.next().unwrap_or("");
    (2..=3).contains(&language.len())
        && language.chars().all(|c| c.is_ascii_alphabetic())
        && parts.all(|p| (1..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

impl Engine<'_> {
    /// `SetTitle` (DOC-001): the typed title goes in `dc:title` in
    /// `docProps/core.xml`.
    pub(super) fn set_title(&mut self, title: &str) -> Result<String, Reason> {
        let core = self.core.ok_or(
            "The document has no properties part (docProps/core.xml). In Word, choose File > Info and type a title in the Title box.",
        )?;
        let root = core.xml.root_element();
        let escaped = escape_text(title);
        let existing = root.descendants().find(|n| {
            n.is_element()
                && n.tag_name().namespace() == Some(DC_NS)
                && n.tag_name().name() == "title"
        });
        let sp = self.splices_for(CORE_PART);
        match existing {
            Some(element) => {
                let t = tag(core.text, element);
                match t.close_start {
                    Some(close) => sp.replace(t.open_end..close, escaped),
                    None => append_child(sp, core.text, element, &escaped),
                }
            }
            None => {
                let xml = match prefix_of(root, DC_NS) {
                    Some(p) => format!("<{p}:title>{escaped}</{p}:title>"),
                    None => format!(r#"<dc:title xmlns:dc="{DC_NS}">{escaped}</dc:title>"#),
                };
                append_child(sp, core.text, root, &xml);
            }
        }
        Ok("Set the document title.".to_string())
    }

    /// `SetDefaultLanguage` (DOC-002): `w:lang` in the document defaults of
    /// `word/styles.xml`.
    pub(super) fn set_default_language(&mut self) -> Result<String, Reason> {
        let language = self.options.default_language.clone();
        if !is_language_tag(&language) {
            return Err(format!(
                "\"{language}\" is not a language tag such as en-US."
            ));
        }
        let styles = self.styles.ok_or(
            "The document has no styles part (word/styles.xml). In Word, press Ctrl+A, choose Review > Language > Set Proofing Language, and pick the language.",
        )?;
        let text = styles.text;
        let root = styles.xml.root_element();
        let lang_xml = format!(
            r#"<{} {}="{}"/>"#,
            w_name(root, "lang"),
            w_name(root, "val"),
            escape_attr(&language)
        );
        let wrap = |name: &str, inner: &str| format!("<{0}>{inner}</{0}>", w_name(root, name));
        let rpr_default = |lang: &str| wrap("rPrDefault", &wrap("rPr", lang));
        let sp = self.splices_for(STYLES_PART);
        match root.w_child("docDefaults") {
            None => prepend_child(
                sp,
                text,
                root,
                &wrap("docDefaults", &rpr_default(&lang_xml)),
            ),
            Some(defaults) => match defaults.w_child("rPrDefault") {
                None => prepend_child(sp, text, defaults, &rpr_default(&lang_xml)),
                Some(rpr_default_node) => match rpr_default_node.w_child("rPr") {
                    None => append_child(sp, text, rpr_default_node, &wrap("rPr", &lang_xml)),
                    Some(rpr) => match rpr.w_child("lang") {
                        Some(lang) => set_w_attr(sp, text, lang, "val", &language),
                        None => insert_children_in_order(
                            sp,
                            text,
                            rpr,
                            RPR_ORDER,
                            &[("lang", lang_xml)],
                        ),
                    },
                },
            },
        }
        Ok(format!("Set the document language to {language}."))
    }

    /// The id of the style for Heading `level`. The style the document has is
    /// used. Otherwise Word's built-in definition is added to `styles.xml`
    /// (decided point 4).
    pub(super) fn heading_style(&mut self, level: u8) -> Result<String, Reason> {
        if let Some(id) = self.heading_styles.get(&level) {
            return Ok(id.clone());
        }
        let styles = self.styles.ok_or(
            "The document has no styles part (word/styles.xml), so the heading style cannot be added. Apply the heading style in Word.",
        )?;
        let root = styles.xml.root_element();
        let wanted = format!("heading {level}");
        let existing = root.children().filter(|n| n.is_w("style")).find(|style| {
            style.w_attr("type").is_none_or(|t| t == "paragraph")
                && style
                    .w_child("name")
                    .and_then(|n| n.w_attr("val"))
                    .is_some_and(|name| name.eq_ignore_ascii_case(&wanted))
        });
        if let Some(id) = existing.and_then(|s| s.w_attr("styleId")) {
            self.heading_styles.insert(level, id.to_string());
            return Ok(id.to_string());
        }
        let body = heading_style_body(level)
            .ok_or_else(|| format!("The tool has no definition of Heading {level}."))?;
        let id_taken = |id: &str| {
            root.children()
                .any(|s| s.is_w("style") && s.w_attr("styleId") == Some(id))
        };
        let mut id = format!("Heading{level}");
        if id_taken(&id) {
            id = format!("AdapdfHeading{level}");
        }
        let default_style = self.doc.styles.default_paragraph_id();
        let based_on = default_style
            .map(|d| {
                format!(
                    r#"<w:basedOn w:val="{0}"/><w:next w:val="{0}"/>"#,
                    escape_attr(d)
                )
            })
            .unwrap_or_default();
        let declaration = if prefix_of(root, W_NS) == Some("w") {
            String::new()
        } else {
            format!(r#" xmlns:w="{W_NS}""#)
        };
        let xml = format!(
            r#"<w:style{declaration} w:type="paragraph" w:styleId="{id}"><w:name w:val="heading {level}"/>{based_on}{body}</w:style>"#
        );
        let sp = self.splices_for(STYLES_PART);
        append_child(sp, styles.text, root, &xml);
        self.heading_styles.insert(level, id.clone());
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_tags() {
        for ok in ["en-US", "en", "fil", "zh-Hans-CN", "es-419"] {
            assert!(is_language_tag(ok), "{ok}");
        }
        for bad in ["", "e", "english", "en_US", "en-", "-US", "en US", "en-\"x"] {
            assert!(!is_language_tag(bad), "{bad}");
        }
    }

    #[test]
    fn every_heading_definition_is_well_formed() {
        for level in 1..=4 {
            let body = heading_style_body(level).unwrap();
            let xml = format!(
                r#"<w:style xmlns:w="{W_NS}" w:styleId="H"><w:name w:val="heading {level}"/>{body}</w:style>"#
            );
            roxmltree::Document::parse(&xml).unwrap();
        }
        assert!(heading_style_body(5).is_none());
    }
}
