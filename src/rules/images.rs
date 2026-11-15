//! Image and object rules IMG-001 to IMG-006.
//!
//! The rules read [`Paragraph::drawings`]: DrawingML (`w:drawing`), VML
//! (`w:pict`) and embedded objects (`w:object`). Alt text is the description
//! (`descr`, or VML `alt`), or the title when there is no description. A
//! drawing marked decorative has no alt text and needs none. A text box with
//! text is not asked for alt text, because TXB-001 reports it. The tool never
//! writes alt text; the fix for IMG-001 and IMG-002 asks the user.

use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::{Document, Drawing, ObjectKind, Paragraph};

/// IMG-003: alt text with more characters than this is too long.
pub const MAX_ALT_TEXT_CHARS: usize = 250;

/// IMG-002: alt text that is only one of these words (ignoring case, spaces
/// and punctuation around it) says nothing about the image.
pub const GENERIC_ALT_WORDS: &[&str] = &[
    "image",
    "picture",
    "screenshot",
    "graphic",
    "photo",
    "figure",
];

/// IMG-002: alt text that starts with one of these and then has only a
/// number or date is a default name, such as `IMG_1234` or `Screenshot
/// 2024-03-05 101530`. The words in [`GENERIC_ALT_WORDS`] count too.
pub const FILE_NAME_PREFIXES: &[&str] = &[
    "image",
    "picture",
    "screenshot",
    "screen shot",
    "graphic",
    "photo",
    "figure",
    "img",
    "dsc",
    "dscn",
    "pic",
];

/// IMG-002: the characters allowed between the digits of a default name.
const FILE_NAME_SEPARATORS: &[char] = &[' ', '_', '-', '.', ':', '(', ')', '#'];

/// IMG-002: alt text that ends in one of these (after a dot) is a file name.
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "jfif", "gif", "bmp", "tif", "tiff", "svg", "emf", "wmf", "webp", "heic",
];

fn label(object: ObjectKind) -> &'static str {
    match object {
        ObjectKind::Picture => "picture",
        ObjectKind::Chart => "chart",
        ObjectKind::Shape => "shape",
        ObjectKind::SmartArt => "SmartArt graphic",
        ObjectKind::Ole => "embedded object",
    }
}

fn drawings(doc: &Document) -> impl Iterator<Item = (&Paragraph, &Drawing)> {
    doc.paragraphs
        .iter()
        .flat_map(|p| p.drawings.iter().map(move |d| (p, d)))
}

