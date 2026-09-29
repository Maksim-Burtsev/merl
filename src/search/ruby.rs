//! Ruby's scopes for `d`, read by indentation as rubocop lays code out (#383).

use std::sync::LazyLock;

use regex::Regex;

use super::*;

/// What a line around another opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opens {
    /// A `def`, a `class` or a `module`: a wall no local passes.
    Gate,
    /// A `do` or a `{` block: it sees the locals around it, and its own stay inside.
    Block,
    /// An `if`, a `case`, a `begin`, a call wrapped over lines: no scope.
    Nothing,
}

/// The lines around 0-based `at` of `lines`, innermost first, each with what it opens, and
/// whether the indentation told them for certain: an `end` or a closing bracket indented less
/// than the line, or a `def` of one line, means the walk lost the nesting (it goes on past it).
fn around(lines: &[&str], literal: &[bool], at: usize) -> (Vec<(usize, Opens)>, bool) {
    static GATE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:[a-z_]+\s+)?def\s|^(?:class|module)\b").unwrap());
    // `def x = …` and `def x; …; end` hold no line below them.
    static ONE_LINE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[a-z_]+\s+)?def\s+[^\s(]+(?:\([^)]*\))?\s*=(?:[^=~>]|$)|\bend\s*(?:#.*)?$")
            .unwrap()
    });
    static BLOCK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\bdo|\{)\s*(?:\|[^|]*\|)?\s*(?:#.*)?$").unwrap());
    // The lines that go on the construct above them at its indent.
    static MIDDLE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:rescue|else|elsif|ensure|when|in|then)\b").unwrap());
    static CLOSER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:end\b|[}\])])").unwrap());
    let mut depth = indent(lines[at]) + usize::from(MIDDLE.is_match(lines[at].trim()));
    let (mut out, mut sure) = (Vec::new(), true);
    for i in (0..at).rev() {
        if depth == 0 {
            break;
        }
        let t = lines[i].trim();
        if t.is_empty() || literal[i] || t.starts_with('#') || indent(lines[i]) >= depth {
            continue;
        }
        if MIDDLE.is_match(t) {
            continue;
        }
        depth = indent(lines[i]);
        if CLOSER.is_match(t) {
            sure = false;
            continue;
        }
        let opens = match GATE.is_match(t) {
            true if ONE_LINE.is_match(t) => {
                sure = false;
                continue;
            }
            true => Opens::Gate,
            false if BLOCK.is_match(t) => Opens::Block,
            false => Opens::Nothing,
        };
        out.push((i, opens));
    }
    (out, sure)
}

/// The 1-based lines of `text` that assign the Ruby local `name` where 1-based `line` sees it
/// (#383): a local belongs to its `def`, and to the block it is assigned in, so it is seen from
/// that block or `def` and not across a `def`, a `class` or a `module`. Outside any of them it
/// belongs to the top of the file. Where the indentation does not tell, every assignment of the
/// file counts.
pub fn ruby_locals(text: &str, line: usize, name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let literal = literal_lines(Kind::Ruby, text);
    let assign = Regex::new(&ruby_assignment(name)).expect("an escaped name keeps it valid");
    let gate = |a: &[(usize, Opens)]| a.iter().find(|(_, o)| *o == Opens::Gate).map(|p| p.0);
    let (cursor, sure) = around(&lines, &literal, at);
    (0..lines.len())
        .filter(|&i| !literal[i] && assign.is_match(lines[i]))
        .filter(|&i| {
            let (scopes, known) = around(&lines, &literal, i);
            let inner = scopes.iter().find(|(_, o)| *o != Opens::Nothing);
            !(sure && known)
                || (gate(&cursor) == gate(&scopes) && inner.is_none_or(|s| cursor.contains(s)))
        })
        .map(|i| i + 1)
        .collect()
}

/// The class or module 1-based `line` of `text` is written in, as its path: `Shop::Basket` for
/// `module Shop` around `class Basket`, and for `class Shop::Basket`. Empty at the top of the
/// file. A `class << self` is the class around it.
pub fn ruby_class_path(text: &str, line: usize) -> String {
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:class|module)\s+((?:::)?[A-Z][\w:]*)").unwrap());
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return String::new();
    };
    let literal = literal_lines(Kind::Ruby, text);
    let (scopes, _) = around(&lines, &literal, at);
    let mut names: Vec<&str> = scopes
        .iter()
        .filter_map(|&(i, _)| NAME.captures(lines[i]))
        .map(|c| c.get(1).map_or("", |m| m.as_str().trim_start_matches(':')))
        .collect();
    names.reverse();
    names.join("::")
}
