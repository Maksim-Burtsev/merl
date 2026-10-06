use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const END: &str = r#"(?:$|[\s()\[\]{}"])"#;
const META: &str = r"(?:\^(?:\{[^}]*\}|\S+)\s+)*";

fn heads(kind: Kind) -> &'static str {
    match kind {
        Kind::Clojure => {
            "def|defn-?|defmacro|defonce|defmulti|defmethod|defprotocol|defrecord|deftype|definterface|defstruct|ns"
        }
        Kind::EmacsLisp => concat!(
            "defun|defmacro|defsubst|defvar|defvar-local|defcustom|defconst|defface|defgroup|",
            "defalias|defclass|defgeneric|defmethod|cl-defun|cl-defmacro|cl-defsubst|",
            "cl-defgeneric|cl-defmethod|cl-defstruct|define-minor-mode|define-derived-mode|",
            "define-globalized-minor-mode|define-inline|define-error|",
            "transient-define-(?:prefix|suffix|infix|argument)"
        ),
        Kind::Scheme => concat!(
            "define|define-syntax|define-syntax-rule|define-record-type|struct|",
            "define/contract|define/public|define/private|define/override|module"
        ),
        _ => concat!(
            "defun|defmacro|defvar|defparameter|defconstant|defclass|defstruct|defgeneric|",
            "defmethod|deftype|define-condition|define-compiler-macro|defpackage|defsystem"
        ),
    }
}

pub fn lisp_patterns(kind: Kind, word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let case = if kind == Kind::CommonLisp { "(?i)" } else { "" };
    let meta = if kind == Kind::Clojure { META } else { "" };
    let heads = heads(kind);
    let mut out = vec![format!(
        r#"{case}^\s*\((?:{heads})\s+{meta}(?:#?'|#?:|")?\(?{w}{END}"#
    )];
    match kind {
        Kind::Clojure => {
            out.push(format!(r"^\s+\({w}\s+\["));
            if let Some(record) = word.strip_prefix("->").or(word.strip_prefix("map->")) {
                let r = regex::escape(record);
                out.push(format!(r"^\s*\((?:defrecord|deftype)\s+{meta}{r}{END}"));
            }
        }
        Kind::Scheme => out.push(format!(r"^\s*\(define-values\s+\((?:[^()]*?\s)?{w}[\s)]")),
        _ => {}
    }
    out
}

pub(super) const CLOJURE_SYMBOL: &str = concat!(
    r"^\s*\((?:defn-?|defmacro|defmulti|defprotocol|defrecord|deftype|definterface|ns)\s+",
    r"(?:\^(?:\{[^}]*\}|\S+)\s+)*(?P<name>[^\s()\[\]{}^]+)"
);
pub(super) const EMACS_LISP_SYMBOL: &str = concat!(
    r"^\s*\((?:defun|defmacro|defsubst|defclass|cl-defun|cl-defmacro|cl-defsubst|",
    r"cl-defgeneric|cl-defstruct|define-minor-mode|define-derived-mode|",
    r"define-globalized-minor-mode|transient-define-(?:prefix|suffix|infix|argument))\s+",
    r"\(?(?P<name>[^\s()\[\]{}']+)"
);
pub(super) const SCHEME_SYMBOL: &str = concat!(
    r"^\s*\((?:define\s+\(|define-syntax\s+\(?|define-syntax-rule\s+\(|struct\s+|",
    r"define-record-type\s+\(?)(?P<name>[^\s()\[\]{}']+)"
);
pub(super) const COMMON_LISP_SYMBOL: &str = concat!(
    r"^\s*\((?i:defun|defmacro|defgeneric|defclass|defstruct|define-condition|defpackage)\s+",
    r#"(?:#?:|")?\(?(?P<name>[^\s()\[\]{}'"]+)"#
);

fn name_byte(kind: Kind, c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || word_chars(Some(kind), false).contains(c as char)
}

pub fn lisp_name(kind: Kind, line: &str, r: Range<usize>) -> Range<usize> {
    let b = line.as_bytes();
    let (mut start, mut end) = (r.start, r.end);
    while start > 0 && name_byte(kind, b[start - 1]) {
        start -= 1;
    }
    while end < b.len() && name_byte(kind, b[end]) {
        end += 1;
    }
    start..end
}

struct Lexed {
    code: Vec<u8>,
    literal: Vec<bool>,
}

