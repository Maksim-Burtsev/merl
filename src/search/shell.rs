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
        for (j, l) in lines.iter().enumerate().take(at).skip(i) {
            let t = l.trim();
            if literal.get(j) == Some(&true) {
                continue;
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
        closed
    };
    let assigns = |i: usize| {
        literal.get(i) != Some(&true)
            && (assign.is_match(lines[i]) || shell_loop_binds(lines[i], name) == Some(true))
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
                (Some(b'"'), b'\\') => i += 1,
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, c @ (b'"' | b'\'')) => quote = Some(c),
                (None, b'(') => depth += 1,
                (None, b')') => depth = depth.saturating_sub(1),
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
