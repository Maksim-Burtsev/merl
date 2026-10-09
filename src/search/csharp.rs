//! C#'s scopes: where a declaration stands (#355), a member of a type, private or not, or a local
//! of a method's body, which the search by name must not offer from anywhere else; and what a
//! method, a lambda or a block binds where the cursor is (#345).

use regex::Regex;

use super::*;

pub struct CsTypeDecl {
    pub keyword: String,
    pub name: String,
}

pub fn cs_type_decl(line: &str) -> Option<CsTypeDecl> {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^\u{feff}?\s*(?:\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|internal|file|static|sealed|abstract|partial|readonly",
            r"|ref|unsafe|new)\s+)*",
            r"(class|struct|interface|enum|record)\b(?:\s+(?:class|struct)\b)?\s+@?(\w+)"
        ))
        .unwrap()
    });
    let c = TYPE.captures(line)?;
    Some(CsTypeDecl {
        keyword: c[1].to_owned(),
        name: c[2].to_owned(),
    })
}

/// Where the declaration of `word` on 1-based `line` of a C# `text` stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsPlace {
    /// Directly in the body of the type `owner`; `private` when only that type reaches it: a
    /// member of a `class`, `struct` or `record` written `private` or with no access modifier,
    /// as C# defaults it.
    Member { owner: String, private: bool },
    /// Inside the body of a method, a constructor, an accessor, a local function or a lambda: a
    /// local, seen only between 1-based lines `from` and `to`, those of the member around it.
    Local { from: usize, to: usize },
    /// At the top of a file or of a namespace.
    Top,
}

pub fn cs_place(text: &str, line: usize, word: &str) -> CsPlace {
    let lines: Vec<&str> = text.lines().collect();
    let Some(target) = line.checked_sub(1).and_then(|k| lines.get(k)) else {
        return CsPlace::Top;
    };
    let namespace = cs_namespace_line;
    let Some(first) = cs_enclosing(&lines, line - 1) else {
        return CsPlace::Top;
    };
    if namespace(lines[first]) {
        return CsPlace::Top;
    }
    if let Some(CsTypeDecl {
        keyword,
        name: owner,
    }) = cs_type_decl(lines[first])
    {
        return CsPlace::Member {
            private: matches!(keyword.as_str(), "class" | "struct" | "record")
                && cs_private(target, word),
            owner,
        };
    }
    // A body: the member it belongs to is the outermost line under a type or a namespace.
    let mut member = first;
    while let Some(e) = cs_enclosing(&lines, member) {
        if namespace(lines[e]) || cs_type_decl(lines[e]).is_some() {
            break;
        }
        member = e;
    }
    CsPlace::Local {
        from: member + 1,
        to: cs_block_end(&lines, member) + 1,
    }
}

/// Whether `line` opens a namespace, past the byte order mark a file's first line may carry.
pub(super) fn cs_namespace_line(line: &str) -> bool {
    line.trim_start_matches(['\u{feff}', ' ', '\t'])
        .starts_with("namespace ")
}

/// Whether a member line reads as private: `private` (not `private protected`), or no access
/// modifier at all, unless it implements an interface's member explicitly (`int I.Name`), which
/// the interface reaches.
fn cs_private(line: &str, word: &str) -> bool {
    static ACCESS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^\s*(?:\[[^\]]*\]\s*)*(?:(?:private|static|readonly|const|sealed|abstract|virtual",
            r"|override|partial|async|extern|unsafe|new|volatile|event|required|fixed|ref)\s+)*",
            r"(?:public|protected|internal|file)\b"
        ))
        .unwrap()
    });
    let explicit =
        Regex::new(&format!(r"\w\.{}\b", regex::escape(word))).is_ok_and(|re| re.is_match(line));
    !ACCESS.is_match(line) && !explicit
}

/// The 0-based index of the line that 0-based `k` of `lines` stands inside: the nearest line above
/// indented less, past blank lines, comments, preprocessor lines, a lone `{` and the tail of a
/// header wrapped over several lines.
pub(super) fn cs_enclosing(lines: &[&str], k: usize) -> Option<usize> {
    let depth = indent(lines[k]);
    (0..k).rev().find(|&i| {
        let t = lines[i].trim();
        !(t.is_empty()
            || comment(Kind::CSharp, t)
            || t.starts_with(['#', '{', ')', ':', '['])
            || t.starts_with("where ")
            || indent(lines[i]) >= depth)
    })
}

