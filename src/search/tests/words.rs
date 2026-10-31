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
        ("shop.bats", Some(Kind::Shell)),
        (".bashrc", Some(Kind::Shell)),
        (".bash_profile", Some(Kind::Shell)),
        (".bash_aliases", Some(Kind::Shell)),
        (".zshrc", Some(Kind::Shell)),
        (".zshenv", Some(Kind::Shell)),
        (".zprofile", Some(Kind::Shell)),
        (".profile", Some(Kind::Shell)),
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
        ("build.ps1", Some(Kind::PowerShell)),
        ("Ledger.psm1", Some(Kind::PowerShell)),
        ("Ledger.psd1", Some(Kind::PowerShell)),
        ("guide.mdx", Some(Kind::Markdown)),
        ("schema.graphqls", Some(Kind::Graphql)),
        ("query.gql", Some(Kind::Graphql)),
        ("user.proto", Some(Kind::Proto)),
        ("index.htm", Some(Kind::Html)),
        ("hash.rbs", None),
    ] {
        assert_eq!(kind_of(Path::new(name)), kind, "{name}");
    }
    assert_eq!(
        opened_kind(Path::new("core/hash.rbs"), String::new),
        Some(Kind::Ruby)
    );
}

#[test]
fn qualifier_is_the_chain_in_front_of_the_word() {
    let q = |l: &str, i| qualifier(l, i);
    assert_eq!(q("    return json.load(fp)", 16), ["json"]);
    assert_eq!(q("x = os.path.join(a)", 12), ["os", "path"]);
    assert_eq!(q("let s = fs::read_to_string(p)", 12), ["fs"]);
    assert_eq!(q("load(fp)", 0), [] as [String; 0]);
    assert_eq!(q("x = load(fp)", 4), [] as [String; 0]);
    assert_eq!(q("this.#root.insert(p)", 11), ["this", "#root"]);
    assert!(
        q("x = make_uow().users.delete_user()", 21).is_empty(),
        "a chain that hangs off a call, an index or `?.` has no name to start from"
    );
    assert_eq!(q("a[0].users.find()", 11), [] as [String; 0]);
    assert_eq!(q("a?.users.find()", 9), [] as [String; 0]);
    assert_eq!(q("  .users.find()", 9), [] as [String; 0]);
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
    assert_eq!(q("    a.super().store(item)", 14), [] as [String; 0]);
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
    assert_eq!(q("  café.load(x)", 8), [] as [String; 0]);
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

#[test]
fn a_ruby_setter_keeps_its_equals_sign() {
    let text = "class Tariff\n  def rate=(value)\n    @rate = value\n  end\nend\n";
    assert_eq!(
        qualified(Kind::Ruby, text, 2, "rate=").as_deref(),
        Some("Tariff.rate=")
    );
}

#[test]
fn a_jsdoc_property_is_a_field_of_its_typedef() {
    let text = "/**\n * @typedef {Object} Tariff\n * @property {number} rate\n */\n";
    assert_eq!(
        qualified(Kind::TsJs, text, 3, "rate").as_deref(),
        Some("Tariff.rate")
    );
}

#[test]
fn a_csharp_parameter_is_a_local_of_its_method() {
    let text = "class Tariff\n{\n    public decimal Gross(decimal rate)\n    {\n        return rate;\n    }\n}\n";
    assert_eq!(
        qualified(Kind::CSharp, text, 3, "rate").as_deref(),
        Some("Tariff.Gross.rate")
    );
}

#[test]
fn a_later_declarator_is_declared_where_its_statement_is() {
    let top = "const fs = require(\"node:fs\"),\n  more = require(\"./more\");\n";
    assert_eq!(qualified(Kind::TsJs, top, 2, "more"), None);
    let inner = "function run() {\n  const fs = 1,\n    more = 2;\n}\n";
    assert_eq!(
        qualified(Kind::TsJs, inner, 3, "more").as_deref(),
        Some("run.more")
    );
}

#[test]
fn a_cpp_parameter_s_class_is_its_function_and_an_enumerator_s_is_its_enum() {
    let cpp = "\
namespace leveldb {

class Benchmark {
 public:
  enum Order { SEQUENTIAL, RANDOM };
  enum class Mode : int { kFast = 1, kSlow };
  Status Open(const std::string& fname,
              Env* env) override {
    for (int i = 0; i < 3; i++) {
    }
    Reset(fname);
    return Status::OK();
  }
  void Clear();
};

Status DBImpl::Get(const ReadOptions& options, const Slice& key,
                   std::string* value) {
  return Status::OK();
}

void leveldb_iter_seek(leveldb_iterator_t* iter, const char* k, size_t klen) {
}
typedef NS_OPTIONS(NSUInteger, SDWebImageOptions) {
};
}
";
    let at = |line1: usize, name: &str| container(Kind::C, cpp, line1, name);
    assert_eq!(at(5, "RANDOM").as_deref(), Some("Benchmark::Order"));
    assert_eq!(at(6, "kSlow").as_deref(), Some("Benchmark::Mode"));
    assert_eq!(at(6, "Mode").as_deref(), Some("Benchmark"));
    assert_eq!(at(7, "fname").as_deref(), Some("Benchmark::Open"));
    assert_eq!(at(8, "env"), None);
    assert_eq!(at(7, "Open").as_deref(), Some("Benchmark"));
    assert_eq!(at(9, "i"), None);
    assert_eq!(at(11, "fname"), None);
    assert_eq!(at(17, "options").as_deref(), Some("DBImpl::Get"));
    assert_eq!(at(22, "klen").as_deref(), Some("leveldb_iter_seek"));
    assert_eq!(at(22, "leveldb_iter_seek"), None);
    assert_eq!(at(24, "SDWebImageOptions"), None);
    let call = "void f() {\n  Reset(fname,\n        1);\n  Reset(x, fname,\n        1);\n}\n";
    let one = "void f() {\n  Reset(fname);\n}\n";
    let caller = container(Kind::C, one, 2, "fname");
    assert_eq!(container(Kind::C, call, 2, "fname"), caller);
    assert_eq!(container(Kind::C, call, 4, "fname"), caller);
}

