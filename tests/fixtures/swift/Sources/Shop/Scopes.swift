// Scopes `d` must not prove empty, and declarations it must not hide (#375, #564, #577).
final class Valve {
    func vent() {}
    struct Seam {}

    func cruise() {
        func vent() {}
        struct Seam {}
        func drift() {
            vent()
        //  ^ d: Sources/Shop/Scopes.swift:7
            _ = Seam()
        //      ^ d: Sources/Shop/Scopes.swift:8
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

// A local `func` or type binds its name for the whole body that declares it, above the cursor and
// below, as `swiftc` reads it: the type's namesake member through `self` is not it (#577).
final class Damper {
    func choke() {}
    struct Plate {}

    func clamp() {
        func choke() {}
        struct Plate {}
        choke()
        // ^ d: Sources/Shop/Scopes.swift:59
        //   status: (local)
        _ = Plate()
        //  ^ d: Sources/Shop/Scopes.swift:60
    }

    func early() {
        choke()
        // ^ d: Sources/Shop/Scopes.swift:73
        _ = Plate()
        //  ^ d: Sources/Shop/Scopes.swift:74
        func choke() {}
        struct Plate {}
    }
}

// Local overloads are all offered, a local type of every keyword binds, and a local type in front
// of a dot still leads to its member (#577).
final class Lathe {
    func shave() {}
    struct Anvil {}
    enum Heat { case low }
    struct Slag {}

    func turn() {
        func shave() {}
        func shave(_ x: Int) {}
        shave()
        // ^ d: picker Sources/Shop/Scopes.swift:87, Sources/Shop/Scopes.swift:88
        //   status: local, 2 declarations
        class Anvil {}
        enum Heat { case high }
        actor Ingot {}
        typealias Slag = Int
        _ = Anvil()
        //  ^ d: Sources/Shop/Scopes.swift:92
        //    status: (local)
        _ = Heat.high
        //  ^ d: Sources/Shop/Scopes.swift:93
        let s: Slag = 1
        //     ^ d: Sources/Shop/Scopes.swift:95
        _ = (s, Ingot())
        //      ^ d: Sources/Shop/Scopes.swift:94
    }
}

protocol Caster {
    static func cast() -> Self
}

final class Forge {
    func strike() {
        struct Bar: Caster {
            static func cast() -> Bar { Bar() }
        }
        _ = Bar.cast()
        //      ^ d: Sources/Shop/Scopes.swift:115
        //        status: via Bar
    }
}

// A local `func` binds in its own block only: not in a sibling method, not from an inner block,
// and from a block below the cursor too (#577).
final class Whetstone {
    func cool() {}
    func buff() {}

    func quench() {
        cool()
        // ^ d: Sources/Shop/Scopes.swift:126
        //   status: via self: Whetstone
    }

    func temper() {
        func cool() {}
        cool()
    }

    func hone(_ on: Bool) {
        cool()
        // ^ d: Sources/Shop/Scopes.swift:126
        if on {
            func cool() {}
            buff()
            // ^ d: Sources/Shop/Scopes.swift:147
            func buff() {}
        }
    }
}

struct Bobbin<T> {
    func wind(_ x: Int) {}
}

extension Bobbin
where T: Equatable {
    func wind() {}
    func spool() {
        wind()
        // ^ d: picker Sources/Shop/Scopes.swift:153, Sources/Shop/Scopes.swift:158
        //   status: by name
    }
}
