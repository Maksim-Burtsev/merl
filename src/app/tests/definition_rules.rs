use super::*;

#[test]
fn a_cut_grep_of_the_rust_type_in_front_marks_nothing() {
    let many = "pub struct Gizmo;\n".repeat(search::MAX_HITS);
    let (dir, mut a) = project_app(
        "rule-cut-pathed",
        &[
            ("src/many.rs", many.as_str()),
            (
                "src/main.rs",
                "pub fn spin() {}\n\nfn main() {\n    Gizmo::spin();\n}\n",
            ),
        ],
    );
    a.no_external();
    d_on(&mut a, "src/main.rs", "Gizmo::spin");
    assert_eq!(
        shown(&mut a),
        jump("spin: by name, 1 match", "src/main.rs:1"),
        "the grep for the many `Gizmo`s is cut, the one for `spin` is not: a jump, no `+`"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_label_whose_callee_has_a_namesake_outside_the_project_is_offered() {
    let (dir, mut a) = project_app(
        "rule-label-outside",
        &[
            (
                "local.py",
                "class Local:\n    def fetch(self, retries):\n        pass\n",
            ),
            ("main.py", "def run(x):\n    x.fetch(retries=1)\n"),
        ],
    );
    let root = external_root(
        "rule-label-outside",
        &[(
            "lib/client.py",
            "class Client:\n    def fetch(self, retries):\n        pass\n",
        )],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    d_on(&mut a, "main.py", "x.fetch(retries");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "retries: parameter of Local.fetch, found by name".into(),
            vec![(
                "Local.fetch.retries".into(),
                "parameter of Local.fetch, found by name".into(),
                "local.py:2".into()
            )],
        ),
        "outside the project nothing is read, so `fetch` may be the library's: offered, not jumped to"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
