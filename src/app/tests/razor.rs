use super::*;

const TARIFF: &str = "public class Tariff\n{\n    public int Rate() => 1;\n}\n";

const TILL: &str = "\
@page \"/till\"
@inject Tariff Tariffs

<p>@Tariffs.Rate()</p>
<p>@nameof(Till)</p>

@code {
    private int Go(int n) => n;
    private int Settle() => Go(n: 2) + Tariffs.Rate();
}
";

fn d_at(a: &mut App, file: &Path, line: usize, word: &str) {
    a.jump_to(file, line);
    a.col = a.line_str().find(word).unwrap();
    press(a, KeyCode::Char('d'), KeyModifiers::NONE);
}

fn edited(a: &mut App) -> String {
    a.buf.lines[0] = "@page \"/till/{id}\"".into();
    a.dirty = true;
    TILL.replacen("/till", "/till/{id}", 1)
}

fn text(a: &App) -> String {
    a.buf.lines.join("\n") + "\n"
}

#[test]
fn d_in_a_razor_file_reads_it_as_csharp_and_leaves_its_text_alone() {
    let (dir, mut a) = project_app("razor-read", &[("Tariff.cs", TARIFF), ("Till.razor", TILL)]);
    let till = dir.join("Till.razor");
    d_at(&mut a, &till, 4, "Rate");
    assert_eq!(at(&a), (dir.join("Tariff.cs"), 2), "{}", a.message);
    d_at(&mut a, &till, 9, "Rate");
    assert_eq!(at(&a), (dir.join("Tariff.cs"), 2), "{}", a.message);
    d_at(&mut a, &till, 2, "Tariff ");
    assert_eq!(at(&a), (dir.join("Tariff.cs"), 0), "{}", a.message);
    d_at(&mut a, &till, 9, "Tariffs");
    assert_eq!(at(&a), (till.clone(), 1), "{}", a.message);
    assert_eq!(text(&a), TILL);
    assert_eq!(std::fs::read_to_string(&till).unwrap(), TILL);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_a_named_argument_of_an_edited_razor_file_leaves_the_file_its_own() {
    let (dir, mut a) = project_app("razor-named", &[("Tariff.cs", TARIFF), ("Till.razor", TILL)]);
    let till = dir.join("Till.razor");
    a.jump_to(&till, 1);
    let want = edited(&mut a);
    d_at(&mut a, &till, 9, "n: 2");
    assert_eq!(at(&a), (till.clone(), 7), "{}", a.message);
    d_at(&mut a, &till, 9, "Go(");
    assert_eq!(at(&a), (till.clone(), 7), "{}", a.message);
    assert_eq!(text(&a), want);
    assert_eq!(std::fs::read_to_string(&till).unwrap(), TILL);
    assert!(a.flush());
    assert_eq!(std::fs::read_to_string(&till).unwrap(), want);
    assert_eq!(text(&a), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_in_an_edited_razor_file_never_lands_on_a_line_the_view_made_up() {
    let (dir, mut a) = project_app("razor-made-up", &[("Tariff.cs", TARIFF), ("Till.razor", TILL)]);
    let till = dir.join("Till.razor");
    a.jump_to(&till, 1);
    let want = edited(&mut a);
    d_at(&mut a, &till, 5, "Till");
    assert_eq!(at(&a), (till.clone(), 4), "{}", a.message);
    assert!(a.picker.is_none(), "{}", a.message);
    assert_eq!(text(&a), want);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_from_a_deleted_razor_line_of_an_edited_file_in_a_review_saves_only_its_text() {
    let (dir, mut a) = project_app("razor-review", &[("Tariff.cs", TARIFF), ("Till.razor", TILL)]);
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    let branch = TILL.replace("<p>@Tariffs.Rate()</p>\n", "");
    let till = dir.join("Till.razor");
    std::fs::write(&till, &branch).unwrap();
    git(&["commit", "-qam", "work"]);
    a.start_review(git::Review::open(&dir, None, None).unwrap());
    a.jump_to(&till, 1);
    a.buf.lines[0] = "@page \"/till/{id}\"".into();
    a.dirty = true;
    let want = branch.replacen("/till", "/till/{id}", 1);
    while a.deleted.is_none() {
        assert!(a.next_line(a.at()).is_some(), "no deleted line");
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    }
    a.col = a.line_str().find("Rate").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("Tariff.cs")),
        "{}",
        a.message
    );
    assert_eq!(std::fs::read_to_string(&till).unwrap(), want);
    a.jump_to(&till, 1);
    assert_eq!(text(&a), want);
    std::fs::remove_dir_all(&dir).unwrap();
}
