use super::*;

#[test]
fn kinds_come_from_the_file_name() {
    for (name, kind) in [
        ("a.py", Some(Kind::Python)),
        ("a.go", Some(Kind::Go)),
        ("lib.rs", Some(Kind::Rust)),
        ("app.tsx", Some(Kind::TsJs)),
        ("util.cjs", Some(Kind::TsJs)),
        ("Invoice.java", Some(Kind::Jvm)),
        ("invoice.rb", Some(Kind::Ruby)),
        ("Rakefile", Some(Kind::Ruby)),
        ("Gemfile", Some(Kind::Ruby)),
        ("config.ru", Some(Kind::Ruby)),
        ("merl.gemspec", Some(Kind::Ruby)),
        ("tasks.rake", Some(Kind::Ruby)),
        ("invoice.rbi", Some(Kind::Ruby)),
        ("invoice.c", Some(Kind::C)),
        ("invoice.h", Some(Kind::C)),
        ("ledger.cc", Some(Kind::C)),
        ("ledger.cpp", Some(Kind::C)),
        ("ledger.cxx", Some(Kind::C)),
        ("ledger.hpp", Some(Kind::C)),
        ("ledger.hh", Some(Kind::C)),
        ("ledger.hxx", Some(Kind::C)),
        ("Ledger.m", Some(Kind::C)),
        ("Ledger.mm", Some(Kind::C)),
        ("Invoice.cs", Some(Kind::CSharp)),
        ("build.csx", Some(Kind::CSharp)),
        ("Session.swift", Some(Kind::Swift)),
        ("Invoice.php", Some(Kind::Php)),
        ("show.phtml", Some(Kind::Php)),
        ("init.lua", Some(Kind::Lua)),
        ("ledger.ex", Some(Kind::Elixir)),
        ("mix.exs", Some(Kind::Elixir)),
        ("ledger.zig", Some(Kind::Zig)),
        // Zig's data format: painted as Zig, but it declares nothing.
        ("build.zig.zon", None),
        ("app.kt", Some(Kind::Jvm)),
        ("build.gradle.kts", Some(Kind::Jvm)),
        ("Ledger.scala", Some(Kind::Jvm)),
        ("run.sc", Some(Kind::Jvm)),
        ("build.sbt", Some(Kind::Jvm)),
        ("build.mill", Some(Kind::Jvm)),
        ("run.sh", Some(Kind::Shell)),
        ("run.bash", Some(Kind::Shell)),
        ("run.zsh", Some(Kind::Shell)),
        ("run.ksh", Some(Kind::Shell)),
        (".bashrc", Some(Kind::Shell)),
        (".bash_profile", Some(Kind::Shell)),
        (".bash_aliases", Some(Kind::Shell)),
        (".zshrc", Some(Kind::Shell)),
        (".zshenv", Some(Kind::Shell)),
        (".zprofile", Some(Kind::Shell)),
        (".profile", Some(Kind::Shell)),
        // A shebang-only script: `kind_of` goes by the name, so it has no kind.
        ("install", None),
        ("schema.sql", Some(Kind::Sql)),
        ("dump.psql", Some(Kind::Sql)),
        ("init.pgsql", Some(Kind::Sql)),
        ("seed.mysql", Some(Kind::Sql)),
        ("001.ddl", Some(Kind::Sql)),
        ("002.dml", Some(Kind::Sql)),
        ("Makefile", Some(Kind::Make)),
        ("GNUmakefile", Some(Kind::Make)),
        ("rules.mk", Some(Kind::Make)),
        ("main.tf", Some(Kind::Terraform)),
        ("prod.tfvars", Some(Kind::Terraform)),
        ("Dockerfile", Some(Kind::Docker)),
        ("Dockerfile.prod", Some(Kind::Docker)),
        ("api.Dockerfile", Some(Kind::Docker)),
        ("Containerfile", Some(Kind::Docker)),
        ("Dockerfile.dockerignore", None),
        ("compose.yaml", Some(Kind::Yaml)),
        (".gitlab-ci.yml", Some(Kind::Yaml)),
        ("README", None),
        ("notes.txt", None),
    ] {
        assert_eq!(kind_of(Path::new(name)), kind, "{name}");
    }
}

#[test]
fn qualifier_is_the_chain_in_front_of_the_word() {
    let q = |l: &str, i| qualifier(l, i);
    assert_eq!(q("    return json.load(fp)", 16), ["json"]);
    assert_eq!(q("x = os.path.join(a)", 12), ["os", "path"]);
    assert_eq!(q("let s = fs::read_to_string(p)", 12), ["fs"]);
    assert!(q("load(fp)", 0).is_empty());
    assert!(q("x = load(fp)", 4).is_empty());
    assert_eq!(q("this.#root.insert(p)", 11), ["this", "#root"]);
    assert!(
        q("x = make_uow().users.delete_user()", 21).is_empty(),
        "a chain that hangs off a call, an index or `?.` has no name to start from"
    );
    assert!(q("a[0].users.find()", 11).is_empty());
    assert!(q("a?.users.find()", 9).is_empty());
    assert!(q("  .users.find()", 9).is_empty());
    assert_eq!(
        q("        super().store(item)", 16),
        ["super"],
        "Python's `super()` is one name, as TypeScript's `super` is"
    );
    assert_eq!(q("    print(super().label)", 18), ["super"]);
    assert!(
        q("    my_super().store(item)", 15).is_empty(),
        "`my_super()` is a call"
    );
    assert!(q("    a.super().store(item)", 14).is_empty());
    assert_eq!(
        q("f(...this.repo.find())", 15),
        ["this", "repo"],
        "a spread is no member access"
    );
    assert_eq!(
        q("for i in 0..v.len() {", 14),
        ["v"],
        "a range is no member access"
    );
    assert!(
        q("    данные.load(x)", 17).is_empty(),
        "a character outside ASCII in front of the word is no name char"
    );
    assert!(q("  café.load(x)", 8).is_empty());
}

