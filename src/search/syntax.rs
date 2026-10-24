use super::{
    Kind, haskell_literal_lines, html_literal_lines, julia_literal_lines, lisp, lisp_literal_lines,
    markdown_literal_lines, ml_literal_lines, nix_literal_lines, perl_literal_lines,
    r_literal_lines, style_blocks,
};
use std::borrow::Cow;

pub(super) fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}
pub(super) fn comment(kind: Kind, t: &str) -> bool {
    match kind {
        Kind::Python | Kind::Gdscript | Kind::Starlark => t.starts_with('#'),
        Kind::Lua => t.starts_with("--"),
        _ => ["//", "/*", "*"].iter().any(|c| t.starts_with(c)),
    }
}
pub fn literal_lines(kind: Kind, text: &str) -> Vec<bool> {
    thread_local! {
        static LEXED: std::cell::RefCell<std::collections::VecDeque<(Kind, String, Vec<bool>)>> =
            const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
    }
    let seen = LEXED.with_borrow(|l| {
        l.iter()
            .find(|(k, t, _)| *k == kind && t == text)
            .map(|(.., lines)| lines.clone())
    });
    if let Some(lines) = seen {
        return lines;
    }
    let lines = lex_literal_lines(kind, text);
    LEXED.with_borrow_mut(|l| {
        if l.len() == LEXED_KEPT {
            l.pop_front();
        }
        l.push_back((kind, text.to_owned(), lines.clone()));
    });
    lines
}
const LEXED_KEPT: usize = 16;
fn lex_literal_lines(kind: Kind, text: &str) -> Vec<bool> {
    if kind == Kind::Markdown {
        return markdown_literal_lines(text);
    }
    if kind == Kind::Html {
        return html_literal_lines(text);
    }
    if kind == Kind::Nix {
        return nix_literal_lines(text);
    }
    if kind == Kind::Haskell {
        return haskell_literal_lines(text);
    }
    if matches!(kind, Kind::Ocaml | Kind::Fsharp) {
        return ml_literal_lines(kind, text);
    }
    if kind == Kind::Julia {
        return julia_literal_lines(text);
    }
    if kind == Kind::R {
        return r_literal_lines(text);
    }
    if kind == Kind::Perl {
        return perl_literal_lines(text);
    }
    if lisp(kind) {
        return lisp_literal_lines(kind, text);
    }
    if kind == Kind::Css && text.contains("<style") {
        let html = html_literal_lines(text);
        let mut out = scan(kind, &style_blocks(text), usize::MAX).lines_in_literal;
        for (o, h) in out.iter_mut().zip(html) {
            *o |= h;
        }
        return out;
    }
    let mut out = scan(kind, text, usize::MAX).lines_in_literal;
    if kind == Kind::C {
        hide_c_define_bodies(text, &mut out);
    }
    out
}
fn hide_c_define_bodies(text: &str, out: &mut [bool]) {
    let mut define = false;
    for (i, l) in text.lines().enumerate() {
        let open = define;
        if !open {
            define = !out.get(i).copied().unwrap_or(false)
                && l.trim_start()
                    .strip_prefix('#')
                    .is_some_and(|d| d.trim_start().starts_with("define"));
        }
        if open && let Some(hidden) = out.get_mut(i) {
            *hidden = true;
        }
        define &= l.trim_end().ends_with('\\');
    }
}
pub fn hide_c_disabled(text: &str, out: &mut [bool]) {
    let mut depth = 0;
    for (l, hidden) in text.lines().zip(out.iter_mut()) {
        let directive = if *hidden {
            ""
        } else {
            l.trim_start().strip_prefix('#').map_or("", str::trim_start)
        };
        let name = directive.split(|c: char| !c.is_ascii_alphabetic()).next();
        if depth == 0 {
            let condition = (directive.strip_prefix("if"))
                .filter(|c| c.starts_with([' ', '\t']))
                .map(|c| c.split("//").next().unwrap_or(c))
                .map(|c| c.split("/*").next().unwrap_or(c).trim());
            depth = usize::from(matches!(condition, Some("0" | "false")));
            continue;
        }
        match name {
            Some("if" | "ifdef" | "ifndef") => depth += 1,
            Some("endif") => depth -= 1,
            Some("else" | "elif" | "elifdef" | "elifndef") if depth == 1 => depth = 0,
            _ => *hidden = true,
        }
    }
}
pub fn in_string(kind: Kind, text: &str, at: usize) -> bool {
    match kind {
        Kind::Rust => scan(kind, text, at).at_in_rust_string,
        Kind::Php => {
            let lead = text.get(..at).and_then(|t| t.rsplit('\n').next());
            let s = scan(kind, text, at);
            let prose = (s.at_php_quote_from)
                .is_none_or(|from| php_prose(text, from) && !php_qualified_name(text, at));
            prose
                && match s.at_in_php_literal {
                    Some(b'\'') => true,
                    Some(_) => lead.is_some_and(|l| !php_interpolated(l)),
                    None => false,
                }
        }
        _ => false,
    }
}
fn php_prose(text: &str, from: usize) -> bool {
    let b = text.as_bytes();
    let Some(&q) = b.get(from) else {
        return false;
    };
    let mut i = from + 1;
    while let Some(&c) = b.get(i) {
        if c == q || c == b'\n' {
            return false;
        }
        if c == b' ' || c == b'\t' {
            return true;
        }
        i += if c == b'\\' { 2 } else { 1 };
    }
    false
}
fn php_qualified_name(text: &str, at: usize) -> bool {
    let b = text.as_bytes();
    let name = |c: &u8| ident(*c) || *c == b'\\';
    let (head, tail) = (b.get(..at).unwrap_or(b), b.get(at..).unwrap_or_default());
    let head = head.iter().rev().take_while(|c| name(c));
    let tail = tail.iter().take_while(|c| name(c));
    head.chain(tail).any(|&c| c == b'\\')
}
fn php_interpolated(lead: &str) -> bool {
    let open = lead.rfind("{$").max(lead.rfind("${"));
    (lead.ends_with('$') && !lead.ends_with("\\$"))
        || lead.ends_with("->")
        || open.is_some_and(|o| lead.get(o..).is_some_and(|l| !l.contains('}')))
}
struct Scan {
    lines_in_literal: Vec<bool>,
    at_in_rust_string: bool,
    at_in_php_literal: Option<u8>,
    at_php_quote_from: Option<usize>,
}
struct Heredoc {
    label: Vec<u8>,
    end: HeredocEnd,
}
enum HeredocEnd {
    LabelLeadsLine,
    LabelAloneOnLine { may_indent: bool },
}
struct Rules {
    triple_quotes: bool,
    template: bool,
    block_comment: bool,
    line_comments: &'static [&'static str],
}
#[expect(
    clippy::match_same_arms,
    reason = "one row per language family of comment rules"
)]
fn rules(kind: Kind) -> Rules {
    let (triple_quotes, template, block_comment, line_comments): (
        _,
        _,
        _,
        &'static [&'static str],
    ) = match kind {
        Kind::Python | Kind::Starlark | Kind::Elixir | Kind::Gdscript => {
            (true, false, false, &["#"])
        }
        Kind::Lua => (false, false, false, &["--"]),
        Kind::Zig => (false, false, false, &["//"]),
        Kind::Swift | Kind::CSharp => (true, true, true, &["//"]),
        Kind::Shell | Kind::Make | Kind::Docker | Kind::Yaml => (false, false, false, &["#"]),
        Kind::Sql => (false, false, true, &["--", "//"]),
        Kind::PowerShell => (false, false, false, &["#"]),
        Kind::Terraform => (false, false, true, &["#", "//"]),
        Kind::Graphql => (true, false, false, &["#"]),
        Kind::Proto | Kind::Solidity => (false, false, true, &["//"]),
        Kind::Css => (false, false, true, &["//"]),
        Kind::Php => (false, true, true, &["//", "#"]),
        Kind::Rust => (false, false, true, &["//"]),
        Kind::Ruby => (false, false, false, &["#"]),
        Kind::Jvm => (true, false, true, &["//"]),
        Kind::Dart => (true, false, true, &["//"]),
        Kind::Cmake => (false, false, false, &["#"]),
        _ => (false, true, true, &["//"]),
    };
    Rules {
        triple_quotes,
        template,
        block_comment,
        line_comments,
    }
}
fn ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}
struct Scanner<'a> {
    kind: Kind,
    b: &'a [u8],
    rules: Rules,
    in_php_tags: bool,
    out: Vec<bool>,
    literal_closer: Option<Cow<'a, [u8]>>,
    long_bracket_level: usize,
    line_quote: Option<u8>,
    line_quote_from: usize,
    i: usize,
    in_verbatim_string: bool,
    in_raw_string: bool,
    nested_comments: Option<usize>,
    heredoc: Option<Heredoc>,
    php_heredoc_quote: u8,
    ruby_heredocs_from_next_line: Vec<Heredoc>,
    template_substitution_braces: Vec<usize>,
    sql_body: Option<&'a [u8]>,
    cursor: Cursor,
}
struct Cursor {
    at: usize,
    line: usize,
    rust_string_start: Option<usize>,
    in_rust_string: bool,
    literal: Option<u8>,
    line_quote_unread: bool,
    quote_from: Option<usize>,
}
fn scan(kind: Kind, text: &str, at: usize) -> Scan {
    let mut s = Scanner {
        kind,
        b: text.as_bytes(),
        rules: rules(kind),
        in_php_tags: false,
        out: vec![false],
        literal_closer: None,
        long_bracket_level: 0,
        line_quote: None,
        line_quote_from: 0,
        i: 0,
        in_verbatim_string: false,
        in_raw_string: false,
        nested_comments: (kind == Kind::Dart || (kind == Kind::Sql && super::sql::sql_nests(text)))
            .then_some(0),
        heredoc: None,
        php_heredoc_quote: b'"',
        ruby_heredocs_from_next_line: Vec::new(),
        template_substitution_braces: Vec::new(),
        sql_body: None,
        cursor: Cursor {
            at,
            line: text
                .get(..at)
                .map_or(usize::MAX, |t| t.bytes().filter(|&c| c == b'\n').count()),
            rust_string_start: None,
            in_rust_string: false,
            literal: None,
            line_quote_unread: kind == Kind::Php,
            quote_from: None,
        },
    };
    while s.i < s.b.len() && s.step() {
        s.i += 1;
    }
    if let Some(from) = s.cursor.rust_string_start {
        s.cursor.in_rust_string |= from < at && at <= s.b.len();
    }
    Scan {
        lines_in_literal: s.out,
        at_in_rust_string: s.cursor.in_rust_string,
        at_in_php_literal: s.cursor.literal,
        at_php_quote_from: s.cursor.quote_from,
    }
}
impl<'a> Scanner<'a> {
    fn powershell(&self) -> bool {
        self.kind == Kind::PowerShell
    }

