use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

/// The pieces of Dart's grammar both `d`'s patterns and `D`'s rows read, as macros so the
/// rows can be `concat!`ed from them: one spelling of what a Dart type is.
macro_rules! dart_generics {
    () => {
        r"(?:<[^<>]*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>[^<>]*)*>)?"
    };
}
/// The annotations in front of a declaration written on its line: `@override String f()`.
macro_rules! dart_annotations {
    () => {
        r"(?:@[\w$.]+(?:\([^)]*\))?\s+)*"
    };
}
macro_rules! dart_mods {
    () => {
        r"(?:(?:static|external|abstract|override)\s+)*"
    };
}
/// A type a declaration is told from a statement by: a primitive, or a name with a capital,
/// behind a library prefix or not, with its generics nested three deep and its `?`. `return`,
/// `await`, `throw`, `yield` and `case` are none.
macro_rules! dart_type {
    () => {
        concat!(
            r"(?:[\w$]+\.)?(?:void|int|double|num|bool|dynamic|[\w$]*[A-Z][\w$]*)",
            dart_generics!(),
            r"\??"
        )
    };
}
/// Any type a keyword already tells, lowercase too: `final int x`, `late final db.Database d`.
macro_rules! dart_any_type {
    () => {
        concat!(r"[\w$.]+", dart_generics!(), r"\??")
    };
}
const ANNOTATIONS: &str = dart_annotations!();
const TYPE: &str = dart_type!();
const ANY_TYPE: &str = dart_any_type!();
const NAME_END_PAST_DOLLAR: &str = r"(?:[^\w$]|$)";

const DART_PATTERN_COUNT: usize = 9;
const DART_CONSTRUCTOR_PATTERN: usize = 4;

/// In this order: a type, a top-level function, a method, a getter or a setter, a constructor of
/// the class `word`, a named constructor `X.word`, a variable or a field, an `enum` value on the
/// `enum` line and one on a line of its own.
pub fn dart_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let mods = dart_mods!();
    let generics = dart_generics!();
    // The parameters a constructor declaration opens with, where a call's arguments are
    // expressions: `this.`, `super.`, a `{` or `[` of optional ones, a type and a name; or the
    // `:` of an initializer list after them.
    let params = format!(
        r"\((?:\s*(?:this\.|super\.|\{{|\[|(?:required\s+)?{TYPE}\s+[A-Za-z_$][\w$]*\s*[,)=])|[^;]*\)\s*:(?:[^:]|$))"
    );
    vec![
        // `class` behind its modifiers, `mixin`, `enum`, a named `extension … on`, an
        // `extension type`, a `typedef` new (`= …`) and old (`void Callback(int code)`).
        format!(
            r"^\s*{ANNOTATIONS}(?:(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|mixin|enum)\s+{w}{NAME_END_PAST_DOLLAR}|extension\s+{w}\s*{generics}\s+on\s|extension\s+type\s+(?:const\s+)?{w}\s*[<(.]|typedef\s+(?:[^=;(]*\s)?{w}\s*(?:<[^>]*>)?\s*[=(])"
        ),
        // In column zero Dart has declarations and directives only, so a name before its `(`
        // there is a function: `main() {`, `T first<T>(…)`.
        format!(r"^(?:external\s+)?(?:{ANY_TYPE}\s+)?{w}\s*(?:<[^()]*>)?\s*\("),
        // Indented, the type before the name tells a method from a call, as in Java.
        format!(r"^\s+{ANNOTATIONS}{mods}{TYPE}\s+(?:operator\s*)?{w}\s*(?:<[^()]*>)?\s*\("),
        format!(
            r"^\s*{ANNOTATIONS}{mods}(?:{ANY_TYPE}\s+)?(?:get\s+{w}{NAME_END_PAST_DOLLAR}|set\s+{w}\s*\()"
        ),
        format!(r"^\s+{ANNOTATIONS}(?:(?:(?:const|factory|external)\s+)+{w}\s*\(|{w}\s*{params})"),
        format!(
            r"^\s+{ANNOTATIONS}(?:(?:(?:const|factory|external)\s+)+[A-Za-z_$][\w$]*\.{w}\s*\(|[A-Za-z_$][\w$]*\.{w}\s*{params})"
        ),
        // Behind `final`, `const`, `var` or `late` with or without its type, or with a type alone;
        // `==` compares, `=>` is no value.
        format!(
            r"^\s*{ANNOTATIONS}(?:(?:(?:static|external|covariant)\s+)*(?:(?:final|const|var|late)\s+)+(?:{ANY_TYPE}\s+)?|(?:(?:static|external|covariant)\s+)*{TYPE}\s+){w}\s*(?:=[^=>]|;|,|$)"
        ),
        format!(r"^\s*enum\s+[\w$]+[^{{]*\{{(?:[^}}]*[,{{\s])?{w}\s*(?:[,}}(]|$)"),
        format!(r"^\s+{w}\s*(?:[,;(]|$)"),
    ]
}

