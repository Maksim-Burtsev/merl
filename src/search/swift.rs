use std::path::{Path, PathBuf};

use regex::Regex;

use super::{
    Binding, Kind, Value, close_of, code, comment, group, indent, literal_lines, names, split_top,
    swift_mods, uncommented,
};

static SWIFT_FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:func\s|(?:init|subscript)\s*[?!(<]|deinit\b|(?:get|set|willSet|didSet|_read|_modify)\b|(?:var|let)\s+`?\w+`?\s*:[^=]*\{{\s*$)",
        swift_mods!()
    ))
    .expect("the regex is a literal")
});
static SWIFT_TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:class|struct|enum|protocol|actor|extension)\s+[`\w\[]",
        swift_mods!()
    ))
    .expect("the regex is a literal")
});
static SWIFT_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(r"{}(?:let|var)\s+(.*)$", swift_mods!())).expect("the regex is a literal")
});
static SWIFT_LOCAL_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:func|class|struct|enum|actor|typealias)\s+`?(\w+)",
        swift_mods!()
    ))
    .expect("the regex is a literal")
});
pub fn swift_right_of_binding(lead: &str) -> bool {
    static RIGHT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:let|var)\s[^=;]*[^=!<>;]=(?:[^=]|$)[^;]*$")
            .expect("the regex is a literal")
    });
    RIGHT.is_match(&uncommented(Kind::Swift, lead))
}
fn swift_code<S: AsRef<str>>(lines: &[S], literal: &[bool], i: usize) -> bool {
    let t = lines[i].as_ref().trim();
    !t.is_empty()
        && !literal.get(i).copied().unwrap_or(false)
        && !t.starts_with('#')
        && !comment(Kind::Swift, t)
}
fn swift_header_start<S: AsRef<str>>(lines: &[S], literal: &[bool], mut i: usize) -> usize {
    loop {
        let ind = indent(lines[i].as_ref());
        let wrapped = lines[i]
            .as_ref()
            .trim_start()
            .starts_with([')', '{', ']', '>']);
        let code = |j: usize| swift_code(lines, literal, j);
        let Some(j) = (0..i)
            .rev()
            .find(|&j| code(j) && indent(lines[j].as_ref()) <= ind)
        else {
            return i;
        };
        let comma = (0..i).rev().find(|&j| code(j)) == Some(j)
            && uncommented(Kind::Swift, lines[j].as_ref())
                .trim_end()
                .ends_with(',');
        if !wrapped && !comma {
            return i;
        }
        i = j;
    }
}
pub fn swift_local_decl(line: &str, name: &str) -> bool {
    SWIFT_LOCAL_DECL
        .captures(&uncommented(Kind::Swift, line))
        .is_some_and(|c| &c[1] == name)
}
#[derive(Clone, Copy)]
pub struct SwiftScope {
    pub header_line1: usize,
    pub local: bool,
}
pub fn swift_scope<S: AsRef<str>>(lines: &[S], literal: &[bool], line: usize) -> SwiftScope {
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return SwiftScope {
            header_line1: 0,
            local: false,
        };
    };
    let (mut depth, mut i, mut local) = (indent(target.as_ref()), line - 1, None);
    while i > 0 && depth > 0 {
        i -= 1;
        if !swift_code(lines, literal, i) || indent(lines[i].as_ref()) >= depth {
            continue;
        }
        depth = indent(lines[i].as_ref());
        i = swift_header_start(lines, literal, i);
        let t = uncommented(Kind::Swift, lines[i].as_ref());
        let func = SWIFT_FUNC.is_match(&t);
        let ty = !func && SWIFT_TYPE.is_match(&t);
        let local = *local.get_or_insert(!ty);
        if func || ty {
            return SwiftScope {
                header_line1: i + 1,
                local,
            };
        }
    }
    SwiftScope {
        header_line1: 0,
        local: local.unwrap_or(false),
    }
}
pub fn swift_type_header(line: &str) -> Option<SwiftTypeHeader> {
    static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"{}(class|struct|enum|protocol|actor|extension)\s+`?([\w.]+)`?(?:<[^>]*>)?\s*(?::\s*([\w.]+))?",
            swift_mods!()
        ))
        .expect("the regex is a literal")
    });
    let t = uncommented(Kind::Swift, line);
    if SWIFT_FUNC.is_match(&t) {
        return None;
    }
    let c = HEADER.captures(&t)?;
    Some(SwiftTypeHeader {
        keyword: c[1].to_owned(),
        name: c[2].to_owned(),
        first_inherited: c.get(3).map(|m| m.as_str().to_owned()),
        where_constrained: t.contains(" where "),
    })
}
pub fn swift_inherited(line: &str) -> Vec<String> {
    static INHERITS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"{}(?:class|struct|enum|protocol|actor|extension)\s+`?[\w.]+`?(?:<[^>]*>)?\s*:([^{{]*)",
            swift_mods!()
        ))
        .expect("the regex is a literal")
    });
    let t = uncommented(Kind::Swift, line);
    let Some(c) = INHERITS.captures(&t) else {
        return Vec::new();
    };
    let list = c[1].split(" where ").next().unwrap_or_default();
    split_top(Kind::Swift, list, b',')
        .into_iter()
        .filter_map(|n| swift_type_name(n.split('<').next().unwrap_or(n)))
        .collect()
}
pub struct SwiftTypeHeader {
    pub keyword: String,
    pub name: String,
    pub first_inherited: Option<String>,
    pub where_constrained: bool,
}
pub fn swift_enclosing_type<S: AsRef<str>>(
    lines: &[S],
    literal: &[bool],
    line: usize,
) -> Option<usize> {
    let mut at = line;
    loop {
        at = swift_scope(lines, literal, at).header_line1;
        if at == 0 {
            return None;
        }
        if swift_type_header(lines[at - 1].as_ref()).is_some() {
            return Some(at);
        }
    }
}
pub fn swift_in_top_level_func<S: AsRef<str>>(lines: &[S], literal: &[bool], line: usize) -> bool {
    let (mut at, mut outer) = (line, 0);
    loop {
        match swift_scope(lines, literal, at).header_line1 {
            0 => break,
            up if swift_type_header(lines[up - 1].as_ref()).is_some() => return false,
            up => (at, outer) = (up, up),
        }
    }
    outer > 0 && {
        let t = uncommented(Kind::Swift, lines[outer - 1].as_ref());
        indent(&t) == 0 && SWIFT_FUNC.is_match(&t)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct SwiftCall {
    pub labels: Vec<String>,
    pub trailing_closure: bool,
}
pub fn swift_call_labels(lines: &[&str], line0: usize, after: usize) -> Option<SwiftCall> {
    static LABEL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*`?(\w+)`?\s*:").expect("the regex is a literal")
    });
    let line = lines.get(line0)?;
    let rest = line.get(after..)?;
    if rest.trim_start().starts_with('{') {
        let condition = ["if ", "guard ", "while ", "for ", "switch ", "} else"]
            .iter()
            .any(|k| line.trim_start().starts_with(k));
        return (!condition).then(|| SwiftCall {
            labels: Vec::new(),
            trailing_closure: true,
        });
    }
    if !rest.starts_with('(') {
        return None;
    }
    let g = group(Kind::Swift, lines, line0, after)?;
    let trailing_closure = g.after_close.trim_start().starts_with('{');
    let inner = g.inner_uncommented;
    let labels = if inner.trim().is_empty() {
        Vec::new()
    } else {
        split_top(Kind::Swift, &inner, b',')
            .into_iter()
            .map(|a| {
                LABEL
                    .captures(a)
                    .map_or_else(|| "_".to_owned(), |c| c[1].to_owned())
            })
            .collect()
    };
    Some(SwiftCall {
        labels,
        trailing_closure,
    })
}
#[derive(Debug, PartialEq, Eq)]
pub struct SwiftParam {
    pub label: String,
    pub defaulted: bool,
    pub variadic: bool,
    pub closure: bool,
}
pub fn swift_param_labels(lines: &[&str], line0: usize, name: &str) -> Option<Vec<SwiftParam>> {
    let line = lines.get(line0)?;
    let from = line.find(name)? + name.len();
    let open = code(Kind::Swift, line)
        .find(|&(i, c)| i >= from && c == b'(')
        .map(|(i, _)| i)?;
    let between = line.get(from..open)?.trim();
    if !SWIFT_FUNC.is_match(line)
        || !(matches!(between, "" | "?" | "!") || between.starts_with('<'))
    {
        return None;
    }
    let inner = group(Kind::Swift, lines, line0, open)?.inner_uncommented;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    split_top(Kind::Swift, &inner, b',')
        .into_iter()
        .map(|p| {
            let parts = split_top(Kind::Swift, p, b'=');
            let (names, ty) = parts[0].split_once(':')?;
            let label = names.split_whitespace().next()?;
            let ty = ty.trim();
            let ty = ty
                .split_whitespace()
                .skip_while(|w| w.starts_with('@'))
                .collect::<Vec<_>>()
                .join(" ");
            let bare = ty
                .strip_prefix('(')
                .and_then(|t| t.strip_suffix(")?").or_else(|| t.strip_suffix(")!")))
                .unwrap_or(&ty);
            Some(SwiftParam {
                label: label.trim_matches('`').to_owned(),
                defaulted: parts.len() > 1,
                variadic: ty.ends_with("..."),
                closure: top_level_arrow(bare),
            })
        })
        .collect()
}
fn top_level_arrow(ty: &str) -> bool {
    let mut depth = 0i32;
    let b = ty.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' | b'<' => depth += 1,
            b')' | b']' => depth -= 1,
            b'>' if i > 0 && b[i - 1] != b'-' => depth -= 1,
            b'-' if depth == 0 && b.get(i + 1) == Some(&b'>') => return true,
            _ => {}
        }
    }
    false
}
pub fn swift_call_fits(call: &SwiftCall, params: &[SwiftParam]) -> bool {
    let mut args = call.labels.iter().peekable();
    let mut rest = params.iter();
    for p in rest.by_ref() {
        match args.peek() {
            Some(l) if **l == p.label => {
                args.next();
                if p.variadic {
                    while args.peek().is_some_and(|l| *l == "_") {
                        args.next();
                    }
                }
            }
            None if call.trailing_closure && p.closure => {
                return rest.all(|p| p.defaulted || p.variadic);
            }
            _ if p.defaulted || p.variadic => {}
            _ => return false,
        }
    }
    args.peek().is_none() && !call.trailing_closure
}
pub fn swift_instance_member(line: &str) -> bool {
    swift_member_kind(line) == Some(SwiftMember::Instance)
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SwiftMember {
    StaticOrClass,
    Instance,
}
pub fn swift_member_kind(line: &str) -> Option<SwiftMember> {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"({})(?:func|let|var)\s", swift_mods!()))
            .expect("the regex is a literal")
    });
    let c = DECL.captures(line)?;
    let shared = c[1]
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|w| w == "static" || w == "class");
    Some(if shared {
        SwiftMember::StaticOrClass
    } else {
        SwiftMember::Instance
    })
}
pub fn swift_case(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("case ") || t.starts_with("indirect case ")
}
pub fn swift_continues(text: &str, line0: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Swift, text);
    let Some(above) = (0..line0.min(lines.len()))
        .rev()
        .find(|&i| swift_code(&lines, &literal, i))
    else {
        return false;
    };
    let code = uncommented(Kind::Swift, lines[above]);
    let code = code.trim_end();
    let keyword = ["return", "case", "in", "else", "throw", "try", "await"]
        .iter()
        .any(|k| {
            code.strip_suffix(k)
                .is_some_and(|b| !b.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
        });
    !keyword && code.ends_with(|c: char| c.is_alphanumeric() || "_)]}?!\">".contains(c))
}
pub fn swift_extended(line: &str) -> Option<String> {
    static EXTENDED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"{}extension\s+(\[[^\]]*\]|[\w.]+)",
            swift_mods!()
        ))
        .expect("the regex is a literal")
    });
    EXTENDED
        .captures(&uncommented(Kind::Swift, line))
        .map(|c| c[1].to_owned())
}
pub fn swift_signature_line(text: &str, line1: usize) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Swift, text);
    let Some(mut i) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return line1;
    };
    loop {
        i = swift_header_start(&lines, &literal, i);
        match (0..i).rev().find(|&j| swift_code(&lines, &literal, j)) {
            Some(j) if uncommented(Kind::Swift, lines[j]).trim_end().ends_with('(') => i = j,
            _ => return i + 1,
        }
    }
}
pub fn swift_alias(line: &str, name: &str) -> Option<String> {
    static ALIAS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"{}typealias\s+`?(\w+)`?\s*(?:<[^=]*>)?\s*=\s*(.+?)\s*$",
            swift_mods!()
        ))
        .expect("the regex is a literal")
    });
    let t = uncommented(Kind::Swift, line);
    let c = ALIAS.captures(&t)?;
    (&c[1] == name).then(|| c[2].to_owned())
}
pub fn swift_call_links(before: &str) -> Option<Vec<String>> {
    let mut rest = before.strip_suffix('.')?;
    let mut links = Vec::new();
    loop {
        let open = code(Kind::Swift, rest)
            .filter(|(_, c)| *c == b'(')
            .map(|(i, _)| i)
            .find(|&i| close_of(Kind::Swift, rest, i) == Some(rest.len()))?;
        let head = rest.get(..open)?;
        let start = head
            .char_indices()
            .rev()
            .find(|&(_, c)| !(c.is_alphanumeric() || c == '_'))
            .map_or(0, |(i, c)| i + c.len_utf8());
        let name = head.get(start..)?;
        if !name.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            return None;
        }
        links.insert(0, name.to_owned());
        match head.get(..start)?.strip_suffix('.') {
            Some(r) if r.ends_with(')') => rest = r,
            Some(_) => return None,
            None => break,
        }
    }
    (links.len() > 1).then_some(links)
}
pub fn swift_literal_call(before: &str) -> Option<(String, String, Vec<String>)> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.)\]?!])([a-z_]\w*(?:\.[A-Za-z_]\w*)*)\.([A-Za-z_]\w*)$")
            .expect("the regex is a literal")
    });
    static LITERAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^(?:\w+\s*:\s*)?(?:-?\d[\d_.]*|"[^"\\]*"|true|false)$"#)
            .expect("the regex is a literal")
    });
    static VALUE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\w+\s*:\s*)?((?:self|[a-z_]\w*)(?:\.[a-z_]\w*)*)$")
            .expect("the regex is a literal")
    });
    let rest = before.strip_suffix('.')?;
    let open = code(Kind::Swift, rest)
        .filter(|(_, c)| *c == b'(')
        .map(|(i, _)| i)
        .find(|&i| close_of(Kind::Swift, rest, i) == Some(rest.len()))?;
    let args = rest.get(open + 1..rest.len() - 1)?.trim();
    let mut values = Vec::new();
    for a in split_top(Kind::Swift, args, b',')
        .iter()
        .filter(|_| !args.is_empty())
    {
        let a = a.trim();
        if !LITERAL.is_match(a) {
            let v = VALUE.captures(a).map(|c| c[1].to_owned())?;
            if v.split('.').skip(1).any(|s| s == "self") {
                return None;
            }
            values.push(v);
        }
    }
    let c = HEAD.captures(rest.get(..open)?)?;
    Some((c[1].to_owned(), c[2].to_owned(), values))
}
pub fn swift_if_arms(lines: &[&str], line1s: &[usize]) -> bool {
    let directive = |l: usize| {
        let t = lines.get(l - 1)?.trim_start();
        ["#if", "#elseif", "#else", "#endif"].into_iter().find(|d| {
            t.strip_prefix(d)
                .is_some_and(|r| !r.starts_with(char::is_alphanumeric))
        })
    };
    let mut at = line1s.to_vec();
    at.sort_unstable();
    let [first, ..] = at.as_slice() else {
        return false;
    };
    at.len() > 1
        && (1..*first).rev().find_map(directive) == Some("#if")
        && at.windows(2).all(|w| {
            let between: Vec<&str> = (w[0] + 1..w[1]).filter_map(directive).collect();
            matches!(between.as_slice(), ["#elseif" | "#else"])
        })
}
pub fn swift_outside_name(written: &str) -> Option<String> {
    let t = written.trim().trim_end_matches(['?', '!']);
    if t.starts_with('[') && t.ends_with(']') {
        let array = swift_element(t).is_some();
        return Some(if array { "Array" } else { "Dictionary" }.to_owned());
    }
    if let Some((head, _)) = t.split_once('.')
        && head.contains('<')
        && let Some(plain) = swift_unspecialized(t)
        && plain.contains('.')
    {
        return swift_nested_type_name(&plain);
    }
    swift_type_name(t).filter(|n| n.starts_with(char::is_uppercase))
}
pub fn swift_unspecialized(written: &str) -> Option<String> {
    let mut depth = 0usize;
    let mut plain = String::new();
    for c in written.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.checked_sub(1)?,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    (depth == 0).then_some(plain)
}
pub fn swift_extension(line: &str) -> bool {
    static EXTENSION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"{}extension\s", swift_mods!())).expect("the regex is a literal")
    });
    EXTENSION.is_match(line)
}
pub fn swift_local<S: AsRef<str>>(lines: &[S], literal: &[bool], line1: usize) -> Option<usize> {
    let text = lines.get(line1.checked_sub(1)?)?.as_ref();
    let SwiftScope {
        header_line1: scope,
        local,
    } = swift_scope(lines, literal, line1);
    (local && SWIFT_DECL.is_match(&uncommented(Kind::Swift, text))).then_some(scope)
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Binds {
    Name,
    Nothing,
    UnreadablePatternMentionsName,
}
impl Binds {
    fn name_if(binds: bool) -> Self {
        if binds { Self::Name } else { Self::Nothing }
    }
    fn first_of(each: impl IntoIterator<Item = Self>) -> Self {
        let mut out = Self::Nothing;
        for b in each {
            match b {
                Self::Name => return Self::Name,
                Self::UnreadablePatternMentionsName => out = b,
                Self::Nothing => {}
            }
        }
        out
    }
}
fn swift_pattern_binds(pattern: &str, bind_all: bool, name: &str) -> Binds {
    let p = pattern.trim();
    let p = p.split(" where ").next().unwrap_or(p).trim();
    let (p, bind_all) = match p.strip_prefix("let ").or_else(|| p.strip_prefix("var ")) {
        Some(rest) => (rest.trim(), true),
        None => (p, bind_all),
    };
    let p = p.split(" as ").next().unwrap_or(p).trim();
    let bare = |n: &str| n.trim().trim_matches('`') == name;
    let Some((_, inner)) = p.split_once('(') else {
        return Binds::name_if(bind_all && bare(p));
    };
    let inner = inner.trim_end();
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    if inner.contains(['(', '[']) {
        return if names(inner, name) {
            Binds::UnreadablePatternMentionsName
        } else {
            Binds::Nothing
        };
    }
    Binds::name_if(split_top(Kind::Swift, inner, b',').iter().any(|e| {
        let past_label = e.rsplit(':').next().unwrap_or(e).trim();
        match past_label
            .strip_prefix("let ")
            .or_else(|| past_label.strip_prefix("var "))
        {
            Some(n) => bare(n),
            None => bind_all && bare(past_label),
        }
    }))
}
fn swift_condition_binds(from_keyword: &str, name: &str) -> Binds {
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:if|while|guard)\s").expect("the regex is a literal")
    });
    let Some(m) = KEYWORD.find(from_keyword) else {
        return Binds::Nothing;
    };
    let Some(rest) = from_keyword.get(m.end()..) else {
        return Binds::Nothing;
    };
    let rest = rest.trim_end().trim_end_matches('{');
    let rest = if m.as_str().starts_with("guard") {
        rest.rsplit_once(" else").map_or(rest, |(c, _)| c)
    } else {
        rest
    };
    Binds::first_of(
        split_top(Kind::Swift, rest, b',')
            .into_iter()
            .filter_map(|clause| {
                let c = clause.trim();
                let (pattern, bind_all) = match c.strip_prefix("case ") {
                    Some(p) => (p, false),
                    None if c.starts_with("let ") || c.starts_with("var ") => (c, true),
                    None => return None,
                };
                let parts = split_top(Kind::Swift, pattern, b'=');
                if bind_all && parts.len() == 1 {
                    return Some(Binds::name_if(swift_shorthand(c, name)));
                }
                (parts.len() >= 2).then(|| {
                    swift_pattern_binds(split_top(Kind::Swift, parts[0], b':')[0], bind_all, name)
                })
            }),
    )
}
fn swift_shorthand(clause: &str, name: &str) -> bool {
    let c = clause.trim();
    c.strip_prefix("let ")
        .or_else(|| c.strip_prefix("var "))
        .is_some_and(|n| n.trim().trim_matches('`') == name)
}
fn swift_captures(list: &str, name: &str) -> bool {
    name != "self"
        && list.split(',').any(|entry| {
            let e = entry.trim();
            let e = ["weak ", "unowned(safe) ", "unowned(unsafe) ", "unowned "]
                .iter()
                .find_map(|m| e.strip_prefix(m))
                .unwrap_or(e);
            e.split('=').next().unwrap_or(e).trim().trim_matches('`') == name
        })
}
pub fn swift_rebinds(line: &str, name: &str) -> bool {
    static CAPTURES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\{\s*\[([^\]]*)\][^{}]*?\bin\b").expect("the regex is a literal")
    });
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\}\s*)?(?:else\s+)?(?:if|while|guard)\s+").expect("the regex is a literal")
    });
    let t = uncommented(Kind::Swift, line);
    let t = t.trim();
    if CAPTURES
        .captures_iter(t)
        .any(|c| !c[1].contains('=') && swift_captures(&c[1], name))
    {
        return true;
    }
    let t = KEYWORD.find(t).and_then(|m| t.get(m.end()..)).unwrap_or(t);
    let t = t.split(" else").next().unwrap_or(t);
    let t = t.trim_end().trim_end_matches('{');
    split_top(Kind::Swift, t, b',')
        .iter()
        .any(|c| swift_shorthand(c, name))
}
fn swift_decl_binds(past_keyword: &str, name: &str) -> Binds {
    Binds::first_of(
        split_top(Kind::Swift, past_keyword, b',')
            .into_iter()
            .map(|part| {
                let pattern =
                    split_top(Kind::Swift, split_top(Kind::Swift, part, b'=')[0], b':')[0];
                swift_pattern_binds(pattern, true, name)
            }),
    )
}
pub(super) fn swift_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    swift_walk(lines, at, name).unwrap_or_default()
}
pub fn swift_no_local(text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    line.checked_sub(1)
        .filter(|&at| at < lines.len())
        .is_some_and(|at| swift_walk(&lines, at, name).is_some_and(|b| b.is_empty()))
}
fn swift_text(lines: &[&str], i: usize) -> String {
    uncommented(Kind::Swift, lines[i]).trim().to_owned()
}
fn swift_line1_writing(lines: &[&str], start: usize, end: usize, re: &Regex) -> usize {
    (start..=end)
        .find(|&j| re.is_match(&swift_text(lines, j)))
        .unwrap_or(start)
        + 1
}
fn swift_local_decls(
    lines: &[&str],
    literal: &[bool],
    at: usize,
    name: &str,
    open: usize,
    depth: usize,
) -> Option<Vec<Binding>> {
    let found: Vec<usize> = (open + 1..lines.len())
        .filter(|&j| swift_code(lines, literal, j))
        .take_while(|&j| indent(lines[j]) >= depth)
        .filter(|&j| indent(lines[j]) == depth && swift_local_decl(lines[j], name))
        .collect();
    let bindings = found.iter().map(|&j| Binding {
        line1: j + 1,
        value: Value::Unknown,
    });
    Some(bindings.collect::<Vec<_>>()).filter(|b| !b.is_empty() && !found.contains(&at))
}
fn swift_statement_binds(
    lines: &[&str],
    at: usize,
    i: usize,
    name: &str,
    word: &Regex,
) -> (Binds, usize) {
    let t = swift_text(lines, i);
    if let Some(c) = SWIFT_DECL.captures(&t) {
        (swift_decl_binds(&c[1], name), i + 1)
    } else if t == "guard" || t.starts_with("guard ") {
        let guard_end = (i..at)
            .find(|&j| swift_text(lines, j).contains("else"))
            .unwrap_or(i);
        let text: Vec<String> = (i..=guard_end).map(|j| swift_text(lines, j)).collect();
        (
            swift_condition_binds(&text.join(" "), name),
            swift_line1_writing(lines, i, guard_end, word),
        )
    } else {
        (Binds::Nothing, 0)
    }
}
fn swift_in_a_type_body(lines: &[&str], literal: &[bool], i: usize) -> bool {
    (0..i)
        .rev()
        .find(|&j| swift_code(lines, literal, j) && indent(lines[j]) < indent(lines[i]))
        .map(|j| swift_text(lines, swift_header_start(lines, literal, j)))
        .is_some_and(|h| !SWIFT_FUNC.is_match(&h) && SWIFT_TYPE.is_match(&h))
}
fn swift_walk(lines: &[&str], at: usize, name: &str) -> Option<Vec<Binding>> {
    let literal = literal_lines(Kind::Swift, &lines.join("\n"));
    let code = |i: usize| swift_text(lines, i);
    let found = |line: usize| {
        Some(vec![Binding {
            line1: line,
            value: Value::Unknown,
        }])
    };
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let (word, param) = (
        rule(format!(r"(?:^|[^\w.]){n}\b")),
        rule(format!(r"\b{n}\s*:")),
    );
    let generic = |text: &str| swift_generics(text).iter().any(|g| g == name);
    if generic(&code(at)) {
        return found(at + 1);
    }
    let mut depth = indent(lines[at]);
    let mut bound_in_block_unless_type_body = None;
    let mut only_type_header_left = false;
    let mut unproven = false;
    let mut i = at;
    while i > 0 && depth > 0 {
        i -= 1;
        if !swift_code(lines, &literal, i) || indent(lines[i]) > depth {
            continue;
        }
        if indent(lines[i]) == depth {
            if bound_in_block_unless_type_body.is_some() || only_type_header_left {
                continue;
            }
            let (binds, line) = swift_statement_binds(lines, at, i, name, &word);
            match binds {
                Binds::UnreadablePatternMentionsName => return None,
                Binds::Name => bound_in_block_unless_type_body = Some(line),
                Binds::Nothing => {}
            }
            continue;
        }
        let body = depth;
        depth = indent(lines[i]);
        let end = i;
        let local = swift_local_decls(lines, &literal, at, name, end, body);
        i = swift_header_start(lines, &literal, i);
        let head = code(i);
        let text = (i..=end).map(code).collect::<Vec<_>>().join(" ");
        if only_type_header_left {
            return if generic(&text) {
                found(i + 1)
            } else {
                (!unproven).then(Vec::new)
            };
        }
        if SWIFT_FUNC.is_match(&head) {
            if let Some(line) = bound_in_block_unless_type_body {
                return found(line);
            }
            if local.is_some() {
                return local;
            }
            let params_end = (end..at).find(|&j| code(j).ends_with('{')).unwrap_or(end);
            let text = (i..=params_end).map(code).collect::<Vec<_>>().join(" ");
            if swift_params_bind(&text, name) {
                return found(swift_line1_writing(lines, i, params_end, &param));
            }
            if generic(&text) {
                return found(i + 1);
            }
            only_type_header_left = swift_in_a_type_body(lines, &literal, i);
            unproven |= !only_type_header_left;
            continue;
        }
        if SWIFT_TYPE.is_match(&head) {
            return if generic(&text) {
                found(i + 1)
            } else {
                (!unproven).then(Vec::new)
            };
        }
        if let Some(line) = bound_in_block_unless_type_body {
            return found(line);
        }
        if local.is_some() && !head.starts_with("where ") {
            return local;
        }
        match swift_header_binds(&head, &text, &code(end), name) {
            Binds::UnreadablePatternMentionsName => return None,
            Binds::Name if head.starts_with("case ") => return found(i + 1),
            Binds::Name => return found(swift_line1_writing(lines, i, end, &word)),
            Binds::Nothing => {}
        }
    }
    None
}
fn swift_params_bind(text: &str, name: &str) -> bool {
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"{}(?:func|init|subscript|set)\b", swift_mods!()))
            .expect("the regex is a literal")
    });
    let mods = PARAMS.find(text).map_or(text.len(), |m| m.end());
    let params = text.get(mods..).and_then(|t| t.find('(')).and_then(|open| {
        let open = mods + open;
        close_of(Kind::Swift, text, open).and_then(|close| text.get(open + 1..close - 1))
    });
    params.is_some_and(|p| {
        split_top(Kind::Swift, p, b',').iter().any(|p| {
            let names = split_top(Kind::Swift, p, b':')[0];
            names
                .split_whitespace()
                .last()
                .is_some_and(|w| w.trim_matches('`') == name)
        })
    })
}
fn swift_header_binds(first_line: &str, joined: &str, opening_line: &str, name: &str) -> Binds {
    static CATCH: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\}\s*)?catch\b\s*(.*?)\s*\{$").expect("the regex is a literal")
    });
    static CONDITION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\}\s*)?(?:else\s+)?(?:if|while)\s").expect("the regex is a literal")
    });
    static FOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^for\s+(?:try\s+)?(?:await\s+)?(?:case\s+)?(.+?)\s+in\s")
            .expect("the regex is a literal")
    });
    static CLOSURE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"\{\s*(?:\[([^\]]*)\]\s*)?(?:@\w+\s+)?(\([^()]*\)|[\w\s,]*?)\s*(?:async\s+)?(?:throws\s+)?(?:->\s*[^{}]+?)?\s*\bin$",
        )
        .expect("the regex is a literal")
    });
    if let Some(c) = CATCH.captures(first_line) {
        let n = regex::escape(name);
        return Binds::name_if(match &c[1] {
            "" => name == "error",
            pattern => Regex::new(&format!(r"\b(?:let|var)\s+{n}\b"))
                .expect("an escaped name keeps the pattern valid")
                .is_match(pattern),
        });
    }
    if let Some(pattern) = first_line.strip_prefix("case ") {
        let pattern = pattern.strip_suffix(':').unwrap_or(pattern);
        return Binds::first_of(
            split_top(Kind::Swift, pattern, b',')
                .into_iter()
                .map(|p| swift_pattern_binds(p, false, name)),
        );
    }
    if let Some(c) = FOR.captures(joined) {
        swift_pattern_binds(&c[1], true, name)
    } else if CONDITION.is_match(first_line) {
        swift_condition_binds(joined, name)
    } else if let Some(c) = CLOSURE.captures(opening_line) {
        Binds::name_if(
            swift_closure_params_bind(&c[2], name)
                || c.get(1).is_some_and(|l| swift_captures(l.as_str(), name)),
        )
    } else {
        Binds::Nothing
    }
}
fn swift_closure_params_bind(params: &str, name: &str) -> bool {
    let params = params.trim().trim_start_matches('(').trim_end_matches(')');
    params.split(',').any(|p| {
        let p = p.split(':').next().unwrap_or(p);
        p.split_whitespace().last() == Some(name)
    })
}
pub fn swift_closure_param(line: &str, name: &str, at: usize) -> bool {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:\[([^\]]*)\]\s*)?(?:@\w+\s+)?(\([^()]*\)|[\w\s,]*?)\s*(?:async\s+)?(?:throws\s+)?(?:->\s*[^{}]+?)?\s*\bin\s",
        )
        .expect("the regex is a literal")
    });
    let mut open = Vec::new();
    for (i, c) in code(Kind::Swift, line).take_while(|&(i, _)| i < at) {
        match c {
            b'{' => open.push(i),
            b'}' => {
                open.pop();
            }
            _ => {}
        }
    }
    open.iter().any(|&o| {
        line.get(o + 1..at)
            .and_then(|s| HEAD.captures(s))
            .is_some_and(|c| {
                swift_closure_params_bind(&c[2], name)
                    || c.get(1).is_some_and(|l| swift_captures(l.as_str(), name))
            })
    })
}
pub fn swift_binds_on(line: &str, name: &str) -> bool {
    let t = uncommented(Kind::Swift, line).trim().to_owned();
    if SWIFT_FUNC.is_match(&t) && swift_params_bind(&t, name) {
        return true;
    }
    let binds = if t.starts_with("guard ") {
        swift_condition_binds(&t, name)
    } else {
        swift_header_binds(&t, &t, &t, name)
    };
    binds == Binds::Name
}
static SWIFT_TYPE_DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(&format!(
        r"{}(?:class|struct|enum|actor|protocol|typealias)\s+`?\w",
        swift_mods!()
    ))
    .expect("the regex is a literal")
});
pub fn swift_file_private(line: &str) -> bool {
    static DECL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"({})(?:func|let|var|class|struct|enum|actor|protocol|typealias|init|subscript)\b",
            swift_mods!()
        ))
        .expect("the regex is a literal")
    });
    DECL.captures(&uncommented(Kind::Swift, line))
        .is_some_and(|c| {
            c[1].split_whitespace()
                .any(|w| w == "private" || w == "fileprivate")
        })
}
pub fn swift_test_dirs(root: &Path) -> Vec<PathBuf> {
    static FIELD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"\b(name|path)\s*:\s*"([^"]*)""#).expect("the regex is a literal")
    });
    let Ok(manifest) = std::fs::read_to_string(root.join("Package.swift")) else {
        return vec![PathBuf::from("Tests")];
    };
    manifest
        .match_indices(".testTarget(")
        .filter_map(|(at, m)| {
            let open = at + m.len() - 1;
            let args = manifest.get(open..close_of(Kind::Swift, &manifest, open)?)?;
            let field = |key: &str| {
                FIELD
                    .captures_iter(args)
                    .find(|c| &c[1] == key)
                    .map(|c| c[2].trim_start_matches("./").to_owned())
            };
            match field("path") {
                Some(path) => Some(PathBuf::from(path)),
                None => field("name").map(|n| Path::new("Tests").join(n)),
            }
        })
        .collect()
}
pub fn swift_type_decl(line: &str) -> bool {
    SWIFT_TYPE_DECL.is_match(&uncommented(Kind::Swift, line))
}
pub fn swift_func_decl(line: &str) -> bool {
    SWIFT_FUNC.is_match(&uncommented(Kind::Swift, line))
}
#[derive(Debug, PartialEq, Eq)]
pub enum SwiftTypePlace {
    FunctionBody { header_line1: usize },
    Nested(String),
}
pub fn swift_type_place(text: &str, line1: usize) -> Option<SwiftTypePlace> {
    let lines: Vec<&str> = text.lines().collect();
    let decl = lines.get(line1.checked_sub(1)?)?;
    if !swift_type_decl(decl) && !swift_func_decl(decl) {
        return None;
    }
    let literal = literal_lines(Kind::Swift, text);
    let SwiftScope {
        header_line1: scope,
        local,
    } = swift_scope(&lines, &literal, line1);
    let header = uncommented(Kind::Swift, lines.get(scope.checked_sub(1)?)?);
    match local {
        true => SWIFT_FUNC
            .is_match(&header)
            .then_some(SwiftTypePlace::FunctionBody {
                header_line1: scope,
            }),
        false
            if !swift_type_header(lines[line1 - 1]).is_some_and(|h| {
                matches!(h.keyword.as_str(), "class" | "struct" | "enum" | "actor")
            }) =>
        {
            None
        }
        false => swift_type_header(&header).and_then(|h| {
            Some(SwiftTypePlace::Nested(
                h.name.rsplit('.').next()?.to_owned(),
            ))
        }),
    }
}
pub fn swift_within<S: AsRef<str>>(
    lines: &[S],
    literal: &[bool],
    line1: usize,
    header_line1: usize,
) -> bool {
    let mut at = line1;
    loop {
        at = swift_scope(lines, literal, at).header_line1;
        if at == header_line1 {
            return true;
        }
        if at == 0 {
            return false;
        }
    }
}
pub fn swift_sees_nested<S: AsRef<str>>(
    lines: &[S],
    literal: &[bool],
    line1: usize,
    outer: &str,
) -> bool {
    let mut at = line1;
    while let Some(header) = swift_enclosing_type(lines, literal, at) {
        if let Some(SwiftTypeHeader {
            keyword,
            name,
            first_inherited: base,
            ..
        }) = swift_type_header(lines[header - 1].as_ref())
            && (name.split('.').any(|n| n == outer)
                || (keyword == "class"
                    && base.is_some_and(|b| b.rsplit('.').next() == Some(outer))))
        {
            return true;
        }
        at = header;
    }
    false
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwiftGiven {
    Type(String),
    Value(String),
    Element(String),
    Item(String, usize),
}
pub fn swift_given(line: &str, name: &str) -> Option<SwiftGiven> {
    static INDEXED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\[\s*\d+\s*\]$")
            .expect("the regex is a literal")
    });
    let t = uncommented(Kind::Swift, line);
    let t = t.trim();
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let each = rule(format!(
        r"^for\s+(?:case\s+)?(?:(?:let|var)\s+)?`?{n}`?\s+in\s+(.+?)\s*(?:where\s.*)?\{{?\s*$"
    ));
    if let Some(c) = each.captures(t) {
        return Some(SwiftGiven::Element(c[1].trim().to_owned()));
    }
    let enumerated = rule(format!(
        r"^for\s+(?:case\s+)?(?:let\s+)?\(\s*\w+\s*,\s*`?{n}`?\s*\)\s+in\s+([A-Za-z_][\w.]*)\.enumerated\(\)\s*\{{?\s*$"
    ));
    if let Some(c) = enumerated.captures(t) {
        return Some(SwiftGiven::Element(c[1].to_owned()));
    }
    let tuple = rule(r"^(?:let|var)\s+\(([\w\s,`]+)\)\s*=\s*([A-Za-z_][\w.]*)$".to_owned());
    if let Some(c) = tuple.captures(t) {
        let at = c[1]
            .split(',')
            .position(|p| p.trim().trim_matches('`') == name)?;
        return Some(SwiftGiven::Item(c[2].to_owned(), at));
    }
    let at = rule(format!(r"(?:^|[^\w.])`?{n}`?\s*([:=])"));
    let c = at.captures_iter(t).find(|c| {
        t.get(c.get_match().end()..)
            .is_some_and(|r| !r.starts_with([':', '=']))
    })?;
    let rest = t.get(c.get_match().end()..)?;
    if &c[1] == ":" {
        let ty = rest.get(..annotation_end(rest))?.trim();
        return (!ty.is_empty()).then(|| SwiftGiven::Type(ty.to_owned()));
    }
    let rest = rest.trim();
    let brace_opens_condition_body = ["if ", "guard ", "while ", "} else if "]
        .iter()
        .any(|k| t.starts_with(k));
    let rest = if brace_opens_condition_body {
        rest.strip_suffix('{')
            .map_or(rest, str::trim_end)
            .trim_end_matches(" else")
            .trim_end()
    } else {
        rest
    };
    let value = split_top(Kind::Swift, super::swift_before_else(rest), b',')[0].trim();
    if let Some(c) = INDEXED.captures(value) {
        return Some(SwiftGiven::Element(c[1].to_owned()));
    }
    (!value.is_empty()).then(|| SwiftGiven::Value(value.to_owned()))
}
fn annotation_end(rest: &str) -> usize {
    let mut depth = 0i32;
    for (i, ch) in rest.char_indices() {
        match ch {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' if depth > 0 => depth -= 1,
            ')' | ',' | '=' | '{' if depth == 0 => return i,
            _ => {}
        }
    }
    rest.len()
}
pub fn swift_type_name(written: &str) -> Option<String> {
    let mut t = written.trim();
    t = t.strip_prefix("inout ").unwrap_or(t).trim_start();
    while let Some(rest) = t.strip_prefix('@') {
        t = rest.split_once(' ')?.1.trim_start();
    }
    let t = t.trim_end_matches(['?', '!']);
    let t = match t.find('<') {
        Some(open) if t.ends_with('>') => t.get(..open)?,
        Some(_) => return None,
        None => t,
    };
    (t.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && t.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !matches!(t, "Self" | "Any" | "AnyObject" | "any" | "some"))
    .then(|| t.to_owned())
}
pub fn swift_nested_type_name(written: &str) -> Option<String> {
    let t = written.trim().trim_end_matches(['?', '!']);
    let (outer, name) = t.rsplit_once('.')?;
    outer
        .split('.')
        .chain([name])
        .all(|p| swift_type_name(p).is_some_and(|n| n.starts_with(char::is_uppercase)))
        .then(|| t.to_owned())
}
pub fn swift_renamed_types(written: &str, mut to: impl FnMut(&str) -> Option<String>) -> String {
    static BARE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(^|[^\w.])([A-Z]\w*)").expect("the regex is a literal")
    });
    BARE.replace_all(written, |c: &regex::Captures| {
        format!("{}{}", &c[1], to(&c[2]).unwrap_or_else(|| c[2].to_owned()))
    })
    .into_owned()
}
pub fn swift_opens_more(line: &str) -> bool {
    let depth = uncommented(Kind::Swift, line)
        .chars()
        .fold(0i32, |d, c| match c {
            '(' | '[' => d + 1,
            ')' | ']' => d - 1,
            _ => d,
        });
    depth > 0
}
pub fn swift_closure_sequence(lead: &str) -> Option<String> {
    static EACH: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.)\]?!])([A-Za-z_][\w.]*)\.(?:map|compactMap|flatMap|filter|forEach|allSatisfy|(?:contains|first|last)\(\s*where:|(?:sorted|min|max)\(\s*by:)\s*\{[^{}]*\$0\.[\w.]*$")
            .expect("the regex is a literal")
    });
    Some(EACH.captures(&uncommented(Kind::Swift, lead))?[1].to_owned())
}
pub fn swift_trailing_closure(line: &str, name: &str) -> Option<(String, String)> {
    let call = r"(?:^|[^\w.)\]?!])([A-Za-z_][\w.]*)\.([A-Za-z_]\w*)\s*\{";
    let pattern = if name == "0" {
        format!(r"{call}[^{{}}]*\$0\.[\w.]*$")
    } else {
        let n = regex::escape(name);
        format!(r"{call}\s*(?:\[[^\]]*\]\s*)?`?{n}`?\s+in\b")
    };
    let code = uncommented(Kind::Swift, line);
    let c = Regex::new(&pattern)
        .expect("an escaped name keeps the pattern valid")
        .captures(&code)?;
    Some((c[1].to_owned(), c[2].to_owned()))
}
pub fn swift_closure_input(decl: &str) -> Option<String> {
    static TAKES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r":\s*(?:@\w+\s+)*\(\s*(?:inout\s+)?([A-Za-z_]\w*)\s*\)\s*(?:async\s+)?(?:(?:re)?throws\s+)?->")
            .expect("the regex is a literal")
    });
    let found: Vec<String> = TAKES
        .captures_iter(&uncommented(Kind::Swift, decl))
        .map(|c| c[1].to_owned())
        .collect();
    match found.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}
