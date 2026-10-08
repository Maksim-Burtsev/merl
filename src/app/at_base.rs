use super::*;
use search_job::deleted_hits;

impl App {
    /// Where the cursor stands in the base's text when the line it reads is base code: a line
    /// the branch deleted, or any line of a file the branch deleted. `None` elsewhere.
    pub(super) fn base_place(&self) -> Option<(PathBuf, usize)> {
        let r = self.review.as_ref()?;
        let rel = self.rel_current()?;
        let file = r.file(&rel);
        let line = match (self.deleted, file) {
            (Some((k, i)), _) => self.diff.ghost_from.get(&k).copied().unwrap_or(0) + i + 1,
            (None, Some(f)) if f.status == 'D' => self.line + 1,
            _ => return None,
        };
        Some((file.and_then(|f| f.old.clone()).unwrap_or(rel), line))
    }

    pub(super) fn definition_at_base(&mut self, path: PathBuf, line: usize) {
        // The answer is mapped onto the files as saved: a buffer ahead of its file is saved
        // first, and one that cannot be leaves its reason on the status bar, as a jump does.
        if !self.flush() {
            return;
        }
        let Some(r) = self.review.clone() else {
            return;
        };
        let col = self.col;
        // The standard library and the dependencies are the same at the base; TypeScript's
        // `node_modules` follow the open file, and the base's `App` finds its own.
        let external: Vec<_> = (self.external.iter())
            .filter(|(k, _)| **k != Kind::TsJs)
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let Some(b) = self.base_app(&r) else {
            self.message = "no base to read".into();
            return;
        };
        for (kind, roots) in external {
            b.external.entry(kind).or_insert(roots);
        }
        b.picker = None;
        b.message.clear();
        b.jump_to(&b.root.join(&path), line);
        b.col = col;
        let before = (b.buf.path.clone(), b.line, b.col);
        b.goto_definition();
        let message = std::mem::take(&mut b.message);
        let base = b.root.clone();
        let picked = b.picker.take().map(|mut p| {
            p.settle();
            let kind = match b.mode {
                Mode::Picker(kind) => kind,
                _ => PickerKind::Definitions,
            };
            let items: Vec<PickItem> = p
                .window(usize::MAX)
                .0
                .into_iter()
                .map(|r| r.item.clone())
                .collect();
            (kind, p.title, items)
        });
        b.mode = Mode::Normal;
        let jumped = (before != (b.buf.path.clone(), b.line, b.col))
            .then(|| (b.buf.path.clone(), b.line + 1, b.col));
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
                    line,
                    col,
                    text: String::new(),
                    deleted: None,
                };
                self.to_review(&r, &base, &mut hit);
                let path = self.root.join(&hit.path);
                match hit.deleted {
                    Some(_) => self.jump_to_deleted(&path, hit.line, col),
                    None => self.jump_to_col(&path, hit.line, col),
                }
                // A refused jump leaves its own reason, not a resolution nobody followed.
                if self.buf.path.as_deref() == Some(path.as_path()) {
                    self.message = message;
                }
            }
            _ => self.message = message,
        }
    }

    /// The `App` over the base's tree, made on the first `d` of a review and again when the
    /// branch's files change.
    fn base_app(&mut self, r: &git::Review) -> Option<&mut App> {
        let dir = r.base_tree(&self.root).ok()?;
        let stamp = std::fs::read_to_string(dir.join(".merl-stamp")).ok()?;
        if self.base_app.as_ref().is_none_or(|(s, _)| *s != stamp) {
            let (tree, files) = crate::tree::build(&dir, false);
            let app = App::new(dir, tree, files, Buffer::empty(), None);
            self.base_app = Some((stamp, Box::new(app)));
        }
        self.base_app.as_mut().map(|(_, a)| a.as_mut())
    }

    /// A row of the base's picker, re-addressed as the review draws it: its label names the
    /// place it lands on now.
    fn item_to_review(&self, r: &git::Review, base: &Path, mut it: PickItem) -> PickItem {
        let mut hit = Hit {
            path: it.path.clone(),
            line: it.line,
            col: it.col,
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
                at_label(&hit.path, hit.line),
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
        (it.path, it.line, it.deleted) = (hit.path, hit.line, hit.deleted.is_some());
        it
    }

    /// `h`, a place in the base's tree at `base`, as the review draws it: under the file's name
    /// in the review, on its red line when the branch deleted it, on its line now when the
    /// branch kept it, on the live copy of a definition the branch moved unchanged. A place
    /// outside the project (a dependency) is where it was.
    fn to_review(&self, r: &git::Review, base: &Path, h: &mut Hit) {
        if let Ok(rel) = h.path.strip_prefix(base) {
            h.path = rel.to_path_buf();
        }
        let file = (r.files.iter()).find(|f| f.old.as_deref().unwrap_or(&f.path) == h.path);
        let Some(file) = file.filter(|f| !f.untracked) else {
            return;
        };
        h.path = file.path.clone();
        if let Some(d) = (r.deleted.iter()).find(|d| d.path == h.path && d.line == h.line) {
            match self.moved_copy(r, d) {
                Some((path, line)) => (h.path, h.line) = (path, line),
                None => h.deleted = Some(d.at),
            }
            return;
        }
        let diff = r.diff(&self.root, &self.root.join(&file.path), Some(file));
        if let Some(now) = kept_line(&diff, h.line) {
            h.line = now;
        }
    }

    /// Where the definition the branch deleted at `d` lives now, when the branch moved it
    /// unchanged: a run the branch added holds its lines, indentation aside. The definition is
    /// its first line and those indented under it, a closing line at its own indentation
    /// included; three lines at least, or two blocks alike would be taken for one.
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
                // `}` or `end` closes the definition; anything else begins the next one.
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

    /// The lines the branch deleted that declare `word` where a definition of it in `here` can
    /// live, found by name (#440): a string or a comment of the file at the base declares
    /// nothing, as in the branch. Nothing outside a review.
    pub(super) fn deleted_definitions(
        &self,
        kind: Kind,
        word: &str,
        here: &Path,
    ) -> Vec<Candidate> {
        let Some(r) = &self.review else {
            return Vec::new();
        };
        let mut patterns = search::def_patterns_for(kind, word, self.objc_file());
        self.spelling_cut(kind, word, &mut patterns);
        patterns.extend(search::member_patterns(kind, word).unwrap_or_default());
        let Ok(re) = Regex::new(&patterns.join("|")) else {
            return Vec::new();
        };
        let hits = deleted_hits(
            &r.deleted,
            |p| search::in_def_scope(kind, here, p),
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
                !literal.get(h.line - 1).copied().unwrap_or(false)
                    && search::declares_where(kind, &h.path, word, h.line, &h.text, || lines)
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
        // Dirty, so the grep reads the open file as it is here, not from the disk.
        (self.line, self.deleted, self.dirty) = (k + i, None, true);
        let r = f(self);
        self.buf.lines.drain(k..k + m);
        (self.line, self.deleted, self.dirty) = kept;
        r
    }
}

/// The 1-based line of the file now that was line `line` of the file at the base, `None` when
/// the branch deleted it. Walks the file's lines: the deleted ones drawn above each take base
/// lines, an added or changed one takes none.
fn kept_line(diff: &git::Diff, line: usize) -> Option<usize> {
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
        if base > line {
            return None;
        }
        if f > last {
            return Some(f + 1 + line - base);
        }
        if !matches!(
            diff.marks.get(&f),
            Some(git::Mark::Added | git::Mark::Changed)
        ) {
            if base == line {
                return Some(f + 1);
            }
            base += 1;
        }
    }
    None
}
