use std::ops::Range;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const NAME: &str = r"[A-Za-z_][\w']*";
macro_rules! hs_args {
    () => {
        r#"(?:\s+[~!]?(?:[a-z_][\w']*@)?[\w'.]+|\s*[~!]?(?:[a-z_][\w']*@)?(?:\((?:[^()]|\([^()]*\))*\)|\[[^\]]*\]|\{[^}]*\}|"[^"]*"))*"#
    };
}
macro_rules! hs_equation_end {
    () => {
        r"\s*(?:=(?:[^=>]|$)|\|(?:[^|]|$)|$)"
    };
}
const ARGS: &str = hs_args!();
const EQUATION_END: &str = hs_equation_end!();
const CONTEXT: &str = r"(?:(?:\([^)]*\)|[A-Z][\w'.]*(?:\s+[\w']+)*)\s*=>\s*)?";
pub const HASKELL_RESERVED: &[&str] = &[
    "module", "import", "data", "newtype", "type", "class", "instance", "deriving", "infix",
    "infixl", "infixr", "foreign", "default", "pattern", "where", "let", "in", "if", "then",
    "else", "case", "of", "do",
];

fn hs_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'\''
}

fn symbol_byte(c: u8) -> bool {
    b"!#$%&*+./<=>?@\\^|-~:".contains(&c)
}

fn code(l: &str) -> bool {
    let t = l.trim();
    !t.is_empty() && !t.starts_with("--") && !t.starts_with("{-")
}

fn top(l: &str) -> bool {
    !l.starts_with([' ', '\t'])
}

fn keyword(l: &str) -> &str {
    l.split_whitespace().next().unwrap_or("")
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

fn owner<S: AsRef<str>>(lines: &[S], i: usize) -> Option<usize> {
    (0..=i).rev().find(|&j| {
        let l = lines[j].as_ref();
        code(l) && top(l)
    })
}

fn has_word(s: &str, name: &str) -> bool {
    s.match_indices(name).any(|(i, _)| {
        s[..i].matches('"').count().is_multiple_of(2)
            && !s[..i].bytes().next_back().is_some_and(hs_name_byte)
            && !s
                .as_bytes()
                .get(i + name.len())
                .copied()
                .is_some_and(hs_name_byte)
    })
}

fn starts_with_name(t: &str, name: &str) -> bool {
    t.strip_prefix(name)
        .is_some_and(|rest| !rest.bytes().next().is_some_and(hs_name_byte))
}

pub fn haskell_name(line: &str, r: Range<usize>) -> Range<usize> {
    if line[..r.start].ends_with('\'') {
        return r;
    }
    let primes = line.as_bytes()[r.end..]
        .iter()
        .take_while(|&&c| c == b'\'')
        .count();
    r.start..r.end + primes
}

fn signature_pattern(w: &str, lead: &str) -> String {
    format!(r"{lead}(?:{NAME}\s*,\s*)*{w}\s*(?:,\s*{NAME}\s*)*::")
}

pub fn haskell_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    if word.starts_with(|c: char| c.is_ascii_uppercase()) {
        return vec![
            format!(r"^(?:data|newtype|type)(?:\s+family)?\s+{CONTEXT}{w}(?:[^\w']|$)"),
            format!(r"^class\s+{CONTEXT}{w}(?:[^\w']|$)"),
            format!(r"^pattern\s+{w}(?:\s[^:]*)?\s*(?:=|<-)"),
            format!(r"^(?:data|newtype)\s[^=]*=\s*(?:.*\|\s*)?{w}(?:[^\w']|$)"),
            format!(r"^\s+[=|]\s*{w}(?:[^\w']|$)"),
            signature_pattern(&w, r"^\s+"),
        ];
    }
    vec![
        format!(r"^{w}{ARGS}{EQUATION_END}"),
        signature_pattern(&w, r"^\s*"),
        signature_pattern(&w, r"[{,]\s*"),
    ]
}

