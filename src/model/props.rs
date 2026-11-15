//! Run and paragraph properties.
//!
//! The XML gives *partial* properties: [`RunFmt`] and [`ParFmt`] hold only
//! what one `w:rPr` or `w:pPr` sets. They are layered with `overlay` (later
//! layers win) and then turned into complete [`RunProps`] and
//! [`ParagraphProps`] with `resolve`.
//!
//! Layer order for a run: document defaults, paragraph style chain (base style
//! first), character style chain, direct formatting. Two simplifications:
//! table styles are not applied, and toggle properties such as bold follow
//! "last layer wins" instead of Word's XOR rule for styles.

use roxmltree::Node;

use super::theme::{Rgb, Theme};
use crate::docx::WordNode;

/// Word's default text size when nothing sets one: 10 pt.
pub const DEFAULT_SIZE_HALF_POINTS: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// "Automatic": black on a light background, white on a dark one.
    Auto,
    Rgb(Rgb),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    /// `both` or `distribute` in the XML.
    Justify,
}

/// Language tags from `w:lang`, for example `en-US`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lang {
    /// Latin and most other scripts.
    pub val: Option<String>,
    /// East Asian text.
    pub east_asia: Option<String>,
    /// Right-to-left text.
    pub bidi: Option<String>,
}

impl Lang {
    fn overlay(&mut self, over: &Lang) {
        if over.val.is_some() {
            self.val.clone_from(&over.val);
        }
        if over.east_asia.is_some() {
            self.east_asia.clone_from(&over.east_asia);
        }
        if over.bidi.is_some() {
            self.bidi.clone_from(&over.bidi);
        }
    }
}

/// Run properties set by one layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunFmt {
    pub bold: Option<bool>,
    pub size_half_points: Option<u32>,
    pub color: Option<Color>,
    pub font_ascii: Option<String>,
    pub font_h_ansi: Option<String>,
    pub lang: Lang,
}

/// Complete run properties after all layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunProps {
    pub bold: bool,
    /// Text size in half points (24 = 12 pt).
    pub size_half_points: u32,
    pub color: Color,
    pub font_ascii: Option<String>,
    pub font_h_ansi: Option<String>,
    pub lang: Lang,
}

impl RunFmt {
    /// Reads a `w:rPr` node. `None` gives an empty layer.
    pub fn parse(rpr: Option<Node>, theme: &Theme) -> RunFmt {
        let Some(rpr) = rpr else {
            return RunFmt::default();
        };
        let mut fmt = RunFmt {
            bold: rpr.w_child("b").map(|n| toggle(&n)),
            size_half_points: rpr
                .w_child("sz")
                .and_then(|n| n.w_attr("val"))
                .and_then(|v| v.parse().ok()),
            ..RunFmt::default()
        };
        if let Some(color) = rpr.w_child("color") {
            fmt.color = parse_color(&color, theme);
        }
        if let Some(fonts) = rpr.w_child("rFonts") {
            let pick = |theme_attr: &str, plain_attr: &str| {
                fonts
                    .w_attr(theme_attr)
                    .and_then(|t| theme.font(t))
                    .or_else(|| fonts.w_attr(plain_attr))
                    .map(str::to_string)
            };
            fmt.font_ascii = pick("asciiTheme", "ascii");
            fmt.font_h_ansi = pick("hAnsiTheme", "hAnsi");
        }
        if let Some(lang) = rpr.w_child("lang") {
            let get = |a: &str| lang.w_attr(a).filter(|v| !v.is_empty()).map(str::to_string);
            fmt.lang = Lang {
                val: get("val"),
                east_asia: get("eastAsia"),
                bidi: get("bidi"),
            };
        }
        fmt
    }

    /// Sets every property that `over` sets.
    pub fn overlay(&mut self, over: &RunFmt) {
        if over.bold.is_some() {
            self.bold = over.bold;
        }
        if over.size_half_points.is_some() {
            self.size_half_points = over.size_half_points;
        }
        if over.color.is_some() {
            self.color = over.color;
        }
        if over.font_ascii.is_some() {
            self.font_ascii.clone_from(&over.font_ascii);
        }
        if over.font_h_ansi.is_some() {
            self.font_h_ansi.clone_from(&over.font_h_ansi);
        }
        self.lang.overlay(&over.lang);
    }

    pub fn resolve(&self) -> RunProps {
        RunProps {
            bold: self.bold.unwrap_or(false),
            size_half_points: self.size_half_points.unwrap_or(DEFAULT_SIZE_HALF_POINTS),
            color: self.color.unwrap_or(Color::Auto),
            font_ascii: self.font_ascii.clone(),
            font_h_ansi: self.font_h_ansi.clone(),
            lang: self.lang.clone(),
        }
    }
}

/// A toggle element such as `w:b`: on unless `w:val` says off.
pub(super) fn toggle(node: &Node) -> bool {
    !matches!(node.w_attr("val"), Some("0" | "false" | "off" | "none"))
}

