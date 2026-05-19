//! TOML config stub — implemented in Phase 9.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EdexConfig {
    pub appearance: AppearanceConfig,
    pub terminal: TerminalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub theme: String,
    pub font_size: f32,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "tron".to_string(),
            font_size: 14.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub shell: String,
    pub font_size: f32,
    pub scrollback_lines: usize,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
            font_size: 13.0,
            scrollback_lines: 10_000,
        }
    }
}
