use std::sync::LazyLock;

use super::returns::iterator_items;
use super::sysroot::rust_element;
use super::{
    App, Bound, Candidate, Hit, InCrate, Kind, MacroUse, Path, PathBuf, Reason, Regex, Typed,
    rust_bare_type, search,
};

struct Link {
    name: String,
    call: bool,
}

struct Walked {
    head: Vec<String>,
    head_call: bool,
    links: Vec<Link>,
    rest: usize,
}

pub(super) fn trailing_name(s: &str) -> &str {
    let start = s
        .char_indices()
        .rev()
        .find(|&(_, c)| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |(i, c)| i + c.len_utf8());
    s.get(start..).unwrap_or_default()
}

pub(super) fn opening(s: &str, close: u8, open: u8) -> Option<usize> {
    let b = s.as_bytes();
    let mut depth = 0i32;
    let mut quoted = false;
    for i in (0..b.len()).rev() {
        let c = b[i];
        if c == b'"' && (i == 0 || b[i - 1] != b'\\') {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        if c == close {
            depth += 1;
        } else if c == open {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn walk_back(expr: &str) -> Option<Walked> {
    let mut s = expr.trim_end();
    let mut links = Vec::new();
    loop {
        while let Some(tried) = s.strip_suffix('?') {
            links.insert(
                0,
                Link {
                    name: "?".to_owned(),
                    call: false,
                },
            );
            s = tried.trim_end();
        }
        let call = s.ends_with(')');
        if call {
            s = s.get(..opening(s, b')', b'(')?)?.trim_end();
            if s.ends_with('>') {
                let open = opening(s, b'>', b'<')?;
                s = s.get(..open)?.strip_suffix("::")?;
            }
        }
        let name = trailing_name(s);
        if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        s = s.get(..s.len() - name.len())?;
        if let Some(head) = s.strip_suffix('.').filter(|h| !h.ends_with('.')) {
            links.insert(
                0,
                Link {
                    name: name.to_owned(),
                    call,
                },
            );
            s = head.trim_end();
            continue;
        }
        let mut head = vec![name.to_owned()];
        while let Some(before) = s.strip_suffix("::") {
            let part = trailing_name(before);
            if part.is_empty() {
                return None;
            }
            head.insert(0, part.to_owned());
            s = before.get(..before.len() - part.len())?;
        }
        return Some(Walked {
            head,
            head_call: call,
            links,
            rest: s.len(),
        });
    }
}

pub(super) fn let_initializer(lines: &[&str], at: usize, name: &str) -> Option<String> {
    let t = lines.get(at)?.trim();
    let rest = t.strip_prefix("let ")?.trim_start();
    let rest = rest.strip_prefix("mut ").unwrap_or(rest).trim_start();
    let rest = rest.strip_prefix(name)?.trim_start().strip_prefix('=')?;
    statement(lines, at, rest)
}

fn nested(part: &str) -> Option<&str> {
    let inner = part.strip_prefix('(')?.strip_suffix(')')?;
    (opening(part, b')', b'(') == Some(0)).then_some(inner)
}

fn plain_name(part: &str) -> Option<&str> {
    let bare = part.strip_prefix("mut ").unwrap_or(part).trim();
    (!bare.is_empty() && bare.chars().all(|c| c.is_alphanumeric() || c == '_')).then_some(bare)
}

fn tuple_names(pattern: &str) -> bool {
    super::returns::top_level(pattern)
        .iter()
        .all(|p| nested(p).map_or_else(|| plain_name(p).is_some(), tuple_names))
}

fn tuple_path(pattern: &str, name: &str) -> Option<Vec<(usize, usize)>> {
    let parts = super::returns::top_level(pattern);
    parts.iter().enumerate().find_map(|(i, p)| {
        let mut below = match nested(p) {
            Some(inner) => tuple_path(inner, name)?,
            None => (plain_name(p)? == name).then(Vec::new)?,
        };
        below.insert(0, (i, parts.len()));
        Some(below)
    })
}

fn tuple_initializer(
    lines: &[&str],
    at: usize,
    name: &str,
) -> Option<(Vec<(usize, usize)>, String)> {
    let t = lines.get(at)?.trim().strip_prefix("let ")?.trim_start();
    let (pattern, rest) = t.split_once('=')?;
    let pattern = pattern.trim_end();
    let names = nested(pattern).filter(|n| tuple_names(n))?;
    let path = tuple_path(names, name)?;
    let init = statement(lines, at, rest)?;
    Some((path, init))
}

fn for_iterated<'a>(text: &'a str, at: usize, name: &str) -> Option<(&'a str, &'a str)> {
    let mut lines = text.lines().skip(at);
    let line = lines.next()?.trim();
    let rest = line.strip_prefix("for ")?.trim_start();
    let rest = rest.strip_prefix("mut ").unwrap_or(rest);
    let expr = rest.strip_prefix(name)?.trim_start().strip_prefix("in ")?;
    let expr = match expr.strip_suffix('{') {
        Some(expr) => expr,
        None => (lines.next()?.trim() == "{").then_some(expr)?,
    }
    .trim();
    let place = expr.strip_prefix('&').map_or(expr, |s| {
        let s = s.trim_start();
        s.strip_prefix("mut ").unwrap_or(s).trim()
    });
    Some((expr, place))
}

fn statement(lines: &[&str], at: usize, rest: &str) -> Option<String> {
    if rest.starts_with('=') {
        return None;
    }
    let mut statement = rest.trim().to_owned();
    let mut i = at;
    while !statement.ends_with(';') {
        i += 1;
        let next = lines.get(i).filter(|_| i <= at + 40)?.trim();
        if !next.starts_with("//") {
            statement.push_str(next);
        }
    }
    Some(statement.trim_end_matches(';').trim().to_owned())
}

struct Arm {
    line0: usize,
    pattern: String,
    expr: String,
}

fn match_arms(lines: &[&str], at: usize) -> Option<Vec<Arm>> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let head = lines.get(at)?;
    if !head.trim_end().ends_with('{') {
        return None;
    }
    let mut arms = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(at + 1).take(40) {
        let t = l.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t == "};" && indent(l) == indent(head) {
            return (!arms.is_empty()).then_some(arms);
        }
        let (pattern, expr) = t.split_once(" => ")?;
        let expr = expr.strip_suffix(',')?.trim();
        if indent(l) <= indent(head) || expr.contains(['{', '}']) || pattern.contains(['{', '}']) {
            return None;
        }
        arms.push(Arm {
            line0: i,
            pattern: pattern.to_owned(),
            expr: expr.to_owned(),
        });
    }
    None
}

fn wrapped_pattern<'a>(t: &'a str, name: &str) -> Option<(&'static str, &'a str)> {
    let (variant, rest) = t.split_once('(')?;
    let wrapper = match variant.trim_end() {
        "Some" => "Option",
        "Ok" => "Result",
        _ => return None,
    };
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("mut ").unwrap_or(rest);
    let rest = rest.strip_prefix(name)?.trim_start().strip_prefix(')')?;
    Some((wrapper, rest.trim_start()))
}

pub(super) fn arm_hides(line: &str, name: &str, col: usize) -> bool {
    let t = line.trim_start();
    wrapped_pattern(t, name)
        .and_then(|(_, rest)| rest.strip_prefix("=>"))
        .is_some_and(|body| col >= line.len() - body.len())
}

fn if_let_scrutinee(lines: &[&str], at: usize, rest: &str) -> Option<String> {
    let mut expr = rest.trim().to_owned();
    let mut i = at;
    loop {
        if let Some(e) = expr.strip_suffix('{') {
            return Some(e.trim_end().to_owned());
        }
        i += 1;
        let next = lines.get(i).filter(|_| i <= at + 5)?.trim();
        if next == "{" {
            return Some(expr);
        }
        expr = format!("{expr} {next}");
    }
}

fn match_scrutinee(lines: &[&str], at: usize) -> Option<String> {
    let indent = |l: &str| l.len() - l.trim_start().len();
    let arm = indent(lines.get(at)?);
    let i = (0..at)
        .rev()
        .take(40)
        .find(|&i| !lines[i].trim().is_empty() && indent(lines[i]) < arm)?;
    let mut head = lines[i].trim().strip_suffix('{')?.trim_end();
    if head.is_empty() {
        head = lines[i.checked_sub(1)?].trim();
    }
    let (before, scrutinee) = head.rsplit_once("match ")?;
    let before = before.trim_end();
    (before.is_empty() || before.ends_with('=') || before.ends_with("=>"))
        .then(|| scrutinee.to_owned())
}

fn unwrapped(written: &str) -> Option<String> {
    let t = written.trim();
    let open = t.find('<')?;
    let name = t.get(..open)?.trim_end().rsplit("::").next()?;
    if !matches!(name, "Result" | "Option") {
        return None;
    }
    let args = t.get(open + 1..)?.strip_suffix('>')?;
    let mut depth = 0i32;
    for (i, c) in args.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => return args.get(..i).map(|a| a.trim().to_owned()),
            _ => {}
        }
    }
    Some(args.trim().to_owned())
}