fn signed_names(lines: &[String], j: usize) -> Vec<&str> {
    static SIGNATURE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^((?:[a-z_][\w']*\s*,\s*)*[a-z_][\w']*)\s*::").unwrap());
    let l = &lines[j];
    if let Some(c) = SIGNATURE.captures(l) {
        let names = c.get(1).map_or("", |m| m.as_str());
        return names.split(',').map(str::trim).collect();
    }
    let alone = l.trim_end();
    let next = lines[j + 1..].iter().find(|l| code(l));
    match next {
        Some(n) if !top(n) && n.trim_start().starts_with("::") && is_name(alone) => vec![alone],
        _ => Vec::new(),
    }
}

fn is_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') && s.bytes().all(hs_name_byte)
}

fn uncommented(l: &str) -> &str {
    let b = l.as_bytes();
    let mut string = false;
    for i in 0..b.len() {
        match b[i] {
            b'"' if !(i > 0 && b[i - 1] == b'\\') => string = !string,
            _ if string => {}
            b'{' if b.get(i + 1) == Some(&b'-') => return &l[..i],
            b'-' if line_comment(b, i) => return &l[..i],
            _ => {}
        }
    }
    l
}

fn line_comment(b: &[u8], i: usize) -> bool {
    b[i..].starts_with(b"--")
        && !(i > 0 && symbol_byte(b[i - 1]))
        && !b[i..]
            .iter()
            .find(|&&c| c != b'-')
            .is_some_and(|&c| symbol_byte(c))
}

pub struct HaskellCode {
    lines: Vec<String>,
    signed: std::collections::HashSet<String>,
}

impl HaskellCode {
    pub fn new<S: AsRef<str>>(lines: &[S]) -> Self {
        let text: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
        let literal = haskell_literal_lines(&text.join("\n"));
        let lines: Vec<String> = text
            .iter()
            .enumerate()
            .map(
                |(i, l)| match literal.get(i) == Some(&true) || l.starts_with('#') {
                    true => String::new(),
                    false => uncommented(l).to_owned(),
                },
            )
            .collect();
        let signed = (0..lines.len())
            .filter(|&j| top(&lines[j]))
            .flat_map(|j| signed_names(&lines, j))
            .map(str::to_owned)
            .collect();
        Self { lines, signed }
    }

    fn closed(&self, i: usize) -> bool {
        match equation(&self.lines[i]) {
            Some((_, _, true)) => self.lines[i + 1..]
                .iter()
                .find(|l| code(l))
                .is_some_and(|n| !top(n) && n.trim_start().starts_with(['|', '='])),
            _ => true,
        }
    }

    pub fn declares(&self, line: usize, word: &str) -> bool {
        let lines = &self.lines;
        let Some(i) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
            return false;
        };
        let l = &lines[i];
        if !code(l) {
            return false;
        }
        if top(l) {
            return match keyword(l) {
                "data" | "newtype" | "type" | "class" | "pattern" => true,
                k if HASKELL_RESERVED.contains(&k) => false,
                _ if signed_names(lines, i).contains(&word) => true,
                _ => {
                    starts_with_name(l, word)
                        && !self.signed.contains(word)
                        && self.closed(i)
                        && !(0..i)
                            .rev()
                            .map(|j| &lines[j])
                            .find(|l| code(l) && top(l))
                            .is_some_and(|l| starts_with_name(l, word))
                }
            };
        }
        let Some(o) = owner(lines, i) else {
            return false;
        };
        match keyword(&lines[o]) {
            "data" | "newtype" => {
                let t = l.trim_start();
                !(t.starts_with(['=', '|'])
                    && t.find(word)
                        .map(|at| &t[at + word.len()..])
                        .is_some_and(|rest| rest.contains('=') || rest.contains("<-")))
            }
            "class" => directly_inside(lines, line, "class"),
            _ => false,
        }
    }
}

pub fn haskell_declares<S: AsRef<str>>(lines: &[S], line: usize, word: &str) -> bool {
    HaskellCode::new(lines).declares(line, word)
}

pub fn haskell_symbol(code: &HaskellCode, line: usize, name: &str) -> bool {
    !HASKELL_RESERVED.contains(&name) && code.declares(line, name)
}