    fn long_bracket(&self) -> bool {
        matches!(self.kind, Kind::Lua | Kind::Cmake)
    }

    fn token_at(&self, at: usize, prefixes: &[&[u8]]) -> bool {
        let b = self.b;
        std::iter::once(&&b""[..]).chain(prefixes).any(|p| {
            at >= p.len()
                && b[at - p.len()..at] == **p
                && (at == p.len() || !ident(b[at - p.len() - 1]))
        })
    }

    fn opens(&self, at: usize) -> Option<(usize, usize)> {
        let b = self.b;
        let long_comment_open = if self.kind == Kind::Cmake {
            &b"#["[..]
        } else {
            b"--["
        };
        let open = if b[at..].starts_with(long_comment_open) {
            at + long_comment_open.len() - 1
        } else {
            at
        };
        if b.get(open) != Some(&b'[') {
            return None;
        }
        let eq = b[open + 1..].iter().take_while(|&&c| c == b'=').count();
        (b.get(open + 1 + eq) == Some(&b'[')).then_some((eq, open + 1 + eq - at))
    }

    fn step(&mut self) -> bool {
        let (b, i, kind) = (self.b, self.i, self.kind);
        let c = b[i];
        if self.cursor.line_quote_unread && i >= self.cursor.at {
            self.cursor.line_quote_unread = false;
            if let Some(q) = self.line_quote.filter(|_| self.in_php_tags) {
                self.cursor.literal = Some(q);
                self.cursor.quote_from = Some(self.line_quote_from);
            }
        }
        if c == b'\n' {
            self.newline();
        } else if let Some(end) = self.literal_closer.take() {
            if kind == Kind::TsJs && *end == *b"`" && b[i..].starts_with(b"${") && b[i - 1] != b'\\'
            {
                self.template_substitution_braces.push(0);
                self.i += 1;
            } else {
                self.in_literal(end, c);
            }
        } else if !self.template_substitution_braces.is_empty()
            && self.line_quote.is_none()
            && (c == b'{' || c == b'}')
            && b[i - 1] != b'\\'
        {
            let depth = (self.template_substitution_braces.last_mut()).expect("not empty");
            match (c, *depth) {
                (b'{', _) => *depth += 1,
                (_, 0) => {
                    self.template_substitution_braces.pop();
                    self.literal_closer = Some(b"`".into());
                }
                _ => *depth -= 1,
            }
        } else if let Some(q) = self.line_quote {
            let escape = if self.powershell() {
                q == b'"' && c == b'`'
            } else {
                c == b'\\'
            };
            if escape && b.get(i + 1) != Some(&b'\n') {
                self.i += 1;
            } else if c == q {
                self.line_quote = None;
            }
        } else if !self.opens_quoted(c) && !self.opens_bracketed(c) {
            if self.ruby_data_end() {
                return false;
            }
            self.opens_rest(c);
        }
        true
    }

