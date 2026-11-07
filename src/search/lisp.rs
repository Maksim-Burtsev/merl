use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::{Binding, Kind, Value, lexical, word_chars};

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
            if let Some(record) = word
                .strip_prefix("->")
                .or_else(|| word.strip_prefix("map->"))
            {
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

fn blank(code: &mut [u8], r: Range<usize>) {
    for c in &mut code[r] {
        if *c != b'\n' {
            *c = b' ';
        }
    }
}

fn string_step(b: &[u8], i: usize) -> (usize, bool) {
    match b[i] {
        b'\\' if b.get(i + 1) != Some(&b'\n') => ((i + 2).min(b.len()), true),
        b'"' => (i + 1, false),
        _ => (i + 1, true),
    }
}

fn block_step(b: &[u8], i: usize, block: usize) -> (usize, usize) {
    if b[i..].starts_with(b"|#") {
        (i + 2, block - 1)
    } else if b[i..].starts_with(b"#|") {
        (i + 2, block + 1)
    } else {
        (i + 1, block)
    }
}

fn char_end(b: &[u8], i: usize) -> usize {
    let lead = match b[i] {
        b'\\' => 1,
        b'#' => 2,
        _ => 1 + usize::from(b.get(i + 1) == Some(&b'\\')),
    };
    let mut i = (i + lead + usize::from(b.get(i + lead) != Some(&b'\n'))).min(b.len());
    while i < b.len() && b[i].is_ascii_alphabetic() && b[i - 1].is_ascii_alphabetic() {
        i += 1;
    }
    i
}

fn here_string_end(b: &[u8], i: usize) -> usize {
    let line_end =
        |from: usize| (b[from..].iter().position(|&c| c == b'\n')).map_or(b.len(), |n| from + n);
    let mut end = line_end(i);
    let term = &b[i + 3..end];
    while end < b.len() {
        let next = line_end(end + 1);
        if &b[end + 1..next] == term {
            return next;
        }
        end = next;
    }
    b.len()
}

fn lex(kind: Kind, text: &str) -> Lexed {
    lex_keeping(kind, text, false)
}

fn char_literal(kind: Kind, b: &[u8], i: usize) -> bool {
    match kind {
        Kind::Clojure => b[i] == b'\\',
        Kind::Scheme | Kind::CommonLisp => b[i..].starts_with(b"#\\"),
        Kind::EmacsLisp => b[i] == b'?' && (i == 0 || delimiter(b[i - 1])),
        _ => false,
    }
}

fn comment_form(b: &[u8], i: usize, depth: usize) -> bool {
    depth == 0 && b[i..].starts_with(b"(comment") && b.get(i + 8).is_none_or(|&c| delimiter(c))
}

pub fn lisp_code(kind: Kind, text: &str) -> Vec<u8> {
    lex_keeping(kind, text, true).code
}

fn lex_keeping(kind: Kind, text: &str, comment_forms: bool) -> Lexed {
    let b = text.as_bytes();
    let clojure = kind == Kind::Clojure;
    let hashed = matches!(kind, Kind::Scheme | Kind::CommonLisp);
    let mut code = b.to_vec();
    let mut literal = vec![false];
    let (mut depth, mut i) = (0usize, 0usize);
    let (mut string, mut block, mut here) = (false, 0usize, 0usize);
    let mut ignored: Option<(usize, usize)> = None;
    let mut datum = false;
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            literal.push(string || block > 0 || ignored.is_some() || i < here);
            i += 1;
            continue;
        }
        let at = |s: &[u8]| b[i..].starts_with(s);
        let start = i;
        if i < here {
            i += 1;
            blank(&mut code, start..i);
        } else if string {
            (i, string) = string_step(b, i);
            if string {
                blank(&mut code, start..i);
            }
        } else if block > 0 {
            (i, block) = block_step(b, i, block);
            blank(&mut code, start..i);
        } else if kind == Kind::Scheme && at(b"#<<") {
            here = here_string_end(b, i);
            datum = false;
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
        } else if char_literal(kind, b, i) {
            i = char_end(b, i);
            blank(&mut code, start..i);
            datum = false;
        } else {
            if clojure && !comment_forms && ignored.is_none() && comment_form(b, i, depth) {
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

pub(super) fn lisp_form_encloses(kind: Kind, text: &str, form_line0: usize, line0: usize) -> bool {
    let start = |n: usize| {
        text.split_inclusive('\n')
            .take(n)
            .map(str::len)
            .sum::<usize>()
    };
    let code = lex(kind, text).code;
    let Some(code) = code.get(start(form_line0)..start(line0)) else {
        return false;
    };
    let Some(open) = code.iter().position(|c| matches!(c, b'(' | b'[' | b'{')) else {
        return false;
    };
    let mut depth = 0usize;
    !code[open..].iter().any(|c| {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ => return false,
        }
        depth == 0
    })
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
                let end = b[i + 1..]
                    .iter()
                    .position(|&c| c == b'"')
                    .map_or(b.len(), |n| i + n + 1);
                out.push((Tok::Atom("\"\""), i));
                i = end;
            }
            _ if delimiter(c) || b"'`~@#^".contains(&c) => {}
            _ => {
                let end = b[i..]
                    .iter()
                    .position(|&c| delimiter(c))
                    .map_or(b.len(), |n| i + n);
                out.extend(code.get(i..end).map(|a| (Tok::Atom(a), i)));
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
    Method,
    Literals,
    Rule,
    Pattern,
    Match,
    MatchPat,
    Quasi,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent fact about the form"
)]
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
    values: bool,
    literal: bool,
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
    "let-values",
    "let*-values",
    "letrec-values",
    "flet",
    "labels",
    "cl-flet",
    "cl-flet*",
    "cl-labels",
    "macrolet",
    "cl-macrolet",
    "symbol-macrolet",
    "cl-symbol-macrolet",
    "do",
    "do*",
    "when-let*",
    "if-let*",
    "and-let*",
    "pcase-let*",
    "pcase-let",
    "when-let",
    "if-let",
];
const MATCH_EXPRS: &[&str] = &["?", "app", "regexp", "pregexp", "=", "struct", "$"];
const MATCH_DATA: &[&str] = &["==", "not", "quote", "quasiquote"];
const SEQUENTIAL: &[&str] = &[
    "let*",
    "let*-values",
    "when-let*",
    "if-let*",
    "and-let*",
    "pcase-let*",
    "when-let",
    "if-let",
];
const RECURSIVE: &[&str] = &["letrec", "letrec*", "letrec-values", "labels", "cl-labels"];
const TESTS: &[&str] = &["when-let*", "if-let*", "and-let*", "when-let", "if-let"];
const METHODS: &[&str] = &[
    "reify",
    "deftype",
    "defrecord",
    "extend-type",
    "extend-protocol",
    "proxy",
];
const LAMBDAS: &[&str] = &[
    "lambda",
    "opt-lambda",
    "destructuring-bind",
    "cl-destructuring-bind",
    "multiple-value-bind",
    "cl-multiple-value-bind",
    "seq-let",
    "with-slots",
    "receive",
];
const LOOPS: &[&str] = &["dolist", "dotimes", "cl-dolist", "cl-dotimes", "guard"];
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
const GENERICS: &[&str] = &["defgeneric", "cl-defgeneric"];
const DEFINES: &[&str] = &[
    "define",
    "define/contract",
    "define/public",
    "define/private",
    "define/override",
];

