use std::collections::BTreeMap;
use std::ops::Range;

use pulldown_cmark::Parser;
use serde::Deserialize;
use serde_json::value::RawValue;

use super::{Block, BlockLine, Doc, Fit, Ink, Lay, Row, data::Parsed};

#[derive(Debug, PartialEq, Eq)]
pub struct Notebook {
    lines: Vec<String>,
    at: Vec<At>,
    cells: Vec<NbCell>,
}

#[derive(Debug, PartialEq, Eq)]
struct At {
    line: usize,
    cols: Vec<(usize, usize)>,
}

impl At {
    fn col(&self, c: usize) -> usize {
        let k = self
            .cols
            .partition_point(|&(d, _)| d <= c)
            .saturating_sub(1);
        let (d, r) = self.cols.get(k).copied().unwrap_or_default();
        r + c.saturating_sub(d)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct NbCell {
    json: Range<usize>,
    pieces: Vec<Piece>,
}

#[derive(Debug, PartialEq, Eq)]
enum Piece {
    Markdown(Range<usize>),
    Code(String, Range<usize>),
    Output(Range<usize>),
}

#[derive(Deserialize)]
struct Nb<'a> {
    nbformat: Option<u64>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(borrow, default)]
    cells: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
struct Cell<'a> {
    #[serde(rename = "cell_type")]
    kind: String,
    #[serde(borrow)]
    source: &'a RawValue,
    #[serde(borrow, default)]
    outputs: Vec<Output<'a>>,
}

#[derive(Deserialize)]
struct Output<'a> {
    #[serde(borrow, default)]
    text: Option<&'a RawValue>,
    #[serde(borrow, default)]
    traceback: Option<&'a RawValue>,
    #[serde(borrow, default)]
    data: BTreeMap<String, &'a RawValue>,
}

struct Build<'a> {
    json: &'a str,
    starts: Vec<usize>,
    nb: Notebook,
}

pub fn parse(lines: &[String]) -> Result<Parsed, String> {
    let json = lines.join("\n");
    let nb: Nb = serde_json::from_str(&json).map_err(|e| {
        if e.is_data() {
            format!("line {} is not nbformat 4", e.line())
        } else {
            format!("line {} is not JSON", e.line())
        }
    })?;
    match nb.nbformat {
        Some(4) => {}
        Some(n) => return Err(format!("nbformat {n} is not 4")),
        None => return Err("no nbformat".into()),
    }
    let meta = nb.metadata.unwrap_or_default();
    let lang = meta["language_info"]["name"]
        .as_str()
        .or_else(|| meta["kernelspec"]["language"].as_str())
        .unwrap_or("python")
        .to_owned();
    let mut b = Build {
        json: &json,
        starts: std::iter::once(0)
            .chain(json.match_indices('\n').map(|(i, _)| i + 1))
            .collect(),
        nb: Notebook {
            lines: Vec::new(),
            at: Vec::new(),
            cells: Vec::new(),
        },
    };
    for raw in nb.cells {
        let (first, _) = b.line_col(b.offset(raw.get()));
        let cell: Cell = serde_json::from_str(raw.get())
            .map_err(|e| format!("line {} is not nbformat 4", first + e.line()))?;
        let end = b.offset(raw.get()) + raw.get().len().saturating_sub(1);
        let json = first..b.line_col(end).0 + 1;
        let pieces = b.cell(&cell, &lang);
        b.nb.cells.push(NbCell { json, pieces });
    }
    Ok(Parsed::Notebook(b.nb))
}

