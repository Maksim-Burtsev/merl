use super::definition::resolution;
use super::*;

impl App {
    pub(super) fn gdscript_definition(
        &mut self,
        here: &Path,
        range: &std::ops::Range<usize>,
        chain: &[String],
        word: &str,
    ) -> bool {
        if search::gdscript_names_nothing(self.line_str(), range.start) {
            self.message = resolution(word, None, &[], None, false);
            return true;
        }
        let dotted = self.line_str()[..range.start].ends_with('.');
        let (name, member) = match chain {
            [] if !dotted => (word, None),
            [first] => (first.as_str(), Some(word)),
            _ => return false,
        };
        let Some(script) = self.gdscript_autoload(here, name) else {
            return false;
        };
        let Some(member) = member else {
            let found = self.module_candidates(vec![script]);
            self.show_definitions(Kind::Gdscript, word, here, found, None);
            return true;
        };
        let found = self.gdscript_members(&script, name, member);
        if found.is_empty() {
            return false;
        }
        self.show_definitions(Kind::Gdscript, word, here, found, None);
        true
    }

    fn gdscript_autoload(&self, here: &Path, name: &str) -> Option<PathBuf> {
        let root = search::gdscript_root(here, &self.files)?;
        let project = self.file_text(&root.join("project.godot"))?;
        let (_, path) = search::gdscript_autoloads(&project)
            .into_iter()
            .find(|(n, _)| n == name)?;
        search::gdscript_files(here, &path, &self.files)
            .into_iter()
            .next()
    }

    fn gdscript_members(&self, script: &Path, owner: &str, word: &str) -> Vec<Candidate> {
        let Some(text) = self.text_of(script) else {
            return Vec::new();
        };
        let Ok(re) = Regex::new(&search::def_patterns(Kind::Gdscript, word).join("|")) else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Gdscript, &text);
        (lines.iter().enumerate())
            .filter(|&(i, l)| {
                !l.starts_with([' ', '\t'])
                    && literal.get(i) != Some(&true)
                    && re.is_match(l)
                    && search::gdscript_declares(&lines, i + 1, l)
            })
            .map(|(i, l)| Candidate {
                hit: Hit {
                    deleted: None,
                    path: script.to_path_buf(),
                    line: i + 1,
                    col: 0,
                    text: (*l).to_owned(),
                },
                reason: Reason::Path(owner.to_owned()),
            })
            .collect()
    }
}
