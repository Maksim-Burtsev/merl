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
    ) -> Option<Vec<Candidate>> {
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
            return Some(self.c_members(here, word, called));
        }
        let class = after
            .starts_with(['(', '{'])
            .then(|| search::c_initialized_member(text, self.line + 1, range.start))
            .flatten()?;
        let found = self.c_class_fields(here, word, &class, true);
        (!found.is_empty()).then_some(found)
    }

    /// `x->word` or `x.word`, `called` with a `(` after it. A data member is a field of a struct,
    /// union or class body ([`search::c_field_rows`]); a called one is a method whose body opens
    /// on its line, an out-of-line `R Type::word(`, or a function-pointer field. Nothing else can
    /// follow `->` or `.`: no function, `#define`, type or global is offered. The files outside
    /// the project are searched the same way when the project has none: a system struct's field
    /// stays findable, and system headers' generic names do not crowd a project's own.
    pub(super) fn c_members(&mut self, here: &Path, word: &str, called: bool) -> Vec<Candidate> {
        let mut pattern = search::c_field_pattern(word);
        if called {
            pattern = format!(r"{pattern}|{}", method_pattern(word));
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

/// A C++ method called `word`: indented with its body opening on the line, as in a class body,
/// or an out-of-line `R Type::word(` in column zero.
fn method_pattern(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"^\s+[^;(){{}}=]*\w[\s*&]+{w}\s*\([^;{{}}]*\)[^;{{}}=]*\{{|^\w[^;(){{}}=]*::{w}\s*\(")
}
