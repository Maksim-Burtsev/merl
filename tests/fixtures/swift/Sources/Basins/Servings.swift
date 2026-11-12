class Crock {
    func stew() {
    }
}

class Wok {
    func stew() {
    }
}

func servePair() -> (Crock, Wok) {
    return (Crock(), Wok())
}

func unpack() {
    let (c, w) = servePair()
    w.stew()
//    ^ d: Sources/Basins/Servings.swift:7
    c.stew()
//    ^ d: Sources/Basins/Servings.swift:2
}