    fn newline(&mut self) {
        let (b, i) = (self.b, self.i);
        if !(self.kind == Kind::Php && self.in_php_tags) {
            self.line_quote = None;
        }
        if self.heredoc.is_none() && !self.ruby_heredocs_from_next_line.is_empty() {
            self.heredoc = Some(self.ruby_heredocs_from_next_line.remove(0));
            self.literal_closer = Some(b"".into());
        }
        if let Some(Heredoc { label, end }) = &self.heredoc {
            let rest = &b[i + 1..];
            let word = rest
                .iter()
                .position(|c| !c.is_ascii_whitespace())
                .map_or(rest, |n| &rest[n..]);
            let ends = match *end {
                HeredocEnd::LabelAloneOnLine { may_indent } => {
                    let line = rest.split(|&c| c == b'\n').next().unwrap_or(rest);
                    line.trim_ascii() == label && (may_indent || line.starts_with(label))
                }
                HeredocEnd::LabelLeadsLine => {
                    word.starts_with(&label[..])
                        && !word[label.len()..].first().is_some_and(|&c| ident(c))
                }
            };
            if ends {
                self.heredoc = None;
                self.literal_closer = None;
            }
        }
        self.out.push(self.literal_closer.is_some());
        if self.heredoc.is_some() && self.out.len() - 1 == self.cursor.line {
            self.cursor.literal = Some(self.php_heredoc_quote);
        }
    }

