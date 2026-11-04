use super::{App, Hit, Kind, Path, search};

impl App {
    pub(super) fn swift_protocol(&self, here: &Path, name: &str) -> bool {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        let decls: Vec<String> = self
            .project_definitions(Kind::Swift, here, name, &pattern)
            .iter()
            .filter(|h| self.in_code(Kind::Swift, h))
            .filter_map(|h| search::swift_type_header(&h.text))
            .filter(|t| t.name == name && t.keyword != "extension")
            .map(|t| t.keyword)
            .collect();
        matches!(decls.as_slice(), [k] if k == "protocol")
    }

    pub(super) fn swift_defaults(
        &self,
        here: &Path,
        types: &[String],
        word: &str,
    ) -> Option<Vec<Hit>> {
        let mut protocols: Vec<String> = Vec::new();
        for t in types {
            let pattern = search::def_patterns(Kind::Swift, t).join("|");
            for h in self.project_definitions(Kind::Swift, here, t, &pattern) {
                let Some(head) = search::swift_type_header(&h.text)
                    .filter(|head| head.name == *t && self.in_code(Kind::Swift, &h))
                else {
                    continue;
                };
                let inherited = search::swift_inherited(&h.text);
                if head.where_constrained && !inherited.is_empty() {
                    return None;
                }
                protocols.extend(inherited);
            }
        }
        protocols.sort();
        protocols.dedup();
        protocols.retain(|p| self.swift_protocol(here, p));
        let mut conformed = protocols.clone();
        let mut levels = vec![protocols];
        while let Some(level) = levels.last() {
            let mut next = Vec::new();
            for p in level {
                for up in self.swift_refined(here, p) {
                    if !conformed.contains(&up) {
                        conformed.push(up.clone());
                        next.push(up);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            levels.push(next);
        }
        for level in &levels {
            let mut rows: Vec<Hit> = Vec::new();
            for p in level {
                rows.extend(self.swift_protocol_rows(here, p, word, false, &conformed)?);
            }
            if !rows.is_empty() {
                return Some(rows);
            }
        }
        None
    }

    fn swift_refined(&self, here: &Path, protocol: &str) -> Vec<String> {
        let pattern = search::def_patterns(Kind::Swift, protocol).join("|");
        self.project_definitions(Kind::Swift, here, protocol, &pattern)
            .iter()
            .filter(|h| {
                self.in_code(Kind::Swift, h)
                    && search::swift_type_header(&h.text)
                        .is_some_and(|t| t.keyword == "protocol" && t.name == protocol)
            })
            .flat_map(|h| search::swift_inherited(&h.text))
            .filter(|p| self.swift_protocol(here, p))
            .collect()
    }

    pub(super) fn swift_protocol_rows(
        &self,
        here: &Path,
        protocol: &str,
        word: &str,
        requirement: bool,
        conformed: &[String],
    ) -> Option<Vec<Hit>> {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        let full = format!("{protocol}.{word}");
        let mut rows = Vec::new();
        for h in self.project_definitions(Kind::Swift, here, word, &pattern) {
            let text = self.text_of(&h.path)?;
            let named = search::qualified(Kind::Swift, &text, h.line1, word)
                .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")));
            if !named || !self.in_code(Kind::Swift, &h) {
                continue;
            }
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Swift, &text);
            let at = search::swift_enclosing_type(&lines, &literal, h.line1)?;
            let head = search::swift_type_header(lines[at - 1])?;
            let constrained = head.where_constrained
                || lines
                    .get(at)
                    .is_some_and(|l| l.trim_start().starts_with("where "));
            let met = || {
                let clause = lines[at - 1]
                    .split_once(" where ")
                    .map_or_else(|| lines.get(at).map_or("", |l| l.trim_start()), |(_, w)| w);
                let clause = clause.strip_prefix("where ").unwrap_or(clause);
                search::swift_self_bounds(clause)
                    .is_some_and(|b| b.iter().all(|q| conformed.contains(q)))
            };
            match head.keyword.as_str() {
                "protocol" if requirement => rows.push(h),
                "extension" if !requirement && constrained && !met() => return None,
                "extension" if !requirement => rows.push(h),
                _ => {}
            }
        }
        Some(rows)
    }
}
