use super::*;

fn attr(code: &str, word: &str) -> Option<RustAttr> {
    let lines: Vec<String> = code.lines().map(str::to_owned).collect();
    let (line, start) = lines
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, l)| {
            l.match_indices(word)
                .find(|&(at, _)| {
                    let ident = |c: char| c.is_alphanumeric() || c == '_';
                    !l[..at].ends_with(ident) && !l[at + word.len()..].starts_with(ident)
                })
                .map(|(at, _)| (i, at))
        })
        .unwrap_or_else(|| panic!("no `{word}` in {code}"));
    rust_attribute(&lines, line, start, start + word.len())
}

#[test]
fn rust_attribute_tells_a_derive_an_attribute_macro_and_a_word_declared_nowhere() {
    use RustAttr::*;
    for (code, word, want) in [
        ("#[derive(Debug, serde::Serialize)]", "Debug", Some(Derive)),
        (
            "#[derive(Debug, serde::Serialize)]",
            "Serialize",
            Some(Derive),
        ),
        ("#[derive(Debug, serde::Serialize)]", "serde", Some(Nothing)),
        ("#[tokio::main]", "main", Some(Macro)),
        ("#[tokio::main]", "tokio", Some(Nothing)),
        ("#[test]", "test", Some(Macro)),
        ("#[allow(dead_code)]", "allow", Some(Nothing)),
        ("#[allow(dead_code)]", "dead_code", Some(Nothing)),
        ("#![allow(dead_code)]", "allow", Some(Nothing)),
        ("#[my_attr(doc(hidden))]", "hidden", Some(Nothing)),
        ("#[cfg(feature == x)]", "x", Some(Nothing)),
        ("#[doc = include_str!(\"a.md\")]", "include_str", None),
        (
            "#[cfg_attr(feature = \"x\", derive(Debug))]",
            "Debug",
            Some(Derive),
        ),
        ("#[cfg_attr(x, doc = '\"', allow(y))]", "y", Some(Nothing)),
        ("#[cfg_attr(x, doc = '\\'', allow(y))]", "y", Some(Nothing)),
        ("#[cfg_attr(x, doc = '\\\"', allow(y))]", "y", Some(Nothing)),
        (
            "#[cfg_attr(\n    // )) closes nothing\n    test,\n    derive(Debug)\n)]",
            "Debug",
            Some(Derive),
        ),
    ] {
        assert_eq!(attr(code, word), want, "{word} in {code}");
    }
}

#[test]
fn rust_proc_macro_reads_its_attribute_past_doc_comments() {
    let text = "#[proc_macro_attribute]\n/// Wraps the item.\npub fn main(a: TokenStream, i: TokenStream) -> TokenStream {\n    i\n}\n\npub fn test(x: u32) {}\n";
    assert!(rust_proc_macro(text, 3));
    assert!(!rust_proc_macro(text, 7));
}

#[test]
fn rust_field_at_is_a_line_directly_in_a_struct_or_a_struct_like_variant() {
    let text = "pub struct Pair<T>\nwhere\n    T: Copy,\n{\n// a note at column zero\n    #[serde(default)]\n\n    pub left: T,\n}\n\npub enum Shape {\n    Square {\n        side: u32,\n    },\n}\n\npub fn made() -> Pair<u32> {\n    Pair {\n        left: 1,\n    }\n}\n";
    assert!(
        rust_field_at(text, 8),
        "past a comment, an attribute, a blank line, `{{` and `where`"
    );
    assert!(rust_field_at(text, 13), "a struct-like variant's field");
    assert!(!rust_field_at(text, 19), "a literal's key");
    let field = Regex::new(&rust_field_pattern("left", false)).unwrap();
    assert!(field.is_match("    pub(crate) left: u32,"));
    assert!(!field.is_match("    left::call(),"));
}

