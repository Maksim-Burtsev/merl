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

func mixed(flag: Bool, other: (Wok, Crock)) {
    let (c, w) = (Wok(), Crock())
    c.stew()
//    ^ d: Sources/Basins/Servings.swift:7
    let (d, e) = flag ? servePair() : other
    d.stew()
//    ^ d: !jump
    let maker = { (Wok(), Crock()) }
    let (f, g) = maker()
    f.stew()
//    ^ d: !jump
}

func serveNamed() -> (crock: Crock, wok: Wok) {
    return (crock: Crock(), wok: Wok())
}

func written() {
    let (c, w): (Crock, Wok) = servePair()
    w.stew()
//    ^ d: Sources/Basins/Servings.swift:7
    c.stew()
//    ^ d: Sources/Basins/Servings.swift:2
    let (k, v) = (Crock(), Wok())
    v.stew()
//    ^ d: Sources/Basins/Servings.swift:7
    k.stew()
//    ^ d: Sources/Basins/Servings.swift:2
    let t = serveNamed()
    t.wok.stew()
//        ^ d: Sources/Basins/Servings.swift:7
    t.crock.stew()
//          ^ d: Sources/Basins/Servings.swift:2
}
