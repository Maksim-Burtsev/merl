use std::path::{Path, PathBuf};

use regex::Regex;

use super::{
    Binding, Kind, Value, close_of, indent, lexical, literal_lines, split_top, uncommented,
};

macro_rules! dart_generics {
    () => {
        r"(?:<[^<>]*(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>[^<>]*)*>)?"
    };
}
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
macro_rules! dart_type {
    () => {
        concat!(
            r"(?:[\w$]+\.)?(?:void|int|double|num|bool|dynamic|[\w$]*[A-Z][\w$]*)",
            dart_generics!(),
            r"\??"
        )
    };
}
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

pub fn dart_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let mods = dart_mods!();
    let generics = dart_generics!();
    let params = format!(
        r"\((?:\s*(?:this\.|super\.|\{{|\[|(?:required\s+)?{TYPE}\s+[A-Za-z_$][\w$]*\s*[,)=])|[^;]*\)\s*:(?:[^:]|$))"
    );
    vec![
        format!(
            r"^\s*{ANNOTATIONS}(?:(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|mixin|enum)\s+{w}{NAME_END_PAST_DOLLAR}|extension\s+{w}\s*{generics}\s+on\s|extension\s+type\s+(?:const\s+)?{w}\s*[<(.]|typedef\s+(?:[^=;(]*\s)?{w}\s*(?:<[^>]*>)?\s*[=(])"
        ),
        format!(r"^(?:external\s+)?(?:{ANY_TYPE}\s+)?{w}\s*(?:<[^()]*>)?\s*\("),
        format!(r"^\s+{ANNOTATIONS}{mods}{TYPE}\s+(?:operator\s*)?{w}\s*(?:<[^()]*>)?\s*\("),
        format!(
            r"^\s*{ANNOTATIONS}{mods}(?:{ANY_TYPE}\s+)?(?:get\s+{w}{NAME_END_PAST_DOLLAR}|set\s+{w}\s*\()"
        ),
        format!(r"^\s+{ANNOTATIONS}(?:(?:(?:const|factory|external)\s+)+{w}\s*\(|{w}\s*{params})"),
        format!(
            r"^\s+{ANNOTATIONS}(?:(?:(?:const|factory|external)\s+)+[A-Za-z_$][\w$]*\.{w}\s*\(|[A-Za-z_$][\w$]*\.{w}\s*{params})"
        ),
        format!(
            r"^\s*{ANNOTATIONS}(?:(?:(?:static|external|covariant)\s+)*(?:(?:final|const|var|late)\s+)+(?:{ANY_TYPE}\s+)?|(?:(?:static|external|covariant)\s+)*{TYPE}\s+){w}\s*(?:=[^=>]|;|,|$)"
        ),
        format!(r"^\s*enum\s+[\w$]+[^{{]*\{{(?:[^}}]*[,{{\s])?{w}\s*(?:[,}}(]|$)"),
        format!(r"^\s+{w}\s*(?:[,;(]|$)"),
    ]
}

pub fn dart_bare_constructor(class: &str, word: &str) -> String {
    let c = regex::escape(class);
    let w = regex::escape(word);
    format!(r"^\s+{ANNOTATIONS}{c}\.{w}\s*\(\s*\)\s*;")
}

pub fn dart_declares<S: AsRef<str>>(lines: &[S], line1: usize, line_text: &str) -> bool {
    static BODY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*{ANNOTATIONS}(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|enum|mixin|extension\s+type)\s"
        ))
        .expect("the regex is a literal")
    });
    static ANNOTATION_LINE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:@[\w$.]+(?:\([^)]*\))?\s*)+$").expect("the regex is a literal")
    });
    static ENUM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(r"^\s*{ANNOTATIONS}enum\s")).expect("the regex is a literal")
    });
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
        .chain([&format!(
            "{}|{}|{}",
            p[4],
            p[5],
            dart_bare_constructor("NAME", "NAME")
        )])
        .chain([&p[6], &p[7]])
        .map(|s| Regex::new(&any(s)).expect("the regex is a literal"))
        .collect()
});