#[test]
fn rust_variant_pattern_takes_each_form_of_a_variant_behind_its_attributes() {
    let variant = Regex::new(&rust_variant_pattern("Auto")).unwrap();
    for line in [
        "    Auto,",
        "    Auto(u32),",
        "    Auto {",
        "    Auto = 1,",
        "    #[default] Auto,",
        "    Auto",
    ] {
        assert!(variant.is_match(line), "{line}");
    }
    for line in ["    Auto == x", "    Auto => 1,", "Auto,"] {
        assert!(!variant.is_match(line), "{line}");
    }
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no `{needle}`"))
        + 1
}

#[test]
fn rust_scope_items_leaves_to_the_next_lookup_what_another_rule_may_answer() {
    let items = |text: &str, at: &str, word: &str| {
        rust_scope_items(text, line_of(text, at), word, RustNamespace::Other)
    };
    let outline =
        "mod helper;\n\nfn helper() {}\n\npub fn run() {\n    helper::go();\n    helper()\n}\n";
    assert_eq!(
        items(outline, "    helper()", "helper"),
        [3],
        "`mod helper;` is its file"
    );
    let typed = "struct Gear;\n\nfn inner() {\n    use crate::other::Gear;\n}\n\npub fn run() {\n    Gear::new();\n}\n\npub fn go() {\n    use crate::far::Gear;\n    Gear::new()\n}\n";
    let path = |line1: usize| rust_scope_items(typed, line1, "Gear", RustNamespace::Path);
    assert_eq!(path(8), [1], "a `use` in a block apart");
    assert_eq!(path(13), Vec::<usize>::new(), "a `use` in a block around");
    let used = "fn spin() {}\n\npub fn run() {\n    spin()\n}\n\nfn inner() {\n    use crate::other::spin;\n}\n\npub fn go() {\n    use crate::far::spin;\n    spin();\n}\n";
    assert_eq!(
        items(used, "    spin()", "spin"),
        [1],
        "a `use` in a block apart is none of the cursor's"
    );
    assert_eq!(
        items(used, "    spin();", "spin"),
        Vec::<usize>::new(),
        "a `use` around"
    );
    let nested = "pub fn outer() -> u32 {\n    fn helper() -> u32 {\n        1\n    }\n    {\n        fn helper() -> u32 {\n            2\n        }\n    }\n    helper()\n}\n";
    assert_eq!(
        items(nested, "    helper()", "helper"),
        Vec::<usize>::new(),
        "a namesake in a block of the function"
    );
    let literal = "fn helper() {}\n\nmod m {\n    fn helper() {}\n\n    pub fn f() -> &'static str {\n        let s = \"\nfn helper() {}\n\";\n        helper();\n        s\n    }\n}\n";
    assert_eq!(
        items(literal, "        helper();", "helper"),
        [4],
        "a string's lines are no block"
    );
}

#[test]
fn rust_glob_uses_reads_the_function_past_its_wrapped_header_and_not_a_string() {
    let text = "use self::Kind::*;\n\nconst DOC: &str = \"\nuse self::Mode::*;\n\";\n\npub fn f(\n    a: u32,\n) -> u32 {\n    use self::Shape::*;\n    Square\n}\n\npub fn g() {\n    Auto\n}\n";
    let RustGlobUses {
        paths_before_star,
        own_to_function,
    } = rust_glob_uses(text, line_of(text, "    Square"));
    assert_eq!(
        (paths_before_star, own_to_function),
        (vec![vec!["self".to_owned(), "Shape".to_owned()]], true)
    );
    let RustGlobUses {
        paths_before_star, ..
    } = rust_glob_uses(text, line_of(text, "    Auto"));
    assert_eq!(paths_before_star, [["self", "Kind"]]);
}

fn bound_at(text: &str, line1: usize, name: &str) -> Vec<usize> {
    bindings(Kind::Rust, text, line1, name)
        .iter()
        .map(|b| b.line1)
        .collect()
}

