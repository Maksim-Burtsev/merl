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

const TOML: Syntax = Syntax {
    line_comment: "#",
    block_comment: None,
    strings: &["\"\"\"", "'''", "\"", "'"],
};

pub(super) fn toml(lines: &[String]) -> Vec<(usize, usize)> {
    let code = code(lines, &TOML);
    let mut folds = brackets(&code, b"[{", false);
    let mut depth = 0usize;
    let mut tables = Vec::new();
    for (n, b) in code.iter().enumerate() {
        if depth == 0 && b.trim_ascii_start().first() == Some(&b'[') {
            tables.push(n);
        }
        for c in b {
            match c {
                b'[' | b'{' => depth += 1,
                b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let ends = tables
        .iter()
        .skip(1)
        .map(|&n| n - 1)
        .chain([lines.len().saturating_sub(1)]);
    folds.extend(tables.iter().copied().zip(ends).filter(|(h, e)| e > h));
    innermost(folds)
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
    innermost(folds)
}

fn innermost(mut folds: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    folds.sort_unstable();
    folds.dedup();
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
            w.strip_prefix(['|', '>']).is_some_and(|flags| {
                flags
                    .bytes()
                    .all(|c| matches!(c, b'-' | b'+' | b'0'..=b'9'))
            })
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

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const CLOSES_P: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "details",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "ul",
];

fn contains(parent: &str, child: &str) -> bool {
    match parent {
        "li" => child != "li",
        "dt" | "dd" => !matches!(child, "dt" | "dd"),
        "p" => !CLOSES_P.contains(&child),
        "colgroup" => child == "col",
        "rb" | "rt" | "rp" => !matches!(child, "rb" | "rt" | "rp"),
        "optgroup" => child != "optgroup",
        "tr" => child != "tr",
        "td" | "th" => !matches!(child, "td" | "th" | "tr"),
        _ => true,
    }
}

pub(super) fn html(lines: &[String]) -> Vec<(usize, usize)> {
    markup(lines, false)
}

pub(super) fn xml(lines: &[String]) -> Vec<(usize, usize)> {
    markup(lines, true)
}

struct Markup<'a> {
    b: &'a [u8],
    lower: String,
    starts: Vec<usize>,
    xml: bool,
}

impl Markup<'_> {
    fn line(&self, at: usize) -> usize {
        self.starts.partition_point(|&s| s <= at) - 1
    }

    fn find(&self, from: usize, what: &str) -> usize {
        (self.lower.get(from..).and_then(|r| r.find(what))).map_or(self.b.len(), |i| from + i)
    }

    fn name_at(&self, from: usize) -> String {
        let (b, xml) = (self.b, self.xml);
        let n = b[from..]
            .iter()
            .position(|&c| {
                !(c.is_ascii_alphanumeric()
                    || matches!(c, b'-' | b':' | b'_')
                    || (xml && c == b'.'))
            })
            .unwrap_or(b.len() - from);
        String::from_utf8_lossy(&self.lower.as_bytes()[from..from + n]).into_owned()
    }

    fn doctype_end(&self, i: usize) -> usize {
        let mut end = self.tag_end(i);
        let subset = self.find(i, "[");
        if subset < end {
            end = self.tag_end(self.find(subset, "]"));
        }
        end
    }

    fn close_tag(
        &self,
        i: usize,
        open: &mut Vec<(String, usize)>,
        folds: &mut Vec<(usize, usize)>,
    ) -> usize {
        let name = self.name_at(i + 2);
        let end = self.tag_end(i);
        if let Some(k) = open.iter().rposition(|(n, _)| *n == name) {
            for (_, h) in open.drain(k + 1..) {
                close_span(h, self.line(i), folds);
            }
            let (_, h) = open.pop().unwrap_or_default();
            close_span(h, self.line(end.min(self.b.len().saturating_sub(1))), folds);
        }
        end
    }

    #[expect(
        clippy::match_same_arms,
        reason = "the scan's arms are grouped by state, the quoted ones first; a merge would mix them"
    )]
    fn tag_end(&self, from: usize) -> usize {
        let b = self.b;
        let mut quote = None;
        for (i, &c) in b.iter().enumerate().skip(from) {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == b'"' || c == b'\'' => quote = Some(c),
                None if c == b'>' => return i,
                None => {}
            }
        }
        b.len()
    }
}

fn close_span(h: usize, e: usize, folds: &mut Vec<(usize, usize)>) {
    if e > h {
        folds.push((h, e));
    }
}

