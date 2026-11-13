// A project generic type's own `map` and `forEach` hand the closure its `T`, not an element.
struct Rundlet<T> {
    func map(_ f: (T) -> Void) {}
    func forEach(_ f: (T) -> Void) {}
}

func broach(rundlet: Rundlet<Tub>) {
    rundlet.map { c in _ = c.dunk() }
    //                       ^ d: Sources/Shop/Tubs/Tub.swift:2
    rundlet.forEach { c in
        _ = c.dunk()
        //    ^ d: Sources/Shop/Tubs/Tub.swift:2
    }
}