/// The 0-based index of the last line of the block the header on 0-based `k` opens: the `}` back
/// at its indent, else the line before the next one there.
fn cs_block_end(lines: &[&str], k: usize) -> usize {
    let depth = indent(lines[k]);
    for (j, l) in lines.iter().enumerate().skip(k + 1) {
        let t = l.trim();
        if t.is_empty() || comment(Kind::CSharp, t) || t.starts_with('#') || indent(l) > depth {
            continue;
        }
        if t.starts_with(['{', ')']) {
            continue;
        }
        return if t.starts_with('}') { j } else { j - 1 };
    }
    lines.len().saturating_sub(1)
}

/// What a C# block header binds `name` to for the block under it (#345), as [`block_bindings`]
/// reads a header: the parameters of a method, a constructor, a local function, an operator, an indexer
/// or a primary constructor, a lambda's, a `foreach`, `for`, `catch`, `using` or `fixed`
/// variable, and a pattern's or an `out` variable in the header's condition. A lambda elsewhere
/// on the header's lines binds without hiding. A deconstruction binds nothing.
pub(super) struct CsOpener {
    pub binds: bool,
    pub hides_outer_scopes: bool,
}

pub(super) fn cs_opener(header: &str, name: &str) -> CsOpener {
    // What follows a signature's parameters: constraints, a base call or list, and the body.
    static AFTER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:where\b[^{=;]*)?(?::[^{;=]*)?(?:\{.*|=>.*|;)?\s*$").unwrap()
    });
    let h = header.replace('\n', " ");
    let n = regex::escape(name);
    let found = |p: String| Regex::new(&p).is_ok_and(|re| re.is_match(&h));
    let ty = r"(?:var|[\w.]+(?:<[^()]*?>)?\??(?:\[[,\s]*\])*)";
    let mut own = [
        format!(r"\bforeach\s*\(\s*[^;()]*?\s{n}\s+in\b"),
        format!(r"\bfor\s*\(\s*[^;()]*?\s{n}\s*="),
        format!(r"\bcatch\s*\(\s*[\w.<>]+\s+{n}\s*\)"),
        format!(r"\b(?:using|fixed)\s*\(\s*[^;()=]*?\s{n}\s*="),
        format!(r"\bis\s+{ty}\s+{n}\b"),
        format!(r"\bcase\s+{ty}\s+{n}\s*(?::|\bwhen\b)"),
        format!(r"\bout\s+{ty}\s+{n}\b"),
    ]
    .into_iter()
    .any(found);
    let mut binds = own;
    // `x =>` and `async x =>`: a lambda's body opening at the end of the header is the block.
    if let Some(c) = Regex::new(&format!(r"(?:^|[^\w.]){n}\s*=>(\s*\{{?\s*$)?"))
        .ok()
        .and_then(|re| re.captures_iter(&h).last())
    {
        binds = true;
        own |= c.get(1).is_some();
    }
    for (open, c) in code(Kind::CSharp, &h).filter(|&(_, c)| c == b'(' || c == b'[') {
        let Some(close) = close_of(Kind::CSharp, &h, open) else {
            continue;
        };
        let (before, after) = (h[..open].trim_end(), &h[close..]);
        let params = &h[open + 1..close - 1];
        // A signature's parameters bind for its body: a method's, a constructor's, a local
        // function's, an operator's, an indexer's `this[…]`, a primary constructor's.
        if let Some(signature) = cs_signature(before)
            && (c == b'(' || signature.name == "this")
            && AFTER.is_match(after)
        {
            if cs_params(params, name, false) {
                binds = true;
                own = true;
            }
            continue;
        }
        // `(x, y) =>`, `(T x, U y) =>`, `async (x) =>`.
        if c == b'(' && after.trim_start().starts_with("=>") && cs_params(params, name, true) {
            binds = true;
            let body = after.trim_start()[2..].trim();
            own |= body.is_empty() || body == "{";
        }
    }
    CsOpener {
        binds,
        hides_outer_scopes: own,
    }
}

struct CsSignature {
    name: String,
    primary_constructor: bool,
}

/// `before` is the whole signature up to the `(`, with a type, a type keyword, an operator or an
/// access modifier in it, so a call such as `var order = new Order` or `new Order` is none.
fn cs_signature(before: &str) -> Option<CsSignature> {
    static SIG: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^(?:\[[^\]]*\]\s*)*(?P<mods>(?:(?:public|private|protected|internal|file|static",
            r"|readonly|sealed|abstract|virtual|override|partial|async|extern|unsafe|new|implicit",
            r"|explicit|ref)\s+)*)(?:(?P<kw>class|struct|interface|record(?:\s+class|\s+struct)?)",
            r"\s+|(?P<ty>",
            cs_type!(),
            r")\s+)?(?:[\w.]+\.)?(?P<name>@?\w+|operator\s*\S+?)\s*",
            cs_generics!(),
            r"$"
        ))
        .unwrap()
    });
    let sig = SIG.captures(before)?;
    let declares = sig.name("kw").is_some()
        || sig.name("ty").is_some()
        || sig["name"].starts_with("operator")
        || sig["mods"].split_whitespace().any(|m| {
            matches!(
                m,
                "public" | "private" | "protected" | "internal" | "static"
            )
        });
    declares.then(|| CsSignature {
        name: sig["name"].to_owned(),
        primary_constructor: sig.name("kw").is_some(),
    })
}