fn racket_for(h: &str) -> bool {
    ["for", "for*"].contains(&h) || h.starts_with("for/") || h.starts_with("for*/")
}

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

fn open_frame<'a>(
    kind: (Kind, bool),
    stack: &mut [Frame<'a>],
    (c, prefix): (u8, u8),
    scopes: &mut Vec<(usize, Option<usize>)>,
    pos: usize,
    found: usize,
) -> Frame<'a> {
    let role = stack
        .last_mut()
        .map_or(Role::Plain, |p| child_role(kind, p, c, prefix));
    let (sequential, entry) = match stack.last() {
        Some(p) if role == Role::Entries => {
            let h = p.head.unwrap_or("").to_ascii_lowercase();
            let sequential = SEQUENTIAL.contains(&h.as_str());
            let parallel = !sequential
                && !racket_for(&h)
                && !RECURSIVE.contains(&h.as_str())
                && !["do", "do*"].contains(&h.as_str());
            (sequential, parallel.then_some(found))
        }
        Some(p) if p.role == Role::Entries && p.sequential => (true, Some(found)),
        Some(p)
            if p.role == Role::Entries
                || (p.values && role == Role::Destructure)
                || role == Role::Quasi =>
        {
            (false, p.entry)
        }
        _ => (false, None),
    };
    let recursive = match stack.last() {
        Some(p) if role == Role::Entries => {
            RECURSIVE.contains(&p.head.unwrap_or("").to_ascii_lowercase().as_str())
        }
        Some(p) => p.role == Role::Letfn || (p.recursive && (p.role == Role::Entries || p.values)),
        None => false,
    };
    let values = match stack.last() {
        Some(p) if role == Role::Entries => p.head.is_some_and(|h| h.ends_with("-values")),
        Some(p) => p.role == Role::Entries && p.values,
        None => false,
    };
    let literal = matches!(role, Role::Rule | Role::Pattern | Role::Destructure)
        && stack.last().is_some_and(|p| p.literal);
    let tests = match stack.last() {
        Some(p) if role == Role::Entries => {
            TESTS.contains(&p.head.unwrap_or("").to_ascii_lowercase().as_str())
        }
        Some(p) => p.role == Role::Entries && p.tests,
        None => false,
    };
    let scope = match (role, &*stack) {
        (Role::Plain | Role::Arity | Role::Method | Role::Rule | Role::Match, _) | (_, []) => {
            scopes.push((pos, None));
            scopes.len() - 1
        }
        (_, [.., o, p]) if p.role == Role::Plain && p.head == Some("match-define") => o.scope,
        (_, [.., p]) => p.scope,
    };
    Frame {
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
        values,
        literal,
        entry,
    }
}

