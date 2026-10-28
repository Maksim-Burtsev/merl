use super::{Candidate, search};

pub(super) fn one_reason(found: &[Candidate]) -> bool {
    found.windows(2).all(|w| w[0].reason == w[1].reason)
}

pub(super) fn resolution(
    word: &str,
    target: Option<&str>,
    found: &[Candidate],
    broke: Option<&str>,
    cut: bool,
) -> String {
    let note = broke.map_or(String::new(), |at| format!(" (chain broke at {at})"));
    let Some(first) = found.first() else {
        return format!("no definition for {word}{note}");
    };
    let reason = &first.reason;
    if let ([_], false) = (found, cut) {
        let mut why = reason.to_string();
        if !reason.proven() {
            why.push_str(", 1 match");
        }
        if let Some(at) = broke {
            why = format!("{why}, chain broke at {at}");
        }
        return match target {
            Some(target) => format!("{word} \u{2192} {target} ({why})"),
            None => format!("{word}: {why}"),
        };
    }
    let n = match found.len() {
        n if cut || n >= search::MAX_HITS => format!("{n}+"),
        n => n.to_string(),
    };
    if one_reason(found) {
        format!("{word}: {reason}, {n} declarations{note}")
    } else {
        format!("{word}: {n} declarations{note}")
    }
}
