//! Ruby for `d`: its scopes, read by indentation as rubocop lays code out (#383), and where the
//! gems, the standard library and the core's signatures live outside the project (#369).

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

/// What a line around another opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opens {
    /// A `def`, a `class` or a `module`: a wall no local passes.
    Gate,
    /// A `do` or a `{` block: it sees the locals around it, and its own stay inside.
    Block,
    /// An `if`, a `case`, a `begin`, a call wrapped over lines: no scope.
    Nothing,
}

struct Around {
    openers_innermost_first: Vec<(usize, Opens)>,
    nesting_certain: bool,
}

/// An `end` or a closing bracket indented less than the line, or a `def` of one line, loses the
/// nesting: the walk goes on past it.
fn around(lines: &[&str], literal: &[bool], line0: usize) -> Around {
    static GATE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:[a-z_]+\s+)?def\s|^(?:class|module)\b").unwrap());
    static HOLDS_NO_LINE_BELOW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[a-z_]+\s+)?def\s+[^\s(]+(?:\([^)]*\))?\s*=(?:[^=~>]|$)|\bend\s*(?:#.*)?$")
            .unwrap()
    });
    static BLOCK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\bdo|\{)\s*(?:\|[^|]*\|)?\s*(?:#.*)?$").unwrap());
    static GOES_ON_THE_CONSTRUCT_ABOVE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:rescue|else|elsif|ensure|when|in|then)\b").unwrap());
    static CLOSER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:end\b|[}\])])").unwrap());
    let mut depth = indent(lines[line0])
        + usize::from(GOES_ON_THE_CONSTRUCT_ABOVE.is_match(lines[line0].trim()));
    let (mut out, mut sure) = (Vec::new(), true);
    for i in (0..line0).rev() {
        if depth == 0 {
            break;
        }
        let t = lines[i].trim();
        if t.is_empty() || literal[i] || t.starts_with('#') || indent(lines[i]) >= depth {
            continue;
        }
        if GOES_ON_THE_CONSTRUCT_ABOVE.is_match(t) {
            continue;
        }
        depth = indent(lines[i]);
        if CLOSER.is_match(t) {
            sure = false;
            continue;
        }
        let opens = match GATE.is_match(t) {
            true if HOLDS_NO_LINE_BELOW.is_match(t) => {
                sure = false;
                continue;
            }
            true => Opens::Gate,
            false if BLOCK.is_match(t) => Opens::Block,
            false => Opens::Nothing,
        };
        out.push((i, opens));
    }
    Around {
        openers_innermost_first: out,
        nesting_certain: sure,
    }
}

pub fn ruby_locals(text: &str, line1: usize, name: &str) -> Vec<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let literal = literal_lines(Kind::Ruby, text);
    let assign = Regex::new(&ruby_assignment(name)).expect("an escaped name keeps it valid");
    let gate = |a: &[(usize, Opens)]| a.iter().find(|(_, o)| *o == Opens::Gate).map(|p| p.0);
    let Around {
        openers_innermost_first: cursor,
        nesting_certain: sure,
    } = around(&lines, &literal, at);
    (0..lines.len())
        .filter(|&i| !literal[i] && assign.is_match(lines[i]))
        .filter(|&i| {
            let Around {
                openers_innermost_first: scopes,
                nesting_certain: known,
            } = around(&lines, &literal, i);
            let inner = scopes.iter().find(|(_, o)| *o != Opens::Nothing);
            !(sure && known)
                || (gate(&cursor) == gate(&scopes) && inner.is_none_or(|s| cursor.contains(s)))
        })
        .map(|i| i + 1)
        .collect()
}

/// The class or module `line1` of `text` is written in, as its path: `Shop::Basket` for `module
/// Shop` around `class Basket`, and for `class Shop::Basket`. Empty at the top of the file. A
/// `class << self` is the class around it.
pub fn ruby_class_path(text: &str, line1: usize) -> String {
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:class|module)\s+((?:::)?[A-Z][\w:]*)").unwrap());
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return String::new();
    };
    let literal = literal_lines(Kind::Ruby, text);
    let mut names: Vec<&str> = around(&lines, &literal, at)
        .openers_innermost_first
        .iter()
        .filter_map(|&(i, _)| NAME.captures(lines[i]))
        .map(|c| c.get(1).map_or("", |m| m.as_str().trim_start_matches(':')))
        .collect();
    names.reverse();
    names.join("::")
}

