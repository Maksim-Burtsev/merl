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
