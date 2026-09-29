//! Strings over several lines (#346): nothing in one names code, and no line of one declares.

pub struct Choose;

pub fn usage() -> &'static str {
    "\
error: unrecognized style. \
    Choose from: bold, italic.
//  ^ d: none
    // status: no definition for Choose
"
}

pub fn fixture() -> &'static str {
    r"
fn spare() {}
"
}

pub fn quoted() -> &'static str {
    r#"a "quoted" word, then
fn phantom() {}
"#
}

pub fn pick<'a>(s: &'a str) -> Choose {
//                             ^ d: src/texts.rs:3
    let _ = (s, '"', '\'', '\u{1F600}', "**/*.rs", b"x", br##"a "# b"##);
    Choose
}

pub fn call() -> String {
    spare();
//  ^ d: none
    phantom();
//  ^ d: none
    let label = usage();
    pick(label);
//  ^ d: src/texts.rs:26
    after();
//  ^ d: src/texts.rs:49
    format!("{label} {{label}} {label:?} label")
    //        ^ d: src/texts.rs:37
    //                 ^ d: none
    //                          ^ d: src/texts.rs:37
    //                                   ^ d: none
}

pub fn after() {}
