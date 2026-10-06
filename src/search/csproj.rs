use std::path::{Component, Path, PathBuf};

use regex::Regex;

/// The projects of a repository as one C# file sees them: every directory holding a `.csproj`,
/// and those the file compiles against.
#[derive(Debug)]
pub struct CsProjects {
    all: Vec<PathBuf>,
    seen: Vec<PathBuf>,
}

impl CsProjects {
    /// Whether a declaration in `path` is in sight: the nearest project above it is seen, or no
    /// project holds it.
    pub fn sees(&self, path: &Path) -> bool {
        nearest(&self.all, path).is_none_or(|d| self.seen.contains(d))
    }
}

/// The projects `here` sees among `files`, the project's paths, each `.csproj` and
/// `Directory.Build.props` read with `read`. The project of a file is the nearest directory
/// above it holding a `.csproj`; it sees its own project and every `<ProjectReference>` of its
/// `.csproj`, and of each `Directory.Build.props` above it, followed transitively. `None` when
/// that cannot be told and every file stays in sight: no `.csproj` above `here` (a script), two
/// in one directory, a `<Compile Include>` reaching out of its directory, a shared project's
/// `.projitems`, a reference that cannot be read.
pub fn cs_projects(
    files: &[PathBuf],
    here: &Path,
    read: impl Fn(&Path) -> Option<String>,
) -> Option<CsProjects> {
    static REFERENCE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"<ProjectReference\b[^>]*?\bInclude\s*=\s*"([^"]*)""#).unwrap()
    });
    // A source file pulled in from outside the project's directory, or a shared project's.
    static OUTSIDE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"<Compile\b[^>]*\.\.|<Import\b[^>]*\.projitems"#).unwrap()
    });
    let manifests: Vec<&PathBuf> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "csproj"))
        .collect();
    let mut all: Vec<PathBuf> = manifests
        .iter()
        .map(|f| f.parent().unwrap_or(Path::new("")).to_path_buf())
        .collect();
    all.sort();
    all.dedup();
    if here.extension().is_some_and(|e| e == "csx") {
        return None;
    }
    let own = cs_own_project(files, here)?;
    let mut seen = vec![own];
    let mut i = 0;
    while let Some(dir) = seen.get(i).cloned() {
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
            // Every reference is read, or none narrows: one the pattern misses would hide its
            // project.
            let written = other.matches("<ProjectReference").count();
            if OUTSIDE.is_match(other) || REFERENCE.captures_iter(other).count() != written {
                return None;
            }
            for c in REFERENCE.captures_iter(other) {
                let target = lexical(&dir, &c[1].replace('\\', "/"))?;
                if !manifests.contains(&&target) {
                    return None;
                }
                let at = target.parent().unwrap_or(Path::new("")).to_path_buf();
                if !seen.contains(&at) {
                    seen.push(at);
                }
            }
        }
    }
    Some(CsProjects { all, seen })
}

/// The directory of the project a C# file is in: the nearest above it, among `files`, holding a
/// `.csproj`.
pub fn cs_own_project(files: &[PathBuf], here: &Path) -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "csproj"))
        .map(|f| f.parent().unwrap_or(Path::new("")).to_path_buf())
        .collect();
    nearest(&dirs, here).cloned()
}

/// The deepest of `dirs` that holds `path`.
fn nearest<'a>(dirs: &'a [PathBuf], path: &Path) -> Option<&'a PathBuf> {
    dirs.iter()
        .filter(|d| path.parent().is_some_and(|p| p.starts_with(d)))
        .max_by_key(|d| d.components().count())
}

/// `rel` read from `dir`, `..` and `.` resolved; `None` past the root, or for a path MSBuild
/// has to evaluate (`$(…)`, a wildcard) or an absolute one.
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
