//! `d` in the template of a Vue, Svelte or Astro component (#413).

use super::definition::resolution;
use super::*;

impl App {
    pub(super) fn on_template(&self, here: &Path) -> bool {
        let code = search::script_lines(here, &self.buf.lines.join("\n"));
        code.is_some_and(|c| c.get(self.line) == Some(&false))
    }

    /// `d` on a line of a component outside its script. A `<style>` block declares and names no
    /// code. A name the template binds is a local of the file, beside the script's own binding
    /// of it. A component's tag no import binds is the component file of its name, or the
    /// script's binding of that name. Anything else is looked up as from the script, with the
    /// tag's word turned into the component's name: `false`, and `word` is what to look up.
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
        if search::in_style(&self.buf.lines, self.line) {
            self.message = resolution(word, None, &[], None, false);
            return true;
        }
        let script = search::script_text(here, &raw, None);
        // What the script binds at its top level: what a template name reads, as from the end
        // of the file.
        let end = self.buf.lines.len() + 1;
        let script_binds = |name: &str| -> Vec<usize> {
            (search::bindings(Kind::TsJs, &format!("{script}\n0"), end, name).iter())
                .map(|b| b.line)
                .collect()
        };
        let bound = search::template_binds(&self.buf.lines, &code, word);
        let tag = search::tag_at(self.line_str(), range.start);
        let lines = if !bound.is_empty() {
            // The template's binding is scoped to its element or block, which is not read: the
            // script's binding of the name may be the one meant, and both are offered.
            let mut lines = script_binds(word);
            lines.extend(bound);
            lines
        } else if let Some(tag) = tag {
            *word = search::component_name(&self.line_str()[tag]);
            if search::imports(Kind::TsJs, &script)
                .iter()
                .any(|(n, _)| n == word)
            {
                return false;
            }
            let names = ["vue", "svelte", "astro"].map(|e| format!("{word}.{e}"));
            let files: Vec<PathBuf> = (self.files.iter())
                .filter(|f| (f.file_name()).is_some_and(|n| names.iter().any(|m| n == m.as_str())))
                .cloned()
                .collect();
            // `const Comp = defineAsyncComponent(…)`, Astro's `const { Component } = …`.
            if files.is_empty() && !script_binds(word).is_empty() {
                return false;
            }
            let found = (self.module_candidates(files).into_iter())
                .map(|c| Candidate {
                    reason: Reason::ByName,
                    ..c
                })
                .collect();
            self.show_definitions(Kind::TsJs, &word.clone(), here, found, None);
            return true;
        } else {
            return false;
        };
        let found = (lines.into_iter())
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
            .collect();
        self.show_definitions(Kind::TsJs, &word.clone(), here, found, None);
        true
    }
}
