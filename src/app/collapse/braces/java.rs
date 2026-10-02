use super::{K, Model};

const CONTROL: &[&str] = &[
    "if",
    "for",
    "while",
    "switch",
    "catch",
    "synchronized",
    "try",
    "return",
    "throw",
    "assert",
    "else",
    "case",
    "yield",
    "do",
    "instanceof",
];
const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "static",
    "final",
    "abstract",
    "synchronized",
    "native",
    "strictfp",
    "default",
    "sealed",
    "non-sealed",
    "transient",
    "volatile",
];
const BLOCK_AFTER: &[&str] = &["else", "try", "finally", "do", "static", "synchronized"];

#[derive(Clone, Copy, PartialEq)]
enum Body {
    Code,
    Class,
    Type,
    Data,
}

impl Model<'_, '_> {
    pub(super) fn java(&mut self, lines: &[String]) {
        let n = self.toks.len();
        let mut kind = vec![Body::Code; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut imports: Option<(usize, usize)> = None;
        let mut k = 0;
        while k < n {
            let parent = stack.last().map(|&p| kind[p]);
            if self.tx(k) == "import" && self.first[k] && stack.is_empty() {
                let end = (k..n).find(|&j| self.punct(j, ";")).unwrap_or(n - 1);
                let span = (self.toks[k].line, self.toks[end].end);
                imports = self.join_run(imports, span, lines);
                k = end + 1;
                continue;
            }
            self.end_run(imports.take());
            if self.closer(k) {
                stack.pop();
            } else if self.opener(k) {
                stack.push(k);
                match self.tx(k) {
                    "{" => {
                        let (body, fold, def) = self.java_brace(k, parent.unwrap_or(Body::Class));
                        kind[k] = body;
                        if fold {
                            self.add(self.toks[k].line, self.closing(k), def);
                        }
                    }
                    "(" if self.java_call(k, parent.unwrap_or(Body::Class)) => {
                        self.add(self.toks[k].line, self.closing(k), false)
                    }
                    _ => {}
                }
            }
            k += 1;
        }
        self.end_run(imports);
    }

    fn params_before(&self, p: usize) -> usize {
        let mut j = p;
        while j > 0 && (self.toks[j].kind == K::Word || matches!(self.tx(j), "." | ",")) {
            if self.tx(j) == "throws" {
                return j - 1;
            }
            j -= 1;
        }
        p
    }

    fn java_brace(&mut self, k: usize, up: Body) -> (Body, bool, bool) {
        let block = (Body::Code, true, false);
        let class = (Body::Class, true, true);
        let Some(p) = k.checked_sub(1) else {
            return block;
        };
        let p = self.params_before(p);
        let t = self.toks[p];
        match t.text {
            ")" => {
                let Some(o) = self.back[p] else {
                    return block;
                };
                let mut name = o.saturating_sub(1);
                if self.tx(name) == ">" {
                    let mut angle = 0i32;
                    while name > 0 {
                        match self.tx(name) {
                            ">" => angle += 1,
                            "<" => angle -= 1,
                            _ => {}
                        }
                        if angle == 0 {
                            break;
                        }
                        name -= 1;
                    }
                    name = name.saturating_sub(1);
                }
                let before = self.before(name);
                if before == "new" || before == "record" {
                    return class;
                }
                if self.tx(name) == "switch" {
                    return (Body::Code, false, false);
                }
                if self.toks[name].kind == K::Word
                    && !CONTROL.contains(&self.tx(name))
                    && self.constructor(name)
                {
                    let start = self.member_start(name);
                    self.add(self.toks[start].line, self.closing(k), true);
                    return (Body::Code, false, false);
                }
                let method = self.toks[name].kind == K::Word && !CONTROL.contains(&self.tx(name));
                (Body::Code, true, method)
            }
            ">" if self.before(p) == "-" => (Body::Code, true, true),
            "]" | "=" | "," | "(" => (Body::Data, false, false),
            "{" if up == Body::Data => (Body::Data, false, false),
            _ if t.kind == K::Word && BLOCK_AFTER.contains(&t.text) => block,
            _ if t.kind == K::Word || t.text == ">" => {
                let mut j = p;
                while j > 0 && !matches!(self.tx(j), ";" | "{" | "}") {
                    match self.tx(j) {
                        "class" | "record" => return class,
                        "interface" | "enum" | "@interface" => return (Body::Type, false, false),
                        _ => {}
                    }
                    j = if self.closer(j) {
                        self.back[j].unwrap_or(j)
                    } else {
                        j
                    };
                    j -= 1;
                }
                class
            }
            _ => block,
        }
    }

    fn constructor(&self, name: usize) -> bool {
        let Some(q) = name.checked_sub(1) else {
            return true;
        };
        let t = self.toks[q];
        matches!(t.text, ";" | "{" | "}")
            || MODIFIERS.contains(&t.text)
            || t.text.starts_with('@')
            || (t.text == ")" && self.back[q].is_some_and(|o| self.before(o).starts_with('@')))
    }

    fn member_start(&self, name: usize) -> usize {
        let mut s = name;
        while s > 0 {
            let q = s - 1;
            let t = self.toks[q];
            if MODIFIERS.contains(&t.text) || t.text.starts_with('@') {
                s = q;
            } else if t.text == ")"
                && let Some(o) = self.back[q]
                && self.before(o).starts_with('@')
            {
                s = o - 1;
            } else if t.text == "." || (t.kind == K::Word && self.before(q) == ".") {
                s = q;
            } else {
                break;
            }
        }
        s
    }

    fn java_call(&self, k: usize, up: Body) -> bool {
        let Some(p) = k.checked_sub(1) else {
            return false;
        };
        let t = self.toks[p];
        if t.text == ">" {
            return self.before(p) != "-";
        }
        if t.kind != K::Word || CONTROL.contains(&t.text) {
            return false;
        }
        if t.text.starts_with('@') {
            return true;
        }
        let q = self.before(p);
        if q == "record" {
            return false;
        }
        if up == Body::Code || up == Body::Data {
            return true;
        }
        let after = self.pair[k].map_or("", |c| self.tx(c + 1));
        let declares = matches!(after, "{" | "throws" | ";" | "default");
        !(declares
            && !matches!(
                q,
                "." | "new" | "=" | "(" | "," | "->" | "?" | ":" | "return"
            ))
    }
}
