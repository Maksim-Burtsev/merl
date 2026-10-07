use super::*;

fn styled(text: &str, class: &str) -> Vec<usize> {
    rules(text)
        .into_iter()
        .filter(|r| r.classes.iter().any(|c| c == class))
        .map(|r| r.line)
        .collect()
}

#[test]
fn a_rule_styles_the_classes_of_its_last_compound() {
    let css = ".btn {}\n.btn:hover,\n.card > .btn {}\na.btn.active {}\n.btn::before {}\n\
               .btn .icon {}\n.btn + .label {}\n:not(.btn) .x {}\n.col-#{$i}.btn {}\n\
               a { color: red; width: .5em; }\n/* .btn { */\n// .btn {\n";
    assert_eq!(styled(css, "btn"), [1, 2, 3, 4, 5, 9]);
    assert_eq!(styled(css, "icon"), [6]);
    assert!(styled(css, "5em").is_empty());
    assert!(styled(css, "col-").is_empty());
}

#[test]
fn an_ampersand_composes_with_the_rule_it_sits_in() {
    let scss = ".card {\n  &__title {}\n  &-body {\n    &-wide {}\n  }\n  &.active {}\n  &:hover {}\n\
                @media (x) {\n    &--dark {}\n  }\n}\n@keyframes k {\n  from {}\n}\n";
    assert_eq!(styled(scss, "card__title"), [2]);
    assert_eq!(styled(scss, "card-body-wide"), [4]);
    assert_eq!(styled(scss, "active"), [6]);
    assert_eq!(styled(scss, "card"), [1]);
    assert_eq!(styled(scss, "card--dark"), [9]);
}

