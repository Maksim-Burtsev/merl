use super::*;

fn declares(line: &str, word: &str) -> bool {
    Regex::new(&perl_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
        && perl_declares(&[line], 1, word, line)
}

#[test]
fn perl_declaration_forms() {
    for (line, word) in [
        ("sub total {", "total"),
        ("sub total($self) {", "total"),
        ("sub _private {", "_private"),
        ("sub Shop::Order::total {", "total"),
        ("sub total", "total"),
        ("sub total :lvalue {", "total"),
        ("method total {", "total"),
        ("package Shop::Order;", "Shop::Order"),
        ("package Shop::Order {", "Shop::Order"),
        ("package Shop::Order 1.02;", "Shop::Order"),
        ("class Shop::Order;", "Shop::Order"),
        ("class Shop::Order :isa(Shop::Base) {", "Shop::Order"),
        ("our $VERSION = '1.02';", "VERSION"),
        ("our @EXPORT_OK = qw(total);", "EXPORT_OK"),
        ("our ($x, %seen);", "seen"),
        ("use constant PI => 3.14;", "PI"),
        ("use constant { PI => 3.14, E => 2.71 };", "E"),
        ("has 'name' => (is => 'ro');", "name"),
        ("has name => sub { 1 };", "name"),
        ("has [qw(host port)];", "port"),
        ("has '+name';", "name"),
        ("field $total :param = 0;", "total"),
    ] {
        assert!(declares(line, word), "{line}: {word}");
    }
}

#[test]
fn perl_refusals() {
    for (line, word) in [
        ("    total(1);", "total"),
        ("    $order->total;", "total"),
        ("use Shop::Order;", "Shop::Order"),
        ("    total => 12,", "total"),
        ("    my $x = $args{total};", "total"),
        ("    print 'total';", "total"),
        ("    local $x = 1;", "x"),
        ("sub total;", "total"),
        ("package Shop::Order::Item;", "Shop::Order"),
        ("my $total = 1;", "total"),
    ] {
        assert!(!declares(line, word), "{line}: {word}");
    }
}

#[test]
fn perl_constant_block_entries_declare_only_inside_their_block() {
    let lines = [
        "use constant {",
        "    MAX => 10,",
        "    # the floor",
        "    MIN => 1,",
        "};",
        "my %h = (",
        "    MAX => 3,",
        ");",
    ];
    let at = |n: usize, word: &str| perl_declares(&lines, n, word, lines[n - 1]);
    assert!(at(2, "MAX"));
    assert!(at(4, "MIN"));
    assert!(!at(7, "MAX"));
    assert!(!perl_declares(
        &["use constant { A => 1 };", "    B => 2,"],
        2,
        "B",
        "    B => 2,"
    ));
}

#[test]
fn perl_literal_lines_cover_pod_heredocs_quotes_and_data() {
    let text = [
        "my $a = <<\"EOT\" . <<'END';", // 0
        "sub one {",                    // 1
        "EOT",                          // 2
        "sub two {",                    // 3
        "END",                          // 4
        "my $b = <<~EOT;",              // 5
        "    sub three {",              // 6
        "    EOT",                      // 7
        "my @w = qw(",                  // 8
        "    sub four {",               // 9
        ");",                           // 10
        "my $q = q{a {nested}",         // 11
        "sub five",                     // 12
        "};",                           // 13
        "=head1 NAME",                  // 14
        "sub six {",                    // 15
        "=cut",                         // 16
        "my $n = $#list; # q{ <<EOT",   // 17
        "my $s = 1 << 2;",              // 18
        "my %h = (q => 1, qw => 2);",   // 19
        "__END__",                      // 20
        "sub seven {",                  // 21
    ]
    .join("\n");
    let literal = perl_literal_lines(&text);
    let code: Vec<usize> = (0..literal.len()).filter(|&i| !literal[i]).collect();
    assert_eq!(code, [0, 5, 8, 11, 17, 18, 19, 20]);
}

#[test]
fn perl_name_reads_a_package_whole() {
    let line = "my $o = Shop::Order->new; Shop::Order::total($o);";
    let at = |s: &str| perl_name(line, line.find(s).unwrap()).map(|r| &line[r]);
    assert_eq!(at("Shop::Order->"), Some("Shop::Order"));
    assert_eq!(at("Order->"), Some("Shop::Order"));
    assert_eq!(at("new"), Some("new"));
    assert_eq!(at("total"), Some("Shop::Order::total"));
    assert_eq!(at("$o)"), None);
}

#[test]
fn perl_imports_bind_the_names_a_use_lists() {
    let text = "use strict;\nuse File::Basename qw(basename dirname);\nuse POSIX 'floor', \"ceil\";\nuse List::Util 1.45 qw(\n  max\n  $VAR\n  :all\n);\nuse Carp;\n=pod\nuse Fake qw(hidden);\n=cut\n";
    assert_eq!(
        perl_imports(text),
        [
            ("basename", "File::Basename"),
            ("dirname", "File::Basename"),
            ("floor", "POSIX"),
            ("ceil", "POSIX"),
            ("max", "List::Util"),
        ]
        .map(|(n, m)| (n.to_owned(), m.to_owned()))
    );
}

#[test]
fn perl_lexical_is_the_nearest_declaration_in_the_blocks_around() {
    let text = [
        "my $self = 1;",                // 1
        "sub new {",                    // 2
        "    my ($class, %args) = @_;", // 3
        "    my $self = {};",           // 4
        "    return $args{x};",         // 5
        "}",                            // 6
        "sub other($self) {",           // 7
        "    for my $item (@_) {",      // 8
        "        print $item;",         // 9
        "    }",                        // 10
        "    state $n = 0;",            // 11
        "    return $self;",            // 12
        "}",                            // 13
        "print $self, $item;",          // 14
    ]
    .join("\n");
    let at = |line: usize, word: &str| {
        let l = text.lines().nth(line - 1).unwrap();
        perl_lexical(&text, line - 1, l.rfind(word).unwrap(), word)
    };
    assert_eq!(at(5, "args"), Some(3));
    assert_eq!(at(9, "item"), Some(8));
    assert_eq!(at(12, "self"), Some(7));
    assert_eq!(at(14, "self"), Some(1));
    assert_eq!(at(14, "item"), None);
    assert_eq!(at(11, "n"), Some(11));
}

#[test]
fn perl_roots_put_carton_first_and_drop_a_relative_inc_entry() {
    let inc = "/Library/Perl/5.34\n.\n/System/Library/Perl/5.34\nlib\n";
    assert_eq!(
        perl_roots(Path::new("/p"), inc),
        [
            "/p/local/lib/perl5",
            "/Library/Perl/5.34",
            "/System/Library/Perl/5.34"
        ]
        .map(PathBuf::from)
    );
}
