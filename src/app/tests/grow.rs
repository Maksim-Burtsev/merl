use super::*;

fn code(name: &str, text: &str) -> App {
    let path = std::env::temp_dir().join(format!("merl-grow-{}-{name}", std::process::id()));
    std::fs::write(&path, text).unwrap();
    let mut a = App::new(
        PathBuf::from("/tmp"),
        Tree::default(),
        Vec::new(),
        Buffer::from_bytes(path, text.as_bytes()),
        None,
    );
    a.view_w = 80;
    a.view_h = 30;
    a
}

fn presses(a: &mut App, key: char, n: usize) -> Vec<String> {
    (0..n)
        .map(|_| {
            press(a, KeyCode::Char(key), KeyModifiers::NONE);
            a.selected_text().unwrap_or_default()
        })
        .collect()
}

const PY: &str = "\
class Keys:
    def rotate(self, key):
        log.info(
            \"rotated\",
            organization_id=key.organization_id,
            name=\"a(b\",
        )

        return key


x = 1
";

#[test]
fn v_takes_the_quotes_and_brackets_before_the_line_and_the_blocks_after_it() {
    let mut a = code("args.py", PY);
    (a.line, a.col) = (4, 14);
    let lines = |from: usize, to: usize| PY.lines().collect::<Vec<_>>()[from..=to].join("\n");
    assert_eq!(
        presses(&mut a, 'v', 9),
        [
            "organization_id".to_owned(),
            lines(4, 4),
            lines(3, 5),
            format!("(\n{}\n        )", lines(3, 5)),
            lines(2, 6),
            lines(2, 8),
            lines(1, 8),
            lines(0, 8),
            PY.trim_end().to_owned(),
        ]
    );
    assert_eq!(presses(&mut a, 'V', 2), [lines(0, 8), lines(1, 8)]);
    presses(&mut a, 'V', 7);
    assert_eq!(a.selection(), None, "back past the word is the cursor");
    assert_eq!((a.line, a.col), (4, 14));
    assert_eq!(presses(&mut a, 'V', 1), [""], "nothing to step back to");
}

#[test]
fn a_bracket_in_a_string_is_text_and_a_blank_line_takes_its_block() {
    let mut a = code("string.py", PY);
    (a.line, a.col) = (5, 20);
    assert_eq!(
        presses(&mut a, 'v', 4),
        ["b", "a(b", "\"a(b\"", "            name=\"a(b\","]
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    (a.line, a.col) = (7, 0);
    let body = PY.lines().collect::<Vec<_>>()[2..=8].join("\n");
    assert_eq!(presses(&mut a, 'v', 1), [body]);
}

#[test]
fn v_steps_back_after_a_move_only_to_what_it_grew_since() {
    let mut a = code("moved.py", PY);
    (a.line, a.col) = (4, 14);
    presses(&mut a, 'v', 2);
    press(&mut a, KeyCode::Right, KeyModifiers::SHIFT);
    assert_eq!(
        presses(&mut a, 'V', 1),
        ["            organization_id=key.organization_id,\n"]
    );
}

#[test]
fn a_file_with_no_grammar_keeps_word_line_paragraph_file() {
    let text = "(ns a)\n\n(defn f [x]\n  (inc x))\n\n(f 1)\n";
    let mut a = code("plain.clj", text);
    (a.line, a.col) = (3, 4);
    assert_eq!(
        presses(&mut a, 'v', 4),
        [
            "inc",
            "  (inc x))",
            "(defn f [x]\n  (inc x))",
            text.trim_end()
        ]
    );
    assert_eq!(presses(&mut a, 'V', 1), ["(defn f [x]\n  (inc x))"]);
}

#[test]
fn v_steps_back_nothing_once_the_text_changed_under_its_history() {
    let mut a = code("edited.py", PY);
    (a.line, a.col) = (4, 14);
    presses(&mut a, 'v', 3);
    let selected = a.selection();
    a.buf.lines[4] = "            x,".to_owned();
    a.buf.edited(4);
    assert_eq!(presses(&mut a, 'V', 1).len(), 1);
    assert_eq!(
        a.selection(),
        selected,
        "the line `V` would restore is gone"
    );
}

#[test]
fn a_shift_run_the_watcher_judges_leaves_v_nothing_to_step_back() {
    let mut a = code("watched.py", "x = foo(alpha, \"beta\") + 1\n");
    a.col = 8;
    let t = Instant::now();
    for i in 0..13 {
        a.key_at(
            KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT),
            t + Duration::from_millis(30) * i,
        );
    }
    assert_eq!(a.selected_text().as_deref(), Some("alpha, \"beta\""));
    a.key_at(
        KeyEvent::new(KeyCode::Char('V'), KeyModifiers::NONE),
        t + Duration::from_secs(2),
    );
    assert_eq!(a.selected_text().as_deref(), Some("alpha, \"beta\""));
}