impl Build<'_> {
    fn offset(&self, raw: &str) -> usize {
        raw.as_ptr()
            .addr()
            .saturating_sub(self.json.as_ptr().addr())
    }

    fn line_col(&self, off: usize) -> (usize, usize) {
        let l = self.starts.partition_point(|&s| s <= off).saturating_sub(1);
        (l, off - self.starts[l])
    }

    fn cell(&mut self, cell: &Cell, lang: &str) -> Vec<Piece> {
        let src = self.text(cell.source, false);
        match cell.kind.as_str() {
            "markdown" if src.is_empty() => Vec::new(),
            "markdown" => vec![Piece::Markdown(src)],
            kind => {
                let src = if src.is_empty() {
                    self.line(String::new(), cell.source.get())
                } else {
                    src
                };
                let lang = if kind == "code" { lang } else { "" };
                let mut pieces = vec![Piece::Code(lang.to_owned(), src)];
                let first = self.nb.lines.len();
                for o in &cell.outputs {
                    self.output(o);
                }
                if self.nb.lines.len() > first {
                    pieces.push(Piece::Output(first..self.nb.lines.len()));
                }
                pieces
            }
        }
    }

    fn text(&mut self, raw: &RawValue, traceback: bool) -> Range<usize> {
        let first = self.nb.lines.len();
        let parts: Vec<&RawValue> = if raw.get().starts_with('[') {
            serde_json::from_str(raw.get()).unwrap_or_default()
        } else {
            vec![raw]
        };
        let mut open = false;
        for part in parts {
            self.part(part.get(), traceback, &mut open);
            if traceback {
                open = false;
            }
        }
        first..self.nb.lines.len()
    }

    fn part(&mut self, raw: &str, traceback: bool, open: &mut bool) {
        if !raw.starts_with('"') {
            return;
        }
        let (line, quote) = self.line_col(self.offset(raw));
        let end = raw.len().saturating_sub(1);
        let (mut i, mut ansi) = (1, false);
        while i < end {
            let rest = raw.get(i..).unwrap_or_default();
            let (c, len) = if rest.starts_with('\\') {
                unescape(rest)
            } else {
                rest.chars()
                    .next()
                    .map_or(('\u{fffd}', 1), |c| (c, c.len_utf8()))
            };
            let col = quote + i;
            i += len;
            if c == '\n' {
                if !*open {
                    self.start(line, col);
                }
                *open = false;
                continue;
            }
            if traceback && (ansi || c == '\u{1b}') {
                ansi = c == '\u{1b}' || !c.is_ascii_alphabetic();
                continue;
            }
            if !*open {
                self.start(line, col);
                *open = true;
            }
            self.push(c, line, col);
        }
    }

    fn start(&mut self, line: usize, col: usize) {
        self.nb.lines.push(String::new());
        self.nb.at.push(At {
            line,
            cols: vec![(0, col)],
        });
    }

    fn push(&mut self, c: char, line: usize, col: usize) {
        let (Some(text), Some(at)) = (self.nb.lines.last_mut(), self.nb.at.last_mut()) else {
            return;
        };
        let d = text.len();
        if at.line == line && at.col(d) != col {
            at.cols.push((d, col));
        }
        text.push(c);
    }

    fn line(&mut self, text: String, raw: &str) -> Range<usize> {
        let (l, c) = self.line_col(self.offset(raw));
        self.start(l, c);
        if let Some(last) = self.nb.lines.last_mut() {
            *last = text;
        }
        self.nb.lines.len() - 1..self.nb.lines.len()
    }

    fn output(&mut self, o: &Output) {
        if let Some(t) = o.text {
            self.text(t, false);
        }
        if let Some(t) = o.traceback {
            self.text(t, true);
        }
        if let Some((mime, raw)) = o.data.iter().find(|(m, _)| m.starts_with("image/")) {
            self.line(format!("\u{25a3} {mime}"), raw.get());
        } else if let Some(t) = o.data.get("text/plain") {
            self.text(t, false);
        }
    }
}

