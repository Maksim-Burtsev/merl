use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use syntect::highlighting::{Color as SynColor, FontStyle, Highlighter, ThemeSet};
use syntect::parsing::Scope;

#[rustfmt::skip]
const THEMES: &[(&str, &[u8], Option<&str>)] = &[
    ("tokyonight-moon", include_bytes!("../themes/tokyonight-moon.tmTheme"), Some(include_str!("../themes/tokyonight-moon.groups.json"))),
    ("tokyonight-night", include_bytes!("../themes/tokyonight-night.tmTheme"), Some(include_str!("../themes/tokyonight-night.groups.json"))),
    ("tokyonight-storm", include_bytes!("../themes/tokyonight-storm.tmTheme"), Some(include_str!("../themes/tokyonight-storm.groups.json"))),
    ("kanagawa-wave", include_bytes!("../themes/kanagawa-wave.tmTheme"), Some(include_str!("../themes/kanagawa-wave.groups.json"))),
    ("kanagawa-dragon", include_bytes!("../themes/kanagawa-dragon.tmTheme"), Some(include_str!("../themes/kanagawa-dragon.groups.json"))),
    ("rose-pine", include_bytes!("../themes/rose-pine.tmTheme"), Some(include_str!("../themes/rose-pine.groups.json"))),
    ("rose-pine-moon", include_bytes!("../themes/rose-pine-moon.tmTheme"), Some(include_str!("../themes/rose-pine-moon.groups.json"))),
    ("everforest-dark", include_bytes!("../themes/everforest-dark.tmTheme"), Some(include_str!("../themes/everforest-dark.groups.json"))),
    ("gruvbox-material-dark", include_bytes!("../themes/gruvbox-material-dark.tmTheme"), Some(include_str!("../themes/gruvbox-material-dark.groups.json"))),
    ("gruvbox-dark", include_bytes!("../themes/gruvbox-dark.tmTheme"), Some(include_str!("../themes/gruvbox-dark.groups.json"))),
    ("catppuccin-mocha", include_bytes!("../themes/catppuccin-mocha.tmTheme"), Some(include_str!("../themes/catppuccin-mocha.groups.json"))),
    ("catppuccin-macchiato", include_bytes!("../themes/catppuccin-macchiato.tmTheme"), Some(include_str!("../themes/catppuccin-macchiato.groups.json"))),
    ("catppuccin-frappe", include_bytes!("../themes/catppuccin-frappe.tmTheme"), Some(include_str!("../themes/catppuccin-frappe.groups.json"))),
    ("flexoki-dark", include_bytes!("../themes/flexoki-dark.tmTheme"), Some(include_str!("../themes/flexoki-dark.groups.json"))),
    ("melange-dark", include_bytes!("../themes/melange-dark.tmTheme"), Some(include_str!("../themes/melange-dark.groups.json"))),
    ("nightfox", include_bytes!("../themes/nightfox.tmTheme"), Some(include_str!("../themes/nightfox.groups.json"))),
    ("nordfox", include_bytes!("../themes/nordfox.tmTheme"), Some(include_str!("../themes/nordfox.groups.json"))),
    ("nord", include_bytes!("../themes/nord.tmTheme"), Some(include_str!("../themes/nord.groups.json"))),
    ("onedark", include_bytes!("../themes/onedark.tmTheme"), Some(include_str!("../themes/onedark.groups.json"))),
    ("github-dark", include_bytes!("../themes/github-dark.tmTheme"), Some(include_str!("../themes/github-dark.groups.json"))),
    ("vscode-dark", include_bytes!("../themes/vscode-dark.tmTheme"), Some(include_str!("../themes/vscode-dark.groups.json"))),
    ("dracula", include_bytes!("../themes/dracula.tmTheme"), Some(include_str!("../themes/dracula.groups.json"))),
    ("solarized-dark", include_bytes!("../themes/solarized-dark.tmTheme"), Some(include_str!("../themes/solarized-dark.groups.json"))),
    ("oxocarbon-dark", include_bytes!("../themes/oxocarbon-dark.tmTheme"), Some(include_str!("../themes/oxocarbon-dark.groups.json"))),
    ("sonokai", include_bytes!("../themes/sonokai.tmTheme"), Some(include_str!("../themes/sonokai.groups.json"))),
    ("material-dark", include_bytes!("../themes/material-dark.tmTheme"), Some(include_str!("../themes/material-dark.groups.json"))),
    ("shokunin-dark", include_bytes!("../themes/shokunin-dark.tmTheme"), None),
    ("adwaita", include_bytes!("../themes/adwaita.tmTheme"), Some(include_str!("../themes/adwaita.groups.json"))),
    ("alabaster", include_bytes!("../themes/alabaster.tmTheme"), Some(include_str!("../themes/alabaster.groups.json"))),
    ("ayu", include_bytes!("../themes/ayu.tmTheme"), Some(include_str!("../themes/ayu.groups.json"))),
    ("ayu-mirage", include_bytes!("../themes/ayu-mirage.tmTheme"), Some(include_str!("../themes/ayu-mirage.groups.json"))),
    ("bamboo", include_bytes!("../themes/bamboo.tmTheme"), Some(include_str!("../themes/bamboo.groups.json"))),
    ("cendre", include_bytes!("../themes/cendre.tmTheme"), Some(include_str!("../themes/cendre.groups.json"))),
    ("darkearth", include_bytes!("../themes/darkearth.tmTheme"), Some(include_str!("../themes/darkearth.groups.json"))),
    ("e-ink", include_bytes!("../themes/e-ink.tmTheme"), Some(include_str!("../themes/e-ink.groups.json"))),
    ("edge", include_bytes!("../themes/edge.tmTheme"), Some(include_str!("../themes/edge.groups.json"))),
    ("gotham", include_bytes!("../themes/gotham.tmTheme"), Some(include_str!("../themes/gotham.groups.json"))),
    ("hojicha", include_bytes!("../themes/hojicha.tmTheme"), Some(include_str!("../themes/hojicha.groups.json"))),
    ("iceberg", include_bytes!("../themes/iceberg.tmTheme"), Some(include_str!("../themes/iceberg.groups.json"))),
    ("jellybeans", include_bytes!("../themes/jellybeans.tmTheme"), Some(include_str!("../themes/jellybeans.groups.json"))),
    ("koda-dark", include_bytes!("../themes/koda-dark.tmTheme"), Some(include_str!("../themes/koda-dark.groups.json"))),
    ("lackluster", include_bytes!("../themes/lackluster.tmTheme"), Some(include_str!("../themes/lackluster.groups.json"))),
    ("lucerna", include_bytes!("../themes/lucerna.tmTheme"), None),
    ("mellifluous", include_bytes!("../themes/mellifluous.tmTheme"), Some(include_str!("../themes/mellifluous.groups.json"))),
    ("mellow", include_bytes!("../themes/mellow.tmTheme"), Some(include_str!("../themes/mellow.groups.json"))),
    ("miasma", include_bytes!("../themes/miasma.tmTheme"), Some(include_str!("../themes/miasma.groups.json"))),
    ("minischeme", include_bytes!("../themes/minischeme.tmTheme"), Some(include_str!("../themes/minischeme.groups.json"))),
    ("moonfly", include_bytes!("../themes/moonfly.tmTheme"), Some(include_str!("../themes/moonfly.groups.json"))),
    ("nightfly", include_bytes!("../themes/nightfly.tmTheme"), Some(include_str!("../themes/nightfly.groups.json"))),
    ("papercolor", include_bytes!("../themes/papercolor.tmTheme"), Some(include_str!("../themes/papercolor.groups.json"))),
    ("pencil", include_bytes!("../themes/pencil.tmTheme"), Some(include_str!("../themes/pencil.groups.json"))),
    ("selenized", include_bytes!("../themes/selenized.tmTheme"), Some(include_str!("../themes/selenized.groups.json"))),
    ("seneca", include_bytes!("../themes/seneca.tmTheme"), None),
    ("soviet-dark", include_bytes!("../themes/soviet-dark.tmTheme"), Some(include_str!("../themes/soviet-dark.groups.json"))),
    ("srcery", include_bytes!("../themes/srcery.tmTheme"), Some(include_str!("../themes/srcery.groups.json"))),
    ("token", include_bytes!("../themes/token.tmTheme"), Some(include_str!("../themes/token.groups.json"))),
    ("vague", include_bytes!("../themes/vague.tmTheme"), Some(include_str!("../themes/vague.groups.json"))),
    ("vesper", include_bytes!("../themes/vesper.tmTheme"), Some(include_str!("../themes/vesper.groups.json"))),
    ("rose-pine-dawn", include_bytes!("../themes/rose-pine-dawn.tmTheme"), Some(include_str!("../themes/rose-pine-dawn.groups.json"))),
    ("kanagawa-lotus", include_bytes!("../themes/kanagawa-lotus.tmTheme"), Some(include_str!("../themes/kanagawa-lotus.groups.json"))),
    ("everforest-light", include_bytes!("../themes/everforest-light.tmTheme"), Some(include_str!("../themes/everforest-light.groups.json"))),
    ("flexoki-light", include_bytes!("../themes/flexoki-light.tmTheme"), Some(include_str!("../themes/flexoki-light.groups.json"))),
    ("catppuccin-latte", include_bytes!("../themes/catppuccin-latte.tmTheme"), Some(include_str!("../themes/catppuccin-latte.groups.json"))),
    ("gruvbox-material-light", include_bytes!("../themes/gruvbox-material-light.tmTheme"), Some(include_str!("../themes/gruvbox-material-light.groups.json"))),
    ("gruvbox-light", include_bytes!("../themes/gruvbox-light.tmTheme"), Some(include_str!("../themes/gruvbox-light.groups.json"))),
    ("melange-light", include_bytes!("../themes/melange-light.tmTheme"), Some(include_str!("../themes/melange-light.groups.json"))),
    ("dawnfox", include_bytes!("../themes/dawnfox.tmTheme"), Some(include_str!("../themes/dawnfox.groups.json"))),
    ("dayfox", include_bytes!("../themes/dayfox.tmTheme"), Some(include_str!("../themes/dayfox.groups.json"))),
    ("tokyonight-day", include_bytes!("../themes/tokyonight-day.tmTheme"), Some(include_str!("../themes/tokyonight-day.groups.json"))),
    ("onedark-light", include_bytes!("../themes/onedark-light.tmTheme"), Some(include_str!("../themes/onedark-light.groups.json"))),
    ("github-light", include_bytes!("../themes/github-light.tmTheme"), Some(include_str!("../themes/github-light.groups.json"))),
    ("vscode-light", include_bytes!("../themes/vscode-light.tmTheme"), Some(include_str!("../themes/vscode-light.groups.json"))),
    ("alucard", include_bytes!("../themes/alucard.tmTheme"), Some(include_str!("../themes/alucard.groups.json"))),
    ("solarized-light", include_bytes!("../themes/solarized-light.tmTheme"), Some(include_str!("../themes/solarized-light.groups.json"))),
    ("oxocarbon-light", include_bytes!("../themes/oxocarbon-light.tmTheme"), Some(include_str!("../themes/oxocarbon-light.groups.json"))),
    ("material-light", include_bytes!("../themes/material-light.tmTheme"), Some(include_str!("../themes/material-light.groups.json"))),
    ("bluloco-light", include_bytes!("../themes/bluloco-light.tmTheme"), Some(include_str!("../themes/bluloco-light.groups.json"))),
    ("shokunin-light", include_bytes!("../themes/shokunin-light.tmTheme"), None),
    ("adwaita-light", include_bytes!("../themes/adwaita-light.tmTheme"), Some(include_str!("../themes/adwaita-light.groups.json"))),
    ("alabaster-light", include_bytes!("../themes/alabaster-light.tmTheme"), Some(include_str!("../themes/alabaster-light.groups.json"))),
    ("ayu-light", include_bytes!("../themes/ayu-light.tmTheme"), Some(include_str!("../themes/ayu-light.groups.json"))),
    ("bamboo-light", include_bytes!("../themes/bamboo-light.tmTheme"), Some(include_str!("../themes/bamboo-light.groups.json"))),
    ("e-ink-light", include_bytes!("../themes/e-ink-light.tmTheme"), Some(include_str!("../themes/e-ink-light.groups.json"))),
    ("edge-light", include_bytes!("../themes/edge-light.tmTheme"), Some(include_str!("../themes/edge-light.groups.json"))),
    ("iceberg-light", include_bytes!("../themes/iceberg-light.tmTheme"), Some(include_str!("../themes/iceberg-light.groups.json"))),
    ("jellybeans-light", include_bytes!("../themes/jellybeans-light.tmTheme"), Some(include_str!("../themes/jellybeans-light.groups.json"))),
    ("koda-light", include_bytes!("../themes/koda-light.tmTheme"), Some(include_str!("../themes/koda-light.groups.json"))),
    ("lightearth", include_bytes!("../themes/lightearth.tmTheme"), Some(include_str!("../themes/lightearth.groups.json"))),
    ("mellifluous-light", include_bytes!("../themes/mellifluous-light.tmTheme"), Some(include_str!("../themes/mellifluous-light.groups.json"))),
    ("minischeme-light", include_bytes!("../themes/minischeme-light.tmTheme"), Some(include_str!("../themes/minischeme-light.groups.json"))),
    ("neomodern-light", include_bytes!("../themes/neomodern-light.tmTheme"), Some(include_str!("../themes/neomodern-light.groups.json"))),
    ("papercolor-light", include_bytes!("../themes/papercolor-light.tmTheme"), Some(include_str!("../themes/papercolor-light.groups.json"))),
    ("pencil-light", include_bytes!("../themes/pencil-light.tmTheme"), Some(include_str!("../themes/pencil-light.groups.json"))),
    ("selenized-light", include_bytes!("../themes/selenized-light.tmTheme"), Some(include_str!("../themes/selenized-light.groups.json"))),
    ("seneca-light", include_bytes!("../themes/seneca-light.tmTheme"), None),
    ("soviet-light", include_bytes!("../themes/soviet-light.tmTheme"), Some(include_str!("../themes/soviet-light.groups.json"))),
    ("token-light", include_bytes!("../themes/token-light.tmTheme"), Some(include_str!("../themes/token-light.groups.json"))),
];

