// Receivers whose type a rule of #384 proves, next to the ones it leaves to the search by name.
class Clapper {
    func peal() -> Int {
        1
    }
}

final class Tam {
    func peal() -> Int {
        2
    }
}

class Steeple: Clapper {}

final class Spire: Steeple {}

typealias Pealer = Clapper

extension Pealer {
    func hum() {}
}

func makeTam() -> Tam {
    Tam()
}

final class Belfry {
    var clapper: Clapper = Clapper()

    func toll(_ maybe: Tam?) {
        _ = clapper.peal()
        //          ^ d: Sources/Shop/Belfry.swift:3
        //            status: via clapper: Clapper
        switch maybe {
        case let clapper?:
            _ = clapper.peal()
            //          ^ d: picker Sources/Shop/Belfry.swift:3, Sources/Shop/Belfry.swift:9
        case nil:
            break
        }
    }
}

func ring(_ pealer: Pealer, makeTam: () -> Clapper) {
    let spire = Spire()
    _ = spire.peal()
    //        ^ d: Sources/Shop/Belfry.swift:3
    //          status: via spire: Spire
    let made = makeTam()
    _ = made.peal()
    //       ^ d: picker Sources/Shop/Belfry.swift:3, Sources/Shop/Belfry.swift:9
    _ = pealer.peal()
    //         ^ d: picker Sources/Shop/Belfry.swift:3, Sources/Shop/Belfry.swift:9
}
