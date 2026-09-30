//! `d` on a line the branch deleted (#440): the code a reviewer reads there is the base's, so the
//! word is looked up in the project as the base had it, by the same `d`, and what that finds is
//! shown where the review draws it now.

use super::*;
use search_job::deleted_hits;
use std::cell::OnceCell;

impl App {
    /// `d` on the deleted line `i` above file line `k`. A second `App` over the base's tree
    /// ([`git::Review::base_tree`]) presses `d` on that line as the base had it; a candidate the
    /// branch kept is shown on its line now, a deleted one on its red line, and one the branch
    /// moved unchanged on its live copy. The status is the lookup's own.
    pub(super) fn definition_at_base(&mut self, k: usize, i: usize) {
        let (Some(r), Some(rel)) = (self.review.clone(), self.rel_current()) else {
            return;
        };
        let old = r.file(&rel).and_then(|f| f.old.clone()).unwrap_or(rel);
        let line = self.diff.ghost_from.get(&k).copied().unwrap_or(0) + i + 1;
        let col = self.col;
        // The dependencies and the standard library are the same at the base.
        let external = self.external.clone();
        let Some(b) = self.base_app(&r) else {
            self.message = "no base to read".into();
            return;
        };
        for (kind, roots) in external {
            b.external.entry(kind).or_insert(roots);
        }
        (b.picker, b.last_definitions) = (None, None);
        b.message.clear();
        let path = b.root.join(&old);
        b.jump_to(&path, line);
        b.col = col;
        let before = (b.buf.path.clone(), b.line, b.col);
        b.goto_definition();
        let (message, title) = (b.message.clone(), b.picker.take().map(|p| p.title));
        let jumped = before != (b.buf.path.clone(), b.line, b.col);
        let base_root = b.root.clone();
        let Some((kind, word, found)) = b.last_definitions.take() else {
            self.message = message;
            return;
        };
        let moves = OnceCell::new();
        let found: Vec<Candidate> = found
            .into_iter()
            .map(|mut c| {
                self.to_review(&r, &base_root, &mut c.hit);
                if c.hit.deleted.is_some()
                    && let Some((path, line)) =
                        self.moved_copy(&r, &c.hit, moves.get_or_init(|| self.added_runs(&r)))
                {
                    (c.hit.path, c.hit.line, c.hit.deleted) = (path, line, None);
                    c.hit.text = self
                        .text_of(&c.hit.path)
                        .and_then(|t| t.lines().nth(line - 1).map(str::to_owned))
                        .unwrap_or_default();
                }
                c
            })
            .collect();
        match (title, found.as_slice()) {
            (Some(title), _) => {
                let items = self.definition_items(kind, &word, found);
                self.show_picker(PickerKind::Definitions, items);
                if let Some(p) = &mut self.picker {
                    p.title = title;
                }
            }
            (None, [one]) if jumped => {
                let path = self.root.join(&one.hit.path);
                let col = word_col(&one.hit.text, &word, search::word_chars(Some(kind), true));
                match one.hit.deleted {
                    Some(_) => self.jump_to_deleted(&path, one.hit.line, col),
                    None => self.jump_to_col(&path, one.hit.line, col),
                }
            }
            _ => {}
        }
        self.message = message;
    }

    /// The `App` over the base's tree, made on the first `d` of a review and again when the
    /// branch's files change.
    fn base_app(&mut self, r: &git::Review) -> Option<&mut App> {
        let dir = r.base_tree(&self.root).ok()?;
        let stale = self.base_app.as_ref().is_none_or(|(stamp, _)| {
            std::fs::read_to_string(dir.join(".merl-stamp"))
                .ok()
                .as_ref()
                != Some(stamp)
        });
        if stale {
            let stamp = std::fs::read_to_string(dir.join(".merl-stamp")).ok()?;
            let (tree, files) = crate::tree::build(&dir, false);
            let app = App::new(dir, tree, files, Buffer::empty(), None);
            self.base_app = Some((stamp, Box::new(app)));
        }
        self.base_app.as_mut().map(|(_, a)| a.as_mut())
    }

