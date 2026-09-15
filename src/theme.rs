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
    ("kanagawa-wave", include_bytes!("../themes/kanagawa-wave.tmTheme")),
    ("kanagawa-dragon", include_bytes!("../themes/kanagawa-dragon.tmTheme")),
    ("rose-pine", include_bytes!("../themes/rose-pine.tmTheme")),
    ("rose-pine-moon", include_bytes!("../themes/rose-pine-moon.tmTheme")),
    ("everforest-dark", include_bytes!("../themes/everforest-dark.tmTheme")),
    ("gruvbox-material-dark", include_bytes!("../themes/gruvbox-material-dark.tmTheme")),
    ("catppuccin-mocha", include_bytes!("../themes/catppuccin-mocha.tmTheme")),
    ("flexoki-dark", include_bytes!("../themes/flexoki-dark.tmTheme")),
    ("melange-dark", include_bytes!("../themes/melange-dark.tmTheme")),
    ("nordfox", include_bytes!("../themes/nordfox.tmTheme")),
    ("shokunin-dark", include_bytes!("../themes/shokunin-dark.tmTheme")),
    ("rose-pine-dawn", include_bytes!("../themes/rose-pine-dawn.tmTheme")),
    ("kanagawa-lotus", include_bytes!("../themes/kanagawa-lotus.tmTheme")),
    ("everforest-light", include_bytes!("../themes/everforest-light.tmTheme")),
    ("flexoki-light", include_bytes!("../themes/flexoki-light.tmTheme")),
    ("catppuccin-latte", include_bytes!("../themes/catppuccin-latte.tmTheme")),
    ("gruvbox-material-light", include_bytes!("../themes/gruvbox-material-light.tmTheme")),
    ("melange-light", include_bytes!("../themes/melange-light.tmTheme")),
    ("dawnfox", include_bytes!("../themes/dawnfox.tmTheme")),
    ("dayfox", include_bytes!("../themes/dayfox.tmTheme")),
    ("tokyonight-day", include_bytes!("../themes/tokyonight-day.tmTheme")),
    ("bluloco-light", include_bytes!("../themes/bluloco-light.tmTheme")),
    ("shokunin-light", include_bytes!("../themes/shokunin-light.tmTheme")),
];

pub const DEFAULT: &str = "tokyonight-moon";

/// The names of [`THEMES`], in order.
pub fn names() -> impl Iterator<Item = &'static str> {
    THEMES.iter().map(|(name, _)| *name)
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
    pub status_bg: Color,
    pub status_fg: Color,
    pub find_bg: Color,
    pub find_fg: Color,
    /// The theme's signature colour, painted on the chrome the user navigates by: directory
    /// names, the tree and picker frames, the file name in the status bar. Taken from the
    /// colour the theme gives function names, so every theme has one without a new key.
    pub accent: Color,
    /// Background of the Shift+Up/Down line selection.
    #[allow(dead_code)]
    pub selection: Color,
    pub syntect: syntect::highlighting::Theme,
}

pub fn load(name: &str) -> Result<Theme> {
    let Some((_, bytes)) = THEMES.iter().find(|(n, _)| *n == name) else {
        let all: Vec<_> = names().collect();
        bail!("unknown theme `{name}`; available: {}", all.join(", "));
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
        accent: rgb(function_color(&syntect).unwrap_or(fg)),
        syntect,
    })
}

/// The foreground the theme paints `entity.name.function` with, if it has a rule for it.
fn function_color(theme: &syntect::highlighting::Theme) -> Option<SynColor> {
    let scope = Scope::new("entity.name.function").ok()?;
    let style = Highlighter::new(theme).style_for_stack(&[scope]);
    (Some(style.foreground) != theme.settings.foreground).then_some(style.foreground)
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
            assert_ne!(
                t.selection, t.line_hl,
                "{name}: the selection is the cursor line colour"
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
}