const JS: &str = "\
function add(a, b) {
  if (a) {
    return total(a, [b, \")\"]);
  }
}
";

const CSS: &str = "\
a {
  color: rgb(1, 2, 3);
}
";

fn embedded(
    name: &str,
    own: &str,
    body: &str,
    before: &str,
    after: &str,
    at: (usize, usize),
) -> Vec<String> {
    let mut alone = code(&format!("{name}.{own}"), body);
    (alone.line, alone.col) = at;
    let mut steps = Vec::new();
    while steps.last().map(String::as_str) != Some(body.trim_end()) {
        steps.extend(presses(&mut alone, 'v', 1));
    }
    let mut host = code(name, &format!("{before}{body}{after}"));
    (host.line, host.col) = (before.lines().count() + at.0, at.1);
    let mut ours = presses(&mut host, 'v', steps.len() + 2);
    assert_eq!(ours[..steps.len()], steps, "{name}: the block's own ladder");
    let back: Vec<String> = ours[..=steps.len()].iter().rev().cloned().collect();
    assert_eq!(presses(&mut host, 'V', back.len()), back, "{name}: V");
    ours.split_off(steps.len())
}

#[test]
fn v_in_an_html_script_walks_its_javascript_then_the_script_and_the_body() {
    let (top, end) = (
        "<html>\n<body>\n<script>\n",
        "</script>\n</body>\n</html>\n",
    );
    assert_eq!(
        embedded("index.html", "js", JS, top, end, (2, 17)),
        [
            format!("<script>\n{JS}</script>"),
            format!("<body>\n<script>\n{JS}</script>\n</body>"),
        ]
    );
}

#[test]
fn v_in_an_html_style_walks_its_css_then_the_style() {
    let (top, end) = ("<head>\n<style>\n", "</style>\n</head>\n");
    assert_eq!(
        embedded("page.html", "css", CSS, top, end, (1, 16)),
        [
            format!("<style>\n{CSS}</style>"),
            format!("{top}{CSS}</style>\n</head>"),
        ]
    );
}

#[test]
fn v_in_a_vue_or_svelte_script_walks_its_javascript_then_the_block() {
    let end = "</script>\n\n<p>{x}</p>\n";
    for name in ["app.vue", "app.svelte"] {
        let grown = embedded(name, "js", JS, "<script>\n", end, (2, 17));
        assert_eq!(grown[0], format!("<script>\n{JS}</script>"), "{name}");
    }
}

#[test]
fn v_in_a_markdown_fence_walks_its_language_then_the_fence() {
    let grown = embedded(
        "notes.md",
        "js",
        JS,
        "# Add\n\n```js\n",
        "```\n\nMore.\n",
        (2, 17),
    );
    assert_eq!(grown[0], format!("```js\n{JS}```"));
}

#[test]
fn v_in_markdown_prose_keeps_its_ladder() {
    let text = "# Notes\n\nSee [the link](https://a.b) and `code` here.\n";
    let mut a = code("prose.md", text);
    (a.line, a.col) = (2, 34);
    assert_eq!(presses(&mut a, 'v', 2)[1], text.lines().nth(2).unwrap());
}

#[test]
fn v_in_a_script_inside_an_html_fence_walks_both_layers() {
    let top = "# Page\n\n```html\n<script>\n";
    let grown = embedded("nested.md", "js", JS, top, "</script>\n```\n", (2, 17));
    assert_eq!(
        grown,
        [
            format!("<script>\n{JS}</script>"),
            format!("```html\n<script>\n{JS}</script>\n```"),
        ]
    );
}

#[test]
fn v_in_a_fence_between_two_html_blocks_still_takes_the_fence() {
    let top = "<div>\na\n</div>\n\n```js\n";
    let grown = embedded(
        "blocks.md",
        "js",
        JS,
        top,
        "```\n\n<div>\nb\n</div>\n",
        (2, 17),
    );
    assert_eq!(grown[0], format!("```js\n{JS}```"));
}

#[test]
fn v_in_an_attribute_keeps_the_inside_of_its_quotes() {
    let text = "<template>\n  <li v-for=\"item in items\">{{ item }}</li>\n</template>\n";
    let mut a = code("list.vue", text);
    (a.line, a.col) = (1, 19);
    assert_eq!(
        presses(&mut a, 'v', 3),
        ["in", "item in items", "\"item in items\""]
    );
}

#[test]
fn v_in_a_file_that_hosts_no_block_keeps_its_ladder() {
    let mut a = code("regex.py", "x = re.compile(r\"(a[bc])\")\n");
    a.col = 21;
    assert_eq!(presses(&mut a, 'v', 3), ["bc", "(a[bc])", "r\"(a[bc])\""]);
}
