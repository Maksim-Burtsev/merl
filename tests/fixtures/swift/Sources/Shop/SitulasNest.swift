struct SitulaJug<Situla> {
    struct Spout {
        func pour(_ x: Situla) {}
        //             ^ d: Sources/Shop/SitulasNest.swift:1
    }
    enum Lid {
        case cap(Situla)
        //       ^ d: Sources/Shop/SitulasNest.swift:1
    }
    struct Bail<Situla> {
        func lift(_ b: Situla) {}
        //             ^ d: Sources/Shop/SitulasNest.swift:10
    }
}

extension SitulaJug {
    struct Handle {
        var grip: Situla
        //        ^ d: Sources/Shop/SitulasNest.swift:1
        struct Rivet {
            func set(_ r: Situla) {}
            //            ^ d: Sources/Shop/SitulasNest.swift:1
        }
    }
}

extension SitulaCask {
    struct Hoop {
        func bind(_ h: Situla) {}
        //             ^ d: Sources/Shop/Situlas.swift:21
    }
}

struct SitulaTray {
    struct Rim {
        func edge(_ e: Situla) {}
        //             ^ d: Sources/Shop/Situlas.swift:1
    }
}

extension SitulaInit {
    struct Ring {
        func ring(_ e: Situla) {}
        //             ^ d: Sources/Shop/Situlas.swift:1
    }
}

func situlaLocal() {
    struct Local {
        func tip(_ l: Situla) {}
        //            ^ d: Sources/Shop/Situlas.swift:1
    }
}

struct SitulaBrim<Brim> {}

extension SitulaBrim {
    struct Clasp {
        func shut(_ b: Brim) {}
        //             ^ d: Sources/Shop/SitulasNest.swift:55
    }
    struct Ferrule {
        typealias Brim = Int
        struct Pin {
            func push(_ b: Brim) {}
            //             ^ d: picker Sources/Shop/SitulasNest.swift:63, Sources/Shop/SitulasFar.swift:34, Sources/Shop/SitulasFar.swift:42; want Sources/Shop/SitulasNest.swift:63 (#1438)
        }
    }
}

class DredgeBase {
    typealias Dredge = Int
}

struct DredgeBox<Dredge> {
    class Walker: DredgeBase {
        var at: Dredge = 0
        //      ^ d: Sources/Shop/SitulasNest.swift:72
    }
}

extension DredgeBox {
    class Strider: DredgeBase {
        var at: Dredge = 0
        //      ^ d: Sources/Shop/SitulasNest.swift:72
    }
}
