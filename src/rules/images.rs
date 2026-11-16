//! Image and object rules IMG-001 to IMG-007.
//!
//! The rules read [`Paragraph::drawings`]: DrawingML (`w:drawing`), VML
//! (`w:pict`) and embedded objects (`w:object`). Alt text is the description
//! (`descr`, or VML `alt`), or the title when there is no description. A
//! drawing marked decorative has no alt text and needs none. A text box with
//! text is not asked for alt text, because TXB-001 reports it. A group or a
//! canvas is one drawing, with its own alt text. The tool never writes alt
//! text; the fix for IMG-001 and IMG-002 asks the user.
//!
//! Annotated screenshots (IMG-007): staff draw arrows and boxes over a
//! screenshot in Word. [`screenshots`] finds each picture with the floating
//! shapes drawn on it. IMG-007 reports the shapes that have no alt text and
//! are not decorative, once for the screenshot. IMG-001 and IMG-004 then skip
//! those shapes (see `unresolved_annotations`), and TXB-001 reports the shapes
//! that have text once for the screenshot.

use std::collections::HashSet;

use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::{Document, Drawing, ObjectKind, Paragraph};

/// IMG-007: a floating shape is drawn on a picture when it is anchored in the
/// paragraph of the picture, or in a paragraph at most this many paragraphs
/// before or after it. The rule uses the paragraph the shape is anchored in,
/// not where the shape is on the page: the position would need page layout.
pub const ANNOTATION_MAX_PARAGRAPH_DISTANCE: usize = 1;

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
        ObjectKind::Group => "group of shapes",
        ObjectKind::Canvas => "drawing canvas",
        ObjectKind::SmartArt => "SmartArt graphic",
        ObjectKind::Ole => "embedded object",
    }
}

fn drawings(doc: &Document) -> impl Iterator<Item = (&Paragraph, &Drawing)> {
    doc.paragraphs
        .iter()
        .flat_map(|p| p.drawings.iter().map(move |d| (p, d)))
}

/// Where a drawing is: the paragraph index and its place in
/// [`Paragraph::drawings`].
pub(crate) type Slot = (usize, usize);

/// A picture with the floating shapes drawn on it. See the module notes.
pub(crate) struct Screenshot<'a> {
    /// The paragraph that holds the picture.
    pub paragraph: &'a Paragraph,
    /// The floating shapes (not pictures, not groups) anchored near the
    /// picture, with where they are.
    pub shapes: Vec<(Slot, &'a Drawing)>,
}

impl Screenshot<'_> {
    /// Shapes with no alt text that are not decorative and have no text: the
    /// shapes IMG-007 reports.
    pub fn unresolved(&self) -> impl Iterator<Item = Slot> + '_ {
        self.shapes
            .iter()
            .filter(|(_, d)| !d.has_box_text() && !d.decorative && d.alt_text().is_none())
            .map(|(slot, _)| *slot)
    }

    /// Shapes that have text and are not decorative: the shapes TXB-001
    /// reports for the screenshot. A decorative shape is an artifact, so its
    /// text is not read and is not reported.
    pub fn text_shapes(&self) -> impl Iterator<Item = &Drawing> + '_ {
        self.shapes
            .iter()
            .map(|(_, d)| *d)
            .filter(|d| d.has_box_text() && !d.decorative)
    }
}

/// Every picture that has a floating shape drawn on it, in document order. A
/// picture is inline or floating. A picture inside a group is not a picture
/// here (a group is one drawing), so a picture and shapes that are already
/// grouped are not a screenshot.
///
/// A shape belongs to one picture: the one in its own paragraph, else the one
/// in the paragraph before, else the one in the paragraph after. A paragraph
/// with two pictures counts as one screenshot.
pub(crate) fn screenshots(doc: &Document) -> Vec<Screenshot<'_>> {
    let mut found: Vec<Screenshot> = Vec::new();
    for p in &doc.paragraphs {
        if p.drawings.iter().any(|d| d.object == ObjectKind::Picture) {
            found.push(Screenshot {
                paragraph: p,
                shapes: Vec::new(),
            });
        }
    }
    for p in &doc.paragraphs {
        for (slot, d) in p.drawings.iter().enumerate() {
            if !(d.floating && d.object == ObjectKind::Shape) {
                continue;
            }
            let nearby = (0..=ANNOTATION_MAX_PARAGRAPH_DISTANCE).flat_map(|distance| {
                [
                    p.index.checked_sub(distance),
                    (distance > 0).then_some(p.index + distance),
                ]
            });
            let owner = nearby
                .flatten()
                .find_map(|i| found.iter().position(|s| s.paragraph.index == i));
            if let Some(owner) = owner {
                found[owner].shapes.push(((p.index, slot), d));
            }
        }
    }
    found.retain(|s| !s.shapes.is_empty());
    found
}

