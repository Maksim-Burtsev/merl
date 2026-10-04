use super::*;

static ELIXIR_QUALIFIER: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?:^|[^\w:]):([a-z]\w*)\.$").unwrap());
static ERLANG_QUALIFIER: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?:^|[^\w:'])(\??[a-zA-Z_]\w*):$").unwrap());
static TYPED: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"^-\s*(?:spec|type|opaque|callback)\b|::").unwrap());
static RECORD_DOT: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"#([a-z]\w*)\.$").unwrap());
static RECORD_OPEN: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"#([a-z]\w*)\{").unwrap());

impl App {
    pub(super) fn erlang_definitions(
        &mut self,
        here: &Path,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> Option<Vec<Candidate>> {
        let line = self.line_str().to_owned();
        let (before, after) = (&line[..range.start], &line[range.end..]);
        let w = regex::escape(word);
        if !search::erlang(here) {
            if before.ends_with(':') && !before.ends_with("::") && after.starts_with('.') {
                return self.in_erlang_module(word, &search::erlang_module_pattern(&w), word);
            }
            let module = ELIXIR_QUALIFIER.captures(before)?[1].to_owned();
            return self.in_erlang_module(&module, &search::erlang_clause_pattern(&w), word);
        }
        if before.ends_with('?') {
            if search::ERLANG_PREDEFINED.contains(&word) {
                return Some(Vec::new());
            }
            return Some(self.erlang_by_name_candidates(
                here,
                &search::erlang_macro_pattern(&w),
                word,
            ));
        }
        if before.ends_with('#') {
            return Some(self.erlang_by_name_candidates(
                here,
                &search::erlang_record_pattern(&w),
                word,
            ));
        }
        if let Some(record) = record_of_field(before, after) {
            return Some(self.erlang_field(here, &record, word));
        }
        if word.starts_with(|c: char| c.is_ascii_uppercase() || c == '_') {
            return Some(Vec::new());
        }
        if before.ends_with("':") {
            return Some(Vec::new());
        }
        if let Some(c) = ERLANG_QUALIFIER.captures(before) {
            let module = match &c[1] {
                "?MODULE" => here.file_stem()?.to_str()?.to_owned(),
                m if m.starts_with(|c: char| c.is_ascii_lowercase()) => m.to_owned(),
                _ => return None,
            };
            return self.in_erlang_module(&module, &search::erlang_clause_pattern(&w), word);
        }
        if after.starts_with(':') && !after.starts_with("::") {
            return self.in_erlang_module(word, &search::erlang_module_pattern(&w), word);
        }
        let named = before.trim_end().strip_prefix('-').map(str::trim_start);
        if after.starts_with('(')
            && TYPED.is_match(&line)
            && !matches!(named, Some("spec" | "callback"))
        {
            return Some(self.erlang_by_name_candidates(
                here,
                &search::erlang_type_pattern(&w),
                word,
            ));
        }
        let called = after.starts_with('(')
            || (before.trim_end().ends_with("fun") && after.trim_start().starts_with('/'));
        if !called {
            return None;
        }
        let re = Regex::new(&search::erlang_clause_pattern(&w)).ok()?;
        let lines: Vec<usize> = (0..self.buf.lines.len())
            .filter(|&i| {
                re.is_match(&self.buf.lines[i])
                    && search::declares_where(
                        Kind::Elixir,
                        here,
                        word,
                        i + 1,
                        &self.buf.lines[i],
                        || &self.buf.lines,
                    )
            })
            .collect();
        if !lines.is_empty() {
            return Some(
                lines
                    .into_iter()
                    .map(|i| self.candidate(here, i + 1, Reason::File))
                    .collect(),
            );
        }
        if let Some(module) = search::erlang_imported(&self.buf.lines.join("\n"), word) {
            return self.in_erlang_module(&module, &search::erlang_clause_pattern(&w), word);
        }
        self.offer_only = true;
        Some(self.erlang_by_name_candidates(here, &search::erlang_clause_pattern(&w), word))
    }

    fn candidate(&self, path: &Path, line: usize, reason: Reason) -> Candidate {
        let text = match self.rel_current().as_deref() == Some(path) {
            true => self.buf.lines.get(line - 1).cloned(),
            false => self
                .file_text(path)
                .and_then(|t| t.lines().nth(line - 1).map(str::to_owned)),
        };
        Candidate {
            hit: Hit {
                deleted: None,
                path: path.to_path_buf(),
                line,
                col: 0,
                text: text.unwrap_or_default(),
            },
            reason,
        }
    }

    fn erlang_grep(
        &mut self,
        pattern: &str,
        word: &str,
        wanted: impl Fn(&Path) -> bool,
    ) -> Vec<Hit> {
        let outside: Vec<PathBuf> = (self.external_files(Kind::Elixir).iter())
            .filter(|p| wanted(p))
            .cloned()
            .collect();
        let mut hits = self
            .grep(pattern, false, false, |p| wanted(p))
            .unwrap_or_default();
        hits.extend(self.external_grep(Kind::Elixir, &outside, pattern));
        self.declaring(Kind::Elixir, word, hits)
    }

    fn in_erlang_module(
        &mut self,
        module: &str,
        pattern: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let file = format!("{module}.erl");
        let named = |p: &Path| p.file_name().is_some_and(|n| *n == *file);
        let known = self.files.iter().any(|p| named(p))
            || self.external_files(Kind::Elixir).iter().any(|p| named(p));
        if !known {
            return None;
        }
        let hits = self.erlang_grep(pattern, word, named);
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Path(module.to_owned()),
                })
                .collect(),
        )
    }

    fn erlang_by_name(&mut self, here: &Path, pattern: &str, word: &str) -> Vec<Hit> {
        let hits = self.erlang_grep(pattern, word, search::erlang);
        let own: Vec<Hit> = hits.iter().filter(|h| h.path == here).cloned().collect();
        match own.is_empty() {
            true => hits,
            false => own,
        }
    }

    fn erlang_field(&mut self, here: &Path, record: &str, field: &str) -> Vec<Candidate> {
        let pattern = search::erlang_record_pattern(&regex::escape(record));
        let records = self.erlang_by_name(here, &pattern, record);
        records
            .into_iter()
            .filter_map(|h| {
                let text = self.text_of(&h.path)?;
                let lines: Vec<&str> = text.lines().collect();
                let at = search::erlang_record_field(&lines, h.line, field)?;
                Some(self.candidate(&h.path, at, Reason::Label(format!("field of #{record}"))))
            })
            .collect()
    }

    fn erlang_by_name_candidates(
        &mut self,
        here: &Path,
        pattern: &str,
        word: &str,
    ) -> Vec<Candidate> {
        self.erlang_by_name(here, pattern, word)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }
}

fn record_of_field(before: &str, after: &str) -> Option<String> {
    if let Some(c) = RECORD_DOT.captures(before) {
        return Some(c[1].to_owned());
    }
    let rest = after.trim_start();
    let assigned = rest.starts_with('=')
        && !["==", "=:=", "=/=", "=<", "=>"]
            .iter()
            .any(|o| rest.starts_with(o));
    if !assigned {
        return None;
    }
    let open = RECORD_OPEN.captures_iter(before).last()?;
    let inside = &before[open.get(0)?.end()..];
    let depth = inside.chars().fold(0i32, |d, c| match c {
        '{' | '(' | '[' => d + 1,
        '}' | ')' | ']' => d - 1,
        _ => d,
    });
    (depth == 0).then(|| open[1].to_owned())
}
