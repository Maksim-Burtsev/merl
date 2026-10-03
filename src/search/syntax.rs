//! The lexer the rules stand on: comments, string literals, brackets and separators.

use super::*;
use std::borrow::Cow;

pub(super) fn indent(s: &str) -> usize {
    s.len() - s.trim_start().len()
}
/// Whether a trimmed line of `kind` is a comment. TypeScript's `#private` is a name.
pub(super) fn comment(kind: Kind, t: &str) -> bool {
    match kind {
        Kind::Python => t.starts_with('#'),
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
/// Each kind says which forms it has rather than inheriting another language's: Zig has none at
/// all — a `\\` string ends with its line — and reading it with the backtick and `/* */` of the C
/// family would take the ``` ``` ``` fences of the markdown a `\\` block holds for a literal and
/// hide the rest of the file behind them, as the `/*` of a glob (`rm -rf build/*`) or a lone
/// backtick in a comment would in a shell script, a Makefile or a CI workflow (#436).
///
/// Rust's strings run over lines as the language runs them (#346): a `"…"` to the `"` no `\`
/// escapes, a raw `r#"…"#` to its `"#`; a `'` opens only a char literal, never a lifetime. A C++
/// raw string, `R"(…)"` or `u8R"x(…)x"`, runs to its `)x"` (#465). Ruby writes `#` comments,
/// `=begin` blocks, heredocs (`<<~SQL`, several on one line read in order) and `__END__`, after
/// which every line is data (#379); its `%q()`, backtick and regex literals end with their line.
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
    if kind == Kind::Css && text.contains("<style") {
        let html = html_literal_lines(text);
        let mut out = scan(kind, &style_blocks(text), usize::MAX).0;
        for (o, h) in out.iter_mut().zip(html) {
            *o |= h;
        }
        return out;
    }
    let mut out = scan(kind, text, usize::MAX).0;
    // The body of a multi-line C `#define` declares nothing (#382): a `typedef ret name##_t
    // args;` there names a macro's parameter. The `#define` line itself declares the macro.
    if kind == Kind::C {
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
    out
}
/// Whether byte `at` of `text` stands inside a Rust string literal, between its quotes: nothing
/// there names code (#346). Other kinds say no: a string there may name a type or a module.
pub fn in_string(kind: Kind, text: &str, at: usize) -> bool {
    kind == Kind::Rust && scan(kind, text, at).1
}
/// [`literal_lines`], and whether byte `at` is inside a Rust string.
fn scan(kind: Kind, text: &str, at: usize) -> (Vec<bool>, bool) {
    // What this kind writes: the triple quote of a heredoc, Lua's long bracket, the backtick
    // template and the `/* */` block of the C family, and the comments that run to the end of a
    // line. Elixir writes its heredocs and its comments exactly as Python does; Swift and C#
    // write the same `"""` block with the C family's comments around it. SQL and Terraform have
    // the `/* */` block but no template; SQL keeps the `//` comment Snowflake writes, and its
    // backtick quotes a MySQL name of one line, whose `/*` opens nothing.
    let (heredoc, long_bracket, template, block_comment, line_comments): (_, _, _, _, &[&str]) =
        match kind {
            Kind::Python | Kind::Elixir => (true, false, false, false, &["#"]),
            Kind::Lua => (false, true, false, false, &["--"]),
            Kind::Zig => (false, false, false, false, &["//"]),
            Kind::Swift | Kind::CSharp => (true, false, true, true, &["//"]),
            // A YAML block scalar (`filters: |`) hides nothing: the keys dorny/paths-filter reads
            // out of one are what `steps.changes.outputs.backend` names.
            Kind::Shell | Kind::Make | Kind::Docker | Kind::Yaml => {
                (false, false, false, false, &["#"])
            }
            Kind::Sql => (false, false, false, true, &["--", "//"]),
            // PowerShell's `<# #>` and here-strings are its own forms, below; a backtick is its
            // escape and its line continuation, never a template (#420).
            Kind::PowerShell => (false, false, false, false, &["#"]),
            Kind::Terraform => (false, false, false, true, &["#", "//"]),
            // GraphQL writes its descriptions in `"""` block strings, and nothing else runs
            // over lines: no `'''`, no `/* */`, no backtick.
            Kind::Graphql => (true, false, false, false, &["#"]),
            // The C family's comments, and no backtick: a Protocol Buffers string ends with its
            // line.
            Kind::Proto => (false, false, false, true, &["//"]),
            // `/* */` in all four stylesheet languages and `//` in SCSS, Sass and Less: one kind
            // reads `//` in a `.css` file too, at the cost of a `/*` after a `url(//…)` on its
            // line (#415). A string ends with its line, and a backtick is nothing.
            Kind::Css => (false, false, false, true, &["//"]),
            // PHP's `#` is a comment as `//` is, save `#[`, which opens an attribute (#488), and
            // only in PHP's code: outside `<?php … ?>` it is the `#id` of CSS, the `#field` of
            // JS or the `&#8212;` of HTML. A line comment ends at `?>` too, as PHP ends it.
            Kind::Php => (false, false, true, true, &["//", "#"]),
            Kind::Rust => (false, false, false, true, &["//"]),
            Kind::Ruby => (false, false, false, false, &["#"]),
            // A Kotlin raw string and a Java text block run over lines between `"""`; neither
            // language has a backtick template: Kotlin's backticks quote a name (#367).
            Kind::Jvm => (true, false, false, true, &["//"]),
            // Dart's `'''` and `"""` (raw `r'''` too) and the C family's comments, `///` among
            // them; no backtick: a Dart string of one quote ends with its line (#414).
            Kind::Dart => (true, false, false, true, &["//"]),
            Kind::Cmake => (false, true, false, false, &["#"]),
            _ => (false, false, true, true, &["//"]),
        };
    // The forms one language each has: C#'s verbatim string, which closes on a `"` that no
    // second `"` follows, since `""` is how it writes a quote, and PHP's `<<<ID`, which closes on
    // the line that repeats its label, as the shell's `<<ID` does in a script, a Dockerfile's
    // `RUN` and Terraform.
    let (verbatim_strings, labelled) = (kind == Kind::CSharp, kind == Kind::Php);
    let shell_heredoc = matches!(kind, Kind::Shell | Kind::Docker | Kind::Terraform);
    // In a shell script, as in a Dockerfile, a `#` opens a comment only where a word starts: `$#`
    // and `${f##*/}` are no comments. A shell quote ends with its line, as every other kind's
    // does: the scan cannot follow a `"…"` inside `"$( … )"`, so a quote it carried over lines
    // would hide the rest of the file behind one misread, and what `eval '…'` holds the shell
    // does declare.
    let word_comment = matches!(kind, Kind::Shell | Kind::Docker | Kind::PowerShell);
    let powershell = kind == Kind::PowerShell;
    // Whether the scan of a PHP file is between `<?php` (or `<?=`) and `?>`.
    let mut php_code = false;
    let b = text.as_bytes();
    let ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    // Whether a token starts at `at`, bare or behind one of `prefixes` (the `b` of `br"…"`).
    let token_at = |at: usize, prefixes: &[&[u8]]| {
        [&b""[..]].iter().chain(prefixes).any(|p| {
            at >= p.len()
                && b[at - p.len()..at] == **p
                && (at == p.len() || !ident(b[at - p.len() - 1]))
        })
    };
    let mut out = vec![false];
    // The multi-line literal the scan is in, by its closing bytes, and, for a long bracket, the
    // number of `=` its closer carries; a one-line quote. A heredoc has no closing bytes at all:
    // `label` holds the word its last line repeats, `exact` whether that line must hold it alone
    // (Ruby's, and whether indented), and `verbatim` marks a `@"…"`, `raw` a Rust string with no
    // escapes. Ruby's heredocs opened on the line read so far wait in `pending`, in order.
    let (mut block, mut level, mut quote, mut i): (Option<Cow<[u8]>>, usize, Option<u8>, usize) =
        (None, 0, None, 0);
    let (mut verbatim, mut raw, mut label, mut exact) = (false, false, Vec::new(), None);
    let mut pending: Vec<(Vec<u8>, bool)> = Vec::new();
    // The braces open in each TypeScript template substitution the scan is in, innermost last.
    let mut holes: Vec<usize> = Vec::new();
    // Where the Rust string the scan is in opened, and whether `at` is inside one.
    let (mut string_from, mut inside) = (None, false);
    let comment = if kind == Kind::Cmake {
        &b"#["[..]
    } else {
        b"--["
    };
    let opens = |at: usize| -> Option<(usize, usize)> {
        let open = if b[at..].starts_with(comment) {
            at + comment.len() - 1
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
            quote = None;
            // Ruby's heredoc starts on the line after the one that opened it.
            if label.is_empty() && !pending.is_empty() {
                let (word, indented) = pending.remove(0);
                (label, exact, block) = (word, Some(indented), Some(b"".into()));
            }
            // A heredoc ends on the line that repeats its label, as `ID;`, `ID,` or `ID)`; a
            // Ruby one on the line that is the label alone, indented only after `<<~` or `<<-`.
            if !label.is_empty() {
                let rest = &b[i + 1..];
                let word = rest
                    .iter()
                    .position(|c| !c.is_ascii_whitespace())
                    .map_or(rest, |n| &rest[n..]);
                let ends = match exact {
                    Some(indented) => {
                        let line = rest.split(|&c| c == b'\n').next().unwrap_or(rest);
                        line.trim_ascii() == label && (indented || line.starts_with(&label))
                    }
                    None => {
                        word.starts_with(&label[..])
                            && !word[label.len()..].first().is_some_and(|&c| ident(c))
                    }
                };
                if ends {
                    label.clear();
                    block = None;
                }
            }
            out.push(block.is_some());
        } else if let Some(end) = block.as_deref()
            && !(kind == Kind::TsJs
                && end == b"`"
                && b[i..].starts_with(b"${")
                && b[i - 1] != b'\\')
        {
            let doubled = verbatim && c == b'"' && b.get(i + 1) == Some(&b'"');
            let escape = matches!(kind, Kind::Rust | Kind::Cmake)
                && !raw
                && end == b"\""
                && c == b'\\'
                && b.get(i + 1) != Some(&b'\n');
            let bracket = long_bracket && end == b"]]";
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
                label.is_empty()
                    && !doubled
                    && b[i..].starts_with(end)
                    && (end.len() > 1
                        || matches!(kind, Kind::Go | Kind::Rust | Kind::Cmake)
                        || verbatim
                        || b[i - 1] != b'\\')
            };
            let skip = end.len().saturating_sub(1);
            if escape {
                i += 1;
            } else if closes {
                block = None;
                verbatim = false;
                if let Some(from) = string_from.take() {
                    inside |= from < at && at < i;
                }
                i += if bracket { level + 1 } else { skip };
            } else if doubled {
                i += 1;
            }
        } else if block.is_some() {
            // A template's `${` opens code, up to the `}` that closes it: a template in there
            // is one of its own, `${a ? `${b}/` : ""}`, and the rest of the outer one follows.
            holes.push(0);
            block = None;
            i += 1;
        } else if !holes.is_empty()
            && quote.is_none()
            && (c == b'{' || c == b'}')
            && b[i - 1] != b'\\'
        {
            // A regex's `\{` is no brace (`${s.replace(/\{/g, "")}`). ponytail: the `{` of
            // `/[{]/` still counts; skip regex literals in a hole if one shows up.
            let depth = holes.last_mut().expect("not empty");
            match (c, *depth) {
                (b'{', _) => *depth += 1,
                (_, 0) => {
                    holes.pop();
                    block = Some(b"`".into());
                }
                _ => *depth -= 1,
            }
        } else if let Some(q) = quote {
            // A backslash escapes the next byte, but the end of a line is still one. PowerShell
            // escapes with a backtick, and only in a `"…"`.
            let escape = match powershell {
                true => q == b'"' && c == b'`',
                false => c == b'\\',
            };
            if escape && b.get(i + 1) != Some(&b'\n') {
                i += 1;
            } else if c == q {
                quote = None;
            }
        } else if heredoc
            && (b[i..].starts_with(b"\"\"\"")
                || (matches!(kind, Kind::Python | Kind::Elixir | Kind::Dart)
                    && b[i..].starts_with(b"'''")))
        {
            // `'''` is Python's, Elixir's and Dart's alone; Swift, C#, GraphQL, Java and Kotlin
            // write the block with `"` only.
            block = Some(if c == b'"' { b"\"\"\"" } else { b"'''" }.into());
            i += 2;
        } else if powershell && b[i..].starts_with(b"<#") {
            block = Some(b"#>".into());
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
            (block, label, exact) = (Some(b"".into()), vec![b[i + 1], b'@'], None);
            i += 1;
        } else if powershell && c == b'`' {
            // An escape outside a string: `` `" `` opens nothing, `` `# `` no comment. At the end
            // of a line it continues the line, and the line still ends.
            if !matches!(b.get(i + 1), Some(b'\n' | b'\r')) {
                i += 1;
            }
        } else if verbatim_strings && (b[i..].starts_with(b"@\"") || b[i..].starts_with(b"@$\"")) {
            // `$@"` is read from its `@"`.
            block = Some(b"\"".into());
            verbatim = true;
            i += if b[i + 1] == b'$' { 2 } else { 1 };
        } else if labelled && b[i..].starts_with(b"<<<") {
            // `<<<SQL`, `<<<"SQL"` or `<<<'SQL'`: the label is what ends it.
            let word: Vec<u8> = b[i + 3..]
                .iter()
                .skip_while(|c| **c == b'"' || **c == b'\'')
                .take_while(|c| ident(**c))
                .copied()
                .collect();
            if !word.is_empty() {
                i += 2;
                block = Some(b"".into());
                label = word;
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
                block = Some(b"".into());
                label = word;
            }
            i += 1;
        } else if let Some((eq, skip)) = long_bracket.then(|| opens(i)).flatten() {
            block = Some(b"]]".into());
            level = eq;
            i += skip;
        } else if template && c == b'`' {
            // ponytail: `/`/` is a regex, told by the slash in front; a division by a template
            // is not written.
            if i == 0 || b[i - 1] != b'/' {
                block = Some(b"`".into());
            }
        } else if block_comment && b[i..].starts_with(b"/*") {
            block = Some(b"*/".into());
            i += 1;
        } else if kind == Kind::Rust && c == b'"' {
            // `"…"`, `b"…"` and `c"…"` run over lines; a `\` at the end of one continues it.
            (block, raw, string_from) = (Some(b"\"".into()), false, Some(i));
        } else if kind == Kind::Cmake && c == b'"' {
            block = Some(b"\"".into());
        } else if let Some(hashes) = (kind == Kind::Rust && c == b'r' && token_at(i, &[b"b", b"c"]))
            .then(|| b[i + 1..].iter().take_while(|&&c| c == b'#').count())
            .filter(|n| b.get(i + 1 + n) == Some(&b'"'))
        {
            // `r"…"`, `r#"…"#`, `br##"…"##`: closed by a `"` and as many `#`. `r#type` is a name.
            let mut end = vec![b'"'];
            end.resize(hashes + 1, b'#');
            (block, raw, string_from) = (Some(end.into()), true, Some(i));
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
            block = Some([&b")"[..], delim, b"\""].concat().into());
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
            (block, label, exact) = (Some(b"".into()), b"=end".to_vec(), None);
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
                pending.push((b[from..from + len].to_vec(), flag));
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
            quote = Some(c);
        } else if c == b'"'
            || (c == b'\'' && kind != Kind::Cmake)
            || (matches!(kind, Kind::Sql | Kind::Ruby) && c == b'`')
        {
            quote = Some(c);
        } else if kind == Kind::Php && (b[i..].starts_with(b"<?") || b[i..].starts_with(b"?>")) {
            php_code = c == b'<';
            i += 1;
        } else if line_comments
            .iter()
            .any(|m| b[i..].starts_with(m.as_bytes()))
            && !(word_comment && i > 0 && !b" \t\n;&|()<>".contains(&b[i - 1]))
            && !(kind == Kind::Php && c == b'#' && (!php_code || b[i..].starts_with(b"#[")))
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
    if let Some(from) = string_from {
        inside |= from < at && at <= b.len();
    }
    (out, inside)
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
        // A backslash escapes the next byte, so `"\\"` ends on its second quote.
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
            b'#' if kind == Kind::Python => {
                comment = true;
                Some((i, 0))
            }
            b'/' if kind != Kind::Python && b.get(i + 1) == Some(&b'/') => {
                comment = true;
                Some((i, 0))
            }
            _ => Some((i, c)),
        }
    })
}
/// `s` without its comments.
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
/// The text inside the bracket that opens at byte `open` of line `at`, over the lines it spans and
/// without comments, and the index of the line it closes on with what follows it there.
pub(super) fn group<'a>(
    kind: Kind,
    lines: &[&'a str],
    at: usize,
    open: usize,
) -> Option<(String, usize, &'a str)> {
    // ponytail: a bracket still open 2000 lines on is not read; fastapi's signatures run to 350.
    let text = lines[at..lines.len().min(at + 2000)].join("\n");
    let end = close_of(kind, &text, open)?;
    let line_start = text[..end].rfind('\n').map_or(0, |n| n + 1);
    let last = at + text[..end].matches('\n').count();
    let inner = uncommented(kind, &text[open + 1..end - 1]);
    Some((inner, last, &lines[last][end - line_start..]))
}
/// Whether `word` stands in `s` as a whole name, not as the tail of `a.word`.
pub(super) fn names(s: &str, word: &str) -> bool {
    s.match_indices(word).any(|(i, _)| {
        let is_name = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.';
        !s[..i].ends_with(is_name) && !s[i + word.len()..].starts_with(is_name)
    })
}