pub fn dart_label(text: &str, line1: usize, col: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let (mut depth, mut comma) = (0usize, false);
    for i in (at.saturating_sub(50)..=at).rev() {
        let code = uncommented(Kind::Dart, lines[i]);
        let end = if i == at {
            col.min(code.len())
        } else {
            code.len()
        };
        for c in code.get(..end).unwrap_or_default().chars().rev() {
            match c {
                ')' | '}' | ']' => depth += 1,
                '(' | '{' | '[' if depth > 0 => depth -= 1,
                '(' => return true,
                '{' | '[' => return false,
                '?' if depth == 0 && !comma => return false,
                ',' if depth == 0 => comma = true,
                _ => {}
            }
        }
    }
    false
}

pub fn dart_called(rest: &str) -> bool {
    static CALLED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:<[^()]*>)?\s*\(").expect("the regex is a literal")
    });
    CALLED.is_match(rest)
}

pub fn dart_narrow(patterns: &mut Vec<String>, line: &str, word: std::ops::Range<usize>) {
    if patterns.len() == DART_PATTERN_COUNT
        && line.get(word.end..).is_some_and(|rest| !dart_called(rest))
    {
        patterns.remove(DART_CONSTRUCTOR_PATTERN);
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DartDirective {
    pub keyword: String,
    pub uri: String,
    pub prefix: Option<String>,
    pub show: Option<Vec<String>>,
    pub hide: Vec<String>,
    pub conditional: bool,
    pub line1: usize,
}

pub fn dart_directives(text: &str) -> Option<Vec<DartDirective>> {
    static DIRECTIVE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r#"(?m)^\s*(import|export|part\s+of|part)\s+(?:'([^']*)'|"([^"]*)"|([\w$.]+))([^;]*);"#,
        )
        .expect("the regex is a literal")
    });
    let mut out = Vec::new();
    for c in DIRECTIVE.captures_iter(text) {
        let keyword = c[1].split_whitespace().collect::<Vec<_>>().join(" ");
        let uri = c.get(2).or_else(|| c.get(3))?.as_str().to_owned();
        let mut d = DartDirective {
            keyword,
            uri,
            line1: text.get(..c.get(1)?.start())?.matches('\n').count() + 1,
            ..DartDirective::default()
        };
        let mut mode = "";
        for t in c[5].split(|ch: char| ch.is_whitespace() || ch == ',') {
            match t {
                "" => {}
                "as" | "show" | "hide" | "deferred" => mode = t,
                _ if t.starts_with("if") => d.conditional = true,
                _ => match mode {
                    "as" => d.prefix = Some(t.to_owned()),
                    "show" => d.show.get_or_insert_with(Vec::new).push(t.to_owned()),
                    "hide" => d.hide.push(t.to_owned()),
                    _ => {}
                },
            }
        }
        out.push(d);
    }
    Some(out)
}

pub fn dart_imports(text: &str) -> Vec<DartImport> {
    let mut out = Vec::new();
    for d in dart_directives(text).unwrap_or_default() {
        if d.keyword != "import" {
            continue;
        }
        let names = match d.prefix {
            Some(p) => vec![(p, true)],
            None => d.show.into_iter().flatten().map(|n| (n, false)).collect(),
        };
        out.extend(names.into_iter().map(|(name, is_prefix)| DartImport {
            name,
            uri_as_written: d.uri.clone(),
            is_prefix,
        }));
    }
    out
}
#[derive(Debug, PartialEq, Eq)]
pub struct DartImport {
    pub name: String,
    pub uri_as_written: String,
    pub is_prefix: bool,
}

