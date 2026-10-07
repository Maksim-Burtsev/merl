use super::*;

const C_H: &str = r#"#ifndef INVOICE_H
#define INVOICE_H

#define LRU_BITS 24
#define serverLog(level, ...) do { emit(level); } while (0)

typedef char *sds;
typedef int (*compare_fn)(const void *a, const void *b);

struct client;

struct __attribute__ ((__packed__)) sdshdr8 {
    uint8_t len;
};

static struct config {
    int port;
} server_config;

typedef struct invoice {
    sds name;
    int total;
    struct {
        int index;
    } offset;
} invoice;

typedef enum {
    STATE_NONE = 0,
    STATE_OPEN,
} state;

union value {
    int n;
    sds s;
};

extern struct invoice *current;

int invoice_total(struct invoice *inv);

#endif
"#;

const C_C: &str = r#"#include "invoice.h"

struct invoice *current = NULL;
static int counter;

int invoice_total(struct invoice *inv) {
    if (invoice_valid(inv)) {
        return compute(inv);
    }
    return counter;
}

static int compute(struct invoice *inv)
{
    struct invoice *copy = inv;
    return copy->total;
}

static unsigned
invoice_index(const struct invoice *inv)
{
    return 0;
}

void invoice_free(struct invoice *inv,
                  int deep)
{
    free(inv);
}

static const int invoice_states[] = {
    STATE_OPEN,
    STATE_NONE
};
"#;

const CPP: &str = r#"#include "invoice.h"

namespace billing {

using Rows = std::vector<int>;
using std::swap;

template <typename T> struct Box : Base {
  T value;
};

template <typename T> struct Box<T *> : Base {
};

template <typename T = int>
class LEDGER_API Ledger : public Base {
 public:
  explicit Ledger(int n) : total_(n) {}

  auto total() const -> int { return total_; }

  void append(Rows rows);

  using Row = int;

 private:
  int total_;
};

void Ledger::append(Rows rows) {
  if (check(rows)) {
    log::write(rows);
  }
}

enum class Status {
  Open,
};

void scan(int k) {
  Slice key(k, 2);
}

}  // namespace billing
"#;

