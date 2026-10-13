use super::{
    App, Hit, MAX_NAME_PAD, Mode, PickItem, Picker, PickerKind, SearchJob, at_label, wrap,
};

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
        let items = Self::symbol_items(named);
        self.picker = Some(Picker::new(PickerKind::Symbols.title(), items, false));
        self.mode = Mode::Picker(PickerKind::Symbols);
        if !cut {
            return;
        }
        self.drop_pending_search();
        if let Some(p) = &mut self.picker {
            p.live = true;
        }
    }

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
                        at_label(&h.path, h.line1),
                    ),
                    deleted: h.deleted.is_some(),
                    path: h.path,
                    line: h.line1,
                    col,
                    code_at: None,
                    path_at: None,
                },
            )
            .collect()
    }
}
