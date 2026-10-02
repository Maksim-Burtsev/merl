use regex::Regex;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// An attribute whose value `d` reads, in a file of any kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Class,
    Id,
    Href,
    Src,
}

/// The attribute value byte `col` of `line` stands in, and the range `d` reads there: the
/// space-separated word of a `class`, `className` or `id` value, the whole value of a `href` or a
/// `src`. `className={'a b'}` is a class value, and so is Svelte's `class:active`; Vue's `:class`
/// binds an expression, which is not one.
///
/// `jsx` reads a line of a JavaScript or TypeScript file, where `id = "main"` is an assignment:
/// an attribute there has no blank around its `=`, and a `class` or an `id` stands in a tag, or
/// first on a line of the tag's attributes.
pub fn attr_at(line: &str, col: usize, jsx: bool) -> Option<(Attr, Range<usize>)> {
    static ATTR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?:^|[\s<{(])(class|className|id|href|src)\s*=\s*(?:\{\s*)?(?:"([^"]*)"|'([^']*)'|`([^`$]*)`)"#,
        )
        .unwrap()
    });
    static SVELTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bclass:([\w-]+)").unwrap());
    for c in ATTR.captures_iter(line) {
        let v = c.get(2).or_else(|| c.get(3)).or_else(|| c.get(4))?;
        if col < v.start() || col > v.end() {
            continue;
        }
        let name = c.get(1)?;
        let spaced =
            !line[name.end()..].starts_with('=') || line[name.end() + 1..].starts_with([' ', '\t']);
        let lead = line[..name.start()].trim_start();
        if jsx && (spaced || (&c[1] != "className" && !lead.is_empty() && !lead.contains('<'))) {
            return None;
        }
        let attr = match &c[1] {
            "class" | "className" => Attr::Class,
            "id" => Attr::Id,
            "href" => Attr::Href,
            _ => Attr::Src,
        };
        if matches!(attr, Attr::Href | Attr::Src) {
            let lead = v.as_str().len() - v.as_str().trim_start().len();
            let start = v.start() + lead;
            return (start < v.start() + v.as_str().trim_end().len())
                .then(|| (attr, start..v.start() + v.as_str().trim_end().len()));
        }
        let b = line.as_bytes();
        let mut start = col.min(v.end());
        let mut end = start;
        while start > v.start() && !b[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        while end < v.end() && !b[end].is_ascii_whitespace() {
            end += 1;
        }
        return (start < end).then_some((attr, start..end));
    }
    // Svelte's directive: a `"class:x"` in a TypeScript string is no attribute.
    if jsx {
        return None;
    }
    SVELTE
        .captures_iter(line)
        .filter_map(|c| c.get(1))
        .find(|m| m.start() <= col && col <= m.end())
        .map(|m| (Attr::Class, m.range()))
}

/// A line of a stylesheet whose rule styles `classes` and `ids`: the names written in the last
/// compound of one selector of its list, the element the rule styles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// 1-based.
    pub line: usize,
    pub classes: Vec<String>,
    pub ids: Vec<String>,
}

/// What the rule a `{` opens is to the rules inside it.
struct Frame {
    /// The class each selector of the list ends in, which an `&-suffix` inside composes with.
    trailing: Vec<String>,
    /// Inside `@keyframes`, `@font-face` or a nested property: nothing in it is a selector.
    opaque: bool,
}

