use std::ops::Range;

use regex::Regex;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    Args,
    Object(usize),
    Typed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub what: &'static str,
    pub owner: Option<LabelOwner>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelOwner {
    pub line0: usize,
    pub byte_col: usize,
    pub how: Owner,
}

static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:[\w@]+\s+)*?(?:class|struct|record|enum|actor|object|interface)\s+([A-Za-z_$][\w$]*)")
        .unwrap()
});

const LINES_BACK_TO_THE_BRACKET: usize = 80;

pub fn label_at(
    kind: Kind,
    java: bool,
    text: &str,
    line0: usize,
    range: Range<usize>,
) -> Option<Label> {
    let lines: Vec<&str> = text.lines().collect();
    let l = *lines.get(line0)?;
    let tight = &l[range.end..];
    let after = match kind {
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
        return jsx(kind, &lines, &literal, line0, range.start);
    }
    let OpenBracket {
        line0: at,
        byte: open,
        bracket,
        first_met,
        ..
    } = open_bracket(kind, &lines, &literal, line0, range.start)?;
    if first_met != Some(bracket) && first_met != Some(b',') {
        return None;
    }
    let key = |owner| Some(Label { what: "key", owner });
    match (kind, bracket) {
        (Kind::TsJs, b'{') => {
            let CodeByte {
                line0: bl,
                index: bi,
                byte: b,
            } = back(kind, &lines, &literal, at, open).next()?;
            let before = &lines[bl][..bi];
            match b {
                b'(' | b',' => {
                    let call = open_bracket(kind, &lines, &literal, at, open)?;
                    let (cl, ci) = (call.line0, call.byte);
                    let owner = (call.bracket == b'(' && !declares(&lines[cl][..ci]))
                        .then(|| callee(kind, lines[cl], ci))
                        .flatten()
                        .map(|col| LabelOwner {
                            line0: cl,
                            byte_col: col,
                            how: Owner::Object(call.commas),
                        });
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
                        .map(|m| LabelOwner {
                            line0: bl,
                            byte_col: m.end() - 1,
                            how: Owner::Typed,
                        });
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
            if let Some(OpenBracket {
                line0: el,
                byte: ei,
                bracket: b'{',
                ..
            }) = up
                && lines[el][..ei].contains("enum ")
            {
                return None;
            }
            if ty.as_str() != "Self" {
                return key(Some(LabelOwner {
                    line0: at,
                    byte_col: ty.end() - 1,
                    how: Owner::Typed,
                }));
            }
            static IMPL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"^\s*(?:unsafe\s+)?impl\b.*?(?:\bfor\s+)?(?:\w+::)*(\w+)\s*(?:<[^{]*>)?\s*(?:where\b.*)?$")
                    .unwrap()
            });
            while let Some(OpenBracket {
                line0: el,
                byte: ei,
                bracket,
                ..
            }) = up
            {
                if bracket == b'{'
                    && let Some(m) = IMPL.captures(&lines[el][..ei]).and_then(|c| c.get(1))
                {
                    return key(Some(LabelOwner {
                        line0: el,
                        byte_col: m.end() - 1,
                        how: Owner::Typed,
                    }));
                }
                up = open_bracket(kind, &lines, &literal, el, ei);
            }
            key(None)
        }
        (Kind::Ruby, b'{') => {
            let block = back(kind, &lines, &literal, at, open)
                .next()
                .is_some_and(|c| {
                    c.line0 == at
                        && (c.byte == b')' || c.byte.is_ascii_alphanumeric() || c.byte == b'_')
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
            let owner = match kind == Kind::Swift
                && word_at(lines[at], col, "").map(|w| w.1) == Some("init")
            {
                true => {
                    let head = lines[at][..col + 1 - "init".len()].strip_suffix('.');
                    head.and_then(|h| {
                        let name = h
                            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                            .next()?;
                        (name.starts_with(|c: char| c.is_ascii_uppercase())).then(|| LabelOwner {
                            line0: at,
                            byte_col: h.len() - 1,
                            how: Owner::Args,
                        })
                    })
                }
                false => Some(LabelOwner {
                    line0: at,
                    byte_col: col,
                    how: Owner::Args,
                }),
            };
            Some(Label {
                what: "argument label",
                owner,
            })
        }
        _ => None,
    }
}

fn declares(before: &str) -> bool {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.$])(?:def|fun|func|function|fn|class|case|subscript|constructor|init|lambda)\b[^=(]*$")
            .unwrap()
    });
    DECL.is_match(before)
}

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
    const STATEMENT_KEYWORDS: &[&str] = &[
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
        && STATEMENT_KEYWORDS.contains(&w)
        && !s[..s.len() - w.len()].ends_with('.')
    {
        return None;
    }
    let last = s.chars().next_back()?;
    let named = last.is_alphanumeric() || last == '_' || last == '$';
    let ruby_name_with_its_suffix = kind == Kind::Ruby && matches!(last, '!' | '?') && s.len() > 1;
    match (named, ruby_name_with_its_suffix) {
        (true, _) => Some(s.len() - last.len_utf8()),
        (false, true) => Some(s.len() - 2),
        _ => None,
    }
}

