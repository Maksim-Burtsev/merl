import Foundation
@testable import LedgerKit

/// A ledger keeps entries in order.
/// - Parameter owner: who keeps it
public struct Entry: Codable, Hashable {
    public let id: Int
    public var amount: Double
    var note: String?
}

enum Kind: String, CaseIterable {
    case credit = "credit"
    case debit
}

protocol Ledgerish: AnyObject {
    associatedtype Item
    func total() -> Double
    var count: Int { get }
}

typealias Handler = (Entry) -> Void

let limit = 1_000
private let pattern = /[a-z]+\d/
fileprivate let hex = 0xFF_FF

@MainActor
final class Ledger<T: Comparable>: Ledgerish, CustomStringConvertible where T: Sendable {
    typealias Item = Entry
    private(set) var entries: [Entry] = []
    weak var delegate: AnyObject?
    lazy var opened = Date()
    static let shared = Ledger(owner: "nobody")
    let owner: String

    var count: Int { entries.count }

    var description: String {
        "Ledger(owner: \(owner), size: \(entries.count))"
    }

    init(owner: String) {
        self.owner = owner
    }

    deinit {
        print("closed")
    }

    func total() -> Double {
        entries.reduce(0) { $0 + $1.amount }
    }

    @discardableResult
    func add(_ amount: Double, note: String? = nil) -> Entry {
        let entry = Entry(id: entries.count, amount: amount, note: note)
        entries.append(entry)
        return entry
    }

    func sync(ids: Int...) async throws {
        for id in ids where id >= 0 {
            guard id < limit else { continue }
            try await Task.sleep(nanoseconds: 1_000)
        }
    }

    func kind(of entry: Entry) -> Kind {
        switch entry.amount {
        case ..<0:
            return .debit
        case 0:
            fatalError("zero")
        default:
            return .credit
        }
    }

    func report() -> String {
        let lines = entries.map { e in
            String(format: "%-10@ %8.2f", e.note ?? "-", abs(e.amount))
        }
        let raw = """
            total: \(total())
            count: \(entries.count)
            """
        return lines.joined(separator: "\n") + raw
    }

    func check(_ x: Any) -> Bool {
        if let s = x as? String, !s.isEmpty {
            return true
        }
        let ratio = 1.5e3
        return x is Int || ratio != 0 && hex > 0 ? true : false
    }
}

extension Ledger where T == Int {
    subscript(index: Int) -> Entry? {
        entries.indices.contains(index) ? entries[index] : nil
    }
}

do {
    let ledger = Ledger<Int>(owner: "me")
    ledger.add(10, note: "start")
    try await ledger.sync(ids: 1, 2)
} catch {
    print(error)
}
// done
