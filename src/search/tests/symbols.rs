//! The names `D` lists.

use super::*;

#[test]
fn code_symbol_names() {
    for (line, name) in [
        ("def render(inv: Invoice) -> str:", Some("render")),
        ("    def total(self) -> int:", Some("total")),
        ("pub fn wrap_line(s: &str) {", Some("wrap_line")),
        ("pub(crate) struct Hit {", Some("Hit")),
        ("export async function parse(x) {", Some("parse")),
        ("export default class Foo {", Some("Foo")),
        ("    return total", None),
        ("class Invoice:", Some("Invoice")),
        ("func (i Invoice) Total() int {", Some("Total")),
        ("func main() {", Some("main")),
        ("type Invoice struct{}", Some("Invoice")),
        ("impl<T> Order<T> {", Some("Order")),
        (
            "pub(crate) const MAX_ORDERS: usize = 10;",
            Some("MAX_ORDERS"),
        ),
        ("static COUNT: u32 = 0;", Some("COUNT")),
        // Indented and bare, `const` and `static` are locals; exported, they are symbols.
        ("  const n = 1;", None),
        ("    static COUNT: u32 = 0;", None),
        ("  export const LIMIT = 10;", Some("LIMIT")),
        ("    pub const MAX: u32 = 1;", Some("MAX")),
        ("const fn zero() -> u32 {", Some("zero")),
        ("extern \"C\" fn c_call() {", Some("c_call")),
        ("macro_rules! order {", Some("order")),
        ("mod orders;", Some("orders")),
        ("pub union Bits {", Some("Bits")),
        (
            "export declare function load(id: string): void;",
            Some("load"),
        ),
        ("function* ids() {", Some("ids")),
        ("export namespace Orders {", Some("Orders")),
        ("export const parse = (s) => s;", Some("parse")),
        ("    render(o);", None),
        ("    return inv.render()", None),
        // A Go constant with a type or with several names, and a JavaScript name with a `$`:
        // the name is whatever follows the keyword, with nothing required after it.
        ("const Limit int = 10", Some("Limit")),
        ("const A, B = 1, 2", Some("A")),
        (
            "export const user$ = new BehaviorSubject(null);",
            Some("user"),
        ),
        // A dotted name is the first part: the namespace, not what is nested in it.
        ("export namespace Validation.Rules {", Some("Validation")),
        // Java, Kotlin and Ruby have rows of their own; their keywords are not here, so a Go
        // struct field and a line of prose are not declarations.
        ("\tmodule module.Version", None),
        ("module in general is to provide", None),
        ("module.exports = helpers;", None),
    ] {
        assert_eq!(symbol(None, line).as_deref(), name, "{line}");
    }
}

#[test]
fn shell_symbol_names() {
    let sh = |line| symbol(Some(Kind::Shell), line);
    assert_eq!(sh("build() {").as_deref(), Some("build"));
    assert_eq!(sh("run_all ( ) {").as_deref(), Some("run_all"));
    assert_eq!(
        sh("  helper() {").as_deref(),
        Some("helper"),
        "nested, like the rest"
    );
    // The `function` forms are listed by the generic pattern instead, so each shows once.
    assert_eq!(sh("function deploy {"), None);
    assert_eq!(sh("function check() {"), None);
    assert_eq!(symbol(None, "function deploy {").as_deref(), Some("deploy"));
    assert_eq!(symbol(None, "function check() {").as_deref(), Some("check"));
    assert_eq!(symbol(None, "build() {"), None);
    for not_a_function in ["  build \"$ROOT\"", "x=$(build)", "start)", "ROOT=/srv"] {
        assert_eq!(sh(not_a_function), None, "{not_a_function}");
    }
}

