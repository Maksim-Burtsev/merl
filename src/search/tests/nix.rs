use super::*;

fn declares(line: &str, word: &str) -> bool {
    Regex::new(&nix_patterns(word).join("|"))
        .unwrap()
        .is_match(line)
}

#[test]
fn nix_declaration_forms() {
    for (line, word) in [
        ("  mkService = { name, port ? 8080 }: {", "mkService"),
        ("mkService =", "mkService"),
        ("  services.nginx.enable = true;", "enable"),
        (r#"  "my-attr" = 1;"#, "my-attr"),
        ("  my-package = 1;", "my-package"),
        ("  x' = 1;", "x'"),
        ("  inherit name port;", "name"),
        ("  inherit name port;", "port"),
        ("  inherit (pkgs) hello;", "hello"),
        ("  inherit (pkgs) hello", "hello"),
        ("{ inherit lib; }", "lib"),
        ("{ a = 1; b = 2; }", "b"),
        ("let x = 1; in x", "x"),
    ] {
        assert!(declares(line, word), "{line}: {word}");
    }
}

#[test]
fn nix_refusals() {
    for (line, word) in [
        ("  services.nginx.enable = true;", "nginx"),
        ("  services.nginx.enable = true;", "services"),
        ("  x = pkgs.hello;", "hello"),
        ("  x = with pkgs; [ hello ];", "pkgs"),
        ("  free = rate == 0;", "rate"),
        ("  rate == 0", "rate"),
        ("  { name, port ? 8080 }:", "port"),
        ("  ${zone} = 3;", "zone"),
        (r#"  "${zone}" = 3;"#, "zone"),
        ("  inherit (pkgs) hello;", "pkgs"),
        ("  my-package-x = 1;", "my-package"),
        ("  xname = 1;", "name"),
        ("  name' = 1;", "name"),
    ] {
        assert!(!declares(line, word), "{line}: {word}");
    }
}

#[test]
fn nix_let_bindings_declare_only_in_their_own_file() {
    let lines = [
        "let",
        "  api = 1;",
        "/* a comment */",
        "  cfg = {",
        "    port = 1;",
        "  };",
        "  script = ''",
        "echo hi",
        "'';",
        "  key = 2;",
        "in",
        "{",
        "  api = 2;",
        "  x = let xx = 1; in xx;",
        "  y = let aa = 1;",
        "          bb = 2;",
        "      in aa + bb;",
        "  # z = 3; inherit foo;",
        "  msg = \"see { foo3 = 1; }\";",
        "  \"my-attr\" = 4;",
        "}",
    ];
    let over = |line: usize, word: &str| nix_declares(&lines, line, word, false);
    let here = |line: usize, word: &str| nix_declares(&lines, line, word, true);
    assert!(!over(2, "api") && here(2, "api"));
    assert!(!over(4, "cfg") && here(4, "cfg"));
    assert!(over(5, "port"));
    assert!(!over(10, "key") && here(10, "key"));
    assert!(over(13, "api"));
    assert!(!over(14, "xx") && here(14, "xx"));
    assert!(!over(15, "aa") && !over(16, "bb"));
    assert!(over(15, "y"));
    assert!(!here(18, "z") && !here(18, "foo"));
    assert!(!here(19, "foo3"));
    assert!(over(20, "my-attr"));
}

fn local(text: &str, line: usize, name: &str) -> Vec<usize> {
    bindings(Kind::Nix, text, line, name)
        .iter()
        .map(|b| b.line)
        .collect()
}

#[test]
fn nix_locals_are_let_bindings_and_parameters_in_scope() {
    let text = "\
{ config, pkgs, ... }:
let
  shopLib = import ../lib { };
  api = shopLib.mkService { };
in
{
  a = api.enable;
  b = pkgs.hello;
  mkService = { name, port ? 8080 }: {
    inherit name;
    p = port;
  };
  c = name;
  double = x: x * 2;
  d = let
    api = 3;
  in api;
  e = map (y: y + 1) [ ];
}
";
    assert_eq!(local(text, 7, "api"), [4]);
    assert_eq!(local(text, 8, "pkgs"), [1]);
    assert_eq!(local(text, 1, "pkgs"), [1]);
    assert_eq!(local(text, 4, "shopLib"), [3]);
    assert_eq!(local(text, 4, "api"), [4]);
    assert_eq!(local(text, 10, "name"), [9]);
    assert_eq!(local(text, 11, "port"), [9]);
    assert!(local(text, 13, "name").is_empty());
    assert_eq!(local(text, 14, "x"), [14]);
    assert_eq!(local(text, 17, "api"), [16]);
    assert_eq!(local(text, 18, "y"), [18]);
    assert!(local(text, 18, "x").is_empty());
    assert!(local(text, 7, "mkService").is_empty());
}

#[test]
fn nix_parameters_of_a_header_over_lines() {
    let text = "\
{
  config,
  # the packages
  pkgs ? import <nixpkgs> { },
  ...
}@args:
{
  a = pkgs.hello;
  b = args;
}
";
    assert_eq!(local(text, 8, "pkgs"), [4]);
    assert_eq!(local(text, 4, "pkgs"), [4]);
    assert_eq!(local(text, 9, "args"), [6]);
    let old = "{ config\n, pkgs\n, ...\n}:\n{\n  a = pkgs.hello;\n}\n";
    assert_eq!(local(old, 6, "pkgs"), [2]);
    assert!(local(old, 6, "hello").is_empty());
}

#[test]
fn nix_literals_hide_declarations() {
    let text = "\
a = ''
  b = 1;
  c = '''quoted''' ''${d} ''\\n
  e = 2;
'';
f = 3;
g = x // { h = ''
  i = 4;
''; };
j = \"over
k = 5;
\";
/* l = 6;
*/
m = 7; # n = 8;
";
    let hidden: Vec<usize> = literal_lines(Kind::Nix, text)
        .iter()
        .enumerate()
        .filter(|(_, h)| **h)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(hidden, [2, 3, 4, 5, 8, 9, 11, 12, 14]);
}

