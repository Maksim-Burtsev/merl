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
