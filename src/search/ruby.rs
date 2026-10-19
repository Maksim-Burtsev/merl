use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::{
    Binding, Kind, Value, code, group, indent, literal_lines, ruby_assignment, ruby_singleton,
    split_top, uncommented,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opens {
    Gate,
    Block,
    Nothing,
}

struct Around {
    openers_innermost_first: Vec<(usize, Opens)>,
    nesting_certain: bool,
}

fn around(lines: &[&str], literal: &[bool], line0: usize) -> Around {
    static GATE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:[a-z_]+\s+)?def\s|^(?:class|module)\b").expect("the regex is a literal")
    });
    static HOLDS_NO_LINE_BELOW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?:[a-z_]+\s+)?def\s+[^\s(]+(?:\([^)]*\)\s*|\s+)=(?:[^=~>]|$)|\bend\s*(?:#.*)?$",
        )
        .expect("the regex is a literal")
    });
    static BLOCK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:\bdo|\{)\s*(?:\|[^|]*\|)?\s*(?:#.*)?$").expect("the regex is a literal")
    });
    static GOES_ON_THE_CONSTRUCT_ABOVE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:rescue|else|elsif|ensure|when|in|then)\b").expect("the regex is a literal")
    });
    static CLOSER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:end\b|[}\])])").expect("the regex is a literal"));
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

fn openers_outermost_first(text: &str, line1: usize) -> Vec<&str> {
    static NAME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:class|module)\s+((?:::)?[A-Z][\w:]*)").expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    let literal = literal_lines(Kind::Ruby, text);
    let mut names: Vec<&str> = around(&lines, &literal, at)
        .openers_innermost_first
        .iter()
        .filter_map(|&(i, _)| NAME.captures(lines[i]))
        .map(|c| c.get(1).map_or("", |m| m.as_str()))
        .collect();
    names.reverse();
    names
}

pub fn ruby_class_path(text: &str, line1: usize) -> String {
    openers_outermost_first(text, line1)
        .iter()
        .map(|n| n.trim_start_matches(':'))
        .collect::<Vec<_>>()
        .join("::")
}

pub fn ruby_nesting_innermost_first(text: &str, line1: usize) -> Vec<String> {
    let mut path = String::new();
    let mut out: Vec<String> = openers_outermost_first(text, line1)
        .into_iter()
        .map(|n| {
            path = match n.strip_prefix("::") {
                Some(top) => top.to_owned(),
                None if path.is_empty() => n.to_owned(),
                None => format!("{path}::{n}"),
            };
            path.clone()
        })
        .collect();
    out.reverse();
    out
}

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
        out.extend(p.get(..end).map(str::to_owned));
    }
    out
}

fn def_binds(lines: &[&str], def_line0: usize, name: &str) -> bool {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^\s*(?:[a-z_]+\s+)?def\s+(?:(?:self|[A-Z]\w*)\.)?(?:[A-Za-z_]\w*[?!=]?|[^\w\s(]+)",
        )
        .expect("the regex is a literal")
    });
    let Some(head) = HEAD.find(lines[def_line0]) else {
        return false;
    };
    let Some(rest) = lines[def_line0].get(head.end()..) else {
        return false;
    };
    let list = match rest.strip_prefix('(') {
        Some(_) => match group(Kind::Ruby, lines, def_line0, head.end()) {
            Some(g) => g.inner_uncommented,
            None => return false,
        },
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
    static PARAMS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:\bdo|\{)\s*\|([^|]*)\|\s*(?:#.*)?$").expect("the regex is a literal")
    });
    PARAMS
        .captures(line)
        .is_some_and(|c| param_names(&c[1]).iter().any(|p| p == name))
}

pub fn ruby_block_param(line: &str, name: &str, at: usize) -> bool {
    static PARAMS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*\|([^|]*)\|").expect("the regex is a literal"));
    let b = line.as_bytes();
    let word = |c: &u8| c.is_ascii_alphanumeric() || *c == b'_';
    let keyword = |i: usize, w: &str| {
        line.get(i..).is_some_and(|s| s.starts_with(w))
            && (i == 0 || !word(&b[i - 1]))
            && b.get(i + w.len()).is_none_or(|c| !word(c))
    };
    let mut open = Vec::new();
    for (i, c) in code(Kind::Ruby, line).take_while(|&(i, _)| i < at) {
        match c {
            b'{' => open.push(i + 1),
            b'd' if keyword(i, "do") => open.push(i + 2),
            b'}' => drop(open.pop()),
            b'e' if keyword(i, "end") => drop(open.pop()),
            _ => {}
        }
    }
    open.iter().any(|&o| {
        line.get(o..at)
            .and_then(|s| PARAMS.captures(s))
            .is_some_and(|c| param_names(&c[1]).iter().any(|p| p == name))
    })
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

pub fn ruby_declared_path(text: &str, line1: usize) -> Option<String> {
    static NAME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:class|module)\s+(?:::)?([A-Z][\w:]*)").expect("the regex is a literal")
    });
    let own = NAME.captures(text.lines().nth(line1.checked_sub(1)?)?)?[1].to_owned();
    let outer = ruby_class_path(text, line1);
    Some(if outer.is_empty() {
        own
    } else {
        format!("{outer}::{own}")
    })
}

