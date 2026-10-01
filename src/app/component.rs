//! `d` in the template of a Vue, Svelte or Astro component (#413).

use super::*;

impl App {
    /// `d` on a line of a component outside its script: a name the template binds is a local of
    /// the file; a component's tag no import binds is the component file of its name. Anything
    /// else is looked up as from the script, with the tag's word turned into the component's
    /// name: `false`, and `word` is what to look up.
    pub(super) fn component_template(
        &mut self,
        here: &Path,
        range: &std::ops::Range<usize>,
        word: &mut String,
    ) -> bool {
        let raw = self.buf.lines.join("\n");
        let Some(code) = search::script_lines(here, &raw) else {
            return false;
        };
        if code.get(self.line) == Some(&true) {
            return false;
        }
        let bound = search::template_binds(&self.buf.lines, &code, word);
        let tag = search::tag_at(self.line_str(), range.start);
        let found: Vec<Candidate> = if !bound.is_empty() {
            (bound.into_iter())
                .map(|line| Candidate {
                    hit: Hit {
                        deleted: None,
                        path: here.to_path_buf(),
                        line,
                        col: 0,
                        text: self.buf.lines[line - 1].clone(),
                    },
                    reason: Reason::Local,
                })
                .collect()
        } else if let Some(tag) = tag {
            *word = search::component_name(&self.line_str()[tag]);
            let script = search::script_text(here, &raw, None);
            if search::imports(Kind::TsJs, &script)
                .iter()
                .any(|(n, _)| n == word)
            {
                return false;
            }
            let names = ["vue", "svelte", "astro"].map(|e| format!("{word}.{e}"));
            let files = (self.files.iter()).filter(|f| {
                f.file_name()
                    .is_some_and(|n| names.iter().any(|m| n == m.as_str()))
            });
            let files = files.cloned().collect();
            (self.module_candidates(files).into_iter())
                .map(|c| Candidate {
                    reason: Reason::ByName,
                    ..c
                })
                .collect()
        } else {
            return false;
        };
        self.show_definitions(Kind::TsJs, &word.clone(), here, found, None);
        true
    }
}
