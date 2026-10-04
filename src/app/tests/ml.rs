use super::*;

#[test]
fn u_reads_a_primed_ocaml_name_whole() {
    let (dir, mut a) = project_app(
        "ocaml-u",
        &[
            ("lib/rate.ml", "let rate' = 4\nlet rate = 5\n"),
            ("lib/cart.ml", "let total = Rate.rate' + Rate.rate\n"),
        ],
    );
    a.jump_to(&dir.join("lib/cart.ml"), 1);
    a.col = a.line_str().find("rate'").unwrap();
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    let rows: Vec<String> = picker
        .window(50)
        .0
        .into_iter()
        .map(|r| {
            r.item.label[..r.item.code_at.unwrap()]
                .trim_end()
                .to_owned()
        })
        .collect();
    assert_eq!(
        rows,
        ["declaration  lib/rate.ml:1:", "             lib/cart.ml:1:"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn shift_d_lists_module_items_of_ml_files_and_fsharp_members() {
    let (dir, mut a) = project_app(
        "ml-symbols",
        &[
            (
                "lib/cart.ml",
                "let label m =\n  let prefix = 1 in\n  prefix\nmodule Ledger = struct\n  let add x = x\nend\n",
            ),
            ("lib/cart.mli", "val label : int -> int\ntype hidden\n"),
            (
                "src/Shop.fs",
                "module Shop\n\ntype Account() =\n    let mutable count = 0\n    member this.Total = count\n",
            ),
        ],
    );
    a.jump_to(&dir.join("lib/cart.ml"), 1);
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    let mut names: Vec<String> = picker
        .window(50)
        .0
        .into_iter()
        .map(|r| r.item.label.split_whitespace().next().unwrap().to_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["Account", "Ledger", "Shop", "Total", "add", "label"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
