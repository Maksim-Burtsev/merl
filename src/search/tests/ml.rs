use super::*;

fn declares(kind: Kind, line: &str, word: &str) -> bool {
    let patterns = ml_patterns(kind, word);
    !patterns.is_empty() && Regex::new(&patterns.join("|")).unwrap().is_match(line)
}

#[test]
fn ocaml_declaration_forms() {
    for (line, word) in [
        ("let format_price { cents } = 1", "format_price"),
        ("let rec loop acc = function", "loop"),
        ("and helper acc = 1", "helper"),
        ("let%lwt user = fetch () in", "user"),
        ("  let* typ = format () in", "typ"),
        ("  and+ rest = more () in", "rest"),
        ("let x' = x + 1 in", "x'"),
        (
            r#"external length : string -> int = "caml_string_length""#,
            "length",
        ),
        ("exception Invalid_price of int", "Invalid_price"),
        ("type t = { cents : int }", "t"),
        ("type 'a tree = Leaf | Node of 'a", "tree"),
        ("type ('k, 'v) table = unit", "table"),
        ("and account = { owner : string }", "account"),
        ("type status = Active | Closed", "Active"),
        ("type status = Active | Closed", "Closed"),
        ("type shape = Circle of float | Square of float", "Square"),
        ("  | Circle of float", "Circle"),
        ("  | Lit : int -> int expr", "Lit"),
        ("  cents : int;", "cents"),
        ("  mutable total : int;", "total"),
        ("type t = { cents : int; mutable total : int }", "total"),
        ("module Ledger = struct", "Ledger"),
        ("module Ledger : sig", "Ledger"),
        ("module type S = sig", "S"),
        ("module M = Make (X)", "M"),
        ("module Make (X : S) = struct", "Make"),
        ("  val format_price : t -> string", "format_price"),
        ("class point x = object", "point"),
        ("  method get_x = px", "get_x"),
        ("  method private hidden = 1", "hidden"),
        ("  val mutable px = x", "px"),
    ] {
        assert!(declares(Kind::Ocaml, line, word), "{line}: {word}");
    }
}

#[test]
fn ocaml_refusals() {
    for (line, word) in [
        ("  prefix ^ Money.format_price m", "format_price"),
        ("open Money", "Money"),
        ("include Money", "Money"),
        ("  inherit point x", "point"),
        ("let area (s : shape) =", "shape"),
        ("  | Circle r -> r *. r", "Circle"),
        ("let reset a = { a with total = 0 }", "total"),
        ("let owner_of a = a.owner", "owner"),
        ("  acc := !acc + x", "acc"),
        ("let () = main ()", "_"),
        ("let _ = main ()", "_"),
        ("let x' = 1", "x"),
        ("let x = 1", "x'"),
        ("let ( +! ) a b = a + b", "a"),
        ("let status = `Active", "Active"),
    ] {
        assert!(!declares(Kind::Ocaml, line, word), "{line}: {word}");
    }
}

#[test]
fn fsharp_declaration_forms() {
    for (line, word) in [
        ("let formatPrice (m: Money) = 1", "formatPrice"),
        ("let rec loop acc xs =", "loop"),
        ("and helper acc = 1", "helper"),
        ("let private fee = 2", "fee"),
        ("    let inline twice x = x * 2", "twice"),
        ("let mutable counter = 0", "counter"),
        ("[<Literal>] let Limit = 10", "Limit"),
        ("let ``returns empty list`` () = 1", "returns empty list"),
        ("    member this.Total = count", "Total"),
        ("    member _.Total = count", "Total"),
        ("    override this.ToString() = \"a\"", "ToString"),
        ("    static member Create(id) = Account(id)", "Create"),
        ("    abstract member Area : float", "Area"),
        ("    abstract Find : int -> string", "Find"),
        ("    default this.Area = 0.0", "Area"),
        ("    val mutable count : int", "count"),
        ("type Money = { Cents: int }", "Money"),
        ("type Money = { Cents: int }", "Cents"),
        ("type Coupon = { Code: string; mutable Rate: int }", "Rate"),
        ("    Cents: int", "Cents"),
        ("type Shape =", "Shape"),
        ("    | Circle of float", "Circle"),
        ("type Account(id: int) =", "Account"),
        ("type IRepo =", "IRepo"),
        ("exception NotFound of string", "NotFound"),
        ("module Ledger =", "Ledger"),
        ("module Shop.Money", "Money"),
        ("namespace Shop.Orders", "Orders"),
    ] {
        assert!(declares(Kind::Fsharp, line, word), "{line}: {word}");
    }
}

