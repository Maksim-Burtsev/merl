use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier};

use super::resolve;
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

struct Exception {
    theme: String,
    file: String,
    line: usize,
    cols: std::ops::RangeInclusive<usize>,
    used: bool,
}

fn exceptions(dir: &Path) -> Vec<Exception> {
    let Ok(text) = std::fs::read_to_string(dir.join("NOTES.md")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| l.strip_prefix("- `")?.split_once('`'))
        .map(|(spec, _)| {
            let (theme, at) = spec.split_once(' ').unwrap();
            let mut parts = at.split(':');
            let (file, line) = (parts.next().unwrap(), parts.next().unwrap());
            let cols = match parts.next().map(|c| c.split_once('-').unwrap_or((c, c))) {
                Some((a, b)) => a.parse().unwrap()..=b.parse().unwrap(),
                None => 1..=usize::MAX,
            };
            Exception {
                theme: theme.to_string(),
                file: file.to_string(),
                line: line.parse().unwrap(),
                cols,
                used: false,
            }
        })
        .collect()
}

fn mismatches(lang: &str) -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/colors")
        .join(lang);
    let mut excepted = exceptions(&dir);
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
            b.parse_more(None);
            b.highlight_to(usize::MAX, &theme);
            for (n, want) in lines {
                let got = drawn(&b, n - 1);
                assert_eq!(got.len(), want.len(), "{file}:{n}: characters");
                for ((col, got), want) in got.iter().zip(&want) {
                    if got == want {
                        continue;
                    }
                    if let Some(e) = excepted.iter_mut().find(|e| {
                        (e.theme == "*" || e.theme == name)
                            && e.file == file
                            && e.line == n
                            && e.cols.contains(&(col + 1))
                    }) {
                        e.used = true;
                    } else {
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
    for e in excepted.iter().filter(|e| !e.used) {
        out.push(format!(
            "NOTES.md lists `{} {}:{}` but it draws as LazyVim",
            e.theme, e.file, e.line
        ));
    }
    out
}

fn coloured_as_lazyvim(lang: &str) {
    let wrong = mismatches(lang);
    assert!(
        wrong.is_empty(),
        "{} characters differ from LazyVim:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn markdown_and_the_code_in_it_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("markdown");
}

#[test]
fn python_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("python");
}

#[test]
fn go_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("go");
}

#[test]
fn rust_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("rust");
}

#[test]
fn typescript_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("typescript");
}

#[test]
fn tsx_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("tsx");
}

#[test]
fn javascript_and_jsx_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("javascript");
}

#[test]
fn each_file_type_gets_its_grammar_and_astro_stays_on_syntect() {
    let sexp = |f: &str, src: &[u8]| {
        let mut b = Buffer::from_bytes(PathBuf::from(f), src);
        b.start_tree_sitter();
        b.parse_more(None);
        b.sexp()
    };
    let jsx = b"const a = <b />;\n";
    for f in ["a.tsx", "a.jsx", "a.js", "a.mjs", "a.cjs"] {
        assert!(sexp(f, jsx).contains("jsx_self_closing_element"), "{f}");
    }
    let ts = b"let a = <b>c;\n";
    for f in ["a.ts", "a.mts", "a.cts"] {
        assert!(sexp(f, ts).contains("type_assertion"), "{f}");
    }
    assert_eq!(sexp("a.astro", jsx), "");
}

#[test]
fn every_vendored_query_compiles_with_what_it_uses() {
    let inherited: Vec<&str> = super::QUERIES
        .iter()
        .flat_map(|(_, highlights, injections)| highlights.lines().chain(injections.lines()))
        .filter_map(|l| l.strip_prefix("; inherits:"))
        .flat_map(|bases| bases.split(','))
        .map(str::trim)
        .collect();
    for (name, _, _) in super::QUERIES {
        match super::grammar(name) {
            Some(grammar) => {
                super::compile(name, grammar).unwrap();
                assert!(super::lang(name).is_some(), "{name}");
            }
            None => assert!(inherited.contains(name), "{name}"),
        }
    }
}

