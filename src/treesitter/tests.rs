use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};

use crate::buffer::Buffer;
use crate::theme;

fn decode(runs: &str, palette: &HashMap<char, String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = runs.chars().peekable();
    while let Some(letter) = chars.next() {
        let mut n = String::new();
        while let Some(d) = chars.next_if(char::is_ascii_digit) {
            n.push(d);
        }
        let n = n.parse().unwrap_or(1);
        out.extend(std::iter::repeat_n(palette[&letter].clone(), n));
    }
    out
}

fn recorded(path: &Path) -> HashMap<String, Vec<(usize, Vec<String>)>> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut palette = HashMap::new();
    let mut out = HashMap::new();
    for row in text.lines() {
        if let Some(p) = row.strip_prefix("# palette ") {
            for pair in p.split(' ') {
                let (letter, colour) = pair.split_once('=').unwrap();
                palette.insert(letter.chars().next().unwrap(), colour.to_string());
            }
        } else if !row.starts_with('#') && !row.is_empty() {
            let (file, cells) = row.split_once('\t').unwrap();
            let lines = cells
                .split(' ')
                .map(|c| {
                    let (n, runs) = c.split_once(':').unwrap();
                    (n.parse().unwrap(), decode(runs, &palette))
                })
                .collect();
            out.insert(file.to_string(), lines);
        }
    }
    out
}

fn drawn(b: &Buffer, line: usize) -> Vec<(usize, String)> {
    let text = &b.lines[line];
    text.char_indices()
        .filter(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| {
            let (style, _) = b.hl[line]
                .iter()
                .find(|(_, r)| r.contains(&i))
                .expect("every character is in a span");
            let Some(Color::Rgb(r, g, bl)) = style.fg else {
                panic!("line {line} col {i}: no colour")
            };
            let m = style.add_modifier;
            let flags = format!(
                "{}{}",
                if m.contains(Modifier::BOLD) { "b" } else { "" },
                if m.contains(Modifier::ITALIC) {
                    "i"
                } else {
                    ""
                },
            );
            (i, format!("{r:02x}{g:02x}{bl:02x}{flags}"))
        })
        .collect()
}

fn token(line: &str, col: usize) -> &str {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let start = line[..col].rfind(|c| !word(c)).map_or(0, |i| i + 1);
    let end = line[col..]
        .find(|c| !word(c))
        .map_or(line.len(), |i| col + i);
    if start < end {
        &line[start..end]
    } else {
        &line[col..col + 1]
    }
}

