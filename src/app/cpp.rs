use super::c::{c_header, method_pattern};
use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, search};

pub(super) enum CppAnswer {
    Found(Vec<Candidate>),
    Outside,
    ByName(Option<String>),
}

type HeadType = (Vec<String>, String, Vec<String>, Option<usize>);

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
        let Some((mut ty, mut scope, mut ctx, mut decl)) =
            self.cpp_head_type(here, text, &head, word)
        else {
            return broke(&head);
        };
        let mut around = search::c_method_class(text, self.line + 1, word);
        let mut links = Vec::new();
        let mut prev = head.clone();
        let last = fields.len();
        for (i, name) in fields.iter().map(String::as_str).chain([word]).enumerate() {
            if ty[0] == "std" && i < last {
                return broke(&prev);
            }
            links.push(format!("{prev}: {}", ty[ty.len() - 1]));
            if ty[0] == "std" {
                let written =
                    decl.and_then(|at| search::cpp_written(&scope, at, &ty[ty.len() - 1]));
                let found = match written {
                    Some(w) if w.pointer == before.ends_with("->") => {
                        self.cpp_std_member(here, &ty, word, w.first_arg.as_deref())
                    }
                    _ => None,
                };
                return match found {
                    Some((owner, ats)) => self.cpp_found(here, text, &owner, &ats, word, &links),
                    None => CppAnswer::Outside,
                };
            }
            let Some(class) = self.cpp_class(here, &ty, &scope, around.as_deref(), &ctx) else {
                return broke(&prev);
            };
            let Some((owner, ats)) = self.cpp_member(here, &class, name, 0) else {
                return broke(&prev);
            };
            if i == last {
                return self.cpp_found(here, text, &owner, &ats, word, &links);
            }
            let Some((next, at)) = (ats.iter())
                .find_map(|&at| Some((search::cpp_type_at(&owner.code, at, name.len())?, at)))
            else {
                return broke(name);
            };
            ctx = search::cpp_scope_path(&owner.code, at);
            (ty, scope, around, decl) = (next, owner.code, Some(owner.name), Some(at));
            name.clone_into(&mut prev);
        }
        CppAnswer::ByName(None)
    }

    fn cpp_found(
        &mut self,
        here: &Path,
        text: &str,
        owner: &Class,
        ats: &[usize],
        word: &str,
        links: &[String],
    ) -> CppAnswer {
        let found = self.cpp_candidates(here, owner, ats, word, links);
        CppAnswer::Found(match self.cpp_call_args(here, text) {
            Some(args) => self.cpp_viable_only(word, &args, found, |c| &c.hit),
            None => found,
        })
    }

    fn cpp_std_member(
        &mut self,
        here: &Path,
        ty: &[String],
        word: &str,
        arg: Option<&str>,
    ) -> Option<(Class, Vec<usize>)> {
        let [_, name] = ty else {
            return None;
        };
        let mut files: Vec<PathBuf> = (self.c_reached(here).into_iter())
            .filter(|f| f.is_absolute() && f.components().any(|c| c.as_os_str() == "c++"))
            .collect();
        files.sort();
        if let Some(class) = self.cpp_std_class(&files, name) {
            return self.cpp_std_walk(&files, class, word, arg, 0);
        }
        let pattern = format!(r"^\s*using\s+{}\s*=\s*\w+\s*<", regex::escape(name));
        let mut real: Vec<String> = (self.external_grep(Kind::C, &files, &pattern).iter())
            .filter_map(|h| {
                Some(
                    h.text
                        .split_once('=')?
                        .1
                        .split('<')
                        .next()?
                        .trim()
                        .to_owned(),
                )
            })
            .collect();
        real.sort();
        real.dedup();
        let [real] = real.as_slice() else {
            return None;
        };
        let class = self.cpp_std_class(&files, real)?;
        self.cpp_std_walk(&files, class, word, None, 0)
    }

    fn cpp_std_class(
        &self,
        files: &[PathBuf],
        name: &str,
    ) -> Option<(Class, Vec<(Class, String)>)> {
        let w = regex::escape(name);
        let pattern = format!(r"^\s*(?:struct|class)\s+(?:[A-Z_][A-Z0-9_]*\s+)*{w}\b");
        let (mut primary, mut special) = (Vec::new(), Vec::new());
        for h in self.external_grep(Kind::C, files, &pattern) {
            let Some(text) = self.text_of(&h.path) else {
                continue;
            };
            let code = search::c_code(&text);
            let Some((open, arg)) = search::cpp_std_class_body(&code, h.line1, name) else {
                continue;
            };
            let class = Class {
                path: h.path,
                text,
                code,
                open,
                name: name.to_owned(),
            };
            match arg {
                Some(arg) => special.push((class, arg)),
                None => primary.push(class),
            }
        }
        let [class] = <[_; 1]>::try_from(primary).ok()?;
        Some((class, special))
    }

    fn cpp_std_walk(
        &self,
        files: &[PathBuf],
        (class, special): (Class, Vec<(Class, String)>),
        word: &str,
        arg: Option<&str>,
        depth: usize,
    ) -> Option<(Class, Vec<usize>)> {
        let other = |a: &str| !a.starts_with('_') && arg.is_some_and(|r| r != a);
        if (special.iter())
            .any(|(s, a)| !other(a) && !search::cpp_body_members(&s.code, s.open, word).is_empty())
        {
            return None;
        }
        let ats = search::cpp_body_members(&class.code, class.open, word);
        if !ats.is_empty() {
            return Some((class, ats));
        }
        if depth == 4 {
            return None;
        }
        for base in search::cpp_bases(&class.code, class.open) {
            let (["std", name] | [name]) = base.iter().map(String::as_str).collect::<Vec<_>>()[..]
            else {
                return None;
            };
            let base = self.cpp_std_class(files, name)?;
            if let Some(found) = self.cpp_std_walk(files, base, word, None, depth + 1) {
                return Some(found);
            }
        }
        None
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
    ) -> Option<HeadType> {
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
                None,
            ));
        }
        match search::cpp_value_type(text, self.line + 1, head) {
            Ok(Some((t, at))) => return Some((t, code, here_ctx, Some(at))),
            Ok(None) => {}
            Err(()) => return None,
        }
        let class = search::c_method_class(text, self.line + 1, word)?;
        let class = self.cpp_class(here, &[class], &code, None, &here_ctx)?;
        let (owner, ats) = self.cpp_member(here, &class, head, 0)?;
        let (t, at) = (ats.iter())
            .find_map(|&at| Some((search::cpp_type_at(&owner.code, at, head.len())?, at)))?;
        let ctx = search::cpp_scope_path(&owner.code, at);
        Some((t, owner.code, ctx, Some(at)))
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
        let Ok(Some((ty, _))) = search::cpp_value_type(text, self.line + 1, name) else {
            return None;
        };
        let name = ty.last()?;
        let p = search::def_patterns(Kind::C, name);
        let aliases = self.project_definitions(Kind::C, here, name, &format!("{}|{}", p[4], p[6]));
        let aliases = self.c_reached_only(here, name, aliases);
        if aliases.iter().any(|h| h.path == here || c_header(&h.path)) {
            return None;
        }
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

    pub(super) fn cpp_free_call(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        start: usize,
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let on = |h: &Hit| h.path == here && h.line1 == self.line + 1;
        if hits.len() < 2 || self.c_source() || self.objc_file() || hits.iter().any(on) {
            return hits;
        }
        let Some((visible, args)) = self.cpp_visible(here, text, word, start, &hits) else {
            return hits;
        };
        let mut keep = visible.into_iter();
        let hits: Vec<Hit> = hits
            .into_iter()
            .filter(|_| keep.next().unwrap_or(true))
            .collect();
        self.cpp_viable_only(word, &args, hits, |h| h)
    }

    fn cpp_visible(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        start: usize,
        hits: &[Hit],
    ) -> Option<(Vec<bool>, Vec<search::CppArg>)> {
        let code = search::c_code(text);
        let pos = offset(text, self.line, start);
        let ctx = search::cpp_call_namespaces(&code, pos)?;
        let mut usings = search::cpp_usings(&code, word, pos).ok()?;
        for f in self.c_reached(here) {
            if f.is_absolute() || f == here {
                continue;
            }
            let Some(t) = self.text_of(&f) else {
                continue;
            };
            let c = search::c_code(&t);
            usings.extend(search::cpp_usings(&c, word, c.len()).ok()?);
        }
        let spaces = (hits.iter())
            .map(|h| search::cpp_function_namespace(&self.text_of(&h.path)?, h.line1, word).ok())
            .collect::<Option<Vec<_>>>()?;
        let args = self.cpp_call_args(here, text)?;
        let mut adl = Vec::new();
        for arg in &args {
            match arg {
                search::CppArg::Class(ns) => adl.push(ns.clone()),
                search::CppArg::NonZeroInt | search::CppArg::Str | search::CppArg::Literal => {}
                search::CppArg::Name(_) | search::CppArg::Other => return None,
            }
        }
        let ordinary = (0..=ctx.len())
            .rev()
            .map(|k| {
                (spaces.iter())
                    .map(|s| {
                        s.as_ref().is_some_and(|s| {
                            s.as_slice() == &ctx[..k] || (k == 0 && usings.contains(s))
                        })
                    })
                    .collect::<Vec<bool>>()
            })
            .find(|v| v.contains(&true))
            .unwrap_or_else(|| vec![false; hits.len()]);
        let visible: Vec<bool> = (spaces.iter().zip(ordinary))
            .map(|(s, o)| o || s.as_ref().is_some_and(|s| adl.contains(s)))
            .collect();
        visible.contains(&true).then_some((visible, args))
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
