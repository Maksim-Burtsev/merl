use regex::Regex;

use super::{Kind, code, split_top};

#[derive(Debug, PartialEq, Eq)]
pub struct SwiftArgument {
    pub owner: String,
    pub callee: String,
    pub open: usize,
    pub label: String,
}

pub fn swift_argument(lead: &str) -> Option<SwiftArgument> {
    static LABEL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^`?(\w+)`?\s*:$").expect("the regex is a literal")
    });
    static CALLEE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`])([A-Z]\w*(?:\.[A-Z]\w*)*)(?:\.([a-z_]\w*))?\s*$")
            .expect("the regex is a literal")
    });
    let bytes: Vec<(usize, u8)> = code(Kind::Swift, lead).collect();
    let mut depth = 0usize;
    let mut open = None;
    for &(i, c) in bytes.iter().rev() {
        match c {
            b')' | b']' | b'}' => depth += 1,
            b'(' if depth == 0 => {
                open = Some(i);
                break;
            }
            b'[' | b'{' if depth == 0 => return None,
            b'(' | b'[' | b'{' => depth -= 1,
            _ => {}
        }
    }
    let open = open?;
    let args = split_top(Kind::Swift, lead.get(open + 1..)?, b',');
    let last = args.last().map_or("", |a| a.trim());
    let label = match LABEL.captures(last) {
        Some(c) => c[1].to_owned(),
        None if last.is_empty() && args.len() <= 1 => "_".to_owned(),
        None => return None,
    };
    let c = CALLEE.captures(lead.get(..open)?)?;
    Some(SwiftArgument {
        owner: c[1].to_owned(),
        callee: c.get(2).map_or("init", |m| m.as_str()).to_owned(),
        open,
        label,
    })
}
