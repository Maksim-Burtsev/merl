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
        if let Some((receiver, method)) = search::ruby_last_call(expr) {
            if method == "new" && receiver.starts_with(|c: char| c.is_ascii_uppercase() || c == ':')
            {
                let class = self.ruby_constant_class(here, receiver)?;
                return self
                    .ruby_methods_from(here, "new", class.clone(), true)
                    .is_empty()
                    .then_some(class);
            }
            let class = self.ruby_value_class(here, text, line1, prefix, receiver, depth + 1)?;
            return self.ruby_returned_class(here, &class, method);
        }
        if !search::ruby_local_name(expr) {
            return None;
        }
        if let Some(tapped) = search::ruby_tapped(prefix, expr) {
            let (head, receiver) = search::ruby_receiver_split(tapped);
            return self.ruby_value_class(here, text, line1, head, receiver, depth + 1);
        }
        let (at, value) = search::ruby_assigned_once(text, line1, expr)?;
        let (head, whole) = search::ruby_receiver_split(&value);
        if !head.trim().is_empty() {
            return None;
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

    fn ruby_returned_class(&self, here: &Path, class: &str, method: &str) -> Option<String> {
        let found = self.ruby_methods_from(here, method, class.to_owned(), false);
        let [one] = found.as_slice() else {
            return None;
        };
        let t = self.text_of(&one.hit.path)?;
        let owner = search::ruby_class_path(&t, one.hit.line1);
        match search::ruby_last_value(&t, one.hit.line1)? {
            search::RubyReturns::Itself => Some(owner),
            search::RubyReturns::New(name) => {
                let made = self.ruby_resolve(here, &owner, &name).or_else(|| {
                    (owner == name || owner.ends_with(&format!("::{name}"))).then(|| owner.clone())
                })?;
                self.ruby_methods_from(here, "new", made.clone(), true)
                    .is_empty()
                    .then_some(made)
            }
        }
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
                let Some((_, value)) = search::ruby_assigned_once(text, line1, expr.trim()) else {
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
