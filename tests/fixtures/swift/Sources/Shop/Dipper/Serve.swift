// A chain of declared returns from a static factory, its link names on several types (#1469).
func serve() -> Int {
    _ = Dipper.builder().size(2).build().dip()
    //                           ^ d: Sources/Shop/Dipper/Dipper.swift:20
    //                                   ^ d: Sources/Shop/Dipper/Dipper.swift:2
    _ = Dipper.builder()
        .size(2)
        .build()
    //   ^ d: Sources/Shop/Dipper/Dipper.swift:20
        .dip()
    //   ^ d: Sources/Shop/Dipper/Dipper.swift:2
    return Tankard.builder().size(2).build().dip()
    //                               ^ d: !jump
    //                                       ^ d: !jump
}

extension Vault {
    func pour() -> Int {
        Dipper.builder().size(2).build().dip()
        //                               ^ d: !jump
    }
}

func shadowed(_ scoops: [Scoop]) {
    scoops.forEach { Dipper in _ = Dipper.size(2).build().dip() }
    //                                                    ^ d: !jump
}
func ladle(_ l: Ladle) async throws -> Int {
    let d = await l.find()
    _ = d.dip()
    //    ^ d: picker Sources/Shop/Dipper/Dipper.swift:2, …; want Sources/Shop/Dipper/Dipper.swift:2 (#1552)
    _ = try await l.fetch().size(2).build().dip()
    //                      ^ d: picker Sources/Shop/Dipper/Dipper.swift:6, Sources/Shop/Dipper/Dipper.swift:16, Sources/Shop/Dipper/Dipper.swift:34; want Sources/Shop/Dipper/Dipper.swift:16 (#1552)
    //                                      ^ d: picker Sources/Shop/Dipper/Dipper.swift:2, …; want Sources/Shop/Dipper/Dipper.swift:2 (#1552)
    return await l.find().dip()
    //                    ^ d: picker Sources/Shop/Dipper/Dipper.swift:2, …; want Sources/Shop/Dipper/Dipper.swift:2 (#1552)
}
