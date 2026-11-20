// An indexed standard container of a declared element type (#1469).
class Lantern {
    func snuff() {}
}

func lightAll(tapers: [Taper], byName: [String: Taper], byKind: [Int: Sconce]) {
    tapers[0].snuff()
    //        ^ d: Sources/Shop/Tapers/Taper.swift:2
    byName["a"]?.snuff()
    //           ^ d: Sources/Shop/Tapers/Taper.swift:2
    byKind[1]?.snuff()
    //         ^ d: Sources/Shop/Tapers/Sconce.swift:2
}

func lightSome<Taper>(byName: [String: Taper]) {
    byName["a"]?.snuff()
    //           ^ d: picker Sources/Shop/Tapers/Lit.swift:3, Sources/Shop/Tapers/Sconce.swift:2, Sources/Shop/Tapers/Taper.swift:2
}
