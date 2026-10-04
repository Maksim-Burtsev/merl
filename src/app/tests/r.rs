use super::*;

#[test]
fn u_reads_a_dotted_and_a_backticked_r_name_whole() {
    let (dir, mut a) = project_app(
        "r-u",
        &[
            (
                "R/money.R",
                "print.invoice <- function(x, ...) x\nprint <- 1\n.onLoad <- function(l, p) NULL\n`%+%` <- function(a, b) a\n",
            ),
            (
                "R/use.R",
                "print.invoice(x)\nprint(x)\n.onLoad(1, 2)\nonLoad <- 3\ny <- 2 %+% 3\n",
            ),
        ],
    );
    let rows = |a: &mut App, line: usize, at: &str| {
        a.jump_to(&dir.join("R/use.R"), line);
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
        press(a, KeyCode::Esc, KeyModifiers::NONE);
        rows
    };
    assert_eq!(
        rows(&mut a, 1, "invoice"),
        ["declaration  R/money.R:1:", "             R/use.R:1:"]
    );
    assert_eq!(
        rows(&mut a, 3, "onLoad"),
        ["declaration  R/money.R:3:", "             R/use.R:3:"]
    );
    assert_eq!(
        rows(&mut a, 5, "%+%"),
        ["declaration  R/money.R:4:", "             R/use.R:5:"]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
