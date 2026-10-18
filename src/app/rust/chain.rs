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

enum Got {
    Ty(Typed),
    Written {
        file: PathBuf,
        written: String,
        line0: usize,
    },
}

impl App {
    fn rust_got(&self, file: &Path, written: String, line0: usize) -> Got {
        match self.rust_resolve(file, &written, line0) {
            Some(ty) => Got::Ty(ty),
            None => Got::Written {
                file: file.to_path_buf(),
                written,
                line0,
            },
        }
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
            _ => None,
        }
    }

    fn rust_head(&self, file: &Path, text: &str, line0: usize, w: &Walked) -> Option<Got> {
        match (w.head.as_slice(), w.head_call) {
            ([name], false) => Some(Got::Ty(self.rust_value_type(file, text, line0, name)?.0)),
            ([.., ty, f], true) if ty.starts_with(|c: char| c.is_ascii_uppercase()) => {
                let owner = w.head[..w.head.len() - 1].join("::");
                let ty = self.rust_resolve(file, &owner, line0)?;
                let [decl] = <[Hit; 1]>::try_from(self.rust_members(&ty, f)).ok()?;
                self.rust_fn_returns(&decl)
            }
            (parts, true) => {
                if !search::bindings(Kind::Rust, text, line0 + 1, &parts[0]).is_empty() {
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
        let mut got = self.rust_head(file, text, at, &w)?;
        for link in &w.links {
            got = self.rust_step(got, link)?;
        }
        match got {
            Got::Ty(ty) => Some(ty),
            Got::Written { .. } => None,
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
        let mut got = self.rust_head(here, text, self.line, &w)?;
        for link in &w.links {
            got = self.rust_step(got, link)?;
        }
        let via = w
            .links
            .iter()
            .rev()
            .find(|l| l.name != "?")
            .map_or(&w.head[w.head.len() - 1], |l| &l.name);
        let ty = match got {
            Got::Ty(ty) => ty,
            Got::Written { file, written, .. } if call => {
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
