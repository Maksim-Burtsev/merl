use regex::Regex;

use super::csharp::{CS_TYPE, cs_split};
use super::{Kind, close_of, code, cs_receiver, cs_steps, cs_type_name, uncommented};

pub fn cs_one_argument(written: &str) -> Option<String> {
    if let Some(e) = written.trim().trim_end_matches('?').strip_suffix("[]") {
        return (cs_type_name(e).is_some() && !e.contains('<')).then(|| e.trim().to_owned());
    }
    let t = written.trim().strip_suffix('>')?;
    let inner = t.get(t.find('<')? + 1..)?.trim();
    (!inner.contains(',') && !inner.is_empty()).then(|| inner.to_owned())
}

pub fn cs_collection(name: &str) -> bool {
    matches!(
        name,
        "IEnumerable"
            | "Array"
            | "IQueryable"
            | "IOrderedEnumerable"
            | "IOrderedQueryable"
            | "IAsyncEnumerable"
            | "ICollection"
            | "IList"
            | "IReadOnlyCollection"
            | "IReadOnlyList"
            | "ISet"
            | "IReadOnlySet"
            | "List"
            | "HashSet"
            | "SortedSet"
            | "LinkedList"
            | "Queue"
            | "Stack"
            | "Collection"
            | "ReadOnlyCollection"
            | "ObservableCollection"
            | "ImmutableArray"
            | "ImmutableList"
            | "ImmutableHashSet"
            | "DbSet"
    )
}

pub fn cs_select_lambda(args: &str) -> Option<(String, String)> {
    static LAMBDA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:@?(\w+)|\(\s*@?(\w+)\s*\))\s*=>\s*(.+)$").expect("the regex is a literal")
    });
    let c = LAMBDA.captures(args.trim())?;
    let param = c.get(1).or_else(|| c.get(2))?.as_str().to_owned();
    let body = c[3].trim();
    (!body.starts_with('{')).then(|| (param, body.to_owned()))
}

pub fn cs_coalesced(expr: &str) -> Option<(&str, &str)> {
    let b = expr.as_bytes();
    let mut depth = 0i32;
    let mut found = None;
    for (i, c) in code(Kind::CSharp, expr) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'?' if depth == 0 && b.get(i + 1) == Some(&b'?') && b.get(i + 2) != Some(&b'=') => {
                if found.is_some() {
                    return None;
                }
                found = Some(i);
            }
            _ => {}
        }
    }
    let at = found?;
    let (left, right) = (expr.get(..at)?.trim(), expr.get(at + 2..)?.trim());
    (!left.is_empty() && !right.is_empty() && !right.starts_with('?')).then_some((left, right))
}

pub fn cs_framework_value(expr: &str) -> Option<(String, String)> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"^(?:System\.Text\.Json\.)?(JsonSerializer)\.Deserialize\s*<({CS_TYPE})>\s*\(|^(?:System\.)?(Array)\.Empty\s*<({CS_TYPE})>\s*\("
        ))
        .expect("the regex is a literal")
    });
    let e = expr.trim();
    let c = CALL.captures(e)?;
    if close_of(Kind::CSharp, e, c.get_match().end() - 1) != Some(e.len()) {
        return None;
    }
    match (c.get(1), c.get(3)) {
        (Some(class), _) => Some((class.as_str().to_owned(), c[2].to_owned())),
        (None, Some(class)) if e.ends_with("()") => {
            Some((class.as_str().to_owned(), format!("{}[]", &c[4])))
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CsLinq {
    Same,
    Element,
}

pub fn cs_linq(method: &str) -> Option<CsLinq> {
    match method {
        "Where" | "OrderBy" | "OrderByDescending" | "ThenBy" | "ThenByDescending" | "Distinct"
        | "DistinctBy" | "Skip" | "Take" | "SkipWhile" | "TakeWhile" | "Reverse" | "ToList"
        | "ToArray" | "ToHashSet" | "AsEnumerable" | "AsQueryable" | "AsNoTracking" => {
            Some(CsLinq::Same)
        }
        "First" | "FirstOrDefault" | "Single" | "SingleOrDefault" | "Last" | "LastOrDefault"
        | "ElementAt" | "ElementAtOrDefault" | "MinBy" | "MaxBy" | "Find" => Some(CsLinq::Element),
        _ => None,
    }
}

fn cs_element_lambda(method: &str) -> bool {
    cs_linq(method).is_some()
        || matches!(
            method,
            "Select"
                | "SelectMany"
                | "Any"
                | "All"
                | "Count"
                | "LongCount"
                | "Sum"
                | "Min"
                | "Max"
                | "Average"
                | "GroupBy"
                | "ToDictionary"
                | "ToLookup"
                | "ForEach"
                | "FindAll"
                | "Exists"
                | "RemoveAll"
        )
}

pub fn cs_lambda_call<S: AsRef<str>>(lines: &[S], at: usize, name: &str) -> Option<String> {
    let n = regex::escape(name.trim_start_matches('@'));
    let re = Regex::new(&format!(
        r"\.\s*(\w+)\s*\(\s*(?:@?{n}|\(\s*@?{n}\s*\))\s*=>"
    ))
    .ok()?;
    let line = lines.get(at)?.as_ref();
    let code = uncommented(Kind::CSharp, line);
    let mut found = re.captures_iter(&code);
    let c = found.next()?;
    let method = c.get(1)?;
    if found.next().is_some() || !cs_element_lambda(method.as_str()) {
        return None;
    }
    let receiver = cs_receiver(lines, at, method.start())?;
    let steps = cs_steps(&receiver)?;
    let first = steps.parts.first().map(|(p, _)| p.as_str());
    (steps.new.is_none() && first != Some(name.trim_start_matches('@'))).then_some(receiver)
}

pub fn cs_rule_lambda(line: &str, name: &str) -> bool {
    let n = regex::escape(name.trim_start_matches('@'));
    let (Ok(rule), Ok(bound)) = (
        Regex::new(&format!(
            r"\b(?:RuleFor|RuleForEach|When)\s*\(\s*(?:@?{n}|\(\s*@?{n}\s*\))\s*=>"
        )),
        Regex::new(&format!(r"(?:^|[^\w@])@?{n}\s*\)?\s*=>")),
    ) else {
        return false;
    };
    let code = uncommented(Kind::CSharp, line);
    let mut found = rule.find_iter(&code);
    let Some(m) = found.next() else {
        return false;
    };
    let unqualified = code
        .get(..m.start())
        .is_some_and(|b| !b.trim_end().ends_with('.'));
    unqualified && found.next().is_none() && bound.find_iter(&code).count() == 1
}

pub fn cs_validated(base: &str) -> Option<String> {
    let t = base.trim();
    let t = t.strip_prefix("global::").unwrap_or(t);
    let t = t.strip_prefix("FluentValidation.").unwrap_or(t);
    let inner = t.strip_prefix("AbstractValidator")?.trim_start();
    let inner = inner.strip_prefix('<')?.strip_suffix('>')?.trim();
    (cs_split(inner).len() == 1 && cs_type_name(inner).is_some()).then(|| inner.to_owned())
}