/// The method, constructor, local function or operator whose parameter list on the C# `line`
/// declares `name`, for the status line to read `Refunds.Register.callbackUrl`, as a local reads.
pub fn cs_parameter_of(line: &str, name: &str) -> Option<String> {
    code(Kind::CSharp, line)
        .filter(|&(_, c)| c == b'(')
        .find_map(|(open, _)| {
            let close = close_of(Kind::CSharp, line, open)?;
            let signature = cs_signature(line[..open].trim())?;
            (!signature.primary_constructor && cs_params(&line[open + 1..close - 1], name, false))
                .then_some(signature.name)
        })
}

/// Whether a C# parameter list declares `name`: the last name of a parameter, past its
/// attributes, modifiers and type, before its default. A lambda's may be bare names.
fn cs_params(params: &str, name: &str, bare: bool) -> bool {
    static LAST: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"@?(\w+)\s*$").unwrap());
    split_top(Kind::CSharp, params, b',').into_iter().any(|p| {
        let mut p = split_top(Kind::CSharp, p, b'=')[0].trim();
        while p.starts_with('[')
            && let Some(end) = close_of(Kind::CSharp, p, 0)
        {
            p = p[end..].trim_start();
        }
        (bare || p.contains(char::is_whitespace)) && LAST.captures(p).is_some_and(|c| &c[1] == name)
    })
}

/// Whether a C# statement at a block's level declares `name` for the lines below it (#345):
/// `var x =`, `T x =`, `T x;`, `using var x =`, a local function, and an `out` or a pattern
/// variable anywhere in it. A deconstruction declares nothing here.
pub(super) fn cs_statement(t: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let ty = cs_type!();
    let ty_short = r"(?:var|[\w.]+(?:<[^()]*?>)?\??(?:\[[,\s]*\])*)";
    [
        format!(
            r"^(?:(?:await\s+)?using\s+|const\s+|ref\s+(?:readonly\s+)?|scoped\s+|static\s+|async\s+|unsafe\s+)*{ty}\s+{n}\s*(?:=[^=>]|=$|;|,|$|(?:<[^()]*>)?\s*\()"
        ),
        format!(r"\bout\s+{ty_short}\s+{n}\b"),
        format!(r"\bis\s+{ty_short}\s+{n}\b"),
    ]
    .iter()
    .any(|p| Regex::new(p).is_ok_and(|re| re.is_match(t)))
}

/// Whether `name` at byte `at` of a C# `line` is bound by a declaration on that same line
/// (#345), which the scope walk read there: the name is that declaration, the first place the
/// line names it, or stands past it; a lambda's parameter binds inside its lambda only, so
/// `xs.Where(x => x.A).Select(x => x.B)` binds each `x` to its own.
pub fn cs_binds_here(line: &str, name: &str, at: usize) -> bool {
    let Ok(word) = Regex::new(&format!(r"(?:^|[^\w.]){}\b", regex::escape(name))) else {
        return false;
    };
    // The last bracket opened before `upto` and still open there.
    let unclosed = |upto: usize| {
        let mut stack = Vec::new();
        for (i, c) in code(Kind::CSharp, &line[..upto]) {
            match c {
                b'(' | b'[' | b'{' => stack.push(i),
                b')' | b']' | b'}' => {
                    stack.pop();
                }
                _ => {}
            }
        }
        stack.last().copied()
    };
    let places: Vec<usize> = word
        .find_iter(line)
        .map(|m| m.end() - name.len())
        .filter(|&d| d <= at)
        .collect();
    for (k, &decl) in places.iter().enumerate() {
        let rest = &line[decl + name.len()..];
        // A lambda starts at the name, or at the `(` of the list it stands in.
        let start = if rest.trim_start().starts_with("=>") {
            Some(decl)
        } else {
            unclosed(decl).filter(|&p| {
                close_of(Kind::CSharp, line, p)
                    .is_some_and(|close| line[close..].trim_start().starts_with("=>"))
            })
        };
        // Standing on the declaration: the first place, or a lambda's parameter.
        if decl == at {
            return k == 0 || start.is_some();
        }
        match start {
            Some(start) => {
                let arrow = line[start..].find("=>").map_or(start, |i| start + i);
                let end = unclosed(start)
                    .and_then(|p| close_of(Kind::CSharp, line, p))
                    .unwrap_or(line.len());
                if at > arrow && at < end {
                    return true;
                }
            }
            None if k == 0 => return true,
            None => {}
        }
    }
    false
}

