use super::*;

fn at(root: &Path, file: &str, line: usize) -> String {
    format!("{}:{line}", root.join(file).display())
}

#[test]
fn a_python_import_of_nothing_installed_finds_nothing_outside() {
    let site = external_root(
        "outside-missing-site",
        &[("otherlib/tools.py", "def thing():\n    pass\n")],
    );
    let (dir, mut a) = project_app(
        "outside-missing",
        &[
            ("app/x.py", "from missing import thing\n\nthing()\n"),
            ("app/y.py", "from .gone import thing\n\nthing()\n"),
        ],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&site));
    for file in ["app/y.py", "app/x.py"] {
        d_on(&mut a, file, "^thing");
        assert_eq!(a.message, "no definition for thing", "{file}");
    }
    for d in [dir, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_python_module_relaxed_past_the_import_is_found_by_name() {
    let site = external_root(
        "outside-relax-site",
        &[
            ("lib/__init__.py", ""),
            ("lib/tools.py", "def thing():\n    pass\n"),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-relax",
        &[("app/x.py", "from lib.gone import thing\n\nthing()\n")],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&site));
    d_on(&mut a, "app/x.py", "^thing");
    assert_eq!(
        shown(&mut a),
        jump("thing: by name, 1 match", &at(&site, "lib/tools.py", 1))
    );
    for d in [dir, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_project_python_import_never_takes_a_module_shorter_than_it_names() {
    let (dir, mut a) = project_app(
        "outside-floor",
        &[
            (
                "pkg/__init__.py",
                "class gone:\n    def thing(self):\n        pass\n",
            ),
            ("app.py", "from pkg.gone import thing\n\nthing()\n"),
        ],
    );
    a.no_external();
    d_on(&mut a, "app.py", "^thing");
    assert_eq!(a.message, "no definition for thing");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_member_of_a_js_global_is_read_from_typescript_lib_and_node_types_only() {
    let (dir, mut a) = project_app(
        "outside-global",
        &[("src/main.ts", "export const v = JSON.parse(\"{}\");\n")],
    );
    let parse = "interface JSON {\n    parse(text: string): any;\n}\n";
    for (path, text) in [
        ("node_modules/typescript/lib/lib.es5.d.ts", parse),
        ("node_modules/json5/lib/index.d.ts", parse),
    ] {
        std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        std::fs::write(dir.join(path), text).unwrap();
    }
    d_on(&mut a, "src/main.ts", "JSON.parse");
    assert_eq!(
        shown(&mut a),
        jump(
            "parse \u{2192} JSON.parse (by name, 1 match)",
            "node_modules/typescript/lib/lib.es5.d.ts:2"
        )
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_function_local_to_a_dependency_is_not_offered() {
    let (dir, mut a) = project_app("outside-local", &[("src/main.ts", "helper();\n")]);
    for (path, text) in [
        (
            "node_modules/one/index.js",
            "export function run() {\n    function helper() {}\n    helper();\n}\n",
        ),
        ("node_modules/two/index.js", "export function helper() {}\n"),
    ] {
        std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        std::fs::write(dir.join(path), text).unwrap();
    }
    d_on(&mut a, "src/main.ts", "^helper");
    assert_eq!(
        shown(&mut a),
        jump("helper: by name, 1 match", "node_modules/two/index.js:1")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn c_reads_no_cpp_header_and_only_objective_c_reads_the_pods() {
    let inc = external_root(
        "outside-c-inc",
        &[
            ("stdio.h", "int puts(const char *s);\n"),
            ("c++/v1/cstdio.h", "int puts(const char *s);\n"),
        ],
    );
    let pods = external_root(
        "outside-c-pods",
        &[("Pods/Lib/Lib.h", "int puts(const char *s);\n")],
    );
    let pods_root = pods.join("Pods");
    let call = "int main(void) {\n    puts(\"x\");\n    return 0;\n}\n";
    let (dir, mut a) = project_app(
        "outside-c",
        &[
            ("main.c", call),
            ("main.m", &format!("// main.m\n{call}")),
            ("main.mm", call),
        ],
    );
    use_roots(&mut a, Kind::C, &[inc.clone(), pods_root]);
    let rows = |a: &mut App, file: &str| {
        d_on(a, file, "puts");
        match shown(a) {
            Shown::Jump(_, place) => vec![place],
            Shown::Picker(_, rows) => rows.into_iter().map(|(_, _, place)| place).collect(),
        }
    };
    assert_eq!(rows(&mut a, "main.c"), [at(&inc, "stdio.h", 1)]);
    assert_eq!(rows(&mut a, "main.m"), ["stdio.h:1", "Lib/Lib.h:1"]);
    assert_eq!(
        rows(&mut a, "main.mm"),
        ["c++/v1/cstdio.h:1", "stdio.h:1", "Lib/Lib.h:1"]
    );
    for d in [dir, inc, pods] {
        drop(std::fs::remove_dir_all(d));
    }
}

#[test]
fn a_dart_qualifier_is_no_directory() {
    let pub_cache = external_root(
        "outside-dart",
        &[(
            "foo/lib/src/widgets.dart",
            "class Thing {\n  static void make() {}\n}\n",
        )],
    );
    let (dir, mut a) = project_app(
        "outside-dart-project",
        &[("lib/main.dart", "void main() {\n  Thing.make();\n}\n")],
    );
    use_roots(&mut a, Kind::Dart, std::slice::from_ref(&pub_cache));
    d_on(&mut a, "lib/main.dart", "Thing.make");
    assert_eq!(
        shown(&mut a),
        jump(
            "make \u{2192} Thing.make (by name, 1 match)",
            &at(&pub_cache, "foo/lib/src/widgets.dart", 2)
        )
    );
    for d in [dir, pub_cache] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_dart_receiver_of_an_sdk_type_answers_from_the_sdk() {
    let sdk = external_root(
        "outside-dart-sdk",
        &[
            ("lib/core/core.dart", "library dart.core;\n"),
            (
                "lib/async/async.dart",
                "library dart.async;\n\npart 'future.dart';\n",
            ),
            (
                "lib/async/future.dart",
                "part of dart.async;\n\nabstract interface class Completer<T> {\n  bool get isCompleted;\n}\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-dart-sdk-project",
        &[(
            "lib/main.dart",
            "import 'dart:async';\n\nclass Handler {\n  bool get isCompleted => false;\n}\n\nvoid run() {\n  final Completer<int> completer = Completer<int>();\n  if (completer.isCompleted) {}\n}\n",
        )],
    );
    use_roots(&mut a, Kind::Dart, &[sdk.join("lib")]);
    d_on(&mut a, "lib/main.dart", "completer.isCompleted");
    assert_eq!(
        shown(&mut a),
        jump(
            "isCompleted \u{2192} Completer.isCompleted (via Completer)",
            &at(&sdk, "lib/async/future.dart", 4)
        )
    );
    for d in [dir, sdk] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_dart_sdk_name_skips_the_patch_files_and_a_static_reads_its_class() {
    let sdk = external_root(
        "outside-dart-patch",
        &[
            (
                "lib/core/core.dart",
                "library dart.core;\n\npart 'uri.dart';\npart 'print.dart';\n",
            ),
            (
                "lib/core/uri.dart",
                "part of dart.core;\n\nabstract interface class Uri {\n  static Uri parse(String s) => throw 0;\n}\n",
            ),
            (
                "lib/core/print.dart",
                "part of dart.core;\n\nvoid print(Object? o) {}\n",
            ),
            (
                "lib/_internal/vm/core_patch.dart",
                "@patch\nclass Uri {\n  @patch\n  static Uri parse(String s) => throw 0;\n}\n\n@patch\nvoid print(Object? o) {}\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-dart-patch-project",
        &[
            (
                "lib/main.dart",
                "void main() {\n  Uri.parse('x');\n  print(1);\n}\n",
            ),
            (
                "lib/other.dart",
                "class Url {\n  static Url parse(String s) => Url();\n}\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Dart, &[sdk.join("lib")]);
    d_on(&mut a, "lib/main.dart", "Uri.parse");
    assert_eq!(
        shown(&mut a),
        jump(
            "parse \u{2192} Uri.parse (via Uri)",
            &at(&sdk, "lib/core/uri.dart", 4)
        )
    );
    d_on(&mut a, "lib/main.dart", "print");
    assert_eq!(
        shown(&mut a),
        jump(
            "print: by name, 1 match",
            &at(&sdk, "lib/core/print.dart", 3)
        )
    );
    for d in [dir, sdk] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_called_dart_sdk_class_lands_on_its_constructor_and_a_written_one_on_the_class() {
    let sdk = external_root(
        "outside-dart-constructor",
        &[
            (
                "lib/core/core.dart",
                "library dart.core;\n\npart 'errors.dart';\n",
            ),
            (
                "lib/core/errors.dart",
                "part of dart.core;\n\nclass StateError extends Error {\n  final String message;\n  StateError(this.message);\n}\n\n@pragma(1)\nabstract interface class Completer<T> {\n  factory Completer() => throw 0;\n}\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-dart-constructor-project",
        &[(
            "lib/main.dart",
            "StateError fail() {\n  throw StateError('x');\n}\n\nfinal c = Completer<int>();\n",
        )],
    );
    use_roots(&mut a, Kind::Dart, &[sdk.join("lib")]);
    d_on(&mut a, "lib/main.dart", "throw |StateError(");
    assert_eq!(
        shown(&mut a),
        jump(
            "StateError \u{2192} StateError.StateError (by name, 1 match)",
            &at(&sdk, "lib/core/errors.dart", 5)
        )
    );
    d_on(&mut a, "lib/main.dart", "^|StateError fail");
    assert_eq!(
        shown(&mut a),
        jump(
            "StateError: by name, 1 match",
            &at(&sdk, "lib/core/errors.dart", 3)
        )
    );
    d_on(&mut a, "lib/main.dart", "c = |Completer<int>");
    assert_eq!(
        shown(&mut a),
        jump(
            "Completer \u{2192} Completer.Completer (by name, 1 match)",
            &at(&sdk, "lib/core/errors.dart", 10)
        )
    );
    for d in [dir, sdk] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_component_reads_as_its_script_with_the_rest_blank() {
    let (dir, a) = project_app(
        "outside-component",
        &[(
            "C.vue",
            "<template>\n  <p>{{ x }}</p>\n</template>\n<script>\nexport default {}\n</script>\n",
        )],
    );
    assert_eq!(
        a.text_of(Path::new("C.vue")).as_deref(),
        Some("\n\n\n\nexport default {}\n")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_path_outside_is_shown_from_its_root() {
    let (dir, mut a) = project_app("outside-rel", &[("mix.exs", "")]);
    let site = dir.join("venv/lib/python3.13");
    use_roots(
        &mut a,
        Kind::Python,
        &[site.clone(), site.join("site-packages")],
    );
    let file = site.join("site-packages/foo/__init__.py");
    assert_eq!(
        a.rel_to_its_root(Kind::Python, &file),
        Path::new("foo/__init__.py")
    );
    let deps = dir.join("deps");
    a.external
        .insert(Kind::Elixir, (vec![deps], Arc::new(Vec::new())));
    let jason = dir.join("deps/jason/lib/jason.ex");
    assert_eq!(
        a.rel_to_its_root(Kind::Elixir, &jason),
        Path::new("deps/jason/lib/jason.ex")
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_php_receiver_typed_with_a_class_outside_the_project_reads_vendor_only() {
    let builder = "<?php\n\nnamespace App\\Builders;\n\nuse Illuminate\\Database\\Query\\JoinClause;\n\nclass ArtistBuilder\n{\n    public function joined(): void\n    {\n        $this->leftJoin('t', function (JoinClause $join): void {\n            $join->on('a', 'b');\n            $join->where('c', 1);\n        });\n    }\n\n    /**\n     * @param Scanner $user\n     */\n    public function __invoke(JoinClause $user): void\n    {\n        $user->on('x');\n    }\n}\n";
    let scanner = "<?php\n\nnamespace App\\Services;\n\nclass Scanner\n{\n    public function on(string $event): void\n    {\n    }\n\n    public function where(): void\n    {\n    }\n}\n";
    let join = "<?php\n\nnamespace Illuminate\\Database\\Query;\n\nclass JoinClause extends Builder\n{\n    public function on($first, $second)\n    {\n    }\n}\n";
    let query = "<?php\n\nnamespace Illuminate\\Database\\Query;\n\nclass Builder\n{\n    public function where($column, $value)\n    {\n    }\n}\n";
    let files = [
        (
            "composer.json",
            r#"{"autoload": {"psr-4": {"App\\": "app/"}}}"#,
        ),
        (".gitignore", "vendor/\n"),
        ("app/Builders/ArtistBuilder.php", builder),
        ("app/Services/Scanner.php", scanner),
        ("vendor/laravel/Query/JoinClause.php", join),
        ("vendor/laravel/Query/Builder.php", query),
    ];
    for vendor in [false, true] {
        let (dir, mut a) = project_app("php-outside-receiver", &files);
        if !vendor {
            a.no_external();
        }
        let mut d = |code: &str| {
            d_on(&mut a, "app/Builders/ArtistBuilder.php", code);
            shown(&mut a)
        };
        let scanner_on = ("Scanner::on", "app/Services/Scanner.php:7");
        let by_name = if vendor {
            picker(
                "on: by name, 2 declarations",
                &[
                    scanner_on,
                    ("JoinClause::on", "laravel/Query/JoinClause.php:7"),
                ],
            )
        } else {
            jump("on \u{2192} Scanner::on (by name, 1 match)", scanner_on.1)
        };
        let (on, where_) = if vendor {
            (
                jump(
                    "on \u{2192} JoinClause::on (via $join: JoinClause)",
                    "vendor/laravel/Query/JoinClause.php:7",
                ),
                jump(
                    "where \u{2192} Builder::where (via $join: JoinClause)",
                    "vendor/laravel/Query/Builder.php:7",
                ),
            )
        } else {
            (
                jump("no definition for on", "app/Builders/ArtistBuilder.php:12"),
                jump(
                    "no definition for where",
                    "app/Builders/ArtistBuilder.php:13",
                ),
            )
        };
        assert_eq!(d("$join->on"), on, "vendor: {vendor}");
        assert_eq!(d("$join->where"), where_, "vendor: {vendor}");
        assert_eq!(
            d("$user->on"),
            by_name,
            "a docblock's @param keeps the search by name: vendor: {vendor}"
        );
        drop(std::fs::remove_dir_all(dir));
    }
}