fn delimiter(c: u8) -> bool {
    c.is_ascii_whitespace() || b"()[]{}\";,".contains(&c)
}

fn lex(kind: Kind, text: &str) -> Lexed {
    let b = text.as_bytes();
    let clojure = kind == Kind::Clojure;
    let hashed = matches!(kind, Kind::Scheme | Kind::CommonLisp);
    let mut code = b.to_vec();
    let mut literal = vec![false];
    let (mut depth, mut i) = (0usize, 0usize);
    let (mut string, mut block) = (false, 0usize);
    let mut ignored: Option<(usize, usize)> = None;
    let mut datum = false;
    let blank = |code: &mut Vec<u8>, r: Range<usize>| {
        for c in &mut code[r] {
            if *c != b'\n' {
                *c = b' ';
            }
        }
    };
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            literal.push(string || block > 0 || ignored.is_some());
            i += 1;
            continue;
        }
        let at = |s: &[u8]| b[i..].starts_with(s);
        let start = i;
        if string {
            match c {
                b'\\' if b.get(i + 1) != Some(&b'\n') => i = (i + 2).min(b.len()),
                b'"' => string = false,
                _ => {}
            }
            if string {
                i += usize::from(i == start);
                blank(&mut code, start..i);
            } else {
                i += 1;
            }
        } else if block > 0 {
            if at(b"|#") {
                block -= 1;
                i += 2;
            } else if at(b"#|") {
                block += 1;
                i += 2;
            } else {
                i += 1;
            }
            blank(&mut code, start..i);
        } else if c == b';' {
            i += b[i..]
                .iter()
                .position(|&c| c == b'\n')
                .unwrap_or(b.len() - i);
            blank(&mut code, start..i);
        } else if c.is_ascii_whitespace() || (clojure && c == b',') {
            i += 1;
        } else if hashed && at(b"#|") {
            block = 1;
            i += 2;
            blank(&mut code, start..i);
        } else if (clojure && at(b"#_")) || (kind == Kind::Scheme && at(b"#;")) {
            if ignored.is_none() {
                ignored = Some((depth, i));
                datum = true;
            }
            i += 2;
        } else if (clojure && c == b'\\')
            || (hashed && at(b"#\\"))
            || (kind == Kind::EmacsLisp && c == b'?' && (i == 0 || delimiter(b[i - 1])))
        {
            let lead = match c {
                b'\\' => 1,
                b'#' => 2,
                _ => 1 + usize::from(b.get(i + 1) == Some(&b'\\')),
            };
            i = (i + lead + usize::from(b.get(i + lead) != Some(&b'\n'))).min(b.len());
            while i < b.len() && b[i].is_ascii_alphabetic() && b[i - 1].is_ascii_alphabetic() {
                i += 1;
            }
            blank(&mut code, start..i);
            datum = false;
        } else {
            if clojure
                && depth == 0
                && ignored.is_none()
                && at(b"(comment")
                && b.get(i + 8).is_none_or(|&c| delimiter(c))
            {
                ignored = Some((0, i));
            }
            match c {
                b'"' => {
                    string = true;
                    datum = false;
                    i += 1;
                }
                b'(' | b'[' | b'{' => {
                    depth += 1;
                    datum = false;
                    i += 1;
                }
                b')' | b']' | b'}' => {
                    depth = depth.saturating_sub(1);
                    if ignored.is_some_and(|(d, _)| depth < d) {
                        ignored = None;
                        datum = false;
                    }
                    i += 1;
                }
                b'\'' | b'`' | b'~' | b'@' | b'^' | b'#' => i += 1,
                _ => {
                    datum = false;
                    i += 1 + b[i + 1..]
                        .iter()
                        .position(|&c| delimiter(c))
                        .unwrap_or(b.len() - i - 1);
                }
            }
        }
        if let Some((d, from)) = ignored
            && !datum
            && !string
            && depth == d
            && i > from
        {
            blank(&mut code, from..i);
            ignored = None;
        }
    }
    Lexed { code, literal }
}

pub(super) fn lisp_literal_lines(kind: Kind, text: &str) -> Vec<bool> {
    lex(kind, text).literal
}

enum Tok<'a> {
    Open(u8),
    Close,
    Atom(&'a str),
}