    fn in_literal(&mut self, end: Cow<'a, [u8]>, c: u8) {
        let (b, i, kind) = (self.b, self.i, self.kind);
        let doubled = self.in_verbatim_string && c == b'"' && b.get(i + 1) == Some(&b'"');
        let escape = matches!(kind, Kind::Rust | Kind::Cmake)
            && !self.in_raw_string
            && *end == *b"\""
            && c == b'\\'
            && b.get(i + 1) != Some(&b'\n');
        let bracket = self.long_bracket() && *end == *b"]]";
        let level = self.long_bracket_level;
        let closes = if bracket {
            c == b']'
                && b[i + 1..]
                    .iter()
                    .take(level)
                    .filter(|&&c| c == b'=')
                    .count()
                    == level
                && b.get(i + 1 + level) == Some(&b']')
        } else {
            self.heredoc.is_none()
                && !doubled
                && b[i..].starts_with(&end)
                && (end.len() > 1
                    || matches!(kind, Kind::Go | Kind::Rust | Kind::Cmake | Kind::Shell)
                    || self.in_verbatim_string
                    || b[i - 1] != b'\\')
        };
        let skip = end.len().saturating_sub(1);
        let nested = self.nested_comments.filter(|_| *end == *b"*/");
        if let Some(n) = nested
            && b[i..].starts_with(b"/*")
        {
            self.nested_comments = Some(n + 1);
            self.i += 1;
        } else if let Some(n) = nested.filter(|&n| closes && n > 0) {
            self.nested_comments = Some(n - 1);
            self.i += 1;
        } else if escape {
            self.i += 1;
        } else if closes {
            self.in_verbatim_string = false;
            if let Some(from) = self.cursor.rust_string_start.take() {
                self.cursor.in_rust_string |= from < self.cursor.at && self.cursor.at < i;
            }
            self.i += if bracket { level + 1 } else { skip };
            return;
        } else if doubled {
            self.i += 1;
        }
        self.literal_closer = Some(end);
    }