pub fn haskell_whole(line: &str, word: &str) -> Option<usize> {
    let b = line.as_bytes();
    line.match_indices(word).map(|(i, _)| i).find(|&i| {
        let quotes = b[..i].iter().rev().take_while(|&&c| c == b'\'').count();
        let start = i - quotes;
        let before = start == 0 || !hs_name_byte(b[start - 1]);
        before && !b.get(i + word.len()).copied().is_some_and(hs_name_byte)
    })
}

pub(super) const HASKELL_SIGNATURE_SYMBOL: &str =
    r"^(?P<name>[a-z_][\w']*)\s*(?:,\s*[a-z_][\w']*\s*)*::";
pub(super) const HASKELL_EQUATION_SYMBOL: &str =
    concat!(r"^(?P<name>[a-z_][\w']*)", hs_args!(), hs_equation_end!());
pub(super) const HASKELL_TYPE_SYMBOL: &str = concat!(
    r"^(?:(?:data|newtype|type)(?:\s+family)?|class)\s+",
    r"(?:(?:\([^)]*\)|[A-Z][\w'.]*(?:\s+[\w']+)*)\s*=>\s*)?(?P<name>[A-Z][\w']*)"
);
pub(super) const HASKELL_PATTERN_SYMBOL: &str =
    r"^pattern\s+(?P<name>[A-Z][\w']*)(?:\s[^:]*)?\s*(?:=|<-)";

fn equation(t: &str) -> Option<(&str, &str, bool)> {
    static EQ: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"^(?P<name>[a-z_][\w']*)(?P<args>{ARGS}){EQUATION_END}"
        ))
        .unwrap()
    });
    let c = EQ.captures(t)?;
    let args = c.name("args")?;
    let open = t[args.end()..].trim().is_empty();
    Some((c.name("name")?.as_str(), args.as_str(), open))
}

fn local_text(l: &str) -> &str {
    let mut t = l.trim_start();
    loop {
        match ["where ", "let "].iter().find_map(|k| t.strip_prefix(k)) {
            Some(rest) => t = rest.trim_start(),
            None => return t,
        }
    }
}

fn next_code<'a>(lines: &[&'a str], j: usize) -> Option<&'a str> {
    lines[j + 1..].iter().copied().find(|l| code(l))
}

fn local_equation<'a>(lines: &[&'a str], j: usize) -> Option<(&'a str, &'a str)> {
    let (name, args, open) = equation(local_text(lines[j]))?;
    let guarded = next_code(lines, j)
        .is_some_and(|n| indent(n) > indent(lines[j]) && n.trim_start().starts_with(['|', '=']));
    (!open || guarded).then_some((name, args))
}

fn covers(lines: &[&str], j: usize, at: usize) -> bool {
    j <= at
        && lines[j + 1..=at]
            .iter()
            .all(|l| !code(l) || indent(l) > indent(lines[j]))
}

fn arrow_pattern(t: &str) -> Option<&str> {
    let lhs = &t[..t.find("->")?];
    let lhs = match (lhs.rfind('\\'), lhs.rfind(" of ")) {
        (Some(a), Some(b)) if a > b => &lhs[a + 1..],
        (_, Some(b)) => &lhs[b + 4..],
        (Some(a), None) => &lhs[a + 1..],
        (None, None) => lhs,
    };
    let refused = ["=", "::", "\"", "<-", "|"].iter().any(|s| lhs.contains(s));
    (!refused && !lhs.trim().is_empty()).then_some(lhs)
}

