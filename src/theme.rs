//! Embedded `.tmTheme` palettes and user config.
//!
//! Everything outside this module sees a [`Theme`] of ratatui colors plus the parsed syntect
//! theme the highlighter needs.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ratatui::style::{Color, Modifier};
use serde::Deserialize;
use syntect::highlighting::{Color as SynColor, FontStyle, Highlighter, ThemeSet};
use syntect::parsing::Scope;

/// Every theme merl ships, in the order shown to the user. `tools/port-theme.sh` prints the row
/// for a new one.
#[rustfmt::skip]
const THEMES: &[(&str, &[u8])] = &[
    ("tokyonight-moon", include_bytes!("../themes/tokyonight-moon.tmTheme")),
    ("tokyonight-night", include_bytes!("../themes/tokyonight-night.tmTheme")),
    ("tokyonight-storm", include_bytes!("../themes/tokyonight-storm.tmTheme")),
    ("kanagawa-wave", include_bytes!("../themes/kanagawa-wave.tmTheme")),
    ("kanagawa-dragon", include_bytes!("../themes/kanagawa-dragon.tmTheme")),
    ("rose-pine", include_bytes!("../themes/rose-pine.tmTheme")),
    ("rose-pine-moon", include_bytes!("../themes/rose-pine-moon.tmTheme")),
    ("everforest-dark", include_bytes!("../themes/everforest-dark.tmTheme")),
    ("gruvbox-material-dark", include_bytes!("../themes/gruvbox-material-dark.tmTheme")),
    ("gruvbox-dark", include_bytes!("../themes/gruvbox-dark.tmTheme")),
    ("catppuccin-mocha", include_bytes!("../themes/catppuccin-mocha.tmTheme")),
    ("catppuccin-macchiato", include_bytes!("../themes/catppuccin-macchiato.tmTheme")),
    ("catppuccin-frappe", include_bytes!("../themes/catppuccin-frappe.tmTheme")),
    ("flexoki-dark", include_bytes!("../themes/flexoki-dark.tmTheme")),
    ("melange-dark", include_bytes!("../themes/melange-dark.tmTheme")),
    ("nightfox", include_bytes!("../themes/nightfox.tmTheme")),
    ("nordfox", include_bytes!("../themes/nordfox.tmTheme")),
    ("nord", include_bytes!("../themes/nord.tmTheme")),
    ("onedark", include_bytes!("../themes/onedark.tmTheme")),
    ("github-dark", include_bytes!("../themes/github-dark.tmTheme")),
    ("vscode-dark", include_bytes!("../themes/vscode-dark.tmTheme")),
    ("dracula", include_bytes!("../themes/dracula.tmTheme")),
    ("solarized-dark", include_bytes!("../themes/solarized-dark.tmTheme")),
    ("oxocarbon-dark", include_bytes!("../themes/oxocarbon-dark.tmTheme")),
    ("sonokai", include_bytes!("../themes/sonokai.tmTheme")),
    ("material-dark", include_bytes!("../themes/material-dark.tmTheme")),
    ("shokunin-dark", include_bytes!("../themes/shokunin-dark.tmTheme")),
    ("adwaita", include_bytes!("../themes/adwaita.tmTheme")),
    ("alabaster", include_bytes!("../themes/alabaster.tmTheme")),
    ("ayu", include_bytes!("../themes/ayu.tmTheme")),
    ("ayu-mirage", include_bytes!("../themes/ayu-mirage.tmTheme")),
    ("bamboo", include_bytes!("../themes/bamboo.tmTheme")),
    ("cendre", include_bytes!("../themes/cendre.tmTheme")),
    ("darkearth", include_bytes!("../themes/darkearth.tmTheme")),
    ("e-ink", include_bytes!("../themes/e-ink.tmTheme")),
    ("edge", include_bytes!("../themes/edge.tmTheme")),
    ("gotham", include_bytes!("../themes/gotham.tmTheme")),
    ("hojicha", include_bytes!("../themes/hojicha.tmTheme")),
    ("iceberg", include_bytes!("../themes/iceberg.tmTheme")),
    ("jellybeans", include_bytes!("../themes/jellybeans.tmTheme")),
    ("koda-dark", include_bytes!("../themes/koda-dark.tmTheme")),
    ("lackluster", include_bytes!("../themes/lackluster.tmTheme")),
    ("mellifluous", include_bytes!("../themes/mellifluous.tmTheme")),
    ("mellow", include_bytes!("../themes/mellow.tmTheme")),
    ("miasma", include_bytes!("../themes/miasma.tmTheme")),
    ("minischeme", include_bytes!("../themes/minischeme.tmTheme")),
    ("moonfly", include_bytes!("../themes/moonfly.tmTheme")),
    ("nightfly", include_bytes!("../themes/nightfly.tmTheme")),
    ("papercolor", include_bytes!("../themes/papercolor.tmTheme")),
    ("pencil", include_bytes!("../themes/pencil.tmTheme")),
    ("selenized", include_bytes!("../themes/selenized.tmTheme")),
    ("soviet-dark", include_bytes!("../themes/soviet-dark.tmTheme")),
    ("srcery", include_bytes!("../themes/srcery.tmTheme")),
    ("token", include_bytes!("../themes/token.tmTheme")),
    ("vague", include_bytes!("../themes/vague.tmTheme")),
    ("vesper", include_bytes!("../themes/vesper.tmTheme")),
    ("rose-pine-dawn", include_bytes!("../themes/rose-pine-dawn.tmTheme")),
    ("kanagawa-lotus", include_bytes!("../themes/kanagawa-lotus.tmTheme")),
    ("everforest-light", include_bytes!("../themes/everforest-light.tmTheme")),
    ("flexoki-light", include_bytes!("../themes/flexoki-light.tmTheme")),
    ("catppuccin-latte", include_bytes!("../themes/catppuccin-latte.tmTheme")),
    ("gruvbox-material-light", include_bytes!("../themes/gruvbox-material-light.tmTheme")),
    ("gruvbox-light", include_bytes!("../themes/gruvbox-light.tmTheme")),
    ("melange-light", include_bytes!("../themes/melange-light.tmTheme")),
    ("dawnfox", include_bytes!("../themes/dawnfox.tmTheme")),
    ("dayfox", include_bytes!("../themes/dayfox.tmTheme")),
    ("tokyonight-day", include_bytes!("../themes/tokyonight-day.tmTheme")),
    ("onedark-light", include_bytes!("../themes/onedark-light.tmTheme")),
    ("github-light", include_bytes!("../themes/github-light.tmTheme")),
    ("vscode-light", include_bytes!("../themes/vscode-light.tmTheme")),
    ("alucard", include_bytes!("../themes/alucard.tmTheme")),
    ("solarized-light", include_bytes!("../themes/solarized-light.tmTheme")),
    ("oxocarbon-light", include_bytes!("../themes/oxocarbon-light.tmTheme")),
    ("material-light", include_bytes!("../themes/material-light.tmTheme")),
    ("bluloco-light", include_bytes!("../themes/bluloco-light.tmTheme")),
    ("shokunin-light", include_bytes!("../themes/shokunin-light.tmTheme")),
    ("adwaita-light", include_bytes!("../themes/adwaita-light.tmTheme")),
    ("alabaster-light", include_bytes!("../themes/alabaster-light.tmTheme")),
    ("ayu-light", include_bytes!("../themes/ayu-light.tmTheme")),
    ("bamboo-light", include_bytes!("../themes/bamboo-light.tmTheme")),
    ("e-ink-light", include_bytes!("../themes/e-ink-light.tmTheme")),
    ("edge-light", include_bytes!("../themes/edge-light.tmTheme")),
    ("iceberg-light", include_bytes!("../themes/iceberg-light.tmTheme")),
    ("jellybeans-light", include_bytes!("../themes/jellybeans-light.tmTheme")),
    ("koda-light", include_bytes!("../themes/koda-light.tmTheme")),
    ("lightearth", include_bytes!("../themes/lightearth.tmTheme")),
    ("mellifluous-light", include_bytes!("../themes/mellifluous-light.tmTheme")),
    ("minischeme-light", include_bytes!("../themes/minischeme-light.tmTheme")),
    ("neomodern-light", include_bytes!("../themes/neomodern-light.tmTheme")),
    ("papercolor-light", include_bytes!("../themes/papercolor-light.tmTheme")),
    ("pencil-light", include_bytes!("../themes/pencil-light.tmTheme")),
    ("selenized-light", include_bytes!("../themes/selenized-light.tmTheme")),
    ("soviet-light", include_bytes!("../themes/soviet-light.tmTheme")),
    ("token-light", include_bytes!("../themes/token-light.tmTheme")),
];

