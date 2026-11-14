class TubQuoted {
    let tubs: [Tub] = []

    func a() { _ = tubs.map { c in "c.dunk()" } }
    //                                ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func b() { _ = tubs.map { c in "dunk" } }
    //                              ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func c() {
        _ = tubs.map { c in """
            c.dunk()
        //    ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
            """ }
    }
    func d() { _ = tubs.map { c in #"c.dunk() \(c.dunk())"# } }
    //                                 ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    //                                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func e() { _ = tubs.map { c in "\(c.dunk())" } }
    //                                  ^ d: Sources/Shop/Tubs/Tub.swift:2
    func f() { _ = tubs.map { c in #"\#(c.dunk())"# } }
    //                                    ^ d: Sources/Shop/Tubs/Tub.swift:2
    func g() { _ = tubs.map { c in "\(c.slop() + "\(c.dunk())".count) c.dunk()" } }
    //                                                ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    //                                                                  ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func h() { _ = tubs.map { c in "\"c.dunk()" } }
    //                                  ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func i() { _ = tubs.map { c in "a\\" + c.dunk() } }
    //                                       ^ d: Sources/Shop/Tubs/Tub.swift:2
}