/// The 1-based line a C# binding made on `line` writes `name` on: that line, or, in a parameter
/// list wrapped one per line, the line below that holds it, as [`written_line`] finds it for
/// TypeScript; a name inside a string (`"/{id:int}"`) or a comment is none.
pub fn cs_written_line<S: AsRef<str>>(lines: &[S], line: usize, name: &str) -> usize {
    let code_only = |l: &str| {
        let mut out = vec![b' '; l.len()];
        for (i, c) in code(Kind::CSharp, l) {
            if c == 0 {
                break;
            }
            out[i] = l.as_bytes()[i];
        }
        String::from_utf8_lossy(&out).into_owned()
    };
    let masked: Vec<String> = lines.iter().map(|l| code_only(l.as_ref())).collect();
    written_line(&masked, line, name)
}

/// What a C# declaration gives a name, as far as its line tells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsValue {
    /// A type as written: `RedirectService`, `List<Item>`, `Label?`.
    Type(String),
    /// A call of the named method, `M`, `x.M` or `Type.M`, and whether it is awaited.
    Call { callee: String, awaited: bool },
    /// Nothing the rules read: `var x = y`, a lambda's parameter, `var` in a `foreach`.
    Unknown,
}

/// A C# type as written, with its generic arguments, nullability and array ranks.
const CS_TYPE: &str = concat!(r"(?:global::)?", cs_type!());

/// Words a type position may hold that name no type.
fn cs_keyword(t: &str) -> bool {
    matches!(
        t,
        "var"
            | "dynamic"
            | "void"
            | "return"
            | "await"
            | "new"
            | "else"
            | "yield"
            | "throw"
            | "in"
            | "is"
            | "as"
            | "case"
            | "using"
            | "ref"
            | "out"
            | "params"
            | "this"
            | "static"
            | "const"
            | "readonly"
            | "not"
    )
}

/// What the C# `line` declaring `name` gives it: `var x = new T()`, `T x = …`, `T x;`, a field, a
/// property, a parameter (`this T x` of an extension method too), `foreach (T x in …)`,
/// `catch (T x)`, `out T x` and a pattern `is T x`.
pub fn cs_declared(line: &str, name: &str) -> CsValue {
    let n = regex::escape(name.trim_start_matches('@'));
    let code = uncommented(Kind::CSharp, line);
    let written = |p: String| {
        Regex::new(&p)
            .ok()
            .and_then(|re| re.captures(&code).map(|c| c[1].to_owned()))
            .filter(|t| !cs_keyword(t))
    };
    if let Some(expr) = written(format!(r"\bvar\s+@?{n}\s*=\s*(.+)$")) {
        return cs_expr(&expr);
    }
    [
        format!(r"\b(?:is|case)\s+({CS_TYPE})\s+@?{n}\b"),
        format!(r"\bout\s+({CS_TYPE})\s+@?{n}\b"),
        format!(r"\bforeach\s*\(\s*({CS_TYPE})\s+@?{n}\s+in\b"),
        format!(r"\bcatch\s*\(\s*({CS_TYPE})\s+@?{n}\b"),
        format!(r"(?:^|[\s(,\[])({CS_TYPE})\s+@?{n}\s*(?:[=;,)\{{]|$)"),
    ]
    .into_iter()
    .find_map(written)
    .map_or(CsValue::Unknown, CsValue::Type)
}

/// What the C# expression `expr` gives: a construction `new T(…)`, a cast `(T)x`, `x as T`, or a
/// call `M(…)`, `await x.M(…)`. A construction or a call with something after it (`new T().U`) is
/// unknown.
pub fn cs_expr(expr: &str) -> CsValue {
    static NEW: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"^new\s+({CS_TYPE})\s*")).unwrap());
    static CAST: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"^\(({CS_TYPE})\)\s*[\w@(]")).unwrap());
    static AS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"^[\w@.]+\s+as\s+({CS_TYPE})$")).unwrap());
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(await\s+)?(@?[A-Za-z_]\w*(?:\.@?[A-Za-z_]\w*)*)\s*(?:<[^()]*>)?\s*\(")
            .unwrap()
    });
    let e = uncommented(Kind::CSharp, expr);
    let e = e.trim().trim_end_matches(';').trim_end();
    // What follows the bracket opening at `open`: nothing, or the lines the statement goes on to.
    let rest = |open: usize| close_of(Kind::CSharp, e, open).map_or("", |end| e[end..].trim());
    if let Some(c) = NEW.captures(e) {
        let end = c.get(0).unwrap().end();
        let after = match e[end..].starts_with('(') {
            true => rest(end),
            false => &e[end..],
        };
        return match after.is_empty() || after.starts_with('{') {
            true => CsValue::Type(c[1].to_owned()),
            false => CsValue::Unknown,
        };
    }
    if let Some(c) = CAST.captures(e).or_else(|| AS.captures(e)) {
        return CsValue::Type(c[1].to_owned());
    }
    if let Some(c) = CALL.captures(e) {
        let awaited = c.get(1).is_some();
        let after = rest(c.get(0).unwrap().end() - 1);
        let configured = awaited && after.starts_with(".ConfigureAwait(") && after.ends_with(')');
        if after.is_empty() || configured {
            return CsValue::Call {
                callee: c[2].replace('@', ""),
                awaited,
            };
        }
    }
    CsValue::Unknown
}

