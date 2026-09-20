//! The lexer the rules stand on: comments, string literals, brackets and separators.

use super::*;

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
/// `[[ ]]` or `[==[ ]==]` long string or block comment, a C# verbatim `@"…"`, a PHP heredoc, a
/// `/* */` block. A line there that reads like a declaration declares nothing — the SQL a
/// migration embeds in one is the common case. Strings of one line end with their line, whatever
/// they hold.
///
/// Each kind says which forms it has rather than inheriting another language's: Zig has none at
/// all — a `\\` string ends with its line — and reading it with the backtick and `/* */` of the C
/// family would take the ``` ``` ``` fences of the markdown a `\\` block holds for a literal and
/// hide the rest of the file behind them.
///
/// ponytail: Elixir's `~S"""` sigil is read from its `"""`, and its one-line `~s(…)` forms not at
/// all.
pub fn literal_lines(kind: Kind, text: &str) -> Vec<bool> {
    // What this kind writes: the comment that runs to the end of a line, the triple quote of a
    // heredoc, Lua's long bracket, and the backtick template with the `/* */` block of the C
    // family. Elixir writes its heredocs and its comments exactly as Python does; Swift and C#
    // write the same `"""` block with the C family's comments around it.
    let (heredoc, long_bracket, template, line_comment): (bool, bool, bool, &[u8]) = match kind {
        Kind::Python | Kind::Elixir => (true, false, false, b"#"),
        Kind::Lua => (false, true, false, b"--"),
        Kind::Zig => (false, false, false, b"//"),
        Kind::Swift | Kind::CSharp => (true, false, true, b"//"),
        _ => (false, false, true, b"//"),
    };
    // The two forms one language each has: C#'s verbatim string, which closes on a `"` that no
    // second `"` follows, since `""` is how it writes a quote, and PHP's `<<<ID`, which closes on
    // the line that repeats its label.
    let (verbatim_strings, labelled) = (kind == Kind::CSharp, kind == Kind::Php);
    let b = text.as_bytes();
    let mut out = vec![false];
    // The multi-line literal the scan is in, by its closing bytes, and, for a long bracket, the
    // number of `=` its closer carries; a one-line quote. A heredoc has no closing bytes at all:
    // `label` holds the word its last line repeats, and `verbatim` marks a `@"…"`.
    let (mut block, mut level, mut quote, mut i): (Option<&[u8]>, usize, Option<u8>, usize) =
        (None, 0, None, 0);
    let (mut verbatim, mut label) = (false, Vec::new());
    // A long bracket opening at `at` — `[[` or `[==[`, behind `--` or not: how many `=` it
    // carries, and how far past `at` its second `[` sits. A `[` that opens nothing, as the one in
    // the `\[[A-Za-z]\+\]` of a Vim regex, is no opener, so the `[=[` around it has to be read.
    let opens = |at: usize| -> Option<(usize, usize)> {
        let open = if b[at..].starts_with(b"--[") {
            at + 2
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
        if c == b'\n' {
            quote = None;
            // A heredoc ends on the line that repeats its label, as `ID;`, `ID,` or `ID)`.
            if !label.is_empty() {
                let rest = &b[i + 1..];
                let word = rest
                    .iter()
                    .position(|c| !c.is_ascii_whitespace())
                    .map_or(rest, |n| &rest[n..]);
                if word.starts_with(&label[..])
                    && !word[label.len()..]
                        .first()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    label.clear();
                    block = None;
                }
            }
            out.push(block.is_some());
        } else if let Some(end) = block {
            // A long bracket closes on `]`, the `=` its opener carried, and `]`; a verbatim
            // string on a `"` that no second `"` follows; a heredoc only on its label, above.
            let doubled = verbatim && c == b'"' && b.get(i + 1) == Some(&b'"');
            let closes = if long_bracket {
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
                    && (end.len() > 1 || b[i - 1] != b'\\')
            };
            if closes {
                block = None;
                verbatim = false;
                i += if long_bracket {
                    level + 1
                } else {
                    end.len() - 1
                };
            } else if doubled {
                i += 1;
            }
        } else if let Some(q) = quote {
            // A backslash escapes the next byte, but the end of a line is still one.
            if c == b'\\' && b.get(i + 1) != Some(&b'\n') {
                i += 1;
            } else if c == q {
                quote = None;
            }
        } else if heredoc
            && (b[i..].starts_with(b"\"\"\"") || (!template && b[i..].starts_with(b"'''")))
        {
            // `'''` is Python's and Elixir's alone; Swift and C# write the block with `"` only.
            block = Some(if c == b'"' { b"\"\"\"" } else { b"'''" });
            i += 2;
        } else if verbatim_strings && b[i..].starts_with(b"@\"") {
            block = Some(b"\"");
            verbatim = true;
            i += 1;
        } else if labelled && b[i..].starts_with(b"<<<") {
            // `<<<SQL`, `<<<"SQL"` or `<<<'SQL'`: the label is what ends it.
            let word: Vec<u8> = b[i + 3..]
                .iter()
                .skip_while(|c| **c == b'"' || **c == b'\'')
                .take_while(|c| c.is_ascii_alphanumeric() || **c == b'_')
                .copied()
                .collect();
            if !word.is_empty() {
                i += 2;
                block = Some(b"");
                label = word;
            }
        } else if let Some((eq, skip)) = long_bracket.then(|| opens(i)).flatten() {
            block = Some(b"]]");
            level = eq;
            i += skip;
        } else if template && c == b'`' {
            // ponytail: `/`/` is a regex, told by the slash in front; a division by a template
            // is not written.
            if i == 0 || b[i - 1] != b'/' {
                block = Some(b"`");
            }
        } else if template && b[i..].starts_with(b"/*") {
            block = Some(b"*/");
            i += 1;
        } else if c == b'"' || c == b'\'' {
            quote = Some(c);
        } else if b[i..].starts_with(line_comment) {
            while i + 1 < b.len() && b[i + 1] != b'\n' {
                i += 1;
            }
        }
        i += 1;
    }
    out
}
/// The bytes of `s` a scan for brackets and separators reads, with their indexes. String literals
/// are skipped; a comment (`#` in Python, `//` elsewhere) yields its first byte as `0` and is
/// skipped to the end of its line.
pub(super) fn code(kind: Kind, s: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    let b = s.as_bytes();
    let (mut quote, mut comment) = (None, false);
    (0..b.len()).filter_map(move |i| {
        let c = b[i];
        if comment {
            comment = c != b'\n';
            return None;
        }
        if let Some(q) = quote {
            if c == q && b[i - 1] != b'\\' {
                quote = None;
            }
            return None;
        }
        match c {
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
