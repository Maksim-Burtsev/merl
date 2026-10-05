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
        ["default.nix:2:", "hosts/web.nix:3:"]
    );
    assert_eq!(
        rows(&mut a, "hosts/web.nix", 4, "rate' "),
        ["default.nix:5:", "hosts/web.nix:4:"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn u_marks_a_let_binding_and_lands_whole_on_a_quoted_word_elsewhere() {
    let (dir, mut a) = project_app(
        "nix-u-let",
        &[
            (
                "g.nix",
                "{ pkgs }:\nlet\n  api = 1;\nin\n{ a = api; b = pkgs.hello; }\n",
            ),
            ("run.sh", "echo 'hello'\n"),
        ],
    );
    let rows = |a: &mut App, line: usize, at: &str| {
        a.jump_to(&dir.join("g.nix"), line);
        a.col = a.line_str().find(at).expect(at);
        press(a, KeyCode::Char('u'), KeyModifiers::NONE);
        let picker = a.picker.as_mut().expect("a picker");
        picker.settle();
        let rows: Vec<String> = (picker.window(50).0.into_iter())
            .map(|r| {
                r.item.label[..r.item.code_at.unwrap()]
                    .trim_end()
                    .to_owned()
            })
            .collect();
        rows
    };
    assert_eq!(rows(&mut a, 5, "api;"), ["g.nix:3:", "g.nix:5:"]);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(rows(&mut a, 5, "hello"), ["g.nix:5:", "run.sh:1:"]);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!((a.line, a.col), (0, 6));
    std::fs::remove_dir_all(&dir).unwrap();
}
