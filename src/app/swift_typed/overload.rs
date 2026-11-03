use super::{App, Candidate, Kind, Path, WrittenVia, generic, search};

enum Typed {
    Literal(&'static str),
    Named(String),
}

#[derive(PartialEq, Eq)]
enum Fit {
    Takes,
    Not,
    Unknown,
}

struct Nominal {
    value: bool,
    literals: Option<&'static [&'static str]>,
}

const SDK_TYPES: [(&str, bool, &[&str]); 10] = [
    ("String", true, &["String"]),
    ("Int", true, &["Int"]),
    ("Double", true, &["Int", "Double"]),
    ("Bool", true, &["Bool"]),
    ("Data", true, &[]),
    ("URL", true, &[]),
    ("Date", true, &[]),
    ("UUID", true, &[]),
    ("URLRequest", true, &[]),
    ("InputStream", false, &[]),
];

fn plain(written: &str) -> Option<&str> {
    let t = written.trim();
    let t = t.strip_suffix(['?', '!']).unwrap_or(t);
    let mut chars = t.chars();
    (chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_'))
    .then_some(t)
}

fn literal(e: &str) -> Option<&'static str> {
    static INT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^-?[0-9][0-9_]*$").expect("the regex is a literal")
    });
    static FLOAT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^-?[0-9][0-9_]*\.[0-9][0-9_]*$").expect("the regex is a literal")
    });
    match e {
        "true" | "false" => Some("Bool"),
        _ if INT.is_match(e) => Some("Int"),
        _ if FLOAT.is_match(e) => Some("Double"),
        _ => e
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .filter(|s| {
                !s.replace("\\\\", "")
                    .replace("\\\"", "")
                    .contains(['"', '\n'])
            })
            .map(|_| "String"),
    }
}

fn sdk_call(e: &str) -> Option<&'static str> {
    let (callee, _) = e.split_once('(')?;
    let name = SDK_TYPES
        .iter()
        .map(|(n, _, _)| *n)
        .find(|n| *n == callee)?;
    matches!(search::swift_expr(e)?, search::SwiftExpr::Call(c) if c == callee).then_some(name)
}

