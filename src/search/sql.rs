use regex::Regex;

pub fn sql_cte(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"(?i)^\s*(?:\)\s*)?(?:WITH\s+(?:RECURSIVE\s+)?|,\s*)?{w}(?:\s*\([^()]*\))?\s+AS(?:\s+(?:NOT\s+)?MATERIALIZED)?\s*\(")
}
pub fn sql_cte_sees(text: &str, cte: usize, word: &str, line: usize, col: usize) -> Option<bool> {
    let start = |l: usize| -> usize { text.split_inclusive('\n').take(l - 1).map(str::len).sum() };
    let head = Regex::new(&sql_cte(word)).expect("an escaped name keeps the pattern valid");
    let Some(m) = text.lines().nth(cte - 1).and_then(|l| head.find(l)) else {
        return Some(false);
    };
    let (open, at, b) = (start(cte) + m.end() - 1, start(line) + col, text.as_bytes());
    let (mut first, mut depth, mut close, mut i) = (0, None::<usize>, None, 0);
    let (mut ended, mut after) = (false, 0_usize);
    while i < b.len() {
        let skip = match &b[i..] {
            [b'-', b'-', ..] => "\n",
            [b'/', b'*', ..] => "*/",
            [b'\'', ..] => "'",
            [b'(', ..] if close.is_some() => {
                after += 1;
                ""
            }
            [b')', ..] if close.is_some() => {
                if after == 0 {
                    ended = true;
                    break;
                }
                after -= 1;
                ""
            }
            [b'(', ..] if i >= open => {
                depth = Some(depth.map_or(1, |d| d + 1));
                ""
            }
            [b')', ..] if close.is_none() => {
                depth = depth.map(|d| d - 1);
                if depth == Some(0) {
                    close = Some(i);
                }
                ""
            }
            [b';', ..] if i < open => {
                first = i + 1;
                ""
            }
            [b';', ..] => {
                ended = true;
                break;
            }
            _ => "",
        };
        i += match skip {
            "" => 1,
            s => text
                .get(i + 1..)
                .and_then(|t| t.find(s))
                .map_or(b.len(), |n| n + 1 + s.len()),
        };
    }
    let recursive = Regex::new(r"(?i)\bWITH\s+RECURSIVE\b").expect("a valid pattern");
    let from = if text.get(first..open).is_some_and(|t| recursive.is_match(t)) {
        open
    } else {
        close.unwrap_or(i)
    };
    if at <= from {
        Some(false)
    } else {
        ended.then_some(at <= i)
    }
}
