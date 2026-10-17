use std::ops::Range;
use std::sync::OnceLock;

use regex::Regex;
use tree_sitter::{
    CaptureQuantifier, Language, Node, Query, QueryCursor, QueryMatch, StreamingIterator,
};

use super::Patterns;

struct Head {
    root: Vec<u16>,
    parent: Vec<u16>,
    kind: Vec<u16>,
    names: Vec<Vec<u8>>,
    text: regex::bytes::Regex,
}

pub struct Split {
    light: Query,
    heavy: Query,
    heads: Vec<Head>,
}

fn ids(language: &Language, name: &str) -> Option<Vec<u16>> {
    if name == "_" {
        return Some(Vec::new());
    }
    let ids: Vec<u16> = (0..u16::try_from(language.node_kind_count()).ok()?)
        .filter(|&id| language.node_kind_for_id(id) == Some(name))
        .collect();
    let visible = ids.iter().all(|&id| language.node_kind_is_visible(id));
    (visible && !ids.is_empty()).then_some(ids)
}

fn uncommented(src: &str) -> String {
    let (mut out, mut quoted, mut escaped, mut comment) = (String::new(), false, false, false);
    for c in src.chars() {
        comment = comment && c != '\n';
        if comment || (c == ';' && !quoted) {
            comment = true;
            continue;
        }
        quoted ^= c == '"' && !escaped;
        escaped = c == '\\' && !escaped;
        out.push(c);
    }
    out
}

fn head(language: &Language, query: &Query, p: usize, src: &str) -> Option<Head> {
    static SHAPE: OnceLock<Regex> = OnceLock::new();
    let shape = SHAPE.get_or_init(|| {
        Regex::new(r"^\(?\s*\(\s*(\w+)\s+\(\s*(\w+)\s+\(\s*(\w+)\s*\)\s*@([\w.]+)")
            .expect("a valid regex")
    });
    if !query.is_pattern_rooted(p) {
        return None;
    }
    let src = &uncommented(src);
    let found = shape.captures(src)?;
    let [root, parent, kind, capture] =
        [1, 2, 3, 4].map(|i| found.get(i).map_or("", |m| m.as_str()));
    let index = query.capture_index_for_name(capture)?;
    if query.capture_quantifiers(p).get(index as usize) != Some(&CaptureQuantifier::One) {
        return None;
    }
    let test = Regex::new(&format!(
        r#"\(#(?:eq|nvim-any-of)\?\s+@{}((?:\s+"[^"\\]*")+)\s*\)"#,
        regex::escape(capture)
    ))
    .ok()?;
    let values = test.captures(src)?.get(1)?.as_str();
    let names: Vec<Vec<u8>> = values
        .split('"')
        .skip(1)
        .step_by(2)
        .map(|v| v.as_bytes().to_vec())
        .collect();
    if names.iter().any(Vec::is_empty) {
        return None;
    }
    let alternatives: Vec<String> = values
        .split('"')
        .skip(1)
        .step_by(2)
        .map(regex::escape)
        .collect();
    Some(Head {
        root: ids(language, root)?,
        parent: ids(language, parent)?,
        kind: ids(language, kind).filter(|k| !k.is_empty())?,
        names,
        text: regex::bytes::Regex::new(&alternatives.join("|")).ok()?,
    })
}

pub fn split(
    name: &str,
    language: &Language,
    query: &Query,
    text: &str,
) -> Result<Option<Split>, String> {
    let heads: Vec<(usize, Head)> = (0..query.pattern_count())
        .filter_map(|p| {
            let src = text.get(query.start_byte_for_pattern(p)..query.end_byte_for_pattern(p))?;
            Some((p, head(language, query, p, src)?))
        })
        .collect();
    if heads.is_empty() {
        return Ok(None);
    }
    let compiled = || Query::new(language, text).map_err(|e| format!("{name}: {e}"));
    let (mut light, mut heavy) = (compiled()?, compiled()?);
    for p in 0..query.pattern_count() {
        if heads.iter().any(|(h, _)| *h == p) {
            light.disable_pattern(p);
        } else {
            heavy.disable_pattern(p);
        }
    }
    Ok(Some(Split {
        light,
        heavy,
        heads: heads.into_iter().map(|(_, h)| h).collect(),
    }))
}

impl Split {
    pub fn disable(&mut self, pattern: usize) {
        self.light.disable_pattern(pattern);
        self.heavy.disable_pattern(pattern);
    }
}

fn any(ids: &[u16], id: u16) -> bool {
    ids.is_empty() || ids.contains(&id)
}

