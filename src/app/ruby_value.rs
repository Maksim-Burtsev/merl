use super::{App, Candidate, Kind, Path, Reason, search};

impl App {
    pub(super) fn ruby_receiver_methods(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> Vec<Candidate> {
        let line1 = self.line + 1;
        let joined = search::ruby_joined_before(text, line1, before);
        let Some(receiver) = joined.strip_suffix('.') else {
            return Vec::new();
        };
        let receiver = receiver.strip_suffix('&').unwrap_or(receiver);
        let (prefix, expr) = search::ruby_receiver_split(receiver);
        match self.ruby_value_class(here, text, line1, prefix, expr, 0) {
            Some(class) => self.ruby_methods_from(here, word, class, false),
            None => self.ruby_struct_member(here, text, line1, expr, word),
        }
    }

    fn ruby_value_class(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        prefix: &str,
        expr: &str,
        depth: usize,
    ) -> Option<String> {
        if depth > 8 {
            return None;
        }
        let expr = expr.trim();
        if let Some(name) = search::ruby_found_record(expr) {
            let class = self.ruby_constant_class(here, name)?;
            return self.ruby_record(here, class);
        }
        if let Some((receiver, method)) = search::ruby_last_call(expr) {
            if method == "new" && receiver.starts_with(|c: char| c.is_ascii_uppercase() || c == ':')
            {
                let class = self.ruby_constant_class(here, receiver)?;
                return self
                    .ruby_methods_from(here, "new", class.clone(), true)
                    .is_empty()
                    .then_some(class);
            }
            if receiver.starts_with(|c: char| c.is_ascii_uppercase())
                && receiver
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
            {
                let class = self.ruby_constant_class(here, receiver)?;
                return self.ruby_returned_class(here, &class, method, depth, true);
            }
            let class = self.ruby_value_class(here, text, line1, prefix, receiver, depth + 1)?;
            return self.ruby_returned_class(here, &class, method, depth, false);
        }
        if !search::ruby_local_name(expr) {
            return None;
        }
        match prefix.chars().next_back() {
            Some('@') if !prefix.ends_with("@@") => {
                return self.ruby_ivar_class(here, text, line1, expr, depth);
            }
            Some('@' | '$') => return None,
            _ => {}
        }
        if let Some(tapped) = search::ruby_tapped(prefix, expr) {
            let (head, receiver) = search::ruby_receiver_split(tapped);
            return self.ruby_value_class(here, text, line1, head, receiver, depth + 1);
        }
        if let Some(name) = search::ruby_param_type(text, line1, expr) {
            return self.ruby_resolve(here, &search::ruby_class_path(text, line1), &name);
        }
        let Some((at, value)) = search::ruby_assigned_once(text, line1, prefix, expr) else {
            let def = format!(r"(?m)^\s*def\s+{}\s*$", regex::escape(expr));
            if !regex::Regex::new(&def).is_ok_and(|re| re.is_match(text)) {
                return None;
            }
            let class = search::ruby_class_path(text, line1);
            let bound = !search::ruby_locals(text, line1, expr).is_empty()
                || !search::bindings(Kind::Ruby, text, line1, expr).is_empty()
                || search::ruby_rebound(
                    text,
                    "",
                    search::ruby_gate_line(text, line1),
                    line1,
                    prefix,
                    expr,
                );
            return (!bound && !class.is_empty() && !search::ruby_self_is_class(text, line1))
                .then(|| self.ruby_returned_class(here, &class, expr, depth, false))
                .flatten();
        };
        let (head, whole) = search::ruby_receiver_split(&value);
        if !head.trim().is_empty() {
            return None;
        }
        if whole.trim() == "self" {
            let class = search::ruby_class_path(text, at);
            let line = text.lines().nth(at.checked_sub(1)?)?;
            return (!class.is_empty()
                && !search::ruby_self_is_class(text, at)
                && !search::ruby_on_class(text, at)
                && !search::ruby_self_rebound(text, at, line))
            .then_some(class);
        }
        self.ruby_value_class(here, text, at, "", whole, depth + 1)
    }

    pub(super) fn ruby_constant_class(&self, here: &Path, name: &str) -> Option<String> {
        if let Some(class) = self.ruby_class_named(here, name) {
            return Some(class);
        }
        let last = name.rsplit("::").next().unwrap_or(name);
        let pattern = search::ruby_constant_alias(last);
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::Ruby, here, last, &pattern);
        self.truncated.set(cut);
        let [hit] = hits.as_slice() else {
            return None;
        };
        let t = self.text_of(&hit.path)?;
        let target = search::ruby_alias_target(&hit.text, last)?;
        let scope = search::ruby_class_path(&t, hit.line1);
        self.ruby_resolve(here, &scope, target)
    }