#[test]
fn a_typescript_key_passed_to_a_call_shows_no_class_and_a_dotted_namespace_is_whole() {
    let call = "export const unpublish = createAction({\n  visible: (c) => true,\n});\n";
    assert_eq!(container(Kind::TsJs, call, 2, "visible"), None);
    let schema = "const UserConfigSchema = z.object({\n\tlogo: LogoConfigSchema(),\n});\n";
    assert_eq!(container(Kind::TsJs, schema, 2, "logo").as_deref(), None);
    let object = "export const tariff = {\n  rate: 1,\n};\n";
    assert_eq!(
        container(Kind::TsJs, object, 2, "rate").as_deref(),
        Some("tariff")
    );
    let ns =
        "declare namespace astroHTML.JSX {\n  interface Anchor {\n    href?: string;\n  }\n}\n";
    assert_eq!(
        qualified(Kind::TsJs, ns, 3, "href").as_deref(),
        Some("astroHTML.JSX.Anchor.href")
    );
}

#[test]
fn a_wrapped_header_still_names_its_members() {
    let ts = "export class Tariff<\n  T,\n> {\n  rate = 1;\n}\n";
    assert_eq!(
        qualified(Kind::TsJs, ts, 4, "rate").as_deref(),
        Some("Tariff.rate")
    );
    let py = "class Tariff(\n    Base,\n):\n    rate = 1\n";
    assert_eq!(
        qualified(Kind::Python, py, 4, "rate").as_deref(),
        Some("Tariff.rate")
    );
}

#[test]
fn a_field_is_reached_by_its_whole_name() {
    assert!(in_method("self.rate = 1", "rate"));
    assert!(in_method("return this.rate;", "rate"));
    assert!(!in_method("self.rates = 1", "rate"));
    assert!(!in_method("myself.rate = 1", "rate"));
    assert!(!in_method("rate = 1", "rate"));
}