fn tokens(code: &str) -> Vec<(Tok<'_>, usize)> {
    let b = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            b'(' | b'[' | b'{' => out.push((Tok::Open(c), i)),
            b')' | b']' | b'}' => out.push((Tok::Close, i)),
            b'"' => {
                let end = code[i + 1..].find('"').map_or(b.len(), |n| i + n + 1);
                out.push((Tok::Atom("\"\""), i));
                i = end;
            }
            _ if delimiter(c) || b"'`~@#^".contains(&c) => {}
            _ => {
                let end = b[i..]
                    .iter()
                    .position(|&c| delimiter(c))
                    .map_or(b.len(), |n| i + n);
                out.push((Tok::Atom(&code[i..end]), i));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Plain,
    Pairs,
    Entries,
    First,
    Params,
    Define,
    Destructure,
    Letfn,
    Arity,
}

struct Frame<'a> {
    head: Option<&'a str>,
    idx: usize,
    role: Role,
    scope: usize,
    params: bool,
    named: bool,
    loop_var: bool,
    sequential: bool,
    recursive: bool,
    tests: bool,
    entry: Option<usize>,
}

const CLOJURE_LETS: &[&str] = &[
    "let",
    "loop",
    "for",
    "doseq",
    "binding",
    "with-open",
    "when-let",
    "if-let",
    "when-some",
    "if-some",
    "dotimes",
];
const LETS: &[&str] = &[
    "let",
    "let*",
    "letrec",
    "letrec*",
    "flet",
    "labels",
    "when-let*",
    "if-let*",
    "and-let*",
    "pcase-let*",
];
const SEQUENTIAL: &[&str] = &["let*", "when-let*", "if-let*", "and-let*", "pcase-let*"];
const RECURSIVE: &[&str] = &["letrec", "letrec*", "labels"];
const TESTS: &[&str] = &["when-let*", "if-let*", "and-let*"];
const LAMBDAS: &[&str] = &["lambda", "opt-lambda"];
const CLASSES: &[&str] = &["class", "class*", "mixin"];
const CLAUSES: &[&str] = &["inherit", "inherit-field", "init-field", "field"];
const LOOP_VARS: &[&str] = &["for", "as", "with"];
const DEFUNS: &[&str] = &[
    "defun",
    "defmacro",
    "defsubst",
    "cl-defun",
    "cl-defmacro",
    "cl-defsubst",
    "define-inline",
    "defmethod",
    "cl-defmethod",
];
const DEFINES: &[&str] = &[
    "define",
    "define/contract",
    "define/public",
    "define/private",
    "define/override",
];

fn binds(atom: &str) -> bool {
    !atom.starts_with([':', '&', '"', '#']) && atom != "."
}

pub fn lisp_bindings_at(
    kind: Kind,
    text: &str,
    line: usize,
    col: usize,
    name: &str,
) -> Vec<Binding> {
    let lines: Vec<&str> = text.lines().collect();
    match line.checked_sub(1).filter(|&i| i < lines.len()) {
        Some(at) => lisp_bindings(kind, &lines, at, Some(col), name),
        None => Vec::new(),
    }
}

