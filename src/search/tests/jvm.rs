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
    assert_eq!(
        bound("fun f(_: Int) {\n    _\n}\n", 2, "_"),
        [] as [usize; 0]
    );
    assert_eq!(
        bound("fun f() {\n    items.map { _ -> 1 }\n}\n", 2, "_"),
        [] as [usize; 0]
    );
    assert_eq!(
        bound(
            "fun f(germ: Int) {\n    class Sprout {\n        fun grow() = germ\n    }\n}\n",
            3,
            "germ"
        ),
        [1],
        "the walk goes on past a local class"
    );
    assert!(
        bound(
            "class Pot {\n    class Sprout {\n        fun grow() = germ\n    }\n}\n",
            3,
            "germ"
        )
        .is_empty(),
        "the walk stops at a type's body that is a member or top level"
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

#[test]
fn a_local_initialized_by_a_literal_has_the_literal_s_type() {
    let kt = |line: &str| jvm_literal_type(line, "s", false);
    assert_eq!(
        kt(r#"    val s = " fun slosh(): Int = 0 ""#),
        Some("String")
    );
    assert_eq!(kt(r#"    val s = "a \" $b""#), Some("String"));
    assert_eq!(kt("    var s = 'c'"), Some("Char"));
    assert_eq!(kt("    val s = 1_000"), Some("Int"));
    assert_eq!(kt("    val s = 3L"), Some("Long"));
    assert_eq!(kt("    val s = 2.5"), Some("Double"));
    assert_eq!(kt("    val s = true"), Some("Boolean"));
    assert_eq!(kt(r#"    val s = "a" + b"#), None);
    assert_eq!(kt(r#"    val s = "a".length"#), None);
    assert_eq!(kt(r#"    val s = "a${f("b")}""#), None);
    let java = |line: &str| jvm_literal_type(line, "s", true);
    assert_eq!(java(r#"        var s = "x";"#), Some("String"));
    assert_eq!(java("        var s = 1;"), None);
}

#[test]
fn a_kotlin_call_chain_is_read_back_to_its_head() {
    let chain = |b: &str| kotlin_call_chain(b).map(|c| (c.head, c.constructed, c.links));
    assert_eq!(
        chain("    return built.grounds + Recipe.Builder().grounds(1).finish()."),
        Some((
            "Recipe.Builder".into(),
            true,
            vec!["grounds()".into(), "finish()".into()]
        ))
    );
    assert_eq!(
        chain("    val x = carafe.pour(f(2), \"a\").brim."),
        Some(("carafe".into(), false, vec!["pour()".into(), "brim".into()]))
    );
    assert_eq!(
        chain("    val x = carafe.brim."),
        None,
        "no call: a qualifier"
    );
    assert_eq!(chain("    val x = pour().brim."), None, "a bare call");
    assert_eq!(chain("    val x = xs[0].pour()."), None);
    assert_eq!(chain("    val x = \"a\".pour()."), None);
    let text =
        "val built = Recipe.Builder()\n    .grounds(2)\n    // .x()\n    ?.water(3)\nval y = 1\n";
    assert_eq!(
        kotlin_initializer(text, 1, "built").as_deref(),
        Some("Recipe.Builder().grounds(2).water(3)")
    );
    let d = kotlin_destructured("    for ((stroke, mesh) in plungers) total += 1", "mesh")
        .map(|d| (d.index, d.source, d.in_loop));
    assert_eq!(d, Some((1, "plungers".into(), true)));
    let d = kotlin_destructured("    val (stroke, mesh) = plunger", "stroke")
        .map(|d| (d.index, d.source, d.in_loop));
    assert_eq!(d, Some((0, "plunger".into(), false)));
    assert!(kotlin_destructured("    val (a, b) = a", "b").is_none());
    let data = "data class Plunger(\n    val stroke: Int,\n    val mesh: press.Mesh,\n)\n";
    assert_eq!(kotlin_component_type(data, 1, 0).as_deref(), Some("Int"));
    assert_eq!(kotlin_component_type(data, 1, 1), None, "a package path");
    assert_eq!(kotlin_component_type("class P(val a: Int)\n", 1, 0), None);
}

#[test]
fn a_java_element_comes_out_of_a_written_collection() {
    assert_eq!(
        java_element_access("        total += stones.get(0).").as_deref(),
        Some("stones")
    );
    assert_eq!(
        java_element_access("        x = q.peek()."),
        Some("q".into())
    );
    assert_eq!(java_element_access("        x = q.size()."), None);
    assert_eq!(java_element_access("        x = a.b.get(0)."), None);
    let field = "    private final List<Stone> stones = new ArrayList<>();";
    assert_eq!(java_element_type(field, "stones").as_deref(), Some("Stone"));
    assert_eq!(
        java_element_type("    Map<K, Stone> stones;", "stones"),
        None
    );
    assert_eq!(
        java_element_type("    Optional<? extends a.B> stones,", "stones"),
        None
    );
}

#[test]
fn a_java_class_lists_its_interfaces_apart_from_its_superclass() {
    let text =
        "public class Quern extends Stone<A, B>\n        implements Turnable, x.Grind<C> {\n}\n";
    assert_eq!(jvm_bases(text, 1, false), ["Stone"]);
    assert_eq!(java_implements(text, 1), ["Turnable", "Grind"]);
    assert_eq!(
        java_implements("class K : Lever {\n}\n", 1),
        Vec::<String>::new()
    );
}

#[test]
fn a_java_member_is_a_method_when_its_name_opens_a_parameter_list() {
    assert!(java_declares_method(
        "    public Hopper hopper() {",
        "hopper"
    ));
    assert!(!java_declares_method(
        "    private Hopper hopper = hopper();",
        "hopper"
    ));
    assert!(!java_declares_method("    private int burr = 3;", "burr"));
}

#[test]
fn a_kotlin_scope_function_hands_on_its_receiver() {
    let returned = |b: &str| kotlin_scope_returned(b);
    assert_eq!(
        returned("    val d = plunger.also { it.stroke }.mesh.").as_deref(),
        Some("    val d = plunger.mesh.")
    );
    assert_eq!(
        returned("    val c = carafe.apply { a { b() } }.").as_deref(),
        Some("    val c = carafe.")
    );
    assert_eq!(returned("    val c = carafe.let { it }."), None);
    assert_eq!(returned("    val c = carafe.apply { it }.f()."), None);
    let text = "fun f() {\n    with(carafe) {\n        slosh()\n    }\n}\n";
    assert_eq!(
        kotlin_lambda_receiver(text, 3, "        ").as_deref(),
        Some("carafe")
    );
    assert_eq!(
        kotlin_lambda_receiver("", 1, "    x.run { y.let { ").as_deref(),
        None
    );
    assert_eq!(
        kotlin_lambda_receiver("", 1, "    x.apply { ").as_deref(),
        Some("x")
    );
    assert_eq!(
        kotlin_lambda_subject("", 1, "    x.also { it.", "it").as_deref(),
        Some("x")
    );
    assert_eq!(
        kotlin_lambda_subject("", 1, "    x.let { c -> c.", "c").as_deref(),
        Some("x")
    );
    assert_eq!(
        kotlin_lambda_subject("", 1, "    x.let { c -> it.", "it"),
        None
    );
}

#[test]
fn a_kotlin_when_subject_keeps_its_type_in_else_and_casts_when_bound_by_val() {
    let text = "fun f(v: Urn, x: Any) {\n    when (v) {\n        is Glass -> v.dose()\n        else -> v.scald()\n    }\n    when (val g = x) {\n        is Samovar -> g.mint()\n    }\n}\n";
    assert_eq!(jvm_when_else(text, 4, "v"), Some(2));
    assert_eq!(jvm_when_else(text, 3, "v"), None, "an `is` branch");
    assert_eq!(jvm_when_else(text, 4, "x"), None, "not the subject");
    assert_eq!(
        jvm_smart_cast(text, 7, "        is Samovar -> g.", "g").as_deref(),
        Some("Samovar")
    );
}

#[test]
fn a_kotlin_property_initializer_reads_the_primary_constructor_parameter() {
    let text = "class S(vapor: Int) {\n    private val vapor: Int = vapor\n    val x: Int get() = vapor\n    fun f() = vapor\n}\n";
    let at =
        |line1: usize, before: &str| kotlin_initializer_parameter(text, line1, before, "vapor");
    assert_eq!(at(2, "    private val vapor: Int = "), Some(1));
    assert_eq!(at(2, "    private val "), None, "the declared name");
    assert_eq!(at(3, "    val x: Int get() = "), None, "a getter");
    assert_eq!(at(4, "    fun f() = "), None, "a function");
}

#[test]
fn a_kotlin_extension_function_s_body_has_its_receiver_as_this() {
    let text = "fun Glass.dip(): Int = dose(1) + with(x) { dose() }\nfun Glass.pour(): Int {\n    return dose(1)\n}\nclass A {\n    fun Glass.tip() = dose()\n}\n";
    let at = |line1: usize, before: &str| {
        kotlin_extension_at(text, line1, before).map(|e| (e.name, e.receiver))
    };
    let glass = |name: &str| Some((name.to_owned(), "Glass".to_owned()));
    assert_eq!(at(1, "fun Glass.dip(): Int = "), glass("dip"));
    assert_eq!(
        at(1, "fun Glass.dip(): Int = dose(1) + with(x) { "),
        None,
        "a lambda"
    );
    assert_eq!(at(1, "fun Glass."), None, "the header");
    assert_eq!(at(3, "    return "), glass("pour"));
    assert_eq!(at(6, "    fun Glass.tip() = "), None, "a member extension");
}

#[test]
fn a_kotlin_lambda_over_a_written_collection_binds_its_element() {
    let text = "fun f(glasses: List<Glass>, n: Map<String, Glass>) {\n    glasses.sumOf { it.dose() }\n    glasses.forEach { g ->\n        g.dose()\n    }\n}\n";
    let at = |line1: usize, before: &str, name: &str| {
        kotlin_lambda_collection(text, line1, before, name)
    };
    assert_eq!(
        at(2, "    glasses.sumOf { ", "it").as_deref(),
        Some("glasses")
    );
    assert_eq!(
        at(4, "        ", "g").as_deref(),
        Some("glasses"),
        "the lambda's header"
    );
    assert_eq!(
        at(2, "    glasses.sumOf { x -> ", "it"),
        None,
        "a named parameter"
    );
    assert_eq!(at(2, "    glasses.let { ", "it"), None, "not one element");
    assert_eq!(
        at(2, "    glasses.sumOf { a.let { ", "it"),
        None,
        "an inner lambda"
    );
    assert_eq!(
        kotlin_element_type(text.lines().next().unwrap(), "glasses").as_deref(),
        Some("Glass")
    );
    assert_eq!(
        kotlin_element_type(text.lines().next().unwrap(), "n"),
        None,
        "a map"
    );
}

#[test]
fn kotlin_arity_counts_defaults_varargs_and_a_trailing_lambda() {
    let parts = |decl: &str| jvm_parameter_parts(decl, 1, "f").expect("a parameter list");
    let takes = |decl: &str, n: usize| jvm_takes(&parts(decl), n);
    assert_eq!(
        parts("fun f(a: Map<String, Int>, b: (Int, Int) -> Unit = { _, _ -> }) {"),
        ["a: Map<String, Int>", "b: (Int, Int) -> Unit = { _, _ -> }"]
    );
    assert!(takes("fun f(a: Int, b: Int = 1): Int", 1));
    assert!(!takes("fun f(a: Int, b: Int = 1): Int", 3));
    assert!(takes("fun f(a: Int, vararg b: Int)", 4));
    assert!(!takes("fun f(a: Int)", 2));
    let call = jvm_call(&["f(1, \"a,b\") { it }"], 0, 1).expect("a call");
    assert_eq!(call.arguments, ["1", "\"a,b\""]);
    assert!(call.trailing_lambda);
    assert_eq!(jvm_literal_parameter("cups: Int"), Some("int"));
    assert_eq!(jvm_literal_parameter("final String cups"), Some("string"));
    assert_eq!(jvm_literal_parameter("cups: Int = 1"), None);
    assert_eq!(jvm_literal_argument("\"two\""), Some("string"));
    assert_eq!(jvm_literal_argument("2L"), None);
}

#[test]
fn a_getter_chain_is_read_up_to_the_name_it_starts_from() {
    let chain = |before: &str| jvm_getter_chain(before).join(" ");
    assert_eq!(chain("x = user.getSpec()."), "user getSpec()");
    assert_eq!(
        chain("this.post.getSpec().isPinned()."),
        "this post getSpec() isPinned()"
    );
    assert_eq!(
        chain("user.spec."),
        "",
        "a chain of fields is the qualifier's"
    );
    assert_eq!(
        chain("getSpec()."),
        "",
        "a call on the own class is left alone"
    );
    assert_eq!(chain("user.find().getSpec()."), "");
    assert_eq!(chain("user.getSpec(id)."), "");
    assert_eq!(chain("user.setSpec()."), "");
    assert_eq!(chain("new User().getSpec()."), "");
    assert_eq!(chain("list.get(0).getSpec()."), "");
    assert_eq!(chain("Foo::getSpec()."), "");
}