/// The rules of a stylesheet `text`, in order. A selector is the text before a `{` since the
/// last `;`, `{` or `}`, over as many lines as its list takes; comments and strings are skipped,
/// and `//` opens a comment outside parentheses, `url(//cdn…)` being inside them.
/// In SCSS and Less an `&` followed by a name composes it with the class the enclosing rule ends
/// in: `&__title` inside `.card` styles `.card__title`. A `#{…}` or `@{…}` interpolation is part
/// of the selector, and a name it is glued to is none.
pub fn rules(text: &str) -> Vec<Rule> {
    let b = text.as_bytes();
    let (mut i, mut line, mut paren) = (0, 1, 0usize);
    let mut buf: Vec<(u8, usize)> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        match c {
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    line += usize::from(b[i] == b'\n');
                    i += 1;
                }
                i += 2;
                continue;
            }
            b'/' if paren == 0 && b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'"' | b'\'' => {
                buf.push((c, line));
                i += 1;
                while i < b.len() && b[i] != c && b[i] != b'\n' {
                    buf.push((b[i], line));
                    i += 1;
                }
                if i < b.len() && b[i] == c {
                    buf.push((c, line));
                    i += 1;
                }
                continue;
            }
            // An interpolation is a piece of the selector, braces and all.
            b'#' | b'@' if b.get(i + 1) == Some(&b'{') => {
                while i < b.len() && b[i] != b'}' && b[i] != b'\n' {
                    buf.push((b[i], line));
                    i += 1;
                }
                if i < b.len() && b[i] == b'}' {
                    buf.push((b'}', line));
                    i += 1;
                }
                continue;
            }
            b'{' => {
                let frame = open(&buf, stack.last(), &mut out);
                stack.push(frame);
                buf.clear();
                paren = 0;
            }
            b'}' => {
                stack.pop();
                buf.clear();
                paren = 0;
            }
            b';' => {
                buf.clear();
                paren = 0;
            }
            _ => {
                match c {
                    b'(' => paren += 1,
                    b')' => paren = paren.saturating_sub(1),
                    _ => {}
                }
                buf.push((c, line));
            }
        }
        line += usize::from(c == b'\n');
        i += 1;
    }
    out
}

pub fn sass_rules(text: &str) -> Vec<Rule> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let lines: Vec<String> = text.lines().map(css_code).collect();
    let mut comment: Option<usize> = None;
    let code: Vec<Option<&str>> = (text.lines().zip(&lines))
        .map(|(raw, l)| {
            let depth = indent(raw);
            if comment.is_some_and(|c| depth > c) || raw.trim().is_empty() {
                return None;
            }
            comment = None;
            let t = raw.trim_start();
            if t.starts_with("//") || t.starts_with("/*") {
                comment = Some(depth);
            }
            Some(l.trim_end()).filter(|l| !l.trim().is_empty())
        })
        .collect();
    let mut stack: Vec<(usize, Frame)> = Vec::new();
    let mut buf: Vec<(u8, usize)> = Vec::new();
    let mut depth = 0;
    let mut out = Vec::new();
    for (i, l) in code.iter().enumerate() {
        let Some(l) = l else { continue };
        if buf.is_empty() {
            depth = indent(l);
            while stack.last().is_some_and(|(d, _)| *d >= depth) {
                stack.pop();
            }
        }
        let t = l.trim_start();
        let t = match t.as_bytes().first() {
            Some(b'=') => format!("@mixin {}", &t[1..]),
            Some(b'+') => format!("@include {}", &t[1..]),
            _ => t.to_owned(),
        };
        buf.extend(t.bytes().map(|c| (c, i + 1)));
        buf.push((b'\n', i + 1));
        if t.ends_with(',') {
            continue;
        }
        let body = code[i + 1..].iter().flatten().next();
        if body.is_some_and(|n| indent(n) > depth) {
            let frame = open(&buf, stack.last().map(|(_, f)| f), &mut out);
            stack.push((depth, frame));
        }
        buf.clear();
    }
    out
}

