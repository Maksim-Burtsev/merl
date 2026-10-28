use super::{App, Kind, Path, search};

const BOUND: &str = "(bound) ";

pub(super) fn unbound(written: &str) -> &str {
    written.strip_prefix(BOUND).unwrap_or(written)
}

pub(super) fn shown(written: &str) -> String {
    let w = unbound(written);
    search::swift_type_name(w).unwrap_or_else(|| w.to_owned())
}

impl App {
    pub(super) fn swift_generic_outside(
        &self,
        here: &Path,
        lines: &[&str],
        at: usize,
        name: &str,
        written: &str,
    ) -> Option<String> {
        if written.trim().trim_end_matches(['?', '!']) != name {
            return None;
        }
        let bound =
            regex::Regex::new(&format!(r"\b{name}\s*:\s*([\w.]+(?:\s*&\s*[\w.]+)*)")).ok()?;
        let bounds_in = |head: &str| -> Vec<String> {
            let head = head.split('{').next().unwrap_or_default();
            let list = head
                .split_once('<')
                .and_then(|(_, r)| r.split_once('>'))
                .map_or("", |(l, _)| l);
            let where_clause = head.split_once(" where ").map_or("", |(_, w)| w);
            [list, where_clause]
                .into_iter()
                .flat_map(|r| bound.captures_iter(r))
                .map(|c| c[1].to_owned())
                .filter(|b| b != "Sendable")
                .collect()
        };
        let mut head = String::new();
        for l in lines.iter().skip(at - 1).take(8) {
            head.push_str(l);
            head.push(' ');
            if l.contains('{') {
                break;
            }
        }
        let mut bounds = bounds_in(&head);
        let owner = search::swift_type_header(lines[at - 1]).map(|t| t.name);
        if let (true, Some(owner)) = (bounds.is_empty(), owner) {
            let text = self.text_of(here)?;
            let mine: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Swift, &text);
            let mut s = search::swift_scope(&mine, &literal, self.line + 1).header_line1;
            while s > 0 && bounds.is_empty() {
                if search::swift_type_header(mine[s - 1])
                    .is_some_and(|t| t.keyword == "extension" && t.name == owner)
                    && mine[s - 1].contains(" where ")
                {
                    bounds = bounds_in(mine[s - 1]);
                }
                s = search::swift_scope(&mine, &literal, s).header_line1;
            }
        }
        let [b] = <[String; 1]>::try_from(bounds).ok()?;
        if b.contains('&')
            || self.swift_protocol(here, &b)
            || self
                .swift_type(here, &b)
                .is_some_and(|t| t.keyword.is_some())
        {
            return None;
        }
        Some(format!("{BOUND}{b}"))
    }
}