pub const DEFAULT: &str = "tokyonight-moon";

pub fn names() -> impl Iterator<Item = &'static str> {
    THEMES.iter().map(|(name, _, _)| *name)
}

#[cfg(test)]
pub const UNCOLOURED: &[&str] = &["seneca", "seneca-light"];

#[cfg(test)]
pub const PLAIN_OUTSIDE_CODE: &[&str] = &["lucerna"];

#[cfg(test)]
pub fn coloured_names() -> impl Iterator<Item = &'static str> {
    names().filter(|name| !UNCOLOURED.contains(name))
}

#[cfg(test)]
pub fn names_colouring_every_language() -> impl Iterator<Item = &'static str> {
    coloured_names().filter(|name| !PLAIN_OUTSIDE_CODE.contains(name))
}

pub fn user_dir() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config/merl/themes"))
}

pub fn entries_in(dir: Option<&Path>) -> Vec<String> {
    let mut names: Vec<String> = names().map(str::to_owned).collect();
    let mut user = user_names(dir);
    user.sort();
    let extra: Vec<String> = user.into_iter().filter(|n| !names.contains(n)).collect();
    names.extend(extra);
    names
}

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

fn user_path(dir: Option<&Path>, name: &str) -> Option<PathBuf> {
    dir.map(|d| d.join(format!("{name}.tmTheme")))
        .filter(|p| p.is_file())
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub gutter_fg: Color,
    pub cursor_gutter_fg: Color,
    pub own_gutter_fg: Color,
    pub line_hl: Color,
    pub line_hl_dim: Color,
    pub ghost_fg: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub find_bg: Color,
    pub find_fg: Option<Color>,
    pub current_find_bg: Color,
    pub current_find_fg: Option<Color>,
    pub accent: Color,
    pub selection: Color,
    pub selection_fg: Option<Color>,
    pub del_bg: Color,
    pub del_bg_hl: Color,
    pub del_word_bg: Color,
    pub add_bg: Color,
    pub add_bg_hl: Color,
    pub add_word_bg: Color,
    pub add_word_bg_hl: Color,
    pub word_fg: Color,
    pub tag_bg: Color,
    pub tag_fg: Color,
    pub cursor: Style,
    pub typing_cursor: Style,
    pub light: bool,
    pub syntect: syntect::highlighting::Theme,
    pub groups: HashMap<String, Style>,
    pub builtin: Option<&'static str>,
}