#[test]
fn a_chain_broken_over_lines_reads_as_one() {
    let lines = |ls: &[&str]| ls.iter().map(ToString::to_string).collect::<Vec<_>>();
    let php = lines(&["$query", "    ->where("]);
    assert_eq!(
        unbroken(Kind::Php, &php, 1, 6),
        Some(LineAsRead {
            line: "$query->where(".into(),
            word_start: 8
        })
    );
    let ts = lines(&["return this.db", "", "  // the table", "  .selectFrom("]);
    assert_eq!(
        unbroken(Kind::TsJs, &ts, 3, 3),
        Some(LineAsRead {
            line: "return this.db.selectFrom(".into(),
            word_start: 15
        }),
        "a comment or a blank line between two links breaks nothing"
    );
}

#[test]
fn a_call_that_starts_a_line_keeps_its_receiver() {
    let lines = |ls: &[&str]| ls.iter().map(ToString::to_string).collect::<Vec<_>>();
    let line = |kind, ls: &[&str], at, col| unbroken(kind, &lines(ls), at, col).map(|l| l.line);
    let java = ["  return service", "    .listReply(name)", "    .map(x)"];
    assert_eq!(
        line(Kind::Jvm, &java, 2, 5).as_deref(),
        Some("  return service.listReply(name).map(x)")
    );
    let swift = ["  AF.request(url)", "    .validate()"];
    assert_eq!(
        line(Kind::Swift, &swift, 1, 5).as_deref(),
        Some("  AF.request(url).validate()")
    );
    for implicit in [
        ["completion(", "  .success(x))"],
        ["f(a,", "  .none)"],
        ["let x: Mode =", "  .fast"],
        ["return", "  .fast"],
    ] {
        assert_eq!(line(Kind::Swift, &implicit, 1, 3), None, "{implicit:?}");
        assert_eq!(line(Kind::Dart, &implicit, 1, 3), None, "{implicit:?}");
    }
    for word in [
        "in", "case", "try", "await", "throw", "else", "is", "as", "some", "any", "yield", "where",
    ] {
        let implicit = [&format!("  x {word}"), "    .fast"];
        assert_eq!(line(Kind::Swift, &implicit, 1, 5), None, "{word}");
    }
    let optional = ["val n = user", "    ?.name"];
    for kind in [Kind::Jvm, Kind::Dart] {
        assert_eq!(
            line(kind, &optional, 1, 6).as_deref(),
            Some("val n = user?.name"),
            "{kind:?}"
        );
    }
    let go = ["\tsess := x.Where(a).", "\t\tAnd(b).", "\t\tAnd(c)"];
    assert_eq!(
        line(Kind::Go, &go, 2, 2).as_deref(),
        Some("\tsess := x.Where(a).And(b).And(c)")
    );
    assert_eq!(line(Kind::Go, &["\tx := 1", "\tf()"], 1, 1), None);
}

#[test]
fn a_member_access_reads_as_a_plain_dot() {
    let plain = |kind, line: &str, start| plain_access(kind, line, start).line;
    assert_eq!(plain(Kind::Swift, "user?.name", 6), "user.name");
    assert_eq!(plain(Kind::TsJs, "user!.name", 6), "user.name");
    assert_eq!(plain(Kind::Php, "$user?->name", 8), "$user.name");
    assert_eq!(
        plain(Kind::Php, "$a.foo()", 3),
        "$a foo()",
        "a PHP dot concatenates"
    );
    let java = "IndexSpecs.<Post, Map<String, ?>>single(x)";
    assert_eq!(plain(Kind::Jvm, java, 33), "IndexSpecs.single(x)");
    assert_eq!(plain(Kind::Jvm, "if (a < b >c)", 11), "if (a < b >c)");
    assert_eq!(plain(Kind::Jvm, "f(x->c)", 5), "f(x->c)");
}

#[test]
fn a_suffix_belongs_to_the_name_but_not_to_an_operator() {
    let word =
        |kind, line: &'static str, col| definition_word(Some(kind), line, col).map(|(_, w)| w);
    assert_eq!(word(Kind::Ruby, "if empty? then", 4), Some("empty?"));
    assert_eq!(word(Kind::Ruby, "if name!= 1", 4), Some("name"));
    assert_eq!(word(Kind::Ruby, "if name!~ /x/", 4), Some("name"));
    assert_eq!(word(Kind::Ruby, "x = @ready?1:0", 6), Some("ready"));
    assert_eq!(word(Kind::Elixir, "ship!(order)", 1), Some("ship!"));
    assert_eq!(word(Kind::Elixir, "if count!= 0", 4), Some("count"));
}

