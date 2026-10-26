use super::*;

#[test]
fn a_component_is_code_only_inside_its_script() {
    let code = |name: &str, text: &str| script_lines(Path::new(name), text).unwrap();
    let vue = "<template>\n  <p>{{ a }}</p>\n</template>\n<script setup lang=\"ts\">\nconst a = 1\n</script>\n<style>\n.a { x: y }\n</style>";
    assert_eq!(
        code("A.vue", vue),
        [false, false, false, false, true, false, false, false, false]
    );
    let svelte = "<script context=\"module\">\nexport const n = 1\n</script>\n<script\n  lang=\"ts\">\nlet m = 2\n</script>\n<script src=\"x.js\"></script>\n<p>{m}</p>";
    assert_eq!(
        code("B.svelte", svelte),
        [false, true, false, false, false, true, false, false, false],
        "Svelte's module script, a tag wrapped over lines, and a `<script src>` closed on its line"
    );
    let astro = "---\nconst t = 1\n---\n<h1>{t}</h1>\n---\n<script>\nlet u = 2\n</script>";
    assert_eq!(
        code("C.astro", astro),
        [false, true, false, false, false, false, true, false],
        "Astro's frontmatter only from the first line, and its `<script>` blocks"
    );
    assert_eq!(script_lines(Path::new("d.ts"), "const x = 1"), None);
    assert_eq!(kind_of(Path::new("C.astro")), Some(Kind::TsJs));
    let text = script_text(Path::new("A.vue"), vue, Some(1));
    assert_eq!(
        text.lines().nth(1),
        Some(" ".repeat(16).as_str()),
        "the cursor's template line keeps its length"
    );
    assert_eq!(text.lines().nth(4), Some("const a = 1"));
    assert_eq!(
        text.lines().nth(7),
        Some(""),
        "the rules read the template as blank lines"
    );
}

