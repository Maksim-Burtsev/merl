use regex::Regex;
use std::path::Path;

pub fn dotted_at(line: &str, col: usize) -> Option<(Vec<&str>, usize)> {
    let name = |b: &u8| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'.';
    let bytes = line.as_bytes();
    let col = col.min(bytes.len());
    let start = bytes[..col]
        .iter()
        .rposition(|b| !name(b))
        .map_or(0, |i| i + 1);
    let end = bytes[col..]
        .iter()
        .position(|b| !name(b))
        .map_or(bytes.len(), |i| col + i);
    let mut segments: Vec<&str> = line[start..end].split('.').collect();
    if line[end..].starts_with('(') {
        segments.pop();
    }
    let at = line[start..col].matches('.').count();
    (at < segments.len() && segments.iter().all(|s| !s.is_empty())).then_some((segments, at))
}

pub fn catalog_key(segments: &[&str]) -> Option<(&'static str, String)> {
    let ["libs", rest @ ..] = segments else {
        return None;
    };
    let (section, key) = match rest {
        ["plugins", key @ ..] => ("plugins", key),
        ["bundles", key @ ..] => ("bundles", key),
        ["versions", key @ ..] => ("versions", key),
        key => ("libraries", key),
    };
    (!key.is_empty()).then(|| (section, key.join("-")))
}

pub fn catalog_line(text: &str, section: &str, key: &str) -> Option<usize> {
    let mut current = "";
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if let Some((name, _)) = line.strip_prefix('[').and_then(|l| l.split_once(']')) {
            current = name.trim();
        } else if current == section
            && let Some((k, _)) = line.split_once('=')
            && k.trim().trim_matches('"').replace(['_', '.'], "-") == key
        {
            return Some(i + 1);
        }
    }
    None
}

const FILE_TYPES: &[&str] = &["drawable", "mipmap", "layout", "raw", "font"];

pub fn value_tags(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "string" => "string",
        "plurals" => "plurals",
        "array" => "string-array|integer-array|array",
        "dimen" => "dimen",
        "color" => "color",
        "bool" => "bool",
        "integer" => "integer",
        _ => return None,
    })
}

pub fn resource_at<'a>(
    text: &str,
    segments: &[&'a str],
    at: usize,
) -> Option<(&'a str, &'a str, &'a str)> {
    let [prefix @ .., receiver, ty, name] = segments else {
        return None;
    };
    let known = value_tags(ty).is_some() || *ty == "id" || FILE_TYPES.contains(ty);
    let platform = Regex::new(r"(?m)^\s*import\s+android\.R\s*;?\s*$").ok()?;
    let receiver_ok = match prefix {
        [] if *receiver == "R" => !platform.is_match(text),
        [] => Regex::new(&format!(
            r"(?m)^\s*import\s+[\w.]+\.R\s+as\s+{}\s*$",
            regex::escape(receiver)
        ))
        .ok()?
        .is_match(text),
        ["android"] => false,
        _ => *receiver == "R",
    };
    (at == segments.len() - 1 && known && receiver_ok).then_some((receiver, ty, name))
}

pub fn in_res(path: &Path, dir: &str) -> bool {
    let parent = path.parent();
    let folder = parent.and_then(|p| p.file_name()).and_then(|n| n.to_str());
    parent.and_then(|p| p.parent()).and_then(|p| p.file_name()) == Some("res".as_ref())
        && folder.is_some_and(|f| {
            f.strip_prefix(dir)
                .is_some_and(|q| q.is_empty() || q.starts_with('-'))
        })
}

pub fn res_file(path: &Path, ty: &str, name: &str) -> bool {
    FILE_TYPES.contains(&ty)
        && in_res(path, ty)
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split('.').next())
            == Some(name)
}

pub fn android_source(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "xml" || e == "toml")
}

pub fn qualified_res(path: &Path) -> bool {
    path.parent()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n.to_string_lossy().contains('-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_catalog_accessor_is_its_section_and_key() {
        let line = "    implementation(libs.google.oss.licenses.get())";
        let (segments, at) = dotted_at(line, line.find("oss").unwrap()).unwrap();
        assert_eq!(
            (segments.clone(), at),
            (vec!["libs", "google", "oss", "licenses"], 2)
        );
        assert_eq!(
            catalog_key(&segments),
            Some(("libraries", "google-oss-licenses".to_owned()))
        );
        assert_eq!(
            catalog_key(&["libs", "plugins", "a", "b"]),
            Some(("plugins", "a-b".to_owned()))
        );
        assert_eq!(catalog_key(&["libs", "plugins"]), None);
        assert_eq!(catalog_key(&["lib", "a"]), None);
        let text = "[versions]\ngoogle-oss-licenses = \"1\"\n[libraries]\n# google-oss-licenses = 1\ngoogle_oss.licenses = { module = \"a:b\" }\n";
        assert_eq!(
            catalog_line(text, "libraries", "google-oss-licenses"),
            Some(5)
        );
        assert_eq!(
            catalog_line(text, "versions", "google-oss-licenses"),
            Some(2)
        );
        assert_eq!(catalog_line(text, "plugins", "google-oss-licenses"), None);
    }

    #[test]
    fn a_resource_is_read_behind_r_or_its_alias_and_never_androids() {
        let at = |text: &str, line: &str, word: &str| {
            let (segments, at) = dotted_at(line, line.find(word).unwrap())?;
            resource_at(text, &segments, at).map(|(r, t, n)| format!("{r} {t} {n}"))
        };
        let line = "stringResource(id = R.string.removed)";
        assert_eq!(at("", line, "removed").as_deref(), Some("R string removed"));
        assert_eq!(at("", line, "string"), None);
        assert_eq!(at("import android.R\n", line, "removed"), None);
        assert_eq!(at("", "android.R.string.ok", "ok"), None);
        assert_eq!(at("", "R.style.Theme", "Theme"), None);
        let alias = "import app.core.ui.R as CoreUiR\n";
        assert_eq!(
            at(alias, "CoreUiR.dimen.gap", "gap").as_deref(),
            Some("CoreUiR dimen gap")
        );
        assert_eq!(at("", "CoreUiR.dimen.gap", "gap"), None);
        assert!(in_res(Path::new("a/res/values-de/strings.xml"), "values"));
        assert!(!in_res(Path::new("a/res/valuesx/strings.xml"), "values"));
        assert!(res_file(
            Path::new("res/drawable-hdpi/ic.9.png"),
            "drawable",
            "ic"
        ));
        assert!(!res_file(Path::new("res/drawable/ic.png"), "string", "ic"));
    }
}
