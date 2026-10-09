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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsPlace {
    Member { owner: String, private: bool },
    Local { from: usize, to: usize },
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

pub(super) fn cs_namespace_line(line: &str) -> bool {
    line.trim_start_matches(['\u{feff}', ' ', '\t'])
        .starts_with("namespace ")
}

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

pub(super) struct CsOpener {
    pub binds: bool,
    pub hides_outer_scopes: bool,
}

pub(super) fn cs_opener(header: &str, name: &str) -> CsOpener {
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
    .any(found)
        || cs_deconstructs(&h, name);
    let mut binds = own;
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
        || cs_deconstructs(t, name)
}

fn cs_deconstructs(t: &str, name: &str) -> bool {
    static START: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|\bforeach\s*\(\s*)(var\s*)?\(").unwrap()
    });
    static IN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^in\b").unwrap());
    START.captures_iter(t).any(|c| {
        let open = c.get(0).unwrap().end() - 1;
        let Some(close) = close_of(Kind::CSharp, t, open) else {
            return false;
        };
        let after = t[close..].trim_start();
        let assigned = after
            .strip_prefix('=')
            .is_some_and(|r| !r.starts_with(['=', '>']))
            || IN.is_match(after);
        assigned && cs_tuple_names(&t[open + 1..close - 1], c.get(1).is_some(), name)
    })
}

fn cs_tuple_names(inner: &str, bare: bool, name: &str) -> bool {
    split_top(Kind::CSharp, inner, b',').into_iter().any(|e| {
        let e = e.trim();
        if let Some(nested) = e.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            return cs_tuple_names(nested, bare, name);
        }
        if let Some(nested) = e
            .strip_prefix("var")
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
        {
            return cs_tuple_names(nested, true, name);
        }
        let words: Vec<&str> = e.split_whitespace().collect();
        let declares = if bare { words.len() == 1 } else { words.len() >= 2 };
        declares && words.last().is_some_and(|w| w.trim_start_matches('@') == name)
    })
}

pub fn cs_binds_here(line: &str, name: &str, at: usize) -> bool {
    let Ok(word) = Regex::new(&format!(r"(?:^|[^\w.]){}\b", regex::escape(name))) else {
        return false;
    };
    let innermost_open_before = |upto: usize| {
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
        let start = if rest.trim_start().starts_with("=>") {
            Some(decl)
        } else {
            innermost_open_before(decl).filter(|&p| {
                close_of(Kind::CSharp, line, p)
                    .is_some_and(|close| line[close..].trim_start().starts_with("=>"))
            })
        };
        if decl == at {
            return k == 0 || start.is_some();
        }
        match start {
            Some(start) => {
                let arrow = line[start..].find("=>").map_or(start, |i| start + i);
                let end = innermost_open_before(start)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsValue {
    Type(String),
    Call { callee: String, awaited: bool },
    Unknown,
}

const CS_TYPE: &str = concat!(r"(?:global::)?", cs_type!());

fn cs_names_no_type(t: &str) -> bool {
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

pub fn cs_declared(line: &str, name: &str) -> CsValue {
    let n = regex::escape(name.trim_start_matches('@'));
    let code = uncommented(Kind::CSharp, line);
    let written = |p: String| {
        Regex::new(&p)
            .ok()
            .and_then(|re| re.captures(&code).map(|c| c[1].to_owned()))
            .filter(|t| !cs_names_no_type(t))
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
    let after_bracket =
        |open: usize| close_of(Kind::CSharp, e, open).map_or("", |end| e[end..].trim());
    if let Some(c) = NEW.captures(e) {
        let end = c.get(0).unwrap().end();
        let after = match e[end..].starts_with('(') {
            true => after_bracket(end),
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
        let after = after_bracket(c.get(0).unwrap().end() - 1);
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

pub fn cs_returns(line: &str, name: &str) -> Option<String> {
    let n = regex::escape(name);
    let re = Regex::new(&format!(
        r"(?:^|\s)({CS_TYPE})\s+(?:[\w.]+\.)?@?{n}\s*(?:<[^()]*>)?\s*\("
    ))
    .ok()?;
    let t = re.captures(&uncommented(Kind::CSharp, line))?[1].to_owned();
    (!cs_names_no_type(&t)).then_some(t)
}

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
    (word && !cs_names_no_type(name) && !name.chars().next()?.is_ascii_digit())
        .then(|| name.to_owned())
}

pub fn cs_awaited(written: &str) -> Option<&str> {
    let t = written.trim().trim_start_matches("global::");
    let t = t.strip_prefix("System.Threading.Tasks.").unwrap_or(t);
    let inner = t
        .strip_prefix("Task<")
        .or_else(|| t.strip_prefix("ValueTask<"))?;
    Some(inner.strip_suffix('>')?.trim())
}

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

pub fn cs_bases(text: &str, decl: usize) -> Vec<String> {
    let Some(header) = cs_header(text, decl) else {
        return Vec::new();
    };
    let Some(CsTypeDecl { name, .. }) = cs_type_decl(&header) else {
        return Vec::new();
    };
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
        return Some(c[1].to_owned()).filter(|t| !cs_names_no_type(t));
    }
    if target.ends_with("return")
        && !target[..target.len() - 6].ends_with(|c: char| c.is_alphanumeric() || c == '_')
    {
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
        let cast_after = Regex::new(concat!(
            r"^",
            cs_generics!(),
            r#"\??(?:\[[,\s]*\])*\s*\)\s*[\w@($"]"#
        ))
        .is_ok_and(|re| re.is_match(after))
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

fn cs_count(inner: &str) -> Option<usize> {
    if inner.trim().is_empty() {
        return Some(0);
    }
    let n = split_top(Kind::CSharp, inner, b',').len();
    (cs_split(inner).len() == n).then_some(n)
}