#[test]
fn nix_symbols_are_bindings_of_functions() {
    let nix = |line| one(Kind::Nix, line);
    for (line, name) in [
        ("  mkService = { name, port ? 8080 }: {", "mkService"),
        ("  double = x: x * 2;", "double"),
        ("  double = x:", "double"),
        ("  mkHost = args@{ config, ... }: { };", "mkHost"),
        ("  mkHost = { config, ... }@args: { };", "mkHost"),
        ("let f = a: b: a + b;", "f"),
        ("  my-fn' = x: x;", "my-fn'"),
    ] {
        assert_eq!(nix(line).as_deref(), Some(name), "{line}");
    }
    for line in [
        "  enable = true;",
        "  services.nginx.enable = true;",
        "  pkgs = import <nixpkgs> { };",
        "  url = https://example.com;",
        "  attrs = { a = 1; };",
    ] {
        assert_eq!(nix(line), None, "{line}");
    }
}

#[test]
fn a_nix_name_holds_its_dashes_and_primes() {
    let line = "  x = my-package.x' f'' b - c;";
    let word = |col| definition_word(Some(Kind::Nix), line, col).map(|(_, w)| w);
    assert_eq!(word(8), Some("my-package"));
    assert_eq!(word(18), Some("x'"));
    assert_eq!(word(21), Some("f''"));
    assert_eq!(word(28), Some("c"));
}

