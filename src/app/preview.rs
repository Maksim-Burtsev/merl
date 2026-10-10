use super::*;
use crate::markdown::{self, Ask, Doc, Kind, data::data};

use super::preview_data::{lines_hash, no_preview};

pub struct Preview {
    pub doc: Doc,
    laid_out_from: LayoutInput,
    pub row: usize,
    pub top: usize,
    at: Option<(usize, usize)>,
    pub code: Vec<Buffer>,
    pub theme: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct LayoutInput {
    lines_hash: u64,
    width: usize,
    diagrams_laid: u64,
    light: bool,
}

impl App {
    pub fn previewing(&self) -> bool {
        self.buf
            .path
            .as_ref()
            .is_some_and(|p| self.previewed.contains(p) && self.parse_ready(p))
    }

    pub fn picture_here(&self) -> Option<PathBuf> {
        let path = self.buf.path.as_ref()?;
        let raster = crate::picture::raster_name(path).is_some() && self.buf.binary();
        let svg = crate::picture::is_svg(path) && self.previewing();
        (raster || svg).then(|| self.root.join(path))
    }

    pub fn picture_shown(&self) -> Option<PathBuf> {
        let path = self.picture_here()?;
        (self.diagrams.on() && self.diagrams.file_failed(&path).is_none()).then_some(path)
    }

    pub(super) fn toggle_preview(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        let svg = crate::picture::is_svg(&path);
        if svg && self.diagrams.on() {
            if !self.previewed.remove(&path) {
                self.previewed.insert(path);
            }
            return;
        }
        if self.parse_toggle(&path) {
            return;
        }
        if self.previewed.remove(&path) {
            let off = self
                .preview
                .as_ref()
                .map_or(0, |p| p.row.saturating_sub(p.top));
            let cur = self.cursor_at();
            (self.top_line, self.top_row) = self.back_rows(cur, off);
            self.sync_want_x();
            return;
        }
        if !markdown::is_markdown(&path) && data(&path).is_none() {
            self.message = no_preview(&path);
            return;
        }
        self.preview_open();
    }

    pub(super) fn preview_open(&mut self) {
        let Some(path) = self.buf.path.clone() else {
            return;
        };
        let (top, cur) = ((self.top_line, self.top_row), self.cursor_at());
        let off = match cur < top {
            true => 0,
            false => self
                .rows_between(top, cur)
                .min(self.view_h.saturating_sub(1)),
        };
        self.previewed.insert(path);
        if self.deleted.is_some() {
            self.go((self.line, 0));
        }
        self.anchor = None;
        if self.preview_sync()
            && let Some(p) = &mut self.preview
        {
            p.top = p.row.saturating_sub(off);
        }
        self.preview_clamp();
    }

    pub(crate) fn preview_sync(&mut self) -> bool {
        let lines_hash = lines_hash(&self.buf.lines);
        if self.parse_stale(lines_hash) {
            return false;
        }
        let input = LayoutInput {
            lines_hash,
            width: self.view_w,
            diagrams_laid: self.diagrams.laid,
            light: self.diagrams.light,
        };
        let pos = (self.line, self.col);
        match &mut self.preview {
            Some(p) if p.laid_out_from == input => {
                if p.at == Some(pos) {
                    return false;
                }
                p.row = p.doc.row_at(pos);
                p.at = Some(pos);
            }
            slot => {
                let old = slot.take();
                let off = old.as_ref().map_or(0, |p| p.row.saturating_sub(p.top));
                let diagrams = &mut self.diagrams;
                let file = self
                    .root
                    .join(self.buf.path.as_deref().unwrap_or(Path::new("")));
                let root = &self.root;
                let kind = data(&file);
                let parsed = self.parse.done.as_ref().map(|d| &d.2);
                let mut fit = |ask: Ask, room| match ask {
                    Ask::Diagram(src) => diagrams.fit(src, room),
                    Ask::Image(dest, width) => {
                        let path = crate::picture::resolve(root, &file, dest)?;
                        let natural = diagrams.file_size(&path)?;
                        let cell = diagrams.cell()?;
                        Some(crate::picture::cells(natural, width, cell, room, None))
                    }
                };
                let lines = &self.buf.lines;
                let doc = match kind {
                    Some(k) => k.layout(parsed, lines, self.view_w, &mut fit),
                    None => markdown::layout(lines, self.view_w, input.light, &mut fit),
                };
                let row = match &old {
                    Some(p) if p.at == Some(pos) => doc.same_row(&p.doc, p.row, pos),
                    _ => doc.row_at(pos),
                };
                let (code, theme) = match old {
                    Some(p)
                        if p.laid_out_from.lines_hash == input.lines_hash
                            && same_blocks(&p.doc, &doc) =>
                    {
                        (p.code, p.theme)
                    }
                    _ => (
                        doc.code
                            .iter()
                            .map(|c| Buffer::block(&c.lang, c.lines.clone()))
                            .collect(),
                        String::new(),
                    ),
                };
                *slot = Some(Preview {
                    doc,
                    laid_out_from: input,
                    row,
                    top: row.saturating_sub(off),
                    at: Some(pos),
                    code,
                    theme,
                });
            }
        }
        true
    }

