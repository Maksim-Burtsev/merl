use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

/// The scope a variable or a function may be written with: `$script:Cache`, `function
/// global:Get-X`. A qualifier, not a part of the name.
const SCOPE: &str = r"(?:(?:global|script|local|private):)?";
/// The attributes and the type in front of a property or a typed assignment: `[string]`,
/// `[Parameter(Mandatory)][string]`.
const TYPES: &str = r"(?:\[[^\]]*\]\s*)*";

/// Line patterns that declare `word` in a PowerShell file: a function or a filter, a class or
/// an enum, a property, a method or a constructor of a class, an enum member, an assignment that
/// opens its line, an alias, a static property and an environment variable. A call, a named
/// argument, a hashtable key (no `$`), a property or element write and a comparison (`-eq`,
/// never `=`) match none. [`powershell_declares`] keeps the indented shapes to where they
/// declare, and [`powershell_sigil`] picks the ones the word's own spelling allows, by index.
pub fn powershell_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"(?i)^\s*(?:function|filter)\s+{SCOPE}{w}\s*(?:[({{#]|$)"),
        format!(r"(?i)^\s*{TYPES}(?:class|enum)\s+{w}\s*(?:[:{{#]|$)"),
        // An assignment, `$Count += 1` and `[string]$Name = 'x'` included, and a property, whose
        // value is optional: `hidden [int]$Count = 0`, `[string] $Name`.
        format!(
            r"(?i)^\s*(?:(?:hidden|static)\s+)*{TYPES}\${SCOPE}{w}\s*(?:[-+*/%]?=(?:[^=]|$)|;|#|$)"
        ),
        // A method or a constructor: `[decimal] Total() {`, `Invoice([string] $id) {`.
        format!(r"(?i)^\s*(?:(?:hidden|static)\s+)*{TYPES}{w}\s*\("),
        // An enum member: `Active`, `Closed = 2`.
        format!(r"(?i)^\s+{w}\s*(?:=\s*[^=\s]|#|$)"),
        format!(r#"(?i)^\s*(?:Set|New)-Alias\s+(?:-Name\s+)?["']?{w}["']?(?:\s|$)"#),
        // A static property, the one `[Type]::Name` reaches: `static [int]$Max = 5`.
        format!(
            r"(?i)^\s*(?:hidden\s+)*static\s+(?:hidden\s+)*{TYPES}\${w}\s*(?:=(?:[^=]|$)|;|#|$)"
        ),
        // An environment variable, set only as `$env:Name`, which no other spelling declares.
        format!(r"(?i)^\s*\$env:{w}\s*[-+]?=(?:[^=]|$)"),
    ]
}

/// [`powershell_patterns`] cut to what the word between `before` and `after` names: a
/// variable behind `$`, a scope's `$script:` or a splat's `@` is assigned or a property, never
/// a function (`$tariff` declares no `Tariff`), and `$env:Name` an environment variable alone;
/// a member behind `.` is a property or a method; a static member behind `::` anything but a
/// variable, a static property included; a bare word is a command or a type, whose
/// constructors count only where it is built: `[Tariff]::new(`.
pub fn powershell_sigil(patterns: &mut Vec<String>, before: &str, after: &str) {
    static SIGIL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)(?:[$@]|\$(?:global|script|local|private|using):)$").unwrap()
    });
    if patterns.len() != 8 {
        return;
    }
    let keep: &[usize] = match before {
        b if b.to_ascii_lowercase().ends_with("$env:") => &[7],
        b if SIGIL.is_match(b) => &[2],
        b if b.ends_with('.') => &[2, 3],
        b if b.ends_with("::") => &[0, 1, 3, 4, 5, 6],
        _ if after.starts_with("]::new(") => &[0, 1, 3, 4, 5],
        _ => &[0, 1, 4, 5],
    };
    let all = std::mem::take(patterns);
    patterns.extend(keep.iter().map(|&i| all[i].clone()));
}

/// Whether the PowerShell `line` declares `word` where it first stands whole in it: the
/// patterns [`powershell_sigil`] keeps for that spelling, so `u` on the class `Tariff` reads
/// `$tariff = [Tariff]::new()` as a use, as `d` does.
pub fn powershell_declares_here(line: &str, word: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let Some(at) = line
        .match_indices(word)
        .map(|(i, _)| i)
        .find(|&i| !line[..i].ends_with(part) && !line[i + word.len()..].starts_with(part))
    else {
        return false;
    };
    let mut patterns = powershell_patterns(word);
    powershell_sigil(&mut patterns, &line[..at], &line[at + word.len()..]);
    Regex::new(&patterns.join("|")).is_ok_and(|re| re.is_match(line))
}

