use super::*;

pub(super) struct NamedSymbol {
    pub name: String,
    pub byte_col: usize,
    pub hit: Hit,
}

pub(super) struct SymbolHits {
    pub named: Vec<NamedSymbol>,
    pub cut: bool,
}

impl App {
    /// `D`: every declaration in the project, recomputed on each press. A list the
    /// [`search::MAX_HITS`] cut left short is not the project's symbols, so the query stops
    /// filtering it: it greps the declaration patterns for a name of its own, as `s` greps
    /// ([`App::search_tick`]). The rows stay on screen meanwhile, under a title that says what
    /// they are.
    pub(super) fn symbols(&mut self) {
        let job = SearchJob {
            deleted: self.symbol_deleted(),
            ..self.grep_job(0, "", |_| true)
        };
        let SymbolHits { named, cut } = job.symbol_hits();
        if named.is_empty() {
            self.message = "no symbols".into();
            return;
        }
        // Not `show_picker`: it reads a cut off the row count, and the cap belongs to each
        // pattern. What is on screen is the cut list until a query is typed, and the answer to
        // that query after, so the title (`ui::draw_picker`) is the live picker's business.
        let items = Self::symbol_items(named);
        self.picker = Some(Picker::new(PickerKind::Symbols.title(), items, false));
        self.mode = Mode::Picker(PickerKind::Symbols);
        if !cut {
            return;
        }
        // An answer still on its way belongs to a search that is gone.
        self.drop_pending_search();
        if let Some(p) = &mut self.picker {
            p.live = true;
        }
    }

    /// `name  path:line` rows for `D`, sorted by name, the names padded into a column.
    pub(super) fn symbol_items(mut named: Vec<NamedSymbol>) -> Vec<PickItem> {
        named.sort_by_cached_key(|s| (s.name.to_lowercase(), s.hit.path.clone(), s.hit.place()));
        let width = named
            .iter()
            .map(|s| wrap::width(&s.name))
            .max()
            .unwrap_or(0)
            .min(MAX_NAME_PAD);
        named
            .into_iter()
            .map(
                |NamedSymbol {
                     name,
                     byte_col: col,
                     hit: h,
                 }| PickItem {
                    label: format!(
                        "{name}{}  {}",
                        " ".repeat(width.saturating_sub(wrap::width(&name))),
                        at_label(&h.path, h.line),
                    ),
                    deleted: h.deleted.is_some(),
                    path: h.path,
                    line: h.line,
                    col,
                    code_at: None,
                    path_at: None,
                },
            )
            .collect()
    }
}