pub fn load(name: &str) -> Result<Theme> {
    load_from(user_dir().as_deref(), name)
}

pub fn load_configured(name: &str) -> Result<Theme> {
    load_configured_from(user_dir().as_deref(), config_path().as_deref(), name)
}

fn load_configured_from(dir: Option<&Path>, config: Option<&Path>, name: &str) -> Result<Theme> {
    let theme = load_from(dir, name);
    match config {
        Some(path) => theme.with_context(|| format!("theme \"{name}\" from {}", path.display())),
        None => theme,
    }
}

impl Theme {
    pub fn group(&self, name: &str) -> Option<Style> {
        let mut name = name;
        loop {
            if let Some(style) = self.groups.get(name) {
                return Some(*style);
            }
            name = name.rsplit_once('.')?.0;
        }
    }

    pub fn modifiers(&self, names: &[&str]) -> Modifier {
        names
            .iter()
            .find_map(|n| self.groups.get(*n))
            .map_or(Modifier::empty(), |s| s.add_modifier - Modifier::REVERSED)
    }

    pub fn gutter_bg(&self) -> Color {
        self.own_bg("LineNr")
    }

    pub fn cursor_gutter_bg(&self) -> Color {
        self.own_bg("CursorLineNr")
    }

    fn own_bg(&self, name: &str) -> Color {
        drawn(&self.groups, name, (self.fg, self.bg)).map_or(self.bg, |(_, bg)| bg)
    }

    pub fn selected(&self, style: Style) -> Style {
        self.selection_fg
            .map_or(style, |fg| style.fg(fg))
            .bg(self.selection)
    }
}

type Source = (Vec<u8>, String, HashMap<String, Style>);

