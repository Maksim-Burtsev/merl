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
