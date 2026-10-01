use super::*;

#[test]
fn u_reads_a_dashed_and_a_primed_nix_name_whole() {
    let (dir, mut a) = project_app(
        "nix-u",
        &[
            (
                "default.nix",
                "{\n  my-package = 1;\n  my-package-x = 2;\n  package = 3;\n  rate' = 4;\n  rate = 5;\n}\n",
            ),
            (
                "hosts/web.nix",
                "{ shop }:\n{\n  a = shop.my-package;\n  b = shop.rate' + shop.rate;\n}\n",
            ),
        ],
    );
    let rows = |a: &mut App, file: &str, line: usize, at: &str| {
        a.jump_to(&dir.join(file), line);
        a.col = a.line_str().find(at).expect(at);
        press(a, KeyCode::Char('u'), KeyModifiers::NONE);
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
        press(a, KeyCode::Esc, KeyModifiers::NONE);
        rows
    };
    assert_eq!(
        rows(&mut a, "hosts/web.nix", 3, "package;"),
        [
            "declaration  default.nix:2:",
            "             hosts/web.nix:3:"
        ]
    );
    assert_eq!(
        rows(&mut a, "hosts/web.nix", 4, "rate' "),
        [
            "declaration  default.nix:5:",
            "             hosts/web.nix:4:"
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
