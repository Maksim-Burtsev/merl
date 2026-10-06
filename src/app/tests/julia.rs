use super::*;

#[test]
fn d_on_a_julia_package_name_reads_the_version_the_manifest_pins_beside_the_projects_methods() {
    let (dir, mut a) = project_app(
        "julia-depot",
        &[
            ("Project.toml", "name = \"Report\"\n"),
            (
                "Manifest.toml",
                "[[deps.DataFrames]]\ngit-tree-sha1 = \"5fab31e2e01e70ad66e3e24c968c264d1cf166d6\"\nuuid = \"a93c6f00-e57d-5684-b7b6-d8193f3e46c0\"\n",
            ),
            (
                "src/report.jl",
                "using DataFrames\nimport DataFrames: select\nfunction report(xs)\n    df = DataFrame(cents = xs)\n    select(df)\n    DataFrames.transform(df)\nend\nselect(x) = x\n",
            ),
        ],
    );
    let package = "module DataFrames\nmutable struct DataFrame <: AbstractDataFrame\nselect(df::DataFrame) = df\nfunction transform(df)\nend\nend\n";
    let depot = external_root(
        "julia-depot",
        &[
            ("packages/DataFrames/0Y1g5/src/DataFrames.jl", package),
            ("packages/DataFrames/AAAAA/src/DataFrames.jl", package),
        ],
    );
    let roots = search::julia_roots(&dir, None, &depot);
    use_roots(&mut a, Kind::Julia, &roots);
    let at = |line: usize| {
        format!(
            "{}:{line}",
            depot
                .join("packages/DataFrames/0Y1g5/src/DataFrames.jl")
                .display()
        )
    };
    for (code, want) in [
        (
            "    df = DataFrame|(",
            jump("DataFrame: by name, 1 match", &at(2)),
        ),
        (
            "DataFrames.transform",
            jump("transform: via import DataFrames", &at(4)),
        ),
    ] {
        d_on(&mut a, "src/report.jl", code);
        assert_eq!(shown(&mut a), want, "{code}");
        assert!(a.buf.readonly.is_some(), "{code}");
        a.jump_to(&dir.join("src/report.jl"), 1);
    }
    d_on(&mut a, "src/report.jl", "    select");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "select: 2 declarations".into(),
            vec![
                (
                    "select".into(),
                    "via import DataFrames".into(),
                    "DataFrames.jl:3".into()
                ),
                ("select".into(), "by name".into(), "src/report.jl:8".into()),
            ]
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&depot).unwrap();
}

#[test]
fn u_tells_a_julia_bang_function_from_its_namesake() {
    let (dir, mut a) = project_app(
        "julia-u",
        &[
            (
                "src/sort.jl",
                "function sort!(xs)\nend\nfunction sort(xs)\nend\n",
            ),
            ("src/use.jl", "sort!(a)\nsort(a)\nsort!(b)\n"),
        ],
    );
    let rows = |a: &mut App, line: usize, at: &str| {
        a.jump_to(&dir.join("src/use.jl"), line);
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
        rows(&mut a, 1, "sort!"),
        ["src/sort.jl:1:", "src/use.jl:1:", "src/use.jl:3:"]
    );
    assert_eq!(rows(&mut a, 2, "sort"), ["src/sort.jl:3:", "src/use.jl:2:"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_a_julia_call_outside_the_project_reads_no_field() {
    let (dir, mut a) = project_app(
        "julia-base",
        &[(
            "src/use.jl",
            "function f(xs)\n    first(xs)\n    xs.first\nend\n",
        )],
    );
    let base = external_root(
        "julia-base",
        &[(
            "base/pair.jl",
            "struct Pair{A, B}\n    first::A\n    second::B\nend\nfirst(p::Pair) = p.first\n",
        )],
    );
    use_roots(&mut a, Kind::Julia, &[base.join("base")]);
    let at = |line: usize| format!("{}:{line}", base.join("base/pair.jl").display());
    d_on(&mut a, "src/use.jl", "    first");
    assert_eq!(shown(&mut a), jump("first: by name, 1 match", &at(5)));
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&base).unwrap();
}
