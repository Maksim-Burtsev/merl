class TubUse {
    let tubs: [Tub] = []
    let c = Tun()
    var more: Array<Tub> = []

    func a() { tubs.forEach { _ = $0.dunk() } }
    //                               ^ d: Sources/Shop/Tubs/Tub.swift:2
    func b() { tubs.forEach { x in _ = x.dunk() } }
    //                                   ^ d: Sources/Shop/Tubs/Tub.swift:2
    func b2() { _ = tubs.map { c in c.dunk() } }
    //                                ^ d: Sources/Shop/Tubs/Tub.swift:2
    func d() { _ = tubs.filter { x in x.dunk() } }
    //                                  ^ d: Sources/Shop/Tubs/Tub.swift:2
    func e() { _ = tubs.map { (x: Tub) in x.dunk() } }
    //                                      ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tub.swift:2 (#1488)
    func f() { _ = tubs.sorted { $0.slop() < $1.slop() } }
    //                              ^ d: Sources/Shop/Tubs/Tub.swift:3
    //                                          ^ d: Sources/Shop/Tubs/Tub.swift:3
    func g() { _ = tubs.sorted { a, b in a.slop() < b.slop() } }
    //                                     ^ d: Sources/Shop/Tubs/Tub.swift:3
    //                                                ^ d: Sources/Shop/Tubs/Tub.swift:3
    func each(_ f: (Tub) -> Void) {}
    func h() { each { x in _ = x.dunk() } }
    //                           ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tub.swift:2 (#1488)
    func i() { each { _ = $0.dunk() } }
    //                       ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tub.swift:2 (#1488)
    func j(list: [Tub]) { list.forEach { x in _ = x.dunk() } }
    //                                              ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tub.swift:2 (#1488)
    func k() { _ = more.map { $0.dunk() } }
    //                           ^ d: Sources/Shop/Tubs/Tub.swift:2
    func l() { _ = c.dunk() }
    //               ^ d: Sources/Shop/Tubs/Tun.swift:2
    func m() { _ = tubs.map { $0.dunk() } }
    //                           ^ d: Sources/Shop/Tubs/Tub.swift:2
    func n() { _ = tubs.first { x in x.dunk() } }
    //                                 ^ d: Sources/Shop/Tubs/Tub.swift:2
    func o() { tubs.forEach { x in let x = Tun(); _ = x.dunk() } }
    //                                                  ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tun.swift:2 (#1488)
}
