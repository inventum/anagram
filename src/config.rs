//! User config: min/max word size, locale, theme placeholder.

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchSettings {
    pub enabled: bool,
    pub rows: u8,
    pub cols: u8,
}

fn default_workbench_enabled() -> bool {
    true
}

fn default_workbench_dim() -> u8 {
    5
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub min_size: u8,
    pub max_size: u8,
    pub locale: String,
    pub theme: String,
    /// Empty answer slot color name (see `ui::parse_color_name`).
    #[serde(default = "default_empty_slot_color")]
    pub empty_slot_color: String,
    #[serde(default = "default_filled_slot_color")]
    pub filled_slot_color: String,
    #[serde(default = "default_pool_color")]
    pub pool_color: String,
    #[serde(default = "default_correct_position_color")]
    pub correct_position_color: String,
    #[serde(default = "default_workbench_enabled")]
    pub workbench: bool,
    #[serde(default = "default_workbench_dim")]
    pub workbench_rows: u8,
    #[serde(default = "default_workbench_dim")]
    pub workbench_cols: u8,
}

fn default_empty_slot_color() -> String {
    "light_magenta".into()
}

fn default_filled_slot_color() -> String {
    "light_yellow".into()
}

fn default_pool_color() -> String {
    "light_cyan".into()
}

fn default_correct_position_color() -> String {
    "light_green".into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            min_size: 4,
            max_size: 10,
            locale: "en-US".into(),
            theme: "default".into(),
            empty_slot_color: default_empty_slot_color(),
            filled_slot_color: default_filled_slot_color(),
            pool_color: default_pool_color(),
            correct_position_color: default_correct_position_color(),
            workbench: default_workbench_enabled(),
            workbench_rows: default_workbench_dim(),
            workbench_cols: default_workbench_dim(),
        }
    }
}

impl AppConfig {
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(default_config_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: AppConfig = toml::from_str(&text)?;
        Ok(cfg)
    }

    pub fn clamp_size(&self, size: u8) -> u8 {
        size.clamp(self.min_size, self.max_size)
    }

    pub fn workbench_settings(&self) -> WorkbenchSettings {
        let mut rows = self.workbench_rows;
        let mut cols = self.workbench_cols;
        if rows != cols || !matches!(rows, 4 | 5) {
            rows = 5;
            cols = 5;
        }
        WorkbenchSettings {
            enabled: self.workbench,
            rows,
            cols,
        }
    }
}

pub fn default_config_path() -> PathBuf {
    #[allow(deprecated)]
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("anagram").join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_config_values() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.min_size, 4);
        assert_eq!(cfg.max_size, 10);
        assert_eq!(cfg.locale, "en-US");
        assert_eq!(cfg.theme, "default");
    }

    #[test]
    fn parse_minimal_toml() {
        let toml = r#"
min_size = 5
max_size = 8
locale = "fr-FR"
theme = "dark"
"#;
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.min_size, 5);
        assert_eq!(cfg.max_size, 8);
        assert_eq!(cfg.locale, "fr-FR");
        assert_eq!(cfg.theme, "dark");
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = std::env::temp_dir().join(format!("anagram-cfg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("nope.toml");
        let _ = std::fs::remove_file(&path);
        let cfg = AppConfig::load(Some(&path)).unwrap();
        assert_eq!(cfg, AppConfig::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_file() {
        let dir = std::env::temp_dir().join(format!("anagram-cfg-load-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"min_size = 6
max_size = 9
"#
        )
        .unwrap();
        drop(f);
        let cfg = AppConfig::load(Some(&path)).unwrap();
        assert_eq!(cfg.min_size, 6);
        assert_eq!(cfg.max_size, 9);
        assert_eq!(cfg.locale, "en-US");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clamp_size_respects_bounds() {
        let cfg = AppConfig {
            min_size: 4,
            max_size: 8,
            ..Default::default()
        };
        assert_eq!(cfg.clamp_size(2), 4);
        assert_eq!(cfg.clamp_size(6), 6);
        assert_eq!(cfg.clamp_size(12), 8);
    }

    #[test]
    fn default_workbench_enabled_and_dims() {
        let cfg = AppConfig::default();
        assert!(cfg.workbench);
        assert_eq!(cfg.workbench_rows, 5);
        assert_eq!(cfg.workbench_cols, 5);
    }

    #[test]
    fn parse_workbench_toml_fields() {
        let toml = r#"
workbench = false
workbench_rows = 4
workbench_cols = 4
"#;
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        assert!(!cfg.workbench);
        assert_eq!(cfg.workbench_rows, 4);
        assert_eq!(cfg.workbench_cols, 4);
    }

    #[test]
    fn workbench_settings_clamps_invalid_dims() {
        let cfg = AppConfig {
            workbench_rows: 3,
            workbench_cols: 5,
            ..Default::default()
        };
        let ws = cfg.workbench_settings();
        assert_eq!(ws.rows, 5);
        assert_eq!(ws.cols, 5);
    }

    #[test]
    fn workbench_settings_returns_expected() {
        let cfg = AppConfig {
            workbench: false,
            workbench_rows: 4,
            workbench_cols: 4,
            ..Default::default()
        };
        let ws = cfg.workbench_settings();
        assert_eq!(
            ws,
            WorkbenchSettings {
                enabled: false,
                rows: 4,
                cols: 4,
            }
        );

        let cfg_default = AppConfig::default();
        assert_eq!(
            cfg_default.workbench_settings(),
            WorkbenchSettings {
                enabled: true,
                rows: 5,
                cols: 5,
            }
        );

        let cfg_mismatch = AppConfig {
            workbench_rows: 4,
            workbench_cols: 5,
            ..Default::default()
        };
        assert_eq!(
            cfg_mismatch.workbench_settings(),
            WorkbenchSettings {
                enabled: true,
                rows: 5,
                cols: 5,
            }
        );
    }
}
