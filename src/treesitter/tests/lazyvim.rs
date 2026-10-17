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
                palette.insert(letter.chars().next().unwrap(), colour.to_owned());
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
            out.insert(file.to_owned(), lines);
        }
    }
    out
}

fn drawn(b: &Buffer, line: usize, plain: [Color; 2]) -> Vec<(usize, String)> {
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
            let bg = match style.bg.filter(|c| !plain.contains(c)) {
                Some(Color::Rgb(r, g, b)) => format!("/{r:02x}{g:02x}{b:02x}"),
                _ => String::new(),
            };
            let flags = format!(
                "{}{}{bg}",
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
    let (before, after) = (line.get(..col).unwrap(), line.get(col..).unwrap());
    let start = before.rfind(|c| !word(c)).map_or(0, |i| i + 1);
    let end = after.find(|c| !word(c)).map_or(line.len(), |i| col + i);
    if start < end {
        line.get(start..end).unwrap()
    } else {
        line.get(col..=col).unwrap()
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
                theme: theme.to_owned(),
                file: file.to_owned(),
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
            b.parse_more(None);
            b.highlight_to(usize::MAX, &theme);
            for (n, want) in lines {
                let got = drawn(&b, n - 1, [theme.bg, theme.line_hl]);
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
fn bash_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("bash");
}

#[test]
fn json_and_jsonc_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("json");
}

#[test]
fn yaml_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("yaml");
}

#[test]
fn toml_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("toml");
}

#[test]
fn lua_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("lua");
}

#[test]
fn c_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("c");
}

#[test]
fn cpp_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("cpp");
}

#[test]
fn java_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("java");
}

#[test]
fn c_sharp_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("c_sharp");
}

#[test]
fn sql_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("sql");
}

#[test]
fn cmake_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("cmake");
}

#[test]
fn hcl_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("hcl");
}

#[test]
fn terraform_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("terraform");
}

#[test]
fn ini_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("ini");
}

#[test]
fn graphql_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("graphql");
}

#[test]
fn nginx_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("nginx");
}

#[test]
fn gdscript_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("gdscript");
}

#[test]
fn starlark_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("starlark");
}

#[test]
fn solidity_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("solidity");
}

#[test]
fn proto_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("proto");
}

#[test]
fn dockerfile_and_the_shell_in_it_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("dockerfile");
}

#[test]
fn kotlin_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("kotlin");
}

#[test]
fn scala_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("scala");
}

#[test]
fn swift_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("swift");
}

#[test]
fn zig_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("zig");
}

#[test]
fn nix_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("nix");
}

#[test]
fn scheme_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("scheme");
}

#[test]
fn r_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("r");
}

#[test]
fn erlang_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("erlang");
}

#[test]
fn racket_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("racket");
}

#[test]
fn common_lisp_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("commonlisp");
}

#[test]
fn ruby_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("ruby");
}

#[test]
fn php_and_the_html_around_it_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("php");
}

#[test]
fn elixir_and_the_markdown_in_its_docs_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("elixir");
}

#[test]
fn html_and_the_css_and_javascript_in_it_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("html");
}

#[test]
fn vue_and_the_typescript_and_css_in_it_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("vue");
}

#[test]
fn svelte_and_the_typescript_and_css_in_it_are_coloured_as_lazyvim() {
    coloured_as_lazyvim("svelte");
}

#[test]
fn css_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("css");
}

#[test]
fn xml_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("xml");
}

#[test]
fn diff_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("diff");
}

#[test]
fn query_is_coloured_as_lazyvim() {
    coloured_as_lazyvim("query");
}