fn source(dir: Option<&Path>, name: &str) -> Result<Source> {
    Ok(match user_path(dir, name) {
        Some(path) => {
            let ctx = path.display().to_string();
            let groups_path = path.with_extension("groups.json");
            let groups = match std::fs::read_to_string(&groups_path) {
                Ok(json) => {
                    parse_groups(&json).with_context(|| groups_path.display().to_string())?
                }
                Err(_) => HashMap::new(),
            };
            (
                std::fs::read(&path).with_context(|| ctx.clone())?,
                ctx,
                groups,
            )
        }
        None => {
            if let Some((_, bytes, groups)) = THEMES.iter().find(|(n, _, _)| *n == name) {
                (
                    bytes.to_vec(),
                    format!("theme `{name}`"),
                    groups.map_or_else(|| Ok(HashMap::new()), parse_groups)?,
                )
            } else {
                let all = entries_in(dir);
                bail!("unknown theme `{name}`; available: {}", all.join(", "));
            }
        }
    })
}

fn dim_line_hl(line_hl_syn: SynColor, line_hl: Color, fg: SynColor, bg: SynColor) -> Color {
    let mut line_hl_dim = blend(line_hl_syn, bg, 50);
    for percent in [7, 12, 20] {
        if line_hl_dim != rgb(bg) && line_hl_dim != line_hl {
            break;
        }
        line_hl_dim = blend(fg, bg, percent);
    }
    line_hl_dim
}

fn ghost(fg: SynColor, bg: SynColor) -> Color {
    (50..=GHOST_MAX_TEXT_PERCENT)
        .step_by(5)
        .map(|percent| blend(fg, bg, percent))
        .find(|&c| wcag_contrast(c, rgb(bg)) >= GHOST_TARGET_CONTRAST)
        .unwrap_or_else(|| blend(fg, bg, GHOST_MAX_TEXT_PERCENT))
}

fn readable_on(tints: &[Color], toward: SynColor, fg: SynColor) -> Color {
    (0..=100)
        .step_by(10)
        .map(|percent| blend(toward, fg, percent))
        .find(|&c| {
            tints
                .iter()
                .all(|&w| wcag_contrast(c, w) >= WORD_ON_TINT_CONTRAST_WCAG_AA)
        })
        .unwrap_or_else(|| rgb(toward))
}

pub fn load_from(dir: Option<&Path>, name: &str) -> Result<Theme> {
    let (bytes, ctx, groups) = source(dir, name)?;
    let syntect = ThemeSet::load_from_reader(&mut Cursor::new(&bytes[..])).with_context(|| ctx)?;

    let s = &syntect.settings;
    let bg = s.background.unwrap_or(SynColor::BLACK);
    let fg = s.foreground.unwrap_or(SynColor::WHITE);
    let over_bg = |c: SynColor| rgb(over_at_own_alpha(c, bg));
    let normal = drawn(&groups, "Normal", (rgb(fg), rgb(bg))).map_or_else(
        || (rgb(fg), rgb(bg)),
        |(f, b)| (f.unwrap_or_else(|| rgb(fg)), b),
    );
    let line_hl_syn = match drawn(&groups, "CursorLine", normal) {
        Some((_, c)) => syn(c),
        None => s
            .line_highlight
            .map_or_else(|| mix(fg, bg, 12), |c| over_at_own_alpha(c, bg)),
    };
    let line_hl = rgb(line_hl_syn);
    let line_hl_dim = dim_line_hl(line_hl_syn, line_hl, fg, bg);
    let ghost_fg = ghost(fg, bg);
    let light = wcag_luminance(rgb(bg)) > wcag_luminance(rgb(fg));
    let hue = |r, g, b| SynColor { r, g, b, a: 255 };
    let ((del, del_row, del_word), (add, add_row, add_word)) = if light {
        (
            (hue(0xf8, 0x51, 0x49), 12, 28),
            (hue(0x2f, 0xd4, 0x55), 17, 40),
        )
    } else {
        (
            (hue(0xf8, 0x51, 0x49), 10, 40),
            (hue(0x2e, 0xa0, 0x43), 15, 40),
        )
    };
    let (add_bg, add_bg_hl) = (blend(add, bg, add_row), blend(add, line_hl_syn, add_row));
    let (del_bg, del_bg_hl) = (blend(del, bg, del_row), blend(del, line_hl_syn, del_row));
    let words = [
        blend(del, bg, del_word),
        blend(add, bg, add_word),
        blend(add, line_hl_syn, add_word),
    ];
    let toward = [SynColor::WHITE, SynColor::BLACK][usize::from(light)];
    let word_fg = readable_on(&words, toward, fg);
    let own_gutter = s
        .gutter_foreground
        .map_or_else(|| mix(fg, bg, 45), |c| over_at_own_alpha(c, bg));
    let find_bg_syn = s
        .find_highlight
        .map_or_else(|| mix(fg, bg, 35), |c| over_at_own_alpha(c, bg));
    let find_bg = rgb(find_bg_syn);
    let tag_bg = blend(hue(0xd2, 0x99, 0x22), bg, if light { 45 } else { 40 });
    let tag_fg = readable_on(&[tag_bg], toward, fg);
    let selection = s.selection.map_or_else(|| blend(fg, bg, 25), over_bg);
    let find_fg = find_fg(s, find_bg, bg, fg);
    let gutter_fg = gutter_fg(own_gutter, bg, comment_color(&syntect), toward);
    let accent = rgb(accent_color(&syntect).unwrap_or(fg));
    let (cursor, typing_cursor) = cursors(&groups, normal, accent);
    let own = Theme {
        bg: rgb(bg),
        fg: rgb(fg),
        gutter_fg,
        cursor_gutter_fg: gutter_fg,
        own_gutter_fg: rgb(own_gutter),
        line_hl,
        line_hl_dim,
        ghost_fg,
        status_bg: line_hl,
        status_fg: rgb(fg),
        find_bg,
        find_fg,
        current_find_bg: blend(fg, find_bg_syn, 35),
        current_find_fg: find_fg,
        selection,
        selection_fg: s.selection_foreground.map(over_bg),
        del_bg,
        del_bg_hl,
        del_word_bg: words[0],
        add_bg,
        add_bg_hl,
        add_word_bg: words[1],
        add_word_bg_hl: words[2],
        word_fg,
        tag_bg,
        tag_fg,
        cursor,
        typing_cursor,
        accent,
        light,
        syntect,
        groups,
        builtin: names()
            .find(|n| *n == name)
            .filter(|_| user_path(dir, name).is_none()),
    };
    Ok(if own.groups.is_empty() {
        own
    } else {
        as_neovim_draws(own, normal)
    })
}