#[test]
fn a_ruby_method_under_a_comment_at_the_left_edge_stays_in_its_block() {
    let text = "class << self\n# the rates\n  def rate\n  end\nend\n";
    assert!(ruby_singleton(text, 3));
}

#[test]
fn a_typescript_receiver_indexed_off_a_name_is_its_element() {
    let head = |line: &str| call_head(Kind::TsJs, line, line.rfind('.').map_or(0, |i| i + 1));
    assert_eq!(
        head("return this.pots[0].lid.lift"),
        Some(CallHead {
            call_without_arguments: "this.pots[]".into(),
            value: Value::Element("this.pots".into()),
            fields_to_word: vec!["lid".into()],
        })
    );
    assert_eq!(head("f()[0].lift"), None);
    assert_eq!(head("[a, b][0].lift"), None);
    assert_eq!(call_head(Kind::Python, "pots[0].lift", 8), None);
}

#[test]
fn a_bare_ruby_loop_keyword_is_no_name() {
    assert!(ruby_loop_keyword("next", "      ", " if done"));
    assert!(ruby_loop_keyword("break", "  x = ", ""));
    assert!(!ruby_loop_keyword("next", "  page.", ""));
    assert!(!ruby_loop_keyword("next", "  page&.", ""));
    assert!(!ruby_loop_keyword("next", "  attribute :", ""));
    assert!(!ruby_loop_keyword("next", "  def ", "(page)"));
    assert!(!ruby_loop_keyword("next", "  link(", ": 1)"));
    assert!(!ruby_loop_keyword("nextt", "  ", ""));
}

#[test]
fn the_at_of_a_ruby_variable_reads_the_variable() {
    let word =
        |line, col| definition_word(Some(Kind::Ruby), line, col).map(|(r, w)| (r, w.to_owned()));
    assert_eq!(word("  @rows.size", 2), Some((3..7, "rows".to_owned())));
    assert_eq!(word("@@beads", 0), Some((2..7, "beads".to_owned())));
    assert_eq!(word("@@beads", 1), Some((2..7, "beads".to_owned())));
    assert_eq!(word("@rows", 1), Some((1..5, "rows".to_owned())));
}

#[test]
fn a_ruby_row_s_class_is_the_class_an_instance_variable_belongs_to_and_a_method_keeps_its_bang() {
    let rb = "module Admin\n  class ReportsController\n    @@count = 0\n    @per_page = 20\n\n    def set_report\n      @report = Report.find(1)\n    end\n\n    def self.cached\n      @cache = {}\n    end\n\n    class << self\n      def built\n        @built = true\n      end\n    end\n\n    def bump; @@count += 1; end\n\n    def publish!\n      due.each do |row|\n        @last = row\n      end\n      count = 1\n    end\n\n    def rate=(value)\n      @rate = value\n    end\n  end\nend\n\nmodule Expireable\n  included do\n    def expires_in=(interval)\n      @expires_in = interval\n    end\n  end\nend\n";
    let class = |line, name| container(Kind::Ruby, rb, line, name);
    let got: Vec<_> = [
        (3, "@@count"),
        (4, "@per_page"),
        (7, "@report"),
        (11, "@cache"),
        (16, "@built"),
        (20, "@@count"),
        (23, "row"),
        (24, "@last"),
        (26, "count"),
        (29, "rate="),
        (30, "@rate"),
        (38, "@expires_in"),
    ]
    .into_iter()
    .map(|(l, n)| format!("{n}={}", class(l, n).unwrap_or_default()))
    .collect();
    assert_eq!(
        got.join(" "),
        "@@count=Admin.ReportsController @per_page=Admin.ReportsController @report=Admin.ReportsController @cache=Admin.ReportsController @built=Admin.ReportsController @@count=Admin.ReportsController row=Admin.ReportsController.publish! @last= count=Admin.ReportsController.publish! rate==Admin.ReportsController @rate=Admin.ReportsController @expires_in="
    );
}