/// A constructor directly inside a class, an enum or an extension type; an `enum` value on a line
/// of its own directly inside an `enum`; a variable nowhere in a parameter list wrapped over lines
/// (`  int retries = 3,` under `void f({`), where it is a parameter.
pub fn dart_declares<S: AsRef<str>>(lines: &[S], line1: usize, line_text: &str) -> bool {
    static BODY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*{ANNOTATIONS}(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|enum|mixin|extension\s+type)\s"
        ))
        .unwrap()
    });
    static ANNOTATION_LINE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:@[\w$.]+(?:\([^)]*\))?\s*)+$").unwrap());
    static ENUM: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(&format!(r"^\s*{ANNOTATIONS}enum\s")).unwrap());
    let nearest_outer_line = || {
        let depth = indent(line_text);
        lines[..line1.saturating_sub(1).min(lines.len())]
            .iter()
            .map(AsRef::as_ref)
            .rev()
            .find(|l| {
                let t = l.trim_start();
                !t.is_empty()
                    && !t.starts_with("//")
                    && !ANNOTATION_LINE.is_match(t)
                    && indent(l) < depth
            })
            .map(str::to_owned)
    };
    let shape = |s: Shape| SHAPES[s as usize].is_match(line_text);
    if [Shape::Type, Shape::Function, Shape::Method, Shape::Accessor]
        .into_iter()
        .any(shape)
    {
        return true;
    }
    if shape(Shape::Constructor) {
        return nearest_outer_line().is_some_and(|o| BODY.is_match(&o));
    }
    if shape(Shape::Variable) {
        let in_wrapped_param_list = nearest_outer_line().as_deref().is_some_and(|o| {
            let t = o.trim_end();
            t.ends_with(['(', '[', ',']) || t.ends_with("({") || t.ends_with("[{")
        });
        return !in_wrapped_param_list;
    }
    if shape(Shape::EnumValueOnEnumLine) {
        return true;
    }
    nearest_outer_line().is_some_and(|o| ENUM.is_match(&o))
}

#[derive(Clone, Copy)]
enum Shape {
    Type,
    Function,
    Method,
    Accessor,
    Constructor,
    Variable,
    EnumValueOnEnumLine,
}

static SHAPES: std::sync::LazyLock<Vec<Regex>> = std::sync::LazyLock::new(|| {
    let p = dart_patterns("NAME");
    let any = |s: &str| s.replace("NAME", r"[A-Za-z_$][\w$]*");
    [&p[0], &p[1], &p[2], &p[3]]
        .into_iter()
        .chain([&format!("{}|{}", p[4], p[5])])
        .chain([&p[6], &p[7]])
        .map(|s| Regex::new(&any(s)).unwrap())
        .collect()
});