fn as_neovim_draws(mut t: Theme, normal: (Color, Color)) -> Theme {
    let nvim = |name: &str| drawn(&t.groups, name, normal);
    let fg = |name: &str| nvim(name).map(|(fg, _)| fg.unwrap_or(normal.0));
    if let Some((fg, bg)) = nvim("Visual") {
        (t.selection_fg, t.selection) = (fg, bg);
    }
    if let Some((fg, bg)) = nvim("Search") {
        (t.find_fg, t.find_bg) = (fg, bg);
    }
    if let Some((fg, bg)) = ["CurSearch", "IncSearch", "Search"]
        .into_iter()
        .find_map(nvim)
    {
        (t.current_find_fg, t.current_find_bg) = (fg, bg);
    }
    if let Some((fg, bg)) = nvim("StatusLine") {
        (t.status_fg, t.status_bg) = (fg.unwrap_or(normal.0), bg);
    }
    t.gutter_fg = fg("LineNr").unwrap_or(t.gutter_fg);
    t.cursor_gutter_fg = fg("CursorLineNr").unwrap_or(t.gutter_fg);
    t
}

fn drawn(
    groups: &HashMap<String, Style>,
    name: &str,
    (fg, bg): (Color, Color),
) -> Option<(Option<Color>, Color)> {
    let s = groups.get(name)?;
    Some(if s.add_modifier.contains(Modifier::REVERSED) {
        (Some(s.bg.unwrap_or(bg)), s.fg.unwrap_or(fg))
    } else {
        (s.fg, s.bg.unwrap_or(bg))
    })
}

fn find_fg(
    s: &syntect::highlighting::ThemeSettings,
    find_bg: Color,
    bg: SynColor,
    fg: SynColor,
) -> Option<Color> {
    match (s.find_highlight, s.find_highlight_foreground) {
        (_, Some(c)) => Some(rgb(over_at_own_alpha(c, bg))),
        (Some(_), None) => None,
        (None, None) => more_readable([rgb(fg), rgb(bg)], find_bg),
    }
}

fn more_readable(colours: [Color; 2], on: Color) -> Option<Color> {
    colours
        .into_iter()
        .max_by(|&a, &b| wcag_contrast(a, on).total_cmp(&wcag_contrast(b, on)))
}

fn apart(a: Color, b: Color) -> bool {
    let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) else {
        return true;
    };
    r1.abs_diff(r2) >= 12 || g1.abs_diff(g2) >= 12 || b1.abs_diff(b2) >= 12
}

fn cursors(
    groups: &HashMap<String, Style>,
    normal: (Color, Color),
    accent: Color,
) -> (Style, Style) {
    let readable = |bg: Color| (more_readable([normal.0, normal.1], bg), bg);
    let block = |name: &str| {
        let (fg, bg) = drawn(groups, name, normal).filter(|&(_, bg)| apart(bg, normal.1))?;
        Some(
            fg.filter(|&fg| apart(fg, bg))
                .map_or_else(|| readable(bg), |fg| (Some(fg), bg)),
        )
    };
    let cursor = block("Cursor").unwrap_or((Some(normal.1), normal.0));
    let typing = [block("lCursor"), Some(readable(accent))]
        .into_iter()
        .flatten()
        .find(|&(_, bg)| apart(bg, cursor.1))
        .unwrap_or_else(|| readable(blend(syn(cursor.1), syn(normal.1), 50)));
    let style = |(fg, bg): (Option<Color>, Color)| Style {
        fg,
        bg: Some(bg),
        sub_modifier: Modifier::REVERSED,
        ..Style::new()
    };
    (style(cursor), style(typing))
}

#[derive(Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an attribute of a highlight group in the theme file"
)]
struct Group {
    fg: Option<String>,
    bg: Option<String>,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    italic: bool,
    #[serde(default)]
    underline: bool,
    #[serde(default)]
    strikethrough: bool,
    #[serde(default)]
    reverse: bool,
}

fn parse_groups(json: &str) -> Result<HashMap<String, Style>> {
    let groups: HashMap<String, Group> = serde_json::from_str(json)?;
    let colour = |hex: Option<String>| -> Result<Option<Color>> {
        hex.map(|h| h.parse().map_err(|_| anyhow::anyhow!("not a colour: {h}")))
            .transpose()
    };
    groups
        .into_iter()
        .map(|(name, g)| {
            let mut style = Style::new();
            if let Some(fg) = colour(g.fg)? {
                style = style.fg(fg);
            }
            if let Some(bg) = colour(g.bg)? {
                style = style.bg(bg);
            }
            for (on, modifier) in [
                (g.bold, Modifier::BOLD),
                (g.italic, Modifier::ITALIC),
                (g.underline, Modifier::UNDERLINED),
                (g.strikethrough, Modifier::CROSSED_OUT),
                (g.reverse, Modifier::REVERSED),
            ] {
                if on {
                    style = style.add_modifier(modifier);
                }
            }
            Ok((name, style))
        })
        .collect()
}

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

fn comment_color(theme: &syntect::highlighting::Theme) -> Color {
    let comment = Scope::new("comment").expect("a valid scope");
    rgb(Highlighter::new(theme)
        .style_for_stack(&[comment])
        .foreground)
}

fn gutter_fg(start: SynColor, bg: SynColor, comment: Color, toward: SynColor) -> Color {
    let on_bg = |c: Color| wcag_contrast(c, rgb(bg));
    let cap = on_bg(comment);
    let mut out = rgb(start);
    if on_bg(out) >= GUTTER_CONTRAST || on_bg(out) >= cap {
        return out;
    }
    for percent in 1..=100 {
        let c = blend(toward, start, percent);
        if on_bg(c) > cap {
            break;
        }
        out = c;
        if on_bg(c) >= GUTTER_CONTRAST {
            break;
        }
    }
    out
}

