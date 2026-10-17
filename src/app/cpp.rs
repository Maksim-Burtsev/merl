use super::c::{c_header, method_pattern};
use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, search};

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
        let Some(search::CReceiver { head, fields, .. }) = search::cpp_receiver(before) else {
            return CppAnswer::ByName(None);
        };
        let broke = |at: &str| CppAnswer::ByName((!fields.is_empty()).then(|| at.to_owned()));
        if let Some(seventh) = fields.get(5) {
            return broke(seventh);
        }
        let Some((mut ty, mut scope, mut ctx)) = self.cpp_head_type(here, text, &head, word) else {
            return broke(&head);
        };
        let mut around = search::c_method_class(text, self.line + 1, word);
        let mut links = Vec::new();
        let mut prev = head.clone();
        let last = fields.len();
        for (i, name) in fields.iter().map(String::as_str).chain([word]).enumerate() {
            if ty[0] == "std" {
                return if i == last {
                    CppAnswer::Outside
                } else {
                    broke(&prev)
                };
            }
            links.push(format!("{prev}: {}", ty[ty.len() - 1]));
            let Some(class) = self.cpp_class(here, &ty, &scope, around.as_deref(), &ctx) else {
                return broke(&prev);
            };
            let Some((owner, ats)) = self.cpp_member(here, &class, name, 0) else {
                return broke(&prev);
            };
            if i == last {
                let found = self.cpp_candidates(here, &owner, &ats, word, &links);
                return CppAnswer::Found(match self.cpp_call_args(here, text) {
                    Some(args) => self.cpp_viable_only(word, &args, found, |c| &c.hit),
                    None => found,
                });
            }
            let Some((next, at)) = (ats.iter())
                .find_map(|&at| Some((search::cpp_type_at(&owner.code, at, name.len())?, at)))
            else {
                return broke(name);
            };
            ctx = search::cpp_scope_path(&owner.code, at);
            (ty, scope, around) = (next, owner.code, Some(owner.name));
            name.clone_into(&mut prev);
        }
        CppAnswer::ByName(None)
    }

    pub(super) fn cpp_class_scope(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        class: &str,
    ) -> Option<Vec<Candidate>> {
        if word == class || self.c_source() || self.objc_file() {
            return None;
        }
        match self.cpp_typed(here, text, "this->", word) {
            CppAnswer::Found(found) => Some(found),
            CppAnswer::Outside | CppAnswer::ByName(_) => None,
        }
    }

    fn cpp_head_type(
        &mut self,
        here: &Path,
        text: &str,
        head: &str,
        word: &str,
    ) -> Option<(Vec<String>, String, Vec<String>)> {
        let code = search::c_code(text);
        let pos = (text.split('\n').take(self.line))
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + self.col;
        let here_ctx = search::cpp_scope_path(&code, pos.min(code.len()));
        if head == "this" {
            return Some((
                vec![search::c_method_class(text, self.line + 1, word)?],
                code,
                here_ctx,
            ));
        }
        match search::cpp_value_type(text, self.line + 1, head) {
            Ok(Some(t)) => return Some((t, code, here_ctx)),
            Ok(None) => {}
            Err(()) => return None,
        }
        let class = search::c_method_class(text, self.line + 1, word)?;
        let class = self.cpp_class(here, &[class], &code, None, &here_ctx)?;
        let (owner, ats) = self.cpp_member(here, &class, head, 0)?;
        let (t, at) = (ats.iter())
            .find_map(|&at| Some((search::cpp_type_at(&owner.code, at, head.len())?, at)))?;
        let ctx = search::cpp_scope_path(&owner.code, at);
        Some((t, owner.code, ctx))
    }

    fn cpp_class(
        &mut self,
        here: &Path,
        ty: &[String],
        scope: &str,
        around: Option<&str>,
        ctx: &[String],
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
            let Some(open) = search::cpp_class_body(&code, h.line1, name) else {
                continue;
            };
            if bodies
                .iter()
                .any(|(b, _)| b.path == h.path && b.open == open)
            {
                continue;
            }
            let lines: Vec<&str> = text.lines().collect();
            let nested = search::c_class_around(&lines, h.line1);
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
        if bodies.len() > 1 {
            let paths: Vec<Vec<String>> = (bodies.iter())
                .map(|(b, _)| {
                    [search::cpp_scope_path(&b.code, b.open), vec![name.clone()]].concat()
                })
                .collect();
            let mine = cpp_namesake(&paths, ty, scope, ctx);
            if let Some(mine) = mine.filter(|m| m.iter().filter(|&&k| k).count() == 1) {
                let mut keep = mine.into_iter();
                bodies.retain(|_| keep.next().unwrap_or(false));
            }
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
            let ctx = search::cpp_scope_path(&class.code, class.open);
            let base = self.cpp_class(here, &base, &class.code, None, &ctx)?;
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
                let search::CPlace {
                    line1: line,
                    byte_col: col,
                } = search::c_place(&owner.code, at);
                let text = owner
                    .text
                    .lines()
                    .nth(line - 1)
                    .unwrap_or_default()
                    .to_owned();
                Candidate {
                    hit: Hit {
                        path: owner.path.clone(),
                        line1: line,
                        byte_col: Some(col),
                        text,
                        deleted: None,
                    },
                    reason: reason.clone(),
                }
            })
            .collect();
        let scopes: Vec<Vec<String>> = (found.iter())
            .filter_map(|c| search::c_member_class(&owner.text, c.hit.line1, word))
            .collect();
        if scopes.is_empty() {
            return found;
        }
        let defs = self.project_definitions(Kind::C, here, word, &method_pattern(word));
        #[expect(
            clippy::needless_collect,
            reason = "the Vec ends the borrow of `found` the filter reads before `extend` takes it mutably"
        )]
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
                        .any(|c| c.hit.path == h.path && c.hit.line1 == h.line1)
                })
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect::<Vec<_>>(),
        );
        found
    }

    fn cpp_call_args(&mut self, here: &Path, text: &str) -> Option<Vec<search::CppArg>> {
        let pos = offset(text, self.line, self.col);
        let rest = text.get(pos..)?;
        let end = pos + rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
        let after = text.get(end..)?;
        let open = end + after.len() - after.trim_start().len();
        let args = search::cpp_call_args(text, open)?;
        let code = search::c_code(text);
        let ctx = search::cpp_scope_path(&code, pos.min(code.len()));
        Some(
            args.into_iter()
                .map(|arg| match arg {
                    search::CppArg::Name(name) => self
                        .cpp_arg_class(here, text, &code, &ctx, &name)
                        .map_or(search::CppArg::Other, search::CppArg::Class),
                    arg => arg,
                })
                .collect(),
        )
    }

    fn cpp_arg_class(
        &mut self,
        here: &Path,
        text: &str,
        code: &str,
        ctx: &[String],
        name: &str,
    ) -> Option<Vec<String>> {
        let Ok(Some(ty)) = search::cpp_value_type(text, self.line + 1, name) else {
            return None;
        };
        let class = self.cpp_class(here, &ty, code, None, ctx)?;
        search::cpp_plain_class_namespace(&class.code, class.open)
    }

    fn cpp_viable_only<T>(
        &self,
        word: &str,
        args: &[search::CppArg],
        found: Vec<T>,
        hit: fn(&T) -> &Hit,
    ) -> Vec<T> {
        let keep: Vec<bool> = (found.iter().map(hit))
            .map(|h| {
                (self.text_of(&h.path))
                    .and_then(|t| search::cpp_parameters(&t, h.line1, word))
                    .is_none_or(|p| !search::cpp_not_viable(&p, args))
            })
            .collect();
        if !keep.contains(&true) {
            return found;
        }
        let mut keep = keep.into_iter();
        found
            .into_iter()
            .filter(|_| keep.next().unwrap_or(true))
            .collect()
    }
}

fn cpp_namesake(
    paths: &[Vec<String>],
    ty: &[String],
    scope: &str,
    ctx: &[String],
) -> Option<Vec<bool>> {
    let name = ty.last()?;
    if let [first, rest @ ..] = ty
        && !rest.is_empty()
    {
        let written = (!paths.iter().any(|p| p.ends_with(ty)))
            .then(|| search::cpp_namespace_alias(scope, first))
            .flatten()
            .map_or_else(|| ty.to_vec(), |real| [real, rest.to_vec()].concat());
        return Some(paths.iter().map(|p| p.ends_with(&written)).collect());
    }
    if search::cpp_using_declares(scope, name) {
        return None;
    }
    (0..=ctx.len()).rev().find_map(|k| {
        let want = [&ctx[..k], std::slice::from_ref(name)].concat();
        let mine: Vec<bool> = paths.iter().map(|p| *p == want).collect();
        mine.contains(&true).then_some(mine)
    })
}

fn offset(text: &str, line: usize, col: usize) -> usize {
    (text.split('\n').take(line))
        .map(|l| l.len() + 1)
        .sum::<usize>()
        + col
}
