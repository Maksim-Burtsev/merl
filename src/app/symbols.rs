//! `D`: the project's declarations, listed and filtered by name.

use super::*;

impl App {
    /// `D`: every declaration in the project, recomputed on each press. A list the
    /// [`search::MAX_HITS`] cut left short is not the project's symbols, so the query stops
    /// filtering it: it greps the declaration patterns for a name of its own, as `s` greps
    /// ([`App::search_tick`]). The rows stay on screen meanwhile, under a title that says what
    /// they are.
    pub(super) fn symbols(&mut self) {
        let (named, cut) = self.grep_job(0, "", |_| true).symbol_hits();
        if named.is_empty() {
            self.message = "no symbols".into();
            return;
        }
        // Not `show_picker`: it reads a cut off the row count, and the cap belongs to each
        // pattern. What is on screen is the cut list until a query is typed, and the answer to
        // that query after, so the title (`ui::draw_picker`) is the live picker's business.
        let items = Self::symbol_items(named);
        let mut picker = Picker::new(PickerKind::Symbols.title(), items, false);
        picker.literal = true;
        self.picker = Some(picker);
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
    pub(super) fn symbol_items(mut named: Vec<(String, usize, Hit)>) -> Vec<PickItem> {
        named.sort_by_cached_key(|(n, _, h)| (n.to_lowercase(), h.path.clone(), h.place()));
        let width = named
            .iter()
            .map(|(n, ..)| wrap::width(n))
            .max()
            .unwrap_or(0)
            .min(MAX_NAME_PAD);
        named
            .into_iter()
            .map(|(name, col, h)| PickItem {
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
            })
            .collect()
    }
}