#[test]
fn rust_let_declares_reads_a_pattern_up_to_its_type_or_value() {
    let at = |line: &str, name: &str| rust_let_declares(line, line.rfind(name).unwrap(), name);
    assert!(at("    let crate::Wrap(x) = w;", "x"));
    assert!(at("    let Point { x: px, .. } = p;", "px"));
    assert!(
        !at("    let Point { x: px, .. } = p;", "x"),
        "a field in front of its `:`"
    );
    assert!(!at("    let crate::x::Wrap(y) = w;", "x"), "a path");
    assert!(!at("    let ($a, b) = p;", "a"), "a macro variable");
    assert!(!at("    let (a,", "a"), "a pattern going on past the line");
}

#[test]
fn rust_bindings_take_each_form_that_binds_a_name() {
    let text = "pub fn run(g: impl Fn(u32) -> u32, x: u32, o: Option<u32>) -> u32 {\n    if a && let Some(x) = o.take() {\n        x\n    }\n    match o {\n        Some(x) if x > 1 => x,\n        Some(y) if x == 1 => y,\n    };\n    g(match o { Some(x) => x, None => x });\n    let (\n        x,\n        y,\n    ) = (1, 2);\n    x\n}\n";
    assert_eq!(bound_at(text, 3, "x"), [2], "`&& let`");
    assert_eq!(
        bound_at(text, 3, "o"),
        [1],
        "the value an `if let` matches binds nothing"
    );
    assert_eq!(bound_at(text, 7, "x"), [1], "a guard binds nothing");
    assert_eq!(bound_at(text, 6, "x"), [6, 1], "an arm behind its guard");
    assert_eq!(
        bound_at(text, 9, "x"),
        [1],
        "a `match` on one line binds nothing of its own"
    );
    assert_eq!(
        bound_at(text, 14, "x"),
        Vec::<usize>::new(),
        "a `let` pattern going on below"
    );
    let header = "pub fn f(x: u32, v: Vec<u32>) -> u32 {\n    if v.iter().any(|x| *x > 1) {\n        x\n    } else {\n        0\n    }\n}\n\npub fn g(x: u32, v: Vec<u32>) -> u32 {\n    v.iter().for_each(|x| {\n        x;\n    });\n    x\n}\n\nmacro_rules! m {\n    ($e:expr, x) => {\n        x\n    };\n}\n";
    assert_eq!(
        bound_at(header, 3, "x"),
        [1],
        "a closure in a block's header binds only in its own body"
    );
    assert_eq!(
        bound_at(header, 11, "x"),
        [10],
        "a closure whose body is the block"
    );
    assert_eq!(bound_at(header, 13, "x"), [9]);
    assert_eq!(
        bound_at(header, 18, "x"),
        Vec::<usize>::new(),
        "a macro's rule"
    );
    assert_eq!(
        bound_at(header, 13, "self"),
        Vec::<usize>::new(),
        "`self` is none"
    );
}

#[test]
fn rust_method_at_names_what_a_fn_sits_in_and_how_far_its_pub_reaches() {
    let text = "pub trait Tr {\n    fn req(&self);\n}\n\nimpl<T> Tr for Vec<T> {\n    fn req(&self) {}\n}\n\nimpl X {\n    pub(crate) fn own(&self) {}\n}\n\nimpl const Tr for Y {\n    fn req(&self) {}\n}\n\nimpl<T>\n    Wrapped for T {\n    fn wr(&self) {}\n}\n\nfn outer() {\n    fn inner() {}\n}\n\nmacro_rules! methods {\n    () => {\n        fn mac(&self) {}\n    };\n}\n\nmod m {\n    pub fn free() {}\n}\n";
    let lines: Vec<&str> = text.lines().collect();
    let at = |needle: &str| {
        rust_method_at(&lines, line_of(text, needle)).map(|m| (m.owner, m.vis, m.owner_line1))
    };
    use RustOwner::*;
    assert_eq!(
        at("    fn req(&self);"),
        Some((Trait("Tr".into()), RustVis::Pub, 1))
    );
    assert_eq!(
        at("    fn req(&self) {}"),
        Some((ImplOf("Tr".into()), RustVis::Private, 5))
    );
    assert_eq!(at("pub(crate) fn own"), Some((Inherent, RustVis::Crate, 9)));
    assert_eq!(lines[13], "    fn req(&self) {}");
    assert_eq!(
        rust_method_at(&lines, 14).map(|m| m.owner),
        Some(Unreadable),
        "`impl const Tr for`"
    );
    assert_eq!(
        at("fn wr"),
        Some((Unreadable, RustVis::Private, 17)),
        "a header wrapped before its `for`"
    );
    assert_eq!(
        at("fn mac"),
        Some((Unreadable, RustVis::Private, 27)),
        "a macro's body"
    );
    assert_eq!(at("fn inner"), None, "nested in a function");
    assert_eq!(at("fn free"), None, "in a `mod` block");
}

