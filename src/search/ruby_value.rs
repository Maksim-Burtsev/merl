use std::sync::LazyLock;

use regex::Regex;

use super::{
    Kind, bindings, indent, literal_lines, ruby_assignment, ruby_last_call, ruby_locals,
    ruby_receiver_split,
};

pub enum RubyReturns {
    Itself,
    New(String),
    Fresh,
    Found(String),
    Chain(usize, String),
}

fn continues(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('.') && !t.starts_with("..") || t.starts_with("&.")
}

fn skipped(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

pub fn ruby_joined_before(text: &str, line1: usize, before: &str) -> String {
    if !continues(before) {
        return before.to_owned();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut joined = before.trim_start().to_owned();
    for line in lines
        .get(..line1.saturating_sub(1))
        .unwrap_or_default()
        .iter()
        .rev()
    {
        if skipped(line) {
            continue;
        }
        joined.insert_str(0, line.trim());
        if !continues(line) {
            break;
        }
    }
    joined
}

pub fn ruby_local_name(expr: &str) -> bool {
    expr.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        && expr.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !matches!(
            expr,
            "self" | "nil" | "true" | "false" | "end" | "super" | "then" | "do" | "not"
        )
}

pub fn ruby_tapped<'a>(prefix: &'a str, name: &str) -> Option<&'a str> {
    static TAP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(.*)\.tap\s*(?:\{|\bdo)\s*\|\s*(\w+)\s*\|\s*$")
            .expect("the regex is a literal")
    });
    let c = TAP.captures(prefix)?;
    (&c[2] == name).then(|| c.get(1).map_or("", |m| m.as_str()))
}

pub fn ruby_assigned_once(
    text: &str,
    line1: usize,
    before: &str,
    name: &str,
) -> Option<(usize, String)> {
    let earlier: Vec<usize> = ruby_locals(text, line1, name)
        .into_iter()
        .filter(|&l| l < line1)
        .collect();
    let [at] = earlier[..] else {
        return None;
    };
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(at.checked_sub(1)?)?;
    let value = line.trim_start().strip_prefix(name)?.trim_start();
    let value = value
        .strip_prefix('=')
        .filter(|v| !v.starts_with(['=', '~', '>']))?;
    let mut value = value.trim().to_owned();
    for next in lines.get(at..line1.saturating_sub(1)).unwrap_or_default() {
        if skipped(next) {
            continue;
        }
        if !continues(next) {
            break;
        }
        value.push_str(next.trim());
    }
    (!ruby_rebound(text, &value, at, line1, before, name)).then_some((at, value))
}

pub fn ruby_constant_alias(name: &str) -> String {
    format!(r"^\s*{}\s*=\s*(?:::)?[A-Z][\w:]*\s*$", regex::escape(name))
}

pub fn ruby_alias_target<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let value = line
        .trim()
        .strip_prefix(name)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    value
        .trim_start_matches(':')
        .starts_with(|c: char| c.is_ascii_uppercase())
        .then_some(value)
}

pub fn ruby_last_value(text: &str, def_line1: usize) -> Option<RubyReturns> {
    static NEW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:((?:::)?[A-Z][\w:]*)\.)?new(?:\(.*\))?$").expect("the regex is a literal")
    });
    static RETURN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\breturn\b").expect("the regex is a literal"));
    let lines: Vec<&str> = text.lines().collect();
    let def = lines.get(def_line1.checked_sub(1)?)?;
    let t = def.trim_start();
    if !t.starts_with("def ") || t.contains('=') || t.contains(';') {
        return None;
    }
    let indent = def.len() - t.len();
    let body: Vec<&str> = lines
        .get(def_line1..)?
        .iter()
        .take_while(|l| skipped(l) || l.len() - l.trim_start().len() > indent)
        .copied()
        .collect();
    if !rescues_then_end(lines.get(def_line1 + body.len()..)?, indent)
        || body.iter().any(|l| RETURN.is_match(l))
    {
        return None;
    }
    let (at, last) = body.iter().enumerate().rev().find(|(_, l)| !skipped(l))?;
    let last = last.trim();
    if body[..at]
        .iter()
        .rev()
        .find(|l| !skipped(l))
        .is_some_and(|l| leaves_open(l))
    {
        return None;
    }
    if last == "self" {
        return Some(RubyReturns::Itself);
    }
    if let Some(c) = NEW.captures(last) {
        return Some(c.get(1).map_or(RubyReturns::Fresh, |m| {
            RubyReturns::New(m.as_str().to_owned())
        }));
    }
    if let Some(c) = ruby_found_record(last) {
        return Some(RubyReturns::Found(c.to_owned()));
    }
    let (memo, chain) = memo_value(last);
    (ruby_last_call(chain).is_some() && ruby_receiver_split(chain).0.trim().is_empty())
        .then(|| RubyReturns::Chain(def_line1 + at + 1, memo.unwrap_or(chain).to_owned()))
}

