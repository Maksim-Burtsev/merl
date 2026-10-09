use super::*;

impl App {
    pub(super) fn enum_constant(
        &self,
        kind: Kind,
        here: &Path,
        owner: &str,
        word: &str,
    ) -> Vec<Candidate> {
        let owners = search::def_patterns(kind, owner).join("|");
        let declared = self.project_definitions(kind, here, owner, &owners);
        self.truncated.set(false);
        let [decl] = declared.as_slice() else {
            return Vec::new();
        };
        let o = regex::escape(owner);
        let is_enum = Regex::new(&format!(r"\benum\s+(?:class\s+)?{o}\b"))
            .is_ok_and(|re| re.is_match(&decl.text));
        let Some(text) = self.text_of(&decl.path).filter(|_| is_enum) else {
            return Vec::new();
        };
        let package = |t: &str| {
            let re = Regex::new(r"^\s*package\s+([\w.]+)").expect("a fixed pattern");
            t.lines()
                .find_map(|l| re.captures(l).map(|c| c[1].to_owned()))
                .unwrap_or_default()
        };
        let import = Regex::new(&format!(r"^\s*import\s+([\w.]+)\.({o}|\*)\s*;?\s*$"))
            .expect("an escaped name keeps the pattern valid");
        let home = package(&text);
        let imports: Vec<(String, bool)> = (self.buf.lines.iter())
            .filter_map(|l| import.captures(l))
            .map(|c| (c[1].to_owned(), &c[2] == "*"))
            .collect();
        let elsewhere = imports.iter().any(|(p, all)| !all && *p != home);
        let sees =
            package(&self.buf.lines.join("\n")) == home || imports.iter().any(|(p, _)| *p == home);
        if elsewhere || !sees {
            return Vec::new();
        }
        let lines: Vec<&str> = text.lines().collect();
        search::enum_constants(&text, decl.line1)
            .into_iter()
            .filter(|c| c.name == word)
            .map(|search::EnumConstant { line1: line, .. }| Candidate {
                hit: Hit {
                    deleted: None,
                    path: decl.path.clone(),
                    line1: line,
                    byte_col: None,
                    text: lines.get(line - 1).copied().unwrap_or_default().to_owned(),
                },
                reason: Reason::Path(owner.to_owned()),
            })
            .collect()
    }

    pub(super) fn jvm_imported(
        &self,
        text: &str,
        chain: &[String],
        word: &str,
        range: std::ops::Range<usize>,
        cursor_on_import: bool,
    ) -> Option<Vec<Candidate>> {
        let found = self.jvm_imported_all(text, chain, word, cursor_on_import)?;
        let here = self.rel_current()?;
        Some(self.jvm_fit_candidates(&here, word, range, chain, found))
    }

    pub(super) fn jenkins_step(&self, here: &Path, word: &str) -> Vec<Candidate> {
        static CALL: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:def|void)\s+call\s*\(").unwrap());
        if !search::groovy(here) {
            return Vec::new();
        }
        let file = format!("{word}.groovy");
        self.files
            .iter()
            .filter(|p| p.ends_with(Path::new("vars").join(&file)))
            .map(|p| {
                let text = self.text_of(p).unwrap_or_default();
                let (line, text) = text
                    .lines()
                    .enumerate()
                    .find(|(_, l)| CALL.is_match(l))
                    .or(text.lines().enumerate().next())
                    .map_or((1, String::new()), |(i, l)| (i + 1, l.to_owned()));
                Candidate {
                    hit: Hit {
                        deleted: None,
                        path: p.clone(),
                        line1: line,
                        byte_col: None,
                        text,
                    },
                    reason: Reason::ByName,
                }
            })
            .collect()
    }

    pub(super) fn jvm_seen(&mut self, here: &Path, hits: Vec<Hit>, behind: bool) -> Vec<Hit> {
        let mut all = hits.len();
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        let mut literals: HashMap<PathBuf, Vec<bool>> = HashMap::new();
        let mut seen_local = false;
        let kept: Vec<Hit> = hits
            .into_iter()
            .filter(|h| {
                if h.path != here && search::jvm_private(&h.text) {
                    return false;
                }
                if !h.text.starts_with([' ', '\t'])
                    || (search::groovy(&h.path) && search::gradle_global(&h.text))
                {
                    return true;
                }
                let text = texts
                    .entry(h.path.clone())
                    .or_insert_with(|| self.text_of(&h.path));
                match text
                    .as_deref()
                    .and_then(|t| search::jvm_local_block(t, h.line1))
                {
                    None => true,
                    Some(end) => {
                        let seen =
                            !behind && h.path == here && (h.line1..=end).contains(&(self.line + 1));
                        seen_local |= seen;
                        let literal = literals
                            .entry(h.path.clone())
                            .or_insert_with(|| {
                                search::literal_lines(Kind::Jvm, text.as_deref().unwrap_or(""))
                            })
                            .get(h.line1 - 1)
                            .copied()
                            .unwrap_or(false);
                        all -= usize::from(!seen && literal);
                        seen
                    }
                }
            })
            .collect();
        self.offer_only |= kept.len() == 1 && kept.len() < all && !seen_local;
        kept
    }

