//! `u` / Shift+F12: every occurrence of the identifier under the cursor.

use super::*;

impl App {
    /// `u` / Shift+F12: every whole-word occurrence of the identifier, case-sensitive, in the
    /// order a reader wants them (#81): the declarations first, marked, then the open file, then
    /// the rest of the project's code nearest first, then tests, mocks, fixtures, generated and
    /// vendored files. The title says how the list splits.
    pub(super) fn usages(&mut self) {
        let extra = search::word_chars(self.kind(), false);
        // A Ruby name is read as `d` reads it (#387): `valid?` lists `valid?`, its own `def`
        // first, and on `x.name = v` the setter `name=` declared by `attr_writer :name` does. So
        // is an Elixir one, whose `?` or `!` is the name's too (#459).
        // On a Ruby `@x` or `@@x` the word keeps its sigil, so that `@x =` declares it (#383).
        // On a line the branch deleted, the word is read there (#440).
        let Some(read) = self.on_drawn(|a| match a.kind() {
            k @ Some(Kind::Ruby | Kind::Elixir) => a.definition_word(k).map(|(r, w)| {
                let lead = &a.line_str()[..r.start];
                let sigil = lead.len() - lead.trim_end_matches('@').len();
                match k == Some(Kind::Ruby) && (1..=2).contains(&sigil) {
                    true => format!("{}{w}", &lead[lead.len() - sigil..]),
                    false => w,
                }
            }),
            _ => a.word_under(extra),
        }) else {
            self.message = "no word under the cursor".into();
            return;
        };
        let (ranked, cut) = self.usage_hits(&read, self.rel_current().as_deref());
        let word = read.trim_start_matches('@');
        let word = word.strip_suffix('=').unwrap_or(word);
        if ranked.is_empty() {
            self.message = format!("no usages of {word}");
            return;
        }
        let tiers: Vec<Tier> = ranked.iter().map(|(t, _)| *t).collect();
        // The grep's column is the first match it calls a whole word, and to it `-` ends one: in a
        // Makefile that is the start of `build-image-arm` for `build-image`. Every row lands on
        // the word whole both as `u` read it under the cursor and as the row's own language reads
        // it, the rule that listed the row (#281): `build-image` of a Makefile in a shell script,
        // `app` of Python on `my-app app` in YAML.
        let hits = ranked
            .into_iter()
            .map(|(_, h)| {
                let row = search::word_chars(search::kind_of(&h.path), false);
                Hit {
                    col: word_col(&h.text, word, &format!("{extra}{row}")),
                    ..h
                }
            })
            .collect();
        let items = Self::hit_items(hits);
        let declarations = tiers.iter().filter(|&&t| t == Tier::Declaration).count();
        let tests = tiers.iter().filter(|&&t| t == Tier::Tests).count();
        // A declaration row says so in the column `d` puts its reason in; with no declaration
        // among the hits the column is not there at all.
        let width = if declarations > 0 {
            "declaration".len() + 2
        } else {
            0
        };
        let items = items
            .into_iter()
            .zip(&tiers)
            .map(|(it, &tier)| {
                let mark = if tier == Tier::Declaration {
                    "declaration"
                } else {
                    ""
                };
                let head = format!("{mark:width$}");
                PickItem {
                    code_at: it.code_at.map(|at| at + head.len()),
                    label: head + &it.label,
                    ..it
                }
            })
            .collect();
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

    /// Every whole-word, case-sensitive hit of `word` in `u`'s order from the file `here`, each
    /// with its tier, and whether the grep stopped at its cap: the filter below can make a cut
    /// list short, and it is still cut.
    pub(super) fn usage_hits(&self, word: &str, here: Option<&Path>) -> (Vec<(Tier, Hit)>, bool) {
        // To grep `-` ends a word, so `db-main` also finds `db-main-2`: a hit goes when its own
        // file's language counts that `-` as part of a word and no occurrence in the line stands
        // whole (#281). In code `db-main-2` is a subtraction and stays.
        // A Ruby setter `name=` is called as `x.name = v`: the rows hold its bare name. On a Ruby
        // `@x` they are every `x`, as on a bare `x`, and its assignment `@x =` declares it too.
        let ivar = word.starts_with('@').then_some(word);
        let word = word.trim_start_matches('@');
        let text = word.strip_suffix('=').unwrap_or(word);
        let mut hits = self
            .grep(&regex::escape(text), true, false, |_| true)
            .unwrap_or_default();
        let cut = hits.len() >= search::MAX_HITS;
        // In a review, the lines the branch deleted too (#440), where the word stands whole as the
        // grep reads a word.
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
        let hits = hits.into_iter().filter(|h| {
            let extra = search::word_chars(search::kind_of(&h.path), false);
            extra.is_empty() || whole_at(&h.text, text, extra).is_some()
        });
        // What tells a declaration of the word from a use of it is `def_patterns`, and which
        // ones apply is the hit file's own kind: one regex per kind met, built once. A Rust `let`
        // declares its local here too, though `d` reads it by scope and never by name (#353).
        let mut rules: HashMap<Option<Kind>, Option<Regex>> = HashMap::new();
        let objc = self.objc_file();
        // A deleted line is read in the file at the base, apart from the file on disk.
        let mut literal: HashMap<(PathBuf, bool), Vec<bool>> = HashMap::new();
        let mut lines: HashMap<(PathBuf, bool), Vec<String>> = HashMap::new();
        let mut marked: Vec<_> = hits
            .map(|h| {
                let kind = search::kind_of(&h.path);
                let re = rules.entry(kind).or_insert_with_key(|k| {
                    let mut patterns = k
                        .map(|k| search::def_patterns_for(k, word, objc))
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
                        .or_insert_with(|| {
                            kind.zip(self.hit_text(&h))
                                .map_or_else(Vec::new, |(k, t)| search::literal_lines(k, &t))
                        })
                        .get(h.line - 1)
                        .copied()
                        .unwrap_or(false)
                    && kind.is_some_and(|k| {
                        search::declares_where(k, &h.path, word, h.line, &h.text, || {
                            lines
                                .entry((h.path.clone(), h.deleted.is_some()))
                                .or_insert_with(|| {
                                    self.hit_text(&h).map_or_else(Vec::new, |t| {
                                        t.lines().map(str::to_owned).collect()
                                    })
                                })
                        })
                    });
                (kind, declares, h)
            })
            .collect();
        // In a Makefile `u` marks what `d` counts (#504): a recipe line declares only what
        // `make_recipe_rule` says, and `X += …` or `release: X := 1.0` declare `X` only when no
        // plain line among the hits does (#499).
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
        let ranked = ranked.into_iter().map(|((tier, _), h)| (tier, h)).collect();
        (ranked, cut)
    }
}
