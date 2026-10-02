use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::*;

/// What a word inside a Rust attribute names, or inside a `cfg!(…)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustAttr {
    /// A word inside `derive(…)`: a derive macro.
    Derive,
    /// The attribute's own name, `test` or the `main` of `tokio::main`: an attribute macro.
    Macro,
    /// A word declared nowhere: a `cfg` predicate, a compiler attribute or its arguments, a key
    /// of a macro attribute's arguments, a path in front of a name.
    Nothing,
}

/// The attributes the compiler reads itself: no `pub macro` or crate declares them.
const COMPILER_ATTRIBUTES: &[&str] = &[
    "allow",
    "deny",
    "warn",
    "forbid",
    "expect",
    "inline",
    "repr",
    "must_use",
    "doc",
    "path",
    "non_exhaustive",
    "cfg",
    "cfg_attr",
    "cold",
    "deprecated",
    "track_caller",
    "no_mangle",
    "export_name",
    "link",
    "link_name",
    "link_section",
    "used",
    "macro_export",
    "macro_use",
    "automatically_derived",
    "ignore",
    "should_panic",
    "recursion_limit",
    "crate_type",
    "crate_name",
    "no_std",
    "no_main",
    "no_implicit_prelude",
    "feature",
    "target_feature",
    "global_allocator",
    "panic_handler",
    "windows_subsystem",
    "type_length_limit",
    "proc_macro",
    "proc_macro_derive",
    "proc_macro_attribute",
    "unsafe",
    "diagnostic",
];

/// Whether the word at bytes `start..end` of 0-based `line` of `lines` stands inside an attribute
/// (`#[…]` or `#![…]` opened before it and not closed, on its line or on a line above for one
/// rustfmt wrapped) or inside a `cfg!(…)`, and what it names there. `None` elsewhere, and for a
/// word in an attribute's value (`#[arg(value_parser = parse)]`), which is an expression like any
/// other, or in an attribute macro's nested argument list, which may name a project type.
pub fn rust_attribute(lines: &[String], line: usize, start: usize, end: usize) -> Option<RustAttr> {
    let cur = lines.get(line)?;
    let (text, stack, _) = frames(lines, line, start)?;
    let pathed = cur[end..].trim_start().starts_with("::");
    if stack.iter().any(|f| f.name == "cfg!") {
        return Some(RustAttr::Nothing);
    }
    let a = stack.iter().rposition(|f| f.attr)?;
    let inner = &stack[a + 1..];
    if stack[a..].iter().any(|f| f.value) {
        return None;
    }
    if inner
        .last()
        .is_some_and(|f| f.open == b'(' && f.name == "derive")
    {
        return Some(if pathed {
            RustAttr::Nothing
        } else {
            RustAttr::Derive
        });
    }
    if !inner.is_empty() {
        let after = cur[end..].trim_start();
        let key = after.starts_with('=') && !after[1..].starts_with(['=', '>']);
        return (key || compiler(&text, &stack[a..])).then_some(RustAttr::Nothing);
    }
    // The attribute's own name: nothing but a path stands between the `#[` and the word.
    static PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(\w+\s*::\s*)*$").unwrap());
    let head = &text[stack[a].at..];
    let word = &cur[start..end];
    let own = PATH.is_match(head) && !pathed;
    let compiler = head.trim().is_empty() && COMPILER_ATTRIBUTES.contains(&word);
    Some(if own && !compiler {
        RustAttr::Macro
    } else {
        RustAttr::Nothing
    })
}
pub fn rust_attribute_path(lines: &[String], line: usize, start: usize) -> bool {
    static PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\w+(?:::\w+)*$").unwrap());
    let Some((text, stack, in_string)) = frames(lines, line, start) else {
        return false;
    };
    let Some(a) = stack.iter().rposition(|f| f.attr) else {
        return false;
    };
    let cur = &lines[line];
    let content = cur[..start].rfind('"').and_then(|open| {
        let close = cur[start..].find('"')?;
        Some(&cur[open + 1..start + close])
    });
    in_string
        && stack.last().is_some_and(|f| f.value)
        && !compiler(&text, &stack[a..])
        && content.is_some_and(|c| PATH.is_match(c))
}
/// Whether the attribute of `stack` (its `#[` frame first) is one the compiler reads: its own
/// name, or the name of an argument list in it, `cfg_attr(test, allow(…))`.
fn compiler(text: &str, stack: &[Frame]) -> bool {
    let head = text[stack[0].at..].trim_start();
    let name = &head[..head
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(head.len())];
    COMPILER_ATTRIBUTES.contains(&name)
        || stack[1..]
            .iter()
            .any(|f| COMPILER_ATTRIBUTES.contains(&f.name.as_str()))
}
struct Frame {
    /// `#[`, `[`, `(` or `{`.
    open: u8,
    attr: bool,
    /// The name in front of a `(`, with its `!`.
    name: String,
    /// An `=` since the last `,` of this frame: a value.
    value: bool,
    /// Where the frame's content starts in the text [`frames`] reads.
    at: usize,
}
/// The brackets open before byte `start` of 0-based `line` of `lines`, read from the attribute a
/// line above opens when rustfmt wrapped one; with the text read and whether `start` stands in a
/// `"…"` string.
fn frames(lines: &[String], line: usize, start: usize) -> Option<(String, Vec<Frame>, bool)> {
    let cur = lines.get(line)?;
    // An attribute rustfmt wrapped starts on a line of its own, and the lines between it and
    // the cursor end in what opens or goes on with a list, or are comments. Any other line above
    // is left out: it may stand inside a string opened further up.
    // ponytail: at most eight lines above the cursor.
    let mut from = line;
    for i in (line.saturating_sub(8)..line).rev() {
        let t = lines[i].trim();
        if t.starts_with("#[") || t.starts_with("#![") {
            from = i;
            break;
        }
        if !t.ends_with([',', '(', '[']) && !t.starts_with("//") {
            break;
        }
    }
    let mut text = lines[from..line].join("\n");
    text.push('\n');
    text.push_str(&cur[..start]);
    let b = text.as_bytes();
    let mut stack: Vec<Frame> = Vec::new();
    let mut in_string = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                in_string = i >= b.len();
            }
            // A char literal, `'"'` or `'\''`, opens nothing; a lifetime is no literal.
            b'\'' if b.get(i + 2) == Some(&b'\'') => i += 2,
            b'\'' if b.get(i + 1) == Some(&b'\\') => {
                i += 3;
                while i < b.len() && b[i] != b'\'' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'#' if b.get(i + 1) == Some(&b'[') || text[i..].starts_with("#![") => {
                i += if b[i + 1] == b'[' { 1 } else { 2 };
                stack.push(Frame {
                    open: b'[',
                    attr: true,
                    name: String::new(),
                    value: false,
                    at: i + 1,
                });
            }
            b'[' | b'(' | b'{' => {
                let before = text[..i].trim_end();
                let name_at = before
                    .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '!')
                    .len();
                stack.push(Frame {
                    open: c,
                    attr: false,
                    name: if c == b'(' {
                        before[name_at..].to_owned()
                    } else {
                        String::new()
                    },
                    value: false,
                    at: i + 1,
                });
            }
            b']' | b')' | b'}' => {
                stack.pop();
            }
            b',' => {
                if let Some(f) = stack.last_mut() {
                    f.value = false;
                }
            }
            b'=' => {
                let prev = i.checked_sub(1).map(|j| b[j]);
                let next = b.get(i + 1).copied();
                let compare = matches!(prev, Some(b'=' | b'!' | b'<' | b'>'))
                    || matches!(next, Some(b'=' | b'>'));
                if !compare && let Some(f) = stack.last_mut() {
                    f.value = true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Some((text, stack, in_string))
}
/// The lines `d` greps outside the project for the macro a word of an attribute names: a
/// `pub macro W` (the standard library declares every built-in derive and attribute so), and a
/// `#[proc_macro_derive(W` for a derive or a `pub fn W` for an attribute, which
/// [`rust_proc_macro`] keeps only under its `#[proc_macro_attribute]`.
pub fn rust_macro_patterns(attr: RustAttr, word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let mut out = vec![format!(r"^\s*pub\s+macro\s+{w}\b")];
    match attr {
        RustAttr::Derive => out.push(format!(r"^\s*#\[proc_macro_derive\(\s*{w}\b")),
        RustAttr::Macro => out.push(format!(r"^\s*pub\s+fn\s+{w}\b")),
        RustAttr::Nothing => {}
    }
    out
}
/// Whether 1-based `line` of `text`, a hit of [`rust_macro_patterns`], declares the macro: a
/// `pub fn` counts only with `#[proc_macro_attribute]` among the attributes right above it.
pub fn rust_proc_macro(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
        return false;
    };
    if !lines[k].trim_start().starts_with("pub fn") {
        return true;
    }
    lines[..k]
        .iter()
        .rev()
        .map(|l| l.trim())
        .take_while(|t| t.starts_with("#[") || t.starts_with("//"))
        .any(|t| t.starts_with("#[proc_macro_attribute"))
}

static STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|union)\s+[A-Za-z_]\w*").unwrap()
});
static ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+[A-Za-z_]\w*").unwrap());
/// A struct-like variant's own line, `Name {`.
static BRACED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+(?:#\[[^\]]*\]\s*)*[A-Za-z_]\w*\s*\{\s*$").unwrap());

/// The 0-based line the Rust item or variant on 0-based line `k` of `lines` is declared in: the
/// nearest line above indented less, past attributes, comments, blank lines and what a wrapped
/// header puts on lines of their own (a `where`, a lone `{`).
fn rust_parent(lines: &[&str], k: usize) -> Option<usize> {
    let depth = indent(lines[k]);
    (0..k).rev().find(|&i| {
        let t = lines[i].trim();
        !(t.is_empty() || comment(Kind::Rust, t) || t.starts_with("#[") || t == "{" || t == "where")
            && indent(lines[i]) < depth
    })
}
/// The line pattern of a Rust field `word`, `name: T` behind a `pub(..)`: a field only when
/// [`rust_field_at`] says so.
pub fn rust_field_pattern(word: &str, public: bool) -> String {
    let vis = match public {
        true => r"pub\s+",
        false => r"(?:pub(?:\([^)]*\))?\s+)?",
    };
    format!(r"^\s+{vis}{}\s*:[^:]", regex::escape(word))
}
/// Whether 1-based `line` of `text`, a hit of [`rust_field_pattern`], declares a field: it sits
/// directly in a `struct`, a `union` or a struct-like variant `Name {` of an `enum`. A struct
/// literal's field, a parameter of a wrapped signature and a `let` sit in something else.
pub fn rust_field_at(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
        return false;
    };
    rust_parent(&lines, k).is_some_and(|p| {
        STRUCT.is_match(lines[p])
            || (BRACED.is_match(lines[p])
                && rust_parent(&lines, p).is_some_and(|e| ENUM.is_match(lines[e])))
    })
}
/// The line pattern of the Rust enum variant `word`: `Name,`, `Name(…)`, `Name {`, `Name = 1`,
/// behind its attributes. A variant only when [`rust_variant_at`] says so.
pub fn rust_variant_pattern(word: &str) -> String {
    format!(
        r"^\s+(?:#\[[^\]]*\]\s*)*{}\s*(?:\(|\{{|,|=[^=>]|$)",
        regex::escape(word)
    )
}
/// Whether 1-based `line` of `text` sits directly in an `enum`: with [`rust_variant_pattern`],
/// the line declares a variant.
pub fn rust_variant_at(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    line.checked_sub(1)
        .filter(|&k| k < lines.len())
        .and_then(|k| rust_parent(&lines, k))
        .is_some_and(|p| ENUM.is_match(lines[p]))
}
static FN_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^\s*(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*fn\s"#,
    )
    .unwrap()
});
static MOD_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z_]\w*\s*\{").unwrap());
/// The 0-based lines 0-based line `k` of `lines` stands inside, innermost first: the lines above
/// indented less, each less than the last, past what a wrapped header or a block's end puts on
/// lines of their own (`) -> T {`, `where`, `} else {`) and the lines of a literal.
fn rust_around(lines: &[&str], k: usize, literal: &[bool]) -> Vec<usize> {
    let mut around = Vec::new();
    let mut depth = indent(lines[k]);
    for i in (0..k).rev() {
        if depth == 0 {
            break;
        }
        let t = lines[i].trim();
        let aside = t.is_empty()
            || comment(Kind::Rust, t)
            || t.starts_with(['#', ')', ']', '}', '{', '>'])
            || t == "where"
            || literal.get(i).copied().unwrap_or(false);
        if aside || indent(lines[i]) >= depth {
            continue;
        }
        depth = indent(lines[i]);
        around.push(i);
    }
    around
}
/// The 0-based lines directly in the block opened on line `h` of `lines` (`None`: the file), at
/// the indent of its first level, outside literals.
fn rust_direct(lines: &[&str], h: Option<usize>, literal: &[bool]) -> Vec<usize> {
    let code = |i: usize| !lines[i].trim().is_empty() && !literal.get(i).copied().unwrap_or(false);
    let Some(h) = h else {
        return (0..lines.len())
            .filter(|&i| code(i) && indent(lines[i]) == 0)
            .collect();
    };
    let base = indent(lines[h]);
    let inside: Vec<usize> = (h + 1..rust_block_end(lines, h))
        .filter(|&i| code(i))
        .collect();
    let child = inside
        .iter()
        .map(|&i| indent(lines[i]))
        .filter(|&n| n > base)
        .min();
    inside
        .into_iter()
        .filter(|&i| Some(indent(lines[i])) == child)
        .collect()
}
/// The 0-based line that closes the block opened on 0-based line `h` of `lines`: the first `}`
/// indented no deeper than `h`, else the end of the text.
fn rust_block_end(lines: &[&str], h: usize) -> usize {
    let base = indent(lines[h]);
    (h + 1..lines.len())
        .find(|&i| indent(lines[i]) <= base && lines[i].trim_start().starts_with('}'))
        .unwrap_or(lines.len())
}
/// The glob `use`s a bare word at 1-based `line` of the Rust `text` can see a variant through,
/// as the paths in front of their `*`: those of the function around it, an indented `use` between
/// its `fn` line and the cursor, else those directly in its module, the innermost inline `mod`
/// around it or the file. The second value says it is the function's.
pub fn rust_glob_uses(text: &str, line: usize) -> (Vec<Vec<String>>, bool) {
    static GLOB: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+((?:\w+\s*::\s*)+)\*\s*;").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
        return (Vec::new(), false);
    };
    let literal = literal_lines(Kind::Rust, text);
    let around = rust_around(&lines, k, &literal);
    let globs = |range: &mut dyn Iterator<Item = usize>| -> Vec<Vec<String>> {
        range
            .filter_map(|i| GLOB.captures(lines[i]))
            .map(|c| {
                c[1].split("::")
                    .map(|p| p.trim().to_owned())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .collect()
    };
    if let Some(&f) = around.iter().find(|&&i| FN_LINE.is_match(lines[i])) {
        let own = globs(&mut (f + 1..k));
        if !own.is_empty() {
            return (own, true);
        }
    }
    let module = around
        .iter()
        .copied()
        .find(|&i| MOD_LINE.is_match(lines[i]));
    (
        globs(&mut rust_direct(&lines, module, &literal).into_iter()),
        false,
    )
}
/// Which of Rust's namespaces a name is looked up in: a macro call `w!`, the head of a path
/// `w::…` (a module or a type), or a value or a type anywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustNamespace {
    Macro,
    Path,
    Other,
}
/// Empty wherever something else may be what the name means, so the caller goes on as before:
/// on the item's own line; for a `mod word;`, whose declaration is its file; when any `use` of
/// the file names the word; when a generic parameter list around it declares the word; when a
/// function around it declares an item of the name other than directly in the innermost one (in
/// a block, in an outer function); and for a lowercase word, when the function mentions it before
/// the cursor, or on the cursor's line, other than as `word(`, `word!` or `word::`: a local the
/// rules cannot read counts as a local (#353 reads them).
pub fn rust_scope_items(text: &str, line: usize, word: &str, ns: RustNamespace) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
        return Vec::new();
    };
    let w = regex::escape(word);
    let rule = |p: &str| Regex::new(p).expect("an escaped name keeps the pattern valid");
    let vis =
        r#"^\s*(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*"#;
    let item = match ns {
        RustNamespace::Macro => rule(&def_patterns(Kind::Rust, word)[1]),
        RustNamespace::Path => rule(&format!(
            r"{vis}(?:struct|enum|union|trait|type|mod)\s+{w}\b"
        )),
        RustNamespace::Other => rule(&def_patterns(Kind::Rust, word)[0]),
    };
    let any_item = rule(&def_patterns(Kind::Rust, word)[..2].join("|"));
    let outline = rule(&format!(r"\bmod\s+{w}\s*;"));
    let uses = rule(&format!(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s[^;]*\b{w}\b[^;]*;"
    ));
    let generic = rule(&format!(
        r"(?:\bfn\s+\w+|\bimpl|\b(?:struct|enum|union|trait|type)\s+\w+)\s*<[^{{]*\b{w}\b"
    ));
    let literal = literal_lines(Kind::Rust, text);
    let code: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| match literal.get(i).copied().unwrap_or(false) {
            true => String::new(),
            false => uncommented(Kind::Rust, l),
        })
        .collect();
    let around = rust_around(&lines, k, &literal);
    // A `use` in a block around the cursor, the file's included: another block's is not in sight.
    let code = code.join("\n");
    let in_sight = uses.find_iter(&code).any(|m| {
        let at = m.start() + m.as_str().find("use").unwrap_or(0);
        let u = code[..at].matches('\n').count();
        rust_around(&lines, u, &literal)
            .first()
            .is_none_or(|b| around.contains(b))
    });
    if in_sight {
        return Vec::new();
    }
    let functions: Vec<usize> = around
        .iter()
        .copied()
        .filter(|&i| FN_LINE.is_match(lines[i]))
        .collect();
    let header_lines = around
        .iter()
        .copied()
        .chain(functions.first().map_or(k..k, |&f| f..k + 1));
    if header_lines
        .into_iter()
        .any(|i| generic.is_match(&uncommented(Kind::Rust, lines[i])))
    {
        return Vec::new();
    }
    if let Some(&f) = functions.first()
        && word.starts_with(|c: char| c.is_lowercase() || c == '_')
    {
        let mention = rule(&format!(r"(^|[^.\w])({w})\b(\s*(?:\(|!|::))?"));
        let mentioned = lines[f..=k].iter().any(|l| {
            mention
                .captures_iter(&uncommented(Kind::Rust, l))
                .any(|c| c.get(3).is_none())
        });
        if mentioned {
            return Vec::new();
        }
    }
    let found = |scope: Option<usize>| -> Vec<usize> {
        rust_direct(&lines, scope, &literal)
            .into_iter()
            .filter(|&i| item.is_match(lines[i]) && !outline.is_match(lines[i]))
            .filter(|&i| ns != RustNamespace::Macro || i < k)
            .collect()
    };
    let mut items = Vec::new();
    if let (Some(&inner), Some(&outer)) = (functions.first(), functions.last()) {
        let declared: Vec<usize> = (outer + 1..rust_block_end(&lines, outer))
            .filter(|&i| any_item.is_match(lines[i]) && !literal.get(i).copied().unwrap_or(false))
            .collect();
        if !declared.is_empty() {
            let own = found(Some(inner));
            if own.is_empty() || !declared.iter().all(|i| own.contains(i)) {
                return Vec::new();
            }
            items = own;
        }
    }
    let modules: Vec<Option<usize>> = around
        .iter()
        .copied()
        .filter(|&i| MOD_LINE.is_match(lines[i]))
        .map(Some)
        .chain(std::iter::once(None))
        .collect();
    let glob = rule(r"^\s*use\s+super\s*::\s*\*\s*;");
    for scope in modules {
        if !items.is_empty() {
            break;
        }
        items = found(scope);
        let through = scope.is_some_and(|m| {
            rust_direct(&lines, Some(m), &literal)
                .iter()
                .any(|&i| glob.is_match(lines[i]))
        });
        if !through {
            break;
        }
    }
    match items.contains(&k) {
        true => Vec::new(),
        false => items.into_iter().map(|i| i + 1).collect(),
    }
}
/// The byte index of the first of `stops` in the Rust code `s` outside brackets, `<…>` included,
/// and strings. The `:` of a `::` is none, nor the `=` of `==`, `=>`, `!=`, `<=`, `>=`.
fn top_stop(s: &str, stops: &[u8]) -> Option<usize> {
    let b = s.as_bytes();
    let mut depth = 0i32;
    for (i, c) in code(Kind::Rust, s) {
        let (prev, next) = (i.checked_sub(1).map(|j| b[j]), b.get(i + 1).copied());
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'>' if !matches!(prev, Some(b'-' | b'=')) => depth -= 1,
            _ if depth == 0 && stops.contains(&c) => {
                let colons = c == b':' && (next == Some(b':') || prev == Some(b':'));
                let compares = c == b'='
                    && (matches!(next, Some(b'=' | b'>'))
                        || matches!(prev, Some(b'=' | b'!' | b'<' | b'>')));
                if !colons && !compares {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
fn top_split(mut s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    while let Some(i) = top_stop(s, b",") {
        out.push(&s[..i]);
        s = &s[i + 1..];
    }
    out.push(s);
    out
}
/// A parameter's pattern: `p` without its `: Type`.
fn untyped(p: &str) -> &str {
    top_stop(p, b":").map_or(p, |i| &p[..i])
}
/// Whether the Rust pattern `pat` binds `name`: `w`, `mut w`, `ref w`, `&w`, `w @ …`, the `w` of
/// `(a, w)`, `Some(w)`, `Point { w, .. }` and `Point { x: w }`. A path, a variant, a macro
/// variable and a field in front of its `:` bind nothing.
fn binds(pat: &str, name: &str) -> bool {
    let mut bytes = vec![b' '; pat.len()];
    for (i, c) in code(Kind::Rust, pat) {
        bytes[i] = if c == 0 { b' ' } else { c };
    }
    let code = String::from_utf8_lossy(&bytes);
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    code.match_indices(name).any(|(i, _)| {
        let (head, tail) = (&code[..i], &code[i + name.len()..]);
        !head.ends_with(|c: char| ident(c) || matches!(c, '.' | '$' | '\'' | ':'))
            && !head.trim_end().ends_with("::")
            && !tail.starts_with(ident)
            && !tail
                .trim_start()
                .starts_with(['(', '{', '!', '<', '>', ':'])
    })
}
/// The pattern of the Rust `let` statement on `line`, as its byte range: from behind the `let`
/// to the `:`, `=` or `;` that ends it. `None` when the line is no `let`, and when the pattern
/// goes on past the line.
fn let_pattern(line: &str) -> Option<Range<usize>> {
    static LET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*let\s+").unwrap());
    let from = LET.find(line)?.end();
    top_stop(&line[from..], b":=;").map(|end| from..from + end)
}
pub fn rust_let_declares(line: &str, start: usize, name: &str) -> bool {
    let_pattern(line).is_some_and(|p| p.contains(&start) && binds(&line[p], name))
}
/// Whether the Rust code `t`, a line or a block's header, binds `name` for what follows it: an
/// `if let`, a `while let` or a `let` of a chain, a `for`, a `match` arm, a closure. `open` asks
/// for a header, whose closure binds only when its body is the block the header opens.
fn line_binds(t: &str, name: &str, open: bool) -> bool {
    static LET: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\bif|\bwhile|&&)\s+let\s+").unwrap());
    static FOR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:^|[\s:])for\s+(.+?)\s+in\s").unwrap());
    static CLOSURE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[(,={]|\bmove|\breturn|=>)\s*\|([^|]*)\|(\s*(?:->[^{]*)?\{?\s*$)?")
            .unwrap()
    });
    let lets = LET.find_iter(t).any(|m| {
        let rest = &t[m.end()..];
        binds(top_stop(rest, b"=").map_or(rest, |i| &rest[..i]), name)
    });
    let each = FOR.captures(t).is_some_and(|c| binds(&c[1], name));
    let closure = CLOSURE.captures_iter(t).any(|c| {
        (!open || c.get(2).is_some()) && top_split(&c[1]).iter().any(|p| binds(untyped(p), name))
    });
    // `Some(w) =>`, behind a guard `if …` at most; not a `match` on one line, nor a macro's rule.
    let arm = t.find("=>").is_some_and(|end| {
        let pat = t[..end].trim().trim_start_matches('|');
        let pat = pat.split(" if ").next().unwrap_or(pat);
        let statement = ["let ", "if ", "while ", "for ", "return", "."]
            .iter()
            .any(|k| pat.starts_with(k));
        !statement && !pat.contains("match ") && !pat.contains('$') && binds(pat, name)
    });
    lets || each || closure || arm
}
/// The 0-based line of the parameter `name` of the Rust function whose `fn` is on 0-based line
/// `f`, its list wrapped over lines or not. `self` is none.
fn rust_param(lines: &[&str], f: usize, name: &str) -> Option<usize> {
    static FN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bfn\s+\w+\s*").unwrap());
    let line = uncommented(Kind::Rust, lines[f]);
    let mut open = FN.find(&line)?.end();
    // `fn get<T: Fn(u32)>(`: past the generics.
    if line[open..].starts_with('<') {
        let mut depth = 0i32;
        let (end, _) = line[open..].char_indices().find(|&(i, c)| {
            match c {
                '<' => depth += 1,
                '>' if !line[..open + i].ends_with('-') => depth -= 1,
                _ => {}
            }
            depth == 0
        })?;
        open += end + 1;
    }
    if !line[open..].starts_with('(') {
        return None;
    }
    let (inner, last, _) = group(Kind::Rust, lines, f, open)?;
    if !top_split(&inner).iter().any(|p| binds(untyped(p), name)) {
        return None;
    }
    let declared = Regex::new(&format!(r"\b{}\s*:(?:[^:]|$)", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    (f..=last)
        .find(|&i| declared.is_match(&uncommented(Kind::Rust, lines[i])))
        .or_else(|| (f..=last).find(|&i| names(lines[i], name)))
}
pub(super) fn rust_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    const KEYWORDS: &[&str] = &[
        "self", "mut", "ref", "box", "crate", "super", "true", "false",
    ];
    if !name.starts_with(|c: char| c.is_lowercase() || c == '_')
        || name == "_"
        || KEYWORDS.contains(&name)
    {
        return Vec::new();
    }
    let literal = literal_lines(Kind::Rust, &lines.join("\n"));
    let code = |i: usize| uncommented(Kind::Rust, lines[i]).trim().to_owned();
    let found = |i: usize| Binding {
        line: i + 1,
        value: Value::Unknown,
    };
    if FN_LINE.is_match(lines[at]) {
        return rust_param(lines, at, name).map(found).into_iter().collect();
    }
    let mut out: Vec<Binding> = line_binds(&code(at), name, false)
        .then(|| found(at))
        .into_iter()
        .collect();
    let mut depth = indent(lines[at]);
    let mut i = at;
    while i > 0 && depth > 0 {
        i -= 1;
        let t = code(i);
        if t.is_empty() || literal[i] || comment(Kind::Rust, &t) || t.starts_with("#[") {
            continue;
        }
        let ind = indent(lines[i]);
        if ind > depth {
            continue;
        }
        if ind == depth {
            match let_pattern(lines[i]) {
                Some(p) if binds(&lines[i][p.clone()], name) => {
                    out.push(found(i));
                    return out;
                }
                Some(_) => {}
                // The pattern goes on below, up to the next statement.
                None if t.starts_with("let ") => {
                    let end = (i + 1..at)
                        .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= depth)
                        .unwrap_or(at);
                    if (i..end).any(|j| names(&code(j), name)) {
                        return Vec::new();
                    }
                }
                None => {}
            }
            continue;
        }
        // The header of the block the walk is in. `} else {` opens a block beside the one above
        // it, which binds nothing for it; `) -> T {`, and a lone `{` under a `where`, end a
        // function's header that starts at the line back at their indent.
        let end = i;
        let opener = |i: usize| {
            (0..i).rev().find(|&j| {
                let l = code(j);
                !l.is_empty()
                    && !literal[j]
                    && indent(lines[j]) <= ind
                    && !l.starts_with([')', '}', '{', '>', '#'])
                    && !l.starts_with("where")
            })
        };
        if t.starts_with('}') {
            if line_binds(&t, name, true) {
                out.push(found(i));
                return out;
            }
            depth = ind;
            match opener(i) {
                Some(j) => i = j,
                None => break,
            }
            continue;
        }
        let signature = (t == "{" || t.starts_with("where"))
            && opener(i).is_some_and(|j| FN_LINE.is_match(lines[j]));
        if t.starts_with(')') || signature {
            match opener(i) {
                Some(j) => i = j,
                None => break,
            }
        }
        if FN_LINE.is_match(lines[i]) {
            out.extend(rust_param(lines, i, name).map(found));
            return out;
        }
        let header: Vec<String> = (i..=end).map(code).collect();
        if line_binds(&header.join(" "), name, true) {
            out.push(found(i));
            return out;
        }
        depth = indent(lines[i]);
    }
    out
}
pub fn rust_method_pattern(word: &str) -> String {
    format!(
        r#"^\s+(?:(?:pub(?:\([^)]*\))?|async|unsafe|const|extern(?:\s+"[^"]*")?|default)\s+)*fn\s+{}\b"#,
        regex::escape(word)
    )
}
/// What a Rust method's `fn` line sits directly in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustOwner {
    /// `trait Tr {`: the trait's own method.
    Trait(String),
    /// `impl<…> Tr for X {`: an implementation of `Tr`'s method.
    ImplOf(String),
    /// `impl<…> X {`, no trait: an inherent method.
    Inherent,
    /// An `impl` whose header the rules do not read: one inside a `macro_rules!`, `impl const Tr
    /// for`, a header wrapped before its `for`.
    Unreadable,
}
/// How far a Rust method's own `pub` reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustVis {
    Pub,
    /// `pub(crate)`, `pub(super)`, `pub(in …)`: its own crate at most.
    Crate,
    /// No `pub`: its module and the modules below it.
    Private,
}
/// Whether 1-based `line` of `lines`, a hit of [`rust_method_pattern`], declares a method: the
/// nearest line above it indented less opens an `impl` or a `trait`, or it stands in a macro's
/// body, which an `impl` may expand it in (`Unreadable`). Its owner, its visibility (a trait
/// method's the trait's) and the 1-based line of the `impl`, `trait` or macro arm around it.
/// `None` for a `fn` at the top level, nested in a function, or in a `mod` block: none of them
/// can follow a `.`.
pub fn rust_method_at(lines: &[&str], line: usize) -> Option<(RustOwner, RustVis, usize)> {
    static TRAIT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:pub(?:\([^)]*\))?|unsafe|auto)\s+)*trait\s+([A-Za-z_]\w*)").unwrap()
    });
    static IMPL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:pub(?:\([^)]*\))?|unsafe|const|default)\s+)*impl\b").unwrap()
    });
    static IMPL_FOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl(?:\s*<[^{]*?>)?\s+(?:\w+::)*([A-Za-z_]\w*)(?:<[^{]*?>)?\s+for\s").unwrap()
    });
    static INHERENT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl(?:\s*<[^{]*?>)?\s+(?:\w+::)*[A-Za-z_]\w*").unwrap()
    });
    static VIS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*pub\s*(\()?").unwrap());
    static MACRO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?macro(?:_rules!|\s)").unwrap()
    });
    let k = line.checked_sub(1).filter(|&k| k < lines.len())?;
    let at = rust_parent(lines, k)?;
    let parent = lines[at];
    let owner = if let Some(c) = TRAIT.captures(parent) {
        RustOwner::Trait(c[1].to_owned())
    } else if !IMPL.is_match(parent) {
        // Inside a `macro_rules!` or a `macro`, at any depth of its arms.
        let mut up = at;
        while !MACRO.is_match(lines[up]) {
            up = rust_parent(lines, up)?;
        }
        RustOwner::Unreadable
    } else if let Some(c) = IMPL_FOR.captures(parent) {
        RustOwner::ImplOf(c[1].to_owned())
    } else {
        // The whole header on its line, or its `where` on the next: no `for` further down.
        let whole = parent.trim_end().ends_with('{')
            || lines
                .get(at + 1)
                .is_some_and(|l| l.trim_start().starts_with("where"));
        match whole && !parent.contains(" for ") && INHERENT.is_match(parent) {
            true => RustOwner::Inherent,
            false => RustOwner::Unreadable,
        }
    };
    // A trait's method has no `pub` of its own: it reaches as far as the trait.
    let at_vis = match owner {
        RustOwner::Trait(_) => parent,
        _ => lines[k],
    };
    let vis = match VIS.captures(at_vis) {
        Some(c) if c.get(1).is_some() => RustVis::Crate,
        Some(_) => RustVis::Pub,
        None => RustVis::Private,
    };
    Some((owner, vis, at + 1))
}
/// A package of a `Cargo.lock`: its directory name in a registry, `name-version`, and its name.
pub type LockPackage = (String, String);
/// The crates of `lock`, a `Cargo.lock`, that the package `from` reaches: itself and its
/// `dependencies` lists followed transitively (normal, dev and build alike), by name; with every
/// package the lock lists, to tell a registry crate by. `None` when the lock does not list
/// `from`.
pub fn cargo_reach(lock: &str, from: &str) -> Option<(Vec<String>, Vec<LockPackage>)> {
    let mut packages: Vec<(String, String, Vec<String>)> = Vec::new();
    let mut in_deps = false;
    for l in lock.lines() {
        let t = l.trim();
        if t == "[[package]]" {
            packages.push(Default::default());
            in_deps = false;
            continue;
        }
        let Some(p) = packages.last_mut() else {
            continue;
        };
        if let Some(v) = t.strip_prefix("name = ") {
            p.0 = v.trim_matches('"').to_owned();
        } else if let Some(v) = t.strip_prefix("version = ") {
            p.1 = v.trim_matches('"').to_owned();
        } else if let Some(v) = t.strip_prefix("dependencies = [") {
            in_deps = !v.contains(']');
            p.2.extend(quoted_names(v));
        } else if in_deps {
            in_deps = !t.starts_with(']');
            p.2.extend(quoted_names(t));
        } else if t.starts_with('[') {
            in_deps = false;
        }
    }
    packages.iter().find(|p| p.0 == from)?;
    let mut reached = vec![from.to_owned()];
    let mut i = 0;
    while i < reached.len() {
        let name = reached[i].clone();
        for p in packages.iter().filter(|p| p.0 == name) {
            for d in &p.2 {
                if !reached.contains(d) {
                    reached.push(d.clone());
                }
            }
        }
        i += 1;
    }
    let dirs = packages
        .into_iter()
        .map(|(n, v, _)| (format!("{n}-{v}"), n))
        .collect();
    Some((reached, dirs))
}
/// The crate names of a `Cargo.lock` dependency list: `"memchr"`, `"serde 1.0.1"`, `"x 1.0
/// (registry+…)"` are `memchr`, `serde`, `x`.
fn quoted_names(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|q| q.split_whitespace().next().map(str::to_owned))
}
pub fn cargo_package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for l in toml.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
        } else if in_package && let Some(v) = t.strip_prefix("name") {
            let v = v.trim_start().strip_prefix('=')?;
            return Some(v.trim().trim_matches('"').to_owned());
        }
    }
    None
}
/// Whether the item on 1-based `line` of `lines` carries a `#[stable(…)]` or `#[unstable(…)]`
/// among the attributes and doc comments right above it: the standard library marks so every
/// item it offers outside (the `missing_stability` check), and what has neither is its own.
pub fn rust_stability(lines: &[&str], line: usize) -> bool {
    lines[..line.saturating_sub(1).min(lines.len())]
        .iter()
        .rev()
        .map(|l| l.trim())
        .take_while(|t| t.starts_with("//") || !(t.is_empty() || t.ends_with(['{', '}', ';'])))
        .any(|t| t.starts_with("#[stable(") || t.starts_with("#[unstable("))
}
/// The Rust type written at the start of `s`, up to the `,`, `)`, `|`, `=`, `{`, `;` or `where`
/// that ends it outside brackets.
fn type_at(s: &str) -> &str {
    let b = s.as_bytes();
    let mut depth = 0i32;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'<' | b'(' | b'[' => depth += 1,
            b'>' if i > 0 && b[i - 1] == b'-' => {}
            b'>' | b')' | b']' if depth > 0 => depth -= 1,
            b',' | b')' | b'|' | b'=' | b'{' | b';' | b']' | b'}' if depth == 0 => {
                return s[..i].trim();
            }
            _ if depth == 0 && s[i..].starts_with(" where") => return s[..i].trim(),
            _ => {}
        }
    }
    s.trim()
}
/// The name of the type whose methods and fields a value written as `written` has, and the path
/// it is spelled behind (`crate::a::`): `&`, `&mut`, lifetimes, `mut`, `Box<T>`, `Rc<T>` and
/// `Arc<T>` are stripped, since a method call derefs through them, and a type's own arguments
/// dropped. `None` for what the project cannot declare the methods of: a primitive, a tuple, a
/// slice, a `dyn` or `impl` trait, a function, and the standard library's own types.
pub fn rust_type_name(written: &str) -> Option<(Vec<String>, String)> {
    const STD: &[&str] = &[
        "Option",
        "Result",
        "Vec",
        "VecDeque",
        "HashMap",
        "HashSet",
        "BTreeMap",
        "BTreeSet",
        "BinaryHeap",
        "LinkedList",
        "String",
        "Cow",
        "Cell",
        "RefCell",
        "Mutex",
        "RwLock",
        "Weak",
        "Pin",
        "PathBuf",
        "Path",
        "OsString",
        "OsStr",
        "Duration",
        "Instant",
    ];
    static PATH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^((?:[A-Za-z_]\w*::)*)([A-Za-z_]\w*)\s*(?:<(.*)>)?$").unwrap()
    });
    static LIFETIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^'\w+\s+").unwrap());
    let mut t = written.trim();
    loop {
        let before = t;
        t = t.trim_start_matches('&').trim_start();
        t = LIFETIME.find(t).map_or(t, |m| &t[m.end()..]);
        t = t.strip_prefix("mut ").unwrap_or(t).trim_start();
        let c = PATH.captures(t)?;
        let name = c.get(2)?.as_str();
        if matches!(name, "Box" | "Rc" | "Arc")
            && let Some(inner) = c.get(3)
        {
            t = inner.as_str().trim();
        }
        if t == before {
            break;
        }
    }
    let c = PATH.captures(t)?;
    let name = c[2].to_owned();
    if !name.starts_with(|c: char| c.is_ascii_uppercase()) || STD.contains(&name.as_str()) {
        return None;
    }
    let path = c[1]
        .split("::")
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    Some((path, name))
}
/// The type an `impl` header on `line` is for: `T` of `impl<…> T` and of `impl<…> Tr for T`.
pub fn rust_impl_type(line: &str) -> Option<String> {
    static IMPL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl\b(?:\s*<[^{]*?>)?\s+(?:[\w:]+(?:<[^{]*?>)?\s+for\s+)?&?(?:\w+::)*([A-Za-z_]\w*)").unwrap()
    });
    IMPL.captures(line).map(|c| c[1].to_owned())
}
pub fn rust_impl_trait(line: &str) -> Option<String> {
    static FOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl\b(?:\s*<[^{]*?>)?\s+(?:\w+::)*([A-Za-z_]\w*)(?:<[^{]*?>)?\s+for\s").unwrap()
    });
    FOR.captures(line).map(|c| c[1].to_owned())
}
/// The type of `self` on 0-based line `at` of `lines`: the `impl` the method around it sits in,
/// with its 0-based line. `None` in a trait's default method, where `self` is any implementor,
/// and outside an `impl`.
pub fn rust_self_type(lines: &[&str], at: usize) -> Option<(String, usize)> {
    static TRAIT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:pub(?:\([^)]*\))?|unsafe|auto)\s+)*trait\s").unwrap()
    });
    let mut depth = indent(lines.get(at)?);
    for i in (0..at).rev() {
        let t = lines[i].trim();
        if depth == 0 {
            break;
        }
        if t.is_empty()
            || comment(Kind::Rust, t)
            || t.starts_with(['#', '{', '}', ')', '>'])
            || t.starts_with("where")
            || indent(lines[i]) >= depth
        {
            continue;
        }
        depth = indent(lines[i]);
        if TRAIT.is_match(lines[i]) {
            return None;
        }
        if let Some(ty) = rust_impl_type(lines[i]) {
            return Some((ty, i));
        }
    }
    None
}
/// Whether an item of the Rust `text` takes a generic parameter `name`: a `fn`, an `impl`, a
/// `struct`, an `enum`, a `trait` or a `type` with `name` among its `<…>`.
pub fn rust_generic(text: &str, name: &str) -> bool {
    Regex::new(&format!(
        r"\b(?:fn\s+\w+|impl|struct\s+\w+|enum\s+\w+|union\s+\w+|trait\s+\w+|type\s+\w+)\s*<[^>{{;]*\b{}\b",
        regex::escape(name)
    ))
    .is_ok_and(|re| re.is_match(text))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustHolds {
    /// A type as written: `x: &T`, `|x: T|`, `let x: T = …`.
    Type(String),
    /// A struct literal of the type written: `let x = T { … }`.
    Literal(String),
    /// A call of an associated function of the type written: `let x = T::new(…)`.
    Assoc(String, String),
    /// A call of the function the path names: `let x = tmpdir()`.
    Call(Vec<String>),
}
/// What the Rust binding of `name` on 0-based line `at` of `lines`, a line [`rust_bindings`]
/// gave, says it holds: a parameter's or a `let`'s written type, or the expression a `let`
/// assigns when it is a struct literal or a call and nothing else (no `?`, no method behind it).
/// `None` for anything else: a pattern, a `for`, an arm, a closure parameter with no type.
pub fn rust_holds(lines: &[&str], at: usize, name: &str) -> Option<RustHolds> {
    static LITERAL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:[A-Za-z_]\w*::)*([A-Z]\w*)\s*\{").unwrap());
    static ASSOC: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^((?:[A-Za-z_]\w*::)*[A-Z]\w*)(?:::<[^()]*>)?::([a-z_]\w*)\s*\(").unwrap()
    });
    static CALL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^((?:[a-z_]\w*::)*[a-z_]\w*)\s*\(").unwrap());
    let n = regex::escape(name);
    let line = uncommented(Kind::Rust, lines.get(at)?);
    let t = line.trim();
    if t.starts_with("let ") {
        let head = Regex::new(&format!(r"^let\s+(?:mut\s+)?{n}\s*(:[^:]|=[^=])"))
            .expect("an escaped name keeps the pattern valid");
        let c = head.captures(t)?;
        let rest = &t[c.get(1)?.start() + 1..];
        if c[1].starts_with(':') {
            return Some(RustHolds::Type(type_at(rest).to_owned()));
        }
        let mut statement = rest.to_owned();
        let mut i = at;
        while !statement.trim_end().ends_with(';') {
            i += 1;
            if i >= lines.len() || i > at + 40 {
                return None;
            }
            statement.push(' ');
            statement.push_str(uncommented(Kind::Rust, lines[i]).trim());
        }
        let e = statement.trim().trim_end_matches(';').trim_end();
        // The bracket a match opens closes at the end of the expression.
        let whole = |open: usize| {
            let mut depth = 0i32;
            for (i, c) in e.char_indices().skip_while(|&(i, _)| i < open) {
                match c {
                    '(' | '{' | '[' => depth += 1,
                    ')' | '}' | ']' => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1 == e.len();
                        }
                    }
                    _ => {}
                }
            }
            false
        };
        if let Some(c) = LITERAL.captures(e) {
            let open = c.get(0)?.end() - 1;
            return whole(open).then(|| RustHolds::Literal(e[..open].trim().to_owned()));
        }
        if let Some(c) = ASSOC.captures(e) {
            let open = c.get(0)?.end() - 1;
            return whole(open).then(|| RustHolds::Assoc(c[1].to_owned(), c[2].to_owned()));
        }
        if let Some(c) = CALL.captures(e) {
            let open = c.get(0)?.end() - 1;
            let path = c[1].split("::").map(str::to_owned).collect();
            return whole(open).then_some(RustHolds::Call(path));
        }
        return None;
    }
    // A closure's typed parameter, `|dir: Dir, mut cmd: TestCommand|`.
    let closure = Regex::new(&format!(
        r"\|(?:[^|]*?[\s(,])?(?:mut\s+)?{n}\s*:([^:][^|]*)\|"
    ))
    .expect("an escaped name keeps the pattern valid");
    if let Some(c) = closure.captures(&line) {
        return Some(RustHolds::Type(type_at(&c[1]).to_owned()));
    }
    // A function's parameter, on the `fn` line or on a line of its wrapped list.
    let param = Regex::new(&format!(r"(?:^|[(,])\s*(?:mut\s+)?{n}\s*:([^:].*)"))
        .expect("an escaped name keeps the pattern valid");
    let signature = FN_LINE.is_match(&line)
        || (0..at)
            .rev()
            .map(|i| lines[i])
            .take_while(|l| !l.trim_end().ends_with(['{', '}', ';']))
            .any(|l| FN_LINE.is_match(l));
    if signature && let Some(c) = param.captures(&line) {
        return Some(RustHolds::Type(type_at(&c[1]).to_owned()));
    }
    None
}
/// The return type of the Rust function whose `fn` is on 0-based line `f` of `lines`, as
/// written behind its `->`.
pub fn rust_return_type(lines: &[&str], f: usize) -> Option<String> {
    let mut signature = String::new();
    for l in lines.iter().skip(f).take(30) {
        let l = uncommented(Kind::Rust, l);
        signature.push(' ');
        signature.push_str(l.trim());
        if l.contains('{') || l.trim_end().ends_with(';') {
            break;
        }
    }
    // The `->` after the parameters: the last one outside brackets.
    let b = signature.as_bytes();
    let mut depth = 0i32;
    let mut arrow = None;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' | b';' if depth == 0 => break,
            b'-' if depth == 0 && b.get(i + 1) == Some(&b'>') => arrow = Some(i + 2),
            _ => {}
        }
    }
    let t = type_at(&signature[arrow?..]);
    (!t.is_empty()).then(|| t.to_owned())
}
/// The field `word` directly in the `struct` or `union` declared on 0-based line `decl` of
/// `lines`: its 0-based line and its type as written.
pub fn rust_struct_field(lines: &[&str], decl: usize, word: &str) -> Option<(usize, String)> {
    if !STRUCT.is_match(lines.get(decl)?) {
        return None;
    }
    let field = Regex::new(&format!(
        r"^\s+(?:pub(?:\([^)]*\))?\s+)?{}\s*:([^:].*)",
        regex::escape(word)
    ))
    .expect("an escaped name keeps the pattern valid");
    let base = indent(lines[decl]);
    for (i, l) in lines.iter().enumerate().skip(decl + 1) {
        let t = l.trim();
        if !t.is_empty()
            && indent(l) <= base
            && !t.starts_with(['{', '#', '/'])
            && !t.starts_with("where")
        {
            break;
        }
        if let Some(c) = field.captures(l)
            && rust_parent(lines, i) == Some(decl)
        {
            return Some((i, type_at(&c[1]).to_owned()));
        }
    }
    None
}
pub fn rust_type_decl_pattern(name: &str) -> String {
    format!(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|union)\s+{}\b",
        regex::escape(name)
    )
}