pub fn dart_uri_file(root: &Path, here: &Path, uri: &str, sdk: Option<&Path>) -> Option<PathBuf> {
    let found = |p: PathBuf| -> Option<PathBuf> {
        let abs = root.join(&p);
        abs.is_file().then(|| {
            abs.strip_prefix(root)
                .map_or_else(|_| abs.clone(), Path::to_path_buf)
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

pub(super) fn dart_packages(config: &Path) -> Vec<DartPackage> {
    static ENTRY: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\{[^{}]*\}").expect("the regex is a literal"));
    static KEY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#""(name|rootUri|packageUri)"\s*:\s*"([^"]*)""#)
            .expect("the regex is a literal")
    });
    let Ok(text) = std::fs::read_to_string(config) else {
        return Vec::new();
    };
    let base = config.parent().unwrap_or_else(|| Path::new(""));
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
        if let Some(v) = hex {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
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

pub(super) fn dart_sdk_of(dart: &Path) -> Option<PathBuf> {
    let bin = std::fs::canonicalize(dart).ok()?.parent()?.to_path_buf();
    let flutter = bin.join("cache/dart-sdk");
    if flutter.is_dir() {
        Some(flutter)
    } else {
        bin.parent().map(Path::to_path_buf)
    }
}

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

pub(super) const DART_TYPE_SYMBOL: &str = concat!(
    r"^\s*",
    dart_annotations!(),
    r"(?:(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|mixin|enum)\s+",
    r"|extension\s+(?:type\s+(?:const\s+)?)?|typedef\s+(?:[^=;(]*\s)?)",
    r"(?P<name>[A-Za-z_$][\w$]*)\s*",
    dart_generics!(),
    r"\s*(?:[({=.:;]|on\s|extends\b|with\b|implements\b|$)"
);
pub(super) const DART_FUNCTION_SYMBOL: &str = concat!(
    r"^(?:(?:external\s+)?(?:",
    dart_any_type!(),
    r"\s+)?|\s+",
    dart_annotations!(),
    dart_mods!(),
    dart_type!(),
    r"\s+(?:operator\s*)?)(?P<name>[A-Za-z_$][\w$]*)\s*(?:<[^()]*>)?\s*\("
);
pub(super) const DART_ACCESSOR_SYMBOL: &str = concat!(
    r"^\s*",
    dart_annotations!(),
    dart_mods!(),
    r"(?:",
    dart_any_type!(),
    r"\s+)?(?:get|set)\s+(?P<name>[A-Za-z_$][\w$]*)\s*(?:=>|\{|\(|;|async\b|sync\*|$)"
);

pub(super) fn dart_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    static TYPE_BODY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"^\s*{ANNOTATIONS}(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|enum|mixin|extension)\b"
        ))
        .expect("the regex is a literal")
    });
    if matches!(name, "this" | "super") {
        return Vec::new();
    }
    let literal = literal_lines(Kind::Dart, &lines.join("\n"));
    let code = |i: usize| uncommented(Kind::Dart, lines[i]);
    let found = |i: usize| {
        vec![Binding {
            line1: i + 1,
            value: Value::Unknown,
        }]
    };
    if dart_header_binds(code(at).trim(), name) {
        return found(at);
    }
    let mut depth = indent(lines[at]);
    let mut hit = None;
    let mut i = at;
    while i > 0 {
        i -= 1;
        let t = code(i);
        let t = t.trim();
        let ind = indent(lines[i]);
        if t.is_empty() || literal[i] || ind > depth {
            continue;
        }
        if ind == depth {
            if hit.is_none() && dart_local(t, name) {
                hit = Some(i);
            }
            continue;
        }
        let end = i;
        if t.starts_with(')') {
            i = (0..i)
                .rev()
                .find(|&j| !lines[j].trim().is_empty() && indent(lines[j]) <= ind)
                .unwrap_or(i);
        }
        depth = indent(lines[i]);
        if TYPE_BODY.is_match(lines[i]) {
            return Vec::new();
        }
        if let Some(h) = hit {
            return found(h);
        }
        let head: Vec<String> = (i..=end).map(code).collect();
        if dart_header_binds(head.join(" ").trim(), name) {
            let w = Regex::new(&format!(
                r"(?:^|[^\w$]){}{NAME_END_PAST_DOLLAR}",
                regex::escape(name)
            ))
            .expect("an escaped name");
            return found(i + head.iter().position(|l| w.is_match(l)).unwrap_or(0));
        }
        if depth == 0 {
            return Vec::new();
        }
    }
    Vec::new()
}

fn dart_local(t: &str, name: &str) -> bool {
    let w = regex::escape(name);
    let declares = Regex::new(&format!(
        r"^(?:(?:(?:final|const|var|late)\s+)+(?:{ANY_TYPE}\s+)?{w}\s*(?:[=;,]|$)|{TYPE}\s+{w}\s*(?:[=;,]|$)|{TYPE}\s+{w}\s*(?:<[^()]*>)?\s*\(.*(?:=>|\{{)|(?:final|var)\s*[\w$.]*\s*\((.*)\)\s*=[^=>])"
    ))
    .expect("an escaped name");
    declares.captures(t).is_some_and(|c| {
        c.get(1)
            .is_none_or(|pattern| dart_pattern_names(pattern.as_str(), &w))
    })
}

