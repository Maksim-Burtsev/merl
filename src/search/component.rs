use super::*;
use std::borrow::Cow;
use std::ops::Range;
use std::path::Path;

pub fn component(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "vue" || e == "svelte" || e == "astro")
}
pub fn script_lines(path: &Path, text: &str) -> Option<Vec<bool>> {
    if !component(path) {
        return None;
    }
    let astro = path.extension().is_some_and(|e| e == "astro");
    #[derive(PartialEq)]
    enum At {
        Outside,
        InWrappedScriptTag,
        InScript,
        InFrontmatter,
    }
    let mut at = At::Outside;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim().trim_start_matches('\u{feff}');
        let code = match at {
            At::Outside if astro && i == 0 && t == "---" => {
                at = At::InFrontmatter;
                false
            }
            At::Outside if t.starts_with("<script") && t[7..].starts_with([' ', '>', '\t']) => {
                at = script_open(t);
                false
            }
            At::Outside if t == "<script" => {
                at = At::InWrappedScriptTag;
                false
            }
            At::Outside => false,
            At::InWrappedScriptTag => {
                if let Some(close) = t.find('>') {
                    at = script_open(&t[close..]);
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
    return Some(out);

    fn script_open(t: &str) -> At {
        match (t.contains("</script"), t.contains('>')) {
            (true, _) => At::Outside,
            (false, true) => At::InScript,
            (false, false) => At::InWrappedScriptTag,
        }
    }
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
        .map(|f| regex::Regex::new(f).unwrap())
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
    let text = |i: usize| lines[i].as_ref();
    let indent = |i: usize| text(i).len() - text(i).trim_start().len();
    let opens = |i: usize| text(i).trim_start().starts_with(['<', '{']);
    template_binds(lines, code, word).into_iter().any(|b| {
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
    })
}
fn pattern_names(pattern: &str) -> Vec<&str> {
    static TOKEN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"[A-Za-z_$][\w$]*|"[^"]*"|'[^']*'|[{}\[\]()=:,]"#).unwrap()
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
    let before = &line[..range.start];
    (before.ends_with('<') || before.ends_with("</")).then_some(range)
}
pub fn component_name(tag: &str) -> String {
    match tag.contains('-') {
        true => (tag.split('-'))
            .map(|p| p[..p.len().min(1)].to_uppercase() + &p[p.len().min(1)..])
            .collect(),
        false => tag.to_owned(),
    }
}