/// IMG-001: a picture, chart, shape or SmartArt has no alt text and is not
/// marked decorative.
pub fn img_001(doc: &Document) -> Vec<Finding> {
    drawings(doc)
        .filter(|(_, d)| d.needs_alt_text() && !d.decorative && d.alt_text().is_none())
        .map(|(p, d)| {
            Finding::new(
                RuleId::Img001,
                format!(
                    "A {} has no alt text and is not marked decorative. Screen reader users get nothing for it.",
                    label(d.object)
                ),
                "Right-click it and choose View Alt Text. Describe what it shows, for the purpose of this guide. If it adds no information, select Mark as decorative.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// True when alt text is a file name or a default name, or only a generic
/// word. See [`GENERIC_ALT_WORDS`], [`FILE_NAME_PREFIXES`] and
/// [`IMAGE_EXTENSIONS`].
pub(crate) fn is_placeholder_alt(alt: &str) -> bool {
    let lower = alt.trim().to_lowercase();
    let bare = lower.trim_matches(|c: char| !c.is_alphanumeric());
    if GENERIC_ALT_WORDS.contains(&bare) {
        return true;
    }
    let has_extension = lower
        .rsplit_once('.')
        .is_some_and(|(name, ext)| !name.is_empty() && IMAGE_EXTENSIONS.contains(&ext));
    has_extension
        || FILE_NAME_PREFIXES.iter().any(|prefix| {
            lower.strip_prefix(prefix).is_some_and(|rest| {
                rest.chars().any(|c| c.is_ascii_digit())
                    && rest
                        .chars()
                        .all(|c| c.is_ascii_digit() || FILE_NAME_SEPARATORS.contains(&c))
            })
        })
}

/// IMG-002: the alt text looks like a file name, or is only a word such as
/// "image".
pub fn img_002(doc: &Document) -> Vec<Finding> {
    drawings(doc)
        .filter_map(|(p, d)| Some((p, d.alt_text().filter(|alt| is_placeholder_alt(alt))?)))
        .map(|(p, alt)| {
            Finding::new(
                RuleId::Img002,
                format!(
                    "The alt text \"{}\" looks like a file name or a placeholder. It does not describe the image.",
                    snippet(alt)
                ),
                "Replace it with a short description of what the image shows, for the purpose of this guide.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// IMG-003: alt text longer than [`MAX_ALT_TEXT_CHARS`] characters.
pub fn img_003(doc: &Document) -> Vec<Finding> {
    drawings(doc)
        .filter_map(|(p, d)| {
            let length = d.alt_text()?.chars().count();
            (length > MAX_ALT_TEXT_CHARS).then_some((p, length))
        })
        .map(|(p, length)| {
            Finding::new(
                RuleId::Img003,
                format!(
                    "The alt text has {length} characters. Alt text longer than {MAX_ALT_TEXT_CHARS} characters is hard to listen to."
                ),
                "Shorten the alt text to the main point. Put the detail in the body text or in a caption.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// IMG-004: a picture is floating (not "In line with text"). Charts, shapes
/// and SmartArt are not checked, because the spec names images.
pub fn img_004(doc: &Document) -> Vec<Finding> {
    drawings(doc)
        .filter(|(_, d)| d.object == ObjectKind::Picture && d.floating)
        .map(|(p, _)| {
            Finding::new(
                RuleId::Img004,
                "A picture floats instead of being in line with the text. Its place in the reading order may be wrong.",
                "Select the picture, choose Layout Options, and select In Line with Text.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// Lower case, spaces collapsed, and no punctuation around the text.
fn normalise(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

/// True for a paragraph in the Caption style.
fn is_caption(p: &Paragraph) -> bool {
    p.style_name
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case("caption"))
}

/// IMG-005: the alt text is the same as the caption next to it. The caption
/// is the paragraph before or after the image, in the Caption style. The
/// texts are the same when they match after case, spaces and punctuation at
/// the ends are ignored. A caption label such as "Figure 1:" is not removed.
pub fn img_005(doc: &Document) -> Vec<Finding> {
    drawings(doc)
        .filter_map(|(p, d)| {
            let alt = normalise(d.alt_text()?);
            let neighbours = [p.index.checked_sub(1), Some(p.index + 1)];
            let caption = neighbours
                .into_iter()
                .flatten()
                .filter_map(|i| doc.paragraphs.get(i))
                .filter(|c| is_caption(c))
                .find(|c| !alt.is_empty() && normalise(&c.text) == alt)?;
            Some((p, caption))
        })
        .map(|(p, caption)| {
            Finding::new(
                RuleId::Img005,
                format!(
                    "The alt text is the same as the caption \"{}\". A screen reader reads the same words twice.",
                    snippet(&caption.text)
                ),
                "Change the alt text to describe what the image shows, or mark the image decorative if the caption says everything.",
                Location::paragraph(doc, p),
            )
        })
        .collect()
}

/// IMG-006: the document has an embedded object (OLE) or an equation. One
/// finding for the document, at the first one, with the counts.
pub fn img_006(doc: &Document) -> Vec<Finding> {
    let objects = drawings(doc)
        .filter(|(_, d)| d.object == ObjectKind::Ole)
        .count();
    let equations: usize = doc.paragraphs.iter().map(|p| p.equations).sum();
    let Some(first) = doc
        .paragraphs
        .iter()
        .find(|p| p.equations > 0 || p.drawings.iter().any(|d| d.object == ObjectKind::Ole))
    else {
        return Vec::new();
    };
    let mut found = Vec::new();
    if objects > 0 {
        found.push(format!(
            "{objects} embedded object{}",
            if objects == 1 { "" } else { "s" }
        ));
    }
    if equations > 0 {
        found.push(format!(
            "{equations} equation{}",
            if equations == 1 { "" } else { "s" }
        ));
    }
    vec![Finding::new(
        RuleId::Img006,
        format!(
            "The document has {}. Embedded objects and equations may not be read correctly in the PDF.",
            found.join(" and ")
        ),
        "Check how they read in the PDF with the PAC screen-reader preview. If they are wrong, replace an equation with text, or an object with a table or a picture that has alt text.",
        Location::paragraph(doc, first),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders() {
        for alt in [
            "image",
            "Image.",
            "  PICTURE ",
            "Screenshot",
            "graphic",
            "Photo",
            "figure",
            "IMG_1234",
            "img_1234.jpg",
            "DSC00123",
            "Screenshot 2024-03-05 101530",
            "Screenshot (12)",
            "Picture 1",
            "Image1.png",
            "Setup screen.PNG",
            "scan.tiff",
        ] {
            assert!(is_placeholder_alt(alt), "{alt}");
        }
    }

    #[test]
    fn real_descriptions_are_not_placeholders() {
        for alt in [
            "Screenshot of the login page",
            "Image 3 shows the printer",
            "A picture of a cat",
            "Images",
            "Figure of merit",
            "Front panel with the power button circled",
            "Save as png",
            "",
        ] {
            assert!(!is_placeholder_alt(alt), "{alt}");
        }
    }

    #[test]
    fn normalise_ignores_case_spacing_and_end_punctuation() {
        assert_eq!(normalise("  The  Front\tPanel. "), "the front panel");
        assert_eq!(normalise("..."), "");
    }
}
