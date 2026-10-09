use super::*;
use std::borrow::Cow;

pub(super) fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}
pub(super) fn comment(kind: Kind, t: &str) -> bool {
    match kind {
        Kind::Python | Kind::Gdscript | Kind::Starlark => t.starts_with('#'),
        _ => ["//", "/*", "*"].iter().any(|c| t.starts_with(c)),
    }
}
/// The 1-based lines of `text` that start inside a literal or a comment running over several
/// lines: a Python or Elixir triple-quoted string (a docstring or an `@moduledoc` with an example
/// in it), which Swift and C# write with `"` alone, a Go raw string, a TypeScript template, a Lua
/// `[[ ]]` or `[==[ ]==]` long string or block comment, a C# verbatim `@"…"`, a PHP heredoc, the
/// shell's heredoc (a Dockerfile's and Terraform's too), a `/* */` block. A line
/// there that reads like a declaration declares nothing — the SQL a migration embeds in one is
/// the common case. Other strings end with their line, whatever they hold.
///
/// ponytail: Elixir's `~S"""` sigil is read from its `"""`, and its one-line `~s(…)` forms not at
/// all; Ruby's multi-line `%q{…}` is read as code, and a `/` opens a regex only after an operator
/// or a bracket, not after `when` or `split `.
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
    // Markdown's are blocks, not tokens: a fence, an HTML comment, the front matter (#421).
    if kind == Kind::Markdown {
        return markdown_literal_lines(text);
    }
    // HTML's is the `<!-- … -->` comment (#415). A stylesheet a `<style>` block of markup holds
    // is lexed alone, the rest of the file blank: a `/*` in a template is no comment of it.
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
pub fn in_string(kind: Kind, text: &str, at: usize) -> bool {
    kind == Kind::Rust && scan(kind, text, at).at_in_rust_string
}
struct Scan {
    lines_in_literal: Vec<bool>,
    at_in_rust_string: bool,
}
struct Heredoc {
    label: Vec<u8>,
    end: HeredocEnd,
}
enum HeredocEnd {
    LabelLeadsLine,
    LabelAloneOnLine { may_indent: bool },
}
fn scan(kind: Kind, text: &str, at: usize) -> Scan {
    let (triple_quotes, long_bracket, template, block_comment, line_comments): (
        _,
        _,
        _,
        _,
        &[&str],
    ) = match kind {
        Kind::Python | Kind::Starlark | Kind::Elixir | Kind::Gdscript => {
            (true, false, false, false, &["#"])
        }
        Kind::Lua => (false, true, false, false, &["--"]),
        Kind::Zig => (false, false, false, false, &["//"]),
        Kind::Swift | Kind::CSharp => (true, false, true, true, &["//"]),
        // A YAML block scalar (`filters: |`) hides nothing: the keys dorny/paths-filter reads
        // out of one are what `steps.changes.outputs.backend` names.
        Kind::Shell | Kind::Make | Kind::Docker | Kind::Yaml => {
            (false, false, false, false, &["#"])
        }
        Kind::Sql => (false, false, false, true, &["--", "//"]),
        Kind::PowerShell => (false, false, false, false, &["#"]),
        Kind::Terraform => (false, false, false, true, &["#", "//"]),
        // GraphQL writes its descriptions in `"""` block strings, and nothing else runs
        // over lines: no `'''`, no `/* */`, no backtick.
        Kind::Graphql => (true, false, false, false, &["#"]),
        // The C family's comments, and no backtick: a Protocol Buffers string ends with its
        // line.
        Kind::Proto | Kind::Solidity => (false, false, false, true, &["//"]),
        // `/* */` in all four stylesheet languages and `//` in SCSS, Sass and Less: one kind
        // reads `//` in a `.css` file too, at the cost of a `/*` after a `url(//…)` on its
        // line (#415). A string ends with its line, and a backtick is nothing.
        Kind::Css => (false, false, false, true, &["//"]),
        Kind::Php => (false, false, true, true, &["//", "#"]),
        Kind::Rust => (false, false, false, true, &["//"]),
        Kind::Ruby => (false, false, false, false, &["#"]),
        // A Kotlin raw string and a Java text block run over lines between `"""`; neither
        // language has a backtick template: Kotlin's backticks quote a name (#367).
        Kind::Jvm => (true, false, false, true, &["//"]),
        Kind::Dart => (true, false, false, true, &["//"]),
        Kind::Cmake => (false, true, false, false, &["#"]),
        _ => (false, false, true, true, &["//"]),
    };
    let (csharp_verbatim_strings, php_heredocs) = (kind == Kind::CSharp, kind == Kind::Php);
    let shell_heredoc = matches!(kind, Kind::Shell | Kind::Docker | Kind::Terraform);
    // A shell quote ends with its line, as every other kind's does: the scan cannot follow a
    // `"…"` inside `"$( … )"`, so a quote it carried over lines would hide the rest of the file
    // behind one misread, and what `eval '…'` holds the shell does declare.
    let comment_only_at_word_start = matches!(kind, Kind::Shell | Kind::Docker | Kind::PowerShell);
    let powershell = kind == Kind::PowerShell;
    let mut in_php_tags = false;
    let b = text.as_bytes();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let token_at = |at: usize, prefixes: &[&[u8]]| {
        [&b""[..]].iter().chain(prefixes).any(|p| {
            at >= p.len()
                && b[at - p.len()..at] == **p
                && (at == p.len() || !ident(b[at - p.len() - 1]))
        })
    };
    let mut out = vec![false];
    let mut literal_closer: Option<Cow<[u8]>> = None;
    let mut long_bracket_level = 0;
    let mut line_quote: Option<u8> = None;
    let mut i = 0;
    let (mut in_verbatim_string, mut in_raw_string) = (false, false);
    let mut heredoc: Option<Heredoc> = None;
    let mut ruby_heredocs_from_next_line: Vec<Heredoc> = Vec::new();
    let mut template_substitution_braces: Vec<usize> = Vec::new();
    let (mut rust_string_start, mut at_in_rust_string) = (None, false);
    let long_comment_open = if kind == Kind::Cmake {
        &b"#["[..]
    } else {
        b"--["
    };
    let opens = |at: usize| -> Option<(usize, usize)> {
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
    };
    while i < b.len() {
        let c = b[i];
        let line_start = i == 0 || b[i - 1] == b'\n';
        if c == b'\n' {
            line_quote = None;
            if heredoc.is_none() && !ruby_heredocs_from_next_line.is_empty() {
                heredoc = Some(ruby_heredocs_from_next_line.remove(0));
                literal_closer = Some(b"".into());
            }
            if let Some(Heredoc { label, end }) = &heredoc {
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
                    heredoc = None;
                    literal_closer = None;
                }
            }
            out.push(literal_closer.is_some());
        } else if let Some(end) = literal_closer.as_deref()
            && !(kind == Kind::TsJs
                && end == b"`"
                && b[i..].starts_with(b"${")
                && b[i - 1] != b'\\')
        {
            let doubled = in_verbatim_string && c == b'"' && b.get(i + 1) == Some(&b'"');
            let escape = matches!(kind, Kind::Rust | Kind::Cmake)
                && !in_raw_string
                && end == b"\""
                && c == b'\\'
                && b.get(i + 1) != Some(&b'\n');
            let bracket = long_bracket && end == b"]]";
            let closes = if bracket {
                c == b']'
                    && b[i + 1..]
                        .iter()
                        .take(long_bracket_level)
                        .filter(|&&c| c == b'=')
                        .count()
                        == long_bracket_level
                    && b.get(i + 1 + long_bracket_level) == Some(&b']')
            } else {
                heredoc.is_none()
                    && !doubled
                    && b[i..].starts_with(end)
                    && (end.len() > 1
                        || matches!(kind, Kind::Go | Kind::Rust | Kind::Cmake)
                        || in_verbatim_string
                        || b[i - 1] != b'\\')
            };
            let skip = end.len().saturating_sub(1);
            if escape {
                i += 1;
            } else if closes {
                literal_closer = None;
                in_verbatim_string = false;
                if let Some(from) = rust_string_start.take() {
                    at_in_rust_string |= from < at && at < i;
                }
                i += if bracket {
                    long_bracket_level + 1
                } else {
                    skip
                };
            } else if doubled {
                i += 1;
            }
        } else if literal_closer.is_some() {
            // A template's `${` opens code, up to the `}` that closes it: a template in there
            // is one of its own, `${a ? `${b}/` : ""}`, and the rest of the outer one follows.
            template_substitution_braces.push(0);
            literal_closer = None;
            i += 1;
        } else if !template_substitution_braces.is_empty()
            && line_quote.is_none()
            && (c == b'{' || c == b'}')
            && b[i - 1] != b'\\'
        {
            // A regex's `\{` is no brace (`${s.replace(/\{/g, "")}`). ponytail: the `{` of
            // `/[{]/` still counts; skip regex literals in a hole if one shows up.
            let depth = template_substitution_braces.last_mut().expect("not empty");
            match (c, *depth) {
                (b'{', _) => *depth += 1,
                (_, 0) => {
                    template_substitution_braces.pop();
                    literal_closer = Some(b"`".into());
                }
                _ => *depth -= 1,
            }
        } else if let Some(q) = line_quote {
            // A backslash escapes the next byte, but the end of a line is still one. PowerShell
            // escapes with a backtick, and only in a `"…"`.
            let escape = match powershell {
                true => q == b'"' && c == b'`',
                false => c == b'\\',
            };
            if escape && b.get(i + 1) != Some(&b'\n') {
                i += 1;
            } else if c == q {
                line_quote = None;
            }
        } else if triple_quotes
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
            literal_closer = Some(if c == b'"' { b"\"\"\"" } else { b"'''" }.into());
            i += 2;
        } else if kind == Kind::Jvm && b[i..].starts_with(b"$/") && token_at(i, &[]) {
            literal_closer = Some(b"/$".into());
            i += 1;
        } else if powershell && b[i..].starts_with(b"<#") {
            literal_closer = Some(b"#>".into());
            i += 1;
        } else if powershell
            && (b[i..].starts_with(b"@\"") || b[i..].starts_with(b"@'"))
            && b[i + 2..]
                .iter()
                .take_while(|&&c| c != b'\n')
                .all(|c| c.is_ascii_whitespace())
        {
            // A here-string, `@"` or `@'` at the end of its line, closes on the line that starts
            // with `"@` or `'@`.
            literal_closer = Some(b"".into());
            heredoc = Some(Heredoc {
                label: vec![b[i + 1], b'@'],
                end: HeredocEnd::LabelLeadsLine,
            });
            i += 1;
        } else if powershell && c == b'`' {
            // An escape outside a string: `` `" `` opens nothing, `` `# `` no comment. At the end
            // of a line it continues the line, and the line still ends.
            if !matches!(b.get(i + 1), Some(b'\n' | b'\r')) {
                i += 1;
            }
        } else if csharp_verbatim_strings
            && (b[i..].starts_with(b"@\"") || b[i..].starts_with(b"@$\""))
        {
            // `$@"` is read from its `@"`.
            literal_closer = Some(b"\"".into());
            in_verbatim_string = true;
            i += if b[i + 1] == b'$' { 2 } else { 1 };
        } else if php_heredocs && b[i..].starts_with(b"<<<") {
            // `<<<SQL`, `<<<"SQL"` or `<<<'SQL'`: the label is what ends it.
            let word: Vec<u8> = b[i + 3..]
                .iter()
                .skip_while(|c| **c == b'"' || **c == b'\'')
                .take_while(|c| ident(**c))
                .copied()
                .collect();
            if !word.is_empty() {
                i += 2;
                literal_closer = Some(b"".into());
                heredoc = Some(Heredoc {
                    label: word,
                    end: HeredocEnd::LabelLeadsLine,
                });
            }
        } else if shell_heredoc && b[i..].starts_with(b"<<") {
            // `<<EOF`, `<<-EOF` or `<< 'EOF'`, read past both `<`, so that the shell's `<<<`, a
            // string of one line, labels nothing. A `<<` inside `$((…))` is a shift.
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
                literal_closer = Some(b"".into());
                heredoc = Some(Heredoc {
                    label: word,
                    end: HeredocEnd::LabelLeadsLine,
                });
            }
            i += 1;
        } else if let Some((eq, skip)) = long_bracket.then(|| opens(i)).flatten() {
            literal_closer = Some(b"]]".into());
            long_bracket_level = eq;
            i += skip;
        } else if template && c == b'`' {
            // ponytail: `/`/` is a regex, told by the slash in front; a division by a template
            // is not written.
            if i == 0 || b[i - 1] != b'/' {
                literal_closer = Some(b"`".into());
            }
        } else if block_comment && b[i..].starts_with(b"/*") {
            literal_closer = Some(b"*/".into());
            i += 1;
        } else if kind == Kind::Rust && c == b'"' {
            // `"…"`, `b"…"` and `c"…"` run over lines; a `\` at the end of one continues it.
            (literal_closer, in_raw_string, rust_string_start) =
                (Some(b"\"".into()), false, Some(i));
        } else if kind == Kind::Cmake && c == b'"' {
            literal_closer = Some(b"\"".into());
        } else if let Some(hashes) = (kind == Kind::Rust && c == b'r' && token_at(i, &[b"b", b"c"]))
            .then(|| b[i + 1..].iter().take_while(|&&c| c == b'#').count())
            .filter(|n| b.get(i + 1 + n) == Some(&b'"'))
        {
            // `r"…"`, `r#"…"#`, `br##"…"##`: closed by a `"` and as many `#`. `r#type` is a name.
            let mut end = vec![b'"'];
            end.resize(hashes + 1, b'#');
            (literal_closer, in_raw_string, rust_string_start) = (Some(end.into()), true, Some(i));
            i += hashes + 1;
        } else if kind == Kind::Rust && c == b'\'' {
            // A char literal, `'x'`, `'é'`, `'\n'` or `'\u{1F600}'`, closes within a few bytes; a
            // lifetime, `'a` or `'static`, does not, and opens nothing.
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
                i = end;
            }
        } else if let Some(delim) = (kind == Kind::C
            && c == b'R'
            && b.get(i + 1) == Some(&b'"')
            && token_at(i, &[b"u8", b"u", b"U", b"L"]))
        .then(|| {
            b[i + 2..]
                .iter()
                .take(17)
                .position(|&c| c == b'(')
                .map(|n| &b[i + 2..i + 2 + n])
        })
        .flatten()
        .filter(|d| !d.iter().any(|c| b" )\\\t\n".contains(c)))
        {
            // C++'s `R"(…)"`, `R"sql(…)sql"`: closed by `)`, the delimiter and `"`, no escapes.
            literal_closer = Some([&b")"[..], delim, b"\""].concat().into());
            i += delim.len() + 2;
        } else if kind == Kind::Ruby && line_start && b[i..].starts_with(b"__END__") && {
            let rest = &b[i + 7..];
            rest.first().is_none_or(|&c| c == b'\n' || c == b'\r')
        } {
            // Whatever follows `__END__` is the script's data.
            out.extend(b[i..].iter().filter(|&&c| c == b'\n').map(|_| true));
            break;
        } else if kind == Kind::Ruby
            && line_start
            && b[i..].starts_with(b"=begin")
            && b.get(i + 6).is_none_or(|c| c.is_ascii_whitespace())
        {
            literal_closer = Some(b"".into());
            heredoc = Some(Heredoc {
                label: b"=end".to_vec(),
                end: HeredocEnd::LabelLeadsLine,
            });
        } else if kind == Kind::Ruby && b[i..].starts_with(b"<<") {
            // `<<~SQL`, `<<-SQL`, `<<SQL`, `<<~'SQL'`: a bare `<<` needs a capital or a quote, so
            // `list << x` and `class << self` open nothing. The rest of the line is code.
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
                ruby_heredocs_from_next_line.push(Heredoc {
                    label: b[from..from + len].to_vec(),
                    end: HeredocEnd::LabelAloneOnLine { may_indent: flag },
                });
                i = from + len + usize::from(q.is_some()) - 1;
            } else {
                i += 1;
            }
        } else if kind == Kind::Ruby
            && c == b'/'
            && b[..i]
                .iter()
                .rfind(|c| **c != b' ' && **c != b'\t')
                .is_none_or(|p| b"\n(,=~!|&;{[".contains(p))
        {
            // A regex, `=~ /#/`: its `#` is no comment.
            line_quote = Some(c);
        } else if c == b'"'
            || (c == b'\'' && kind != Kind::Cmake)
            || (matches!(kind, Kind::Sql | Kind::Ruby) && c == b'`')
        {
            line_quote = Some(c);
        } else if kind == Kind::Php && (b[i..].starts_with(b"<?") || b[i..].starts_with(b"?>")) {
            in_php_tags = c == b'<';
            i += 1;
        } else if line_comments
            .iter()
            .any(|m| b[i..].starts_with(m.as_bytes()))
            && !(comment_only_at_word_start && i > 0 && !b" \t\n;&|()<>".contains(&b[i - 1]))
            && !(kind == Kind::Php && c == b'#' && (!in_php_tags || b[i..].starts_with(b"#[")))
        {
            while i + 1 < b.len()
                && b[i + 1] != b'\n'
                && !(kind == Kind::Php && b[i + 1..].starts_with(b"?>"))
            {
                i += 1;
            }
        }
        i += 1;
    }
    if let Some(from) = rust_string_start {
        at_in_rust_string |= from < at && at <= b.len();
    }
    Scan {
        lines_in_literal: out,
        at_in_rust_string,
    }
}
/// The bytes of `s` a scan for brackets and separators reads, with their indexes. String literals
/// are skipped; a comment (`#` in Python, `//` elsewhere) yields its first byte as `0` and is
/// skipped to the end of its line.
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
            // Rust's lifetime `'a` and C++'s digit separator `1'000` open nothing: there `'`
            // quotes only a char literal, `'x'` or `'\n'`.
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
            b'#' if matches!(kind, Kind::Python | Kind::Gdscript) => {
                comment = true;
                Some((i, 0))
            }
            b'/' if !matches!(kind, Kind::Python | Kind::Gdscript)
                && b.get(i + 1) == Some(&b'/') =>
            {
                comment = true;
                Some((i, 0))
            }
            _ => Some((i, c)),
        }
    })
}
pub(super) fn uncommented(kind: Kind, s: &str) -> String {
    let (mut out, mut from) = (String::with_capacity(s.len()), 0);
    for (i, _) in code(kind, s).filter(|(_, c)| *c == 0) {
        out.push_str(&s[from..i]);
        from = s[i..].find('\n').map_or(s.len(), |n| i + n);
    }
    out.push_str(&s[from..]);
    out
}
/// The byte index past the bracket that closes the one at `open`; `None` when `s` ends first.
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
/// `s` cut at every `sep` outside brackets and strings; TypeScript's generics `<…>` count as
/// brackets. An `=` in `=>`, `==`, `<=`, `>=` or `!=` is no separator.
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
    // ponytail: a bracket still open 2000 lines on is not read; fastapi's signatures run to 350.
    let text = lines[at..lines.len().min(at + 2000)].join("\n");
    let end = close_of(kind, &text, open)?;
    let line_start = text[..end].rfind('\n').map_or(0, |n| n + 1);
    let last = at + text[..end].matches('\n').count();
    let inner = uncommented(kind, &text[open + 1..end - 1]);
    Some(Group {
        inner_uncommented: inner,
        close_line: last,
        after_close: &lines[last][end - line_start..],
    })
}
/// Whether `word` stands in `s` as a whole name, not as the tail of `a.word`.
pub(super) fn names(s: &str, word: &str) -> bool {
    s.match_indices(word).any(|(i, _)| {
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.';
        !s[..i].ends_with(is_name) && !s[i + word.len()..].starts_with(is_name)
    })
}