fn atom<'a>(
    (kind, racket): (Kind, bool),
    stack: &mut [Frame<'a>],
    (a, prefix): (&'a str, u8),
    pos: usize,
    name: &str,
    found: &mut Vec<(usize, usize, usize)>,
) -> bool {
    let class = match stack.len() {
        n if n >= 2 && kind == Kind::Scheme => (stack[n - 2].head)
            .filter(|h| CLASSES.contains(h))
            .map(|_| stack[n - 2].scope),
        _ => None,
    };
    let pending = stack.last().is_some_and(|f| f.entry.is_some());
    let body = match &*stack {
        _ if kind != Kind::Scheme || stack.iter().any(|f| f.role == Role::Rule) => None,
        [.., o, d]
            if d.idx == 1
                && d.role == Role::Plain
                && d.head.is_some_and(|h| {
                    DEFINES.contains(&h) || (racket && h == "match-define" && prefix != b'\'')
                }) =>
        {
            Some(o.scope)
        }
        [.., o, d, p]
            if d.idx == 1
                && ((p.role == Role::Define && p.idx == 0)
                    || (p.role == Role::Params && d.head == Some("define-values"))) =>
        {
            Some(o.scope)
        }
        _ => None,
    };
    let Some(f) = stack.last_mut() else {
        return false;
    };
    if f.idx == 0 {
        f.head = Some(a);
    }
    let head = f.head.unwrap_or("");
    let looping = f.role == Role::Plain
        && f.idx > 0
        && match kind {
            Kind::CommonLisp => head.eq_ignore_ascii_case("loop"),
            Kind::EmacsLisp => head == "cl-loop",
            _ => false,
        };
    let named = kind == Kind::Scheme && f.role == Role::Plain && f.idx == 1 && head == "let";
    let own = f.role == Role::Plain
        && match kind {
            Kind::Clojure => f.idx == 2 && ["as->", "catch"].contains(&head),
            Kind::Scheme => f.idx == 1 && head == "lambda",
            Kind::EmacsLisp => f.idx == 1 && head == "condition-case",
            _ => false,
        };
    let bound = body.is_some()
        || match f.role {
            Role::Pairs => f.idx.is_multiple_of(2),
            Role::Entries => !f.tests,
            Role::Params | Role::Destructure => true,
            Role::Define | Role::Pattern => f.idx >= 1,
            Role::First | Role::Arity => f.idx == 0,
            Role::Plain => (looping && f.loop_var) || named || own,
            Role::Match | Role::MatchPat => in_pattern(f) && !b"'`#".contains(&prefix),
            Role::Quasi => prefix == b',',
            _ => false,
        };
    let same = match kind {
        Kind::CommonLisp => a.eq_ignore_ascii_case(name),
        _ => a == name,
    };
    let literal = f.role == Role::Literals && same;
    if bound && !f.literal && binds(a) && same {
        let visible = match (f.recursive, pending) {
            _ if own && head == "condition-case" => usize::MAX,
            (true, _) => 0,
            (false, true) => usize::MAX,
            (false, false) => pos,
        };
        found.push((body.unwrap_or(f.scope), pos, visible));
    }
    if looping {
        f.loop_var = !f.loop_var && LOOP_VARS.iter().any(|k| k.eq_ignore_ascii_case(a));
    }
    match f.role {
        Role::Pairs => f.loop_var = f.idx.is_multiple_of(2) && a == ":let",
        Role::Entries => f.loop_var = a.starts_with(':'),
        _ => {}
    }
    f.named |= named;
    if let Some(scope) = class.filter(|_| f.idx == 0 && CLAUSES.contains(&a)) {
        f.role = Role::Entries;
        f.scope = scope;
    }
    f.idx += 1;
    if literal && let [.., rules, _] = stack {
        rules.literal = true;
    }
    true
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
    let mut scopes: Vec<(usize, Option<usize>)> = Vec::new();
    let mut found: Vec<(usize, usize, usize)> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let from_top = code.get(starts[top]..).unwrap_or_default();
    let kind = (kind, kind == Kind::Scheme && racket_match(&text, &code));
    for (tok, pos) in tokens(from_top) {
        let prefix = from_top.as_bytes()[pos.saturating_sub(1)];
        let pos = pos + starts[top];
        if pos > line_end {
            break;
        }
        match tok {
            Tok::Open(c) => {
                let frame =
                    open_frame(kind, &mut stack, (c, prefix), &mut scopes, pos, found.len());
                stack.push(frame);
            }
            Tok::Close => {
                let Some(f) = stack.pop() else { break };
                let generic = f.role == Role::Params
                    && (stack.last().and_then(|p| p.head))
                        .is_some_and(|h| GENERICS.contains(&h.to_ascii_lowercase().as_str()));
                if generic
                    || matches!(
                        f.role,
                        Role::Plain | Role::Arity | Role::Method | Role::Rule | Role::Match
                    )
                {
                    scopes[f.scope].1.get_or_insert(pos);
                }
                if let Some(from) = f.entry.filter(|_| f.tests && f.idx < 2) {
                    found.truncate(from);
                }
                if let Some(from) = f.entry.filter(|_| settles(&f)) {
                    for e in found[from..].iter_mut().filter(|e| e.2 == usize::MAX) {
                        e.2 = pos;
                    }
                }
                if let Some(p) = stack.last().filter(|p| {
                    p.idx == 2
                        && p.head
                            .is_some_and(|h| ["if-let", "if-let*", "if-some"].contains(&h))
                }) {
                    scopes.push((scopes[p.scope].0, Some(pos)));
                    let until = scopes.len() - 1;
                    for e in found.iter_mut().filter(|e| e.0 == p.scope) {
                        e.0 = until;
                    }
                }
                if let Some(p) = stack
                    .last()
                    .filter(|p| p.idx == 2 && p.head == Some("condition-case"))
                {
                    for e in found
                        .iter_mut()
                        .filter(|e| e.0 == p.scope && e.2 == usize::MAX)
                    {
                        e.2 = pos;
                    }
                }
                match stack.last_mut() {
                    Some(p) => p.idx += 1,
                    None => break,
                }
            }
            Tok::Atom(a) => {
                if !atom(kind, &mut stack, (a, prefix), pos, name, &mut found) {
                    break;
                }
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
            line1: line_of(&text, pos),
            value: Value::Unknown,
        })
        .into_iter()
        .collect()
}

fn in_pattern(f: &Frame) -> bool {
    let h = f.head.unwrap_or("");
    match f.role {
        Role::Match => f.idx == 0,
        _ => f.idx >= 1 && !MATCH_DATA.contains(&h) && (f.idx > 1 || !MATCH_EXPRS.contains(&h)),
    }
}

fn settles(f: &Frame) -> bool {
    f.role == Role::Entries || (f.role == Role::First && f.sequential)
}

fn racket_match(text: &str, code: &str) -> bool {
    let lang = text
        .lines()
        .any(|l| matches!(l.trim(), "#lang racket" | "#lang typed/racket"));
    if !lang && !code.contains("racket/match") {
        return false;
    }
    let mut stack: Vec<(Option<&str>, usize)> = Vec::new();
    for (tok, pos) in tokens(code) {
        match tok {
            Tok::Open(_) => stack.push((None, 0)),
            Tok::Close => {
                stack.pop();
                if let Some(p) = stack.last_mut() {
                    p.1 += 1;
                }
            }
            Tok::Atom(a) => {
                let quoted = code.as_bytes()[pos.saturating_sub(1)] == b'\'';
                let head = match &*stack {
                    [.., (Some(h), i), (None, 0)] => {
                        *i != 1 || !(h.starts_with("define") || ["lambda", "λ"].contains(h))
                    }
                    [(None, 0)] => true,
                    _ => false,
                };
                if ["match", "match-lambda", "match-define"].contains(&a) && !quoted && !head {
                    return false;
                }
                if let Some(f) = stack.last_mut() {
                    if f.1 == 0 {
                        f.0 = Some(a);
                    }
                    f.1 += 1;
                }
            }
        }
    }
    true
}

fn pattern_child((kind, racket): (Kind, bool), p: &Frame, prefix: u8) -> Option<Role> {
    let scheme = racket && p.role == Role::Plain;
    match (p.role, p.head.unwrap_or(""), p.idx) {
        (Role::First, _, 0) if prefix == b'`' && kind == Kind::EmacsLisp => Some(Role::Quasi),
        (Role::Quasi, ..) if prefix != b',' => Some(Role::Quasi),
        (Role::Match | Role::MatchPat, ..) if in_pattern(p) => Some(pattern(prefix)),
        (_, "match", 2..) | (_, "match-lambda", 1..) if scheme => Some(Role::Match),
        (_, "match-define", 1) if scheme => Some(pattern(prefix)),
        _ => None,
    }
}

fn pattern(prefix: u8) -> Role {
    match prefix {
        b'`' => Role::Quasi,
        b'\'' | b'#' => Role::Plain,
        _ => Role::MatchPat,
    }
}

fn child_role((kind, racket): (Kind, bool), p: &mut Frame, c: u8, prefix: u8) -> Role {
    if let Some(role) = pattern_child((kind, racket), p, prefix) {
        return role;
    }
    let clojure = kind == Kind::Clojure;
    let h = p.head.unwrap_or("");
    match p.role {
        Role::Pairs | Role::Entries if p.loop_var => {
            p.loop_var = false;
            return if p.role == Role::Pairs {
                Role::Pairs
            } else {
                Role::Plain
            };
        }
        Role::Pairs if p.idx.is_multiple_of(2) => return Role::Destructure,
        Role::Entries | Role::Letfn => return Role::First,
        Role::First if p.values && p.idx == 0 => return Role::Destructure,
        Role::Params | Role::Define if c == b'(' => return Role::First,
        Role::Params | Role::Define | Role::Destructure | Role::Pattern => {
            return Role::Destructure;
        }
        Role::Arity if p.idx == 0 && (c == b'[' || !clojure) => return Role::Params,
        Role::Method if p.idx == 1 && (c == b'[' || !clojure) => return Role::Params,
        Role::Rule if p.idx == 0 => return Role::Pattern,
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
        if METHODS.contains(&h) && p.idx >= 1 {
            match c {
                b'(' => return Role::Method,
                b'[' if p.idx == 2 && h.starts_with("def") => return Role::Params,
                _ => {}
            }
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
    if h == "syntax-rules" && p.idx >= 1 {
        return if p.idx == 1 {
            Role::Literals
        } else {
            Role::Rule
        };
    }
    if h == "case-lambda" && p.idx >= 1 {
        return Role::Arity;
    }
    if c != b'(' {
        return Role::Plain;
    }
    let h = h.to_ascii_lowercase();
    let h = h.as_str();
    match p.idx {
        1 if LETS.contains(&h) || racket_for(h) => Role::Entries,
        2 if h == "let" && p.named => Role::Entries,
        2 if racket_for(h) && h.contains("/fold") => Role::Entries,
        1 if h == "define-values" => Role::Params,
        n if n >= 2 && h == "handler-case" => Role::Method,
        1 if LAMBDAS.contains(&h) => Role::Params,
        1 if LOOPS.contains(&h) => Role::First,
        1 if DEFINES.contains(&h) => Role::Define,
        2 if DEFUNS.contains(&h) || GENERICS.contains(&h) => Role::Params,
        1 if h == ":method" => Role::Params,
        _ => Role::Plain,
    }
}

#[expect(
    clippy::naive_bytecount,
    reason = "the bytecount crate would be a dependency for one count of newlines"
)]
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
                        out.push((a.to_owned(), a.to_owned()));
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
        LazyLock::new(|| Regex::new(r"\(require\s+'([^\s()]+)").expect("the regex is a literal"));
    static RACKET: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?:\(require|^)\s+(?:[^"]*?\s)?("[^"]+"|[\w\-]+(?:/[\w\-]+)+)"#)
            .expect("the regex is a literal")
    });
    static CLOJURE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"'?\[([\w\-]+(?:\.[\w\-]+)+)(?:\s|\])").expect("the regex is a literal")
    });
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