#[test]
fn a_template_binds_its_loop_slot_and_block_names() {
    let binds = |line: &str, word: &str| !template_binds(&[line], &[false], word).is_empty();
    assert!(binds(r#"<li v-for="(item, i) in items" :key="i">"#, "item"));
    assert!(binds(r#"<li v-for="(item, i) in items" :key="i">"#, "i"));
    assert!(!binds(
        r#"<li v-for="(item, i) in items" :key="i">"#,
        "items"
    ));
    assert!(binds(r#"<Row v-slot="{ row }">"#, "row"));
    assert!(binds(r#"<template #default="{ cell }">"#, "cell"));
    assert!(!binds(r##"<a href="#top" title="x">"##, "x"));
    assert!(binds("{#each items as item, i (item.id)}", "item"));
    assert!(binds("{#each items as { id, name }}", "name"));
    assert!(!binds("{#each items as item, i (key)}", "key"));
    assert!(binds("{:then value}", "value"));
    assert!(binds("{#await load() then data}", "data"));
    assert!(binds("{:catch error}", "error"));
    assert!(binds("<Cell let:item>", "item"));
    assert!(binds("<Cell let:item={row}>", "row"));
    assert!(!binds("<Cell let:item={row}>", "item"));
    assert!(binds("{@const total = a + b}", "total"));
    assert!(binds("{#snippet row(item: Item, index)}", "index"));
    assert!(!binds("{#snippet row(item: Item, index)}", "Item"));
    assert!(
        template_binds(&["{#each a as b}"], &[true], "b").is_empty(),
        "a script line binds nothing the template's way"
    );
}

#[test]
fn a_tag_is_the_component_of_its_name() {
    let line = "  <user-card :user=\"u\"></user-card> <UserCard/>";
    assert_eq!(tag_at(line, 5), Some(3..12));
    assert_eq!(tag_at(line, 30), Some(25..34));
    assert_eq!(tag_at(line, 40), Some(37..45));
    assert_eq!(tag_at(line, 15), None);
    assert_eq!(component_name("user-card"), "UserCard");
    assert_eq!(component_name("UserCard"), "UserCard");
    assert_eq!(component_name("div"), "div");
    assert_eq!(component_name("user-éclair"), "UserÉclair");
}

#[test]
fn a_destructuring_binds_the_name_after_its_key() {
    let binds = |line: &str, word: &str| !template_binds(&[line], &[false], word).is_empty();
    assert!(binds(r#"<Row #default="{ cell: entry }">"#, "entry"));
    assert!(!binds(r#"<Row #default="{ cell: entry }">"#, "cell"));
    assert!(binds("{#each users as { id: userId }}", "userId"));
    assert!(binds("<Btn v-slot=\"{ size = 'md' }\">", "size"));
    assert!(!binds("<Btn v-slot=\"{ size = md }\">", "md"));
}

#[test]
fn a_component_saved_with_a_bom_still_has_its_script() {
    let code = script_lines(
        Path::new("A.vue"),
        "\u{feff}<script setup>\nconst a = 1\n</script>",
    );
    assert_eq!(code.unwrap(), [false, true, false]);
    let code = script_lines(Path::new("B.astro"), "\u{feff}---\nconst t = 1\n---\n<h1/>");
    assert_eq!(code.unwrap(), [false, true, false, false]);
}

#[test]
fn a_style_block_and_the_last_line_are_no_script() {
    let lines = [
        "<script>",
        "const a = 1",
        "</script>",
        "<style>",
        ".a {",
        "  color: red;",
        "}",
        "</style>",
        "<p/>",
    ];
    assert!(!in_style(&lines, 1));
    assert!(in_style(&lines, 5));
    assert!(!in_style(&lines, 7));
    assert!(!in_style(&lines, 8));
    let hidden = hidden_lines(Kind::TsJs, Path::new("A.vue"), &lines.join("\n"));
    assert_eq!(
        hidden,
        [false, false, true, true, true, true, true, true, true],
        "the last line, outside the script, is hidden like every other one but the first"
    );
}

#[test]
fn a_script_tag_is_read_by_its_name_and_its_end() {
    let code = |text: &str| script_lines(Path::new("A.vue"), text).unwrap();
    assert_eq!(
        code("<scripts>\nconst a = 1\n</scripts>"),
        [false, false, false],
        "another tag of the same start"
    );
    assert_eq!(
        code("<script setup\n  lang=\"ts\">\nconst a = 1\n</script>"),
        [false, false, true, false],
        "a tag wrapped after its attributes"
    );
    assert!(!in_style(&["<style>.a {}</style>", ".b {"], 1));
}

#[test]
fn an_arrow_parameter_of_a_template_expression_reaches_the_lines_of_its_body() {
    let lines = [
        "{",
        "\t<Select",
        "\t\toptions={entries.map(([code, locale]) => ({",
        "\t\t\tvalue: code,",
        "\t\t\tlabel: locale,",
        "\t\t}))}",
        "\t\tother={code}",
        "\t/>",
        "\t<ul>{xs.map((x) => go(x))}",
        "\t\t<li>{x}</li>",
        "\t<ul>{ys.map(y =>",
        "\t\t<li>{y}</li>",
        "\t)}</ul>",
        "}",
    ];
    let code = [false; 14];
    let reach = |word: &str, line: usize| template_reaching(&lines, &code, word, line);
    assert_eq!(
        reach("code", 3),
        [3],
        "a destructured parameter, the body below"
    );
    assert_eq!(reach("locale", 4), [3]);
    assert!(reach("code", 6).is_empty(), "past the body's indentation");
    assert!(reach("x", 9).is_empty(), "an arrow closed on its own line");
    assert_eq!(
        reach("y", 11),
        [11],
        "a bare parameter, the body on the next line"
    );
    let script = [true; 14];
    assert!(template_reaching(&lines, &script, "code", 3).is_empty());
}
