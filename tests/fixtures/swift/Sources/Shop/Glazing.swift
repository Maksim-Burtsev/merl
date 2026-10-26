struct Glaze {
    let coats: Set<Int>
}

extension Glaze {
    static func glaze(layers: Int = 1,
                      coats: Set<Int> = [],
                      //^ d: picker Sources/Shop/Glazing.swift:2
                      //  status: at a declaration
                      shine: Bool = false) -> Glaze {
                      //^ d: Sources/Shop/Glazing.swift:10
        Glaze(coats: coats)
        //           ^ d: Sources/Shop/Glazing.swift:7
    }
}

final class Kiln<Glaze: Sendable>: Sendable {
    //           ^ d: picker Sources/Shop/Glazing.swift:1
    //             status: at a declaration
}

func fire<Glaze: Sendable, Ash>(_ g: Glaze, _ a: Ash) {}
//        ^ d: picker Sources/Shop/Glazing.swift:1
//                         ^ d: Sources/Shop/Glazing.swift:22
//                                   ^ d: Sources/Shop/Glazing.swift:22

func glost(rack: Int,
           _ ware: (Int) -> Int = { $0 },
           coats: Int) {}
//         ^ d: picker Sources/Shop/Glazing.swift:2

func sinter() {
    _ = Glaze.glaze(layers: 2,
                    coats: [])
    //              ^ d: Sources/Shop/Glazing.swift:6
}
