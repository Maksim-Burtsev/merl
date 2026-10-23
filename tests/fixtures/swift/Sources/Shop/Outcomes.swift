// The night pass of 2026-10-19 over `d` in Swift (#1067, #1068, #1053, #1052, #1069, #1047, #1054,
// #1055), answers sourcekit-lsp's. An implicit `.x` is never an instance property (#1067).
extension Result {
    var success: Success? {
        guard case let .success(value) = self else { return nil }
        return value
    }
}

func settleOutcome(_ value: Int) -> Result<Int, Error> {
    return .success(value)
    //      ^ d: none
}

func outcomeValue(_ outcome: Result<Int, Error>) -> Int? {
    outcome.success
    //      ^ d: Sources/Shop/Outcomes.swift:4
}

func outcomeValueBelow(_ outcome: Result<Int, Error>) -> Int? {
    outcome
        .success
    //   ^ d: Sources/Shop/Outcomes.swift:4
}

// A receiver of a type from outside the project answers from the project's extensions of
// outside types alone, and a value is never followed by a case or a static member (#1068).
final class Logbook {
    var isEmpty: Bool { true }
    static let blank = Logbook()
}

enum Casing {
    case lowercased
}

extension Collection {
    var isPacked: Bool { !isEmpty }
}

func ledgerOf(_ path: [Int], _ table: [String: Int], _ word: Substring, _ ledger: Logbook) -> Bool {
    _ = word.lowercased()
    //       ^ d: none
    _ = ledger.isEmpty
    //         ^ d: Sources/Shop/Outcomes.swift:29
    _ = table.isEmpty
    //        ^ d: none
    _ = path.isPacked
    //       ^ d: Sources/Shop/Outcomes.swift:38
    _ = Logbook.blank
    //          ^ d: Sources/Shop/Outcomes.swift:30
    _ = Casing.lowercased
    //         ^ d: Sources/Shop/Outcomes.swift:34
    return path.isEmpty
    //          ^ d: none
}

func casings(_ names: [String]) -> [String] {
    names.map { $0.lowercased() }
    //             ^ d: none
}

extension Collection<String> {
    func quoted() -> String { "" }
}

extension [String] {
    func ampersanded() -> String { "" }
}

func joinedUp(_ parts: [String]) -> String {
    let more: [String] = parts
    return parts.quoted() + more.ampersanded()
    //           ^ d: Sources/Shop/Outcomes.swift:64
    //                             ^ d: Sources/Shop/Outcomes.swift:68
}

protocol Bundled {}

extension Bundled {
    var bundle: Int { 0 }
}

extension Array: Bundled {}

func bundled(_ items: [Int]) -> Int {
    items.bundle
    //    ^ d: Sources/Shop/Outcomes.swift:81
}

// A receiver typed through a `typealias` reads the aliased type, one hop (#1053).
struct Satchel<Contents, Fault> {
    let unwrapped: Contents?
}

struct Envelope {
    let unwrapped: Int
}

public typealias ShopSatchel<Contents> = Satchel<Contents, Error>
typealias Posted = ShopSatchel<Int>

func opened(_ satchel: ShopSatchel<Int>?, _ posted: Posted) -> Int? {
    _ = posted.unwrapped
    //         ^ d: picker Sources/Shop/Outcomes.swift:93, Sources/Shop/Outcomes.swift:97
    return satchel?.unwrapped
    //              ^ d: Sources/Shop/Outcomes.swift:93
}

// Overloads that differ by a defaulted parameter or a trailing closure: the one the call fits (#1052).
final class Turnstile {
    func authorize(user: String, password: String, keep: Bool = true) -> Turnstile { self }
    func authorize(with token: Int) -> Turnstile { self }
    func announce(on queue: Int, calling handler: @escaping (String) -> Void) -> Turnstile { self }
    func announce(calling handler: @escaping (String) -> Void) -> Turnstile { self }
}