pub(super) fn lisp_bindings(
    kind: Kind,
    lines: &[&str],
    at: usize,
    col: Option<usize>,
    name: &str,
) -> Vec<Binding> {
    let text = lines.join("\n");
    let code = String::from_utf8_lossy(&lex(kind, &text).code).into_owned();
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let Some(top) = (0..=at)
        .rev()
        .find(|&l| code.as_bytes().get(starts[l]) == Some(&b'('))
    else {
        return Vec::new();
    };
    let here = starts[at];
    let line_end = starts.get(at + 1).map_or(text.len(), |&s| s - 1);
    let same = |a: &str| match kind {
        Kind::CommonLisp => a.eq_ignore_ascii_case(name),
        _ => a == name,
    };
    let clojure = kind == Kind::Clojure;
    let mut scopes: Vec<(usize, Option<usize>)> = Vec::new();
    let mut found: Vec<(usize, usize, usize)> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    for (tok, pos) in tokens(&code[starts[top]..]) {
        let pos = pos + starts[top];
        if pos > line_end {
            break;
        }
        match tok {
            Tok::Open(c) => {
                let role = stack
                    .last_mut()
                    .map_or(Role::Plain, |p| child_role(clojure, p, c));
                let (sequential, entry) = match stack.last() {
                    Some(p) if role == Role::Entries => (
                        SEQUENTIAL.contains(&p.head.unwrap_or("").to_ascii_lowercase().as_str()),
                        None,
                    ),
                    Some(p) if p.role == Role::Entries && p.sequential => {
                        (false, Some(found.len()))
                    }
                    _ => (false, None),
                };
                let recursive = match stack.last() {
                    Some(p) if role == Role::Entries => {
                        RECURSIVE.contains(&p.head.unwrap_or("").to_ascii_lowercase().as_str())
                    }
                    Some(p) => p.role == Role::Letfn || (p.role == Role::Entries && p.recursive),
                    None => false,
                };
                let tests = match stack.last() {
                    Some(p) if role == Role::Entries => {
                        TESTS.contains(&p.head.unwrap_or("").to_ascii_lowercase().as_str())
                    }
                    Some(p) => p.role == Role::Entries && p.tests,
                    None => false,
                };
                let scope = match (role, stack.last()) {
                    (Role::Plain | Role::Arity, _) | (_, None) => {
                        scopes.push((pos, None));
                        scopes.len() - 1
                    }
                    (_, Some(p)) => p.scope,
                };
                stack.push(Frame {
                    head: None,
                    idx: 0,
                    role,
                    scope,
                    params: false,
                    named: false,
                    loop_var: false,
                    sequential,
                    recursive,
                    tests,
                    entry,
                });
            }
            Tok::Close => {
                let Some(f) = stack.pop() else { break };
                if matches!(f.role, Role::Plain | Role::Arity) {
                    scopes[f.scope].1 = Some(pos);
                }
                if let Some(from) = f.entry.filter(|_| f.tests && f.idx < 2) {
                    found.truncate(from);
                }
                if let Some(from) = f.entry {
                    for e in found[from..].iter_mut().filter(|e| e.2 == usize::MAX) {
                        e.2 = pos;
                    }
                }
                match stack.last_mut() {
                    Some(p) => p.idx += 1,
                    None => break,
                }
            }
            Tok::Atom(a) => {
                let class = match stack.len() {
                    n if n >= 2 && kind == Kind::Scheme => (stack[n - 2].head)
                        .filter(|h| CLASSES.contains(h))
                        .map(|_| stack[n - 2].scope),
                    _ => None,
                };
                let pending = stack.last().is_some_and(|f| f.entry.is_some());
                let Some(f) = stack.last_mut() else { break };
                if f.idx == 0 {
                    f.head = Some(a);
                }
                let head = f.head.unwrap_or("");
                let looping = kind == Kind::CommonLisp
                    && f.role == Role::Plain
                    && f.idx > 0
                    && head.eq_ignore_ascii_case("loop");
                let named =
                    kind == Kind::Scheme && f.role == Role::Plain && f.idx == 1 && head == "let";
                let bound = match f.role {
                    Role::Pairs => f.idx.is_multiple_of(2),
                    Role::Entries => !f.tests,
                    Role::Params | Role::Destructure => true,
                    Role::Define => f.idx >= 1,
                    Role::First => f.idx == 0,
                    Role::Plain => (looping && f.loop_var) || named,
                    _ => false,
                };
                if bound && binds(a) && same(a) {
                    let visible = match (f.recursive, pending) {
                        (true, _) => 0,
                        (false, true) => usize::MAX,
                        (false, false) => pos,
                    };
                    found.push((f.scope, pos, visible));
                }
                if looping {
                    f.loop_var = !f.loop_var && LOOP_VARS.iter().any(|k| k.eq_ignore_ascii_case(a));
                }
                f.named |= named;
                if let Some(scope) = class.filter(|_| f.idx == 0 && CLAUSES.contains(&a)) {
                    f.role = Role::Entries;
                    f.scope = scope;
                }
                f.idx += 1;
            }
        }
    }
    let open_at =
        |scope: usize, at: usize| scopes[scope].0 < at && scopes[scope].1.is_none_or(|c| c >= at);
    found
        .into_iter()
        .filter(|&(scope, pos, visible)| match col {
            None => pos >= here || open_at(scope, here),
            Some(col) => pos == here + col || (visible < here + col && open_at(scope, here + col)),
        })
        .max_by_key(|&(scope, pos, _)| (scopes[scope].0, pos))
        .map(|(_, pos, _)| Binding {
            line: line_of(&text, pos),
            value: Value::Unknown,
        })
        .into_iter()
        .collect()
}

