//! Theme palette and user config.
//!
//! Step 2 replaces [`load`] with real embedded `.tmTheme` parsing; everything outside this
//! module only ever sees a [`Theme`] of ratatui colors, so `ui.rs` will not change.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use ratatui::style::Color;
use serde::Deserialize;

/// Every theme merl ships, in the order shown to the user.
pub const NAMES: &[&str] = &["tokyonight-moon", "shokunin-light", "shokunin-dark"];

pub const DEFAULT: &str = "tokyonight-moon";

/// The handful of colors the editor chrome needs. Syntax colors come later, from syntect.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub gutter_fg: Color,
    pub line_hl: Color,
    pub status_bg: Color,
    pub status_fg: Color,
}

/// ponytail: placeholder palette, identical for every name. Step 2 parses the real tmThemes.
pub fn load(name: &str) -> Result<Theme> {
    if !NAMES.contains(&name) {
        bail!("unknown theme `{name}`; available: {}", NAMES.join(", "));
    }
    Ok(Theme {
        bg: Color::from_u32(0x0022_2436),
        fg: Color::from_u32(0x00c8_d3f5),
        gutter_fg: Color::from_u32(0x0044_4a73),
        line_hl: Color::from_u32(0x002f_334d),
        status_bg: Color::from_u32(0x001e_2030),
        status_fg: Color::from_u32(0x00c8_d3f5),
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    DEFAULT.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config/merl/config.toml"))
}

/// Reads `~/.config/merl/config.toml`. A missing file is not an error; a broken one is.
pub fn config() -> Result<Config> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).with_context(|| format!("{}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("{}", path.display())),
    }
}