    fn ruby_returned_class(
        &self,
        here: &Path,
        class: &str,
        method: &str,
        depth: usize,
        on_class: bool,
    ) -> Option<String> {
        let found = self.ruby_methods_from(here, method, class.to_owned(), on_class);
        let [one] = found.as_slice() else {
            return None;
        };
        let t = self.text_of(&one.hit.path)?;
        let owner = search::ruby_class_path(&t, one.hit.line1);
        let made = match search::ruby_last_value(&t, one.hit.line1)? {
            search::RubyReturns::Itself if !on_class => return Some(owner),
            search::RubyReturns::New(name) => {
                self.ruby_resolve(here, &owner, &name).or_else(|| {
                    (owner == name || owner.ends_with(&format!("::{name}"))).then(|| owner.clone())
                })?
            }
            search::RubyReturns::Fresh if on_class => class.to_owned(),
            search::RubyReturns::Found(name) => {
                let found = self.ruby_resolve(here, &owner, &name)?;
                return self.ruby_record(here, found);
            }
            search::RubyReturns::Chain(line1, chain) if !on_class => {
                let (prefix, expr) = search::ruby_receiver_split(&chain);
                return self.ruby_value_class(here, &t, line1, prefix, expr, depth + 1);
            }
            _ => return None,
        };
        self.ruby_methods_from(here, "new", made.clone(), true)
            .is_empty()
            .then_some(made)
    }

    fn ruby_record(&self, here: &Path, class: String) -> Option<String> {
        let mut c = class.clone();
        let mut record = false;
        for _ in 0..8 {
            let supers = self.ruby_parents(here, &c).superclasses;
            record = supers
                .iter()
                .any(|s| s.trim_start_matches(':') == "ActiveRecord::Base");
            let ([one], false) = (supers.as_slice(), record) else {
                break;
            };
            let Some(up) = self.ruby_resolve(here, &c, one) else {
                break;
            };
            c = up;
        }
        (record
            && self
                .ruby_methods_from(here, "find", class.clone(), true)
                .is_empty())
        .then_some(class)
    }

    fn ruby_ivar_class(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        name: &str,
        depth: usize,
    ) -> Option<String> {
        let ivar = format!("@{name}");
        let class = search::ruby_class_path(text, line1);
        if class.is_empty() {
            return None;
        }
        let assign = regex::Regex::new(&search::ruby_assignment(&ivar)).ok()?;
        let rebind = search::RubyRebinding::ivar(name)?;
        let writer = regex::Regex::new(r"\battr_(?:writer|accessor)\b").ok()?;
        let setter = format!(r"\.{}\s*=(?:[^=~>]|$)", regex::escape(name));
        let called = std::cell::OnceCell::new();
        let mut files = self.ruby_declarations(here, &class);
        files.dedup_by(|a, b| a.0.path == b.0.path);
        let assigned: Vec<(usize, &str, &str)> = files
            .iter()
            .flat_map(|(_, t)| {
                t.lines()
                    .enumerate()
                    .filter(|(i, l)| {
                        rebind.count(l) > 0
                            && search::ruby_class_path(t, i + 1) == class
                            && (!writer.is_match(l)
                                || *called.get_or_init(|| {
                                    !self.project_grep(Kind::Ruby, here, &setter).is_empty()
                                }))
                    })
                    .map(move |(i, l)| (i + 1, l, t.as_str()))
            })
            .collect();
        let [(line, assignment, t)] = assigned[..] else {
            return None;
        };
        if !assign.is_match(assignment)
            || rebind.count(assignment) > 1
            || search::ruby_self_is_class(t, line)
            || search::ruby_self_is_class(text, line1)
        {
            return None;
        }
        let value = assignment.trim_start().strip_prefix(&ivar)?.trim_start();
        let value = value
            .strip_prefix("||=")
            .or_else(|| value.strip_prefix('='))?
            .trim();
        let (head, whole) = search::ruby_receiver_split(value);
        if !head.trim().is_empty() {
            return None;
        }
        self.ruby_value_class(here, t, line, "", whole, depth + 1)
    }

    pub(super) fn ruby_super(&self, here: &Path, text: &str) -> Vec<Candidate> {
        let line1 = self.line + 1;
        let Some(method) = search::ruby_enclosing_def(text, line1) else {
            return Vec::new();
        };
        let class = search::ruby_class_path(text, line1);
        if class.is_empty() {
            return Vec::new();
        }
        let parents = self.ruby_parents(here, &class);
        for name in parents.includes.iter().rev().chain(&parents.superclasses) {
            let Some(path) = self.ruby_resolve(here, &class, name) else {
                return Vec::new();
            };
            let found = self.ruby_methods_from(here, &method, path, false);
            if !found.is_empty() {
                return found;
            }
        }
        Vec::new()
    }

    fn ruby_struct_member(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        expr: &str,
        word: &str,
    ) -> Vec<Candidate> {
        let made = match search::ruby_last_call(expr.trim()) {
            Some((made, "new")) => made.to_owned(),
            None if search::ruby_local_name(expr.trim()) => {
                let Some((_, value)) = search::ruby_assigned_once(text, line1, "", expr.trim())
                else {
                    return Vec::new();
                };
                match search::ruby_last_call(&value) {
                    Some((made, "new")) => made.to_owned(),
                    _ => return Vec::new(),
                }
            }
            _ => return Vec::new(),
        };
        if !made
            .trim_start_matches(':')
            .starts_with(|c: char| c.is_ascii_uppercase())
            || self.ruby_class_named(here, &made).is_some()
        {
            return Vec::new();
        }
        let last = made.rsplit("::").next().unwrap_or(&made);
        let cut = self.truncated.get();
        let hits =
            self.project_definitions(Kind::Ruby, here, last, &search::ruby_struct_line(last));
        self.truncated.set(cut);
        match hits.as_slice() {
            [hit] if search::ruby_struct_member(&hit.text, word) => vec![Candidate {
                hit: hit.clone(),
                reason: Reason::Receiver(last.to_owned()),
            }],
            _ => Vec::new(),
        }
    }
}
