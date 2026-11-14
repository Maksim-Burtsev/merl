use regex::Regex;
use std::sync::LazyLock;

pub struct OutsideValue {
    pub written: String,
    pub names: Vec<String>,
}

pub fn outside_value(line: &str, name: &str) -> Option<OutsideValue> {
    static LITERAL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(concat!(
            r#"^(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|-?\d[\w.]*"#,
            r"|\[[^\[\]]*\]|<[\w\s,?]*>\s*\[\]|\[[\w\s:,?]*\]\(\))$"
        ))
        .expect("the pattern is valid")
    });
    static CALL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(new\s+)?((?:\\?(?:\w+\\)*|(?:\w+::)*)(\w+))\s*(?:<([\w\s,:?]*)>)?\s*\(((?:[^()]|\([^()]*\))*)\)$",
        )
        .expect("the pattern is valid")
    });
    let bind = Regex::new(&format!(
        r"(?:^|[^\w$.]){}\s*=\s*([^=].*?)\s*;?\s*$",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let written = bind.captures(line)?.get(1)?.as_str().trim().to_owned();
    if LITERAL.is_match(&written) {
        return Some(OutsideValue {
            written,
            names: Vec::new(),
        });
    }
    let call = CALL.captures(&written)?;
    let callee = call.get(3)?.as_str();
    if callee == "new" {
        return None;
    }
    let constructs = call.get(1).is_some() || callee.starts_with(char::is_uppercase);
    if !constructs && !call.get(5)?.as_str().trim().is_empty() {
        return None;
    }
    let words = |s: &str| {
        s.split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.starts_with(|c: char| c.is_alphabetic() || c == '_'))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let mut names = words(call.get(2)?.as_str());
    if !constructs {
        let types = call.get(4).map(|t| words(t.as_str())).unwrap_or_default();
        names.extend(
            types
                .into_iter()
                .filter(|w| w.starts_with(char::is_uppercase)),
        );
    }
    Some(OutsideValue { written, names })
}

pub fn declares_extension(text: &str, line1: usize, word: &str) -> bool {
    static OWNER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:(?:public|private|fileprivate|internal|@\w+)\s+)*extension\b")
            .expect("the pattern is valid")
    });
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    let receiver_typed = Regex::new(&format!(
        r"\b(?:fun|val|var)\s+(?:<[^>]*>\s*)?[\w.<>?, *]+\.{}\b",
        regex::escape(word)
    ))
    .expect("an escaped name keeps the pattern valid");
    let head = lines[at..lines.len().min(at + 3)].join(" ");
    if receiver_typed.is_match(lines[at])
        || lines[at].contains(&format!(".prototype.{word}"))
        || Regex::new(r"[(,]\s*this\s+\w")
            .expect("the pattern is valid")
            .is_match(&head)
    {
        return true;
    }
    let indent = |l: &str| l.len() - l.trim_start().len();
    let own = indent(lines[at]);
    lines[..at]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < own)
        .is_some_and(|l| OWNER.is_match(l.trim_start()))
}

#[cfg(test)]
mod tests {
    use super::{declares_extension, outside_value};

    #[test]
    fn an_extension_member_is_told_from_a_class_member() {
        let swift = "extension String {\n    func shout() -> String { self }\n}\n";
        assert!(declares_extension(swift, 2, "shout"));
        let dart = "extension Loud on String {\n  String shout() => this;\n}\n";
        assert!(declares_extension(dart, 2, "shout"));
        assert!(declares_extension(
            "fun String.shout() = this\n",
            1,
            "shout"
        ));
        let cs = "    public static StringBuilder Shout(\n        this StringBuilder sb) => sb;\n";
        assert!(declares_extension(cs, 1, "Shout"));
        let class = "struct Basket {\n    func append(_ i: Int) {}\n}\n";
        assert!(!declares_extension(class, 2, "append"));
    }

    fn names(line: &str, name: &str) -> Option<Vec<String>> {
        outside_value(line, name).map(|v| v.names)
    }

    #[test]
    fn literals_and_outside_constructors_are_read_and_the_rest_is_not() {
        let none: Vec<String> = Vec::new();
        assert_eq!(
            names(r#"  const text = "a,b";"#, "text"),
            Some(none.clone())
        );
        assert_eq!(names("    var out = [Int]()", "out"), Some(none.clone()));
        assert_eq!(names("  final out = <int>[];", "out"), Some(none.clone()));
        assert_eq!(names("  var seen = [];", "seen"), Some(none.clone()));
        assert_eq!(names("  let n = 0", "n"), Some(none));
        assert_eq!(
            names("  auto seen = std::vector<int>();", "seen"),
            Some(vec!["std".to_owned(), "vector".to_owned()])
        );
        assert_eq!(
            names("    val out = mutableListOf<Int>()", "out"),
            Some(vec!["mutableListOf".to_owned(), "Int".to_owned()])
        );
        assert_eq!(
            names(r"    $out = new \ArrayObject();", "$out"),
            Some(vec!["ArrayObject".to_owned()])
        );
        assert_eq!(names("    val b = requireNotNull(basket)", "b"), None);
        assert_eq!(names("    val b = Basket.make()", "b"), None);
        assert_eq!(names("    const x = [a][0];", "x"), None);
        assert_eq!(names("    if (x == []) {", "x"), None);
        assert_eq!(names("    const t = other.text", "t"), None);
        assert_eq!(names("        List<int> nums = new();", "nums"), None);
        let built = "        var told = new StringBuilder().Append(name);";
        assert_eq!(names(built, "told"), None);
    }
}
