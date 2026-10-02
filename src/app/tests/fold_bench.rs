use super::*;

#[test]
#[ignore]
fn fold_bench() {
    let Ok(input) = std::env::var("BENCH_IN") else {
        return;
    };
    let rows = std::fs::read_to_string(input).unwrap();
    let mut out = String::new();
    let mut apps: HashMap<String, App> = HashMap::new();
    for row in rows.lines() {
        let c: Vec<&str> = row.split('\t').collect();
        let (id, path, line) = (c[0], c[1], c[2].parse::<usize>().unwrap() - 1);
        let a = apps.entry(path.to_owned()).or_insert_with(|| {
            let buf = Buffer::load(Path::new(path)).unwrap();
            App::new(PathBuf::from("/"), Tree::default(), Vec::new(), buf, None)
        });
        a.collapsed.clear();
        a.message.clear();
        a.go((line, 0));
        press(a, KeyCode::Char('f'), KeyModifiers::NONE);
        let got = match a.collapsed.first() {
            Some(&(h, e)) => format!("{}\t{}", h + 1, e + 1),
            None => "-\t-".into(),
        };
        out.push_str(&format!("{id}\t{got}\t{}\n", a.message));
    }
    std::fs::write(std::env::var("BENCH_OUT").unwrap(), out).unwrap();
}
