pub mod theme;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::theme::Theme;
use crate::domain::model::{CaretStyle, Model};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml deserialize: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("toml serialize: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CaretConfig {
    #[serde(default)]
    pub style: CaretStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub caret: CaretConfig,
    pub theme: Theme,
}

pub fn load_or_default() -> AppConfig {
    load_or_default_from(&config_path())
}

pub fn apply_to_model(model: &mut Model, cfg: AppConfig) {
    model.theme = cfg.theme;
    model.config.caret_style = cfg.caret.style;
}

pub fn write_if_missing() -> Result<(), ConfigError> {
    write_if_missing_to(&config_path())
}

pub(crate) fn load_or_default_from(path: &Path) -> AppConfig {
    if !path.exists() {
        return AppConfig::default();
    }
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ktype: config read error ({}), using default", e);
            return AppConfig::default();
        }
    };
    // Parse into a raw table first so each section can fail independently.
    let raw: toml::Table = match toml::from_str(&s) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ktype: config parse error ({}), using default", e);
            return AppConfig::default();
        }
    };
    AppConfig {
        theme: load_section(&raw, "theme"),
        caret: load_section(&raw, "caret"),
    }
}

/// Deserializes a named top-level section from a raw TOML table, falling back
/// to `T::default()` if the section is absent or fails to parse.
fn load_section<T: serde::de::DeserializeOwned + Default>(raw: &toml::Table, key: &str) -> T {
    let Some(value) = raw.get(key) else {
        return T::default();
    };
    if !value.is_table() {
        eprintln!(
            "ktype: [{}] config error: expected a table section, found {}, using default",
            key,
            value.type_str()
        );
        return T::default();
    }
    // Re-serialize the sub-table as a top-level document then parse as T.
    // This keeps each section's errors isolated from the rest of the config.
    let serialized = match toml::to_string(value) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ktype: [{}] config error ({}), using default", key, e);
            return T::default();
        }
    };
    match toml::from_str::<T>(&serialized) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ktype: [{}] config error ({}), using default", key, e);
            T::default()
        }
    }
}

pub(crate) fn write_if_missing_to(path: &Path) -> Result<(), ConfigError> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string_pretty(&AppConfig::default())?)?;
    Ok(())
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("ktype")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_or_default_returns_default_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn load_or_default_returns_default_on_invalid_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, b"not valid toml = [[[").unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn load_or_default_returns_default_on_invalid_hex() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let bad = r##"
[theme]
bg = "INVALID"
main = "#e2b714"
caret = "#e2b714"
sub = "#646669"
sub_alt = "#2c2e31"
text = "#d1d0c5"
error = "#ca4754"
error_extra = "#7e2a33"
colorful_error = "#ca4754"
colorful_error_extra = "#7e2a33"
"##;
        std::fs::write(&path, bad).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn load_or_default_parses_valid_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#7aa2f7");
    }

    #[test]
    fn write_if_missing_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        assert!(!path.exists());
        write_if_missing_to(&path).unwrap();
        assert!(path.exists());
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.bg.0, "#323437");
    }

    #[test]
    fn write_if_missing_does_not_overwrite_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
"##;
        std::fs::write(&path, custom).unwrap();
        write_if_missing_to(&path).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#7aa2f7");
    }

    #[test]
    fn written_file_is_valid_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        write_if_missing_to(&path).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("[theme]"));
        assert!(contents.contains("bg ="));
    }

    #[test]
    fn empty_file_falls_back_to_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, b"").unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn theme_section_with_no_fields_falls_back_to_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, b"[theme]\n").unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn partial_theme_falls_back_to_default() {
        // Only bg specified; remaining required fields are missing → default
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, b"[theme]\nbg = \"#1a1b1e\"\n").unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn unknown_theme_keys_are_ignored() {
        // Extra keys under [theme] not in Theme struct are silently ignored
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
future_unknown_field = "#ffffff"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#7aa2f7");
    }

    #[test]
    fn unknown_top_level_section_is_ignored() {
        // Extra top-level sections not in AppConfig are silently ignored
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"

[future_section]
some_setting = true
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#7aa2f7");
    }

    #[test]
    fn wrong_type_for_color_field_falls_back_to_default() {
        // bg = 42 (integer instead of string) → serde type mismatch → default
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let bad = r##"
[theme]
bg = 42
main = "#e2b714"
caret = "#e2b714"
sub = "#646669"
sub_alt = "#2c2e31"
text = "#d1d0c5"
error = "#ca4754"
error_extra = "#7e2a33"
colorful_error = "#ca4754"
colorful_error_extra = "#7e2a33"
"##;
        std::fs::write(&path, bad).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.theme.main.0, "#e2b714");
    }

    #[test]
    fn caret_style_defaults_to_block_when_section_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.caret.style, CaretStyle::Block);
    }

    #[test]
    fn caret_style_parses_underline_from_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[caret]
style = "underline"

[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.caret.style, CaretStyle::Underline);
    }

    #[test]
    fn invalid_caret_style_leaves_theme_intact() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[caret]
style = "bogus"

[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        // theme must still be the custom values
        assert_eq!(config.theme.main.0, "#7aa2f7");
        // caret falls back to default
        assert_eq!(config.caret.style, CaretStyle::Block);
    }

    #[test]
    fn invalid_theme_leaves_caret_intact() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
[caret]
style = "underline"

[theme]
bg = "NOT_A_COLOR"
main = "#e2b714"
caret = "#e2b714"
sub = "#646669"
sub_alt = "#2c2e31"
text = "#d1d0c5"
error = "#ca4754"
error_extra = "#7e2a33"
colorful_error = "#ca4754"
colorful_error_extra = "#7e2a33"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        // theme falls back to default
        assert_eq!(config.theme.main.0, "#e2b714");
        // caret must still be the custom value
        assert_eq!(config.caret.style, CaretStyle::Underline);
    }

    #[test]
    fn bare_string_caret_value_falls_back_to_default() {
        // User writes `caret = "underline"` (bare string) instead of a [caret] table.
        // The section must be skipped gracefully and return the default style.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let custom = r##"
caret = "underline"

[theme]
bg = "#1a1b1e"
main = "#7aa2f7"
caret = "#7aa2f7"
sub = "#565f89"
sub_alt = "#1f2335"
text = "#c0caf5"
error = "#f7768e"
error_extra = "#8c4351"
colorful_error = "#f7768e"
colorful_error_extra = "#8c4351"
"##;
        std::fs::write(&path, custom).unwrap();
        let config = load_or_default_from(&path);
        assert_eq!(config.caret.style, CaretStyle::Block);
        assert_eq!(config.theme.main.0, "#7aa2f7");
    }

    #[test]
    fn written_default_config_contains_caret_section() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        write_if_missing_to(&path).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("[caret]"));
        assert!(contents.contains("style ="));
    }
}
