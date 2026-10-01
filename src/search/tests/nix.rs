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
        ("  free = (rate == 0);", "free_"),
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
fn nix_let_bindings_declare_only_where_they_are_read() {
    let lines = [
        "let",
        "  api = 1;",
        "# a comment at the margin",
        "  cfg = {",
        "    port = 1;",
        "  };",
        "in",
        "{",
        "  api = 2;",
        "  x = let",
        "    api = 3;",
        "  in api;",
        "}",
        "let api = 4;",
    ];
    let bound: Vec<usize> = (1..=lines.len())
        .filter(|&l| lines[l - 1].contains('=') && nix_let_bound(&lines, l))
        .collect();
    assert_eq!(bound, [2, 4, 11, 14]);
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
    assert_eq!(local(text, 9, "args"), [1]);
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
    let line = "  x = my-package.x' ''a'' b - c;";
    let word = |col| definition_word(Some(Kind::Nix), line, col).map(|(_, w)| w);
    assert_eq!(word(8), Some("my-package"));
    assert_eq!(word(18), Some("x'"));
    assert_eq!(word(23), Some("a"));
    assert_eq!(word(30), Some("c"));
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
}

#[test]
fn nix_is_told_by_its_extension() {
    assert_eq!(kind_of(Path::new("hosts/web.nix")), Some(Kind::Nix));
    assert_eq!(kind_of(Path::new("flake.lock")), None);
    assert_eq!(word_chars(Some(Kind::Nix), false), "-'");
}