/// The names a Ruby parameter list binds: `a, b = 1, *c, d:, e: 2, **f, &g`, a block's
/// `(y, z)` destructured and its `; x` block-locals.
fn param_names(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in list.split(';').flat_map(|l| split_top(Kind::Ruby, l, b',')) {
        let p = part.trim();
        if let Some(inner) = p.strip_prefix('(') {
            out.extend(param_names(inner.strip_suffix(')').unwrap_or(inner)));
            continue;
        }
        let p = p.trim_start_matches(['*', '&']);
        let end = p
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(p.len());
        out.push(p[..end].to_owned());
    }
    out
}

/// Whether the parameters of the `def` on `def_line0` bind `name`: `def m(a, b)` wrapped over
/// lines or not, `def m a, b`, `def self.m(…)`.
fn def_binds(lines: &[&str], def_line0: usize, name: &str) -> bool {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:[a-z_]+\s+)?def\s+(?:(?:self|[A-Z]\w*)\.)?(?:[A-Za-z_]\w*[?!=]?|[^\w\s(]+)",
        )
        .unwrap()
    });
    let Some(head) = HEAD.find(lines[def_line0]) else {
        return false;
    };
    let rest = &lines[def_line0][head.end()..];
    let list = match rest.strip_prefix('(') {
        Some(_) => match group(Kind::Ruby, lines, def_line0, head.end()) {
            Some((inner, _, _)) => inner,
            None => return false,
        },
        // `def m a, b`: up to a `;` or a comment. `def m = …` has none.
        None if rest.starts_with([' ', '\t'])
            && !rest.trim_start().starts_with(['=', ';', '#']) =>
        {
            uncommented(Kind::Ruby, rest.split(';').next().unwrap_or(""))
        }
        None => return false,
    };
    param_names(&list).iter().any(|p| p == name)
}

fn block_binds(line: &str, name: &str) -> bool {
    static PARAMS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\bdo|\{)\s*\|([^|]*)\|\s*(?:#.*)?$").unwrap());
    PARAMS
        .captures(line)
        .is_some_and(|c| param_names(&c[1]).iter().any(|p| p == name))
}

pub(super) fn ruby_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let n = regex::escape(name);
    let local = Regex::new(&format!(
        r"^\s*(?:\*?[a-z_]\w*\s*,\s*)*\*?{n}(?:\s*,\s*\*?[a-z_]\w*)*\s*(?:\|\|)?=(?:[^=~>]|$)|^\s*rescue\b[^#]*=>\s*{n}\s*(?:#.*|then\b.*)?$|^\s*for\s+(?:[a-z_]\w*\s*,\s*)*{n}\b[^#]*\bin\b"
    ))
    .expect("an escaped name keeps the pattern valid");
    let found = |line: usize| Binding {
        line1: line + 1,
        value: Value::Unknown,
    };
    if def_binds(lines, at, name) || block_binds(lines[at], name) {
        return vec![found(at)];
    }
    let text = lines.join("\n");
    let literal = literal_lines(Kind::Ruby, &text);
    let Around {
        openers_innermost_first: scopes,
        nesting_certain,
    } = around(lines, &literal, at);
    if !nesting_certain {
        return Vec::new();
    }
    let chain = scopes
        .iter()
        .filter(|(_, o)| *o != Opens::Nothing)
        .map(|&(i, o)| (Some(i), o))
        .chain(std::iter::once((None, Opens::Gate)));
    for (opener, opens) in chain {
        let from = opener.map_or(0, |o| o + 1);
        let mut out: Vec<Binding> = (from..=at)
            .filter(|&i| !literal[i] && local.is_match(lines[i]))
            .filter(|&i| {
                let Around {
                    openers_innermost_first: own,
                    nesting_certain: known,
                } = around(lines, &literal, i);
                let inner = own.iter().find(|(_, o)| *o != Opens::Nothing).map(|p| p.0);
                known && inner == opener
            })
            .map(found)
            .collect();
        if let Some(o) = opener {
            let header = match opens {
                Opens::Block => block_binds(lines[o], name),
                _ => def_binds(lines, o, name),
            };
            if header {
                out.insert(0, found(o));
            }
        }
        if !out.is_empty() || opens == Opens::Gate {
            return out;
        }
    }
    Vec::new()
}

/// The path a Ruby `class` or `module` line on `line1` of `text` declares, the classes and
/// modules around it included: `Shop::Basket` for `class Basket` inside `module Shop`. `None`
/// for any other line and for `class << self`.
pub fn ruby_declared_path(text: &str, line1: usize) -> Option<String> {
    static NAME: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*(?:class|module)\s+(?:::)?([A-Z][\w:]*)").unwrap());
    let own = NAME.captures(text.lines().nth(line1.checked_sub(1)?)?)?[1].to_owned();
    let outer = ruby_class_path(text, line1);
    Some(match outer.is_empty() {
        true => own,
        false => format!("{outer}::{own}"),
    })
}

