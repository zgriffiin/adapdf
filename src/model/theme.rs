//! Theme colours and fonts (`word/theme/theme1.xml`).

use std::collections::HashMap;

use crate::docx::parse_xml;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

    /// Reads `RRGGBB`.
    pub fn from_hex(hex: &str) -> Option<Rgb> {
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(Rgb {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
        })
    }

    /// Word's `themeShade`: darkens the colour. 0xFF keeps it as is.
    /// This is a per-channel approximation of what Word does.
    pub fn shade(self, shade: u8) -> Rgb {
        let f = |c: u8| (u16::from(c) * u16::from(shade) / 255) as u8;
        Rgb {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }

    /// Word's `themeTint`: mixes the colour with white. 0xFF keeps it as is.
    /// This is a per-channel approximation of what Word does.
    pub fn tint(self, tint: u8) -> Rgb {
        let f = |c: u8| (255 - (255 - u16::from(c)) * u16::from(tint) / 255) as u8;
        Rgb {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    colors: HashMap<&'static str, Rgb>,
    major_font: Option<String>,
    minor_font: Option<String>,
}

/// The colour slots of a theme, and the `w:themeColor` names that select them.
const COLOR_SLOTS: &[(&str, &[&str])] = &[
    ("dk1", &["dark1", "text1"]),
    ("lt1", &["light1", "background1"]),
    ("dk2", &["dark2", "text2"]),
    ("lt2", &["light2", "background2"]),
    ("accent1", &["accent1"]),
    ("accent2", &["accent2"]),
    ("accent3", &["accent3"]),
    ("accent4", &["accent4"]),
    ("accent5", &["accent5"]),
    ("accent6", &["accent6"]),
    ("hlink", &["hyperlink"]),
    ("folHlink", &["followedHyperlink"]),
];

impl Theme {
    pub fn parse(part: &str, text: &str) -> Result<Theme> {
        let doc = parse_xml(part, text)?;
        let mut theme = Theme::default();
        let first = |name: &str| {
            doc.descendants()
                .find(|n| n.is_element() && n.tag_name().name() == name)
        };
        // Only the first colour scheme counts; later ones are extra schemes.
        for node in first("clrScheme").iter().flat_map(|s| s.children()) {
            let name = node.tag_name().name();
            let Some((slot, _)) = COLOR_SLOTS.iter().find(|(slot, _)| *slot == name) else {
                continue;
            };
            let hex = node.children().find_map(|c| match c.tag_name().name() {
                "srgbClr" => c.attribute("val"),
                "sysClr" => c.attribute("lastClr"),
                _ => None,
            });
            if let Some(rgb) = hex.and_then(Rgb::from_hex) {
                theme.colors.insert(slot, rgb);
            }
        }
        for node in first("fontScheme").iter().flat_map(|s| s.children()) {
            let latin = node
                .children()
                .find(|c| c.tag_name().name() == "latin")
                .and_then(|c| c.attribute("typeface"))
                .filter(|t| !t.is_empty())
                .map(str::to_string);
            match node.tag_name().name() {
                "majorFont" => theme.major_font = latin,
                "minorFont" => theme.minor_font = latin,
                _ => {}
            }
        }
        Ok(theme)
    }

    /// The colour for a `w:themeColor` value such as `accent1` or `text1`.
    pub fn color(&self, theme_color: &str) -> Option<Rgb> {
        let (slot, _) = COLOR_SLOTS
            .iter()
            .find(|(slot, names)| *slot == theme_color || names.contains(&theme_color))?;
        self.colors.get(slot).copied()
    }

    /// The Latin font for a `w:asciiTheme` or `w:hAnsiTheme` value such as
    /// `minorHAnsi` or `majorAscii`.
    pub fn font(&self, theme_font: &str) -> Option<&str> {
        if theme_font.starts_with("major") {
            self.major_font.as_deref()
        } else if theme_font.starts_with("minor") {
            self.minor_font.as_deref()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements>
        <a:clrScheme name="t">
          <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
          <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
          <a:dk2><a:srgbClr val="0E2841"/></a:dk2>
          <a:accent1><a:srgbClr val="156082"/></a:accent1>
        </a:clrScheme>
        <a:fontScheme name="f">
          <a:majorFont><a:latin typeface="Aptos Display"/></a:majorFont>
          <a:minorFont><a:latin typeface="Aptos"/></a:minorFont>
        </a:fontScheme></a:themeElements></a:theme>"#;

    #[test]
    fn reads_colours_by_slot_and_word_name() {
        let theme = Theme::parse("theme1.xml", THEME).unwrap();
        assert_eq!(theme.color("accent1"), Rgb::from_hex("156082"));
        assert_eq!(theme.color("text2"), Rgb::from_hex("0E2841"));
        assert_eq!(theme.color("background1"), Rgb::from_hex("FFFFFF"));
        assert_eq!(theme.color("text1"), Some(Rgb::BLACK));
        assert_eq!(theme.color("accent6"), None);
        assert_eq!(theme.color("nonsense"), None);
    }

    #[test]
    fn reads_theme_fonts() {
        let theme = Theme::parse("theme1.xml", THEME).unwrap();
        assert_eq!(theme.font("minorHAnsi"), Some("Aptos"));
        assert_eq!(theme.font("majorAscii"), Some("Aptos Display"));
        assert_eq!(theme.font("other"), None);
    }

    #[test]
    fn shade_and_tint() {
        let c = Rgb::from_hex("808080").unwrap();
        assert_eq!(c.shade(0xFF), c);
        assert_eq!(c.shade(0).r, 0);
        assert_eq!(c.tint(0xFF), c);
        assert_eq!(c.tint(0), Rgb::from_hex("FFFFFF").unwrap());
    }

    #[test]
    fn hex_must_have_six_digits() {
        assert!(Rgb::from_hex("12345").is_none());
        assert!(Rgb::from_hex("GGGGGG").is_none());
        assert!(Rgb::from_hex("é1234").is_none());
    }
}
