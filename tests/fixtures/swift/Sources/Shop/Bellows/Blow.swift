// A method whose overloads return different types, told apart by the call's labels (#1469).
func blowAll(bellows: Bellows, fuel: String) {
    bellows.blow(fuel).melt().skim { slag in
    //                 ^ d: Sources/Shop/Bellows/Crucible.swift:7
    //                        ^ d: Sources/Shop/Bellows/Crucible.swift:3
        _ = slag
    }
    bellows.puff(fuel).melt()
    //                 ^ d: picker Sources/Shop/Bellows/Crucible.swift:2, Sources/Shop/Bellows/Crucible.swift:7, Sources/Shop/Bellows/Smelter.swift:2
}
