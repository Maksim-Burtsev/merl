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

/// The names a Ruby parameter list binds: `a, b = 1, *c, d:, e: 2, **f, &g`, a block's
/// `(y, z)` destructured and its `; x` block-locals.
fn param_names(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in list.split(';').flat_map(|l| split_top(Kind::Ruby, l, b',')) {
        let p = part.trim();
        if let Some(inner) = p.strip_prefix('(') {
            out.extend(param_names(inner.strip_suffix(')').unwrap_or(inner)));
            continue;
        }
        let p = p.trim_start_matches(['*', '&']);
        let end = p
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(p.len());
        out.push(p[..end].to_owned());
    }
    out
}

/// The 0-based lines that hold the parameters of the `def` on 0-based `d`, and whether they bind
/// `name`: `def m(a, b)` wrapped over lines or not, `def m a, b`, `def self.m(…)`.
fn def_binds(lines: &[&str], d: usize, name: &str) -> bool {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:[a-z_]+\s+)?def\s+(?:(?:self|[A-Z]\w*)\.)?(?:[A-Za-z_]\w*[?!=]?|[^\w\s(]+)",
        )
        .unwrap()
    });
    let Some(head) = HEAD.find(lines[d]) else {
        return false;
    };
    let rest = &lines[d][head.end()..];
    let list = match rest.strip_prefix('(') {
        Some(_) => match group(Kind::Ruby, lines, d, head.end()) {
            Some((inner, _, _)) => inner,
            None => return false,
        },
        // `def m a, b`: up to a `;` or a comment. `def m = …` has none.
        None if rest.starts_with([' ', '\t'])
            && !rest.trim_start().starts_with(['=', ';', '#']) =>
        {
            uncommented(Kind::Ruby, rest.split(';').next().unwrap_or(""))
        }
        None => return false,
    };
    param_names(&list).iter().any(|p| p == name)
}

/// Whether the block the line `line` opens (`do |x|`, `{ |x|`) names `name` among its parameters.
fn block_binds(line: &str, name: &str) -> bool {
    static PARAMS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\bdo|\{)\s*\|([^|]*)\|\s*(?:#.*)?$").unwrap());
    PARAMS
        .captures(line)
        .is_some_and(|c| param_names(&c[1]).iter().any(|p| p == name))
}

/// Ruby's parameters and locals (#365): what the innermost scope around 1-based `line` that binds
/// `name` binds it with — the parameters of its `def`, the parameters of its block, and what is
/// assigned to the name above the cursor directly in it (`x =`, `x ||=`, `a, x = …`,
/// `rescue => x`, `for x in`). A block sees what the scopes around it bind, up to its `def`; a
/// `def`, a `class` or a `module` sees nothing outside. Where the indentation does not tell, none:
/// the search by name decides. Numbered parameters, `it`, `define_method` and `binding` are not
/// read.
pub(super) fn ruby_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let n = regex::escape(name);
    let local = Regex::new(&format!(
        r"^\s*(?:\*?[a-z_]\w*\s*,\s*)*\*?{n}(?:\s*,\s*\*?[a-z_]\w*)*\s*(?:\|\|)?=(?:[^=~>]|$)|^\s*rescue\b[^#]*=>\s*{n}\s*(?:#.*|then\b.*)?$|^\s*for\s+(?:[a-z_]\w*\s*,\s*)*{n}\b[^#]*\bin\b"
    ))
    .expect("an escaped name keeps the pattern valid");
    let found = |line: usize| Binding {
        line: line + 1,
        value: Value::Unknown,
    };
    // The cursor on a parameter of the `def` or the block its own line opens.
    if def_binds(lines, at, name) || block_binds(lines[at], name) {
        return vec![found(at)];
    }
    let text = lines.join("\n");
    let literal = literal_lines(Kind::Ruby, &text);
    let (scopes, sure) = around(lines, &literal, at);
    if !sure {
        return Vec::new();
    }
    // The scopes the cursor is in, innermost first, and the top of the file last.
    let chain = scopes
        .iter()
        .filter(|(_, o)| *o != Opens::Nothing)
        .map(|&(i, o)| (Some(i), o))
        .chain(std::iter::once((None, Opens::Gate)));
    for (opener, opens) in chain {
        let from = opener.map_or(0, |o| o + 1);
        let mut out: Vec<Binding> = (from..=at)
            .filter(|&i| !literal[i] && local.is_match(lines[i]))
            .filter(|&i| {
                let (own, known) = around(lines, &literal, i);
                let inner = own.iter().find(|(_, o)| *o != Opens::Nothing).map(|p| p.0);
                known && inner == opener
            })
            .map(found)
            .collect();
        if let Some(o) = opener {
            let header = match opens {
                Opens::Block => block_binds(lines[o], name),
                _ => def_binds(lines, o, name),
            };
            if header {
                out.insert(0, found(o));
            }
        }
        if !out.is_empty() || opens == Opens::Gate {
            return out;
        }
    }
    Vec::new()
}