/// The return type the C# method declared on `line` as `name` writes in front of its name.
pub fn cs_returns(line: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let re = Regex::new(&format!(
        r"(?:^|\s)({CS_TYPE})\s+(?:[\w.]+\.)?@?{n}\s*(?:<[^()]*>)?\s*\("
    ))
    .ok()?;
    let t = re.captures(&uncommented(Kind::CSharp, line))?[1].to_owned();
    (!cs_keyword(&t)).then_some(t)
}

/// The name a written C# type comes down to: `List<Item>` is `List`, `Label?` is `Label`,
/// `eShop.Models.Item` is `Item`. An array, a tuple, `dynamic` and `var` have none.
pub fn cs_type_name(written: &str) -> Option<String> {
    let t = written.trim().trim_start_matches("global::");
    let t = t.trim_end_matches('?');
    if t.ends_with(']') || t.starts_with('(') {
        return None;
    }
    let t = match t.find('<') {
        Some(i) if t.ends_with('>') => &t[..i],
        Some(_) => return None,
        None => t,
    };
    let name = t.rsplit('.').next()?.trim();
    let word = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    (word && !cs_keyword(name) && !name.chars().next()?.is_ascii_digit()).then(|| name.to_owned())
}

/// What `await` makes of a C# `Task<T>` or `ValueTask<T>`: `T`.
pub fn cs_awaited(written: &str) -> Option<&str> {
    let t = written.trim().trim_start_matches("global::");
    let t = t.strip_prefix("System.Threading.Tasks.").unwrap_or(t);
    let inner = t
        .strip_prefix("Task<")
        .or_else(|| t.strip_prefix("ValueTask<"))?;
    Some(inner.strip_suffix('>')?.trim())
}

