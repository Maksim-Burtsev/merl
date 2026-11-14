class TubDollars {
    let tubs: [Tub] = []
    let tuns: [Tun] = []
    let byName: [String: Tub] = [:]

    func a() { _ = tuns.sorted { $0.slop() < $1.slop() } }
    //                                          ^ d: Sources/Shop/Tubs/Tun.swift:3
    func b() { _ = tubs.max { $0.slop() < $1.slop() } }
    //                           ^ d: Sources/Shop/Tubs/Tub.swift:3
    //                                       ^ d: Sources/Shop/Tubs/Tub.swift:3
    func c() { _ = tubs.min(by: { $0.slop() < $1.slop() }) }
    //                                           ^ d: Sources/Shop/Tubs/Tub.swift:3
    func d() { _ = byName.sorted { $0.value.slop() < $1.value.slop() } }
    //                                                        ^ d: !jump
    func e() { _ = tubs.map { $0.slop() + tuns.reduce(0) { $1.slop() } } }
    //                                                        ^ d: !jump
    func f() { _ = tubs.reduce(0) { $0 + $1.slop() } }
    //                                      ^ d: !jump
    func g() { _ = tubs.lazy.sorted { $0.slop() < $1.slop() } }
    //                                               ^ d: !jump
}
