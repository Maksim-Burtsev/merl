use super::*;

fn call(line: &str, word: &str) -> Option<SwiftCall> {
    let after = line.find(word)? + word.len();
    swift_call_labels(&[line], 0, after)
}

fn params(decl: &str, name: &str) -> Option<Vec<SwiftParam>> {
    let lines: Vec<&str> = decl.lines().collect();
    swift_param_labels(&lines, 0, name)
}

fn labels(decl: &str, name: &str) -> Option<Vec<String>> {
    Some(params(decl, name)?.into_iter().map(|p| p.label).collect())
}

fn fits(line: &str, word: &str, decl: &str) -> bool {
    swift_call_fits(
        &call(line, word).expect("a call"),
        &params(decl, word).expect("a declaration"),
    )
}

#[test]
fn a_call_and_a_declaration_are_read_label_by_label() {
    let v = |s: &[&str]| Some(s.iter().map(|l| (*l).to_owned()).collect::<Vec<_>>());
    let called = |line: &str, word: &str| call(line, word).map(|c| c.labels);
    assert_eq!(called("k.pour(over: Saucer())", "pour"), v(&["over"]));
    assert_eq!(
        called("pour(5, b ? x : y, into: c)", "pour"),
        v(&["_", "_", "into"])
    );
    assert_eq!(called("let f = k.pour(into:)", "pour"), v(&["into"]));
    assert_eq!(called("k.pour()", "pour"), v(&[]));
    assert_eq!(
        call("k.boil(then: 1) { }", "boil"),
        Some(SwiftCall {
            labels: vec!["then".to_owned()],
            trailing_closure: true
        }),
        "a trailing closure"
    );
    assert_eq!(
        call("k.boil { }", "boil"),
        Some(SwiftCall {
            labels: Vec::new(),
            trailing_closure: true
        })
    );
    assert_eq!(call("if k.empty {", "empty"), None, "a condition's body");
    assert_eq!(call("k.pour", "pour"), None, "no call");

    assert_eq!(
        labels("    func pour(into cup: Cup) {}", "pour"),
        v(&["into"])
    );
    assert_eq!(
        labels("func pour<T>(_ a: T, b: Int)", "pour"),
        v(&["_", "b"])
    );
    assert_eq!(
        labels("    init?(from saucer: Saucer) {", "init"),
        v(&["from"])
    );
    assert_eq!(
        labels("func pour(\n    into cup: Cup\n) {", "pour"),
        v(&["into"])
    );
    assert_eq!(
        labels("var pour: (Int) -> Void", "pour"),
        None,
        "a closure property"
    );
}

#[test]
fn a_call_fits_an_overload_by_its_defaults_and_its_trailing_closure() {
    let auth =
        "func authenticate(username: String, password: String, persistence: P = .forSession)";
    assert!(fits(
        "r.authenticate(username: u, password: p)",
        "authenticate",
        auth
    ));
    assert!(fits(
        "r.authenticate(username: u, password: p, persistence: .none)",
        "authenticate",
        auth
    ));
    assert!(!fits("r.authenticate(with: c)", "authenticate", auth));
    let with = "func authenticate(with credential: URLCredential) -> Self";
    assert!(!fits(
        "r.authenticate(username: u, password: p)",
        "authenticate",
        with
    ));
    let write = "func write<U>(_ closure: (inout T) throws -> U) rethrows -> U";
    assert!(fits("state.write { $0 = 1 }", "write", write));
    assert!(!fits(
        "state.write(1)",
        "write",
        "func write(_ value: T, on queue: Q)"
    ));
    assert!(fits("state.write(1)", "write", "func write(_ value: T)"));
    let curl = "func cURLDescription(on queue: DispatchQueue, calling handler: @escaping (String) -> Void) -> Self";
    assert!(fits(
        "r.cURLDescription(on: q) { _ in }",
        "cURLDescription",
        curl
    ));
    assert!(!fits("r.cURLDescription { _ in }", "cURLDescription", curl));
    let optional = "func load(done: (() -> Void)? = nil)";
    assert!(fits("x.load { }", "load", optional));
    assert!(fits("x.load()", "load", optional));
    let sum = "func sum(_ xs: Int...)";
    assert!(fits("sum(1, 2, 3)", "sum", sum));
}

#[test]
fn a_header_lists_what_it_inherits() {
    assert_eq!(
        swift_inherited("final class Kettle: Brewing {"),
        ["Brewing"]
    );
    assert_eq!(
        swift_inherited("struct Box<T>: Equatable, Store<T> where T: Hashable {"),
        ["Equatable", "Store"]
    );
    assert_eq!(swift_inherited("extension Teapot: Brewing {}"), ["Brewing"]);
    assert_eq!(swift_inherited("struct Teapot {"), Vec::<String>::new());
}

#[test]
fn a_binding_starts_right_of_its_equals_sign() {
    assert!(swift_right_of_binding("        let validators = "));
    assert!(swift_right_of_binding("if let x = "));
    assert!(swift_right_of_binding("var n: [Int]="));
    assert!(!swift_right_of_binding("        let "));
    assert!(!swift_right_of_binding("let a = 1; let "));
    assert!(!swift_right_of_binding("guard let == "));
    assert!(!swift_right_of_binding("let x: Set<Int>>= "));
    assert!(!swift_right_of_binding("x = "));
}
