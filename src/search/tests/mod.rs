use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

mod bindings;
mod c_family;
mod defs;
mod fields;
mod go;
mod grep;
mod imports;
mod other_languages;
mod scope;
mod symbols;
mod types;
mod words;

const PY: &str = "class Invoice:\n    def total(self):\n        return 0\n\n\ndef parse(t):\n    return Invoice()\n\n\nDEFAULT_LIMIT = 10\ntotal_foobar = 1\nprint(total_foobar, DEFAULT_LIMIT)\nNAME_RE: Final[re.Pattern[str]] = re.compile(r\"x\")\n";
const RS: &str = "pub struct Order<T> {\n    items: Vec<T>,\n}\n\nimpl<T> Order<T> {\n    pub fn sum(&self) -> u32 { 0 }\n}\n\npub(crate) const MAX_ORDERS: usize = 10;\n\npub async fn parse_order(s: &str) -> Order<u8> {\n    let mut order = Order { items: vec![] };\n    order.items.push(1);\n    order\n}\n\nmacro_rules! order {\n    () => {};\n}\n\nmod orders;\n";
const TS: &str = "export interface Order {\n  id: Id;\n}\n\nexport type Id = string;\n\nconst enum Status {\n  Open,\n}\n\nexport default class OrderService {\n  private cache = new Map();\n\n  async load(id: Id): Promise<Order> {\n    render(o);\n    return parse(id);\n  }\n\n  sum = (o: Order) => 0;\n}\n\nexport const parseOrder = (s: string): Order => JSON.parse(s);\n\nexport function render(o: Order) {\n  const n = 1;\n}\n\nfunction* ids() {}\n";
const JS: &str = "const helpers = {\n  parse(s) {\n    return s;\n  },\n  format: function (o) {\n    return o;\n  },\n};\nmodule.exports = helpers;\n";
const GO: &str = "package main\n\ntype Invoice struct{}\n\nfunc (i Invoice) Total() int { return 0 }\n\nfunc Parse(s string) Invoice { return Invoice{} }\n\nconst Limit = 10\n\nfunc main() {\n\tinv := Parse(\"x\")\n}\n";

/// A throwaway project on disk; grep needs real files.
fn scratch(tag: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<PathBuf>) {
    let dir = std::env::temp_dir().join(format!("merl-grep-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in files {
        std::fs::create_dir_all(dir.join(name).parent().unwrap()).unwrap();
        std::fs::write(dir.join(name), text).unwrap();
    }
    (dir, files.iter().map(|(n, _)| PathBuf::from(n)).collect())
}

fn project(tag: &str) -> (PathBuf, Vec<PathBuf>) {
    scratch(
        tag,
        &[
            ("a.py", PY),
            ("b.go", GO),
            ("c.rs", RS),
            ("d.ts", TS),
            ("e.js", JS),
        ],
    )
}

fn grep(dir: &Path, files: &[PathBuf], pat: &str, word: bool, smart: bool) -> Vec<Hit> {
    grep_project(dir, files, pat, word, smart, None, None).unwrap()
}

fn lines(hits: &[Hit]) -> Vec<(String, usize)> {
    hits.iter()
        .map(|h| (h.path.display().to_string(), h.line))
        .collect()
}

/// The lines `d`'s patterns match for `word` in `files`.
fn defs(dir: &Path, files: &[PathBuf], kind: Kind, word: &str) -> Vec<usize> {
    let pat = def_patterns(kind, word).join("|");
    grep(dir, files, &pat, false, false)
        .iter()
        .map(|h| h.line)
        .collect()
}

/// The lines `d`'s member patterns match for `word` in `files`: what `x.word` reaches.
fn members(dir: &Path, files: &[PathBuf], kind: Kind, word: &str) -> Vec<usize> {
    let pat = member_patterns(kind, word).unwrap().join("|");
    grep(dir, files, &pat, false, false)
        .iter()
        .map(|h| h.line)
        .collect()
}

/// The bindings of `name` on `line` of `text`, as (line, value) pairs.
fn bound_at(kind: Kind, text: &str, line: usize, name: &str) -> Vec<(usize, Value)> {
    bindings(kind, text, line, name)
        .into_iter()
        .map(|b| (b.line, b.value))
        .collect()
}

fn ty(t: &str) -> Value {
    Value::Type(t.into())
}

fn symbol(kind: Option<Kind>, line: &str) -> Option<String> {
    let (_, pattern) = SYMBOLS.iter().find(|(k, _)| *k == kind).unwrap();
    symbol_name(&Regex::new(pattern).unwrap(), line)
}

/// Every name `D` lists for `line` in a file of `kind`: each [`SYMBOLS`] row such a file is
/// read with, the all-language one included when [`shared_symbols`] takes it. A line listed
/// twice comes back twice.
fn listed(kind: Kind, line: &str) -> Vec<String> {
    SYMBOLS
        .iter()
        .filter(|(k, _)| *k == Some(kind) || (k.is_none() && shared_symbols(Some(kind))))
        .filter_map(|(_, p)| symbol_name(&Regex::new(p).unwrap(), line))
        .collect()
}

/// The one name `D` lists, and no second one.
fn one(kind: Kind, line: &str) -> Option<String> {
    let names = listed(kind, line);
    assert!(names.len() <= 1, "{line}: listed as {names:?}");
    names.into_iter().next()
}