    pub(super) fn jvm_fit_candidates(
        &self,
        here: &Path,
        word: &str,
        range: std::ops::Range<usize>,
        chain: &[String],
        found: Vec<Candidate>,
    ) -> Vec<Candidate> {
        let hits: Vec<Hit> = found.iter().map(|c| c.hit.clone()).collect();
        let kept = self.jvm_use_fit(here, word, range, chain, hits);
        let fit: Vec<Candidate> = found
            .iter()
            .filter(|c| {
                kept.iter()
                    .any(|h| (&h.path, h.line1) == (&c.hit.path, c.hit.line1))
            })
            .cloned()
            .collect();
        match fit.is_empty() {
            true => found,
            false => fit,
        }
    }

    fn jvm_imported_all(
        &self,
        text: &str,
        chain: &[String],
        word: &str,
        cursor_on_import: bool,
    ) -> Option<Vec<Candidate>> {
        let imports = search::jvm_imports(text);
        let first = chain.first().map_or(word, String::as_str);
        let bound = imports
            .iter()
            .find(|i| i.bound_name == first)
            .map(|i| &i.path);
        let capital = word.starts_with(|c: char| c.is_ascii_uppercase());
        let wildcards: Vec<&Vec<String>> = imports
            .iter()
            .filter(|i| i.bound_name == "*")
            .map(|i| &i.path)
            .collect();
        let unbound = chain.is_empty() && (capital || !wildcards.is_empty());
        if !cursor_on_import && bound.is_none() && !unbound {
            return None;
        }
        let packages = self.jvm_packages()?;
        let longest_project_package_and_rest = |path: &[String]| {
            (1..path.len()).rev().find_map(|n| {
                let files = packages.get(&path[..n].join("."))?;
                Some((files, path[n..].to_vec()))
            })
        };
        if cursor_on_import {
            let mut path = chain.to_vec();
            path.push(word.to_owned());
            let Some((files, names)) = longest_project_package_and_rest(&path) else {
                return Some(Vec::new());
            };
            let found = self.jvm_declared(files, &names);
            return (!found.is_empty()).then_some(found);
        }
        if let Some(path) = bound {
            let Some((files, mut names)) = longest_project_package_and_rest(path) else {
                return Some(Vec::new());
            };
            if !chain.is_empty() {
                names.extend(chain[1..].iter().cloned());
                names.push(word.to_owned());
            }
            let found = self.jvm_declared(files, &names);
            return (!found.is_empty()).then_some(found);
        }
        let own = search::jvm_package(text);
        if capital && let Some(files) = own.as_ref().and_then(|p| packages.get(p)) {
            let found = self.jvm_declared(files, &[word.to_owned()]);
            if !found.is_empty() {
                let package = own.unwrap_or_default();
                return Some(
                    found
                        .into_iter()
                        .map(|c| Candidate {
                            reason: Reason::Path(package.clone()),
                            ..c
                        })
                        .collect(),
                );
            }
        }
        let mut found = Vec::new();
        for path in wildcards {
            let base = &path[..path.len() - 1];
            let names = match packages.get(&base.join(".")) {
                Some(files) if capital => Some((files, vec![word.to_owned()])),
                Some(_) => None,
                None => longest_project_package_and_rest(base).map(|(files, mut names)| {
                    names.push(word.to_owned());
                    (files, names)
                }),
            };
            if let Some((files, names)) = names {
                found.extend(self.jvm_declared(files, &names));
            }
        }
        (!found.is_empty()).then_some(found)
    }

