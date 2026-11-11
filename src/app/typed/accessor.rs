use super::{App, Hit, Kind, Regex, search};
use crate::app::members::ts_modifiers;
use std::sync::LazyLock;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Get,
    Set,
    Delete,
}

static WRITTEN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*=(?:[^=>]|$)").expect("the regex is a literal"));
static READ: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:$|[.);\[]|\?\.|[!=]==?[^=]|&&|\|\||:\s*(?:#.*)?$)")
        .expect("the regex is a literal")
});

impl App {
    pub(super) fn accessor_side(&self, kind: Kind, word: &str, mut hits: Vec<Hit>) -> Vec<Hit> {
        if !matches!(kind, Kind::TsJs | Kind::Python) {
            return hits;
        }
        let Some(side) = self.use_side(kind) else {
            return hits;
        };
        let Some(first) = hits.first().cloned() else {
            return hits;
        };
        let Some(text) = self.text_of(&first.path) else {
            return hits;
        };
        let owner = search::owner_decl(kind, &text, first.line1);
        if owner.is_none()
            || hits
                .iter()
                .any(|h| h.path != first.path || search::owner_decl(kind, &text, h.line1) != owner)
        {
            return hits;
        }
        let sides: Vec<Option<Side>> = hits
            .iter()
            .map(|h| accessor(kind, &text, h.line1, word))
            .collect();
        if sides.iter().all(Option::is_some)
            && sides.contains(&Some(Side::Get))
            && sides.contains(&Some(Side::Set))
        {
            let mut sides = sides.into_iter();
            hits.retain(|_| sides.next().flatten() == Some(side));
            return hits;
        }
        if kind != Kind::TsJs || side != Side::Set || sides != [Some(Side::Get)] {
            return hits;
        }
        let setters: Vec<usize> = (1..=text.lines().count())
            .filter(|&n| {
                accessor(kind, &text, n, word) == Some(Side::Set)
                    && search::owner_decl(kind, &text, n) == owner
            })
            .collect();
        let [line1] = setters[..] else {
            return hits;
        };
        vec![Hit {
            path: first.path,
            line1,
            byte_col: None,
            text: text.lines().nth(line1 - 1).unwrap_or_default().to_owned(),
            deleted: None,
        }]
    }

    fn use_side(&self, kind: Kind) -> Option<Side> {
        let (range, _) = self.word_here(Some(kind))?;
        let line = self.line_str();
        let tail = line.get(range.end..)?;
        let operand = line
            .get(..range.start)?
            .trim_end_matches(|c: char| c.is_alphanumeric() || "_$.?!#".contains(c))
            .trim();
        if WRITTEN.is_match(tail) {
            return operand.is_empty().then_some(Side::Set);
        }
        let changed = operand.ends_with("++")
            || operand.ends_with("--")
            || operand
                .rsplit(|c: char| !c.is_alphanumeric())
                .next()
                .is_some_and(|w| matches!(w, "del" | "delete"));
        let keyed = matches!(tail.trim(), "" | ",")
            && operand.strip_suffix(':').is_some_and(|k| {
                !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || "_$".contains(c))
            });
        ((READ.is_match(tail) || keyed) && !changed).then_some(Side::Get)
    }
}

fn accessor(kind: Kind, text: &str, line1: usize, word: &str) -> Option<Side> {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(line1.checked_sub(1)?)?;
    if kind == Kind::TsJs {
        let named = line
            .split_whitespace()
            .find(|w| !ts_modifiers(line).contains(w) && !w.starts_with('@'))?
            .trim_start_matches('#')
            .starts_with(&format!("{word}("));
        let mods = ts_modifiers(line);
        return match (named, mods.contains(&"get"), mods.contains(&"set")) {
            (true, true, false) => Some(Side::Get),
            (true, false, true) => Some(Side::Set),
            _ => None,
        };
    }
    if !line.trim_start().starts_with("def ") && !line.trim_start().starts_with("async def ") {
        return None;
    }
    lines[..line1 - 1]
        .iter()
        .rev()
        .map(|l| l.trim())
        .take_while(|l| l.starts_with('@'))
        .find_map(|l| match l.strip_prefix('@')? {
            "property" => Some(Side::Get),
            d if d == format!("{word}.setter") => Some(Side::Set),
            d if d == format!("{word}.deleter") => Some(Side::Delete),
            _ => None,
        })
}