/// The frame a `{` after the selector text `buf` opens, with the rules it adds to `out`.
fn open(buf: &[(u8, usize)], parent: Option<&Frame>, out: &mut Vec<Rule>) -> Frame {
    let text: String = buf.iter().map(|&(c, _)| c as char).collect();
    let head = text.trim();
    let inherited = || Frame {
        trailing: parent.map(|p| p.trailing.clone()).unwrap_or_default(),
        opaque: parent.is_some_and(|p| p.opaque),
    };
    let opaque = Frame {
        trailing: Vec::new(),
        opaque: true,
    };
    if parent.is_some_and(|p| p.opaque) {
        return opaque;
    }
    if let Some(at) = head.strip_prefix('@') {
        let name: String = at
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        return match name.as_str() {
            "media" | "supports" | "layer" | "container" | "include" | "at-root" | "if"
            | "else" | "each" | "for" | "while" | "document" | "scope" | "starting-style" => {
                inherited()
            }
            // A mixin's body styles whatever includes it; its `&` is unknown here.
            "mixin" => Frame {
                trailing: Vec::new(),
                opaque: false,
            },
            _ => opaque,
        };
    }
    // `font: { family: x; }` nests properties, and `@detached: {` is a Less ruleset in a variable.
    if head.is_empty() || head.ends_with(':') || head.starts_with('%') && head.contains(':') {
        return opaque;
    }
    let parents = parent.map(|p| p.trailing.as_slice()).unwrap_or_default();
    let mut trailing = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    let mut parts = Vec::new();
    for (k, &(c, _)) in buf.iter().enumerate() {
        match c {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(&buf[start..k]);
                start = k + 1;
            }
            _ => {}
        }
    }
    parts.push(&buf[start..]);
    for part in parts {
        let Some((compound, line)) = last_compound(part) else {
            continue;
        };
        let (classes, ids, ends_in) = names(&compound, parents);
        trailing.extend(ends_in);
        if !classes.is_empty() || !ids.is_empty() {
            out.push(Rule { line, classes, ids });
        }
    }
    Frame {
        trailing,
        opaque: false,
    }
}

/// The last compound of the selector `part`, past its last combinator, and the line it starts on.
fn last_compound(part: &[(u8, usize)]) -> Option<(String, usize)> {
    let end = part.iter().rposition(|&(c, _)| !c.is_ascii_whitespace())? + 1;
    let part = &part[..end];
    let mut depth = 0i32;
    let mut start = 0;
    for (k, &(c, _)) in part.iter().enumerate() {
        match c {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'+' | b'~' if depth == 0 => start = k + 1,
            _ => {}
        }
    }
    let rest = &part[start..];
    let line = rest.first()?.1;
    Some((rest.iter().map(|&(c, _)| c as char).collect(), line))
}

