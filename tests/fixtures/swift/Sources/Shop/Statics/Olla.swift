// #1489: members through a type, its answers sourcekit-lsp's.
final class Olla {
    static let shared = Olla()
    enum Slip {
        case matte
        case gloss
        func sheen() {}
    }
    static func make() -> Olla { Olla() }
    func make(_ size: Int) -> Olla { self }
    func fill() {}
}

final class Pithos {
    static let shared = Pithos()
    enum Slip {
        case matte
        func sheen() {}
    }
    static func make() -> Pithos { Pithos() }
    func fill() {}
}

func serveOlla(o: Olla) {
    Olla.shared.fill()
    //          ^ d: Sources/Shop/Statics/Olla.swift:11
    Olla.make().fill()
    //          ^ d: Sources/Shop/Statics/Olla.swift:11
    _ = Olla.make()
    //       ^ d: Sources/Shop/Statics/Olla.swift:9
    _ = o.make(2)
    //    ^ d: Sources/Shop/Statics/Olla.swift:10
    Olla.Slip.matte.sheen()
    //              ^ d: Sources/Shop/Statics/Olla.swift:7
    _ = Olla.Slip.matte
    //            ^ d: Sources/Shop/Statics/Olla.swift:5
    Pithos.shared.fill()
    //            ^ d: Sources/Shop/Statics/Olla.swift:21
    Pithos.Slip.matte.sheen()
    //                ^ d: Sources/Shop/Statics/Olla.swift:18
}
