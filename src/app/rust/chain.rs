use super::{App, Candidate, Hit, Kind, Path, Reason, Typed, rust_bare_type, search};

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

fn trailing_name(s: &str) -> &str {
    let start = s
        .char_indices()
        .rev()
        .find(|&(_, c)| !(c.is_ascii_alphanumeric() || c == '_'))
        .map_or(0, |(i, c)| i + c.len_utf8());
    s.get(start..).unwrap_or_default()
}

fn opening(s: &str, close: u8, open: u8) -> Option<usize> {
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

impl App {
    fn rust_returned(&self, ty: &Typed, method: &str) -> Result<Typed, Option<String>> {
        let [decl] = <[Hit; 1]>::try_from(self.rust_members(ty, method)).map_err(|_| None)?;
        let t = self.text_of(&decl.path).ok_or(None)?;
        let lines: Vec<&str> = t.lines().collect();
        let returns = search::rust_return_type(&lines, decl.line1 - 1).ok_or(None)?;
        self.rust_resolve(&decl.path, &returns, decl.line1 - 1)
            .ok_or(Some(returns))
    }

    fn rust_step(&self, ty: &Typed, link: &Link) -> Result<Typed, Option<String>> {
        if link.call {
            return self.rust_returned(ty, &link.name);
        }
        let (hit, written) = self.rust_field(ty, &link.name).ok_or(None)?;
        self.rust_resolve(&ty.path, &written, hit.line1 - 1)
            .ok_or(Some(written))
    }

    fn rust_head(&self, file: &Path, text: &str, line0: usize, w: &Walked) -> Option<Typed> {
        match (w.head.as_slice(), w.head_call) {
            ([name], false) => Some(self.rust_value_type(file, text, line0, name)?.0),
            ([.., ty, f], true) if ty.starts_with(|c: char| c.is_ascii_uppercase()) => {
                let owner = w.head[..w.head.len() - 1].join("::");
                let ty = self.rust_resolve(file, &owner, line0)?;
                self.rust_returned(&ty, f).ok()
            }
            (parts, true) => {
                if !search::bindings(Kind::Rust, text, line0 + 1, &parts[0]).is_empty() {
                    return None;
                }
                let decl = self.declaration(Kind::Rust, file, parts)?;
                if !decl.text.contains("fn ") {
                    return None;
                }
                let t = self.text_of(&decl.path)?;
                let l: Vec<&str> = t.lines().collect();
                let returns = search::rust_return_type(&l, decl.line1 - 1)?;
                self.rust_resolve(&decl.path, &returns, decl.line1 - 1)
            }
            _ => None,
        }
    }

    pub(super) fn rust_let_chain(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        let lines: Vec<&str> = text.lines().collect();
        let init = let_initializer(&lines, at, name)?;
        let w = walk_back(&init)?;
        if w.rest != 0 || w.links.is_empty() || w.head.first().is_some_and(|h| h == name) {
            return None;
        }
        let mut ty = self.rust_head(file, text, at, &w)?;
        for link in &w.links {
            ty = self.rust_step(&ty, link).ok()?;
        }
        Some(ty)
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
        let mut ty = self.rust_head(here, text, self.line, &w)?;
        let last = w.links.len();
        for (i, link) in w.links.iter().enumerate() {
            match self.rust_step(&ty, link) {
                Ok(next) => ty = next,
                Err(Some(written)) if i + 1 == last && call => {
                    let std = rust_bare_type(&written)?;
                    let owner = self.text_of(&ty.path)?;
                    return self.rust_std_call(&owner, &link.name, &std, word);
                }
                Err(_) => return None,
            }
        }
        let hits = if call {
            self.rust_members(&ty, word)
        } else {
            self.rust_field(&ty, word)
                .map(|(hit, _)| vec![hit])
                .unwrap_or_default()
        };
        let via = w
            .links
            .last()
            .map_or(&w.head[w.head.len() - 1], |l| &l.name);
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
