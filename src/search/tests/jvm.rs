use super::*;

#[test]
fn jvm_type_bodies_hold_members_and_other_blocks_hold_locals() {
    let member = |text: &str, line1: usize| {
        assert_eq!(jvm_local_block(text, line1), None, "a member in:\n{text}");
    };
    member("public @interface Tag {\n    String value();\n}\n", 2);
    member(
        "class A {\n    companion object {\n        val x = 1\n    }\n}\n",
        3,
    );
    member("extension (s: String)\n  def shout: String = s\n", 2);
    member("fun interface Op {\n    fun run()\n}\n", 2);
    member("record Point(int x) {\n    static int zero;\n}\n", 2);
    member("trait Shape {\n  def area: Double\n}\n", 2);
    member("class A {\n@Deprecated\n    void f() {}\n}\n", 3);
    member("class A\n{\n    void f() {}\n}\n", 3);
    member("class Box<\n    T\n> {\n    val x = 1\n}\n", 4);
    member("class Box[\n    T\n] {\n  val x = 1\n}\n", 4);
    let local = "fun f() {\n    val l = object : Runnable {\n        val n = 1\n    }\n}\n";
    assert_eq!(
        jvm_local_block(local, 3),
        Some(3),
        "an anonymous `object :` opens no type"
    );
}

#[test]
fn jvm_this_names_the_innermost_named_type() {
    let text = "class A {\n    val r = object : Runnable {\n        override fun run() {\n            this.x\n        }\n    }\n    fun g() {\n        this.x\n    }\n}\n";
    assert_eq!(jvm_this_owner(text, 4), None, "inside `object :`");
    assert_eq!(jvm_this_owner(text, 8).as_deref(), Some("A"));
    assert_eq!(jvm_this_owner("val x = this\n", 1), None, "column 0");
}

#[test]
fn jvm_parameters_bind_in_their_body() {
    let bound = |text: &str, line1: usize, name: &str| -> Vec<usize> {
        bindings(Kind::Jvm, text, line1, name)
            .iter()
            .map(|b| b.line1)
            .collect()
    };
    assert_eq!(bound("fun f(vararg xs: Int) {\n    xs\n}\n", 2, "xs"), [1]);
    assert_eq!(bound("void f(final String s) {\n    s;\n}\n", 2, "s"), [1]);
    assert_eq!(
        bound("void f(String... names) {\n    names;\n}\n", 2, "names"),
        [1]
    );
    assert!(bound("fun f(_: Int) {\n    _\n}\n", 2, "_").is_empty());
    assert!(bound("fun f() {\n    items.map { _ -> 1 }\n}\n", 2, "_").is_empty());
    assert!(
        bound(
            "fun f(germ: Int) {\n    class Sprout {\n        fun grow() = germ\n    }\n}\n",
            3,
            "germ"
        )
        .is_empty(),
        "the walk out of the blocks stops at a type's body"
    );
    assert!(
        bound("fun f() {\n    val it = 1\n    it\n}\n", 3, "it").is_empty(),
        "`it`, `this` and `super` never bind"
    );
    assert_eq!(
        bound(
            "def f(t: Tree) =\n  t match\n    case pkg.Leaf(x) => x\n",
            3,
            "x"
        ),
        [3]
    );
    assert!(
        bound(
            "def f(t: Tree) =\n  t match\n    case pkg.Leaf(x) => pkg\n",
            3,
            "pkg"
        )
        .is_empty(),
        "a name followed by `.` or `(` in a pattern binds nothing"
    );
}

#[test]
fn jvm_function_names_methods_and_constructors_but_no_type() {
    assert_eq!(jvm_function("class A(val x: Int) {"), None);
    assert_eq!(jvm_function("data class P(val x: Int)"), None);
    assert_eq!(
        jvm_function("    constructor(x: Int) : this()").as_deref(),
        Some("constructor")
    );
    assert_eq!(
        jvm_function("    fun String.shout(times: Int) {").as_deref(),
        Some("shout")
    );
    assert_eq!(
        jvm_function("    public Basket(int size) {").as_deref(),
        Some("Basket")
    );
}

