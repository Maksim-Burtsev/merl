//! The word under the cursor and the access chain around it: what `d` is asked about,
//! and the declaration a line stands inside.

use std::ops::Range;

use regex::Regex;

use super::*;

/// The name a reader knows the declaration on 1-based `line` of `text` by: `name` behind what it
/// is declared in, `UserRepository.delete_user` for a method, `Outer.Inner.run` for a nested
/// one, the receiver type of a Go method, the type a Rust `impl … for Type` is for. `None` at
/// the top level. The enclosing declarations are the lines above indented less, named by the
/// [`SYMBOLS`] rows a file of `kind` is read with; the walk stops at the first enclosing line
/// they name nothing on (a `return {`, an `if`), so a declaration stays unqualified rather than
/// wrongly qualified. A YAML anchor names a value, not a container: YAML is never qualified.
pub fn qualified(kind: Kind, text: &str, line: usize, name: &str) -> Option<String> {
    static RECEIVER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+\(\s*(?:\w+\s+)?\*?\s*([A-Za-z_]\w*)").unwrap()
    });
    static FUNC: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^func\s+(?:\([^)]*\)\s*)?([A-Za-z_]\w*)").unwrap()
    });
    static PY_DEF: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)").unwrap());
    static IMPL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:unsafe\s+)?impl\b(?:\s*<[^{]*?>)?\s+(?:[\w:]+(?:<[^{]*?>)?\s+for\s+)?&?(?:\w+::)*([A-Za-z_]\w*)").unwrap()
    });
    static ROWS: std::sync::LazyLock<Vec<(Option<Kind>, Regex)>> = std::sync::LazyLock::new(|| {
        SYMBOLS
            .iter()
            .map(|(k, p)| {
                (
                    *k,
                    Regex::new(p).expect("built-in symbol patterns are valid"),
                )
            })
            .collect()
    });
    if kind == Kind::Yaml {
        return None;
    }
    let sep = separator(kind);
    let lines: Vec<&str> = text.lines().collect();
    let target = *lines.get(line.checked_sub(1)?)?;
    // Any other name on a Go function's line is the function's, a parameter or a named result,
    // and reads as a local of its body does (#100): `Load.err`, not the field `Issue.err`.
    if kind == Kind::Go
        && let Some(c) = FUNC.captures(target).filter(|c| &c[1] != name)
    {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    if let Some(c) = RECEIVER.captures(target).filter(|_| kind == Kind::Go) {
        return Some(format!("{}{sep}{name}", &c[1]));
    }
    // A field declared inside a method or in a constructor's parameters is the class's:
    // `Issue.repo`, not `Issue.__init__.repo`.
    if field_like(kind, target.trim_start(), name)
        && let Some(decl) = enclosing_type(kind, &lines, line - 1)
        && field_bindings(kind, text, decl, name)
            .iter()
            .any(|b| b.line == line)
        && let Some(ty) = type_name(kind, lines[decl - 1])
    {
        let owner = qualified(kind, text, decl, &ty).unwrap_or(ty);
        return Some(format!("{owner}{sep}{name}"));
    }
    // Any other name on a Python `def` line is a parameter (#100): `Recipes.get_one.slug`, as a
    // local of the body reads, not `Recipes.slug`, a field's name.
    if kind == Kind::Python
        && let Some(c) = PY_DEF.captures(target).filter(|c| &c[1] != name)
    {
        let owner = qualified(kind, text, line, &c[1]).unwrap_or_else(|| c[1].to_owned());
        return Some(format!("{owner}{sep}{name}"));
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    let mut depth = indent(target);
    let mut names = vec![name.to_owned()];
    for l in lines[..line - 1].iter().rev() {
        if depth == 0 {
            break;
        }
        let t = l.trim_start();
        // A lone `{` opens the body of a declaration wrapped over the lines above it, as
        // prettier writes a long TypeScript class header; it names nothing itself. Neither does
        // a C++ access specifier, which is a label inside the class, not a wall in front of it.
        let access = kind == Kind::C && matches!(t, "public:" | "private:" | "protected:");
        // `> extends Base<K> {` closes the type parameters of the header above it.
        // Python's `):` closes a class header or a signature wrapped over several lines (#100):
        // what it opens is named on the line the bracket opened on, further up.
        let closer = kind == Kind::Python && t.starts_with([')', ']']);
        if t.is_empty()
            || t == "{"
            || (kind == Kind::TsJs && t.starts_with('>'))
            || access
            || closer
            || indent(l) >= depth
            || ["#", "//", "/*", "*", "--"]
                .iter()
                .any(|c| t.starts_with(c))
        {
            continue;
        }
        depth = indent(l);
        let named = IMPL
            .captures(l)
            .filter(|_| kind == Kind::Rust)
            .map(|c| c[1].to_owned())
            .or_else(|| {
                ROWS.iter()
                    .filter(|(k, _)| {
                        *k == Some(kind) || (k.is_none() && shared_symbols(Some(kind)))
                    })
                    .find_map(|(_, re)| symbol_name(re, l))
            });
        match named {
            Some(n) => names.push(n),
            None => break,
        }
    }
    names.reverse();
    (names.len() > 1).then(|| names.join(sep))
}
/// Whether the trimmed line `t` can declare the field `name` of a class from inside one of its
/// functions: it assigns `self.name` or `this.name`, or, in TypeScript, it is a parameter behind
/// an access modifier. A cheap test before [`field_bindings`] decides.
fn field_like(kind: Kind, t: &str, name: &str) -> bool {
    static MODIFIED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@[\w$.]+(?:\([^)]*\))?\s*)*(?:(?:public|private|protected|readonly|override)\s+)+([\w$]+)").unwrap()
    });
    match kind {
        Kind::Python => in_method(t, name),
        Kind::TsJs => in_method(t, name) || MODIFIED.captures(t).is_some_and(|c| &c[1] == name),
        _ => false,
    }
}
/// Whether line `t` reaches the field `name` through `self.` or `this.`: an assignment inside a
/// method, as against a declaration in the class body or a constructor's parameters.
pub(super) fn in_method(t: &str, name: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    ["self.", "this."].iter().any(|p| {
        let at = format!("{p}{name}");
        t.match_indices(&at)
            .any(|(i, m)| !t[..i].ends_with(ident) && !t[i + m.len()..].starts_with(ident))
    })
}
/// The dotted or `::` chain in front of the word under the cursor: `["json"]` for
/// `json.load(`, `["os", "path"]` for `os.path.join(`, `["fs"]` for `fs::read(`. Empty when the
/// word stands alone. A TypeScript private field keeps its `#`: `["this", "#root"]`. A chain that
/// hangs off a call, an index or `?.` is empty too: `users` in `make_uow().users.x` is no name of
/// its own.
pub fn qualifier(line: &str, word_start: usize) -> Vec<String> {
    let mut before = &line[..word_start];
    let mut chain = Vec::new();
    while let Some(rest) = before
        .strip_suffix("::")
        .or_else(|| before.strip_suffix('.'))
    {
        // Python's `super().` reads as TypeScript's `super.` does: one name.
        if let Some(head) = rest.strip_suffix("super()")
            && !head.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            chain.insert(0, "super".to_owned());
            before = head;
            break;
        }
        let mut start = rest
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(0, |i| i + 1);
        if start == rest.len() {
            break;
        }
        if rest[..start].ends_with(".#") {
            start -= 1;
        }
        chain.insert(0, rest[start..].to_owned());
        before = &rest[..start];
    }
    // `f().`, `a[0].`, `a?.` or a line starting with `.` are left; a spread `...` or a range `..`
    // is not a member access.
    if before.ends_with('.') && !before.ends_with("..") {
        return Vec::new();
    }
    chain
}
/// The line a member access reads as when prettier has broken it in front of its dots (#100):
/// `return this.db` over `  .selectFrom(` is `return this.db.selectFrom(`. Gives the lines joined,
/// without their comments, and where the word that starts at byte `word_start` of line `at`
/// stands in them; `None` for a line that does not start with a dot.
pub fn unbroken(
    kind: Kind,
    lines: &[String],
    at: usize,
    word_start: usize,
) -> Option<(String, usize)> {
    let led = |l: &str| l.trim_start().starts_with('.') && !l.trim_start().starts_with("..");
    if kind != Kind::TsJs || !led(&lines[at]) {
        return None;
    }
    let mut joined = lines[at].trim_start().to_owned();
    let mut start = word_start - indent(&lines[at]);
    // ponytail: forty lines of one expression.
    for above in lines[at.saturating_sub(40)..at].iter().rev() {
        let code = uncommented(kind, above);
        let code = match led(&code) {
            true => code.trim(),
            false => code.trim_end(),
        };
        // A comment between two links, or a blank line, breaks nothing.
        if code.is_empty() {
            continue;
        }
        start += code.len();
        joined.insert_str(0, code);
        if !led(code) {
            return Some((joined, start));
        }
    }
    None
}
/// A TypeScript line with `a?.b` and `a!.b` in front of byte `start` written as the plain `a.b`
/// they are for a member lookup (#100), and where `start` stands in it.
pub fn plain_access(kind: Kind, line: &str, start: usize) -> (String, usize) {
    if kind != Kind::TsJs {
        return (line.to_owned(), start);
    }
    let before = line[..start].replace("?.", ".").replace("!.", ".");
    (format!("{before}{}", &line[start..]), before.len())
}
/// The call a member access hangs off, where [`qualifier`] has no name to start from:
/// `pkg.New(x).word`, `make_uow().users.word`, `new Repo().word`, or the cast: `(x as T).word`,
/// `i.(T).word`, `cast(T, x).word`. Gives the call without its arguments (a cast as written), what
/// it is worth ([`Value::Call`], [`Value::New`] or the [`Value::Type`] of a cast) and the names
/// between it and the word. Only a call that starts the expression: `a.b().c().word` hangs off a
/// call of a value nobody typed.
pub fn call_head(
    kind: Kind,
    line: &str,
    word_start: usize,
) -> Option<(String, Value, Vec<String>)> {
    let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut before = line[..word_start].strip_suffix('.')?;
    let mut fields = Vec::new();
    while !before.ends_with(')') {
        let start = before.rfind(|c: char| !is_name(c)).map_or(0, |i| i + 1);
        if start == before.len() {
            return None;
        }
        fields.insert(0, before[start..].to_owned());
        before = before[..start].strip_suffix('.')?;
    }
    // The `(` this `)` closes, as a scan that knows strings pairs them.
    let open = code(kind, before)
        .filter(|(_, c)| *c == b'(')
        .map(|(i, _)| i)
        .find(|&i| close_of(kind, before, i) == Some(before.len()))?;
    let mut start = before[..open]
        .rfind(|c: char| !(is_name(c) || c == '.'))
        .map_or(0, |i| i + 1);
    // `.c` in `a.b().c()` is no callee [`value_of`] reads, and nor is nothing at all.
    let callee = &before[start..open];
    if kind == Kind::TsJs
        && let Some(rest) = before[..start].trim_end().strip_suffix("new")
        && !rest.ends_with(is_name)
    {
        start = rest.len();
    }
    // `(x as T)` is read without its brackets.
    let written = &before[start..];
    let inner = match callee.is_empty() {
        true => &written[1..written.len() - 1],
        false => written,
    };
    match value_of(kind, inner) {
        Value::Call(name) => Some((format!("{name}()"), Value::Call(name), fields)),
        Value::New(name) => Some((format!("new {name}()"), Value::New(name), fields)),
        // A cast the chain hangs off, as written: `(x as T)`, `i.(T)`, `cast(T, x)`.
        value @ (Value::Type(_) | Value::Cast(..)) => {
            Some((inner.trim().to_owned(), value, fields))
        }
        _ => None,
    }
}
/// The word `d` asks about at byte `col` of `line`: [`word_at`], and in TypeScript a `#private`
/// name with its `#`, on the `#` as on the name (#100): `#addRoute` is no `addRoute`.
pub fn definition_word(kind: Option<Kind>, line: &str, col: usize) -> Option<(Range<usize>, &str)> {
    let extra = word_chars(kind, true);
    let hash = |i: usize| kind == Some(Kind::TsJs) && line.as_bytes().get(i) == Some(&b'#');
    // On the `#`, the word is the one right behind it.
    let (range, _) = word_at(line, if hash(col) { col + 1 } else { col }, extra)?;
    // A private name stands behind a `.` or starts a member's line, behind its modifiers. An
    // issue number in a comment, a CSS id or a hash route in a string is no such name.
    let private = |i: usize| {
        let before = line[..i].trim_end();
        before.ends_with('.')
            || before.split_whitespace().all(|w| {
                matches!(
                    w,
                    "static" | "readonly" | "async" | "get" | "set" | "accessor" | "declare"
                )
            })
    };
    let start = match range
        .start
        .checked_sub(1)
        .filter(|&i| hash(i) && private(i))
    {
        Some(i) => i,
        None => range.start,
    };
    Some((start..range.end, &line[start..range.end]))
}
/// The run of `[A-Za-z0-9_]` and `extra` characters at byte offset `col`, or the one that ends
/// there when the cursor sits right after a word. `extra` characters do not start or end a word.
pub fn word_at<'a>(line: &'a str, col: usize, extra: &str) -> Option<(Range<usize>, &'a str)> {
    let b = line.as_bytes();
    let word =
        |i: usize| b[i].is_ascii_alphanumeric() || b[i] == b'_' || extra.contains(b[i] as char);
    let mut i = col.min(b.len());
    if i == b.len() || !word(i) {
        if i > 0 && word(i - 1) {
            i -= 1;
        } else {
            return None;
        }
    }
    let mut start = i;
    while start > 0 && word(start - 1) {
        start -= 1;
    }
    let mut end = i + 1;
    while end < b.len() && word(end) {
        end += 1;
    }
    while start < end && extra.contains(b[start] as char) {
        start += 1;
    }
    while end > start && extra.contains(b[end - 1] as char) {
        end -= 1;
    }
    (start < end).then(|| (start..end, &line[start..end]))
}
