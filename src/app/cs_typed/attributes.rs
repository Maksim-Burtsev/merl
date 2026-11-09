use super::{CsAnswer, Up, one_namespace, receivers};
use crate::app::{App, Hit, Kind, Path, Regex, Typed, search};

impl App {
    pub(in crate::app) fn cs_attribute_slot(&self) -> bool {
        self.definition_word(Some(Kind::CSharp))
            .and_then(|(r, _)| self.line_str().get(..r.start).map(attribute_slot))
            .unwrap_or(false)
    }

    pub(in crate::app) fn cs_attribute_types(&self, here: &Path, word: &str) -> Vec<Hit> {
        let name = format!("{word}Attribute");
        let cut = self.truncated.get();
        let pattern = search::def_patterns(Kind::CSharp, &name)[..4].join("|");
        let mut hits = self.project_definitions(Kind::CSharp, here, &name, &pattern);
        self.truncated.set(cut);
        hits.retain(|h| search::cs_type_decl(&h.text).is_some_and(|d| d.name == name));
        hits
    }

    pub(in crate::app) fn cs_in_scope(&self, here: &Path, word: &str, hits: Vec<Hit>) -> Vec<Hit> {
        let text = self.buf.lines.join("\n");
        if here.extension().is_none_or(|e| e != "cs") || !one_namespace(&text, None) {
            return hits;
        }
        let Some(seen) = self.cs_namespaces_seen(here, &text, word) else {
            return hits;
        };
        let ctor = Regex::new(&search::def_patterns(Kind::CSharp, word)[4])
            .expect("an escaped name keeps the pattern valid");
        hits.into_iter()
            .filter(|h| {
                let Some(t) = self
                    .text_of(&h.path)
                    .filter(|t| one_namespace(t, Some(h.line1)))
                else {
                    return true;
                };
                let top = match search::cs_owner(&t, h.line1) {
                    None => search::cs_type_decl(&h.text).is_some(),
                    Some(d) => search::cs_owner(&t, d).is_none() && ctor.is_match(&h.text),
                };
                h.deleted.is_some() || !top || seen.contains(&search::cs_namespace(&t, h.line1))
            })
            .collect()
    }

    fn cs_namespaces_seen(&self, here: &Path, text: &str, word: &str) -> Option<Vec<String>> {
        let mut usings: Vec<search::CsUsing> = search::cs_code_lines(text)
            .filter_map(|(_, l)| search::cs_using(l))
            .collect();
        for h in self.project_grep(Kind::CSharp, here, r"^\u{feff}?\s*global\s+using\s") {
            if self.hidden_now(Kind::CSharp, &h).get(h.line1 - 1) != Some(&true) {
                usings.extend(search::cs_using(&h.text));
            }
        }
        let csproj = |p: &Path| p.extension().is_some_and(|e| e == "csproj");
        let mut seen = Vec::new();
        for h in self.grep(r"<Using\b", false, false, csproj).ok()? {
            if h.text.contains("Alias") {
                return None;
            }
            seen.extend(search::cs_usings(&h.text));
        }
        let names = [word.to_owned(), format!("{word}Attribute")];
        for u in usings {
            if u.alias.as_ref().is_some_and(|a| names.contains(a)) {
                return None;
            }
            if u.alias.is_none() && !u.fixed {
                seen.push(u.target.trim_start_matches("global::").to_owned());
            }
        }
        let mut at = search::cs_namespace(text, self.line + 1);
        while let Some((outer, _)) = at.rsplit_once('.') {
            let outer = outer.to_owned();
            seen.push(std::mem::replace(&mut at, outer));
        }
        seen.push(at);
        seen.push(String::new());
        Some(seen)
    }

    pub(in crate::app) fn cs_attribute_argument(
        &self,
        here: &Path,
        word: &str,
        range: &std::ops::Range<usize>,
    ) -> Option<CsAnswer> {
        let line = self.line_str();
        let name = attribute_of_argument(line.get(..range.start)?)?;
        let after = line.get(range.end..)?.trim_start();
        if !after.starts_with('=') || after.starts_with("==") {
            return None;
        }
        let cut = self.truncated.get();
        let pattern = search::def_patterns(Kind::CSharp, &name)[..4].join("|");
        let mut types = self.project_definitions(Kind::CSharp, here, &name, &pattern);
        self.truncated.set(cut);
        types.retain(|h| search::cs_type_decl(&h.text).is_some_and(|d| d.name == name));
        types.extend(self.cs_attribute_types(here, &name));
        let label = format!("[{name}]");
        let outside = Some(CsAnswer::Outside {
            links: label.clone(),
        });
        let Ok([t]) = <[Hit; 1]>::try_from(self.cs_in_scope(here, &name, types)) else {
            return outside;
        };
        let ty = Typed {
            name: search::cs_type_decl(&t.text)?.name,
            path: t.path,
            line: t.line1,
        };
        match self.cs_up_call(&ty, word) {
            Up::Found(hits) => Some(CsAnswer::Found(receivers(hits, &label))),
            _ => outside,
        }
    }
}

fn attribute_slot(before: &str) -> bool {
    static SLOT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:\w+\s*:)?(?:[^\[\]]*,)?\s*$").expect("the regex is a literal")
    });
    let Some((head, rest)) = before.rsplit_once('[') else {
        return false;
    };
    let head = head.trim_end();
    (head.is_empty() || head.ends_with(['(', ',', ']']))
        && SLOT.is_match(rest)
        && rest.matches('(').count() == rest.matches(')').count()
}

fn attribute_of_argument(before: &str) -> Option<String> {
    static NAME: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.])(\w+)\s*$").expect("the regex is a literal")
    });
    let mut depth = 0i32;
    let open = (0..before.len()).rev().find(|&i| {
        depth += match before.as_bytes()[i] {
            b')' => 1,
            b'(' => -1,
            _ => 0,
        };
        depth < 0
    })?;
    let inside = before.get(open + 1..)?.trim_end();
    if !(inside.is_empty() || inside.ends_with(',')) {
        return None;
    }
    let head = before.get(..open)?.trim_end();
    let c = NAME.captures(head)?;
    let name = c.get(1)?;
    attribute_slot(head.get(..name.start())?).then(|| name.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{attribute_of_argument, attribute_slot};

    #[test]
    fn an_attribute_slot_is_read_from_the_text_before_the_name() {
        assert!(attribute_slot("    ["));
        assert!(attribute_slot("    [HttpGet, "));
        assert!(attribute_slot("    [Route(\"x\"), "));
        assert!(attribute_slot("[return: "));
        assert!(attribute_slot("    public void M(["));
        assert!(!attribute_slot("    var x = items["));
        assert!(!attribute_slot("    [Route(\"x\", "));
        assert!(!attribute_slot("    x = new "));
    }

    #[test]
    fn a_named_argument_names_its_attribute() {
        assert_eq!(
            attribute_of_argument("    [Authorize(").as_deref(),
            Some("Authorize")
        );
        assert_eq!(
            attribute_of_argument("    [HttpGet, Project(\"a\", ").as_deref(),
            Some("Project")
        );
        assert_eq!(attribute_of_argument("    Call(x, "), None);
        assert_eq!(
            attribute_of_argument("    [Route(Make(1), "),
            Some("Route".to_owned())
        );
        assert_eq!(attribute_of_argument("    [Route(\"a\" + "), None);
    }
}
