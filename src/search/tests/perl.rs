#![expect(clippy::string_slice, reason = "#858")]
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
    let lines = [
        "my $a = <<\"EOT\" . <<'END';",
        "sub one {",
        "EOT",
        "sub two {",
        "END",
        "my $b = <<~EOT;",
        "    sub three {",
        "    EOT",
        "my @w = qw(",
        "    sub four {",
        ");",
        "my $q = q{a {nested}",
        "sub five",
        "};",
        "=head1 NAME",
        "sub six {",
        "=cut",
        "my $n = $#list; # q{ <<EOT",
        "my $s = 1 << 2;",
        "my %h = (q => 1, qw => 2);",
        "__END__",
        "sub seven {",
    ];
    let literal = perl_literal_lines(&lines.join("\n"));
    let code: Vec<&str> = (0..literal.len())
        .filter(|&i| !literal[i])
        .map(|i| lines[i])
        .collect();
    assert_eq!(
        code,
        [
            "my $a = <<\"EOT\" . <<'END';",
            "my $b = <<~EOT;",
            "my @w = qw(",
            "my $q = q{a {nested}",
            "my $n = $#list; # q{ <<EOT",
            "my $s = 1 << 2;",
            "my %h = (q => 1, qw => 2);",
            "__END__",
        ]
    );
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
    let lines = [
        "my $self = 1;",
        "sub new {",
        "    my ($class, %args) = @_;",
        "    my $self = {};",
        "    return $args{x};",
        "}",
        "sub other($self) {",
        "    for my $item (@_) {",
        "        print $item;",
        "    }",
        "    state $n = 0;",
        "    return $self;",
        "}",
        "print $self, $item;",
    ];
    let text = lines.join("\n");
    let at = |line: &str, word: &str| {
        let i = lines.iter().position(|l| *l == line).unwrap();
        perl_lexical(&text, i, line.rfind(word).unwrap(), word).map(|n| lines[n - 1])
    };
    assert_eq!(
        at("    return $args{x};", "args"),
        Some("    my ($class, %args) = @_;")
    );
    assert_eq!(
        at("        print $item;", "item"),
        Some("    for my $item (@_) {")
    );
    assert_eq!(at("    return $self;", "self"), Some("sub other($self) {"));
    assert_eq!(at("print $self, $item;", "self"), Some("my $self = 1;"));
    assert_eq!(at("print $self, $item;", "item"), None);
    assert_eq!(at("    state $n = 0;", "n"), Some("    state $n = 0;"));
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