pub fn scheme_renames(
    text: &str,
    line: usize,
    word: &str,
    own: bool,
    includes_here: impl Fn(&str) -> bool,
) -> bool {
    struct Open<'a> {
        head: Option<&'a str>,
        idx: usize,
    }
    let code = String::from_utf8_lossy(&lex(Kind::Scheme, text).code).into_owned();
    let mut starts = std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1));
    let Some(from) = starts.nth(line.saturating_sub(1)) else {
        return false;
    };
    let on_line = from..starts.next().unwrap_or(text.len());
    let mut stack: Vec<Open> = Vec::new();
    let (mut renamed, mut included) = (false, own);
    for (tok, pos) in tokens(&code) {
        match tok {
            Tok::Open(_) => stack.push(Open { head: None, idx: 0 }),
            Tok::Close => {
                if stack.pop().is_none() {
                    continue;
                }
                match stack.last_mut() {
                    Some(p) => p.idx += 1,
                    None if renamed && included => return true,
                    None => (renamed, included) = (false, own),
                }
            }
            Tok::Atom(a) => {
                let n = stack.len();
                renamed |= a == word
                    && on_line.contains(&pos)
                    && n >= 3
                    && stack[n - 1].idx == 1
                    && stack[n - 2].head == Some("rename")
                    && stack[n - 2].idx >= 2
                    && stack[..n - 2].iter().any(|o| o.head == Some("import"));
                let Some(f) = stack.last_mut() else { continue };
                included |= f.idx > 0
                    && matches!(f.head, Some("include" | "include-ci"))
                    && a == "\"\""
                    && (text.get(pos + 1..).and_then(|t| t.split_once('"')))
                        .is_some_and(|(inside, _)| includes_here(inside));
                f.head = f.head.or(Some(a));
                f.idx += 1;
            }
        }
    }
    false
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
    let Some(rest) = text.get(at..) else {
        return false;
    };
    let at = at + rest.len() - rest.trim_start().len();
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
                    && let Some((inside, _)) = text.get(pos + 1..).and_then(|t| t.split_once('"'))
                    && includes_here(inside)
                    && let Some(b) = stack.iter_mut().rev().find(|o| o.branch_of.is_some())
                {
                    b.here = true;
                }
            }
        }
    }
    branches.iter().any(|(e, span, own)| {
        span.contains(&at)
            && !own
            && (branches.iter()).any(|(o, other, here)| o == e && other != span && *here)
    })
}
