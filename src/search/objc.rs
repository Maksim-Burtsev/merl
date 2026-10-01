//! Objective-C, read as part of the C kind (#417): the lines that declare a class, a protocol, a
//! method, a property or an `NS_ENUM`, and which files read the SDK's frameworks.

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
/// - a property, `@property (copy) NSString *name;`, a block's `(^completion)` included;
/// - `typedef NS_ENUM(NSInteger, Status)` and its kin: the name after the comma.
pub fn objc_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*@(?:interface|protocol)\s+{w}\s*(?:[:<{{]|//|/\*|$)"),
        format!(r"{METHOD}\s*(?:[^{{;]*?[\s)])?{w}\s*:"),
        format!(r"{METHOD}\s*{w}\s*(?:[;{{]|//|/\*|\b[A-Z_]{{2}}|$)"),
        objc_property(word),
        format!(
            r"^\s*typedef\s+NS_(?:ENUM|OPTIONS|CLOSED_ENUM|ERROR_ENUM)\s*\([^,()]*,\s*{w}\s*\)"
        ),
    ]
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
/// `System/Library/Frameworks` of the SDK, or the project's `Pods/`.
pub fn objc_root(root: &Path) -> bool {
    root.ends_with("System/Library/Frameworks") || root.ends_with("Pods")
}