/// What the Ruby class or module declared on `line1` of `text` inherits, as written: a `prepend`
/// counts as an `include`, and only what its body says directly.
pub fn ruby_class_parents(text: &str, line1: usize) -> RubyParents {
    static SUPER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*class\s+[\w:]+\s*<\s*(?:::)?([A-Z][\w:]*)").unwrap());
    static MIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(include|prepend|extend)\s+((?:::)?[A-Z][\w:]*(?:\s*,\s*(?:::)?[A-Z][\w:]*)*)\s*(?:#.*)?$")
            .unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(head) = line1.checked_sub(1).and_then(|i| lines.get(i)) else {
        return RubyParents::default();
    };
    let superclasses = SUPER
        .captures(head)
        .map(|c| c[1].to_owned())
        .into_iter()
        .collect();
    let literal = literal_lines(Kind::Ruby, text);
    let (mut includes, mut extends) = (Vec::new(), Vec::new());
    for i in line1..lines.len() {
        let t = lines[i].trim();
        if !t.is_empty() && !literal[i] && indent(lines[i]) <= indent(head) {
            break;
        }
        let Some(c) = MIX.captures(lines[i]).filter(|_| !literal[i]) else {
            continue;
        };
        let directly_in_body = around(&lines, &literal, i)
            .openers_innermost_first
            .iter()
            .find(|(_, o)| *o != Opens::Nothing)
            .map(|p| p.0)
            == Some(line1 - 1);
        if !directly_in_body {
            continue;
        }
        let names = c[2]
            .split(',')
            .map(|n| n.trim().trim_start_matches(':').to_owned());
        match &c[1] {
            "extend" => extends.extend(names),
            _ => includes.extend(names),
        }
    }
    RubyParents {
        superclasses,
        includes,
        extends,
    }
}
#[derive(Default)]
pub struct RubyParents {
    pub superclasses: Vec<String>,
    pub includes: Vec<String>,
    pub extends: Vec<String>,
}

pub fn ruby_on_class(text: &str, line1: usize) -> bool {
    static ON: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:[a-z_]+\s+)?def\s+(?:self|[A-Z]\w*)\.|scope\s*\(?\s*:)").unwrap()
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    ON.is_match(lines[at])
        || around(&lines, &literal, at)
            .openers_innermost_first
            .iter()
            .find(|(_, o)| *o != Opens::Nothing)
            .is_some_and(|&(i, _)| lines[i].trim_start().starts_with("class << self"))
}

/// Whether `self` at `line1` of `text` is the class rather than an instance of it: in the body of
/// a class method (see [`ruby_singleton`]), in `class << self`, or in the class body itself, where
/// `has_many :x` is a call on the class.
pub fn ruby_self_is_class(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    let scopes = around(&lines, &literal, at).openers_innermost_first;
    let def = Regex::new(r"^\s*(?:[a-z_]+\s+)?def\s").expect("a valid pattern");
    match scopes.iter().find(|(_, o)| *o == Opens::Gate) {
        Some(&(g, _)) if def.is_match(lines[g]) => ruby_singleton(text, g + 1),
        Some(_) => true,
        None => false,
    }
}

