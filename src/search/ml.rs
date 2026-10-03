use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const END: &str = r"(?:[^\w']|$)";

pub fn ml_patterns(kind: Kind, word: &str) -> Vec<String> {
    if word == "_" {
        return Vec::new();
    }
    let w = match kind {
        Kind::Fsharp => format!("(?:``{0}``|{0})", regex::escape(word)),
        _ => regex::escape(word),
    };
    let capital = word.starts_with(|c: char| c.is_ascii_uppercase());
    let ocaml = kind == Kind::Ocaml;
    let attr = r"(?:\[<[^\]]*>\]\s*)?";
    let mut patterns = vec![match ocaml {
        true => format!(r"^\s*(?:let|and)(?:%[\w.]+|[*+])?\s+(?:rec\s+)?{w}{END}"),
        false => format!(
            r"^\s*{attr}(?:let|and)\s+(?:(?:rec|private|internal|public|inline|mutable)\s+)*{w}{END}"
        ),
    }];
    patterns.push(format!(r"^\s*exception\s+{w}{END}"));
    patterns.push(format!(r"^\s*val\s+(?:mutable\s+)?{w}\s*[:=]"));
    patterns.push(format!(
        r"^\s*(?:\{{\s*)?(?:(?:mutable\s+)?[\w']+\s*:[^;{{}}=]*;\s*)*(?:mutable\s+)?{w}\s*:(?:[^=]|$)"
    ));
    patterns.push(format!(
        r"^\s*{attr}(?:type|and)\s[^=]*=\s*(?:private\s+)?\{{\s*(?:(?:mutable\s+)?[\w']+\s*:[^;{{}}]*;\s*)*(?:mutable\s+)?{w}\s*:"
    ));
    if capital {
        patterns.push(format!(
            r"^\s*{attr}(?:type|and)\s[^=]*=\s*(?:private\s+)?(?:\|?\s*[A-Z][\w']*(?:\s+of\b[^|]*)?\s*\|\s*)*\|?\s*{w}(?:\s+of\b|\s*\||\s*$|\s*\(\*|\s*//)"
        ));
        patterns.push(format!(
            r"^\s*\|\s*{w}(?:\s*:.*|(?:[^\w'\-]|-[^>])(?:[^-]|-[^>])*)?$"
        ));
    }
    match ocaml {
        true => patterns.extend([
            format!(r"^\s*external\s+{w}\s*:"),
            format!(r"^\s*(?:type|and)\s+(?:nonrec\s+)?(?:'\w+\s+|\([^)]*\)\s+)?{w}{END}"),
            format!(r"^\s*module\s+(?:rec\s+)?(?:type\s+)?{w}{END}"),
            format!(r"^\s*(?:class|and)\s+(?:type\s+)?(?:virtual\s+)?(?:\[[^\]]*\]\s*)?{w}{END}"),
            format!(r"^\s*method\s+(?:(?:private|virtual)\s+|!\s*)*{w}{END}"),
        ]),
        false => patterns.extend([
            format!(r"^\s*{attr}(?:type|and)\s+(?:(?:private|internal|public)\s+)?{w}{END}"),
            format!(
                r"^\s*{attr}(?:(?:static|abstract|override|default|private|internal|public|inline)\s+)*(?:member|override|default|abstract)\s+(?:(?:val|private|internal|public|inline)\s+)*(?:[\w']+\.)?{w}{END}"
            ),
            format!(
                r"^\s*module\s+(?:(?:rec|private|internal|public)\s+)*(?:[\w.]+\.)?{w}\s*(?:=|$)"
            ),
            format!(r"^\s*namespace\s+(?:(?:rec|global)\s+)?(?:[\w.]+\.)?{w}\s*$"),
        ]),
    }
    patterns
}

pub(super) const OCAML_SYMBOL: &str = concat!(
    r"^\s*(?:let(?:\s+rec)?|and|type(?:\s+nonrec)?|module(?:\s+type)?|exception|external|class)",
    r"\s+(?:'\w+\s+|\([^)]*\)\s+)?(?P<name>[A-Za-z][\w']*|_[\w']+)"
);

pub(super) const FSHARP_SYMBOL: &str = concat!(
    r"^\s*(?:\[<[^\]]*>\]\s*)?(?:(?:let|and)(?:\s+(?:rec|private|internal|public|inline|mutable))*",
    r"|type(?:\s+(?:private|internal|public))?|module|exception",
    r"|(?:(?:static|abstract|override|default|private|internal|public|inline)\s+)*",
    r"(?:member|override|default|abstract)(?:\s+(?:val|private|internal|public|inline))*)",
    r"\s+(?:[\w']+\.)?(?P<name>[A-Za-z][\w']*|_[\w']+)"
);

pub fn ml_signature(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "mli" || e == "fsi")
}

pub fn ml_implementation(path: &Path) -> Option<PathBuf> {
    let ext = match path.extension()?.to_str()? {
        "mli" => "ml",
        "fsi" => "fs",
        _ => return None,
    };
    Some(path.with_extension(ext))
}

