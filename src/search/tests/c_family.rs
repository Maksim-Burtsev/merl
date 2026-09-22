//! C, C++, C#, Swift and PHP.

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
    // The `typedef struct invoice {` and the `} invoice;` it closes with: `d` offers both,
    // where `D` lists the type once, under the name the project uses.
    assert_eq!(d("invoice"), [20, 26]);
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
        Vec::<usize>::new(),
        "an enum constant has no rule: `NAME,` is also a line of an initializer list"
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
    // Column zero is where C declares; indented, only a body opening on the line counts.
    assert_eq!(
        d("invoice_valid"),
        Vec::<usize>::new(),
        "a call inside `if`"
    );
    assert_eq!(d("free"), Vec::<usize>::new(), "a call statement");
    assert_eq!(d("copy"), Vec::<usize>::new(), "a local");
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
    // The out-of-line definition. The declaration on line 22 has no rule: indented, it is
    // the shape of a call, and the definition is what `d` is asked for anyway.
    assert_eq!(d("append"), [30]);
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
    assert_eq!(d("Open"), Vec::<usize>::new(), "an enum constant");
    assert_eq!(d("total_"), Vec::<usize>::new(), "a field");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn c_and_cpp_scope_roots_and_names() {
    // One kind: a `.cc` is searched for a definition asked for in a `.h`, and a file of
    // another kind is not.
    let here = Path::new("ledger.hpp");
    assert!(in_def_scope(Kind::C, here, Path::new("src/invoice.c")));
    assert!(in_def_scope(Kind::C, here, Path::new("ledger.cc")));
    assert!(!in_def_scope(Kind::C, here, Path::new("main.rs")));
    // `#include` binds no name, and a member of a value has no rule of its own.
    assert!(imports(Kind::C, C_C).is_empty());
    assert!(member_patterns(Kind::C, "total").is_none());
    // The system headers, wherever this machine keeps them: the SDK on a Mac,
    // `/usr/include` on Linux. Both exist on CI, so the list is never empty there.
    let roots = external_roots(Kind::C, Path::new("/"));
    assert!(
        roots.iter().any(|r| r.ends_with("usr/include")),
        "no system include directory among {roots:?}"
    );
    // C++ writes a member behind `::`, as Rust does, and an access specifier is a label
    // inside the class, not a wall in front of it. The enclosing `namespace billing {` is
    // not indented, so, as in every kind, the walk ends at the first column-zero declaration.
    assert_eq!(
        qualified(Kind::C, CPP, 20, "total").as_deref(),
        Some("Ledger::total")
    );
    assert_eq!(qualified(Kind::C, CPP, 3, "billing"), None);
}

#[test]
fn c_and_cpp_files_find_each_other() {
    let (dir, files) = scratch(
        "c-family",
        &[("invoice.h", C_H), ("invoice.c", C_C), ("ledger.cc", CPP)],
    );
    // The header's prototype and the definition that follows it are both offered; the
    // picker's rows say which file each is in.
    let pat = def_patterns(Kind::C, "invoice_total").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("invoice.c".into(), 6), ("invoice.h".into(), 40)],
        "the picker's order is by path, as everywhere: it is not definition before prototype"
    );
    // A type of the C header, reached from the C++ file that includes it.
    let pat = def_patterns(Kind::C, "sds").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("invoice.h".into(), 7)]
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
    // The class past its generic parameters and its attribute, and the constructor; the
    // caller shows a picker.
    assert_eq!(d("Invoice"), [6, 12]);
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
    // A `using` opens a whole namespace, so it binds no name of its own, and there is nothing
    // to bind it to: a NuGet package ships assemblies, not source.
    assert!(imports(Kind::CSharp, CS).is_empty());
    assert!(external_roots(Kind::CSharp, Path::new("/")).is_empty());
    assert!(member_patterns(Kind::CSharp, "Total").is_none());
    // A member is named by the type it is declared in, as the picker rows show it.
    assert_eq!(
        qualified(Kind::CSharp, CS, 22, "LoadAsync").as_deref(),
        Some("Invoice.LoadAsync")
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
    // The class and the extension of it: a project's own members of a type live in one.
    assert_eq!(d("Session"), [10, 40], "not the `Session()` calls");
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
    // The `let`, not the `case opened:` of the `switch` below it, which matches against
    // that constant: a bare name there is a pattern, not a declaration.
    assert_eq!(d("opened"), [71]);
    std::fs::remove_dir_all(&dir).unwrap();
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
    // An `import` names a module and makes everything in it visible unqualified, so it binds
    // no name of its own.
    assert!(imports(Kind::Swift, SWIFT).is_empty());
    // Outside the project is where SwiftPM checks the dependencies out; nothing else on the
    // machine holds Swift source, so an absent directory leaves the list empty.
    let (dir, _) = scratch(
        "swift-roots",
        &[(".build/checkouts/nio/Sources/a.swift", "")],
    );
    assert_eq!(
        external_roots(Kind::Swift, &dir),
        [dir.join(".build/checkouts")]
    );
    assert!(external_roots(Kind::Swift, Path::new("/")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
    // A member is named by the type its extension extends.
    assert_eq!(
        qualified(Kind::Swift, SWIFT, 41, "didFinish").as_deref(),
        Some("Session.didFinish")
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
    // The property and the method that returns it, not the `$this->rows` uses.
    assert_eq!(d("rows"), [14, 33]);
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
    // Not the `$total == 0` that opens line 72: `==` compares, it declares nothing.
    assert_eq!(d("total"), [21, 23]);
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

#[test]
fn php_scope_roots_imports_and_names() {
    let here = Path::new("src/Invoice.php");
    assert!(in_def_scope(Kind::Php, here, Path::new("views/show.phtml")));
    assert!(!in_def_scope(Kind::Php, here, Path::new("main.rs")));
    // A `use` in column zero binds the last part of the path, or its alias; PSR-4 spells the
    // namespace the way the file system does, so the path is the name split on `\`.
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
        ]
    );
    // An indented `use` pulls a trait into a class body and names no file.
    assert!(imports(Kind::Php, "class X {\n    use Macroable;\n}\n").is_empty());
    // Composer installs the dependencies into `vendor/`, which is gitignored and so outside
    // the project walk, the way `node_modules` is.
    let (dir, _) = scratch("php-roots", &[("vendor/laravel/framework/src/a.php", "")]);
    assert_eq!(external_roots(Kind::Php, &dir), [dir.join("vendor")]);
    assert!(external_roots(Kind::Php, Path::new("/")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
    // PHP writes a method of a class behind `::`, as its own documentation does.
    assert_eq!(
        qualified(Kind::Php, PHP, 28, "parse").as_deref(),
        Some("Invoice::parse")
    );
}
