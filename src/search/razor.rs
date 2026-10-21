use std::path::Path;

pub fn razor(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "razor")
}

pub fn razor_usings(text: &str) -> Vec<String> {
    (text.lines())
        .filter_map(|l| {
            l.trim()
                .trim_start_matches('\u{feff}')
                .strip_prefix("@using ")
        })
        .filter(|rest| !rest.trim_start().starts_with('('))
        .map(|rest| format!("using {};", rest.trim().trim_end_matches(';')))
        .collect()
}

pub fn razor_view(
    path: &Path,
    lines: &[String],
    cursor: usize,
    imports: &str,
) -> (Vec<String>, Option<usize>) {
    let mut view = Vec::with_capacity(lines.len());
    let mut code: Option<i64> = None;
    let mut opening = false;
    let mut comment = false;
    for line in lines {
        let t = line.trim().trim_start_matches('\u{feff}');
        let lead = line.len() - line.trim_start().trim_start_matches('\u{feff}').len();
        let v = if let Some(depth) = code.as_mut() {
            *depth += braces(line);
            if *depth <= 0 {
                code = None;
            }
            line.clone()
        } else if opening {
            opening = t.is_empty();
            if t.starts_with('{') {
                code = Some(braces(line)).filter(|&d| d > 0);
            }
            String::new()
        } else if let Some(rest) = t
            .strip_prefix("@code")
            .or_else(|| t.strip_prefix("@functions"))
            && (rest.is_empty() || rest.starts_with([' ', '{']))
        {
            code = Some(braces(line)).filter(|&d| d > 0);
            opening = code.is_none() && !rest.contains('{');
            String::new()
        } else if let Some(rest) = t.strip_prefix("@inject ") {
            directive(lead + "@inject ".len(), rest)
        } else if t
            .strip_prefix("@using ")
            .is_some_and(|r| !r.trim_start().starts_with('('))
            || t.starts_with('@')
                && (t.get(1..).and_then(|r| r.split_whitespace().next()))
                    .is_some_and(|d| DIRECTIVES.contains(&d))
        {
            String::new()
        } else if !t.starts_with('<') && !t.starts_with('@') && t.ends_with(';') && !comment {
            line.clone()
        } else {
            markup(line, &mut comment)
        };
        view.push(v);
    }
    let class = view
        .iter()
        .enumerate()
        .find(|&(i, v)| i != cursor && v.trim().is_empty())
        .map(|(i, _)| i);
    let name = path.file_stem().and_then(|s| s.to_str());
    if let (Some(i), Some(name)) = (class, name)
        && let Some(v) = view.get_mut(i)
    {
        *v = format!("partial class {name} {{");
    }
    if let (Some(v), Some(l)) = (view.get_mut(cursor), lines.get(cursor))
        && v.is_empty()
    {
        *v = " ".repeat(l.len());
    }
    view.extend(razor_usings(&lines.join("\n")));
    view.extend(razor_usings(imports));
    (view, class)
}

const DIRECTIVES: &[&str] = &[
    "page",
    "attribute",
    "layout",
    "namespace",
    "implements",
    "inherits",
    "typeparam",
    "rendermode",
    "preservewhitespace",
    "model",
    "addTagHelper",
    "removeTagHelper",
    "tagHelperPrefix",
];

const STATEMENTS: &[&str] = &[
    "if", "foreach", "for", "while", "switch", "using", "lock", "do", "try",
];

fn directive(at: usize, rest: &str) -> String {
    let rest = rest.trim_end();
    let semi = if rest.ends_with(';') { "" } else { ";" };
    format!("{}{rest}{semi}", " ".repeat(at))
}

fn braces(line: &str) -> i64 {
    let count = |c: char| i64::try_from(line.matches(c).count()).unwrap_or(0);
    count('{') - count('}')
}

fn markup(line: &str, comment: &mut bool) -> String {
    let b = line.as_bytes();
    let mut out = vec![b' '; b.len()];
    let mut i = 0;
    while i < b.len() {
        if *comment {
            match line.get(i..).and_then(|r| r.find("*@")) {
                Some(end) => {
                    *comment = false;
                    i += end + 2;
                }
                None => break,
            }
            continue;
        }
        let Some(at) = line.get(i..).and_then(|r| r.find('@')).map(|a| a + i) else {
            break;
        };
        let next = b.get(at + 1).copied().unwrap_or(b' ');
        let after_word = at > 0 && b.get(at - 1).is_some_and(u8::is_ascii_alphanumeric);
        i = at + 1;
        if next == b'*' {
            *comment = true;
            i = at + 2;
            continue;
        }
        if after_word || next == b'@' {
            i = at + 2;
            continue;
        }
        let end = if next == b'(' {
            group(b, at + 1)
        } else if ident_start(next) {
            let word_end = ident_end(b, at + 1);
            let word = line.get(at + 1..word_end).unwrap_or_default();
            if STATEMENTS.contains(&word) {
                Some(b.len())
            } else if b
                .get(word_end)
                .is_some_and(|c| matches!(c, b'=' | b':' | b'-'))
            {
                let name_end = (word_end..b.len())
                    .find(|&j| {
                        b.get(j)
                            .is_none_or(|c| !(c.is_ascii_alphanumeric() || *c == b'-'))
                    })
                    .unwrap_or(b.len());
                let name = line.get(at + 1..name_end).unwrap_or_default();
                let code = name.starts_with("on")
                    || name == "bind"
                    || name.starts_with("bind-")
                    || matches!(name, "ref" | "key" | "attributes");
                let value = b.get(name_end..name_end + 2) == Some(b"=\"")
                    && b.get(name_end + 2) != Some(&b'@');
                match (code && value)
                    .then(|| line.get(name_end + 2..)?.find('"'))
                    .flatten()
                {
                    Some(len) => {
                        let from = name_end + 2;
                        keep(&mut out, b, from, from + len);
                        i = from + len + 1;
                    }
                    None => i = name_end,
                }
                None
            } else {
                Some(chain(b, word_end))
            }
        } else {
            None
        };
        if let Some(end) = end {
            keep(&mut out, b, at + 1, end);
            i = end;
        }
    }
    let kept = String::from_utf8(out).unwrap_or_default();
    if kept.trim().is_empty() {
        String::new()
    } else {
        kept
    }
}

