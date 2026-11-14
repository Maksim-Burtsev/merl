class TubOneLine {
    let tubs: [Tub] = []
    let c = Tun()

    func l() {
        _ = c.dunk()
    //        ^ d: Sources/Shop/Tubs/Tun.swift:2
    }
    func l2() { _ = c.dunk() }
    //                ^ d: Sources/Shop/Tubs/Tun.swift:2
    func p(c: Hogshead) { _ = c.dunk() }
    //                          ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Hogshead.swift:2 (#1488)
    func q() { let c = Hogshead(); _ = c.dunk() }
    //                                   ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Hogshead.swift:2 (#1488)
    func r() { if let c = Optional(Hogshead()) { _ = c.dunk() } }
    //                                                 ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Hogshead.swift:2 (#1488)
    var s: Bool { c.dunk() }
    //              ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tun.swift:2 (#1488)
    func t() { tubs.forEach { c in _ = c.dunk() } }
    //                                   ^ d: Sources/Shop/Tubs/Tub.swift:2
    func u() { for c in tubs { _ = c.dunk() } }
    //                               ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tub.swift:2 (#1488)
    func v() { guard let c = Optional(Hogshead()) else { return }; _ = c.dunk() }
    //                                                                   ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Hogshead.swift:2 (#1488)
    func v2() { guard let c = Optional(Hogshead()) else { return }; _ = c.dunk() }
    //                                                                  ^ d: Sources/Shop/Tubs/OneLine.swift:25
    func w(_ c: Hogshead) { _ = c.dunk() }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Hogshead.swift:2 (#1488)
    func x<T: Hogshead>(c: T) { _ = c.dunk() }
    //                                ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Hogshead.swift:2 (#1488)
    func y() { if let c = Optional(Hogshead()) { _ = c }; _ = c.dunk() }
    //                                                        ^ d: Sources/Shop/Tubs/OneLine.swift:3
    func y2(c: Hogshead) { _ = c }
    //                         ^ d: Sources/Shop/Tubs/OneLine.swift:33
    func y3() { tubs.forEach { c in _ = c } }
    //                                  ^ d: Sources/Shop/Tubs/OneLine.swift:35
    func y4() { for c in tubs { _ = c } }
    //                              ^ d: Sources/Shop/Tubs/OneLine.swift:37
    func y5() { run(c: Tun()) { _ = c } }
    //                              ^ d: Sources/Shop/Tubs/OneLine.swift:3
    func z() { if let c = Optional(Hogshead()) { _ = c } else { _ = c } }
    //                                                              ^ d: Sources/Shop/Tubs/OneLine.swift:3
    func z2() { guard let c = Optional(Hogshead()) else { _ = c; return }; _ = c.dunk() }
    //                                                        ^ d: Sources/Shop/Tubs/OneLine.swift:3
    func z3() { if let c = Optional(Tub()) { _ = c } else if let c = Optional(Hogshead()) { _ = c } }
    //                                                                                          ^ d: Sources/Shop/Tubs/OneLine.swift:45
    func run(c: Tun, _ f: () -> Void) {}
}