fn unescape(s: &str) -> (char, usize) {
    let hex = |k: usize| {
        s.get(k..k + 4)
            .and_then(|h| u32::from_str_radix(h, 16).ok())
    };
    match s.as_bytes().get(1) {
        Some(b'u') => {
            let Some(hi) = hex(2) else {
                return ('\u{fffd}', 2);
            };
            if (0xd800..0xdc00).contains(&hi)
                && s.get(6..8) == Some("\\u")
                && let Some(lo) = hex(8).filter(|lo| (0xdc00..0xe000).contains(lo))
            {
                let c = char::from_u32(0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00));
                return (c.unwrap_or('\u{fffd}'), 12);
            }
            (char::from_u32(hi).unwrap_or('\u{fffd}'), 6)
        }
        Some(b'n') => ('\n', 2),
        Some(b't') => ('\t', 2),
        Some(b'r') => ('\r', 2),
        Some(b'b') => ('\u{8}', 2),
        Some(b'f') => ('\u{c}', 2),
        Some(&c) => (char::from(c), 2),
        None => ('\\', 1),
    }
}

pub fn layout(nb: &Notebook, n: usize, width: usize, light: bool, fit: Fit) -> Doc {
    let src = nb.lines.join("\n");
    let mut starts = vec![0];
    starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
    let start = |l: usize| starts.get(l).copied().unwrap_or(src.len());
    let mut lay = Lay {
        src: &src,
        line_starts: starts.clone(),
        width: width.max(1),
        fit: Some(fit),
        light,
        ..Default::default()
    };
    let mut heads = Vec::with_capacity(nb.cells.len());
    for cell in &nb.cells {
        heads.push((cell.json.start, lay.rows.len()));
        for piece in &cell.pieces {
            match piece {
                Piece::Markdown(r) => {
                    let a = start(r.start);
                    let b = starts.get(r.end).map_or(src.len(), |&s| s - 1);
                    let text = src.get(a..b).unwrap_or_default();
                    for (ev, er) in Parser::new_ext(text, super::options()).into_offset_iter() {
                        lay.event(ev, er.start + a..er.end + a);
                    }
                    lay.flush();
                }
                Piece::Code(lang, r) => {
                    lay.block_start(start(r.start));
                    lay.code_block(Block {
                        info_first_word: lang.clone(),
                        lines: r
                            .clone()
                            .map(|l| BlockLine {
                                as_written: nb.lines[l].clone(),
                                src_line_col: (l, 0),
                                tab_spaces_parser_added: 0,
                            })
                            .collect(),
                        last_line_unended: false,
                        span: r.clone(),
                    });
                }
                Piece::Output(r) => {
                    lay.block_start(start(r.start));
                    for l in r.clone() {
                        lay.lines(&nb.lines[l], Ink::Dim.plain(), (l, 0));
                    }
                }
            }
            lay.gap_before_next_block = true;
        }
    }
    let line = |l: usize| nb.at.get(l).map_or(n, |a| a.line);
    for row in &mut lay.rows {
        let (l, c) = row.src;
        row.src = match nb.at.get(l) {
            Some(_) if c == usize::MAX => (line(l), c),
            Some(at) => (at.line, at.col(c)),
            None => (n, 0),
        };
        row.lines = if row.lines.is_empty() {
            line(row.lines.start)..line(row.lines.start)
        } else {
            let (a, b) = (line(row.lines.start), line(row.lines.end - 1));
            a.min(b)..a.max(b) + 1
        };
    }
    let mut doc = lay.done(0);
    own(&mut doc.rows, &heads, n);
    doc
}

fn own(rows: &mut [Row], heads: &[(usize, usize)], n: usize) {
    let mut covered = vec![false; n];
    for r in rows.iter() {
        for l in r.lines.clone().filter(|&l| l < n) {
            covered[l] = true;
        }
    }
    let ends = heads.iter().skip(1).map(|h| h.1).chain([rows.len()]);
    let head: Vec<Option<usize>> = heads
        .iter()
        .zip(ends)
        .map(|(&(_, a), b)| (a..b).find(|&i| !rows[i].lines.is_empty()))
        .collect();
    for l in (0..n).filter(|&l| !covered[l]) {
        let k = heads.partition_point(|h| h.0 <= l);
        let i = head[..k].iter().rev().find_map(|h| *h).unwrap_or(0);
        rows[i].owns.push(l);
    }
}

#[cfg(test)]
mod tests;
