extension SitulaCask {
    func far(_ y: Situla) {}
    //            ^ d: Sources/Shop/Situlas.swift:21
    func farOwn<Situla>(_ y: Situla) {}
    //                       ^ d: Sources/Shop/SitulasFar.swift:4
}

extension SitulaKeg where Situla: Equatable {
    func farther(_ k: Situla) -> Bool { k == k }
    //                ^ d: Sources/Shop/Situlas.swift:33
}

extension SitulaInit {
    func farPlain(_ v: Situla) {}
    //                 ^ d: Sources/Shop/Situlas.swift:1
}

extension Float {
    func farFloat(_ v: Situla) {}
    //                 ^ d: Sources/Shop/Situlas.swift:1
}

extension SitulaVat {
    func farVat(_ v: Situla) {}
    //               ^ d: picker Sources/Shop/Situlas.swift:1, Sources/Shop/SitulaVat.swift:1
}

extension SitulaBrim {
    struct Hoop {
        func fit(_ b: Brim) {}
        //            ^ d: Sources/Shop/SitulasNest.swift:55
    }
    struct Stave {
        typealias Brim = Int
        func bend(_ b: Brim) {}
        //             ^ d: Sources/Shop/SitulasFar.swift:34
    }
}

extension SitulaBrim {
    struct Band {
        typealias Brim = Int
        struct Rivet {
            func set(_ b: Brim) {}
            //            ^ d: picker Sources/Shop/SitulasFar.swift:34, Sources/Shop/SitulasFar.swift:42, Sources/Shop/SitulasNest.swift:63; want Sources/Shop/SitulasFar.swift:42 (#1438)
        }
    }
}
