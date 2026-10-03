use super::*;

fn declares(lines: &[&str], line: usize, word: &str) -> bool {
    Regex::new(&julia_patterns(word).join("|"))
        .unwrap()
        .is_match(lines[line - 1])
        && julia_declares(lines, line, word)
}

fn alone(line: &str, word: &str) -> bool {
    declares(&[line], 1, word)
}

#[test]
fn julia_declaration_forms() {
    for (line, word) in [
        ("function total(xs::Vector{Money})", "total"),
        ("function Base.show(io::IO, m::Money)", "show"),
        ("@inline function weigh(x)", "weigh"),
        ("format_price(m::Money) = string(m)", "format_price"),
        ("format_price(m::Money)::String = string(m)", "format_price"),
        ("f(x::T) where {T} = x", "f"),
        ("convert(::Type{Money}, x) = Money(x)", "convert"),
        ("Base.show(io::IO, m::Money) = print(io, m)", "show"),
        ("sort!(xs) = xs", "sort!"),
        ("struct Money", "Money"),
        ("mutable struct Acc", "Acc"),
        ("struct Point{T} <: AbstractPoint{T}", "Point"),
        ("@kwdef struct Options", "Options"),
        ("Base.@kwdef mutable struct Options", "Options"),
        ("abstract type Shape end", "Shape"),
        ("primitive type Int24 24 end", "Int24"),
        ("macro check(ex)", "check"),
        ("module Shop", "Shop"),
        ("baremodule Core2", "Core2"),
        ("const LIMIT = 10", "LIMIT"),
        ("config = load()", "config"),
        ("@enum Color red green blue", "Color"),
        ("@enum Color red green blue", "green"),
        ("@enum Color::UInt8 red green", "red"),
        ("@enum Fruit apple=1 orange=2", "orange"),
        ("@inline f(x) = x", "f"),
        ("Point{T}(x::T) where {T} = new(x)", "Point"),
    ] {
        assert!(alone(line, word), "{line}: {word}");
    }
}

#[test]
fn julia_refusals() {
    for (line, word) in [
        ("    total(xs)", "total"),
        ("println(format_price(m))", "format_price"),
        ("x == 1", "x"),
        ("rate(t) == rate(c)", "rate"),
        ("f(x) => 1", "f"),
        ("a[i] = x", "a"),
        ("p.cents = 1", "cents"),
        ("f(x = 1)", "x"),
        ("import Base: show", "show"),
        ("export format_price", "format_price"),
        ("@check x", "check"),
        ("    m::Money", "Money"),
        ("function (m::Money)(x)", "m"),
        ("function sort!(xs)", "sort"),
        ("sort!(xs) = xs", "sort"),
        ("function sort(xs)", "sort!"),
        ("struct Moneybag", "Money"),
        ("function Base.show(io::IO, m::Money)", "Base"),
        ("function Base.:+(a::Money, b::Money)", "Base"),
        ("Base.show(io::IO, m::Money) = print(io, m)", "Base"),
        ("@show total(xs)", "total"),
        ("@test rate(t) == 1", "rate"),
        ("x => 1", "x"),
    ] {
        assert!(!alone(line, word), "{line}: {word}");
    }
}

#[test]
fn a_julia_field_is_one_directly_inside_its_struct() {
    let lines = [
        "struct Money",
        "    cents::Int",
        "    note",
        "end",
        "Base.@kwdef mutable struct Options",
        "    # the speed",
        "    express::Bool = false",
        "end",
        "function f(x)",
        "    cents = x",
        "    note",
        "    g(",
        "        express = 1)",
        "end",
    ];
    assert!(declares(&lines, 2, "cents"));
    assert!(declares(&lines, 3, "note"));
    assert!(declares(&lines, 7, "express"));
    let documented = [
        "struct Money",
        "    \"\"\"",
        "doc",
        "    \"\"\"",
        "    cents::Int",
        "end",
    ];
    assert!(declares(&documented, 5, "cents"));
    assert!(!declares(&lines, 10, "cents"));
    assert!(!declares(&lines, 11, "note"));
    assert!(!declares(&lines, 13, "express"));
    let narrowed = |line: &str, word: &str| {
        let at = line.find(word).unwrap();
        let mut p = julia_patterns(word);
        julia_narrow(&mut p, line, at..at + word.len());
        Regex::new(&p.join("|")).unwrap()
    };
    assert!(!narrowed("cents(x)", "cents").is_match(lines[1]));
    assert!(!narrowed("y = cents + 1", "cents").is_match(lines[1]));
    assert!(narrowed("y = m.cents + 1", "cents").is_match(lines[1]));
    assert!(narrowed("    cents::Int", "cents").is_match(lines[1]));
    assert!(narrowed("@check x", "check").is_match("macro check(ex)"));
    assert!(!narrowed("@check x", "check").is_match("check(x) = x > 0"));
    assert!(!narrowed("check(x)", "check").is_match("macro check(ex)"));
    assert!(narrowed("macro check(ex)", "check").is_match("check(x) = x > 0"));
}

