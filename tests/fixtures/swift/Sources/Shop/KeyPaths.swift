// A key path handed to a generic wrapper's method that takes a closure of the wrapped value is
// rooted at that value, the way `$0` of the closure is (#1341).
struct Till {
    var balance = 0
    static var minted = 0
}

struct Purse {
    var balance = 0
}

final class Lockbox<Value> {
    private var value: Value

    init(_ value: Value) {
        self.value = value
    }

    func peek<U>(_ path: KeyPath<Value, U>) -> U {
        value[keyPath: path]
    }

    func read<U>(_ closure: (Value) throws -> U) rethrows -> U {
        try closure(value)
    }

    func read<U>(_ closure: (U) -> U, seed: U) -> U {
        closure(seed)
    }
}

final class Bursar {
    let ledger = Strongbox(Till())
    let purse = Strongbox(Purse())
    let lockbox = Lockbox(Till())

    var balance: Int {
        ledger.read(\.balance)
        //             ^ d: Sources/Shop/KeyPaths.swift:4
        //               status: balance → Till.balance (via \.balance: Till)
    }

    var stowed: Int {
        get { self.purse.read(\.balance) }
        //                       ^ d: Sources/Shop/KeyPaths.swift:9
    }

    var peeked: Int {
        lockbox.peek(\.balance)
        //              ^ d: picker Sources/Shop/KeyPaths.swift:4, Sources/Shop/KeyPaths.swift:9, Sources/Shop/KeyPaths.swift:37
    }

    var shadowed: Int {
        lockbox.read(\.balance)
        //              ^ d: picker Sources/Shop/KeyPaths.swift:4, Sources/Shop/KeyPaths.swift:9, Sources/Shop/KeyPaths.swift:37
    }

    var summed: [Int] {
        [Till()].map(\.balance)
        //             ^ d: picker Sources/Shop/KeyPaths.swift:4, Sources/Shop/KeyPaths.swift:9, Sources/Shop/KeyPaths.swift:37
    }
}