func admit(_ gate: Turnstile) {
    _ = gate.authorize(user: "a", password: "b")
    //       ^ d: Sources/Shop/Outcomes.swift:112
    _ = gate.announce { _ in }
    //       ^ d: Sources/Shop/Outcomes.swift:115
    _ = gate.announce(on: 1) { _ in }
    //       ^ d: Sources/Shop/Outcomes.swift:114
}

func untyped(_ gates: [Turnstile]) {
    gates.randomElement()!.authorize(user: "a", password: "b")
    //                     ^ d: Sources/Shop/Outcomes.swift:112
}

func consign(
    to place: String,
    express: Bool = false
) {}

func consigned() {
    consign(to: "dock", express: true)
    //                  ^ d: Sources/Shop/Outcomes.swift:132
}

// A case of an enum is a value of that enum (#1055).
enum Roast {
    case light, dark

    var level: Int { 0 }
}

struct Blend {
    let level: Int
}

func roasted() -> Int {
    Roast.dark.level
    //         ^ d: Sources/Shop/Outcomes.swift:146
}

// A `let` binds its name only after its `=`: the name on the right is the one outside (#1047).
func warden(_ n: Int) -> Int { n }

final class Ward {
    let sentries: [Int] = []

    func patrol() -> Int {
        let sentries = sentries.count
        //             ^ d: Sources/Shop/Outcomes.swift:162
        let warden = warden(sentries)
        //           ^ d: Sources/Shop/Outcomes.swift:159
        //                  ^ d: Sources/Shop/Outcomes.swift:165
        return warden
        //     ^ d: Sources/Shop/Outcomes.swift:167
    }
}

// An implicit `.x` reads the type written next to it: a closure parameter's, the declared type of
// a returning func or property, an annotation; a type from outside the project answers none (#1069).
final class Gate {
    enum ResponseDisposition {
        case allow, cancel
    }
}

func respond(completion: @escaping @Sendable (URLSession.ResponseDisposition) -> Void) {
    completion(.allow)
    //          ^ d: none
}

func disposed() -> URLSession.ResponseDisposition {
    let pick = { () -> Gate.ResponseDisposition in
        return .allow
        //      ^ d: Sources/Shop/Outcomes.swift:179
    }
    _ = pick
    return .cancel
    //      ^ d: none
}

func gated() -> Gate.ResponseDisposition {
    return .cancel
    //      ^ d: Sources/Shop/Outcomes.swift:179
}

extension Gate.ResponseDisposition {
    var outward: URLSession.ResponseDisposition {
        switch self {
        case .allow: .allow
        //            ^ d: none
        case .cancel: .cancel
        //    ^ d: Sources/Shop/Outcomes.swift:179
        }
    }
}

func annotated() -> Roast {
    let posted: URLSession.ResponseDisposition = .cancel
    //                                            ^ d: none
    _ = posted
    let roast: Roast = .dark
    //                  ^ d: Sources/Shop/Outcomes.swift:144
    return roast
}

// An unqualified call inside a type reaches a method through the implicit `self` (#1054).
class Bench {
    func stored(_ crate: Crate) -> Crate { crate }
}

final class Crate {
    func unload() {}
}

final class Pallet {
    func unload() {}
}

final class Dock: Bench {
    func work() {
        let crate = stored(Crate())
        crate.unload()
        //    ^ d: Sources/Shop/Outcomes.swift:229
    }
}

// An element taken by index, and a type from outside the project as the receiver of a static
// member, answer from the project's extensions or none (#1068).
struct Label {
    var stringValue: String
    static var isCancelled: Bool { false }
}

extension DispatchQueue {
    static let shopQueue = DispatchQueue(label: "shop")
}

func indexed(_ keys: [any CodingKey], _ ledgers: [Logbook]) -> Bool {
    let end = keys[0]
    _ = end.stringValue
    //      ^ d: none
    let first = ledgers[0]
    _ = first.isEmpty
    //        ^ d: Sources/Shop/Outcomes.swift:29
    _ = DispatchQueue.shopQueue
    //                ^ d: Sources/Shop/Outcomes.swift:252
    return Task.isCancelled
    //          ^ d: none
}

