//! Vue, Svelte and Astro components (#413): TypeScript in a `<script>` block or an Astro
//! frontmatter, around a template that declares nothing.

use super::*;
use std::borrow::Cow;
use std::ops::Range;
use std::path::Path;

/// Whether `path` is a `.vue`, `.svelte` or `.astro` component.
pub fn component(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "vue" || e == "svelte" || e == "astro")
}
/// For a component, which lines of `text` are its script, one flag per line: the lines between
/// a `<script …>` line and its `</script>` line, and in Astro the frontmatter between a `---`
/// first line and the next `---` line. The tags and fences are on lines of their own, as the
/// three write them. `None` for any other file.
pub fn script_lines(path: &Path, text: &str) -> Option<Vec<bool>> {
    if !component(path) {
        return None;
    }
    let astro = path.extension().is_some_and(|e| e == "astro");
    // Where a line stands: outside, inside a `<script` tag wrapped over lines, in the script.
    #[derive(PartialEq)]
    enum At {
        Out,
        Tag,
        Script,
        Front,
    }
    let mut at = At::Out;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        // A file saved with a BOM opens with it, read from the disk.
        let t = line.trim().trim_start_matches('\u{feff}');
        let code = match at {
            At::Out if astro && i == 0 && t == "---" => {
                at = At::Front;
                false
            }
            At::Out if t.starts_with("<script") && t[7..].starts_with([' ', '>', '\t']) => {
                at = script_open(t);
                false
            }
            At::Out if t == "<script" => {
                at = At::Tag;
                false
            }
            At::Out => false,
            At::Tag => {
                if let Some(close) = t.find('>') {
                    at = script_open(&t[close..]);
                }
                false
            }
            At::Script if t.starts_with("</script") => {
                at = At::Out;
                false
            }
            At::Front if t == "---" => {
                at = At::Out;
                false
            }
            At::Script | At::Front => true,
        };
        out.push(code);
    }
    return Some(out);

    /// After `<script …>` opens on a line: in the script, unless the line closes it too
    /// (`<script src="x"></script>`) or its tag goes on to the next line.
    fn script_open(t: &str) -> At {
        match (t.contains("</script"), t.contains('>')) {
            (true, _) => At::Out,
            (false, true) => At::Script,
            (false, false) => At::Tag,
        }
    }
}
/// `text` as the TypeScript rules read it: for a component, every line outside its script left
/// blank, so a template's `{{ }}` or `{#if}` is never a brace of the script and a `<style>` line
/// shaped like a field declares nothing; any other file as it is. The 0-based line `cursor`
/// blanks to spaces instead, keeping its length for the rules that read the word's place on it.
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
/// The lines of `text`, a file as it is, where a declaration-shaped line declares nothing:
/// [`literal_lines`], and in a component every line outside its script but the first, a tag or
/// a fence no pattern declares by, where `d` lands on the file itself.
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
/// The 1-based lines of a component's template, `code` being its [`script_lines`], that bind
/// `word` themselves: Vue's `v-for`, `v-slot` and `#slot="…"`, Svelte's `{#each … as …}`,
/// `{:then …}`, `{:catch …}`, `let:`, `{@const …}` and `{#snippet name(…)}`.
pub fn template_binds<S: AsRef<str>>(lines: &[S], code: &[bool], word: &str) -> Vec<usize> {
    static FORMS: std::sync::LazyLock<Vec<regex::Regex>> = std::sync::LazyLock::new(|| {
        [
            r#"\bv-for\s*=\s*["']\s*(.+?)\s+(?:in|of)\s"#,
            r#"\bv-slot(?::[\w-]+|:\[[^\]]*\])?\s*=\s*["']([^"']*)["']"#,
            r#"(?:^|\s)#[\w-]+\s*=\s*["']([^"']*)["']"#,
            r"\{#each\s+[^}]*?\s+as\s+([^}]*?)\s*(?:\([^()]*\))?\s*\}",
            r"\{(?::then|:catch)\s+([^}]*)\}",
            r"\{#await\s+[^}]*?\s(?:then|catch)\s+([^}]*)\}",
            // `let:item={row}` binds `row` alone.
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
/// The names a binding pattern binds: `item, i`, `{ id, name: label }` (`id` and `label`), `row:
/// Row` (`row`), with no default value's names (`{ size = md }` binds `size`).
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
            // A default value runs to the next `,`; outside a destructuring a `:` opens a type.
            "=" => value = true,
            ":" if depth == 0 => value = true,
            _ if value || !t.starts_with(|c: char| c.is_alphabetic() || c == '_' || c == '$') => {}
            // Inside a destructuring a name before a `:` is the key, not the name bound.
            _ if depth > 0 && tokens.get(i + 1) == Some(&":") => {}
            _ => names.push(*t),
        }
    }
    names
}
/// Whether the 0-based `line` of a component stands inside a `<style …>` block, between the
/// tags on lines of their own: its rules are a stylesheet's, no code.
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
/// The tag name the byte `col` of `line` stands on, `-` included: `user-card` in
/// `<user-card :user="u">`, `UserCard` in `</UserCard>`.
pub fn tag_at(line: &str, col: usize) -> Option<Range<usize>> {
    let (range, _) = word_at(line, col, "-")?;
    let before = &line[..range.start];
    (before.ends_with('<') || before.ends_with("</")).then_some(range)
}
/// The name a component is imported and registered under for its tag: `UserCard` for
/// `user-card`; a tag without `-` as it is.
pub fn component_name(tag: &str) -> String {
    match tag.contains('-') {
        true => (tag.split('-'))
            .map(|p| p[..p.len().min(1)].to_uppercase() + &p[p.len().min(1)..])
            .collect(),
        false => tag.to_owned(),
    }
}
