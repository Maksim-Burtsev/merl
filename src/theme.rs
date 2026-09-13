//! Embedded `.tmTheme` palettes and user config.
//!
//! Everything outside this module sees a [`Theme`] of ratatui colors plus the parsed syntect
//! theme the highlighter needs.

use std::io::Cursor;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use ratatui::style::{Color, Modifier};
use serde::Deserialize;
use syntect::highlighting::{Color as SynColor, FontStyle, ThemeSet};

/// Every theme merl ships, in the order shown to the user.
pub const NAMES: &[&str] = &["tokyonight-moon", "shokunin-light", "shokunin-dark"];

pub const DEFAULT: &str = "tokyonight-moon";

const TOKYONIGHT_MOON: &[u8] = include_bytes!("../themes/tokyonight-moon.tmTheme");
const SHOKUNIN_LIGHT: &[u8] = include_bytes!("../themes/shokunin-light.tmTheme");
const SHOKUNIN_DARK: &[u8] = include_bytes!("../themes/shokunin-dark.tmTheme");

/// The colors the editor chrome needs, resolved to opaque RGB. Syntax colors come from
/// [`Theme::syntect`] via [`style`].
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub gutter_fg: Color,
    pub line_hl: Color,
    /// The tree cursor row while the code pane has the keys: `line_hl` at half strength.
    pub line_hl_dim: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub find_bg: Color,
    pub find_fg: Color,
    /// Background of the Shift+Up/Down line selection.
    #[allow(dead_code)]
    pub selection: Color,
    pub syntect: syntect::highlighting::Theme,
}

pub fn load(name: &str) -> Result<Theme> {
    let bytes = match name {
        "tokyonight-moon" => TOKYONIGHT_MOON,
        "shokunin-light" => SHOKUNIN_LIGHT,
        "shokunin-dark" => SHOKUNIN_DARK,
        _ => bail!("unknown theme `{name}`; available: {}", NAMES.join(", ")),
    };
    let syntect = ThemeSet::load_from_reader(&mut Cursor::new(bytes))
        .with_context(|| format!("theme `{name}`"))?;

    let s = &syntect.settings;
    let bg = s.background.unwrap_or(SynColor::BLACK);
    let fg = s.foreground.unwrap_or(SynColor::WHITE);
    // tmTheme colors carry an alpha byte; flattening it over the background is what an editor
    // shows. Without this tokyonight's `lineHighlight` (#00000030) paints pure black.
    let over_bg = |c: SynColor| rgb(composite(c, bg));
    let line_hl_syn = s
        .line_highlight
        .map_or_else(|| mix(fg, bg, 12), |c| composite(c, bg));
    let line_hl = rgb(line_hl_syn);

    Ok(Theme {
        bg: rgb(bg),
        fg: rgb(fg),
        gutter_fg: s
            .gutter_foreground
            .map_or_else(|| blend(fg, bg, 45), over_bg),
        line_hl,
        line_hl_dim: blend(line_hl_syn, bg, 50),
        status_bg: line_hl,
        status_fg: rgb(fg),
        find_bg: s.find_highlight.map_or_else(|| blend(fg, bg, 35), over_bg),
        find_fg: s.find_highlight_foreground.map_or_else(|| rgb(bg), over_bg),
        selection: s.selection.map_or_else(|| blend(fg, bg, 25), over_bg),
        syntect,
    })
}

/// Converts one syntect span style into a ratatui style. The span background is ignored: merl
/// paints its own (theme background, or the cursor-line highlight).
pub fn style(s: syntect::highlighting::Style) -> ratatui::style::Style {
    let mut out = ratatui::style::Style::new().fg(rgb(s.foreground));
    for (flag, modifier) in [
        (FontStyle::BOLD, Modifier::BOLD),
        (FontStyle::ITALIC, Modifier::ITALIC),
        (FontStyle::UNDERLINE, Modifier::UNDERLINED),
    ] {
        if s.font_style.contains(flag) {
            out = out.add_modifier(modifier);
        }
    }
    out
}

fn rgb(c: SynColor) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// `fg` over `bg` at `fg`'s own alpha.
fn composite(c: SynColor, bg: SynColor) -> SynColor {
    if c.a == 255 {
        return c;
    }
    mix(c, bg, c.a as u32 * 100 / 255)
}

/// `fg` over `bg` at `percent` opacity, as a ratatui color.
fn blend(fg: SynColor, bg: SynColor, percent: u32) -> Color {
    rgb(mix(fg, bg, percent))
}

fn mix(fg: SynColor, bg: SynColor, percent: u32) -> SynColor {
    let m = |f: u8, b: u8| ((f as u32 * percent + b as u32 * (100 - percent)) / 100) as u8;
    SynColor {
        r: m(fg.r, bg.r),
        g: m(fg.g, bg.g),
        b: m(fg.b, bg.b),
        a: 255,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_theme_parses() {
        for name in NAMES {
            let t = load(name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            assert!(matches!(t.bg, Color::Rgb(..)), "{name} has no background");
            assert!(matches!(t.fg, Color::Rgb(..)), "{name} has no foreground");
            assert!(!t.syntect.scopes.is_empty(), "{name} has no token rules");
        }
    }

    #[test]
    fn unknown_theme_lists_the_valid_ones() {
        let e = load("nope").unwrap_err().to_string();
        for name in NAMES {
            assert!(e.contains(name), "{e}");
        }
    }

    #[test]
    fn alpha_is_composited_over_the_background() {
        // tokyonight-moon's lineHighlight is #00000030: without compositing it is pure black,
        // and its gutterForeground #3b415caa would be too dark to read.
        let t = load("tokyonight-moon").unwrap();
        assert_eq!(t.bg, Color::Rgb(0x22, 0x24, 0x36));
        assert_ne!(t.line_hl, Color::Rgb(0, 0, 0));
        let Color::Rgb(r, g, b) = t.line_hl else {
            unreachable!()
        };
        // 0x30/255 ≈ 19% black over #222436.
        assert!(r > 0x10 && g > 0x10 && b > 0x20, "{r:02x}{g:02x}{b:02x}");
        assert_ne!(t.gutter_fg, Color::Rgb(0x3b, 0x41, 0x5c));
    }

    #[test]
    fn unfocused_tree_row_sits_between_the_background_and_the_cursor_row() {
        for name in NAMES {
            let t = load(name).unwrap();
            assert_ne!(t.line_hl_dim, t.bg, "{name}");
            assert_ne!(t.line_hl_dim, t.line_hl, "{name}");
        }
    }
}
