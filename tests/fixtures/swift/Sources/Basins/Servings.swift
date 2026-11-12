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
//    ^ d: picker Sources/Basins/Servings.swift:2, Sources/Basins/Servings.swift:7; want Sources/Basins/Servings.swift:7 (#1484)
    c.stew()
//    ^ d: picker Sources/Basins/Servings.swift:2, Sources/Basins/Servings.swift:7; want Sources/Basins/Servings.swift:2 (#1484)
}
