//! `d` in C and C++ on the members a line pattern cannot tell from a function, a macro or a
//! global of the same name: `x->word`, `x.word`, a constructor's initializer list (#359), a bare
//! member inside a method and a value that is no type (#378).

use super::*;

impl App {
    /// What `d` on `word` at `range` of the cursor's line answers before any other lookup in a C
    /// or C++ file; `None` leaves the word to the rest of [`App::goto_definition`].
    pub(super) fn c_early(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> Option<(Vec<Candidate>, Option<String>)> {
        static TRAILING: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?:\)|\b(?:const|noexcept|mutable|override))\s+->\s*$").unwrap()
        });
        let line = self.line_str();
        let (before, after) = (
            line[..range.start].trim_end(),
            line[range.end..].trim_start(),
        );
        let called = after.starts_with('(');
        // `auto f() -> T` names a type, and an `#include` a file.
        let member = (before.ends_with("->") && !TRAILING.is_match(before))
            || (before.ends_with('.') && !before.ends_with(".."));
        if member
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && !line.trim_start().starts_with('#')
        {
            // A receiver whose struct is proven narrows the member to its field (#386).
            let broke = match self.c_receiver_field(here, text, before, word) {
                Ok(Some(found)) => return Some((vec![found], None)),
                Ok(None) => None,
                Err(at) => Some(at),
            };
            return Some((self.c_members(here, word, called), broke));
        }
        let class = after
            .starts_with(['(', '{'])
            .then(|| search::c_initialized_member(text, self.line + 1, range.start))
            .flatten()?;
        let found = self.c_class_fields(here, word, &class, true);
        (!found.is_empty()).then_some((found, None))
    }

    /// `x->word` in C (#386): the field `word` of the struct the receiver in front of it is
    /// declared as, followed through each link of `a->b.c->word`, with the links that prove it
    /// (`c: client → bstate: blockingState`). `Ok(None)` leaves the word to the search by name:
    /// no receiver to read, a C++ file, a proven struct without the field. `Err` names where a
    /// chain of two names or more broke, which the search by name then says.
    fn c_receiver_field(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> Result<Option<Candidate>, String> {
        let c = here.extension().is_some_and(|e| e == "c" || e == "h");
        let Some((head, called, fields)) = c.then(|| search::c_receiver(before)).flatten() else {
            return Ok(None);
        };
        let broke = |at: &str| match fields.is_empty() {
            true => Ok(None),
            false => Err(at.to_owned()),
        };
        let start = match called {
            true => self
                .c_return_type(here, &head)
                .map(|t| (here.to_path_buf(), search::CType::Name(t))),
            false => self.c_head_type(here, text, &head),
        };
        let Some(mut ty) = start else {
            return broke(&head);
        };
        let mut links = Vec::new();
        let mut prev = head.clone();
        for (i, name) in fields.iter().chain([&word.to_owned()]).enumerate() {
            let Some(body) = self.c_body(here, &ty) else {
                return broke(&prev);
            };
            let link = match &ty.1 {
                search::CType::Name(t) if i == 0 && called => format!("{prev}(): {t}"),
                search::CType::Name(t) => format!("{prev}: {t}"),
                search::CType::Body(open) => match search::c_body_name(&body.code, *open) {
                    t if t.is_empty() => prev.clone(),
                    t => format!("{prev}: {t}"),
                },
            };
            links.push(link);
            let Some(at) = search::c_body_field(&body.code, body.open, name) else {
                return match i == fields.len() {
                    true => Ok(None),
                    false => broke(name),
                };
            };
            if i == fields.len() {
                let (line, col) = search::c_place(&body.code, at);
                let hit = Hit {
                    path: body.path,
                    line,
                    col,
                    text: body
                        .text
                        .lines()
                        .nth(line - 1)
                        .unwrap_or_default()
                        .to_owned(),
                    deleted: None,
                };
                let reason = Reason::Receiver(links.join(" \u{2192} "));
                return Ok(Some(Candidate { hit, reason }));
            }
            let Some(next) = search::c_decl_type(&body.code, at) else {
                return broke(name);
            };
            ty = (body.path, next);
            prev = name.clone();
        }
        Ok(None)
    }

    /// The type the C value `name` read on the cursor's line is declared with: its parameter or
    /// local, else its declaration at file scope in the file on screen, else a global declared
    /// once in a header of the project (`extern struct redisServer server;`).
    fn c_head_type(&self, here: &Path, text: &str, name: &str) -> Option<(PathBuf, search::CType)> {
        match search::c_value_type(text, self.line + 1, name) {
            Ok(Some(t)) => return Some((here.to_path_buf(), t)),
            Ok(None) => {}
            Err(()) => return None,
        }
        let p = search::def_patterns(Kind::C, name);
        let hits = self.project_definitions(Kind::C, here, name, &p[8]);
        let [h] = hits
            .iter()
            .filter(|h| c_header(&h.path))
            .collect::<Vec<_>>()[..]
        else {
            return None;
        };
        let code = search::c_code(&self.text_of(&h.path)?);
        let t = search::c_words_on_line(&code, h.line, name, None)
            .into_iter()
            .find_map(|at| search::c_decl_type(&code, at))?;
        matches!(t, search::CType::Name(_)).then(|| (h.path.clone(), t))
    }

    /// The type the C function `name` returns, when each of its prototypes and its definition
    /// declare the same; `None` for a macro.
    fn c_return_type(&self, here: &Path, name: &str) -> Option<String> {
        let p = search::def_patterns(Kind::C, name);
        let hits = self.project_definitions(Kind::C, here, name, &format!("{}|{}", p[0], p[7]));
        let mut found: Option<String> = None;
        for h in &hits {
            if h.text.trim_start().starts_with('#') {
                return None;
            }
            let code = search::c_code(&self.text_of(&h.path)?);
            let t = search::c_words_on_line(&code, h.line, name, Some('('))
                .into_iter()
                .find_map(|at| search::c_decl_type(&code, at));
            let Some(search::CType::Name(t)) = t else {
                return None;
            };
            if found.as_ref().is_some_and(|f| *f != t) {
                return None;
            }
            found = Some(t);
        }
        found
    }

    /// The body of the struct or union `ty` is, read from its file: a `} name;` body as it
    /// stands, a type by name through [`App::c_bodies`]. A type with more than one body is the
    /// one in the file on screen or in a header, and one in another source file only when it is
    /// the only body (an opaque struct); `None` when that leaves none or more than one.
    fn c_body(&self, here: &Path, ty: &(PathBuf, search::CType)) -> Option<CBody> {
        let read = |path: &Path, open: usize| {
            let text = self.text_of(path)?;
            let code = search::c_code(&text);
            Some(CBody {
                path: path.to_path_buf(),
                text,
                code,
                open,
            })
        };
        let name = match &ty.1 {
            search::CType::Body(open) => return read(&ty.0, *open),
            search::CType::Name(name) => name,
        };
        let mut bodies = Vec::new();
        self.c_bodies(here, name, 0, &mut bodies);
        if bodies.len() > 1 {
            bodies.retain(|(p, _)| p == here || c_header(p));
        }
        match bodies.as_slice() {
            [(path, open)] => read(path, *open),
            _ => None,
        }
    }

    /// Into `bodies`, each struct or union body the type `name` has in the project, as a path and
    /// the byte of its `{`: `struct name {`, `typedef struct … { … } name;`, and through a
    /// `typedef struct TAG name;` the bodies of `TAG`, a few typedefs deep.
    fn c_bodies(&self, here: &Path, name: &str, depth: usize, bodies: &mut Vec<(PathBuf, usize)>) {
        let p = search::def_patterns(Kind::C, name);
        let pattern = [&p[2], &p[4], &p[5]].map(String::as_str).join("|");
        let mut aliases = Vec::new();
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for h in self.project_definitions(Kind::C, here, name, &pattern) {
            if !files.iter().any(|(p, _)| *p == h.path) {
                let Some(text) = self.text_of(&h.path) else {
                    continue;
                };
                files.push((h.path.clone(), search::c_code(&text)));
            }
            let code = &files
                .iter()
                .find(|(p, _)| *p == h.path)
                .expect("pushed above")
                .1;
            let def = search::c_words_on_line(code, h.line, name, None)
                .into_iter()
                .find_map(|at| search::c_type_def(code, at));
            match def {
                Some(search::CType::Body(open)) if !bodies.contains(&(h.path.clone(), open)) => {
                    bodies.push((h.path, open))
                }
                Some(search::CType::Name(alias)) if !aliases.contains(&alias) => {
                    aliases.push(alias)
                }
                _ => {}
            }
        }
        if depth < 3 {
            for alias in aliases {
                self.c_bodies(here, &alias, depth + 1, bodies);
            }
        }
    }

    /// `x->word` or `x.word`, `called` with a `(` after it. A data member is a field of a struct,
    /// union or class body ([`search::c_field_rows`]); a called one is a method whose body opens
    /// on its line, an out-of-line `R Type::word(`, a member a class body declares with no body (#373),
    /// or a function-pointer field. Nothing else can
    /// follow `->` or `.`: no function, `#define`, type or global is offered. The files outside
    /// the project are searched the same way when the project has none: a system struct's field
    /// stays findable, and system headers' generic names do not crowd a project's own.
    pub(super) fn c_members(&mut self, here: &Path, word: &str, called: bool) -> Vec<Candidate> {
        let mut pattern = search::c_field_pattern(word);
        if called {
            pattern = format!(
                r"{pattern}|{}|{}",
                method_pattern(word),
                search::c_member_decl(Some(word))
            );
        }
        let hits = self.project_definitions(Kind::C, here, word, &pattern);
        let mut found = self.c_member_rows(word, hits, called);
        if found.is_empty() {
            let files = self.external_files(Kind::C);
            let hits = self.external_grep(Kind::C, &files, &pattern);
            found = self.c_member_rows(word, hits, called);
        }
        found
            .into_iter()
            .map(|(hit, _)| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    /// Of `hits` for `word`, the fields (a `called` word: only a function pointer) and, when
    /// `called`, the methods: each with the name of the type it is a field of, `""` for a method.
    fn c_member_rows(&self, word: &str, hits: Vec<Hit>, called: bool) -> Vec<(Hit, String)> {
        let w = regex::escape(word);
        let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
        let method = re(method_pattern(word));
        let declared = re(search::c_member_decl(Some(word)));
        let pointer = re(format!(r"\(\s*\*+\s*{w}\s*\)"));
        let mut rows = Vec::new();
        let mut files: Vec<(PathBuf, Vec<Hit>)> = Vec::new();
        for h in hits {
            match files.iter_mut().find(|(p, _)| *p == h.path) {
                Some((_, hs)) => hs.push(h),
                None => files.push((h.path.clone(), vec![h])),
            }
        }
        for (path, hits) in files {
            let Some(text) = self.text_of(&path) else {
                continue;
            };
            let lines: Vec<usize> = hits.iter().map(|h| h.line).collect();
            let fields = search::c_field_rows(&text, &lines, word);
            for h in hits {
                match fields.iter().find(|(l, _)| *l == h.line) {
                    Some((_, owner)) if !called || pointer.is_match(&h.text) => {
                        rows.push((h, owner.clone()))
                    }
                    None if called && method.is_match(&h.text) => rows.push((h, String::new())),
                    // A member declared with no body, a pure virtual among them (#373).
                    None if called
                        && declared.is_match(&h.text)
                        && search::c_member_class(&text, h.line, word).is_some() =>
                    {
                        rows.push((h, String::new()))
                    }
                    _ => {}
                }
            }
        }
        rows
    }

    /// `a_` in a C++ constructor's `X::X(…) : a_(x), b_{y} {` (#359), or a bare `a_` in a
    /// method of `X` (#378): the fields `a_` of `X`, and with `by_name` when `X` has none, the
    /// fields of the name in any type. Empty when no field has the name, as for a base class,
    /// which the lookup by name finds.
    pub(super) fn c_class_fields(
        &mut self,
        here: &Path,
        word: &str,
        class: &str,
        by_name: bool,
    ) -> Vec<Candidate> {
        let hits = self.project_definitions(Kind::C, here, word, &search::c_field_pattern(word));
        let rows = self.c_member_rows(word, hits, false);
        let own = rows.iter().any(|(_, owner)| owner == class);
        rows.into_iter()
            .filter(|(_, owner)| owner == class || (by_name && !own))
            .map(|(hit, _)| Candidate {
                hit,
                reason: match own {
                    true => Reason::Receiver(class.to_owned()),
                    false => Reason::ByName,
                },
            })
            .collect()
    }

    /// Whether `hit` declares `word` as a type: a struct, union, enum or class tag, a namespace,
    /// a `typedef` or a `using` alias. A word followed by `->` or `.` is a value, none of these
    /// (#378).
    pub(super) fn c_type_line(word: &str, hit: &Hit) -> bool {
        let p = search::def_patterns(Kind::C, word);
        Regex::new(&[&p[2], &p[4], &p[6]].map(String::as_str).join("|"))
            .is_ok_and(|re| re.is_match(&hit.text))
    }
}

/// A struct or union body of a C file: the byte of its `{` in the file's [`search::c_code`].
struct CBody {
    path: PathBuf,
    text: String,
    code: String,
    open: usize,
}

/// Whether `path` is a C or C++ header.
fn c_header(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| ["h", "hh", "hpp", "hxx"].iter().any(|h| e == *h))
}

/// A C++ method called `word`: indented with its body opening on the line, as in a class body,
/// or an out-of-line `R Type::word(` in column zero.
fn method_pattern(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"^\s+[^;(){{}}=]*\w[\s*&]+{w}\s*\([^;{{}}]*\)[^;{{}}=]*\{{|^\w[^;(){{}}=]*::{w}\s*\(")
}
