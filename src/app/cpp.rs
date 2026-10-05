use super::c::{c_header, method_pattern};
use super::*;

pub(super) enum CppAnswer {
    Found(Vec<Candidate>),
    Outside,
    ByName(Option<String>),
}

#[derive(Clone)]
struct Class {
    path: PathBuf,
    text: String,
    code: String,
    open: usize,
    name: String,
}

impl App {
    pub(super) fn cpp_typed(
        &mut self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> CppAnswer {
        let Some((head, fields)) = search::cpp_receiver(before) else {
            return CppAnswer::ByName(None);
        };
        let broke = |at: &str| CppAnswer::ByName((!fields.is_empty()).then(|| at.to_owned()));
        if let Some(seventh) = fields.get(5) {
            return broke(seventh);
        }
        let Some((mut ty, mut scope)) = self.cpp_head_type(here, text, &head, word) else {
            return broke(&head);
        };
        let mut around = search::c_method_class(text, self.line + 1, word);
        let mut links = Vec::new();
        let mut prev = head.clone();
        let last = fields.len();
        for (i, name) in fields.iter().map(String::as_str).chain([word]).enumerate() {
            if ty[0] == "std" {
                return match i == last {
                    true => CppAnswer::Outside,
                    false => broke(&prev),
                };
            }
            links.push(format!("{prev}: {}", ty[ty.len() - 1]));
            let Some(class) = self.cpp_class(here, &ty, &scope, around.as_deref()) else {
                return broke(&prev);
            };
            let Some((owner, ats)) = self.cpp_member(here, &class, name, 0) else {
                return broke(&prev);
            };
            if i == last {
                return CppAnswer::Found(self.cpp_candidates(here, &owner, &ats, word, &links));
            }
            let Some(next) =
                (ats.iter()).find_map(|&at| search::cpp_type_at(&owner.code, at, name.len()))
            else {
                return broke(name);
            };
            (ty, scope, around) = (next, owner.code, Some(owner.name));
            prev = name.to_owned();
        }
        CppAnswer::ByName(None)
    }

    fn cpp_head_type(
        &mut self,
        here: &Path,
        text: &str,
        head: &str,
        word: &str,
    ) -> Option<(Vec<String>, String)> {
        let code = search::c_code(text);
        if head == "this" {
            return Some((
                vec![search::c_method_class(text, self.line + 1, word)?],
                code,
            ));
        }
        match search::cpp_value_type(text, self.line + 1, head) {
            Ok(Some(t)) => return Some((t, code)),
            Ok(None) => {}
            Err(()) => return None,
        }
        let class = search::c_method_class(text, self.line + 1, word)?;
        let class = self.cpp_class(here, &[class], &code, None)?;
        let (owner, ats) = self.cpp_member(here, &class, head, 0)?;
        let t = (ats.iter()).find_map(|&at| search::cpp_type_at(&owner.code, at, head.len()))?;
        Some((t, owner.code))
    }

    fn cpp_class(
        &mut self,
        here: &Path,
        ty: &[String],
        scope: &str,
        around: Option<&str>,
    ) -> Option<Class> {
        let name = ty.last()?;
        if ty.len() == 1 && search::cpp_template_param(scope, name) {
            return None;
        }
        let p = search::def_patterns(Kind::C, name);
        let hits = self.project_definitions(Kind::C, here, name, &p[2]);
        let hits = self.c_reached_only(here, name, hits);
        let mut bodies: Vec<(Class, Option<String>)> = Vec::new();
        for h in hits {
            let Some(text) = self.text_of(&h.path) else {
                continue;
            };
            let code = search::c_code(&text);
            let Some(open) = search::cpp_class_body(&code, h.line, name) else {
                continue;
            };
            if bodies
                .iter()
                .any(|(b, _)| b.path == h.path && b.open == open)
            {
                continue;
            }
            let lines: Vec<&str> = text.lines().collect();
            let nested = search::c_class_around(&lines, h.line);
            let class = Class {
                path: h.path,
                text: text.clone(),
                code,
                open,
                name: name.clone(),
            };
            bodies.push((class, nested));
        }
        let outer = match ty {
            [.., o, _] => Some(o.as_str()),
            _ => around,
        };
        if outer.is_some() && bodies.iter().any(|(_, n)| n.as_deref() == outer) {
            bodies.retain(|(_, n)| n.as_deref() == outer);
        } else if bodies.iter().any(|(_, n)| n.is_none()) {
            bodies.retain(|(_, n)| n.is_none());
        }
        if bodies.len() > 1 {
            bodies.retain(|(b, _)| b.path == here || c_header(&b.path));
        }
        match <[_; 1]>::try_from(bodies) {
            Ok([(class, _)]) => Some(class),
            Err(_) => None,
        }
    }

    fn cpp_member(
        &mut self,
        here: &Path,
        class: &Class,
        word: &str,
        depth: usize,
    ) -> Option<(Class, Vec<usize>)> {
        let ats = search::cpp_body_members(&class.code, class.open, word);
        if !ats.is_empty() {
            return Some((class.clone(), ats));
        }
        if depth == 4 {
            return None;
        }
        for base in search::cpp_bases(&class.code, class.open) {
            if base[0] == "std" {
                return None;
            }
            let base = self.cpp_class(here, &base, &class.code, None)?;
            if let Some(found) = self.cpp_member(here, &base, word, depth + 1) {
                return Some(found);
            }
        }
        None
    }

    fn cpp_candidates(
        &self,
        here: &Path,
        owner: &Class,
        ats: &[usize],
        word: &str,
        links: &[String],
    ) -> Vec<Candidate> {
        let reason = Reason::Receiver(links.join(" \u{2192} "));
        let mut found: Vec<Candidate> = ats
            .iter()
            .map(|&at| {
                let (line, col) = search::c_place(&owner.code, at);
                let text = owner
                    .text
                    .lines()
                    .nth(line - 1)
                    .unwrap_or_default()
                    .to_owned();
                Candidate {
                    hit: Hit {
                        path: owner.path.clone(),
                        line,
                        col,
                        text,
                        deleted: None,
                    },
                    reason: reason.clone(),
                }
            })
            .collect();
        let scopes: Vec<Vec<String>> = (found.iter())
            .filter_map(|c| search::c_member_class(&owner.text, c.hit.line, word))
            .collect();
        if scopes.is_empty() {
            return found;
        }
        let defs = self.project_definitions(Kind::C, here, word, &method_pattern(word));
        found.extend(
            (defs.into_iter())
                .filter(|h| {
                    scopes
                        .iter()
                        .any(|s| search::c_defines_member(&h.text, s, word))
                })
                .filter(|h| {
                    !found
                        .iter()
                        .any(|c| c.hit.path == h.path && c.hit.line == h.line)
                })
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect::<Vec<_>>(),
        );
        found
    }
}