#[test]
fn word_at_covers_the_run_under_and_before_the_cursor() {
    let line = "    inv = parse_it(\"x\")";
    assert_eq!(word_at(line, 4, ""), Some((4..7, "inv")));
    assert_eq!(word_at(line, 6, ""), Some((4..7, "inv")));
    assert_eq!(
        word_at(line, 7, ""),
        Some((4..7, "inv")),
        "right after a word counts as being on it"
    );
    assert_eq!(word_at(line, 8, ""), None, "on a space it does not");
    assert_eq!(word_at(line, 10, ""), Some((10..18, "parse_it")));
    assert_eq!(word_at(line, line.len(), ""), None);
    assert_eq!(word_at("", 0, ""), None);
    assert_eq!(word_at("x", 99, ""), Some((0..1, "x")));
}

#[test]
fn extra_word_chars_join_names_but_do_not_start_them() {
    let line = "  bucket = var.my-region[0]";
    assert_eq!(word_at(line, 17, ""), Some((15..17, "my")));
    assert_eq!(word_at(line, 17, "-"), Some((15..24, "my-region")));
    assert_eq!(word_at(line, 12, "-."), Some((11..24, "var.my-region")));
    assert_eq!(
        word_at("COPY --from=build", 8, "-"),
        Some((7..11, "from")),
        "a flag's dashes are not part of the name"
    );
    assert_eq!(
        word_at("x = var.", 6, "-."),
        Some((4..7, "var")),
        "nor is a trailing dot"
    );
    assert_eq!(word_at("- db", 0, "-"), None);
    assert_eq!(word_chars(Some(Kind::Terraform), true), "-.");
    assert_eq!(word_chars(Some(Kind::Terraform), false), "-");
    assert_eq!(word_chars(Some(Kind::Python), true), "");
    assert_eq!(word_chars(None, false), "");
}

#[test]
fn enclosing_declarations_are_the_headers_a_line_stands_in() {
    let pins = |kind, text: &str| -> Vec<Vec<usize>> {
        let lines: Vec<String> = text.lines().map(String::from).collect();
        (0..lines.len())
            .map(|l| enclosing_declarations(Some(kind), &lines, l))
            .collect()
    };
    let rs = "\
impl Search {
    /// Greps.
    pub fn grep_project(
        root: &Path,
    ) -> Result<Vec<Hit>> {
        fn helper() {}
        let mut hits = Vec::new();

        for rel in files {
// dbg!(rel);
            hits.push(rel);
        }
    }
}

fn next() {
    body();
}
";
    let (none, imp, method, next): (&[usize], &[usize], &[usize], &[usize]) =
        (&[], &[0], &[0, 2], &[15]);
    assert_eq!(
        pins(Kind::Rust, rs),
        [
            none, imp, imp, method, method, method, method, method, method, method, method, method,
            imp, none, none, none, next, none
        ],
        "outermost first; a loop is not pinned, nor a fn declared earlier in the body; a wrapped \
         signature's tail, a blank line and a comment at the left edge keep the header around them"
    );
    let py = "\
class Store:
    @property
    def items(self):
        return [
            1,
        ]

    count = 0
";
    let (class, def): (&[usize], &[usize]) = (&[0], &[0, 2]);
    assert_eq!(
        pins(Kind::Python, py),
        [none, class, class, def, def, def, class, class]
    );
    let cpp = "\
class Store {
public:
    int count() {
        return 1;
    }
};
";
    let (class, method): (&[usize], &[usize]) = (&[0], &[0, 2]);
    assert_eq!(
        pins(Kind::C, cpp),
        [none, class, class, method, class, none],
        "a C++ access specifier at the left edge is a label inside the class, as `d` reads it"
    );
    assert_eq!(
        pins(Kind::Yaml, "base: &base\n  a: 1\n  b: 2\n"),
        [none, none, none],
        "a YAML anchor names a value, not a container"
    );
}

#[test]
fn a_kotlin_extension_is_named_by_its_receiver() {
    let text = "fun Topic.asExternalModel() = this\nfun <T> List<T>.second(): T = this[1]\nfun Topic?.orEmpty() = this\nval Topic.testTag get() = id\nfun plain() = 1\n";
    let q = |line, name| qualified(Kind::Jvm, text, line, name);
    assert_eq!(
        q(1, "asExternalModel").as_deref(),
        Some("Topic.asExternalModel")
    );
    assert_eq!(q(2, "second").as_deref(), Some("List.second"));
    assert_eq!(q(3, "orEmpty").as_deref(), Some("Topic.orEmpty"));
    assert_eq!(q(4, "testTag").as_deref(), Some("Topic.testTag"));
    assert_eq!(q(5, "plain"), None);
}

#[test]
fn a_wrapped_kotlin_header_and_a_companion_qualify_their_members() {
    let text = "class Repo @Inject constructor(\n    private val seed: String,\n) : Base {\n    fun topics() = 1\n\n    companion object {\n        const val DEFAULT = 1\n    }\n}\n";
    let q = |line, name| qualified(Kind::Jvm, text, line, name);
    assert_eq!(q(4, "topics").as_deref(), Some("Repo.topics"));
    assert_eq!(q(7, "DEFAULT").as_deref(), Some("Repo.DEFAULT"));
}