#[test]
fn c_def_patterns_find_types_macros_functions_and_globals() {
    let (dir, files) = scratch("c-h", &[("invoice.h", C_H)]);
    let d = |w| defs(&dir, &files, Kind::C, w);
    assert_eq!(d("LRU_BITS"), [4]);
    assert_eq!(d("serverLog"), [5], "a function-like macro");
    assert_eq!(d("sds"), [7], "not the `sds name;` field it types");
    assert_eq!(d("compare_fn"), [8], "a function pointer");
    assert_eq!(d("client"), [10], "a forward declaration");
    assert_eq!(d("sdshdr8"), [12], "behind a lower-case attribute");
    assert_eq!(d("config"), [16], "behind a storage specifier");
    assert_eq!(d("server_config"), [18], "the name the block closes with");
    assert_eq!(
        d("invoice"),
        [20, 26],
        "the `typedef struct invoice {{` and the `}} invoice;` it closes with"
    );
    assert_eq!(d("state"), [31]);
    assert_eq!(d("value"), [33]);
    assert_eq!(d("current"), [38]);
    assert_eq!(d("invoice_total"), [40], "the prototype");
    assert_eq!(d("total"), Vec::<usize>::new(), "a field has no rule");
    assert_eq!(
        d("offset"),
        Vec::<usize>::new(),
        "an indented closing brace ends a nested anonymous struct: a field, not a type"
    );
    assert_eq!(
        d("STATE_OPEN"),
        [30],
        "an enum constant, in its enum's body (#373)"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn c_def_patterns_tell_a_definition_from_a_call() {
    let (dir, files) = scratch("c-c", &[("invoice.c", C_C)]);
    let d = |w| defs(&dir, &files, Kind::C, w);
    assert_eq!(d("current"), [3]);
    assert_eq!(d("counter"), [4], "a global, not the `return counter;`");
    assert_eq!(d("invoice_total"), [6]);
    assert_eq!(d("compute"), [13], "the brace opens on the next line");
    assert_eq!(
        d("invoice_index"),
        [20],
        "the return type is on the line above"
    );
    assert_eq!(d("invoice_free"), [25], "the parameters wrap");
    assert_eq!(
        d("invoice_valid"),
        Vec::<usize>::new(),
        "a call inside `if`: indented, only a body opening on the line declares"
    );
    assert_eq!(d("free"), Vec::<usize>::new(), "a call statement");
    assert_eq!(d("copy"), Vec::<usize>::new(), "a local");
    assert_eq!(
        d("STATE_NONE"),
        Vec::<usize>::new(),
        "`NAME,` in an initializer list declares nothing (#373)"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cpp_def_patterns_cover_classes_methods_and_aliases() {
    let (dir, files) = scratch("cpp", &[("ledger.cc", CPP)]);
    let d = |w| defs(&dir, &files, Kind::C, w);
    assert_eq!(d("billing"), [3], "not the closing comment");
    assert_eq!(d("Rows"), [5]);
    assert_eq!(d("Box"), [8, 12], "the template and its specialization");
    assert_eq!(
        d("Ledger"),
        [16, 18],
        "past the template head; and its constructor"
    );
    assert_eq!(d("total"), [20], "a method defined in the class body");
    assert_eq!(d("Row"), [24], "a `using` alias indented in a class body");
    assert_eq!(
        d("append"),
        [22, 30],
        "the declaration in the class body and the out-of-line definition (#373)"
    );
    assert_eq!(d("Status"), [36]);
    assert_eq!(
        d("T"),
        Vec::<usize>::new(),
        "a template parameter is no global: `template <typename T = int>` declares nothing"
    );
    assert_eq!(
        d("swap"),
        Vec::<usize>::new(),
        "`using std::swap;` imports a name"
    );
    assert_eq!(d("check"), Vec::<usize>::new(), "a call inside `if`");
    assert_eq!(d("write"), Vec::<usize>::new(), "a qualified call");
    assert_eq!(d("Open"), [37], "an enum constant (#373)");
    assert_eq!(
        d("key"),
        Vec::<usize>::new(),
        "`Slice key(k, 2);` in a function body is a local object, no member (#373)"
    );
    assert_eq!(d("total_"), Vec::<usize>::new(), "a field");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn c_and_cpp_scope_roots_and_names() {
    let here = Path::new("ledger.hpp");
    assert!(in_def_scope(Kind::C, here, Path::new("src/invoice.c")));
    assert!(
        in_def_scope(Kind::C, here, Path::new("ledger.cc")),
        "one kind: a `.cc` is searched for a definition asked for in a `.h`"
    );
    assert!(!in_def_scope(Kind::C, here, Path::new("main.rs")));
    assert!(imports(Kind::C, C_C).is_empty(), "`#include` binds no name");
    assert!(
        member_patterns(Kind::C, "total").is_none(),
        "a member of a value has no rule of its own"
    );
    let roots = external_roots(Kind::C, Path::new("/"));
    assert!(
        roots.iter().any(|r| r.ends_with("usr/include")),
        "the SDK's on a Mac, `/usr/include` on Linux, both on CI: no system include directory \
         among {roots:?}"
    );
    assert_eq!(
        qualified(Kind::C, CPP, 20, "total").as_deref(),
        Some("Ledger::total"),
        "a member behind `::`, as Rust writes it; an access specifier is a label inside the \
         class, not a wall in front of it"
    );
    assert_eq!(
        qualified(Kind::C, CPP, 3, "billing"),
        None,
        "the enclosing `namespace billing {{` is not indented: the walk ends at the first \
         column-zero declaration"
    );
    assert_eq!(
        qualified(Kind::C, CPP, 30, "append").as_deref(),
        Some("Ledger::append"),
        "an out-of-line body is its class's by the qualifier on its own line (#508)"
    );
}

#[test]
fn c_and_cpp_files_find_each_other() {
    let (dir, files) = scratch(
        "c-family",
        &[("invoice.h", C_H), ("invoice.c", C_C), ("ledger.cc", CPP)],
    );
    let pat = def_patterns(Kind::C, "invoice_total").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("invoice.c".into(), 6), ("invoice.h".into(), 40)],
        "the header's prototype and the definition are both offered, in the picker's order by \
         path, as everywhere: it is not definition before prototype"
    );
    let pat = def_patterns(Kind::C, "sds").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("invoice.h".into(), 7)],
        "a type of the C header, reached from the C++ file that includes it"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const CS: &str = r#"namespace Billing.Core;

using Rows = System.Collections.Generic.List<int>;

[Serializable]
public sealed partial class Invoice<T> : Base, IEnumerable<T>
{
    private const int Limit = 10;
    private readonly ILogger<Invoice<T>> _logger;
    public static event EventHandler? Saved;

    public Invoice(int n)
    {
        _logger = Create(n);
    }

    public int Total { get; private set; }

    public string Name => _name;

    [HttpGet("{id}")]
    public async Task<Invoice<T>> LoadAsync(int id)
    {
        var rows = Compute(id);
        if (Check(rows))
        {
            Console.WriteLine(rows);
        }
        return new Invoice<T>(id);
    }

    int IComparable.CompareTo(object? other) => 0;

    private static Rows Compute(int id) => new Rows();
}

public interface IStore
{
    void Save(Invoice<int> inv);
}

public record struct Point(int X, int Y);

public record Money(decimal Amount);

public enum Status
{
    Open,
}

public delegate int Comparison<T>(T a, T b);

public static class Registry
{
    public static Dictionary<string, Invoice<int>> All = new();
}
"#;

#[test]
fn csharp_def_patterns_find_types_members_and_fields() {
    let (dir, files) = scratch("cs", &[("Invoice.cs", CS)]);
    let d = |w| defs(&dir, &files, Kind::CSharp, w);
    assert_eq!(d("Core"), [1], "a file-scoped namespace, by its last part");
    assert_eq!(
        d("Rows"),
        [3],
        "a `using` alias, not the `Rows` it is used as"
    );
    assert_eq!(
        d("Invoice"),
        [6, 12],
        "the class past its generic parameters and its attribute, and the constructor; the caller shows a picker"
    );
    assert_eq!(d("Limit"), [8]);
    assert_eq!(d("_logger"), [9], "not the `_logger = Create(n);` write");
    assert_eq!(d("Saved"), [10], "an event");
    assert_eq!(d("Total"), [17], "a property, by its accessor block");
    assert_eq!(d("Name"), [19], "an expression-bodied property");
    assert_eq!(
        d("LoadAsync"),
        [22],
        "behind an attribute and `public async`"
    );
    assert_eq!(d("rows"), [24], "a local");
    assert_eq!(d("Compute"), [34], "not the `Compute(id)` call above it");
    assert_eq!(d("CompareTo"), [32], "an explicit interface implementation");
    assert_eq!(d("IStore"), [37]);
    assert_eq!(d("Save"), [39], "an interface method has no modifiers");
    assert_eq!(d("Point"), [42], "a positional `record struct`");
    assert_eq!(d("Money"), [44]);
    assert_eq!(d("Status"), [46]);
    assert_eq!(d("Comparison"), [51], "a delegate, past its return type");
    assert_eq!(d("Registry"), [53]);
    assert_eq!(d("All"), [55]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn csharp_def_patterns_tell_a_declaration_from_a_call() {
    let (dir, files) = scratch("cs-calls", &[("Invoice.cs", CS)]);
    let d = |w| defs(&dir, &files, Kind::CSharp, w);
    let none = Vec::<usize>::new();
    assert_eq!(d("Check"), none, "a call inside `if`");
    assert_eq!(d("Console"), none);
    assert_eq!(d("WriteLine"), none, "a call statement");
    assert_eq!(d("Create"), none, "a call on the right of an assignment");
    assert_eq!(d("Base"), none, "a base list is a use of the type");
    assert_eq!(d("IEnumerable"), none);
    assert_eq!(d("id"), none, "a parameter has no rule");
    assert_eq!(
        d("Open"),
        none,
        "an enum member has no rule: `Open,` is also a line of a collection initialiser"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn csharp_scope_stays_in_the_project() {
    let here = Path::new("Invoice.cs");
    assert!(in_def_scope(Kind::CSharp, here, Path::new("src/Store.csx")));
    assert!(!in_def_scope(Kind::CSharp, here, Path::new("main.rs")));
    assert!(
        imports(Kind::CSharp, CS).is_empty(),
        "a `using` opens a whole namespace: it binds no name of its own"
    );
    assert!(
        external_roots(Kind::CSharp, Path::new("/")).is_empty(),
        "a NuGet package ships assemblies, not source"
    );
    assert!(member_patterns(Kind::CSharp, "Total").is_none());
    assert_eq!(
        qualified(Kind::CSharp, CS, 22, "LoadAsync").as_deref(),
        Some("Invoice.LoadAsync"),
        "a member is named by the type it is declared in"
    );
}

#[test]
fn csharp_bindings_read_headers_not_calls_or_fields() {
    let text = "\
public class Cart
{
    Item item = new Item();

    public void Fill(Item seed)
    {
        var order = new Order(seed)
        {
            Name = seed.Name,
        };
        order.Ship();
        var (left, right) = Split(order);
        Use(item, left);
    }
}
";
    let lines = |name: &str, line: usize| -> Vec<usize> {
        bindings(Kind::CSharp, text, line, name)
            .iter()
            .map(|b| b.line)
            .collect()
    };
    assert_eq!(
        lines("seed", 9),
        [5],
        "the object initialiser's `new Order(seed)` over `{{` is no signature: `seed` is Fill's"
    );
    assert_eq!(lines("order", 11), [7]);
    assert!(
        lines("left", 13).is_empty(),
        "a deconstruction binds nothing"
    );
    assert!(
        lines("item", 13).is_empty(),
        "a field of the class is no local"
    );
    assert!(
        lines("item", 3).is_empty(),
        "on a member's own line the class body is not read as statements either"
    );
}

const SWIFT: &str = r#"import Foundation

public protocol RequestDelegate: AnyObject {
    associatedtype Value

    func didFinish(_ request: Request)
}

@objc(AFSession)
public final class Session: NSObject {
    public static let `default` = Session()

    private let queue: DispatchQueue
    public var isRunning = false

    public init(queue: DispatchQueue = .main) {
        self.queue = queue
    }

    convenience init?(name: String) {
        self.init()
    }

    public func request<T: Encodable>(_ url: URL, with body: T) -> Request {
        let request = Request(url)
        queue.async {
            self.start(request)
        }
        if let delegate = delegate {
            delegate.didFinish(request)
        }
        return request
    }

    class func shared() -> Session {
        return Session()
    }
}

extension Session: RequestDelegate {
    public func didFinish(_ request: Request) {
        switch request.state {
        case .finished:
            break
        case let .failed(error):
            print(error)
        }
    }
}

public enum State {
    case initialized
    case resumed(Int), suspended
    case failed(Error)
}

public struct Response<Value> {
    let value: Value
}

actor Cache {
    var entries: [String: Data] = [:]
}

public typealias Rows = [Int]

public final class Store {
    public private(set) weak var owner: Session?
}

let opened = 0

func describe(_ code: Int) -> String {
    switch code {
    case opened:
        return "opened"
    default:
        return ""
    }
}
"#;

#[test]
fn swift_def_patterns_find_declarations_behind_attributes_and_modifiers() {
    let (dir, files) = scratch("swift", &[("Session.swift", SWIFT)]);
    let d = |w| defs(&dir, &files, Kind::Swift, w);
    assert_eq!(d("RequestDelegate"), [3], "a protocol");
    assert_eq!(d("Value"), [4], "an `associatedtype`");
    assert_eq!(
        d("Session"),
        [10, 40],
        "the class and the extension of it, not the `Session()` calls"
    );
    assert_eq!(d("didFinish"), [6, 41], "not the `delegate.didFinish` call");
    assert_eq!(d("default"), [11], "a backticked name");
    assert_eq!(d("queue"), [13], "not the `self.queue = queue` write");
    assert_eq!(d("isRunning"), [14]);
    assert_eq!(
        d("init"),
        [16, 20],
        "`init?` too, not the `self.init()` call"
    );
    assert_eq!(d("request"), [24, 25], "the function and the local");
    assert_eq!(d("shared"), [35], "behind `class`, Swift's static method");
    assert_eq!(d("initialized"), [52], "an enum case");
    assert_eq!(d("resumed"), [53], "with its associated value");
    assert_eq!(d("suspended"), [53], "second on the line");
    assert_eq!(
        d("failed"),
        [54],
        "not the `case let .failed(error):` pattern"
    );
    assert_eq!(d("State"), [51]);
    assert_eq!(d("Response"), [57], "past the generic parameters");
    assert_eq!(d("Cache"), [61], "an actor");
    assert_eq!(d("entries"), [62]);
    assert_eq!(d("Rows"), [65], "a `typealias`");
    assert_eq!(d("Store"), [67]);
    assert_eq!(d("owner"), [68], "behind `public private(set) weak`");
    assert_eq!(
        d("opened"),
        [71],
        "the `let`, not the `case opened:` of the `switch` below it, which matches against that constant: a bare name there is a pattern, not a declaration"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn swift_locals_are_a_functions_and_members_a_types() {
    let text = r#"let global = 1
final class Box {
#if DEBUG
    let debug = 1
#endif
    var size: Int {
        let side = 2
        return side
    }
    func open(
        _ lid: Int
    ) {
        items.forEach { item in
            var seen = item
        }
    }
}
protocol Boxed {
    var lid: Int { get }
}"#;
    let lines: Vec<&str> = text.lines().collect();
    let literal = literal_lines(Kind::Swift, text);
    let local = |line| swift_local(&lines, &literal, line);
    assert_eq!(local(1), None, "a global");
    assert_eq!(local(4), None, "a member under an `#if`");
    assert_eq!(local(7), Some(6), "a computed property's local");
    assert_eq!(
        local(14),
        Some(10),
        "a closure's, of the wrapped `func` around it"
    );
    assert_eq!(local(19), None, "a protocol's requirement");
    assert_eq!(swift_scope(&lines, &literal, 13).0, 10);
    assert!(swift_extension("public extension Box where T: Equatable {"));
    assert!(!swift_extension("let extensionCount = 1"));
}

#[test]
fn swift_def_patterns_tell_a_declaration_from_a_call_or_a_pattern() {
    let (dir, files) = scratch("swift-calls", &[("Session.swift", SWIFT)]);
    let d = |w| defs(&dir, &files, Kind::Swift, w);
    let none = Vec::<usize>::new();
    assert_eq!(d("finished"), none, "`case .finished:` is a pattern");
    assert_eq!(d("start"), none, "a call on `self`");
    assert_eq!(d("print"), none);
    assert_eq!(d("Request"), none, "a type this file only uses");
    assert_eq!(
        d("delegate"),
        none,
        "an `if let` rebinds a name declared elsewhere: no rule, so `u` answers"
    );
    assert_eq!(d("body"), none, "a parameter has no rule");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn swift_scope_roots_and_names() {
    let here = Path::new("Session.swift");
    assert!(in_def_scope(
        Kind::Swift,
        here,
        Path::new("Source/Request.swift")
    ));
    assert!(!in_def_scope(Kind::Swift, here, Path::new("main.rs")));
    assert!(
        imports(Kind::Swift, SWIFT).is_empty(),
        "an `import` makes a whole module visible unqualified: it binds no name of its own"
    );
    let (dir, _) = scratch(
        "swift-roots",
        &[(".build/checkouts/nio/Sources/a.swift", "")],
    );
    assert_eq!(
        external_roots(Kind::Swift, &dir),
        [dir.join(".build/checkouts")],
        "where SwiftPM checks the dependencies out"
    );
    assert!(
        external_roots(Kind::Swift, Path::new("/")).is_empty(),
        "nothing else on the machine holds Swift source"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        qualified(Kind::Swift, SWIFT, 41, "didFinish").as_deref(),
        Some("Session.didFinish"),
        "a member is named by the type its extension extends"
    );
}

const PHP: &str = r#"<?php

namespace App\Services;

use Illuminate\Support\Str;
use App\Models\User as Account;

define('BILLING_LIMIT', 10);

abstract class Invoice implements Arrayable
{
    public const STATUS_OPEN = 'open';

    protected array $rows = [];

    private ?Logger $logger;

    public function __construct(private readonly Account $account, string $name)
    {
        $this->logger = null;
        $total = 0;
        foreach ($this->rows as $key => $value) {
            $total += $value;
        }
        $this->name = $name;
    }

    final public static function parse(string $text): static
    {
        return new static($text);
    }

    public function &rows(): array
    {
        return $this->rows;
    }

    abstract protected function compute(): int;
}

interface Arrayable
{
    public function toArray(): array;
}

trait Macroable
{
    public function macro(string $name): void
    {
    }
}

enum Status: string
{
    case Open = 'open';
    case Closed;
}

function billing_total(Invoice $invoice): int
{
    $sum = 0;
    return $sum;
}

function billing_report(array $rows, int $total): array
{
    $map = [
        $key => $value,
    ];

    return
        $total == 0 ? $map : $rows;
}
"#;

#[test]
fn php_def_patterns_find_declarations_behind_modifiers() {
    let (dir, files) = scratch("php", &[("Invoice.php", PHP)]);
    let d = |w| defs(&dir, &files, Kind::Php, w);
    assert_eq!(d("Services"), [3], "a namespace, by its last part");
    assert_eq!(d("BILLING_LIMIT"), [8], "a `define()` constant");
    assert_eq!(d("Invoice"), [10], "not the `Invoice $invoice` parameter");
    assert_eq!(d("STATUS_OPEN"), [12], "a class constant");
    assert_eq!(
        d("rows"),
        [14, 33],
        "the property and the method that returns it, not the `$this->rows` uses"
    );
    assert_eq!(d("logger"), [16], "not the `$this->logger = null;` write");
    assert_eq!(d("account"), [18], "a promoted constructor parameter");
    assert_eq!(
        d("total"),
        [21, 23],
        "the assignment and the `+=` that follows"
    );
    assert_eq!(d("parse"), [28], "behind `final public static`");
    assert_eq!(d("compute"), [38], "an abstract method has no body");
    assert_eq!(d("Arrayable"), [41], "not the `implements Arrayable`");
    assert_eq!(d("toArray"), [43]);
    assert_eq!(d("Macroable"), [46], "a trait");
    assert_eq!(d("macro"), [48]);
    assert_eq!(d("Status"), [53], "a backed enum");
    assert_eq!(d("Open"), [55], "an enum case with its value");
    assert_eq!(d("Closed"), [56]);
    assert_eq!(d("billing_total"), [59], "a function at the top level");
    assert_eq!(d("sum"), [61], "not the `return $sum;`");
    assert_eq!(d("map"), [67]);
    assert_eq!(
        d("total"),
        [21, 23],
        "not the `$total == 0` that opens line 72: `==` compares, it declares nothing"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn php_def_patterns_tell_a_declaration_from_a_use() {
    let (dir, files) = scratch("php-uses", &[("Invoice.php", PHP)]);
    let d = |w| defs(&dir, &files, Kind::Php, w);
    let none = Vec::<usize>::new();
    assert_eq!(d("Str"), none, "a `use` imports a name, it declares none");
    assert_eq!(d("Account"), none, "nor does the alias of one");
    assert_eq!(d("Logger"), none, "a type a property is written with");
    assert_eq!(d("text"), none, "a parameter has no rule");
    assert_eq!(
        d("name"),
        none,
        "`$this->name = $name;` writes to a property declared elsewhere"
    );
    assert_eq!(
        d("key"),
        none,
        "a `foreach` target has no rule, and `$key => $value,` is an array pair"
    );
    assert_eq!(d("value"), none);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Typed class constants, the tags of a class's docblock and namespace segments (#344).
const PHP_TAGS: &str = r#"<?php

namespace App\Models;

/**
 * @property string $title
 * @property-read int $plays
 * @property-write array $tags
 * @method static SongBuilder whereTitle(string $title)
 * @method int length()
 */
#[Table]
class Song
{
    const int LIMIT = 500;
    private const ?string LABEL = null;
    public const A|B UNION = 1, SECOND = 2;
    protected const \Foo\Bar QUALIFIED = 3;
    const (A&B)|null DNF = 4;
    const PLAIN = 5;

    /**
     * @param int $count
     * @property int $inner
     * @method void helper()
     */
    public function first(int $count): string
    {
    }
}

/*
 * @property int $loose
 */

/**
 * @property string $email
 */
#[Guarded([
    'id',
])]
#[Table]
final class User
{
}
"#;

#[test]
fn php_typed_constants_and_class_docblock_tags_declare() {
    let (dir, files) = scratch("php-tags", &[("Song.php", PHP_TAGS)]);
    let d = |w| defs(&dir, &files, Kind::Php, w);
    assert_eq!(d("LIMIT"), [15], "`const int`");
    assert_eq!(d("LABEL"), [16], "a nullable type");
    assert_eq!(d("UNION"), [17], "a union");
    assert_eq!(
        d("SECOND"),
        Vec::<usize>::new(),
        "the second name stays unread"
    );
    assert_eq!(d("QUALIFIED"), [18]);
    assert_eq!(d("DNF"), [19], "a disjunctive normal form type");
    assert_eq!(d("PLAIN"), [20], "untyped, as before");
    assert_eq!(
        d("title"),
        [6],
        "not the `$title` parameter of the `@method` tag"
    );
    assert_eq!(d("plays"), [7]);
    assert_eq!(d("tags"), [8]);
    assert_eq!(d("whereTitle"), [9], "behind `static` and a return type");
    assert_eq!(d("length"), [10]);
    assert_eq!(d("count"), Vec::<usize>::new(), "`@param` declares nothing");
    let lines: Vec<&str> = PHP_TAGS.lines().collect();
    for at in [5, 6, 7, 8, 9] {
        assert_eq!(php_tag_class(&lines, at), Some(12), "line {}", at + 1);
    }
    for at in [23, 24, 33] {
        assert_eq!(php_tag_class(&lines, at), None, "line {}", at + 1);
    }
    assert_eq!(
        php_tag_class(&lines, 36),
        Some(42),
        "past an attribute wrapped over lines"
    );
    assert_eq!(
        qualified(Kind::Php, PHP_TAGS, 9, "whereTitle").as_deref(),
        Some("Song::whereTitle")
    );
    assert_eq!(
        qualified(Kind::Php, PHP_TAGS, 6, "title").as_deref(),
        Some("Song::title")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn php_namespace_line_answers_only_the_namespace_written_up_to_the_word() {
    let text = "<?php\nnamespace App\\Repos;\n";
    // The patterns for the word of `line`, and which of `decls` they match.
    let answers = |line: &str, word: &str| {
        let start = line.find(word).unwrap();
        let mut patterns = def_patterns(Kind::Php, word);
        php_namespace_patterns(&mut patterns, text, line, start..start + word.len());
        let re = regex::Regex::new(&patterns.join("|")).unwrap();
        let decls = [
            "namespace Illuminate\\Support;",
            "namespace App\\Repos\\Support;",
            "namespace App\\Services\\Auth\\Support;",
            "class Support",
        ];
        let hit: Vec<&str> = decls.into_iter().filter(|d| re.is_match(d)).collect();
        (patterns.len(), hit)
    };
    assert_eq!(
        answers("use Illuminate\\Support\\Facades\\Route;", "Support"),
        (1, vec!["namespace Illuminate\\Support;"]),
        "a segment: only the namespace written up to it, absolute on a `use` line, and no class"
    );
    assert_eq!(
        answers("    Support\\Str::of();", "Support").1,
        ["namespace App\\Repos\\Support;"],
        "elsewhere, relative to the file's own namespace"
    );
    assert_eq!(
        answers("    \\Illuminate\\Support\\Str::of();", "Support").1,
        ["namespace Illuminate\\Support;"],
        "unless it starts with `\\`"
    );
    for line in [
        "use App\\Support;",
        "    public function first(Support $s): string",
        "        Support::of('x');",
        "        new Support();",
    ] {
        assert_eq!(
            answers(line, "Support").1,
            ["class Support"],
            "{line}: the last part of a `use`, a type hint, a class before `::` or after `new` \
             is no namespace"
        );
    }
    assert_eq!(
        answers("namespace App\\Repos\\Support;", "Support").1.len(),
        4,
        "on a `namespace` line's last part, the other files of the namespace are its namesakes"
    );
}

#[test]
fn php_scope_roots_imports_and_names() {
    let here = Path::new("src/Invoice.php");
    assert!(in_def_scope(Kind::Php, here, Path::new("views/show.phtml")));
    assert!(!in_def_scope(Kind::Php, here, Path::new("main.rs")));
    assert_eq!(
        imports(Kind::Php, PHP),
        [
            (
                "Str".to_owned(),
                vec!["Illuminate".into(), "Support".into(), "Str".into()]
            ),
            (
                "Account".to_owned(),
                vec!["App".into(), "Models".into(), "User".into()]
            ),
        ],
        "a `use` in column zero binds the last part of the path, or its alias; PSR-4 spells the \
         namespace as the file system does, so the path is the name split on `\\`"
    );
    assert!(
        imports(Kind::Php, "class X {\n    use Macroable;\n}\n").is_empty(),
        "an indented `use` pulls a trait into a class body and names no file"
    );
    let (dir, _) = scratch("php-roots", &[("vendor/laravel/framework/src/a.php", "")]);
    assert_eq!(
        external_roots(Kind::Php, &dir),
        [dir.join("vendor")],
        "Composer's `vendor/` is gitignored, outside the project walk as `node_modules` is"
    );
    assert!(external_roots(Kind::Php, Path::new("/")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        qualified(Kind::Php, PHP, 28, "parse").as_deref(),
        Some("Invoice::parse"),
        "a method of a class behind `::`, as PHP's own documentation writes it"
    );
}

#[test]
fn a_cpp_alias_of_a_qualified_type_is_no_member_of_it() {
    let text = "typedef ns::Foo Foo;\nns::Foo Foo(1);\nRefund::Refund(const Refund& other) {}\n";
    assert_eq!(qualified(Kind::C, text, 1, "Foo"), None);
    assert_eq!(
        qualified(Kind::C, text, 2, "Foo"),
        None,
        "a qualified name before a `(` is a type in front of the name its line declares"
    );
    assert_eq!(
        qualified(Kind::C, text, 3, "Refund").as_deref(),
        Some("Refund::Refund"),
        "an out-of-line constructor still reads its class"
    );
}

#[test]
fn swift_receiver_types_are_read_off_their_lines() {
    use SwiftGiven::*;
    let given = |l: &str, n: &str| swift_given(l, n);
    assert_eq!(
        given(
            "    static func - (lhs: Instant, rhs: Instant) -> Double {",
            "rhs"
        ),
        Some(Type("Instant".into()))
    );
    assert_eq!(
        given("    let encoder: FormEncoder = FormEncoder()", "encoder"),
        Some(Type("FormEncoder".into()))
    );
    assert_eq!(
        given("        let printer = FormEncoder()", "printer"),
        Some(Value("FormEncoder()".into()))
    );
    assert_eq!(
        given("    guard let s = self.session else {", "s"),
        Some(Value("self.session".into()))
    );
    assert_eq!(
        given("        for p in printers where p.ok {", "p"),
        Some(Element("printers".into()))
    );
    assert_eq!(given("    case let .bytes(count):", "count"), None);
    assert_eq!(given("        items.map { item in", "item"), None);

    assert_eq!(swift_type_name("Cache?").as_deref(), Some("Cache"));
    assert_eq!(swift_type_name("inout Box<Int>").as_deref(), Some("Box"));
    for refused in [
        "any P", "some P", "(A, B)", "() -> A", "[A]", "A & B", "Mod.A", "Self",
    ] {
        assert_eq!(swift_type_name(refused), None, "{refused}");
    }
    assert_eq!(
        swift_element("[JSONPrinter]?").as_deref(),
        Some("JSONPrinter")
    );
    assert_eq!(swift_element("Set<Tag>").as_deref(), Some("Tag"));
    assert_eq!(swift_element("[String: Tag]"), None);
    assert_eq!(swift_generics("func f<T, U: P>(_ t: T) -> U {"), ["T", "U"]);

    let wrapped = "func make(\n    _ build: (Int) -> Int\n) async throws -> Session where A: B {\n";
    assert_eq!(swift_returns(wrapped, 1).as_deref(), Some("Session"));
    assert_eq!(swift_returns("func run() {\n", 1), None);

    assert_eq!(
        swift_expr("try? makeEncoder()"),
        Some(SwiftExpr::Call("makeEncoder".into()))
    );
    assert_eq!(
        swift_expr("Wire { $0 }"),
        Some(SwiftExpr::Call("Wire".into()))
    );
    assert_eq!(
        swift_expr("Wire(a) { $0 }"),
        Some(SwiftExpr::Call("Wire".into()))
    );
    assert_eq!(swift_expr("Wire(a,"), Some(SwiftExpr::Call("Wire".into())));
    assert_eq!(swift_expr("Wire().encoder"), None);
    assert_eq!(
        swift_expr("self.wire"),
        Some(SwiftExpr::Chain("self.wire".into()))
    );
    assert_eq!(
        swift_expr("found as! Wire"),
        Some(SwiftExpr::Cast("Wire".into()))
    );
    assert_eq!(swift_expr("a + b"), None);
}

#[test]
fn c_bindings_read_a_head_with_non_ascii_names() {
    let text = "int (r *R\u{e9}po) M\u{e9}thode(int x)\n{\n    return x;\n}\n";
    assert_eq!(c_bindings_at(text, 3, "x").first().map(|b| b.0), Some(1));
}

#[test]
fn php_namespace_patterns_cut_after_a_non_ascii_character() {
    let text = "<?php\nnamespace App;\n";
    let line = "    new \u{a9}Ns\\Bar();";
    let start = line.find("Ns").unwrap();
    let mut patterns = def_patterns(Kind::Php, "Ns");
    php_namespace_patterns(&mut patterns, text, line, start..start + 2);
    assert_eq!(patterns, [r"^\s*(?:<\?php\s+)?namespace\s+App\\Ns\s*[;{]"]);
}

#[test]
fn c_receiver_cuts_at_a_character() {
    assert_eq!(
        c_receiver("café."),
        None,
        "a name a non-ASCII character goes on with is not read"
    );
    assert_eq!(c_receiver("cafe\u{301} = e\u{301}tude."), None);
    assert_eq!(c_receiver("x = \"🇫🇷\" + 👨‍👩‍👧."), None);
    assert_eq!(c_receiver("$ßar->"), None);
    assert_eq!(
        c_receiver("ß = bar->"),
        Some(CReceiver {
            head: "bar".to_owned(),
            head_called: false,
            fields: vec![]
        })
    );
}

#[test]
fn swift_test_targets_come_from_the_manifest() {
    let dir = std::env::temp_dir().join(format!("merl-swift-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(
        swift_test_dirs(&dir),
        [PathBuf::from("Tests")],
        "with no manifest, `Tests`"
    );
    std::fs::write(
        dir.join("Package.swift"),
        r#"let package = Package(
    name: "App",
    targets: [
        .target(name: "App"),
        .testTarget(name: "AppTests", dependencies: ["App"]),
        .testTarget(
            name: "Checks",
            dependencies: ["App"],
            path: "./Checks/Unit"
        ),
    ]
)
"#,
    )
    .unwrap();
    assert_eq!(
        swift_test_dirs(&dir),
        [
            PathBuf::from("Tests/AppTests"),
            PathBuf::from("Checks/Unit")
        ],
        "a target's `path:`, else `Tests/<name>`"
    );
    std::fs::write(
        dir.join("Package.swift"),
        "let package = Package(name: \"App\")\n",
    )
    .unwrap();
    assert!(swift_test_dirs(&dir).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_csharp_file_sees_its_project_and_the_ones_it_references() {
    let files: Vec<PathBuf> = [
        "Shop.Api/Shop.Api.csproj",
        "Shop.App/Shop.App.csproj",
        "Shop.Core/Shop.Core.csproj",
        "Shop.App/Page.cs",
        "Shop.App/Views/Home.cs",
        "Shop.Api/Address.cs",
        "Shop.Core/Money.cs",
        "Shared/Clock.cs",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    fn read<'a>(manifests: &'a [(&'a str, &'a str)]) -> impl Fn(&Path) -> Option<String> + 'a {
        move |p: &Path| {
            manifests
                .iter()
                .find(|(f, _)| Path::new(f) == p)
                .map(|(_, t)| (*t).to_owned())
        }
    }
    const PLAIN: &str = r#"<Project Sdk="Microsoft.NET.Sdk"></Project>"#;
    let alone: &[(&str, &str)] = &[
        ("Shop.Api/Shop.Api.csproj", PLAIN),
        ("Shop.App/Shop.App.csproj", PLAIN),
        ("Shop.Core/Shop.Core.csproj", PLAIN),
    ];
    let here = Path::new("Shop.App/Views/Home.cs");
    let seen = cs_projects(&files, here, read(alone)).unwrap();
    assert!(seen.sees(Path::new("Shop.App/Page.cs")));
    assert!(!seen.sees(Path::new("Shop.Api/Address.cs")));
    assert!(
        seen.sees(Path::new("Shared/Clock.cs")),
        "a file under no project is anybody's"
    );
    let chained: &[(&str, &str)] = &[
        (
            "Shop.App/Shop.App.csproj",
            r#"<ItemGroup><ProjectReference Include="..\Shop.Api\Shop.Api.csproj" /></ItemGroup>"#,
        ),
        (
            "Shop.Api/Shop.Api.csproj",
            r#"<ProjectReference Include="../Shop.Core/Shop.Core.csproj"/>"#,
        ),
        ("Shop.Core/Shop.Core.csproj", PLAIN),
    ];
    let seen = cs_projects(&files, here, read(chained)).unwrap();
    assert!(
        seen.sees(Path::new("Shop.Api/Address.cs")),
        "a reference, `\\` and `..` as MSBuild writes them"
    );
    assert!(
        seen.sees(Path::new("Shop.Core/Money.cs")),
        "and the references of that one"
    );
    let props: &[(&str, &str)] = &[
        ("Shop.App/Shop.App.csproj", PLAIN),
        ("Shop.Api/Shop.Api.csproj", PLAIN),
        ("Shop.Core/Shop.Core.csproj", PLAIN),
        (
            "Directory.Build.props",
            r#"<Project><ItemGroup><ProjectReference Include="..\Shop.Core\Shop.Core.csproj" /></ItemGroup></Project>"#,
        ),
    ];
    let seen = cs_projects(&files, here, read(props)).unwrap();
    assert!(
        seen.sees(Path::new("Shop.Core/Money.cs")),
        "`Directory.Build.props` above the project references for it"
    );
    assert!(!seen.sees(Path::new("Shop.Api/Address.cs")));
    assert!(
        cs_projects(&files, Path::new("Shop.App/build.csx"), read(alone)).is_none(),
        "a script cannot be told: every file stays in sight"
    );
    assert!(
        cs_projects(&files, Path::new("Shared/Clock.cs"), read(alone)).is_none(),
        "nor can a file outside every project"
    );
    for app in [
        r#"<Compile Include="..\Shared\Clock.cs" Link="Clock.cs" />"#,
        r#"<Import Project="..\Shared\Shared.projitems" Label="Shared" />"#,
        r#"<ProjectReference Include="$(RepoRoot)\Shop.Api\Shop.Api.csproj" />"#,
        r#"<ProjectReference Include="..\Gone\Gone.csproj" />"#,
    ] {
        let manifests = [
            ("Shop.App/Shop.App.csproj", app),
            ("Shop.Api/Shop.Api.csproj", PLAIN),
        ];
        assert!(
            cs_projects(&files, here, read(&manifests)).is_none(),
            "{app}: a source file pulled in from outside the directory, a shared project, a \
             reference MSBuild has to evaluate or that names no project here cannot be told"
        );
    }
    let conditioned = [
        (
            "Shop.App/Shop.App.csproj",
            r#"<ProjectReference Condition="'$(Os)' == 'mac'" Include="..\Shop.Api\Shop.Api.csproj" />"#,
        ),
        ("Shop.Api/Shop.Api.csproj", PLAIN),
    ];
    let seen = cs_projects(&files, here, read(&conditioned)).unwrap();
    assert!(
        seen.sees(Path::new("Shop.Api/Address.cs")),
        "an attribute before `Include` is still read"
    );
    let quoted = [
        (
            "Shop.App/Shop.App.csproj",
            "<ProjectReference Include='..\\Shop.Api\\Shop.Api.csproj' />",
        ),
        ("Shop.Api/Shop.Api.csproj", PLAIN),
    ];
    assert!(
        cs_projects(&files, here, read(&quoted)).is_none(),
        "a reference the reader cannot read refuses"
    );
    let linked = [
        ("Shop.App/Shop.App.csproj", PLAIN),
        (
            "Directory.Build.props",
            r#"<Compile Include="..\Shared\Clock.cs" />"#,
        ),
    ];
    assert!(cs_projects(&files, here, read(&linked)).is_none());
    let mut nested = files.clone();
    nested.push(PathBuf::from("Shop.App/Tests/Shop.App.Tests.csproj"));
    let inner = [
        ("Shop.App/Shop.App.csproj", PLAIN),
        ("Shop.App/Tests/Shop.App.Tests.csproj", PLAIN),
    ];
    let seen = cs_projects(&nested, Path::new("Shop.App/Page.cs"), read(&inner)).unwrap();
    assert!(
        !seen.sees(Path::new("Shop.App/Tests/PageTests.cs")),
        "a project inside another's directory is a project of its own"
    );
    let mut two = files.clone();
    two.push(PathBuf::from("Shop.App/Shop.App.Tests.csproj"));
    assert!(
        cs_projects(&two, here, read(alone)).is_none(),
        "two projects in one directory cannot be told"
    );
}

#[test]
fn csharp_type_positions_namespace_segments_and_arity() {
    let at = |line: &str, word: &str| {
        let start = line.find(word).unwrap();
        cs_type_position(line, start, start + word.len())
    };
    for (line, word) in [
        ("    Buyer Update(Buyer buyer);", "Buyer"),
        ("    void Update(Buyer buyer);", "Buyer"),
        ("    Buyer[] all = Load();", "Buyer"),
        ("    Buyer? b = null;", "Buyer"),
        ("    List<Buyer> all;", "List"),
        ("    var d = new Dictionary<int, Buyer>();", "Buyer"),
        ("    var a = new Address { Street = s };", "Address"),
        ("    if (x is Buyer) return;", "Buyer"),
        ("    var t = typeof(Buyer);", "Buyer"),
        ("    var b = o as Buyer;", "Buyer"),
        ("    var b = (Buyer)o;", "Buyer"),
        ("    return (Buyer)o;", "Buyer"),
        ("public class Order : Entity, IAggregateRoot", "Entity"),
        (
            "public class Order : Entity, IAggregateRoot",
            "IAggregateRoot",
        ),
    ] {
        assert!(at(line, word), "{line} / {word}");
    }
    for (line, word) in [
        ("    var ok = valid ? a : b;", "valid"),
        ("    if (x is null) return;", "x"),
        ("    foreach (var item in items)", "item"),
        ("    Courier.Weigh(grams);", "Courier"),
        ("    if (a < Max && b > c) return;", "Max"),
        ("    if (ready) return;", "ready"),
        ("    Run(Buyer);", "Buyer"),
        ("    var n = (count) * 2;", "count"),
        ("    var empty = (Items) is null;", "Items"),
        ("    var b = order switch { _ => 1 };", "order"),
        ("    var x = new Courier.Inner();", "Courier"),
        ("public class Order : Base(Total, Other)", "Other"),
    ] {
        assert!(!at(line, word), "{line} / {word}");
    }
    let constant = |line: &str, word: &str| {
        let start = line.find(word).unwrap();
        cs_constant_may_stand(line, start, start + word.len())
    };
    assert!(
        constant("    if (n is Max) return;", "Max"),
        "after `is` a constant pattern may stand as well as a type"
    );
    assert!(constant("    bool full = n is Max;", "Max"));
    for line in [
        "    if (n is Max m) return;",
        "    if (n is Max<int>) return;",
        "    var m = n as Max;",
        "    var m = new Max();",
        "    Axis Max;",
    ] {
        assert!(
            !constant(line, "Max"),
            "{line}: no constant pattern stands here"
        );
    }
    let prefix = |line: &str, word: &str| {
        let start = line.find(word).unwrap();
        cs_namespace_prefix(line, start, start + word.len())
    };
    let certain = |prefix: &str| {
        Some(CsNamespacePrefix {
            prefix: prefix.to_owned(),
            certain: true,
        })
    };
    assert_eq!(
        prefix(
            "using Microsoft.EntityFrameworkCore.Migrations;",
            "EntityFrameworkCore"
        ),
        certain("Microsoft.EntityFrameworkCore")
    );
    assert_eq!(
        prefix("global using Microsoft.Extensions.Options;", "Options"),
        certain("Microsoft.Extensions.Options")
    );
    assert_eq!(
        prefix("namespace Shop.Catalog {", "Catalog"),
        certain("Shop.Catalog")
    );
    assert_eq!(
        prefix("    global::Shop.Pricing.Tariff t;", "Pricing"),
        Some(CsNamespacePrefix {
            prefix: "Shop.Pricing".to_owned(),
            certain: false
        })
    );
    assert_eq!(prefix("using Rows = List<int>;", "List"), None);
    assert_eq!(prefix("using static System.Math;", "Math"), None);
    assert_eq!(prefix("using var stream = Open();", "stream"), None);
    let args = |text: &str| {
        let lines: Vec<&str> = text.lines().collect();
        let end = lines[0].find("Equals").unwrap() + "Equals".len();
        cs_arguments(&lines, 0, end)
    };
    assert_eq!(args("return Equals(a, Pick(b, c));"), Some(2));
    assert_eq!(args("return Equals();"), Some(0));
    assert_eq!(
        args("return Equals<Dictionary<int, string>>(\n    a,\n    b);"),
        Some(2)
    );
    assert_eq!(args("return Equals;"), None);
    assert_eq!(args("return Equals(a < b, c > d);"), None);
    assert_eq!(args("return Equals(a,"), None);
    let params = |line: &str| {
        cs_parameters(line, 1, "Equals").map(|a| (a.fewest, a.most_unless_params, a.extension_this))
    };
    assert_eq!(
        params("public bool Equals(object other)"),
        Some((1, Some(1), false))
    );
    assert_eq!(
        params("public bool Equals(Dictionary<int, string> a, int b = 0)"),
        Some((1, Some(2), false))
    );
    assert_eq!(
        params("static bool Equals(this X x, params int[] rest)"),
        Some((1, None, true))
    );
    assert_eq!(params("public record Equals(int A);"), None);
    assert_eq!(params("public Func<int, bool> Equals { get; }"), None);
}
