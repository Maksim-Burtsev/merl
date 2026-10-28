use super::{App, Candidate, HashSet, Hit, Kind, Path, Reason, search};

mod generic;

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

const SDK_VALUE: &str = "(SDK)";

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
            let Some(SwiftChain { written, links }) =
                self.swift_chain(here, here, text, self.line + 1, chain, 3)
            else {
                if let [name] = chain
                    && let Some((_, rows)) =
                        self.swift_static_rows(here, text, self.line + 1, name, word)
                {
                    return Some(
                        rows.into_iter()
                            .map(|hit| Candidate {
                                hit,
                                reason: Reason::Receiver(name.clone()),
                            })
                            .collect(),
                    );
                }
                return self.swift_outside_static(here, text, word, chain);
            };
            let label = links.join(" \u{2192} ");
            let arms = match self.swift_type(here, &written) {
                Some(ty) => vec![ty],
                None => self.swift_alias_arms(here, &written).unwrap_or_default(),
            };
            let Some(ty) = arms.first() else {
                if written == SDK_VALUE {
                    return (!self.swift_extended_with(here, word, false)).then(Vec::new);
                }
                let w = generic::unbound(&written).trim();
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
            let mut rows = self.swift_member_rows(here, ty, word)?;
            for arm in &arms[1..] {
                for h in self.swift_member_rows(here, arm, word)? {
                    if !rows.iter().any(|r| r.path == h.path && r.line1 == h.line1) {
                        rows.push(h);
                    }
                }
            }
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

    pub(super) fn swift_on_call(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        call: &str,
        then: &[String],
    ) -> Option<Vec<Candidate>> {
        let mut via = self.swift_expr(here, here, text, self.line + 1, call, 2)?;
        for method in then {
            let ty = self.swift_type(here, &via.written)?;
            let rows = self.swift_member_rows(here, &ty, method)?;
            via = self.swift_overloads_returning(&rows, &ty.name, method)?;
        }
        let WrittenVia {
            written,
            call_signature,
        } = via;
        let ty = self.swift_type(here, &written)?;
        let mut rows = self.swift_member_rows(here, &ty, word)?;
        rows.retain(|h| {
            search::swift_member_kind(&h.text) != Some(search::SwiftMember::StaticOrClass)
        });
        let label = call_signature.unwrap_or_else(|| format!("{call}: {}", ty.name));
        Some(
            rows.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        )
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
        let (first, skip) = match self.swift_value(here, file, text, line1, &names[0], hops) {
            Some(v) => (v, 1),
            None => (self.swift_enum_case(here, text, line1, names)?, 2),
        };
        let WrittenVia {
            mut written,
            call_signature: link,
        } = first;
        let shown = generic::shown;
        let mut links = vec![link.unwrap_or_else(|| format!("{}: {}", names[0], shown(&written)))];
        for (i, field) in names[skip..].iter().enumerate() {
            if i == 5 {
                return None;
            }
            let ty = self.swift_type(here, &written);
            if ty.as_ref().is_none_or(|t| t.keyword.is_none())
                && self.swift_sdk_value(here, &written, field)
            {
                links.push(format!("{field}: {SDK_VALUE}"));
                SDK_VALUE.clone_into(&mut written);
                continue;
            }
            let WrittenVia {
                written: next,
                call_signature: link,
            } = match ty {
                Some(ty) => self.swift_field(here, &ty, field, hops),
                None => self.swift_requirement(here, &written, field, hops),
            }
            .filter(|v| (i == 0 && names[0] == "self") || !generic::bound(&v.written))?;
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

    pub(super) fn swift_receiver_rows(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        method: &str,
    ) -> Option<Vec<Hit>> {
        let SwiftChain { written, .. } =
            self.swift_chain(here, here, text, self.line + 1, chain, 3)?;
        let ty = self
            .swift_type(here, &written)
            .filter(|t| t.keyword.is_some())?;
        self.swift_member_rows(here, &ty, method)
    }

    fn swift_enum_case(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        names: &[String],
    ) -> Option<WrittenVia> {
        let [own, case, ..] = names else {
            return None;
        };
        if !search::bindings(Kind::Swift, text, line1, own).is_empty() {
            return None;
        }
        let ty = self.swift_type(here, own)?;
        if ty.keyword.as_deref() != Some("enum") {
            return None;
        }
        let rows = self.swift_member_rows(here, &ty, case)?;
        let [row] = rows.as_slice() else {
            return None;
        };
        search::swift_case(&row.text).then(|| WrittenVia {
            call_signature: Some(format!("{own}.{case}: {}", ty.name)),
            written: ty.name,
        })
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
        if name == "0" {
            let lead = (file == here && line1 == self.line + 1)
                .then(|| lines[line1 - 1].get(..self.col))
                .flatten()?;
            let Some(each) = search::swift_closure_sequence(lead) else {
                let call = search::swift_trailing_closure(lead, name)?;
                return self.swift_closure_value(here, file, text, line1, call, hops);
            };
            let seq = self.swift_expr(here, file, text, line1, &each, hops)?;
            if seq.written.trim_end().ends_with(['?', '!']) {
                return None;
            }
            let given = search::SwiftGiven::Element(each);
            return self.swift_given_type(here, file, text, line1, given, hops);
        }
        if name == "super" {
            let own = search::swift_type_header(lines[around? - 1])?.name;
            let ty = self.swift_type(here, &own)?;
            let base = ty.base.filter(|_| ty.keyword.as_deref() == Some("class"))?;
            let up = self.swift_type(here, &base)?;
            return (up.keyword.as_deref() == Some("class")).then(|| WrittenVia::plain(up.name));
        }
        match search::bindings(Kind::Swift, text, line1, name).as_slice() {
            [b] => {
                let line = lines.get(b.line1 - 1)?;
                match search::swift_given(line, name) {
                    Some(given) => {
                        let given = self.swift_nested_given(here, text, b.line1, given);
                        self.swift_given_type(here, file, text, b.line1, given, hops)
                    }
                    None if b.line1 < line1 && search::swift_rebinds(line, name) => {
                        self.swift_value(here, file, text, b.line1, name, hops)
                    }
                    None => match search::swift_trailing_closure(line, name) {
                        Some(call) => {
                            let shadows = call.0.split('.').next() == Some(name);
                            let at = b.line1.checked_sub(usize::from(shadows))?;
                            self.swift_closure_value(here, file, text, at, call, hops)
                        }
                        None => {
                            self.swift_bare_closure_value(here, file, text, b.line1, name, hops)
                        }
                    },
                }
            }
            [] if file == here
                && line1 == self.line + 1
                && search::swift_closure_param(lines[line1 - 1], name, self.col) =>
            {
                let call = search::swift_trailing_closure(lines[line1 - 1].get(..self.col)?, name)?;
                self.swift_closure_value(here, file, text, line1, call, hops)
            }
            [] if file == here
                && line1 == self.line + 1
                && let Some(value) = lines[line1 - 1]
                    .get(..self.col)
                    .and_then(|lead| search::swift_bound_before(lead, name)) =>
            {
                let given = search::SwiftGiven::Value(value);
                self.swift_given_type(here, file, text, line1, given, hops)
            }
            [] => {
                let at = around?;
                let mut top = match search::swift_scope(&lines, &literal, line1 + 1).header_line1 {
                    h if h == line1 && h > at && line1 < lines.len() => line1,
                    _ => search::swift_scope(&lines, &literal, line1).header_line1,
                };
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

    fn swift_closure_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        (receiver, method): (String, String),
        hops: usize,
    ) -> Option<WrittenVia> {
        let names: Vec<String> = receiver.split('.').map(str::to_owned).collect();
        let held = self
            .swift_chain(here, file, text, line1, &names, hops.checked_sub(1)?)?
            .written;
        let ty = self
            .swift_type(here, &held)
            .filter(|t| t.keyword.is_some())?;
        let [param] = self.swift_type_generics(here, &ty.name)?;
        let [arg] = <[String; 1]>::try_from(search::swift_generic_arguments(&held)).ok()?;
        let rows = self.swift_member_rows(here, &ty, &method)?;
        let takes: Vec<String> = rows
            .iter()
            .filter(|h| h.text.contains("func "))
            .filter_map(|h| search::swift_closure_input(&h.text))
            .collect();
        let shadowed = rows
            .iter()
            .any(|h| search::swift_generics(&h.text).contains(&param));
        if takes.is_empty() || shadowed || takes.iter().any(|t| *t != param) {
            return None;
        }
        self.swift_given_type(here, file, text, line1, search::SwiftGiven::Type(arg), hops)
    }

    fn swift_bare_closure_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let lines: Vec<&str> = text.lines().collect();
        let (method, at, count) = search::swift_bare_closure(lines.get(line1 - 1)?, name)?;
        if !search::bindings(Kind::Swift, text, line1, &method).is_empty() {
            return None;
        }
        let literal = search::literal_lines(Kind::Swift, text);
        let around = search::swift_enclosing_type(&lines, &literal, line1)?;
        let ty = self.swift_type(here, &search::swift_type_header(lines[around - 1])?.name)?;
        ty.keyword.as_ref()?;
        let mut inputs = None;
        let mut generics = self.swift_scope_generics(here, text, line1);
        for row in self.swift_member_rows(here, &ty, &method)? {
            if !row.text.contains("func ") {
                return None;
            }
            let params = search::swift_params(&row.text)?;
            let (last, before) = params.split_last()?;
            if before.iter().any(|p| {
                p.contains("->")
                    || search::swift_type_name(p)
                        .is_some_and(|n| self.swift_project_alias(here, &n))
            }) {
                return None;
            }
            generics.extend(search::swift_generics(&row.text));
            if last.is_empty() || last.contains("@autoclosure") {
                continue;
            }
            let bare = last
                .split_whitespace()
                .filter(|w| !w.starts_with('@'))
                .collect::<Vec<_>>()
                .join(" ");
            if search::swift_constrained_generic(&row.text, &bare) {
                continue;
            }
            let takes = if let Some(takes) = search::swift_function_inputs(last) {
                takes
            } else {
                let [alias] =
                    <[Hit; 1]>::try_from(self.swift_member_rows(here, &ty, &bare)?).ok()?;
                search::swift_function_inputs(&search::swift_alias(&alias.text, &bare)?)?
            };
            if inputs.as_ref().is_some_and(|i| *i != takes) {
                return None;
            }
            inputs = Some(takes);
        }
        let inputs = inputs.filter(|i| i.len() == count)?;
        let input = inputs.get(at)?;
        let named = search::swift_type_name(input).or_else(|| search::swift_element(input))?;
        if generics.contains(&named)
            || ["Element", "Index", "Indices", "Iterator", "SubSequence"].contains(&named.as_str())
        {
            return None;
        }
        let given = search::SwiftGiven::Type(input.clone());
        self.swift_given_type(here, file, text, line1, given, hops)
    }

    fn swift_project_alias(&self, here: &Path, name: &str) -> bool {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        self.project_definitions(Kind::Swift, here, name, &pattern)
            .iter()
            .any(|h| search::swift_alias(&h.text, name).is_some())
    }

    fn swift_scope_generics(&self, here: &Path, text: &str, line1: usize) -> Vec<String> {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut out = Vec::new();
        let mut at = line1;
        while let Some(line) = at.checked_sub(1).and_then(|i| lines.get(i)) {
            out.extend(search::swift_generics(line));
            if let Some(t) = search::swift_extended(line) {
                out.extend(self.swift_decl_generics(here, &t).into_iter().flatten());
            }
            at = search::swift_scope(&lines, &literal, at).header_line1;
        }
        out
    }

    fn swift_nested_given(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        given: search::SwiftGiven,
    ) -> search::SwiftGiven {
        match given {
            search::SwiftGiven::Type(t) => {
                search::SwiftGiven::Type(self.swift_nested(here, text, line1, &t))
            }
            other => other,
        }
    }

    fn swift_nested(&self, here: &Path, text: &str, line1: usize, written: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let declared: Vec<(usize, String)> = (lines.iter().enumerate())
            .filter(|&(i, _)| !literal.get(i).copied().unwrap_or(false))
            .filter_map(|(i, l)| search::swift_type_header(l).map(|h| (i + 1, h)))
            .filter(|(_, h)| h.keyword != "extension")
            .map(|(at, h)| (at, h.name))
            .collect();
        search::swift_renamed_types(written, |name| {
            let mut scope = line1;
            let (up, head) = loop {
                if search::swift_generics(lines[scope - 1])
                    .iter()
                    .any(|g| g == name)
                {
                    return None;
                }
                let up = search::swift_scope(&lines, &literal, scope).header_line1;
                if up == 0 {
                    return None;
                }
                let head = search::swift_type_header(lines[up - 1]);
                if declared.iter().any(|(d, n)| {
                    n == name && search::swift_scope(&lines, &literal, *d).header_line1 == up
                }) {
                    break (up, head?);
                }
                if head.is_some_and(|h| h.first_inherited.is_some()) {
                    return None;
                }
                scope = up;
            };
            if self.swift_type(here, name).is_some() {
                return None;
            }
            let outer = search::qualified(Kind::Swift, text, up, &head.name).unwrap_or(head.name);
            let full = format!("{outer}.{name}");
            self.swift_type(here, &full).map(|_| full)
        })
    }

    fn swift_wraps(&self, here: &Path, ty: &SwiftType) -> bool {
        let Some([param]) = self.swift_type_generics(here, &ty.name) else {
            return false;
        };
        self.swift_member_rows(here, ty, "init")
            .is_some_and(|inits| matches!(inits.as_slice(), [init] if search::swift_init_takes(&init.text, &param)))
    }

    fn swift_type_generics(&self, here: &Path, name: &str) -> Option<[String; 1]> {
        let [generics] = <[Vec<String>; 1]>::try_from(self.swift_decl_generics(here, name)).ok()?;
        <[String; 1]>::try_from(generics).ok()
    }

    fn swift_decl_generics(&self, here: &Path, name: &str) -> Vec<Vec<String>> {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        self.project_definitions(Kind::Swift, here, name, &pattern)
            .iter()
            .filter(|h| {
                self.in_code(Kind::Swift, h)
                    && search::swift_type_header(&h.text)
                        .is_some_and(|t| t.name == name && t.keyword != "extension")
            })
            .map(|h| search::swift_generics(&h.text))
            .collect()
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
            search::SwiftGiven::Item(e, at) => {
                let tuple = self.swift_expr(here, file, text, line1, &e, hops)?;
                WrittenVia {
                    written: search::swift_tuple_item(&tuple.written, at)?,
                    call_signature: tuple.call_signature,
                }
            }
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
                return self
                    .swift_generic_outside(here, &lines, at, &name, &via.written)
                    .map(WrittenVia::plain);
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
        if let Some(call) = search::swift_array_call(e) {
            return self.swift_array_call(here, file, text, line1, call, hops);
        }
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
            && let Some(t) = self.swift_type(here, &self.swift_nested(here, text, line1, ty))
        {
            t.keyword.as_ref()?;
            let held = search::swift_sole_argument(e)
                .filter(|_| parts.len() == 1 && self.swift_wraps(here, &t))
                .and_then(|arg| {
                    self.swift_expr(here, file, text, line1, &arg, hops.checked_sub(1)?)
                })
                .filter(|arg| {
                    search::swift_type_name(&arg.written)
                        .or_else(|| search::swift_nested_type_name(&arg.written))
                        .is_some()
                })
                .map(|arg| format!("{}<{}>", t.name, arg.written));
            return Some(WrittenVia::plain(held.unwrap_or(t.name)));
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
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Swift, text);
            let mut at = line1;
            let mut own = None;
            while let Some(up) = search::swift_enclosing_type(&lines, &literal, at) {
                let ty = self.swift_type(here, &search::swift_type_header(lines[up - 1])?.name)?;
                let rows = self.swift_member_rows(here, &ty, method)?;
                if !rows.is_empty() {
                    own = Some((<[Hit; 1]>::try_from(rows).ok()?, ty.name));
                    break;
                }
                at = up;
            }
            match (own, <[Hit; 1]>::try_from(funcs)) {
                (Some(([decl], owner)), _) => (decl, Some(owner)),
                (None, Ok([decl])) => (decl, None),
                (None, Err(_)) => return None,
            }
        } else {
            let hops = hops.checked_sub(1)?;
            let (ty, rows) = match self.swift_chain(here, file, text, line1, receiver, hops) {
                Some(chain) => {
                    let ty = self.swift_type(here, &chain.written);
                    if ty.as_ref().is_none_or(|t| t.keyword.is_none())
                        && let Some((_, _, args)) = search::swift_literal_call(&format!("{e}."))
                        && self.swift_sdk_value(here, &chain.written, method)
                        && !self
                            .swift_scope_generics(here, text, line1)
                            .contains(&chain.written.trim().trim_end_matches(['?', '!']).to_owned())
                        && !self.swift_extended_with(here, method, false)
                        && self.swift_outside_args(here, file, text, line1, &args, hops)
                    {
                        return Some(WrittenVia::plain(SDK_VALUE.to_owned()));
                    }
                    let ty = ty?;
                    let rows = self.swift_member_rows(here, &ty, method)?;
                    (ty, rows)
                }
                None => match receiver {
                    [name] => self.swift_static_rows(here, text, line1, name, method)?,
                    _ => return None,
                },
            };
            return self.swift_overloads_returning(&rows, &ty.name, &callee);
        };
        self.swift_returning(&decl, owner, &callee)
    }

    fn swift_array_call(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        (items, compact, method): (Vec<String>, bool, String),
        hops: usize,
    ) -> Option<WrittenVia> {
        let hops = hops.checked_sub(1)?;
        let typed: Vec<String> = items
            .iter()
            .filter_map(|i| self.swift_expr(here, file, text, line1, i, hops))
            .map(|v| v.written.trim().to_owned())
            .filter(|w| w != SDK_VALUE)
            .map(|w| {
                if compact {
                    w.trim_end_matches('?').to_owned()
                } else {
                    w
                }
            })
            .collect();
        let element = typed.first()?;
        if typed.iter().any(|t| t != element)
            || !["String", "Int", "Double", "Bool"].contains(&element.as_str())
        {
            return None;
        }
        let array = format!("[{element}]");
        let rows = self
            .swift_outside_rows(here, &array, &method)
            .filter(|r| !r.is_empty())?;
        self.swift_overloads_returning(&rows, &array, &method)
    }

    fn swift_overloads_returning(
        &self,
        rows: &[Hit],
        owner: &str,
        callee: &str,
    ) -> Option<WrittenVia> {
        let mut vias = rows
            .iter()
            .map(|d| self.swift_returning(d, Some(owner.to_owned()), callee));
        let first = vias.next()??;
        vias.all(|v| v.is_some_and(|v| v.written == first.written))
            .then_some(first)
    }

    fn swift_returning(
        &self,
        decl: &Hit,
        owner: Option<String>,
        callee: &str,
    ) -> Option<WrittenVia> {
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
        let mut decl = row.text.clone();
        if matches!(search::swift_given(&decl, name)?, search::SwiftGiven::Type(t) if search::swift_opens_more(&t))
        {
            for more in text.lines().skip(row.line1).take(4) {
                decl.push(' ');
                decl.push_str(more.trim());
                if !search::swift_opens_more(&decl) {
                    break;
                }
            }
        }
        let given = search::swift_given(&decl, name)?;
        let given = self.swift_nested_given(here, &text, row.line1, given);
        self.swift_given_type(here, &row.path, &text, row.line1, given, hops)
    }

    fn swift_requirement(
        &self,
        here: &Path,
        written: &str,
        field: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let t = written.trim().trim_end_matches(['?', '!']);
        let t = t
            .strip_prefix('(')
            .and_then(|t| t.strip_suffix(')'))
            .unwrap_or(t)
            .trim();
        let t = t.strip_prefix("any ").unwrap_or(t);
        let p = search::swift_type_name(t).filter(|p| self.swift_protocol(here, p))?;
        let rows = self.swift_protocol_rows(here, &p, field, true)?;
        let [row] = <[Hit; 1]>::try_from(rows).ok()?;
        if !search::swift_instance_member(&row.text) || row.text.contains("func ") {
            return None;
        }
        let text = self.text_of(&row.path)?;
        let given = search::swift_given(&row.text, field)?;
        self.swift_given_type(here, &row.path, &text, row.line1, given, hops)
    }

    fn swift_type(&self, here: &Path, written: &str) -> Option<SwiftType> {
        self.swift_type_through(here, written, true)
    }

    #[expect(
        clippy::match_same_arms,
        reason = "a guarded arm sits between the two; merging them would move one past it"
    )]
    fn swift_type_through(&self, here: &Path, written: &str, alias: bool) -> Option<SwiftType> {
        let full =
            search::swift_type_name(written).or_else(|| search::swift_nested_type_name(written))?;
        let name = full.rsplit('.').next()?.to_owned();
        let nested = full != name;
        let plain = search::swift_unspecialized(&full).unwrap_or_else(|| full.clone());
        let pattern = search::def_patterns(Kind::Swift, &name).join("|");
        let mut decls = Vec::new();
        let mut extended = false;
        let mut aliases = Vec::new();
        for h in self.project_definitions(Kind::Swift, here, &name, &pattern) {
            if !self.in_code(Kind::Swift, &h) {
                continue;
            }
            let placed = || {
                !nested
                    || self
                        .text_of(&h.path)
                        .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, &name))
                        .is_some_and(|q| q == plain || q.ends_with(&format!(".{plain}")))
            };
            if let Some(target) = search::swift_alias(&h.text, &name) {
                if placed() {
                    aliases.push(target);
                }
                continue;
            }
            match search::swift_type_header(&h.text) {
                Some(t) if t.keyword == "extension" => extended |= t.name == full,
                Some(t) if t.name == name && placed() => {
                    decls.push((h, t.keyword, t.first_inherited));
                }
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
        if let ([], false, [target]) = (decls.as_slice(), extended, aliases.as_slice()) {
            return alias
                .then(|| self.swift_type_through(here, target, false))
                .flatten();
        }
        if !aliases.is_empty() {
            return None;
        }
        match decls.as_slice() {
            [] if extended => Some(SwiftType {
                owner: full.clone(),
                name: full,
                keyword: None,
                base: None,
            }),
            [(h, k, b)] if k != "protocol" => {
                let text = self.text_of(&h.path)?;
                Some(SwiftType {
                    owner: search::qualified(Kind::Swift, &text, h.line1, &name)
                        .unwrap_or_else(|| name.clone()),
                    name: full,
                    keyword: Some(k.clone()),
                    base: b.clone(),
                })
            }
            _ => None,
        }
    }

    fn swift_alias_arms(&self, here: &Path, written: &str) -> Option<Vec<SwiftType>> {
        let name = search::swift_type_name(written)?;
        let pattern = search::def_patterns(Kind::Swift, &name).join("|");
        let hits: Vec<Hit> = self
            .project_definitions(Kind::Swift, here, &name, &pattern)
            .into_iter()
            .filter(|h| self.in_code(Kind::Swift, h))
            .collect();
        let targets: Vec<String> = hits
            .iter()
            .filter_map(|h| search::swift_alias(&h.text, &name))
            .collect();
        let first = hits.first()?;
        if targets.len() != hits.len() || hits.iter().any(|h| h.path != first.path) {
            return None;
        }
        let text = self.text_of(&first.path)?;
        let lines: Vec<&str> = text.lines().collect();
        let at: Vec<usize> = hits.iter().map(|h| h.line1).collect();
        if !search::swift_if_arms(&lines, &at) {
            return None;
        }
        targets
            .iter()
            .map(|t| {
                self.swift_type_through(here, t, false)
                    .filter(|ty| ty.keyword.is_some())
            })
            .collect()
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
        if let Some((outer, _)) = name.split_once('.') {
            let pattern = search::def_patterns(Kind::Swift, outer).join("|");
            if !self
                .project_definitions(Kind::Swift, here, outer, &pattern)
                .iter()
                .all(|h| search::swift_extension(&h.text))
            {
                return None;
            }
        }
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
                let defs = self.project_definitions(Kind::Swift, here, &name, &pattern);
                if let [h] = defs.as_slice()
                    && self.in_code(Kind::Swift, h)
                    && let Some(target) = search::swift_alias(&h.text, &name)
                    && target.trim_start().starts_with('[')
                {
                    return self.swift_outside_rows(here, &target, word);
                }
                if !defs.is_empty() {
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
                        || !name.contains('.')
                            && [
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

    pub(super) fn swift_cast_rows(
        &self,
        here: &Path,
        cast: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let rows = self
            .swift_outside_rows(here, cast, word)
            .filter(|rows| !rows.is_empty() || !self.swift_extended_with(here, word, false))?;
        Some(
            rows.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(cast.to_owned()),
                })
                .collect(),
        )
    }

    fn swift_sdk_value(&self, here: &Path, written: &str, member: &str) -> bool {
        let t = written.trim().trim_end_matches(['?', '!']);
        if t == SDK_VALUE {
            return !self.swift_extended_with(here, member, false);
        }
        if let Some(item) = search::swift_dictionary_part(t, member) {
            let pattern = search::def_patterns(Kind::Swift, &item).join("|");
            return matches!(
                item.as_str(),
                "Any" | "AnyHashable" | "String" | "Substring" | "Int" | "Double" | "Bool"
            ) && self
                .project_definitions(Kind::Swift, here, &item, &pattern)
                .iter()
                .all(|h| search::swift_extension(&h.text))
                && !self.swift_extended_with(here, member, true);
        }
        t.starts_with(|c: char| c.is_ascii_uppercase())
            && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && self.swift_alias_arms(here, t).is_none()
            && self
                .swift_outside_rows(here, t, member)
                .is_some_and(|rows| rows.is_empty())
            && !self.swift_extended_with(here, member, true)
    }

    fn swift_extended_with(&self, here: &Path, word: &str, known: bool) -> bool {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        self.project_definitions(Kind::Swift, here, word, &pattern)
            .iter()
            .any(|h| {
                self.text_of(&h.path).is_some_and(|text| {
                    let lines: Vec<&str> = text.lines().collect();
                    let literal = search::literal_lines(Kind::Swift, &text);
                    let at = search::swift_scope(&lines, &literal, h.line1).header_line1;
                    at > 0
                        && search::swift_extended(lines[at - 1]).is_some_and(|t| {
                            let outer = t.split('.').next().unwrap_or_default();
                            self.swift_type(here, outer)
                                .is_none_or(|own| own.keyword.is_none())
                                && !(known && self.swift_protocol(here, outer))
                        })
                })
            })
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

impl App {
    pub(super) fn swift_expected_rows(
        &self,
        here: &Path,
        text: &str,
        lead: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        if self.root.join(".build/checkouts").exists() {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let line1 = self.line + 1;
        let written = match search::swift_called_closure(lead) {
            Some(callee) => {
                let bound = search::bindings(Kind::Swift, text, line1, &callee);
                let [b] = bound.as_slice() else {
                    return None;
                };
                let search::SwiftGiven::Type(t) =
                    search::swift_given(lines.get(b.line1 - 1)?, &callee)?
                else {
                    return None;
                };
                search::swift_closure_takes(&t)?
            }
            None => match search::swift_expected(&lines, &literal, line1, lead) {
                Some(w) => w,
                None => match self.swift_argument_rows(here, lead, word)? {
                    Ok(rows) => return Some(rows),
                    Err(outside) => outside,
                },
            },
        };
        let written = written.trim_end_matches(['?', '!']).to_owned();
        let first = written.split('.').next()?.to_owned();
        let mut scope = search::swift_scope(&lines, &literal, line1).header_line1;
        while scope > 0 {
            if first == "Self" || search::swift_generics(lines[scope - 1]).contains(&first) {
                return None;
            }
            scope = search::swift_scope(&lines, &literal, scope).header_line1;
        }
        let implicit = |h: &Hit| {
            search::swift_case(&h.text)
                || search::swift_member_kind(&h.text) == Some(search::SwiftMember::StaticOrClass)
        };
        let cut = self.truncated.get();
        let rows = (|| {
            if written == first {
                let mut rows = match self.swift_type(here, &written) {
                    Some(ty) if ty.keyword.is_some() => {
                        let rows = self.swift_member_rows(here, &ty, word)?;
                        (!rows.is_empty()).then_some(rows)?
                    }
                    _ => self.swift_outside_rows(here, &written, word)?,
                };
                rows.retain(|h| implicit(h));
                return Some(rows);
            }
            let full = format!("{written}.{word}");
            let pattern = search::def_patterns(Kind::Swift, word).join("|");
            let rows: Vec<Hit> = self
                .project_definitions(Kind::Swift, here, word, &pattern)
                .into_iter()
                .filter(|h| {
                    implicit(h)
                        && self.in_code(Kind::Swift, h)
                        && self
                            .text_of(&h.path)
                            .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, word))
                            .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")))
                })
                .collect();
            if !rows.is_empty() {
                return Some(rows);
            }
            let pattern = search::def_patterns(Kind::Swift, &first).join("|");
            self.project_definitions(Kind::Swift, here, &first, &pattern)
                .iter()
                .all(|h| search::swift_extension(&h.text))
                .then(Vec::new)
        })();
        self.truncated.set(cut);
        let label = format!(".{word}: {written}");
        Some(
            rows?
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        )
    }
}

impl App {
    fn swift_static_rows(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        name: &str,
        word: &str,
    ) -> Option<(SwiftType, Vec<Hit>)> {
        if !name.starts_with(char::is_uppercase)
            || name == "Self"
            || !search::bindings(Kind::Swift, text, line1, name).is_empty()
        {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut scope = search::swift_scope(&lines, &literal, line1).header_line1;
        while scope > 0 {
            if search::swift_generics(lines[scope - 1])
                .iter()
                .any(|g| g == name)
            {
                return None;
            }
            scope = search::swift_scope(&lines, &literal, scope).header_line1;
        }
        let ty = self
            .swift_type(here, name)
            .filter(|t| t.keyword.is_some())?;
        let mut rows = self.swift_member_rows(here, &ty, word)?;
        rows.retain(|h| {
            search::swift_member_kind(&h.text) == Some(search::SwiftMember::StaticOrClass)
        });
        (!rows.is_empty()).then_some((ty, rows))
    }

    pub(super) fn swift_outside_call(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> bool {
        let Some((receiver, method, args)) = search::swift_literal_call(before).or_else(|| {
            search::swift_subscript_receiver(before).map(|r| (r, "subscript".to_owned(), vec![]))
        }) else {
            return false;
        };
        let names: Vec<String> = receiver.split('.').map(str::to_owned).collect();
        let line = text.lines().nth(self.line).unwrap_or_default();
        let shadowed =
            |n: &str| search::swift_closure_param(line, n.split('.').next().unwrap_or(n), self.col);
        if shadowed(&receiver) || args.iter().any(|a| shadowed(a)) {
            return false;
        }
        let cut = self.truncated.get();
        let chain = self.swift_chain(here, here, text, self.line + 1, &names, 2);
        let outside = chain.is_some_and(|c| {
            self.swift_type(here, &c.written)
                .is_none_or(|t| t.keyword.is_none())
                && self.swift_sdk_value(here, &c.written, &method)
                && (method != "subscript"
                    || !self
                        .swift_project_alias(here, c.written.trim().trim_end_matches(['?', '!'])))
        }) && !self.swift_extended_with(here, word, false)
            && self.swift_outside_args(here, here, text, self.line + 1, &args, 2);
        self.truncated.set(cut);
        outside
    }

    fn swift_outside_args(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        args: &[String],
        hops: usize,
    ) -> bool {
        args.iter().all(|a| {
            let names: Vec<String> = a.split('.').map(str::to_owned).collect();
            self.swift_chain(here, file, text, line1, &names, hops)
                .is_some_and(|c| {
                    let t = c.written.trim().trim_end_matches(['?', '!']);
                    t == SDK_VALUE
                        || t.starts_with(|c: char| c.is_ascii_uppercase())
                            && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                            && self
                                .swift_type(here, t)
                                .is_none_or(|ty| ty.keyword.is_none())
                            && self.swift_alias_arms(here, t).is_none()
                            && !self.swift_project_alias(here, t)
                })
        })
    }

    fn swift_outside_static(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
    ) -> Option<Vec<Candidate>> {
        let [name] = chain else {
            return None;
        };
        if !name.starts_with(char::is_uppercase)
            || name == "Self"
            || !word.starts_with(char::is_lowercase)
            || matches!(word, "self" | "init")
            || !search::bindings(Kind::Swift, text, self.line + 1, name).is_empty()
        {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut scope = search::swift_scope(&lines, &literal, self.line + 1).header_line1;
        while scope > 0 {
            if search::swift_generics(lines[scope - 1]).contains(name) {
                return None;
            }
            scope = search::swift_scope(&lines, &literal, scope).header_line1;
        }
        let package = std::fs::read_to_string(self.root.join("Package.swift")).unwrap_or_default();
        let module = self.root.file_name().is_some_and(|d| d == name.as_str())
            || package.contains(&format!("\"{name}\""))
            || (self.files.iter()).any(|f| {
                f.parent()
                    .is_some_and(|d| d.iter().any(|c| c == name.as_str()))
            });
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        if module
            || !self
                .project_definitions(Kind::Swift, here, name, &pattern)
                .iter()
                .all(|h| search::swift_extension(&h.text))
        {
            return None;
        }
        let rows = self.swift_outside_rows(here, name, word)?;
        let label = name.clone();
        Some(
            rows.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        )
    }
}
