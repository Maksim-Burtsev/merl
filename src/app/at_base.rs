use super::{
    App, Buffer, Candidate, FileLine, HashMap, Hit, Kind, Mode, Path, PathBuf, PickItem,
    PickerKind, Reason, Regex, at_label, git, search, search_job,
};
use search_job::deleted_hits;

pub(super) type AnswerOfD = (
    String,
    Option<(PickerKind, String, Vec<PickItem>)>,
    Option<(Option<PathBuf>, usize, usize)>,
);

impl App {
    pub(super) fn base_place(&self) -> Option<FileLine> {
        let r = self.review.as_ref()?;
        let rel = self.rel_current()?;
        let file = r.file(&rel);
        let line1 = match (self.deleted, file) {
            (Some((k, i)), _) => self.diff.ghost_from.get(&k).copied().unwrap_or(0) + i + 1,
            (None, Some(f)) if f.status == 'D' => self.line + 1,
            _ => return None,
        };
        Some(FileLine {
            path: file.and_then(|f| f.old.clone()).unwrap_or(rel),
            line1,
        })
    }

    pub(super) fn definition_at_base(&mut self, path: &Path, line: usize) {
        if !self.flush() {
            return;
        }
        let Some(r) = self.review.clone() else {
            return;
        };
        let col = self.col;
        let node_modules_follow_the_file = |k: Kind| k == Kind::TsJs;
        let external_same_at_base: Vec<_> = (self.external.iter())
            .filter(|(k, _)| !node_modules_follow_the_file(**k))
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let Some(b) = self.base_app(&r) else {
            self.message = "no base to read".into();
            return;
        };
        for (kind, roots) in external_same_at_base {
            b.external.entry(kind).or_insert(roots);
        }
        b.picker = None;
        b.message.clear();
        b.jump_to(&b.root.join(path), line);
        b.col = col;
        let (message, picked, jumped) = b.answer_of_d();
        let base = b.root.clone();
        match (picked, jumped) {
            (Some((kind, title, items)), _) => {
                let items = items
                    .into_iter()
                    .map(|it| self.item_to_review(&r, &base, it))
                    .collect();
                self.show_picker(kind, items);
                if let Some(p) = &mut self.picker {
                    p.title = title;
                }
                self.message = message;
            }
            (None, Some((Some(to), line, col))) => {
                let mut hit = Hit {
                    path: to,
                    line1: line,
                    byte_col: Some(col),
                    text: String::new(),
                    deleted: None,
                };
                self.to_review(&r, &base, &mut hit);
                let path = self.root.join(&hit.path);
                match hit.deleted {
                    Some(_) => self.jump_to_deleted(&path, hit.line1, col),
                    None => self.jump_to_col(&path, hit.line1, col),
                }
                let jump_refused = self.buf.path.as_deref() != Some(path.as_path());
                if !jump_refused {
                    self.message = message;
                }
            }
            _ => self.message = message,
        }
    }

    pub(super) fn answer_of_d(&mut self) -> AnswerOfD {
        let before = (self.buf.path.clone(), self.line, self.col);
        self.goto_definition();
        let message = std::mem::take(&mut self.message);
        let picked = self.picker.take().map(|mut p| {
            p.settle();
            let kind = match self.mode {
                Mode::Picker(kind) => kind,
                _ => PickerKind::Definitions,
            };
            let items: Vec<PickItem> = p.window(usize::MAX).0.into_iter().map(|r| r.item).collect();
            (kind, p.title, items)
        });
        self.mode = Mode::Normal;
        let jumped = (before != (self.buf.path.clone(), self.line, self.col))
            .then(|| (self.buf.path.clone(), self.line + 1, self.col));
        (message, picked, jumped)
    }

    fn base_app(&mut self, r: &git::Review) -> Option<&mut Self> {
        let dir = r.base_tree(&self.root).ok()?;
        let branch_files_stamp = std::fs::read_to_string(dir.join(".merl-stamp")).ok()?;
        if (self.at_base.as_ref()).is_none_or(|(s, _)| *s != branch_files_stamp) {
            let (tree, files) = crate::tree::build(&dir, false);
            let app = Self::new(dir, tree, files, Buffer::empty(), None);
            self.at_base = Some((branch_files_stamp, Box::new(app)));
        }
        self.at_base.as_mut().map(|(_, a)| a.as_mut())
    }

    fn item_to_review(&self, r: &git::Review, base: &Path, mut it: PickItem) -> PickItem {
        let mut hit = Hit {
            path: it.path.clone(),
            line1: it.line,
            byte_col: None,
            text: String::new(),
            deleted: None,
        };
        self.to_review(r, base, &mut hit);
        let rel = |p: &Path| p.strip_prefix(base).unwrap_or(p).to_path_buf();
        let (was, now) = match it.line {
            0 => (
                rel(&it.path).display().to_string(),
                hit.path.display().to_string(),
            ),
            _ => (
                at_label(&rel(&it.path), it.line),
                at_label(&hit.path, hit.line1),
            ),
        };
        if let Some(at) = it.label.find(&was) {
            it.label.replace_range(at..at + was.len(), &now);
            if let Some(code) = &mut it.code_at
                && *code > at
            {
                *code = *code + now.len() - was.len();
            }
            if let Some(path) = &mut it.path_at
                && path.start == at
            {
                path.end = at + hit.path.display().to_string().len();
            }
        }
        (it.path, it.line, it.deleted) = (hit.path, hit.line1, hit.deleted.is_some());
        it
    }