fn leaves_open(line: &str) -> bool {
    let t = line.trim_end();
    t.ends_with([
        '|', '&', '+', '-', '*', '/', '%', '=', '<', '>', ':', ',', '(', '[', '{', '\\', '^', '~',
    ]) || [" ?", " and", " or", " not"].iter().any(|w| t.ends_with(w))
}

fn memo_value(value: &str) -> (Option<&str>, &str) {
    static MEMO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(@[a-z_]\w*)\s*\|\|=\s*(.+)$").expect("the regex is a literal")
    });
    MEMO.captures(value)
        .and_then(|c| Some((Some(c.get(1)?.as_str()), c.get(2)?.as_str())))
        .unwrap_or((None, value))
}

fn rescues_then_end(rest: &[&str], indent: usize) -> bool {
    let mut clause = false;
    for line in rest {
        let t = line.trim_start();
        if skipped(line) {
            continue;
        }
        if line.len() - t.len() > indent {
            if !clause || !(t == "nil" || t.starts_with("raise ")) {
                return false;
            }
            continue;
        }
        if line.len() - t.len() < indent {
            return false;
        }
        if t == "end" || t.starts_with("end ") {
            return true;
        }
        if t != "rescue" && !t.starts_with("rescue ") {
            return false;
        }
        clause = true;
    }
    false
}

pub fn ruby_found_record(value: &str) -> Option<&str> {
    static FIND: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:@[a-z_]\w*\s*\|\|=\s*)?((?:::)?[A-Z][\w:]*)\.find\([^()]*\)$")
            .expect("the regex is a literal")
    });
    FIND.captures(value)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
}

pub fn ruby_struct_line(name: &str) -> String {
    format!(r"^\s*{}\s*=\s*(?:::)?Struct\.new\s*\(", regex::escape(name))
}

pub fn ruby_struct_member(line: &str, word: &str) -> bool {
    Regex::new(&format!(
        r"Struct\.new\s*\([^)]*:{}\s*[,)]",
        regex::escape(word)
    ))
    .is_ok_and(|re| re.is_match(line))
}

pub struct RubyRebinding {
    code: Regex,
    raw: Regex,
}

impl RubyRebinding {
    fn new(code: &str, raw: &str) -> Option<Self> {
        Some(Self {
            code: Regex::new(code).ok()?,
            raw: Regex::new(raw).ok()?,
        })
    }

    fn local(name: &str) -> Option<Self> {
        let n = regex::escape(name);
        Self::new(
            &format!(
                r"(?:^|[^@$:.\w]){n}\s*(?:[-+*/%|&^]|\*\*|<<|>>|&&|\|\|)?=(?:$|[^=~>])|^\s*[\w\s,*()]*\b{n}\b[\w\s,*()]*=(?:$|[^=~>])|=>\s*{n}(?:$|[^\w.(\[?!:&])|=>\s*[\[{{(].*\b{n}\b|(?:^|;)\s*in\b.*\b{n}\b|\bin\s*[\[{{(].*\b{n}\b|\bfor\s+[\w\s,]*\b{n}\b|->\s*\(?[^{{]*\b{n}\b|(?:\{{|\bdo)\s*\|[^|]*\b{n}\b[^|]*\||^\s*[\w\s,*()]*\b{n}\b[^|]*\|\s*$"
            ),
            &format!(r"\(\?<{n}>|variable_set\s*\(?\s*[:'\x22]{n}\b"),
        )
    }

    pub fn ivar(name: &str) -> Option<Self> {
        let n = regex::escape(name);
        Self::new(
            &format!(
                r"(?:^|[^@\w])@{n}\b\s*(?:[-+*/%|&^]|\*\*|<<|>>|&&|\|\|)?=(?:$|[^=~>])|(?:^|[^@\w])@{n}\b\s*,[^=]*=(?:$|[^=~>])|=>\s*@{n}\b|\bfor\s+[\w\s,@]*@{n}\b|\battr_(?:writer|accessor)\b.*:{n}\b"
            ),
            &format!(r"variable_set\s*\(?\s*[:'\x22]@{n}\b"),
        )
    }

    pub fn count(&self, line: &str) -> usize {
        self.code.find_iter(&code_only(line)).count() + self.raw.find_iter(line).count()
    }
}

fn code_only(line: &str) -> String {
    let mut code = vec![b' '; line.len()];
    for (i, c) in super::code(Kind::Ruby, line) {
        if c == 0 {
            break;
        }
        code[i] = c;
    }
    String::from_utf8_lossy(&code).into_owned()
}

