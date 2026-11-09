func kwserve() -> Int {
    let mugful = 5
    let pot = Kwpot()
    _ = kwbrew(mugful: mugful)
    //         ^ d: Sources/Shop/Kwcafe/Brewer.swift:3
    //                 ^ d: Sources/Shop/Kwcafe/Uses.swift:2
    _ = pot.kwbrew(mugful: 4)
    //             ^ d: Sources/Shop/Kwcafe/Pot.swift:4
    return mugful
}
