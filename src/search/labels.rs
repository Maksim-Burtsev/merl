//! A word in a label position (#315): a named argument, a key of an object literal, a JSX
//! attribute. It names a parameter of the callee or a field of the literal's type, never whatever
//! else happens to be spelled so, and that parameter or field is where `d` lands (#316).

use std::ops::Range;

use regex::Regex;

use super::*;

/// What a label is a name of: the name at a position, which `d` resolves as it would there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// `f(word: …)`, `Type(word = …)`: a parameter of the function, or of the type's
    /// constructors.
    Args,
    /// `f(a, { word: … })`, `<Tag word={…}>`: a key of the object passed as argument `n`.
    Object(usize),
    /// `const x: T = { word: … }`: a field of `T`.
    Typed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// What the status line calls the word: `argument label` or `key`.
    pub what: &'static str,
    /// The 0-based line and the byte column of the name the label belongs to, and how; `None`
    /// when nothing in front of the bracket names it.
    pub owner: Option<(usize, usize, Owner)>,
}

/// A line declaring a type, with the name it gives it.
static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:[\w@]+\s+)*?(?:class|struct|record|enum|actor|object|interface)\s+([A-Za-z_$][\w$]*)")
        .unwrap()
});

/// How far back the bracket around a label is looked for.
// ponytail: 80 lines; a call or a literal written over more is read as no label.
const REACH: usize = 80;

