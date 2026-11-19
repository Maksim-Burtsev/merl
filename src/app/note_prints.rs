use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::App;
use super::notes::{Note, Seen, note_id, place_of, reads};
use crate::git::{self, TextLine};

pub fn prints_path(notes: &Path) -> PathBuf {
    let mut p = notes.as_os_str().to_owned();
    p.push(".blobs");
    p.into()
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        match (esc, c) {
            (false, '\\') => esc = true,
            (true, 'n') => {
                out.push('\n');
                esc = false;
            }
            (_, c) => {
                out.push(c);
                esc = false;
            }
        }
    }
    out
}

pub fn parse_prints(text: &str) -> impl Iterator<Item = (String, String)> {
    text.lines().filter_map(|l| {
        let (blob, id) = l.split_once(' ')?;
        if !blob.is_empty() && blob.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some((unescape(id), blob.to_owned()));
        }
        let (at, blob) = l.rsplit_once(' ')?;
        Some((at.to_owned(), blob.to_owned()))
    })
}

type Diffs = HashMap<(String, String), Option<git::Diff>>;

fn diff_of<'a>(root: &Path, diffs: &'a mut Diffs, from: &str, to: &str) -> Option<&'a git::Diff> {
    (diffs.entry((from.to_owned(), to.to_owned())))
        .or_insert_with(|| git::diff_objects(root, from, to))
        .as_ref()
}

fn old_to_new(d: &git::Diff, old: usize, len: usize) -> Option<TextLine> {
    let mut o = 0;
    for l in 0..=len {
        if let Some(g) = d.ghosts.get(&l) {
            if (o..o + g.len()).contains(&old) {
                return Some(TextLine::Deleted(l, old - o));
            }
            o += g.len();
        }
        if l == len {
            break;
        }
        if d.marks.get(&l) == Some(&git::Mark::Added) {
            continue;
        }
        if o == old {
            return Some(TextLine::File(l));
        }
        o += 1;
    }
    None
}

fn new_to_old(d: &git::Diff, l: usize) -> Option<usize> {
    if d.marks.get(&l) == Some(&git::Mark::Added) {
        return None;
    }
    let del: usize = d.ghosts.range(..=l).map(|(_, g)| g.len()).sum();
    let add = (d.marks.iter())
        .filter(|&(&k, &m)| k < l && m == git::Mark::Added)
        .count();
    (l + del).checked_sub(add)
}

impl App {
    pub(super) fn print_of(&self, n: &Note) -> Option<&String> {
        let at = place_of(n);
        let alone = || self.notes.list.iter().filter(|m| place_of(m) == at).count() == 1;
        (self.notes.prints.get(&note_id(n)))
            .or_else(|| self.notes.prints.get(&at).filter(|_| alone()))
    }

    pub(super) fn prints_text(&self) -> String {
        (self.notes.list.iter())
            .filter_map(|n| Some(format!("{} {}\n", self.print_of(n)?, escape(&note_id(n)))))
            .collect()
    }

    pub(super) fn print_note(&mut self, n: &Note) {
        if !n.quote.last().is_some_and(|q| q.starts_with("> ")) {
            return;
        }
        if let Some(blob) = git::hash_object(&self.root, &self.buf.to_bytes()) {
            self.notes.prints.insert(note_id(n), blob);
        }
    }

    pub(super) fn print_places(&mut self) {
        if self.dirty {
            return;
        }
        let Some(rel) = self.rel_current() else {
            return;
        };
        let todo: Vec<(String, String, Note)> = (self.notes.list.iter())
            .filter(|n| n.path == rel)
            .filter_map(|n| {
                let id = note_id(n);
                let blob = self.print_of(n)?;
                let known = self.notes.printed.get(&id).is_some_and(|p| p.0 == *blob);
                (!known).then(|| (id, blob.clone(), n.clone()))
            })
            .collect();
        if todo.is_empty() {
            return;
        }
        let now = git::hash_object(&self.root, &self.buf.to_bytes());
        let mut diffs = Diffs::new();
        for (id, then, n) in todo {
            let at = (now.as_deref()).and_then(|now| self.print_line(&n, &then, now, &mut diffs));
            let place = at.and_then(|at| self.print_place(&n, &then, at, &mut diffs));
            self.notes.printed.insert(id, (then, at.is_some(), place));
        }
    }

    fn print_line(&self, n: &Note, then: &str, now: &str, diffs: &mut Diffs) -> Option<TextLine> {
        let text = n.quote.last()?.strip_prefix("> ")?;
        let line = n.last.checked_sub(1)?;
        let moved = if then == now {
            None
        } else {
            Some(diff_of(&self.root, diffs, then, now)?)
        };
        let at = match moved {
            None => TextLine::File(line),
            Some(d) => old_to_new(d, line, self.buf.lines.len())?,
        };
        let was = match (at, moved) {
            (TextLine::File(l), _) => self.buf.lines.get(l),
            (TextLine::Deleted(k, i), Some(d)) => d.ghosts.get(&k).and_then(|g| g.get(i)),
            (TextLine::Deleted(..), None) => None,
        };
        (was.map(String::as_str) == Some(text)).then_some(at)
    }

    fn print_place(&self, n: &Note, then: &str, at: TextLine, diffs: &mut Diffs) -> Option<Seen> {
        if let TextLine::File(l) = at
            && reads(self, at, &n.quote) == Some(true)
        {
            return Some((at, false, new_to_old(&self.diff, l)));
        }
        let base = self.base_line(n, then, n.last.checked_sub(1)?, diffs);
        let ghost = base.and_then(|b| {
            (self.diff.ghost_from.iter()).find_map(|(&k, &from)| {
                let i = b.checked_sub(from)?;
                (i < self.diff.ghosts.get(&k)?.len()).then_some(TextLine::Deleted(k, i))
            })
        });
        let ghost = ghost.filter(|&t| reads(self, t, &n.quote).is_some());
        ghost.map(|t| (t, true, base))
    }

    fn base_line(&self, n: &Note, then: &str, line: usize, diffs: &mut Diffs) -> Option<usize> {
        let r = self.review.as_ref()?;
        let from = (r.file(&n.path).and_then(|f| f.old.as_deref())).unwrap_or(&n.path);
        let base = format!("{}:{}", r.merge_base, from.display());
        new_to_old(diff_of(&self.root, diffs, &base, then)?, line)
    }
}