pub fn swift_sole_argument(e: &str) -> Option<String> {
    let e = e.trim();
    let open = e.find('(')?;
    let close = close_of(Kind::Swift, e, open)?;
    let inner = e.get(open + 1..close.checked_sub(1)?)?.trim();
    let unlabeled = !Regex::new(r"^`?\w+`?\s*:")
        .expect("the regex is a literal")
        .is_match(inner);
    (e.get(close..)?.trim().is_empty()
        && !inner.is_empty()
        && unlabeled
        && split_top(Kind::Swift, inner, b',').len() == 1)
        .then(|| inner.to_owned())
}
pub fn swift_init_takes(decl: &str, param: &str) -> bool {
    let p = regex::escape(param);
    Regex::new(&format!(r"\binit\s*\(\s*_\s+\w+\s*:\s*{p}\s*\)"))
        .expect("an escaped name keeps the pattern valid")
        .is_match(&uncommented(Kind::Swift, decl))
}
pub fn swift_generic_arguments(written: &str) -> Vec<String> {
    let t = written.trim().trim_end_matches(['?', '!']);
    t.find('<')
        .and_then(|open| t.strip_suffix('>')?.get(open + 1..))
        .map(|inner| {
            split_top(Kind::Swift, inner, b',')
                .into_iter()
                .map(|a| a.trim().to_owned())
                .collect()
        })
        .unwrap_or_default()
}
pub fn swift_cast_receiver(lead: &str) -> Option<String> {
    static CAST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.)\]?!>])\(\s*[A-Za-z_][\w.]*\s+as[?!]\s+([A-Z]\w*)\s*\)[?!]?\.$")
            .expect("the regex is a literal")
    });
    Some(CAST.captures(&uncommented(Kind::Swift, lead))?[1].to_owned())
}
pub fn swift_tuple_item(written: &str, at: usize) -> Option<String> {
    let inner = written.trim().strip_prefix('(')?.strip_suffix(')')?;
    let items = split_top(Kind::Swift, inner, b',');
    let item = items.get(at).filter(|_| items.len() > 1)?.trim();
    let item = match item.split_once(':') {
        Some((label, t))
            if label
                .trim()
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_') =>
        {
            t
        }
        _ => item,
    };
    Some(item.trim().to_owned())
}
pub fn swift_element(written: &str) -> Option<String> {
    let t = written.trim().trim_end_matches(['?', '!']);
    let inner = match t.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        Some(inner) => inner,
        None => ["Array<", "Set<", "ContiguousArray<", "ArraySlice<"]
            .iter()
            .find_map(|p| t.strip_prefix(p))?
            .strip_suffix('>')?,
    };
    (split_top(Kind::Swift, inner, b':').len() == 1).then(|| inner.trim().to_owned())
}
pub fn swift_dictionary_part(written: &str, member: &str) -> Option<String> {
    let t = written.trim().trim_end_matches(['?', '!']);
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    let [key, value] = <[&str; 2]>::try_from(split_top(Kind::Swift, inner, b':')).ok()?;
    match member {
        "keys" => Some(key.trim().to_owned()),
        "values" => Some(value.trim().to_owned()),
        _ => None,
    }
}
pub fn swift_generics(line: &str) -> Vec<String> {
    static GENERICS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:\bfunc\s+`?[^\s(<]+`?|\binit[?!]?|\bsubscript|\b(?:class|struct|enum|actor|typealias)\s+`?\w+`?)\s*<([^>]*)>",
        )
        .expect("the regex is a literal")
    });
    GENERICS
        .captures(line)
        .map(|c| {
            c[1].split(',')
                .filter_map(|p| p.split(':').next())
                .map(|p| p.trim().to_owned())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
pub fn swift_returns(text: &str, line1: usize) -> Option<String> {
    let mut header = String::new();
    for l in text.lines().skip(line1.checked_sub(1)?).take(16) {
        let l = uncommented(Kind::Swift, l);
        match l.split_once('{') {
            Some((head, _)) => {
                header.push_str(head);
                break;
            }
            None => header.push_str(&l),
        }
        header.push(' ');
    }
    let mut depth = 0i32;
    let arrow_outside_params = header.char_indices().find_map(|(i, c)| {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' => depth -= 1,
            '>' if depth > 0 && header.get(..i).is_some_and(|h| !h.ends_with('-')) => depth -= 1,
            '-' if depth == 0 && header.get(i..).is_some_and(|r| r.starts_with("->")) => {
                return Some(i);
            }
            _ => {}
        }
        None
    })?;
    let ret = header.get(arrow_outside_params + 2..)?.trim();
    let ret = ret.split(" where ").next().unwrap_or(ret).trim();
    (!ret.is_empty()).then(|| ret.to_owned())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwiftExpr {
    Call(String),
    Chain(String),
    Cast(String),
}
pub fn swift_expr(e: &str) -> Option<SwiftExpr> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([A-Za-z_]\w*(?:[?!]?\.[A-Za-z_]\w*)*)\s*(?:<[^()]*>)?\s*([({])?")
            .expect("the regex is a literal")
    });
    static CAST: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\sas[!?]?\s+([\w<>\[\]?]+)$").expect("the regex is a literal")
    });
    let mut e = e.trim();
    loop {
        let rest = ["try? ", "try! ", "try ", "await "]
            .iter()
            .find_map(|p| e.strip_prefix(p));
        match rest {
            Some(rest) => e = rest.trim_start(),
            None => break,
        }
    }
    if let Some(c) = CAST.captures(e) {
        return Some(SwiftExpr::Cast(c[1].to_owned()));
    }
    let c = CALL.captures(e)?;
    let callee = c[1].replace(['?', '!'], "");
    let Some(open) = c.get(2) else {
        return (c.get_match().end() == e.len()).then_some(SwiftExpr::Chain(callee));
    };
    let mut at = open.start();
    while let Some(end) = close_of(Kind::Swift, e, at) {
        let rest = e.get(end..)?.trim_start();
        if rest.is_empty() {
            break;
        }
        if !rest.starts_with('{') || e.as_bytes()[at] == b'{' {
            return None;
        }
        at = e.len() - rest.len();
    }
    Some(SwiftExpr::Call(callee))
}
pub fn swift_may_bind(line: &str, name: &str) -> bool {
    let n = regex::escape(name);
    Regex::new(&format!(
        r"\b(?:let|var)\s[^=]*\b{n}\b|\{{[^{{}}]*\b{n}\b[^{{}}]*\bin\b|\bfor\b[^{{]*\b{n}\b[^{{]*\bin\b"
    ))
    .expect("an escaped name keeps the pattern valid")
    .is_match(&uncommented(Kind::Swift, line))
}
pub fn swift_called_closure(lead: &str) -> Option<String> {
    static CALLED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`])([a-z_]\w*)\(\s*$").expect("the regex is a literal")
    });
    CALLED.captures(lead).map(|c| c[1].to_owned())
}
pub fn swift_closure_takes(written: &str) -> Option<String> {
    let mut t = written.trim();
    while let Some(rest) = t.strip_prefix('@') {
        let name_end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
        t = rest.get(name_end..)?.trim_start();
    }
    let t = t.strip_prefix('(').unwrap_or("");
    let close = t.find(')')?;
    let (inner, rest) = (t.get(..close)?.trim(), t.get(close + 1..)?.trim_start());
    let rest = rest.strip_prefix("async").map_or(rest, str::trim_start);
    let rest = rest.strip_prefix("throws").map_or(rest, str::trim_start);
    (rest.starts_with("->") && !inner.is_empty() && !inner.contains([',', '(', ':']))
        .then(|| inner.to_owned())
}
pub fn swift_expected(
    lines: &[&str],
    literal: &[bool],
    line1: usize,
    lead: &str,
) -> Option<String> {
    static ANNOTATED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,\s])`?\w+`?\s*:\s*([A-Z]\w*(?:\.[A-Z]\w*)*)\s*[?!]?\s*=\s*$")
            .expect("the regex is a literal")
    });
    static RETURN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:guard\s[^{]*\selse\s*\{\s*)?return\s*$")
            .expect("the regex is a literal")
    });
    static ARM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:case\s.*|default)\s*:\s*$").expect("the regex is a literal")
    });
    static VAR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"{}var\s+`?\w+`?\s*:\s*([A-Z]\w*(?:\.[A-Z]\w*)*)\s*\{{\s*$",
            swift_mods!()
        ))
        .expect("the regex is a literal")
    });
    let lead = uncommented(Kind::Swift, lead);
    if let Some(c) = ANNOTATED.captures(&lead) {
        return Some(c[1].to_owned());
    }
    let switch = |t: &str| t.starts_with("switch ") || t.starts_with("return switch ");
    let mut i = line1.checked_sub(1)?;
    if ARM.is_match(&lead) {
        let depth = indent(lines[i]);
        i = (0..i).rev().find(|&j| {
            swift_code(lines, literal, j) && indent(lines[j]) <= depth && {
                let t = swift_text(lines, j);
                !(indent(lines[j]) == depth && (t.starts_with("case ") || t.starts_with("default")))
            }
        })?;
        if !switch(&swift_text(lines, i)) {
            return None;
        }
    } else if !RETURN.is_match(&lead) {
        return None;
    }
    loop {
        let depth = indent(lines[i]);
        let j = (0..i)
            .rev()
            .find(|&j| swift_code(lines, literal, j) && indent(lines[j]) < depth)?;
        i = swift_header_start(lines, literal, j);
        let t = swift_text(lines, i);
        let flow = [
            "if ", "guard ", "else", "} else", "for ", "while ", "do ", "do {", "catch", "} catch",
            "repeat", "case ", "default", "get {", "get{",
        ];
        if switch(&t) || flow.iter().any(|f| t.starts_with(f)) {
            continue;
        }
        if let Some(c) = VAR.captures(&t) {
            return Some(c[1].to_owned());
        }
        let text = lines.get(i..)?.join("\n");
        return (SWIFT_FUNC.is_match(&t) && t.contains("func "))
            .then(|| swift_returns(&text, 1))
            .flatten()
            .filter(|r| {
                r.trim_end_matches(['?', '!']).split('.').all(|p| {
                    p.starts_with(char::is_uppercase)
                        && p.chars().all(|c| c.is_alphanumeric() || c == '_')
                })
            })
            .map(|r| r.trim_end_matches(['?', '!']).to_owned());
    }
}