/// Whether the word at `range` of 0-based `line` of `text` stands in a label position, and what
/// it is a label of:
/// - Python and Kotlin `f(name = …)`, Swift, C#, PHP and Ruby `f(name: …)`: behind the `(` or a
///   `,` of a call, whose callee is no declaration (`def`, `func`, `init`, `case`…) and no control
///   keyword (`for (…)`, `return (…)`); in a `java` file only an annotation's `@A(name = …)`, as
///   Java has no named arguments and `for (i = 0; …)` assigns;
/// - TypeScript and JavaScript: a key `name:` behind the `{` or a `,` of an object literal (not a
///   type's, a block's or a class body's `{`), and `name=` inside a JSX tag;
/// - Ruby `{ name: … }`.
pub fn label_at(
    kind: Kind,
    java: bool,
    text: &str,
    line: usize,
    range: Range<usize>,
) -> Option<Label> {
    let lines: Vec<&str> = text.lines().collect();
    let l = *lines.get(line)?;
    let tight = &l[range.end..];
    let after = match kind {
        // `a ? b : c` spaces its `:`; a Ruby label never does.
        Kind::Ruby => tight,
        _ => tight.trim_start(),
    };
    let sep = match kind {
        Kind::Python | Kind::Jvm => b'=',
        Kind::Swift | Kind::CSharp | Kind::Php | Kind::Ruby | Kind::Rust => b':',
        Kind::TsJs if after.starts_with('=') => b'=',
        Kind::TsJs => b':',
        _ => return None,
    };
    let rest = after.as_bytes();
    if rest.first() != Some(&sep) || matches!(rest.get(1), Some(b'=' | b'>' | b':')) {
        return None;
    }
    let literal = literal_lines(kind, text);
    if kind == Kind::TsJs && sep == b'=' {
        return jsx(kind, &lines, &literal, line, range.start);
    }
    let (at, open, bracket, prev, _) = open_bracket(kind, &lines, &literal, line, range.start)?;
    if prev != Some(bracket) && prev != Some(b',') {
        return None;
    }
    let key = |owner| Some(Label { what: "key", owner });
    match (kind, bracket) {
        (Kind::TsJs, b'{') => {
            let (bl, bi, b) = back(kind, &lines, &literal, at, open).next()?;
            let before = &lines[bl][..bi];
            match b {
                b'(' | b',' => {
                    let (cl, ci, c, _, n) = open_bracket(kind, &lines, &literal, at, open)?;
                    let owner = (c == b'(' && !declares(&lines[cl][..ci]))
                        .then(|| callee(kind, lines[cl], ci))
                        .flatten()
                        .map(|col| (cl, col, Owner::Object(n)));
                    key(owner)
                }
                b'=' if !before.ends_with([
                    '=', '!', '<', '>', '+', '-', '*', '/', '%', '&', '|', '^', '?',
                ]) =>
                {
                    static ALIAS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                        Regex::new(r"^\s*(?:export\s+)?(?:declare\s+)?type\s").unwrap()
                    });
                    static TYPED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                        Regex::new(r"[\w$]\s*[?!]?\s*:\s*([A-Za-z_$][\w$.]*)\s*(?:<[^=]*>)?\s*$")
                            .unwrap()
                    });
                    if ALIAS.is_match(before) {
                        return None;
                    }
                    let owner = TYPED
                        .captures(before)
                        .and_then(|c| c.get(1))
                        .map(|m| (bl, m.end() - 1, Owner::Typed));
                    key(owner)
                }
                b'[' | b'?' => key(None),
                b'&' | b'|' if before.ends_with(b as char) => key(None),
                _ if ["return", "yield"].iter().any(|k| {
                    lines[bl][..=bi]
                        .strip_suffix(k)
                        .is_some_and(|s| !s.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
                }) =>
                {
                    key(None)
                }
                _ => None,
            }
        }
        (Kind::Rust, b'{') => {
            static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?:^|[^\w:])(?:\w+::)*([A-Z]\w*|Self)\s*$").unwrap()
            });
            static DECLARES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"\b(?:struct|enum|union|impl|trait)\b").unwrap()
            });
            let before = &lines[at][..open];
            let ty = LITERAL.captures(before)?.get(1)?;
            if DECLARES.is_match(&before[..ty.start()]) {
                return None;
            }
            let mut up = open_bracket(kind, &lines, &literal, at, ty.start());
            if let Some((el, ei, b'{', ..)) = up
                && lines[el][..ei].contains("enum ")
            {
                return None;
            }
            if ty.as_str() != "Self" {
                return key(Some((at, ty.end() - 1, Owner::Typed)));
            }
            // `Self { … }`: the type of the `impl` around it.
            static IMPL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"^\s*(?:unsafe\s+)?impl\b.*?(?:\bfor\s+)?(?:\w+::)*(\w+)\s*(?:<[^{]*>)?\s*(?:where\b.*)?$")
                    .unwrap()
            });
            while let Some((el, ei, c, ..)) = up {
                if c == b'{'
                    && let Some(m) = IMPL.captures(&lines[el][..ei]).and_then(|c| c.get(1))
                {
                    return key(Some((el, m.end() - 1, Owner::Typed)));
                }
                up = open_bracket(kind, &lines, &literal, el, ei);
            }
            key(None)
        }
        (Kind::Ruby, b'{') => {
            // `each { |x| … }` and `f(x) { … }` open a block; a line of its own a hash.
            let block = back(kind, &lines, &literal, at, open)
                .next()
                .is_some_and(|(l, _, b)| {
                    l == at && (b == b')' || b.is_ascii_alphanumeric() || b == b'_')
                });
            (!block).then_some(Label {
                what: "key",
                owner: None,
            })
        }
        (Kind::Python | Kind::Jvm | Kind::Swift | Kind::CSharp | Kind::Php | Kind::Ruby, b'(') => {
            let before = &lines[at][..open];
            if declares(before) {
                return None;
            }
            static ANNOTATION: std::sync::LazyLock<Regex> =
                std::sync::LazyLock::new(|| Regex::new(r"@[\w.]+\s*$").unwrap());
            if java && !ANNOTATION.is_match(before) {
                return None;
            }
            let col = callee(kind, lines[at], open)?;
            // `Type.init(…)` is a call of `Type`'s initializer; `self.init`, `super.init` and a
            // bare `.init` name no type the line spells.
            let owner = match kind == Kind::Swift
                && word_at(lines[at], col, "").map(|w| w.1) == Some("init")
            {
                true => {
                    let head = lines[at][..col + 1 - "init".len()].strip_suffix('.');
                    head.and_then(|h| {
                        let name = h
                            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                            .next()?;
                        (name.starts_with(|c: char| c.is_ascii_uppercase()))
                            .then(|| (at, h.len() - 1, Owner::Args))
                    })
                }
                false => Some((at, col, Owner::Args)),
            };
            Some(Label {
                what: "argument label",
                owner,
            })
        }
        _ => None,
    }
}

