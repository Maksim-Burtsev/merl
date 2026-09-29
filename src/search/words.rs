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
/// wrongly qualified. YAML is never qualified (see [`nests`]).
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
    // An Elixir module is named as written, `Shop.Pricing`, and one nested in it adds its own
    // name: a call spells the module out that way (#459).
    static EX_MODULE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*def(?:module|protocol)\s+([A-Z](?:[\w.]*\w)?)").unwrap()
    });
    static EX_DEF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*def(?:p|macrop?|guardp?|delegate)?\s+([\w?!]+)").unwrap()
    });
    if !nests(Some(kind)) {
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
    // local of the body reads, not `Recipes.slug`, a field's name. So is one on an Elixir
    // function's `def` line (#460).
    let def = match kind {
        Kind::Python => PY_DEF.captures(target),
        Kind::Elixir => EX_DEF.captures(target),
        _ => None,
    };
    if let Some(c) = def.filter(|c| &c[1] != name) {
        let owner = qualified(kind, text, line, &c[1]).unwrap_or_else(|| c[1].to_owned());
        return Some(format!("{owner}{sep}{name}"));
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    let mut depth = indent(target);
    let mut names = vec![name.to_owned()];
    // Ruby writes the namespace into the line (#387): `class A::B` is `B` inside `A`, and
    // `def Klass.m` is `m` of `Klass`.
    let ruby = kind == Kind::Ruby;
    if ruby {
        names.extend(ruby_namespace(target));
    }
    // C++ writes it in front of an out-of-line body (#508): `struct Drawer::Scanner {` is
    // `Scanner` inside `Drawer`, and `std::string Tariff::describe()` is `describe` of `Tariff`.
    let cpp = kind == Kind::C;
    if cpp {
        names.extend(cpp_namespace(target, name));
    }
    for l in lines[..line - 1].iter().rev() {
        if depth == 0 {
            break;
        }
        if steps_over(Some(kind), l.trim_start()) || indent(l) >= depth {
            continue;
        }
        depth = indent(l);
        // `class << self` opens the class around it, which the walk goes on to name.
        if ruby && l.trim_start().starts_with("class << self") {
            continue;
        }
        let named = IMPL
            .captures(l)
            .filter(|_| kind == Kind::Rust)
            .map(|c| c[1].to_owned())
            .or_else(|| {
                EX_MODULE
                    .captures(l)
                    .filter(|_| kind == Kind::Elixir)
                    .map(|c| c[1].to_owned())
            })
            .or_else(|| declared_name(Some(kind), l));
        match named {
            Some(n) => {
                if cpp {
                    names.push(n.clone());
                    names.extend(cpp_namespace(l, &n));
                } else {
                    names.push(n);
                }
            }
            None => break,
        }
        if ruby {
            names.extend(ruby_namespace(l));
        }
    }
    names.reverse();
    (names.len() > 1).then(|| names.join(sep))
}
/// `chain`, the names in front of an Elixir word, with the first read through an `alias` of
/// `text` (#459): `W` behind `alias Shop.Warehouse, as: W` is `Shop.Warehouse`, and `Courier`
/// behind `alias Shop.Warehouse.Courier` or `alias Shop.Warehouse.{Courier, Depot}` is
/// `Shop.Warehouse.Courier`. An `alias` counts above 0-based `line`, the cursor's, and only while
/// the block it is written in is still open there: none of the lines between is indented less
/// than it. Another module's `alias`, or one inside another function, renames nothing here; the
/// nearest one wins.
/// ponytail: a `{…}` group wrapped over several lines is not read.
pub fn elixir_unalias(text: &str, line: usize, mut chain: Vec<String>) -> Vec<String> {
    static ALIAS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*alias\s+([A-Z](?:[\w.]*\w)?)(?:\.\{([^}]*)\}|\s*,\s*as:\s*([A-Z]\w*))?")
            .unwrap()
    });
    let Some(first) = chain.first().cloned() else {
        return chain;
    };
    let last = |s: &str| s.rsplit('.').next() == Some(first.as_str());
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Elixir, text);
    // The least indent from the cursor's line up to the line being read.
    let mut floor = lines.get(line).map_or(usize::MAX, |l| indent(l));
    let full = (0..line.min(lines.len()))
        .rev()
        .filter(|&i| !literal[i] && !lines[i].trim().is_empty())
        .find_map(|i| {
            let open = indent(lines[i]) <= floor;
            floor = floor.min(indent(lines[i]));
            let c = ALIAS.captures(lines[i]).filter(|_| open)?;
            let module = &c[1];
            match (c.get(2), c.get(3)) {
                (Some(group), _) => group
                    .as_str()
                    .split(',')
                    .map(str::trim)
                    .find(|n| last(n))
                    .map(|n| format!("{module}.{n}")),
                (None, Some(named)) => (named.as_str() == first).then(|| module.to_owned()),
                (None, None) => last(module).then(|| module.to_owned()),
            }
        });
    if let Some(full) = full {
        chain.splice(..1, full.split('.').map(str::to_owned));
    }
    chain
}
/// The namespace a Ruby declaration line writes in front of its name, innermost first: `B`, `A`
/// for `class A::B::C`, `Klass` for `def Klass.m`. Empty for every other line.
fn ruby_namespace(line: &str) -> Vec<String> {
    static SPELLED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:class|module)\s+(?:::)?((?:\w+::)+)\w|def\s+([A-Z]\w*)\.\w)")
            .unwrap()
    });
    let Some(c) = SPELLED.captures(line) else {
        return Vec::new();
    };
    let spelled = c.get(1).or(c.get(2)).map_or("", |m| m.as_str());
    let mut names: Vec<String> = spelled
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    names.reverse();
    names
}
/// The qualifier a C++ declaration line writes in front of the `name` it declares, innermost
/// first: `Drawer`, `shop` for `int shop::Drawer::count() {`, `Box` for `T Box<T>::get()`. Only
/// the first `name` on the line counts: the `run` a body calls, `Other::run()`, is not it.
fn cpp_namespace(line: &str, name: &str) -> Vec<String> {
    let spelled = Regex::new(&format!(
        r"((?:\w+(?:<[^<>]*>)?::)*)\b{}\b",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let Some(c) = spelled.captures(line) else {
        return Vec::new();
    };
    // Written again before a `(`, the qualified one is a type in front of the name the line
    // declares: `typedef ns::Foo Foo;` declares a `Foo` of no `ns`.
    let rest = &line[c.get(0).map_or(0, |m| m.end())..];
    let rest = rest.split('(').next().unwrap_or(rest);
    let again = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    if again.is_match(rest) {
        return Vec::new();
    }
    let mut names: Vec<String> = c[1]
        .split("::")
        .filter(|s| !s.is_empty())
        .map(|s| s.split('<').next().unwrap_or(s).to_owned())
        .collect();
    names.reverse();
    names
}
/// The name `D` lists for `line` in a file of `kind`: the first [`SYMBOLS`] row such a file is
/// read with that names something on it.
fn declared_name(kind: Option<Kind>, line: &str) -> Option<String> {
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
    ROWS.iter()
        .filter(|(k, _)| *k == kind || (k.is_none() && shared_symbols(kind)))
        .find_map(|(_, re)| symbol_name(re, line))
}
/// Whether a file of `kind` names what it nests: a YAML anchor names a value, not a container,
/// so YAML qualifies no name and pins no header. Markdown declares nothing.
fn nests(kind: Option<Kind>) -> bool {
    !matches!(kind, Some(Kind::Yaml | Kind::Markdown))
}
/// Whether the trimmed line `t` is blank, a comment, an attribute or C's `#ifdef`: lines that
/// can stand at the left edge inside a body.
fn aside(t: &str) -> bool {
    t.is_empty()
        || ["#", "//", "/*", "*", "--"]
            .iter()
            .any(|c| t.starts_with(c))
}
/// Whether a walk up the indentation from a line to the declarations around it steps over the
/// trimmed line `t` of a file of `kind`: an [`aside`], or a line that names nothing itself but
/// belongs to the header above it.
fn steps_over(kind: Option<Kind>, t: &str) -> bool {
    // A lone `{` opens the body of a declaration wrapped over the lines above it, as prettier
    // writes a long TypeScript class header; it names nothing itself. Neither does a C++ access
    // specifier, which is a label inside the class, not a wall in front of it.
    let access = kind == Some(Kind::C) && matches!(t, "public:" | "private:" | "protected:");
    // `> extends Base<K> {` closes the type parameters of the header above it.
    // Python's `):` closes a class header or a signature wrapped over several lines (#100):
    // what it opens is named on the line the bracket opened on, further up.
    let closer = kind == Some(Kind::Python) && t.starts_with([')', ']']);
    aside(t) || t == "{" || (kind == Some(Kind::TsJs) && t.starts_with('>')) || access || closer
}
/// The declarations 0-based `line` of `lines` stands inside, outermost first, for the code pane
/// to pin over the text once their own lines scroll off (#248). merl has no parser: the scopes
/// are the lines above indented less, each less than the last, as VS Code's indentation model
/// reads them, and of those only the ones [`declared_name`] names something on count. A `for`
/// or an `if` is a scope too, so it moves the walk out, but pins nothing: the question the pins
/// answer is which function this is. A blank line or a comment belongs to the code after it, so
/// scrolling over the blank lines of a body keeps its header.
pub fn enclosing_declarations(kind: Option<Kind>, lines: &[String], line: usize) -> Vec<usize> {
    if !nests(kind) {
        return Vec::new();
    }
    let indent = |s: &str| s.len() - s.trim_start().len();
    // The tail of a header wrapped over several lines in any language (`) -> Result<()> {`,
    // `where`) belongs to what its first line opens, further up. `d`'s walk steps over fewer
    // of them ([`steps_over`]): widening it changes what `d` answers.
    let tail = |t: &str| t.starts_with([')', ']', '>', '{']) || t == "where";
    // The code at the top is the first line the walk would not step over; a tail counts, as
    // it keeps the header it ends.
    let code = |l: &&String| {
        let t = l.trim_start();
        tail(t) || !steps_over(kind, t)
    };
    let Some(first) = lines.get(line..).and_then(|rest| rest.iter().find(code)) else {
        return Vec::new();
    };
    // On a tail, the header it ends is its scope: the line that opened it is indented as much.
    let mut depth = indent(first) + usize::from(tail(first.trim_start()));
    let mut out = Vec::new();
    for (i, l) in lines[..line].iter().enumerate().rev() {
        if depth == 0 {
            break;
        }
        let t = l.trim_start();
        if steps_over(kind, t) || tail(t) || indent(l) >= depth {
            continue;
        }
        depth = indent(l);
        if declared_name(kind, l).is_some() {
            out.push(i);
        }
    }
    out.reverse();
    out
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
/// The byte the name at the end of `s` starts at: past the last character that is no name char,
/// or 0 where every character is one. `rfind` gives that character's first byte, and a character
/// outside ASCII is wider than the byte a slice at `i + 1` would assume (#150).
fn name_start(s: &str, is_name: impl Fn(char) -> bool) -> usize {
    s.char_indices()
        .rev()
        .find(|&(_, c)| !is_name(c))
        .map_or(0, |(i, c)| i + c.len_utf8())
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
        let mut start = name_start(rest, |c| c.is_ascii_alphanumeric() || c == '_');
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
        let start = name_start(before, is_name);
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
    let mut start = name_start(&before[..open], |c| is_name(c) || c == '.');
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
    // A Ruby method (#387) and an Elixir function (#459) take their `?` or `!` with them:
    // `empty?` is no `empty`, `ship!` no `ship`. The `!` of a `!=` is the operator's, as in Ruby
    // are `!~` and an instance or global variable, which has no suffix.
    let mut end = range.end;
    let rest = &line.as_bytes()[end..];
    let suffixed = match kind {
        Some(Kind::Ruby) => {
            !matches!(rest.get(1), Some(b'=' | b'~')) && !line[..start].ends_with(['@', '$'])
        }
        Some(Kind::Elixir) => rest.get(1) != Some(&b'='),
        _ => false,
    };
    if suffixed && matches!(rest.first(), Some(b'?' | b'!')) {
        end += 1;
    }
    Some((start..end, &line[start..end]))
}
/// The line 0-based `at` of `lines` stands directly inside: the nearest one above it indented
/// less that is no blank line or comment.
fn owner_line(lines: &[&str], at: usize) -> Option<usize> {
    let depth = indent(lines.get(at)?);
    (0..at)
        .rev()
        .find(|&i| !aside(lines[i].trim_start()) && indent(lines[i]) < depth)
}
/// Whether the Ruby method declared on 1-based `line` of `text` is a class method (#387):
/// `def self.m`, `def Const.m`, a `def` inside `class << self`, or one of a module that is
/// `extend self` or `module_function`.
pub fn ruby_singleton(text: &str, line: usize) -> bool {
    static ON: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*def\s+(?:self|[A-Z]\w*)\.").unwrap());
    static MODULE_WIDE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:extend\s+self|module_function)\s*(?:#|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(t) = line.checked_sub(1).and_then(|i| lines.get(i)) else {
        return false;
    };
    if ON.is_match(t) {
        return true;
    }
    let Some(owner) = owner_line(&lines, line - 1) else {
        return false;
    };
    let head = lines[owner].trim_start();
    // ponytail: a bare `module_function` anywhere in the module's body counts for every `def`
    // of it, the ones above it too; its list form, `module_function :m`, is not read.
    head.starts_with("class << self")
        || (head.starts_with("module ")
            && lines[owner + 1..]
                .iter()
                .take_while(|l| l.trim().is_empty() || indent(l) > indent(lines[owner]))
                .any(|l| MODULE_WIDE.is_match(l)))
}
/// Where the ActiveSupport concern `module` keeps the class methods it gives the class that
/// includes it (#387): whether the Ruby method on 1-based `line` of `text` is inside its
/// `class_methods do` block or its `module ClassMethods`.
pub fn ruby_concern_class_method(text: &str, line: usize, module: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(owner) = line.checked_sub(1).and_then(|i| owner_line(&lines, i)) else {
        return false;
    };
    let head = lines[owner].trim_start();
    let inside = |block: &str| {
        qualified(Kind::Ruby, text, owner + 1, block).is_some_and(|q| {
            q == format!("{module}.{block}") || q.ends_with(&format!(".{module}.{block}"))
        })
    };
    (head.starts_with("class_methods do") && inside("class_methods"))
        || (head.starts_with("module ClassMethods") && inside("ClassMethods"))
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