/// `s` cut at the commas outside brackets, generic arguments included.
fn cs_split(s: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, c) in code(Kind::CSharp, s) {
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// The header of the C# type declared on 1-based `decl` of `text`, over the lines it wraps onto,
/// up to its body.
fn cs_header(text: &str, decl: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let k = decl.checked_sub(1).filter(|&k| k < lines.len())?;
    let mut header = String::new();
    // ponytail: eight lines of header.
    for l in &lines[k..lines.len().min(k + 8)] {
        let l = uncommented(Kind::CSharp, l);
        let end = l.find(['{', ';']);
        header.push_str(&l[..end.unwrap_or(l.len())]);
        header.push(' ');
        if end.is_some() {
            break;
        }
    }
    Some(header)
}

/// The bases and interfaces the C# type declared on 1-based `decl` of `text` names after its `:`,
/// as written: `ContentView`, `IRepository<Item>`. A primary constructor's arguments to its base
/// are left out.
pub fn cs_bases(text: &str, decl: usize) -> Vec<String> {
    let Some(header) = cs_header(text, decl) else {
        return Vec::new();
    };
    let Some(CsTypeDecl { name, .. }) = cs_type_decl(&header) else {
        return Vec::new();
    };
    // Past the name, its type parameters and its primary constructor.
    let at = header.find(name.as_str()).map_or(0, |i| i + name.len());
    let mut rest = header[at..].trim_start();
    for (open, close) in [('<', '>'), ('(', ')')] {
        if rest.starts_with(open) {
            let mut depth = 0;
            let Some(end) = rest.char_indices().find_map(|(i, c)| {
                depth += (c == open) as i32 - (c == close) as i32;
                (depth == 0).then_some(i + 1)
            }) else {
                return Vec::new();
            };
            rest = rest[end..].trim_start();
        }
    }
    let Some(list) = rest.strip_prefix(':') else {
        return Vec::new();
    };
    let list = list.split(" where ").next().unwrap_or(list);
    cs_split(list)
        .into_iter()
        .map(|b| b.split('(').next().unwrap_or(b).trim().to_owned())
        .filter(|b| !b.is_empty())
        .collect()
}

/// The 1-based line of the C# type whose body 1-based `line` of `text` stands in: its own members,
/// or the body of one of them.
pub fn cs_owner(text: &str, line: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut k = line.checked_sub(1).filter(|&k| k < lines.len())?;
    while let Some(e) = cs_enclosing(&lines, k) {
        if cs_namespace_line(lines[e]) {
            return None;
        }
        if cs_type_decl(lines[e]).is_some() {
            return Some(e + 1);
        }
        k = e;
    }
    None
}

/// Whether `name` is a type parameter where 1-based `line` of `text` writes it: of the method, the
/// local function or the type around it, or of the declaration on the line itself.
pub fn cs_generic_param(text: &str, line: usize, name: &str) -> bool {
    static PARAMS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\w\s*<([^()<>]*)>\s*(?:\(|:|\{|\bwhere\b|$)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(mut k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
        return false;
    };
    let declares = |l: &str| {
        PARAMS.captures_iter(l).any(|c| {
            cs_split(&c[1]).iter().any(|p| {
                let p = p.trim();
                let p = p
                    .strip_prefix("in ")
                    .or_else(|| p.strip_prefix("out "))
                    .unwrap_or(p);
                p.trim() == name
            })
        })
    };
    loop {
        if declares(lines[k]) {
            return true;
        }
        match cs_enclosing(&lines, k) {
            Some(e) => k = e,
            None => return false,
        }
    }
}

pub fn cs_initialized(lines: &[String], at: usize, start: usize, end: usize) -> Option<String> {
    static NEW: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"\bnew\s+({CS_TYPE})$")).unwrap());
    static DECLARED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"(?:^|[\s(,])({CS_TYPE})\s+@?\w+(?:\s*\{{[^{{}}]*\}})?$"
        ))
        .unwrap()
    });
    let line = lines.get(at)?;
    let after = line.get(end..)?.trim_start();
    if !after.starts_with('=') || after.starts_with("==") || after.starts_with("=>") {
        return None;
    }
    let before = line[..start].trim_end();
    if !(before.is_empty() || before.ends_with(['{', ','])) {
        return None;
    }
    // ponytail: an initializer opening up to 200 lines above its member.
    let from = at.saturating_sub(200);
    let mut prefix = lines[from..at].join("\n");
    if at > from {
        prefix.push('\n');
    }
    prefix.push_str(&line[..start]);
    let mut stack = Vec::new();
    for (i, c) in code(Kind::CSharp, &prefix) {
        match c {
            b'(' | b'[' | b'{' => stack.push((i, c)),
            b')' | b']' | b'}' => {
                stack.pop();
            }
            _ => {}
        }
    }
    let &(open, b'{') = stack.last()? else {
        return None;
    };
    let head = uncommented(Kind::CSharp, &prefix[..open]);
    let mut head = head.trim_end();
    let mut args = false;
    if head.ends_with(')') {
        let mut depth = 0i32;
        let close = head.len() - 1;
        let open = (0..=close).rev().find(|&i| {
            depth += match head.as_bytes()[i] {
                b')' => 1,
                b'(' => -1,
                _ => 0,
            };
            depth == 0
        })?;
        head = head[..open].trim_end();
        args = true;
    }
    if let Some(c) = NEW.captures(head) {
        return Some(c[1].to_owned());
    }
    // `new { A = 1 }` is anonymous; `new() { … }` takes the type its declaration writes.
    let target = head
        .strip_suffix("new")
        .filter(|h| args && !h.ends_with(|c: char| c.is_alphanumeric() || c == '_'))?;
    let target = target.trim_end();
    if let Some(left) = target
        .strip_suffix('=')
        .filter(|l| !l.ends_with(['=', '!', '<', '>']))
    {
        let left = left.trim_end();
        let c = DECLARED.captures(left.lines().last()?.trim())?;
        return Some(c[1].to_owned()).filter(|t| !cs_keyword(t));
    }
    if target.ends_with("return")
        && !target[..target.len() - 6].ends_with(|c: char| c.is_alphanumeric() || c == '_')
    {
        // The method the `return` returns from: no lambda or local function in between.
        let k = from + target.matches('\n').count();
        let ls: Vec<&str> = lines.iter().map(String::as_str).collect();
        let mut e = cs_enclosing(&ls, k)?;
        loop {
            let header = ls[e].trim_start();
            if header.contains("=>") || header.contains("delegate") {
                return None;
            }
            let control = [
                "if",
                "else",
                "for",
                "foreach",
                "while",
                "do",
                "switch",
                "try",
                "catch",
                "finally",
                "using",
                "lock",
                "case",
                "default",
                "checked",
                "unchecked",
                "fixed",
            ]
            .iter()
            .any(|kw| {
                header.starts_with(kw)
                    && !header[kw.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
            });
            if !control {
                break;
            }
            e = cs_enclosing(&ls, e)?;
        }
        if cs_enclosing(&ls, e).is_none_or(|t| cs_type_decl(ls[t]).is_none()) {
            return None;
        }
        let name = cs_signature(uncommented(Kind::CSharp, ls[e]).split('(').next()?.trim())?.name;
        let written = cs_returns(ls[e], &name)?;
        let async_ = Regex::new(r"\basync\b").is_ok_and(|re| re.is_match(ls[e]));
        return match async_ {
            true => cs_awaited(&written).map(str::to_owned),
            false => Some(written),
        };
    }
    None
}

