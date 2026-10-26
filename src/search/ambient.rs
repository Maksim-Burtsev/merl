use std::path::{Path, PathBuf};

use regex::Regex;

use super::{Hit, lexical};

pub fn ambient_module_pattern(module: &str) -> String {
    format!(
        r#"^declare\s+module\s+['"]{}['"]\s*\{{\s*$"#,
        regex::escape(module)
    )
}

pub fn ambient_export(text: &str, module: &str, name: &str) -> Option<usize> {
    let open = Regex::new(&ambient_module_pattern(module)).ok()?;
    let export = Regex::new(&format!(
        r"^\s+export\s+(?:declare\s+)?(?:async\s+)?(?:function\*?|const|let|var|class|interface|type|enum|namespace)\s+{}\b",
        regex::escape(name)
    ))
    .ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = (lines.iter().enumerate()).filter(|(_, l)| open.is_match(l));
    let (start, _) = blocks.next()?;
    if blocks.next().is_some() {
        return None;
    }
    let body = lines.get(start + 1..)?;
    let end = body.iter().position(|l| l.starts_with('}'))?;
    let at = body.get(..end)?.iter().position(|l| export.is_match(l))?;
    Some(start + at + 2)
}

pub fn mapped_source(dts: &Path, line1: usize) -> Option<(PathBuf, usize)> {
    let mut map = dts.as_os_str().to_owned();
    map.push(".map");
    let text = std::fs::read_to_string(map).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let sources = json.get("sources")?.as_array()?;
    let mappings = json.get("mappings")?.as_str()?;
    let root = json
        .get("sourceRoot")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let (mut source, mut line) = (0_i64, 0_i64);
    let mut named = Vec::new();
    let rows: Vec<&str> = mappings.split(';').take(line1).collect();
    if rows.len() < line1 {
        return None;
    }
    for row in rows {
        named.clear();
        for segment in row.split(',').filter(|s| !s.is_empty()) {
            match vlq(segment)?[..] {
                [_] => {}
                [_, s, l, _] | [_, s, l, _, _] => {
                    source += s;
                    line += l;
                    if !named.contains(&(source, line)) {
                        named.push((source, line));
                    }
                }
                _ => return None,
            }
        }
    }
    let [(source, line)] = named[..] else {
        return None;
    };
    let rel = sources.get(usize::try_from(source).ok()?)?.as_str()?;
    let path = lexical(&dts.parent()?.join(root).join(rel))?;
    let line1 = usize::try_from(line).ok()?.checked_add(1)?;
    path.is_file().then_some((path, line1))
}

fn vlq(segment: &str) -> Option<Vec<i64>> {
    let mut out = Vec::new();
    let (mut value, mut shift) = (0_i64, 0_u32);
    for b in segment.bytes() {
        let digit = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        value |= i64::from(digit & 31) << shift;
        if digit & 32 == 0 {
            out.push(if value & 1 == 1 {
                -(value >> 1)
            } else {
                value >> 1
            });
            (value, shift) = (0, 0);
        } else {
            shift += 5;
            if shift > 55 {
                return None;
            }
        }
    }
    (shift == 0).then_some(out)
}

pub fn ambient_target(dts: &Path, module: &str, name: &str) -> Option<Hit> {
    let text = std::fs::read_to_string(dts).ok()?;
    let line1 = ambient_export(&text, module, name)?;
    let mapped = mapped_source(dts, line1).and_then(|(path, line1)| {
        let source = std::fs::read_to_string(&path).ok()?;
        let line = source.lines().nth(line1 - 1)?.to_owned();
        Some((path, line1, line))
    });
    let (path, line1, text) = match mapped {
        Some(found) => found,
        None => (
            dts.to_path_buf(),
            line1,
            text.lines().nth(line1 - 1)?.to_owned(),
        ),
    };
    Some(Hit {
        deleted: None,
        path,
        line1,
        byte_col: None,
        text,
    })
}

pub fn ambient_member(dts: &Path, module: &str, name: &str, member: &str) -> Option<Hit> {
    let text = std::fs::read_to_string(dts).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let export = lines.get(ambient_export(&text, module, name)? - 1)?;
    let typed = Regex::new(r#":\s*import\(\s*['"]([^'"]+)['"]\s*\)\.(\w+)\s*;\s*$"#).ok()?;
    let caps = typed.captures(export)?;
    let interface = Regex::new(&format!(
        r"^\s+export\s+interface\s+{}\b",
        regex::escape(&caps[2])
    ))
    .ok()?;
    let open = Regex::new(&ambient_module_pattern(&caps[1])).ok()?;
    let mut blocks = (lines.iter().enumerate()).filter(|(_, l)| open.is_match(l));
    let (start, _) = blocks.next()?;
    if blocks.next().is_some() {
        return None;
    }
    let end = start
        + 1
        + lines
            .get(start + 1..)?
            .iter()
            .position(|l| l.starts_with('}'))?;
    let mut heads = (start + 1..end).filter(|&i| interface.is_match(lines[i]));
    let head = heads.next()?;
    if heads.next().is_some() {
        return None;
    }
    let key = Regex::new(&format!(
        r"^\s*(?:readonly\s+)?{}\??\s*[:(<]",
        regex::escape(member)
    ))
    .ok()?;
    let (mut depth, mut opened) = (0_usize, false);
    let mut found = None;
    for (i, line) in lines.iter().enumerate().take(end).skip(head) {
        let code = line.trim_start();
        if code.starts_with('*') || code.starts_with("/*") || code.starts_with("//") {
            continue;
        }
        if depth == 1 && key.is_match(line) {
            if found.is_some() {
                return None;
            }
            found = Some(i);
        }
        for c in code.chars() {
            match c {
                '{' => (depth, opened) = (depth + 1, true),
                '}' => depth = depth.checked_sub(1)?,
                _ => {}
            }
        }
        if opened && depth == 0 {
            break;
        }
    }
    let line1 = found? + 1;
    Some(Hit {
        deleted: None,
        path: dts.to_path_buf(),
        line1,
        byte_col: None,
        text: lines[line1 - 1].to_owned(),
    })
}

pub fn package_of(path: &Path) -> Option<PathBuf> {
    let parts: Vec<_> = path.components().collect();
    let at = parts
        .iter()
        .rposition(|c| c.as_os_str() == "node_modules")?;
    let rest = parts.get(at + 1..)?;
    let scoped = rest.first()?.as_os_str().to_string_lossy().starts_with('@');
    Some(rest.iter().take(1 + usize::from(scoped)).collect())
}

pub fn manifest_dependencies(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut names = Vec::new();
    for d in root
        .join(dir)
        .ancestors()
        .take_while(|d| d.starts_with(root))
    {
        let Some(json) = std::fs::read_to_string(d.join("package.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        else {
            continue;
        };
        for field in [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ] {
            if let Some(deps) = json.get(field).and_then(serde_json::Value::as_object) {
                names.extend(deps.keys().map(PathBuf::from));
            }
        }
    }
    names
}