fn keep(out: &mut [u8], b: &[u8], from: usize, to: usize) {
    if let (Some(o), Some(src)) = (out.get_mut(from..to), b.get(from..to)) {
        o.copy_from_slice(src);
    }
    if let Some(c) = out.get_mut(to) {
        *c = b';';
    }
}

fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn ident_end(b: &[u8], from: usize) -> usize {
    (from..b.len())
        .find(|&j| {
            b.get(j)
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || *c == b'_'))
        })
        .unwrap_or(b.len())
}

fn group(b: &[u8], from: usize) -> Option<usize> {
    let (open, close) = match b.get(from) {
        Some(b'(') => (b'(', b')'),
        Some(b'[') => (b'[', b']'),
        _ => return None,
    };
    let mut depth = 0usize;
    for (j, &c) in b.iter().enumerate().skip(from) {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(j + 1);
            }
        }
    }
    None
}

fn chain(b: &[u8], mut end: usize) -> usize {
    loop {
        if let Some(e) = group(b, end) {
            end = e;
            continue;
        }
        let dot = match (b.get(end), b.get(end + 1)) {
            (Some(b'?'), Some(b'.')) => 2,
            (Some(b'.'), _) => 1,
            _ => return end,
        };
        match b.get(end + dot) {
            Some(&c) if ident_start(c) => end = ident_end(b, end + dot),
            _ => return end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(text: &str, cursor: usize) -> Vec<String> {
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        razor_view(
            Path::new("Pages/CartPage.razor"),
            &lines,
            cursor,
            "@using Shop.Shared\n<p>x</p>",
        )
        .0
    }

    #[test]
    fn a_razor_file_reads_as_a_class_of_its_injects_expressions_and_code() {
        let text = "\u{feff}@page \"/cart\"\n@using Shop.Services\n@inject BasketState Basket\n\n<div class='cart'>\n    @if (items is null)\n    {\n        <p>Loading</p>\n    }\n    <img src=\"@Images.Url(item.Id)\" @key=\"@item.Id\" @onclick=\"@(() => Go(1))\" />\n    <a href=\"mailto:me@shop.com\">@@me</a> $@item.Price.Show().\n    @* @Hidden() *@\n    var q = Pending(item);\n</div>\n\n@code {\n    private int count;\n\n    private async Task Go(int n)\n    {\n        await Basket.Add(n);\n    }\n}\n<p>@after</p>";
        let v = view(text, 9);
        let want = [
            "partial class CartPage {",
            "",
            "        BasketState Basket;",
            "",
            "",
            "     if (items is null)",
            "",
            "",
            "",
            "               Images.Url(item.Id);        item.Id;            (() => Go(1));   ",
            "                                            item.Price.Show();",
            "",
            "    var q = Pending(item);",
            "",
            "",
            "",
            "    private int count;",
            "",
            "    private async Task Go(int n)",
            "    {",
            "        await Basket.Add(n);",
            "    }",
            "}",
            "    after;   ",
            "using Shop.Services;",
            "using Shop.Shared;",
        ];
        assert_eq!(v, want);
        for (v, l) in v.iter().zip(text.lines()).skip(4) {
            for (i, c) in v
                .bytes()
                .enumerate()
                .filter(|&(_, c)| !matches!(c, b' ' | b'}' | b';'))
            {
                assert_eq!(l.as_bytes()[i], c, "{l}");
            }
        }
        assert_eq!(
            view(text, 0)[0],
            " ".repeat(text.lines().next().unwrap().len())
        );
        assert_eq!(view(text, 0)[1], "partial class CartPage {");
    }

    #[test]
    fn a_directive_attribute_keeps_its_value_only_when_it_is_code() {
        let line = r#"<b @onclick="Settle" @bind:event="oninput" @formname="cart" @key="@k" @bind-Value="Total">"#;
        let want = r"             Settle;                                               k;              Total; ";
        assert_eq!(view(line, 1)[0], want);
    }

    #[test]
    fn a_code_block_opens_on_its_own_line_too() {
        let v = view("<p>@x</p>\n@code\n{\n    int x;\n}\n", 0);
        assert_eq!(
            v,
            [
                "    x;   ",
                "partial class CartPage {",
                "",
                "    int x;",
                "}",
                "using Shop.Shared;"
            ]
        );
    }
}
