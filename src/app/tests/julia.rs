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
                r.item
                    .label
                    .get(..r.item.code_at.unwrap())
                    .unwrap()
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

#[test]
fn d_on_a_julia_type_annotation_reads_the_type_not_a_member_of_the_value() {
    let (dir, mut a) = project_app(
        "julia-typed",
        &[(
            "src/use.jl",
            "f(i::Integer) = i\ng(bc::Base.Broadcast.Broadcasted) = bc\nh(x::N) where N = x\n\
struct Grid{T,\n            M}\n    size::M\nend\n\
function k(x::AbstractArray{T}, n::K) where {\n        T,\n        K<:Integer}\n    n\nend\n\
struct Box{T<:Union{Int,Float64},J}\n    n::J\nend\n\
m(x::I) where {T<:Tuple{Int},I} = x\n\
struct Bare{Q}\n    q::Q\nend\n\
p(x::R) where {T, R} = x\n",
        )],
    );
    let base = external_root(
        "julia-typed",
        &[
            (
                "base/boot.jl",
                "abstract type Integer <: Real end\nconst N = 382\nconst M = 1\nconst K = 2\nconst J = 3\nconst I = 4\nconst Q = 5\nconst R = 6\n",
            ),
            (
                "base/broadcast.jl",
                "module Broadcast\nstruct Broadcasted end\nend\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Julia, &[base.join("base")]);
    let at = |file: &str, line: usize| format!("{}:{line}", base.join(file).display());
    for (code, want) in [
        (
            "f(i::Integer",
            jump("Integer: by name, 1 match", &at("base/boot.jl", 1)),
        ),
        (
            "Base.Broadcast",
            jump("Broadcast: by name, 1 match", &at("base/broadcast.jl", 1)),
        ),
        ("h(x::N", jump("no definition for N", "src/use.jl:3")),
        ("size::M", jump("no definition for M", "src/use.jl:6")),
        ("n::K", jump("no definition for K", "src/use.jl:8")),
        ("n::J", jump("no definition for J", "src/use.jl:14")),
        ("m(x::I", jump("no definition for I", "src/use.jl:16")),
        ("q::Q", jump("no definition for Q", "src/use.jl:18")),
        ("p(x::R", jump("no definition for R", "src/use.jl:20")),
    ] {
        d_on(&mut a, "src/use.jl", code);
        assert_eq!(shown(&mut a), want, "{code}");
        a.jump_to(&dir.join("src/use.jl"), 1);
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn d_on_a_julia_function_the_project_only_extends_offers_base_beside_the_extension() {
    let (dir, mut a) = project_app(
        "julia-extends",
        &[
            (
                "src/frames.jl",
                "Base.isempty(df::Frame) = true\nfunction g(df)\n    isempty(df)\nend\n",
            ),
            ("test/use.jl", "@test isempty(keys(df))\n"),
        ],
    );
    let base = external_root(
        "julia-extends",
        &[("base/essentials.jl", "isempty(itr) = true\n")],
    );
    use_roots(&mut a, Kind::Julia, &[base.join("base")]);
    d_on(&mut a, "test/use.jl", "@test isempty");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "isempty: by name, 2 declarations".into(),
            vec![
                ("isempty".into(), "by name".into(), "src/frames.jl:1".into()),
                ("isempty".into(), "by name".into(), "essentials.jl:1".into()),
            ]
        )
    );
    a.jump_to(&dir.join("src/frames.jl"), 1);
    d_on(&mut a, "src/frames.jl", "    isempty");
    assert_eq!(
        shown(&mut a),
        jump("isempty: by name, 1 match", "src/frames.jl:1")
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn d_on_a_julia_function_the_project_only_extends_offers_the_extension_when_no_base_is_found() {
    let (dir, mut a) = project_app(
        "julia-extends-alone",
        &[
            ("src/frames.jl", "Base.isempty(df::Frame) = true\n"),
            ("test/use.jl", "@test isempty(keys(df))\n"),
        ],
    );
    use_roots(&mut a, Kind::Julia, &[]);
    d_on(&mut a, "test/use.jl", "@test isempty");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "isempty: by name, 1 match".into(),
            vec![("isempty".into(), "by name".into(), "src/frames.jl:1".into())]
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_a_julia_function_the_project_imports_from_itself_or_by_dotted_path() {
    let (dir, mut a) = project_app(
        "julia-import-forms",
        &[
            (
                "src/Pkg.jl",
                "module Inner\nhelper(x) = x\nend\nusing .Inner: helper\n",
            ),
            (
                "src/money.jl",
                "import Base.isempty\nisempty(m::Money) = true\n",
            ),
            ("src/b.jl", "helper(1)\nisempty(xs)\n"),
        ],
    );
    use_roots(&mut a, Kind::Julia, &[]);
    d_on(&mut a, "src/b.jl", "helper");
    assert_eq!(
        shown(&mut a),
        jump("helper: by name, 1 match", "src/Pkg.jl:2")
    );
    d_on(&mut a, "src/b.jl", "isempty");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "isempty: by name, 1 match".into(),
            vec![("isempty".into(), "by name".into(), "src/money.jl:2".into())]
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_julia_s_base_reads_the_base_module_not_a_submodule_s_own_binding() {
    let (dir, mut a) = project_app(
        "julia-base-module",
        &[
            (
                "src/show.jl",
                "Base.summary(df) = 1\nIterators.Base.summary(df) = 1\n",
            ),
            (
                "src/bare.jl",
                "baremodule Bare\nBase.summary(df) = 1\nend\n",
            ),
        ],
    );
    let base = external_root(
        "julia-base-module",
        &[
            ("base/Base.jl", "\nbaremodule Base\nend\n"),
            (
                "base/iterators.jl",
                "baremodule Iterators\nconst Base = parentmodule(@__MODULE__)\nend\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Julia, &[base.join("base")]);
    d_on(&mut a, "src/show.jl", "Base|.summary");
    assert_eq!(
        shown(&mut a),
        jump(
            "Base: by name, 1 match",
            &format!("{}:2", base.join("base/Base.jl").display())
        )
    );
    a.jump_to(&dir.join("src/show.jl"), 1);
    d_on(&mut a, "src/show.jl", "Iterators.Base|.summary");
    assert_eq!(
        shown(&mut a),
        jump(
            "Base: by name, 1 match",
            &format!("{}:2", base.join("base/iterators.jl").display())
        )
    );
    a.jump_to(&dir.join("src/bare.jl"), 1);
    d_on(&mut a, "src/bare.jl", "Base|.summary");
    assert!(
        matches!(shown(&mut a), Shown::Picker(title, _) if title == "Base: by name, 2 declarations"),
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn d_on_a_julia_field_reads_the_struct_its_parameter_is_typed_by() {
    let types = "abstract type AbstractFrame end\nmutable struct Group{T<:AbstractFrame}\n    parent::T\n    cols::Vector{Symbol}\n    idx::Vector{Int}\nend\nstruct Frame <: AbstractFrame\n    cols::Vector{Symbol}\nend\nstruct Key\n    cols::Vector{Symbol}\nend\nstruct Sorter\n    cols::Vector{Symbol}\n    function Sorter(c)\n        cols = c\n        new(cols)\n    end\nend\nfunction Base.getproperty(gd::Group, f::Symbol)\n    if f === :idx\n        return getfield(gd, :idx)\n    end\n    return getfield(gd, f)\nend\nBase.getproperty(df::AbstractFrame, s::Symbol) = df[!, s]\nfunction Base.getproperty(k::Key, p::Symbol)\n    p === :cols && return 1\n    return getfield(k, p)\nend\nstruct Disk\n    r::Float64\n    area::Float64\nend\nstruct Square\n    area::Float64\nend\nconst AREA = :area\nfunction Base.getproperty(d::Disk, name::Symbol)\n    name === AREA && return getfield(d, :r)^2\n    return getfield(d, name)\nend\nstruct CGroup\n    cols::Vector{Symbol}\nend\nconst PROPS = (:cols,)\nfunction Base.getproperty(c::CGroup, s::Symbol)\n    s in PROPS && return 1\n    return getfield(c, s)\nend\n";
    let uses = "function plain(g1::Group, x)\n    g1.cols\nend\nfunction para(g2::Group{Frame})\n    g2.cols\nend\nabstr(f3::AbstractFrame) = f3.cols\nfunction bound(g4::G) where {G<:Group}\n    g4.cols\nend\nfunction again(g5::Group)\n    g5 = pick()\n    g5.cols\nend\nfunction inherited(f6::Frame)\n    f6.cols\nend\nfunction keyed(k7::Key)\n    k7.cols\nend\nfunction sorted(s8::Sorter)\n    s8.cols\nend\nfunction looped(g9::Group)\n    for i in 1:3\n        g9.cols\n        g9 = pick()\n    end\nend\nmapped(g10::Group, xs) = map(g10 -> g10.cols, xs)\nlisted(g11::Group, gs) = [g11.cols for g11 in gs]\nfunction later(g17::Group, ks)\n    [g17.cols\n     for g17 in ks]\nend\nfunction each(g19::Group, xs)\n    foreach(g19 -> begin\n        g19.cols\n    end, xs)\nend\nfunction anon(g21::Group)\n    h = function (g21)\n        g21.cols\n    end\nend\nfunction shadowed(g25::Group, k)\n    let g25 = k; g25.cols; end\nend\nfunction disk(d22::Disk)\n    d22.area\nend\nfunction consts(c1::CGroup)\n    c1.cols\nend\n";
    let (dir, mut a) = project_app(
        "julia-typed-field",
        &[("src/types.jl", types), ("src/use.jl", uses)],
    );
    for (code, line) in [
        ("g1.cols", Some(4)),
        ("g2.cols", Some(4)),
        ("f3.cols", None),
        ("g4.cols", None),
        ("g5.cols", None),
        ("f6.cols", None),
        ("k7.cols", None),
        ("s8.cols", Some(14)),
        ("g9.|cols", None),
        ("-> g10.|cols", None),
        ("[g11.|cols", None),
        ("[g17.|cols", None),
        ("        g19.|cols", None),
        ("        g21.|cols", None),
        ("; g25.|cols", None),
        ("d22.|area", None),
        ("c1.|cols", None),
    ] {
        d_on(&mut a, "src/use.jl", code);
        match (shown(&mut a), line) {
            (Shown::Jump(_, at), Some(line)) => assert_eq!(at, format!("src/types.jl:{line}")),
            (Shown::Picker(..), None) => {}
            (got, _) => panic!("{code}: {got:?}"),
        }
        a.jump_to(&dir.join("src/use.jl"), 1);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_a_julia_field_keeps_the_picker_when_a_getproperty_reads_an_unwritten_type() {
    let types = "struct Group\n    cols::Vector{Symbol}\nend\nstruct Key\n    cols::Vector{Symbol}\nend\nfor T in (:Group,)\n    @eval function Base.getproperty(e::$T, s::Symbol)\n        s === :cols && return 1\n        return getfield(e, s)\n    end\nend\n";
    let uses = "function plain(e1::Group)\n    e1.cols\nend\n";
    let (dir, mut a) = project_app(
        "julia-eval-getproperty",
        &[("src/types.jl", types), ("src/use.jl", uses)],
    );
    d_on(&mut a, "src/use.jl", "e1.|cols");
    assert!(matches!(shown(&mut a), Shown::Picker(..)));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_julia_s_base_keeps_the_picker_when_the_project_owns_a_base() {
    let (dir, mut a) = project_app(
        "julia-own-base",
        &[
            ("src/Base.jl", "module Base\nend\n"),
            ("vendor/base.jl", "baremodule Base\nend\n"),
            ("src/show.jl", "Base.summary(df) = 1\n"),
        ],
    );
    d_on(&mut a, "src/show.jl", "Base|.summary");
    let got = shown(&mut a);
    assert!(
        matches!(got, Shown::Picker(ref title, _) if title == "Base: by name, 2 declarations"),
        "{got:?}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
