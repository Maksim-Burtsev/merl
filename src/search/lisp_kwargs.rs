use super::Kind;
use super::lisp::{Tok, delimiter, lisp_code, open_before, tokens};

type Form<'t, 'a> = &'t [(Tok<'a>, usize)];

pub fn clojure_kwarg_call(text: &str, at: usize) -> Option<(usize, usize)> {
    let code = String::from_utf8(lisp_code(Kind::Clojure, text)).ok()?;
    let b = code.as_bytes();
    let colon = at.checked_sub(1).filter(|&c| b.get(c) == Some(&b':'))?;
    let open = open_before(b, colon).filter(|&o| b[o] == b'(')?;
    let starts_form = |i: usize| i == 0 || delimiter(b[i - 1]);
    let inner = code.get(open + 1..colon)?;
    if !starts_form(colon)
        || !starts_form(open)
        || inner.contains(['\'', '`', '~', '@', '^', '#'])
        || threaded(&code, open)
    {
        return None;
    }
    let toks = tokens(inner);
    let forms = top_forms(&toks);
    match forms.first() {
        Some([(Tok::Atom(head), i)])
            if !head.starts_with([':', '"']) && !head.starts_with(|c: char| c.is_ascii_digit()) =>
        {
            Some((open + 1 + i, forms.len() - 1))
        }
        _ => None,
    }
}

fn threaded(code: &str, open: usize) -> bool {
    let b = code.as_bytes();
    let parent = open_before(b, open).filter(|&p| b[p] == b'(');
    let head = parent.and_then(|p| code.get(p + 1..open)).map(tokens);
    matches!(head.as_deref(), Some([(Tok::Atom(h), _), ..])
        if h.ends_with("->") || h.ends_with("->>") || ["doto", ".."].contains(h))
}

fn top_forms<'t, 'a>(toks: Form<'t, 'a>) -> Vec<Form<'t, 'a>> {
    let mut out = Vec::new();
    let (mut depth, mut from) = (0usize, 0usize);
    for (n, (t, _)) in toks.iter().enumerate() {
        match t {
            Tok::Open(_) => {
                if depth == 0 {
                    from = n;
                }
                depth += 1;
            }
            Tok::Close if depth == 0 => break,
            Tok::Close => {
                depth -= 1;
                if depth == 0 {
                    out.extend(toks.get(from..=n));
                }
            }
            Tok::Atom(_) if depth == 0 => out.extend(toks.get(n..=n)),
            Tok::Atom(_) => {}
        }
    }
    out
}

pub fn clojure_keys_line(
    text: &str,
    decl_line1: usize,
    name: &str,
    word: &str,
    index: usize,
) -> Option<usize> {
    let code = String::from_utf8(lisp_code(Kind::Clojure, text)).ok()?;
    let start: usize = (code.split_inclusive('\n'))
        .take(decl_line1.checked_sub(1)?)
        .map(str::len)
        .sum();
    let rest = code.get(start..)?;
    let toks = tokens(rest);
    let params = match &toks[..] {
        [
            (Tok::Open(b'('), _),
            (Tok::Atom("defn" | "defn-"), _),
            (Tok::Atom(n), _),
            more @ ..,
        ] if *n == name => match more {
            [(Tok::Atom("\"\""), _), (Tok::Open(b'['), _), p @ ..]
            | [(Tok::Open(b'['), _), p @ ..] => top_forms(p),
            _ => return None,
        },
        _ => return None,
    };
    let amp = params
        .iter()
        .position(|f| matches!(f, [(Tok::Atom("&"), _)]))?;
    let [[(Tok::Open(b'{'), _), map @ .., (Tok::Close, _)]] = params.get(amp + 1..)? else {
        return None;
    };
    let map = top_forms(map);
    let keys = map
        .iter()
        .position(|f| matches!(f, [(Tok::Atom(":keys"), _)]))?;
    let [(Tok::Open(b'['), _), names @ .., (Tok::Close, _)] = map.get(keys + 1)? else {
        return None;
    };
    let (_, at) = names
        .iter()
        .find(|(t, _)| matches!(t, Tok::Atom(a) if *a == word))?;
    (index >= amp && (index - amp).is_multiple_of(2))
        .then(|| decl_line1 + rest.get(..*at).map_or(0, |r| r.matches('\n').count()))
}