#[test]
fn cargo_reach_follows_the_lock_from_a_package_it_lists() {
    let lock = "version = 3\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\n \"mid\",\n \"serde 1.0.1\",\n]\n\n[[package]]\nname = \"mid\"\nversion = \"1.0.0\"\ndependencies = [\"low 2.0.0 (registry+https://github.com/rust-lang/crates.io-index)\"]\n\n[[package]]\nname = \"low\"\nversion = \"2.0.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.1\"\n\n[[package]]\nname = \"far\"\nversion = \"1.0.0\"\n";
    let CargoReach {
        reached,
        every_locked,
    } = cargo_reach(lock, "app").unwrap();
    assert_eq!(reached, ["app", "mid", "serde", "low"]);
    assert_eq!(every_locked.len(), 5);
    assert_eq!(every_locked[1].registry_dir, "mid-1.0.0");
    assert!(
        cargo_reach(lock, "absent").is_none(),
        "a package the lock does not list"
    );
}

#[test]
fn rust_stability_reads_the_attributes_past_a_doc_example() {
    let lines = [
        "#[stable(feature = \"x\", since = \"1.0.0\")]",
        "/// ```",
        "/// let v = f();",
        "/// ```",
        "pub fn f() {}",
        "",
        "#[unstable(feature = \"y\", issue = \"1\")]",
        "pub fn g() {}",
        "",
        "pub fn h() {}",
    ];
    assert!(rust_stability(&lines, 5));
    assert!(rust_stability(&lines, 8));
    assert!(!rust_stability(&lines, 10));
}

#[test]
fn rust_return_type_is_the_type_behind_the_last_arrow_outside_brackets() {
    let ret = |sig: &str| rust_return_type(&[sig], 0);
    assert_eq!(
        ret("fn f() -> Wagon where T: Copy {").as_deref(),
        Some("Wagon")
    );
    assert_eq!(
        ret("fn f(g: impl Fn() -> u8) -> Wagon {").as_deref(),
        Some("Wagon")
    );
    assert_eq!(ret("fn f(g: impl Fn() -> u8) {"), None);
    assert_eq!(
        ret("fn make<F>(g: F) -> Wagon where F: Fn() -> Sled {").as_deref(),
        Some("Wagon")
    );
    assert_eq!(ret("fn f<F>(g: F) where F: Fn() -> Sled {"), None);
    assert_eq!(
        ret("fn f() -> Somewhere where T: Copy {").as_deref(),
        Some("Somewhere")
    );
    assert_eq!(
        ret("fn f<F: Fn() -> u8>(g: F) -> Wagon {").as_deref(),
        Some("Wagon")
    );
}

#[test]
fn rust_type_name_strips_what_a_method_call_derefs_through() {
    let name = |w: &str| rust_type_name(w).map(|t| (t.path.join("::"), t.name));
    for written in [
        "&'a mut Wagon",
        "Box<Wagon>",
        "Rc<Wagon>",
        "Arc<Wagon>",
        "mut Wagon",
        "Wagon<u8>",
    ] {
        assert_eq!(
            name(written),
            Some((String::new(), "Wagon".to_owned())),
            "{written}"
        );
    }
    assert_eq!(
        name("crate::carts::Wagon"),
        Some(("crate::carts".to_owned(), "Wagon".to_owned()))
    );
    for written in [
        "u32",
        "(Wagon, Sled)",
        "[Wagon]",
        "dyn Roll",
        "impl Roll",
        "fn() -> Wagon",
        "Vec<Wagon>",
        "Option<Wagon>",
        "String",
    ] {
        assert_eq!(name(written), None, "{written}");
    }
}