pub fn ml_symbol_kept(kind: Kind, path: &Path, lines: &[&str], line: usize) -> bool {
    let Some(text) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let t = text.trim_start();
    let binding = t.starts_with("let") || t.starts_with("and") || t.starts_with("[<");
    !(kind == Kind::Ocaml && ml_signature(path)) && (!binding || module_level(kind, lines, line))
}

pub fn ml_name(line: &str, r: Range<usize>, col: usize) -> Range<usize> {
    let mut at = 0;
    while let Some(open) = line[at..].find("``").map(|n| at + n) {
        let Some(close) = line[open + 2..].find("``").map(|n| open + 2 + n) else {
            break;
        };
        if (open..close + 2).contains(&col) && close > open + 2 {
            return open + 2..close;
        }
        at = close + 2;
    }
    let primes = line.as_bytes()[r.end..]
        .iter()
        .take_while(|&&c| c == b'\'')
        .count();
    r.start..r.end + primes
}

fn keyword_line(t: &str) -> bool {
    static KEYWORD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:\[<|\||(?:let|and|type|module|namespace|val|exception|external|class|method|member|override|default|abstract|static|inline|private|internal|public|open|include|inherit)\b)").unwrap()
    });
    KEYWORD.is_match(t)
}

fn binding_line(t: &str) -> bool {
    static BINDING: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:\[<[^\]]*>\]\s*)?(?:let|and)(?:[\s%!*+]|$)").unwrap());
    BINDING.is_match(t)
}

fn module_level<S: AsRef<str>>(kind: Kind, lines: &[S], line: usize) -> bool {
    static FS_MODULE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*module\s+[\w.']+\s*=\s*$").unwrap());
    let Some(target) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let depth = indent(target.as_ref());
    if depth == 0 {
        return true;
    }
    let opener = lines[..line - 1].iter().map(AsRef::as_ref).rev().find(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("(*") && !t.starts_with("//") && indent(l) < depth
    });
    opener.is_some_and(|l| match kind {
        Kind::Ocaml => {
            let code = l.split("(*").next().unwrap_or(l).trim_end();
            code.ends_with("struct") && !code.ends_with("_struct")
        }
        _ => FS_MODULE.is_match(l),
    })
}

fn in_braces<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    let mut depth = 0usize;
    for l in lines[..line.saturating_sub(1).min(lines.len())]
        .iter()
        .rev()
        .take(200)
    {
        for c in l.as_ref().bytes().rev() {
            match c {
                b'}' => depth += 1,
                b'{' if depth == 0 => return true,
                b'{' => depth -= 1,
                _ => {}
            }
        }
    }
    false
}

pub fn ml_declares<S: AsRef<str>>(kind: Kind, lines: &[S], line: usize, _word: &str) -> bool {
    let Some(text) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    let t = text.as_ref().trim_start();
    let t = t.strip_prefix('{').map_or(t, str::trim_start);
    if binding_line(t) && !t.starts_with("and") {
        return module_level(kind, lines, line);
    }
    if t.starts_with("and") {
        return indent(text.as_ref()) == 0 || module_level(kind, lines, line);
    }
    keyword_line(t) || in_braces(lines, line)
}

pub fn ml_local<S: AsRef<str>>(kind: Kind, lines: &[S], line: usize, word: &str) -> Option<usize> {
    let pattern = ml_patterns(kind, word).into_iter().next()?;
    let re = Regex::new(&pattern).expect("an escaped name keeps the pattern valid");
    let text = lines
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<&str>>()
        .join("\n");
    let literal = ml_literal_lines(kind, &text);
    for i in (0..line.min(lines.len())).rev() {
        let l = lines[i].as_ref();
        if literal.get(i) == Some(&true) || l.trim().is_empty() {
            continue;
        }
        let t = l.trim_start();
        if re.is_match(l) {
            return (!module_level(kind, lines, i + 1)).then_some(i + 1);
        }
        let top = indent(l) == 0 && !t.starts_with("(*") && !t.starts_with("//");
        if top || (binding_line(t) && module_level(kind, lines, i + 1)) {
            return None;
        }
    }
    None
}

pub fn ml_modules<S: AsRef<str>>(path: &Path, lines: &[S], line: usize) -> Vec<String> {
    static OPENER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:\[<[^\]]*>\]\s*)?(?:module|type|and)\s+(?:(?:rec|nonrec|private|internal|public)\s+)*(?:[\w']+\.)*([A-Z][\w']*)")
            .unwrap()
    });
    let mut names: Vec<String> = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            let mut c = s.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .into_iter()
        .collect();
    let mut depth = line
        .checked_sub(1)
        .and_then(|i| lines.get(i))
        .map_or(0, |l| indent(l.as_ref()));
    if depth == 0 {
        return names;
    }
    for l in lines[..line.saturating_sub(1).min(lines.len())]
        .iter()
        .rev()
    {
        let l = l.as_ref();
        let i = indent(l);
        if l.trim().is_empty() || i >= depth {
            continue;
        }
        if let Some(c) = OPENER.captures(l) {
            names.push(c[1].to_owned());
        }
        depth = i;
        if i == 0 {
            match l.starts_with("end") {
                true => depth = 1,
                false => break,
            }
        }
    }
    names
}