impl App {
    pub(super) fn swift_scope_generics(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
    ) -> Vec<String> {
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

    pub(in crate::app) fn swift_by_argument_types(
        &self,
        call: &search::SwiftCall,
        params: &[Vec<search::SwiftParam>],
        found: &mut Vec<Candidate>,
    ) {
        if found.len() < 2
            || found.len() != params.len()
            || params.iter().flatten().any(|p| p.variadic)
        {
            return;
        }
        let Some(here) = self.rel_current() else {
            return;
        };
        let text = self.buf.lines.join("\n");
        let cut = self.truncated.get();
        let typed: Vec<(usize, Typed)> = call
            .args
            .iter()
            .enumerate()
            .filter_map(|(i, e)| Some((i, self.swift_arg_type(&here, &text, e)?)))
            .collect();
        let open: Option<Vec<(usize, bool)>> = (!typed.is_empty())
            .then(|| {
                found
                    .iter()
                    .zip(params)
                    .enumerate()
                    .map(|(k, (c, ps))| {
                        let slots = slots(&call.labels, ps)?;
                        let generics = self
                            .text_of(&c.hit.path)
                            .map(|t| self.swift_scope_generics(&here, &t, c.hit.line1))?;
                        let fits: Vec<Fit> = typed
                            .iter()
                            .map(|(i, t)| {
                                let p = ps.get(*slots.get(*i)?)?;
                                Some(self.swift_fit(&here, &p.ty, t, &generics))
                            })
                            .collect::<Option<_>>()?;
                        Some((k, fits))
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
            .map(|all| {
                all.into_iter()
                    .filter(|(_, f)| !f.contains(&Fit::Not))
                    .map(|(k, f)| (k, f.contains(&Fit::Takes)))
                    .collect()
            });
        self.truncated.set(cut);
        if let Some([(k, true)]) = open.as_deref() {
            let kept = found.swap_remove(*k);
            *found = vec![kept];
        }
    }

    pub(super) fn swift_call_fields(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        (call, fields): (String, Vec<String>),
        hops: usize,
    ) -> Option<WrittenVia> {
        let hops = hops.checked_sub(1)?;
        let call = self.swift_expr(here, file, text, line1, &call, hops)?;
        fields.iter().try_fold(call, |via, field| {
            let ty = self.swift_type(here, &via.written)?;
            ty.keyword.as_ref()?;
            self.swift_field(here, &ty, field, hops)
                .filter(|v| !generic::bound(&v.written))
        })
    }

    fn swift_arg_type(&self, here: &Path, text: &str, e: &str) -> Option<Typed> {
        if let Some(l) = literal(e) {
            return Some(Typed::Literal(l));
        }
        let line1 = self.line + 1;
        let given = || {
            let ident = e.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
                && e.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !ident {
                return None;
            }
            let bound = search::bindings(Kind::Swift, text, line1, e);
            let [b] = bound.as_slice() else {
                return None;
            };
            let line = text.lines().nth(b.line1.checked_sub(1)?)?;
            match search::swift_given(line, e)? {
                search::SwiftGiven::Value(v) => sdk_call(&v).map(str::to_owned),
                _ => None,
            }
        };
        let name = if let Some(n) = sdk_call(e).map(str::to_owned).or_else(given) {
            n
        } else {
            let written = self.swift_expr(here, here, text, line1, e, 3)?.written;
            plain(&written)?.to_owned()
        };
        self.swift_nominal(here, &name)?;
        (!self.swift_scope_generics(here, text, line1).contains(&name))
            .then_some(Typed::Named(name))
    }

    fn swift_fit(&self, here: &Path, written: &str, arg: &Typed, generics: &[String]) -> Fit {
        let Some(p) = plain(written).filter(|p| !generics.iter().any(|g| g == p)) else {
            return Fit::Unknown;
        };
        match arg {
            Typed::Named(n) if p == n => Fit::Takes,
            Typed::Literal(l) if p == *l => Fit::Takes,
            Typed::Named(n) => match self.swift_nominal(here, p) {
                Some(Nominal { value: true, .. }) => Fit::Not,
                Some(_) if self.swift_nominal(here, n).is_some_and(|t| t.value) => Fit::Not,
                _ => Fit::Unknown,
            },
            Typed::Literal(l) => match self.swift_nominal(here, p) {
                Some(Nominal {
                    literals: Some(ls), ..
                }) if !ls.contains(l) => Fit::Not,
                _ => Fit::Unknown,
            },
        }
    }

    fn swift_nominal(&self, here: &Path, name: &str) -> Option<Nominal> {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        let hits: Vec<_> = self
            .project_definitions(Kind::Swift, here, name, &pattern)
            .into_iter()
            .filter(|h| self.in_code(Kind::Swift, h))
            .collect();
        let headers: Vec<_> = hits
            .iter()
            .filter(|h| search::swift_alias(&h.text, name).is_none())
            .filter_map(|h| search::swift_type_header(&h.text))
            .filter(|t| t.name == name)
            .collect();
        let aliased = hits
            .iter()
            .any(|h| search::swift_alias(&h.text, name).is_some());
        if aliased || headers.iter().any(|t| t.keyword == "protocol") {
            return None;
        }
        let declared: Vec<_> = headers
            .iter()
            .filter(|t| t.keyword != "extension")
            .collect();
        match declared.as_slice() {
            [t] => matches!(t.keyword.as_str(), "struct" | "enum" | "class" | "actor").then_some(
                Nominal {
                    value: matches!(t.keyword.as_str(), "struct" | "enum"),
                    literals: None,
                },
            ),
            [] => {
                let (_, value, literals) = SDK_TYPES.iter().find(|(n, _, _)| *n == name)?;
                Some(Nominal {
                    value: *value,
                    literals: headers.is_empty().then_some(*literals),
                })
            }
            _ => None,
        }
    }
}

fn slots(labels: &[String], params: &[search::SwiftParam]) -> Option<Vec<usize>> {
    let mut at = 0;
    labels
        .iter()
        .map(|l| {
            while params.get(at)?.label != *l {
                at += 1;
            }
            at += 1;
            Some(at - 1)
        })
        .collect()
}