    pub(super) fn jvm_members(
        &self,
        here: &Path,
        text: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let candidate = |path: &Path, lines: &[&str], line: usize, reason: Reason| Candidate {
            hit: Hit {
                deleted: None,
                path: path.to_path_buf(),
                line1: line,
                byte_col: None,
                text: lines[line - 1].to_owned(),
            },
            reason,
        };
        let types = search::jvm_enclosing_types(text, self.line + 1);
        let mut found: Vec<Candidate> = Vec::new();
        for &decl in &types {
            let reason = match search::jvm_type_name(lines[decl - 1]) {
                Some(name) => Reason::Path(name),
                None => Reason::Local,
            };
            found = search::jvm_members_of(text, decl, word)
                .into_iter()
                .map(|line| candidate(here, &lines, line, reason.clone()))
                .collect();
            if !found.is_empty() {
                break;
            }
        }
        let inner = types
            .iter()
            .find(|&&d| search::jvm_type_name(lines[d - 1]).is_some());
        if found.is_empty()
            && let Some(&decl) = inner
        {
            for base in search::jvm_bases(text, decl, search::scala(here)) {
                let pattern = search::def_patterns(Kind::Jvm, &base).join("|");
                let cut = self.truncated.get();
                let declared: Vec<Hit> = self
                    .project_definitions(Kind::Jvm, here, &base, &pattern)
                    .into_iter()
                    .filter(|h| search::jvm_type_name(&h.text).as_deref() == Some(base.as_str()))
                    .collect();
                self.truncated.set(cut);
                let [hit] = declared.as_slice() else {
                    continue;
                };
                let Some(t) = self.text_of(&hit.path) else {
                    continue;
                };
                let base_lines: Vec<&str> = t.lines().collect();
                found.extend(
                    search::jvm_members_of(&t, hit.line1, word)
                        .into_iter()
                        .map(|line| {
                            candidate(&hit.path, &base_lines, line, Reason::Path(base.clone()))
                        }),
                );
            }
        }
        let on_it = found
            .iter()
            .any(|c| c.hit.path == here && c.hit.line1 == self.line + 1);
        (!found.is_empty() && !on_it).then_some(found)
    }

    pub(super) fn jvm_packages(&self) -> Option<HashMap<String, Vec<PathBuf>>> {
        let jvm = |p: &Path| search::kind_of(p) == Some(Kind::Jvm);
        let hits = self
            .grep(r"^[ \t]*package\s+[\w.]*\w", false, false, jvm)
            .ok()?;
        if hits.len() >= search::MAX_HITS {
            return None;
        }
        let mut out: HashMap<String, Vec<PathBuf>> = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        for h in hits {
            if seen.insert(h.path.clone())
                && let Some(p) = search::jvm_package(&h.text)
            {
                out.entry(p).or_default().push(h.path);
            }
        }
        Some(out)
    }

    pub(super) fn jvm_declared(&self, files: &[PathBuf], names: &[String]) -> Vec<Candidate> {
        let Some(name) = names.last() else {
            return Vec::new();
        };
        let files: Vec<PathBuf> = files
            .iter()
            .filter(|f| {
                f.extension().is_none_or(|e| e != "java")
                    || f.file_stem() == Some(names[0].as_ref())
            })
            .cloned()
            .collect();
        if files.is_empty() {
            return Vec::new();
        }
        let pattern = search::def_patterns(Kind::Jvm, name).join("|");
        let hits = self.grep_in(&pattern, &files);
        let within = (names.len() > 1).then(|| names.join("."));
        let constructor = Some(format!("{}.{name}", names.join(".")));
        self.declaring(Kind::Jvm, name, hits)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .map(|t| search::qualified(Kind::Jvm, &t, h.line1, name))
                    .is_some_and(|q| q == within || (q.is_some() && q == constructor))
            })
            .map(|hit| Candidate {
                reason: Reason::Import(hit.path.display().to_string()),
                hit,
            })
            .collect()
    }
}

const KOTLIN_KEYWORDS: &[&str] = &[
    "in",
    "is",
    "as",
    "else",
    "return",
    "val",
    "var",
    "fun",
    "class",
    "object",
    "interface",
    "if",
    "when",
    "while",
    "for",
    "do",
    "try",
    "catch",
    "finally",
    "throw",
    "import",
    "package",
    "typealias",
    "by",
    "where",
    "get",
    "set",
    "init",
    "constructor",
    "private",
    "public",
    "protected",
    "internal",
    "override",
    "open",
    "abstract",
    "lateinit",
    "const",
    "data",
    "enum",
    "sealed",
    "inline",
    "suspend",
    "operator",
    "infix",
    "companion",
    "annotation",
    "inner",
    "value",
    "out",
    "reified",
    "vararg",
    "noinline",
    "crossinline",
    "external",
    "tailrec",
    "final",
    "expect",
    "actual",
];