fn passes(heads: &[&Head], parent: u16, g: Node, bytes: &[u8]) -> bool {
    let kind = g.kind_id();
    heads.iter().any(|h| {
        h.kind.contains(&kind)
            && any(&h.parent, parent)
            && bytes
                .get(g.byte_range())
                .is_some_and(|t| h.names.iter().any(|n| n == t))
    })
}

fn named_at(heads: &[&Head], node: Node, bytes: &[u8]) -> Vec<usize> {
    let Some(hay) = bytes.get(node.byte_range()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for h in heads.iter().filter(|h| h.parent.is_empty()) {
        let mut at = 0;
        while let Some(m) = h.text.find_at(hay, at) {
            out.push(node.start_byte() + m.start());
            at = m.start() + 1;
        }
    }
    out.sort_unstable();
    out
}

fn may_start(heads: &[Head], node: Node, bytes: &[u8]) -> bool {
    let heads: Vec<&Head> = heads
        .iter()
        .filter(|h| any(&h.root, node.kind_id()))
        .collect();
    if heads.is_empty() {
        return false;
    }
    let starts = named_at(&heads, node, bytes);
    let mut next = starts.iter().peekable();
    let mut cursor = node.walk();
    let mut more = cursor.goto_first_child();
    while more {
        let child = cursor.node();
        let parent = child.kind_id();
        while next.next_if(|&&s| s < child.start_byte()).is_some() {}
        let named = next.peek().is_some_and(|&&s| s < child.end_byte());
        let kind = heads.iter().any(|h| h.parent.contains(&parent));
        if (named || kind) && cursor.goto_first_child() {
            loop {
                if passes(&heads, parent, cursor.node(), bytes) {
                    return true;
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        more = cursor.goto_next_sibling();
    }
    false
}

fn run<'t>(
    query: &Query,
    node: Node<'t>,
    range: &Range<usize>,
    depth: Option<u32>,
    bytes: &[u8],
    found: &mut impl FnMut(&QueryMatch<'_, 't>),
) {
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(range.clone());
    cursor.set_max_start_depth(depth);
    let mut matches = cursor.matches(query, node, bytes);
    while let Some(m) = matches.next() {
        found(m);
    }
}

pub fn each_match<'t>(
    patterns: &Patterns,
    root: Node<'t>,
    range: &Range<usize>,
    bytes: &[u8],
    ordered: bool,
    mut found: impl FnMut(&QueryMatch<'_, 't>),
) {
    let Some(split) = &patterns.split else {
        run(&patterns.query, root, range, None, bytes, &mut found);
        return;
    };
    let starts = starts(split, root, range, bytes);
    if ordered && !starts.is_empty() {
        run(&patterns.query, root, range, None, bytes, &mut found);
        return;
    }
    run(&split.light, root, range, None, bytes, &mut found);
    for node in starts {
        run(&split.heavy, node, range, Some(0), bytes, &mut found);
    }
}

fn starts<'t>(split: &Split, root: Node<'t>, range: &Range<usize>, bytes: &[u8]) -> Vec<Node<'t>> {
    let end = if range.end == 0 {
        usize::MAX
    } else {
        range.end
    };
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if may_start(&split.heads, node, bytes) {
            out.push(node);
        }
        let mut cursor = node.walk();
        let near = cursor.goto_first_child_for_byte(range.start.saturating_sub(1));
        if near.is_none() && !cursor.goto_first_child() {
            continue;
        }
        let first = stack.len();
        loop {
            let child = cursor.node();
            if child.start_byte() > end {
                break;
            }
            if child.end_byte() >= range.start {
                stack.push(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        stack[first..].reverse();
    }
    out
}

#[cfg(test)]
mod tests {
    use tree_sitter::{Parser, QueryCursor, StreamingIterator};

    use super::super::extends::themed;
    use super::super::{Patterns, lang};
    use super::{each_match, starts};

    type Set = Vec<(usize, Vec<(u32, usize, usize)>)>;

    fn add(set: &mut Set, p: &Patterns, m: &tree_sitter::QueryMatch, bytes: &[u8]) {
        if !p.matches(m, bytes) {
            return;
        }
        let captures = m
            .captures()
            .iter()
            .map(|c| (c.index, c.node.start_byte(), c.node.end_byte()))
            .collect();
        set.push((m.pattern_index, captures));
    }

    fn whole(
        p: &Patterns,
        root: tree_sitter::Node,
        range: std::ops::Range<usize>,
        bytes: &[u8],
    ) -> Set {
        let mut out = Set::new();
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(range);
        let mut matches = cursor.matches(&p.query, root, bytes);
        while let Some(m) = matches.next() {
            add(&mut out, p, m, bytes);
        }
        out
    }

    #[test]
    fn the_tag_name_patterns_of_html_are_split_off() {
        let html = lang("html").unwrap();
        let count = |p: &Patterns| p.split.as_ref().map_or(0, |s| s.heads.len());
        assert_eq!(count(&html.highlights), 13);
        let moonfly = themed(html, Some("moonfly"));
        assert!(!std::ptr::eq(moonfly, html));
        assert_eq!(count(&moonfly.highlights), 13);
        assert_eq!(count(html.injections.as_ref().unwrap()), 3);
        assert_eq!(count(&lang("python").unwrap().highlights), 0);
    }

    fn html() -> String {
        let item = |i: usize| match i % 6 {
            _ if i == 100 => "<b>\n".to_owned() + &"w <i>z</i>\n".repeat(300) + "</b>\n",
            _ if i == 150 => "</p><v    <style>a { color: red; }</style>\n".to_owned(),
            0 => format!("<h1>t{i}</h1>\n"),
            1 => format!("x{i} <b>y</b>\n"),
            2 => "<input pattern=\"[0-9]\">\n".to_owned(),
            3 => format!("<py-script>p{i}</py-script>\n"),
            4 => format!("<p><em>e</em> <code>c</code> {i}</p>\n"),
            _ if i % 4 == 1 => "<style>a { color: red; }</style>\n".to_owned(),
            _ => format!("text {i}\n"),
        };
        format!(
            "<html><body>\n{}</body></html>\n",
            (0..300).map(item).collect::<String>()
        )
    }

    fn repeated(lines: &[&str]) -> String {
        (0..300)
            .map(|i| lines[i % lines.len()].replace('N', &i.to_string()) + "\n")
            .collect()
    }

    #[test]
    fn matches_split_by_tag_name_are_the_matches_of_one_query_from_the_root() {
        let ruby = repeated(&[
            "require \"aN\"",
            "require_relative \"bN\"",
            "def fN(x) load \"c\" end",
            "load \"dN\"; xN = 1",
        ]);
        let terraform = repeated(&[
            "resource \"x\" \"yN\" {",
            "  a = var.fooN",
            "  b = path.root",
            "  c = terraform.workspace",
            "  d = local.zN.e",
            "}",
            "variable \"vN\" { type = string }",
        ]);
        let jsx = repeated(&[
            "const aN = <div><style jsx>{`a { color: red; }`}</style><p>{x}</p></div>;",
            "function fN() { return <style jsx>{`b { margin: 0; }`}</style>; }",
            "const bN = <style>{`c {}`}</style>;",
        ]);
        let html = html();
        let cases = [
            ("html", &html),
            ("vue", &html),
            ("svelte", &html),
            ("xml", &html),
            ("ruby", &ruby),
            ("terraform", &terraform),
            ("javascript", &jsx),
            ("tsx", &jsx),
        ];
        for (name, text) in cases {
            let bytes = text.as_bytes();
            let lang = lang(name).unwrap();
            let mut parser = Parser::new();
            parser.set_language(&lang.language).unwrap();
            let tree = parser.parse(text, None).unwrap();
            let root = tree.root_node();
            let sets = [
                Some(&lang.highlights),
                lang.injections.as_ref(),
                lang.combined.as_ref(),
                Some(&themed(lang, Some("moonfly")).highlights),
            ];
            let mut heavy = 0;
            for p in sets.into_iter().flatten() {
                if let Some(split) = &p.split {
                    heavy += starts(split, root, &(0..text.len()), bytes).len();
                }
                let ranges = (0..text.len())
                    .step_by(61)
                    .flat_map(|at| [at..at + 2, at.saturating_sub(40)..at + 40, at..at])
                    .chain(std::iter::once(0..text.len()));
                for range in ranges {
                    let one = whole(p, root, range.clone(), bytes);
                    let mut ordered = Set::new();
                    each_match(p, root, &range, bytes, true, |m| {
                        add(&mut ordered, p, m, bytes);
                    });
                    assert_eq!(ordered, one, "{name} {range:?} in order");
                    let mut split = Set::new();
                    each_match(p, root, &range, bytes, false, |m| {
                        add(&mut split, p, m, bytes);
                    });
                    split.sort_by_key(|(pattern, _)| *pattern);
                    let mut one = one;
                    one.sort_by_key(|(pattern, _)| *pattern);
                    assert_eq!(split, one, "{name} {range:?}");
                }
            }
            assert!(heavy > 0, "{name}");
        }
    }
}