/// The path a Ruby `class` or `module` line on 1-based `line` of `text` declares, the classes and
/// modules around it included: `Shop::Basket` for `class Basket` inside `module Shop`. `None`
/// for any other line and for `class << self`.
pub fn ruby_declared_path(text: &str, line: usize) -> Option<String> {
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:class|module)\s+(?:::)?([A-Z][\w:]*)").unwrap());
    let own = NAME.captures(text.lines().nth(line.checked_sub(1)?)?)?[1].to_owned();
    let outer = ruby_class_path(text, line);
    Some(match outer.is_empty() {
        true => own,
        false => format!("{outer}::{own}"),
    })
}

/// What the Ruby class or module declared on 1-based `line` of `text` inherits, as written: its
/// superclass (`class A < B`), and the modules its body `include`s (or `prepend`s) and `extend`s
/// directly.
pub fn ruby_class_parents(text: &str, line: usize) -> (Option<String>, Vec<String>, Vec<String>) {
    static SUPER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*class\s+[\w:]+\s*<\s*(?:::)?([A-Z][\w:]*)").unwrap());
    static MIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(include|prepend|extend)\s+((?:::)?[A-Z][\w:]*(?:\s*,\s*(?:::)?[A-Z][\w:]*)*)\s*(?:#.*)?$")
            .unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(head) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return (None, Vec::new(), Vec::new());
    };
    let superclass = SUPER.captures(head).map(|c| c[1].to_owned());
    let literal = literal_lines(Kind::Ruby, text);
    let (mut includes, mut extends) = (Vec::new(), Vec::new());
    for i in line..lines.len() {
        let t = lines[i].trim();
        if !t.is_empty() && !literal[i] && indent(lines[i]) <= indent(head) {
            break;
        }
        let Some(c) = MIX.captures(lines[i]).filter(|_| !literal[i]) else {
            continue;
        };
        // Directly in the body: no `def`, block or `class << self` in between.
        let (scopes, _) = around(&lines, &literal, i);
        if scopes
            .iter()
            .find(|(_, o)| *o != Opens::Nothing)
            .map(|p| p.0)
            != Some(line - 1)
        {
            continue;
        }
        let names = c[2]
            .split(',')
            .map(|n| n.trim().trim_start_matches(':').to_owned());
        match &c[1] {
            "extend" => extends.extend(names),
            _ => includes.extend(names),
        }
    }
    (superclass, includes, extends)
}

/// Whether the Ruby declaration on 1-based `line` of `text` is on the class itself, not on its
/// instances: `def self.m`, `def Const.m`, a `scope :m` (#374), or anything directly inside
/// `class << self`.
pub fn ruby_on_class(text: &str, line: usize) -> bool {
    static ON: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:[a-z_]+\s+)?def\s+(?:self|[A-Z]\w*)\.|scope\s*\(?\s*:)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    let (scopes, _) = around(&lines, &literal, at);
    ON.is_match(lines[at])
        || scopes
            .iter()
            .find(|(_, o)| *o != Opens::Nothing)
            .is_some_and(|&(i, _)| lines[i].trim_start().starts_with("class << self"))
}

/// Whether `self` at 1-based `line` of `text` is the class rather than an instance of it: in the
/// body of a class method (see [`ruby_singleton`]), in `class << self`, or in the class body
/// itself, where `has_many :x` is a call on the class.
pub fn ruby_self_is_class(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    let (scopes, _) = around(&lines, &literal, at);
    let def = Regex::new(r"^\s*(?:[a-z_]+\s+)?def\s").expect("a valid pattern");
    match scopes.iter().find(|(_, o)| *o == Opens::Gate) {
        Some(&(g, _)) if def.is_match(lines[g]) => ruby_singleton(text, g + 1),
        Some(_) => true,
        None => false,
    }
}