fn child_role(clojure: bool, p: &mut Frame, c: u8) -> Role {
    let h = p.head.unwrap_or("");
    match p.role {
        Role::Pairs if p.idx.is_multiple_of(2) => return Role::Destructure,
        Role::Entries | Role::Letfn => return Role::First,
        Role::Params | Role::Define if c == b'(' => return Role::First,
        Role::Params | Role::Define | Role::Destructure => return Role::Destructure,
        Role::Arity if p.idx == 0 && c == b'[' => return Role::Params,
        Role::Plain => {}
        _ => return Role::Plain,
    }
    if p.loop_var {
        p.loop_var = false;
        return Role::Destructure;
    }
    if clojure {
        if c == b'[' && p.idx == 1 && CLOJURE_LETS.contains(&h) {
            return Role::Pairs;
        }
        if c == b'[' && p.idx == 1 && h == "letfn" {
            return Role::Letfn;
        }
        if ["defn", "defn-", "defmacro", "fn"].contains(&h) && p.idx >= 1 && !p.params {
            match c {
                b'[' => {
                    p.params = true;
                    return Role::Params;
                }
                b'(' => return Role::Arity,
                _ => {}
            }
        }
        return Role::Plain;
    }
    if c != b'(' {
        return Role::Plain;
    }
    let h = h.to_ascii_lowercase();
    let h = h.as_str();
    match p.idx {
        1 if LETS.contains(&h) => Role::Entries,
        2 if h == "let" && p.named => Role::Entries,
        1 if LAMBDAS.contains(&h) => Role::Params,
        1 if DEFINES.contains(&h) => Role::Define,
        2 if DEFUNS.contains(&h) => Role::Params,
        _ => Role::Plain,
    }
}

fn line_of(text: &str, pos: usize) -> usize {
    text.as_bytes()[..pos]
        .iter()
        .filter(|&&c| c == b'\n')
        .count()
        + 1
}

pub(super) fn lisp_declares<S: AsRef<str>>(
    kind: Kind,
    lines: &[S],
    line: usize,
    text: &str,
) -> bool {
    if kind != Kind::Clojure
        || text.trim_start().starts_with("(def")
        || text.trim_start().starts_with("(ns")
    {
        return true;
    }
    lines[..line.min(lines.len())]
        .iter()
        .rev()
        .map(AsRef::as_ref)
        .find(|l| l.starts_with('('))
        .is_some_and(|l| l.starts_with("(defprotocol") || l.starts_with("(definterface"))
}

pub fn clojure_requires(text: &str) -> Vec<(String, String)> {
    let lexed = lex(Kind::Clojure, text);
    let code = String::from_utf8_lossy(&lexed.code).into_owned();
    let toks = tokens(&code);
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let required = matches!(
            toks[i].0,
            Tok::Atom(":require" | "require" | ":use" | "use")
        ) && i > 0
            && matches!(toks[i - 1].0, Tok::Open(b'('));
        i += 1;
        if !required {
            continue;
        }
        let mut depth = 1usize;
        let mut spec: Vec<&str> = Vec::new();
        let mut spec_depth = None;
        while i < toks.len() && depth > 0 {
            match toks[i].0 {
                Tok::Open(c) => {
                    depth += 1;
                    if c == b'[' && spec_depth.is_none() {
                        spec_depth = Some(depth);
                        spec.clear();
                    }
                }
                Tok::Close => {
                    if spec_depth == Some(depth) {
                        clojure_spec(&spec, &mut out);
                        spec_depth = None;
                    }
                    depth -= 1;
                }
                Tok::Atom(a) => match spec_depth {
                    Some(_) => spec.push(a),
                    None if depth == 1 && !a.starts_with(':') => {
                        out.push((a.to_owned(), a.to_owned()))
                    }
                    None => {}
                },
            }
            i += 1;
        }
    }
    out
}

fn clojure_spec(spec: &[&str], out: &mut Vec<(String, String)>) {
    let Some((ns, rest)) = spec.split_first() else {
        return;
    };
    out.push(((*ns).to_owned(), (*ns).to_owned()));
    let mut it = rest.iter().peekable();
    while let Some(&k) = it.next() {
        match k {
            ":as" | ":as-alias" => {
                if let Some(a) = it.next() {
                    out.push(((*a).to_owned(), (*ns).to_owned()));
                }
            }
            ":refer" => {
                while let Some(&&n) = it.peek() {
                    if n.starts_with(':') {
                        break;
                    }
                    out.push((n.to_owned(), (*ns).to_owned()));
                    it.next();
                }
            }
            _ => {}
        }
    }
}

