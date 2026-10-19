use std::path::{Component, Path, PathBuf};

use regex::Regex;

#[derive(Debug)]
pub struct CsProjects {
    csproj_dirs: Vec<PathBuf>,
    compiled_against: Vec<PathBuf>,
    included: Vec<PathBuf>,
}

impl CsProjects {
    pub fn sees(&self, path: &Path) -> bool {
        self.included.iter().any(|f| f == path)
            || deepest_holding(&self.csproj_dirs, path)
                .is_none_or(|d| self.compiled_against.contains(d))
    }
}

pub fn cs_projects(
    files: &[PathBuf],
    here: &Path,
    read: impl Fn(&Path) -> Option<String>,
) -> Option<CsProjects> {
    static REFERENCE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"<ProjectReference\b[^>]*?\bInclude\s*=\s*"([^"]*)""#)
            .expect("the regex is a literal")
    });
    static SHARED_PROJECT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"<Import\b[^>]*\.projitems").expect("the regex is a literal")
    });
    static COMPILE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"<Compile\b[^>]*").expect("the regex is a literal")
    });
    static ONE_FILE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^<Compile\s+Include\s*=\s*"([^";]*)"(?:\s+Link\s*=\s*"[^"]*")?\s*/?$"#)
            .expect("the regex is a literal")
    });
    let manifests: Vec<&PathBuf> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "csproj"))
        .collect();
    let mut csproj_dirs: Vec<PathBuf> = manifests
        .iter()
        .map(|f| f.parent().unwrap_or_else(|| Path::new("")).to_path_buf())
        .collect();
    csproj_dirs.sort();
    csproj_dirs.dedup();
    if here.extension().is_some_and(|e| e == "csx") {
        return None;
    }
    let own = cs_own_project(files, here)?;
    let mut compiled_against = vec![own];
    let mut included = Vec::new();
    let mut i = 0;
    while let Some(dir) = compiled_against.get(i).cloned() {
        i += 1;
        let [manifest] = manifests
            .iter()
            .filter(|f| f.parent() == Some(dir.as_path()))
            .collect::<Vec<_>>()[..]
        else {
            return None;
        };
        let text = read(manifest)?;
        let props = dir
            .ancestors()
            .filter_map(|d| read(&d.join("Directory.Build.props")))
            .collect::<Vec<_>>();
        for other in std::iter::once(&text).chain(&props) {
            let written = other.matches("<ProjectReference").count();
            if SHARED_PROJECT.is_match(other) || REFERENCE.captures_iter(other).count() != written {
                return None;
            }
            for tag in COMPILE.find_iter(other).map(|m| m.as_str()) {
                if !tag.contains("..") {
                    continue;
                }
                let file = ONE_FILE
                    .captures(tag)
                    .and_then(|c| lexical(&dir, &c[1].replace('\\', "/")))
                    .filter(|f| files.contains(f))?;
                included.push(file);
            }
            for c in REFERENCE.captures_iter(other) {
                let target = lexical(&dir, &c[1].replace('\\', "/"))?;
                if !manifests.contains(&&target) {
                    return None;
                }
                let at = target
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .to_path_buf();
                if !compiled_against.contains(&at) {
                    compiled_against.push(at);
                }
            }
        }
    }
    Some(CsProjects {
        csproj_dirs,
        compiled_against,
        included,
    })
}

pub fn cs_own_project(files: &[PathBuf], here: &Path) -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "csproj"))
        .map(|f| f.parent().unwrap_or_else(|| Path::new("")).to_path_buf())
        .collect();
    deepest_holding(&dirs, here).cloned()
}

fn deepest_holding<'a>(dirs: &'a [PathBuf], path: &Path) -> Option<&'a PathBuf> {
    dirs.iter()
        .filter(|d| path.parent().is_some_and(|p| p.starts_with(d)))
        .max_by_key(|d| d.components().count())
}

fn lexical(dir: &Path, rel: &str) -> Option<PathBuf> {
    if rel.contains(['$', '*', '%']) || rel.starts_with('/') || rel.contains(':') {
        return None;
    }
    let mut out = dir.to_path_buf();
    for c in Path::new(rel).components() {
        match c {
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::Normal(n) => out.push(n),
            _ => {}
        }
    }
    Some(out)
}
