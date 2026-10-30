use super::*;

fn write_outside(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        std::fs::write(dir.join(path), text).unwrap();
    }
}

#[test]
fn a_package_installed_twice_is_read_from_the_copy_node_loads() {
    let main = "function f(node) {\n    node.getText();\n}\n";
    let (dir, mut a) = project_app("ts-nearest", &[("src/main.ts", main)]);
    let typescript = "export interface Node {\n    getText(): string;\n}\n";
    write_outside(
        &dir,
        &[
            ("node_modules/typescript/lib/typescript.d.ts", typescript),
            (
                "node_modules/@arethetypeswrong/core/node_modules/typescript/lib/typescript.d.ts",
                typescript,
            ),
            (
                "node_modules/@arethetypeswrong/core/node_modules/solo/index.d.ts",
                "export interface Solo {\n    getText(): string;\n}\n",
            ),
        ],
    );
    d_on(&mut a, "src/main.ts", "node.getText");
    assert_eq!(
        shown(&mut a),
        picker(
            "getText: by name, 2 declarations",
            &[
                (
                    "Solo.getText",
                    "@arethetypeswrong/core/node_modules/solo/index.d.ts:2"
                ),
                ("Node.getText", "typescript/lib/typescript.d.ts:2"),
            ]
        ),
        "the copy another package keeps of typescript is not read; a package only it has is"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_pnpm_store_keeps_the_version_the_project_links_to() {
    let main = "function f(node) {\n    node.getText();\n}\n";
    let (dir, mut a) = project_app("ts-pnpm", &[("src/main.ts", main)]);
    let text = |t: &str| format!("export interface {t} {{\n    getText(): string;\n}}\n");
    write_outside(
        &dir,
        &[
            (
                "node_modules/.pnpm/lib@2.0.0/node_modules/lib/index.d.ts",
                &text("Two"),
            ),
            (
                "node_modules/.pnpm/lib@1.0.0/node_modules/lib/index.d.ts",
                &text("One"),
            ),
            (
                "node_modules/.pnpm/other@1.0.0/node_modules/other/index.d.ts",
                &text("Other"),
            ),
        ],
    );
    std::os::unix::fs::symlink(
        dir.join("node_modules/.pnpm/lib@2.0.0/node_modules/lib"),
        dir.join("node_modules/lib"),
    )
    .unwrap();
    d_on(&mut a, "src/main.ts", "node.getText");
    assert_eq!(
        shown(&mut a),
        picker(
            "getText: by name, 2 declarations",
            &[
                (
                    "Two.getText",
                    ".pnpm/lib@2.0.0/node_modules/lib/index.d.ts:2"
                ),
                (
                    "Other.getText",
                    ".pnpm/other@1.0.0/node_modules/other/index.d.ts:2"
                ),
            ]
        ),
        "the store's other version of lib is not read; the one linked in and a package only the store has are"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_member_outside_is_looked_for_in_the_imported_packages_first() {
    let main = "import ts from \"typescript\";\nimport { x } from \"./x\";\n\nfunction f(node) {\n    node.getText();\n    node.lonely();\n}\n";
    let (dir, mut a) = project_app(
        "ts-reach",
        &[("src/main.ts", main), ("src/x.ts", "export const x = 1;\n")],
    );
    write_outside(
        &dir,
        &[
            (
                "node_modules/typescript/lib/typescript.d.ts",
                "export interface Node {\n    getText(): string;\n    lonely(): void;\n}\nexport interface Token {\n    getText(): string;\n}\n",
            ),
            (
                "node_modules/unrelated/index.d.ts",
                "export interface Doc {\n    getText(): string;\n    lonely(): void;\n}\n",
            ),
        ],
    );
    d_on(&mut a, "src/main.ts", "node.getText");
    assert_eq!(
        shown(&mut a),
        picker(
            "getText: by name, 2 declarations",
            &[
                ("Node.getText", "typescript/lib/typescript.d.ts:2"),
                ("Token.getText", "typescript/lib/typescript.d.ts:6"),
            ]
        ),
        "the imported package declares it twice: no namesake of another package is offered"
    );
    d_on(&mut a, "src/main.ts", "node.lonely");
    assert_eq!(
        shown(&mut a),
        picker(
            "lonely: by name, 2 declarations",
            &[
                ("Node.lonely", "typescript/lib/typescript.d.ts:3"),
                ("Doc.lonely", "unrelated/index.d.ts:3"),
            ]
        ),
        "one declaration in the imported package is no proof: every package is read, as before"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_member_only_a_nested_copy_declares_is_still_offered() {
    let main = "import { make } from \"a\";\n\nconst n = make();\nn.onlyV2();\n";
    let (dir, mut a) = project_app("ts-nested-only", &[("src/main.ts", main)]);
    write_outside(
        &dir,
        &[
            (
                "node_modules/a/index.d.ts",
                "import { B } from \"b\";\nexport declare function make(): B;\n",
            ),
            (
                "node_modules/b/index.d.ts",
                "export interface B {\n    v1(): void;\n}\n",
            ),
            (
                "node_modules/a/node_modules/b/index.d.ts",
                "export interface B {\n    v1(): void;\n    onlyV2(): void;\n}\n",
            ),
            (
                "node_modules/unrelated/index.d.ts",
                "export interface U {\n    onlyV2(): void;\n}\n",
            ),
        ],
    );
    d_on(&mut a, "src/main.ts", "n.onlyV2");
    assert_eq!(
        shown(&mut a),
        picker(
            "onlyV2: by name, 2 declarations",
            &[
                ("B.onlyV2", "a/node_modules/b/index.d.ts:3"),
                ("U.onlyV2", "unrelated/index.d.ts:2"),
            ]
        ),
        "the version of b that a returns declares it: it stays offered beside the namesake"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_reexport_from_a_nested_copy_lands_in_that_copy() {
    let main = "import { X } from \"other\";\n\nX;\n";
    let (dir, mut a) = project_app("ts-nested-reexport", &[("src/main.ts", main)]);
    write_outside(
        &dir,
        &[
            (
                "node_modules/other/index.d.ts",
                "export { X } from \"lib\";\n",
            ),
            (
                "node_modules/other/node_modules/lib/index.d.ts",
                "export declare class X {}\n",
            ),
            (
                "node_modules/lib/index.d.ts",
                "export declare const pad = 1;\nexport declare function X(): void;\n",
            ),
        ],
    );
    d_on(&mut a, "src/main.ts", "^X");
    let shown = shown(&mut a);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        shown,
        jump(
            "X: by name, 1 match",
            "node_modules/other/node_modules/lib/index.d.ts:1",
        ),
        "other's own copy of lib is the one its re-export names"
    );
}

#[test]
fn a_dependency_merl_saved_is_read_again() {
    let place = "node_modules/lib/index.d.ts";
    let (dir, mut a) = project_app(
        "ts-saved",
        &[
            ("src/main.ts", "import { pick } from \"lib\";\n\npick();\n"),
            (place, "export declare function pick(): void;\n"),
        ],
    );
    let landed = |line: usize| jump("pick: via import lib", &format!("{place}:{line}"));
    d_on(&mut a, "src/main.ts", "^pick");
    assert_eq!(shown(&mut a), landed(1));
    a.line = 0;
    a.col = 0;
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.save();
    assert!(
        std::fs::read_to_string(dir.join(place))
            .unwrap()
            .starts_with('\n'),
        "{}",
        a.message
    );
    d_on(&mut a, "src/main.ts", "^pick");
    assert_eq!(
        shown(&mut a),
        landed(2),
        "the lines kept of a file merl saved are not the file any more"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_anonymous_class_s_import_name_lasts_one_d() {
    let (_, mut a) = project_app(
        "ts-anonymous-carry",
        &[
            (
                "spinner.ts",
                "export default class extends Object {\n  spin(): void {}\n}\n",
            ),
            ("second.ts", "export function f(x) {\n  x.spin();\n}\n"),
        ],
    );
    a.anonymous_named.set(Some(Typed {
        name: "Whirl".into(),
        path: "spinner.ts".into(),
        line: 1,
    }));
    d_on(&mut a, "second.ts", "x.spin");
    assert_eq!(
        shown(&mut a),
        jump("spin → extends.spin (by name, 1 match)", "spinner.ts:2"),
        "a name an earlier d left behind is not this d's"
    );
}

#[cfg(unix)]
#[test]
fn a_review_folder_reads_its_workspace_packages_as_your_checkout_reads_its_own() {
    let main = "import { helper, fresh, stale, gone } from \"@app/shared\";\nimport * as sh from \"@app/shared\";\n\nhelper();\nfresh();\nstale();\ngone();\nsh.helper();\nsh.fresh();\nsh.stale();\n";
    let shared = "packages/shared/index.ts";
    let mine = "export function helper() {}\nexport function fresh() {}\n";
    let theirs = "export function helper() {}\nexport function stale() {}\n";
    let outside = [
        (
            "node_modules/other/node_modules/@app/shared/index.d.ts",
            "export declare function gone(): void;\nexport declare function stale(): void;\n",
        ),
        (
            "node_modules/lib/index.d.ts",
            "export declare function fresh(): void;\n",
        ),
    ];
    let link = |node_modules: &Path| {
        std::fs::create_dir_all(node_modules.join("@app")).unwrap();
        std::os::unix::fs::symlink("../../packages/shared", node_modules.join("@app/shared"))
            .unwrap();
    };
    let files = [("src/main.ts", main), (shared, mine)];
    let (own, mut checkout) = project_app("ts-workspace-checkout", &files);
    write_outside(&own, &outside);
    link(&own.join("node_modules"));
    let (folder, mut review) = project_app("ts-workspace-review", &files);
    let yours = folder.with_extension("yours");
    drop(std::fs::remove_dir_all(&yours));
    write_outside(&yours, &[(shared, theirs)]);
    write_outside(&yours, &outside);
    link(&yours.join("node_modules"));
    std::os::unix::fs::symlink(yours.join("node_modules"), folder.join("node_modules")).unwrap();
    let codes = [
        "^helper",
        "^fresh",
        "^stale",
        "^gone",
        "sh.helper",
        "sh.fresh",
        "sh.stale",
    ];
    let answers = |a: &mut App| {
        (codes.iter())
            .map(|code| {
                d_on(a, "src/main.ts", code);
                (*code, shown(a))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        answers(&mut review),
        answers(&mut checkout),
        "the reviewed branch's own package, never the copy in your checkout its node_modules links to"
    );
    for dir in [own, folder, yours] {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

fn sveltekit_project() -> (PathBuf, App) {
    let main = "import { goto, missing } from '$app/navigation';\nimport { page, navigating } from '$app/state';\nimport { enhance } from '$app/forms';\nimport { browser } from '$app/environment';\nimport { base } from '$app/paths';\n\ngoto('/');\npage.data;\nnavigating;\nenhance;\nbrowser;\nbase;\nmissing;\npage.pathname;\nnavigating.from;\npage.state;\n";
    let kit = "declare module '$app/navigation' {\n\texport function goto(url: string): Promise<void>;\n\texport {};\n}\n\ndeclare module '$app/state' {\n\texport const page: import(\"@sveltejs/kit\").Page;\n\texport const navigating: boolean;\n\texport {};\n}\n\ndeclare module '@sveltejs/kit' {\n\texport interface PageData {}\n\texport interface Page<\n\t\tP extends string = string\n\t> {\n\t\turl: URL & { pathname: P };\n\t\t/**\n\t\t * The merged {@link data}.\n\t\t */\n\t\tdata: PageData & Record<string, any>;\n\t}\n\texport interface Other {\n\t\tstate: object;\n\t}\n}\n";
    let map = r#"{"version":3,"file":"index.d.ts","sources":["../src/client.js","../src/state.js"],"mappings":";CAEgB,KAAI;;;;;CCFA;CAGA,KAEA"}"#;
    let ambient = |m: &str, name: &str| {
        format!("declare module '{m}' {{\n\texport const {name}: boolean;\n}}\n")
    };
    let package = r#"{"devDependencies":{"@sveltejs/kit":"2","dupe-a":"1","dupe-b":"1"}}"#;
    let (dir, a) = project_app(
        "ts-ambient",
        &[("src/main.ts", main), ("package.json", package)],
    );
    write_outside(
        &dir,
        &[
            ("node_modules/@sveltejs/kit/types/index.d.ts", kit),
            ("node_modules/@sveltejs/kit/types/index.d.ts.map", map),
            (
                "node_modules/@sveltejs/kit/src/client.js",
                "import x from 'x';\n\nexport function goto(url) {}\n",
            ),
            (
                "node_modules/@sveltejs/kit/src/state.js",
                "export const page = {};\n",
            ),
            (
                "node_modules/@sveltejs/kit/types/forms.d.ts",
                &ambient("$app/forms", "enhance"),
            ),
            (
                "node_modules/@sveltejs/kit/types/forms.d.ts.map",
                "{\"version\": 3,",
            ),
            (
                "node_modules/dupe-a/index.d.ts",
                &ambient("$app/environment", "browser"),
            ),
            (
                "node_modules/dupe-b/index.d.ts",
                &ambient("$app/environment", "browser"),
            ),
            (
                "node_modules/stray/index.d.ts",
                &ambient("$app/paths", "base"),
            ),
        ],
    );
    (dir, a)
}

#[test]
fn an_ambient_module_of_a_dependency_answers_through_its_declaration_map() {
    let (dir, mut a) = sveltekit_project();
    let at = |p: &str| format!("node_modules/@sveltejs/kit/{p}");
    for (code, want, why) in [
        (
            "^goto",
            jump("goto: via import $app/navigation", &at("src/client.js:3")),
            "the declaration map names one source line",
        ),
        (
            "page.data",
            jump("data → Page.data (via page)", &at("types/index.d.ts:21")),
            "a member of the export is its declared type's member",
        ),
        (
            "page.pathname",
            jump("no definition for pathname", "src/main.ts:14"),
            "a key nested in a member is not the type's member",
        ),
        (
            "navigating.from",
            jump("no definition for from", "src/main.ts:15"),
            "an export typed without an import keeps no definition",
        ),
        (
            "page.state",
            jump("no definition for state", "src/main.ts:16"),
            "another interface's member is not the type's",
        ),
        (
            "^navigating",
            jump(
                "navigating: via import $app/state",
                &at("types/index.d.ts:8"),
            ),
            "a map naming two source lines keeps the declaration",
        ),
        (
            "^enhance",
            jump("enhance: via import $app/forms", &at("types/forms.d.ts:2")),
            "a map that does not parse keeps the declaration",
        ),
        (
            "^browser",
            jump(
                "browser: via import $app/environment (not installed)",
                "src/main.ts:4",
            ),
            "two packages declare the module",
        ),
        (
            "^base",
            jump(
                "base: via import $app/paths (not installed)",
                "src/main.ts:5",
            ),
            "a package the project does not depend on",
        ),
        (
            "^missing",
            jump(
                "missing: via import $app/navigation (not installed)",
                "src/main.ts:1",
            ),
            "the block does not export the name",
        ),
    ] {
        d_on(&mut a, "src/main.ts", code);
        assert_eq!(shown(&mut a), want, "{why}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_default_import_lands_on_the_local_a_module_exports_as_default() {
    let main = "import vue from \"plug\";\nimport pack from \"pack\";\nimport relay from \"relay\";\nimport own from \"./own\";\n\nexport default [\n    vue(),\n    pack(),\n    relay(),\n    own(),\n];\n";
    let own =
        "function ownPlugin() {}\nfunction other() {}\nexport { other, ownPlugin as default };\n";
    let (dir, mut a) = project_app(
        "ts-as-default",
        &[("vite.config.mjs", main), ("own.js", own)],
    );
    let vue = "interface Options {\n}\ndeclare function vuePlugin(o?: Options): void;\ndeclare function vueCjs(o: Options): void;\nexport { Options, vuePlugin as default, vueCjs as \"module.exports\" };\n";
    write_outside(
        &dir,
        &[
            (
                "node_modules/plug/package.json",
                "{\"name\": \"plug\", \"exports\": {\".\": \"./dist/index.mjs\"}}\n",
            ),
            ("node_modules/plug/dist/index.d.mts", vue),
            (
                "node_modules/plug/dist/index.mjs",
                "function vuePlugin() {}\nexport { vuePlugin as default };\n",
            ),
            (
                "node_modules/pack/package.json",
                "{\"name\": \"pack\", \"types\": \"dist/index.d.ts\"}\n",
            ),
            (
                "node_modules/pack/dist/index.d.ts",
                "declare function export_default(): void;\n\nexport { export_default as default };\n",
            ),
            (
                "node_modules/relay/index.d.ts",
                "declare function relayPlugin(): void;\nexport { relayPlugin as default } from \"./impl\";\n",
            ),
        ],
    );
    for (code, want, why) in [
        (
            "    vue|()",
            jump(
                "vue: via import plug",
                "node_modules/plug/dist/index.d.mts:3",
            ),
            "the declaration file, not the module beside it",
        ),
        (
            "    pack|()",
            jump(
                "pack: via import pack",
                "node_modules/pack/dist/index.d.ts:1",
            ),
            "a package's `types` entry",
        ),
        (
            "    own|()",
            jump("own: via import own.js", "own.js:1"),
            "a module of the project",
        ),
    ] {
        d_on(&mut a, "vite.config.mjs", code);
        assert_eq!(shown(&mut a), want, "{why}");
    }
    d_on(&mut a, "vite.config.mjs", "    relay|()");
    assert_ne!(
        shown(&mut a),
        jump("relay: via import relay", "node_modules/relay/index.d.ts:1"),
        "a default handed on from another module is not the local namesake"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
