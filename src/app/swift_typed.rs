use super::{App, Candidate, HashSet, Hit, Kind, Path, Reason, search};

struct SwiftChain {
    written: String,
    links: Vec<String>,
}
struct WrittenVia {
    written: String,
    call_signature: Option<String>,
}
impl WrittenVia {
    fn plain(written: String) -> Self {
        Self {
            written,
            call_signature: None,
        }
    }
}

struct SwiftType {
    name: String,
    owner: String,
    keyword: Option<String>,
    base: Option<String>,
}

impl App {
    pub(super) fn swift_typed(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
    ) -> Option<Vec<Candidate>> {
        let cut = self.truncated.get();
        let found = (|| {
            let SwiftChain { written, links } =
                self.swift_chain(here, here, text, self.line + 1, chain, 1)?;
            let label = links.join(" \u{2192} ");
            let Some(ty) = self.swift_type(here, &written) else {
                let w = written.trim();
                let w = w
                    .strip_prefix("any ")
                    .or_else(|| w.strip_prefix("some "))
                    .unwrap_or(w);
                let a_type = search::bindings(Kind::Swift, text, self.line + 1, &chain[0])
                    .iter()
                    .any(|b| {
                        text.lines()
                            .nth(b.line1 - 1)
                            .is_some_and(|l| search::swift_type_header(l).is_some())
                    });
                let Some(p) =
                    search::swift_type_name(w).filter(|p| !a_type && self.swift_protocol(here, p))
                else {
                    let rows = self.swift_outside_rows(here, w, word).filter(|_| !a_type)?;
                    return Some(
                        rows.into_iter()
                            .map(|hit| Candidate {
                                hit,
                                reason: Reason::Receiver(label.clone()),
                            })
                            .collect(),
                    );
                };
                let mut rows = self.swift_protocol_rows(here, &p, word, true)?;
                if rows.is_empty() {
                    rows = self.swift_protocol_rows(here, &p, word, false)?;
                }
                return (!rows.is_empty()).then(|| {
                    rows.into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::Receiver(label.clone()),
                        })
                        .collect()
                });
            };
            let mut rows = self.swift_member_rows(here, &ty, word)?;
            if rows
                .iter()
                .any(|h| search::swift_member_kind(&h.text) == Some(search::SwiftMember::Instance))
            {
                rows.retain(|h| {
                    search::swift_member_kind(&h.text) != Some(search::SwiftMember::StaticOrClass)
                });
            }
            Some(
                rows.into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Receiver(label.clone()),
                    })
                    .collect(),
            )
        })();
        self.truncated.set(cut);
        found
    }

    fn swift_chain(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        names: &[String],
        hops: usize,
    ) -> Option<SwiftChain> {
        let WrittenVia {
            mut written,
            call_signature: link,
        } = self.swift_value(here, file, text, line1, &names[0], hops)?;
        let shown = |w: &str| search::swift_type_name(w).unwrap_or_else(|| w.to_owned());
        let mut links = vec![link.unwrap_or_else(|| format!("{}: {}", names[0], shown(&written)))];
        for (i, field) in names[1..].iter().enumerate() {
            if i == 5 {
                return None;
            }
            let ty = self.swift_type(here, &written)?;
            let WrittenVia {
                written: next,
                call_signature: link,
            } = self.swift_field(here, &ty, field, hops)?;
            let link = link.unwrap_or_else(|| {
                if i == 0 && names[0] == "self" {
                    format!("self.{field}: {}", shown(&next))
                } else {
                    format!("{field}: {}", shown(&next))
                }
            });
            if i == 0 && names[0] == "self" {
                links[0] = link;
            } else {
                links.push(link);
            }
            written = next;
        }
        Some(SwiftChain { written, links })
    }

    fn swift_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let around = search::swift_enclosing_type(&lines, &literal, line1);
        if name == "self" {
            let own = search::swift_type_header(lines[around? - 1])?.name;
            return Some(WrittenVia::plain(own));
        }
        match search::bindings(Kind::Swift, text, line1, name).as_slice() {
            [b] => {
                let given = search::swift_given(lines.get(b.line1 - 1)?, name)?;
                self.swift_given_type(here, file, text, b.line1, given, hops)
            }
            [] => {
                let at = around?;
                let mut top = search::swift_scope(&lines, &literal, line1).header_line1;
                while top > at {
                    match search::swift_scope(&lines, &literal, top).header_line1 {
                        up if up > at => top = up,
                        _ => break,
                    }
                }
                let upto = |l: usize| {
                    let line = lines[l - 1];
                    match (l == line1 && file == here && line1 == self.line + 1)
                        .then(|| line.get(..self.col))
                    {
                        Some(Some(head)) => head,
                        _ => line,
                    }
                };
                if top == 0 || (top..=line1).any(|l| search::swift_may_bind(upto(l), name)) {
                    return None;
                }
                let own = search::swift_type_header(lines[at - 1])?.name;
                let ty = self.swift_type(here, &own)?;
                self.swift_field(here, &ty, name, hops)
            }
            _ => None,
        }
    }

    fn swift_given_type(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        given: search::SwiftGiven,
        hops: usize,
    ) -> Option<WrittenVia> {
        let via = match given {
            search::SwiftGiven::Type(t) => WrittenVia::plain(t),
            search::SwiftGiven::Value(e) => self.swift_expr(here, file, text, line1, &e, hops)?,
            search::SwiftGiven::Element(e) => {
                let collection = self.swift_expr(here, file, text, line1, &e, hops)?;
                WrittenVia {
                    written: search::swift_element(&collection.written)?,
                    call_signature: collection.call_signature,
                }
            }
        };
        let name = search::swift_type_name(&via.written)
            .or_else(|| search::swift_element(&via.written))
            .unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut at = line1;
        while at > 0 {
            if search::swift_generics(lines[at - 1]).contains(&name) {
                return None;
            }
            at = search::swift_scope(&lines, &literal, at).header_line1;
        }
        Some(via)
    }

    fn swift_expr(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        e: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let callee = match search::swift_expr(e)? {
            search::SwiftExpr::Cast(t) => return Some(WrittenVia::plain(t)),
            search::SwiftExpr::Chain(c) => {
                let names: Vec<String> = c.split('.').map(str::to_owned).collect();
                let chain =
                    self.swift_chain(here, file, text, line1, &names, hops.checked_sub(1)?)?;
                return Some(WrittenVia::plain(chain.written));
            }
            search::SwiftExpr::Call(callee) => callee,
        };
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        if let [ty] | [ty, _] = parts.as_slice()
            && (parts.len() == 1 || parts[1] == "init")
            && let Some(t) = self.swift_type(here, ty)
        {
            return t.keyword.is_some().then_some(WrittenVia::plain(t.name));
        }
        let (method, receiver) = parts.split_last()?;
        let (decl, owner) = if receiver.is_empty() {
            if !search::bindings(Kind::Swift, text, line1, method).is_empty() {
                return None;
            }
            let pattern = search::def_patterns(Kind::Swift, method).join("|");
            let funcs: Vec<Hit> = self
                .project_definitions(Kind::Swift, here, method, &pattern)
                .into_iter()
                .filter(|h| {
                    h.text.contains("func ")
                        && self.text_of(&h.path).is_some_and(|t| {
                            search::qualified(Kind::Swift, &t, h.line1, method)
                                .is_none_or(|q| q == *method)
                        })
                })
                .collect();
            let [decl] = <[Hit; 1]>::try_from(funcs).ok()?;
            (decl, None)
        } else {
            let chain =
                self.swift_chain(here, file, text, line1, receiver, hops.checked_sub(1)?)?;
            let ty = self.swift_type(here, &chain.written)?;
            let rows = self.swift_member_rows(here, &ty, method)?;
            let [decl] = <[Hit; 1]>::try_from(rows).ok()?;
            (decl, Some(ty.name))
        };
        if !decl.text.contains("func ") {
            return None;
        }
        let returns = search::swift_returns(&self.text_of(&decl.path)?, decl.line1)?;
        let returns = match (returns.as_str(), owner) {
            ("Self", Some(owner)) => owner,
            _ => returns,
        };
        let link = format!("{callee}() -> {returns}");
        Some(WrittenVia {
            written: returns,
            call_signature: Some(link),
        })
    }

    fn swift_field(
        &self,
        here: &Path,
        ty: &SwiftType,
        name: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let rows = self.swift_member_rows(here, ty, name)?;
        let [row] = <[Hit; 1]>::try_from(rows).ok()?;
        if search::swift_member_kind(&row.text).is_none() || row.text.contains("func ") {
            return None;
        }
        let text = self.text_of(&row.path)?;
        let given = search::swift_given(&row.text, name)?;
        self.swift_given_type(here, &row.path, &text, row.line1, given, hops)
    }

    #[expect(
        clippy::match_same_arms,
        reason = "a guarded arm sits between the two; merging them would move one past it"
    )]
    fn swift_type(&self, here: &Path, written: &str) -> Option<SwiftType> {
        let name = search::swift_type_name(written)?;
        let pattern = search::def_patterns(Kind::Swift, &name).join("|");
        let mut decls = Vec::new();
        let mut extended = false;
        for h in self.project_definitions(Kind::Swift, here, &name, &pattern) {
            if !self.in_code(Kind::Swift, &h) {
                continue;
            }
            match search::swift_type_header(&h.text) {
                Some(t) if t.keyword == "extension" => extended |= t.name == name,
                Some(t) if t.name == name => decls.push((h, t.keyword, t.first_inherited)),
                Some(_) => {}
                None if !h.text.contains("func ")
                    && search::swift_member_kind(&h.text).is_none()
                    && !search::swift_case(&h.text) =>
                {
                    return None;
                }
                None => {}
            }
        }
        match decls.as_slice() {
            [] if extended => Some(SwiftType {
                owner: name.clone(),
                name,
                keyword: None,
                base: None,
            }),
            [(h, k, b)] if k != "protocol" => {
                let text = self.text_of(&h.path)?;
                Some(SwiftType {
                    owner: search::qualified(Kind::Swift, &text, h.line1, &name)
                        .unwrap_or_else(|| name.clone()),
                    name,
                    keyword: Some(k.clone()),
                    base: b.clone(),
                })
            }
            _ => None,
        }
    }

    fn swift_member_rows(&self, here: &Path, ty: &SwiftType, word: &str) -> Option<Vec<Hit>> {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        let hits = self.project_definitions(Kind::Swift, here, word, &pattern);
        let mut owner = ty.owner.clone();
        let (mut keyword, mut base) = (ty.keyword.clone(), ty.base.clone());
        let mut seen = HashSet::from([owner.clone()]);
        let mut walked = vec![ty.name.clone()];
        loop {
            let full = format!("{owner}.{word}");
            let rows: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, word))
                        .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")))
                        && !search::swift_extension(&h.text)
                        && self.in_code(Kind::Swift, h)
                })
                .cloned()
                .collect();
            if !rows.is_empty() {
                return Some(rows);
            }
            match (keyword.as_deref(), base) {
                (None, _) => return Some(Vec::new()),
                (Some("class"), Some(next)) if seen.insert(next.clone()) => {
                    match self.swift_type(here, &next) {
                        Some(up) if up.keyword.as_deref() == Some("class") => {
                            walked.push(up.name);
                            (owner, keyword, base) = (up.owner, up.keyword, up.base);
                        }
                        None if self.swift_protocol(here, &next) => {
                            return self.swift_defaults(here, &walked, word);
                        }
                        _ => return None,
                    }
                }
                (Some("class"), Some(_)) => return None,
                (Some(_), _) => return self.swift_defaults(here, &walked, word),
            }
        }
    }

    fn swift_outside_rows(&self, here: &Path, written: &str, word: &str) -> Option<Vec<Hit>> {
        if self.root.join(".build/checkouts").exists() {
            return None;
        }
        let name = search::swift_outside_name(written)?;
        let mut rows = match self.swift_type(here, &name) {
            Some(ty) if ty.keyword.is_some() => return self.swift_member_rows(here, &ty, word),
            Some(ty) => {
                let mut rows = self.swift_member_rows(here, &ty, word)?;
                rows.extend(
                    self.swift_defaults(here, std::slice::from_ref(&name), word)
                        .unwrap_or_default(),
                );
                rows
            }
            None => {
                let pattern = search::def_patterns(Kind::Swift, &name).join("|");
                if !self
                    .project_definitions(Kind::Swift, here, &name, &pattern)
                    .is_empty()
                {
                    return None;
                }
                Vec::new()
            }
        };
        let squeezed = |t: &str| t.split_whitespace().collect::<String>();
        let receiver = squeezed(written.trim().trim_end_matches(['?', '!']));
        let fits = |extended: &str| {
            if extended.starts_with('[') {
                squeezed(extended) == receiver
            } else {
                search::swift_outside_name(extended).is_some_and(|n| {
                    n == name
                        || [
                            "Sequence",
                            "Collection",
                            "BidirectionalCollection",
                            "RandomAccessCollection",
                        ]
                        .contains(&n.as_str())
                })
            }
        };
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        for h in self.project_definitions(Kind::Swift, here, word, &pattern) {
            let seen = rows.iter().any(|r| r.path == h.path && r.line1 == h.line1);
            let extends = self.in_code(Kind::Swift, &h)
                && self.text_of(&h.path).is_some_and(|text| {
                    let lines: Vec<&str> = text.lines().collect();
                    let literal = search::literal_lines(Kind::Swift, &text);
                    let at = search::swift_scope(&lines, &literal, h.line1).header_line1;
                    at > 0 && search::swift_extended(lines[at - 1]).is_some_and(|t| fits(&t))
                });
            if extends && !seen {
                rows.push(h);
            }
        }
        Some(rows)
    }

    pub(super) fn swift_self_default(
        &self,
        here: &Path,
        own: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let cut = self.truncated.get();
        let rows = self
            .swift_type(here, own)
            .and_then(|ty| self.swift_member_rows(here, &ty, word))
            .filter(|rows| !rows.is_empty());
        self.truncated.set(cut);
        Some(
            rows?
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(format!("self: {own}")),
                })
                .collect(),
        )
    }

    fn swift_protocol(&self, here: &Path, name: &str) -> bool {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        let decls: Vec<String> = self
            .project_definitions(Kind::Swift, here, name, &pattern)
            .iter()
            .filter(|h| self.in_code(Kind::Swift, h))
            .filter_map(|h| search::swift_type_header(&h.text))
            .filter(|t| t.name == name && t.keyword != "extension")
            .map(|t| t.keyword)
            .collect();
        matches!(decls.as_slice(), [k] if k == "protocol")
    }

    fn swift_defaults(&self, here: &Path, types: &[String], word: &str) -> Option<Vec<Hit>> {
        let mut protocols: Vec<String> = Vec::new();
        for t in types {
            let pattern = search::def_patterns(Kind::Swift, t).join("|");
            for h in self.project_definitions(Kind::Swift, here, t, &pattern) {
                let Some(head) = search::swift_type_header(&h.text)
                    .filter(|head| head.name == *t && self.in_code(Kind::Swift, &h))
                else {
                    continue;
                };
                let inherited = search::swift_inherited(&h.text);
                if head.where_constrained && !inherited.is_empty() {
                    return None;
                }
                protocols.extend(inherited);
            }
        }
        protocols.sort();
        protocols.dedup();
        let mut rows: Vec<Hit> = Vec::new();
        for p in protocols.iter().filter(|p| self.swift_protocol(here, p)) {
            rows.extend(self.swift_protocol_rows(here, p, word, false)?);
        }
        (!rows.is_empty()).then_some(rows)
    }

    fn swift_protocol_rows(
        &self,
        here: &Path,
        protocol: &str,
        word: &str,
        requirement: bool,
    ) -> Option<Vec<Hit>> {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        let full = format!("{protocol}.{word}");
        let mut rows = Vec::new();
        for h in self.project_definitions(Kind::Swift, here, word, &pattern) {
            let text = self.text_of(&h.path)?;
            let named = search::qualified(Kind::Swift, &text, h.line1, word)
                .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")));
            if !named || !self.in_code(Kind::Swift, &h) {
                continue;
            }
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Swift, &text);
            let at = search::swift_enclosing_type(&lines, &literal, h.line1)?;
            let head = search::swift_type_header(lines[at - 1])?;
            let constrained = head.where_constrained
                || lines
                    .get(at)
                    .is_some_and(|l| l.trim_start().starts_with("where "));
            match head.keyword.as_str() {
                "protocol" if requirement => rows.push(h),
                "extension" if !requirement && constrained => return None,
                "extension" if !requirement => rows.push(h),
                _ => {}
            }
        }
        Some(rows)
    }
}