// A field of a type from outside the project, which no extension of the project declares, is a
// value from outside too: its members answer from the project's extensions or none (#1068).
struct Gateway {
    var host: String
}

func outsideFields(_ fileURL: URL, _ request: URLRequest) -> Int {
    let fileName = fileURL.lastPathComponent
    _ = fileName.isEmpty
    //           ^ d: none
    guard
        let url = request.url,
        let host = url.host else { return 0 }
    //                 ^ d: none
    return host.sniffed() + fileURL.path.sniffed()
    //          ^ d: Sources/Shop/Infuse/Sencha.swift:37
    //                                   ^ d: Sources/Shop/Infuse/Sencha.swift:37
}

// Two steps of reading before the value from outside: `request` from a field, `url` from it. The
// `url` of the project's own types and protocols below is not one of `URLRequest` (#1068).
final class Postman {
    var lastRequest: URLRequest?
}

extension Postman {
    func described() -> String {
        guard
            let request = lastRequest,
            let url = request.url,
            let host = url.host else { return "" }
        //                 ^ d: none
        return host
    }
}

extension Gateway {
    var url: String { host }
}

extension Postman {
    struct Route {}
}

extension Postman.Route {
    var url: String { "" }
}

protocol Addressed {}

extension Addressed {
    var url: Gateway { Gateway(host: "") }
}

extension URLCredential: Addressed {}

func addressed(_ credential: URLCredential) -> String {
    credential.url.host
    //             ^ d: Sources/Shop/Outcomes.swift:271
}

// A typealias of a collection from outside is that collection: its members answer from the
// project's extensions of it or none (#1068).
typealias Tags = [String: Int]

func tagged(_ tags: Tags?) -> Bool {
    guard let tags else { return true }
    return tags.isEmpty
    //          ^ d: none
}

// A value cast to a type from outside the project in parentheses is that type; the result of a
// call that takes a cast is not (#1068).
final class Beacon {
    var errorCode = 0
}

func beaconed(_ failure: URLError?) -> Beacon { Beacon() }

func casted(_ failure: any Error, _ relay: Any) -> Int {
    let code = (failure as? URLError)?.errorCode ?? (relay as? Beacon)?.errorCode ?? 0
    //                                 ^ d: none
    //                                                                  ^ d: Sources/Shop/Outcomes.swift:342
    return code + beaconed(failure as? URLError).errorCode
    //                                           ^ d: Sources/Shop/Outcomes.swift:342
}

// A cast to a type from outside keeps the project's extension of a type it may inherit from; an
// extension of a class of the project is not one of its supertypes (#1068).
extension NSResponder {
    func nudged() -> Int { 0 }
}

extension Beacon {
    func pinged() -> Int { 0 }
}

func lifted(_ relay: Any) -> Int {
    let up = (relay as? NSView)?.nudged() ?? 0
    //                           ^ d: Sources/Shop/Outcomes.swift:358
    let down = (relay as? URLError)?.pinged() ?? 0
    //                               ^ d: none
    return up + down
}

// A closure parameter of a generic wrapper's method that hands the closure the wrapped value is
// that value; its fields from outside answer from the project's extensions or none (#1068).
final class Strongbox<Value> {
    private var value: Value

    init(_ value: Value) {
        self.value = value
    }

    func read<U>(_ closure: (Value) throws -> U) rethrows -> U {
        try closure(value)
    }

    func write<U>(_ closure: (inout Value) throws -> U) rethrows -> U {
        try closure(&value)
    }

    func write(_ value: Value) {
        self.value = value
    }
}

struct Coffer {
    var queued: [Int] = []
    var seals: Set<String> = []
}

final class Vaultkeeper {
    func remove(_ seal: String) {}

    let coffer = Strongbox(Coffer())
    let strongbox: Strongbox<Coffer>

    init(_ strongbox: Strongbox<Coffer>) {
        self.strongbox = strongbox
    }