/// The written type of the `this` parameter of the C# extension method declared on `line`.
pub fn cs_extended(line: &str) -> Option<String> {
    static THIS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"\(\s*this\s+({CS_TYPE})\s+@?\w+")).unwrap()
    });
    Some(THIS.captures(line)?[1].to_owned())
}

pub fn cs_type_position(line: &str, start: usize, end: usize) -> bool {
    let (before, after) = (&line[..start], &line[end..]);
    if before.ends_with('.') || after.trim_start().starts_with('.') {
        return false;
    }
    if cs_named_after(after) {
        return true;
    }
    let b = before.trim_end();
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let keyword = |s: &str, k: &str| s.strip_suffix(k).is_some_and(|r| !r.ends_with(ident));
    if ["new", "is", "as"].iter().any(|k| keyword(b, k)) {
        return true;
    }
    if let Some(pre) = before.strip_suffix('(').map(str::trim_end) {
        if keyword(pre, "typeof") {
            return true;
        }
        // A cast: `(T)` with no call or keyword in front of its bracket, a value after it.
        let cast_after = Regex::new(concat!(
            r"^",
            cs_generics!(),
            r#"\??(?:\[[,\s]*\])*\s*\)\s*[\w@($"]"#
        ))
        .is_ok_and(|re| re.is_match(after))
            // `(Items) is null`: a keyword after the bracket, not a value being cast.
            && !Regex::new(r"^[^)]*\)\s*(?:is|as|switch|with|and|or|when)\b")
                .is_ok_and(|re| re.is_match(after));
        let opens = !pre.ends_with([')', ']', '>'])
            && (!pre.ends_with(ident)
                || ["return", "await", "throw"].iter().any(|k| keyword(pre, k)));
        if cast_after && opens {
            return true;
        }
    }
    cs_generic_argument(before, after) || cs_base_listed(line, before)
}

/// Whether a name follows the C# type that `after` continues, past its generic arguments, a `?`
/// and `[]`: `Buyer buyer`, `Buyer Update(`, save a contextual keyword (`x is T or U`).
fn cs_named_after(after: &str) -> bool {
    static FOLLOWED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            "^",
            cs_generics!(),
            r"\??(?:\[[,\s]*\])*\??\s+@?([A-Za-z_]\w*)"
        ))
        .unwrap()
    });
    const CONTEXTUAL: &[&str] = &[
        "is",
        "as",
        "in",
        "and",
        "or",
        "not",
        "when",
        "with",
        "switch",
        "by",
        "on",
        "equals",
        "into",
        "ascending",
        "descending",
        "select",
        "where",
        "orderby",
        "group",
        "join",
        "let",
        "from",
    ];
    FOLLOWED
        .captures(after)
        .is_some_and(|c| !CONTEXTUAL.contains(&&c[1]))
}

/// The rules of `def_patterns` a C# type position keeps (#360): a type, a delegate and a `using`
/// alias, and no constructor, property or namespace of the name.
pub fn cs_type_patterns(word: &str) -> Vec<String> {
    let all = def_patterns(Kind::CSharp, word);
    vec![all[0].clone(), all[1].clone(), all[3].clone()]
}

pub fn cs_constant_may_stand(line: &str, start: usize, end: usize) -> bool {
    let after = &line[end..];
    let is = line[..start].trim_end().strip_suffix("is");
    is.is_some_and(|b| !b.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
        && !after.starts_with(['<', '['])
        && !cs_named_after(after)
}

/// Whether the word between `before` and `after` is inside a generic argument list: a `<` right
/// after a name, closed by a `>` on the line, with nothing but names, `,`, `.`, `?`, `[]` and
/// nested lists in between. A `<` after a space is a comparison.
fn cs_generic_argument(before: &str, after: &str) -> bool {
    let typeish = |c: char| c.is_alphanumeric() || " \t_.,?[]@".contains(c);
    let mut depth = 0;
    let mut open = None;
    for (i, c) in before.char_indices().rev() {
        match c {
            '>' => depth += 1,
            '<' if depth == 0 => {
                open = Some(i);
                break;
            }
            '<' => depth -= 1,
            c if typeish(c) => {}
            _ => return false,
        }
    }
    let Some(open) = open else {
        return false;
    };
    if !before[..open].ends_with(|c: char| c.is_alphanumeric() || c == '_') {
        return false;
    }
    let mut depth = 0;
    for c in after.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth == 0 => return true,
            '>' => depth -= 1,
            c if typeish(c) => {}
            _ => return false,
        }
    }
    false
}