#[test]
fn fsharp_refusals() {
    for (line, word) in [
        ("    prefix + formatPrice m", "formatPrice"),
        ("open Shop.Money", "Money"),
        ("    inherit Base()", "Base"),
        ("let area (s: Shape) =", "Shape"),
        ("    | Circle r -> r * r", "Circle"),
        ("let reset c = { c with Rate = 0 }", "Rate"),
        ("let () = main ()", "_"),
        ("let (|Even|Odd|) n = 1", "Even"),
        ("let ( +! ) a b = a + b", "a"),
    ] {
        assert!(!declares(Kind::Fsharp, line, word), "{line}: {word}");
    }
}

#[test]
fn a_let_is_a_module_item_at_the_top_or_directly_inside_a_module() {
    let ocaml = [
        "let label m =",
        "  let prefix = 1 in",
        "  prefix",
        "module Ledger = struct",
        "  let add x = x",
        "end",
        "  and helper = 1",
    ];
    assert!(ml_declares(Kind::Ocaml, &ocaml, 1, "label"));
    assert!(!ml_declares(Kind::Ocaml, &ocaml, 2, "prefix"));
    assert!(ml_declares(Kind::Ocaml, &ocaml, 5, "add"));
    let fsharp = [
        "module Ledger =",
        "    let add x = x",
        "    let total xs =",
        "        let acc = 0",
        "        acc",
        "type Account() =",
        "    let mutable count = 0",
        "type Money = {",
        "    Cents: int",
        "}",
    ];
    assert!(ml_declares(Kind::Fsharp, &fsharp, 2, "add"));
    assert!(!ml_declares(Kind::Fsharp, &fsharp, 4, "acc"));
    assert!(!ml_declares(Kind::Fsharp, &fsharp, 7, "count"));
    assert!(ml_declares(Kind::Fsharp, &fsharp, 9, "Cents"));
}

#[test]
fn a_local_is_the_nearest_let_above_in_its_item() {
    let lines = [
        "let first xs =",
        "  let acc = 0 in",
        "  acc",
        "let second xs =",
        "  acc",
        "let third xs =",
        "  let acc = 1 in",
        "  let acc = acc + 1 in",
        "  acc",
    ];
    assert_eq!(ml_local(Kind::Ocaml, &lines, 3, "acc"), Some(2));
    assert_eq!(ml_local(Kind::Ocaml, &lines, 5, "acc"), None);
    assert_eq!(ml_local(Kind::Ocaml, &lines, 9, "acc"), Some(8));
    assert_eq!(ml_local(Kind::Ocaml, &lines, 1, "first"), None);
}

#[test]
fn ocaml_literals() {
    let text = [
        "(* outer (* inner *)",
        "let fake = 0",
        "*)",
        "(* \"*)\" still a comment",
        "let fake = 1",
        "*)",
        "let s = {sql|",
        "let fake = 2",
        "|sql}",
        "let q = \"a",
        "let fake = 3",
        "\"",
        "let c = '\"'",
        "let v = `Active",
        "let a' = 'a'",
        "let real = 4",
    ]
    .join("\n");
    let lit = ml_literal_lines(Kind::Ocaml, &text);
    let hidden: Vec<usize> = (0..16).filter(|&i| lit[i]).map(|i| i + 1).collect();
    assert_eq!(hidden, [2, 3, 5, 6, 8, 9, 11, 12]);
}

