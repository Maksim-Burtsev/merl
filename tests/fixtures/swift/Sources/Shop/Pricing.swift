/* A block comment that reads like code declares nothing:
struct Tariff {
*/

public let rateCap = 100
public var currency = "EUR"

public struct Tariff {
    public let base: Int
    public init(base: Int) {
        self.base = base
    }
    public func rate() -> Int {
        1
    }
    public func describe() -> String {
        "tariff"
    }
}

public final class Coupon {
    public private(set) var code: String
    public init?(code: String) {
        if code.isEmpty { return nil }
        self.code = code
    }
    deinit {
    }
    public func rate() -> Int {
        2
    }
    public func describe() -> String {
        "coupon"
    }
}

public protocol Priced {
    associatedtype Amount
    func price() -> Amount
}

public enum Offer {
    case plain
    case cut(Int), bundle(count: Int)
}

public enum Level: Int {
    case low = 1
}

public typealias Money = Int

public actor Ledger {
    var entries: [Money] = []
    subscript(index: Int) -> Money {
        entries[index]
    }
}

@inlinable public func discount(_ total: Int) -> Int {
    min(total, rateCap) - 1
}

public func settle<T: Priced>(_ item: T) -> T.Amount {
    item.price()
}

public func `default`() -> Int {
    0
}

public let banner = """
func weigh(_ grams: Int) -> Int {
struct Basket {
"""

public let pattern = #"""
func settle() -> Int {
"""#

public struct Parcel {
    enum Seal { case wax }
    func sealed() -> Seal { .wax }
    //               ^ d: picker Sources/Shop/Pricing.swift:82, Tests/ShopTests/ParcelTests.swift:1; want Sources/Shop/Pricing.swift:82 (#375)
}
