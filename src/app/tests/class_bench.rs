use super::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use tree_sitter::{Node, Point};

const CONTAINERS: &[&str] = &[
    "class_definition",
    "class_declaration",
    "abstract_class_declaration",
    "class",
    "class_specifier",
    "struct_specifier",
    "union_specifier",
    "enum_specifier",
    "namespace_definition",
    "namespace_declaration",
    "file_scoped_namespace_declaration",
    "function_definition",
    "function_declaration",
    "generator_function_declaration",
    "function_signature",
    "function_item",
    "impl_item",
    "trait_item",
    "mod_item",
    "struct_item",
    "enum_item",
    "union_item",
    "method_declaration",
    "method_definition",
    "constructor_declaration",
    "method",
    "singleton_method",
    "module",
    "internal_module",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "struct_declaration",
    "trait_declaration",
    "annotation_type_declaration",
    "protocol_declaration",
    "object_declaration",
    "companion_object",
    "object_definition",
    "trait_definition",
    "type_spec",
    "variable_declarator",
    "contract_declaration",
    "type_alias_declaration",
    "library_declaration",
];

fn after_an_error_in_a_cpp_body(n: Node, root: Node) -> bool {
    let body = n
        .parent()
        .is_some_and(|p| matches!(p.kind(), "declaration_list" | "field_declaration_list"));
    let class_read_as_a_function = |x: Node| {
        x.kind() == "function_definition"
            && x.child_by_field_name("type")
                .is_some_and(|t| matches!(t.kind(), "class_specifier" | "struct_specifier"))
    };
    let mut s = n.prev_sibling();
    while let Some(x) = s
        && !x.is_error()
        && !class_read_as_a_function(x)
    {
        s = x.prev_sibling();
    }
    root.kind() == "translation_unit" && body && s.is_some()
}

fn prototype(n: Node) -> bool {
    let mut up = n.parent();
    while let Some(p) = up
        && p.kind().ends_with("declarator")
    {
        up = p.parent();
    }
    n.kind() == "function_declarator" && up.is_none_or(|p| p.kind() != "function_definition")
}

fn innermost_declarator(mut n: Node) -> Node {
    while let Some(d) = n.child_by_field_name("declarator") {
        n = d;
    }
    n
}

fn name_of<'a>(n: Node<'a>, src: &'a str) -> Option<Node<'a>> {
    match n.kind() {
        "impl_item" => n.child_by_field_name("type"),
        "function_declarator" => n.child_by_field_name("declarator"),
        "function_definition" if n.child_by_field_name("name").is_none() => n
            .child_by_field_name("declarator")
            .map(innermost_declarator),
        "companion_object" => None,
        _ => n.child_by_field_name("name").or_else(|| {
            let mut c = n.walk();
            n.named_children(&mut c).find(|k| {
                matches!(
                    k.kind(),
                    "type_identifier" | "simple_identifier" | "identifier"
                )
            })
        }),
    }
    .filter(|m| src.get(m.byte_range()).is_some())
}

fn segments(s: &str) -> Vec<String> {
    let mut depth = 0usize;
    let plain: String = s
        .chars()
        .filter(|&c| {
            match c {
                '<' | '[' | '(' => depth += 1,
                '>' | ']' | ')' => {
                    depth = depth.saturating_sub(1);
                    return false;
                }
                _ => {}
            }
            depth == 0
        })
        .collect();
    plain
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '$' | '?' | '!' | '\u{2026}')))
        .filter(|s| !s.is_empty() && *s != "struct")
        .map(str::to_owned)
        .collect()
}

fn go_receiver(n: Node, src: &str) -> Option<String> {
    let r = n.child_by_field_name("receiver")?;
    let text = src
        .get(r.byte_range())?
        .trim_start_matches('(')
        .trim_end_matches(')');
    segments(text).last().cloned()
}

fn php_tag_reference(
    root: Node,
    comment: Node,
    src: &str,
    row: usize,
    col: usize,
) -> Option<Vec<String>> {
    let tag = src.lines().nth(row)?.get(..col)?;
    let class = comment.next_named_sibling().filter(|c| {
        matches!(
            c.kind(),
            "class_declaration"
                | "interface_declaration"
                | "trait_declaration"
                | "enum_declaration"
        )
    })?;
    let name = class.child_by_field_name("name")?;
    let text = src.get(name.byte_range())?;
    let p = name.start_position();
    let mut chain = reference(root, src, p.row, p.column, text)?;
    chain.push(text.to_owned());
    (tag.contains("@method") || tag.contains("@property")).then_some(chain)
}

