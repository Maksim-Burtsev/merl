use regex::Regex;

use super::dart::DartType;

pub fn dart_list_element<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:final|var)\s*\[([^\[\](){}]*)\]\s*=\s*([a-z_$][\w$]*)\s*;")
            .expect("the regex is a literal")
    });
    let c = PATTERN.captures(line)?;
    c[1].split(',')
        .any(|item| item.trim() == name)
        .then(|| c.get(2).map(|m| m.as_str()))?
}

pub fn dart_element_typed(line: &str, name: &str) -> Option<DartType> {
    let re = Regex::new(&format!(
        r"(?:^|[(,{{\[\s])(?:(?:final|late|required|covariant)\s+)*List<(?:([a-z_$][\w$]*)\.)?(_?[A-Z][\w$]*)>\??\s+{}\s*(?:[,)=;}}\]]|$)",
        regex::escape(name)
    ))
    .ok()?;
    let c = re.captures(line)?;
    Some(DartType {
        prefix: c.get(1).map(|m| m.as_str().to_owned()),
        class: c[2].to_owned(),
        constructor: None,
    })
}

pub fn dart_entry_part<'a>(line: &'a str, name: &str) -> Option<(&'a str, bool)> {
    let re = Regex::new(&format!(
        r"^\s*(?:final|var)\s+{}\s*=\s*([a-z_$][\w$]*)\.(key|value)\s*;",
        regex::escape(name)
    ))
    .ok()?;
    let c = re.captures(line)?;
    Some((c.get(1)?.as_str(), &c[2] == "value"))
}

pub fn dart_entry_typed(line: &str, entry: &str, value: bool) -> Option<DartType> {
    let one = r"(?:([a-z_$][\w$]*)\.)?([A-Za-z_$][\w$]*)\??";
    let re = Regex::new(&format!(
        r"(?:^|[(,{{\[\s])(?:(?:final|late|required|covariant)\s+)*MapEntry<\s*{one}\s*,\s*{one}\s*>\??\s+{}\s*(?:[,)=;}}\]]|$)",
        regex::escape(entry)
    ))
    .ok()?;
    let c = re.captures(line)?;
    let (prefix, class) = if value { (3, 4) } else { (1, 2) };
    let class = c[class].to_owned();
    class
        .trim_start_matches('_')
        .starts_with(char::is_uppercase)
        .then(|| DartType {
            prefix: c.get(prefix).map(|m| m.as_str().to_owned()),
            class,
            constructor: None,
        })
}

pub fn dart_for_in<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let re = Regex::new(&format!(
        r"\bfor\s*\(\s*(?:final|var)\s+{}\s+in\s+([A-Za-z_$][\w$]*)\s*\)",
        regex::escape(name)
    ))
    .ok()?;
    Some(re.captures(line)?.get(1)?.as_str())
}

pub fn dart_for_in_call<'a>(line: &'a str, name: &str) -> Option<(&'a str, bool)> {
    let re = Regex::new(&format!(
        r"(?:^|[^\w$.])(await\s+)?for\s*\(\s*(?:final|var)\s+{}\s+in\s+([A-Za-z_$][\w$]*)\s*\(\s*\)\s*\)",
        regex::escape(name)
    ))
    .ok()?;
    let c = re.captures(line)?;
    Some((c.get(2)?.as_str(), c.get(1).is_some()))
}

pub fn dart_read_links(value: &str) -> Option<Vec<String>> {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[a-z_$][\w$]*(?:\.[a-z_$][\w$]*)+$").expect("the regex is a literal")
    });
    PATTERN
        .is_match(value)
        .then(|| value.split('.').map(str::to_owned).collect())
}

pub fn dart_lambda_over<'a>(line: &'a str, name: &str) -> Option<(&'a str, bool)> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"(?:^|[^\w$.])(this\.)?([a-z_$][\w$]*)\s*\.\s*(forEach|map|where|any|every|firstWhere",
            r"|lastWhere|singleWhere|expand|takeWhile|skipWhile|removeWhere|retainWhere|sort|reduce)",
            r"\s*\(\s*\(\s*([a-z_$][\w$]*)(?:\s*,\s*([a-z_$][\w$]*))?\s*\)\s*(?:=>|\{)"
        ))
        .expect("the regex is a literal")
    });
    let mut found = CALL.captures_iter(line).filter(|c| {
        let pair = matches!(&c[3], "sort" | "reduce");
        match (c.get(5), pair) {
            (None, false) => &c[4] == name,
            (Some(second), true) => (&c[4] == name) != (second.as_str() == name),
            _ => false,
        }
    });
    let c = found.next()?;
    if found.next().is_some() || &c[2] == name {
        return None;
    }
    Some((c.get(2)?.as_str(), c.get(1).is_some()))
}

pub fn dart_lambda_callee<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let re = Regex::new(&format!(
        r"(?:^|[^\w$.])([a-z_$][\w$]*)\s*\(\s*\(\s*{}\s*\)\s*(?:=>|\{{)",
        regex::escape(name)
    ))
    .ok()?;
    let mut found = re.captures_iter(line);
    let callee = found.next()?.get(1)?.as_str();
    (found.next().is_none() && callee != name).then_some(callee)
}

pub fn dart_callback_param(text: &str, callee: &str) -> Option<DartType> {
    let re = Regex::new(&format!(
        r"^(\s*)[\w$<>?]+\s+{}\s*\(\s*(?:void\s+)?Function\s*\(\s*(?:([a-z_$][\w$]*)\.)?(_?[A-Z][\w$]*)(?:\s+[a-z_$][\w$]*)?\s*\)\??\s+[a-z_$][\w$]*\s*[,)]",
        regex::escape(callee)
    ))
    .ok()?;
    let mut found = text
        .lines()
        .filter_map(|l| re.captures(l))
        .filter(|c| c[1].is_empty());
    let c = found.next()?;
    found.next().is_none().then(|| DartType {
        prefix: c.get(2).map(|m| m.as_str().to_owned()),
        class: c[3].to_owned(),
        constructor: None,
    })
}

pub fn dart_await_operand(value: &str) -> Option<&str> {
    let rest = value.strip_prefix("await")?;
    rest.starts_with(char::is_whitespace).then(|| rest.trim())
}

pub fn dart_paren_awaited(before: &str) -> Option<&str> {
    let rest = before.strip_suffix('.')?;
    let open = super::code(super::Kind::Dart, rest)
        .filter(|(_, c)| *c == b'(')
        .map(|(i, _)| i)
        .find(|&i| super::close_of(super::Kind::Dart, rest, i) == Some(rest.len()))?;
    let head = rest.get(..open)?.trim_end();
    if head.ends_with(|c: char| c.is_alphanumeric() || "_$>)]?!.".contains(c)) {
        return None;
    }
    let inner = rest.get(open + 1..rest.len() - 1)?.trim();
    dart_await_operand(inner).map(|_| inner)
}