fn mismatches(lang: &str) -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/colors")
        .join(lang);
    let mut tables: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "tsv"))
        .collect();
    tables.sort();
    assert!(
        tables.len() >= 4,
        "{}: the four themes' recordings",
        dir.display()
    );
    let mut out = Vec::new();
    for table in tables {
        let name = table.file_stem().unwrap().to_str().unwrap();
        let theme = theme::load(name).unwrap();
        for (file, lines) in recorded(&table) {
            let path = dir.join(&file);
            let mut b = Buffer::from_bytes(path.clone(), &std::fs::read(&path).unwrap());
            b.start_tree_sitter();
            b.highlight_to(usize::MAX, &theme);
            for (n, want) in lines {
                let got = drawn(&b, n - 1);
                assert_eq!(got.len(), want.len(), "{file}:{n}: characters");
                for ((col, got), want) in got.iter().zip(&want) {
                    if got != want {
                        let tok = token(&b.lines[n - 1], *col);
                        out.push(format!(
                            "{name} {file}:{n}:{} `{tok}` want {want} got {got}",
                            col + 1
                        ));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn python_is_coloured_as_lazyvim() {
    let wrong = mismatches("python");
    assert!(
        wrong.is_empty(),
        "{} characters differ from LazyVim:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn every_vendored_query_compiles_with_what_it_uses() {
    for (name, _) in super::QUERIES {
        assert!(super::lang(name).is_some(), "{name}");
    }
    super::compile("python", tree_sitter_python::LANGUAGE.into()).unwrap();
}

#[test]
fn a_query_using_what_merl_does_not_implement_is_refused() {
    let python = || tree_sitter_python::LANGUAGE.into();
    for text in [
        "((identifier) @x (#match? @x \"^a\"))",
        "((identifier) @x (#vim-match? @x \"^a\"))",
        "((identifier) @x (#has-ancestor? @x call))",
        "((identifier) @x (#offset! @x 0 1 0 -1))",
        "((identifier) @x (#set! conceal \"\"))",
        "((identifier) @x (#lua-match? @x \"%bxy\"))",
    ] {
        let got = super::compile_text("python", python(), text);
        assert!(got.is_err(), "{text} compiled");
    }
    super::compile_text(
        "python",
        python(),
        "((identifier) @x (#lua-match? @x \"^%u\"))",
    )
    .unwrap();
}

#[test]
fn lua_patterns_match_as_lua_does() {
    for (pattern, yes, no) in [
        ("^[A-Z].*[a-z]", "Order", "MAX"),
        ("^[A-Z][A-Z_0-9]*$", "MAX_2", "Max"),
        ("^__[a-zA-Z0-9_]*__$", "__init__", "_x_"),
        ("^#!/", "#!/usr/bin/env", "# !/"),
        ("^[%l_].*$", "_private", "Public"),
        ("^%u", "Point", "point"),
        ("a%.b-c", "xa.c", "axbc"),
        ("^[^%d]+$", "abc", "a1"),
    ] {
        let re = super::lua_pattern(pattern).unwrap();
        assert!(re.is_match(yes.as_bytes()), "{pattern} on {yes}");
        assert!(!re.is_match(no.as_bytes()), "{pattern} on {no}");
    }
}

#[test]
fn a_theme_without_groups_still_colours_every_kind_of_capture() {
    let theme = theme::load("shokunin-dark").unwrap();
    assert!(theme.groups.is_empty());
    let src = "# c\ndef f(x):\n    return \"s\"\n";
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    b.start_tree_sitter();
    b.highlight_to(usize::MAX, &theme);
    let fg = |l: usize, col: usize| b.hl[l].iter().find(|(_, r)| r.contains(&col)).unwrap().0.fg;
    let (comment, keyword, function, string) = (fg(0, 0), fg(1, 0), fg(1, 4), fg(2, 11));
    assert_ne!(comment, keyword);
    assert_ne!(keyword, string);
    assert_ne!(
        function,
        theme
            .syntect
            .settings
            .foreground
            .map(|c| Color::Rgb(c.r, c.g, c.b))
    );
}

#[test]
fn without_the_setting_python_goes_through_syntect() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = b"import os\n";
    let mut plain = Buffer::from_bytes(PathBuf::from("a.py"), src);
    plain.highlight_to(0, &theme);
    let mut ts = Buffer::from_bytes(PathBuf::from("a.py"), src);
    ts.start_tree_sitter();
    ts.highlight_to(0, &theme);
    assert_ne!(plain.hl, ts.hl);
}

#[test]
fn a_higher_priority_paints_over_a_later_capture_and_equal_ones_paint_in_order() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let paint = |query: &str| {
        let lang =
            super::compile_text("python", tree_sitter_python::LANGUAGE.into(), query).unwrap();
        let lang: &'static super::Lang = Box::leak(Box::new(lang));
        let lines = vec!["x".to_string()];
        let parsed = super::Parsed::new(lang, &lines).unwrap();
        let styles = super::Styles::new(lang, &theme);
        parsed.spans(&lines, 0, 0, &styles)[0][0].0
    };
    let keyword = theme.group("@keyword.python").unwrap().fg;
    let string = theme.group("@string.python").unwrap().fg;
    let raised = "((identifier) @keyword (#set! priority 101))\n(identifier) @string";
    assert_eq!(paint(raised).fg, keyword);
    assert_eq!(
        paint("(identifier) @keyword\n(identifier) @string").fg,
        string
    );
    assert_eq!(
        paint("(identifier) @string\n(identifier) @keyword").fg,
        keyword
    );
}