pub(super) enum Got {
    Ty(Typed),
    Written {
        file: PathBuf,
        written: String,
        line0: usize,
    },
    Bound(Bound),
}

impl App {
    pub(super) fn rust_got(&self, file: &Path, written: String, line0: usize) -> Got {
        match self.rust_resolve(file, &written, line0) {
            Some(ty) => Got::Ty(ty),
            None => Got::Written {
                file: file.to_path_buf(),
                written,
                line0,
            },
        }
    }

    pub(super) fn rust_call_returns(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        parts: &[String],
    ) -> Option<(Hit, String)> {
        if search::bindings(Kind::Rust, text, at + 1, &parts[0])
            .iter()
            .any(|b| b.line1 != at + 1)
        {
            return None;
        }
        let decl = if let Some(decl) = self.declaration(Kind::Rust, file, parts) {
            decl
        } else {
            let [f] = parts else { return None };
            let ns = search::RustNamespace::Other;
            let [l] = search::rust_scope_items_right_of_let(text, at + 1, f, ns)[..] else {
                return None;
            };
            Hit {
                path: file.to_path_buf(),
                line1: l,
                byte_col: None,
                text: text.lines().nth(l - 1)?.to_owned(),
                deleted: None,
            }
        };
        if !decl.text.contains("fn ") {
            return None;
        }
        let t = self.text_of(&decl.path)?;
        let l: Vec<&str> = t.lines().collect();
        let returns = search::rust_return_type(&l, decl.line1 - 1)?;
        Some((decl, returns))
    }