#[test]
fn sql_symbol_names() {
    let sql = |line| symbol(Some(Kind::Sql), line);
    assert_eq!(
        sql("CREATE TABLE public.orders (").as_deref(),
        Some("public.orders"),
        "the schema stays part of the name"
    );
    assert_eq!(
        sql("CREATE OR REPLACE FUNCTION total(o int)").as_deref(),
        Some("total")
    );
    assert_eq!(
        sql("CREATE UNIQUE INDEX orders_id_idx ON orders (id);").as_deref(),
        Some("orders_id_idx")
    );
    assert_eq!(
        sql("create materialized view daily_totals as").as_deref(),
        Some("daily_totals")
    );
    assert_eq!(
        sql("CREATE TABLE IF NOT EXISTS billing.invoices (").as_deref(),
        Some("billing.invoices")
    );
    assert_eq!(
        sql(r#"CREATE TABLE "user" ("#).as_deref(),
        Some(r#""user""#)
    );
    assert_eq!(
        sql("CREATE TYPE mood AS ENUM ('ok');").as_deref(),
        Some("mood")
    );
    for not_a_definition in [
        "SELECT * FROM orders;",
        "ALTER TABLE orders ADD COLUMN x int;",
        "DROP TABLE orders;",
        "INSERT INTO orders VALUES (1);",
        "CREATE TABLESPACE fast LOCATION '/x';",
    ] {
        assert_eq!(sql(not_a_definition), None, "{not_a_definition}");
    }
    // A CTE is a query's own scaffolding, not a project symbol.
    assert_eq!(sql("WITH recent AS ("), None);
    // The all-language pattern must not list SQL lines a second time: its keywords are
    // matched case-sensitively at the start of the line.
    for line in [
        "CREATE TYPE mood AS ENUM ('ok');",
        "create type mood as enum ('ok');",
        "CREATE TABLE public.orders (",
    ] {
        assert_eq!(symbol(None, line), None, "{line}");
    }
}

#[test]
fn jvm_symbol_names() {
    let jvm = |line| one(Kind::Jvm, line);
    // What either language declares with a keyword, behind annotations and modifiers.
    for (line, name) in [
        ("public final class Invoice {", Some("Invoice")),
        ("interface Store {", Some("Store")),
        ("record Point(int x, int y) {}", Some("Point")),
        ("enum Status {", Some("Status")),
        ("public @interface Json {", Some("Json")),
        ("data class Order(val id: String)", Some("Order")),
        ("enum class Status {", Some("Status")),
        ("annotation class Json", Some("Json")),
        ("    suspend fun load(id: String): Order {", Some("load")),
        ("@Test fun parsesInvoice() {", Some("parsesInvoice")),
        ("external fun nativeInit()", Some("nativeInit")),
        ("object Registry {", Some("Registry")),
        ("typealias Rows = List<Order>", Some("Rows")),
        ("fun interface Handler {", Some("Handler")),
        // An extension function is listed under its own name, past the receiver.
        ("fun String.slug(): String = lowercase()", Some("slug")),
        ("fun <T> List<T>.second(): T = this[1]", Some("second")),
        (
            "fun String?.orEmpty(): String = this ?: \"\"",
            Some("orEmpty"),
        ),
        // A method: the return type before the name is what tells it from a call.
        ("    public int total() {", Some("total")),
        (
            "    static Map<String, Integer> compute(Map<String, Integer> rows) {",
            Some("compute"),
        ),
        (
            "    Map<String, List<Integer>> group(List<Integer> xs) {",
            Some("group"),
        ),
        ("    void save(Invoice inv);", Some("save")),
        (
            "    @Override public static <T> List<T> of(T one) {",
            Some("of"),
        ),
        // A constant behind `const`; a plain property or field is not a symbol.
        ("const val LIMIT = 10", Some("LIMIT")),
        ("    private const val TAG = \"Invoice\"", Some("TAG")),
        ("        const val MAX = 1", Some("MAX")),
        ("    val all = listOf<Order>()", None),
        ("    private static final int LIMIT = 10;", None),
        ("    static final Invoice EMPTY = new Invoice(0);", None),
        // `companion object` names nothing, and a call is not a declaration.
        ("    companion object {", None),
        ("    return compute(items);", None),
        ("    Map<String, Integer> rows = compute(items);", None),
        ("    System.out.println(x);", None),
        ("    } catch (IOException e) {", None),
        ("    if (parse(x)) {", None),
        ("    Card(title = \"Invoice\") {", None),
        ("        withContext(Dispatchers.IO) {", None),
        ("        return new Runnable() {", None),
        // The constructor is listed under its class.
        ("    public Invoice(int n) {", None),
    ] {
        assert_eq!(jvm(line).as_deref(), name, "{line}");
    }
}

#[test]
fn ruby_symbol_names() {
    let rb = |line| one(Kind::Ruby, line);
    for (line, name) in [
        ("module Billing", Some("Billing")),
        ("  class Invoice < Base", Some("Invoice")),
        ("  class Billing::Invoice < Base", Some("Invoice")),
        ("    def initialize(id)", Some("initialize")),
        ("    def self.parse(text)", Some("parse")),
        ("    def Invoice.build(text)", Some("build")),
        ("    def total=(value)", Some("total")),
        ("    def empty?", Some("empty")),
        // No keyword to go by: a constant, and one `attr_accessor` line can declare
        // several names at once.
        ("  LIMIT = 10", None),
        ("    attr_accessor :total", None),
        ("    @cache ||= {}", None),
        ("    class << self", None),
        ("    Invoice.new(1)", None),
        ("  end", None),
    ] {
        assert_eq!(rb(line).as_deref(), name, "{line}");
    }
}

#[test]
fn c_symbol_names() {
    let c = |line| one(Kind::C, line);
    for (line, name) in [
        // A function: in column zero, where C has no statements, anything but a prototype.
        (
            "int invoice_total(struct invoice *inv) {",
            Some("invoice_total"),
        ),
        ("static int compute(struct invoice *inv)", Some("compute")),
        (
            "void invoice_free(struct invoice *inv,",
            Some("invoice_free"),
        ),
        (
            "sds *sdssplitlen(const char *s, ssize_t len)",
            Some("sdssplitlen"),
        ),
        ("void Ledger::append(Rows rows) {", Some("append")),
        // GNU style: the return type is on the line above, so the name starts the line.
        (
            "edata_ind_get(const edata_t *edata) {",
            Some("edata_ind_get"),
        ),
        ("static unsigned", None),
        (
            "FMT_FUNC auto vformat(string_view f) -> std::string {",
            Some("vformat"),
        ),
        ("int invoice_total(struct invoice *inv);", None),
        // A method, indented, told from a call by the body it opens.
        (
            "  auto total() const -> int { return total_; }",
            Some("total"),
        ),
        ("  explicit Ledger(int n) : total_(n) {}", Some("Ledger")),
        (
            "  template <typename T> void write(T value) {",
            Some("write"),
        ),
        ("  void append(Rows rows);", None),
        ("    if (check(rows)) {", None),
        ("    log::write(rows);", None),
        ("    return compute(inv);", None),
        ("        fmt::format_to(out, \"{}\", 42);", None),
        ("template <typename Context = context, typename... T,", None),
        // A type, a namespace and an alias.
        ("struct invoice {", Some("invoice")),
        (
            "struct __attribute__ ((__packed__)) sdshdr8 {",
            Some("sdshdr8"),
        ),
        ("static struct config {", Some("config")),
        ("template <typename T> struct Box : Base {", Some("Box")),
        (
            "struct formatter<std::filesystem::path, Char> {",
            Some("formatter"),
        ),
        ("struct client;", None),
        ("union value {", Some("value")),
        ("enum class Status {", Some("Status")),
        ("namespace billing {", Some("billing")),
        ("class LEDGER_API Ledger : public Base {", Some("Ledger")),
        (
            "class basic_memory_buffer : public detail::buffer<T> {",
            Some("basic_memory_buffer"),
        ),
        ("using Rows = std::vector<int>;", Some("Rows")),
        ("using namespace detail;", None),
        // `using a::b;` imports a name; the row would otherwise be called `a`.
        ("using std::swap;", None),
        ("  using fmt::buffered_file;", None),
        ("struct invoice *current = NULL;", None),
        // The name a typedef gives a type, once: the opening `typedef struct invoice {` is
        // not listed, so the type is one row, under the name the project writes.
        ("typedef char *sds;", Some("sds")),
        ("typedef struct redisObject robj;", Some("robj")),
        ("} invoice;", Some("invoice")),
        ("typedef struct invoice {", None),
        // Indented, a closing brace ends a nested anonymous struct: that name is a field.
        ("    } offset;", None),
        ("} while (0);", None),
        ("};", None),
        // A macro, function-like or not.
        ("#define LRU_BITS 24", Some("LRU_BITS")),
        ("#  define FMT_THROW(x) throw x", Some("FMT_THROW")),
        ("#ifndef INVOICE_H", None),
    ] {
        assert_eq!(c(line).as_deref(), name, "{line}");
    }
}

#[test]
fn csharp_symbol_names() {
    let cs = |line| one(Kind::CSharp, line);
    for (line, name) in [
        // A type, past its attributes, its modifiers and the generics it declares.
        (
            "public sealed partial class Invoice<T> : Base, IEnumerable<T>",
            Some("Invoice"),
        ),
        ("internal readonly struct Tag", Some("Tag")),
        ("public interface IStore<T> where T : class", Some("IStore")),
        ("public enum Status", Some("Status")),
        ("public record Money(decimal Amount);", Some("Money")),
        ("public record struct Point(int X, int Y);", Some("Point")),
        (
            "public delegate int Comparison<T>(T a, T b);",
            Some("Comparison"),
        ),
        // A namespace under its last part, the one `d` finds it by.
        ("namespace Billing.Core;", Some("Core")),
        ("namespace Billing", Some("Billing")),
        // A member: the type before the name is what tells it from a call.
        (
            "    public async Task<Invoice<T>> LoadAsync(int id)",
            Some("LoadAsync"),
        ),
        ("    void Save(Invoice<int> inv);", Some("Save")),
        (
            "    [Fact] public void Handles_Empty() {",
            Some("Handles_Empty"),
        ),
        (
            "    int IComparable.CompareTo(object? other) => 0;",
            Some("CompareTo"),
        ),
        (
            "    private static Rows Compute(int id) => new Rows();",
            Some("Compute"),
        ),
        // A property, by its accessors, its expression body, or the brace on the next line.
        ("    public int Total { get; private set; }", Some("Total")),
        ("    public string Name => _name;", Some("Name")),
        ("    public IReadOnlyList<int> Rows", Some("Rows")),
        // A field is not a symbol, in this kind as in every other.
        ("    private const int Limit = 10;", None),
        ("    private readonly ILogger<Invoice<T>> _logger;", None),
        ("    public static event EventHandler? Saved;", None),
        (
            "    public static Dictionary<string, Invoice<int>> All = new();",
            None,
        ),
        // The constructor is listed under its class, and a `using` alias is file-local.
        ("    public Invoice(int n)", None),
        ("using Rows = System.Collections.Generic.List<int>;", None),
        ("using System.Text.Json;", None),
        // A call, a statement and a block header are not declarations.
        ("        var rows = Compute(id);", None),
        ("        if (Check(rows))", None),
        ("        Console.WriteLine(rows);", None),
        ("        services.AddSingleton<IFoo, Foo>();", None),
        ("        return new Invoice<T>(id);", None),
        ("        foreach (var row in rows)", None),
        ("        catch (InvalidOperationException ex)", None),
        ("        using (var scope = provider.CreateScope())", None),
        ("        await client.SendAsync(request);", None),
        ("    Open,", None),
    ] {
        assert_eq!(cs(line).as_deref(), name, "{line}");
    }
}

#[test]
fn swift_symbol_names() {
    let sw = |line| one(Kind::Swift, line);
    for (line, name) in [
        ("public final class Session: NSObject {", Some("Session")),
        (
            "@MainActor public struct Response<Value> {",
            Some("Response"),
        ),
        ("actor Cache {", Some("Cache")),
        ("public enum State {", Some("State")),
        (
            "public protocol RequestDelegate: AnyObject {",
            Some("RequestDelegate"),
        ),
        ("public typealias Rows = [Int]", Some("Rows")),
        ("    associatedtype Value", Some("Value")),
        // An extension is listed under the type it extends, which is what a project's own
        // members of that type sit in.
        ("extension Session: RequestDelegate {", Some("Session")),
        ("extension Array where Element: Hashable {", Some("Array")),
        // A function, past its generics; `class func` is a static method, not a class.
        (
            "    public func request<T: Encodable>(_ url: URL) -> Request {",
            Some("request"),
        ),
        ("    class func shared() -> Session {", Some("shared")),
        ("    mutating func append(_ row: Int) {", Some("append")),
        (
            "    @discardableResult func resume() -> Self {",
            Some("resume"),
        ),
        ("    func `default`() {", Some("default")),
        // What a type holds is not a symbol, in this kind as in every other, and an `init` is
        // listed under its type.
        ("    public static let `default` = Session()", None),
        ("    private let queue: DispatchQueue", None),
        ("    public var isRunning = false", None),
        ("    case initialized", None),
        ("    public init(queue: DispatchQueue = .main) {", None),
        // An operator has no name a reader would look it up by.
        (
            "    public static func == (lhs: Self, rhs: Self) -> Bool {",
            None,
        ),
        // A call, a binding and a pattern are not declarations.
        ("        let request = Request(url)", None),
        ("        queue.async {", None),
        ("        if let delegate = delegate {", None),
        ("        guard let url = url else { return }", None),
        ("        return Session()", None),
        ("        case let .failed(error):", None),
        ("        switch request.state {", None),
    ] {
        assert_eq!(sw(line).as_deref(), name, "{line}");
    }
}

#[test]
fn php_symbol_names() {
    let php = |line| one(Kind::Php, line);
    for (line, name) in [
        (
            "abstract class Invoice implements Arrayable",
            Some("Invoice"),
        ),
        ("#[Attribute] final class Money", Some("Money")),
        ("interface Arrayable", Some("Arrayable")),
        ("trait Macroable", Some("Macroable")),
        ("enum Status: string", Some("Status")),
        ("namespace App\\Services;", Some("Services")),
        (
            "function billing_total(Invoice $invoice): int",
            Some("billing_total"),
        ),
        (
            "    final public static function parse(string $text): static",
            Some("parse"),
        ),
        (
            "    abstract protected function compute(): int;",
            Some("compute"),
        ),
        ("    public function &rows(): array", Some("rows")),
        (
            "    public const STATUS_OPEN = 'open';",
            Some("STATUS_OPEN"),
        ),
        // A property is a field, an enum case is what a type holds, and a `define()` has no
        // keyword before the name: none of them is a symbol.
        ("    protected array $rows = [];", None),
        ("    private ?Logger $logger;", None),
        ("    case Open = 'open';", None),
        ("define('BILLING_LIMIT', 10);", None),
        // A magic method is the language's hook, not the project's, as in C.
        (
            "    public function __construct(private readonly Account $account)",
            None,
        ),
        ("    public function __toString(): string", None),
        // A `use` imports, an anonymous function has no name, and a call is not a
        // declaration.
        ("use Illuminate\\Support\\Str;", None),
        ("    use Macroable;", None),
        ("$handler = function ($x) use ($y) {", None),
        ("        return $this->rows;", None),
        ("        foreach ($this->rows as $key => $value) {", None),
        ("        $total = 0;", None),
    ] {
        assert_eq!(php(line).as_deref(), name, "{line}");
    }
}

#[test]
fn the_shared_pattern_skips_the_kinds_with_rows_of_their_own() {
    // Java, Kotlin, Ruby, C, C++, Lua and Elixir are listed from their own rows only, so
    // nothing is listed twice, `def self.parse` is not `self` and `function M.setup(` is
    // not `M`.
    assert!(!shared_symbols(Some(Kind::Jvm)));
    assert!(!shared_symbols(Some(Kind::Ruby)));
    assert!(!shared_symbols(Some(Kind::C)));
    assert!(!shared_symbols(Some(Kind::CSharp)));
    assert!(!shared_symbols(Some(Kind::Swift)));
    assert!(!shared_symbols(Some(Kind::Php)));
    assert!(!shared_symbols(Some(Kind::Lua)));
    assert!(!shared_symbols(Some(Kind::Elixir)));
    // Shell and SQL rows complement the shared pattern instead, and it reads every other
    // file, known kind or not.
    assert!(shared_symbols(Some(Kind::Shell)));
    assert!(shared_symbols(Some(Kind::Sql)));
    assert!(shared_symbols(Some(Kind::Python)));
    assert!(shared_symbols(None));
}

#[test]
fn infra_symbol_names() {
    let make = |line| symbol(Some(Kind::Make), line);
    assert_eq!(make("build test: deps $(SRC)").as_deref(), Some("build"));
    assert_eq!(make("deps::").as_deref(), Some("deps"));
    assert_eq!(make("build-release:").as_deref(), Some("build-release"));
    for not_a_target in [
        ".PHONY: build",
        "%.o: %.c",
        "CC := gcc",
        "CC ::= gcc",
        "CC ?= gcc",
        "\t@echo a: b",
        "ifeq ($(OS),Windows_NT)",
        "$(OBJ): x",
    ] {
        assert_eq!(make(not_a_target), None, "{not_a_target}");
    }

    let tf = |line| symbol(Some(Kind::Terraform), line);
    assert_eq!(
        tf(r#"resource "aws_s3_bucket" "logs" {"#).as_deref(),
        Some("aws_s3_bucket.logs")
    );
    assert_eq!(
        tf(r#"data "aws_ami" "ubuntu" {"#).as_deref(),
        Some("data.aws_ami.ubuntu")
    );
    assert_eq!(tf(r#"variable "region" {"#).as_deref(), Some("var.region"));
    assert_eq!(tf(r#"module "vpc" {"#).as_deref(), Some("module.vpc"));
    assert_eq!(tf(r#"output "url" {"#).as_deref(), Some("output.url"));
    assert_eq!(tf(r#"  dynamic "ingress" {"#), None);

    let docker = |line| symbol(Some(Kind::Docker), line);
    assert_eq!(docker("FROM rust:1.80 AS build").as_deref(), Some("build"));
    assert_eq!(
        docker("from --platform=$P debian as runtime").as_deref(),
        Some("runtime")
    );
    assert_eq!(docker("FROM debian"), None);

    let yaml = |line| symbol(Some(Kind::Yaml), line);
    assert_eq!(yaml("x-common: &common").as_deref(), Some("&common"));
    assert_eq!(yaml("  <<: *common"), None);
    assert_eq!(yaml("apiVersion: v1"), None);
}
