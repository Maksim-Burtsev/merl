use super::{Kind, file_literal_lines, literal_lines, word_at};
use std::borrow::Cow;
use std::ops::Range;
use std::path::Path;

pub fn component(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "vue" || e == "svelte" || e == "astro")
}
pub fn script_lines(path: &Path, text: &str) -> Option<Vec<bool>> {
    #[derive(PartialEq)]
    enum At {
        Outside,
        InWrappedScriptTag,
        InScript,
        InFrontmatter,
    }
    fn script_open(t: &str) -> At {
        match (t.contains("</script"), t.contains('>')) {
            (true, _) => At::Outside,
            (false, true) => At::InScript,
            (false, false) => At::InWrappedScriptTag,
        }
    }
    if !component(path) {
        return None;
    }
    let astro = path.extension().is_some_and(|e| e == "astro");
    let mut at = At::Outside;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim().trim_start_matches('\u{feff}');
        let code = match at {
            At::Outside if astro && i == 0 && t == "---" => {
                at = At::InFrontmatter;
                false
            }
            At::Outside
                if t.strip_prefix("<script")
                    .is_some_and(|r| r.starts_with([' ', '>', '\t'])) =>
            {
                at = script_open(t);
                false
            }
            At::Outside if t == "<script" => {
                at = At::InWrappedScriptTag;
                false
            }
            At::Outside => false,
            At::InWrappedScriptTag => {
                if let Some(rest) = t.find('>').and_then(|close| t.get(close..)) {
                    at = script_open(rest);
                }
                false
            }
            At::InScript if t.starts_with("</script") => {
                at = At::Outside;
                false
            }
            At::InFrontmatter if t == "---" => {
                at = At::Outside;
                false
            }
            At::InScript | At::InFrontmatter => true,
        };
        out.push(code);
    }
    Some(out)
}
pub fn script_text<'a>(path: &Path, text: &'a str, cursor: Option<usize>) -> Cow<'a, str> {
    let Some(code) = script_lines(path, text) else {
        return Cow::Borrowed(text);
    };
    let lines = text
        .lines()
        .zip(code)
        .enumerate()
        .map(|(i, (l, c))| match c {
            true => Cow::Borrowed(l),
            false if cursor == Some(i) => Cow::Owned(" ".repeat(l.len())),
            false => Cow::Borrowed(""),
        });
    Cow::Owned(lines.collect::<Vec<_>>().join("\n"))
}
pub fn hidden_lines(kind: Kind, path: &Path, text: &str) -> Vec<bool> {
    let Some(code) = script_lines(path, text) else {
        return file_literal_lines(kind, path, text);
    };
    let mut out = literal_lines(kind, &script_text(path, text, None));
    out.resize(code.len(), false);
    for (i, (hidden, code)) in out.iter_mut().zip(code).enumerate() {
        *hidden |= i > 0 && !code;
    }
    out
}
pub fn template_binds<S: AsRef<str>>(lines: &[S], code: &[bool], word: &str) -> Vec<usize> {
    static FORMS: std::sync::LazyLock<Vec<regex::Regex>> = std::sync::LazyLock::new(|| {
        [
            r#"\bv-for\s*=\s*["']\s*(.+?)\s+(?:in|of)\s"#,
            r#"\bv-slot(?::[\w-]+|:\[[^\]]*\])?\s*=\s*["']([^"']*)["']"#,
            r#"(?:^|\s)#[\w-]+\s*=\s*["']([^"']*)["']"#,
            r"\{#each\s+[^}]*?\s+as\s+([^}]*?)\s*(?:\([^()]*\))?\s*\}",
            r"\{(?::then|:catch)\s+([^}]*)\}",
            r"\{#await\s+[^}]*?\s(?:then|catch)\s+([^}]*)\}",
            r"\blet:([\w$]+)(?:\s*=\s*\{([^}]*)\})?",
            r"\{@const\s+([^=]*)=",
            r"\{#snippet\s+[\w$]+\s*\(([^)]*)\)",
        ]
        .iter()
        .map(|f| regex::Regex::new(f).expect("the regex is a literal"))
        .collect()
    });
    let binds = |pattern: &str| pattern_names(pattern).contains(&word);
    let lines = lines.iter().map(AsRef::as_ref);
    lines
        .enumerate()
        .filter(|&(i, line)| {
            !code.get(i).copied().unwrap_or(false)
                && FORMS.iter().any(|re| {
                    (re.captures_iter(line)).any(|c| {
                        c.iter()
                            .skip(1)
                            .flatten()
                            .last()
                            .is_some_and(|m| binds(m.as_str()))
                    })
                })
        })
        .map(|(i, _)| i + 1)
        .collect()
}
pub fn template_reaches<S: AsRef<str>>(
    lines: &[S],
    code: &[bool],
    word: &str,
    line: usize,
) -> bool {
    !template_reaching(lines, code, word, line).is_empty()
}
pub fn template_reaching<S: AsRef<str>>(
    lines: &[S],
    code: &[bool],
    word: &str,
    line: usize,
) -> Vec<usize> {
    let text = |i: usize| lines[i].as_ref();
    let indent = |i: usize| text(i).len() - text(i).trim_start().len();
    let opens = |i: usize| text(i).trim_start().starts_with(['<', '{']);
    let mut bound = template_binds(lines, code, word);
    bound.retain(|&b| {
        let at = b - 1;
        let start = (0..=at)
            .rev()
            .take_while(|&i| !code.get(i).copied().unwrap_or(false))
            .find(|&i| opens(i))
            .unwrap_or(at);
        let base = indent(start);
        let sibling = text(start).trim_start().starts_with("{@const");
        let within = |i: usize| {
            let t = text(i).trim();
            t.is_empty() || t == ">" || indent(i) > base || sibling && indent(i) == base
        };
        start <= line && (line <= at || (at + 1..=line).all(within))
    });
    bound.extend(arrow_reaching(lines, code, word, line));
    bound
}
fn arrow_reaching<S: AsRef<str>>(
    lines: &[S],
    code: &[bool],
    word: &str,
    line: usize,
) -> Vec<usize> {
    static ARROW: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?:\(([^()]*)\)|([A-Za-z_$][\w$]*))\s*=>(.*)$")
            .expect("the regex is a literal")
    });
    let text = |i: usize| lines[i].as_ref();
    let indent = |i: usize| text(i).len() - text(i).trim_start().len();
    let open = |rest: &str| {
        let opens = rest.matches(['(', '[', '{']).count();
        rest.trim().is_empty() || opens > rest.matches([')', ']', '}']).count()
    };
    if code.get(line).copied().unwrap_or(true) {
        return Vec::new();
    }
    (0..line)
        .rev()
        .take_while(|&i| !code.get(i).copied().unwrap_or(true))
        .filter(|&at| {
            ARROW.captures_iter(text(at)).any(|c| {
                let params = c.get(1).or_else(|| c.get(2)).map_or("", |m| m.as_str());
                pattern_names(params).contains(&word) && c.get(3).is_some_and(|r| open(r.as_str()))
            }) && (at + 1..=line).all(|i| text(i).trim().is_empty() || indent(i) > indent(at))
        })
        .take(1)
        .map(|at| at + 1)
        .collect()
}
fn pattern_names(pattern: &str) -> Vec<&str> {
    static TOKEN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"[A-Za-z_$][\w$]*|"[^"]*"|'[^']*'|[{}\[\]()=:,]"#)
            .expect("the regex is a literal")
    });
    let tokens: Vec<&str> = TOKEN.find_iter(pattern).map(|m| m.as_str()).collect();
    let (mut names, mut depth, mut value) = (Vec::new(), 0usize, false);
    for (i, t) in tokens.iter().enumerate() {
        match *t {
            "{" | "[" | "(" => depth += 1,
            "}" | "]" | ")" => depth = depth.saturating_sub(1),
            "," => value = false,
            "=" => value = true,
            ":" if depth == 0 => value = true,
            _ if value || !t.starts_with(|c: char| c.is_alphabetic() || c == '_' || c == '$') => {}
            _ if depth > 0 && tokens.get(i + 1) == Some(&":") => {}
            _ => names.push(*t),
        }
    }
    names
}
pub fn in_style<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    let mut inside = false;
    for l in lines.iter().take(line + 1).map(|l| l.as_ref().trim()) {
        if inside {
            inside = !l.starts_with("</style");
        } else if let Some(rest) = l.strip_prefix("<style") {
            inside = rest.starts_with([' ', '>', '\t']) && !rest.contains("</style");
        }
    }
    inside
        && !lines
            .get(line)
            .is_some_and(|l| l.as_ref().trim().starts_with("</style"))
}
pub fn tag_at(line: &str, col: usize) -> Option<Range<usize>> {
    let (range, _) = word_at(line, col, "-")?;
    let before = line.get(..range.start)?;
    (before.ends_with('<') || before.ends_with("</")).then_some(range)
}
pub fn component_name(tag: &str) -> String {
    if tag.contains('-') {
        (tag.split('-'))
            .map(|p| {
                let mut chars = p.chars();
                chars
                    .next()
                    .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
            })
            .collect()
    } else {
        tag.to_owned()
    }
}