    #[expect(
        clippy::suspicious_operation_groupings,
        reason = "a deleted line's `line` and a hit's `line1` are the same row, named apart"
    )]
    fn to_review(&self, r: &git::Review, base: &Path, h: &mut Hit) {
        if let Ok(rel) = h.path.strip_prefix(base) {
            h.path = rel.to_path_buf();
        }
        let file = (r.files.iter()).find(|f| f.old.as_deref().unwrap_or(&f.path) == h.path);
        let Some(file) = file.filter(|f| !f.untracked) else {
            return;
        };
        h.path = file.path.clone();
        if let Some(d) = (r.deleted.iter()).find(|d| d.path == h.path && d.line == h.line1) {
            match self.moved_copy(r, d) {
                Some((path, line)) => (h.path, h.line1) = (path, line),
                None => h.deleted = Some(d.at),
            }
            return;
        }
        let diff = r.diff(&self.root, &self.root.join(&file.path), Some(file));
        if let Some(now) = kept_line(&diff, h.line1) {
            h.line1 = now;
        }
    }

    fn moved_copy(&self, r: &git::Review, d: &git::DeletedLine) -> Option<(PathBuf, usize)> {
        let indent = |t: &str| t.len() - t.trim_start().len();
        let own = indent(&d.text);
        let mut block = vec![d.text.trim().to_owned()];
        for next in (r.deleted.iter()).filter(|n| n.path == d.path && n.line > d.line) {
            if next.line != d.line + block.len() {
                break;
            }
            let t = next.text.trim();
            if !t.is_empty() && indent(&next.text) <= own {
                if indent(&next.text) == own && t.starts_with(['}', ')', ']']) || t == "end" {
                    block.push(t.to_owned());
                }
                break;
            }
            block.push(t.to_owned());
        }
        while block.last().is_some_and(String::is_empty) {
            block.pop();
        }
        if block.len() < 3 {
            return None;
        }
        (r.added.iter())
            .filter(|a| a.len >= block.len())
            .find_map(|a| {
                let text = self.text_of(&a.path)?;
                let lines: Vec<&str> = text.lines().skip(a.line - 1).take(a.len).collect();
                let at = lines.windows(block.len()).position(|w| {
                    w.iter()
                        .map(|l| l.trim())
                        .eq(block.iter().map(String::as_str))
                })?;
                Some((a.path.clone(), a.line + at))
            })
    }

    pub(super) fn deleted_definitions(
        &self,
        kind: Kind,
        word: &str,
        here: &Path,
        cs_type: bool,
    ) -> Vec<Candidate> {
        let Some(r) = &self.review else {
            return Vec::new();
        };
        let patterns = if cs_type {
            search::cs_type_patterns(word)
        } else {
            let mut all = search::def_patterns_for(kind, word, self.c_dialect());
            self.spelling_cut(kind, word, &mut all);
            all.extend(search::member_patterns(kind, word).unwrap_or_default());
            all
        };
        let Ok(re) = Regex::new(&patterns.join("|")) else {
            return Vec::new();
        };
        let sight = self.cs_sight(kind, here);
        let hits = deleted_hits(
            &r.deleted,
            |p| self.in_def_scope(kind, here, p) && sight.as_ref().is_none_or(|s| s.sees(p)),
            |t| re.is_match(t).then_some(0),
        );
        let mut base: HashMap<PathBuf, (Vec<String>, Vec<bool>)> = HashMap::new();
        let mut found: Vec<Candidate> = hits
            .into_iter()
            .filter(|h| {
                let (lines, literal) = base.entry(h.path.clone()).or_insert_with(|| {
                    let text = self.hit_text(h).unwrap_or_default();
                    let literal = self.hidden_of(kind, h);
                    (text.lines().map(str::to_owned).collect(), literal)
                });
                !literal.get(h.line1 - 1).copied().unwrap_or(false)
                    && search::declares_where(kind, &h.path, word, h.line1, &h.text, || lines)
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        found.sort_by_cached_key(|c| search::rank(&c.hit.path, Some(here), true).tier);
        found.truncate(search::MAX_HITS);
        found
    }

    pub(super) fn on_drawn<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let Some((k, i)) = self.deleted else {
            return f(self);
        };
        let block = self.diff.ghosts.get(&k).cloned().unwrap_or_default();
        let m = block.len();
        let kept = (self.line, self.deleted, self.dirty);
        self.buf.lines.splice(k..k, block);
        (self.line, self.deleted, self.dirty) = (k + i, None, true);
        let r = f(self);
        self.buf.lines.drain(k..k + m);
        (self.line, self.deleted, self.dirty) = kept;
        r
    }
}

fn kept_line(diff: &git::Diff, base_line1: usize) -> Option<usize> {
    let mut base = 1;
    let last = diff
        .marks
        .keys()
        .chain(diff.ghosts.keys())
        .max()
        .copied()
        .unwrap_or(0);
    for f in 0.. {
        base += diff.ghosts.get(&f).map_or(0, Vec::len);
        if base > base_line1 {
            return None;
        }
        if f > last {
            return Some(f + 1 + base_line1 - base);
        }
        if !matches!(
            diff.marks.get(&f),
            Some(git::Mark::Added | git::Mark::Changed)
        ) {
            if base == base_line1 {
                return Some(f + 1);
            }
            base += 1;
        }
    }
    None
}