/// Where Ruby's code outside the project at `root` lives (#369): the core's RBS signatures, the
/// standard library and the gems `Gemfile.lock` names, in that order. Nothing the project ships
/// is run (#183): `bundle` would evaluate its `Gemfile`, so the lockfile is read instead.
///
/// - The gems are the `specs:` of its `GEM` and `GIT` sections, at the versions it locks (a
///   `PATH` gem is in the project already): `gems/<name>-<version>` and
///   `bundler/gems/<repository>-<revision>` of the first gem directory that has them (their
///   `lib` when there is one), which are
///   the project's `BUNDLE_PATH` from `.bundle/config`, then `gem_env` (`GEM_HOME`, `GEM_PATH`),
///   then the Ruby's own.
/// - The Ruby is the one `.ruby-version` names under rbenv, mise, asdf or chruby in `home`, else
///   what `ask` says of the `ruby` on the PATH: its `rubylibdir`, then its gem path.
/// - The standard library is that Ruby's `lib/ruby/<abi>`, and the core is the `core/` of the
///   newest `rbs` gem found, signatures of what is written in C.
///
/// Empty when no gem the lockfile names is installed, or there is no lockfile: `d` stays in the
/// project then.
pub fn ruby_roots(
    root: &Path,
    home: &Path,
    gem_env: &[PathBuf],
    ask: impl FnOnce() -> Option<String>,
) -> Vec<PathBuf> {
    let lock = std::fs::read_to_string(root.join("Gemfile.lock")).unwrap_or_default();
    let mut gems: Vec<PathBuf> = Vec::new();
    let (mut section, mut remote, mut revision) = ("", "", "");
    for line in lock.lines() {
        if !line.starts_with(' ') {
            section = line.trim();
        } else if let Some(r) = line.strip_prefix("  remote: ") {
            remote = r.trim();
        } else if let Some(r) = line.strip_prefix("  revision: ") {
            revision = r.trim();
        } else if let Some(spec) = line.strip_prefix("    ").filter(|s| !s.starts_with(' '))
            && let Some((name, version)) = spec
                .trim_end()
                .strip_suffix(')')
                .and_then(|s| s.split_once(" ("))
        {
            let dir = match section {
                "GEM" => Path::new("gems").join(format!("{name}-{version}")),
                // Bundler checks a repository out once, under its name and short revision.
                "GIT" => {
                    let repo = remote
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap_or("");
                    let repo = repo.trim_end_matches(".git");
                    let short = revision.get(..12).unwrap_or(revision);
                    Path::new("bundler/gems").join(format!("{repo}-{short}"))
                }
                _ => continue,
            };
            if !gems.contains(&dir) {
                gems.push(dir);
            }
        }
    }
    if gems.is_empty() {
        return Vec::new();
    }
    let subdirs = |dir: &Path| -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        found.sort();
        found
    };
    let abi_dirs_named_after_a_version = |dir: &Path| -> Vec<PathBuf> {
        subdirs(dir)
            .into_iter()
            .filter(|p| {
                p.file_name().is_some_and(|n| {
                    n.to_string_lossy()
                        .starts_with(|c: char| c.is_ascii_digit())
                })
            })
            .collect()
    };
    let config = std::fs::read_to_string(root.join(".bundle/config")).unwrap_or_default();
    // `../bundle` is read without its `..`, so a gem there is named from its own root.
    let bundle = config.lines().find_map(|l| {
        let value = l
            .strip_prefix("BUNDLE_PATH:")?
            .trim()
            .trim_matches(['"', '\'']);
        let mut path = PathBuf::new();
        for part in root.join(value).join("ruby").components() {
            match part {
                std::path::Component::ParentDir => _ = path.pop(),
                part => path.push(part),
            }
        }
        Some(path)
    });
    let mut homes: Vec<PathBuf> = bundle
        .iter()
        .flat_map(|b| abi_dirs_named_after_a_version(b))
        .collect();
    homes.extend(gem_env.iter().cloned());
    let version = std::fs::read_to_string(root.join(".ruby-version")).unwrap_or_default();
    let version = version.lines().next().unwrap_or("").trim();
    let version = version.strip_prefix("ruby-").unwrap_or(version);
    let prefix = [
        home.join(".rbenv/versions").join(version),
        home.join(".local/share/mise/installs/ruby").join(version),
        home.join(".asdf/installs/ruby").join(version),
        home.join(".rubies").join(format!("ruby-{version}")),
    ]
    .into_iter()
    .find(|p| !version.is_empty() && p.is_dir());
    let stdlib: Vec<PathBuf> = match prefix {
        Some(prefix) => {
            let lib = prefix.join("lib/ruby");
            homes.extend(abi_dirs_named_after_a_version(&lib.join("gems")));
            abi_dirs_named_after_a_version(&lib)
        }
        None => {
            let said = ask().unwrap_or_default();
            let mut said = said.lines().map(PathBuf::from);
            let stdlib = said.next().into_iter().collect();
            homes.extend(said);
            stdlib
        }
    };
    // A gem is required from its `lib`: its `spec/` and `test/` declare helpers of its own.
    let found: Vec<PathBuf> = gems
        .iter()
        .filter_map(|g| homes.iter().map(|h| h.join(g)).find(|d| d.is_dir()))
        .map(|d| match d.join("lib") {
            lib if lib.is_dir() => lib,
            _ => d,
        })
        .collect();
    if found.is_empty() {
        return Vec::new();
    }
    // `rbs-3.10.0` is newer than `rbs-3.9.1`.
    let rbs_version = |p: &PathBuf| -> Option<Vec<u64>> {
        let name = p.file_name()?.to_str()?.strip_prefix("rbs-")?;
        name.split('.').map(|n| n.parse().ok()).collect()
    };
    let core = homes
        .iter()
        .flat_map(|h| subdirs(&h.join("gems")))
        .filter_map(|p| Some((rbs_version(&p)?, p)))
        .max()
        .map(|(_, p)| p.join("core"));
    core.into_iter().chain(stdlib).chain(found).collect()
}
