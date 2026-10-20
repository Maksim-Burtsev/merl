use std::collections::BTreeMap;
use std::ops::Range;

use pulldown_cmark::Parser;
use serde::Deserialize;
use serde_json::value::RawValue;

use super::{Block, BlockLine, Doc, Fit, Ink, Lay, data::Parsed};

#[derive(Debug, PartialEq, Eq)]
pub struct Notebook {
    lines: Vec<String>,
    at: Vec<(usize, usize)>,
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
    cells: Vec<Cell<'a>>,
}

#[derive(Deserialize)]
struct Cell<'a> {
    cell_type: String,
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
            pieces: Vec::new(),
        },
    };
    for cell in &nb.cells {
        let src = b.text(cell.source, false);
        match cell.cell_type.as_str() {
            "markdown" if !src.is_empty() => b.nb.pieces.push(Piece::Markdown(src)),
            "markdown" => {}
            kind => {
                let src = b.non_empty(src, cell.source);
                let lang = if kind == "code" {
                    lang.clone()
                } else {
                    String::new()
                };
                b.nb.pieces.push(Piece::Code(lang, src));
                let first = b.nb.lines.len();
                for o in &cell.outputs {
                    b.output(o);
                }
                if b.nb.lines.len() > first {
                    b.nb.pieces.push(Piece::Output(first..b.nb.lines.len()));
                }
            }
        }
    }
    Ok(Parsed::Notebook(b.nb))
}

impl Build<'_> {
    fn pos(&self, raw: &str) -> (usize, usize) {
        let off = raw
            .as_ptr()
            .addr()
            .saturating_sub(self.json.as_ptr().addr());
        let l = self.starts.partition_point(|&s| s <= off).saturating_sub(1);
        (l, off - self.starts[l] + 1)
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
            let Ok(s) = serde_json::from_str::<String>(part.get()) else {
                continue;
            };
            let s = if traceback { strip_ansi(&s) } else { s };
            let at = self.pos(part.get());
            for piece in s.split_inclusive('\n') {
                let text = piece.strip_suffix('\n').unwrap_or(piece);
                match self.nb.lines.last_mut() {
                    Some(last) if open => last.push_str(text),
                    _ => self.line(text.to_owned(), at),
                }
                open = !piece.ends_with('\n');
            }
            if traceback {
                open = false;
            }
        }
        first..self.nb.lines.len()
    }

    fn line(&mut self, text: String, at: (usize, usize)) {
        self.nb.lines.push(text);
        self.nb.at.push(at);
    }

    fn non_empty(&mut self, r: Range<usize>, raw: &RawValue) -> Range<usize> {
        if !r.is_empty() {
            return r;
        }
        self.line(String::new(), self.pos(raw.get()));
        r.start..r.start + 1
    }

    fn output(&mut self, o: &Output) {
        if let Some(t) = o.text {
            self.text(t, false);
        }
        if let Some(t) = o.traceback {
            self.text(t, true);
        }
        if let Some((mime, raw)) = o.data.iter().find(|(m, _)| m.starts_with("image/")) {
            self.line(format!("\u{25a3} {mime}"), self.pos(raw.get()));
        } else if let Some(t) = o.data.get("text/plain") {
            self.text(t, false);
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        match c {
            '\u{1b}' => esc = true,
            c if esc => esc = !c.is_ascii_alphabetic(),
            c => out.push(c),
        }
    }
    out
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
    for piece in &nb.pieces {
        match piece {
            Piece::Markdown(r) => {
                let a = start(r.start);
                let b = starts.get(r.end).map_or(src.len(), |&s| s - 1);
                let cell = src.get(a..b).unwrap_or_default();
                for (ev, er) in Parser::new_ext(cell, super::options()).into_offset_iter() {
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
    let to = |l: usize| nb.at.get(l).copied().unwrap_or((n, 0));
    for row in &mut lay.rows {
        let (l, c) = row.src;
        let (jl, jc) = to(l);
        row.src = (
            jl,
            if c == usize::MAX {
                c
            } else {
                jc.saturating_add(c)
            },
        );
        row.lines = if row.lines.is_empty() {
            to(row.lines.start).0..to(row.lines.start).0
        } else {
            to(row.lines.start).0..to(row.lines.end - 1).0 + 1
        };
    }
    lay.done(n)
}

#[cfg(test)]
mod tests;