pub fn style(s: syntect::highlighting::Style) -> Style {
    let mut out = Style::new().fg(rgb(s.foreground));
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

fn syn(c: Color) -> SynColor {
    let Color::Rgb(r, g, b) = c else {
        return SynColor::BLACK;
    };
    SynColor { r, g, b, a: 255 }
}

fn over_at_own_alpha(c: SynColor, bg: SynColor) -> SynColor {
    if c.a == 255 {
        return c;
    }
    mix(c, bg, u32::from(c.a) * 100 / 255)
}

fn blend(fg: SynColor, bg: SynColor, percent: u32) -> Color {
    rgb(mix(fg, bg, percent))
}

const GHOST_TARGET_CONTRAST: f64 = 4.0;
const GHOST_MAX_TEXT_PERCENT: u32 = 85;

const WORD_ON_TINT_CONTRAST_WCAG_AA: f64 = 4.5;

const GUTTER_CONTRAST: f64 = 3.0;

#[expect(
    clippy::suboptimal_flops,
    reason = "the WCAG formula term by term: mul_add rounds otherwise and could move a contrast check the theme colours are picked by"
)]
fn wcag_luminance(c: Color) -> f64 {
    let Color::Rgb(r, g, b) = c else { return 0.0 };
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn wcag_contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (wcag_luminance(a), wcag_luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn mix(fg: SynColor, bg: SynColor, percent: u32) -> SynColor {
    let m = |f: u8, b: u8| {
        u8::try_from((u32::from(f) * percent + u32::from(b) * (100 - percent)) / 100)
            .unwrap_or(u8::MAX)
    };
    SynColor {
        r: m(fg.r, bg.r),
        g: m(fg.g, bg.g),
        b: m(fg.b, bg.b),
        a: 255,
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is a setting of its own in the config file"
)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_autosave")]
    pub autosave_delay_ms: u64,
    #[serde(default = "default_true")]
    pub review_panel_colours: bool,
    #[serde(default = "default_true")]
    pub review_list_marks: bool,
    #[serde(default = "default_true")]
    pub review_open_files_first: bool,
    #[serde(default)]
    pub review_notes: bool,
    #[serde(default)]
    pub clock: crate::clock::Clock,
}

fn default_autosave() -> u64 {
    1000
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    DEFAULT.to_owned()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            autosave_delay_ms: default_autosave(),
            review_panel_colours: true,
            review_list_marks: true,
            review_open_files_first: true,
            review_notes: false,
            clock: crate::clock::Clock::default(),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".config/merl/config.toml"))
}

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

pub fn config() -> Result<Config> {
    config_from(config_path().as_deref())
}