    func drain() -> Bool {
        coffer.write { $0.seals.remove("lead") }
        //                      ^ d: none
        let idle = coffer.read { state in state.queued.isEmpty }
        //                                             ^ d: none
        //                                      ^ d: Sources/Shop/Outcomes.swift:396
        return strongbox.write { strongbox in
            strongbox.queued.isEmpty && idle
            //               ^ d: none
            //        ^ d: Sources/Shop/Outcomes.swift:396
        }
    }
}

// A nested type that several types declare is the one the enclosing types declare, innermost
// first, when a wrapper holds it by construction or by annotation, and a field typed over several
// lines is read whole. Past a type that inherits (`Teller: Basis`, whose `Basis` may declare a
// `Sheet` of its own), and for a type declared inside a function, the name stays ambiguous: a
// picker, never a jump (#1068).
final class Cashbook {
    struct Sheet {
        var seals: Set<String> = []
        var pending: [Int] = []
    }

    let sheet = Strongbox(Sheet())

    func settle(_ seal: String) {
        sheet.write { $0.seals.remove(seal) }
        //                     ^ d: none
        //               ^ d: Sources/Shop/Outcomes.swift:431
    }

    final class Clerk {
        let sheet = Strongbox(Sheet())

        func idle() -> Bool {
            sheet.read { $0.pending.isEmpty }
            //                      ^ d: none
        }
    }

    func tally() -> Bool {
        struct Sheet {
            var pending: [Int] = []
        }
        let sheet = Strongbox(Sheet())
        return sheet.read { $0.pending.isEmpty }
        //                     ^ d: picker Sources/Shop/Outcomes.swift:432, Sources/Shop/Outcomes.swift:454, Sources/Shop/Outcomes.swift:473; want Sources/Shop/Outcomes.swift:454 (#1068)
    }

    final class Teller: Basis {
        let sheet = Strongbox(Sheet())

        func idle() -> Bool {
            sheet.read { $0.pending.isEmpty }
            //              ^ d: picker Sources/Shop/Outcomes.swift:432, Sources/Shop/Outcomes.swift:454, Sources/Shop/Outcomes.swift:473
        }
    }
}

final class Auditor<Witness> {
    private struct Sheet {
        var pending: [Int] = []
        var lines: [(entry: String,
                     amount: Int)] = []
    }

    private let sheet: Strongbox<Sheet>

    init() {
        sheet = Strongbox(Sheet())
    }

    func check() -> Bool {
        sheet.write { sheet in
            sheet.pending.isEmpty && sheet.lines.isEmpty
            //                                   ^ d: none
            //            ^ d: none
            //    ^ d: Sources/Shop/Outcomes.swift:473
        }
    }
}

// The keys and the values of a dictionary of outside types are values from outside; of a
// project type, they keep the search by name (#1068).
final class Tally {
    func sorted() -> Tally { self }
}

func tallied(_ table: [String: Int], _ shelf: [String: Logbook]) -> Bool {
    _ = table.keys.sorted()
    //             ^ d: none
    _ = table.values.sorted()
    //               ^ d: none
    return shelf.values.first?.isEmpty ?? false
    //                         ^ d: Sources/Shop/Outcomes.swift:29
}

// A type nested in a generic outside type is from outside, though a project type names an alias
// like it; one nested in a generic project type is that type's (#1068).
struct Drip<Element> {
    typealias AsyncIterator = Iterator

    struct Iterator {
        private var iterator: AsyncStream<Element>.AsyncIterator
        private var mine: Hopper<Element>.Slot

        mutating func next() async -> Element? {
            _ = mine.weight
            //       ^ d: Sources/Shop/Outcomes.swift:529
            return await iterator.next()
            //                    ^ d: none
        }
    }
}

struct Hopper<Item> {
    struct Slot {
        var weight: Int
    }
}

struct Barrow {
    var weight: Int
}

// A nested name counts where Swift places it (#1068): an alias under a generic outer type or in an
// extension of a nested type answers, and one written inside a namespace, where it may name the
// namespace's own alias or a top-level namesake, asks. A type nested in an outside generic is no
// Sequence, so a project's `extension Sequence` does not answer for it (`iterator.next()` above).
struct Flask {
    func tilt() {}
}
struct Beaker {
    func tilt() {}
}

