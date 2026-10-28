use regex::Regex;

use super::{Kind, SwiftExpr, code, split_top, swift_expr};

#[derive(Debug, PartialEq, Eq)]
pub struct SwiftArgument {
    pub owner: String,
    pub callee: String,
    pub open: usize,
    pub label: String,
    pub on_value: bool,
}

pub fn swift_argument(lead: &str) -> Option<SwiftArgument> {
    static LABEL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^`?(\w+)`?\s*:$").expect("the regex is a literal")
    });
    static CALLEE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`])([A-Z]\w*(?:\.[A-Z]\w*)*)(?:\.([a-z_]\w*))?\s*$")
            .expect("the regex is a literal")
    });
    static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.?!`])([a-z_]\w*(?:\.[a-z_]\w*)*)\.([a-z_]\w*)\s*$")
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
    let before = lead.get(..open)?;
    if let Some(c) = CALLEE.captures(before) {
        return Some(SwiftArgument {
            owner: c[1].to_owned(),
            callee: c.get(2).map_or("init", |m| m.as_str()).to_owned(),
            open,
            label,
            on_value: false,
        });
    }
    let c = METHOD.captures(before)?;
    Some(SwiftArgument {
        owner: c[1].to_owned(),
        callee: c[2].to_owned(),
        open,
        label,
        on_value: true,
    })
}

pub fn swift_case_value(value: &str) -> Option<(String, String)> {
    let (SwiftExpr::Call(e) | SwiftExpr::Chain(e)) = swift_expr(value)? else {
        return None;
    };
    let (ty, case) = e.rsplit_once('.')?;
    let typed = ty.split('.').all(|p| p.starts_with(char::is_uppercase));
    (typed && case.starts_with(|c: char| c.is_lowercase() || c == '_'))
        .then(|| (ty.to_owned(), case.to_owned()))
}

pub fn swift_var(line: &str, name: &str) -> bool {
    Regex::new(&format!(r"\bvar\s+`?{}`?\s*=", regex::escape(name))).is_ok_and(|r| r.is_match(line))
}
