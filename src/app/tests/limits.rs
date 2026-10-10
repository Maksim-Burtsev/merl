use super::*;

#[test]
fn a_reload_carries_lines_past_one_rewritten_region_only() {
    let old: Vec<String> = (0..10).map(|i| i.to_string()).collect();
    let mut new = old.clone();
    new.insert(6, "y".into());
    new.insert(2, "x".into());
    assert_eq!(new[9], "7");
    assert_eq!(carried(&old, &new, 7), 9);
    assert_eq!(new[5], "4");
    assert_eq!(carried(&old, &new, 4), 4);
}

#[test]
fn the_files_outside_the_project_are_walked_once_per_session() {
    let (dir, mut a) = project_app("outside-once", &[("app.py", "import vendored\n")]);
    let root = external_root("outside-once", &[("vendored.py", "X = 1\n")]);
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    assert_eq!(a.external_files(Kind::Python).len(), 1);
    std::fs::write(root.join("installed.py"), "Y = 1\n").unwrap();
    assert_eq!(a.external_files(Kind::Python).len(), 1);
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_hunk_left_is_its_index_so_a_hunk_written_above_it_takes_its_place() {
    let (dir, mut a) = review_app("limits-hunk-left");
    let src = dir.join("src/a.rs");
    a.jump_to(&src, 3);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(a.line_str(), "F");
    a.jump_to(&dir.join("src/keep.rs"), 2);
    std::fs::write(&src, "A\nB\nc\nD\ne\nF\n").unwrap();
    let fresh = a.review.as_ref().unwrap().refresh(&a.root).unwrap();
    a.review_refreshed(fresh);
    press(&mut a, KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!((at(&a), a.line_str()), ((src, 3), "D"));
    let _ = std::fs::remove_dir_all(dir);
}

fn d_in(tag: &str, files: &[(String, String)], file: &str, code: &str) -> Shown {
    let files: Vec<(&str, &str)> = files.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    let (dir, mut a) = project_app(tag, &files);
    d_on(&mut a, file, code);
    let found = shown(&mut a);
    let _ = std::fs::remove_dir_all(dir);
    found
}

fn ts_relays(modules: usize, last: &str) -> Vec<(String, String)> {
    let mut files = vec![(
        "app.ts".to_string(),
        "import { Tariff } from './r1';\n\nconst t = new Tariff();\n".to_string(),
    )];
    for i in 1..modules {
        let relay = format!("export {{ Tariff }} from './r{}';\n", i + 1);
        files.push((format!("r{i}.ts"), relay));
    }
    files.push((format!("r{modules}.ts"), last.to_string()));
    files
}

#[test]
fn a_typescript_name_is_followed_through_four_modules_that_hand_it_on() {
    let at = |n: usize| {
        let files = ts_relays(n, "export class Tariff {}\n");
        d_in(&format!("ts-relay-{n}"), &files, "app.ts", "new Tariff")
    };
    assert_eq!(at(5), jump("Tariff: via import r5.ts", "r5.ts:1"));
    assert_eq!(at(6), jump("Tariff: by name, 1 match", "r6.ts:1"));
}

#[test]
fn a_typescript_package_is_found_behind_four_barrels() {
    let at = |n: usize| {
        let files = ts_relays(n, "export { Tariff } from 'tariffs';\n");
        d_in(&format!("ts-barrel-{n}"), &files, "app.ts", "new Tariff")
    };
    let missing = "Tariff: via import tariffs (not installed)";
    assert_eq!(at(4), jump(missing, "app.ts:1"));
    assert_eq!(at(5), jump("no definition for Tariff", "app.ts:3"));
}

#[test]
fn two_typescript_modules_handing_a_name_to_each_other_end() {
    let mut files = ts_relays(2, "export { Tariff } from './r1';\n");
    let none = jump("no definition for Tariff", "app.ts:3");
    assert_eq!(d_in("ts-cycle", &files, "app.ts", "new Tariff"), none);
    files.push(("tariffs.ts".into(), "export class Tariff {}\n".into()));
    let by_name = jump("Tariff: by name, 1 match", "tariffs.ts:1");
    assert_eq!(d_in("ts-cycle-named", &files, "app.ts", "new Tariff"), by_name);
}
