//! One-click suggestions for the title (DOC-001), taken from the document itself.
//!
//! The tool never makes up a title. It offers up to three texts that are
//! already there: the file name, the first paragraph in the Title style, and the
//! first Heading 1. Nothing is filled in until the person chooses one, and the
//! person can change the text afterwards. Alt text has no suggestion: a caption
//! next to a picture would make IMG-005 report that the alt text repeats it.

use std::path::Path;

use serde::Serialize;

use crate::model::Document;

use super::safety::original_stem;

/// Where a suggestion comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TitleSource {
    FileName,
    TitleParagraph,
    FirstHeading1,
}

impl TitleSource {
    /// The words of the button: "Use the file name".
    pub fn label(self) -> &'static str {
        match self {
            TitleSource::FileName => "Use the file name",
            TitleSource::TitleParagraph => "Use the Title paragraph",
            TitleSource::FirstHeading1 => "Use the first Heading 1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TitleSuggestion {
    pub source: TitleSource,
    /// The words of the button.
    pub label: String,
    /// The text that goes into the title field when the person chooses it.
    pub text: String,
}

/// The title that the file name gives: the name without `.docx` and without
/// ` (accessible)`, with each underscore and each run of spaces made into one
/// space. A date at the start (`2026.11.18 - Guide`) stays as it is: no word is
/// added, taken out or moved.
pub fn file_name_title(file_name: &str) -> String {
    let path = Path::new(file_name);
    let stem = original_stem(path).or_else(|| {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
    });
    stem.unwrap_or_default()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The suggestions for the title of `doc`, in this order: the file name, the
/// first Title paragraph, the first Heading 1. A text that is empty is left
/// out, and a text that repeats an earlier one is left out.
pub fn title_suggestions(doc: &Document, file_name: &str) -> Vec<TitleSuggestion> {
    let title_paragraph = doc
        .paragraphs
        .iter()
        .filter(|p| !p.is_blank())
        .find(|p| {
            p.style_name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case("title"))
        })
        .map(|p| one_line(&p.text));
    let first_heading = doc.headings().find(|p| p.heading_level == Some(1));
    let candidates = [
        (TitleSource::FileName, Some(file_name_title(file_name))),
        (TitleSource::TitleParagraph, title_paragraph),
        (
            TitleSource::FirstHeading1,
            first_heading.map(|p| one_line(&p.text)),
        ),
    ];
    let mut found: Vec<TitleSuggestion> = Vec::new();
    for (source, text) in candidates {
        let Some(text) = text.filter(|t| !t.is_empty()) else {
            continue;
        };
        if found.iter().any(|f| f.text == text) {
            continue;
        }
        found.push(TitleSuggestion {
            source,
            label: source.label().to_string(),
            text,
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{DocxBuilder, Para};

    fn suggestions(builder: DocxBuilder, file_name: &str) -> Vec<(TitleSource, String)> {
        let doc = Document::from_bytes(&builder.build()).unwrap();
        title_suggestions(&doc, file_name)
            .into_iter()
            .map(|s| (s.source, s.text))
            .collect()
    }

    #[test]
    fn the_file_name_is_cleaned_and_nothing_else_changes() {
        for (name, title) in [
            ("Guide.docx", "Guide"),
            ("Guide (accessible).docx", "Guide"),
            ("Parks_Training_Guide.docx", "Parks Training Guide"),
            (
                "Parks   Training__Guide  v2.docx",
                "Parks Training Guide v2",
            ),
            // A date at the start stays as it is.
            (
                "2026.11.18 - Parks_Training_Guide.docx",
                "2026.11.18 - Parks Training Guide",
            ),
            (
                "2026.11.18 - Parks_Training_Guide (accessible).docx",
                "2026.11.18 - Parks Training Guide",
            ),
            // The case of the letters and the order of the words are not touched.
            ("zoning_BASICS part 2.docx", "zoning BASICS part 2"),
            ("Guía_de_uso.docx", "Guía de uso"),
            ("_.docx", ""),
        ] {
            assert_eq!(file_name_title(name), title, "{name}");
        }
    }

    #[test]
    fn the_three_sources_come_in_order() {
        let builder = DocxBuilder::new()
            .paragraph(Para::new("Welcome page").style("Title"))
            .heading(2, "Not this one")
            .heading(1, "First steps")
            .heading(1, "Second");
        assert_eq!(
            suggestions(builder, "Parks_Guide.docx"),
            [
                (TitleSource::FileName, "Parks Guide".to_string()),
                (TitleSource::TitleParagraph, "Welcome page".to_string()),
                (TitleSource::FirstHeading1, "First steps".to_string()),
            ]
        );
    }

    #[test]
    fn a_source_that_is_not_there_gives_no_button() {
        // No Title paragraph and no Heading 1: only the file name.
        let builder = DocxBuilder::new()
            .heading(2, "Only a Heading 2")
            .para("Text");
        assert_eq!(
            suggestions(builder, "Guide.docx"),
            [(TitleSource::FileName, "Guide".to_string())]
        );
        // A Title paragraph with no text is not a source.
        let builder = DocxBuilder::new()
            .paragraph(Para::new("   ").style("Title"))
            .heading(1, "Setup");
        assert_eq!(
            suggestions(builder, "Guide.docx"),
            [
                (TitleSource::FileName, "Guide".to_string()),
                (TitleSource::FirstHeading1, "Setup".to_string()),
            ]
        );
        // A file name with nothing in it gives no button either.
        let builder = DocxBuilder::new().para("Text");
        assert!(suggestions(builder, "_.docx").is_empty());
    }

    #[test]
    fn identical_texts_are_offered_once() {
        // The heading repeats the file name, and the Title repeats the heading.
        let builder = DocxBuilder::new()
            .paragraph(Para::new("Parks Guide").style("Title"))
            .heading(1, "Parks   Guide");
        assert_eq!(
            suggestions(builder, "Parks_Guide.docx"),
            [(TitleSource::FileName, "Parks Guide".to_string())]
        );
        let builder = DocxBuilder::new()
            .paragraph(Para::new("A title").style("Title"))
            .heading(1, "A title");
        assert_eq!(
            suggestions(builder, "Guide.docx"),
            [
                (TitleSource::FileName, "Guide".to_string()),
                (TitleSource::TitleParagraph, "A title".to_string()),
            ]
        );
    }

    #[test]
    fn the_buttons_have_the_words_of_the_plan() {
        assert_eq!(TitleSource::FileName.label(), "Use the file name");
        assert_eq!(
            TitleSource::TitleParagraph.label(),
            "Use the Title paragraph"
        );
        assert_eq!(
            TitleSource::FirstHeading1.label(),
            "Use the first Heading 1"
        );
    }
}
