use std::{fs, path::Path, sync::OnceLock};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use super::colors::{Theme, AMBER, MATRIX, TRON};

pub const TRON_TOML: &str = include_str!("../../../themes/tron.toml");
pub const MATRIX_TOML: &str = include_str!("../../../themes/matrix.toml");
pub const AMBER_TOML: &str = include_str!("../../../themes/amber.toml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub background: String,
    pub panel_bg: String,
    pub border: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub accent: String,
}

impl ThemeConfig {
    pub fn to_theme(&self) -> Result<Theme> {
        Ok(Theme {
            background: parse_color(&self.background)?,
            panel_bg: parse_color(&self.panel_bg)?,
            border: parse_color(&self.border)?,
            border_glow: {
                let mut c = parse_color(&self.border)?;
                c[3] = 0.3;
                c
            },
            text_primary: parse_color(&self.text_primary)?,
            text_secondary: parse_color(&self.text_secondary)?,
            text_dim: {
                let mut c = parse_color(&self.text_secondary)?;
                c[3] = 0.5;
                c
            },
            accent: parse_color(&self.accent)?,
            warning: [1.0, 0.624, 0.0, 1.0],
            error: [1.0, 0.267, 0.267, 1.0],
        })
    }
}

pub fn load_theme_from_toml_str(toml_str: &str) -> Result<Theme> {
    toml::from_str::<ThemeConfig>(toml_str)
        .context("failed to deserialize theme config")?
        .to_theme()
}

pub fn load_theme_from_path(path: impl AsRef<Path>) -> Result<Theme> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read theme file {}", path.display()))?;
    load_theme_from_toml_str(&contents)
}

pub fn builtin_theme(name: &str) -> Result<&'static Theme> {
    static TRON_THEME: OnceLock<Theme> = OnceLock::new();
    static MATRIX_THEME: OnceLock<Theme> = OnceLock::new();
    static AMBER_THEME: OnceLock<Theme> = OnceLock::new();

    match name.to_ascii_lowercase().as_str() {
        "tron" => {
            Ok(TRON_THEME.get_or_init(|| load_theme_from_toml_str(TRON_TOML).unwrap_or(TRON)))
        }
        "matrix" => Ok(
            MATRIX_THEME.get_or_init(|| load_theme_from_toml_str(MATRIX_TOML).unwrap_or(MATRIX))
        ),
        "amber" => {
            Ok(AMBER_THEME.get_or_init(|| load_theme_from_toml_str(AMBER_TOML).unwrap_or(AMBER)))
        }
        other => Err(anyhow!("unknown builtin theme: {other}")),
    }
}

pub fn parse_color(hex: &str) -> Result<[f32; 4]> {
    if !hex.starts_with('#') || hex.len() != 7 {
        return Err(anyhow!("expected color in #rrggbb format, got {hex}"));
    }

    let r = u8::from_str_radix(&hex[1..3], 16).context("invalid red channel")?;
    let g = u8::from_str_radix(&hex[3..5], 16).context("invalid green channel")?;
    let b = u8::from_str_radix(&hex[5..7], 16).context("invalid blue channel")?;

    Ok([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
}
