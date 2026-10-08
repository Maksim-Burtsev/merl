use super::c::Ctx;
use super::{K, Model};

impl Model<'_, '_> {
    pub(super) fn objc_container(&mut self, k: usize, marked: &mut [Option<Ctx>]) -> Option<usize> {
        let t = self.toks[k];
        let container =
            t.kind == K::Word && matches!(t.text, "@interface" | "@implementation" | "@protocol");
        if !container || self.punct(k + 1, "(") || matches!(self.tx(k + 2), ";" | ",") {
            return None;
        }
        let end = (k + 1..self.toks.len()).find(|&j| self.tx(j) == "@end")?;
        let attribute = (k.checked_sub(1).and_then(|p| self.back[p]))
            .and_then(|o| o.checked_sub(1))
            .filter(|&w| self.word_at(w) && self.first[w]);
        let start = attribute.map_or(t.line, |w| self.toks[w].line);
        self.add(start, Some(self.toks[end].line), true);
        let header = |j: usize| {
            self.toks[j].line == t.line
                && (self.word_at(j) || matches!(self.tx(j), ":" | "<" | ">" | "," | "*"))
        };
        let mut j = k + 1;
        while j < end && header(j) {
            j += 1;
        }
        if self.punct(j, "(") {
            j = self.pair[j].map_or(j, |c| c + 1);
        }
        while j < end && header(j) {
            j += 1;
        }
        if self.punct(j, "{") {
            marked[j] = Some(Ctx::Decl);
        }
        Some(end)
    }

    pub(super) fn objc_method(&mut self, k: usize, marked: &mut [Option<Ctx>]) {
        if !(self.first[k] && matches!(self.tx(k), "-" | "+")) {
            return;
        }
        let mut j = k + 1;
        while j < self.toks.len() {
            match self.tx(j) {
                "(" | "[" => match self.pair[j] {
                    Some(c) => j = c,
                    None => return,
                },
                ";" => return self.add(self.toks[k].line, Some(self.end_of(j)), false),
                "{" => {
                    marked[j] = Some(Ctx::Code);
                    return self.add(self.toks[k].line, self.closing(j), true);
                }
                "}" => return,
                w if w.starts_with('@') => return,
                _ => {}
            }
            j += 1;
        }
    }

    pub(super) fn block_literal(&self, p: usize) -> bool {
        let mut j = match self.tx(p) {
            ")" => match self.back[p].and_then(|o| o.checked_sub(1)) {
                Some(j) => j,
                None => return false,
            },
            _ => p,
        };
        while j > 0 && (self.word_at(j) || self.tx(j) == "*") {
            j -= 1;
        }
        self.tx(j) == "^"
    }
}
