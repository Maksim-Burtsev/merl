struct Situla {
    func tipple() {}
}

protocol Tippler {
    func tipple()
}

func situlaPour<Situla: Tippler>(_ x: Situla) {
//                                    ^ d: Sources/Shop/Situlas.swift:9
    x.tipple()
    //^ d: picker Sources/Shop/Situlas.swift:2, Sources/Shop/Situlas.swift:6
}

func situlaWhere<Situla>(_ x: Situla) where Situla: Tippler {
//                            ^ d: Sources/Shop/Situlas.swift:15
    x.tipple()
    //^ d: picker Sources/Shop/Situlas.swift:2, Sources/Shop/Situlas.swift:6
}

struct SitulaCask<Situla: Tippler> {
    var held: Situla
    //        ^ d: Sources/Shop/Situlas.swift:21
    func use(_ y: Situla) {
    //            ^ d: Sources/Shop/Situlas.swift:21
        y.tipple()
        //^ d: picker Sources/Shop/Situlas.swift:2, Sources/Shop/Situlas.swift:6
        held.tipple()
        //   ^ d: picker Sources/Shop/Situlas.swift:2, Sources/Shop/Situlas.swift:6
    }
}

class SitulaKeg<Situla: Tippler & Hashable> {
    var held: Situla?
    //        ^ d: Sources/Shop/Situlas.swift:33
}

extension SitulaCask {
    func again(_ z: Situla) {
    //              ^ d: Sources/Shop/Situlas.swift:1; want Sources/Shop/Situlas.swift:21 (#1438)
        z.tipple()
        //^ d: picker Sources/Shop/Situlas.swift:2, Sources/Shop/Situlas.swift:6
    }
}

enum SitulaChoice<Situla> {
    case one(Situla)
    //       ^ d: Sources/Shop/Situlas.swift:46
}

typealias SitulaPair<Situla> = (Situla, Situla)
//                              ^ d: Sources/Shop/Situlas.swift:51

extension SitulaCask where Situla: Equatable {
    func third(_ w: Situla) -> Bool { w == w }
    //              ^ d: Sources/Shop/Situlas.swift:1; want Sources/Shop/Situlas.swift:21 (#1438)
}

struct SitulaInit {
    init<Situla: Tippler>(_ x: Situla) {
    //                         ^ d: Sources/Shop/Situlas.swift:60
        x.tipple()
        //^ d: picker Sources/Shop/Situlas.swift:2, Sources/Shop/Situlas.swift:6
    }
}

func situlaPlain(_ x: Situla) {
//                    ^ d: Sources/Shop/Situlas.swift:1
    x.tipple()
    //^ d: Sources/Shop/Situlas.swift:2
}
