use super::*;

/// The `d` bench's player (`tools/d-bench/run`): reads `id root file line col` rows from
/// `BENCH_IN`, presses `d` on each, one `App` per project, and writes
/// `id outcome ms status targets` to `BENCH_OUT`. The outcome is `jump`, `picker` or `stay`;
/// the targets are `abs_path:line` joined by `;`, one for a jump, every row for a picker.
/// Without `BENCH_IN` it does nothing, so `cargo test -- --ignored` stays green.
#[test]
#[ignore]
fn d_bench() {
    let Ok(input) = std::env::var("BENCH_IN") else {
        return;
    };
    let rows = std::fs::read_to_string(input).unwrap();
    let mut out = String::new();
    let mut apps: HashMap<String, App> = HashMap::new();
    for row in rows.lines() {
        let c: Vec<&str> = row.split('\t').collect();
        let (id, root, file, line, col) = (c[0], c[1], c[2], c[3], c[4]);
        let a = apps.entry(root.to_owned()).or_insert_with(|| {
            let dir = PathBuf::from(root);
            let (tree, files) = crate::tree::build(&dir, false);
            App::new(dir, tree, files, Buffer::empty(), None)
        });
        a.picker = None;
        a.message.clear();
        let path = a.root.join(file);
        a.jump_to(&path, line.parse().unwrap());
        a.col = col.parse().unwrap();
        let before = (a.buf.path.clone(), a.line);
        let t = std::time::Instant::now();
        press(a, KeyCode::Char('d'), KeyModifiers::NONE);
        let ms = t.elapsed().as_micros() as f64 / 1000.0;
        let root = a.root.clone();
        let abs = |p: &Path| root.join(p).display().to_string();
        let (outcome, targets) = match a.picker.as_mut() {
            Some(p) => {
                p.settle();
                let rows = p.window(100_000).0;
                let t: Vec<String> = rows
                    .iter()
                    .map(|r| format!("{}:{}", abs(&r.item.path), r.item.line))
                    .collect();
                ("picker", t.join(";"))
            }
            None => {
                let now = (a.buf.path.clone(), a.line);
                if now == before {
                    ("stay", String::new())
                } else {
                    let p = now.0.as_deref().map(abs).unwrap_or_default();
                    ("jump", format!("{p}:{}", now.1 + 1))
                }
            }
        };
        let status = a.message.replace(['\t', '\n'], " ");
        a.picker = None;
        out.push_str(&format!("{id}\t{outcome}\t{ms:.2}\t{status}\t{targets}\n"));
    }
    std::fs::write(std::env::var("BENCH_OUT").unwrap(), out).unwrap();
}
