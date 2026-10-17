use super::*;

fn call(line: &str, word: &str) -> Option<Vec<String>> {
    let after = line.find(word)? + word.len();
    swift_call_labels(&[line], 0, after)
}

fn params(decl: &str, name: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = decl.lines().collect();
    swift_param_labels(&lines, 0, name)
}

#[test]
fn a_call_and_a_declaration_are_read_label_by_label() {
    let v = |s: &[&str]| Some(s.iter().map(|l| (*l).to_owned()).collect::<Vec<_>>());
    assert_eq!(call("k.pour(over: Saucer())", "pour"), v(&["over"]));
    assert_eq!(
        call("pour(5, b ? x : y, into: c)", "pour"),
        v(&["_", "_", "into"])
    );
    assert_eq!(call("let f = k.pour(into:)", "pour"), v(&["into"]));
    assert_eq!(call("k.pour()", "pour"), v(&[]));
    assert_eq!(
        call("k.boil(then: 1) { }", "boil"),
        None,
        "a trailing closure"
    );
    assert_eq!(call("k.pour", "pour"), None, "no call");

    assert_eq!(
        params("    func pour(into cup: Cup) {}", "pour"),
        v(&["into"])
    );
    assert_eq!(
        params("func pour<T>(_ a: T, b: Int)", "pour"),
        v(&["_", "b"])
    );
    assert_eq!(
        params("    init?(from saucer: Saucer) {", "init"),
        v(&["from"])
    );
    assert_eq!(
        params("func pour(\n    into cup: Cup\n) {", "pour"),
        v(&["into"])
    );
    assert_eq!(params("func pour(a: Int = 1)", "pour"), None, "a default");
    assert_eq!(params("func pour(a: Int...)", "pour"), None, "variadic");
    assert_eq!(
        params("var pour: (Int) -> Void", "pour"),
        None,
        "a closure property"
    );
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