/// Whether the text in front of a `(` declares what the brackets hold: a function, an
/// initializer, a class's bases, an enum case, a lambda.
fn declares(before: &str) -> bool {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.$])(?:def|fun|func|function|fn|class|case|subscript|constructor|init|lambda)\b[^=(]*$")
            .unwrap()
    });
    DECL.is_match(before)
}

/// The byte column of the last character of the name right in front of byte `open` of `line`,
/// past type arguments `<…>`: `f` of `f(`, `T` of `new T<K>(`, `snapshot!` of `snapshot!(`.
fn callee(kind: Kind, line: &str, open: usize) -> Option<usize> {
    let mut s = line[..open].trim_end();
    if s.ends_with('>') && matches!(kind, Kind::TsJs | Kind::Swift | Kind::CSharp | Kind::Jvm) {
        let mut depth = 0i32;
        let lt = s.char_indices().rev().find(|&(_, c)| {
            depth += match c {
                '>' => 1,
                '<' => -1,
                _ => 0,
            };
            depth == 0
        })?;
        s = s[..lt.0].trim_end();
    }
    // `for (…)`, `return (…)`: a statement's brackets, or a tuple's, are no call.
    const KEYWORDS: &[&str] = &[
        "if",
        "elif",
        "for",
        "foreach",
        "while",
        "until",
        "unless",
        "switch",
        "when",
        "match",
        "catch",
        "synchronized",
        "using",
        "lock",
        "return",
        "yield",
        "throw",
        "await",
        "not",
        "and",
        "or",
        "in",
    ];
    if let Some(w) = s
        .rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .next()
        && KEYWORDS.contains(&w)
        && !s[..s.len() - w.len()].ends_with('.')
    {
        return None;
    }
    let last = s.chars().next_back()?;
    let named = last.is_alphanumeric() || last == '_' || last == '$';
    // On the name itself, which takes a Ruby `!` or `?` with it.
    let ruby = kind == Kind::Ruby && matches!(last, '!' | '?') && s.len() > 1;
    match (named, ruby) {
        (true, _) => Some(s.len() - last.len_utf8()),
        (false, true) => Some(s.len() - 2),
        _ => None,
    }
}

/// The code bytes that are no whitespace of a line, up to its comment.
fn events(kind: Kind, line: &str) -> Vec<(usize, u8)> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    for (i, c) in code(kind, line) {
        let hash =
            c == b'#' && (kind == Kind::Ruby || (kind == Kind::Php && b.get(i + 1) != Some(&b'[')));
        if c == 0 || hash {
            break;
        }
        if !c.is_ascii_whitespace() {
            out.push((i, c));
        }
    }
    out
}

/// The code bytes in front of byte `end` of 0-based line `at`, nearest first, as `(line, byte
/// index, byte)`, over the lines above as far as [`REACH`]; a line inside a literal has none.
fn back<'a>(
    kind: Kind,
    lines: &'a [&'a str],
    literal: &'a [bool],
    at: usize,
    end: usize,
) -> impl Iterator<Item = (usize, usize, u8)> + 'a {
    (at.saturating_sub(REACH)..=at).rev().flat_map(move |li| {
        let ev = match literal.get(li) {
            Some(true) => Vec::new(),
            _ => events(kind, lines[li]),
        };
        ev.into_iter()
            .filter(move |&(i, _)| li != at || i < end)
            .rev()
            .map(move |(i, c)| (li, i, c))
    })
}