fn table_of<'a>(p: Node, leaf: Node, src: &'a str) -> Option<&'a str> {
    let table = if p.kind() == "call"
        && p.child_by_field_name("method")
            .and_then(|m| src.get(m.byte_range()))
            == Some("create_table")
    {
        p.child_by_field_name("arguments")?.named_child(0)?
    } else if p.kind() == "create_table"
        && leaf
            .parent()
            .filter(|c| c.kind() == "column_definition")
            .and_then(|c| c.child_by_field_name("name"))
            .is_some_and(|m| m.id() == leaf.id())
    {
        p.named_children(&mut p.walk())
            .find(|t| t.kind() == "object_reference")?
    } else {
        return None;
    };
    src.get(table.byte_range())
}

fn reference(root: Node, src: &str, row: usize, col: usize, word: &str) -> Option<Vec<String>> {
    let at = Point::new(row, col);
    let end = Point::new(row, col + word.len());
    let leaf = root.descendant_for_point_range(at, end)?;
    if leaf.kind() == "comment" {
        return php_tag_reference(root, leaf, src, row, col);
    }
    if src.get(leaf.byte_range())? != word {
        return None;
    }
    let mut chain: Vec<String> = Vec::new();
    let mut found_def = false;
    let before = src.get(..leaf.start_byte())?;
    let member_of_self = ["self.", "this.", "@"].iter().any(|s| before.ends_with(s))
        || matches!(leaf.kind(), "instance_variable" | "class_variable")
        || leaf
            .parent()
            .and_then(|p| p.parent())
            .is_some_and(|p| p.kind() == "property_promotion_parameter");
    let mut n = leaf;
    while let Some(p) = n.parent() {
        if after_an_error_in_a_cpp_body(n, root) {
            return None;
        }
        n = p;
        if let Some(table) = table_of(p, leaf, src) {
            let mut seg = segments(table);
            seg.append(&mut chain);
            chain = seg;
            continue;
        }
        if !CONTAINERS.contains(&p.kind()) && !prototype(p) {
            continue;
        }
        let function = p.kind().contains("function")
            || p.kind().contains("method")
            || p.kind().contains("constructor");
        if member_of_self && function {
            continue;
        }
        if p.child_by_field_name("name")
            .is_some_and(|m| m.kind().contains("string"))
        {
            continue;
        }
        if p.kind() == "variable_declarator"
            && p.child_by_field_name("value").is_none_or(|v| {
                !matches!(
                    v.kind(),
                    "arrow_function" | "function" | "function_expression" | "class" | "object"
                )
            })
        {
            if !found_def && p.start_byte() <= leaf.start_byte() {
                found_def = p
                    .child_by_field_name("name")
                    .is_some_and(|m| m.byte_range().contains(&leaf.start_byte()));
            }
            continue;
        }
        let Some(name) = name_of(p, src) else {
            continue;
        };
        let text = src.get(name.byte_range())?;
        if !found_def && name.byte_range().contains(&leaf.start_byte()) {
            found_def = true;
            let mut own = segments(text);
            own.pop();
            if p.kind() == "method_declaration"
                && let Some(r) = go_receiver(p, src)
            {
                own.insert(0, r);
            }
            let lead = src.get(p.start_byte()..name.start_byte()).unwrap_or("");
            if own.is_empty()
                && lead.ends_with('.')
                && let Some(r) = segments(lead).last()
            {
                own.push(r.clone());
            }
            chain.splice(0..0, own);
            continue;
        }
        let mut seg = segments(text);
        seg.append(&mut chain);
        chain = seg;
    }
    Some(chain)
}

