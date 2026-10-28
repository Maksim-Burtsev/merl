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
fn a_macro_clang_s_float_h_undefines_and_defines_again_is_clang_s_after_it() {
    let sdk = external_root(
        "outside-clang-sdk",
        &[
            ("math.h", "#define INFINITY HUGE_VALF\n"),
            ("float.h", "#define FLT_MAX 1e38\n"),
            ("stddef.h", "#include <_null.h>\n"),
            ("_null.h", "#define NULL ((void *)0)\n"),
        ],
    );
    let clang = external_root(
        "outside-clang-include",
        &[
            (
                "float.h",
                "#include <__inf.h>\n#undef FLT_MAX\n#define FLT_MAX 2\n",
            ),
            (
                "__inf.h",
                "#undef INFINITY\n#define INFINITY (__builtin_inff())\n",
            ),
            ("stddef.h", "#undef NULL\n#define NULL ((void*)0)\n"),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-clang",
        &[
            (
                "a.c",
                "#include <math.h>\n#include <float.h>\ndouble a = INFINITY;\n",
            ),
            (
                "b.c",
                "#include <float.h>\n#include <math.h>\ndouble b = INFINITY;\n",
            ),
            (
                "c.c",
                "#include <math.h>\ndouble c = INFINITY;\n#include <float.h>\n",
            ),
            ("d.c", "#include <float.h>\nfloat d = FLT_MAX;\n"),
            ("e.c", "#include <stddef.h>\nvoid *e = NULL;\n"),
        ],
    );
    use_roots(&mut a, Kind::C, std::slice::from_ref(&sdk));
    a.clang_include = std::sync::OnceLock::from(Some(clang.clone()));
    let place = |a: &mut App, file: &str, code: &str| {
        d_on(a, file, code);
        match shown(a) {
            Shown::Jump(_, place) => place,
            picker @ Shown::Picker(..) => format!("{picker:?}"),
        }
    };
    assert_eq!(place(&mut a, "a.c", "= INFINITY"), at(&clang, "__inf.h", 2));
    assert_eq!(place(&mut a, "b.c", "= INFINITY"), at(&sdk, "math.h", 1));
    assert_eq!(place(&mut a, "c.c", "= INFINITY"), at(&sdk, "math.h", 1));
    assert_eq!(place(&mut a, "d.c", "= FLT_MAX"), at(&sdk, "float.h", 1));
    assert_eq!(place(&mut a, "e.c", "= NULL"), at(&sdk, "_null.h", 1));
    for d in [dir, sdk, clang] {
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
fn a_dart_sdk_class_another_library_exports_types_a_chain_however_it_is_reached() {
    let sdk = external_root(
        "outside-dart-export",
        &[
            (
                "lib/core/core.dart",
                "library dart.core;\n\nexport 'dart:async' show Future;\n\npart 'iterable.dart';\n",
            ),
            (
                "lib/async/async.dart",
                "library dart.async;\n\nabstract interface class Future<T> {\n  Future<T> whenComplete(void action());\n}\n",
            ),
            (
                "lib/core/iterable.dart",
                "part of \"dart:core\";\n\nabstract mixin class Iterable<E> {\n  bool get isNotEmpty => true;\n}\n",
            ),
            (
                "lib/libraries.json",
                "{\"vm\": {\"libraries\": {\"_http\": {\"uri\": \"_http/http.dart\"}}}}\n",
            ),
            (
                "lib/io/io.dart",
                "library dart.io;\n\nexport 'dart:_http' show HttpClientResponse;\n",
            ),
            (
                "lib/_http/http.dart",
                "library dart._http;\n\nabstract interface class HttpClientResponse {\n  Iterable<int> get redirects;\n}\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-dart-export-project",
        &[(
            "lib/main.dart",
            "import 'dart:async';\nimport 'dart:io';\n\nclass Hops {\n  bool get isNotEmpty => false;\n  void whenComplete() {}\n}\n\nFuture<bool> fetch(Future<HttpClientResponse> future) async {\n  late final HttpClientResponse response;\n  try {\n    response = await future;\n  } catch (_) {\n    future.whenComplete(() {});\n    rethrow;\n  }\n  return response.redirects.isNotEmpty;\n}\n",
        )],
    );
    use_roots(&mut a, Kind::Dart, &[sdk.join("lib")]);
    d_on(&mut a, "lib/main.dart", "response.redirects.isNotEmpty");
    assert_eq!(
        shown(&mut a),
        jump(
            "isNotEmpty \u{2192} Iterable.isNotEmpty (via Iterable)",
            &at(&sdk, "lib/core/iterable.dart", 4)
        )
    );
    d_on(&mut a, "lib/main.dart", "future.whenComplete");
    assert_eq!(
        shown(&mut a),
        jump(
            "whenComplete \u{2192} Future.whenComplete (via Future)",
            &at(&sdk, "lib/async/async.dart", 4)
        )
    );
    for d in [dir, sdk] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_dart_sdk_class_reads_a_parent_of_its_own_library_and_a_list_literal_is_a_list() {
    let sdk = external_root(
        "outside-dart-own-library",
        &[
            (
                "lib/core/core.dart",
                "library dart.core;\n\npart 'list.dart';\npart 'iterable.dart';\n",
            ),
            (
                "lib/core/list.dart",
                "part of \"dart:core\";\n\nabstract interface class List<E> implements Iterable<E> {\n  void add(E value);\n}\n",
            ),
            (
                "lib/core/iterable.dart",
                "part of \"dart:core\";\n\nabstract mixin class Iterable<E> {\n  bool contains(Object? element) => false;\n}\n",
            ),
            (
                "lib/io/io.dart",
                "library dart.io;\n\npart 'file.dart';\npart 'file_system_entity.dart';\n",
            ),
            (
                "lib/io/file.dart",
                "part of \"dart:io\";\n\nabstract interface class File implements FileSystemEntity {\n  factory File(String path) => throw 0;\n}\n",
            ),
            (
                "lib/io/file_system_entity.dart",
                "part of \"dart:io\";\n\nabstract class FileSystemEntity {\n  bool existsSync();\n}\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-dart-own-library-project",
        &[(
            "lib/main.dart",
            "import 'dart:io';\n\nclass Disk {\n  bool existsSync() => false;\n  bool contains(Object? o) => false;\n}\n\nbool check(String path, String word) {\n  final File file;\n  if (path.isEmpty) {\n    file = File('a');\n  } else {\n    file = File(path);\n  }\n  return file.existsSync() && ['a', 'b'].contains(word);\n}\n",
        )],
    );
    use_roots(&mut a, Kind::Dart, &[sdk.join("lib")]);
    d_on(&mut a, "lib/main.dart", "file.existsSync");
    assert_eq!(
        shown(&mut a),
        jump(
            "existsSync \u{2192} FileSystemEntity.existsSync (via FileSystemEntity)",
            &at(&sdk, "lib/io/file_system_entity.dart", 4)
        )
    );
    d_on(&mut a, "lib/main.dart", "'b'].contains");
    assert_eq!(
        shown(&mut a),
        jump(
            "contains \u{2192} Iterable.contains (via Iterable)",
            &at(&sdk, "lib/core/iterable.dart", 4)
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
fn a_dart_sdk_class_written_through_an_import_prefix_keeps_its_class_and_constructor() {
    let sdk = external_root(
        "outside-dart-prefixed",
        &[
            (
                "lib/core/core.dart",
                "library dart.core;\n\npart 'errors.dart';\n",
            ),
            (
                "lib/core/errors.dart",
                "part of dart.core;\n\nclass StateError extends Error {\n  StateError(this.message);\n}\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "outside-dart-prefixed-project",
        &[(
            "lib/main.dart",
            "import 'dart:core' as core;\n\nfinal e = core.StateError('x');\n",
        )],
    );
    use_roots(&mut a, Kind::Dart, &[sdk.join("lib")]);
    d_on(&mut a, "lib/main.dart", "core.|StateError(");
    assert_eq!(
        shown(&mut a),
        picker(
            "StateError: by name, 2 declarations",
            &[
                ("StateError", "core/errors.dart:3"),
                ("StateError.StateError", "core/errors.dart:4"),
            ]
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

#[test]
fn a_dart_name_skips_the_packages_the_library_cannot_see() {
    let cache = external_root(
        "outside-dart-unimported",
        &[
            (
                "stack/lib/stack.dart",
                "class Trace {}\n\nclass Base extends Root {}\n\nvoid ping() {}\n",
            ),
            (
                "wrap/lib/wrap.dart",
                "export 'src/stub.dart' if (dart.library.io) 'src/io.dart';\n",
            ),
            ("wrap/lib/src/stub.dart", "class Stub {}\n"),
            ("wrap/lib/src/io.dart", "export 'package:deep/deep.dart';\n"),
            ("deep/lib/deep.dart", "class Frame {}\n"),
            (
                "noise/lib/noise.dart",
                "class Trace {}\n\nclass Frame {\n  Frame();\n}\n\nclass Root {\n  void ping() {}\n}\n\nclass Lost {}\n",
            ),
        ],
    );
    let packages = ["stack", "wrap", "deep", "noise"].map(|p| {
        format!(
            "{{\"name\": \"{p}\", \"rootUri\": \"file://{}\", \"packageUri\": \"lib/\"}}",
            cache.join(p).display()
        )
    });
    let config = format!(
        "{{\"configVersion\": 2, \"packages\": [{}]}}\n",
        packages.join(", ")
    );
    let (dir, mut a) = project_app(
        "outside-dart-unimported-project",
        &[
            ("pubspec.yaml", "name: app\n"),
            (".dart_tool/package_config.json", &config),
            (
                "lib/main.dart",
                "import 'package:stack/stack.dart';\nimport 'package:wrap/wrap.dart';\n\nTrace t() => Trace();\nFrame f() => Frame();\n\nclass Mine extends Base {\n  void go() {\n    ping();\n  }\n}\n\nLost l() => Lost();\n",
            ),
        ],
    );
    let roots = ["stack", "wrap", "deep", "noise"].map(|p| cache.join(p).join("lib"));
    use_roots(&mut a, Kind::Dart, &roots);
    d_on(&mut a, "lib/main.dart", "^|Trace t()");
    assert_eq!(
        shown(&mut a),
        jump(
            "Trace: by name, 1 match",
            &at(&cache, "stack/lib/stack.dart", 1)
        )
    );
    d_on(&mut a, "lib/main.dart", "^|Frame f()");
    assert_eq!(
        shown(&mut a),
        jump(
            "Frame: by name, 1 match",
            &at(&cache, "deep/lib/deep.dart", 1)
        )
    );
    d_on(&mut a, "lib/main.dart", "|ping();");
    let Shown::Picker(_, rows) = shown(&mut a) else {
        panic!("an inherited member stays beside a top-level namesake");
    };
    assert_eq!(
        rows.into_iter()
            .map(|(_, _, place)| place)
            .collect::<Vec<_>>(),
        ["stack.dart:5", "noise.dart:8"]
    );
    d_on(&mut a, "lib/main.dart", "^|Lost l()");
    assert_eq!(
        shown(&mut a),
        jump(
            "Lost: by name, 1 match",
            &at(&cache, "noise/lib/noise.dart", 11)
        ),
        "a name no candidate of which is reached keeps them all"
    );
    for d in [dir, cache] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_dart_import_that_does_not_resolve_keeps_every_package() {
    let cache = external_root(
        "outside-dart-unresolved",
        &[
            ("stack/lib/stack.dart", "class Trace {}\n"),
            ("noise/lib/noise.dart", "class Trace {}\n"),
        ],
    );
    let packages = ["stack", "noise"].map(|p| {
        format!(
            "{{\"name\": \"{p}\", \"rootUri\": \"file://{}\", \"packageUri\": \"lib/\"}}",
            cache.join(p).display()
        )
    });
    let config = format!(
        "{{\"configVersion\": 2, \"packages\": [{}]}}\n",
        packages.join(", ")
    );
    let (dir, mut a) = project_app(
        "outside-dart-unresolved-project",
        &[
            ("pubspec.yaml", "name: app\n"),
            (".dart_tool/package_config.json", &config),
            (
                "lib/main.dart",
                "import 'package:stack/stack.dart';\nimport 'gone.g.dart';\n\nTrace t() => Trace();\n",
            ),
        ],
    );
    let roots = ["stack", "noise"].map(|p| cache.join(p).join("lib"));
    use_roots(&mut a, Kind::Dart, &roots);
    d_on(&mut a, "lib/main.dart", "^|Trace t()");
    let Shown::Picker(_, rows) = shown(&mut a) else {
        panic!("an import that does not resolve leaves the walk undecided");
    };
    assert_eq!(
        rows.into_iter()
            .map(|(_, _, place)| place)
            .collect::<Vec<_>>(),
        ["stack.dart:1", "noise.dart:1"]
    );
    for d in [dir, cache] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_dart_awaited_getter_of_a_package_class_reads_the_future_its_factory_wraps() {
    let cache = external_root(
        "outside-dart-awaited",
        &[
            ("sdk/lib/core/core.dart", "library dart.core;\n"),
            ("async/lib/async.dart", "export 'src/op.dart';\n"),
            (
                "async/lib/src/op.dart",
                "class Op<T> {\n  factory Op.fromFuture(\n    Future<T> result, {\n    Object? onCancel,\n  }) =>\n      throw 0;\n\n  Future<T> get value => throw 0;\n}\n",
            ),
        ],
    );
    let config = format!(
        "{{\"configVersion\": 2, \"packages\": [{{\"name\": \"async\", \"rootUri\": \"file://{}\", \"packageUri\": \"lib/\"}}]}}\n",
        cache.join("async").display()
    );
    let (dir, mut a) = project_app(
        "outside-dart-awaited-project",
        &[
            ("pubspec.yaml", "name: app\n"),
            (".dart_tool/package_config.json", &config),
            (
                "lib/main.dart",
                "import 'package:async/async.dart';\n\nclass Body {\n  int get headers => 0;\n}\n\nclass Options {\n  int get headers => 1;\n}\n\nabstract class Adapter {\n  Future<Body> fetch(int n);\n}\n\nmixin Client {\n  late Adapter adapter;\n\n  Future<int> send() async {\n    final operation = Op.fromFuture(\n      adapter.fetch(\n        1,\n      ),\n    );\n    final body = await operation.value;\n    return body.headers;\n  }\n}\n",
            ),
        ],
    );
    use_roots(
        &mut a,
        Kind::Dart,
        &[cache.join("sdk/lib"), cache.join("async/lib")],
    );
    d_on(&mut a, "lib/main.dart", "body.headers");
    assert_eq!(
        shown(&mut a),
        jump(
            "headers \u{2192} Body.headers (via Body)",
            "lib/main.dart:4"
        )
    );
    for d in [dir, cache] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_php_helper_call_lands_on_the_one_global_function_composer_loads() {
    let ctl = "<?php\n\nnamespace App\\Http;\n\nclass Ctl\n{\n    public function __invoke(?string $n): array\n    {\n        return [\n            'a' => env('X'),\n            'b' => $n ?: config('k'),\n            'c' => str_contains($n, 'x'),\n            'd' => dupe(),\n            'e' => stray(),\n        ];\n    }\n}\n";
    let helpers = "<?php\n\nif (! function_exists('env')) {\n    function env($key, $default = null)\n    {\n    }\n}\n\nif (! function_exists('config')) {\n    function config($key = null)\n    {\n    }\n}\n\nfunction dupe()\n{\n}\n";
    let di = "<?php\n\nnamespace DI;\n\nfunction env(string $name)\n{\n}\n";
    let repo = "<?php\n\nnamespace Illuminate\\Config;\n\nclass Repository\n{\n    public function config()\n    {\n    }\n\n    public function str_contains()\n    {\n    }\n\n    public function stray()\n    {\n    }\n}\n";
    let polyfill = "<?php\n\nif (! function_exists('str_contains')) {\n    function str_contains($h, $n)\n    {\n    }\n}\n";
    let listing = "<?php\n\n$vendorDir = dirname(__DIR__);\n$baseDir = dirname($vendorDir);\n\nreturn array(\n    'a1' => $vendorDir . '/laravel/src/helpers.php',\n    'b2' => $vendorDir . '/php-di/src/functions.php',\n    'c3' => $vendorDir . '/symfony/polyfill-php80/bootstrap.php',\n    'd4' => $vendorDir . '/other/dupe.php',\n);\n";
    let files = [
        (
            "composer.json",
            r#"{"autoload": {"psr-4": {"App\\": "app/"}}}"#,
        ),
        (".gitignore", "vendor/\n"),
        ("app/Http/Ctl.php", ctl),
        ("vendor/composer/autoload_files.php", listing),
        ("vendor/laravel/src/helpers.php", helpers),
        ("vendor/laravel/src/Config/Repository.php", repo),
        ("vendor/php-di/src/functions.php", di),
        ("vendor/symfony/polyfill-php80/bootstrap.php", polyfill),
        ("vendor/other/dupe.php", "<?php\n\nfunction dupe()\n{\n}\n"),
        (
            "vendor/other/tests/stray.php",
            "<?php\n\nfunction stray()\n{\n}\n",
        ),
    ];
    let (dir, mut a) = project_app("php-helper-call", &files);
    let mut d = |code: &str| {
        d_on(&mut a, "app/Http/Ctl.php", code);
        shown(&mut a)
    };
    let got = [
        d("env"),
        d("?: config"),
        d("dupe"),
        d("str_contains"),
        d("stray"),
    ];
    drop(std::fs::remove_dir_all(dir));
    let repo_row = |name: &'static str, line: usize| {
        (name, format!("laravel/src/Config/Repository.php:{line}"))
    };
    let (contains, stray) = (repo_row("str_contains", 11), repo_row("stray", 15));
    assert_eq!(
        got,
        [
            jump("env: by name, 1 match", "vendor/laravel/src/helpers.php:4"),
            jump(
                "config: by name, 1 match",
                "vendor/laravel/src/helpers.php:10"
            ),
            picker(
                "dupe: by name, 2 declarations",
                &[
                    ("dupe", "laravel/src/helpers.php:15"),
                    ("dupe", "other/dupe.php:3")
                ]
            ),
            picker(
                "str_contains: by name, 2 declarations",
                &[
                    ("Repository::str_contains", contains.1.as_str()),
                    ("str_contains", "symfony/polyfill-php80/bootstrap.php:4")
                ]
            ),
            picker(
                "stray: by name, 2 declarations",
                &[
                    ("Repository::stray", stray.1.as_str()),
                    ("stray", "other/tests/stray.php:3")
                ]
            ),
        ],
        "after `=>` and `?:` a call is a call; DI\\env and a config() method are no namesakes of \
         the global functions; two global dupe()s, a polyfill of a builtin and a file composer \
         never loads keep the picker"
    );
}

#[test]
fn a_php_method_a_project_class_inherits_from_vendor_lands_on_its_one_declaration() {
    let user = "<?php\n\nnamespace App\\Models;\n\nuse Illuminate\\Foundation\\Auth\\User as Authenticatable;\n\nclass User extends Authenticatable\n{\n}\n";
    let song =
        "<?php\n\nnamespace App\\Models;\n\nuse Twice\\Base;\n\nclass Song extends Base\n{\n}\n";
    let ctl = "<?php\n\nnamespace App\\Http;\n\nuse App\\Models\\Song;\nuse App\\Models\\User;\n\nclass Ctl\n{\n    public function __invoke(User $user, Song $song): void\n    {\n        $user->refresh();\n        $user->getAttribute('a');\n        $user->getKey();\n        $user->where('a', 1);\n        $song->fresh();\n    }\n}\n";
    let auth = "<?php\n\nnamespace Illuminate\\Foundation\\Auth;\n\nuse Illuminate\\Database\\Eloquent\\Model;\n\nclass User extends Model\n{\n    use \\Illuminate\\Auth\\Authenticatable;\n}\n";
    let authenticatable = "<?php\n\nnamespace Illuminate\\Auth;\n\ntrait Authenticatable\n{\n    public function getKey()\n    {\n    }\n}\n";
    let model = "<?php\n\nnamespace Illuminate\\Database\\Eloquent;\n\nabstract class Model\n{\n    use Concerns\\HasEvents,\n        Concerns\\HasAttributes;\n\n    public function refresh()\n    {\n    }\n\n    public function getKey()\n    {\n    }\n\n    public function __call($method, $parameters)\n    {\n    }\n}\n";
    let attributes = "<?php\n\nnamespace Illuminate\\Database\\Eloquent\\Concerns;\n\ntrait HasAttributes\n{\n    public function getAttribute($key)\n    {\n    }\n}\n";
    let events =
        "<?php\n\nnamespace Illuminate\\Database\\Eloquent\\Concerns;\n\ntrait HasEvents\n{\n}\n";
    let builder = "<?php\n\nnamespace Illuminate\\Database\\Eloquent;\n\nclass Builder\n{\n    public function where($column, $value)\n    {\n    }\n\n    public function refresh()\n    {\n    }\n}\n";
    let base = "<?php\n\nnamespace Twice;\n\nclass Base\n{\n    public function fresh()\n    {\n    }\n}\n";
    let e = "vendor/laravel/framework/src/Illuminate/Database/Eloquent";
    let files = [
        (
            "composer.json",
            r#"{"autoload": {"psr-4": {"App\\": "app/"}}}"#,
        ),
        (".gitignore", "vendor/\n"),
        ("app/Models/User.php", user),
        ("app/Models/Song.php", song),
        ("app/Http/Ctl.php", ctl),
        (
            "vendor/laravel/framework/src/Illuminate/Foundation/Auth/User.php",
            auth,
        ),
        (&format!("{e}/Model.php"), model),
        (&format!("{e}/Builder.php"), builder),
        (
            "vendor/laravel/framework/src/Illuminate/Database/Query/Builder.php",
            "<?php\n\nnamespace Illuminate\\Database\\Query;\n\nclass Builder\n{\n    public function where($column)\n    {\n    }\n}\n",
        ),
        (
            "vendor/laravel/framework/src/Illuminate/Auth/Authenticatable.php",
            authenticatable,
        ),
        (&format!("{e}/Concerns/HasAttributes.php"), attributes),
        (&format!("{e}/Concerns/HasEvents.php"), events),
        ("vendor/a/src/Base.php", base),
        ("vendor/b/src/Base.php", base),
    ];
    let (dir, mut a) = project_app("php-vendor-parent", &files);
    let mut d = |code: &str| {
        d_on(&mut a, "app/Http/Ctl.php", code);
        shown(&mut a)
    };
    let got = [
        d("$user->refresh"),
        d("$user->getAttribute"),
        d("$user->getKey"),
        d("$user->where"),
        d("$song->fresh"),
    ];
    drop(std::fs::remove_dir_all(dir));
    let f = "laravel/framework/src/Illuminate/Database";
    let song = |place: &str| {
        let why = "via $song: Song".to_owned();
        ("Base::fresh".to_owned(), why, place.to_owned())
    };
    assert_eq!(
        got,
        [
            jump(
                "refresh \u{2192} Model::refresh (via $user: User)",
                &format!("vendor/{f}/Eloquent/Model.php:10"),
            ),
            jump(
                "getAttribute \u{2192} HasAttributes::getAttribute (via $user: User)",
                &format!("vendor/{f}/Eloquent/Concerns/HasAttributes.php:7"),
            ),
            jump(
                "getKey \u{2192} Authenticatable::getKey (via $user: User)",
                "vendor/laravel/framework/src/Illuminate/Auth/Authenticatable.php:7",
            ),
            picker(
                "where: by name, 2 declarations",
                &[
                    ("Builder::where", &format!("{f}/Eloquent/Builder.php:7")),
                    ("Builder::where", &format!("{f}/Query/Builder.php:7"))
                ]
            ),
            Shown::Picker(
                "fresh: via $song: Song, 2 declarations".to_owned(),
                vec![song("a/src/Base.php:7"), song("b/src/Base.php:7")]
            ),
        ],
        "a method declared up a vendor parent chain, traits included and a trait before the \
         parent, is the one PHP calls; a method only __call answers, and a parent two vendor \
         files declare, keep the picker"
    );
}

#[test]
fn a_php_chain_answers_only_when_every_link_declares_what_it_returns() {
    let e = "vendor/laravel/framework/src/Illuminate/Database/Eloquent";
    let model = "<?php\n\nnamespace Illuminate\\Database\\Eloquent;\n\nabstract class Model\n{\n    /**\n     * @return \\Illuminate\\Database\\Eloquent\\Builder<static>\n     */\n    public static function query()\n    {\n    }\n\n    public static function __callStatic($method, $parameters)\n    {\n    }\n}\n";
    let builder = "<?php\n\nnamespace Illuminate\\Database\\Eloquent;\n\nclass Builder\n{\n    /**\n     * @param  string  $column\n     * @return $this\n     */\n    public function where($column, $value)\n    {\n    }\n\n    /**\n     * @return $this|null\n     */\n    public function maybe()\n    {\n    }\n\n    /** @return $this */\n    public function oneLine()\n    {\n    }\n\n    public function find($id)\n    {\n    }\n\n    public function __call($method, $parameters)\n    {\n    }\n\n    /** @return static */\n    public static function make()\n    {\n    }\n}\n";
    let album = "<?php\n\nnamespace App\\Models;\n\nuse App\\Builders\\AlbumBuilder;\nuse Illuminate\\Database\\Eloquent\\Model;\n\nclass Album extends Model\n{\n    public static function query(): AlbumBuilder\n    {\n    }\n}\n";
    let song = "<?php\n\nnamespace App\\Models;\n\nuse App\\Builders\\SongBuilder;\nuse Illuminate\\Database\\Eloquent\\Attributes\\UseEloquentBuilder;\nuse Illuminate\\Database\\Eloquent\\Model;\n\n#[UseEloquentBuilder(SongBuilder::class)]\nclass Song extends Model\n{\n}\n";
    let album_builder = "<?php\n\nnamespace App\\Builders;\n\nuse Illuminate\\Database\\Eloquent\\Builder;\n\nclass AlbumBuilder extends Builder\n{\n    public function onlyStandard(): self\n    {\n    }\n\n    /** @return $this */\n    public function toSong(): SongBuilder\n    {\n    }\n}\n";
    let song_builder = "<?php\n\nnamespace App\\Builders;\n\nuse Illuminate\\Database\\Eloquent\\Builder;\n\nclass SongBuilder extends Builder\n{\n    public function where($column, $value)\n    {\n    }\n\n    public function find($id)\n    {\n    }\n\n    public function onlyStandard(): self\n    {\n    }\n}\n";
    let ctl = "<?php\n\nnamespace App\\Http;\n\nuse App\\Builders\\AlbumBuilder;\nuse App\\Models\\Album;\nuse App\\Models\\Song;\n\nclass Ctl\n{\n    public function __invoke(): void\n    {\n        Album::query()->find(1);\n        Album::query()\n            ->where('a.b', \"c\")\n            ->onlyStandard();\n        Song::query()->find(2);\n        Album::query()->maybe()->find(3);\n        Album::query()->oneLine()->find(4);\n        Album::magic()->find(5);\n        Album::query()->where(')', 1)->find(6);\n    }\n\n    public function more(AlbumBuilder $b): void\n    {\n        $q = $b->oneLine();\n        $q->onlyStandard();\n        $m = AlbumBuilder::make();\n        $m->onlyStandard();\n        Album::query()->toSong()->find(9);\n    }\n}\n";
    let files = [
        (
            "composer.json",
            r#"{"autoload": {"psr-4": {"App\\": "app/"}}}"#,
        ),
        (".gitignore", "vendor/\n"),
        ("app/Models/Album.php", album),
        ("app/Models/Song.php", song),
        ("app/Builders/AlbumBuilder.php", album_builder),
        ("app/Builders/SongBuilder.php", song_builder),
        ("app/Http/Ctl.php", ctl),
        (&format!("{e}/Model.php"), model),
        (&format!("{e}/Builder.php"), builder),
    ];
    let (dir, mut a) = project_app("php-chain", &files);
    let mut d = |code: &str| {
        d_on(&mut a, "app/Http/Ctl.php", code);
        shown(&mut a)
    };
    let got = [
        d("Album::query()->find"),
        d("->onlyStandard"),
        d("Song::query()->find"),
        d("maybe()->find"),
        d("oneLine()->find"),
        d("magic()->find"),
        d("1)->find"),
        d("$q->onlyStandard"),
        d("$m->onlyStandard"),
        d("toSong()->find"),
    ];
    drop(std::fs::remove_dir_all(dir));
    let finds = || {
        picker(
            "find: by name, 2 declarations",
            &[
                ("SongBuilder::find", "app/Builders/SongBuilder.php:13"),
                (
                    "Builder::find",
                    "laravel/framework/src/Illuminate/Database/Eloquent/Builder.php:27",
                ),
            ],
        )
    };
    assert_eq!(
        got,
        [
            jump(
                "find \u{2192} Builder::find (via Album::query(): AlbumBuilder)",
                &format!("{e}/Builder.php:27"),
            ),
            jump(
                "onlyStandard \u{2192} AlbumBuilder::onlyStandard (via Album::query(): AlbumBuilder \u{2192} where(): static)",
                "app/Builders/AlbumBuilder.php:9",
            ),
            finds(),
            finds(),
            jump(
                "find \u{2192} Builder::find (via Album::query(): AlbumBuilder \u{2192} oneLine(): static)",
                &format!("{e}/Builder.php:27"),
            ),
            finds(),
            finds(),
            jump(
                "onlyStandard \u{2192} AlbumBuilder::onlyStandard (via $b->oneLine(): static)",
                "app/Builders/AlbumBuilder.php:9",
            ),
            jump(
                "onlyStandard \u{2192} AlbumBuilder::onlyStandard (via AlbumBuilder::make(): static)",
                "app/Builders/AlbumBuilder.php:9",
            ),
            jump(
                "find \u{2192} SongBuilder::find (via Album::query(): AlbumBuilder \u{2192} toSong(): SongBuilder)",
                "app/Builders/SongBuilder.php:13",
            ),
        ],
        "a chain from a static call answers when each link's return type is declared natively or \
         as a lone `@return $this` / `static`, quoted arguments included; Model::query()'s \
         documented Builder<static>, which a custom builder replaces, a `$this|null`, \
         __callStatic magic and a paren in a string keep the picker"
    );
}

#[test]
fn a_php_static_call_on_a_vendor_class_lands_on_what_that_one_class_declares() {
    let ctl = "<?php\n\nnamespace App\\Http;\n\nuse Illuminate\\Support\\Arr;\nuse Illuminate\\Support\\Collection;\nuse Illuminate\\Support\\Facades\\File;\nuse Twice\\Base;\n\nclass Ctl\n{\n    public function __invoke(array $a): void\n    {\n        Arr::get($a, 'k');\n        Collection::wrap($a);\n        File::fake();\n        File::exists('x');\n        Base::make();\n    }\n}\n";
    let arr = "<?php\n\nnamespace Illuminate\\Support;\n\nclass Arr\n{\n    public static function get($array, $key)\n    {\n    }\n}\n";
    let env = "<?php\n\nnamespace Illuminate\\Support;\n\nclass Env\n{\n    public static function get($key)\n    {\n    }\n}\n";
    let collection = "<?php\n\nnamespace Illuminate\\Support;\n\nclass Collection\n{\n    use Traits\\EnumeratesValues;\n}\n";
    let values = "<?php\n\nnamespace Illuminate\\Support\\Traits;\n\ntrait EnumeratesValues\n{\n    public static function wrap($value)\n    {\n    }\n}\n";
    let file =
        "<?php\n\nnamespace Illuminate\\Support\\Facades;\n\nclass File extends Facade\n{\n}\n";
    let facade = "<?php\n\nnamespace Illuminate\\Support\\Facades;\n\nabstract class Facade\n{\n    public static function fake()\n    {\n    }\n\n    public static function __callStatic($method, $args)\n    {\n    }\n}\n";
    let filesystem = "<?php\n\nnamespace Illuminate\\Filesystem;\n\nclass Filesystem\n{\n    public function exists($path)\n    {\n    }\n}\n";
    let base = "<?php\n\nnamespace Twice;\n\nclass Base\n{\n    public static function make()\n    {\n    }\n}\n";
    let s = "vendor/laravel/framework/src/Illuminate";
    let files = [
        (
            "composer.json",
            r#"{"autoload": {"psr-4": {"App\\": "app/"}}}"#,
        ),
        (".gitignore", "vendor/\n"),
        ("app/Http/Ctl.php", ctl),
        (&format!("{s}/Collections/Arr.php"), arr),
        (&format!("{s}/Support/Env.php"), env),
        (&format!("{s}/Collections/Collection.php"), collection),
        (
            &format!("{s}/Collections/Traits/EnumeratesValues.php"),
            values,
        ),
        (&format!("{s}/Support/Facades/File.php"), file),
        (&format!("{s}/Support/Facades/Facade.php"), facade),
        (&format!("{s}/Filesystem/Filesystem.php"), filesystem),
        ("vendor/a/src/Base.php", base),
        ("vendor/b/src/Base.php", base),
    ];
    let (dir, mut a) = project_app("php-vendor-static", &files);
    let mut d = |code: &str| {
        d_on(&mut a, "app/Http/Ctl.php", code);
        shown(&mut a)
    };
    let got = [
        d("Arr::get"),
        d("Collection::wrap"),
        d("File::fake"),
        d("File::exists"),
        d("Base::make"),
        d("^use Illuminate\\Support\\Arr"),
    ];
    drop(std::fs::remove_dir_all(dir));
    let via = |class: &str| format!("via import Illuminate/Support/{class}");
    assert_eq!(
        got,
        [
            jump(
                &format!("get \u{2192} Arr::get ({})", via("Arr")),
                &format!("{s}/Collections/Arr.php:7"),
            ),
            jump(
                &format!(
                    "wrap \u{2192} EnumeratesValues::wrap ({})",
                    via("Collection")
                ),
                &format!("{s}/Collections/Traits/EnumeratesValues.php:7"),
            ),
            jump(
                &format!("fake \u{2192} Facade::fake ({})", via("Facades/File")),
                &format!("{s}/Support/Facades/Facade.php:7"),
            ),
            jump(
                "exists \u{2192} Filesystem::exists (by name, 1 match)",
                &format!("{s}/Filesystem/Filesystem.php:7"),
            ),
            jump("no definition for make", "app/Http/Ctl.php:18"),
            jump("Arr: via Arr", &format!("{s}/Collections/Arr.php:5")),
        ],
        "a static call on the one vendor class its import names lands on what that class, its \
         traits and its parents declare, wherever its file lives; a method only __callStatic \
         answers, and a class two vendor files declare, stay as they were; a class name lands on \
         the class"
    );
}

#[test]
fn a_php_static_call_on_a_vendor_class_the_project_also_declares_keeps_its_answer() {
    let ctl = "<?php\n\nnamespace App\\Http;\n\nuse Illuminate\\Support\\Arr;\n\nclass Ctl\n{\n    public function __invoke(array $a): void\n    {\n        Arr::get($a, 'k');\n    }\n}\n";
    let arr = "<?php\n\nnamespace Illuminate\\Support;\n\nclass Arr\n{\n    public static function get($array, $key)\n    {\n    }\n}\n";
    let stub = "<?php\n\nnamespace Illuminate\\Support;\n\nclass Arr\n{\n    public static function get($array, $key, $default = null)\n    {\n    }\n}\n";
    let mine = "<?php\n\nnamespace App\\Support;\n\nclass Arr\n{\n    public static function get($key)\n    {\n    }\n}\n";
    let s = "vendor/laravel/framework/src/Illuminate";
    let files = [
        (
            "composer.json",
            r#"{"autoload": {"psr-4": {"App\\": "app/"}}}"#,
        ),
        (".gitignore", "vendor/\n"),
        ("app/Http/Ctl.php", ctl),
        ("app/Support/Arr.php", mine),
        (&format!("{s}/Collections/Arr.php"), arr),
    ];
    let (dir, mut a) = project_app("php-vendor-static-mine", &files);
    d_on(&mut a, "app/Http/Ctl.php", "Arr::get");
    let only_vendor = shown(&mut a);
    let (dir2, mut a) = project_app(
        "php-vendor-static-stub",
        &[files.as_slice(), &[("ide_helper.php", stub)]].concat(),
    );
    d_on(&mut a, "app/Http/Ctl.php", "Arr::get");
    let stubbed = shown(&mut a);
    let empty = "<?php\n\nnamespace Illuminate\\Support;\n\nclass Arr\n{\n}\n";
    let (dir3, mut a) = project_app(
        "php-vendor-static-empty-stub",
        &[files.as_slice(), &[("ide_helper.php", empty)]].concat(),
    );
    d_on(&mut a, "app/Http/Ctl.php", "Arr::get");
    let empty_stub = shown(&mut a);
    drop(std::fs::remove_dir_all(dir));
    drop(std::fs::remove_dir_all(dir2));
    drop(std::fs::remove_dir_all(dir3));
    assert_eq!(
        [only_vendor, stubbed, empty_stub],
        [
            jump(
                "get \u{2192} Arr::get (via import Illuminate/Support/Arr)",
                &format!("{s}/Collections/Arr.php:7"),
            ),
            picker(
                "get: by name, 2 declarations",
                &[
                    ("Arr::get", "app/Support/Arr.php:7"),
                    ("Arr::get", "ide_helper.php:7"),
                ],
            ),
            jump(
                "get \u{2192} Arr::get (via import Illuminate/Support/Arr)",
                &format!("{s}/Collections/Arr.php:7"),
            ),
        ],
        "a project class sharing only the short name leaves the vendor class's answer; one \
         declaring the vendor class itself keeps the search it had, unless it declares no such \
         member"
    );
}