/// The bracket still open in front of byte `end` of line `at`: its line, byte and character, the
/// first code byte met on the way back, and the commas at the bracket's own level. A `;` at that
/// level ends the statement first.
fn open_bracket(
    kind: Kind,
    lines: &[&str],
    literal: &[bool],
    at: usize,
    end: usize,
) -> Option<(usize, usize, u8, Option<u8>, usize)> {
    let (mut depth, mut prev, mut commas) = (0usize, None, 0);
    for (li, i, c) in back(kind, lines, literal, at, end) {
        prev.get_or_insert(c);
        match c {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' if depth > 0 => depth -= 1,
            b'(' | b'[' | b'{' => return Some((li, i, c, prev, commas)),
            b',' if depth == 0 => commas += 1,
            b';' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// `name=` inside a JSX tag `<Tag … name={…}>`: the tag is the owner when it is a component, a
/// capitalised name; an element's attribute (`<input value=…>`) has none.
fn jsx(kind: Kind, lines: &[&str], literal: &[bool], at: usize, start: usize) -> Option<Label> {
    if !lines[at][..start].is_empty() && !lines[at][..start].ends_with(char::is_whitespace) {
        return None;
    }
    let mut depth = 0usize;
    for (li, i, c) in back(kind, lines, literal, at, start) {
        match c {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            b'<' => {
                let l = lines[li];
                let name: String = l[i + 1..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'))
                    .collect();
                let glued = l[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_');
                if name.is_empty() || glued || !name.starts_with(|c: char| c.is_alphabetic()) {
                    return None;
                }
                let last = name.rsplit('.').next().unwrap_or(&name);
                let owner = last
                    .starts_with(|c: char| c.is_ascii_uppercase())
                    .then(|| (li, i + name.len(), Owner::Object(0)));
                return Some(Label {
                    what: "argument label",
                    owner,
                });
            }
            c if c.is_ascii_alphanumeric()
                || matches!(c, b'_' | b'$' | b'-' | b'=' | b'.' | b':' | b'/') => {}
            _ => return None,
        }
    }
    None
}

/// The 1-based lines of what the declaration on 1-based `decl` of `text` names `word` as, when
/// `name` there is the owner of a label: a parameter of the function, or of the constructors of
/// the type ([`Owner::Args`]); a key of the object parameter `n` ([`Owner::Object`]); a field of
/// the type ([`Owner::Typed`]).
pub fn label_lines(
    kind: Kind,
    text: &str,
    decl: usize,
    name: &str,
    word: &str,
    owner: &Owner,
) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    // The type's own name: an alias or an import may call it otherwise.
    let ty = TYPE.captures(lines[k]).map(|c| c[1].to_owned());
    match (owner, ty.as_deref()) {
        (Owner::Args, Some(name)) => {
            let n = regex::escape(name);
            // Kotlin's and C#'s primary constructor is on the class's own line; Python's
            // brackets there hold its bases.
            let primary = matches!(kind, Kind::Jvm | Kind::CSharp)
                .then(|| params(kind, &lines, k, name, |p| param_named(kind, p, word)))
                .flatten();
            if let Some(line) = primary {
                return vec![line];
            }
            let ctor = match kind {
                Kind::Python => r"^\s*def\s+__init__\s*\(".to_owned(),
                Kind::Swift => r"^\s*(?:(?:@\w+|public|private|internal|fileprivate|open|convenience|required|override|package)\s+)*init[?!]?\s*[(<]".to_owned(),
                Kind::Jvm => r"^\s*(?:(?:public|private|protected|internal)\s+)*constructor\s*\(".to_owned(),
                Kind::CSharp => format!(r"^\s*(?:(?:public|private|protected|internal)\s+)*{n}\s*\("),
                Kind::Php => r"\bfunction\s+__construct\s*\(".to_owned(),
                Kind::Ruby => r"^\s*def\s+initialize\b".to_owned(),
                _ => return Vec::new(),
            };
            let ctor = Regex::new(&ctor).expect("an escaped name keeps the pattern valid");
            let members = members(&lines, k);
            let ctors: Vec<usize> = members
                .iter()
                .copied()
                .filter(|&i| ctor.is_match(lines[i]))
                .collect();
            let mut out: Vec<usize> = ctors
                .iter()
                .filter_map(|&i| {
                    let head = if kind == Kind::Ruby { "initialize" } else { "" };
                    params(kind, &lines, i, head, |p| param_named(kind, p, word))
                })
                .collect();
            // With no initializer of its own, a Swift struct or a dataclass is built from its
            // stored properties.
            if ctors.is_empty() && matches!(kind, Kind::Swift | Kind::Python) {
                let w = regex::escape(word);
                let stored = Regex::new(&match kind {
                    Kind::Swift => format!(r"^\s*(?:(?:@\w+|public|private|internal|fileprivate|package)\s+)*(?:let|var)\s+{w}\b"),
                    _ => format!(r"^\s*{w}\s*:"),
                })
                .expect("an escaped name keeps the pattern valid");
                out.extend(
                    members
                        .iter()
                        .filter(|&&i| stored.is_match(lines[i]))
                        .map(|&i| i + 1),
                );
            }
            out
        }
        (Owner::Args, None) => params(kind, &lines, k, name, |p| param_named(kind, p, word))
            .into_iter()
            .collect(),
        (Owner::Object(n), _) => {
            let key = Regex::new(&format!(
                r"(?:^|[{{,;])\s*(?:readonly\s+)?{}\s*\??\s*[:,}};=]",
                regex::escape(word)
            ))
            .expect("an escaped name keeps the pattern valid");
            let mut seen = 0;
            params(kind, &lines, k, name, |p| {
                seen += 1;
                (seen == n + 1)
                    .then(|| key.find(p).map(|m| m.end() - 1))
                    .flatten()
            })
            .into_iter()
            .collect()
        }
        (Owner::Typed, _) => {
            let key = Regex::new(&format!(
                r"^\s*(?:(?:readonly|public|private|protected|static|declare|pub(?:\([^)]*\))?)\s+)*{}\s*[?!]?\s*:",
                regex::escape(word)
            ))
            .expect("an escaped name keeps the pattern valid");
            members(&lines, k)
                .into_iter()
                .filter(|&i| key.is_match(lines[i]))
                .map(|i| i + 1)
                .take(1)
                .collect()
        }
    }
}

/// The 1-based line of the type the 0-based `line` of `text` stands inside, for a constructor
/// the type calls by no name of its own: PHP's `new self(…)` and `new static(…)`, Python's
/// `cls(…)`, Swift's `Self(…)`.
pub fn own_type(kind: Kind, text: &str, line: usize, name: &str) -> Option<usize> {
    let own = match kind {
        Kind::Php => matches!(name, "self" | "static"),
        Kind::Python => name == "cls",
        Kind::Swift => name == "Self",
        _ => false,
    };
    if !own {
        return None;
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut depth = indent(lines.get(line)?);
    for i in (0..line).rev() {
        let l = lines[i];
        if matches!(l.trim(), "" | "{") || indent(l) >= depth {
            continue;
        }
        depth = indent(l);
        if TYPE.is_match(l) {
            return Some(i + 1);
        }
    }
    None
}

/// The name the object parameter `n` of the function declared on 1-based `decl` is typed by,
/// `Props` of `({ a }: Props)` or `(p: Props)`: where its keys are declared when the parameter
/// does not spell them.
pub fn object_type(kind: Kind, text: &str, decl: usize, name: &str, n: usize) -> Option<String> {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r":\s*([A-Za-z_$][\w$]*)\s*(?:<[^>]*>)?\s*(?:=.*)?$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let k = decl.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut seen = 0;
    let mut found = None;
    params(kind, &lines, k, name, |p| {
        seen += 1;
        if seen == n + 1 {
            found = TYPE.captures(p.trim()).map(|c| c[1].to_owned());
        }
        None::<usize>
    });
    found
}

/// The lines of the members directly inside the declaration on 0-based line `k`: its body's
/// lines at the indent of the first of them.
fn members(lines: &[&str], k: usize) -> Vec<usize> {
    // A lone `{` under a C# or PHP header opens the body.
    let member = |i: &usize| !matches!(lines[*i].trim(), "" | "{");
    let body = body_of(Kind::Swift, lines, k);
    let Some(depth) = body.clone().find(member).map(|i| indent(lines[i])) else {
        return Vec::new();
    };
    body.filter(|i| member(i) && indent(lines[*i]) == depth)
        .collect()
}

/// The first 1-based line on which `find` finds something in a parameter of the declaration on
/// 0-based line `k`, reading the brackets that open after `name` there. `find` takes each
/// parameter in turn, as written, and answers the byte in it that it found.
fn params(
    kind: Kind,
    lines: &[&str],
    k: usize,
    name: &str,
    mut find: impl FnMut(&str) -> Option<usize>,
) -> Option<usize> {
    let from = match name {
        "" => 0,
        _ => lines[k].find(name).map_or(0, |i| i + name.len()),
    };
    let open = code(kind, lines[k])
        .find(|&(i, c)| i >= from && c == b'(')
        .map(|(i, _)| i)?;
    let (inner, ..) = group(kind, lines, k, open)?;
    let base = inner.as_ptr() as usize;
    split_top(kind, &inner, b',').into_iter().find_map(|p| {
        let at = find(p)? + (p.as_ptr() as usize - base);
        Some(k + 1 + inner[..at].matches('\n').count())
    })
}

/// The byte of `part`, one parameter as a declaration writes it, where it names `word`: Swift's
/// external label, else the parameter's name behind its modifiers and in front of its type,
/// `$` and `*` aside.
fn param_named(kind: Kind, part: &str, word: &str) -> Option<usize> {
    static ATTR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:@[\w.]+(?:\([^)]*\))?|#\[[^\]]*\])\s*").unwrap()
    });
    let head = split_top(kind, part, b'=').into_iter().next()?;
    let head = ATTR.replace_all(head, |c: &regex::Captures| " ".repeat(c[0].len()));
    let typed = match kind {
        Kind::CSharp | Kind::Php => None,
        _ => head.find(':'),
    };
    let names = &head[..typed.unwrap_or(head.len())];
    let mut tokens = names
        .split(|c: char| c.is_whitespace())
        .filter(|t| !t.is_empty());
    let token = match (kind, typed) {
        (Kind::Swift, _) => tokens.next()?,
        _ => tokens.next_back()?,
    };
    let bare = token
        .trim_start_matches(['$', '*', '&', '.'])
        .trim_end_matches(['?', '!']);
    if bare != word {
        return None;
    }
    let at = token.as_ptr() as usize - names.as_ptr() as usize;
    Some(at + (token.len() - token.trim_start_matches(['$', '*', '&', '.']).len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(kind: Kind, text: &str, probe: &str) -> Option<Label> {
        let (line, l) = text
            .lines()
            .enumerate()
            .find(|(_, l)| l.contains(probe))
            .expect("the probe is in the text");
        let start = l.find(probe).unwrap() + probe.find(|c: char| c.is_alphanumeric()).unwrap();
        let (range, _) = word_at(l, start, "").unwrap();
        label_at(kind, false, text, line, range)
    }

    fn owner(kind: Kind, text: &str, probe: &str) -> Option<(String, Owner)> {
        let l = label(kind, text, probe)?;
        let (line, col, owner) = l.owner?;
        let (_, w) = definition_word(Some(kind), text.lines().nth(line)?, col)?;
        Some((w.to_owned(), owner))
    }

    #[test]
    fn a_label_is_told_from_what_merely_looks_like_one() {
        let py = "def f(a, b=1):\n    return g(a, c=2) or (x == y)\nclass C(Base, metaclass=M):\n    pass\n";
        assert_eq!(
            owner(Kind::Python, py, "c=2"),
            Some(("g".into(), Owner::Args))
        );
        assert_eq!(label(Kind::Python, py, "b=1"), None);
        assert_eq!(label(Kind::Python, py, "metaclass"), None);
        assert_eq!(label(Kind::Python, py, "x =="), None);
        let kt = "fun card(vm: Vm) { vm.followTopic(\n    followedTopicId = \"a\", false) }\nfun f(a: Int = 1) {}\n";
        assert_eq!(
            owner(Kind::Jvm, kt, "followedTopicId"),
            Some(("followTopic".into(), Owner::Args))
        );
        assert_eq!(label(Kind::Jvm, kt, "Int = 1"), None);
        let tuple = "return (count: n, total: t);\nvar p = f(count: n);\n";
        assert_eq!(
            label(Kind::CSharp, tuple, "count: n,"),
            None,
            "a control keyword's brackets, or a tuple's, are no call's"
        );
        assert!(label(Kind::CSharp, tuple, "count: n)").is_some());
        let java = "for (i = 0; i < n; i++) {}\n@Size(max = 3)\n";
        assert!(
            label_at(Kind::Jvm, true, java, 0, 5..6).is_none(),
            "Java has no named arguments"
        );
        assert!(
            label_at(Kind::Jvm, true, java, 1, 6..9).is_some(),
            "only an annotation's brackets hold labels"
        );
        assert!(label_at(Kind::Jvm, false, java, 1, 6..9).is_some());
        let swift = "let w = RefreshWindow.init(interval: 30, maximumAttempts: 1)\nlet v = RefreshWindow(interval: 30)\ninit(interval: Double) {}\nlet t = a ? b : c\nx = self.init(name: 1)\n";
        assert_eq!(
            owner(Kind::Swift, swift, "maximumAttempts"),
            Some(("RefreshWindow".into(), Owner::Args))
        );
        assert_eq!(
            owner(Kind::Swift, swift, "interval: 30)"),
            Some(("RefreshWindow".into(), Owner::Args))
        );
        assert_eq!(label(Kind::Swift, swift, "interval: Double"), None);
        assert_eq!(label(Kind::Swift, swift, "b :"), None);
        assert_eq!(
            label(Kind::Swift, swift, "name: 1").map(|l| l.owner),
            Some(None)
        );
        let php = "$cap ??= discount(total: 5);\n$a = $x ? $b : $c;\nFoo::bar();\n";
        assert_eq!(
            owner(Kind::Php, php, "total"),
            Some(("discount".into(), Owner::Args))
        );
        assert_eq!(label(Kind::Php, php, "b :"), None);
        assert_eq!(label(Kind::Php, php, "Foo::"), None);
        let rb = "def f(rate: 1)\nend\nsnapshot!(x, rate_limit: false)\nh = { name: 1 }\nx = a ? b : c\n";
        assert_eq!(
            owner(Kind::Ruby, rb, "rate_limit"),
            Some(("snapshot!".into(), Owner::Args))
        );
        assert_eq!(label(Kind::Ruby, rb, "rate: 1"), None);
        assert_eq!(label(Kind::Ruby, rb, "name").map(|l| l.what), Some("key"));
        assert_eq!(label(Kind::Ruby, rb, "b :"), None);
        let cs = "var c = new Courier(name: \"post\");\nswitch (x) { case A: break; }\n";
        assert_eq!(
            owner(Kind::CSharp, cs, "name"),
            Some(("Courier".into(), Owner::Args))
        );
        assert_eq!(label(Kind::CSharp, cs, "A:"), None);
    }

    #[test]
    fn a_typescript_key_is_one_of_a_literal() {
        let ts = "context.report({ node: lastItem });\nconst o: Opts = { weight: 1 };\ntype P = { size: number };\nfunction f(o: { depth: number }) {}\nclass A { field: string }\nouter: for (;;) {}\nconst x = { a: b ? c : d, e };\nif (x) { label: while (1) {} }\n<Placeholder columns={all.length} />\n<input value={v} />\nfunction g<T = string>() {}\n";
        assert_eq!(
            owner(Kind::TsJs, ts, "node:"),
            Some(("report".into(), Owner::Object(0)))
        );
        assert_eq!(
            owner(Kind::TsJs, ts, "weight"),
            Some(("Opts".into(), Owner::Typed))
        );
        assert_eq!(label(Kind::TsJs, ts, "size"), None);
        assert_eq!(label(Kind::TsJs, ts, "depth"), None);
        assert_eq!(label(Kind::TsJs, ts, "field"), None);
        assert_eq!(label(Kind::TsJs, ts, "outer"), None);
        assert_eq!(label(Kind::TsJs, ts, "a: b").map(|l| l.what), Some("key"));
        assert_eq!(label(Kind::TsJs, ts, "c :"), None);
        assert_eq!(label(Kind::TsJs, ts, "label:"), None);
        assert_eq!(
            owner(Kind::TsJs, ts, "columns"),
            Some(("Placeholder".into(), Owner::Object(0)))
        );
        assert_eq!(label(Kind::TsJs, ts, "value").map(|l| l.owner), Some(None));
        assert_eq!(label(Kind::TsJs, ts, "T ="), None);
    }

    #[test]
    fn a_label_lands_on_the_parameter_or_the_field_it_names() {
        let swift = "struct RefreshWindow {\n    let maximumAttempts: Int\n    init(interval: Double = 30, maximumAttempts: Int = 5) { self.maximumAttempts = maximumAttempts }\n}\nstruct Plain {\n    let depth: Int\n}\nfunc go(_ a: Int, label inner: Int) {}\n";
        let lines =
            |decl, name, word| label_lines(Kind::Swift, swift, decl, name, word, &Owner::Args);
        assert_eq!(lines(1, "RefreshWindow", "maximumAttempts"), vec![3]);
        assert_eq!(lines(5, "Plain", "depth"), vec![6]);
        assert_eq!(lines(8, "go", "label"), vec![8]);
        assert_eq!(lines(8, "go", "inner"), Vec::<usize>::new());
        assert_eq!(lines(8, "go", "a"), Vec::<usize>::new());
        let py = "class Basket:\n    def __init__(\n        self,\n        tariff: Tariff = None,\n    ) -> None:\n        pass\n";
        assert_eq!(
            label_lines(Kind::Python, py, 1, "Basket", "tariff", &Owner::Args),
            vec![4]
        );
        let kt = "class Vm(val repo: Repo) {\n    fun followTopic(followedTopicId: String, followed: Boolean) {}\n}\n";
        assert_eq!(
            label_lines(Kind::Jvm, kt, 1, "Vm", "repo", &Owner::Args),
            vec![1]
        );
        assert_eq!(
            label_lines(
                Kind::Jvm,
                kt,
                2,
                "followTopic",
                "followedTopicId",
                &Owner::Args
            ),
            vec![2]
        );
        let php = "function discount(int $total): int\n{\n}\nclass Song {\n    public function __construct(private readonly int $ids) {}\n}\n";
        assert_eq!(
            label_lines(Kind::Php, php, 1, "discount", "total", &Owner::Args),
            vec![1]
        );
        assert_eq!(
            label_lines(Kind::Php, php, 4, "Song", "ids", &Owner::Args),
            vec![5]
        );
        let cs = "public class Courier\n{\n    public Courier(string name) { }\n    public Courier(int id) { }\n}\n";
        assert_eq!(
            label_lines(Kind::CSharp, cs, 1, "Courier", "name", &Owner::Args),
            vec![3]
        );
        let ts = "function schedule(\n  a: number,\n  o: { weight: number },\n): number {}\nfunction Placeholder({ columns, rows }: Props) {}\ninterface Opts {\n  readonly weight?: number;\n}\n";
        assert_eq!(
            label_lines(Kind::TsJs, ts, 1, "schedule", "weight", &Owner::Object(1)),
            vec![3]
        );
        assert_eq!(
            label_lines(Kind::TsJs, ts, 1, "schedule", "weight", &Owner::Object(0)),
            Vec::<usize>::new()
        );
        assert_eq!(
            label_lines(Kind::TsJs, ts, 5, "Placeholder", "rows", &Owner::Object(0)),
            vec![5]
        );
        assert_eq!(
            object_type(Kind::TsJs, ts, 5, "Placeholder", 0),
            Some("Props".into())
        );
        assert_eq!(
            label_lines(Kind::TsJs, ts, 6, "Opts", "weight", &Owner::Typed),
            vec![7]
        );
        let own = "final class Artist\n{\n    public function __construct(\n        public ?string $image,\n    ) {}\n\n    public static function make(): self\n    {\n        return new self(image: null);\n    }\n}\n";
        assert_eq!(own_type(Kind::Php, own, 8, "self"), Some(1));
        assert_eq!(own_type(Kind::Php, own, 8, "make"), None);
        assert_eq!(
            label_lines(Kind::Php, own, 1, "self", "image", &Owner::Args),
            vec![4]
        );
    }

    /// #529: a Rust struct literal's key is a field of its type; the `{` of a declaration, of a
    /// struct-like variant in an `enum` or of a block opens no literal.
    #[test]
    fn a_rust_literal_key_is_a_field_of_its_type() {
        let rs = "pub struct Printer {\n    pub(crate) hyperlink: u32,\n}\n\npub enum Page {\n    Linked { hyperlink: u32 },\n}\n\nimpl Printer {\n    fn fresh() -> Self {\n        Self { hyperlink: 0 }\n    }\n}\n\nfn f<T>(t: T) -> Printer\nwhere\n    T: Clone,\n{\n    crate::p::Printer { hyperlink: 1 }\n}\n";
        let typed = |w: &str| Some((w.to_owned(), Owner::Typed));
        assert_eq!(owner(Kind::Rust, rs, "{ hyperlink: 1"), typed("Printer"));
        assert_eq!(owner(Kind::Rust, rs, "{ hyperlink: 0"), typed("Printer"));
        assert_eq!(label(Kind::Rust, rs, ") hyperlink"), None);
        assert_eq!(label(Kind::Rust, rs, "{ hyperlink: u32"), None);
        assert_eq!(label(Kind::Rust, rs, "    T: Clone"), None);
        assert_eq!(
            label_lines(Kind::Rust, rs, 1, "Printer", "hyperlink", &Owner::Typed),
            vec![2]
        );
    }
}