struct Vial<Dose> {
    typealias Neck = Flask
}

enum Cellar {
    struct Rack {
        typealias Neck = Flask
    }
}
extension Cellar.Rack {
    typealias Spout = Beaker
}

struct Rack {
    struct Neck {
        func tilt() {}
    }
}

func tilted(_ v: Vial<Int>.Neck, _ s: Cellar.Rack.Spout) {
    v.tilt()
    //^ d: Sources/Shop/Outcomes.swift:542
    s.tilt()
    //^ d: Sources/Shop/Outcomes.swift:545
}

extension Cellar {
    static func tilted(_ n: Rack.Neck) {
        n.tilt()
        //^ d: picker Sources/Shop/Outcomes.swift:542, Sources/Shop/Outcomes.swift:545, Sources/Shop/Outcomes.swift:563
    }
}

extension Sequence {
    func next() -> Int? { nil }
}

// A call with only literal arguments on a value from outside, which no extension of the project
// declares, returns a value from outside: its members answer from the project's extensions or
// none. A call on a project value, or one whose argument is not a literal, keeps its path (#1068).
func calledOutside(_ key: String, _ sencha: Sencha, _ n: Int) -> String {
    _ = key.prefix(1).sniffed()
    //                ^ d: Sources/Shop/Infuse/Sencha.swift:37
    _ = sencha.steeped().uppercased()
    //                   ^ d: Sources/Shop/Infuse/Sencha.swift:10
    _ = key.prefix(n).uppercased()
    //                ^ d: Sources/Shop/Infuse/Sencha.swift:10; want none (#1068)
    return key.prefix(1).uppercased() + key.dropFirst(2).uppercased()
    //                   ^ d: none
    //                                                   ^ d: none
}

// A closure's parameter shadows the outer value of its name, so a literal call on it keeps the
// typed path. Labelled, negative, `true` and string arguments are literals too (#1068).
struct Twig {
    var budded: String
    func child(_ i: Int) -> Twig { self }
}

func twigged(_ item: String, _ twigs: [Twig], _ key: String) -> String {
    twigs.forEach { item in _ = item.child(1).budded }
    //                                        ^ d: Sources/Shop/Outcomes.swift:603
    _ = key.padding(toLength: 4).uppercased()
    //                           ^ d: none
    _ = key.dropLast(-1).uppercased()
    //                   ^ d: none
    _ = key.split(omittingEmptySubsequences: true).uppercased()
    //                                             ^ d: none
    return key.appending("s").uppercased()
    //                        ^ d: none
}

// An array literal of strings, through `compactMap(\.self)` or not, is a `[String]`: a method the
// project's `extension [String]` declares types the value it returns (#1068).
extension [String] {
    func potted() -> Sencha { Sencha(grade: count) }
}

func ampersandedAll(_ query: String?, _ more: String) -> Bool {
    let joined = [query, more].compactMap(\.self).ampersanded()
    let plain = [more, more].potted()
    _ = plain.grade
    //        ^ d: Sources/Shop/Infuse/Sencha.swift:2
    return joined.isEmpty
    //            ^ d: none
}

// A closure passed to a method of the enclosing type takes its parameters' types from the
// method's parameter, a typealias of that type; overloads that cannot take a closure (a
// constrained generic, an autoclosure, none) step aside (#1068).
final class Steamer {
    typealias Check = (Int, String?, Sencha) -> Bool
    func check(_ c: @escaping Check) -> Self { self }
    func check<S: Sequence>(grades: S) -> Self { self }
    func check(label: @autoclosure () -> String) -> Self { self }
    func check() -> Self { self }
}

extension Steamer {
    func boiled() -> Self {
        check { [unowned self] _, note, sencha in
            _ = sencha.grade
            //         ^ d: Sources/Shop/Infuse/Sencha.swift:2
            return note?.isEmpty == true
            //           ^ d: none
        }
    }
}
