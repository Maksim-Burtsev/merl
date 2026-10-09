use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

const METHOD: &str = r"^[-+]\s*\((?:[^()]|\([^()]*\))*\)";

pub fn objc_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    vec![
        format!(r"^\s*@(?:interface|protocol)\s+{w}\s*(?:[:<{{]|//|/\*|$)"),
        format!(r"{METHOD}\s*(?:[^{{;]*?[\s)])?{w}\s*:"),
        format!(r"{METHOD}\s*{w}\s*(?:[;{{]|//|/\*|\b[A-Z_]{{2}}|$)"),
        objc_property(word),
        format!(r"^\s*@property\s*\([^)]*\bgetter\s*=\s*{w}\b"),
        format!(
            r"^\s*typedef\s+NS_(?:ENUM|OPTIONS|CLOSED_ENUM|ERROR_ENUM)\s*\([^,()]*,\s*{w}\s*\)"
        ),
        format!(r"^\s*typedef\s+[^;{{]*\(\s*\^\s*{w}\s*\)"),
        format!(r"^\s*typedef\s+[^;{{(]*[\s*]{w}\s+(?:NS|CF|API)_\w*\s*(?:\([^()]*\))?\s*;"),
    ]
}

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
pub fn def_patterns_for(kind: super::Kind, word: &str, dialect: CDialect) -> Vec<String> {
    let mut p = super::def_patterns(kind, word);
    if kind == super::Kind::C && dialect == CDialect::COrCpp {
        p.truncate(p.len() - objc_patterns(word).len());
    }
    p
}

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

pub fn objc_property(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"^\s*@property\b[^;]*?(?:[\s*]{w}\s*(?:;|\b[A-Z_]{{2}}|$)|\(\s*\^\s*{w}\s*\))")
}

pub const OBJC_TYPE_SYMBOL: &str =
    r"^\s*@(?:interface|protocol)\s+(?P<name>[A-Za-z_]\w*)\s*(?:[:<{]|//|/\*|$)";
pub const OBJC_METHOD_SYMBOL: &str =
    r"^[-+]\s*\((?:[^()]|\([^()]*\))*\)\s*(?P<name>[A-Za-z_]\w*)[^;]*$";

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

pub fn objc_root(root: &Path) -> bool {
    root.ends_with("Pods") || objc_frameworks(root)
}

pub fn objc_frameworks(root: &Path) -> bool {
    root.ends_with("Frameworks")
        && root
            .to_string_lossy()
            .contains("/System/Library/Frameworks")
}

