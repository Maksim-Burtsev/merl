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