pub(super) fn haskell_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let blanked = HaskellCode::new(lines);
    let lines: Vec<&str> = blanked.lines.iter().map(String::as_str).collect();
    let lines = &lines[..];
    let Some(start) = owner(lines, at).filter(|_| !name.starts_with(|c: char| c.is_uppercase()))
    else {
        return Vec::new();
    };
    let end = (at + 1..lines.len())
        .find(|&j| code(lines[j]) && top(lines[j]))
        .unwrap_or(lines.len());
    let mut found: Vec<(usize, bool)> = Vec::new();
    let opener = keyword(lines[start]);
    let methods = matches!(opener, "class" | "instance");
    if HASKELL_RESERVED.contains(&opener) && !methods {
        return Vec::new();
    }
    if equation(lines[start]).is_some_and(|(_, args, _)| has_word(args, name)) {
        found.push((start, false));
    }
    let method_indent = (start + 1..end)
        .find(|&j| code(lines[j]))
        .map(|j| indent(lines[j]));
    let signature = Regex::new(&signature_pattern(&regex::escape(name), "^")).ok();
    for j in start + 1..end {
        if !code(lines[j]) {
            continue;
        }
        let t = local_text(lines[j]);
        let eq = local_equation(lines, j);
        let method = methods && Some(indent(lines[j])) == method_indent;
        if signature.as_ref().is_some_and(|re| re.is_match(t)) {
            if !method {
                found.push((j, true));
            }
            continue;
        }
        let bound = |lhs: &str| has_word(lhs, name);
        let named = eq.is_some_and(|(n, _)| n == name && !method);
        let parameter = eq.is_some_and(|(_, args)| bound(args)) && covers(lines, j, at);
        let drawn = t
            .find("<-")
            .map(|p| &t[..p])
            .is_some_and(|lhs| j <= at && !lhs.contains(['=', '"', '|', '[']) && bound(lhs));
        let alternative = arrow_pattern(t).is_some_and(bound) && covers(lines, j, at);
        if named || parameter || drawn || alternative {
            found.push((j, false));
        }
    }
    if found.is_empty() {
        return Vec::new();
    }
    let mut pick = found
        .iter()
        .rfind(|(j, _)| *j <= at)
        .or_else(|| found.first())
        .map(|&(j, _)| j)
        .expect("not empty");
    while let Some(prev) = (0..pick)
        .rev()
        .find(|&j| code(lines[j]) && indent(lines[j]) <= indent(lines[pick]))
        .filter(|&j| j > start && indent(lines[j]) == indent(lines[pick]))
        .filter(|&j| local_equation(lines, j).is_some_and(|(n, _)| n == name))
        .filter(|_| local_equation(lines, pick).is_some_and(|(n, _)| n == name))
    {
        pick = prev;
    }
    let pick = match local_equation(lines, pick) {
        Some((n, _)) if n == name => found
            .iter()
            .find(|&&(j, sig)| sig && indent(lines[j]) == indent(lines[pick]))
            .map_or(pick, |&(j, _)| j),
        _ => pick,
    };
    vec![Binding {
        line1: pick + 1,
        value: Value::Unknown,
    }]
}

pub struct HaskellImport {
    pub module: String,
    pub alias: Option<String>,
    pub names: Vec<String>,
    pub qualified: bool,
}

pub fn haskell_imports(text: &str) -> Vec<HaskellImport> {
    let mut joined: Vec<String> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for l in &HaskellCode::new(&lines).lines {
        if l.starts_with("import ") {
            joined.push(l.to_owned());
        } else if let Some(last) = joined.last_mut()
            && !top(l)
            && !l.trim().is_empty()
            && (!last.contains('(') || last.matches('(').count() > last.matches(')').count())
        {
            last.push(' ');
            last.push_str(l.trim());
        }
    }
    joined.iter().filter_map(|l| parse_import(l)).collect()
}

