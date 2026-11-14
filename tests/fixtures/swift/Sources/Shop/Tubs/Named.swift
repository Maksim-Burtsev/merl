class TubNamed {
    let tubs: [Tub] = []
    let c = Tun()

    func each(_ f: (Tub) -> Void) {}

    func a() {
        tubs.forEach { c in
            _ = c.dunk()
            //    ^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
    func b() {
        each { c in
            _ = c.dunk()
            //    ^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
    func d() {
        _ = tubs.map { (c: Tub) in
            c.dunk()
            //^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
    func e() {
        each { (c: Tub) in
            _ = c.dunk()
            //    ^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
    func f() {
        _ = tubs.sorted { a, b in
            a.slop() < b.slop()
            //^ d: Sources/Shop/Tubs/Tub.swift:3
            //           ^ d: Sources/Shop/Tubs/Tub.swift:3
        }
    }
    func g() {
        _ = tubs.map { (c: Hogshead) in
            c.dunk()
            //^ d: Sources/Shop/Tubs/Hogshead.swift:2
        }
    }
    func h() {
        each { c in _ = c.dunk() }
        //                ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    }
    func i() {
        tubs.forEach { c in
            let c = Tun()
            _ = c.dunk()
            //    ^ d: Sources/Shop/Tubs/Tun.swift:2
        }
    }
    func j() {
        _ = c.dunk()
        //    ^ d: Sources/Shop/Tubs/Tun.swift:2
    }
    func k() {
        _ = tubs.map { c in
            c.dunk()
            //^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
    func l() {
        each { (c) in
            _ = c.dunk()
            //    ^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
}
class TubNamedLine {
    let tubs: [Tub] = []
    let c = Tun()
    let keyed: [String: Tub] = [:]
    var maybe: [Tub]? = nil

    func a() { tubs.forEach { c in _ = c.dunk() } }
    //                                   ^ d: Sources/Shop/Tubs/Tub.swift:2
    func b() { _ = tubs.map { c in c.dunk() } }
    //                               ^ d: Sources/Shop/Tubs/Tub.swift:2
    func d() { _ = tubs.sorted { a, b in a.slop() < b.slop() } }
    //                                     ^ d: Sources/Shop/Tubs/Tub.swift:3
    //                                                ^ d: Sources/Shop/Tubs/Tub.swift:3
    func e() { tubs.forEach { x in let c = x; _ = c.dunk() } }
    //                                              ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func f() { tubs.forEach { c in [1].forEach { _ in _ = c.dunk() } } }
    //                                                      ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func g() { keyed.forEach { c in _ = c } }
    func h() { _ = tubs.map { c in c.dunk() }.map { c in c } }
    //                               ^ d: Sources/Shop/Tubs/Tub.swift:2
    func i() { _ = tubs.first { c in c.dunk() } }
    //                                 ^ d: Sources/Shop/Tubs/Tub.swift:2
    func j() { _ = tubs.sorted { c in c.dunk() } }
    //                                  ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func k() { _ = tubs.map { a, c in c.dunk() } }
    //                                  ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    func l() { _ = tubs.min(by: { a, b in a.slop() < b.slop() }) }
    //                                      ^ d: Sources/Shop/Tubs/Tub.swift:3
    //                                                 ^ d: Sources/Shop/Tubs/Tub.swift:3
    func m() { _ = tubs.forEach { (c) in _ = c.dunk() } }
    //                                         ^ d: Sources/Shop/Tubs/Tub.swift:2
    func n() {
        tubs.forEach { c in
            [Tun()].forEach { c in
                _ = c.dunk()
                //    ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
            }
        }
    }
    func o() {
        tubs.forEach { c in
            let c = Tun()
            _ = c.dunk()
            //    ^ d: Sources/Shop/Tubs/Tun.swift:2
        }
    }
}
