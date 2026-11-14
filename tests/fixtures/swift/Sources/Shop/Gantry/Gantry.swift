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
    func pick<T>(_ x: T) -> T { x }
    var top: T { load }
    var maybe: T? { load }
}

extension Rung {
    func tidier() {
        self.load.brim()
        //        ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    }
}

func reach(p: Perch, l: Lofty, r: Bare) {
    p.last().brim()
    //       ^ d: Sources/Shop/Gantry/Gantry.swift:9
    p.load.brim()
    //     ^ d: Sources/Shop/Gantry/Gantry.swift:9
    l.load.brim()
    //     ^ d: Sources/Shop/Gantry/Gantry.swift:9
    l.last().brim()
    //       ^ d: Sources/Shop/Gantry/Gantry.swift:9
    p.top.brim()
    //    ^ d: Sources/Shop/Gantry/Gantry.swift:9
    p.maybe!.brim()
    //       ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    r.load.brim()
    //     ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    r.last().brim()
    //       ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
    p.pick(Ladle()).brim()
    //              ^ d: picker Sources/Shop/Gantry/Gantry.swift:9, Sources/Shop/Gantry/Gantry.swift:13
}

final class T {
    func scour() {}
}

final class Jar {
    func scour() {}
}

final class Dish<T> {
    var held: T
    init(_ held: T) { self.held = held }
}

extension Dish {
    var lid: T { held }
}

func lift(d: Dish<Jar>) {
    d.lid.scour()
    //    ^ d: picker Sources/Shop/Gantry/Gantry.swift:95, Sources/Shop/Gantry/Gantry.swift:99
}