    fn rust_fn_returns(&self, decl: &Hit) -> Option<Got> {
        let t = self.text_of(&decl.path)?;
        let lines: Vec<&str> = t.lines().collect();
        let returns = search::rust_return_type(&lines, decl.line1 - 1)?;
        Some(self.rust_got(&decl.path, returns, decl.line1 - 1))
    }

    fn rust_awaited_returns(&self, decl: &Hit, awaited: bool) -> Option<Got> {
        if awaited && !decl.text.contains("async fn ") {
            return None;
        }
        self.rust_fn_returns(decl)
    }

    fn rust_links(&self, mut got: Got, links: &[Link]) -> Option<Got> {
        let awaits = |i: usize| links.get(i).is_some_and(|l| l.name == "await" && !l.call);
        for (i, link) in links.iter().enumerate() {
            if awaits(i) {
                if !i
                    .checked_sub(1)
                    .and_then(|p| links.get(p))
                    .is_some_and(|l| l.call)
                {
                    return None;
                }
                continue;
            }
            got = self.rust_step(got, link, awaits(i + 1))?;
        }
        Some(got)
    }

    fn rust_step(&self, got: Got, link: &Link, awaited: bool) -> Option<Got> {
        let unwraps =
            link.name == "?" || (link.call && matches!(link.name.as_str(), "unwrap" | "expect"));
        match got {
            Got::Ty(ty) if link.call => {
                let [decl] = <[Hit; 1]>::try_from(self.rust_members(&ty, &link.name)).ok()?;
                self.rust_awaited_returns(&decl, awaited)
            }
            Got::Bound(b) if link.call => {
                self.rust_awaited_returns(&self.rust_bound_method(&b, &link.name)?.0, awaited)
            }
            _ if awaited => None,
            Got::Ty(ty) if link.name != "?" => {
                let (hit, written) = self.rust_field(&ty, &link.name)?;
                Some(self.rust_got(&ty.path, written, hit.line1 - 1))
            }
            Got::Written {
                file,
                written,
                line0,
            } if unwraps => Some(self.rust_got(&file, unwrapped(&written)?, line0)),
            Got::Written {
                file,
                written,
                line0,
            } if link.call => self.rust_std_returns(&file, &written, line0, &link.name),
            _ => None,
        }
    }

