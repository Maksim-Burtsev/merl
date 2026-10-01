// Scopes `d` must not prove empty, and declarations it must not hide (#375, #564).
final class Valve {
    func vent() {}
    struct Seam {}

    func cruise() {
        func vent() {}
        struct Seam {}
        func drift() {
            vent()
        //  ^ d: picker Sources/Shop/Scopes.swift:3, Sources/Shop/Scopes.swift:7
            _ = Seam()
        //      ^ d: picker Sources/Shop/Scopes.swift:4, Sources/Shop/Scopes.swift:8
        }
        drift()
    }
}

protocol Abacus {
    typealias Entry = String
}

struct CashTally: Abacus {
    func first() -> Entry { "" }
    //              ^ d: Sources/Shop/Scopes.swift:20
}

struct Spool<Yarn> {
    struct Reel {
        typealias Yarn = Int
        func next() -> Yarn? { nil }
        //             ^ d: Sources/Shop/Scopes.swift:30
    }
}

func cheapestOf<Lot: Comparable>(_ lots: [Lot]) -> Lot? {
    let first: Lot? = lots.first
    //         ^ d: Sources/Shop/Scopes.swift:36
    //           status: Lot: local
    return first
}

func reread(again r: Prober.Reading) {}
//                          ^ d: Sources/Shop/Visibility.swift:19

struct Strap {}

let strap = Strap()
//          ^ d: Sources/Shop/Scopes.swift:46
//            status: Strap: by name, 1 match
