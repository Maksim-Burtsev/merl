protocol Inking<Print> {
    associatedtype Print
    func ink(blank: Int?) throws -> Print
}

protocol Pressing<Print> {
    associatedtype Print
    func press(blank: Int?, platen: Int?) throws -> Print
    func proof(blank: Int?) -> Print?
    func trim(blank: Int?) -> Print?
}

protocol Varnished {}

protocol Letterpress<Print>: Inking & Pressing {}

// Reached through `Letterpress`, and only for what conforms to `Inking` too.
extension Pressing where Self: Inking {
    func press(blank: Int?, platen: Int?) throws -> Print {
        try ink(blank: blank)
    }
}

extension Pressing where Self: Varnished {
    func proof(blank: Int?) -> Print? { nil }
}

// Not decided by conformance: `trim` is left to the search by name.
extension Pressing where Print == Int {
    func trim(blank: Int?) -> Print? { nil }
}

struct Platen: Pressing {
    func press(blank: Int?, platen: Int?) throws -> Int { 0 }
    func proof(blank: Int?) -> Int? { nil }
    func trim(blank: Int?) -> Int? { nil }
}

final class Linocut: Letterpress {
    func ink(blank: Int?) throws -> Int { 0 }
    func proof(blank: Int?) -> Int? { nil }
}

final class Woodcut: Letterpress {
    func ink(blank: Int?) throws -> Int { 0 }
    func press(blank: Int?, platen: Int?) throws -> Int { 1 }
    func proof(blank: Int?) -> Int? { nil }
    func trim(blank: Int?) -> Int? { nil }
}

final class Etching: Letterpress, Varnished {
    func ink(blank: Int?) throws -> Int { 0 }
    func trim(blank: Int?) -> Int? { nil }
}

func printRun() {
    let lino = Linocut()
    _ = try? lino.press(blank: nil, platen: nil)
    //            ^ d: Sources/Shop/Printshop.swift:19
    //                              ^ d: Sources/Shop/Printshop.swift:19
    _ = lino.trim(blank: nil)
    //       ^ d: !jump
    let wood = Woodcut()
    _ = try? wood.press(blank: nil, platen: nil)
    //            ^ d: Sources/Shop/Printshop.swift:46
    _ = Etching().proof(blank: nil)
    //            ^ d: Sources/Shop/Printshop.swift:25
}