#[test]
fn nix_paths_name_files() {
    let files: Vec<PathBuf> = ["lib/default.nix", "hosts/web.nix", "hosts/nginx.nix"]
        .map(PathBuf::from)
        .to_vec();
    let open = |line: &str, col: usize, here: &str| {
        let path = nix_path(line, col)?;
        Some(nix_files(Path::new(here).parent().unwrap(), &path, &files))
    };
    let line = "  imports = [ ./nginx.nix ../lib ];";
    assert_eq!(nix_path(line, 17).as_deref(), Some("./nginx.nix"));
    assert_eq!(
        open(line, 17, "hosts/web.nix"),
        Some(vec![PathBuf::from("hosts/nginx.nix")])
    );
    assert_eq!(
        open(line, 29, "hosts/web.nix"),
        Some(vec![PathBuf::from("lib/default.nix")])
    );
    assert_eq!(
        open("x = import ./hosts/web.nix;", 20, "flake.nix"),
        Some(vec![PathBuf::from("hosts/web.nix")])
    );
    assert_eq!(open("x = import ./missing;", 14, "flake.nix"), Some(vec![]));
    assert_eq!(nix_path(line, 6), None);
    assert_eq!(nix_path("x = <nixpkgs/lib>;", 8), None);
    assert_eq!(nix_path(r#"x = "./nginx.nix";"#, 8), None);
    assert_eq!(nix_path(r#"url = "https://a/b";"#, 16), None);
    assert_eq!(nix_path("x = a / b;", 6), None);
    assert_eq!(nix_path(r#"x = "see ./nginx.nix";"#, 12), None);
    assert_eq!(nix_path(r#"x = "\" ./nginx.nix";"#, 12), None);
}

#[test]
fn nix_is_told_by_its_extension() {
    assert_eq!(kind_of(Path::new("hosts/web.nix")), Some(Kind::Nix));
    assert_eq!(kind_of(Path::new("flake.lock")), None);
    assert_eq!(word_chars(Some(Kind::Nix), false), "-'");
}

#[test]
fn nix_scopes_end_where_their_body_ends() {
    let sibling = "\
{ config, ... }:
let
  cfg = config.services.a;
in
{
  x = let
    cfg = config.services.b;
  in cfg.port;
  y = cfg.enable;
}
";
    assert_eq!(local(sibling, 9, "cfg"), [3]);
    assert_eq!(local(sibling, 8, "cfg"), [7]);
    let closed = "{ p }:\n{\n  xs = builtins.filter (p: p != null) [\n    p\n  ];\n}\n";
    assert_eq!(local(closed, 4, "p"), [1]);
    assert_eq!(local(closed, 3, "p"), [3]);
    let overlay = "final: prev:\n{\n  hello = prev.hello;\n}\n";
    assert_eq!(local(overlay, 3, "prev"), [1]);
    assert_eq!(local(overlay, 3, "final"), [1]);
    let one_line = "let a = 1; b = 2; in\nb + a\n";
    assert_eq!(local(one_line, 2, "b"), [1]);
    let with = "let\n  a = 1;\nin\nwith lib; assert a > 0; {\n  b = a;\n}\n";
    assert_eq!(local(with, 5, "a"), [2]);
    let branch = "let\n  a = 1;\nin\nif a > 0 then let b = a; in b else a\n";
    assert_eq!(local(branch, 4, "a"), [2]);
}

#[test]
fn nix_parameters_land_on_their_own_line() {
    let dashed = "{ lib\n, pkgs-unstable\n, pkgs\n}:\n{\n  a = pkgs.hello;\n}\n";
    assert_eq!(local(dashed, 6, "pkgs"), [3]);
    let default = "{ bar ? baz\n, baz\n}:\n{\n  a = baz;\n}\n";
    assert_eq!(local(default, 5, "baz"), [2]);
    let alias = "args@{ pkgs, ... }:\n{\n  a = args;\n  b = pkgs;\n}\n";
    assert_eq!(local(alias, 3, "args"), [1]);
    assert_eq!(local(alias, 4, "pkgs"), [1]);
    let nested = "{ meta ? { }, name }:\n{\n  a = name;\n}\n";
    assert_eq!(local(nested, 3, "name"), [1]);
    let inherits = "let\n  inherit (lib) mkIf;\n  \"my-attr\" = 1;\nin\nmkIf x\n";
    assert_eq!(local(inherits, 5, "mkIf"), [2]);
    assert_eq!(local(inherits, 5, "my-attr"), [3]);
}

#[test]
fn nix_literals_of_every_form() {
    let hidden = |text: &str| -> Vec<usize> {
        (literal_lines(Kind::Nix, text).iter().enumerate())
            .filter(|(_, h)| **h)
            .map(|(i, _)| i + 1)
            .collect()
    };
    assert_eq!(
        hidden("a = ''\n  x = '''\n  b = 1;\n'';\nc = 2;\n"),
        [2, 3, 4]
    );
    assert_eq!(
        hidden("a = ''\n  ''${x}\n  b = 1;\n'';\nc = 2;\n"),
        [2, 3, 4]
    );
    assert_eq!(hidden("x = \"a \\\" b\n  y = 1;\n\";\nz = 1;\n"), [2, 3]);
    assert_eq!(hidden("f' = ''\n  a = 1;\n'';\nb = 2;\n"), [2, 3]);
    assert!(hidden("let\n  f'' = 1;\n  g = 2;\nin\ng\n").is_empty());
    let nested = "{\n  a = ''\n    ${lib.optionalString x ''\n      dest=$out\n    ''}\n    b = 1\n  '';\n  c = 2;\n}\n";
    assert_eq!(hidden(nested), [3, 4, 5, 6, 7]);
}

#[test]
fn nix_lists_nothing_with_the_shared_pattern() {
    assert!(listed(Kind::Nix, "    function deploy() {").is_empty());
    assert_eq!(
        listed(Kind::Nix, "  mk = { name, meta ? {} }: name;"),
        ["mk"]
    );
}