fn events(kind: Kind, line: &str) -> Vec<(usize, u8)> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    for (i, c) in code(kind, line) {
        let hash = c == b'#' && kind == Kind::Php && b.get(i + 1) != Some(&b'[');
        if c == 0 || hash {
            break;
        }
        if !c.is_ascii_whitespace() {
            out.push((i, c));
        }
    }
    out
}

struct CodeByte {
    line0: usize,
    index: usize,
    byte: u8,
}

fn back<'a>(
    kind: Kind,
    lines: &'a [&'a str],
    literal: &'a [bool],
    line0: usize,
    end: usize,
) -> impl Iterator<Item = CodeByte> + 'a {
    (line0.saturating_sub(LINES_BACK_TO_THE_BRACKET)..=line0)
        .rev()
        .flat_map(move |li| {
            let ev = match literal.get(li) {
                Some(true) => Vec::new(),
                _ => events(kind, lines[li]),
            };
            ev.into_iter()
                .filter(move |&(i, _)| li != line0 || i < end)
                .rev()
                .map(move |(i, c)| CodeByte {
                    line0: li,
                    index: i,
                    byte: c,
                })
        })
}

struct OpenBracket {
    line0: usize,
    byte: usize,
    bracket: u8,
    first_met: Option<u8>,
    commas: usize,
}

fn open_bracket(
    kind: Kind,
    lines: &[&str],
    literal: &[bool],
    line0: usize,
    end: usize,
) -> Option<OpenBracket> {
    let (mut depth, mut first_met, mut commas) = (0usize, None, 0);
    for CodeByte {
        line0: li,
        index: i,
        byte: c,
    } in back(kind, lines, literal, line0, end)
    {
        first_met.get_or_insert(c);
        match c {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' if depth > 0 => depth -= 1,
            b'(' | b'[' | b'{' => {
                return Some(OpenBracket {
                    line0: li,
                    byte: i,
                    bracket: c,
                    first_met,
                    commas,
                });
            }
            b',' if depth == 0 => commas += 1,
            b';' if depth == 0 => return None,
            _ => {}
        }
    }
    None
}

