func souse(_ f: (Tub) -> Void) {}

func sousing() {
    let x = Tun()
    souse { x in _ = x.dunk() }
    //                 ^ d: Sources/Shop/Tubs/Tub.swift:2
    souse { x in
        _ = x.dunk()
        //    ^ d: Sources/Shop/Tubs/Tub.swift:2
    }
    _ = x.dunk()
    //    ^ d: Sources/Shop/Tubs/Tun.swift:2
}

class Sousing {
    func a() { souse { x in _ = x.dunk() } }
    //                            ^ d: Sources/Shop/Tubs/Tub.swift:2
}

class OwnSouse {
    func souse(_ f: (Tun) -> Void) {}
    func b() { souse { x in _ = x.dunk() } }
    //                            ^ d: Sources/Shop/Tubs/Tun.swift:2
}

class SousedObject: NSObject {
    func c() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

struct SousedCodable: Codable {
    func e() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SousedLater {
    func g() { souse { x in _ = x.dunk() } }
    //                            ^ d: Sources/Shop/Tubs/Tun.swift:2
}

class SousedConforming {
    func h() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SouseOuter {
    static func souse(_ f: (Tun) -> Void) {}
    struct SouseInner {
        func i() { souse { x in _ = x.dunk() } }
        //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
    }
}

class SouseFlatOuter {
    static func souse(_ f: (Tun) -> Void) {}
struct SouseFlat {
    func j() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}
}

class SousedParam {
    func k(souse: ((Tun) -> Void) -> Void) { souse { x in _ = x.dunk() } }
    //                                                          ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

struct SousedPlain {
    func m() {
        souse { x in
            _ = x.dunk()
            //    ^ d: Sources/Shop/Tubs/Tub.swift:2
        }
    }
}

class SousedSnug {
    func n() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SousedNicked {
    func o() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SousedQualified {
    func q() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SousedMarked {
    func r() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SouseHull {
    static func souse(_ f: (Tun) -> Void) {}
    struct SouseKeel {}
}

extension SouseHull.SouseKeel {
    func s() { souse { x in _ = x.dunk() } }
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}

class SousedQuoted {
    func t() { souse { x in _ = x.dunk() } }
    //                            ^ d: Sources/Shop/Tubs/Tub.swift:2
}

typealias SouseHandler = (Tub) -> Void
func souseAliased(_ f: SouseHandler) {}

class SouseAliasShadow {
    typealias SouseHandler = (Tun) -> Void
    func u() { souseAliased { x in _ = x.dunk() } }
    //                                   ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2
}