fn config_from(path: Option<&Path>) -> Result<Config> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    match std::fs::read_to_string(path) {
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

    #[test]
    fn every_theme_draws_a_cursor_apart_from_its_background_and_a_typing_one_apart_from_both() {
        for name in names() {
            let t = load_from(None, name).unwrap();
            let (cursor, typing) = (t.cursor.bg.unwrap(), t.typing_cursor.bg.unwrap());
            assert!(apart(cursor, t.bg) && apart(cursor, t.line_hl), "{name}");
            assert!(apart(typing, t.line_hl) && apart(typing, cursor), "{name}");
        }
        let hex = |h: &str| h.parse::<Color>().unwrap();
        let github = load_from(None, "github-light").unwrap();
        assert_eq!(github.cursor.bg, Some(hex("#1f2328")), "its Cursor group");
        let rose = load_from(None, "rose-pine").unwrap();
        assert_eq!(rose.cursor.bg, Some(hex("#524f67")), "not Neovim's default");
        assert_eq!(
            (rose.typing_cursor.fg, rose.typing_cursor.bg),
            (Some(hex("#191724")), Some(hex("#e0def4"))),
            "its lCursor, apart from Cursor"
        );
        let koda = load_from(None, "koda-dark").unwrap();
        assert_eq!(
            (koda.cursor.fg, koda.cursor.bg),
            (Some(hex("#101010")), Some(hex("#b0b0b0"))),
            "a Cursor drawn #b0b0b0 on #b0b0b0 keeps its character readable"
        );
        let seneca = load_from(None, "seneca").unwrap();
        assert_eq!(
            (seneca.cursor.fg, seneca.cursor.bg),
            (Some(seneca.bg), Some(seneca.fg)),
            "no groups: Neovim's own Cursor, guifg=bg guibg=fg"
        );
    }

    #[test]
    fn every_theme_has_the_basics() {
        for name in coloured_names() {
            let t = load(name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            let s = &t.syntect.settings;
            assert!(s.background.is_some(), "{name} has no background");
            assert!(s.foreground.is_some(), "{name} has no foreground");
            assert!(s.line_highlight.is_some(), "{name} has no lineHighlight");
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
            assert!(
                colours.len() >= 3,
                "{name}: comment, keyword, string and function in under three colours: {colours:?}"
            );
        }
    }

    #[test]
    fn every_ported_theme_carries_its_neovim_groups() {
        let manifest = include_str!("../tools/theme-bench/themes.tsv");
        let skipped: Vec<&str> = manifest
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .filter(|(_, rest)| rest.starts_with("skip\t"))
            .map(|(name, _)| name)
            .collect();
        assert_ne!(skipped, [] as [&str; 0]);
        for (name, _, groups) in THEMES {
            assert_eq!(groups.is_none(), skipped.contains(name), "{name}");
            let t = load(name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            if groups.is_none() {
                assert!(t.groups.is_empty(), "{name}");
                continue;
            }
            let normal = t
                .group("Normal")
                .unwrap_or_else(|| panic!("{name}: no Normal"));
            assert_eq!((normal.fg, normal.bg), (Some(t.fg), Some(t.bg)), "{name}");
        }
    }

    #[test]
    fn the_chrome_is_what_lazyvim_draws_in_every_ported_theme() {
        let bench = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/theme-bench/colors");
        let hex = |c: Color| format!("{c}").to_lowercase();
        let mut wrong = Vec::new();
        for name in names() {
            let Ok(tsv) = std::fs::read_to_string(bench.join(name).join("chrome.tsv")) else {
                continue;
            };
            let t = load(name).unwrap();
            let lazyvim: HashMap<&str, (&str, &str)> = tsv
                .lines()
                .filter(|l| !l.starts_with('#'))
                .filter_map(|l| {
                    let mut cols = l.split('\t');
                    Some((cols.next()?, (cols.next()?, cols.next()?)))
                })
                .collect();
            for (group, merl, side) in [
                ("Normal", t.bg, 1),
                ("CursorLine", t.line_hl, 1),
                ("Visual", t.selection, 1),
                ("Search", t.find_bg, 1),
                ("CurSearch", t.current_find_bg, 1),
                ("LineNr", t.gutter_fg, 0),
                ("LineNr", t.gutter_bg(), 1),
                ("CursorLineNr", t.cursor_gutter_fg, 0),
                ("CursorLineNr", t.cursor_gutter_bg(), 1),
                ("StatusLine", t.status_bg, 1),
                ("StatusLine", t.status_fg, 0),
            ] {
                let (fg, bg) = lazyvim[group];
                let want = if side == 0 { fg } else { bg };
                if hex(merl) != want {
                    wrong.push(format!(
                        "{name} {group}: LazyVim {want}, merl {}",
                        hex(merl)
                    ));
                }
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }

    #[test]
    fn a_capture_falls_back_to_its_parent_group_as_in_neovim() {
        let t = load("kanagawa-wave").unwrap();
        let function = t.groups["@function"];
        assert!(!t.groups.contains_key("@function.method.call"));
        assert_eq!(t.group("@function.method.call.python"), Some(function));
        assert_eq!(t.group("@function"), Some(function));
        assert_eq!(t.group("NoSuchGroup.at.all"), None);
    }

    #[test]
    fn a_user_theme_brings_its_groups_beside_it() {
        let dir = std::env::temp_dir().join(format!("merl-user-groups-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let tm = std::fs::read(format!(
            "{}/themes/nord.tmTheme",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        for name in ["plain", "grouped", "broken"] {
            std::fs::write(dir.join(format!("{name}.tmTheme")), &tm).unwrap();
        }
        std::fs::write(
            dir.join("grouped.groups.json"),
            r##"{"Comment": {"fg": "#616e88", "italic": true}, "Search": {"reverse": true}}"##,
        )
        .unwrap();
        std::fs::write(dir.join("broken.groups.json"), "{").unwrap();

        assert!(load_from(Some(&dir), "plain").unwrap().groups.is_empty());
        std::fs::write(dir.join("nord.tmTheme"), &tm).unwrap();
        assert!(load_from(Some(&dir), "nord").unwrap().groups.is_empty());
        assert!(!load_from(None, "nord").unwrap().groups.is_empty());
        let t = load_from(Some(&dir), "grouped").unwrap();
        assert_eq!(
            t.group("Comment"),
            Some(
                Style::new()
                    .fg(Color::Rgb(0x61, 0x6e, 0x88))
                    .add_modifier(Modifier::ITALIC)
            )
        );
        assert_eq!(
            t.group("Search"),
            Some(Style::new().add_modifier(Modifier::REVERSED))
        );
        let e = format!("{:#}", load_from(Some(&dir), "broken").unwrap_err());
        assert!(e.contains("broken.groups.json"), "{e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_uncoloured_theme_still_sets_its_comments_apart() {
        for &name in UNCOLOURED.iter().chain(PLAIN_OUTSIDE_CODE) {
            assert!(names().any(|n| n == name), "{name} is not a theme");
            let t = load(name).unwrap();
            assert_ne!(comment_color(&t.syntect), t.fg, "{name}");
        }
    }

    #[test]
    fn every_find_match_reads_on_its_tint() {
        for name in names() {
            let t = load(name).unwrap();
            if !t.groups.is_empty() {
                continue;
            }
            let s = &t.syntect.settings;
            if s.find_highlight_foreground.is_some() {
                continue;
            }
            assert_eq!(t.find_fg.is_none(), s.find_highlight.is_some(), "{name}");
            let text = t.find_fg.unwrap_or(t.fg);
            let c = wcag_contrast(text, t.find_bg);
            assert!(c >= WORD_ON_TINT_CONTRAST_WCAG_AA, "{name}: {c:.2}:1");
        }
    }

    #[test]
    fn user_themes_are_listed_loaded_and_win_over_a_built_in() {
        let dir = std::env::temp_dir().join(format!("merl-user-themes-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let light = std::fs::read(format!(
            "{}/themes/dayfox.tmTheme",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        std::fs::write(dir.join("mine.tmTheme"), &light).unwrap();
        std::fs::write(dir.join(format!("{DEFAULT}.tmTheme")), &light).unwrap();

        assert_eq!(
            entries_in(Some(&dir.join("missing"))),
            names().collect::<Vec<_>>(),
            "a missing directory adds none"
        );
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
        assert_eq!(
            load_from(Some(&dir), DEFAULT).unwrap().bg,
            load_from(None, "dayfox").unwrap().bg,
            "shadowing a built-in loads the user's file, not the theme it covers"
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

    #[test]
    fn a_broken_configured_theme_names_the_config() {
        let dir = std::env::temp_dir().join(format!("merl-configured-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("zz-broken.tmTheme"), b"not a plist").unwrap();
        let config = dir.join("config.toml");
        std::fs::write(&config, "theme = \"zz-broken\"\n").unwrap();
        let name = toml::from_str::<Config>(&std::fs::read_to_string(&config).unwrap())
            .unwrap()
            .theme;
        let e = load_configured_from(Some(&dir), Some(&config), &name).unwrap_err();
        let e = format!("{e:#}");
        let head = format!("theme \"zz-broken\" from {}: ", config.display());
        assert!(e.starts_with(&head), "{e}");
        assert!(e.contains("zz-broken.tmTheme"), "the cause stays: {e}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
        for name in names_colouring_every_language() {
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
    fn a_missing_config_is_the_defaults_and_a_broken_one_an_error() {
        let dir = std::env::temp_dir().join(format!("merl-config-read-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        assert_eq!(config_from(Some(&path)).unwrap().theme, DEFAULT);
        assert_eq!(config_from(None).unwrap().theme, DEFAULT);
        std::fs::write(&path, "theme = ").unwrap();
        let e = format!("{:#}", config_from(Some(&path)).unwrap_err());
        assert!(e.contains("config.toml"), "{e}");
        std::fs::write(&path, "theme = \"tokyonight-moon\"\n").unwrap();
        assert!(!config_from(Some(&path)).unwrap().review_notes);
        std::fs::write(&path, "review_notes = true\n").unwrap();
        assert!(config_from(Some(&path)).unwrap().review_notes);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn light_is_a_background_lighter_than_the_text() {
        assert!(!load_from(None, "tokyonight-moon").unwrap().light);
        assert!(load_from(None, "dayfox").unwrap().light);
    }

    #[test]
    fn a_span_keeps_its_colour_and_font_but_not_its_background() {
        let red = SynColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let out = style(syntect::highlighting::Style {
            foreground: red,
            background: SynColor::WHITE,
            font_style: FontStyle::BOLD | FontStyle::ITALIC,
        });
        assert_eq!(
            out,
            Style::new()
                .fg(Color::Rgb(255, 0, 0))
                .add_modifier(Modifier::BOLD | Modifier::ITALIC)
        );
    }

    #[test]
    fn save_rewrites_only_the_theme_line() {
        let dir = std::env::temp_dir().join(format!("merl-config-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
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
    fn review_panel_colours_is_on_unless_turned_off() {
        let read = |text| toml::from_str::<Config>(text).unwrap().review_panel_colours;
        assert!(read(""));
        assert!(!read("review_panel_colours = false"));
        let c = toml::from_str::<Config>("").unwrap();
        assert!(c.review_list_marks && c.review_open_files_first);
        let c =
            toml::from_str::<Config>("review_list_marks = false\nreview_open_files_first = false")
                .unwrap();
        assert!(!c.review_list_marks && !c.review_open_files_first);
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
        let t = load("tokyonight-moon").unwrap();
        assert_eq!(t.bg, Color::Rgb(0x22, 0x24, 0x36));
        assert_ne!(t.line_hl, Color::Rgb(0, 0, 0));
        let Color::Rgb(r, g, b) = t.line_hl else {
            unreachable!()
        };
        assert!(
            r > 0x10 && g > 0x10 && b > 0x20,
            "{r:02x}{g:02x}{b:02x} is not 0x30/255 ≈ 19% black over #222436"
        );
        assert_ne!(t.gutter_fg, Color::Rgb(0x3b, 0x41, 0x5c));
    }

    #[test]
    fn accent_is_the_function_colour_and_differs_from_the_text() {
        let t = load("tokyonight-moon").unwrap();
        assert_eq!(
            t.accent,
            Color::Rgb(0x82, 0xaa, 0xff),
            "tokyonight-moon paints functions #82aaff, the blue LazyVim uses for directories"
        );
        for name in coloured_names() {
            let t = load(name).unwrap();
            assert_ne!(t.accent, t.fg, "{name}");
            assert_ne!(t.accent, t.bg, "{name}");
        }
    }

    #[test]
    fn infra_scopes_are_coloured_in_every_theme() {
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
        for name in names_colouring_every_language() {
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
    fn diff_colours_read_in_every_theme() {
        for name in names() {
            let t = load(name).unwrap();
            for (row, word, under) in [
                (t.del_bg, t.del_word_bg, t.bg),
                (t.add_bg, t.add_word_bg, t.bg),
                (t.add_bg_hl, t.add_word_bg_hl, t.line_hl),
            ] {
                assert!(apart(row, under), "{name}: row {row:?} on {under:?}");
                assert!(apart(word, row), "{name}: word {word:?} on its row {row:?}");
                let c = wcag_contrast(t.word_fg, word);
                assert!(
                    c >= WORD_ON_TINT_CONTRAST_WCAG_AA,
                    "{name}: changed text at {c:.2} on {word:?}"
                );
            }
        }
    }

    #[test]
    fn a_tag_reads_on_every_row_in_every_theme() {
        for name in names() {
            let t = load(name).unwrap();
            for under in [
                t.bg,
                t.line_hl,
                t.selection,
                t.add_bg,
                t.add_bg_hl,
                t.del_bg,
                t.add_word_bg,
                t.del_word_bg,
            ] {
                assert!(
                    apart(t.tag_bg, under),
                    "{name}: tag {:?} on {under:?}",
                    t.tag_bg
                );
            }
            let c = wcag_contrast(t.tag_fg, t.tag_bg);
            assert!(
                c >= WORD_ON_TINT_CONTRAST_WCAG_AA,
                "{name}: tag text at {c:.2}"
            );
        }
    }

    #[test]
    fn the_selection_is_the_themes_own_in_every_theme() {
        for name in names() {
            let t = load(name).unwrap();
            let (s, bg) = (&t.syntect.settings, t.syntect.settings.background.unwrap());
            let own = |c: SynColor| rgb(over_at_own_alpha(c, bg));
            assert_eq!(Some(t.selection), s.selection.map(own), "{name}");
            assert_eq!(t.selection_fg, s.selection_foreground.map(own), "{name}");
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
            let (text, ghost) = (wcag_contrast(t.fg, t.bg), wcag_contrast(t.ghost_fg, t.bg));
            assert!(
                ghost <= text * 0.9,
                "{name}: ghost {ghost:.2} vs text {text:.2} is not clearly weaker than live text"
            );
            assert!(
                ghost >= GHOST_TARGET_CONTRAST || ghost >= text * 0.7,
                "{name}: ghost {ghost:.2} vs text {text:.2}"
            );
            assert!(ghost >= 2.0, "{name}: ghost {ghost:.2}");
        }
    }

    #[test]
    fn line_numbers_reach_3_to_1_but_never_pass_the_comments() {
        let mut changed = 0;
        for name in names() {
            let t = load(name).unwrap();
            if !t.groups.is_empty() {
                continue;
            }
            let own = t.own_gutter_fg;
            let (before, after) = (wcag_contrast(own, t.bg), wcag_contrast(t.gutter_fg, t.bg));
            let comments = wcag_contrast(comment_color(&t.syntect), t.bg);
            if before >= GUTTER_CONTRAST || before >= comments {
                assert_eq!(t.gutter_fg, own, "{name}");
                continue;
            }
            changed += 1;
            assert!(
                after <= comments,
                "{name}: {after:.2} past comments {comments:.2}"
            );
            assert!(
                after >= GUTTER_CONTRAST || comments - after < 0.15,
                "{name}: {before:.2} -> {after:.2}, comments {comments:.2}"
            );
        }
        assert!(changed > 0, "{changed}");
    }
}