fn jsx(kind: Kind, lines: &[&str], literal: &[bool], line0: usize, start: usize) -> Option<Label> {
    if !lines[line0][..start].is_empty() && !lines[line0][..start].ends_with(char::is_whitespace) {
        return None;
    }
    let mut depth = 0usize;
    for CodeByte {
        line0: li,
        index: i,
        byte: c,
    } in back(kind, lines, literal, line0, start)
    {
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
                let owner =
                    last.starts_with(|c: char| c.is_ascii_uppercase())
                        .then(|| LabelOwner {
                            line0: li,
                            byte_col: i + name.len(),
                            how: Owner::Object(0),
                        });
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

pub fn label_lines(
    kind: Kind,
    text: &str,
    decl_line1: usize,
    name: &str,
    word: &str,
    owner: &Owner,
) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = decl_line1.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let types_own_name = TYPE.captures(lines[k]).map(|c| c[1].to_owned());
    match (owner, types_own_name.as_deref()) {
        (Owner::Args, Some(name)) => {
            let n = regex::escape(name);
            let kotlin_or_csharp_primary_constructor = matches!(kind, Kind::Jvm | Kind::CSharp)
                .then(|| params(kind, &lines, k, name, |p| param_named(kind, p, word)))
                .flatten();
            if let Some(line) = kotlin_or_csharp_primary_constructor {
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

pub fn own_type(kind: Kind, text: &str, line0: usize, name: &str) -> Option<usize> {
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
    let mut depth = indent(lines.get(line0)?);
    for i in (0..line0).rev() {
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

pub fn object_type(
    kind: Kind,
    text: &str,
    decl_line1: usize,
    name: &str,
    n: usize,
) -> Option<String> {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r":\s*([A-Za-z_$][\w$]*)\s*(?:<[^>]*>)?\s*(?:=.*)?$").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let k = decl_line1.checked_sub(1).filter(|&k| k < lines.len())?;
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

fn members(lines: &[&str], decl_line0: usize) -> Vec<usize> {
    let member = |i: &usize| !matches!(lines[*i].trim(), "" | "{");
    let body = body_of(Kind::Swift, lines, decl_line0);
    let Some(depth) = body.clone().find(member).map(|i| indent(lines[i])) else {
        return Vec::new();
    };
    body.filter(|i| member(i) && indent(lines[*i]) == depth)
        .collect()
}

fn params(
    kind: Kind,
    lines: &[&str],
    decl_line0: usize,
    name: &str,
    mut find: impl FnMut(&str) -> Option<usize>,
) -> Option<usize> {
    let from = match name {
        "" => 0,
        _ => lines[decl_line0].find(name).map_or(0, |i| i + name.len()),
    };
    let open = code(kind, lines[decl_line0])
        .find(|&(i, c)| i >= from && c == b'(')
        .map(|(i, _)| i)?;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(kind, lines, decl_line0, open)?;
    let base = inner.as_ptr() as usize;
    split_top(kind, &inner, b',').into_iter().find_map(|p| {
        let at = find(p)? + (p.as_ptr() as usize - base);
        Some(decl_line0 + 1 + inner[..at].matches('\n').count())
    })
}

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
        let LabelOwner {
            line0: line,
            byte_col: col,
            how: owner,
        } = l.owner?;
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

    #[test]
    fn a_label_is_read_past_comments_literals_and_type_arguments() {
        let rb =
            "run { name: 1 }\nf(x) { size: 1 }\nrun(\n  { tag: 1 })\ng(a, # note\n  rate: 1)\n";
        assert_eq!(label(Kind::Ruby, rb, "name"), None, "a block's brace");
        assert_eq!(label(Kind::Ruby, rb, "size"), None, "a block's brace");
        assert_eq!(label(Kind::Ruby, rb, "tag").map(|l| l.what), Some("key"));
        assert_eq!(
            owner(Kind::Ruby, rb, "rate"),
            Some(("g".into(), Owner::Args)),
            "past a comment on the line above"
        );
        let py = "x = g(\"\"\"\n(\n\"\"\", c=2)\nh(a; b, c=3)\n";
        assert_eq!(
            owner(Kind::Python, py, "c=2"),
            Some(("g".into(), Owner::Args)),
            "a bracket inside a string opens nothing"
        );
        assert_eq!(label(Kind::Python, py, "c=3"), None, "a `;` ends it");
        let cs = "var c = new Courier<int>(name: 1);\n";
        assert_eq!(
            owner(Kind::CSharp, cs, "name"),
            Some(("Courier".into(), Owner::Args))
        );
    }

    #[test]
    fn a_label_lands_where_the_type_builds_itself() {
        let py = "@dataclass\nclass Point:\n    x: int\n    y: int = 0\n\n    @classmethod\n    def origin(cls):\n        return cls(x=0)\n";
        assert_eq!(
            label_lines(Kind::Python, py, 2, "Point", "y", &Owner::Args),
            vec![4]
        );
        assert_eq!(own_type(Kind::Python, py, 7, "cls"), Some(2));
        let swift = "struct Box {\n    let w: Int\n    static func make() -> Self {\n        Self(w: 1)\n    }\n}\n";
        assert_eq!(own_type(Kind::Swift, swift, 3, "Self"), Some(1));
        let php = "class A\n{\n    public static function make()\n    {\n        return new static(x: 1);\n    }\n}\n";
        assert_eq!(own_type(Kind::Php, php, 4, "static"), Some(1));
        let ts = "function f(p: Props) {}\n";
        assert_eq!(object_type(Kind::TsJs, ts, 1, "f", 0), Some("Props".into()));
        let star = "def f(*items, **opts):\n    pass\n";
        assert_eq!(
            label_lines(Kind::Python, star, 1, "f", "opts", &Owner::Args),
            vec![1]
        );
    }

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