#[test]
fn rust_self_type_is_the_impl_around_and_none_in_a_trait() {
    let text = "impl<T> Roll for Wagon<T> {\n    fn roll(&self) {\n        self.go();\n    }\n}\n\ntrait Spin {\n    fn spin(&self) {\n        self.go();\n    }\n}\n";
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        rust_self_type(&lines, 2).map(|t| (t.written, t.impl_line0)),
        Some(("Wagon".to_owned(), 0))
    );
    assert!(rust_self_type(&lines, 8).is_none());
    assert_eq!(
        rust_impl_type("impl<T: Copy> crate::Roll for &Wagon<T> {").as_deref(),
        Some("Wagon")
    );
}

#[test]
fn rust_generic_finds_a_parameter_of_any_item() {
    for item in [
        "fn f<Wagon>() {}",
        "impl<Wagon> X {}",
        "struct S<Wagon>;",
        "enum E<Wagon> {}",
        "union U<Wagon> {}",
        "trait T<Wagon> {}",
        "type A<Wagon> = u8;",
    ] {
        assert!(rust_generic(item, "Wagon"), "{item}");
    }
    assert!(!rust_generic("fn f(w: Vec<Wagon>) {}", "Wagon"));
}

#[test]
fn rust_holds_reads_a_written_type_or_a_whole_literal_or_call() {
    let holds = |lines: &[&str], name: &str| rust_holds(lines, lines.len() - 1, name);
    use RustHolds::*;
    assert_eq!(
        holds(&["    let w: Wagon = make();"], "w"),
        Some(Type("Wagon".into()))
    );
    assert_eq!(
        holds(&["    let w = Wagon { n: 1 };"], "w"),
        Some(Literal("Wagon".into()))
    );
    assert_eq!(holds(&["    let w = Wagon { n: 1 }.done();"], "w"), None);
    assert_eq!(
        holds(&["    let w = Wagon::new(1);"], "w"),
        Some(AssocCall {
            written_type: "Wagon".into(),
            function: "new".into()
        })
    );
    assert_eq!(holds(&["    let w = Wagon::new(1).unwrap();"], "w"), None);
    assert_eq!(holds(&["    let w = make(1).unwrap();"], "w"), None);
    assert_eq!(
        rust_holds(&["    let w = make(", "        1,", "    );"], 0, "w"),
        Some(Call(vec!["make".into()]))
    );
    assert_eq!(
        holds(
            &["fn f(w: HashMap<Box<dyn Fn() -> u8>, Wagon>, x: u32) {"],
            "w"
        ),
        Some(Type("HashMap<Box<dyn Fn() -> u8>, Wagon>".into())),
        "an arrow closes no bracket"
    );
    assert_eq!(
        holds(&["    v.iter().map(|a, w: &Wagon| w.roll())"], "w"),
        Some(Type("&Wagon".into()))
    );
    assert_eq!(
        holds(&["fn f(", "    w: Wagon,"], "w"),
        Some(Type("Wagon".into()))
    );
    assert_eq!(
        holds(&["    call(w: Wagon)"], "w"),
        None,
        "no signature around"
    );
}

#[test]
fn rust_struct_field_is_directly_in_the_struct() {
    let text = "pub struct Wagon {\n    pub load: Load,\n    inner: Inner,\n}\n\npub enum Cargo {\n    Load: u8,\n}\n";
    let lines: Vec<&str> = text.lines().collect();
    let field = |decl: usize, word: &str| {
        rust_struct_field(&lines, decl, word).map(|f| (f.line0, f.written_type))
    };
    assert_eq!(field(0, "load"), Some((1, "Load".to_owned())));
    assert_eq!(field(0, "missing"), None);
    assert_eq!(field(5, "Load"), None, "an enum has no fields");
}