/// Whether the word behind `before` is a named argument, `-Id` of `Get-ShopUser -Id 42`: it
/// names a parameter of the command called, which no rule reads, never a variable of the caller.
pub fn powershell_argument(before: &str) -> bool {
    before
        .strip_suffix('-')
        .is_some_and(|b| b.is_empty() || b.ends_with([' ', '\t', '(']))
}

/// Whether 1-based `line` of `lines`, matched by [`powershell_patterns`], declares where it sits:
/// an enum member directly inside an `enum`, a method directly inside a `class`, and no variable
/// or property inside a `param(` block, which is a parameter: [`powershell_params`] reads it, in
/// its own file only.
pub fn powershell_declares<S: AsRef<str>>(lines: &[S], line: usize, line_text: &str) -> bool {
    static KEYWORD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(?:(?:function|filter)\s|(?:\[[^\]]*\]\s*)*(?:class|enum)\s|(?:Set|New)-Alias\s)").unwrap()
    });
    if KEYWORD.is_match(line_text) {
        return true;
    }
    // A variable or a property behind its modifiers, attributes and type, which may hold a `(`:
    // `[ValidateRange(1, 9)][int]$Retries = 3`.
    if VARIABLE.is_match(line_text) {
        return !in_param_block(lines, line);
    }
    // A name and its `(`: a method; a name alone, whatever its value or comment holds: an enum
    // member.
    match METHOD.is_match(line_text) {
        true => in_class(lines, line),
        false => owner(lines, line).is_some_and(|o| opens(o, "enum")),
    }
}

/// A line that opens with a variable, behind `hidden`, `static`, attributes and a type.
static VARIABLE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:(?:hidden|static)\s+)*(?:\[[^\]]*\]\s*)*\$").unwrap()
});
/// A line that opens with a name and its `(`, behind `hidden`, `static`, attributes and a type:
/// a method or a constructor where a class holds it.
static METHOD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:(?:hidden|static)\s+)*(?:\[[^\]]*\]\s*)*[A-Za-z_][\w-]*\s*\(").unwrap()
});

/// Whether the line `l` opens a `keyword` block, behind attributes: `[Flags()] enum Status {`.
fn opens(l: &str, keyword: &str) -> bool {
    static OPENER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(?:\[[^\]]*\]\s*)*(class|enum)\s").unwrap()
    });
    OPENER
        .captures(l)
        .is_some_and(|c| c[1].eq_ignore_ascii_case(keyword))
}

fn in_class<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    owner(lines, line).is_some_and(|o| opens(o, "class"))
}

/// The byte of the `(` that opens a parameter list on 0-based line `i` of `lines`: a `param(`, a
/// `function Name(` header, or a method's or a constructor's header directly inside a class.
fn params_at<S: AsRef<str>>(lines: &[S], i: usize) -> Option<usize> {
    let l = lines[i].as_ref();
    PARAM
        .find(l)
        .or_else(|| METHOD.find(l).filter(|_| in_class(lines, i + 1)))
        .map(|m| m.end() - 1)
}

/// The nearest line above 1-based `line` of `lines` that is indented less, past blanks and
/// comments.
fn owner<S: AsRef<str>>(lines: &[S], line: usize) -> Option<&str> {
    let depth = indent(lines.get(line.checked_sub(1)?)?.as_ref());
    lines[..line - 1].iter().map(AsRef::as_ref).rev().find(|l| {
        let t = l.trim_start();
        !t.is_empty() && !t.starts_with('#') && indent(l) < depth
    })
}

/// Whether 1-based `line` of `lines` starts inside a `param(` block: the brackets the nearest
/// `param(` above it opened are not all closed when the line starts.
fn in_param_block<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    let Some(k) = line.checked_sub(1) else {
        return false;
    };
    let Some(open) = (0..k)
        .rev()
        .take(80)
        .find(|&i| PARAM.is_match(lines[i].as_ref()))
    else {
        return false;
    };
    let mut depth = 0i32;
    for l in &lines[open..k] {
        let l = l.as_ref();
        let from = match PARAM.find(l) {
            Some(m) => m.end() - 1,
            None => 0,
        };
        depth += brackets(&l[from..]);
    }
    depth > 0
}