pub const DEFAULT: &str = "tokyonight-moon";

/// The names of [`THEMES`], in order.
pub fn names() -> impl Iterator<Item = &'static str> {
    THEMES.iter().map(|(name, _)| *name)
}

/// Where merl looks for the user's own themes.
pub fn user_dir() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config/merl/themes"))
}

/// Every theme that can be loaded: the built-ins in their order, then the user's files by name.
/// A file named after a built-in replaces it, in its place, which is what [`load`] reads too.
pub fn entries() -> Vec<String> {
    entries_in(user_dir().as_deref())
}

fn entries_in(dir: Option<&Path>) -> Vec<String> {
    let mut names: Vec<String> = names().map(str::to_string).collect();
    let mut user = user_names(dir);
    user.sort();
    let extra: Vec<String> = user.into_iter().filter(|n| !names.contains(n)).collect();
    names.extend(extra);
    names
}

/// The names of the `.tmTheme` files in `dir`. A missing or unreadable directory has none.
fn user_names(dir: Option<&Path>) -> Vec<String> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "tmTheme"))
        .filter_map(|p| Some(p.file_stem()?.to_string_lossy().into_owned()))
        .collect()
}

/// The user's file for `name`, when there is one.
fn user_path(dir: Option<&Path>, name: &str) -> Option<PathBuf> {
    dir.map(|d| d.join(format!("{name}.tmTheme")))
        .filter(|p| p.is_file())
}

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
    /// Review's deleted lines: the text colour greyed toward the background, but never below a
    /// contrast a reviewer can read. `gutter_fg` is too faint for text in most themes.
    pub ghost_fg: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub find_bg: Color,
    pub find_fg: Color,
    /// The theme's signature colour, painted on the chrome the user navigates by: directory
    /// names, the tree and picker frames, the file name in the status bar. Taken from the colour
    /// the theme gives function names, or the first other scope it colours, so every theme has
    /// one without a new key.
    pub accent: Color,
    /// Background of the Shift+Up/Down line selection.
    #[allow(dead_code)]
    pub selection: Color,
    pub syntect: syntect::highlighting::Theme,
}