#[test]
fn julia_literals_hide_declarations() {
    let text = [
        "\"\"\"",
        "    struct Doc end",
        "\"\"\"",
        "#=",
        "function hidden() end",
        "#= nested =#",
        "struct Hidden end",
        "=#",
        "cmd = `ls",
        "function shell() end",
        "`",
        "s = \"over",
        "struct Quoted end\"",
        "r = r\"a",
        "struct Code end",
        "x = A' * B'",
        "struct Adjoint end",
        "c = '\"'",
        "struct Char end",
        "# struct Comment end",
        "struct Last end",
        "y = A' * \"'\"",
        "struct AfterAdjoint end",
        "# don't \"quote",
        "struct AfterComment end",
    ]
    .join("\n");
    let hidden: Vec<usize> = literal_lines(Kind::Julia, &text)
        .iter()
        .enumerate()
        .filter(|(_, h)| **h)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 5, 6, 7, 8, 10, 11, 13]);
}

#[test]
fn julia_locals_stay_in_their_function() {
    let lines = [
        "m = 0",
        "function report(xs, tariff::Tariff; verbose = false)",
        "    m = total(xs)",
        "    for (i, item) in enumerate(xs)",
        "        m.cents + i + item + verbose",
        "    end",
        "    [x for x in xs]",
        "    map(xs) do x",
        "        x + tariff",
        "    end",
        "end",
        "rate(t::Tariff) = t.rate",
        "use(m)",
        "function outer()",
        "    inner(y) = y",
        "    m",
        "end",
        "for k in 1:3",
        "    k += 1",
        "    k",
        "end",
        "map(xs) do",
        "    xs",
        "end",
        "function g(xs, color)",
        "    for x in xs",
        "        x = 2x",
        "        x",
        "    end",
        "    xs = sort(xs)",
        "    draw(xs;",
        "         color = :red,",
        "         size = 3)",
        "    color + size",
        "    sq(v) = v^2",
        "    map(rate -> rate * 2, xs) + map((a, b) -> a + b, xs)",
        "end",
    ];
    let at = |line: usize, name: &str| {
        bindings(Kind::Julia, &lines.join("\n"), line, name)
            .iter()
            .map(|b| b.line)
            .collect::<Vec<_>>()
    };
    assert_eq!(at(5, "m"), [3]);
    assert!(at(3, "m").is_empty());
    assert_eq!(at(5, "i"), [4]);
    assert_eq!(at(5, "item"), [4]);
    assert_eq!(at(5, "verbose"), [2]);
    assert_eq!(at(5, "xs"), [2]);
    assert_eq!(at(7, "x"), [7]);
    assert_eq!(at(9, "x"), [8]);
    assert_eq!(at(9, "tariff"), [2]);
    assert_eq!(at(12, "t"), [12]);
    assert!(at(13, "m").is_empty());
    assert!(at(5, "total").is_empty());
    assert!(at(16, "m").is_empty());
    assert_eq!(at(20, "k"), [18]);
    assert!(at(23, "xs").is_empty());
    assert_eq!(at(28, "x"), [27]);
    assert_eq!(at(30, "xs"), [25]);
    assert_eq!(at(34, "color"), [25]);
    assert!(at(34, "size").is_empty());
    assert_eq!(at(35, "v"), [35]);
    assert_eq!(at(36, "rate"), [36]);
    assert_eq!(at(36, "b"), [36]);
    let line = "    xs = sort(xs)";
    assert!(binds_at(Kind::Julia, line, 4, "xs"));
    assert!(!binds_at(Kind::Julia, line, 14, "xs"));
    assert!(!binds_at(Kind::Julia, "    xs == ys", 4, "xs"));
}