#[test]
fn jvm_anonymous_classes_enclose_but_name_nothing() {
    let text = "class A {\n    void f() {\n        new Runnable() {\n            int n;\n        };\n    }\n    val r = object : Runnable {\n        val m = 1\n    }\n}\n";
    assert_eq!(jvm_enclosing_types(text, 4), [3, 1]);
    assert_eq!(jvm_enclosing_types(text, 8), [7, 1]);
    assert_eq!(jvm_type_name("        new Runnable() {"), None);
    assert_eq!(jvm_type_name("    val r = object : Runnable {"), None);
    assert_eq!(jvm_type_name("    companion object {"), None);
    assert_eq!(
        jvm_type_name("    companion object Factory {").as_deref(),
        Some("Factory")
    );
}

#[test]
fn jvm_members_of_a_type() {
    let java = "class A {\n    A() {}\n    int size;\n}\n";
    assert!(
        jvm_members_of(java, 1, "A").is_empty(),
        "a constructor is no member"
    );
    assert_eq!(jvm_members_of(java, 1, "size"), [3]);
    let record = "record Point(\n    int x,\n    int y\n) {}\n";
    assert!(jvm_members_of(record, 1, "x").contains(&2));
    assert_eq!(
        jvm_members_of("record Point(int x, int y) {}\n", 1, "y"),
        [1]
    );
    let kotlin = "class A {\n    companion object {\n        fun make() = A()\n    }\n}\n";
    assert_eq!(jvm_members_of(kotlin, 1, "make"), [3]);
}

#[test]
fn jvm_bases_are_simple_names() {
    let bases = |text: &str| jvm_bases(text, 1, false);
    assert_eq!(bases("class A : Base(), Face {"), ["Base", "Face"]);
    assert_eq!(bases("class A : Base(x, y), Face {"), ["Base", "Face"]);
    assert_eq!(bases("class A : pkg.Base() {"), ["Base"]);
    assert_eq!(
        bases("class A<T> : Base(), Face where T : Comparable<T>, T : Any {"),
        ["Base", "Face"]
    );
    assert_eq!(bases("class A extends Base implements Face {"), ["Base"]);
    assert_eq!(
        jvm_bases(
            "case class P(x: Int) extends Base derives Eq, Show\n",
            1,
            true
        ),
        ["Base"]
    );
}

#[test]
fn jvm_type_parameters_and_accessors() {
    assert!(jvm_type_parameter("class Box<T> {", "T"));
    assert!(jvm_type_parameter("fun <T> pick(a: T) = a", "T"));
    assert!(jvm_type_parameter("class Box<out T> {", "T"));
    assert!(jvm_type_parameter("inline fun <reified T> f() {}", "T"));
    assert!(!jvm_type_parameter("class Box<T> {", "Box"));

    let read = |w: &str| jvm_accessor(w).map(|a| (a.fields_it_may_read, a.setter));
    assert_eq!(
        read("isActive"),
        Some((vec!["active".to_owned(), "isActive".to_owned()], false))
    );
    assert_eq!(read("getTitle"), Some((vec!["title".to_owned()], false)));
    assert_eq!(read("setTitle"), Some((vec!["title".to_owned()], true)));
    assert_eq!(read("getter"), None);
    assert_eq!(read("settle"), None);

    let fields = "@Data\nclass User {\n    private String title;\n}\n";
    assert!(
        jvm_lombok_fields(fields, 2, "getTitle").is_empty(),
        "without an `import lombok.` Lombok writes nothing"
    );
    assert_eq!(
        jvm_lombok_fields(&format!("import lombok.Data;\n{fields}"), 3, "getTitle"),
        [4]
    );
}

#[test]
fn jvm_assigned_calls_are_of_a_lowercase_name() {
    let call = |line: &str| jvm_assigned_call(line, "s").map(|c| (c.receiver, c.method));
    assert_eq!(
        call("    var s = plugin.statusNonNull();"),
        Some((Some("plugin".to_owned()), "statusNonNull".to_owned()))
    );
    assert_eq!(call("    val s = make()"), Some((None, "make".to_owned())));
    assert_eq!(call("    val s = Make()"), None);
}
