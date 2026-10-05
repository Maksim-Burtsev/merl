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
