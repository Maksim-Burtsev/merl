use super::*;

pub(super) struct UsageHits {
    pub ranked: Vec<(Tier, Hit)>,
    pub grep_cut: bool,
}

impl App {
    pub(super) fn usages(&mut self) {
        let extra = search::word_chars(self.kind(), false);
        let Some(read) = self.on_drawn(|a| a.usage_word()) else {
            self.message = "no word under the cursor".into();
            return;
        };
        let UsageHits {
            ranked,
            grep_cut: cut,
        } = self.usage_hits(&read, self.rel_current().as_deref());
        let word = bare_name(&read);
        if ranked.is_empty() {
            self.message = format!("no usages of {word}");
            return;
        }
        let tiers: Vec<Tier> = ranked.iter().map(|(t, _)| *t).collect();
        let hits = ranked
            .into_iter()
            .map(|(_, h)| {
                let kind = search::kind_of(&h.path);
                let row = search::word_chars(kind, false);
                let col = match kind {
                    Some(Kind::Haskell) => search::haskell_whole(&h.text, word),
                    Some(Kind::Julia) => search::julia_col(&h.text, word),
                    _ => None,
                };
                Hit {
                    byte_col: Some(col.unwrap_or_else(|| {
                        word_col(&h.text, word, &format!("{}{row}", extra.replace('\'', "")))
                    })),
                    ..h
                }
            })
            .collect();
        let items = Self::hit_items(hits);
        let declarations = tiers.iter().filter(|&&t| t == Tier::Declaration).count();
        let tests = tiers.iter().filter(|&&t| t == Tier::Tests).count();
        let counts = [
            (
                declarations,
                if declarations == 1 {
                    "declaration"
                } else {
                    "declarations"
                },
            ),
            (tiers.len() - declarations - tests, "in code"),
            (tests, "in tests"),
        ];
        let split: Vec<String> = counts
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| format!("{n} {what}"))
            .collect();
        let status = format!("Usages of {word}: {}", split.join(", "));
        self.show_cut_picker(PickerKind::Usages, items, cut);
        if let Some(p) = &mut self.picker {
            p.title = p.title.replacen(PickerKind::Usages.title(), &status, 1);
        }
    }

    pub(super) fn usage_word(&self) -> Option<String> {
        match self.kind() {
            k @ Some(
                Kind::Ruby
                | Kind::Elixir
                | Kind::Cmake
                | Kind::Nix
                | Kind::Haskell
                | Kind::Ocaml
                | Kind::Fsharp
                | Kind::Julia
                | Kind::R
                | Kind::Clojure
                | Kind::EmacsLisp
                | Kind::Scheme
                | Kind::CommonLisp,
            ) => self.definition_word(k).map(|(r, w)| {
                let lead = &self.line_str()[..r.start];
                let sigil = lead.len() - lead.trim_end_matches('@').len();
                match k == Some(Kind::Ruby) && (1..=2).contains(&sigil) {
                    true => format!("{}{w}", &lead[lead.len() - sigil..]),
                    false => w,
                }
            }),
            k => self
                .css_word()
                .or_else(|| self.word_under(search::word_chars(k, false))),
        }
    }

    /// Every whole-word, case-sensitive hit of `word` in `u`'s order from the file `here`. The
    /// filter below can make a cut list short, and it is still cut.
    pub(super) fn usage_hits(&self, word: &str, here: Option<&Path>) -> UsageHits {
        // A Ruby setter `name=` is called as `x.name = v`: the rows hold its bare name. On a Ruby
        // `@x` they are every `x`, as on a bare `x`, and its assignment `@x =` declares it too.
        let ivar = word.starts_with('@').then_some(word);
        let word = word.trim_start_matches('@');
        let text = bare_name(word);
        let mut hits = self
            .grep(&regex::escape(text), true, false, |_| true)
            .unwrap_or_default();
        let cut = hits.len() >= search::MAX_HITS;
        // A word past the matcher's size limit finds none, as the grep's does.
        let deleted = self.deleted_lines();
        if !deleted.is_empty()
            && let Ok(whole) = Regex::new(&format!(r"(?:^|\W)({})(?:$|\W)", regex::escape(text)))
        {
            hits.extend(deleted_hits(
                &deleted,
                |_| true,
                |t| whole.captures(t).and_then(|c| c.get(1)).map(|m| m.start()),
            ));
        }
        let hits = hits.into_iter().filter(|h| match search::kind_of(&h.path) {
            Some(Kind::Haskell) => search::haskell_whole(&h.text, text).is_some(),
            k => {
                let extra = search::word_chars(k, false);
                (extra.is_empty() || whole_at(&h.text, text, extra).is_some())
                    && (k != Some(Kind::Julia) || search::julia_whole(&h.text, text))
            }
        });
        // What tells a declaration of the word from a use of it is `def_patterns`, and which
        // ones apply is the hit file's own kind: one regex per kind met, built once.
        let mut rules: HashMap<Option<Kind>, Option<Regex>> = HashMap::new();
        let dialect = self.c_dialect();
        // A deleted line is read in the file at the base, apart from the file on disk.
        let mut literal: HashMap<(PathBuf, bool), Vec<bool>> = HashMap::new();
        let mut lines: HashMap<(PathBuf, bool), Vec<String>> = HashMap::new();
        let mut marked: Vec<_> = hits
            .map(|h| {
                let kind = search::kind_of(&h.path);
                let re = rules.entry(kind).or_insert_with_key(|k| {
                    let mut patterns = k
                        .map(|k| search::def_patterns_for(k, word, dialect))
                        .unwrap_or_default();
                    if let (Some(Kind::Ruby), Some(ivar)) = (k, ivar) {
                        patterns.push(search::ruby_assignment(ivar));
                    }
                    if *k == Some(Kind::Rust) {
                        let w = regex::escape(word);
                        patterns.push(format!(r"^\s*let\s+(?:mut\s+)?{w}\b"));
                    }
                    (!patterns.is_empty())
                        .then(|| Regex::new(&patterns.join("|")).ok())
                        .flatten()
                });
                // A pattern that matched inside a docstring, a raw string or a block comment
                // declares nothing, and neither does a line the lines around it make a use, as
                // `d` reads them too; only a file with a match is read.
                // A PowerShell line declares what the word's own spelling there allows (#420).
                let declares = re.as_ref().is_some_and(|re| re.is_match(&h.text))
                    && (kind != Some(Kind::PowerShell)
                        || search::powershell_declares_here(&h.text, word))
                    && !literal
                        .entry((h.path.clone(), h.deleted.is_some()))
                        .or_insert_with(|| kind.map_or_else(Vec::new, |k| self.hidden_of(k, &h)))
                        .get(h.line1 - 1)
                        .copied()
                        .unwrap_or(false)
                    && kind.is_some_and(|k| {
                        let key = (h.path.clone(), h.deleted.is_some());
                        let read = || {
                            self.hit_text(&h)
                                .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                        };
                        match k {
                            Kind::Nix => {
                                let file = lines.entry(key).or_insert_with(read);
                                search::nix_declares(file, h.line1, word, true)
                            }
                            _ => search::declares_where(k, &h.path, word, h.line1, &h.text, || {
                                lines.entry(key).or_insert_with(read)
                            }),
                        }
                    });
                (kind, declares, h)
            })
            .collect();
        let mut recipe = self.make_recipe_rule(here, word);
        let make = Some(Kind::Make);
        for (kind, declares, h) in &mut marked {
            *declares &= *kind != make || recipe(h).unwrap_or(true);
        }
        if !marked.iter().any(|(k, d, _)| *k == make && *d) {
            let fallback = Regex::new(&search::make_fallback_patterns(word).join("|")).ok();
            for (kind, declares, h) in &mut marked {
                *declares |= *kind == make
                    && fallback.as_ref().is_some_and(|re| re.is_match(&h.text))
                    && recipe(h).is_none();
            }
        }
        let mut ranked: Vec<_> = marked
            .into_iter()
            .map(|(_, declares, h)| (search::rank(&h.path, here, declares), h))
            .collect();
        ranked.sort_by(|(a, x), (b, y)| {
            a.cmp(b)
                .then_with(|| (&x.path, x.place()).cmp(&(&y.path, y.place())))
        });
        let ranked = ranked.into_iter().map(|(rank, h)| (rank.tier, h)).collect();
        UsageHits {
            ranked,
            grep_cut: cut,
        }
    }

    pub(super) fn make_recipe_rule<'a>(
        &'a self,
        here: Option<&Path>,
        word: &str,
    ) -> impl FnMut(&Hit) -> Option<bool> + 'a {
        let line = self.line_str();
        let shell =
            search::definition_word(Some(Kind::Make), line, self.col).is_some_and(|(r, w)| {
                let before = &line[..r.start];
                let shell_ref = before.ends_with("$$(") || before.ends_with("$${");
                let make_ref = before.ends_with("$(") || before.ends_with("${");
                w == word && (shell_ref || !make_ref)
            });
        let command = search::make_recipe_command(&self.buf.lines.join("\n"), self.line + 1)
            .filter(|_| shell);
        let here = here.map(Path::to_path_buf);
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        move |h: &Hit| {
            let text = texts
                .entry(h.path.clone())
                .or_insert_with(|| self.text_of(&h.path));
            let at = search::make_recipe_command(text.as_deref()?, h.line1)?;
            Some(here.as_ref() == Some(&h.path) && Some(at) == command)
        }
    }
}

pub(super) fn bare_name(word: &str) -> &str {
    let word = word.trim_start_matches('@');
    word.strip_suffix('=').unwrap_or(word)
}