/// Whether the word after `before` is a base in the type header `line` declares: right after the
/// `:` of its base list or a `,` of it, outside any bracket.
fn cs_base_listed(line: &str, before: &str) -> bool {
    if cs_type_decl(line).is_none() || !before.trim_end().ends_with([':', ',']) {
        return false;
    }
    let mut depth = 0i32;
    let mut colon = false;
    for c in before.chars() {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth -= 1,
            ':' if depth == 0 => colon = true,
            _ => {}
        }
    }
    colon && depth == 0
}

#[derive(Debug, PartialEq, Eq)]
pub struct CsNamespacePrefix {
    pub prefix: String,
    pub certain: bool,
}

pub fn cs_namespace_prefix(line: &str, start: usize, end: usize) -> Option<CsNamespacePrefix> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\u{feff}?\s*(?:(?:global\s+)?using|namespace)\s+([\w.\s]*)$").unwrap()
    });
    let dotted = |s: &str| -> String { s.chars().filter(|c| !c.is_whitespace()).collect() };
    let rest = line[end..].trim_start();
    if let Some(c) = HEAD.captures(&line[..start])
        && !c[0].contains("static ")
        && (rest.starts_with(['.', ';', '{']) || rest.is_empty())
    {
        return Some(CsNamespacePrefix {
            prefix: dotted(&format!("{}{}", &c[1], &line[start..end])),
            certain: true,
        });
    }
    let at = line[..start].rfind("global::")?;
    let path = &line[at + "global::".len()..start];
    path.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        .then(|| CsNamespacePrefix {
            prefix: format!("{path}{}", &line[start..end]),
            certain: false,
        })
}

pub fn cs_arguments<S: AsRef<str>>(lines: &[S], line: usize, end: usize) -> Option<usize> {
    let text = lines.get(line)?.as_ref();
    let after = &text[end..];
    let generics = Regex::new(concat!("^", cs_generics!(), r"\s*\(")).ok()?;
    let open = end + generics.find(after)?.end() - 1;
    let rows: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let Group {
        inner_uncommented: inner,
        ..
    } = group(Kind::CSharp, &rows, line, open)?;
    cs_count(&inner)
}

#[derive(Debug, PartialEq, Eq)]
pub struct CsArity {
    pub fewest: usize,
    pub most_unless_params: Option<usize>,
    pub extension_this: bool,
}

/// `None` for a line declaring no method of the name, or a list not read.
pub fn cs_parameters(text: &str, line1: usize, word: &str) -> Option<CsArity> {
    let rows: Vec<&str> = text.lines().collect();
    let l = *rows.get(line1.checked_sub(1)?)?;
    if cs_type_decl(l).is_some() || Regex::new(r"\bdelegate\b").ok()?.is_match(l) {
        return None;
    }
    let head = Regex::new(&format!(
        concat!(r"\b{}\s*", cs_generics!(), r"\s*\("),
        regex::escape(word)
    ))
    .ok()?;
    let open = head.find(l)?.end() - 1;
    let Group {
        inner_uncommented: inner,
        ..
    } = group(Kind::CSharp, &rows, line1 - 1, open)?;
    // A declaration compares nothing, so every `<` opens a generic argument list.
    let parts = cs_split(&inner);
    let total = if inner.trim().is_empty() {
        0
    } else {
        parts.len()
    };
    let params = parts.iter().any(|p| p.trim_start().starts_with("params "));
    let optional = parts
        .iter()
        .filter(|p| p.contains('=') || p.trim_start().starts_with("params "))
        .count();
    let this = parts
        .first()
        .is_some_and(|p| p.trim_start().starts_with("this "));
    Some(CsArity {
        fewest: total - optional,
        most_unless_params: (!params).then_some(total),
        extension_this: this,
    })
}

/// The items of a C# call's `inner` text, cut at its top-level commas; `None` when a `<` makes it
/// read two ways, a comparison or a generic argument list.
fn cs_count(inner: &str) -> Option<usize> {
    if inner.trim().is_empty() {
        return Some(0);
    }
    let n = split_top(Kind::CSharp, inner, b',').len();
    (cs_split(inner).len() == n).then_some(n)
}