fn parse_color(node: &Node, theme: &Theme) -> Option<Color> {
    let hex_attr = |name: &str| {
        node.w_attr(name)
            .and_then(|v| u8::from_str_radix(v, 16).ok())
    };
    if let Some(base) = node.w_attr("themeColor").and_then(|t| theme.color(t)) {
        let mut rgb = base;
        if let Some(shade) = hex_attr("themeShade") {
            rgb = rgb.shade(shade);
        }
        if let Some(tint) = hex_attr("themeTint") {
            rgb = rgb.tint(tint);
        }
        return Some(Color::Rgb(rgb));
    }
    match node.w_attr("val")? {
        "auto" => Some(Color::Auto),
        hex => Rgb::from_hex(hex).map(Color::Rgb),
    }
}

/// Paragraph properties set by one layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParFmt {
    pub alignment: Option<Alignment>,
    /// Raw `w:outlineLvl`: 0 to 8 are levels 1 to 9, and 9 is "body text".
    pub outline_level: Option<u8>,
    pub num_id: Option<i64>,
    pub num_level: Option<u8>,
}

/// A paragraph's place in an automatic list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumPr {
    pub num_id: i64,
    /// 0-based list level (`w:ilvl`).
    pub level: u8,
}

/// Complete paragraph properties after all layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphProps {
    pub alignment: Alignment,
    /// Outline level 1 to 9 as Word shows it. `None` means body text.
    pub outline_level: Option<u8>,
    /// Set when the paragraph is in an automatic list.
    pub num: Option<NumPr>,
}

impl ParFmt {
    /// Reads a `w:pPr` node. `None` gives an empty layer.
    pub fn parse(ppr: Option<Node>) -> ParFmt {
        let Some(ppr) = ppr else {
            return ParFmt::default();
        };
        let mut fmt = ParFmt {
            alignment: ppr.w_child("jc").and_then(|n| match n.w_attr("val")? {
                "left" | "start" => Some(Alignment::Left),
                "center" => Some(Alignment::Center),
                "right" | "end" => Some(Alignment::Right),
                "both" | "distribute" => Some(Alignment::Justify),
                _ => None,
            }),
            outline_level: ppr
                .w_child("outlineLvl")
                .and_then(|n| n.w_attr("val"))
                .and_then(|v| v.parse().ok()),
            ..ParFmt::default()
        };
        if let Some(num) = ppr.w_child("numPr") {
            fmt.num_id = num
                .w_child("numId")
                .and_then(|n| n.w_attr("val"))
                .and_then(|v| v.parse().ok());
            fmt.num_level = num
                .w_child("ilvl")
                .and_then(|n| n.w_attr("val"))
                .and_then(|v| v.parse().ok());
        }
        fmt
    }

    pub fn overlay(&mut self, over: &ParFmt) {
        if over.alignment.is_some() {
            self.alignment = over.alignment;
        }
        if over.outline_level.is_some() {
            self.outline_level = over.outline_level;
        }
        if over.num_id.is_some() {
            self.num_id = over.num_id;
        }
        if over.num_level.is_some() {
            self.num_level = over.num_level;
        }
    }