#[test]
fn julia_imports_bind_names_and_modules() {
    let text = "using DataFrames\nusing CSV, Tables\nimport DataFrames: select, transform as tf\nusing Base.Iterators: flatten\nimport LinearAlgebra as LA\nusing ..Shop\nimport Base.show\nusing Test: @test\n# using Hidden\n";
    let got = imports(Kind::Julia, text);
    let has = |name: &str, path: &[&str]| {
        got.iter()
            .any(|(n, p)| n == name && p.iter().map(String::as_str).eq(path.iter().copied()))
    };
    assert!(has("DataFrames", &["DataFrames"]));
    assert!(has("CSV", &["CSV"]));
    assert!(has("Tables", &["Tables"]));
    assert!(has("select", &["DataFrames", "select"]));
    assert!(has("tf", &["DataFrames", "transform"]));
    assert!(has("flatten", &["Base", "Iterators", "flatten"]));
    assert!(has("LA", &["LinearAlgebra"]));
    assert!(has("Shop", &["..", "Shop"]));
    assert!(has("show", &["Base", "show"]));
    assert!(has("test", &["Test", "test"]));
    assert!(!got.iter().any(|(n, _)| n == "Hidden"));
    let documented = imports(
        Kind::Julia,
        "\"\"\"\n    using Example\n\"\"\"\nusing Real\n",
    );
    assert!(!documented.iter().any(|(n, _)| n == "Example"));
    assert!(documented.iter().any(|(n, _)| n == "Real"));
}

#[test]
fn julia_names_keep_their_bang() {
    let line = "sort!(xs); sort(xs); a != b; f!=g";
    let word = |col| definition_word(Some(Kind::Julia), line, col).map(|(_, w)| w);
    assert_eq!(word(1), Some("sort!"));
    assert_eq!(word(12), Some("sort"));
    assert_eq!(word(21), Some("a"));
    assert_eq!(word(29), Some("f"));
    assert_eq!(
        julia_include("include(\"../src/money.jl\")", 12).as_deref(),
        Some("../src/money.jl")
    );
    assert_eq!(julia_include("x = include_dependency(\"a.jl\")", 25), None);
    assert_eq!(julia_col("sort!(a); sort(b)", "sort"), Some(10));
    assert_eq!(julia_col("sort!(a); resort(b)", "sort"), None);
    assert_eq!(julia_col("a!=b", "a"), Some(0));
    assert_eq!(julia_col("sort!(a)", "sort!"), Some(0));
}

