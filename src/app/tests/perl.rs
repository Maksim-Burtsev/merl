use super::*;

#[test]
fn d_on_a_perl_import_opens_the_first_root_of_inc_that_holds_the_module() {
    let (dir, mut a) = project_app(
        "perl-inc",
        &[(
            "bin/report.pl",
            "use File::Basename qw(basename);\nuse Missing::Module;\nmy $self = basename($0);\n",
        )],
    );
    let root = external_root(
        "perl",
        &[
            ("site/Other.pm", "package Other;\n1;\n"),
            (
                "core/File/Basename.pm",
                "package File::Basename;\n\nsub basename {\n    return 1;\n}\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Perl, &[root.join("site"), root.join("core")]);
    let at = |p: &str| format!("{}", root.join(p).display());
    for (code, want) in [
        (
            "my $self = basename",
            jump(
                "basename: via import File::Basename",
                &at("core/File/Basename.pm:3"),
            ),
        ),
        (
            "use File::Basename",
            jump(
                "File::Basename: module File/Basename.pm",
                &at("core/File/Basename.pm:1"),
            ),
        ),
        (
            "use Missing::Module",
            jump("no definition for Missing::Module", "bin/report.pl:2"),
        ),
    ] {
        d_on(&mut a, "bin/report.pl", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn perl_symbols_are_subs_packages_classes_and_methods() {
    let (dir, mut a) = project_app(
        "perl-symbols",
        &[(
            "lib/Shop/Order.pm",
            "package Shop::Order;\nour $VERSION = 1;\nuse constant PI => 3;\nhas 'name';\nsub total { 1 }\nsub forward;\nclass Shop::Coupon :isa(Shop::Order) {\n    field $x;\n    method apply($p) { $p }\n}\n",
        )],
    );
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().unwrap();
    picker.settle();
    let names: Vec<String> = (picker.window(20).0.into_iter())
        .map(|r| r.item.label.split_whitespace().next().unwrap().to_string())
        .collect();
    assert_eq!(names, ["apply", "Shop::Coupon", "Shop::Order", "total"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn u_on_a_perl_sub_marks_its_sub_and_no_fat_comma_key() {
    let (dir, mut a) = project_app(
        "perl-u",
        &[
            (
                "lib/Shop/Order.pm",
                "package Shop::Order;\nsub total { 1 }\n1;\n",
            ),
            (
                "bin/report.pl",
                "my $o = Shop::Order->new(\n    total => 12,\n);\nprint $o->total;\n",
            ),
        ],
    );
    a.jump_to(&dir.join("bin/report.pl"), 4);
    a.col = a.line_str().find("total").unwrap();
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    let rows: Vec<String> = (picker.window(50).0.into_iter())
        .map(|r| {
            r.item.label[..r.item.code_at.unwrap()]
                .trim_end()
                .to_owned()
        })
        .collect();
    assert_eq!(
        rows,
        [
            "declaration  lib/Shop/Order.pm:2:",
            "             bin/report.pl:2:",
            "             bin/report.pl:4:",
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_a_method_of_a_perl_receiver_named_beyond_ascii_reads_the_receiver_whole() {
    let (dir, mut a) = project_app(
        "perl-unicode-receiver",
        &[(
            "lib/Odd.pm",
            "package Odd;\nsub total { 1 }\nmy $ßar = Odd->new;\n$ßar->total();\n",
        )],
    );
    d_on(&mut a, "lib/Odd.pm", "$ßar->|total(");
    assert_eq!(
        shown(&mut a),
        jump("total: by name, 1 match", "lib/Odd.pm:2")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