/// The classes and ids the compound writes, `&-suffix` composed with each of `parents`, and the
/// classes it ends in for the rules nested inside it to compose with.
fn names(compound: &str, parents: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([.#])([\w-]+)").unwrap());
    static SUFFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^&([\w-]+)").unwrap());
    // What a `:not(…)`, an `[attr]` or a Less mixin's parameters hold styles nothing.
    let mut plain = String::new();
    let mut depth = 0;
    for c in compound.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let (mut classes, mut ids, mut ends_in) = (Vec::new(), Vec::new(), Vec::new());
    if let Some(c) = SUFFIX.captures(&plain) {
        let glued = plain[c[0].len()..].starts_with(['#', '@']);
        if !glued {
            for p in parents {
                classes.push(format!("{p}{}", &c[1]));
            }
            if plain.len() == c[0].len() {
                ends_in.clone_from(&classes);
            }
        }
    }
    for m in NAME.captures_iter(&plain) {
        let whole = m.get(0).unwrap();
        // `.col-#{$i}`: an interpolation finishes the name, which is then nobody's.
        if plain[whole.end()..].starts_with(['#', '@', '$'])
            || m[2].starts_with(|c: char| c.is_ascii_digit())
        {
            continue;
        }
        match &m[1] {
            "." => classes.push(m[2].to_owned()),
            _ => ids.push(m[2].to_owned()),
        }
        if &m[1] == "." && whole.end() == plain.len() {
            ends_in = vec![m[2].to_owned()];
        }
    }
    (classes, ids, ends_in)
}

/// `text` with everything outside its `<style>` blocks blanked, the lines kept: the stylesheet
/// an HTML, Vue, Svelte or Astro file holds.
pub fn style_blocks(text: &str) -> String {
    static STYLE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)<style\b[^>]*>(.*?)</style\s*>").unwrap());
    let mut out: Vec<u8> = text
        .bytes()
        .map(|c| if c == b'\n' { c } else { b' ' })
        .collect();
    for c in STYLE.captures_iter(text) {
        let m = c.get(1).unwrap();
        out[m.range()].copy_from_slice(m.as_str().as_bytes());
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Whether a file at `path` holds styles: a stylesheet, or markup with `<style>` blocks.
pub fn styles_in(path: &Path) -> Option<bool> {
    match path.extension()?.to_str()? {
        "css" | "scss" | "sass" | "less" => Some(true),
        "html" | "htm" | "vue" | "svelte" | "astro" => Some(false),
        _ => None,
    }
}

/// What the cursor in a stylesheet stands on, for `d`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sheet {
    /// `--brand`, with its dashes.
    Custom(String),
    /// A SCSS `$name`, behind the namespace of a `@use` or not.
    Var(Option<String>, String),
    /// A Less `@name`.
    LessVar(String),
    Mixin(Option<String>, String),
    Function(Option<String>, String),
    Placeholder(String),
    Class(String),
    Id(String),
    Keyframes(String),
    /// The path of an `@import`, a `@use` or a `@forward`, as written.
    Import(String),
}

/// What byte `col` of the stylesheet line `line` stands on; `less` reads `@name` as a variable.
pub fn sheet_at(line: &str, col: usize, less: bool) -> Option<Sheet> {
    static IMPORT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*@(?:import|use|forward|require)\b").unwrap());
    static QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#""([^"]*)"|'([^']*)'|url\(\s*([^)"'\s]+)\s*\)"#).unwrap());
    static ANIMATION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:-[a-z]+-)?animation(?:-name)?\s*:").unwrap());
    static NS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\w-]+)\.$").unwrap());
    if IMPORT.is_match(line) {
        return QUOTED.captures_iter(line).find_map(|c| {
            let m = c.get(1).or_else(|| c.get(2)).or_else(|| c.get(3))?;
            (m.start() <= col && col <= m.end()).then(|| Sheet::Import(m.as_str().to_owned()))
        });
    }
    let (range, word) = super::word_at(line, col, "-")?;
    let word = word.to_owned();
    let before = &line[..range.start];
    let after = &line[range.end..];
    let ns = |b: &str| NS.captures(b).map(|c| c[1].to_owned());
    if before.ends_with("--") {
        return Some(Sheet::Custom(format!("--{word}")));
    }
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    if let Some(b) = before.strip_suffix('$') {
        return Some(Sheet::Var(ns(b), word));
    }
    if before.ends_with('%') {
        return Some(Sheet::Placeholder(word));
    }
    if before.ends_with('@') {
        return less.then_some(Sheet::LessVar(word));
    }
    static AT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"@(include|mixin|function|extend)\s+(?:([\w-]+)\.)?(\.)?$").unwrap()
    });
    if let Some(c) = AT.captures(before) {
        let ns = c.get(2).map(|m| m.as_str().to_owned());
        return match (&c[1], c.get(3).is_some()) {
            ("include", false) => Some(Sheet::Mixin(ns, word)),
            ("mixin", false) if ns.is_none() => Some(Sheet::Mixin(None, word)),
            ("function", false) if ns.is_none() => Some(Sheet::Function(None, word)),
            // `@extend .btn;` is a use of the class.
            ("extend", true) if ns.is_none() => Some(Sheet::Class(word)),
            _ => None,
        };
    }
    if ANIMATION.is_match(line) && before.contains(':') {
        return Some(Sheet::Keyframes(word));
    }
    // A Less mixin call is a statement, `.bordered();` or `.bordered;`: the class lookup.
    let statement = after.trim_start().starts_with(';')
        || (after.starts_with('(') && after.trim_end().ends_with(';'));
    if before.trim_start() == "." && statement && less {
        return Some(Sheet::Class(word));
    }
    // A function call in a value: `tint-color($c, 10%)`, `math.div(…)`.
    if after.starts_with('(') && (before.contains(':') || before.trim_start().starts_with('@')) {
        return Some(Sheet::Function(ns(before), word));
    }
    // Selector text: a line that opens a rule or continues its list, and holds no declaration.
    let t = line.trim_end();
    let selector = (t.ends_with('{') || t.ends_with(',')) && !t.contains(';');
    match before.chars().last() {
        Some('.') if selector => Some(Sheet::Class(word)),
        Some('#') if selector => Some(Sheet::Id(word)),
        _ => None,
    }
}

