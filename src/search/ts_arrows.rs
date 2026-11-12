use regex::Regex;

use super::{Kind, Value, close_of, code, return_type, split_top, ts_callback_element};

pub fn ts_arrow_binds(line: &str, name: &str, at: usize) -> bool {
    ts_arrows(line, name).iter().any(|a| a.holds(at))
}
pub(super) fn ts_arrow_element(text: &str, line: &str, name: &str, at: usize) -> Option<Value> {
    let a = ts_arrows(line, name)
        .into_iter()
        .filter(|a| a.holds(at))
        .max_by_key(|a| a.body_start)?;
    let n = regex::escape(name);
    let rebinds = Regex::new(&format!(
        r"\b(?:const|let|var)\s+{n}\b|\bcatch\s*\(\s*{n}\b|\bfunction\b[^(]*\([^)]*\b{n}\b"
    ))
    .ok()?;
    let bound = Regex::new(&format!(r"(?:^|[{{\[,:]|\.\.\.)\s*{n}\s*(?:$|[}}\],=])")).ok()?;
    let body = line.get(a.body_start..at)?;
    if rebinds.is_match(body) || destructures(body, &bound) {
        return None;
    }
    let (before, params) = (line.get(..a.head)?, line.get(a.params)?);
    let value = ts_callback_element(before, params, name)
        .or_else(|| ts_callee_param(text, before, params, name))?;
    match &value {
        Value::Element(over) if over.split('.').next() == Some(name) => None,
        _ => Some(value),
    }
}
fn destructures(body: &str, bound: &Regex) -> bool {
    static PATTERN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:const|let|var)\s*[\[{]").expect("the regex is a literal")
    });
    PATTERN.find_iter(body).any(|m| {
        let close = close_of(Kind::TsJs, body, m.end() - 1).unwrap_or(body.len() + 1);
        body.get(m.end()..close - 1)
            .is_some_and(|p| bound.is_match(p))
    })
}
fn ts_callee_param(text: &str, before: &str, params: &str, name: &str) -> Option<Value> {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*$").expect("the regex is a literal")
    });
    let mut open = Vec::new();
    for (i, c) in code(Kind::TsJs, before) {
        match c {
            b'(' | b'[' | b'{' => open.push((i, c)),
            b')' | b']' | b'}' => {
                open.pop();
            }
            _ => {}
        }
    }
    let (open, b'(') = open.pop()? else {
        return None;
    };
    let args = split_top(Kind::TsJs, before.get(open + 1..)?, b',');
    if !args.last()?.trim().is_empty() {
        return None;
    }
    let head = before.get(..open)?.trim_end();
    let receiver = head.trim_end_matches(|c: char| c.is_alphanumeric() || "_$".contains(c));
    let callee = head
        .get(receiver.len()..)
        .filter(|c| !c.is_empty() && !receiver.trim_end().ends_with('.'))?;
    let word = Regex::new(&format!(r"(?:^|[^\w$]){}\b", regex::escape(callee))).ok()?;
    let only_called = word.find_iter(text).all(|m| {
        text.get(m.end()..)
            .is_some_and(|t| t.trim_start().starts_with('('))
    });
    let declared = Regex::new(&format!(
        r"(?m)^(?:export\s+)?(?:async\s+)?function\s+{}\s*\(",
        regex::escape(callee)
    ))
    .ok()?;
    let mut headers = declared.find_iter(text);
    let (Some(header), None, true) = (headers.next(), headers.next(), only_called) else {
        return None;
    };
    let close = close_of(Kind::TsJs, text, header.end() - 1)?;
    let declared = split_top(Kind::TsJs, text.get(header.end()..close - 1)?, b',');
    let (_, f) = declared.get(args.len() - 1)?.split_once(':')?;
    let f = f.trim();
    let f_close = close_of(Kind::TsJs, f, 0).filter(|_| f.starts_with('('))?;
    f.get(f_close..)?
        .trim_start()
        .starts_with("=>")
        .then_some(())?;
    let at = split_top(Kind::TsJs, params, b',')
        .iter()
        .position(|p| p.trim() == name)?;
    let (_, t) = split_top(Kind::TsJs, f.get(1..f_close - 1)?, b',')
        .get(at)?
        .split_once(':')?;
    TYPE.is_match(t.trim())
        .then(|| Value::Type(t.trim().to_owned()))
}
pub fn ts_arrow_param(line: &str, name: &str, at: usize) -> bool {
    let (Some(before), Some(after)) = (line.get(..at), line.get(at + name.len()..)) else {
        return false;
    };
    let (before, after) = (before.trim_end(), after.trim_start());
    let returned = before
        .strip_suffix(':')
        .is_some_and(|b| b.trim_end().ends_with(')'));
    let slot = (after.starts_with("=>") && !returned)
        || before.ends_with(['(', ','])
        || before.ends_with("...");
    slot && ts_arrows(line, name).iter().any(|a| a.param_at == at)
}
struct TsArrow {
    param_at: usize,
    head: usize,
    params: std::ops::Range<usize>,
    body_start: usize,
    body_end: usize,
}
impl TsArrow {
    fn holds(&self, at: usize) -> bool {
        self.param_at < at && at > self.body_start && at < self.body_end
    }
}
fn ts_arrows(line: &str, name: &str) -> Vec<TsArrow> {
    let Ok(word) = Regex::new(&format!(r"(?:^|[^\w$.]){}\b", regex::escape(name))) else {
        return Vec::new();
    };
    if code(Kind::TsJs, &format!("{line}\n)")).last() != Some((line.len() + 1, b')')) {
        return Vec::new();
    }
    let b = line.as_bytes();
    let bytes: Vec<(usize, u8)> = code(Kind::TsJs, line)
        .take_while(|&(_, c)| c != 0)
        .collect();
    let last_open_bracket_before = |upto: usize| {
        let mut stack = Vec::new();
        for &(i, c) in bytes.iter().take_while(|&&(i, _)| i < upto) {
            match c {
                b'(' | b'[' | b'{' => stack.push(i),
                b')' | b']' | b'}' => {
                    stack.pop();
                }
                _ => {}
            }
        }
        stack.last().copied()
    };
    let arrow_after_params = |params_end: usize, behind_return_type: bool| {
        let t = line.get(params_end..)?.trim_start();
        let t = match t.strip_prefix(':').filter(|_| behind_return_type) {
            Some(ty) => ty
                .find("=>")
                .filter(|&e| {
                    ty.get(..e)
                        .is_some_and(|h| !h.contains([';', '{', '}', '(', ')']))
                })
                .and_then(|e| ty.get(e..))?,
            None => t,
        };
        t.starts_with("=>").then(|| line.len() - t.len())
    };
    word.find_iter(line)
        .map(|m| m.end() - name.len())
        .filter_map(|decl| {
            let bare = arrow_after_params(decl + name.len(), false)
                .map(|start| (start, decl, decl..decl + name.len()));
            let (start, head, params) = bare.or_else(|| {
                let open = last_open_bracket_before(decl).filter(|&p| b[p] == b'(')?;
                let before = line.get(..open)?.trim_end();
                if return_type(before) || before.ends_with('<') {
                    return None;
                }
                let close = close_of(Kind::TsJs, line, open)?;
                Some((arrow_after_params(close, true)?, open, open + 1..close - 1))
            })?;
            let (mut depth, mut ternaries_awaiting_colon) = (0i32, 0i32);
            let end = bytes
                .iter()
                .skip_while(|&&(i, _)| i < start + 2)
                .find(|&&(i, c)| match c {
                    b'(' | b'[' | b'{' => {
                        depth += 1;
                        false
                    }
                    b')' | b']' | b'}' => {
                        depth -= 1;
                        depth < 0
                    }
                    b',' | b';' => depth == 0,
                    b'?' if depth == 0 => {
                        let chain = b[i - 1] == b'?' || matches!(b.get(i + 1), Some(b'.' | b'?'));
                        ternaries_awaiting_colon += i32::from(!chain);
                        false
                    }
                    b':' if depth == 0 => {
                        ternaries_awaiting_colon -= 1;
                        ternaries_awaiting_colon < 0
                    }
                    b'=' if depth == 0 => {
                        !b"=<>!".contains(&b[i - 1])
                            && !b.get(i + 1).is_some_and(|c| b"=>{\"'".contains(c))
                    }
                    _ => false,
                })
                .map_or(line.len(), |&(i, _)| i);
            Some(TsArrow {
                param_at: decl,
                head,
                params,
                body_start: start,
                body_end: end,
            })
        })
        .collect()
}