    fn opens_quoted(&mut self, c: u8) -> bool {
        let (b, i, kind) = (self.b, self.i, self.kind);
        if self.rules.triple_quotes
            && (b[i..].starts_with(b"\"\"\"")
                || (matches!(
                    kind,
                    Kind::Python
                        | Kind::Starlark
                        | Kind::Elixir
                        | Kind::Dart
                        | Kind::Jvm
                        | Kind::Gdscript
                ) && b[i..].starts_with(b"'''")))
        {
            self.literal_closer = Some(if c == b'"' { b"\"\"\"" } else { b"'''" }.into());
            self.i += 2;
        } else if kind == Kind::Jvm && b[i..].starts_with(b"$/") && self.token_at(i, &[]) {
            self.literal_closer = Some(b"/$".into());
            self.i += 1;
        } else if self.powershell() && b[i..].starts_with(b"<#") {
            self.literal_closer = Some(b"#>".into());
            self.i += 1;
        } else if self.powershell()
            && (b[i..].starts_with(b"@\"") || b[i..].starts_with(b"@'"))
            && b[i + 2..]
                .iter()
                .take_while(|&&c| c != b'\n')
                .all(u8::is_ascii_whitespace)
        {
            self.literal_closer = Some(b"".into());
            self.heredoc = Some(Heredoc {
                label: vec![b[i + 1], b'@'],
                end: HeredocEnd::LabelLeadsLine,
            });
            self.i += 1;
        } else if self.powershell() && c == b'`' {
            if !matches!(b.get(i + 1), Some(b'\n' | b'\r')) {
                self.i += 1;
            }
        } else if kind == Kind::CSharp
            && (b[i..].starts_with(b"@\"") || b[i..].starts_with(b"@$\""))
        {
            self.literal_closer = Some(b"\"".into());
            self.in_verbatim_string = true;
            self.i += if b[i + 1] == b'$' { 2 } else { 1 };
        } else if kind == Kind::Php && b[i..].starts_with(b"<<<") {
            self.php_heredoc();
        } else if matches!(kind, Kind::Shell | Kind::Docker | Kind::Terraform)
            && b[i..].starts_with(b"<<")
        {
            self.shell_heredoc();
        } else {
            return false;
        }
        true
    }

    fn php_heredoc(&mut self) {
        let (b, i) = (self.b, self.i);
        let word: Vec<u8> = b[i + 3..]
            .iter()
            .skip_while(|c| **c == b'"' || **c == b'\'')
            .take_while(|c| ident(**c))
            .copied()
            .collect();
        if !word.is_empty() {
            self.php_heredoc_quote = if b.get(i + 3) == Some(&b'\'') {
                b'\''
            } else {
                b'"'
            };
            self.i += 2;
            self.literal_closer = Some(b"".into());
            self.heredoc = Some(Heredoc {
                label: word,
                end: HeredocEnd::LabelLeadsLine,
            });
        }
    }

    fn shell_heredoc(&mut self) {
        let (b, i) = (self.b, self.i);
        let line = b[..i]
            .iter()
            .rposition(|&c| c == b'\n')
            .map_or(0, |n| n + 1);
        let word: Vec<u8> = b[i + 2..]
            .iter()
            .skip_while(|c| b"-'\"\\ \t".contains(c))
            .take_while(|c| ident(**c))
            .copied()
            .collect();
        if word.first().is_some_and(|c| !c.is_ascii_digit())
            && !b[line..i].windows(2).any(|w| w == b"((")
        {
            self.literal_closer = Some(b"".into());
            self.heredoc = Some(Heredoc {
                label: word,
                end: HeredocEnd::LabelLeadsLine,
            });
        }
        self.i += 1;
    }