fn markup(lines: &[String], xml: bool) -> Vec<(usize, usize)> {
    let text = lines.join("\n");
    let b = text.as_bytes();
    let starts: Vec<usize> = std::iter::once(0)
        .chain(
            b.iter()
                .enumerate()
                .filter(|c| *c.1 == b'\n')
                .map(|(i, _)| i + 1),
        )
        .collect();
    let lower = if xml {
        text.clone()
    } else {
        text.to_ascii_lowercase()
    };
    let m = Markup {
        b,
        lower,
        starts,
        xml,
    };
    let mut open: Vec<(String, usize)> = Vec::new();
    let mut folds = Vec::new();
    let mut at = 0;
    while let Some(i) =
        (b.get(at..).and_then(|r| r.iter().position(|&c| c == b'<'))).map(|i| at + i)
    {
        let rest = &b[i + 1..];
        if rest.starts_with(b"!--") {
            at = m.find(i + 4, "-->") + 3;
            if xml {
                close_span(m.line(i), m.line(at.min(b.len()) - 1), &mut folds);
            }
            continue;
        }
        if xml && rest.starts_with(b"![CDATA[") {
            at = m.find(i + 9, "]]>") + 3;
            continue;
        }
        if xml && rest.starts_with(b"!DOCTYPE") {
            let end = m.doctype_end(i);
            close_span(
                m.line(i),
                m.line(end.min(b.len().saturating_sub(1))),
                &mut folds,
            );
            at = end + 1;
            continue;
        }
        if rest.starts_with(b"!") || rest.starts_with(b"?") {
            at = m.tag_end(i) + 1;
            continue;
        }
        if rest.first() == Some(&b'/') {
            at = m.close_tag(i, &mut open, &mut folds) + 1;
            continue;
        }
        if !rest.first().is_some_and(u8::is_ascii_alphabetic) {
            at = i + 1;
            continue;
        }
        let name = m.name_at(i + 1);
        while let Some((parent, h)) = open.last()
            && !xml
            && !contains(parent, &name)
        {
            close_span(*h, m.line(i), &mut folds);
            open.pop();
        }
        let end = m.tag_end(i);
        at = end + 1;
        if !xml && matches!(name.as_str(), "script" | "style") {
            let stop = m.tag_end(m.find(at.min(b.len()), &format!("</{name}")));
            close_span(
                m.line(i),
                m.line(stop.min(b.len().saturating_sub(1))),
                &mut folds,
            );
            at = stop + 1;
        } else if b.get(end.wrapping_sub(1)) == Some(&b'/') {
            if xml {
                close_span(
                    m.line(i),
                    m.line(end.min(b.len().saturating_sub(1))),
                    &mut folds,
                );
            }
        } else if xml || !VOID.contains(&name.as_str()) {
            open.push((name, m.line(i)));
        }
    }
    let last = lines.len().saturating_sub(1);
    for (_, h) in open {
        close_span(h, last, &mut folds);
    }
    innermost(folds)
}

enum Frame {
    List {
        bullet: u8,
        within: usize,
        start: usize,
        last: usize,
    },
    Item {
        col: usize,
        nested: bool,
        start: usize,
        last: usize,
    },
}

impl Frame {
    fn reach(&mut self, n: usize) {
        let (Self::List { last, .. } | Self::Item { last, .. }) = self;
        *last = n;
    }
}

fn columns(line: &str) -> usize {
    (line.bytes().take(indent(line))).fold(0, |c, b| if b == b'\t' { c / 4 * 4 + 4 } else { c + 1 })
}

fn marker(line: &str) -> Option<(u8, usize)> {
    let (ind, t) = (columns(line), line.trim_start());
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    let (bullet, len) = match t.bytes().next()? {
        c @ (b'-' | b'*' | b'+') => (c, 1),
        _ if (1..10).contains(&digits) && matches!(t.as_bytes().get(digits), Some(b'.' | b')')) => {
            (t.as_bytes()[digits], digits + 1)
        }
        _ => return None,
    };
    let after = &t.as_bytes()[len..];
    let gap = after.iter().take_while(|&&c| c == b' ').count();
    match after.is_empty() {
        true => Some((bullet, ind + len + 1)),
        false if (1..=4).contains(&gap) => Some((bullet, ind + len + gap)),
        false if gap > 4 => Some((bullet, ind + len + 1)),
        false => None,
    }
}

fn heading(t: &str) -> Option<usize> {
    let rest = t.trim_start_matches('#');
    let level = t.len() - rest.len();
    ((1..=6).contains(&level) && (rest.is_empty() || rest.starts_with([' ', '\t'])))
        .then_some(level)
}

fn rule(t: &str) -> bool {
    let t = t.trim_end();
    let c = t.bytes().next();
    matches!(c, Some(b'-' | b'*' | b'_'))
        && t.bytes().filter(|&b| Some(b) == c).count() >= 3
        && t.bytes().all(|b| Some(b) == c || b == b' ' || b == b'\t')
}

fn fold_frame(f: &Frame, folds: &mut Vec<(usize, usize)>) {
    let (start, last, wanted) = match *f {
        Frame::List {
            within,
            start,
            last,
            ..
        } => (start, last, within == 0),
        Frame::Item {
            nested,
            start,
            last,
            ..
        } => (start, last, nested),
    };
    if wanted && last > start {
        folds.push((start, last));
    }
}

fn front_matter_end(lines: &[String]) -> usize {
    match lines.first().map(|l| l.trim_end()) {
        Some("---") => (1..lines.len())
            .find(|&l| lines[l].trim_end() == "---")
            .map_or(0, |l| l + 1),
        _ => 0,
    }
}

