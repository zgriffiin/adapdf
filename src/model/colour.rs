//! Colour maths for the contrast rule (CLR-001): WCAG relative luminance and
//! contrast ratio, and Word's choice of text colour for "automatic".

use std::fmt;

use super::theme::Rgb;

/// Word shows automatic text colour as white when the background is darker
/// than this (see [`auto_text_colour`]).
pub const AUTO_DARK_LUMA: f64 = 0.5;

impl fmt::Display for Rgb {
    /// `#RRGGBB`, upper case.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Rgb {
    pub const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };
}

/// WCAG 2.1 relative luminance, 0 (black) to 1 (white).
pub fn relative_luminance(c: Rgb) -> f64 {
    let linear = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

/// WCAG 2.1 contrast ratio, 1 to 21. The order of the colours does not matter.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (light, dark) = if la >= lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

/// Brightness of the colour as people see it, 0 to 1: the Rec. 601 weights
/// on the gamma-encoded channels, not on linear light.
pub fn luma(c: Rgb) -> f64 {
    (0.299 * f64::from(c.r) + 0.587 * f64::from(c.g) + 0.114 * f64::from(c.b)) / 255.0
}

/// What colour "automatic" text has on this background: white when the
/// background [`luma`] is below [`AUTO_DARK_LUMA`], black otherwise. Word
/// makes automatic text white on a dark background. The exact rule inside Word
/// is not documented; this is an approximation. It uses `luma`, not WCAG
/// luminance, because WCAG luminance of mid grey (#808080 is 0.22) would give
/// white text where Word still draws black.
pub fn auto_text_colour(background: Rgb) -> Rgb {
    if luma(background) < AUTO_DARK_LUMA {
        Rgb::WHITE
    } else {
        Rgb::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(h: &str) -> Rgb {
        Rgb::from_hex(h).unwrap()
    }

    #[test]
    fn black_on_white_is_21_to_1() {
        let ratio = contrast_ratio(Rgb::BLACK, Rgb::WHITE);
        assert!((ratio - 21.0).abs() < 1e-9, "{ratio}");
        assert_eq!(contrast_ratio(Rgb::WHITE, Rgb::BLACK), ratio);
    }

    #[test]
    fn same_colour_is_1_to_1() {
        assert!((contrast_ratio(hex("336699"), hex("336699")) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn known_grey_on_white_values() {
        let r777 = contrast_ratio(hex("777777"), Rgb::WHITE);
        assert!((r777 - 4.48).abs() < 0.005, "{r777}");
        let r767 = contrast_ratio(hex("767676"), Rgb::WHITE);
        assert!((r767 - 4.54).abs() < 0.005, "{r767}");
    }

    #[test]
    fn luminance_of_the_primaries() {
        assert!((relative_luminance(hex("FF0000")) - 0.2126).abs() < 1e-9);
        assert!((relative_luminance(hex("00FF00")) - 0.7152).abs() < 1e-9);
        assert!((relative_luminance(hex("0000FF")) - 0.0722).abs() < 1e-9);
    }

    #[test]
    fn automatic_text_is_white_on_dark_and_black_on_light() {
        assert_eq!(auto_text_colour(Rgb::WHITE), Rgb::BLACK);
        assert_eq!(auto_text_colour(Rgb::BLACK), Rgb::WHITE);
        assert_eq!(auto_text_colour(hex("1F3864")), Rgb::WHITE);
        assert_eq!(auto_text_colour(hex("FFFF00")), Rgb::BLACK);
        assert_eq!(auto_text_colour(hex("808080")), Rgb::BLACK, "luma 0.502");
        assert_eq!(auto_text_colour(hex("7F7F7F")), Rgb::WHITE, "luma 0.498");
    }

    #[test]
    fn colours_print_as_hex() {
        assert_eq!(hex("0a1b2c").to_string(), "#0A1B2C");
    }
}
