class Firkin {
    func slosh() -> Int { return 1 }
}

typealias Noggin = Firkin

func noggins() -> Int {
    let a = Noggin()
    //      ^ d: Sources/Shop/Noggins.swift:5
    return a.slosh() + Noggin().slosh()
    //       ^ d: Sources/Shop/Noggins.swift:2
    //                          ^ d: Sources/Shop/Noggins.swift:2
}
