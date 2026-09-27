//! `u` / Shift+F12: every occurrence of the identifier under the cursor.

use super::*;

impl App {
    /// `u` / Shift+F12: every whole-word occurrence of the identifier, case-sensitive, in the
    /// order a reader wants them (#81): the declarations first, marked, then the open file, then
    /// the rest of the project's code nearest first, then tests, mocks, fixtures, generated and
    /// vendored files. The title says how the list splits.
    pub(super) fn usages(&mut self) {
        let extra = search::word_chars(self.kind(), false);
        let Some(word) = self.word_under(extra) else {
            self.message = "no word under the cursor".into();
            return;
        };
        let (ranked, cut) = self.usage_hits(&word, self.rel_current().as_deref());
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
                    col: word_col(&h.text, &word, &format!("{extra}{row}")),
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
        let hits = self
            .grep(&regex::escape(word), true, false, |_| true)
            .unwrap_or_default();
        let cut = hits.len() >= search::MAX_HITS;
        let hits = hits.into_iter().filter(|h| {
            let extra = search::word_chars(search::kind_of(&h.path), false);
            extra.is_empty() || whole_at(&h.text, word, extra).is_some()
        });
        // What tells a declaration of the word from a use of it is `def_patterns`, and which
        // ones apply is the hit file's own kind: one regex per kind met, built once.
        let mut rules: HashMap<Option<Kind>, Option<Regex>> = HashMap::new();
        let mut literal: HashMap<PathBuf, Vec<bool>> = HashMap::new();
        let mut ranked: Vec<_> = hits
            .map(|h| {
                let kind = search::kind_of(&h.path);
                let re = rules.entry(kind).or_insert_with_key(|k| {
                    let patterns = k.map(|k| search::def_patterns(k, word)).unwrap_or_default();
                    (!patterns.is_empty())
                        .then(|| Regex::new(&patterns.join("|")).ok())
                        .flatten()
                });
                // A pattern that matched inside a docstring, a raw string or a block comment
                // declares nothing, as `d` reads it too; only a file with a match is read.
                let declares = re.as_ref().is_some_and(|re| re.is_match(&h.text))
                    && !literal
                        .entry(h.path.clone())
                        .or_insert_with(|| {
                            kind.zip(self.text_of(&h.path))
                                .map_or_else(Vec::new, |(k, t)| search::literal_lines(k, &t))
                        })
                        .get(h.line - 1)
                        .copied()
                        .unwrap_or(false);
                (search::rank(&h.path, here, declares), h)
            })
            .collect();
        ranked.sort_by(|(a, x), (b, y)| {
            a.cmp(b)
                .then_with(|| (&x.path, x.line).cmp(&(&y.path, y.line)))
        });
        let ranked = ranked.into_iter().map(|((tier, _), h)| (tier, h)).collect();
        (ranked, cut)
    }
}