    fn rust_head(
        &self,
        file: &Path,
        text: &str,
        line0: usize,
        w: &Walked,
        own: Option<&str>,
    ) -> Option<Got> {
        match (w.head.as_slice(), w.head_call) {
            ([name], false) => self
                .rust_value_type(file, text, line0, name, own.is_some(), None)
                .map(|(ty, _)| Got::Ty(ty))
                .or_else(|| self.rust_std_head(file, text, line0, name, own.is_some()))
                .or_else(|| {
                    let b = self
                        .rust_param_bound(file, text, line0, name)
                        .filter(|_| name != "self");
                    b.map(Got::Bound)
                }),
            ([.., ty, f], true) if ty.starts_with(|c: char| c.is_ascii_uppercase()) => {
                let owner = w.head[..w.head.len() - 1].join("::");
                let Some(ty) = self.rust_resolve(file, &owner, line0) else {
                    return self.rust_std_assoc(file, text, line0, &owner, f);
                };
                let [decl] = <[Hit; 1]>::try_from(self.rust_members(&ty, f)).ok()?;
                self.rust_fn_returns(&decl)
            }
            (parts, true) => {
                let own_let =
                    |b: &search::Binding| own == Some(parts[0].as_str()) && b.line1 == line0 + 1;
                let seen: Vec<search::Binding> =
                    search::bindings(Kind::Rust, text, line0 + 1, &parts[0])
                        .into_iter()
                        .filter(|b| !own_let(b))
                        .collect();
                if let ([name], [b]) = (parts, seen.as_slice()) {
                    return self.rust_closure_returns(file, text, b.line1 - 1, name);
                }
                if !seen.is_empty() {
                    return None;
                }
                let decl = self
                    .declaration(Kind::Rust, file, parts)
                    .or_else(|| match parts {
                        [name] => self.rust_used_fn(file, text, name),
                        _ => None,
                    })?;
                if !decl.text.contains("fn ") {
                    return None;
                }
                self.rust_fn_returns(&decl)
            }
            _ => None,
        }
    }

