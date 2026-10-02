use super::*;

mod spans;

impl App {
    pub(super) fn toggle_collapse(&mut self) {
        if self.deleted.is_some() || self.previewing() {
            return;
        }
        let l = self.line;
        if let Some(i) = self.collapsed.iter().position(|&(h, _)| h == l) {
            self.collapsed.remove(i);
            return;
        }
        let Some(folds) = folds(&self.buf) else {
            self.message = match self.buf.path.as_ref().and_then(|p| p.extension()) {
                Some(ext) => format!("no fold rules for .{}", ext.to_string_lossy()),
                None => "no fold rules for this file".into(),
            };
            return;
        };
        let Some((h, end)) = folds.target(l) else {
            self.message = "nothing to fold".into();
            return;
        };
        self.collapsed.push((h, end));
        self.nest_collapsed();
        self.anchor = None;
        self.set_at(TextLine::File(h));
        self.apply_want_x(0);
    }

    pub fn hidden(&self, l: usize) -> bool {
        let at = self.at().key();
        let inside = |l: usize, (h, e): (usize, usize)| h < l && l <= e;
        (self.collapsed.iter()).any(|&f| inside(l, f) && !inside(at, f))
    }

    pub fn collapsed_tail(&self, l: usize) -> Option<&str> {
        let e = self.collapsed_at(l)?;
        let (head, end) = (&self.buf.lines[l], &self.buf.lines[e]);
        let level = indent(head) == indent(end) && closes(end.trim());
        Some(if level { end.trim() } else { "" })
    }

    pub fn collapsed_at(&self, l: usize) -> Option<usize> {
        self.collapsed
            .iter()
            .find(|&&(h, _)| h == l)
            .map(|&(_, e)| e)
    }

    pub(super) fn reveal_cursor(&mut self) {
        self.unfold_over(self.at().key());
    }

    pub(super) fn unfold_over(&mut self, l: usize) {
        self.collapsed.retain(|&(h, e)| !(h < l && l <= e));
    }

    pub(super) fn shift_collapsed(&mut self, at: usize, old: usize, new: usize) {
        let Some(folds) = folds(&self.buf) else {
            return;
        };
        let moved = |(h, e): (usize, usize)| match () {
            _ if (at..at + old).contains(&h) => None,
            _ if e < at => Some((h, e)),
            _ if h >= at + old => Some((h + new - old, e + new - old)),
            _ => Some((h, folds.region(h)?)),
        };
        self.collapsed = self.collapsed.iter().filter_map(|&f| moved(f)).collect();
        self.nest_collapsed();
    }

    pub(super) fn carry_collapsed(&mut self, old: &[String]) {
        let heads: Vec<usize> = (self.collapsed.iter())
            .map(|&(h, _)| (h, super::open::carried(old, &self.buf.lines, h)))
            .filter(|&(h, n)| self.buf.lines.get(n) == old.get(h))
            .map(|(_, n)| n)
            .collect();
        self.measure_collapsed(heads);
    }

    pub(super) fn swap_collapsed(&mut self, old: &Buffer, path: &Path) {
        let left = std::mem::take(&mut self.collapsed);
        if let Some(p) = old.path.clone()
            && !left.is_empty()
        {
            let heads = left.iter().map(|&(h, _)| (h, old.lines[h].clone()));
            self.collapsed_stash.insert(p, heads.collect());
        }
        let heads = (self.collapsed_stash.remove(path).into_iter().flatten())
            .filter(|(h, text)| self.buf.lines.get(*h) == Some(text))
            .map(|(h, _)| h)
            .collect();
        self.measure_collapsed(heads);
    }

    fn measure_collapsed(&mut self, heads: Vec<usize>) {
        let Some(folds) = folds(&self.buf) else {
            self.collapsed.clear();
            return;
        };
        self.collapsed = heads
            .into_iter()
            .filter_map(|h| Some((h, folds.region(h)?)))
            .collect();
        self.nest_collapsed();
    }

    pub(super) fn nest_collapsed(&mut self) {
        self.collapsed.sort_unstable();
        self.collapsed.dedup_by_key(|f| f.0);
        for i in (0..self.collapsed.len()).rev() {
            let (h, mut e) = self.collapsed[i];
            for &(h2, e2) in &self.collapsed[i + 1..] {
                if h2 <= e {
                    e = e.max(e2);
                }
            }
            self.collapsed[i] = (h, e);
        }
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static FOLD_EVERY_KIND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn every_kind() -> bool {
    FOLD_EVERY_KIND.get()
}

#[cfg(not(test))]
fn every_kind() -> bool {
    false
}

fn folds(buf: &Buffer) -> Option<Folds<'_>> {
    let (path, lines) = (buf.path.as_deref()?, &buf.lines);
    if search::opened_kind(path) == Some(Kind::Python) {
        return Some(Folds::Shape(Shape::python(lines)));
    }
    Some(Folds::Spans(match path.extension()?.to_str()? {
        "json" => spans::json(lines),
        "css" => spans::css(lines),
        "scss" => spans::scss(lines),
        "yml" | "yaml" => spans::yaml(lines),
        "toml" => spans::toml(lines),
        _ if every_kind() => return Some(Folds::Shape(Shape::plain(lines))),
        _ => return None,
    }))
}

enum Folds<'a> {
    Shape(Shape<'a>),
    Spans(Vec<(usize, usize)>),
}

impl Folds<'_> {
    fn region(&self, h: usize) -> Option<usize> {
        match self {
            Folds::Shape(shape) => shape.region(h),
            Folds::Spans(spans) => spans.iter().find(|s| s.0 == h).map(|s| s.1),
        }
    }