pub fn objc_parameter(head: &str, name: &str) -> Option<usize> {
    static HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[-+]\s*\(").unwrap());
    let at = HEAD.find(head)?.start();
    let w = regex::escape(name);
    let re = Regex::new(&format!(r":\s*\((?:[^()]|\([^()]*\))*\)\s*({w})\b")).ok()?;
    let m = re.captures(&head[at..])?.get(1)?;
    Some(at + m.start())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declares(word: &str, line: &str) -> bool {
        Regex::new(&objc_patterns(word).join("|"))
            .unwrap()
            .is_match(line)
    }

    #[test]
    fn objective_c_declares_classes_protocols_methods_properties_and_types() {
        for (word, line) in [
            ("Repo", "@interface Repo : NSObject <Store>"),
            ("Store", "@protocol Store <NSObject>"),
            ("Store", "@protocol Store"),
            ("find", "- (void)find:(A *)a inContext:(C *)c;"),
            ("inContext", "- (void)find:(A *)a inContext:(C *)c {"),
            ("shared", "+ (instancetype)shared;"),
            ("handler", "- (void (^)(int))handler {"),
            ("name", "@property (copy) NSString *name;"),
            (
                "completion",
                "@property (copy) void (^completion)(BOOL ok);",
            ),
            ("isFinished", "@property (getter=isFinished) BOOL finished;"),
            ("Status", "typedef NS_ENUM(NSInteger, Status) {"),
            ("Done", "typedef void (^Done)(NSError *error);"),
            ("Mode", "typedef NSString * Mode NS_TYPED_EXTENSIBLE_ENUM;"),
        ] {
            assert!(declares(word, line), "{line}");
        }
        for (word, line) in [
            ("Repo", "@interface Repo (Slug)"),
            ("Repo", "@interface Repo ()"),
            ("Repo", "@implementation Repo"),
            ("Repo", "@class Repo;"),
            ("Store", "@protocol Store;"),
            ("count", "    - (int)count;"),
        ] {
            assert!(!declares(word, line), "{line}");
        }
    }

    #[test]
    fn dot_syntax_reaches_a_property_its_getter_or_a_method_without_arguments() {
        let member =
            |word: &str, line: &str| Regex::new(&objc_member(word)).unwrap().is_match(line);
        assert!(member("name", "@property (nonatomic) id name;"));
        assert!(member("isOn", "@property (getter=isOn) BOOL on;"));
        assert!(member("nextObject", "- (id)nextObject;"));
        assert!(!member("find", "- (void)find:(A *)a;"));
    }

    #[test]
    fn a_c_or_cpp_file_reads_no_objective_c_pattern() {
        let all = super::super::def_patterns(super::super::Kind::C, "Repo");
        let c = def_patterns_for(super::super::Kind::C, "Repo", CDialect::COrCpp);
        let objc = def_patterns_for(super::super::Kind::C, "Repo", CDialect::ObjectiveC);
        assert_eq!(c.len(), all.len() - objc_patterns("Repo").len());
        assert_eq!(objc, all);
        let c = Regex::new(&c.join("|")).unwrap();
        assert!(!c.is_match("@interface Repo : NSObject"));
        assert!(objc_only("Repo")("@interface Repo : NSObject"));
        assert!(!objc_only("x")("int x;"));
        let both = "- (void)done { [self stop]; } done;";
        let c_done = def_patterns_for(super::super::Kind::C, "done", CDialect::COrCpp);
        assert!(declares("done", both) && Regex::new(&c_done.join("|")).unwrap().is_match(both));
        assert!(!objc_only("done")(both));
    }

    #[test]
    fn a_method_symbol_is_read_off_its_definition() {
        let re = Regex::new(OBJC_METHOD_SYMBOL).unwrap();
        assert_eq!(
            &re.captures("- (void)load:(id)x {").unwrap()["name"],
            "load"
        );
        assert!(!re.is_match("- (void)load:(id)x;"));
    }

    #[test]
    fn objective_c_files_and_roots() {
        assert!(objc_file(Path::new("a.m"), String::new));
        assert!(objc_file(Path::new("a.mm"), String::new));
        assert!(objc_file(Path::new("a.h"), || {
            "#import <Foundation/Foundation.h>\n".to_owned()
        }));
        assert!(objc_file(Path::new("a.h"), || "@class Repo;\n".to_owned()));
        assert!(!objc_file(Path::new("a.h"), || "int x;\n".to_owned()));
        assert!(!objc_file(Path::new("a.c"), || "@interface X\n".to_owned()));
        let sdk = "/Xcode.app/SDKs/MacOSX.sdk/System/Library/Frameworks";
        assert!(objc_root(Path::new("/p/Pods")));
        assert!(objc_root(Path::new(sdk)));
        assert!(objc_frameworks(Path::new(&format!(
            "{sdk}/Accelerate.framework/Frameworks"
        ))));
        assert!(!objc_frameworks(Path::new(&format!(
            "{sdk}/Foundation.framework/Headers"
        ))));
        assert!(!objc_root(Path::new("/usr/include")));
    }

    #[test]
    fn a_methods_parameters_are_the_names_after_each_parts_type() {
        let head = "- (void)load:(NSString *)name inContext:(void (^)(int))ctx ";
        assert_eq!(objc_parameter(head, "ctx"), head.rfind("ctx"));
        assert_eq!(objc_parameter(head, "name"), head.find("name"));
        assert_eq!(objc_parameter(head, "load"), None);
        assert_eq!(objc_parameter("void f(int ctx) ", "ctx"), None);
    }
}