    pub(super) fn rust_closure_param(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        static CLOSURE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"[(,]\s*(?:move\s+)?\|([\w\s,]*)\|").expect("the regex is a literal")
        });
        let lines: Vec<&str> = text.lines().collect();
        let line = lines.get(at)?;
        let found: Vec<(usize, usize)> = CLOSURE
            .captures_iter(line)
            .filter_map(|c| {
                let params: Vec<&str> = c[1].split(',').map(str::trim).collect();
                let input = params
                    .iter()
                    .position(|p| p.strip_prefix("mut ").unwrap_or(p).trim() == name)?;
                Some((c.get(0)?.start() + 1, input))
            })
            .collect();
        let [(start, input)] = found[..] else {
            return None;
        };
        let mut expr = line.get(..start)?.trim().to_owned();
        let mut i = at;
        while expr.starts_with('.') && i > 0 {
            i -= 1;
            expr.insert_str(0, lines[i].trim());
        }
        let open = opening(&format!("{expr})"), b')', b'(')?;
        let arg = search::top_split(expr.get(open + 1..)?).len() - 1;
        let mut w = walk_back(expr.get(..open)?)?;
        let method = w.links.pop().filter(|l| !l.call)?.name;
        if w.rest != 0 {
            return None;
        }
        let got = self.rust_head(file, text, at, &w, None)?;
        let Got::Ty(ty) = self.rust_links(got, &w.links)? else {
            return None;
        };
        let [decl] = <[Hit; 1]>::try_from(self.rust_members(&ty, &method)).ok()?;
        let t = self.text_of(&decl.path)?;
        let l: Vec<&str> = t.lines().collect();
        let sig = search::rust_signature(&l, decl.line1 - 1)?;
        let written = search::rust_closure_input(&sig, arg, input)?;
        let written = written.trim_start_matches(['&', ' ']);
        let written = written.strip_prefix("mut ").unwrap_or(written).trim();
        let Some(assoc) = written.strip_prefix("Self::") else {
            return self.rust_resolve(&decl.path, written, decl.line1 - 1);
        };
        let (search::RustOwner::Trait(tr) | search::RustOwner::ImplOf(tr)) =
            search::rust_method_at(&l, decl.line1)?.owner
        else {
            return None;
        };
        let impls = format!(
            r"^\s*(?:unsafe\s+)?impl\b[^{{]*\b{}\b[^{{]*\bfor\s+(?:\w+::)*{}\b",
            regex::escape(&tr),
            regex::escape(&ty.name)
        );
        let [imp] = <[Hit; 1]>::try_from(
            self.grep_in(&impls, &self.rust_type_files(&ty))
                .into_iter()
                .filter(|h| search::rust_impl_trait(&h.text).as_ref() == Some(&tr))
                .collect::<Vec<_>>(),
        )
        .ok()?;
        let it = self.text_of(&imp.path)?;
        let il: Vec<&str> = it.lines().collect();
        let assoc = search::rust_assoc_type(&il, imp.line1 - 1, assoc)?;
        self.rust_resolve(&imp.path, &assoc, imp.line1 - 1)
    }

    fn rust_closure_returns(&self, file: &Path, text: &str, at: usize, name: &str) -> Option<Got> {
        let lines: Vec<&str> = text.lines().collect();
        let init = let_initializer(&lines, at, name)?;
        let init = init.strip_prefix("move ").unwrap_or(&init);
        let (params, body) = init.strip_prefix('|')?.split_once('|')?;
        let body = body.trim();
        if let Some(returns) = body.strip_prefix("->") {
            let (written, _) = returns.split_once('{')?;
            return Some(self.rust_got(file, written.trim().to_owned(), at));
        }
        let body = match body.strip_prefix('{') {
            Some(inner) => inner.strip_suffix('}')?.trim(),
            None => body,
        };
        if body.contains(';') {
            return None;
        }
        let w = walk_back(body)?;
        let head = w.head.first()?;
        let param = params
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|p| p == head);
        if head == name || param || (w.links.is_empty() && !w.head_call) {
            return None;
        }
        self.rust_walked_got(file, text, at, name, false, &w)
    }

    fn rust_let_got(&self, file: &Path, text: &str, at: usize, name: &str) -> Option<Got> {
        let lines: Vec<&str> = text.lines().collect();
        let Some(init) = let_initializer(&lines, at, name) else {
            let (path, init) = tuple_initializer(&lines, at, name)?;
            return self.rust_tuple_got(file, text, at, name, &path, &init);
        };
        if init.starts_with("match ") {
            return self.rust_match_got(file, text, &match_arms(&lines, at)?, name);
        }
        let w = walk_back(&init)?;
        let assoc = w.head_call
            && w.head
                .iter()
                .rev()
                .nth(1)
                .is_some_and(|t| t.starts_with(|c: char| c.is_ascii_uppercase()));
        if w.links.is_empty() && !assoc {
            return None;
        }
        self.rust_walked_got(file, text, at, name, true, &w)
    }

    fn rust_tuple_got(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
        path: &[(usize, usize)],
        init: &str,
    ) -> Option<Got> {
        let mut expr = init.trim();
        for (k, &(i, n)) in path.iter().enumerate() {
            if let Some(inner) = nested(expr) {
                let parts = super::returns::top_level(inner);
                expr = parts.get(i).filter(|_| parts.len() == n)?;
                continue;
            }
            let w = walk_back(expr)?;
            if w.links.is_empty() && !w.head_call {
                return None;
            }
            let Got::Written {
                file,
                written,
                line0,
            } = self.rust_walked_got(file, text, at, name, true, &w)?
            else {
                return None;
            };
            let mut element = written.trim();
            for &(i, n) in &path[k..] {
                let parts = super::returns::top_level(nested(element.trim())?);
                element = parts.get(i).filter(|_| parts.len() == n)?;
            }
            return Some(self.rust_got(&file, element.to_owned(), line0));
        }
        self.rust_walked_got(file, text, at, name, true, &walk_back(expr)?)
    }

    fn rust_walked_got(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
        shadowing: bool,
        w: &Walked,
    ) -> Option<Got> {
        let shadows = !w.head_call && w.head.first().is_some_and(|h| h == name);
        if w.rest != 0 || (shadows && !shadowing) {
            return None;
        }
        if let ([head], false, [link]) = (w.head.as_slice(), w.head_call, w.links.as_slice())
            && link.call
            && link.name == "as_ref"
            && let Some(got) = super::returns::as_ref_head(file, text, at, head)
        {
            return Some(got);
        }
        let got = self.rust_head(file, text, at, w, Some(name))?;
        self.rust_links(got, &w.links)
    }

    fn rust_match_got(&self, file: &Path, text: &str, arms: &[Arm], name: &str) -> Option<Got> {
        let bound: Vec<&str> = arms
            .iter()
            .flat_map(|a| a.pattern.split(|c: char| !c.is_alphanumeric() && c != '_'))
            .collect();
        let mut agreed: Option<Got> = None;
        for arm in arms {
            let Some(w) = walk_back(&arm.expr) else {
                continue;
            };
            let [head, ..] = w.head.as_slice() else {
                continue;
            };
            if w.rest != 0 || w.links.is_empty() || head == name || bound.contains(&head.as_str()) {
                continue;
            }
            let got = self.rust_head(file, text, arm.line0, &w, None).or_else(|| {
                match (w.head.as_slice(), w.head_call) {
                    ([one], false) => {
                        self.rust_std_written(file, text, arm.line0, one)
                            .map(|written| Got::Written {
                                file: file.to_path_buf(),
                                written,
                                line0: arm.line0,
                            })
                    }
                    _ => None,
                }
            });
            let Some(mut got) = got else {
                continue;
            };
            got = self.rust_links(got, &w.links)?;
            match (&agreed, &got) {
                (None, _) => agreed = Some(got),
                (Some(Got::Ty(a)), Got::Ty(b)) if (&a.path, a.line) == (&b.path, b.line) => {}
                (Some(Got::Written { written: a, .. }), Got::Written { written: b, .. })
                    if rust_bare_type(a).is_some() && rust_bare_type(a) == rust_bare_type(b) => {}
                _ => return None,
            }
        }
        agreed
    }

    pub(super) fn rust_for_item(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<String> {
        let (expr, place) = for_iterated(text, at, name)?;
        if !place.is_empty() && place.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let bindings = search::bindings(Kind::Rust, text, at + 1, place);
            let [b] = bindings.as_slice() else {
                return None;
            };
            let lines: Vec<&str> = text.lines().collect();
            let Some(search::RustHolds::Type(w)) = search::rust_holds(&lines, b.line1 - 1, place)
            else {
                return None;
            };
            let (container, element) = rust_element(&w)?;
            if container.is_some_and(|c| !self.rust_std_named(text, &c)) {
                return None;
            }
            return rust_bare_type(&element);
        }
        let w = walk_back(expr)?;
        if w.rest != 0 || (!w.head_call && w.links.is_empty()) {
            return None;
        }
        let mut got = match self.rust_head(file, text, at, &w, None) {
            Some(got) => got,
            None if w.head_call => self.rust_std_fn_returns(text, at, &w.head)?,
            None => return None,
        };
        got = self.rust_links(got, &w.links)?;
        self.rust_std_item(got)
    }

    pub(super) fn rust_if_let_typed(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        self.rust_if_let_header_typed(file, text, at, name, text.lines().nth(at)?.trim())
    }

    pub(super) fn rust_returns_own(&self, ty: &Typed, f: &str) -> Option<bool> {
        Some(match <[Hit; 1]>::try_from(self.rust_members(ty, f)) {
            Ok([decl]) => {
                let t = self.text_of(&decl.path)?;
                let l: Vec<&str> = t.lines().collect();
                let n =
                    search::rust_type_name(&search::rust_return_type(&l, decl.line1 - 1)?)?.name;
                n == "Self" || n == ty.name
            }
            Err(none) => none.is_empty() && f == "default",
        })
    }

    pub(super) fn rust_if_let_then(
        &self,
        file: &Path,
        text: &str,
        line0: usize,
        name: &str,
        col: Option<usize>,
        bindings: &mut Vec<search::Binding>,
    ) -> Option<Typed> {
        if bindings.first().is_none_or(|b| b.line1 != line0 + 1) {
            return None;
        }
        let (header, then) = search::rust_if_let_branch(text.lines().nth(line0)?, name, col?)?;
        if !then {
            bindings.retain(|b| b.line1 != line0 + 1);
            return None;
        }
        bindings.clear();
        self.rust_if_let_header_typed(file, text, line0, name, header)
    }

    fn rust_if_let_header_typed(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
        line: &str,
    ) -> Option<Typed> {
        let lines: Vec<&str> = text.lines().collect();
        let header = ["} else if let ", "if let ", "while let ", "let "]
            .iter()
            .find_map(|p| line.strip_prefix(p));
        let (wrapper, rest) = wrapped_pattern(header.unwrap_or(line), name)?;
        let expr = match (header, rest.strip_prefix("=>")) {
            (None, Some(_)) => match_scrutinee(&lines, at)?,
            (Some(_), None) if line.starts_with("let ") => {
                let (expr, after) = rest.strip_prefix('=')?.split_once(" else ")?;
                (after.trim() == "{" || after.trim_end().ends_with(['}', ';']))
                    .then(|| expr.to_owned())?
            }
            (Some(_), None) => if_let_scrutinee(&lines, at, rest.strip_prefix('=')?)?,
            _ => return None,
        };
        let w = walk_back(expr.trim())?;
        if w.links.is_empty() && !w.head_call {
            return None;
        }
        let Got::Written {
            file,
            written,
            line0,
        } = self.rust_walked_got(file, text, at, name, false, &w)?
        else {
            return None;
        };
        if rust_bare_type(&written).as_deref() != Some(wrapper) {
            return None;
        }
        self.rust_resolve(&file, &unwrapped(&written)?, line0)
    }

    pub(super) fn rust_for_typed(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        static MAP_COLLECT: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\.map\(\s*\|[^|]*\|\s*((?:[A-Za-z_]\w*::)*[A-Za-z_]\w*)\s*\([^()]*\)\s*\)\s*\.collect\(\s*\)$")
                .expect("the regex is a literal")
        });
        let (expr, place) = for_iterated(text, at, name)?;
        if place.is_empty() || !place.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return (expr == place)
                .then(|| self.rust_for_project_item(file, text, at, expr))
                .flatten();
        }
        let bindings = search::bindings(Kind::Rust, text, at + 1, place);
        let [b] = bindings.as_slice() else {
            return None;
        };
        let lines: Vec<&str> = text.lines().collect();
        let let_at = b.line1 - 1;
        let Some(search::RustHolds::Type(w)) = search::rust_holds(&lines, let_at, place) else {
            return None;
        };
        let (container, element) = rust_element(&w)?;
        if container.is_some_and(|c| !self.rust_std_named(text, &c)) {
            return None;
        }
        if element != "_" {
            return self.rust_resolve(file, &element, let_at);
        }
        let t = lines.get(let_at)?.trim().strip_prefix("let ")?.trim_start();
        let t = t.strip_prefix("mut ").unwrap_or(t).trim_start();
        let (written, rest) = t
            .strip_prefix(place)?
            .trim_start()
            .strip_prefix(':')?
            .split_once('=')?;
        if written.trim() != w.trim() {
            return None;
        }
        let init = statement(&lines, let_at, rest)?;
        let parts: Vec<String> = MAP_COLLECT.captures(&init)?[1]
            .split("::")
            .map(str::to_owned)
            .collect();
        let (decl, returns) = self.rust_call_returns(file, text, let_at, &parts)?;
        self.rust_resolve(&decl.path, &returns, decl.line1 - 1)
    }

    fn rust_for_project_item(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        expr: &str,
    ) -> Option<Typed> {
        const SAME_ITEM: [&str; 11] = [
            "by_ref",
            "filter",
            "fuse",
            "inspect",
            "peekable",
            "rev",
            "skip",
            "skip_while",
            "step_by",
            "take",
            "take_while",
        ];
        let mut w = walk_back(expr)?;
        if w.rest != 0 {
            return None;
        }
        let mut adapters = Vec::new();
        while let Some(l) = w
            .links
            .pop_if(|l| l.call && SAME_ITEM.contains(&l.name.as_str()))
        {
            adapters.push(l.name);
        }
        if !w.head_call && w.links.is_empty() && adapters.is_empty() {
            return None;
        }
        let got = self.rust_head(file, text, at, &w, None)?;
        let Got::Ty(ty) = self.rust_links(got, &w.links)? else {
            return None;
        };
        if adapters
            .iter()
            .any(|a| !self.rust_members(&ty, a).is_empty())
        {
            return None;
        }
        let t = self.text_of(&ty.path)?;
        let lines: Vec<&str> = t.lines().collect();
        let [(impl_line0, item)] = iterator_items(&lines, &ty.name)[..] else {
            return None;
        };
        let generics = lines[impl_line0].trim().strip_prefix("impl")?;
        let lifetimes_only = generics.strip_prefix('<').is_none_or(|g| {
            g.split('>')
                .next()
                .is_some_and(|g| g.split(',').all(|p| p.trim().starts_with('\'')))
        });
        lifetimes_only.then_some(())?;
        self.rust_resolve(&ty.path, item, impl_line0)
    }

    pub(super) fn rust_let_chain(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        let lines: Vec<&str> = text.lines().collect();
        let unit = |s: &str| {
            s.starts_with(|c: char| c.is_ascii_uppercase())
                && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        };
        let module = |s: &str| {
            s.starts_with(|c: char| c.is_ascii_lowercase())
                && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        };
        if let Some(init) = let_initializer(&lines, at, name) {
            if unit(&init) {
                return self.rust_unit_struct(file, at, &init);
            }
            if let Some((head, last)) = init.rsplit_once("::")
                && unit(last)
                && head.split("::").all(module)
                && let Some(ty) = self.rust_unit_struct(file, at, last)
                && self.rust_unit_path(file, text, at, head, &ty)
            {
                return Some(ty);
            }
        }
        match self.rust_let_got(file, text, at, name)? {
            Got::Ty(ty) => Some(ty),
            Got::Written { .. } | Got::Bound(_) => None,
        }
    }

    fn rust_unit_struct(&self, file: &Path, at: usize, init: &str) -> Option<Typed> {
        let ty = self.rust_resolve(file, init, at)?;
        let decl = self.text_of(&ty.path)?;
        let decl = decl.lines().nth(ty.line - 1)?.trim();
        let decl = decl.strip_prefix("pub").map_or(decl, |d| {
            d.trim_start()
                .strip_prefix('(')
                .and_then(|d| d.split_once(')'))
                .map_or(d, |(_, d)| d)
        });
        (decl.trim_start() == format!("struct {init};")).then_some(ty)
    }

    fn rust_unit_path(&self, file: &Path, text: &str, at: usize, head: &str, ty: &Typed) -> bool {
        let module_of = |p: &Path| search::rust_module_of(&self.files, p, |f| self.text_of(f));
        let (Some(here), Some(there), Some(decl)) =
            (module_of(file), module_of(&ty.path), self.text_of(&ty.path))
        else {
            return false;
        };
        let lines: Vec<&str> = text.lines().collect();
        let decl: Vec<&str> = decl.lines().collect();
        if here.crate_src != there.crate_src
            || search::rust_inline_mod(&lines, at).is_some()
            || search::rust_inline_mod(&decl, ty.line - 1).is_some()
        {
            return false;
        }
        let mut parts = head.split("::");
        let mut want = match parts.next() {
            Some("crate") => Vec::new(),
            Some("self") => here.path,
            Some("super") if !here.path.is_empty() => {
                let mut up = here.path;
                up.pop();
                up
            }
            Some(m) if super::declares_mod(text, m) => {
                let mut down = here.path;
                down.push(m.to_owned());
                down
            }
            _ => return false,
        };
        want.extend(parts.map(str::to_owned));
        want == there.path
    }

    pub(super) fn rust_let_written(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<String> {
        match self.rust_let_got(file, text, at, name)? {
            Got::Written { written, .. } => rust_bare_type(&written),
            Got::Ty(_) | Got::Bound(_) => None,
        }
    }

    pub(in crate::app) fn rust_call_chain(
        &mut self,
        here: &Path,
        text: &str,
        receiver: &str,
        word: &str,
        call: bool,
    ) -> Option<Vec<Candidate>> {
        let w = walk_back(receiver)?;
        if !w.head_call && w.links.is_empty() {
            return None;
        }
        let line = self.line;
        let walked = |a: &Self| {
            let got = a.rust_head(here, text, line, &w, None)?;
            a.rust_links(got, &w.links)
        };
        let mut got = walked(self);
        if got.is_none() {
            self.external_files(Kind::Rust);
            got = walked(self);
        }
        let got = got?;
        let via = w
            .links
            .iter()
            .rev()
            .find(|l| l.name != "?" && (l.call || l.name != "await"))
            .map_or(&w.head[w.head.len() - 1], |l| &l.name);
        let ty = match got {
            Got::Ty(ty) => ty,
            Got::Written { file, written, .. } if call => {
                self.external_files(Kind::Rust);
                if let Some(hit) = self.rust_std_marked(&written, word) {
                    let reason = Reason::Receiver(format!("{via}(): {written}"));
                    return Some(vec![Candidate { hit, reason }]);
                }
                let std = rust_bare_type(&written)?;
                let owner = self.text_of(&file)?;
                return self.rust_std_call(&owner, via, &std, word);
            }
            Got::Written { .. } | Got::Bound(_) => return None,
        };
        let hits = if call {
            self.rust_members(&ty, word)
        } else {
            self.rust_field(&ty, word)
                .map(|(hit, _)| vec![hit])
                .unwrap_or_default()
        };
        let reason = Reason::Receiver(format!("{via}(): {}", ty.name));
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }
}

