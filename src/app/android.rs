use super::*;

impl App {
    pub(super) fn android_definition(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        col: usize,
    ) -> bool {
        let Some((segments, at)) = search::dotted_at(self.line_str(), col) else {
            return false;
        };
        let free = |name: &str| search::bindings(Kind::Jvm, text, self.line + 1, name).is_empty();
        let build_script = here.to_string_lossy().ends_with(".gradle.kts");
        let found = if build_script
            && let Some((section, key)) = search::catalog_key(&segments)
            && free("libs")
            && let Some((catalog, toml)) = (here.ancestors().skip(1))
                .map(|dir| dir.join("gradle/libs.versions.toml"))
                .find_map(|p| Some((p.clone(), self.file_text(&p)?)))
        {
            let line = search::catalog_line(&toml, section, &key);
            let hits = line.map(|n| hit_at(&catalog, n, toml.lines().nth(n - 1)));
            hits.into_iter().collect()
        } else if let Some((receiver, ty, name)) = search::resource_at(text, &segments, at)
            && free(receiver)
        {
            self.resources(ty, name)
        } else {
            return false;
        };
        let found = found
            .into_iter()
            .map(|hit: Hit| Candidate {
                reason: Reason::Path(hit.path.display().to_string()),
                hit,
            })
            .collect();
        self.show_definitions(Kind::Jvm, word, here, found, None);
        true
    }

    fn resources(&self, ty: &str, name: &str) -> Vec<Hit> {
        let xml = |p: &Path, dir: &str| {
            p.extension().is_some_and(|e| e == "xml") && search::in_res(p, dir)
        };
        let name_re = regex::escape(name);
        let mut hits = if ty == "id" {
            let pattern = format!(r#""@\+id/{name_re}""#);
            self.grep(&pattern, false, false, |p| xml(p, "layout"))
                .unwrap_or_default()
        } else if let Some(tags) = search::value_tags(ty) {
            let pattern = format!(r#"<(?:{tags})\s[^>]*\bname="{name_re}""#);
            self.grep(&pattern, false, false, |p| xml(p, "values"))
                .unwrap_or_default()
        } else {
            (self.files.iter())
                .filter(|p| search::res_file(p, ty, name))
                .map(|p| {
                    let text = self.file_text(p);
                    hit_at(p, 1, text.as_deref().and_then(|t| t.lines().next()))
                })
                .collect()
        };
        hits.sort_by_key(|h| search::qualified_res(&h.path));
        hits
    }
}

fn hit_at(path: &Path, line: usize, text: Option<&str>) -> Hit {
    Hit {
        deleted: None,
        path: path.to_owned(),
        line,
        col: 0,
        text: text.unwrap_or_default().to_owned(),
    }
}
