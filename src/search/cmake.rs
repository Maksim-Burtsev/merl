use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

pub fn cmake_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let end = r"(?:[\s)]|$)";
    vec![
        format!(r"^\s*(?i:(?:function|macro)\s*\(\s*{w}){end}"),
        format!(r"^\s*(?i:set|option)\s*\(\s*{w}{end}"),
        format!(r"^\s*(?i:add_library|add_executable|add_custom_target)\s*\(\s*{w}{end}"),
    ]
}

pub(super) const CMAKE_FUNCTION_SYMBOL: &str =
    r"^\s*(?i:function|macro)\s*\(\s*(?P<name>[A-Za-z_][\w]*)";
pub(super) const CMAKE_TARGET_SYMBOL: &str = concat!(
    r"^\s*(?i:add_library|add_executable|add_custom_target)\s*\(\s*",
    r"(?P<name>[\w.+-]+(?:::[\w.+-]+)*)",
    r"(?:\s*(?:\)|$)|\s+(?:[^A\s]|A[^L]|AL[^I]|ALI[^A]|ALIA[^S]|ALIAS\S))"
);

pub fn cmake_name(line: &str, r: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let b = line.as_bytes();
    let part = |c: u8| c.is_ascii_alphanumeric() || b"_-.".contains(&c);
    let (mut start, mut end) = (r.start, r.end);
    while start >= 3 && &b[start - 2..start] == b"::" && part(b[start - 3]) {
        start -= 2;
        while start > 0 && part(b[start - 1]) {
            start -= 1;
        }
    }
    while b.get(end..end + 2) == Some(b"::") && b.get(end + 2).is_some_and(|&c| part(c)) {
        end += 2;
        while end < b.len() && part(b[end]) {
            end += 1;
        }
    }
    start..end
}

pub fn cmake_import(line: &str, col: usize) -> Option<(String, String)> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^\s*(?i)(include|add_subdirectory|find_package)\s*\(\s*"?([^\s"()$]+)"#)
            .unwrap()
    });
    let c = IMPORT.captures(line)?;
    let arg = c.get(2)?;
    (arg.start() <= col && col <= arg.end())
        .then(|| (c[1].to_ascii_lowercase(), arg.as_str().to_owned()))
}

pub fn cmake_files(command: &str, arg: &str, dir: &Path, files: &[PathBuf]) -> Vec<PathBuf> {
    let named = |names: &[String]| -> Vec<PathBuf> {
        let named = |f: &&PathBuf| {
            f.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| names.iter().any(|m| m == n))
        };
        files.iter().filter(named).cloned().collect()
    };
    let at = |path: PathBuf| -> Vec<PathBuf> {
        lexical(&path)
            .filter(|f| files.contains(f))
            .into_iter()
            .collect()
    };
    match command {
        "add_subdirectory" => at(dir.join(arg).join("CMakeLists.txt")),
        "include" if arg.contains('/') || arg.ends_with(".cmake") => at(dir.join(arg)),
        "include" => named(&[format!("{arg}.cmake")]),
        _ => named(&[
            format!("Find{arg}.cmake"),
            format!("{arg}Config.cmake"),
            format!("{}-config.cmake", arg.to_ascii_lowercase()),
        ]),
    }
}

pub(super) fn cmake_roots(cmake: Option<PathBuf>) -> Vec<PathBuf> {
    let share = cmake
        .and_then(|c| std::fs::canonicalize(c).ok())
        .and_then(|c| Some(c.parent()?.parent()?.join("share")));
    let mut dirs: Vec<PathBuf> = share.iter().map(|s| s.join("cmake/Modules")).collect();
    let versioned = share
        .iter()
        .flat_map(|s| std::fs::read_dir(s).into_iter().flatten());
    dirs.extend(versioned.flatten().filter_map(|e| {
        let name = e.file_name();
        name.to_str()?
            .starts_with("cmake-")
            .then(|| e.path().join("Modules"))
    }));
    dirs.extend(["/opt/homebrew", "/usr/local", "/usr"].map(|p| Path::new(p).join("lib/cmake")));
    let triplets = std::fs::read_dir("/usr/lib")
        .into_iter()
        .flatten()
        .flatten();
    dirs.extend(triplets.filter_map(|e| {
        let name = e.file_name();
        name.to_str()?
            .contains("-linux-")
            .then(|| e.path().join("cmake"))
    }));
    dirs
}
