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