    fn opens_bracketed(&mut self, c: u8) -> bool {
        let (b, i, kind) = (self.b, self.i, self.kind);
        if let Some((eq, skip)) = self.long_bracket().then(|| self.opens(i)).flatten() {
            self.literal_closer = Some(b"]]".into());
            self.long_bracket_level = eq;
            self.i += skip;
        } else if self.rules.template && c == b'`' {
            if i == 0 || b[i - 1] != b'/' {
                self.literal_closer = Some(b"`".into());
            }
        } else if self.rules.block_comment && b[i..].starts_with(b"/*") {
            self.literal_closer = Some(b"*/".into());
            self.i += 1;
        } else if kind == Kind::Rust && c == b'"' {
            (
                self.literal_closer,
                self.in_raw_string,
                self.cursor.rust_string_start,
            ) = (Some(b"\"".into()), false, Some(i));
        } else if kind == Kind::Cmake && c == b'"' {
            self.literal_closer = Some(b"\"".into());
        } else if let Some(hashes) =
            (kind == Kind::Rust && c == b'r' && self.token_at(i, &[b"b", b"c"]))
                .then(|| b[i + 1..].iter().take_while(|&&c| c == b'#').count())
                .filter(|n| b.get(i + 1 + n) == Some(&b'"'))
        {
            let mut end = vec![b'"'];
            end.resize(hashes + 1, b'#');
            (
                self.literal_closer,
                self.in_raw_string,
                self.cursor.rust_string_start,
            ) = (Some(end.into()), true, Some(i));
            self.i += hashes + 1;
        } else if kind == Kind::Rust && c == b'\'' {
            self.rust_char();
        } else if let Some(delim) = self.c_raw_string_delimiter(c) {
            self.literal_closer = Some([&b")"[..], delim, b"\""].concat().into());
            self.i += delim.len() + 2;
        } else {
            return false;
        }
        true
    }

    fn rust_char(&mut self) {
        let (b, i) = (self.b, self.i);
        let end = match b.get(i + 1) {
            Some(b'\\') => b[(i + 3).min(b.len())..]
                .iter()
                .take(9)
                .position(|&c| c == b'\'')
                .map(|n| i + 3 + n),
            Some(&c) => Some(i + 1 + (c.leading_ones() as usize).max(1)),
            None => None,
        };
        if let Some(end) = end.filter(|&e| b.get(e) == Some(&b'\'')) {
            self.i = end;
        }
    }

