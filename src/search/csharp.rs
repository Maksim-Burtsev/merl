//! Where a C# declaration stands (#355): a member of a type, private or not, or a local of a
//! method's body, which the search by name must not offer from anywhere else.

use regex::Regex;

use super::*;

/// The keyword (`class`, `struct`, `interface`, `enum`, `record`) and the name of the C# type
/// the line declares.
pub fn cs_type_decl(line: &str) -> Option<(String, String)> {
    static TYPE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^\u{feff}?\s*(?:\[[^\]]*\]\s*)*",
            r"(?:(?:public|private|protected|internal|file|static|sealed|abstract|partial|readonly",
            r"|ref|unsafe|new)\s+)*",
            r"(class|struct|interface|enum|record)\b(?:\s+(?:class|struct)\b)?\s+@?(\w+)"
        ))
        .unwrap()
    });
    let c = TYPE.captures(line)?;
    Some((c[1].to_owned(), c[2].to_owned()))
}

/// Where the declaration of `word` on 1-based `line` of a C# `text` stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsPlace {
    /// Directly in the body of the type `owner`; `private` when only that type reaches it: a
    /// member of a `class`, `struct` or `record` written `private` or with no access modifier,
    /// as C# defaults it.
    Member { owner: String, private: bool },
    /// Inside the body of a method, a constructor, an accessor, a local function or a lambda: a
    /// local, seen only between 1-based lines `from` and `to`, those of the member around it.
    Local { from: usize, to: usize },
    /// At the top of a file or of a namespace.
    Top,
}

/// Where the declaration of `word` on 1-based `line` of `text` stands (#355): the lines above
/// indented less, as [`qualified`] walks them, told apart by whether they declare a type.
pub fn cs_place(text: &str, line: usize, word: &str) -> CsPlace {
    let lines: Vec<&str> = text.lines().collect();
    let Some(target) = line.checked_sub(1).and_then(|k| lines.get(k)) else {
        return CsPlace::Top;
    };
    let namespace = |l: &str| l.trim_start().starts_with("namespace ");
    let Some(first) = cs_enclosing(&lines, line - 1) else {
        return CsPlace::Top;
    };
    if namespace(lines[first]) {
        return CsPlace::Top;
    }
    if let Some((keyword, owner)) = cs_type_decl(lines[first]) {
        return CsPlace::Member {
            private: matches!(keyword.as_str(), "class" | "struct" | "record")
                && cs_private(target, word),
            owner,
        };
    }
    // A body: the member it belongs to is the outermost line under a type or a namespace.
    let mut member = first;
    while let Some(e) = cs_enclosing(&lines, member) {
        if namespace(lines[e]) || cs_type_decl(lines[e]).is_some() {
            break;
        }
        member = e;
    }
    CsPlace::Local {
        from: member + 1,
        to: cs_block_end(&lines, member) + 1,
    }
}

/// Whether a member line reads as private: `private` (not `private protected`), or no access
/// modifier at all, unless it implements an interface's member explicitly (`int I.Name`), which
/// the interface reaches.
fn cs_private(line: &str, word: &str) -> bool {
    static ACCESS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^\s*(?:\[[^\]]*\]\s*)*(?:(?:private|static|readonly|const|sealed|abstract|virtual",
            r"|override|partial|async|extern|unsafe|new|volatile|event|required|fixed|ref)\s+)*",
            r"(?:public|protected|internal|file)\b"
        ))
        .unwrap()
    });
    let explicit =
        Regex::new(&format!(r"\w\.{}\b", regex::escape(word))).is_ok_and(|re| re.is_match(line));
    !ACCESS.is_match(line) && !explicit
}

/// The 0-based index of the line that 0-based `k` of `lines` stands inside: the nearest line above
/// indented less, past blank lines, comments, preprocessor lines, a lone `{` and the tail of a
/// header wrapped over several lines.
pub(super) fn cs_enclosing(lines: &[&str], k: usize) -> Option<usize> {
    let depth = indent(lines[k]);
    (0..k).rev().find(|&i| {
        let t = lines[i].trim();
        !(t.is_empty()
            || comment(Kind::CSharp, t)
            || t.starts_with(['#', '{', ')', ':', '['])
            || t.starts_with("where ")
            || indent(lines[i]) >= depth)
    })
}

/// The 0-based index of the last line of the block the header on 0-based `k` opens: the `}` back
/// at its indent, else the line before the next one there.
fn cs_block_end(lines: &[&str], k: usize) -> usize {
    let depth = indent(lines[k]);
    for (j, l) in lines.iter().enumerate().skip(k + 1) {
        let t = l.trim();
        if t.is_empty() || comment(Kind::CSharp, t) || t.starts_with('#') || indent(l) > depth {
            continue;
        }
        if t.starts_with(['{', ')']) {
            continue;
        }
        return if t.starts_with('}') { j } else { j - 1 };
    }
    lines.len().saturating_sub(1)
}
