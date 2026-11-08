use std::path::{Path, PathBuf};

use regex::Regex;

use super::{Binding, Value, indent, lexical};

pub(super) fn shell_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    shell_function_at(lines, at).map_or_else(Vec::new, |f| {
        (f + 1..=at)
            .filter(|&i| shell_local_of(lines[i]).is_some_and(|names| names.contains(&name)))
            .map(|i| Binding {
                line1: i + 1,
                value: Value::Unknown,
            })
            .collect()
    })
}
pub fn shell_assigned(lines: &[&str], literal: &[bool], at: usize, name: &str) -> Option<usize> {
    let assign = Regex::new(&format!(
        r"^\s*(?:(?:export|readonly|declare|typeset)(?:\s+-\w+)*\s+)?{}=",
        regex::escape(name)
    ))
    .ok()?;
    let branch = |l: &str| {
        let t = l.trim_start();
        t == "else" || t.starts_with("else ") || t.starts_with("elif ")
    };
    let fi_at = |l: &str, depth: usize| {
        let t = l.trim();
        indent(l) == depth && (t == "fi" || t.starts_with("fi ") || t.starts_with("fi;"))
    };
    let sibling = |i: usize| {
        (i + 1..at).any(|j| {
            let depth = indent(lines[j]);
            branch(lines[j])
                && depth < indent(lines[i])
                && (0..i)
                    .rev()
                    .find(|&k| {
                        indent(lines[k]) == depth && lines[k].trim_start().starts_with("if ")
                    })
                    .is_some_and(|k| !lines[k + 1..j].iter().any(|l| fi_at(l, depth)))
                && !lines[j + 1..=at].iter().any(|l| fi_at(l, depth))
        })
    };
    let arm_closed = |i: usize| {
        let (mut depth, mut closed) = (0, false);
        let mut loops = (0..i)
            .filter(|&j| literal.get(j) != Some(&true))
            .fold(0_usize, |d, j| {
                (d + DO.find_iter(lines[j]).count())
                    .saturating_sub(DONE.find_iter(lines[j]).count())
            });
        for (j, l) in lines.iter().enumerate().take(at).skip(i) {
            let t = l.trim();
            if literal.get(j) == Some(&true) {
                continue;
            }
            loops = (loops + DO.find_iter(l).count()).saturating_sub(DONE.find_iter(l).count());
            let words = CASE_ESAC.find_iter(l).count();
            let exact = match words {
                0 => true,
                1 => {
                    (j > i && t.starts_with("case "))
                        || t == "esac"
                        || t.starts_with("esac ")
                        || t.starts_with("esac;")
                }
                _ => j > i && t.starts_with("case ") && t.ends_with("esac") && words == 2,
            };
            if !exact {
                return false;
            }
            if j > i && t.starts_with("case ") && !t.ends_with("esac") {
                depth += 1;
            } else if t == "esac" || t.starts_with("esac ") || t.starts_with("esac;") {
                if depth == 0 {
                    closed = false;
                } else {
                    depth -= 1;
                }
            } else if depth == 0 && t.ends_with(";;") {
                closed = true;
            }
        }
        closed && loops == 0
    };
    let assigns = |i: usize| {
        literal.get(i) != Some(&true)
            && ((assign.is_match(lines[i])
                && !(ASSIGN_WORD.is_match(lines[i].trim_start())
                    && shell_runs_a_command(lines[i])))
                || shell_loop_binds(lines[i], name) == Some(true))
            && !sibling(i)
            && !arm_closed(i)
    };
    let mine = shell_function_at(lines, at);
    mine.and_then(|f| (f + 1..at).rev().find(|&i| assigns(i)))
        .or_else(|| {
            (0..mine.unwrap_or(at))
                .rev()
                .find(|&i| assigns(i) && shell_function_at(lines, i).is_none())
        })
        .map(|i| i + 1)
}
static ASSIGN_WORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\w+(?:\[[^\]]*\])?\+?=").expect("the regex is a literal")
});
pub fn shell_assigns_word(tail: &str) -> bool {
    ASSIGN_WORD.is_match(&format!("x{tail}"))
}
fn shell_runs_a_command(segment: &str) -> bool {
    static REDIRECT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:\d+|&)?[<>]+[&|]?").expect("the regex is a literal")
    });
    let mut words = shell_words(segment).into_iter();
    while let Some(mut w) = words.next() {
        if w.starts_with('#') {
            return false;
        }
        if w == "&&" || w == "||" {
            return false;
        }
        if let Some(op) = REDIRECT.find(w) {
            if op.end() == w.len() {
                w = words.next().unwrap_or("");
            }
            if w.ends_with(';') {
                return false;
            }
            continue;
        }
        if w.starts_with(['&', '|']) {
            return true;
        }
        if !ASSIGN_WORD.is_match(w) {
            return true;
        }
        if w.ends_with(';') {
            return false;
        }
    }
    false
}
fn shell_segment_binds(segment: &str, name: &str) -> bool {
    let mut s = segment.trim_start();
    while let Some(rest) = ["do ", "then ", "else ", "! "]
        .iter()
        .find_map(|k| s.strip_prefix(k))
    {
        s = rest.trim_start();
    }
    if let Some(names) = shell_local_of(s) {
        return names.contains(&name);
    }
    if shell_loop_binds(s, name) == Some(true) {
        return true;
    }
    !shell_runs_a_command(s)
        && shell_words(s)
            .iter()
            .any(|w| ASSIGN_WORD.is_match(w) && w.split(['=', '[', '+']).next() == Some(name))
}
pub fn shell_binds_before(line: &str, col: usize, name: &str) -> bool {
    let b = line.as_bytes();
    let end = col.min(b.len());
    let (mut quote, mut depth, mut i) = (None, 0_i32, 0);
    let mut starts = vec![(0, 0_i32)];
    let mut seps: Vec<(usize, i32)> = Vec::new();
    let mut low = vec![0_i32; end + 1];
    while i < end {
        let (c, from) = (b[i], i);
        match (quote, c) {
            (Some(b'"') | None, b'\\') => i += 1,
            (Some(q), _) if c == q => quote = None,
            (None, b'"' | b'\'') => quote = Some(c),
            (None, b'#') if i == 0 || b[i - 1].is_ascii_whitespace() => return false,
            (None, b'{' | b'(') => {
                depth += 1;
                starts.push((i + 1, depth));
            }
            (None, b'}' | b')') => {
                depth -= 1;
                starts.push((i + 1, depth));
            }
            (None, b';') => {
                seps.push((i, depth));
                starts.push((i + 1, depth));
            }
            (None, b'&' | b'|') if b.get(i + 1) == Some(&c) => {
                seps.push((i, depth));
                starts.push((i + 2, depth));
                i += 1;
            }
            (None, b'&' | b'|') => starts.push((i + 1, i32::MIN)),
            _ => {}
        }
        i += 1;
        for l in low.iter_mut().take(i.min(end) + 1).skip(from + 1) {
            *l = depth;
        }
    }
    for k in (0..end).rev() {
        low[k] = low[k].min(low[k + 1]);
    }
    starts.iter().any(|&(s, d)| {
        let Some(&(sep, sd)) = seps.iter().find(|&&(p, _)| p >= s) else {
            return false;
        };
        d != i32::MIN
            && sd == d
            && low.get(s).is_some_and(|&l| l >= d)
            && line
                .get(s..sep)
                .is_some_and(|seg| shell_segment_binds(seg, name))
    })
}
static CASE_ESAC: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\b(?:case|esac)\b").expect("the regex is a literal"));
static DO: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?:^|;)\s*do\b").expect("the regex is a literal"));
static DONE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?:^|;)\s*done\b").expect("the regex is a literal"));
pub fn shell_source(line: &str, col: usize, bats: bool) -> Option<String> {
    static SOURCE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(load|source|\.)\s+(.*)").expect("the regex is a literal")
    });
    let c = SOURCE.captures(line)?;
    let (keyword, rest) = (c.get(1)?, c.get(2)?);
    let load = keyword.as_str() == "load";
    if load && !bats {
        return None;
    }
    let word = *shell_words(rest.as_str()).first()?;
    let start = rest.start() + rest.as_str().find(word)?;
    let unquoted = word.replace(['"', '\''], "");
    let mut path = unquoted.as_str();
    if path.starts_with('~') {
        return None;
    }
    while let Some(after) = path.strip_prefix('$') {
        let at = match after.as_bytes().first() {
            Some(b'(') => after.find(')')? + 1,
            Some(b'{') => after.find('}')? + 1,
            _ => (after.find(|c: char| !c.is_ascii_alphanumeric() && c != '_'))
                .unwrap_or(after.len()),
        };
        if after.get(..at)?.trim_matches(['{', '}']) == "HOME" {
            return None;
        }
        path = after.get(at..)?.strip_prefix('/')?;
    }
    if path.is_empty() || path.starts_with('/') || path.contains(['$', '`', '*']) {
        return None;
    }
    let literal = start + word.rfind(path)?;
    let on_keyword = !load && keyword.range().contains(&col);
    if !(on_keyword || (literal..=start + word.len()).contains(&col)) {
        return None;
    }
    if !load {
        return Some(path.to_owned());
    }
    let bash = if Path::new(path).extension().is_some_and(|e| e == "bash") {
        ""
    } else {
        ".bash"
    };
    Some(format!("./{path}{bash}"))
}
pub fn shell_source_from_here(line: &str) -> bool {
    ![
        "$(dirname \"$0\")",
        "$(dirname \"${BASH_SOURCE[0]}\")",
        "$(dirname \"$BASH_SOURCE\")",
        "${BASH_SOURCE%/*}",
        "${0%/*}",
    ]
    .iter()
    .fold(line.to_owned(), |l, here| l.replace(here, ""))
    .contains(['$', '`'])
}
pub fn shell_directive_source(line: &str) -> Option<&str> {
    static DIRECTIVE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*#\s*shellcheck\s(?:.*\s)?source=([\w./-]+)(?:\s|$)")
            .expect("the regex is a literal")
    });
    Some(DIRECTIVE.captures(line)?.get(1)?.as_str())
}
pub fn shell_files(dir: &Path, path: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    match lexical(&dir.join(path)).filter(|f| files.contains(f)) {
        Some(f) => vec![f],
        None => files
            .iter()
            .filter(|f| f.ends_with(path))
            .cloned()
            .collect(),
    }
}
pub fn shell_function_at(lines: &[&str], at: usize) -> Option<usize> {
    static ONE_LINE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\{.*[;\s]\}\s*(?:#.*)?$|\(\s*$").expect("the regex is a literal")
    });
    (0..at.min(lines.len())).rev().find(|&k| {
        let l = lines[k];
        (HEADER.is_match(l) || l.starts_with("@test "))
            && !ONE_LINE.is_match(l)
            && !lines[k + 1..=at]
                .iter()
                .any(|b| b.trim_start().starts_with('}') && indent(b) == indent(l))
    })
}
pub fn shell_local_of(line: &str) -> Option<Vec<&str>> {
    static LOCAL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:local|declare|typeset)((?:\s+-\w+)*)\s+(.*)")
            .expect("the regex is a literal")
    });
    let c = LOCAL.captures(line)?;
    if c[1].contains(['g', 'p', 'f', 'F']) {
        return None;
    }
    let rest = c.get(2).map_or("", |m| m.as_str());
    let names = shell_words(rest)
        .into_iter()
        .map(|w| w.split(['=', '+']).next().unwrap_or(""))
        .take_while(|n| {
            n.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .collect();
    Some(names)
}
pub fn shell_loop_binds(line: &str, name: &str) -> Option<bool> {
    static FOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:for|select)\s+(\w+)\s*(?:\bin\b|;|$)").expect("the regex is a literal")
    });
    static READ: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:while|until)\s+)?(?:\w+=\S*\s+)*(read|mapfile|readarray)\s+(.*)")
            .expect("the regex is a literal")
    });
    if let Some(c) = FOR.captures(line) {
        return Some(&c[1] == name);
    }
    let c = READ.captures(line)?;
    let read = &c[1] == "read";
    let takes = if read { "adinNptu" } else { "dnOsuCc" };
    let is_name = |n: &str| {
        n.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    let mut words = shell_words(c.get(2).map_or("", |m| m.as_str())).into_iter();
    while let Some(w) = words.next() {
        if let Some(flags) = w.strip_prefix('-') {
            if flags.ends_with(|f| takes.contains(f)) {
                let arg = words.next()?;
                if read && flags.ends_with('a') && arg == name {
                    return Some(true);
                }
            }
            continue;
        }
        let n = w.split([';', '|', '&', '<', '>']).next().unwrap_or("");
        if !is_name(n) {
            break;
        }
        if n == name {
            return Some(true);
        }
        if n.len() < w.len() || !read {
            break;
        }
    }
    Some(false)
}
#[expect(
    clippy::match_same_arms,
    reason = "the scan's arms are grouped by state, the quoted ones first; a merge would mix them"
)]
fn shell_words(s: &str) -> Vec<&str> {
    let (b, mut out, mut i) = (s.as_bytes(), Vec::new(), 0);
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let (start, mut depth, mut quote) = (i, 0usize, None);
        while i < b.len() && (quote.is_some() || depth > 0 || !b[i].is_ascii_whitespace()) {
            match (quote, b[i]) {
                (Some(b'"' | b'`') | None, b'\\') => i += 1,
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, c @ (b'"' | b'\'' | b'`')) => quote = Some(c),
                (None, b'$') if b.get(i + 1) == Some(&b'{') => {
                    depth += 1;
                    i += 1;
                }
                (None, b'(') => depth += 1,
                (None, b')' | b'}') if depth > 0 => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        out.extend(s.get(start..i.min(b.len())));
    }
    out
}
static HEADER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\s*(?:function\s+[^\s(){}]+|[\w.:-]+\s*\(\s*\))").expect("the regex is a literal")
});
pub fn shell_names_a_command(line: &str) -> bool {
    HEADER.is_match(line) || line.trim_start().starts_with("alias ")
}
pub fn shell_command_position(lead: &str) -> Option<bool> {
    let l = lead.trim_end();
    if ["$", "${", "${#", "${!"].iter().any(|s| lead.ends_with(s)) || l.ends_with("((") {
        return Some(false);
    }
    let last = l.rsplit(char::is_whitespace).next().unwrap_or("");
    let function_flag = last == "-f"
        && ["export", "unset", "declare", "typeset"].iter().any(|w| {
            l.strip_suffix("-f")
                .is_some_and(|r| r.trim_end().ends_with(w))
        });
    (l.is_empty()
        || function_flag
        || l.ends_with([';', '&', '|', '(', '{', '!'])
        || matches!(
            last,
            "then" | "do" | "else" | "elif" | "if" | "while" | "until"
        ))
    .then_some(true)
}
pub fn bats_runs(lead: &str) -> bool {
    let mut l = lead.trim_end();
    if l.len() == lead.len() {
        return false;
    }
    loop {
        let (rest, last) = l.rsplit_once([' ', '\t']).unwrap_or(("", l));
        if last == "run" {
            return shell_command_position(rest) == Some(true);
        }
        if !(last.starts_with('-') || last == "!") {
            return false;
        }
        l = rest.trim_end();
    }
}
pub fn shell_env_line(line: &str) -> bool {
    static ENV: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:exec\s+)?env\s+[-\w]").expect("the regex is a literal")
    });
    ENV.is_match(line)
}
pub fn shell_reads_another_file(line: &str) -> bool {
    static SOURCES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[\s;&|({])(?:source|\.|load|bats_load_library)\s+\S")
            .expect("the regex is a literal")
    });
    SOURCES.is_match(line)
}
pub fn shell_no_name(lines: &[String], at: usize, lead: &str, tail: &str) -> bool {
    let quoted = !lead.contains('"')
        && lead.matches('\'').count() % 2 == 1
        && shell_command_position(lead) == Some(false);
    let bypass = lead.ends_with([' ', '\t'])
        && ["command", "builtin"].iter().any(|w| {
            lead.trim_end()
                .strip_suffix(w)
                .is_some_and(|rest| shell_command_position(rest) == Some(true))
        });
    if quoted || bypass {
        return true;
    }
    tail.starts_with(')')
        && lead
            .trim_start()
            .chars()
            .all(|c| c.is_alphanumeric() || "_|*-.( ".contains(c))
        && lines[..at].iter().rev().map(|l| l.trim()).find_map(|l| {
            if l.starts_with("case ") && l.ends_with(" in") {
                Some(true)
            } else {
                (l == "esac" || l.starts_with("esac ")).then_some(false)
            }
        }) == Some(true)
}
pub fn shell_out_arg_pattern(word: &str) -> String {
    format!(
        r"^\s*[\w.:-]+\s+(?:\S+\s+)*{}(?:\s|;|$)",
        regex::escape(word)
    )
}
pub fn shell_out_arg<'a>(line: &'a str, word: &str) -> Option<(&'a str, usize)> {
    let words = shell_words(line);
    let (f, args) = words.split_first()?;
    let n = args.iter().position(|a| *a == word)?;
    (f.chars()
        .all(|c| c.is_ascii_alphanumeric() || "_.:-".contains(c))
        && !args[..n]
            .iter()
            .any(|a| a.contains(['@', '*', ';', '|', '&', '<', '>'])))
    .then_some((*f, n + 1))
}
pub fn shell_writes_param(lines: &[&str], def: usize, n: usize) -> bool {
    let Some(head) = lines.get(def) else {
        return false;
    };
    let body = if head.trim_end().ends_with('}') {
        std::slice::from_ref(head)
    } else {
        lines.get(def + 1..).unwrap_or_default()
    };
    let end = body
        .iter()
        .position(|b| b.trim_start().starts_with('}') && indent(b) == indent(head))
        .unwrap_or(body.len());
    let refs = |x: &str| format!(r#""?\$(?:{x}\b|\{{{x}\}})"?"#);
    let mut names = vec![n.to_string()];
    for l in &body[..end] {
        let alias = names.iter().find_map(|x| {
            Regex::new(&format!(
                r"^\s*(?:(?:local|declare|typeset|readonly)(?:\s+-\w+)*\s+)?(\w+)={}\s*(?:;|#|$)",
                refs(x)
            ))
            .ok()?
            .captures(l)
            .map(|c| c[1].to_owned())
        });
        names.extend(alias);
    }
    names.iter().any(|x| {
        let r = refs(x);
        Regex::new(&format!(
            r#"(?:^|[\s;&|(])(?:printf\s+-v\s+{r}\s|read(?:\s+(?:-[a-zA-Z]*[dtnNuip]\s+\S+|-[a-zA-Z]*[a-zA-Z&&[^dtnNuip]]|\w+))*\s+{r}(?:\s|;|<|$)|eval\s+"?\\?\$\{{?{x}\}}?=|(?:local|declare|typeset)\s+-[a-zA-Z]*n[a-zA-Z]*\s+\w+={r}(?:\s|;|$))"#
        ))
        .is_ok_and(|re| body[..end].iter().any(|l| re.is_match(l)))
    })
}
pub fn shell_function_name(line: &str) -> Option<&str> {
    let s = HEADER.find(line)?.as_str().trim_start();
    let s = s
        .strip_prefix("function")
        .filter(|r| r.starts_with(char::is_whitespace))
        .map_or(s, str::trim_start);
    s.split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .filter(|n| !n.is_empty())
}
fn shell_plain(line: &str) -> bool {
    const FLOW: &[&str] = &[
        "if", "then", "else", "elif", "fi", "case", "esac", "for", "select", "while", "until",
        "do", "done", "return", "exit", "exec", "break", "continue", "function", "trap",
    ];
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return true;
    }
    let first = t.split([' ', '\t', ';']).next().unwrap_or("");
    !FLOW.contains(&first)
        && !t.starts_with(['{', '}', '(', ')'])
        && !["&&", "||", ";;", "<<"].iter().any(|s| t.contains(s))
        && !t.ends_with(['\\', '{', '(', '|'])
        && shell_quotes_closed(t)
}
fn shell_quotes_closed(s: &str) -> bool {
    let mut quote = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some('\''), '\'') => quote = None,
            (Some('\''), _) => {}
            (_, '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            _ => {}
        }
    }
    quote.is_none()
}
pub fn shell_fails_after(lines: &[&str], at: usize, name: &str) -> bool {
    lines
        .get(at)
        .is_some_and(|l| shell_plain(l) && l.trim_start().starts_with(&format!("{name}=")))
        && lines[at + 1..]
            .iter()
            .find(|l| !shell_plain(l))
            .is_some_and(|l| {
                let mut w = l.split_whitespace();
                matches!(w.next(), Some("return" | "exit"))
                    && w.next()
                        .is_some_and(|n| n.parse::<u8>().is_ok_and(|n| n != 0))
                    && w.next().is_none_or(|c| c.starts_with('#'))
            })
}
pub fn shell_runs_first(lines: &[&str], def: usize, at: usize) -> bool {
    lines.get(def).is_some_and(|l| l.trim_end().ends_with('{'))
        && lines
            .get(def + 1..=at)
            .is_some_and(|b| b.iter().all(|l| shell_plain(l)))
}
pub fn shell_gated_by(line: &str, f: &str, g: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut any = false;
    for (i, _) in line.match_indices(f) {
        let (Some(before), Some(after)) = (line.get(..i), line.get(i + f.len()..)) else {
            continue;
        };
        if before.ends_with(word) || after.starts_with(word) {
            continue;
        }
        let gated = shell_quotes_closed(before)
            && before
                .trim_end()
                .strip_suffix("&&")
                .and_then(|b| b.trim_end().strip_suffix(g))
                .is_some_and(|b| {
                    let b = b.trim_end();
                    b.is_empty() || b.ends_with("&&")
                });
        if !gated {
            return false;
        }
        any = true;
    }
    any
}
pub(super) fn shell_text_string_end(b: &[u8], open: usize) -> Option<usize> {
    let q = b[open];
    let from = b[..open]
        .iter()
        .rposition(|&c| c == b'\n')
        .map_or(0, |n| n + 1);
    let lead = if q == b'"' {
        &b[from..open]
    } else {
        b[from..open].strip_suffix(b"$")?
    };
    let continued = from > 0 && b[..from - 1].last() == Some(&b'\\');
    if continued || !shell_text_lead(lead.trim_ascii_start()) {
        return None;
    }
    let mut i = open + 1;
    while let Some(&c) = b.get(i) {
        i = match c {
            b'\\' => i + 2,
            c if c == q => break,
            b'$' if q == b'"' && b.get(i + 1) == Some(&b'(') => shell_line_group_end(b, i + 1)?,
            b'$' if q == b'"' && b.get(i + 1) == Some(&b'{') => {
                let n = b[i..].iter().position(|&c| c == b'}')?;
                if b[i + 2..i + n].iter().any(|c| b"'\"`{\n$\\".contains(c)) {
                    return None;
                }
                i + n + 1
            }
            b'`' if q == b'"' => return None,
            _ => i + 1,
        };
    }
    let end = i + 1;
    let after = b.get(end).copied();
    let word_ends = after.is_none_or(|c| b" \t\r\n;|&)<>".contains(&c));
    (end <= b.len() && word_ends && b[open..end].contains(&b'\n')).then_some(end)
}
fn shell_text_lead(lead: &[u8]) -> bool {
    let assigned = |w: &[u8]| {
        w.strip_suffix(b"=")
            .map(|n| n.strip_suffix(b"+").unwrap_or(n))
            .is_some_and(|n| {
                n.first().is_some_and(|c| !c.is_ascii_digit()) && n.iter().all(|&c| ident(c))
            })
    };
    let mut quoted = false;
    for &c in lead {
        if c == b'\'' {
            quoted = !quoted;
        } else if !quoted && b";|&`()<>\"\\{}$#".contains(&c) {
            return false;
        }
    }
    let words: Vec<&[u8]> = lead.split(u8::is_ascii_whitespace).collect();
    let last = words.last().copied().unwrap_or_default();
    !quoted
        && match words.as_slice() {
            [w] => assigned(w),
            [b"echo" | b"printf" | b":", ..] => true,
            [
                b"local" | b"export" | b"declare" | b"readonly" | b"typeset",
                ..,
            ] => assigned(last),
            _ => false,
        }
}
fn shell_line_group_end(b: &[u8], open: usize) -> Option<usize> {
    let line_quote_end = |at: usize, stops: &[u8]| {
        let n = b.get(at + 1..)?.iter().position(|c| stops.contains(c))?;
        (b[at + 1 + n] == b[at]).then_some(at + n + 2)
    };
    let mut depth = 0usize;
    let mut i = open;
    while let Some(&c) = b.get(i) {
        i = match c {
            b'\n' | b'`' | b'#' => return None,
            b'<' if b.get(i + 1) == Some(&b'<') => return None,
            b'c' if b[i..].starts_with(b"case") => return None,
            b'\\' => i + 2,
            b'\'' => line_quote_end(i, b"'\n")?,
            b'"' => line_quote_end(i, b"\"\n\\$`")?,
            b'(' => {
                depth += 1;
                i + 1
            }
            b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i + 1);
                }
                i + 1
            }
            _ => i + 1,
        };
    }
    None
}
fn ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}