    pub(crate) fn preview_clamp(&mut self) {
        let h = self.view_h.max(1);
        let center = std::mem::take(&mut self.center);
        let Some(p) = &mut self.preview else {
            return;
        };
        let last = p.doc.rows.len() - 1;
        p.row = p.row.min(last);
        if center {
            p.top = p.row.saturating_sub(h / 2);
        }
        if p.row < p.top {
            p.top = p.row;
        }
        if p.row >= p.top + h {
            p.top = p.row + 1 - h;
        }
        p.top = p.top.min((last + 1).saturating_sub(h));
    }

    pub(super) fn preview_key(&mut self, key: KeyEvent) -> bool {
        if !self.previewing() {
            let walk = self.review.is_some() && matches!(key.code, KeyCode::Char('c' | 'C'));
            if walk && self.preview_pending() {
                self.toggle_preview();
            }
            return false;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let plain = key.modifiers.is_empty();
        let tree = matches!(
            key.code,
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right | KeyCode::Enter
        );
        if self.review.is_some() && matches!(key.code, KeyCode::Char('c' | 'C')) {
            self.toggle_preview();
            return false;
        }
        let elsewhere = match key.code {
            KeyCode::Esc | KeyCode::Tab => true,
            KeyCode::Char(c) if ctrl => matches!(c, 'g' | 'n' | 'e' | 's' | 'r' | 'z' | 'y'),
            KeyCode::Char(c) => "q?:sDotT[]cCmp".contains(c),
            _ => false,
        };
        if elsewhere || tree && self.focus == Focus::Tree {
            return false;
        }
        if self.buf.path.as_deref().is_some_and(crate::picture::is_svg) {
            if self.picture_shown().is_none() {
                return false;
            }
            if key.code == KeyCode::Enter && plain {
                self.start_edit();
            }
            return true;
        }
        self.preview_sync();
        let Some(p) = &self.preview else {
            return false;
        };
        let (row, top, last) = (p.row, p.top, p.doc.rows.len() - 1);
        let (h, half) = (self.view_h.max(1), (self.view_h / 2).max(1));
        let gap = |i: &usize| p.doc.rows[*i].kind == Kind::Gap;
        let (to, scroll, reads) = match key.code {
            KeyCode::Up if plain => (row.saturating_sub(1), 0, true),
            KeyCode::Down if plain => ((row + 1).min(last), 0, true),
            KeyCode::PageUp if plain => (row.saturating_sub(h), 0, true),
            KeyCode::PageDown if plain => ((row + h).min(last), 0, true),
            KeyCode::Char('u') if ctrl => (row.saturating_sub(half), -1, true),
            KeyCode::Char('d') if ctrl => ((row + half).min(last), 1, true),
            KeyCode::Home if ctrl => (0, 0, false),
            KeyCode::End if ctrl => (last, 0, false),
            KeyCode::Char('{') => ((0..row).rev().find(gap).unwrap_or(0), 0, false),
            KeyCode::Char('}') => ((row + 1..=last).find(gap).unwrap_or(last), 0, false),
            KeyCode::Enter if plain => {
                self.start_edit();
                return true;
            }
            _ => return true,
        };
        let moved = to.abs_diff(row);
        let top = match scroll {
            -1 => top.saturating_sub(moved),
            1 => top + moved,
            _ => top,
        };
        let rows = &p.doc.rows;
        let src = match rows[to].lines.is_empty() {
            false => rows[to].src,
            true => rows[to..]
                .iter()
                .find(|r| !r.lines.is_empty())
                .map_or((usize::MAX, usize::MAX), |r| (r.lines.start, 0)),
        };
        self.go(self.clamp_pos(src));
        let at = (self.line, self.col);
        if let Some(p) = &mut self.preview {
            (p.row, p.top, p.at) = (to, top, Some(at));
        }
        if reads && let (Some(pos), Some(cur)) = (self.pos(), self.history.get_mut(self.hist_idx)) {
            *cur = pos;
        }
        self.preview_clamp();
        true
    }

    pub(super) fn preview_blank(&self) -> bool {
        let pos = Some((self.line, self.col));
        let p = self.preview.as_ref().filter(|p| p.at == pos);
        self.previewing() && p.is_some_and(|p| p.doc.rows[p.row].lines.is_empty())
    }

    pub(super) fn preview_jumped(&mut self) {
        if let Some(p) = &mut self.preview {
            p.at = None;
        }
    }
}

fn same_blocks(old: &Doc, new: &Doc) -> bool {
    let block = |c: &markdown::Code| (c.lang.clone(), c.lines.clone());
    old.code.iter().map(block).eq(new.code.iter().map(block))
}