pub fn ruby_class_parents(text: &str, line1: usize) -> RubyParents {
    static SUPER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*class\s+[\w:]+\s*<\s*((?:::)?[A-Z][\w:]*)")
            .expect("the regex is a literal")
    });
    static MIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(include|prepend|extend)\s+((?:::)?[A-Z][\w:]*(?:\s*,\s*(?:::)?[A-Z][\w:]*)*)(?:\s+(?:if|unless)\s.*)?\s*(?:#.*)?$")
            .expect("the regex is a literal")
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
        let names = c[2].split(',').map(|n| n.trim().to_owned());
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

pub fn ruby_loop_keyword(word: &str, lead: &str, tail: &str) -> bool {
    matches!(word, "next" | "redo" | "retry" | "break")
        && !lead.trim_end().ends_with(['.', ':'])
        && lead.split_whitespace().last() != Some("def")
        && (!tail.starts_with(':') || tail.starts_with("::"))
}

pub struct RubyMade {
    pub class: String,
    pub by_singleton: bool,
}

pub fn ruby_constructed_receiver(before: &str) -> Option<RubyMade> {
    static NEW: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:^|[^\w:.@$])((?:::)?[A-Z]\w*(?:::[A-Z]\w*)*)\.(?:new(?:\s*\([^()]*\))?|(instance))\.$",
        )
        .expect("the regex is a literal")
    });
    NEW.captures(before.trim_end()).map(|c| RubyMade {
        class: c[1].to_owned(),
        by_singleton: c.get(2).is_some(),
    })
}

pub fn ruby_instance_of_class(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    !ruby_self_is_class(text, line1)
        && around(&lines, &literal, at)
            .openers_innermost_first
            .iter()
            .map(|&(i, _)| lines[i].trim_start())
            .find(|l| l.starts_with("class ") || l.starts_with("module "))
            .is_some_and(|l| !l.starts_with("module ") && !l.starts_with("class <<"))
}

pub fn ruby_in_module(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    around(&lines, &literal, at)
        .openers_innermost_first
        .iter()
        .map(|&(i, _)| lines[i].trim_start())
        .find(|l| {
            l.starts_with("module ") || (l.starts_with("class ") && !l.starts_with("class <<"))
        })
        .is_some_and(|l| l.starts_with("module "))
}

pub fn ruby_in_block(text: &str, line1: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let literal = literal_lines(Kind::Ruby, text);
    around(&lines, &literal, at)
        .openers_innermost_first
        .iter()
        .find(|(_, o)| *o != Opens::Nothing)
        .is_some_and(|(_, o)| *o == Opens::Block)
}

pub fn ruby_on_class(text: &str, line1: usize) -> bool {
    static ON: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\s*(?:(?:[a-z_]+\s+)?def\s+(?:self|[A-Z]\w*)\.|scope\s*\(?\s*:)")
            .expect("the regex is a literal")
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

fn locked_gems(root: &Path) -> Vec<PathBuf> {
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
    gems
}

pub fn ruby_roots(
    root: &Path,
    home: &Path,
    gem_env: &[PathBuf],
    ask: impl FnOnce() -> Option<String>,
) -> Vec<PathBuf> {
    let gems = locked_gems(root);
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
    let stdlib: Vec<PathBuf> = if let Some(prefix) = prefix {
        let lib = prefix.join("lib/ruby");
        homes.extend(abi_dirs_named_after_a_version(&lib.join("gems")));
        abi_dirs_named_after_a_version(&lib)
    } else {
        let said = ask().unwrap_or_default();
        let mut said = said.lines().map(PathBuf::from);
        let stdlib = said.next().into_iter().collect();
        homes.extend(said);
        stdlib
    };
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