pub fn load(name: &str) -> Result<Theme> {
    load_from(user_dir().as_deref(), name)
}

fn load_from(dir: Option<&Path>, name: &str) -> Result<Theme> {
    // A user file shadows the built-in of the same name; a broken one is an error, never a
    // silent fall back to the built-in it shadows.
    let (bytes, ctx) = match user_path(dir, name) {
        Some(path) => {
            let ctx = path.display().to_string();
            (std::fs::read(&path).with_context(|| ctx.clone())?, ctx)
        }
        None => match THEMES.iter().find(|(n, _)| *n == name) {
            Some((_, bytes)) => (bytes.to_vec(), format!("theme `{name}`")),
            None => {
                let all = entries_in(dir);
                bail!("unknown theme `{name}`; available: {}", all.join(", "));
            }
        },
    };
    let syntect = ThemeSet::load_from_reader(&mut Cursor::new(&bytes[..])).with_context(|| ctx)?;

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
    // The chrome colours are derived, so on a palette that barely varies two of them can land on
    // the same value and the row they paint stops reading as marked. Each fallback below pushes
    // the collision apart instead of leaving the chrome flat.
    let mut line_hl_dim = blend(line_hl_syn, bg, 50);
    for percent in [7, 12, 20] {
        if line_hl_dim != rgb(bg) && line_hl_dim != line_hl {
            break;
        }
        line_hl_dim = blend(fg, bg, percent);
    }
    // As grey as still reads: the weakest mix that reaches 4:1 against the background. A theme
    // whose own text is below ~5.4:1 never reaches it, so there the mix stops at 85 %, which keeps
    // the ghost greyer than the text and within a quarter of the text's own contrast.
    let ghost_fg = (50..=GHOST_MAX)
        .step_by(5)
        .map(|percent| blend(fg, bg, percent))
        .find(|&c| contrast(c, rgb(bg)) >= GHOST_CONTRAST)
        .unwrap_or_else(|| blend(fg, bg, GHOST_MAX));
    // The selection is drawn over the cursor line (#62), so a theme whose own selection colour
    // sits within a few points of it gets one blended further from the background instead.
    let mut selection = s.selection.map_or_else(|| blend(fg, bg, 25), over_bg);
    for percent in [35, 45, 55, 65] {
        if apart(selection, line_hl) {
            break;
        }
        selection = blend(fg, bg, percent);
    }

    Ok(Theme {
        bg: rgb(bg),
        fg: rgb(fg),
        gutter_fg: s
            .gutter_foreground
            .map_or_else(|| blend(fg, bg, 45), over_bg),
        line_hl,
        line_hl_dim,
        ghost_fg,
        status_bg: line_hl,
        status_fg: rgb(fg),
        find_bg: s.find_highlight.map_or_else(|| blend(fg, bg, 35), over_bg),
        find_fg: s.find_highlight_foreground.map_or_else(|| rgb(bg), over_bg),
        selection,
        accent: rgb(accent_color(&syntect).unwrap_or(fg)),
        syntect,
    })
}

