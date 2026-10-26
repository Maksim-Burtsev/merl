use regex::Regex;

use super::{Kind, uncommented};

const KEEPS_ELEMENT: &[&str] = &[
    "filter",
    "filterWhen",
    "doOnNext",
    "doOnSuccess",
    "doOnError",
    "doOnSubscribe",
    "doFinally",
    "switchIfEmpty",
    "defaultIfEmpty",
    "cache",
    "onErrorResume",
    "onErrorMap",
    "retry",
    "retryWhen",
    "timeout",
    "log",
    "distinct",
    "sorted",
    "take",
    "skip",
    "limit",
    "peek",
    "delayUntil",
    "next",
    "stream",
];

const WRAPPERS: &str = r"(?:Mono|Flux|Optional|Stream|List|ArrayList|LinkedList|Set|HashSet|LinkedHashSet|Collection|Iterable)";

pub struct JavaChainSource {
    pub receiver: Option<String>,
    pub call: Option<(String, String)>,
    pub reference: Option<(String, String)>,
}

pub fn java_lambda_source(text: &str, line1: usize, name: &str) -> Option<JavaChainSource> {
    let lines: Vec<&str> = text.lines().collect();
    let at = uncommented(Kind::Jvm, lines.get(line1.checked_sub(1)?)?);
    let lambda = Regex::new(&format!(
        r"\.\s*(?:(?:map|mapNotNull|filter|filterWhen|flatMap|flatMapMany|flatMapIterable|concatMap|doOnNext|doOnSuccess|delayUntil|forEach|anyMatch|allMatch|noneMatch|ifPresent|peek)\s*\(|collect\s*\(\s*(?:Collectors\s*\.\s*)?(?:toMap|groupingBy)\s*\()\s*(?:\(\s*{0}\s*\)|{0})\s*->",
        regex::escape(name)
    ))
    .ok()?;
    let mut found = lambda.find_iter(&at);
    let m = found.next()?;
    if found.next().is_some() {
        return None;
    }
    let mut prefix: Vec<char> = lines[..line1 - 1]
        .iter()
        .flat_map(|l| {
            uncommented(Kind::Jvm, l)
                .chars()
                .chain(['\n'])
                .collect::<Vec<_>>()
        })
        .collect();
    prefix.extend(at.get(..m.start())?.chars());
    chain_source(&prefix)
}

fn chain_source(mut rest: &[char]) -> Option<JavaChainSource> {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    loop {
        rest = trim_end(rest);
        if rest.last()? != &')' {
            let start = rest.iter().rposition(|&c| !ident(c)).map_or(0, |p| p + 1);
            let receiver: String = rest.get(start..)?.iter().collect();
            let head = trim_end(rest.get(..start)?);
            if !receiver.starts_with(|c: char| c.is_ascii_lowercase())
                || head.last().is_some_and(|&c| c == '.' || c == ')')
            {
                return None;
            }
            return Some(JavaChainSource {
                receiver: Some(receiver),
                call: None,
                reference: None,
            });
        }
        let open = matching_open(rest)?;
        let args: String = rest.get(open + 1..rest.len() - 1)?.iter().collect();
        let named = trim_end(rest.get(..open)?);
        let start = named.iter().rposition(|&c| !ident(c)).map_or(0, |p| p + 1);
        let method: String = named.get(start..)?.iter().collect();
        if !method.starts_with(|c: char| c.is_ascii_lowercase()) {
            return None;
        }
        let before = trim_end(named.get(..start)?);
        let Some(dot) = before.strip_suffix(&['.']) else {
            let word = before.iter().rposition(|&c| !ident(c)).map_or(0, |p| p + 1);
            let word: String = before.get(word..)?.iter().collect();
            if before.last() == Some(&')') || !(word.is_empty() || word == "return") {
                return None;
            }
            return Some(JavaChainSource {
                receiver: None,
                call: Some((method, args)),
                reference: None,
            });
        };
        if let ("map", Some(reference)) = (method.as_str(), method_reference(&args)) {
            return Some(JavaChainSource {
                receiver: None,
                call: None,
                reference: Some(reference),
            });
        }
        if KEEPS_ELEMENT.contains(&method.as_str()) {
            rest = dot;
            continue;
        }
        let dot = trim_end(dot);
        let start = dot.iter().rposition(|&c| !ident(c)).map_or(0, |p| p + 1);
        let receiver: String = dot.get(start..)?.iter().collect();
        let head = trim_end(dot.get(..start)?);
        if !receiver.starts_with(|c: char| c.is_ascii_lowercase())
            || head.last().is_some_and(|&c| c == '.' || c == ')')
        {
            return None;
        }
        return Some(JavaChainSource {
            receiver: Some(receiver),
            call: Some((method, args)),
            reference: None,
        });
    }
}

