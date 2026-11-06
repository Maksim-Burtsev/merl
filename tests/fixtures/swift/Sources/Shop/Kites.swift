// An implicit `.x` passed to a method of a value whose binding a closure parameter, a pattern or
// a nested func's parameter shadows on the cursor's own line keeps a picker (#1341).
enum Gust {
    case squall, breeze
}

enum Tide {
    case squall, breeze
}

final class Kite {
    func fly(_ gust: Gust) -> Bool { true }
}

final class Buoy {
    func fly(_ tide: Tide) -> Bool { true }
    func make() -> Buoy { self }
}

struct Pier {
    var buoys: [Buoy] = []
    var reserve: Buoy? = nil
    var only: Buoy = Buoy()
}

enum Mooring {
    case held(Buoy)
}

func flights(kite: Kite, pier: Pier, buoy maybe: Buoy?, mooring: Mooring) {
    kite.fly(.squall)
    //        ^ d: Sources/Shop/Kites.swift:4
    _ = { (kite: Buoy) in kite.fly(.squall) }
    //                              ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    _ = { (kite: Buoy) in
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    pier.buoys.forEach { kite in kite.fly(.squall) }
    //                                     ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    pier.buoys.forEach { kite in
        kite.fly(.squall)
        //        ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    }
    pier.buoys.forEach { (kite: Buoy) -> Void in
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    if let kite = pier.reserve {
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    if let kite = maybe { kite.fly(.squall) }
    //                              ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    guard let kite2 = maybe else { return }
    kite2.fly(.squall)
    //         ^ d: Sources/Shop/Kites.swift:8
    if case let .held(kite) = mooring {
        kite.fly(.squall)
        //        ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    }
    switch mooring {
    case let .held(kite):
        kite.fly(.squall)
        //        ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    }
    for kite in pier.buoys {
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    for kite in pier.buoys { kite.fly(.squall) }
    //                                 ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    func inner(kite: Buoy) {
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    func inner2(_ kite: Buoy) {
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    func inner3(with kite: Buoy) {
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    do {
        let kite = pier.only
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    do {
        let kite = Buoy().make()
        kite.fly(.squall)
        //        ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    }
    while let kite = maybe {
        kite.fly(.squall)
        //        ^ d: Sources/Shop/Kites.swift:8
    }
    _ = pier.buoys.map { kite -> Int in
        kite.fly(.squall)
        //        ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
        return 0
    }
    _ = pier.buoys.map {
        $0.fly(.squall)
        //      ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    }
    func inner4(kite: Buoy) { kite.fly(.squall) }
    //                                  ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    if case let .held(kite) = mooring { kite.fly(.squall) }
    //                                            ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    switch mooring { case let .held(kite): kite.fly(.squall) }
    //                                               ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    if let kite = pier.reserve, true { kite.fly(.squall) }
    //                                           ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    _ = pier.buoys.map { (kite) in kite.fly(.squall) }
    //                                       ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    _ = { [kite] in kite.fly(.squall) }
    //                        ^ d: Sources/Shop/Kites.swift:4
    if let kite = maybe, kite.fly(.squall) {}
    //                             ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    while let kite = maybe, kite.fly(.squall) {}
    //                                ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    for kite in pier.buoys where kite.fly(.squall) {}
    //                                     ^ d: picker Sources/Shop/Kites.swift:4, Sources/Shop/Kites.swift:8; want Sources/Shop/Kites.swift:8 (#1341)
    do {
        let kite = kite.fly(.squall)
        //                   ^ d: Sources/Shop/Kites.swift:4
    }
    kite.fly(.squall)
    //        ^ d: Sources/Shop/Kites.swift:4
}
