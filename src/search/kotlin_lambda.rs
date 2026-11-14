use regex::Regex;

use super::jvm::{anonymous, jvm_bindings, jvm_members_of, opens_type, scope_of};
use super::{Kind, jvm_rebound_before, path_bindings, uncommented};

pub fn kotlin_lambda_collection(
    path: &std::path::Path,
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"(?:^|[^\w.])([a-z]\w*|[A-Z]\w*\s*\.\s*entries)\s*\.\s*(?:forEach|map|mapNotNull|flatMap|filter|filterNot|sumOf",
            r"|any|all|none|count|first|firstOrNull|last|lastOrNull|find|findLast|single|singleOrNull",
            r"|onEach|sortedBy|sortedByDescending|maxBy|minBy|maxByOrNull|minByOrNull|maxOf|minOf",
            r"|groupBy|associateBy|associateWith|partition|takeWhile|dropWhile|distinctBy)\s*\{$"
        ))
        .expect("the regex is a literal")
    });
    static COMPARE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:^|[^\w.])([a-z]\w*)\s*\.\s*(?:sortWith|sortedWith|maxWith|minWith|maxWithOrNull|minWithOrNull)\s*\{$",
        )
        .expect("the regex is a literal")
    });
    let (_, head, paired) = kotlin_binding_lambda(path, text, line1, before, name)?;
    let call = if paired { &COMPARE } else { &CALL };
    Some(call.captures(&head)?[1].replace(char::is_whitespace, ""))
}

fn kotlin_binding_lambda(
    path: &std::path::Path,
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
) -> Option<(usize, String, bool)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
    if let Some((head, bound)) = kotlin_lambda_head(text, line1, before)
        && bound.split(',').any(|b| b == name)
    {
        let code = uncommented(Kind::Jvm, before).replace("?.", ".");
        if code.contains('{')
            && let Some(body) = code.get(head.len()..)
        {
            let body = body.replacen("->", ";", 1);
            let paired = bound.contains(',');
            return (!jvm_rebound_before(&body, name)).then_some((at, head, paired));
        }
    }
    let nearest: Vec<usize> = jvm_bindings(path, &lines, at, name, None)
        .iter()
        .map(|b| b.line1)
        .collect();
    for _ in 0..6 {
        let s = scope_of(&lines, at)?;
        let code = uncommented(Kind::Jvm, lines[s]);
        let fun = code
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "fun");
        if opens_type(&code) || (fun && fun_header(&code).is_none_or(|(_, block)| block)) {
            return None;
        }
        let opens = code.trim_end().ends_with('{') || code.trim_end().ends_with("->");
        if opens
            && let Some((head, bound)) = kotlin_lambda_head(text, s + 1, &code)
            && bound == name
        {
            return (name == "it" || nearest == [s + 1]).then_some((s, head, false));
        }
        if fun {
            return None;
        }
        at = s;
    }
    None
}

pub fn kotlin_lambda_callee(
    path: &std::path::Path,
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.@])([a-z]\w*)\s*\{$").expect("the regex is a literal")
    });
    let (mut at, head, paired) = kotlin_binding_lambda(path, text, line1, before, name)?;
    (!paired && head.matches('{').count() == head.matches('}').count() + 1).then_some(())?;
    fun_header(&head)
        .is_none_or(|(fun, _)| !fun.contains('.'))
        .then_some(())?;
    let callee = CALL.captures(&head)?[1].to_owned();
    let lines: Vec<&str> = text.lines().collect();
    let class = loop {
        let scope = scope_of(&lines, at)?;
        let code = uncommented(Kind::Jvm, lines[scope]);
        if opens_type(&code) && !anonymous(&code) {
            let generic = code.split('(').next().is_some_and(|h| h.contains('<'));
            break (!generic).then_some(scope)?;
        }
        let (fun, block) = fun_header(&code)?;
        (block && !fun.contains('.')).then_some(())?;
        at = scope;
    };
    let named = |pattern: &str| Regex::new(&pattern.replace("NAME", &regex::escape(&callee)));
    let declared = named(r"\bfun\s+(?:[^\s(=]+\.)?NAME\s*[(<]").ok()?;
    let typed = named(concat!(
        r"^\s*(?:(?:private|protected|internal|public|inline|open|override|final)\s+)*fun\s+NAME",
        r"\s*\(\s*(?:(?:noinline|crossinline)\s+)?\w+\s*:\s*\(\s*([A-Z]\w*)\s*\)\s*->\s*[\w.?]+\s*\)"
    ))
    .ok()?;
    let mut decls =
        (0..lines.len()).filter(|&i| declared.is_match(&uncommented(Kind::Jvm, lines[i])));
    let only = decls.next()?;
    decls.next().is_none().then_some(())?;
    (jvm_members_of(path, text, class + 1, &callee) == [only + 1]).then_some(())?;
    path_bindings(Kind::Jvm, path, text, line1, &callee)
        .is_empty()
        .then_some(())?;
    Some(typed.captures(lines[only])?[1].to_owned())
}