    fn c_raw_string_delimiter(&self, c: u8) -> Option<&'a [u8]> {
        let (b, i) = (self.b, self.i);
        (self.kind == Kind::C
            && c == b'R'
            && b.get(i + 1) == Some(&b'"')
            && self.token_at(i, &[b"u8", b"u", b"U", b"L"]))
        .then(|| {
            b[i + 2..]
                .iter()
                .take(17)
                .position(|&c| c == b'(')
                .map(|n| &b[i + 2..i + 2 + n])
        })
        .flatten()
        .filter(|d| !d.iter().any(|c| b" )\\\t\n".contains(c)))
    }

    fn line_start(&self) -> bool {
        self.i == 0 || self.b[self.i - 1] == b'\n'
    }

    fn shell_quote_leads_line(&self) -> bool {
        let (b, i) = (self.b, self.i);
        let from = b[..i]
            .iter()
            .rposition(|&c| c == b'\n')
            .map_or(0, |n| n + 1);
        let lead = b[from..i].trim_ascii_start();
        let name = lead
            .strip_suffix(b"=")
            .map(|n| n.strip_suffix(b"+").unwrap_or(n));
        name.is_some_and(|n| {
            n.first().is_some_and(|c| !c.is_ascii_digit()) && n.iter().all(|&c| ident(c))
        }) || lead.strip_prefix(b":").is_some_and(|s| {
            s.first().is_some_and(|&c| c == b' ' || c == b'\t') && s.trim_ascii().is_empty()
        })
    }

    fn ruby_data_end(&mut self) -> bool {
        let (b, i) = (self.b, self.i);
        let ends = self.kind == Kind::Ruby
            && self.line_start()
            && b[i..].starts_with(b"__END__")
            && b[i + 7..].first().is_none_or(|&c| c == b'\n' || c == b'\r');
        if ends {
            self.out
                .extend(b[i..].iter().filter(|&&c| c == b'\n').map(|_| true));
        }
        ends
    }

    fn sql_dollar_quote(&mut self) -> bool {
        let (b, i) = (self.b, self.i);
        let len = b[i + 1..].iter().take_while(|&&c| ident(c)).count();
        if (i > 0 && ident(b[i - 1]))
            || b.get(i + 1).is_some_and(u8::is_ascii_digit)
            || b.get(i + 1 + len) != Some(&b'$')
        {
            return false;
        }
        let tag = &b[i..i + len + 2];
        match self.sql_body {
            None => self.sql_body = Some(tag),
            Some(open) if open == tag => self.sql_body = None,
            Some(_) => self.literal_closer = Some(tag.into()),
        }
        self.i += tag.len() - 1;
        true
    }

    fn opens_rest(&mut self, c: u8) {
        let (b, i, kind) = (self.b, self.i, self.kind);
        if kind == Kind::Sql && c == b'$' && self.sql_dollar_quote() {
            return;
        }
        if kind == Kind::Ruby
            && self.line_start()
            && b[i..].starts_with(b"=begin")
            && b.get(i + 6).is_none_or(u8::is_ascii_whitespace)
        {
            self.literal_closer = Some(b"".into());
            self.heredoc = Some(Heredoc {
                label: b"=end".to_vec(),
                end: HeredocEnd::LabelLeadsLine,
            });
        } else if kind == Kind::Ruby && b[i..].starts_with(b"<<") {
            self.ruby_heredoc();
        } else if kind == Kind::Sql && c == b'\'' {
            let end = super::sql::sql_string_end(b, i).unwrap_or(b.len());
            self.out
                .extend(b[i..end].iter().filter(|&&c| c == b'\n').map(|_| true));
            self.i = end - 1;
        } else if kind == Kind::Shell && c == b'\'' && self.shell_quote_leads_line() {
            self.literal_closer = Some(b"'".into());
        } else if (kind == Kind::Ruby && c == b'/' && ruby_literal_may_start(&b[..i]))
            || c == b'"'
            || (c == b'\'' && kind != Kind::Cmake)
            || (matches!(kind, Kind::Sql | Kind::Ruby) && c == b'`')
        {
            self.line_quote = Some(c);
            self.line_quote_from = i;
        } else if kind == Kind::Php && (b[i..].starts_with(b"<?") || b[i..].starts_with(b"?>")) {
            self.in_php_tags = c == b'<';
            self.i += 1;
        } else if self.opens_line_comment(c) {
            while self.i + 1 < b.len()
                && b[self.i + 1] != b'\n'
                && !(kind == Kind::Php && b[self.i + 1..].starts_with(b"?>"))
            {
                self.i += 1;
            }
        }
    }

    fn opens_line_comment(&self, c: u8) -> bool {
        let (b, i, kind) = (self.b, self.i, self.kind);
        let comment_only_at_word_start =
            matches!(kind, Kind::Shell | Kind::Docker | Kind::PowerShell);
        self.rules
            .line_comments
            .iter()
            .any(|m| b[i..].starts_with(m.as_bytes()))
            && !(comment_only_at_word_start && i > 0 && !b" \t\n;&|()<>".contains(&b[i - 1]))
            && !(kind == Kind::Php && c == b'#' && (!self.in_php_tags || b[i..].starts_with(b"#[")))
    }

    fn ruby_heredoc(&mut self) {
        let (b, i) = (self.b, self.i);
        let flag = matches!(b.get(i + 2), Some(b'~' | b'-'));
        let at_word = i + 2 + usize::from(flag);
        let q = b.get(at_word).copied().filter(|c| b"'\"`".contains(c));
        let from = at_word + usize::from(q.is_some());
        let len = b[from..].iter().take_while(|&&c| ident(c)).count();
        let first = b.get(from).copied().filter(|_| len > 0);
        if first.is_some_and(|c| !c.is_ascii_digit())
            && (flag || q.is_some() || first.is_some_and(|c| c.is_ascii_uppercase()))
            && (q.is_none() || b.get(from + len).copied() == q)
        {
            self.ruby_heredocs_from_next_line.push(Heredoc {
                label: b[from..from + len].to_vec(),
                end: HeredocEnd::LabelAloneOnLine { may_indent: flag },
            });
            self.i = from + len + usize::from(q.is_some()) - 1;
        } else {
            self.i += 1;
        }
    }
}
pub(super) fn code(kind: Kind, s: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    let b = s.as_bytes();
    let (mut quote, mut comment, mut escaped) = (None, false, false);
    (0..b.len()).filter_map(move |i| {
        let c = b[i];
        if comment {
            comment = c != b'\n';
            return None;
        }
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            return None;
        }
        match c {
            b'\''
                if matches!(kind, Kind::Rust | Kind::C)
                    && b.get(i + 1) != Some(&b'\\')
                    && b.get(i + 2) != Some(&b'\'') =>
            {
                Some((i, c))
            }
            b'"' | b'\'' | b'`' => {
                quote = Some(c);
                None
            }
            b'/' | b'%' if kind == Kind::Ruby && ruby_literal_may_start(&b[..i]) => {
                let letter = c == b'%' && b.get(i + 1).is_some_and(|l| b"qQwWiIrsx".contains(l));
                let closer = match b.get(i + 2).filter(|_| letter) {
                    Some(b'{') => b'}',
                    Some(b'[') => b']',
                    Some(b'(') => b')',
                    Some(b'<') => b'>',
                    _ if c == b'/' => b'/',
                    _ => return Some((i, c)),
                };
                quote = Some(closer);
                None
            }
            b'#' if matches!(kind, Kind::Python | Kind::Gdscript | Kind::Ruby) => {
                comment = true;
                Some((i, 0))
            }
            b'-' if kind == Kind::Lua && b.get(i + 1) == Some(&b'-') => {
                comment = true;
                Some((i, 0))
            }
            b'/' if !matches!(kind, Kind::Python | Kind::Gdscript | Kind::Ruby)
                && b.get(i + 1) == Some(&b'/') =>
            {
                comment = true;
                Some((i, 0))
            }
            _ => Some((i, c)),
        }
    })
}
fn ruby_literal_may_start(before: &[u8]) -> bool {
    before
        .iter()
        .rfind(|c| **c != b' ' && **c != b'\t')
        .is_none_or(|p| b"\n(,=~!|&;{[".contains(p))
}
#[expect(
    clippy::string_slice,
    reason = "code() marks a comment at its first byte, an ASCII one, and a cut resumes at a newline"
)]
pub(super) fn uncommented(kind: Kind, s: &str) -> String {
    let (mut out, mut from) = (String::with_capacity(s.len()), 0);
    for (i, _) in code(kind, s).filter(|(_, c)| *c == 0) {
        out.push_str(&s[from..i]);
        from = s[i..].find('\n').map_or(s.len(), |n| i + n);
    }
    out.push_str(&s[from..]);
    out
}
pub(super) fn close_of(kind: Kind, s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in code(kind, s).skip_while(|(i, _)| *i < open) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}
#[expect(
    clippy::string_slice,
    reason = "every cut is at or right after an ASCII separator code() yielded"
)]
pub(super) fn split_top(kind: Kind, s: &str, sep: u8) -> Vec<&str> {
    let b = s.as_bytes();
    let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
    for (i, c) in code(kind, s) {
        let arrow = c == b'>' && i > 0 && b[i - 1] == b'=';
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b'<' if kind == Kind::TsJs => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'>' if kind == Kind::TsJs && !arrow => depth -= 1,
            c if c == sep && depth == 0 => {
                let part_of = |j: usize| b.get(j).is_some_and(|c| b"=<>!".contains(c));
                if sep != b'=' || !(part_of(i + 1) || (i > 0 && part_of(i - 1))) {
                    out.push(&s[start..i]);
                    start = i + 1;
                }
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}
pub(super) struct Group<'a> {
    pub inner_uncommented: String,
    pub close_line: usize,
    pub after_close: &'a str,
}
pub(super) fn group<'a>(
    kind: Kind,
    lines: &[&'a str],
    at: usize,
    open: usize,
) -> Option<Group<'a>> {
    let text = lines[at..lines.len().min(at + 2000)].join("\n");
    let end = close_of(kind, &text, open)?;
    let through_close = text.get(..end)?;
    let line_start = through_close.rfind('\n').map_or(0, |n| n + 1);
    let last = at + through_close.matches('\n').count();
    let inner = uncommented(kind, text.get(open + 1..end - 1)?);
    Some(Group {
        inner_uncommented: inner,
        close_line: last,
        after_close: lines[last].get(end - line_start..)?,
    })
}
pub(super) fn names(s: &str, word: &str) -> bool {
    s.match_indices(word).any(|(i, _)| {
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.';
        s.split_at_checked(i).is_some_and(|(before, rest)| {
            !before.ends_with(is_name)
                && rest
                    .strip_prefix(word)
                    .is_some_and(|after| !after.starts_with(is_name))
        })
    })
}
