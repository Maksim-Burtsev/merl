use super::definition::resolution;
use super::links::in_project;
use super::*;

use search::RefAt;

impl App {
    pub(super) fn follow_ref(&mut self, here: &Path) -> bool {
        if !search::is_ref_file(here) {
            return false;
        }
        let lines: Vec<&str> = self.buf.lines.iter().map(String::as_str).collect();
        let found = search::ref_at(here, &lines, self.line, self.col);
        let word = |r: &str| {
            self.word_here(self.kind()).map_or_else(
                || r.rsplit(['/', '#']).next().unwrap_or(r).to_owned(),
                |(_, w)| w.to_owned(),
            )
        };
        match found {
            None => false,
            Some(RefAt::Refused) => {
                self.message = resolution(&word(""), None, &[], None, false);
                true
            }
            Some(RefAt::Value(r)) => {
                let word = word(&r);
                self.open_ref(here, &r, &word)
            }
        }
    }

    fn open_ref(&mut self, here: &Path, reference: &str, word: &str) -> bool {
        let (file, fragment) = reference.split_once('#').unwrap_or((reference, ""));
        let rel = match file {
            "" => Some(here.to_path_buf()),
            url if url.contains("://") => self
                .files
                .iter()
                .find(|f| {
                    search::is_ref_file(f)
                        && self
                            .text_of(f)
                            .is_some_and(|t| search::keyword_line(&t, "$id", url).is_some())
                })
                .cloned(),
            path => in_project(
                here.parent().unwrap_or(Path::new("")),
                &search::percent_decoded(path),
            ),
        };
        let Some(rel) = rel else {
            return false;
        };
        let Some(text) = self.text_of(&rel) else {
            return false;
        };
        let line = match fragment {
            "" => Some(1),
            pointer if pointer.starts_with('/') => search::pointer_line(&rel, &text, pointer),
            anchor => search::keyword_line(&text, "$anchor", anchor),
        };
        let Some(line) = line else {
            return false;
        };
        let shown = match file {
            "" => String::new(),
            _ => rel.display().to_string(),
        };
        let fragment = match fragment {
            "" => String::new(),
            f => format!("#{f}"),
        };
        self.open_link(
            &self.root.join(&rel),
            line,
            format!("{word}: via {shown}{fragment}"),
        );
        true
    }
}