fn dart_pattern_names(pattern: &str, w: &str) -> bool {
    Regex::new(&format!(r"(?:^|[^\w$.]){w}\s*(?:[^\w$\s:]|$)"))
        .expect("an escaped name")
        .is_match(pattern)
}

fn dart_header_binds(head: &str, name: &str) -> bool {
    static CONTROL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^[}\s]*(?:if|else|for|await\s+for|while|switch|do|try|catch|on|finally|case|default)\b")
            .expect("the regex is a literal")
    });
    static BODY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\)\s*(?:async\*?|sync\*)?\s*(?:=>|\{)").expect("the regex is a literal")
    });
    let w = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name");
    let pattern = rule(format!(
        r"[:(,]\s*(?:final|var)\s+(?:{ANY_TYPE}\s+)?{w}{NAME_END_PAST_DOLLAR}"
    ));
    let before_arrow = head.split("=>").next().unwrap_or_default();
    if CONTROL.is_match(head) {
        let clause = rule(format!(
            r"\bfor\s*\(\s*(?:(?:final|var|const)\s+)?(?:{ANY_TYPE}\s+)?{w}\s*(?:\bin\b|[=;])|\bcatch\s*\(\s*(?:{w}\s*[,)]|[\w$]+\s*,\s*{w}\s*\))"
        ));
        return clause.is_match(head) || (head.contains("case") && pattern.is_match(before_arrow));
    }
    if head.contains("=>") && pattern.is_match(before_arrow) {
        return true;
    }
    let Some(close) = BODY.find(head).map(|m| m.start()) else {
        return false;
    };
    let Some(open) = (0..close)
        .rev()
        .find(|&o| head.as_bytes()[o] == b'(' && close_of(Kind::Dart, head, o) == Some(close + 1))
    else {
        return false;
    };
    let mut inner = head.get(open + 1..close).unwrap_or_default().to_owned();
    let generic = Regex::new(r"<[^<>]*>").expect("the regex is a literal");
    while generic.is_match(&inner) {
        inner = generic.replace_all(&inner, "").into_owned();
    }
    let inner = inner.replace(['{', '}', '[', ']'], " ");
    split_top(Kind::Dart, &inner, b',').iter().any(|item| {
        let item = split_top(Kind::Dart, item, b'=')[0].trim();
        let item = match item.strip_suffix(')') {
            Some(_) => item.split('(').next().unwrap_or_default(),
            None => item,
        };
        !item.starts_with("this.")
            && !item.starts_with("super.")
            && item
                .rsplit(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                .next()
                == Some(name)
    })
}

pub fn dart_body_at(text: &str, at: usize) -> Option<(std::ops::Range<usize>, bool)> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(&format!(
            r"^{ANNOTATIONS}(?:(?:abstract|sealed|base|final|interface|mixin)\s+)*(?:class|enum|mixin|extension)\b"
        ))
        .expect("the regex is a literal")
    });
    static SUPERS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:extends|with|on)\b").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Dart, text);
    let outer = |l: &str| !l.is_empty() && !l.starts_with(char::is_whitespace);
    if outer(lines.get(at)?) {
        return Some((0..0, false));
    }
    let head = (0..at)
        .rev()
        .find(|&i| outer(lines[i]) && !literal[i] && !lines[i].starts_with("//"))?;
    if SHAPES[Shape::Function as usize].is_match(lines[head]) && !HEAD.is_match(lines[head]) {
        return Some((0..0, false));
    }
    if !HEAD.is_match(lines[head]) {
        return None;
    }
    let end = (at..lines.len())
        .find(|&i| lines[i].starts_with('}'))
        .unwrap_or(lines.len());
    let open = (head..end)
        .find(|&i| lines[i].contains('{'))
        .unwrap_or(head);
    let supers = SUPERS.is_match(&lines[head..=open].join(" "));
    Some((head + 1..end + 2, supers))
}
