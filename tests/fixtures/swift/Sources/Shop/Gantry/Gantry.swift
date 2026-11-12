class Gantry<T> {
    var load: T
    init(_ load: T) { self.load = load }
    func first() -> T { load }
    func grab<T>(_ x: T) -> T { x }
}

final class Ewer {
    func brim() {}
}

final class Ladle {
    func brim() {}
}

class Perch: Gantry<Ewer> {
    func tidy() {
        self.first().brim()
        //           ^ d: Sources/Shop/Gantry/Gantry.swift:9
        first().brim()
        //      ^ d: Sources/Shop/Gantry/Gantry.swift:9
    }
}

class Lofty: Perch {}

class Rung<U>: Gantry<U> {
    func tidy() {
        self.first().brim()
        //           ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    }
}

class Bare: Gantry {}

class Pair<K, V> {
    func right() -> V { fatalError() }
}

class Twin: Pair<Ladle, Ewer> {}

class Odd: Pair<Ladle, Ewer, Ladle> {}

func hoist(p: Perch, l: Lofty, r: Bare, t: Twin, o: Odd) {
    p.first().brim()
    //        ^ d: Sources/Shop/Gantry/Gantry.swift:9
    l.first().brim()
    //        ^ d: Sources/Shop/Gantry/Gantry.swift:9
    r.first().brim()
    //        ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    p.grab(Ladle()).brim()
    //              ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    t.right().brim()
    //        ^ d: Sources/Shop/Gantry/Gantry.swift:9
    o.right().brim()
    //        ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
}

extension Gantry {
    func last() -> T { load }
}

func reach(p: Perch) {
    p.last().brim()
    //       ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13; want Sources/Shop/Gantry/Gantry.swift:9 (#1481)
    p.load.brim()
    //     ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13; want Sources/Shop/Gantry/Gantry.swift:9 (#1481)
}
