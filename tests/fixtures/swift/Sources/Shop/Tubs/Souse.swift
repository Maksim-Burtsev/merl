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
    //                            ^ d: picker Sources/Shop/Tubs/Hogshead.swift:2, Sources/Shop/Tubs/Tub.swift:2, Sources/Shop/Tubs/Tun.swift:2; want Sources/Shop/Tubs/Tub.swift:2 (#1488)
}

class OwnSouse {
    func souse(_ f: (Tun) -> Void) {}
    func b() { souse { x in _ = x.dunk() } }
    //                            ^ d: Sources/Shop/Tubs/Tun.swift:2
}
