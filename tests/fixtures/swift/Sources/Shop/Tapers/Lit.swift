// An indexed standard container of a declared element type (#1469).
class Lantern {
    func snuff() {}
}

func lightAll(tapers: [Taper], byName: [String: Taper]) {
    tapers[0].snuff()
    //        ^ d: picker Sources/Shop/Tapers/Lit.swift:3, Sources/Shop/Tapers/Sconce.swift:2, Sources/Shop/Tapers/Taper.swift:2; want Sources/Shop/Tapers/Taper.swift:2 (#1469)
    byName["a"]?.snuff()
    //           ^ d: picker Sources/Shop/Tapers/Lit.swift:3, Sources/Shop/Tapers/Sconce.swift:2, Sources/Shop/Tapers/Taper.swift:2; want Sources/Shop/Tapers/Taper.swift:2 (#1469)
}