fn fun_header(code: &str) -> Option<(String, bool)> {
    static FUN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"\bfun\s+(?:<[^>]*>\s*)?([^\s(=]+)\s*\((?:[^()]|\([^()]*\))*\)(\s*(?::[^={]*)?\{\s*$)?",
        )
        .expect("the regex is a literal")
    });
    let c = FUN.captures(code)?;
    Some((c[1].to_owned(), c.get(2).is_some()))
}

pub fn kotlin_lambda_subject(text: &str, line1: usize, before: &str, name: &str) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[^\w.])([a-z]\w*)\s*\.\s*(?:let|also)\s*\{$")
            .expect("the regex is a literal")
    });
    let (head, bound) = kotlin_lambda_head(text, line1, before)?;
    (bound == name).then_some(())?;
    Some(CALL.captures(&head)?[1].to_owned())
}

pub fn kotlin_lambda_receiver(text: &str, line1: usize, before: &str) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?:(?:^|[^\w.])([a-z]\w*)\s*\.\s*(?:run|apply)|\bwith\s*\(\s*([a-z]\w*)\s*\))\s*\{$",
        )
        .expect("the regex is a literal")
    });
    let (head, bound) = kotlin_lambda_head(text, line1, before)?;
    (bound == "it").then_some(())?;
    let c = CALL.captures(&head)?;
    Some(c.get(1).or_else(|| c.get(2))?.as_str().to_owned())
}

pub fn jvm_receiver_lambda(text: &str, line1: usize, before: &str) -> bool {
    static GROOVY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\.\s*(?:with|tap)\s*\{$").expect("the regex is a literal")
    });
    kotlin_lambda_head(text, line1, before)
        .is_some_and(|(head, bound)| bound == "it" && GROOVY.is_match(&head))
}

pub fn kotlin_scope_returned(before: &str) -> Option<String> {
    static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\.\s*(?:apply|also)\s*$").expect("the regex is a literal")
    });
    let close = before.rfind("}.")?;
    let tail = before.get(close + 1..)?;
    if !tail
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return None;
    }
    let body = before.get(..close)?;
    let mut depth = 1usize;
    let open = body.char_indices().rev().find_map(|(i, c)| {
        match c {
            '}' => depth += 1,
            '{' => depth -= 1,
            _ => {}
        }
        (depth == 0).then_some(i)
    })?;
    let head = body.get(..open)?;
    let m = CALL.find(head)?;
    Some(format!("{}{tail}", head.get(..m.start())?))
}

fn kotlin_lambda_head(text: &str, line1: usize, before: &str) -> Option<(String, String)> {
    static PARAMETER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*([A-Za-z_]\w*(?:\s*,\s*[A-Za-z_]\w*)?)\s*->")
            .expect("the regex is a literal")
    });
    let innermost = |code: &str| {
        let mut open = Vec::new();
        for (i, c) in code.char_indices() {
            match c {
                '{' => open.push(i),
                '}' => {
                    open.pop();
                }
                _ => {}
            }
        }
        open.pop()
    };
    let mut code = uncommented(Kind::Jvm, before).replace("?.", ".");
    if innermost(&code).is_none() {
        if code.contains('}') {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let at = line1.checked_sub(1).filter(|&i| i < lines.len())?;
        code = uncommented(Kind::Jvm, lines[scope_of(&lines, at)?]).replace("?.", ".");
        let tail = code.get(innermost(&code)? + 1..)?;
        if !(tail.trim().is_empty()
            || PARAMETER
                .find(tail)
                .is_some_and(|m| m.end() == tail.trim_end().len()))
        {
            return None;
        }
    }
    let brace = innermost(&code)?;
    let inside = code.get(brace + 1..)?;
    let bound = match PARAMETER.captures(inside) {
        Some(c) => c[1].replace(char::is_whitespace, ""),
        None if inside.contains("->") => return None,
        None => "it".to_owned(),
    };
    Some((code.get(..=brace)?.to_owned(), bound))
}
