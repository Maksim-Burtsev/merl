use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

/// A method's head in column zero, `-` or `+` and the return type in its parentheses, a block
/// type's one level of nesting included. C has no line opening so: a wrapped expression is
/// indented.
const METHOD: &str = r"^[-+]\s*\((?:[^()]|\([^()]*\))*\)";

/// The lines that declare `word` in Objective-C, appended to the C kind's patterns:
/// - a class, `@interface Name : Super <P>`, and a protocol, `@protocol Name <P>` or alone on its
///   line. A category `(Slug)` or an extension `()` declares no class of the name, and neither do
///   `@implementation`, `@class Name;` and `@protocol Name;`;
/// - a method by any part of its selector, `- (T)find:(A)a inContext:(C)c` declaring `find` and
///   `inContext`, its declaration and the line of its definition alike, and one that takes no
///   argument, `+ (instancetype)shared`;
/// - a property, `@property (copy) NSString *name;`, a block's `(^completion)` included, and the
///   getter it names;
/// - `typedef NS_ENUM(NSInteger, Status)` and its kin: the name after the comma;
/// - a block type, and a `typedef` an `NS_`, `CF_` or `API_` macro follows.
pub fn objc_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*@(?:interface|protocol)\s+{w}\s*(?:[:<{{]|//|/\*|$)"),
        format!(r"{METHOD}\s*(?:[^{{;]*?[\s)])?{w}\s*:"),
        format!(r"{METHOD}\s*{w}\s*(?:[;{{]|//|/\*|\b[A-Z_]{{2}}|$)"),
        objc_property(word),
        // The getter a property names, `getter=isFinished`, which a message calls.
        format!(r"^\s*@property\s*\([^)]*\bgetter\s*=\s*{w}\b"),
        format!(
            r"^\s*typedef\s+NS_(?:ENUM|OPTIONS|CLOSED_ENUM|ERROR_ENUM)\s*\([^,()]*,\s*{w}\s*\)"
        ),
        // A block type, `typedef void (^Done)(NSError *error);`, and a type a macro follows,
        // `typedef NSString * Mode NS_TYPED_EXTENSIBLE_ENUM;`.
        format!(r"^\s*typedef\s+[^;{{]*\(\s*\^\s*{w}\s*\)"),
        format!(r"^\s*typedef\s+[^;{{(]*[\s*]{w}\s+(?:NS|CF|API)_\w*\s*(?:\([^()]*\))?\s*;"),
    ]
}

/// What `x.word` reaches in Objective-C (#417), as a C field is what `x.word` does: a property, the
/// getter it names, or a method that takes no argument, which dot syntax calls too
/// (`enumerator.nextObject`).
pub fn objc_member(word: &str) -> String {
    let w = regex::escape(word);
    format!(
        r"{}|^\s*@property\s*\([^)]*\bgetter\s*=\s*{w}\b|{METHOD}\s*{w}\s*(?:[;{{]|//|/\*|\b[A-Z_]{{2}}|$)",
        objc_property(word)
    )
}

pub fn objc_for_in(inside_for_parens: &str, name: &str) -> Option<usize> {
    let w = regex::escape(name);
    let re = Regex::new(&format!(r"^\s*[A-Za-z_][\w\s*<>,]*?[\s*>]({w})\s+in\b")).ok()?;
    Some(re.captures(inside_for_parens)?.get(1)?.start())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CDialect {
    COrCpp,
    ObjectiveC,
}
/// [`super::def_patterns`] as a file reads them (#417).
pub fn def_patterns_for(kind: super::Kind, word: &str, dialect: CDialect) -> Vec<String> {
    let mut p = super::def_patterns(kind, word);
    if kind == super::Kind::C && dialect == CDialect::COrCpp {
        p.truncate(p.len() - objc_patterns(word).len());
    }
    p
}

/// Whether a line a C-kind pattern of `word` matched is Objective-C's alone: an Objective-C
/// pattern matches it and no C or C++ one does. The regexes are built on the first line that
/// could be one.
pub fn objc_only(word: &str) -> impl Fn(&str) -> bool {
    let word = word.to_owned();
    let res = std::sync::OnceLock::new();
    move |line: &str| {
        let t = line.trim_start();
        let typedef = t.starts_with("typedef")
            && (t.contains('^') || ["NS_", "CF_", "API_"].iter().any(|m| t.contains(m)));
        if !(t.starts_with(['@', '-', '+']) || typedef) {
            return false;
        }
        let (objc, c): &(Regex, Regex) = res.get_or_init(|| {
            let c = def_patterns_for(super::Kind::C, &word, CDialect::COrCpp);
            let re = |p: &[String]| Regex::new(&p.join("|")).expect("escaped names compile");
            (re(&objc_patterns(&word)), re(&c))
        });
        objc.is_match(line) && !c.is_match(line)
    }
}

/// A property named `word`: what `self.word` reads, as a C field is what `x.word` does.
pub fn objc_property(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"^\s*@property\b[^;]*?(?:[\s*]{w}\s*(?:;|\b[A-Z_]{{2}}|$)|\(\s*\^\s*{w}\s*\))")
}

/// `D`'s Objective-C rows: a class and a protocol under their names, and a method from the line
/// of its definition, one that ends with no `;`, under the first part of its selector, as a C
/// prototype is left out and its definition listed.
pub const OBJC_TYPE_SYMBOL: &str =
    r"^\s*@(?:interface|protocol)\s+(?P<name>[A-Za-z_]\w*)\s*(?:[:<{]|//|/\*|$)";
pub const OBJC_METHOD_SYMBOL: &str =
    r"^[-+]\s*\((?:[^()]|\([^()]*\))*\)\s*(?P<name>[A-Za-z_]\w*)[^;]*$";

/// Whether the C-kind file at `path`, whose text is `text`, is Objective-C: a `.m` or `.mm`, or a
/// header with an `@interface`, `@protocol`, `@class` or `#import` line. Only such a file reads
/// the SDK's frameworks and CocoaPods' `Pods/`.
pub fn objc_file(path: &Path, text: impl FnOnce() -> String) -> bool {
    static MARK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^[ \t]*(?:@(?:interface|protocol|class)\b|#[ \t]*import\b)").unwrap()
    });
    match path.extension().and_then(|e| e.to_str()) {
        Some("m" | "mm") => true,
        Some("h") => MARK.is_match(&text()),
        _ => false,
    }
}

