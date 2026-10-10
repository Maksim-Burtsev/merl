use super::*;

#[test]
fn a_jsdoc_type_is_one_plain_name() {
    for (braced, want) in [
        ("Steward", Some("Steward")),
        ("lib.Steward", Some("lib.Steward")),
        ("Steward[]", Some("Steward[]")),
        ("Array<Steward>", Some("Array<Steward>")),
        ("?Steward", Some("Steward")),
        ("!Steward", Some("Steward")),
        ("Steward=", Some("Steward")),
        ("Steward|null", Some("Steward")),
        ("undefined | Steward", Some("Steward")),
        ("Steward|Keeper", None),
        ("Map<string, Steward>", None),
        ("{a: number}", None),
        ("function(): Steward", None),
        ("*", None),
        ("any", None),
    ] {
        assert_eq!(jsdoc_type(braced).as_deref(), want, "{braced}");
    }
    let typedefs = jsdoc_typedefs("/** @typedef {{a: {b: number}}} Inline */", "Inline");
    assert_eq!(
        typedefs
            .iter()
            .map(|t| t.written_type.as_str())
            .collect::<Vec<_>>(),
        ["{a: {b: number}}"],
        "the braces of a type are balanced"
    );
}

#[test]
fn a_jsdoc_comment_types_only_the_declaration_it_stands_on() {
    assert_eq!(
        behind_doc(Kind::TsJs, "  /** @type {Steward} */ let solo = load();"),
        " let solo = load();"
    );
    for (kind, line) in [
        (Kind::TsJs, "/* note */ call();"),
        (Kind::Rust, "/* note */ let solo = 1;"),
    ] {
        assert_eq!(behind_doc(kind, line), line);
    }
    assert_eq!(
        jsdoc_binding(
            "/*\n * @type {Steward}\n */\nconst plain = load();",
            4,
            "plain"
        ),
        None,
        "a `/*` comment is no JSDoc"
    );
    assert_eq!(
        jsdoc_binding("/** @type {Steward} */\nconst typed = load();", 2, "typed").as_deref(),
        Some("Steward")
    );
}

#[test]
fn a_typedef_owns_the_properties_of_its_block() {
    let text = "/**\n * @typedef {Object} First\n * @prop {number} one\n * @typedef {Object} Second\n * @property {number} two\n */\n * @property {number} after\n/**\n * @typedef {Function} Callback\n * @property {number} three\n */";
    let lines: Vec<&str> = text.lines().collect();
    let found = |line0: usize, name: &str| {
        jsdoc_properties(&lines, line0, name).map(|b| b.iter().map(|b| b.line1).collect::<Vec<_>>())
    };
    assert_eq!(found(1, "one"), Some(vec![3]), "`@prop` is `@property`");
    assert_eq!(found(1, "two"), Some(vec![]), "the next typedef's");
    assert_eq!(found(3, "two"), Some(vec![5]));
    assert_eq!(found(3, "after"), Some(vec![]), "past the block's end");
    assert_eq!(found(8, "three"), None, "a `Function` declares no fields");
    assert_eq!(jsdoc_owner(&lines, 4), Some(3));
    assert_eq!(jsdoc_owner(&lines, 6), None, "past the block's end");
    assert_eq!(jsdoc_owner(&lines, 9), None);
}