/// The class's own constructor only where the class is built, `User(…)`, and the type itself
/// everywhere.
pub fn dart_narrow(patterns: &mut Vec<String>, line: &str, word: std::ops::Range<usize>) {
    static CALLED: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*(?:<[^()]*>)?\s*\(").unwrap());
    if patterns.len() == DART_PATTERN_COUNT && !CALLED.is_match(&line[word.end..]) {
        patterns.remove(DART_CONSTRUCTOR_PATTERN);
    }
}

/// What a Dart file's imports bind: the prefix of `import '…' as p;` (`true`) and each name of
/// `import '…' show A, B;` (`false`), with the URI as written. A plain `import` binds no name, as
/// Swift's makes a whole module visible.
pub fn dart_imports(text: &str) -> Vec<(String, String, bool)> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*import\s+(?:'([^']*)'|"([^"]*)")([^;]*);"#).unwrap()
    });
    static PREFIX: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bas\s+([A-Za-z_$][\w$]*)").unwrap());
    static SHOW: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\bshow\s+([\w$,\s]+)").unwrap());
    let mut out = Vec::new();
    for c in IMPORT.captures_iter(text) {
        let uri = c.get(1).or_else(|| c.get(2)).map_or("", |m| m.as_str());
        let rest = &c[3];
        if let Some(p) = PREFIX.captures(rest) {
            out.push((p[1].to_owned(), uri.to_owned(), true));
        } else if let Some(s) = SHOW.captures(rest) {
            // `show A, B hide C` ends the list at the next keyword.
            for name in s[1].split(',').filter_map(|n| n.split_whitespace().next()) {
                out.push((name.to_owned(), uri.to_owned(), false));
            }
        }
    }
    out
}

/// The file a Dart import's `uri` names from `here`, project-relative when it is the project's,
/// absolute outside it; `None` when nothing on disk answers it. `root` is the project's.
///
/// - A relative URI is next to `here`.
/// - `package:<name>/<path>` is `lib/<path>` of the package whose `pubspec.yaml` is the nearest
///   above `here` when it is called `<name>`, else of `<name>` in the
///   `.dart_tool/package_config.json` beside that `pubspec.yaml`, which `pub get` writes.
/// - `dart:<lib>` is `<lib>/<lib>.dart` in the SDK's `lib/`, `dart:ui` the `sky_engine` package's
///   `lib/ui/ui.dart`.
pub fn dart_uri_file(root: &Path, here: &Path, uri: &str, sdk: Option<&Path>) -> Option<PathBuf> {
    let found = |p: PathBuf| -> Option<PathBuf> {
        let abs = root.join(&p);
        abs.is_file().then(|| {
            abs.strip_prefix(root)
                .map_or(abs.clone(), Path::to_path_buf)
        })
    };
    let package_dir = here
        .ancestors()
        .skip(1)
        .find(|d| root.join(d).join("pubspec.yaml").is_file());
    let packages = || {
        package_dir
            .map(|d| dart_packages(&root.join(d).join(".dart_tool/package_config.json")))
            .unwrap_or_default()
    };
    if let Some(lib) = uri.strip_prefix("dart:") {
        if lib == "ui" {
            let sky = packages().into_iter().find(|p| p.name == "sky_engine")?;
            return found(sky.package_uri_dir.join("ui/ui.dart"));
        }
        return found(sdk?.join("lib").join(lib).join(format!("{lib}.dart")));
    }
    let Some(rest) = uri.strip_prefix("package:") else {
        return found(lexical(&here.parent()?.join(uri))?);
    };
    let (name, path) = rest.split_once('/')?;
    if let Some(dir) = package_dir {
        let pubspec = std::fs::read_to_string(root.join(dir).join("pubspec.yaml")).ok()?;
        let own = pubspec
            .lines()
            .find_map(|l| l.strip_prefix("name:"))
            .map(|n| n.trim().trim_matches(['"', '\'']));
        if own == Some(name) {
            return found(dir.join("lib").join(path));
        }
    }
    let package = packages().into_iter().find(|p| p.name == name)?;
    found(package.package_uri_dir.join(path))
}

pub(super) struct DartPackage {
    name: String,
    package_uri_dir: PathBuf,
}