static DEF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bdef\b").expect("the regex is a literal"));
static CLOSED_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<open>\{\s*)\|[^|]*\|(?P<body>[^{}]*\})").expect("the regex is a literal")
});

fn opens_a_def(lines: &[&str], at: usize, until: usize) -> bool {
    static ONE_LINE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\bend\s*$|\bdef\s+[\w.?!]+(?:\([^)]*\))?\s*=(?:$|[^=~>])")
            .expect("the regex is a literal")
    });
    static END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*end\b").expect("the regex is a literal"));
    let code = code_only(lines[at]);
    DEF.is_match(&code)
        && !ONE_LINE.is_match(&code)
        && !lines
            .get(at + 1..until)
            .unwrap_or_default()
            .iter()
            .any(|l| indent(l) == indent(lines[at]) && END.is_match(l))
}

fn closed_before(line: &str, before: &str, name: &str) -> Option<String> {
    let rest = line.strip_prefix(before)?.strip_prefix(name)?;
    (!before.is_empty()
        && code_only(before) == before
        && !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    .then(|| CLOSED_BLOCK.replace_all(before, "$open$body").into_owned())
}

pub fn ruby_rebound(
    text: &str,
    value: &str,
    after1: usize,
    line1: usize,
    before: &str,
    name: &str,
) -> bool {
    let Some(re) = RubyRebinding::local(name) else {
        return true;
    };
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Ruby, text);
    let line = |l1: usize| l1.checked_sub(1).and_then(|i| lines.get(i));
    let in_a_branch = line(after1)
        .zip(line(line1))
        .is_some_and(|(a, c)| indent(a) > indent(c));
    let assigned = Regex::new(&ruby_assignment(name)).ok();
    let cut = line(line1).and_then(|l| closed_before(l, before, name));
    let cursor_value = cut
        .as_deref()
        .or_else(|| line(line1).copied())
        .map_or("", |l| {
            assigned
                .as_ref()
                .and_then(|a| a.find(l))
                .and_then(|m| l.get(m.end()..))
                .filter(|tail| !tail.contains(';'))
                .unwrap_or(l)
        });
    let until = line1.saturating_sub(1).min(lines.len());
    re.count(value) > 0
        || re.count(cursor_value) > 0
        || DEF.is_match(&code_only(cursor_value))
        || (after1..until).any(|i| {
            !literal[i]
                && (re.count(&CLOSED_BLOCK.replace_all(lines[i], "$open$body")) > 0
                    || opens_a_def(&lines, i, until))
        })
        || in_a_branch
            && bindings(Kind::Ruby, text, line1, name)
                .iter()
                .any(|b| b.line1 != after1)
}

pub fn ruby_param_type(text: &str, line1: usize, name: &str) -> Option<String> {
    static TAG: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^#\s*@param\s+(?:\[([^\]]*)\]\s+([a-z_]\w*)|([a-z_]\w*)\s+\[([^\]]*)\])(?:\s|$)",
        )
        .expect("the regex is a literal")
    });
    static CONSTANT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:::)?[A-Z][\w:]*$").expect("the regex is a literal"));
    let bound = bindings(Kind::Ruby, text, line1, name);
    let [def] = bound.as_slice() else {
        return None;
    };
    let lines: Vec<&str> = text.lines().collect();
    let head = lines.get(def.line1.checked_sub(1)?)?;
    let piped = Regex::new(&format!(r"\|[^|]*\b{}\b[^|]*\|", regex::escape(name))).ok()?;
    let n = regex::escape(name);
    let splat = Regex::new(&format!(r"[*&]\s*{n}\b")).ok()?;
    let indent = head.len() - head.trim_start().len();
    let end = lines[def.line1..]
        .iter()
        .position(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() <= indent)
        .map_or(lines.len(), |p| def.line1 + p)
        .max(line1);
    if !head.trim_start().starts_with("def ")
        || splat.is_match(head)
        || !ruby_locals(text, line1, name).is_empty()
        || lines
            .get(def.line1 - 1..line1)?
            .iter()
            .any(|l| piped.is_match(l))
        || ruby_rebound(text, "", def.line1, end, "", name)
    {
        return None;
    }
    let types: Vec<&str> = lines[..def.line1 - 1]
        .iter()
        .rev()
        .map(|l| l.trim())
        .take_while(|l| l.starts_with('#'))
        .filter_map(|l| {
            let c = TAG.captures(l)?;
            let (ty, param) = match c.get(1) {
                Some(ty) => (ty, c.get(2)?),
                None => (c.get(4)?, c.get(3)?),
            };
            (param.as_str() == name).then_some(ty.as_str().trim())
        })
        .collect();
    let [ty] = types[..] else {
        return None;
    };
    CONSTANT.is_match(ty).then(|| ty.to_owned())
}