fn rows(cache: &Path) -> Vec<(PathBuf, usize, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/d-bench");
    let mut out = HashSet::new();
    let mut words: HashMap<String, (String, String)> = HashMap::new();
    for f in std::fs::read_dir(dir.join("cursors")).unwrap() {
        let f = f.unwrap().path();
        for l in std::fs::read_to_string(&f).unwrap().lines() {
            let c: Vec<&str> = l.split('\t').collect();
            if l.starts_with('#') || c.len() < 7 {
                continue;
            }
            words.insert(c[0].to_owned(), (c[1].to_owned(), c[6].to_owned()));
        }
    }
    for f in std::fs::read_dir(dir.join("answers")).unwrap() {
        let f = f.unwrap().path();
        for l in std::fs::read_to_string(&f).unwrap().lines() {
            let c: Vec<&str> = l.split('\t').collect();
            let (Some(id), Some(targets)) = (c.first(), c.get(1)) else {
                continue;
            };
            let Some((project, word)) = words.get(*id) else {
                continue;
            };
            for t in targets.split(';').filter(|t| !t.starts_with(['~', '/'])) {
                if let Some((p, n)) = t.rsplit_once(':')
                    && let Ok(n) = n.parse()
                {
                    out.insert((cache.join(project).join(p), n, word.clone()));
                }
            }
        }
    }
    if let Ok(merl) = std::fs::read_to_string(cache.join("last-merl.tsv")) {
        for l in merl.lines() {
            let c: Vec<&str> = l.split('\t').collect();
            let (Some((_, word)), Some(&"picker"), Some(targets)) =
                (words.get(c[0]), c.get(1), c.get(4))
            else {
                continue;
            };
            for t in targets.split(';') {
                if let Some((p, n)) = t.rsplit_once(':')
                    && let Ok(n) = n.parse()
                    && p.starts_with(cache.to_str().unwrap())
                {
                    out.insert((PathBuf::from(p), n, word.clone()));
                }
            }
        }
    }
    let mut out: Vec<_> = out.into_iter().collect();
    out.sort();
    out
}

#[test]
#[ignore = "compares the class d's picker rows show with the syntax tree's, on the d bench's projects"]
fn class_bench() {
    let Ok(cache) = std::env::var("D_BENCH_CACHE") else {
        return;
    };
    let mut table: BTreeMap<String, [usize; 6]> = BTreeMap::new();
    let mut misses = String::new();
    let mut files: HashMap<PathBuf, Option<Buffer>> = HashMap::new();
    for (path, line1, word) in rows(Path::new(&cache)) {
        let Some(kind) = search::kind_of(&path) else {
            continue;
        };
        let b = files.entry(path.clone()).or_insert_with(|| {
            let bytes = std::fs::read(&path).ok()?;
            let mut b = Buffer::from_bytes(path.clone(), &bytes);
            b.parse_more(None);
            Some(b)
        });
        let Some(b) = b.as_ref() else { continue };
        let Some(text) = b.lines.get(line1.wrapping_sub(1)) else {
            continue;
        };
        let name = jvm::declared_as(kind, &word, text);
        let ext = path
            .extension()
            .map_or(String::new(), |e| e.to_string_lossy().into_owned());
        let all = b.lines.join("\n") + "\n";
        let ours = search::container(kind, &all, line1, &name).map(|c| segments(&c));
        let theirs = b.tree().and_then(|t| {
            text.match_indices(name.as_str())
                .find_map(|(col, _)| reference(t.root_node(), &all, line1 - 1, col, &name))
        });
        let row = table.entry(ext.clone()).or_default();
        row[0] += 1;
        let verdict = match (&ours, &theirs) {
            (_, None) => 4,
            (Some(o), Some(t)) if o == t => 1,
            (None, Some(t)) if t.is_empty() => 1,
            (None, Some(_)) => 3,
            (Some(o), Some(t)) if !o.is_empty() && (t.starts_with(o) || t.ends_with(o)) => 5,
            (Some(_), Some(_)) => 2,
        };
        row[verdict] += 1;
        if verdict != 1 {
            _ = writeln!(
                misses,
                "{ext}\t{verdict}\t{}:{line1}\t{name}\t{:?}\t{:?}",
                path.display(),
                ours,
                theirs
            );
        }
    }
    let mut out = String::from("ext\trows\tright\twrong\tno class\tno reference\tpart of it\n");
    for (ext, r) in &table {
        _ = writeln!(
            out,
            "{ext}\t{}\t{}\t{}\t{}\t{}\t{}",
            r[0], r[1], r[2], r[3], r[4], r[5]
        );
    }
    std::fs::write(std::env::var("CLASS_OUT").unwrap(), out + "\n" + &misses).unwrap();
}
