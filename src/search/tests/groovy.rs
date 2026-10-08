use super::*;

#[test]
fn groovy_gradle_and_jenkins_files_are_of_the_jvm_kind() {
    for name in [
        "src/Ledger.groovy",
        "run.gvy",
        "build.gradle",
        "settings.gradle",
        "Jenkinsfile",
        "ci/release.jenkinsfile",
    ] {
        assert_eq!(kind_of(Path::new(name)), Some(Kind::Jvm), "{name}");
        assert!(groovy(Path::new(name)), "{name}");
    }
    for name in [
        "build.gradle.kts",
        "Ledger.java",
        "Ledger.scala",
        "Jenkinsfile.bak",
    ] {
        assert!(!groovy(Path::new(name)), "{name}");
    }
}

#[test]
fn groovy_and_dollar_slashy_strings_hide_their_lines() {
    let text = "class A {\n    String a = '''\n    def ghost() {\n    '''\n    String b = $/\n    class Phantom {\n    /$\n    def real() {}\n}\n";
    assert_eq!(
        literal_lines(Kind::Jvm, text),
        [
            false, false, true, true, false, true, true, false, false, false
        ]
    );
}

#[test]
fn a_gradle_line_declares_only_in_a_groovy_file() {
    let declares = |path: &str, word: &str, text: &str, line: usize| {
        let lines: Vec<&str> = text.lines().collect();
        let re = Regex::new(&def_patterns(Kind::Jvm, word).join("|")).unwrap();
        re.is_match(lines[line - 1])
            && declares_where(
                Kind::Jvm,
                Path::new(path),
                word,
                line,
                lines[line - 1],
                || &lines[..],
            )
    };
    let gradle = "ext {\n    tool = '2'\n}\next.lint = 3\nproject.ext.rev = 'a'\ntask site {\n}\ntask zip(type: Zip) {\n}\ntasks.register('docs', Copy)\ntasks.create(\"api\")\nversion = '1'\nrepositories {\n    mirror = 'x'\n}\n";
    for (word, line) in [
        ("tool", 2),
        ("lint", 4),
        ("rev", 5),
        ("site", 6),
        ("zip", 8),
    ] {
        assert!(declares("build.gradle", word, gradle, line), "{word}");
    }
    for (word, line) in [("docs", 10), ("api", 11)] {
        assert!(declares("build.gradle", word, gradle, line), "{word}");
    }
    assert!(!declares("build.gradle", "mirror", gradle, 14));
    assert!(!declares("build.gradle", "version", gradle, 12));
    for word in ["tool", "lint", "site", "docs"] {
        let line = gradle.lines().position(|l| l.contains(word)).unwrap() + 1;
        assert!(!declares("Build.java", word, gradle, line), "{word}");
        assert!(!declares("build.gradle.kts", word, gradle, line), "{word}");
    }
}

#[test]
fn d_lists_groovy_methods_and_gradle_tasks_but_no_variables() {
    let rows: Vec<(&str, Regex)> = SYMBOLS
        .iter()
        .filter(|(k, _)| *k == Some(Kind::Jvm))
        .map(|(_, p)| (*p, Regex::new(p).unwrap()))
        .collect();
    let listed = |path: &str, line: &str| {
        rows.iter()
            .filter(|(p, _)| row_reads(p, Path::new(path)))
            .find_map(|(_, re)| symbol_name(re, line))
    };
    for (line, name) in [
        ("def call(Map config = [:]) {", "call"),
        ("    static def helper(Tint t) {", "helper"),
        ("    private def load() {", "load"),
        ("trait Auditable {", "Auditable"),
        ("task buildDocs(type: Copy) {", "buildDocs"),
        ("tasks.register('buildDocs', Copy) {", "buildDocs"),
    ] {
        assert_eq!(
            listed("build.gradle", line).as_deref(),
            Some(name),
            "{line}"
        );
    }
    for line in [
        "    def total = 0",
        "def version = '1.0'",
        "ext.kotlinVersion = '2'",
    ] {
        assert_eq!(listed("Ledger.groovy", line), None, "{line}");
    }
    assert_eq!(
        listed("Ledger.scala", "  def total = 0").as_deref(),
        Some("total")
    );
    assert_eq!(listed("App.java", "task buildDocs {"), None);
}

#[test]
fn an_untyped_parameter_is_bound_only_on_a_groovy_def() {
    let groovy = "def deploy(env, Map config = [:]) {\n    echo env\n    echo config\n}\n";
    assert_eq!(bindings(Kind::Jvm, groovy, 2, "env")[0].line1, 1);
    assert_eq!(bindings(Kind::Jvm, groovy, 3, "config")[0].line1, 1);
    let kotlin =
        "fun f() {\n    val seen = load()\n    check(\n        Item(id = 1, seen),\n    )\n}\n";
    assert_eq!(bindings(Kind::Jvm, kotlin, 4, "seen")[0].line1, 2);
}

#[test]
fn groovy_parameters_read_past_an_annotation_with_arguments() {
    let p = |decl: &str| jvm_parameters(decl, 1, "X", true);
    assert_eq!(
        p(r#"X(@Named("cfg") Map<String, Object> m) {"#),
        Some((1, 0))
    );
    assert_eq!(
        p(r#"X(@Named(value = "x") Long id, String tag = "a, b") {"#),
        Some((1, 1))
    );
    assert_eq!(
        p("X(@Size(max = 3) String s, Map<K, V> m = [:], int n=1) {"),
        Some((1, 2))
    );
    assert_eq!(p("X(@Tag(\"a\") Object... rest) {"), Some((0, usize::MAX)));
    assert_eq!(p("X(Closure c = { -> 1 }, int b) {"), Some((1, 1)));
    assert_eq!(
        p("X(int a, Closure c = { x, y -> x }, int b) {"),
        Some((2, 1))
    );
    assert_eq!(p("X(boolean f = x >= 1, int b) {"), Some((1, 1)));
    assert_eq!(p("X(int n = 1 >> 2, m) {"), Some((1, 1)));
    assert_eq!(p("X(boolean f = a < 1, int b) {"), Some((1, 1)));
    assert_eq!(
        p("X(boolean f = a > b, Map<K, List<V>> m, int c) {"),
        Some((2, 1))
    );
    assert_eq!(
        jvm_parameters("X(int n = 1) {", 1, "X", false),
        Some((1, 0))
    );
}