pub fn sheet_patterns(sheet: &Sheet) -> Vec<String> {
    let w = |n: &str| regex::escape(n);
    let end = r"(?:[^\w-]|$)";
    match sheet {
        Sheet::Custom(n) => vec![
            format!(r"(?:^|[\s;{{]){}\s*:", w(n)),
            format!(r"@property\s+{}{end}", w(n)),
        ],
        Sheet::Var(_, n) => vec![format!(r"^\s*\${}\s*:", w(n))],
        Sheet::LessVar(n) => vec![format!(r"^\s*@{}\s*:", w(n))],
        Sheet::Mixin(_, n) => vec![
            format!(r"^\s*@mixin\s+{}{end}", w(n)),
            format!(r"^\s*={}{end}", w(n)),
        ],
        Sheet::Function(_, n) => vec![format!(r"^\s*@function\s+{}{end}", w(n))],
        Sheet::Placeholder(n) => vec![format!(r"^\s*%{}[^\w;-]*[{{,]\s*$", w(n))],
        Sheet::Keyframes(n) => vec![format!(r"^\s*@(?:-[a-z]+-)?keyframes\s+{}{end}", w(n))],
        Sheet::Class(_) | Sheet::Id(_) | Sheet::Import(_) => Vec::new(),
    }
}

/// The `@use` lines of a Sass file: the namespace each binds (`None` for `as *`) and the module.
/// `@use 'mixins'` binds `mixins`, `@use 'src/corners' as c` binds `c`; `sass:math` is a
/// built-in module, nobody's file. `@import` binds no namespace.
pub fn sass_uses(text: &str) -> Vec<(Option<String>, String)> {
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\s*@use\s+["']([^"']+)["'](?:\s+as\s+([\w-]+|\*))?"#).unwrap()
    });
    text.lines()
        .filter_map(|l| USE.captures(l))
        .filter(|c| !c[1].starts_with("sass:"))
        .map(|c| {
            let module = c[1].to_owned();
            let ns = match c.get(2).map(|m| m.as_str()) {
                Some("*") => None,
                Some(n) => Some(n.to_owned()),
                None => {
                    let last = module.rsplit('/').next().unwrap_or(&module);
                    let last = last.split('.').next().unwrap_or(last);
                    Some(last.trim_start_matches('_').to_owned())
                }
            };
            (ns, module)
        })
        .collect()
}

/// The files a Sass `@use`, `@forward` or `@import` of `module` may be, in the order Sass tries
/// them, relative to the importing file's directory: `_x.scss`, `x.scss`, `_x.sass`, `x.sass`,
/// `x.css`, `x/_index.scss`, `x/index.scss`; a module written with its extension is that file or
/// its partial. Webpack's `~` in front names a package.
pub fn sass_candidates(module: &str) -> Vec<PathBuf> {
    let module = module.trim_start_matches('~');
    let path = Path::new(module);
    let dir = path.parent().unwrap_or(Path::new(""));
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    if [".scss", ".sass", ".css"].iter().any(|e| name.ends_with(e)) {
        return vec![dir.join(format!("_{name}")), dir.join(name)];
    }
    [
        format!("_{name}.scss"),
        format!("{name}.scss"),
        format!("_{name}.sass"),
        format!("{name}.sass"),
        format!("{name}.css"),
        format!("{name}/_index.scss"),
        format!("{name}/index.scss"),
    ]
    .into_iter()
    .map(|f| dir.join(f))
    .collect()
}

/// The files a Less or CSS `@import` of `module` may be: as written, and a Less one with
/// `.less` added when it has no extension.
pub fn import_candidates(module: &str, less: bool) -> Vec<PathBuf> {
    let module = module.trim_start_matches('~');
    let mut out = vec![PathBuf::from(module)];
    if less && Path::new(module).extension().is_none() {
        out.insert(0, PathBuf::from(format!("{module}.less")));
    }
    out
}