    /// `h`, a hit in the base's tree at `base`, as the review draws it: under the file's name in
    /// the review, on its red line when the branch deleted it, on its line now when the branch
    /// kept it. A hit outside the project (a dependency) is where it was.
    fn to_review(&self, r: &git::Review, base: &Path, h: &mut Hit) {
        if let Ok(rel) = h.path.strip_prefix(base) {
            h.path = rel.to_path_buf();
        }
        let file = (r.files.iter())
            .find(|f| !f.untracked && f.old.as_deref().unwrap_or(&f.path) == h.path);
        let Some(file) = file else { return };
        h.path = file.path.clone();
        if let Some(d) = r
            .deleted
            .iter()
            .find(|d| d.path == h.path && d.line == h.line)
        {
            h.deleted = Some(d.at);
            return;
        }
        let diff = r.diff(&self.root, &self.root.join(&file.path), Some(file));
        if let Some(now) = kept_line(&diff, h.line) {
            h.line = now;
        }
    }

    /// Where the definition the branch deleted at `h` lives now, when it was moved unchanged: an
    /// added block holding its lines, indentation aside, three at least.
    fn moved_copy(
        &self,
        r: &git::Review,
        h: &Hit,
        runs: &[(PathBuf, usize, Vec<String>)],
    ) -> Option<(PathBuf, usize)> {
        let mut block: Vec<&str> = Vec::new();
        for d in r
            .deleted
            .iter()
            .filter(|d| d.path == h.path && d.line >= h.line)
        {
            if d.line != h.line + block.len() {
                break;
            }
            block.push(d.text.trim());
        }
        while block.last() == Some(&"") {
            block.pop();
        }
        if block.len() < 3 {
            return None;
        }
        runs.iter().find_map(|(path, first, run)| {
            (0..=run.len().checked_sub(block.len())?)
                .find(|&w| {
                    run[w..w + block.len()]
                        .iter()
                        .map(|t| t.trim())
                        .eq(block.iter().copied())
                })
                .map(|w| (path.clone(), first + w))
        })
    }

    /// Every run of lines the branch added, file by file: the file, its first line, the lines.
    fn added_runs(&self, r: &git::Review) -> Vec<(PathBuf, usize, Vec<String>)> {
        let patch = git::branch_patch(&self.root, &r.merge_base).unwrap_or_default();
        let mut runs: Vec<(PathBuf, usize, Vec<String>)> = Vec::new();
        let (mut file, mut next) = (PathBuf::new(), 0);
        let mut run: Option<(usize, Vec<String>)> = None;
        let mut flush = |file: &Path, run: &mut Option<(usize, Vec<String>)>| {
            if let Some((first, lines)) = run.take() {
                runs.push((file.to_path_buf(), first, lines));
            }
        };
        for l in patch.lines() {
            if let Some(p) = l.strip_prefix("+++ b/") {
                flush(&file, &mut run);
                file = PathBuf::from(p);
            } else if l.starts_with("@@ ") {
                flush(&file, &mut run);
                next = l
                    .split(' ')
                    .nth(2)
                    .and_then(|s| s[1..].split(',').next()?.parse().ok())
                    .unwrap_or(0);
            } else if let Some(t) = l.strip_prefix('+') {
                run.get_or_insert_with(|| (next, Vec::new()))
                    .1
                    .push(t.to_owned());
                next += 1;
            }
        }
        flush(&file, &mut run);
        runs
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
        let mut patterns = search::def_patterns(kind, word);
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
                    let literal = search::literal_lines(kind, &text);
                    (text.lines().map(str::to_owned).collect(), literal)
                });
                !literal.get(h.line - 1).copied().unwrap_or(false)
                    && search::declares_where(kind, word, h.line, &h.text, || lines)
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        found.sort_by_cached_key(|c| search::rank(&c.hit.path, Some(here), true).0);
        found.truncate(search::MAX_HITS);
        found
    }

    /// `f` run with the cursor's deleted line read where the review draws it (#440): the lines
    /// the branch deleted there are put back into the open file above the line they were
    /// deleted from, the cursor on its own, as the file's text for everything `f` reads. The
    /// text is the file's again afterwards.
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