/// Whether `root`, a root outside the project of the C kind, is Objective-C's alone: a
/// `System/Library/Frameworks` of the SDK or the `Frameworks` an umbrella framework holds
/// (`Accelerate.framework/Frameworks`, where vImage is), or the project's `Pods/`.
pub fn objc_root(root: &Path) -> bool {
    root.ends_with("Pods") || objc_frameworks(root)
}

/// Whether `root` is a directory of frameworks, each one's headers in `X.framework/Headers`.
pub fn objc_frameworks(root: &Path) -> bool {
    root.ends_with("Frameworks")
        && root
            .to_string_lossy()
            .contains("/System/Library/Frameworks")
}

/// Where the head of a body, `head` as [`super::c_code`] reads it, names `name` as a parameter
/// of the Objective-C method it opens: the name after a part's type, `- (void)load:(NSString
/// *)name inContext:(Context *)ctx`. A method's parameters are its locals, as a C function's are.
pub fn objc_parameter(head: &str, name: &str) -> Option<usize> {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[-+]\s*\(").unwrap());
    let at = HEAD.find(head)?.start();
    let w = regex::escape(name);
    let re = Regex::new(&format!(r":\s*\((?:[^()]|\([^()]*\))*\)\s*({w})\b")).ok()?;
    let m = re.captures(&head[at..])?.get(1)?;
    Some(at + m.start())
}
