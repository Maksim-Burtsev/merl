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
