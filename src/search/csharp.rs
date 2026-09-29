//! C#'s scopes: where a declaration stands (#355), a member of a type, private or not, or a local
//! of a method's body, which the search by name must not offer from anywhere else; and what a
//! method, a lambda or a block binds where the cursor is (#345).

use regex::Regex;

use super::*;

/// The keyword (`class`, `struct`, `interface`, `enum`, `record`) and the name of the C# type
/// the line declares.
pub fn cs_type_decl(line: &str) -> Option<(String, String)> {
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
    Some((c[1].to_owned(), c[2].to_owned()))
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

/// Where the declaration of `word` on 1-based `line` of `text` stands (#355): the lines above
/// indented less, as [`qualified`] walks them, told apart by whether they declare a type.
pub fn cs_place(text: &str, line: usize, word: &str) -> CsPlace {
    let lines: Vec<&str> = text.lines().collect();
    let Some(target) = line.checked_sub(1).and_then(|k| lines.get(k)) else {
        return CsPlace::Top;
    };
    let namespace = |l: &str| l.trim_start().starts_with("namespace ");
    let Some(first) = cs_enclosing(&lines, line - 1) else {
        return CsPlace::Top;
    };
    if namespace(lines[first]) {
        return CsPlace::Top;
    }
    if let Some((keyword, owner)) = cs_type_decl(lines[first]) {
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

// ---- bindings (#345) ----------------------------------------------------------------------
/// What a C# block header binds `name` to for the block under it (#345), as [`block_bindings`]
/// reads a header: whether it binds it, and whether for that block (so it hides the scopes
/// around): the parameters of a method, a constructor, a local function, an operator, an indexer
/// or a primary constructor, a lambda's, a `foreach`, `for`, `catch`, `using` or `fixed`
/// variable, and a pattern's or an `out` variable in the header's condition. A lambda elsewhere
/// on the header's lines binds without hiding. A deconstruction binds nothing.
pub(super) fn cs_opener(header: &str, name: &str) -> (bool, bool) {
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
        if let Some((declared, _)) = cs_signature(before)
            && (c == b'(' || declared == "this")
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
    (binds, own)
}

/// The name the C# declaration whose parameter list opens after `before` gives, and whether it
/// declares a type (a primary constructor): `before` is the whole signature up to the `(`, with
/// a type, a type keyword, an operator or an access modifier in it, so a call such as
/// `var order = new Order` or `new Order` is none.
fn cs_signature(before: &str) -> Option<(String, bool)> {
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
    declares.then(|| (sig["name"].to_owned(), sig.name("kw").is_some()))
}

/// The method, constructor, local function or operator whose parameter list on the C# `line`
/// declares `name`, for the status line to read `Refunds.Register.callbackUrl`, as a local reads.
pub fn cs_parameter_of(line: &str, name: &str) -> Option<String> {
    code(Kind::CSharp, line)
        .filter(|&(_, c)| c == b'(')
        .find_map(|(open, _)| {
            let close = close_of(Kind::CSharp, line, open)?;
            let (declared, of_type) = cs_signature(line[..open].trim())?;
            (!of_type && cs_params(&line[open + 1..close - 1], name, false)).then_some(declared)
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