/// The theme's signature colour: what it paints function names with, or — for the minimal themes
/// that leave functions in the plain text colour — the first other scope it does colour. `None`
/// when a theme paints every one of them like plain text.
fn accent_color(theme: &syntect::highlighting::Theme) -> Option<SynColor> {
    let highlighter = Highlighter::new(theme);
    [
        "entity.name.function",
        "keyword",
        "string",
        "constant.numeric",
        "variable",
    ]
    .into_iter()
    .find_map(|scope| {
        let style = highlighter.style_for_stack(&[Scope::new(scope).ok()?]);
        (Some(style.foreground) != theme.settings.foreground).then_some(style.foreground)
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

/// The contrast `ghost_fg` aims for, and the most of the text colour it may take to get there.
const GHOST_CONTRAST: f64 = 4.0;
const GHOST_MAX: u32 = 85;

/// WCAG contrast ratio of two colours, 1.0 (the same) to 21.0 (black on white).
fn contrast(a: Color, b: Color) -> f64 {
    let lum = |c: Color| {
        let Color::Rgb(r, g, b) = c else { return 0.0 };
        let lin = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
    };
    let (a, b) = (lum(a), lum(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Whether two chrome colours read as two colours: at least 12 points apart on one channel, the
/// distance `every_theme_has_the_basics` requires of the selection and the cursor line.
fn apart(a: Color, b: Color) -> bool {
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) else {
        return true;
    };
    r1.abs_diff(r2) >= 12 || g1.abs_diff(g2) >= 12 || b1.abs_diff(b2) >= 12
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
    /// Edits are written this long after the last keystroke; VS Code's `files.autoSaveDelay`.
    #[serde(default = "default_autosave")]
    pub autosave_delay_ms: u64,
}

fn default_autosave() -> u64 {
    1000
}

fn default_theme() -> String {
    DEFAULT.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            autosave_delay_ms: default_autosave(),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config/merl/config.toml"))
}

/// Writes `theme = "NAME"` into the config file at `path`: over its `theme` line, or appended,
/// so the rest of the file and its comments stay as they were. A missing file is created.
pub fn save(path: &Path, name: &str) -> Result<()> {
    let ctx = || format!("{}", path.display());
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(ctx),
    };
    let line = format!("theme = \"{name}\"");
    let re = regex::Regex::new(r"(?m)^[ \t]*theme[ \t]*=.*$").expect("a valid regex");
    let text = if re.is_match(&text) {
        re.replace(&text, regex::NoExpand(&line)).into_owned()
    } else if text.is_empty() || text.ends_with('\n') {
        format!("{text}{line}\n")
    } else {
        format!("{text}\n{line}\n")
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(ctx)?;
    }
    std::fs::write(path, text).with_context(ctx)
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
    use std::collections::HashSet;

    use super::*;
    use crate::buffer::Buffer;

    /// A broken port fails here: the chrome colours are set, the selection does not pass for the
    /// cursor line, and comment, keyword, string and function are not painted in one or two
    /// colours between them.
    #[test]
    fn every_theme_has_the_basics() {
        for name in names() {
            let t = load(name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            let s = &t.syntect.settings;
            assert!(s.background.is_some(), "{name} has no background");
            assert!(s.foreground.is_some(), "{name} has no foreground");
            assert!(s.line_highlight.is_some(), "{name} has no lineHighlight");
            // The selection is drawn on top of the cursor line highlight (#62), so a selection
            // inside the cursor line is only as visible as these two colours are apart. #48 was
            // reported on a pair 5 apart; 12 is where it reads as two colours.
            let (Color::Rgb(r, g, b), Color::Rgb(r2, g2, b2)) = (t.selection, t.line_hl) else {
                panic!("{name}: chrome colours are not RGB");
            };
            let apart = [r.abs_diff(r2), g.abs_diff(g2), b.abs_diff(b2)];
            assert!(
                apart.iter().any(|d| *d >= 12),
                "{name}: the selection {:?} passes for the cursor line {:?}",
                t.selection,
                t.line_hl
            );
            let highlighter = Highlighter::new(&t.syntect);
            let colours: HashSet<_> = ["comment", "keyword", "string", "entity.name.function"]
                .into_iter()
                .map(|scope| {
                    let c = highlighter
                        .style_for_stack(&[Scope::new(scope).unwrap()])
                        .foreground;
                    (c.r, c.g, c.b)
                })
                .collect();
            assert!(colours.len() >= 3, "{name}: {colours:?}");
        }
    }

    /// A `.tmTheme` in the user's directory is listed after the built-ins and loads; one named
    /// after a built-in replaces it. A broken file names itself in the error instead of falling
    /// back to the built-in it shadows.
    #[test]
    fn user_themes_are_listed_loaded_and_win_over_a_built_in() {
        let dir = std::env::temp_dir().join(format!("merl-user-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let light = std::fs::read(format!(
            "{}/themes/dayfox.tmTheme",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        std::fs::write(dir.join("mine.tmTheme"), &light).unwrap();
        std::fs::write(dir.join(format!("{DEFAULT}.tmTheme")), &light).unwrap();

        let entries = entries_in(Some(&dir));
        assert_eq!(
            entries.len(),
            THEMES.len() + 1,
            "one new name, one shadowed"
        );
        assert_eq!(
            entries.last().unwrap().as_str(),
            "mine",
            "after the built-ins"
        );
        // Shadowing a built-in loads the user's file, not the theme it covers.
        assert_eq!(
            load_from(Some(&dir), DEFAULT).unwrap().bg,
            load_from(None, "dayfox").unwrap().bg
        );
        let e = load_from(Some(&dir), "nope").unwrap_err().to_string();
        assert!(
            e.contains("mine"),
            "an unknown name lists the user's too: {e}"
        );

        std::fs::write(dir.join("broken.tmTheme"), b"not a plist").unwrap();
        let e = load_from(Some(&dir), "broken").unwrap_err().to_string();
        assert!(e.contains("broken.tmTheme"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The gallery in docs/themes.md shows every theme, with its screenshot, and nothing else.
    #[test]
    fn gallery_shows_every_theme() {
        let page = include_str!("../docs/themes.md");
        let shown: Vec<&str> = page
            .split("../assets/themes/")
            .skip(1)
            .filter_map(|s| s.split_once(".png"))
            .map(|(name, _)| name)
            .collect();
        for name in names() {
            assert!(shown.contains(&name), "docs/themes.md is missing {name}");
            let shot = format!("{}/assets/themes/{name}.png", env!("CARGO_MANIFEST_DIR"));
            assert!(
                Path::new(&shot).exists(),
                "{shot} is missing; run tools/theme-shots.sh {name}"
            );
        }
        for name in &shown {
            assert!(
                names().any(|n| n == *name),
                "docs/themes.md shows {name}, which is not a theme"
            );
        }
    }

    /// The infrastructure half of a repo (#16): a grammar that highlights, under a theme with no
    /// rule for its scopes, looks like no highlighting at all.
    #[test]
    fn every_theme_colours_infra_files() {
        const SAMPLES: &[(&str, &str)] = &[
            (
                "compose.yaml",
                "services:\n  web:\n    image: \"nginx:1.27\"  # public\n    ports: [8080]\n",
            ),
            (
                "Dockerfile",
                "FROM rust:1.85 AS build\n# compile\nRUN cargo build --release\nENV PORT=8080\n",
            ),
            (
                "Makefile",
                "BIN := merl\n# install it\ninstall: build\n\tcp target/release/$(BIN) /usr/local/bin\n",
            ),
            (
                "Cargo.toml",
                "[package]\nname = \"merl\"  # the binary\nversion = \"0.3.0\"\nedition = 2024\n",
            ),
        ];
        for name in names() {
            let theme = load(name).unwrap();
            for (file, text) in SAMPLES {
                let mut b = Buffer::from_bytes(PathBuf::from(file), text.as_bytes());
                b.highlight_to(usize::MAX, &theme);
                let colours: HashSet<_> = b.hl.iter().flatten().filter_map(|(s, _)| s.fg).collect();
                assert!(colours.len() >= 3, "{name} paints {file} in {colours:?}");
            }
        }
    }

    #[test]
    fn save_rewrites_only_the_theme_line() {
        let dir = std::env::temp_dir().join(format!("merl-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("merl/config.toml");
        let read = || std::fs::read_to_string(&path).unwrap();

        save(&path, "dayfox").unwrap();
        assert_eq!(read(), "theme = \"dayfox\"\n", "a missing file is created");

        std::fs::write(&path, "# mine\ntheme = \"nordfox\"\n# end\n").unwrap();
        save(&path, "dawnfox").unwrap();
        assert_eq!(read(), "# mine\ntheme = \"dawnfox\"\n# end\n");

        std::fs::write(&path, "# no theme yet").unwrap();
        save(&path, "rose-pine").unwrap();
        assert_eq!(read(), "# no theme yet\ntheme = \"rose-pine\"\n");
        assert_eq!(
            toml::from_str::<Config>(&read()).unwrap().theme,
            "rose-pine"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_theme_lists_the_valid_ones() {
        let e = load("nope").unwrap_err().to_string();
        for name in names() {
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
    fn accent_is_the_function_colour_and_differs_from_the_text() {
        // tokyonight-moon paints functions #82aaff, the blue LazyVim uses for directories.
        let t = load("tokyonight-moon").unwrap();
        assert_eq!(t.accent, Color::Rgb(0x82, 0xaa, 0xff));
        for name in names() {
            let t = load(name).unwrap();
            assert_ne!(t.accent, t.fg, "{name}");
            assert_ne!(t.accent, t.bg, "{name}");
        }
    }

    /// Names, keys and sections the infrastructure grammars emit. A theme with no rule for one
    /// paints it in the default foreground, which reads as no highlighting at all.
    #[test]
    fn infra_scopes_are_coloured_in_every_theme() {
        // Whole stacks as the grammars emit them: a parent such as `meta.tag` often carries the
        // colour, so a lone scope would not match what is drawn.
        const STACKS: &[&str] = &[
            "source.toml meta.tag.key.toml entity.name.tag.toml",
            "source.toml meta.tag.table.toml entity.name.table.toml",
            "source.ini meta.tag.section.ini entity.section.ini",
            "source.nginx meta.context.server.nginx meta.context.location.nginx \
             entity.name.context.location.nginx",
            "source.nginx meta.context.server.nginx punctuation.definition.variable \
             variable.other.nginx",
            "source.terraform meta.block.terraform variable.declaration.terraform \
             variable.other.readwrite.terraform",
            "source.terraform variable.other.member.terraform",
            "source.terraform constant.language.terraform",
            "source.env variable.other.env",
            "source.env constant.language.env",
            "source.makefile variable.other.makefile",
            "source.makefile meta.function.body.makefile source.shell variable.parameter.makefile",
            "source.makefile meta.function.body.makefile source.shell \
             meta.function-call.arguments.shell variable.language.automatic.makefile",
            "source.dockerfile.bash variable.stage-name",
            "source.dockerfile.bash entity.name.enum.tag-digest",
            "source.dockerfile.bash source.shell variable.parameter.option.shell",
            "source.yaml meta.property.yaml entity.name.other.anchor.yaml",
            "source.yaml variable.other.alias.yaml",
            "source.yaml constant.language.merge.yaml",
            "text.git.ignore string.unquoted.git.ignore entity.name.pattern.git.ignore",
        ];
        for name in names() {
            let t = load(name).unwrap();
            let hl = Highlighter::new(&t.syntect);
            for stack in STACKS {
                let scopes: syntect::parsing::ScopeStack = stack.parse().unwrap();
                let style = hl.style_for_stack(scopes.as_slice());
                assert_ne!(
                    Some(style.foreground),
                    t.syntect.settings.foreground,
                    "{name} leaves `{stack}` in the default colour"
                );
            }
        }
    }

    #[test]
    fn unfocused_tree_row_sits_between_the_background_and_the_cursor_row() {
        for name in names() {
            let t = load(name).unwrap();
            assert_ne!(t.line_hl_dim, t.bg, "{name}");
            assert_ne!(t.line_hl_dim, t.line_hl, "{name}");
        }
    }

    #[test]
    fn deleted_lines_are_grey_but_readable_in_every_theme() {
        for name in names() {
            let t = load(name).unwrap();
            let (text, ghost) = (contrast(t.fg, t.bg), contrast(t.ghost_fg, t.bg));
            // Greyed: clearly weaker than live text.
            assert!(
                ghost <= text * 0.9,
                "{name}: ghost {ghost:.2} vs text {text:.2}"
            );
            // Readable: 4:1, or, where the theme's own text is too soft for a grey of it to
            // get there (material-light is 2.5:1 itself), most of what the text has.
            assert!(
                ghost >= GHOST_CONTRAST || ghost >= text * 0.7,
                "{name}: ghost {ghost:.2} vs text {text:.2}"
            );
            assert!(ghost >= 2.0, "{name}: ghost {ghost:.2}");
        }
    }
}