#[test]
fn fsharp_literals() {
    let text = [
        "let product = List.reduce (*) xs",
        "let real = 0",
        "(* outer (* inner *)",
        "let fake = 1",
        "*)",
        "let v = @\"C:\\path\\\"\"",
        "let fake = 2",
        "\"",
        "let t = \"\"\"",
        "let fake = 3",
        "\"\"\"",
        "let ``a \"quoted\" name`` () = 1 // \"",
        "let i = $\"{x}\"",
        "let real = 4",
    ]
    .join("\n");
    let lit = ml_literal_lines(Kind::Fsharp, &text);
    let hidden: Vec<usize> = (0..14).filter(|&i| lit[i]).map(|i| i + 1).collect();
    assert_eq!(hidden, [4, 5, 7, 8, 10, 11]);
}

#[test]
fn a_name_takes_its_primes_and_a_backticked_name_is_whole() {
    let line = "let x' = x'' + ``returns empty list`` 'a";
    let at = |word: &str| line.find(word).unwrap();
    let word =
        |col: usize| definition_word(Some(Kind::Ocaml), line, col).map(|(_, w)| w.to_owned());
    assert_eq!(word(at("x'")).as_deref(), Some("x'"));
    assert_eq!(word(at("x''")).as_deref(), Some("x''"));
    assert_eq!(word(at("'a") + 1).as_deref(), Some("a"));
    let fs = |col: usize| definition_word(Some(Kind::Fsharp), line, col).map(|(_, w)| w.to_owned());
    assert_eq!(fs(at("empty")).as_deref(), Some("returns empty list"));
}

#[test]
fn ml_symbols_list_module_items_and_skip_signatures() {
    let lines = [
        "let label m =",
        "  let prefix = 1 in",
        "  prefix",
        "module Ledger = struct",
        "  let add x = x",
        "end",
    ];
    let ml = Path::new("lib/cart.ml");
    assert!(ml_symbol_kept(Kind::Ocaml, ml, &lines, 1));
    assert!(!ml_symbol_kept(Kind::Ocaml, ml, &lines, 2));
    assert!(ml_symbol_kept(Kind::Ocaml, ml, &lines, 5));
    assert!(!ml_symbol_kept(
        Kind::Ocaml,
        Path::new("lib/cart.mli"),
        &lines,
        1
    ));
    let row = |kind: Kind, line: &str| {
        SYMBOLS
            .iter()
            .filter(|(k, _)| *k == Some(kind))
            .find_map(|(_, p)| symbol_name(&Regex::new(p).unwrap(), line))
    };
    assert_eq!(
        row(Kind::Ocaml, "module type S = sig").as_deref(),
        Some("S")
    );
    assert_eq!(
        row(Kind::Ocaml, "type 'a tree = Leaf").as_deref(),
        Some("tree")
    );
    assert_eq!(row(Kind::Ocaml, "let () = main ()"), None);
    assert_eq!(row(Kind::Ocaml, "let _ = main ()"), None);
    assert_eq!(
        row(Kind::Fsharp, "    member this.Total = 1").as_deref(),
        Some("Total")
    );
    assert_eq!(
        row(Kind::Fsharp, "let private fee = 2").as_deref(),
        Some("fee")
    );
    assert!(!shared_symbols(Some(Kind::Ocaml)) && !shared_symbols(Some(Kind::Fsharp)));
}

#[test]
fn kinds_and_roots() {
    for (file, kind) in [
        ("a.ml", Kind::Ocaml),
        ("a.mli", Kind::Ocaml),
        ("a.fs", Kind::Fsharp),
        ("a.fsi", Kind::Fsharp),
        ("a.fsx", Kind::Fsharp),
    ] {
        assert_eq!(kind_of(Path::new(file)), Some(kind), "{file}");
    }
    assert_eq!(
        ocaml_roots(Some("/opam/default/lib/ocaml\n".into())),
        [
            PathBuf::from("/opam/default/lib/ocaml"),
            PathBuf::from("/opam/default/lib")
        ]
    );
    assert!(ocaml_roots(None).is_empty());
    assert_eq!(
        ml_modules(
            Path::new("lib/pricing.ml"),
            &["module Ledger = struct", "  let add x = x"],
            2
        ),
        ["Pricing", "Ledger"]
    );
}