#[test]
fn julia_symbols() {
    let julia = |line| one(Kind::Julia, line);
    for (line, name) in [
        ("function total(xs)", "total"),
        ("function Base.show(io::IO, m::Money)", "show"),
        ("format_price(m::Money) = string(m)", "format_price"),
        ("Base.show(io::IO, m::Money) = print(io, m)", "show"),
        ("f(x::T)::T where {T} = x", "f"),
        ("sort!(xs) = xs", "sort!"),
        ("f(x::Vector{Tuple{Int,Int}}, g = h(k(1))) = 1", "f"),
        ("mutable struct Acc", "Acc"),
        ("@kwdef struct Options", "Options"),
        ("abstract type Shape end", "Shape"),
        ("primitive type Int24 24 end", "Int24"),
        ("macro check(ex)", "check"),
        ("module Shop", "Shop"),
    ] {
        assert_eq!(julia(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "const LIMIT = 10",
        "config = load()",
        "    cents::Int",
        "    m = total(xs)",
        "    inner(y) = y",
        "rate(t) == rate(c)",
        "function (m::Money)(x)",
        "function Base.:+(a::Money, b::Money)",
    ] {
        assert_eq!(julia(line), None, "{line}");
    }
}

#[test]
fn julia_slugs_are_the_ones_julia_computes() {
    let uuid = "a93c6f00-e57d-5684-b7b6-d8193f3e46c0";
    let slug = |u: &str, t: &str| julia_slug(u, t);
    assert_eq!(
        slug(uuid, "5fab31e2e01e70ad66e3e24c968c264d1cf166d6").as_deref(),
        Some("0Y1g5")
    );
    assert_eq!(
        slug(uuid, "0123456789abcdef0123456789abcdef01234567").as_deref(),
        Some("fki4w")
    );
    let ones = "f".repeat(40);
    assert_eq!(
        slug("00000000-0000-0000-0000-000000000001", &ones).as_deref(),
        Some("NW94s")
    );
    assert_eq!(slug(uuid, "xyz"), None);
}

#[test]
fn julia_roots_read_the_manifest_and_the_depot() {
    let dir = std::env::temp_dir().join(format!("merl-julia-roots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mk = |p: &str| std::fs::create_dir_all(dir.join(p)).unwrap();
    for p in [
        "share/julia/base",
        "share/julia/stdlib/v1.11/Dates/src",
        "depot/packages/DataFrames/0Y1g5/src",
        "depot/packages/DataFrames/AAAAA/src",
        "depot/packages/CSV/b1/src",
        "depot/packages/CSV/b2/src",
        "depot/packages/Shop/s1/src",
        "project",
    ] {
        mk(p);
    }
    std::fs::write(dir.join("project/Project.toml"), "name = \"Shop\"\n").unwrap();
    std::fs::write(
        dir.join("project/Manifest.toml"),
        "[[deps.DataFrames]]\ngit-tree-sha1 = \"5fab31e2e01e70ad66e3e24c968c264d1cf166d6\"\nuuid = \"a93c6f00-e57d-5684-b7b6-d8193f3e46c0\"\n\n[[deps.Dates]]\nuuid = \"ade2ca70-3891-5945-98fb-dc099432e06a\"\n",
    )
    .unwrap();
    let roots = julia_roots(
        &dir.join("project"),
        Some(dir.join("share/julia")),
        &dir.join("depot"),
    );
    let rel: Vec<String> = roots
        .iter()
        .map(|r| r.strip_prefix(&dir).unwrap().display().to_string())
        .collect();
    assert_eq!(
        rel,
        [
            "share/julia/base",
            "share/julia/stdlib/v1.11/Dates/src",
            "depot/packages/CSV/b1/src",
            "depot/packages/CSV/b2/src",
            "depot/packages/DataFrames/0Y1g5/src",
        ]
    );
    std::fs::create_dir_all(dir.join("other")).unwrap();
    std::fs::write(dir.join("other/JuliaProject.toml"), "name = \"CSV\"\n").unwrap();
    std::fs::write(dir.join("other/Manifest.toml"), "").unwrap();
    std::fs::write(
        dir.join("other/JuliaManifest.toml"),
        "[[DataFrames]]\ngit-tree-sha1 = \"5fab31e2e01e70ad66e3e24c968c264d1cf166d6\"\nuuid = \"a93c6f00-e57d-5684-b7b6-d8193f3e46c0\"\n",
    )
    .unwrap();
    let other: Vec<String> = julia_roots(&dir.join("other"), None, &dir.join("depot"))
        .iter()
        .map(|r| r.strip_prefix(&dir).unwrap().display().to_string())
        .collect();
    assert_eq!(
        other,
        [
            "depot/packages/DataFrames/0Y1g5/src",
            "depot/packages/Shop/s1/src"
        ]
    );
    assert_eq!(
        julia_share("/opt/julia-1.11.7/bin\n"),
        Some(PathBuf::from("/opt/julia-1.11.7/share/julia"))
    );
    let home = Path::new("/home/u");
    assert_eq!(julia_depot(None, home), home.join(".julia"));
    assert_eq!(julia_depot(Some("/d:/e".into()), home), PathBuf::from("/d"));
    assert_eq!(julia_depot(Some(":/e".into()), home), home.join(".julia"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn julia_is_told_by_its_extension() {
    assert_eq!(kind_of(Path::new("src/money.jl")), Some(Kind::Julia));
}