    fn target(&self, l: usize) -> Option<(usize, usize)> {
        match self {
            Folds::Shape(shape) => shape.target(l),
            Folds::Spans(spans) => (spans.iter().find(|s| s.0 == l))
                .or_else(|| spans.iter().filter(|s| s.0 < l && l <= s.1).max())
                .copied(),
        }
    }
}

const COMPOUND: &[&str] = &[
    "def", "class", "if", "elif", "else", "for", "while", "with", "try", "except", "finally",
    "async",
];

pub(crate) struct Shape<'a> {
    lines: &'a [String],
    quiet: Vec<bool>,
    code: Vec<usize>,
    strings: HashMap<usize, usize>,
    imports: Vec<(usize, usize)>,
    nested: Vec<bool>,
    python: bool,
}

impl<'a> Shape<'a> {
    pub(crate) fn plain(lines: &'a [String]) -> Self {
        Shape {
            lines,
            quiet: vec![false; lines.len()],
            code: lines.iter().map(String::len).collect(),
            strings: HashMap::new(),
            imports: Vec::new(),
            nested: vec![false; lines.len()],
            python: false,
        }
    }

    pub(crate) fn python(lines: &'a [String]) -> Self {
        let mut shape = Shape::plain(lines);
        shape.python = true;
        let mut open: Option<(usize, &[u8])> = None;
        let mut depth = 0usize;
        for (n, line) in lines.iter().enumerate() {
            let b = line.as_bytes();
            shape.nested[n] = depth > 0;
            shape.quiet[n] = open.is_some() || line.trim_start().starts_with('#');
            let mut i = 0;
            while i < b.len() {
                if let Some((start, delim)) = open {
                    if b[i] == b'\\' {
                        i += 2;
                    } else if b[i..].starts_with(delim) {
                        if n > start {
                            shape.strings.insert(start, n);
                        }
                        open = None;
                        i += 3;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                match b[i] {
                    b'#' => {
                        shape.code[n] = i;
                        break;
                    }
                    q @ (b'"' | b'\'') => {
                        let triple: &[u8] = if q == b'"' { b"\"\"\"" } else { b"'''" };
                        if b[i..].starts_with(triple) {
                            open = Some((n, triple));
                            i += 3;
                            continue;
                        }
                        i += 1;
                        while i < b.len() && b[i] != q {
                            i += if b[i] == b'\\' { 2 } else { 1 };
                        }
                        i += 1;
                    }
                    b'(' | b'[' | b'{' => {
                        depth += 1;
                        i += 1;
                    }
                    b')' | b']' | b'}' => {
                        depth = depth.saturating_sub(1);
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
        }
        shape.imports = shape.import_runs();
        shape
    }

    fn code(&self, l: usize) -> &str {
        let line = &self.lines[l];
        line[..self.code[l].min(line.len())].trim()
    }

    fn silent(&self, l: usize) -> bool {
        self.quiet[l] || self.code(l).is_empty()
    }

    fn import(&self, l: usize) -> bool {
        let t = self.code(l);
        !self.quiet[l] && (t.starts_with("import ") || t.starts_with("from "))
    }

    fn import_runs(&self) -> Vec<(usize, usize)> {
        let mut runs = Vec::new();
        let mut i = 0;
        while i < self.lines.len() {
            if !self.import(i) {
                i += 1;
                continue;
            }
            let (start, ind) = (i, indent(&self.lines[i]));
            let mut count = 0;
            let end = loop {
                let end = self.block(i).unwrap_or(i);
                count += 1;
                let next = (end + 1..self.lines.len()).find(|&j| !self.silent(j));
                match next {
                    Some(j) if self.import(j) && indent(&self.lines[j]) == ind => i = j,
                    _ => break end,
                }
            };
            if count > 1 {
                runs.push((start, end));
            }
            i = end + 1;
        }
        runs
    }

    fn run_at(&self, l: usize) -> Option<(usize, usize)> {
        let ind = indent(&self.lines[l]);
        (self.imports.iter().copied())
            .find(|&(s, e)| s <= l && l <= e && self.import(l) && indent(&self.lines[s]) == ind)
    }

    pub(crate) fn region(&self, h: usize) -> Option<usize> {
        if self.quiet.get(h).copied().unwrap_or(true) {
            return None;
        }
        if let Some((_, end)) = self.imports.iter().find(|r| r.0 == h) {
            return Some(*end);
        }
        self.block(h).or_else(|| self.strings.get(&h).copied())
    }

    fn compound(&self, l: usize) -> bool {
        if self.nested[l] {
            return false;
        }
        let t = self.code(l);
        let word = t.split(|c: char| !c.is_alphanumeric() && c != '_').next();
        match word {
            Some(w @ ("match" | "case")) => {
                let rest = t[w.len()..].trim_start();
                t[w.len()..].starts_with(' ') && !rest.starts_with(['=', '.', ')', ',', ':'])
            }
            Some(w) => COMPOUND.contains(&w),
            None => false,
        }
    }

    fn defines(&self, l: usize) -> bool {
        let t = self.code(l);
        let t = t.strip_prefix("async ").unwrap_or(t);
        t.starts_with("def ") || t.starts_with("class ")
    }

    fn enclosing(&self, at: usize, covers: impl Fn(usize) -> bool) -> Option<usize> {
        let tail = !self.silent(at) && self.code(at).starts_with([')', ']', '}']);
        let mut depth = self.depth(at) + usize::from(tail);
        for i in (0..at).rev() {
            if self.silent(i) || indent(&self.lines[i]) >= depth {
                continue;
            }
            depth = indent(&self.lines[i]);
            if self.code(i).starts_with([')', ']', '}']) {
                depth += 1;
            } else if self.defines(i) && covers(i) {
                return Some(i);
            }
        }
        None
    }

    fn block(&self, h: usize) -> Option<usize> {
        let head = self.lines.get(h).filter(|_| !self.silent(h))?;
        let ind = indent(head);
        let header = !self.python || self.compound(h);
        let mut end = None;
        let mut i = h + 1;
        while i < self.lines.len() {
            if self.silent(i) {
                i += 1;
                continue;
            }
            let (line, t) = (&self.lines[i], self.code(i));
            if indent(line) > ind {
                let last = self.strings.get(&i).copied().unwrap_or(i);
                end = Some(last);
                i = last + 1;
                continue;
            }
            if indent(line) < ind {
                break;
            }
            let tail = match self.python {
                true => t.starts_with([')', ']', '}']) && (t.ends_with([':', '{']) || !closes(t)),
                false => t.starts_with(')') && !closes(t),
            };
            if header && (t == "{" || tail) {
                end = Some(i);
                i += 1;
                continue;
            }
            if end.is_some() && closes(t) {
                end = Some(i);
            }
            break;
        }
        end
    }

    fn opens(&self, l: usize) -> bool {
        let t = self.code(l);
        let tail = t.starts_with([')', ']', '}']) && !t.ends_with(['(', '[', '{']);
        if self.silent(l) || tail || self.region(l).is_none() {
            return false;
        }
        self.compound(l)
            || t.ends_with([':', '(', '[', '{'])
            || self.strings.contains_key(&l)
            || self.defines(l)
    }

    fn anchor(&self, l: usize) -> usize {
        (self.strings.iter())
            .filter(|&(&s, &e)| s < l && l <= e)
            .map(|(&s, _)| s)
            .max()
            .unwrap_or(l)
    }

    pub(crate) fn target(&self, l: usize) -> Option<(usize, usize)> {
        if let Some(run) = self.run_at(l) {
            return Some(run);
        }
        let at = self.anchor(l);
        let covers = |h: usize| self.region(h).is_some_and(|e| e >= l);
        let h = (at == l && self.opens(l))
            .then_some(l)
            .or_else(|| self.enclosing(at, covers))
            .or_else(|| {
                let deep = self.depth(at);
                (0..at)
                    .rev()
                    .find(|&h| !self.silent(h) && indent(&self.lines[h]) < deep && covers(h))
            })?;
        Some((h, self.region(h)?))
    }

    fn depth(&self, l: usize) -> usize {
        (l..self.lines.len())
            .find(|&i| !self.silent(i))
            .map_or(0, |i| indent(&self.lines[i]))
    }
}

#[cfg(test)]
pub(crate) fn block_end(lines: &[String], h: usize) -> Option<usize> {
    Shape::plain(lines).block(h)
}

fn closes(t: &str) -> bool {
    let brackets = t.starts_with([')', ']', '}'])
        && t.chars()
            .all(|c| matches!(c, ')' | ']' | '}' | ';' | ',' | ':' | ' '));
    let end = t
        .strip_prefix("end")
        .is_some_and(|r| r.chars().all(|c| matches!(c, ';' | ')' | ',' | ' ')));
    brackets || end || (t.starts_with("</") && t.ends_with('>'))
}

fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}