impl App {
    fn rust_used_fn(&self, file: &Path, text: &str, name: &str) -> Option<Hit> {
        let mut used = search::rust_uses(text)
            .into_iter()
            .filter(|u| u.name == name && u.in_column_zero);
        let (Some(u), None) = (used.next(), used.next()) else {
            return None;
        };
        let (root, rest) = u.path.split_first()?;
        if !matches!(root.as_str(), "crate" | "super" | "self") {
            return None;
        }
        let in_crate = self
            .rust_own_crate(file, root, rest)
            .and_then(|(krate, module)| {
                let rest: Vec<String> = rest.iter().filter(|p| *p != "super").cloned().collect();
                match self.rust_in_crate(&krate, &module, &rest, MacroUse::NotMacro, 0)? {
                    InCrate {
                        hits, proven: true, ..
                    } => Some(hits),
                    InCrate { .. } => None,
                }
            });
        let hits = match (in_crate, rest.split_last()) {
            (Some(hits), _) => hits,
            (None, Some((_, parts))) if root == "crate" && !parts.is_empty() => {
                let module = parts.join("/");
                let rs = |p: &Path| {
                    p.ends_with(format!("{module}.rs")) || p.ends_with(format!("{module}/mod.rs"))
                };
                self.grep(
                    &search::def_patterns(Kind::Rust, name).join("|"),
                    false,
                    false,
                    rs,
                )
                .unwrap_or_default()
            }
            _ => return None,
        };
        let fns: Vec<Hit> = hits
            .into_iter()
            .filter(|h| h.text.contains("fn "))
            .collect();
        let [hit] = <[Hit; 1]>::try_from(fns).ok()?;
        Some(hit)
    }
}
