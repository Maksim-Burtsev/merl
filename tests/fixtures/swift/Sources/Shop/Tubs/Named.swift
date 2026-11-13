class TubNamed {
    let tubs: [Tub] = []
    let c = Tun()

    func each(_ f: (Tub) -> Void) {}

    func a() {
        tubs.forEach { c in
            _ = c.dunk()
        }
    }
    func b() {
        each { c in
            _ = c.dunk()
        }
    }
    func d() {
        _ = tubs.map { (c: Tub) in
            c.dunk()
        }
    }
    func e() {
        each { (c: Tub) in
            _ = c.dunk()
        }
    }
    func f() {
        _ = tubs.sorted { a, b in
            a.slop() < b.slop()
        }
    }
    func g() {
        _ = tubs.map { (c: Hogshead) in
            c.dunk()
        }
    }
    func h() {
        each { c in _ = c.dunk() }
    }
    func i() {
        tubs.forEach { c in
            let c = Tun()
            _ = c.dunk()
        }
    }
    func j() {
        _ = c.dunk()
    }
    func k() {
        _ = tubs.map { c in
            c.dunk()
        }
    }
    func l() {
        each { (c) in
            _ = c.dunk()
        }
    }
}
