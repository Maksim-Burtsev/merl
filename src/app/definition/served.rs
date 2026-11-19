use super::super::resolution::resolution;
use super::{App, Candidate, Hit, Path, Reason, search};
use crate::lsp;
use std::time::Instant;

fn byte_col(line: &str, character: usize) -> usize {
    let mut units = 0;
    for (at, c) in line.char_indices() {
        if units >= character {
            return at;
        }
        units += c.len_utf16();
    }
    line.len()
}

impl App {
    pub(in crate::app) fn define(&mut self) {
        if let Err(note) = self.served() {
            self.goto_definition();
            if let Some(note) = note.filter(|_| !self.message.is_empty()) {
                self.message = format!("{} \u{b7} {note}", self.message);
            }
        }
    }

    fn served(&mut self) -> Result<(), Option<String>> {
        if !lsp::on() || self.deleted.is_some() || self.probe.is_some() {
            return Err(None);
        }
        let kind = self.kind().ok_or(None)?;
        let path = self.buf.path.clone().ok_or(None)?;
        let spec = lsp::spec_for(&path).ok_or(None)?;
        let (range, word) = self.definition_word(Some(kind)).ok_or(None)?;
        let real = |p: &Path| crate::real::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let (root, here) = (real(&self.root), real(&self.root.join(&path)));
        let col = if range.contains(&self.col) {
            self.col
        } else {
            range.start
        };
        let character = (self.line_str().get(..col)).map_or(0, |s| s.encode_utf16().count());
        let disk = self.buf.to_bytes();
        let text = String::from_utf8_lossy(&disk).trim_start_matches('\u{feff}').to_owned();
        let (line0, started) = (self.line, Instant::now());
        let client = (self.lsp.entry(spec.name)).or_insert_with(|| lsp::Client::start(spec, &root));
        let outcome = match client {
            Some(client) => client.ask(here.clone(), text, line0, character),
            None => return Err(None),
        };
        let waited = started.elapsed().as_secs_f64() * 1000.0;
        let press = |what: &str| lsp::log(&format!("{} press waited={waited:.1} {what}", spec.name));
        let answer = match outcome {
            lsp::Outcome::Answer(a) => a,
            lsp::Outcome::Loading => {
                press("outcome=loading");
                return Err(Some(format!("{} is loading", spec.name)));
            }
            lsp::Outcome::Slow => {
                press("outcome=slow");
                return Err(Some(format!("{} is slow", spec.name)));
            }
            lsp::Outcome::Gone => {
                press("outcome=gone");
                return Err(None);
            }
        };
        let rt = answer.round_trip.as_secs_f64() * 1000.0;
        let all = answer.places.len();
        let found: Vec<Candidate> = (answer.places.into_iter())
            .filter(|p| p.path != here || p.line1 != line0 + 1)
            .map(|p| {
                let rel = (p.path.strip_prefix(&root)).map_or_else(|_| p.path.clone(), Path::to_path_buf);
                let text = (self.text_of(&rel))
                    .and_then(|t| t.lines().nth(p.line1 - 1).map(str::to_owned))
                    .unwrap_or_default();
                Candidate {
                    hit: Hit {
                        byte_col: Some(byte_col(&text, p.character)),
                        path: rel,
                        line1: p.line1,
                        text,
                        deleted: None,
                    },
                    reason: Reason::Path(spec.name.to_owned()),
                }
            })
            .collect();
        let used = match found.len() {
            0 if all == 0 => "rules-empty",
            0 => "rules-self",
            1 => "jump",
            _ => "pick",
        };
        press(&format!("outcome=answer rt={rt:.1} n={all} used={used}"));
        match found.as_slice() {
            [] => return Err((all == 0).then(|| format!("{} has none", spec.name))),
            [one] => {
                let target = (self.hit_text(&one.hit))
                    .and_then(|t| search::qualified(kind, &t, one.hit.line1, &word));
                let status = resolution(&word, target.as_deref(), &found, None, false);
                let to = self.root.join(&one.hit.path);
                self.jump_to_col(&to, one.hit.line1, one.hit.byte_col.unwrap_or(0));
                if self.buf.path.as_deref() == Some(to.as_path()) {
                    self.message = status;
                }
            }
            _ => self.pick_definitions(kind, &word, found, None, false, false),
        }
        Ok(())
    }
}