fn arguments(lines: &[String], at: usize, open_byte: usize) -> Option<(usize, bool)> {
    let mut depth = 0usize;
    let (mut commas, mut any, mut angle) = (0, false, false);
    let mut quote: Option<&[u8]> = None;
    for (n, line) in lines.iter().enumerate().skip(at).take(60) {
        let from = if n == at { open_byte } else { 0 };
        let b = line.as_bytes();
        let mut i = from;
        while i < b.len() {
            let rest = &b[i..];
            if let Some(q) = quote {
                if rest.starts_with(b"\\") {
                    i += 2;
                    continue;
                }
                if rest.starts_with(q) {
                    i += q.len();
                    quote = None;
                    continue;
                }
                i += 1;
                continue;
            }
            let c = b[i];
            if rest.starts_with(b"\"\"\"") {
                quote = Some(b"\"\"\"");
                i += 3;
                continue;
            }
            match c {
                b'"' => quote = Some(b"\""),
                b'\'' => quote = Some(b"'"),
                b'/' if rest.starts_with(b"//") => break,
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((if any { commas + 1 } else { 0 }, angle));
                    }
                }
                b',' if depth == 1 => commas += 1,
                b'<' => angle = true,
                _ => {}
            }
            if depth >= 1 && !(depth == 1 && c == b'(') && !c.is_ascii_whitespace() {
                any = true;
            }
            i += 1;
        }
        if quote != Some(b"\"\"\"") {
            quote = None;
        }
    }
    None
}

static MAP_ARGUMENT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\(\s*(?:\[\s*(?:\w+\s*:|:\s*\])|\w+\s*:[^:])").unwrap()
});

impl App {
    pub(super) fn jvm_use_fit(
        &self,
        here: &Path,
        word: &str,
        range: std::ops::Range<usize>,
        chain: &[String],
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let line = self.line_str();
        let (before, after) = (&line[..range.start], &line[range.end..]);
        let java = here.extension().is_some_and(|e| e == "java");
        let groovy = search::groovy(here);
        if !java && !groovy {
            return match self.kotlin_infix(before, after, word) {
                true => hits
                    .into_iter()
                    .filter(|h| Regex::new(r"\binfix\b").is_ok_and(|re| re.is_match(&h.text)))
                    .collect(),
                false => hits,
            };
        }
        let w = regex::escape(word);
        let class_re = Regex::new(&format!(r"\b(?:class|record|enum|interface)\s+{w}\b"))
            .expect("an escaped name keeps the pattern valid");
        let call_re =
            Regex::new(&format!(r"\b{w}\s*\(")).expect("an escaped name keeps the pattern valid");
        let is_class = |h: &Hit| class_re.is_match(&h.text);
        let java_or_groovy =
            |h: &Hit| h.path.extension().is_some_and(|e| e == "java") || search::groovy(&h.path);
        let constructor = |h: &Hit| {
            java_or_groovy(h)
                && !is_class(h)
                && word.starts_with(char::is_uppercase)
                && call_re.is_match(&h.text)
        };
        let constructed = before
            .trim_end()
            .strip_suffix("new")
            .is_some_and(|b| !b.ends_with(is_word));
        let mut hits = hits;
        let has_class = hits.iter().any(is_class);
        let has_constructor = hits.iter().any(constructor);
        if constructed && has_constructor {
            hits.retain(|h| !is_class(h));
        } else if !constructed && has_class && has_constructor {
            hits.retain(|h| !constructor(h));
        }
        let call = after.starts_with('(');
        if !call || (groovy && !constructed) {
            return hits;
        }
        if !constructed && matches!(word, "values" | "valueOf") {
            let found = self.jvm_enum_of(here, chain);
            if !found.is_empty() {
                return found;
            }
        }
        if !constructed {
            let component = Regex::new(&format!(
                r"\brecord\s+\w+.*\b{w}\s*[,)]|^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*[\w.<>\[\]]+\s+{w}\s*(?:,\s*$|\))"
            ))
            .expect("an escaped name keeps the pattern valid");
            hits.retain(|h| {
                call_re.is_match(&h.text) || is_class(h) || component.is_match(&h.text)
            });
        }
        let Some((count, angle)) = arguments(&self.buf.lines, self.line, range.end) else {
            return hits;
        };
        if angle || hits.len() < 2 {
            return hits;
        }
        let map_argument = groovy && constructed && MAP_ARGUMENT.is_match(after);
        let count = match map_argument {
            true => 1,
            false => count,
        };
        let fits = |h: &Hit| {
            if !java_or_groovy(h) || is_class(h) {
                return true;
            }
            self.text_of(&h.path)
                .and_then(|t| search::jvm_parameters(&t, h.line1, word, search::groovy(&h.path)))
                .is_none_or(|(n, more)| (n..=n.saturating_add(more)).contains(&count))
        };
        let fit: Vec<Hit> = hits.iter().filter(|h| fits(h)).cloned().collect();
        let fit = match map_argument {
            true => {
                let map = Regex::new(&format!(r"\b{w}\s*\(\s*(?:final\s+)?Map\b"))
                    .expect("an escaped name keeps the pattern valid");
                let maps: Vec<Hit> = fit
                    .iter()
                    .filter(|h| map.is_match(&h.text))
                    .cloned()
                    .collect();
                match maps.is_empty() {
                    true => fit,
                    false => maps,
                }
            }
            false => fit,
        };
        match fit.is_empty() {
            true => hits,
            false => fit,
        }
    }

