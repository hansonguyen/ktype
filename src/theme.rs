use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("invalid hex color '{0}': must be #rrggbb (7 chars, valid hex digits)")]
    InvalidHex(String),
}

/// A validated 7-character hex color string in `#rrggbb` form.
#[derive(Debug, Clone, Serialize)]
pub struct HexColor(pub String);

impl HexColor {
    pub fn parse(s: &str) -> Result<Self, ThemeError> {
        let s = s.to_string();
        if s.len() != 7 || !s.starts_with('#') {
            return Err(ThemeError::InvalidHex(s));
        }
        if !s[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ThemeError::InvalidHex(s));
        }
        Ok(HexColor(s))
    }

    pub fn to_ratatui_color(&self) -> Color {
        let hex = &self.0[1..];
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::Rgb(r, g, b)
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        HexColor::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// MonkeyType-compatible theme with 10 color slots.
///
/// Stored under the `[theme]` section of `~/.config/ktype/config.toml`.
/// Users customize this by editing that file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub bg: HexColor,
    pub main: HexColor,
    pub caret: HexColor,
    pub sub: HexColor,
    pub sub_alt: HexColor,
    pub text: HexColor,
    pub error: HexColor,
    pub error_extra: HexColor,
    pub colorful_error: HexColor,
    pub colorful_error_extra: HexColor,
}

impl Default for Theme {
    /// Built-in "serika dark" theme — matches MonkeyType's default palette.
    fn default() -> Self {
        Theme {
            bg: HexColor("#323437".into()),
            main: HexColor("#e2b714".into()),
            caret: HexColor("#e2b714".into()),
            sub: HexColor("#646669".into()),
            sub_alt: HexColor("#2c2e31".into()),
            text: HexColor("#d1d0c5".into()),
            error: HexColor("#ca4754".into()),
            error_extra: HexColor("#7e2a33".into()),
            colorful_error: HexColor("#ca4754".into()),
            colorful_error_extra: HexColor("#7e2a33".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hex_color_parses() {
        let c = HexColor::parse("#e2b714").unwrap();
        assert_eq!(c.0, "#e2b714");
    }

    #[test]
    fn hex_color_without_hash_is_rejected() {
        assert!(HexColor::parse("e2b714").is_err());
    }

    #[test]
    fn hex_color_wrong_length_is_rejected() {
        assert!(HexColor::parse("#fff").is_err());
        assert!(HexColor::parse("#ffffffff").is_err());
    }

    #[test]
    fn hex_color_invalid_chars_is_rejected() {
        assert!(HexColor::parse("#zzzzzz").is_err());
    }

    #[test]
    fn hex_color_uppercase_parses() {
        assert!(HexColor::parse("#E2B714").is_ok());
    }

    #[test]
    fn to_ratatui_color_converts_correctly() {
        let c = HexColor::parse("#e2b714").unwrap();
        assert_eq!(
            c.to_ratatui_color(),
            ratatui::style::Color::Rgb(0xe2, 0xb7, 0x14)
        );
    }

    #[test]
    fn theme_default_has_valid_hex_colors() {
        let t = Theme::default();
        let _ = t.bg.to_ratatui_color();
        let _ = t.main.to_ratatui_color();
        let _ = t.caret.to_ratatui_color();
        let _ = t.sub.to_ratatui_color();
        let _ = t.sub_alt.to_ratatui_color();
        let _ = t.text.to_ratatui_color();
        let _ = t.error.to_ratatui_color();
        let _ = t.error_extra.to_ratatui_color();
        let _ = t.colorful_error.to_ratatui_color();
        let _ = t.colorful_error_extra.to_ratatui_color();
    }

    #[test]
    fn theme_deserializes_via_serde() {
        let json = r##"{
            "bg": "#323437",
            "main": "#e2b714",
            "caret": "#e2b714",
            "sub": "#646669",
            "sub_alt": "#2c2e31",
            "text": "#d1d0c5",
            "error": "#ca4754",
            "error_extra": "#7e2a33",
            "colorful_error": "#ca4754",
            "colorful_error_extra": "#7e2a33"
        }"##;
        let t: Theme = serde_json::from_str(json).unwrap();
        assert_eq!(t.main.0, "#e2b714");
    }

    #[test]
    fn theme_rejects_invalid_hex_via_serde() {
        let json = r##"{
            "bg": "not-a-color",
            "main": "#e2b714",
            "caret": "#e2b714",
            "sub": "#646669",
            "sub_alt": "#2c2e31",
            "text": "#d1d0c5",
            "error": "#ca4754",
            "error_extra": "#7e2a33",
            "colorful_error": "#ca4754",
            "colorful_error_extra": "#7e2a33"
        }"##;
        assert!(serde_json::from_str::<Theme>(json).is_err());
    }

    #[test]
    fn theme_round_trips_via_serde() {
        let original = Theme::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: Theme = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.bg.0, original.bg.0);
        assert_eq!(restored.error.0, original.error.0);
    }
}
