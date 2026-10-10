use std::path::Path;

use super::{Block, BlockLine, Doc, Fit, Ink, Inline, Kind, Lay, Table, TableRow};

pub const THREAD: &str = "parse";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Data {
    Csv,
    Tsv,
    Jsonl,
    Mermaid,
}

pub fn data(path: &Path) -> Option<Data> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "csv" => Data::Csv,
        "tsv" => Data::Tsv,
        "jsonl" | "ndjson" => Data::Jsonl,
        "mmd" | "mermaid" => Data::Mermaid,
        _ => return None,
    })
}

pub fn parses(path: &Path) -> bool {
    data(path).is_some_and(|d| d != Data::Mermaid)
}

#[derive(Debug, PartialEq)]
pub enum Parsed {
    Table(Vec<(usize, Vec<String>)>),
    Records(Vec<Vec<(String, (usize, usize))>>),
}

pub fn parse(data: Data, lines: &[String]) -> Result<Parsed, String> {
    match data {
        Data::Csv => csv(lines).and_then(same_width),
        Data::Tsv => same_width(tsv(lines)),
        Data::Jsonl => jsonl(lines),
        Data::Mermaid => Err("nothing to parse".into()),
    }
}

fn empty(lines: &[String]) -> bool {
    lines.len() == 1 && lines[0].is_empty()
}

fn tsv(lines: &[String]) -> Vec<(usize, Vec<String>)> {
    if empty(lines) {
        return Vec::new();
    }
    let cells = |l: &String| l.split('\t').map(String::from).collect();
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| (i, cells(l)))
        .collect()
}

fn csv(lines: &[String]) -> Result<Vec<(usize, Vec<String>)>, String> {
    let mut rows = Vec::new();
    if empty(lines) {
        return Ok(rows);
    }
    let (mut cells, mut cell) = (Vec::new(), String::new());
    let (mut quoted, mut closed, mut start) = (false, false, 0);
    for (l, line) in lines.iter().enumerate() {
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if quoted && chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                }
                '"' if quoted => (quoted, closed) = (false, true),
                _ if quoted => cell.push(c),
                ',' => {
                    cells.push(std::mem::take(&mut cell));
                    closed = false;
                }
                '"' if cell.is_empty() && !closed => quoted = true,
                _ if c == '"' || closed => return Err(format!("line {} has a stray quote", l + 1)),
                _ => cell.push(c),
            }
        }
        if quoted {
            cell.push('\n');
            continue;
        }
        cells.push(std::mem::take(&mut cell));
        closed = false;
        rows.push((start, std::mem::take(&mut cells)));
        start = l + 1;
    }
    match quoted {
        true => Err(format!("line {} has an unclosed quote", start + 1)),
        false => Ok(rows),
    }
}

fn same_width(rows: Vec<(usize, Vec<String>)>) -> Result<Parsed, String> {
    if let Some((_, head)) = rows.first()
        && let Some((l, row)) = rows.iter().find(|(_, r)| r.len() != head.len())
    {
        let fields = if row.len() == 1 { "field" } else { "fields" };
        return Err(format!(
            "line {} has {} {fields}, not {}",
            l + 1,
            row.len(),
            head.len()
        ));
    }
    Ok(Parsed::Table(rows))
}

fn jsonl(lines: &[String]) -> Result<Parsed, String> {
    let mut records = Vec::new();
    for (l, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if serde_json::from_str::<serde::de::IgnoredAny>(line).is_err() {
            return Err(format!("line {} is not JSON", l + 1));
        }
        records.push(pretty(line, l));
    }
    Ok(Parsed::Records(records))
}

fn pretty(json: &str, l: usize) -> Vec<(String, (usize, usize))> {
    let mut out: Vec<(String, (usize, usize))> = Vec::new();
    let (mut depth, mut fresh) = (0usize, true);
    let b = json.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        i += match b[i] {
            b'"' => {
                let mut j = i + 1;
                while b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                j + 1 - i
            }
            b'{' | b'[' | b'}' | b']' | b',' | b':' => 1,
            c if c.is_ascii_whitespace() => {
                i += 1;
                continue;
            }
            _ => b[i..]
                .iter()
                .position(|c| b"{}[],:\" \t\r\n".contains(c))
                .unwrap_or(b.len() - i),
        };
        let token = &json[start..i];
        let next = json[i..].trim_start().bytes().next();
        let closes = |c: u8| matches!((c, next), (b'{', Some(b'}')) | (b'[', Some(b']')));
        if matches!(b[start], b'}' | b']') && !fresh {
            depth -= 1;
            fresh = true;
        }
        if fresh {
            out.push(("  ".repeat(depth), (l, start)));
            fresh = false;
        }
        let row = &mut out.last_mut().unwrap().0;
        row.push_str(token);
        match b[start] {
            c @ (b'{' | b'[') if closes(c) => {
                let at = json[i..].find(['}', ']']).unwrap() + i;
                row.push(b[at] as char);
                i = at + 1;
            }
            b'{' | b'[' => (depth, fresh) = (depth + 1, true),
            b',' => fresh = true,
            b':' => row.push(' '),
            _ => {}
        }
    }
    out
}

impl Data {
    pub fn layout(self, parsed: Option<&Parsed>, lines: &[String], width: usize, fit: Fit) -> Doc {
        let mut lay = Lay {
            width: width.max(1),
            fit: Some(fit),
            ..Default::default()
        };
        let line = |(text, at): (String, (usize, usize))| BlockLine {
            as_written: text,
            src_line_col: at,
            tab_spaces_parser_added: 0,
        };
        match (self, parsed) {
            (Data::Mermaid, _) => lay.code_block(Block {
                info_first_word: "mermaid".into(),
                lines: (0..lines.len())
                    .map(|l| line((lines[l].clone(), (l, 0))))
                    .collect(),
                last_line_unended: false,
                span: 0..lines.len(),
            }),
            (_, Some(Parsed::Table(rows))) => lay.table(Table {
                aligns: Vec::new(),
                delimiter_row: false,
                rows: rows.iter().map(|(l, cells)| table_row(*l, cells)).collect(),
            }),
            (_, Some(Parsed::Records(records))) => {
                for (k, record) in records.iter().enumerate() {
                    let l = record[0].1.0;
                    if k > 0 {
                        let rule = "\u{2500}".repeat(lay.avail());
                        let look = vec![(Ink::Line.plain(), 0..rule.len())];
                        let at = lay.after();
                        lay.push(rule, look, at, Kind::Gap);
                    }
                    lay.code_block(Block {
                        info_first_word: "json".into(),
                        lines: record.iter().cloned().map(line).collect(),
                        last_line_unended: false,
                        span: l..l + 1,
                    });
                }
            }
            (_, None) => {}
        }
        lay.done(lines.len())
    }
}

fn table_row(src_line: usize, cells: &[String]) -> TableRow {
    let cell = |text: &String| {
        let text = text.replace('\t', crate::buffer::TAB);
        Inline {
            looks: vec![(Ink::Text.plain(), 0..text.len())],
            text,
            pieces: Vec::new(),
        }
    };
    TableRow {
        src_line,
        cells: cells.iter().map(cell).collect(),
    }
}

#[cfg(test)]
mod tests;