/// Whether `target` is a URL, which `d` does not follow: `https://…`, `//cdn…`, `data:`.
pub fn is_url(target: &str) -> bool {
    static URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:[A-Za-z][A-Za-z0-9+.-]*:|//)").unwrap());
    URL.is_match(target)
}

/// The 1-based line of the element of the HTML `text` with `id="name"` or `name="name"`.
pub fn element_with_id(text: &str, name: &str) -> Option<usize> {
    let re = Regex::new(&format!(
        r#"\s(?:id|name)\s*=\s*["']{}["']"#,
        regex::escape(name)
    ))
    .ok()?;
    text.lines().position(|l| re.is_match(l)).map(|i| i + 1)
}

/// The 1-based lines of `<!-- … -->` comments in the HTML `text` that start inside one.
pub fn html_literal_lines(text: &str) -> Vec<bool> {
    let mut out = vec![false];
    let mut open = false;
    let mut rest = text;
    while !rest.is_empty() {
        let (line, next) = rest.split_once('\n').unwrap_or((rest, ""));
        let mut l = line;
        loop {
            match open {
                true => match l.find("-->") {
                    Some(i) => {
                        open = false;
                        l = &l[i + 3..];
                    }
                    None => break,
                },
                false => match l.find("<!--") {
                    Some(i) => {
                        open = true;
                        l = &l[i + 4..];
                    }
                    None => break,
                },
            }
        }
        out.push(open);
        rest = next;
    }
    out.truncate(text.lines().count().max(1));
    out
}

/// The lines of a stylesheet that declare `word`, as `u` marks them: a custom property's
/// `--word:`, a variable, a mixin, a function, a placeholder, a keyframes name, and a selector
/// that writes the class or the id, which [`css_declares`] then holds to what its rule styles.
pub fn css_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let mut out = named_patterns(word);
    if !word.starts_with("--") {
        out.push(format!(r"[.#]{w}(?:[^\w;-][^;]*)?[{{,]\s*$"));
    }
    out
}

/// [`css_patterns`] but the selector.
fn named_patterns(word: &str) -> Vec<String> {
    if word.starts_with("--") {
        return sheet_patterns(&Sheet::Custom(word.to_owned()));
    }
    let n = word.to_owned();
    [
        Sheet::Var(None, n.clone()),
        Sheet::LessVar(n.clone()),
        Sheet::Mixin(None, n.clone()),
        Sheet::Function(None, n.clone()),
        Sheet::Placeholder(n.clone()),
        Sheet::Keyframes(n),
    ]
    .iter()
    .flat_map(sheet_patterns)
    .collect()
}

/// Whether 1-based `line` of a stylesheet, which one of [`css_patterns`] matched, declares
/// `word`: a selector only when its rule styles the class or the id, as `d` reads it; any other
/// declaration line as it stands. `text` reads the file, and only for a selector.
pub fn css_declares(
    word: &str,
    line: usize,
    line_text: &str,
    text: impl FnOnce() -> String,
) -> bool {
    let named = Regex::new(&named_patterns(word).join("|")).is_ok_and(|re| re.is_match(line_text));
    named
        || rules(&text())
            .iter()
            .any(|r| r.line == line && r.classes.iter().chain(&r.ids).any(|n| n == word))
}

/// `line` of a stylesheet without its comments: a `/* … */` on it, and what follows a `//` outside
/// parentheses, where `url(//cdn…)` keeps its slashes.
pub fn css_code(line: &str) -> String {
    let (mut out, mut depth, mut rest) = (String::new(), 0usize, line);
    while let Some(c) = rest.chars().next() {
        if rest.starts_with("/*") {
            rest = rest[2..].split_once("*/").map_or("", |(_, r)| r);
            continue;
        }
        if rest.starts_with("//") && depth == 0 {
            break;
        }
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}
