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
