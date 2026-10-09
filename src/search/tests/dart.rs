use super::*;

/// Whether `word` is declared on line 1-based `at` of `text` by `d`'s patterns and where it sits.
fn declares(text: &str, at: usize, word: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines[at - 1];
    Regex::new(&dart_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
        && declares_where(Kind::Dart, Path::new("a.dart"), word, at, line, || &lines)
}

#[test]
fn dart_declaration_forms() {
    let one = |line: &str, word: &str| declares(line, 1, word);
    for (line, word) in [
        ("class Cart {", "Cart"),
        ("abstract class Shape {", "Shape"),
        ("sealed class Result {", "Result"),
        ("base class Animal {", "Animal"),
        ("final class Money {", "Money"),
        ("abstract interface class Repo {", "Repo"),
        ("mixin class Walker {", "Walker"),
        ("mixin Walker on Animal {", "Walker"),
        ("extension StringX on String {", "StringX"),
        ("extension type UserId(int id) {", "UserId"),
        ("enum Status { open, closed }", "Status"),
        ("enum Status { open, closed }", "closed"),
        ("typedef Json = Map<String, dynamic>;", "Json"),
        ("typedef void Callback(int code);", "Callback"),
        ("String formatPrice(int cents) =>", "formatPrice"),
        ("Future<void> main() async {", "main"),
        ("T first<T>(List<T> items) {", "first"),
        ("main() {", "main"),
        ("  Widget build(BuildContext context) {", "build"),
        ("  Future<User> find(int id);", "find"),
        ("  static Money parse(String s) =>", "parse"),
        ("  @override String toString() => '';", "toString"),
        ("  db.Database open(String path) {", "open"),
        ("  Future<Map<String, List<int>>> load() async {", "load"),
        ("  String get name => _name;", "name"),
        ("  set name(String value) {", "name"),
        ("final cache = <String, User>{};", "cache"),
        ("  static const int limit = 5;", "limit"),
        ("  late final Database db;", "db"),
        ("  late final sql.connection conn;", "conn"),
        ("  String? label;", "label"),
        ("  int count = 0;", "count"),
        (
            "Tariff _$TariffFromJson(Json json) => Tariff();",
            "_$TariffFromJson",
        ),
        ("class $TariffCopyWith {}", "$TariffCopyWith"),
    ] {
        assert!(one(line, word), "{line} declares {word}");
    }
    let user = "class User {\n  const User({required this.id});\n  factory User.fromJson(Map<String, dynamic> json) =>\n  User.guest() : id = 0;\n  User(this.id);\n}\nenum Level {\n  low,\n  high('h');\n}\n";
    assert!(declares(user, 2, "User"));
    assert!(declares(user, 3, "fromJson"));
    assert!(declares(user, 4, "guest"));
    assert!(
        declares(user, 5, "User"),
        "a constructor declares inside its type's body"
    );
    assert!(
        declares(user, 8, "low"),
        "an enum value declares inside its enum's body"
    );
    assert!(declares(user, 9, "high"));
    let more = "@immutable class Shop {\n  const Shop();\n  Shop.named(String name, int id);\n  Shop.copy(super.key);\n}\n@JsonEnum() enum Mode {\n  @JsonValue('o')\n  open,\n}\n";
    assert!(declares(more, 2, "Shop"));
    assert!(
        declares(more, 3, "named"),
        "a constructor with typed parameters"
    );
    assert!(
        declares(more, 4, "copy"),
        "a constructor with `super.` parameters"
    );
    assert!(
        declares(more, 8, "open"),
        "a value under an enum annotated on its line"
    );
    let optional = "class Cart {\n  Cart({this.items});\n  Cart.of([this.items]);\n}\nenum Tone {\n  soft;\n  const Tone();\n}\nextension type Cents(int value) {\n  Cents.zero() : value = 0;\n}\n";
    assert!(declares(optional, 2, "Cart"), "named parameters");
    assert!(declares(optional, 3, "of"), "optional positional ones");
    assert!(declares(optional, 7, "Tone"), "a constructor in an enum");
    assert!(
        declares(optional, 10, "zero"),
        "a constructor in an extension type"
    );
}

#[test]
fn dart_refusals() {
    let one = |line: &str, word: &str| declares(line, 1, word);
    for (line, word) in [
        ("  Navigator.push(context, route);", "push"),
        ("  setState(() {", "setState"),
        ("  return Foo(x);", "Foo"),
        ("  await repo.find(id);", "find"),
        ("  throw StateError('x');", "StateError"),
        ("  User.empty();", "empty"),
        ("  build(context) {", "build"),
        ("extension on String {", "on"),
        ("  case Circle(:final radius):", "radius"),
        ("  return x;", "x"),
        ("  if (a == b) return;", "a"),
        ("  Status current == next;", "current"),
        ("    Circle c => pi * c.r * c.r,", "c"),
    ] {
        assert!(!one(line, word), "{line} declares no {word}");
    }
    let body = "void f() {\n  const SizedBox(height: 8);\n  open,\n}\n";
    assert!(
        !declares(body, 2, "SizedBox"),
        "a call shaped like a constructor is none outside a type's body"
    );
    assert!(
        !declares(body, 3, "open"),
        "a value is none outside an enum's body"
    );
    let params = "void f({\n  int retries = 3,\n  String name,\n}) {}\n";
    assert!(
        !declares(params, 2, "retries"),
        "a parameter wrapped onto a line of its own is no field"
    );
    assert!(!declares(params, 3, "name"));
    let positional = "void f(\n  int retries,\n) {}\n";
    assert!(!declares(positional, 2, "retries"));
}

#[test]
fn dart_constructor_only_where_it_is_built() {
    let narrowed = |line: &str, word: &str| {
        let mut p = dart_patterns(word);
        let at = line.find(word).unwrap();
        dart_narrow(&mut p, line, at..at + word.len());
        Regex::new(&p.join("|"))
            .unwrap()
            .is_match("  const User({required this.id});")
    };
    assert!(narrowed("final u = User(id: 1);", "User"));
    assert!(!narrowed("User? current;", "User"));
}

#[test]
fn dart_symbol_names() {
    let dart = |line| listed(Kind::Dart, line);
    for (line, name) in [
        ("abstract interface class Repo {", "Repo"),
        ("sealed class Result<T> {", "Result"),
        ("mixin Walker on Animal {", "Walker"),
        ("extension StringX on String {", "StringX"),
        ("extension type UserId(int id) {", "UserId"),
        ("enum Status { open, closed }", "Status"),
        ("typedef Json = Map<String, dynamic>;", "Json"),
        ("typedef void Callback(int code);", "Callback"),
        ("String formatPrice(int cents) =>", "formatPrice"),
        ("main() {", "main"),
        ("  Widget build(BuildContext context) {", "build"),
        ("  static Money parse(String s) =>", "parse"),
        ("  String get name => _name;", "name"),
        ("  set name(String value) {", "name"),
        ("  Future<Database> get database async {", "database"),
        ("class Node<T extends Comparable<T>> {", "Node"),
        ("Future<(int, String)> fetch() async {", "fetch"),
    ] {
        assert_eq!(dart(line), [name], "{line}");
    }
    for none in [
        "extension on String {",
        "  String? label;",
        "  int count = 0;",
        "final cache = {};",
        "  const User({required this.id});",
        "  User.guest() : id = 0;",
        "  open,",
        "  return Foo(x);",
        "  setState(() {",
        "import 'package:http/http.dart' as http;",
    ] {
        assert!(dart(none).is_empty(), "{none}: {:?}", dart(none));
    }
}

#[test]
fn dart_literals_run_over_lines() {
    let text = "const a = '''\nclass A {\n''';\nconst b = r\"\"\"\nclass B {\n\"\"\";\n/*\nclass C {\n*/\nx = `;\nclass D {}\n";
    let literal: Vec<usize> = literal_lines(Kind::Dart, text)
        .iter()
        .enumerate()
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(
        literal,
        [2, 3, 5, 6, 8, 9],
        "a backtick opens nothing: `class D` is code"
    );
}

#[test]
fn a_dart_name_keeps_its_dollar() {
    let word =
        |line: &str, col| definition_word(Some(Kind::Dart), line, col).map(|(_, w)| w.to_owned());
    assert_eq!(
        word("final c = $UserCopyWith(u);", 12).as_deref(),
        Some("$UserCopyWith")
    );
    assert_eq!(
        word("final u = _$UserFromJson(j);", 14).as_deref(),
        Some("_$UserFromJson")
    );
    assert_eq!(
        word("print('hi $name');", 12).as_deref(),
        Some("name"),
        "in a string, `$name` interpolates `name`"
    );
    assert_eq!(word("print('$a$b');", 10).as_deref(), Some("b"));
    assert_eq!(word("final x = y$;", 10).as_deref(), Some("y"));
}

#[test]
fn dart_imports_bind_a_prefix_and_shown_names() {
    let text = "import 'package:http/http.dart' as http;\nimport \"a.dart\"\n    show A, B hide C;\nimport 'dart:async';\nimport 'x.dart' deferred as x;\n";
    let bound = |name: &str, uri: &str, is_prefix| DartImport {
        name: name.to_owned(),
        uri_as_written: uri.to_owned(),
        is_prefix,
    };
    assert_eq!(
        dart_imports(text),
        [
            bound("http", "package:http/http.dart", true),
            bound("A", "a.dart", false),
            bound("B", "a.dart", false),
            bound("x", "x.dart", true),
        ]
    );
}

#[test]
fn dart_uris_name_files() {
    let cache = std::env::temp_dir().join(format!("merl dart cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    std::fs::create_dir_all(cache.join("http-1.2.0/lib")).unwrap();
    std::fs::write(
        cache.join("http-1.2.0/lib/http.dart"),
        "Future get(Uri u) => x;\n",
    )
    .unwrap();
    std::fs::create_dir_all(cache.join("sky_engine/lib/ui")).unwrap();
    std::fs::write(
        cache.join("sky_engine/lib/ui/ui.dart"),
        "library dart.ui;\n",
    )
    .unwrap();
    let sdk = cache.join("sdk");
    std::fs::create_dir_all(sdk.join("lib/async")).unwrap();
    std::fs::write(sdk.join("lib/async/async.dart"), "library dart.async;\n").unwrap();
    let config = format!(
        r#"{{"configVersion": 2, "packages": [
  {{"name": "http", "rootUri": "file://{0}/http-1.2.0", "packageUri": "lib/", "languageVersion": "3.0"}},
  {{"name": "sky_engine", "rootUri": "file://{0}/sky_engine", "packageUri": "lib/"}},
  {{"name": "core", "rootUri": "../../packages/core", "packageUri": "lib/"}},
  {{"name": "shop", "rootUri": "../", "packageUri": "lib/"}}
]}}"#,
        cache.display().to_string().replace(' ', "%20")
    );
    let nested = format!(
        r#"{{"packages": [{{"name": "path", "rootUri": "file://{}/path-1.9.0", "packageUri": "lib/"}}]}}"#,
        cache.display().to_string().replace(' ', "%20")
    );
    let (root, _) = scratch(
        "dart-uris",
        &[
            ("app/pubspec.yaml", "name: shop\n"),
            ("app/.dart_tool/package_config.json", &config),
            ("app/lib/money.dart", "String formatPrice(int c) => '';\n"),
            ("app/lib/cart.dart", ""),
            ("packages/core/pubspec.yaml", "name: \"core\"\n"),
            ("packages/core/lib/x.dart", "int x = 1;\n"),
            ("packages/core/.dart_tool/package_config.json", &nested),
        ],
    );
    let here = Path::new("app/lib/cart.dart");
    let file = |uri| dart_uri_file(&root, here, uri, Some(&sdk));
    assert_eq!(
        file("money.dart"),
        Some(PathBuf::from("app/lib/money.dart"))
    );
    assert_eq!(
        file("package:shop/money.dart"),
        Some(PathBuf::from("app/lib/money.dart"))
    );
    assert_eq!(
        file("package:http/http.dart"),
        Some(cache.join("http-1.2.0/lib/http.dart")),
        "a space in the pub cache's path is `%20` in its URI"
    );
    assert_eq!(file("dart:async"), Some(sdk.join("lib/async/async.dart")));
    assert_eq!(file("package:missing/x.dart"), None);
    assert_eq!(
        file("../lib/money.dart"),
        Some(PathBuf::from("app/lib/money.dart"))
    );
    assert_eq!(
        file("package:core/x.dart"),
        Some(PathBuf::from("packages/core/lib/x.dart")),
        "a path dependency of the project is a project file"
    );
    assert_eq!(
        file("dart:ui"),
        Some(cache.join("sky_engine/lib/ui/ui.dart"))
    );
    assert_eq!(
        dart_roots(&root, Some(sdk.clone())),
        [
            cache.join("http-1.2.0/lib/"),
            cache.join("sky_engine/lib/"),
            cache.join("path-1.9.0/lib/"),
            sdk.join("lib")
        ],
        "the packages outside the project and the SDK's `lib/`, not the project's own"
    );
    std::fs::create_dir_all(cache.join("flutter/bin/cache/dart-sdk")).unwrap();
    std::fs::write(cache.join("flutter/bin/dart"), "").unwrap();
    std::fs::create_dir_all(sdk.join("bin")).unwrap();
    std::fs::write(sdk.join("bin/dart"), "").unwrap();
    let real = |p: PathBuf| std::fs::canonicalize(p).unwrap();
    assert_eq!(
        dart_sdk_of(&cache.join("flutter/bin/dart")),
        Some(real(cache.join("flutter/bin/cache/dart-sdk"))),
        "Flutter's `bin/dart` stands beside `bin/cache/dart-sdk`"
    );
    assert_eq!(
        dart_sdk_of(&sdk.join("bin/dart")),
        Some(real(sdk)),
        "a plain SDK's is in its `bin/`"
    );
    let _ = std::fs::remove_dir_all(&cache);
    let _ = std::fs::remove_dir_all(&root);

    let (own, _) = scratch(
        "dart-own-package",
        &[
            ("pubspec.yaml", "name: 'own'\n"),
            ("lib/a.dart", ""),
            ("lib/src/b.dart", ""),
        ],
    );
    assert_eq!(
        dart_uri_file(
            &own,
            Path::new("lib/src/b.dart"),
            "package:own/a.dart",
            None
        ),
        Some(PathBuf::from("lib/a.dart")),
        "the package's own name needs no `pub get`"
    );
    let _ = std::fs::remove_dir_all(&own);
}