pub fn clojure_ns_files(ns: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    let rel = ns.replace('.', "/").replace('-', "_");
    files
        .iter()
        .filter(|f| {
            let s = f.to_string_lossy();
            ["clj", "cljs", "cljc"].iter().any(|e| {
                let tail = format!("{rel}.{e}");
                s == tail || s.ends_with(&format!("/{tail}"))
            })
        })
        .cloned()
        .collect()
}

pub fn lisp_require(kind: Kind, line: &str, col: usize) -> Option<String> {
    static ELISP: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\(require\s+'([^\s()]+)").unwrap());
    static RACKET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?:\(require|^)\s+(?:[^"]*?\s)?("[^"]+"|[\w\-]+(?:/[\w\-]+)+)"#).unwrap()
    });
    static CLOJURE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"'?\[([\w\-]+(?:\.[\w\-]+)+)(?:\s|\])").unwrap());
    let (re, required) = match kind {
        Kind::EmacsLisp => (&*ELISP, true),
        Kind::Scheme => (&*RACKET, line.contains("(require")),
        _ => (&*CLOJURE, true),
    };
    if !required {
        return None;
    }
    let m = re
        .captures_iter(line)
        .filter_map(|c| c.get(1))
        .find(|m| m.start() <= col && col < m.end())?;
    let m = m.as_str();
    Some(match kind {
        Kind::EmacsLisp => format!("{m}.el"),
        Kind::Scheme if m.starts_with('"') => m.trim_matches('"').to_owned(),
        Kind::Scheme => format!("{m}.rkt"),
        _ => m.to_owned(),
    })
}

pub fn lisp_files(kind: Kind, dir: &Path, module: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    match kind {
        Kind::Clojure => clojure_ns_files(module, files),
        Kind::Scheme if module.starts_with('.') || !module.contains('/') => {
            lexical(&dir.join(module))
                .filter(|f| files.contains(f))
                .into_iter()
                .collect()
        }
        _ => files
            .iter()
            .filter(|f| {
                let s = f.to_string_lossy();
                s == module || s.ends_with(&format!("/{module}"))
            })
            .cloned()
            .collect(),
    }
}

pub fn scheme_foreign_branch(
    text: &str,
    line: usize,
    includes_here: impl Fn(&str) -> bool,
) -> bool {
    struct Open<'a> {
        head: Option<&'a str>,
        idx: usize,
        start: usize,
        own: Option<usize>,
        branch_of: Option<usize>,
        here: bool,
    }
    let code = String::from_utf8_lossy(&lex(Kind::Scheme, text).code).into_owned();
    let Some(at) = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .nth(line.saturating_sub(1))
    else {
        return false;
    };
    let at = at + text[at..].len() - text[at..].trim_start().len();
    let mut stack: Vec<Open> = Vec::new();
    let mut expands = 0;
    let mut branches: Vec<(usize, Range<usize>, bool)> = Vec::new();
    for (tok, pos) in tokens(&code) {
        match tok {
            Tok::Open(_) => {
                let branch_of = stack.last().and_then(|p| p.own.filter(|_| p.idx >= 1));
                stack.push(Open {
                    head: None,
                    idx: 0,
                    start: pos,
                    own: None,
                    branch_of,
                    here: false,
                });
            }
            Tok::Close => {
                let Some(f) = stack.pop() else { continue };
                if let Some(e) = f.branch_of {
                    branches.push((e, f.start..pos, f.here));
                }
                if let Some(p) = stack.last_mut() {
                    p.idx += 1;
                }
            }
            Tok::Atom(a) => {
                let Some(f) = stack.last_mut() else { continue };
                if f.idx == 0 && a == "cond-expand" {
                    f.own = Some(expands);
                    expands += 1;
                }
                let include = f.idx > 0 && matches!(f.head, Some("include" | "include-ci"));
                f.head = f.head.or(Some(a));
                f.idx += 1;
                if include
                    && a == "\"\""
                    && let Some(end) = text[pos + 1..].find('"')
                    && includes_here(&text[pos + 1..pos + 1 + end])
                    && let Some(b) = stack.iter_mut().rev().find(|o| o.branch_of.is_some())
                {
                    b.here = true;
                }
            }
        }
    }
    branches.iter().any(|(e, span, _)| {
        span.contains(&at)
            && (branches.iter()).any(|(o, other, here)| o == e && other != span && *here)
    })
}