fn fence_closes(t: &str, ind: usize, open: &str, at: usize) -> bool {
    let close = t.trim_end();
    ind < at + 4 && close.len() >= open.len() && close.bytes().all(|c| c == open.as_bytes()[0])
}

fn reach_all(stack: &mut [Frame], n: usize) {
    for f in stack {
        f.reach(n);
    }
}

fn push_item(stack: &mut Vec<Frame>, (bullet, col): (u8, usize), n: usize) {
    if !matches!(stack.last(), Some(Frame::List { .. })) {
        let within = match stack.last_mut() {
            Some(Frame::Item { col, nested, .. }) => {
                *nested = true;
                *col
            }
            _ => 0,
        };
        stack.push(Frame::List {
            bullet,
            within,
            start: n,
            last: n,
        });
    }
    stack.push(Frame::Item {
        col,
        nested: false,
        start: n,
        last: n,
    });
}

pub(super) fn markdown(lines: &[String]) -> Vec<(usize, usize)> {
    let mut folds = Vec::new();
    let mut headings: Vec<(usize, usize)> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut fence: Option<(usize, String, usize)> = None;
    let mut code: Option<(usize, usize, usize)> = None;
    let mut paragraph: Option<usize> = None;
    let mut blank = true;
    let body = front_matter_end(lines);
    for (n, line) in lines.iter().enumerate().skip(body) {
        let ind = columns(line);
        let t = line.trim_start();
        if let Some((start, ref open, at)) = fence {
            if fence_closes(t, ind, open, at) {
                folds.push((start, n));
                fence = None;
            }
            reach_all(&mut stack, n);
            continue;
        }
        if t.is_empty() {
            blank = true;
            paragraph = None;
            continue;
        }
        let item = marker(line).filter(|_| !rule(t));
        let ticks = t.bytes().take_while(|&c| c == b'`').count();
        let tildes = t.bytes().take_while(|&c| c == b'~').count();
        let fenced = (ticks >= 3 && !t.trim_start_matches('`').contains('`')) || tildes >= 3;
        let starter =
            item.is_some() || fenced || rule(t) || heading(t).is_some() || t.starts_with('>');
        if let Some((start, last, base)) = code {
            if ind >= base + 4 {
                code = Some((start, n, base));
                reach_all(&mut stack, n);
                continue;
            }
            if last > start {
                folds.push((start, last));
            }
            code = None;
        }
        loop {
            match stack.last() {
                Some(Frame::Item { col, .. }) if ind >= *col => break,
                Some(Frame::Item { .. }) if !blank && !starter && paragraph.is_some() => break,
                Some(Frame::List { bullet, within, .. })
                    if ind >= *within && item.is_some_and(|(b, _)| b == *bullet) =>
                {
                    break;
                }
                Some(_) => fold_frame(&stack.pop().expect("last() was Some"), &mut folds),
                None => break,
            }
        }
        let top = stack.is_empty();
        if let Some(level) = heading(t).filter(|_| top && ind < 4) {
            headings.push((n, level));
        }
        let base = match stack.last() {
            Some(Frame::Item { col, .. }) => Some(*col),
            Some(Frame::List { .. }) => None,
            None => Some(0),
        };
        if let Some(base) = base.filter(|b| ind >= b + 4 && blank && paragraph.is_none()) {
            code = Some((n, n, base));
            reach_all(&mut stack, n);
            blank = false;
            continue;
        }
        if let Some(item) = item {
            push_item(&mut stack, item, n);
        }
        reach_all(&mut stack, n);
        if fenced {
            let open = (if ticks > 0 { "`" } else { "~" }).repeat(ticks.max(tildes));
            fence = Some((n, open, ind));
        }
        let text = !(fenced || rule(t) || heading(t).is_some());
        paragraph = if text { paragraph.or(Some(n)) } else { None };
        blank = false;
    }
    if let Some((start, last, _)) = code.filter(|c| c.1 > c.0) {
        folds.push((start, last));
    }
    if let Some((start, ..)) = fence {
        let last = (start..lines.len())
            .rev()
            .find(|&l| !lines[l].trim().is_empty());
        folds.extend(last.filter(|&l| l > start).map(|l| (start, l)));
    }
    while let Some(f) = stack.pop() {
        fold_frame(&f, &mut folds);
    }
    if headings.first().is_none_or(|&(h, _)| h > body) {
        headings.insert(0, (body, 6));
    }
    heading_folds(lines, &headings, &mut folds);
    innermost(folds)
}

fn heading_folds(lines: &[String], headings: &[(usize, usize)], folds: &mut Vec<(usize, usize)>) {
    for (k, &(h, level)) in headings.iter().enumerate() {
        let next = headings[k + 1..]
            .iter()
            .find(|&&(_, l)| l <= level)
            .map_or(lines.len(), |&(n, _)| n);
        let last = (h..next).rev().find(|&l| !lines[l].trim().is_empty());
        folds.extend(last.filter(|&l| l > h).map(|l| (h, l)));
    }
}
