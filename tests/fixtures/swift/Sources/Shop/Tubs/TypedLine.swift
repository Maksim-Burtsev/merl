class TubTypedLine {
    let tubs: [Tub] = []
    let c = Tun()
    func each(_ f: (Tub) -> Void) {}
    func pair(_ f: (Tub, Tun) -> Void) {}

    func a() { _ = tubs.map { (c: Tub) in c.dunk() } }
    //                                      ^ d: Sources/Shop/Tubs/Tub.swift:2
    func b() { each { (c: Tub) in _ = c.dunk() } }
    //                                  ^ d: Sources/Shop/Tubs/Tub.swift:2
    func d() { pair { (a: Tub, c: Tun) in _ = c.dunk() } }
    //                                          ^ d: Sources/Shop/Tubs/Tun.swift:2
    func e() { _ = tubs.map { (c: Hogshead) in c.dunk() } }
    //                                           ^ d: Sources/Shop/Tubs/Hogshead.swift:2
    func f() { _ = tubs.map { [c] (x: Tub) in c.dunk() } }
    //                                          ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func g() { _ = tubs.map { (c: Tub) -> Bool in c.dunk() } }
    //                                              ^ d: Sources/Shop/Tubs/Tub.swift:2
    func h() { _ = tubs.map { (x: Tub) in c.dunk() } }
    //                                      ^ d: Sources/Shop/Tubs/Tun.swift:2
    func i() { _ = tubs.map { (c: Tub) in let c = Tun(); return c.dunk() } }
    //                                                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func j() { _ = tubs.map { (c: Tub) in [c].map { c in c.dunk() } } }
    //                                                     ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func k() { _ = tubs.map { (c: Tub) in { c.dunk() }() } }
    //                                        ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func l() { _ = tubs.map { (c: Tub) in c }.map { c.dunk() } }
    //                                                ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func m() { _ = tubs.map { (c: Tub) in "c.dunk()" } }
    //                                       ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func n() { _ = tubs.map { (x: Tub) in /* { */ x.dunk() } }
    //                                              ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}