    fn jvm_enum_of(&self, here: &Path, chain: &[String]) -> Vec<Hit> {
        let text = self.buf.lines.join("\n");
        let enum_re = Regex::new(r"\benum\s+(?:class\s+)?(\w+)").expect("a fixed pattern");
        let hit = |path: &Path, line: usize, text: &str| Hit {
            deleted: None,
            path: path.to_path_buf(),
            line1: line,
            byte_col: None,
            text: text.to_owned(),
        };
        match chain {
            [] => search::jvm_enclosing_types(&text, self.line + 1)
                .into_iter()
                .find(|&d| enum_re.is_match(&self.buf.lines[d - 1]))
                .map(|d| hit(here, d, &self.buf.lines[d - 1]))
                .into_iter()
                .collect(),
            [owner] => {
                let pattern = search::def_patterns(Kind::Jvm, owner).join("|");
                let mut found = self.project_definitions(Kind::Jvm, here, owner, &pattern);
                found.retain(|h| enum_re.captures(&h.text).is_some_and(|c| &c[1] == owner));
                found
            }
            _ => Vec::new(),
        }
    }

    fn kotlin_infix(&self, before: &str, after: &str, word: &str) -> bool {
        if KOTLIN_KEYWORDS.contains(&word) || after.starts_with('(') || before.ends_with('.') {
            return false;
        }
        let b = before.trim_end();
        let a = after.trim_start();
        if b.len() == before.len() || a.len() == after.len() {
            return false;
        }
        let token = |s: &str| -> String { s.chars().take_while(|c| is_word(*c)).collect() };
        let last: String = {
            let rev: String = b.chars().rev().take_while(|c| is_word(*c)).collect();
            rev.chars().rev().collect()
        };
        let next = token(a);
        let operand_end = b.ends_with([')', ']', '"', '\'']) || !last.is_empty();
        let operand_start = a.starts_with(['(', '"', '\'']) || !next.is_empty();
        let keyword = |t: &str| KOTLIN_KEYWORDS.contains(&t);
        let condition = b.ends_with(')') && {
            let mut depth = 0i32;
            let open = b
                .char_indices()
                .rev()
                .find(|&(_, c)| {
                    match c {
                        ')' => depth += 1,
                        '(' => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })
                .map_or(0, |(i, _)| i);
            let head: String = {
                let t = b[..open].trim_end();
                let rev: String = t.chars().rev().take_while(|c| is_word(*c)).collect();
                rev.chars().rev().collect()
            };
            matches!(head.as_str(), "if" | "while" | "for" | "when" | "catch")
        };
        operand_end
            && operand_start
            && !condition
            && !keyword(&last)
            && !keyword(&next)
            && !b.ends_with(['=', ':', ',', '{', '('])
    }
}

pub(super) fn declared_as(kind: Kind, word: &str, text: &str) -> String {
    if kind != Kind::Jvm || whole_at(text, word, "").is_some() {
        return word.to_owned();
    }
    search::jvm_accessor(word)
        .and_then(|a| (a.fields_it_may_read.into_iter()).find(|n| whole_at(text, n, "").is_some()))
        .unwrap_or_else(|| word.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_counted_past_non_ascii_characters() {
        let lines = |ls: &[&str]| ls.iter().map(|l| l.to_string()).collect::<Vec<_>>();
        assert_eq!(
            arguments(&lines(&["naïve = Straße(ñ, ü)"]), 0, 16),
            Some((2, false))
        );
        assert_eq!(
            arguments(&lines(&[r#"f("ñ", 'ü')"#]), 0, 1),
            Some((2, false))
        );
        assert_eq!(
            arguments(
                &lines(&["\tx := new(", "s = \"open", "t = ü + `x`", ")"]),
                0,
                9
            ),
            Some((1, false))
        );
    }
}
