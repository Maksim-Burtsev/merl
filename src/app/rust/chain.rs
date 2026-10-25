use super::sysroot::rust_element;
use super::{
    App, Candidate, Hit, InCrate, Kind, MacroUse, Path, PathBuf, Reason, Typed, rust_bare_type,
    search,
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

fn tuple_initializer(lines: &[&str], at: usize, name: &str) -> Option<(usize, usize, String)> {
    let t = lines.get(at)?.trim().strip_prefix("let ")?.trim_start();
    let (names, rest) = t.strip_prefix('(')?.split_once(')')?;
    let names: Vec<&str> = names
        .split(',')
        .map(|p| {
            let p = p.trim();
            p.strip_prefix("mut ").unwrap_or(p).trim()
        })
        .collect();
    if names
        .iter()
        .any(|p| p.is_empty() || !p.chars().all(|c| c.is_alphanumeric() || c == '_'))
    {
        return None;
    }
    let i = names.iter().position(|p| *p == name)?;
    let init = statement(lines, at, rest.trim_start().strip_prefix('=')?)?;
    Some((i, names.len(), init))
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
        if !search::bindings(Kind::Rust, text, at + 1, &parts[0]).is_empty() {
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

    fn rust_step(&self, got: Got, link: &Link) -> Option<Got> {
        let unwraps =
            link.name == "?" || (link.call && matches!(link.name.as_str(), "unwrap" | "expect"));
        match got {
            Got::Ty(ty) if link.call => {
                let [decl] = <[Hit; 1]>::try_from(self.rust_members(&ty, &link.name)).ok()?;
                self.rust_fn_returns(&decl)
            }
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
                .rust_value_type(file, text, line0, name)
                .map(|(ty, _)| Got::Ty(ty))
                .or_else(|| self.rust_std_head(file, text, line0, name)),
            ([.., ty, f], true) if ty.starts_with(|c: char| c.is_ascii_uppercase()) => {
                let owner = w.head[..w.head.len() - 1].join("::");
                let ty = self.rust_resolve(file, &owner, line0)?;
                let [decl] = <[Hit; 1]>::try_from(self.rust_members(&ty, f)).ok()?;
                self.rust_fn_returns(&decl)
            }
            (parts, true) => {
                let own_let =
                    |b: &search::Binding| own == Some(parts[0].as_str()) && b.line1 == line0 + 1;
                if search::bindings(Kind::Rust, text, line0 + 1, &parts[0])
                    .iter()
                    .any(|b| !own_let(b))
                {
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

    fn rust_let_got(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
        shadowing: bool,
    ) -> Option<Got> {
        let lines: Vec<&str> = text.lines().collect();
        let Some(init) = let_initializer(&lines, at, name) else {
            let (i, n, init) = tuple_initializer(&lines, at, name)?;
            let w = walk_back(&init)?;
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
            let parts = super::returns::top_level(written.trim().strip_prefix('(')?.strip_suffix(')')?);
            return (parts.len() == n).then(|| self.rust_got(&file, parts[i].to_owned(), line0));
        };
        if init.starts_with("match ") {
            return self.rust_match_got(file, text, &match_arms(&lines, at)?, name);
        }
        let w = walk_back(&init)?;
        if w.links.is_empty() {
            return None;
        }
        self.rust_walked_got(file, text, at, name, shadowing, &w)
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
        let mut got = self.rust_head(file, text, at, w, Some(name))?;
        for link in &w.links {
            got = self.rust_step(got, link)?;
        }
        Some(got)
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
            for link in &w.links {
                got = self.rust_step(got, link)?;
            }
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
        let line = text.lines().nth(at)?.trim();
        let rest = line.strip_prefix("for ")?.trim_start();
        let rest = rest.strip_prefix("mut ").unwrap_or(rest);
        let expr = rest.strip_prefix(name)?.trim_start().strip_prefix("in ")?;
        let expr = expr.strip_suffix('{')?.trim();
        let place = expr.strip_prefix('&').map_or(expr, |s| {
            let s = s.trim_start();
            s.strip_prefix("mut ").unwrap_or(s).trim()
        });
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
        for link in &w.links {
            got = self.rust_step(got, link)?;
        }
        self.rust_std_item(got)
    }

    pub(super) fn rust_let_chain(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        match self.rust_let_got(file, text, at, name, false)? {
            Got::Ty(ty) => Some(ty),
            Got::Written { .. } => None,
        }
    }

    pub(super) fn rust_let_written(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<String> {
        match self.rust_let_got(file, text, at, name, true)? {
            Got::Written { written, .. } => rust_bare_type(&written),
            Got::Ty(_) => None,
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
            let mut got = a.rust_head(here, text, line, &w, None)?;
            for link in &w.links {
                got = a.rust_step(got, link)?;
            }
            Some(got)
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
            .find(|l| l.name != "?")
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
            Got::Written { .. } => return None,
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