#[test]
fn a_php_row_s_class_is_the_function_of_a_parameter_and_the_class_of_a_promoted_property() {
    let php = "<?php\n\nnamespace App\\Models;\n\nclass Song extends Model\n{\n    public function __construct(public readonly string $value, bool $plain, private(set) int $set)\n    {\n    }\n\n    public function __construct(\n        #[SensitiveParameter] private readonly Result $result,\n        readonly int $ro,\n        string $plain2,\n    ) {\n    }\n\n    public static function query(?PlayableType $type = null, array $ids = [1, 2], &$ref, ...$rest): Builder\n    {\n        $local = 1;\n        $f = function ($arg) use ($local) { return $arg; };\n    }\n\n    public function value(int $value): int\n    {\n    }\n\n    public $prop = 1;\n    private static ?int $sprop;\n}\n\nfunction helper(int $count): int\n{\n}\n\ninterface Playable\n{\n    public function play(int $at);\n}\n";
    let got: Vec<_> = [
        (7, "value"),
        (7, "plain"),
        (7, "set"),
        (12, "result"),
        (13, "ro"),
        (14, "plain2"),
        (18, "type"),
        (18, "ids"),
        (18, "ref"),
        (18, "rest"),
        (18, "query"),
        (20, "local"),
        (21, "arg"),
        (24, "value"),
        (28, "prop"),
        (29, "sprop"),
        (32, "count"),
        (32, "helper"),
        (38, "at"),
    ]
    .into_iter()
    .map(|(l, n)| {
        format!(
            "{n}={}",
            container(Kind::Php, php, l, n).unwrap_or_default()
        )
    })
    .collect();
    assert_eq!(
        got.join(" "),
        "value=Song plain=Song::__construct set=Song result=Song ro=Song plain2= type=Song::query ids=Song::query ref=Song::query rest=Song::query query=Song local=Song::query arg=Song::query value=Song prop=Song sprop=Song count=helper helper= at=Playable::play"
    );
}

#[test]
fn a_sql_row_s_class_is_the_table_of_its_column_and_nothing_else() {
    let sql = "CREATE TABLE user_ (\n    id serial PRIMARY KEY,\n    -- a comment\n    community_id int REFERENCES community ON DELETE CASCADE,\n    CONSTRAINT uq_name UNIQUE (id)\n);\n\nCREATE TABLE IF NOT EXISTS public.post (\n    title text\n);\n\nCREATE VIEW post_view AS\nSELECT\n    p.id,\n    p.title AS heading\nFROM\n    post p;\n\nCREATE OR REPLACE FUNCTION bump (target int)\n    RETURNS void\n    LANGUAGE plpgsql\n    AS $$\nDECLARE\n    counter int := 0;\nBEGIN\n    UPDATE post SET score = score + 1;\nEND\n$$;\n\nALTER TABLE post\n    ADD COLUMN score int;\n\nWITH recent AS (\n    SELECT\n        id\n    FROM\n        post\n)\nSELECT * FROM recent;\n\nCREATE TYPE mood AS ENUM (\n    'sad',\n    'happy'\n);\n\nCREATE INDEX idx_post ON post (\n    title\n);\n\nCREATE TRIGGER t_bump\n    AFTER INSERT ON post\n    FOR EACH ROW\n    EXECUTE FUNCTION bump ();\n";
    let got: Vec<_> = [
        (2, "id"),
        (4, "community_id"),
        (4, "community"),
        (5, "uq_name"),
        (9, "title"),
        (14, "id"),
        (15, "heading"),
        (19, "bump"),
        (19, "target"),
        (24, "counter"),
        (26, "score"),
        (31, "score"),
        (35, "id"),
        (41, "sad"),
        (46, "title"),
        (51, "bump"),
        (45, "idx_post"),
        (8, "post"),
        (8, "public"),
        (1, "user_"),
        (30, "post"),
    ]
    .into_iter()
    .map(|(l, n)| {
        format!(
            "{l}:{n}={}",
            container(Kind::Sql, sql, l, n).unwrap_or_default()
        )
    })
    .collect();
    assert_eq!(
        got.join(" "),
        "2:id=user_ 4:community_id=user_ 4:community= 5:uq_name= 9:title=public.post 14:id= 15:heading= 19:bump= 19:target= 24:counter= 26:score= 31:score= 35:id= 41:sad= 46:title= 51:bump= 45:idx_post= 8:post= 8:public= 1:user_= 30:post="
    );
}