    pub fn resolve(&self) -> ParagraphProps {
        ParagraphProps {
            alignment: self.alignment.unwrap_or_default(),
            outline_level: self.outline_level.filter(|raw| *raw < 9).map(|raw| raw + 1),
            // numId 0 means "no list".
            num: self.num_id.filter(|id| *id != 0).map(|num_id| NumPr {
                num_id,
                level: self.num_level.unwrap_or(0),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docx::parse_xml;

    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

    fn run_fmt(inner: &str, theme: &Theme) -> RunFmt {
        let xml = format!("<w:rPr {NS}>{inner}</w:rPr>");
        let doc = parse_xml("t", &xml).unwrap();
        RunFmt::parse(Some(doc.root_element()), theme)
    }

    fn par_fmt(inner: &str) -> ParFmt {
        let xml = format!("<w:pPr {NS}>{inner}</w:pPr>");
        let doc = parse_xml("t", &xml).unwrap();
        ParFmt::parse(Some(doc.root_element()))
    }

    fn theme() -> Theme {
        Theme::parse(
            "theme1.xml",
            r#"<a:theme xmlns:a="x"><a:clrScheme><a:accent1><a:srgbClr val="156082"/></a:accent1></a:clrScheme>
            <a:fontScheme><a:minorFont><a:latin typeface="Aptos"/></a:minorFont></a:fontScheme></a:theme>"#,
        )
        .unwrap()
    }

    #[test]
    fn bold_toggle_values() {
        let t = Theme::default();
        assert_eq!(run_fmt("<w:b/>", &t).bold, Some(true));
        assert_eq!(run_fmt(r#"<w:b w:val="0"/>"#, &t).bold, Some(false));
        assert_eq!(run_fmt(r#"<w:b w:val="false"/>"#, &t).bold, Some(false));
        assert_eq!(run_fmt(r#"<w:b w:val="true"/>"#, &t).bold, Some(true));
        assert_eq!(run_fmt("", &t).bold, None);
    }

    #[test]
    fn size_colour_and_language() {
        let t = Theme::default();
        let f = run_fmt(
            r#"<w:sz w:val="28"/><w:color w:val="FF0000"/><w:lang w:val="es-ES" w:eastAsia="ja-JP"/>"#,
            &t,
        );
        assert_eq!(f.size_half_points, Some(28));
        assert_eq!(f.color, Some(Color::Rgb(Rgb::from_hex("FF0000").unwrap())));
        assert_eq!(f.lang.val.as_deref(), Some("es-ES"));
        assert_eq!(f.lang.east_asia.as_deref(), Some("ja-JP"));
        assert_eq!(f.lang.bidi, None);
    }

    #[test]
    fn automatic_colour() {
        let f = run_fmt(r#"<w:color w:val="auto"/>"#, &Theme::default());
        assert_eq!(f.color, Some(Color::Auto));
    }

    #[test]
    fn theme_colour_wins_over_value_and_applies_shade() {
        let t = theme();
        let f = run_fmt(r#"<w:color w:val="FF0000" w:themeColor="accent1"/>"#, &t);
        assert_eq!(f.color, Some(Color::Rgb(Rgb::from_hex("156082").unwrap())));
        let f = run_fmt(
            r#"<w:color w:val="FF0000" w:themeColor="accent1" w:themeShade="80"/>"#,
            &t,
        );
        let Some(Color::Rgb(rgb)) = f.color else {
            panic!("expected a colour")
        };
        assert!(rgb.b < 0x82, "shade darkens the colour: {rgb:?}");
    }

    #[test]
    fn unknown_theme_colour_falls_back_to_the_value() {
        let f = run_fmt(
            r#"<w:color w:val="00FF00" w:themeColor="accent5"/>"#,
            &theme(),
        );
        assert_eq!(f.color, Some(Color::Rgb(Rgb::from_hex("00FF00").unwrap())));
    }

    #[test]
    fn theme_fonts_are_resolved() {
        let f = run_fmt(
            r#"<w:rFonts w:asciiTheme="minorHAnsi" w:ascii="Calibri" w:hAnsi="Arial"/>"#,
            &theme(),
        );
        assert_eq!(f.font_ascii.as_deref(), Some("Aptos"));
        assert_eq!(f.font_h_ansi.as_deref(), Some("Arial"));
    }

    #[test]
    fn later_layers_win_and_missing_values_fall_through() {
        let t = Theme::default();
        let mut base = run_fmt(r#"<w:b/><w:sz w:val="24"/><w:lang w:val="en-US"/>"#, &t);
        base.overlay(&run_fmt(
            r#"<w:b w:val="0"/><w:lang w:eastAsia="ja-JP"/>"#,
            &t,
        ));
        let p = base.resolve();
        assert!(!p.bold);
        assert_eq!(p.size_half_points, 24);
        assert_eq!(p.lang.val.as_deref(), Some("en-US"));
        assert_eq!(p.lang.east_asia.as_deref(), Some("ja-JP"));
    }

    #[test]
    fn defaults_when_nothing_is_set() {
        let p = RunFmt::default().resolve();
        assert!(!p.bold);
        assert_eq!(p.size_half_points, DEFAULT_SIZE_HALF_POINTS);
        assert_eq!(p.color, Color::Auto);
    }

    #[test]
    fn paragraph_alignment_values() {
        assert_eq!(
            par_fmt(r#"<w:jc w:val="both"/>"#).alignment,
            Some(Alignment::Justify)
        );
        assert_eq!(
            par_fmt(r#"<w:jc w:val="center"/>"#).alignment,
            Some(Alignment::Center)
        );
        assert_eq!(
            par_fmt(r#"<w:jc w:val="start"/>"#).alignment,
            Some(Alignment::Left)
        );
        assert_eq!(par_fmt("").resolve().alignment, Alignment::Left);
    }

    #[test]
    fn outline_level_is_one_based_and_nine_means_body_text() {
        assert_eq!(
            par_fmt(r#"<w:outlineLvl w:val="0"/>"#)
                .resolve()
                .outline_level,
            Some(1)
        );
        assert_eq!(
            par_fmt(r#"<w:outlineLvl w:val="2"/>"#)
                .resolve()
                .outline_level,
            Some(3)
        );
        assert_eq!(
            par_fmt(r#"<w:outlineLvl w:val="9"/>"#)
                .resolve()
                .outline_level,
            None
        );
    }

    #[test]
    fn numbering_reference_and_numid_zero() {
        let f = par_fmt(r#"<w:numPr><w:ilvl w:val="1"/><w:numId w:val="3"/></w:numPr>"#);
        assert_eq!(
            f.resolve().num,
            Some(NumPr {
                num_id: 3,
                level: 1
            })
        );
        let off = par_fmt(r#"<w:numPr><w:numId w:val="0"/></w:numPr>"#);
        let mut layered = f.clone();
        layered.overlay(&off);
        assert_eq!(layered.resolve().num, None);
    }
}