/// `package_uri_dir` is the `rootUri` (a `file://` URI, or a path relative to the file's
/// directory) joined with the `packageUri` (`lib/`).
pub(super) fn dart_packages(config: &Path) -> Vec<DartPackage> {
    static ENTRY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\{[^{}]*\}").unwrap());
    static KEY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#""(name|rootUri|packageUri)"\s*:\s*"([^"]*)""#).unwrap()
    });
    let Ok(text) = std::fs::read_to_string(config) else {
        return Vec::new();
    };
    let base = config.parent().unwrap_or(Path::new(""));
    ENTRY
        .find_iter(&text)
        .filter_map(|e| {
            let get = |k: &str| {
                KEY.captures_iter(e.as_str())
                    .find(|c| &c[1] == k)
                    .map(|c| c[2].to_owned())
            };
            let name = get("name")?;
            let root = get("rootUri")?;
            let root = match root.strip_prefix("file://") {
                Some(p) => PathBuf::from(percent_decode(p)),
                None => lexical(&base.join(percent_decode(&root)))?,
            };
            Some(DartPackage {
                name,
                package_uri_dir: root.join(get("packageUri").unwrap_or_default()),
            })
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = (b[i] == b'%')
            .then(|| s.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match hex {
            Some(v) => {
                out.push(v);
                i += 3;
            }
            None => {
                out.push(b[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn dart_sdk() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let dart = std::env::split_paths(&path)
        .map(|d| d.join("dart"))
        .find(|p| p.is_file())?;
    dart_sdk_of(&dart)
}

/// The SDK a `dart` executable belongs to, symlinks followed: the directory above its `bin/`,
/// or in a Flutter install, where `bin/dart` sits beside `bin/cache/dart-sdk`, that one.
pub(super) fn dart_sdk_of(dart: &Path) -> Option<PathBuf> {
    let bin = std::fs::canonicalize(dart).ok()?.parent()?.to_path_buf();
    let flutter = bin.join("cache/dart-sdk");
    match flutter.is_dir() {
        true => Some(flutter),
        false => bin.parent().map(Path::to_path_buf),
    }
}

/// Where Dart's sources outside the project `root` are: every package the
/// `.dart_tool/package_config.json` of the project (or of a package of it one or two
/// directories down, as a Flutter app's `app/` is) lists outside `root`, and the SDK's `lib/`.
/// No `pub get` yet, no packages.
pub(super) fn dart_roots(root: &Path, sdk: Option<PathBuf>) -> Vec<PathBuf> {
    let sorted_subdirs = |d: &Path| -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(d)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && !p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .collect();
        dirs.sort();
        dirs
    };
    let mut dirs = vec![root.to_path_buf()];
    for d in sorted_subdirs(root) {
        dirs.extend(sorted_subdirs(&d));
        dirs.push(d);
    }
    let mut out: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| dart_packages(&d.join(".dart_tool/package_config.json")))
        .map(|p| p.package_uri_dir)
        .filter(|dir| !dir.starts_with(root))
        .collect();
    out.extend(sdk.map(|s| s.join("lib")));
    out
}

/// What `D` lists of a Dart file, first row: a type — a `class` behind its modifiers, a `mixin`,
/// an `enum`, a named `extension`, an `extension type`, a `typedef`. An unnamed `extension on X`
/// names nothing.
pub(super) const DART_TYPE_SYMBOL: &str = concat!(
    r"^\s*",
    dart_annotations!(),
    r"(?:(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|mixin|enum)\s+",
    r"|extension\s+(?:type\s+(?:const\s+)?)?|typedef\s+(?:[^=;(]*\s)?)",
    r"(?P<name>[A-Za-z_$][\w$]*)\s*",
    dart_generics!(),
    r"\s*(?:[({=.:;]|on\s|extends\b|with\b|implements\b|$)"
);
/// Second row: a function in column zero and an indented method, by the type before its name.
pub(super) const DART_FUNCTION_SYMBOL: &str = concat!(
    r"^(?:(?:external\s+)?(?:",
    dart_any_type!(),
    r"\s+)?|\s+",
    dart_annotations!(),
    dart_mods!(),
    dart_type!(),
    r"\s+(?:operator\s*)?)(?P<name>[A-Za-z_$][\w$]*)\s*(?:<[^()]*>)?\s*\("
);
/// Third row: a getter and a setter. Their own row, since a getter ends where a method goes on
/// with its `(`, and one pattern for both would read a field. A getter's body may be `async`.
pub(super) const DART_ACCESSOR_SYMBOL: &str = concat!(
    r"^\s*",
    dart_annotations!(),
    dart_mods!(),
    r"(?:",
    dart_any_type!(),
    r"\s+)?(?:get|set)\s+(?P<name>[A-Za-z_$][\w$]*)\s*(?:=>|\{|\(|;|async\b|sync\*|$)"
);