fn method_reference(args: &str) -> Option<(String, String)> {
    static REFERENCE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*([A-Z]\w*(?:\.[A-Z]\w*)*)\s*::\s*([a-z]\w*)\s*$")
            .expect("the regex is a literal")
    });
    let c = REFERENCE.captures(args)?;
    Some((c[1].to_owned(), c[2].to_owned()))
}

fn trim_end(s: &[char]) -> &[char] {
    let end = s
        .iter()
        .rposition(|c| !c.is_whitespace())
        .map_or(0, |p| p + 1);
    s.get(..end).unwrap_or_default()
}

fn matching_open(s: &[char]) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = s.len();
    while i > 0 {
        i -= 1;
        match s[i] {
            ')' => depth += 1,
            '(' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            '"' | '\'' => {
                let quote = s[i];
                i = (0..i)
                    .rev()
                    .find(|&j| s[j] == quote && (j == 0 || s[j - 1] != '\\'))?;
            }
            _ => {}
        }
    }
    None
}

pub fn java_class_argument(args: &str) -> Option<String> {
    static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*([A-Z]\w*(?:\.[A-Z]\w*)*)\.class\s*(?:,|$)")
            .expect("the regex is a literal")
    });
    Some(CLASS.captures(args)?[1].to_owned())
}

pub fn java_takes_class(line: &str, method: &str) -> bool {
    Regex::new(&format!(
        r"\b{}\s*\(\s*(?:final\s+)?Class\s*<",
        regex::escape(method)
    ))
    .is_ok_and(|re| re.is_match(&uncommented(Kind::Jvm, line)))
}

pub fn java_returns_class_element(line: &str, method: &str) -> bool {
    Regex::new(&format!(
        r"<\s*(\w+)\b[^<>]*(?:<[^<>]*>[^<>]*)*>\s+{WRAPPERS}<\s*(?:\?\s+extends\s+)?(\w+)\s*>\s+{}\s*\(\s*(?:final\s+)?Class\s*<\s*(?:\?\s+extends\s+)?(\w+)\s*>",
        regex::escape(method)
    ))
    .ok()
    .and_then(|re| {
        let code = uncommented(Kind::Jvm, line);
        let c = re.captures(&code)?;
        Some(c[1] == c[2] && c[2] == c[3])
    })
    .unwrap_or(false)
}

pub fn java_returned_element(line: &str, method: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"(?:^|[\s>]){WRAPPERS}<\s*(?:\?\s+extends\s+)?([A-Z]\w*(?:\.[A-Z]\w*)*)\s*>\s+{}\s*\(",
        regex::escape(method)
    ))
    .ok()?;
    Some(re.captures(&uncommented(Kind::Jvm, line))?[1].to_owned())
}

pub fn java_declared_element(line: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"(?:^|[\s(,]){WRAPPERS}<\s*(?:\?\s+extends\s+)?([A-Z]\w*(?:\.[A-Z]\w*)*)\s*>\s+{}\s*(?:[=;,)]|$)",
        regex::escape(name)
    ))
    .ok()?;
    Some(re.captures(&uncommented(Kind::Jvm, line))?[1].to_owned())
}