fn parse_import(l: &str) -> Option<HaskellImport> {
    let head = l.find('(').map_or(l, |p| &l[..p]);
    let mut words = head.split_whitespace().skip(1).filter(|w| {
        !matches!(*w, "safe" | "qualified" | "{-#" | "SOURCE" | "#-}") && !w.starts_with('"')
    });
    let module = words.next()?.to_owned();
    let qualified = head.split_whitespace().any(|w| w == "qualified");
    let mut alias = None;
    let mut hiding = false;
    while let Some(w) = words.next() {
        match w {
            "as" => alias = words.next().map(str::to_owned),
            "hiding" => hiding = true,
            _ => {}
        }
    }
    let mut names = Vec::new();
    if let (false, Some(open)) = (hiding, l.find('(')) {
        let mut depth = 0;
        let mut item = String::new();
        let mut items = Vec::new();
        for c in l[open + 1..].chars() {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => break,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    items.push(std::mem::take(&mut item));
                    continue;
                }
                _ => {}
            }
            item.push(c);
        }
        items.push(item);
        for item in items {
            let item = item.trim();
            let item = ["type ", "pattern "]
                .iter()
                .find_map(|k| item.strip_prefix(k))
                .unwrap_or(item);
            let (head, inner) = item.split_once('(').unwrap_or((item, ""));
            names.extend(
                std::iter::once(head)
                    .chain(inner.trim_end_matches(')').split(','))
                    .map(str::trim)
                    .filter(|n| n.starts_with(|c: char| c.is_alphabetic() || c == '_'))
                    .map(str::to_owned),
            );
        }
    }
    Some(HaskellImport {
        module,
        alias,
        names,
        qualified,
    })
}

pub fn haskell_import_module(line: &str, col: usize) -> Option<String> {
    static IMPORT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^import\s+(?:safe\s+)?(?:\{-#\s*SOURCE\s*#-\}\s+)?(?:qualified\s+)?(?:"[^"]*"\s+)?([A-Z][\w'.]*)"#).unwrap()
    });
    let m = IMPORT.captures(line)?.get(1)?;
    (m.start() <= col && col < m.end()).then(|| m.as_str().to_owned())
}

pub fn haskell_files(module: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    let file = PathBuf::from(format!("{}.hs", module.replace('.', "/")));
    files
        .iter()
        .filter(|f| f.ends_with(&file))
        .cloned()
        .collect()
}

fn char_literal(b: &[u8]) -> Option<usize> {
    match b.get(1)? {
        b'\\' => b
            .iter()
            .skip(2)
            .take(10)
            .position(|&c| c == b'\'')
            .map(|n| n + 3),
        &c => {
            let len = match c {
                c if c < 0x80 => 1,
                c if c >= 0xF0 => 4,
                c if c >= 0xE0 => 3,
                _ => 2,
            };
            (b.get(1 + len) == Some(&b'\'')).then_some(2 + len)
        }
    }
}

fn quasi_quote(b: &[u8]) -> Option<usize> {
    let n = b[1..]
        .iter()
        .take_while(|&&c| hs_name_byte(c) || c == b'.')
        .count();
    let opens = n > 0
        && b[1].is_ascii_alphabetic()
        && b.get(1 + n) == Some(&b'|')
        && b.get(2 + n) != Some(&b'|');
    (opens && b.windows(2).any(|w| w == b"|]")).then_some(n + 2)
}

pub(super) fn haskell_literal_lines(text: &str) -> Vec<bool> {
    let b = text.as_bytes();
    let mut out = vec![false];
    let (mut depth, mut quasi, mut string) = (0usize, false, false);
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            string = false;
            out.push(depth > 0 || quasi);
        } else if depth > 0 {
            if b[i..].starts_with(b"{-") {
                depth += 1;
                i += 1;
            } else if b[i..].starts_with(b"-}") {
                depth -= 1;
                i += 1;
            }
        } else if quasi {
            if b[i..].starts_with(b"|]") {
                quasi = false;
                i += 1;
            }
        } else if string {
            if c == b'\\' && b.get(i + 1).is_some_and(|&n| n != b'\n') {
                i += 1;
            } else if c == b'"' {
                string = false;
            }
        } else if b[i..].starts_with(b"{-") {
            depth = 1;
            i += 1;
        } else if line_comment(b, i) {
            i += b[i..]
                .iter()
                .position(|&c| c == b'\n')
                .unwrap_or(b.len() - i);
            continue;
        } else if c == b'"' {
            string = true;
        } else if c == b'\'' && !(i > 0 && hs_name_byte(b[i - 1])) {
            if let Some(n) = char_literal(&b[i..]) {
                i += n;
                continue;
            }
        } else if c == b'['
            && let Some(n) = quasi_quote(&b[i..])
        {
            quasi = true;
            i += n;
            continue;
        }
        i += 1;
    }
    out
}
