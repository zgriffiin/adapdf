//! Colour rules CLR-001 and CLR-002.
//!
//! Both rules read the runs of the body paragraphs, including paragraphs in
//! table cells. Runs with no visible text are skipped. Header, footer and
//! footnote text is not checked. Table styles are applied (see
//! [`crate::model::table_styles`]), so a fill or a text colour that comes only
//! from a table style is seen.

use crate::findings::{Finding, Location, RuleId, snippet};
use crate::model::colour::{auto_text_colour, contrast_ratio};
use crate::model::props::Color;
use crate::model::theme::Rgb;
use crate::model::{Document, Paragraph, Run};

/// CLR-001: the smallest contrast ratio for normal text (WCAG 1.4.3).
pub const MIN_CONTRAST_NORMAL: f64 = 4.5;

/// CLR-001: the smallest contrast ratio for large text.
pub const MIN_CONTRAST_LARGE: f64 = 3.0;

/// CLR-001: text of this size or more is large, in half points (18 pt).
pub const LARGE_TEXT_HALF_POINTS: u32 = 36;

/// CLR-001: bold text of this size or more is large, in half points (14 pt).
pub const LARGE_BOLD_TEXT_HALF_POINTS: u32 = 28;

/// Large text: 18 pt or more, or 14 pt or more when bold.
pub fn is_large_text(size_half_points: u32, bold: bool) -> bool {
    size_half_points >= LARGE_TEXT_HALF_POINTS
        || (bold && size_half_points >= LARGE_BOLD_TEXT_HALF_POINTS)
}

/// The colour behind a run, from the top layer down: highlight, run shading,
/// paragraph shading, table cell (or table) shading, the page colour, white.
/// The page colour counts only when the document shows it
/// (`displayBackgroundShape` in the settings).
pub fn background(doc: &Document, p: &Paragraph, run: &Run) -> Rgb {
    let page = doc
        .page_background
        .filter(|_| doc.settings.display_background_shape);
    run.props
        .highlight
        .or(run.props.shading)
        .or(p.props.shading)
        .or(p.container_fill)
        .or(page)
        .unwrap_or(Rgb::WHITE)
}

/// The text colour of a run on this background. "Automatic" is white on a
/// dark background and black on a light one (see [`auto_text_colour`]).
pub fn foreground(run: &Run, background: Rgb) -> Rgb {
    match run.props.color {
        Color::Rgb(c) => c,
        Color::Auto => auto_text_colour(background),
    }
}

/// A ratio written with two decimals. A ratio below `needed` is never rounded
/// up to it: 4.4999 shows as 4.49, not 4.50.
fn show_ratio(ratio: f64, needed: f64) -> String {
    let rounded = (ratio * 100.0).round() / 100.0;
    let shown = if ratio < needed && rounded >= needed {
        (ratio * 100.0).floor() / 100.0
    } else {
        rounded
    };
    format!("{shown:.2}")
}

/// CLR-001: the contrast of text is below 4.5:1, or below 3:1 for large text.
/// One finding for each different pair of colours in a paragraph.
pub fn clr_001(doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    for p in &doc.paragraphs {
        let mut seen: Vec<(Rgb, Rgb, bool)> = Vec::new();
        for run in p.runs.iter().filter(|r| r.has_visible_text()) {
            let back = background(doc, p, run);
            let fore = foreground(run, back);
            let large = is_large_text(run.props.size_half_points, run.props.bold);
            let (needed, kind) = if large {
                (MIN_CONTRAST_LARGE, "Large text")
            } else {
                (MIN_CONTRAST_NORMAL, "Normal text")
            };
            let ratio = contrast_ratio(fore, back);
            if ratio >= needed || seen.contains(&(fore, back, large)) {
                continue;
            }
            seen.push((fore, back, large));
            let automatic = if run.props.color == Color::Auto {
                " (automatic)"
            } else {
                ""
            };
            findings.push(Finding::new(
                RuleId::Clr001,
                format!(
                    "The text \"{}\" has a contrast ratio of {}:1 (text colour {fore}{automatic} on {back}). {kind} needs at least {needed}:1.",
                    snippet(&run.text),
                    show_ratio(ratio, needed),
                ),
                format!("Use a darker text colour, or a lighter background, so that the contrast is at least {needed}:1. In Word, select the text and choose a darker colour under Home > Font Color."),
                Location::paragraph(doc, p),
            ));
        }
    }
    findings
}

/// The same colour, for CLR-002: automatic text and black are not told apart.
fn same_colour(a: Color, b: Color) -> bool {
    let black = |c: Color| c == Color::Auto || c == Color::Rgb(Rgb::BLACK);
    a == b || (black(a) && black(b))
}

/// CLR-002: a run has a text colour that is not the colour of its paragraph
/// style, and nothing else marks it: it is not bold, italic or underlined
/// (compared with its paragraph style), has no highlight or shading, names no
/// character style and is not in a hyperlink. One finding for each colour in
/// a paragraph.
pub fn clr_002(doc: &Document) -> Vec<Finding> {
    let mut findings = Vec::new();
    for p in &doc.paragraphs {
        let base = &p.style_run;
        let mut seen: Vec<Color> = Vec::new();
        for run in p.runs.iter().filter(|r| r.has_visible_text()) {
            let r = &run.props;
            let only_colour = !same_colour(r.color, base.color)
                && r.bold == base.bold
                && r.italic == base.italic
                && r.underline == base.underline
                && r.highlight.is_none()
                && r.shading.is_none()
                && run.char_style.is_none()
                && !run.in_hyperlink;
            if !only_colour || seen.contains(&r.color) {
                continue;
            }
            seen.push(r.color);
            let colour = match r.color {
                Color::Rgb(c) => c.to_string(),
                Color::Auto => "automatic".to_string(),
            };
            findings.push(Finding::new(
                RuleId::Clr002,
                format!("The text \"{}\" is in a different colour ({colour}) from the text around it, and nothing else marks it. People who cannot see the colour miss the difference.", snippet(&run.text)),
                "Mark the text in another way as well: make it bold, or use a style such as Strong, or say in words what the colour means.",
                Location::paragraph(doc, p),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_text_boundaries() {
        assert!(is_large_text(36, false), "18 pt");
        assert!(!is_large_text(35, false), "17.5 pt");
        assert!(is_large_text(28, true), "14 pt bold");
        assert!(!is_large_text(28, false), "14 pt regular");
        assert!(!is_large_text(27, true), "13.5 pt bold");
    }

    #[test]
    fn ratios_are_rounded_but_never_up_to_the_limit() {
        assert_eq!(show_ratio(4.4999, 4.5), "4.49");
        assert_eq!(show_ratio(4.4776, 4.5), "4.48");
        assert_eq!(show_ratio(4.5, 4.5), "4.50");
        assert_eq!(show_ratio(21.0, 4.5), "21.00");
    }

    #[test]
    fn automatic_and_black_are_the_same_colour() {
        let black = Color::Rgb(Rgb::BLACK);
        assert!(same_colour(Color::Auto, black));
        assert!(same_colour(black, Color::Auto));
        assert!(!same_colour(Color::Auto, Color::Rgb(Rgb::WHITE)));
    }
}