/// The shapes drawn on a screenshot that IMG-007 reports. IMG-001 and IMG-004
/// skip them, because IMG-007 replaces those findings.
fn unresolved_annotations(doc: &Document) -> HashSet<Slot> {
    screenshots(doc)
        .iter()
        .flat_map(|s| s.unresolved())
        .collect()
}

/// Every drawing with its place.
fn slotted_drawings(doc: &Document) -> impl Iterator<Item = (&Paragraph, Slot, &Drawing)> {
    doc.paragraphs.iter().flat_map(|p| {
        p.drawings
            .iter()
            .enumerate()
            .map(move |(slot, d)| (p, (p.index, slot), d))
    })
}

/// A drawing that a rule reports: its paragraph, its place and the drawing.
pub(crate) type Hit<'a> = (&'a Paragraph, Slot, &'a Drawing);

/// The drawings IMG-001 reports, in document order. The fix for IMG-001 uses
/// this list to find the drawing of a finding.
pub(crate) fn img_001_hits(doc: &Document) -> Vec<Hit<'_>> {
    let annotations = unresolved_annotations(doc);
    slotted_drawings(doc)
        .filter(|(_, slot, d)| {
            d.needs_alt_text()
                && !d.decorative
                && d.alt_text().is_none()
                && !annotations.contains(slot)
        })
        .collect()
}

/// IMG-001: a picture, chart, shape, group, canvas or SmartArt has no alt text
/// and is not marked decorative. A shape drawn on a screenshot is reported by
/// IMG-007 instead.
pub fn img_001(doc: &Document) -> Vec<Finding> {
    img_001_hits(doc)
        .into_iter()
        .map(|(p, _, d)| {
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

/// The drawings IMG-002 reports, in document order.
pub(crate) fn img_002_hits(doc: &Document) -> Vec<Hit<'_>> {
    slotted_drawings(doc)
        .filter(|(_, _, d)| d.alt_text().is_some_and(is_placeholder_alt))
        .collect()
}

/// IMG-002: the alt text looks like a file name, or is only a word such as
/// "image".
pub fn img_002(doc: &Document) -> Vec<Finding> {
    img_002_hits(doc)
        .into_iter()
        .map(|(p, _, d)| {
            let alt = d.alt_text().unwrap_or_default();
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

/// The drawings IMG-004 reports, in document order.
pub(crate) fn img_004_hits(doc: &Document) -> Vec<Hit<'_>> {
    let annotations = unresolved_annotations(doc);
    slotted_drawings(doc)
        .filter(|(_, slot, d)| {
            d.floating
                && !d.decorative
                && d.object != ObjectKind::Ole
                && !d.is_text_box_with_text()
                && !annotations.contains(slot)
        })
        .collect()
}

/// IMG-004: a floating object (not "In line with text") needs a place in the
/// reading order: a picture, chart, SmartArt, group, canvas or shape. Not
/// reported: a decorative object (an artifact has no reading order), a text
/// box with text (TXB-001 reports it) and a shape drawn on a screenshot that
/// IMG-007 reports.
pub fn img_004(doc: &Document) -> Vec<Finding> {
    img_004_hits(doc)
        .into_iter()
        .map(|(p, _, d)| {
            let label = label(d.object);
            Finding::new(
                RuleId::Img004,
                format!("A {label} floats instead of being in line with the text. Its place in the reading order may be wrong."),
                format!("Select the {label}, choose Layout Options, and select In Line with Text."),
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

/// IMG-007: a screenshot has arrows, boxes or lines drawn on it in Word that
/// are not grouped with it and are not marked decorative. One finding for each
/// screenshot, at the paragraph of the picture, with the number of shapes that
/// have no alt text. See [`screenshots`] for how shapes are matched to a
/// picture.
pub fn img_007(doc: &Document) -> Vec<Finding> {
    screenshots(doc)
        .iter()
        .filter_map(|s| {
            let count = s.unresolved().count();
            (count > 0).then(|| {
                let (things, them, they) = if count == 1 {
                    ("arrow or box", "it", "it has")
                } else {
                    ("arrows or boxes", "them", "they have")
                };
                Finding::new(
                    RuleId::Img007,
                    format!(
                        "This screenshot has {count} {things} drawn on it in Word. Screen readers cannot read {them}, and {they} no alt text."
                    ),
                    "Choose one way to fix it. (a) Make sure the alt text of the screenshot says what the arrows and boxes point to, for example \"The Save dialog. A red box shows the File name field.\" Then mark each arrow and box as decorative: right-click it, choose Edit Alt Text, and select Mark as decorative. (b) Select the screenshot and the arrows and boxes, right-click, and choose Group. Give the group alt text that describes the screenshot and what is highlighted, then set the group to In Line with Text. The tool can mark the arrows and boxes as decorative for you, but only after you confirm that the alt text of the screenshot describes what they highlight.",
                    Location::paragraph(doc, s.paragraph),
                )
            })
        })
        .collect()
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