/// `param(`, or the parameter list of a `function Name(` header.
static PARAM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)\bparam\s*\(|^\s*(?:function|filter)\s+[\w:-]+\s*\(").unwrap()
});

/// The opened minus the closed round brackets of `s`, outside quotes and a `#` comment.
fn brackets(s: &str) -> i32 {
    let (mut depth, mut quote) = (0, None);
    for c in s.bytes() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(c),
            (None, b'#') => break,
            (None, b'(') => depth += 1,
            (None, b')') => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// The parameter `name` of the `param(` blocks above 0-based `at` in the blocks around it, told
/// by indentation, innermost first: a function's, a script block's or the script's, or the list
/// of a `function Name($a) {` header or of a method or a constructor of a class. A function beside the cursor's, at its level or deeper,
/// is not around it, nor are its parameters.
pub fn powershell_params(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let var = Regex::new(&format!(r"(?i)\${}(?:[^\w-]|$)", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let mut depth = indent(lines[at]);
    for i in (0..=at).rev() {
        let l = lines[i];
        if l.trim().is_empty() || indent(l) > depth {
            continue;
        }
        let open = params_at(lines, i);
        // A function or a method beside the cursor's, at its level: none of its parameters.
        let header = HEADER.is_match(l) || (METHOD.is_match(l) && in_class(lines, i + 1));
        let beside = i != at && indent(l) == depth && header;
        depth = indent(l);
        let Some(from) = open.filter(|_| !beside) else {
            continue;
        };
        let mut open = 0;
        for (n, b) in lines.iter().enumerate().skip(i).take(80) {
            let from = if n == i { from } else { 0 };
            if var.is_match(&b[from..]) {
                return vec![Binding {
                    line: n + 1,
                    value: Value::Unknown,
                }];
            }
            open += brackets(&b[from..]);
            if open <= 0 {
                break;
            }
        }
    }
    Vec::new()
}

static HEADER: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"(?i)^\s*(?:function|filter)\s").unwrap());

/// The path a dot-source (`. ./helpers.ps1`, `. $PSScriptRoot/helpers.ps1`), an `Import-Module
/// ./Shop/Users.psm1` or a `using module ./Shop.psm1` on `line` names, when byte `col` stands on
/// it, relative to the file's directory: `$PSScriptRoot` is that directory.
pub fn powershell_import(line: &str, col: usize) -> Option<String> {
    static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r#"(?i)^\s*(?:\.|Import-Module|using\s+module)\s+(?:-Name\s+)?(["']?)([^"'\s;|]+\.ps[dm]?1)["']?"#,
        )
        .unwrap()
    });
    let c = IMPORT.captures(line)?;
    let path = c.get(2)?;
    if !(c.get(1)?.start() <= col && col <= path.end()) {
        return None;
    }
    let p = path.as_str().replace('\\', "/");
    let p = ["$PSScriptRoot/", "${PSScriptRoot}/"]
        .iter()
        .find_map(|r| {
            (p.len() >= r.len() && p[..r.len()].eq_ignore_ascii_case(r)).then(|| &p[r.len()..])
        })
        .unwrap_or(&p);
    Some(p.trim_start_matches("./").to_owned())
}

/// The module directories of `PSModulePath`: `env` when it is set, else PowerShell 7's defaults
/// on macOS and Linux under `home`, and the `Modules` beside `pwsh`, the real file behind the
/// one on the PATH.
pub(super) fn powershell_roots(
    env: Option<std::ffi::OsString>,
    home: &Path,
    pwsh: Option<PathBuf>,
) -> Vec<PathBuf> {
    if let Some(env) = env.filter(|e| !e.is_empty()) {
        return std::env::split_paths(&env).collect();
    }
    let beside = pwsh
        .and_then(|p| std::fs::canonicalize(p).ok())
        .and_then(|p| Some(p.parent()?.join("Modules")));
    [
        home.join(".local/share/powershell/Modules"),
        PathBuf::from("/usr/local/share/powershell/Modules"),
    ]
    .into_iter()
    .chain(beside)
    .collect()
}

/// What `D` lists of a PowerShell file: a function and a filter under the whole `Verb-Noun`
/// name, a class and an enum.
pub(super) const POWERSHELL_SYMBOL: &str = r"(?i)^\s*(?:(?:function|filter)\s+(?:(?:global|script|local|private):)?|(?:\[[^\]]*\]\s*)*(?:class|enum)\s+)(?P<name>[A-Za-z_][\w-]*)";