#[test]
fn a_class_attribute_is_read_in_markup_and_jsx_only() {
    fn at<'a>(line: &'a str, word: &str, jsx: bool) -> Option<(Attr, &'a str)> {
        attr_at(line, line.find(word).unwrap(), jsx).map(|(a, r)| (a, &line[r]))
    }
    let tsx = r#"<b className="btn btn-primary" id="main">"#;
    assert_eq!(at(tsx, "primary", true), Some((Attr::Class, "btn-primary")));
    assert_eq!(at(tsx, "main", true), Some((Attr::Id, "main")));
    assert_eq!(
        at("<b className={'a b'}>", "b'", true),
        Some((Attr::Class, "b"))
    );
    assert_eq!(
        at("  className=\"wrap\"", "wrap", true),
        Some((Attr::Class, "wrap"))
    );
    assert_eq!(
        at("<p class:active={on}>", "active", false),
        Some((Attr::Class, "active"))
    );
    assert_eq!(
        at(r#"const id = "main""#, "main", true),
        None,
        "an assignment is no attribute"
    );
    assert_eq!(at(r#"let id="main""#, "main", true), None);
    assert_eq!(
        at(r#"<p class = "x">"#, "x\"", false),
        Some((Attr::Class, "x"))
    );
    assert_eq!(
        at(r#"<a href="src/a.css#top">"#, "a.css", false),
        Some((Attr::Href, "src/a.css#top"))
    );
}

#[test]
fn a_stylesheet_cursor_says_which_lookup_applies() {
    let at = |line: &str, word: &str, less: bool| sheet_at(line, line.find(word).unwrap(), less);
    let s = |n: &str| n.to_owned();
    assert_eq!(
        at("  color: var(--brand, #000);", "brand", false),
        Some(Sheet::Custom(s("--brand")))
    );
    assert_eq!(
        at("  a: v.$primary;", "primary", false),
        Some(Sheet::Var(Some(s("v")), s("primary")))
    );
    assert_eq!(
        at(".b { @include m.btn-v(1); }", "btn-v", false),
        Some(Sheet::Mixin(Some(s("m")), s("btn-v")))
    );
    assert_eq!(
        at("  c: tint-color($c, 1%);", "tint", false),
        Some(Sheet::Function(None, s("tint-color")))
    );
    assert_eq!(
        at("  @extend %shared;", "shared", false),
        Some(Sheet::Placeholder(s("shared")))
    );
    assert_eq!(
        at("  @extend .btn;", "btn", false),
        Some(Sheet::Class(s("btn")))
    );
    assert_eq!(
        at("  animation: spin 1s;", "spin", false),
        Some(Sheet::Keyframes(s("spin")))
    );
    assert_eq!(
        at("  border: @w solid @brand;", "brand", true),
        Some(Sheet::LessVar(s("brand")))
    );
    assert_eq!(at("  border: @w solid @brand;", "brand", false), None);
    assert_eq!(
        at("  .bordered();", "bordered", true),
        Some(Sheet::Class(s("bordered")))
    );
    assert_eq!(
        at("@use 'sass:math';", "math", false),
        Some(Sheet::Import(s("sass:math")))
    );
    assert_eq!(
        at("@import url(base.css);", "base", false),
        Some(Sheet::Import(s("base.css")))
    );
    assert_eq!(
        at("  background: url(i.svg#check);", "check", false),
        None,
        "a value is no selector"
    );
    assert_eq!(
        at("  \"primary\": $blue,", "primary", false),
        None,
        "a map key declares nothing"
    );
    assert_eq!(at("  color: #fff;", "fff", false), None);
}

#[test]
fn sass_tries_partials_then_index_files() {
    let got: Vec<String> = sass_candidates("lib/x")
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    assert_eq!(
        got,
        [
            "lib/_x.scss",
            "lib/x.scss",
            "lib/_x.sass",
            "lib/x.sass",
            "lib/x.css",
            "lib/x/_index.scss",
            "lib/x/index.scss"
        ]
    );
    let uses = sass_uses(
        "@use 'variables' as v;\n@use 'src/mixins';\n@use 'all' as *;\n@use 'sass:math';\n",
    );
    assert_eq!(
        uses,
        [
            (Some("v".to_owned()), "variables".to_owned()),
            (Some("mixins".to_owned()), "src/mixins".to_owned()),
            (None, "all".to_owned()),
        ]
    );
}

#[test]
fn style_blocks_keep_their_lines_and_blank_the_rest() {
    let html = "<p class=\"x\">\n<style>\n.x { }\n</style>\n<script>.y {}</script>\n";
    let s = style_blocks(html);
    assert_eq!(s.lines().count(), html.lines().count());
    assert_eq!(rules(&s).iter().map(|r| r.line).collect::<Vec<_>>(), [3]);
    assert_eq!(
        html_literal_lines("a\n<!-- b\nc -->\nd\n"),
        [false, false, true, false]
    );
}

#[test]
fn d_lists_a_stylesheets_mixins_functions_placeholders_and_keyframes() {
    for (line, want) in [
        ("@mixin button-variant($bg) {", Some("button-variant")),
        ("@function tint-color($c) {", Some("tint-color")),
        ("%message-shared {", Some("message-shared")),
        ("@keyframes spin {", Some("spin")),
        ("@-webkit-keyframes spin {", Some("spin")),
        (".btn-primary {", None),
        ("  --brand: #0a7;", None),
        ("$primary: #0d6efd;", None),
    ] {
        let want: Vec<String> = want.into_iter().map(str::to_owned).collect();
        assert_eq!(listed(Kind::Css, line), want, "{line}");
    }
    assert!(!shared_symbols(Some(Kind::Css)));
    assert!(shared_symbols(Some(Kind::Html)));
}

#[test]
fn a_style_block_is_lexed_alone_in_its_markup() {
    let vue =
        "<p>src/*.ts</p>\n<style>\n.a {}\n/* x\n*/\n</style>\n<!--\n<style>.b {}</style>\n-->\n";
    let literal = literal_lines(Kind::Css, vue);
    assert!(!literal[2], ".a after a glob in the template");
    assert!(literal[4], "inside the block's own comment");
    assert!(literal[7], "inside an HTML comment");
}
