//! Dart's rules (#414): what declares a name and what does not, what `D` lists, the literals that
//! run over lines, a `$` in a name, the imports and the files they name, and the roots.

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
        ("  String get name => _name;", "name"),
        ("  set name(String value) {", "name"),
        ("final cache = <String, User>{};", "cache"),
        ("  static const int limit = 5;", "limit"),
        ("  late final Database db;", "db"),
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
    // A constructor and an enum value declare inside their type's body.
    let user = "class User {\n  const User({required this.id});\n  factory User.fromJson(Map<String, dynamic> json) =>\n  User.guest() : id = 0;\n  User(this.id);\n}\nenum Level {\n  low,\n  high('h');\n}\n";
    assert!(declares(user, 2, "User"));
    assert!(declares(user, 3, "fromJson"));
    assert!(declares(user, 4, "guest"));
    assert!(declares(user, 5, "User"));
    assert!(declares(user, 8, "low"));
    assert!(declares(user, 9, "high"));
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
    ] {
        assert!(!one(line, word), "{line} declares no {word}");
    }
    // A call shaped like a constructor or a value is none outside a type's or an enum's body.
    let body = "void f() {\n  const SizedBox(height: 8);\n  open,\n}\n";
    assert!(!declares(body, 2, "SizedBox"));
    assert!(!declares(body, 3, "open"));
    // A parameter wrapped onto a line of its own is no field.
    let params = "void f({\n  int retries = 3,\n  String name,\n}) {}\n";
    assert!(!declares(params, 2, "retries"));
    assert!(!declares(params, 3, "name"));
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
    let text = "const a = '''\nclass A {\n''';\nconst b = r\"\"\"\nclass B {\n\"\"\";\n/*\nclass C {\n*/\nconst t = '`';\nclass D {}\n";
    let literal: Vec<usize> = literal_lines(Kind::Dart, text)
        .iter()
        .enumerate()
        .filter(|(_, l)| **l)
        .map(|(i, _)| i + 1)
        .collect();
    // A backtick opens nothing: `class D` is code.
    assert_eq!(literal, [2, 3, 5, 6, 8, 9]);
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
    // In a string, `$name` interpolates `name`.
    assert_eq!(word("print('hi $name');", 12).as_deref(), Some("name"));
}

#[test]
fn dart_imports_bind_a_prefix_and_shown_names() {
    let text = "import 'package:http/http.dart' as http;\nimport 'a.dart'\n    show A, B hide C;\nimport 'dart:async';\nimport 'x.dart' deferred as x;\n";
    assert_eq!(
        dart_imports(text),
        [
            ("http".to_owned(), "package:http/http.dart".to_owned()),
            ("A".to_owned(), "a.dart".to_owned()),
            ("B".to_owned(), "a.dart".to_owned()),
            ("x".to_owned(), "x.dart".to_owned()),
        ]
    );
}

#[test]
fn dart_uris_name_files() {
    let cache = std::env::temp_dir().join(format!("merl-dart-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    std::fs::create_dir_all(cache.join("http-1.2.0/lib")).unwrap();
    std::fs::write(
        cache.join("http-1.2.0/lib/http.dart"),
        "Future get(Uri u) => x;\n",
    )
    .unwrap();
    let sdk = cache.join("sdk");
    std::fs::create_dir_all(sdk.join("lib/async")).unwrap();
    std::fs::write(sdk.join("lib/async/async.dart"), "library dart.async;\n").unwrap();
    let config = format!(
        r#"{{"configVersion": 2, "packages": [
  {{"name": "http", "rootUri": "file://{}/http-1.2.0", "packageUri": "lib/", "languageVersion": "3.0"}},
  {{"name": "shop", "rootUri": "../", "packageUri": "lib/"}}
]}}"#,
        cache.display()
    );
    let (root, _) = scratch(
        "dart-uris",
        &[
            ("app/pubspec.yaml", "name: shop\n"),
            ("app/.dart_tool/package_config.json", &config),
            ("app/lib/money.dart", "String formatPrice(int c) => '';\n"),
            ("app/lib/cart.dart", ""),
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
        Some(cache.join("http-1.2.0/lib/http.dart"))
    );
    assert_eq!(file("dart:async"), Some(sdk.join("lib/async/async.dart")));
    assert_eq!(file("package:missing/x.dart"), None);
    // The packages outside the project and the SDK's `lib/`; the project's own is not one.
    assert_eq!(
        dart_roots(&root, Some(sdk.clone())),
        [cache.join("http-1.2.0/lib/"), sdk.join("lib")]
    );
    let _ = std::fs::remove_dir_all(&cache);
    let _ = std::fs::remove_dir_all(&root);
}