#[test]
fn a_query_using_what_merl_does_not_implement_is_refused() {
    let python = || tree_sitter_python::LANGUAGE.into();
    for text in [
        "((identifier) @x (#match? @x \"<a\"))",
        "((identifier) @x (#vim-match? @x \"a{2}\"))",
        "((identifier) @x (#has-ancestor? @x call))",
        "((identifier) @x (#offset! @x 0 1))",
        "((identifier) @x (#set! injection.language \"regex\"))",
        "((identifier) @x (#set! @x priority 101))",
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
fn contains_and_has_parent_and_their_not_forms_test_as_neovim_does() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let keyword = theme.group("@keyword.python").unwrap().fg;
    let painted = |query: &str, src: &str, col: usize| {
        let lang =
            super::compile_text("python", tree_sitter_python::LANGUAGE.into(), query).unwrap();
        let lang: &'static super::Lang = Box::leak(Box::new(lang));
        let lines = vec![src.to_string()];
        let mut parsed = super::Parsed::new(lang, &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        let spans = parsed.spans(&lines, 0, 0, &mut styles);
        let fg = spans[0]
            .iter()
            .find(|(_, r)| r.contains(&col))
            .unwrap()
            .0
            .fg;
        fg == keyword
    };
    let contains = "((identifier) @keyword (#contains? @keyword \"ss\" \"er\"))";
    assert!(painted(contains, "classer = 1", 0));
    assert!(!painted(contains, "class_ = 1", 0));
    let not_contains = "((identifier) @keyword (#not-contains? @keyword \"ss\" \"er\"))";
    assert!(!painted(not_contains, "classer = 1", 0));
    assert!(painted(not_contains, "class_ = 1", 0));
    let parent = "((identifier) @keyword (#has-parent? @keyword call decorator))";
    assert!(painted(parent, "f(x)", 0));
    assert!(!painted(parent, "f(x)", 2));
    let not_parent = "((identifier) @keyword (#not-has-parent? @keyword call))";
    assert!(!painted(not_parent, "f(x)", 0));
    assert!(painted(not_parent, "f(x)", 2));
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
fn vim_patterns_match_as_neovim_reads_them_very_magic() {
    for (pattern, yes, no) in [
        ("/\\*!([a-zA-Z]+:)?re2c", "/*!local:re2c", "/*!re2"),
        ("/\\*!([a-zA-Z]+:)?re2c", "x /*!re2c", "/*!:re2c"),
        ("^a(b|c)+$", "abcb", "a(b|c)+"),
        ("\\v^\\d+$", "42", "4a"),
        ("\\<a\\>", "<a>", "a"),
    ] {
        let re = super::vim_pattern(pattern).unwrap();
        assert!(re.is_match(yes.as_bytes()), "{pattern} on {yes}");
        assert!(!re.is_match(no.as_bytes()), "{pattern} on {no}");
    }
    for pattern in ["<a>", "\\Va", "a{2}", "a\\zsb"] {
        assert!(super::vim_pattern(pattern).is_err(), "{pattern}");
    }
}

#[test]
fn a_theme_without_groups_still_colours_every_kind_of_capture() {
    let theme = theme::load("shokunin-dark").unwrap();
    assert!(theme.groups.is_empty());
    let src = "# c\ndef f(x):\n    return \"s\"\n";
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    b.start_tree_sitter();
    b.parse_more(None);
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
fn a_theme_without_groups_colours_markdown_by_its_markup_scopes() {
    for name in ["shokunin-dark", "seneca"] {
        let theme = theme::load(name).unwrap();
        assert!(theme.groups.is_empty());
        let src = "# Title\n\n**bold** `code` [l](u)\n";
        let mut b = Buffer::from_bytes(PathBuf::from("a.md"), src.as_bytes());
        b.start_tree_sitter();
        b.parse_more(None);
        b.highlight_to(usize::MAX, &theme);
        let normal = theme
            .syntect
            .settings
            .foreground
            .map(|c| Color::Rgb(c.r, c.g, c.b));
        let heading = b.hl[0].iter().find(|(_, r)| r.contains(&3)).unwrap().0;
        assert!(
            heading.fg != normal || !heading.add_modifier.is_empty(),
            "{name}"
        );
    }
}

#[test]
fn a_block_language_resolves_by_its_file_extension_as_neovim_does() {
    let name = |alias| resolve(alias).map(|l| l.name);
    assert_eq!(name("rs"), Some("rust"));
    assert_eq!(name("md"), Some("markdown"));
    assert_eq!(name("rust"), Some("rust"));
    assert_eq!(name("pandoc"), Some("markdown"));
    assert_eq!(name("astro"), None);
    assert_eq!(name("yaml"), None);
}

#[test]
fn without_the_setting_python_goes_through_syntect() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = b"import os\n";
    let mut plain = Buffer::from_bytes(PathBuf::from("a.py"), src);
    plain.highlight_to(0, &theme);
    let mut ts = Buffer::from_bytes(PathBuf::from("a.py"), src);
    ts.start_tree_sitter();
    ts.parse_more(None);
    ts.highlight_to(0, &theme);
    assert_ne!(plain.hl, ts.hl);
}

#[test]
fn a_preview_code_block_is_coloured_as_the_same_code_in_a_file() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = "import re\nWORD = re.compile(r\"[a-z]+\\d\")\n";
    let lines: Vec<String> = src.lines().map(str::to_string).collect();
    let mut file = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    file.start_tree_sitter();
    file.parse_more(None);
    file.paint_to(usize::MAX, &theme);
    for token in ["python", "py", " Python "] {
        let mut block = Buffer::block(token, lines.clone());
        let syntect = {
            block.highlight_to(usize::MAX, &theme);
            block.hl.clone()
        };
        block.start_block_tree_sitter(token);
        block.parse_more(None);
        block.paint_to(usize::MAX, &theme);
        assert_eq!(block.hl, file.hl, "{token}");
        assert_ne!(block.hl, syntect, "{token}");
    }
    let mut ruby = Buffer::block("ruby", vec!["x = 1".to_string()]);
    ruby.start_block_tree_sitter("ruby");
    ruby.highlight_to(0, &theme);
    assert!(
        !ruby.hl.is_empty(),
        "a language not on tree-sitter keeps syntect's colours"
    );
}

#[test]
fn a_higher_priority_paints_over_a_later_capture_and_equal_ones_paint_in_order() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let paint = |query: &str| {
        let lang =
            super::compile_text("python", tree_sitter_python::LANGUAGE.into(), query).unwrap();
        let lang: &'static super::Lang = Box::leak(Box::new(lang));
        let lines = vec!["x".to_string()];
        let mut parsed = super::Parsed::new(lang, &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        parsed.spans(&lines, 0, 0, &mut styles)[0][0].0
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

#[test]
fn a_parse_cut_short_resumes_and_an_edit_in_between_restarts_it() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let src = include_str!("../../tests/colors/python/classes.py").repeat(20);
    let mut b = Buffer::from_bytes(PathBuf::from("a.py"), src.as_bytes());
    b.start_tree_sitter();
    assert!(!b.parse_more(Some(std::time::Duration::ZERO)));
    b.paint_to(10, &theme);
    assert!(b.hl.is_empty(), "no colours before the first tree");
    b.lines[3].insert(0, '"');
    b.edited(3);
    let mut steps = 0;
    while !b.parse_more(Some(std::time::Duration::ZERO)) {
        steps += 1;
    }
    assert!(steps > 1, "the parse was cut short {steps} times");
    b.lines[3].remove(0);
    b.lines[40].insert(0, 'x');
    b.edited(3);
    b.parse_more(Some(std::time::Duration::ZERO));
    b.paint_to(usize::MAX, &theme);
    b.parse_more(None);
    b.paint_to(usize::MAX, &theme);
    let text = b.lines.join("\n") + "\n";
    let mut fresh = Buffer::from_bytes(PathBuf::from("a.py"), text.as_bytes());
    fresh.start_tree_sitter();
    fresh.parse_more(None);
    fresh.paint_to(usize::MAX, &theme);
    assert_eq!(b.sexp(), fresh.sexp());
    assert_eq!(b.hl, fresh.hl);
}

#[test]
#[ignore]
fn speed() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let files = std::env::var("MERL_TS_SPEED").expect("MERL_TS_SPEED=file.py,…");
    let ms = |t: std::time::Instant| t.elapsed().as_secs_f64() * 1000.0;
    let pct = |v: &[(f64, f64)], coloured: bool, p: usize| {
        let mut v: Vec<f64> = v.iter().map(|x| if coloured { x.1 } else { x.0 }).collect();
        v.sort_by(f64::total_cmp);
        v[(v.len() * p / 100).min(v.len() - 1)]
    };
    let settle = |b: &mut Buffer, last: usize| {
        while b.parsing() {
            b.parse_more(Some(crate::buffer::PARSE_IDLE_STEP));
        }
        b.highlight_to(last, &theme);
    };
    for file in files.split(',') {
        let bytes = std::fs::read(file).unwrap();
        for ts in [false, true] {
            let open = |at: usize| {
                let t = std::time::Instant::now();
                let mut b = Buffer::from_bytes(PathBuf::from(file), &bytes);
                if ts {
                    b.start_tree_sitter();
                }
                b.highlight_to(at + 54, &theme);
                let frame = ms(t);
                settle(&mut b, at + 54);
                ((frame, ms(t)), b)
            };
            let n = open(0).1.lines.len();
            let top: Vec<_> = (0..15).map(|_| open(0).0).collect();
            let middle: Vec<_> = (0..15).map(|_| open(n / 2).0).collect();
            let typing = |col0: bool| {
                let (_, mut b) = open(0);
                let mut keys = Vec::new();
                for k in 0..60 {
                    let mut l = (k * 7919) % n;
                    while col0 && !b.lines[l].starts_with([' ', '\t']) {
                        l = (l + 1) % n;
                    }
                    settle(&mut b, l + 54);
                    let t = std::time::Instant::now();
                    let indent = b.lines[l].len() - b.lines[l].trim_start().len();
                    b.lines[l].insert(if col0 { 0 } else { indent }, 'x');
                    b.edited(l);
                    b.highlight_to(l + 54, &theme);
                    let frame = ms(t);
                    settle(&mut b, l + 54);
                    keys.push((frame, ms(t)));
                }
                keys
            };
            let rows = [
                ("first paint, top", top),
                ("first paint, middle", middle),
                ("keystroke after the indentation", typing(false)),
                ("keystroke at column 0 of an indented line", typing(true)),
            ];
            for (what, v) in rows {
                println!(
                    "{file} {n} lines {}: {what}: frame p50 {:.2} p90 {:.2}, coloured p50 {:.2} p90 {:.2} ms",
                    if ts { "tree-sitter" } else { "syntect" },
                    pct(&v, false, 50),
                    pct(&v, false, 90),
                    pct(&v, true, 50),
                    pct(&v, true, 90),
                );
            }
        }
    }
}

#[test]
fn an_injected_language_without_a_grammar_stays_plain_outside_markdown() {
    let theme = theme::load("tokyonight-moon").unwrap();
    let lang = super::lang("tsx").unwrap();
    let fg_at = |src: &str, word: &str| {
        let lines = vec![src.to_string()];
        let mut parsed = super::Parsed::new(lang, &lines).unwrap();
        parsed.parse(None);
        let mut styles = super::Styles::new(&theme);
        let col = src.find(word).unwrap();
        parsed.spans(&lines, 0, 0, &mut styles)[0]
            .iter()
            .find(|(_, r)| r.contains(&col))
            .unwrap()
            .0
            .fg
    };
    assert_eq!(
        fg_at("t(`Add people to x`);", "Add"),
        fg_at("x(`Add people to x`);", "Add")
    );
}
