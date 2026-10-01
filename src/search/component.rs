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
        let t = line.trim();
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
/// [`literal_lines`] of a file's text as [`script_text`] gave it, with a component's lines
/// outside its script hidden too: those the mask left blank that a grep found a word on. Its
/// first line stays, a tag or a fence no pattern declares by: there `d` lands on the file
/// itself.
pub fn hidden_lines(kind: Kind, path: &Path, text: &str) -> Vec<bool> {
    let mut out = literal_lines(kind, text);
    if component(path) {
        out.resize(text.lines().count(), false);
        for (i, (hidden, line)) in out.iter_mut().zip(text.lines()).enumerate() {
            *hidden |= i > 0 && line.trim().is_empty();
        }
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
    // The names a pattern binds: its identifiers, save a type or a key after a `:`.
    static NAME: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(:\s*)?([A-Za-z_$][\w$]*)").unwrap());
    let binds = |pattern: &str| {
        NAME.captures_iter(pattern)
            .any(|c| c.get(1).is_none() && &c[2] == word)
    };
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
