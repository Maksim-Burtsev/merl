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
        let read_as = match jsx {
            true => AttrLine::Jsx,
            false => AttrLine::Markup,
        };
        attr_at(line, line.find(word).unwrap(), read_as).map(|(a, r)| (a, &line[r]))
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
    let at = |line: &str, word: &str, less: bool| {
        let syntax = match less {
            true => SheetSyntax::Less,
            false => SheetSyntax::CssOrSass,
        };
        sheet_at(line, line.find(word).unwrap(), syntax)
    };
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
            SassUse {
                namespace: SassNamespace::Named("v".to_owned()),
                module: "variables".to_owned()
            },
            SassUse {
                namespace: SassNamespace::Named("mixins".to_owned()),
                module: "src/mixins".to_owned()
            },
            SassUse {
                namespace: SassNamespace::FlatAsStar,
                module: "all".to_owned()
            },
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

#[test]
fn an_attribute_is_told_from_what_merely_looks_like_one() {
    let at = |line: &str, word: &str, read_as| {
        attr_at(line, line.find(word).unwrap(), read_as).map(|(a, r)| (a, line[r].to_owned()))
    };
    assert_eq!(
        at(r#"<p :class="active">"#, "active", AttrLine::Markup),
        None,
        "Vue's `:class` binds an expression"
    );
    assert_eq!(
        at(r#"const s = "class:active";"#, "active", AttrLine::Jsx),
        None,
        "a Svelte directive spelled in a string"
    );
    assert_eq!(
        at(r#"  id="main""#, "main", AttrLine::Jsx),
        Some((Attr::Id, "main".to_owned())),
        "first on a line of a tag's attributes"
    );
}

#[test]
fn a_selector_reads_past_values_strings_and_its_mixin() {
    assert!(
        styled(".a { b: url(//cdn.x/i.png); }\n&-c {}\n", "a-c").is_empty(),
        "`//` inside parentheses opens no comment"
    );
    assert_eq!(
        styled(".x { content: \"}\"; &-y {} }\n", "x-y"),
        [1],
        "a `}}` in a string closes nothing"
    );
    assert!(styled(".col-@{i} {}\n", "col-").is_empty());
    assert_eq!(
        styled("@mixin m {\n  .y {}\n}\n", "y"),
        [2],
        "a mixin's body styles"
    );
    assert!(
        styled(".a {\n  font: {\n    .b {}\n  }\n}\n", "b").is_empty(),
        "nested properties hold no rule"
    );
    assert!(styled("@detached: {\n  .c {}\n}\n", "c").is_empty());
    let both = ".a, .b {\n  &-x {}\n}\n";
    assert_eq!(styled(both, "a-x"), [2]);
    assert_eq!(styled(both, "b-x"), [2]);
}

#[test]
fn an_import_names_the_files_it_may_be() {
    let shown = |v: Vec<PathBuf>| -> Vec<String> {
        v.iter().map(|p| p.display().to_string()).collect()
    };
    assert_eq!(
        shown(sass_candidates("lib/x.scss")),
        ["lib/_x.scss", "lib/x.scss"]
    );
    assert_eq!(
        shown(sass_candidates("~pkg/x.css")),
        ["pkg/_x.css", "pkg/x.css"]
    );
    assert_eq!(
        shown(import_candidates("base", SheetSyntax::Less)),
        ["base.less", "base"]
    );
    assert_eq!(
        shown(import_candidates("~pkg/base.css", SheetSyntax::Less)),
        ["pkg/base.css"]
    );
    assert_eq!(
        shown(import_candidates("base", SheetSyntax::CssOrSass)),
        ["base"]
    );
    assert!(sass_uses("@import 'x';\n").is_empty());
    for url in ["https://x/a.css", "//cdn/a.css", "data:text/css,a"] {
        assert!(is_url(url), "{url}");
    }
    assert!(!is_url("lib/a.css"));
    assert_eq!(
        element_with_id("<p>\n<a name=\"top\">\n<b id=\"top\">\n", "top"),
        Some(2)
    );
}

#[test]
fn u_marks_a_selector_only_where_its_rule_styles_the_name() {
    let declares = |word: &str, text: &str| -> Vec<usize> {
        let re = Regex::new(&css_patterns(word).join("|")).unwrap();
        text.lines()
            .enumerate()
            .filter(|(_, l)| re.is_match(l))
            .map(|(i, _)| i + 1)
            .filter(|&n| css_declares(word, n, text.lines().nth(n - 1).unwrap(), || text.to_owned()))
            .collect()
    };
    let scss = ".btn:hover {\n}\n.btn .icon {\n}\n$btn: 1;\n@mixin btn {}\n";
    assert_eq!(declares("btn", scss), [1, 5, 6]);
    assert_eq!(declares("--brand", ":root { --brand: #000; }\n"), [1]);
    assert_eq!(
        css_code("a { b: url(//cdn/x); } // c /* d */"),
        "a { b: url(//cdn/x); } "
    );
    assert_eq!(css_code("a /* x */ { }"), "a  { }");
}