pub(super) fn ocaml_roots(where_: Option<String>) -> Vec<PathBuf> {
    let Some(dir) = where_.map(|w| PathBuf::from(w.trim())) else {
        return Vec::new();
    };
    let parent = dir.parent().map(Path::to_path_buf);
    std::iter::once(dir).chain(parent).collect()
}

enum State {
    Code,
    Comment(usize),
    Str { comment: usize, verbatim: bool },
    Triple,
    Quoted(String),
}

pub fn ml_literal_lines(kind: Kind, text: &str) -> Vec<bool> {
    let fsharp = kind == Kind::Fsharp;
    let b = text.as_bytes();
    let mut literal = vec![false];
    let mut state = State::Code;
    let name = |c: u8| c.is_ascii_alphanumeric() || c == b'_' || c == b'\'';
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let at = |s: &[u8]| b[i..].starts_with(s);
        if c == b'\n' {
            literal.push(!matches!(state, State::Code));
            i += 1;
            continue;
        }
        match &mut state {
            State::Code => {
                if fsharp && at(b"(*)") {
                    i += 3;
                } else if at(b"(*") {
                    state = State::Comment(1);
                    i += 2;
                } else if fsharp && at(b"//") {
                    i = b[i..]
                        .iter()
                        .position(|&c| c == b'\n')
                        .map_or(b.len(), |n| i + n);
                } else if fsharp && at(b"``") {
                    let line_end = b[i..]
                        .iter()
                        .position(|&c| c == b'\n')
                        .map_or(b.len(), |n| i + n);
                    i = text[i + 2..line_end]
                        .find("``")
                        .map_or(i + 2, |n| i + 2 + n + 2);
                } else if fsharp && (at(b"\"\"\"") || at(b"$\"\"\"")) {
                    state = State::Triple;
                    i += if c == b'$' { 4 } else { 3 };
                } else if c == b'"'
                    || (fsharp && (at(b"@\"") || at(b"$\"") || at(b"$@\"") || at(b"@$\"")))
                {
                    let verbatim = fsharp
                        && b[i..]
                            .iter()
                            .take_while(|&&c| c != b'"')
                            .any(|&c| c == b'@');
                    let quote = b[i..].iter().position(|&c| c == b'"').unwrap_or(0);
                    state = State::Str {
                        comment: 0,
                        verbatim,
                    };
                    i += quote + 1;
                } else if !fsharp && c == b'{' {
                    let id: String = text[i + 1..]
                        .chars()
                        .take_while(|c| c.is_ascii_lowercase() || *c == '_')
                        .collect();
                    if b.get(i + 1 + id.len()) == Some(&b'|') {
                        i += id.len() + 2;
                        state = State::Quoted(id);
                    } else {
                        i += 1;
                    }
                } else if c == b'\'' && !(i > 0 && name(b[i - 1])) {
                    i += match (b.get(i + 1), b.get(i + 2)) {
                        (Some(b'\\'), _) => b[i + 2..]
                            .iter()
                            .position(|&c| c == b'\'' || c == b'\n')
                            .map_or(1, |n| n + 3),
                        (Some(&n), Some(b'\'')) if n != b'\n' => 3,
                        _ => 1,
                    };
                } else {
                    i += 1;
                }
            }
            State::Comment(depth) => {
                if at(b"(*") {
                    *depth += 1;
                    i += 2;
                } else if at(b"*)") {
                    *depth -= 1;
                    i += 2;
                    if *depth == 0 {
                        state = State::Code;
                    }
                } else if c == b'"' {
                    let comment = *depth;
                    state = State::Str {
                        comment,
                        verbatim: false,
                    };
                    i += 1;
                } else {
                    i += 1;
                }
            }
            State::Str { comment, verbatim } => {
                let back = |comment: usize| match comment {
                    0 => State::Code,
                    d => State::Comment(d),
                };
                let escaped = match *verbatim {
                    true => at(b"\"\""),
                    false => c == b'\\' && b.get(i + 1).is_some_and(|&n| n != b'\n'),
                };
                if escaped {
                    i += 2;
                } else if c == b'"' {
                    state = back(*comment);
                    i += 1;
                } else {
                    i += 1;
                }
            }
            State::Triple => {
                if at(b"\"\"\"") {
                    state = State::Code;
                    i += 3;
                } else {
                    i += 1;
                }
            }
            State::Quoted(id) => {
                let close = format!("|{id}}}");
                if at(close.as_bytes()) {
                    i += close.len();
                    state = State::Code;
                } else {
                    i += 1;
                }
            }
        }
    }
    literal
}
