struct Syntax {
    line_comment: &'static str,
    block_comment: Option<(&'static str, &'static str)>,
    strings: &'static [&'static str],
}

const JSON: Syntax = Syntax {
    line_comment: "//",
    block_comment: Some(("/*", "*/")),
    strings: &["\""],
};

const CSS: Syntax = Syntax {
    line_comment: "",
    block_comment: Some(("/*", "*/")),
    strings: &["\"", "'"],
};

const SCSS: Syntax = Syntax {
    line_comment: "//",
    ..CSS
};

pub(super) fn css(lines: &[String]) -> Vec<(usize, usize)> {
    brackets(&code(lines, &CSS), b"{", true)
}

pub(super) fn scss(lines: &[String]) -> Vec<(usize, usize)> {
    brackets(&code(lines, &SCSS), b"{", true)
}

pub(super) fn json(lines: &[String]) -> Vec<(usize, usize)> {
    brackets(&code(lines, &JSON), b"{[", false)
}

fn code(lines: &[String], syntax: &Syntax) -> Vec<Vec<u8>> {
    let mut open: Option<(&str, bool)> = None;
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let b = line.as_bytes();
        let mut kept = vec![b' '; b.len()];
        let mut i = 0;
        while i < b.len() {
            if let Some((close, comment)) = open {
                if !comment && b[i] == b'\\' {
                    i += 2;
                } else if b[i..].starts_with(close.as_bytes()) {
                    open = None;
                    i += close.len();
                } else {
                    i += 1;
                }
                continue;
            }
            let rest = &b[i..];
            let url = i > 0 && matches!(b[i - 1], b':' | b'(');
            if !syntax.line_comment.is_empty()
                && rest.starts_with(syntax.line_comment.as_bytes())
                && !url
            {
                break;
            }
            if let Some((start, end)) = syntax.block_comment
                && rest.starts_with(start.as_bytes())
            {
                open = Some((end, true));
                i += start.len();
                continue;
            }
            if let Some(q) = syntax
                .strings
                .iter()
                .find(|q| rest.starts_with(q.as_bytes()))
            {
                kept[i] = b'"';
                open = Some((q, false));
                i += q.len();
                continue;
            }
            kept[i] = b[i];
            i += 1;
        }
        if let Some((close, false)) = open
            && close.len() == 1
            && !line.ends_with('\\')
        {
            open = None;
        }
        out.push(kept);
    }
    out
}

fn brackets(code: &[Vec<u8>], opens: &[u8], statements: bool) -> Vec<(usize, usize)> {
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut folds = Vec::new();
    let mut start = None;
    for (n, b) in code.iter().enumerate() {
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            i += 1;
            if statements && c == b'#' && b.get(i) == Some(&b'{') {
                i += b[i..]
                    .iter()
                    .position(|&c| c == b'}')
                    .map_or(b.len() - i, |p| p + 1);
                continue;
            }
            if opens.contains(&c) {
                stack.push((if statements { start.unwrap_or(n) } else { n }, n));
                start = None;
            } else if let Some(o) = match c {
                b'}' => Some(b'{'),
                b']' => Some(b'['),
                _ => None,
            } && opens.contains(&o)
            {
                if let Some((h, _)) = stack.pop()
                    && n > h
                {
                    folds.push((h, n));
                }
                start = None;
            } else if c == b';' {
                start = None;
            } else if !c.is_ascii_whitespace() && start.is_none() {
                start = Some(n);
            }
        }
    }
    outermost(folds)
}

fn outermost(mut folds: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    folds.sort_unstable_by_key(|&(h, e)| (h, std::cmp::Reverse(e)));
    folds.dedup_by_key(|f| f.0);
    folds
}

pub(super) fn yaml(lines: &[String]) -> Vec<(usize, usize)> {
    let item = |t: &str| t == "-" || t.starts_with("- ");
    let code = |t: &'_ str| t.split(" #").next().unwrap_or("").trim_end().to_owned();
    let mut folds = Vec::new();
    for h in 0..lines.len() {
        let head = lines[h].trim_start();
        if matches!(head.bytes().next(), None | Some(b'#')) {
            continue;
        }
        let (ind, value) = (indent(&lines[h]), code(head));
        let key = !item(head) && value.ends_with(':');
        let mapping = item(head) && (value.contains(": ") || value.ends_with(':'));
        let scalar = (value.rsplit(' ').next()).is_some_and(|w| {
            w.starts_with(['|', '>'])
                && w[1..]
                    .bytes()
                    .all(|c| matches!(c, b'-' | b'+' | b'0'..=b'9'))
        });
        let mut end = None;
        for (i, line) in lines.iter().enumerate().skip(h + 1) {
            let t = line.trim_start();
            match t.bytes().next() {
                None => continue,
                Some(b'#') if !(scalar && indent(line) > ind) => {
                    if (end.is_some() || mapping) && !scalar {
                        end = Some(i);
                    }
                    continue;
                }
                _ => {}
            }
            if indent(line) > ind || (indent(line) == ind && key && item(t)) {
                end = Some(i);
                continue;
            }
            if indent(line) == ind && t.starts_with([']', '}']) {
                end = Some(i);
            }
            break;
        }
        folds.extend(end.map(|e| (h, e)));
    }
    folds
}

fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}
