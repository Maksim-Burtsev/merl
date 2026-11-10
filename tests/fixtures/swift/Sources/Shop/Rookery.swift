// A case two enums of the project share, written without its enum (#1448).
func perch(_ bird: Corvid) -> Int {
    switch bird {
    case .rook:
       // ^ d: Sources/Shop/Corvid.swift:2
        return 1
    case .raven:
        return 0
    }
}

func advance(_ piece: Chessman) -> Int {
    switch piece {
    case .rook:
       // ^ d: Sources/Shop/Chessman.swift:2
        return 1
    default:
        return 0
    }
}

func roost(bird: Corvid, piece: Chessman) -> Bool {
    _ = perch(.rook)
            // ^ d: picker Sources/Shop/Chessman.swift:2, Sources/Shop/Corvid.swift:2; want Sources/Shop/Corvid.swift:2 (#1448)
    _ = advance(.rook)
              // ^ d: picker Sources/Shop/Chessman.swift:2, Sources/Shop/Corvid.swift:2; want Sources/Shop/Chessman.swift:2 (#1448)
    let held: Corvid = .rook
                     // ^ d: Sources/Shop/Corvid.swift:2
    _ = held
    if piece == .rook { return true }
              // ^ d: Sources/Shop/Chessman.swift:2
    _ = Chessman.rook
              // ^ d: Sources/Shop/Chessman.swift:2
    return bird == .rook
                 // ^ d: Sources/Shop/Corvid.swift:2
}

func perchOver(bird: Corvid, kinds: [Chessman]) -> Bool {
    for bird in kinds {
        if bird == .rook { return true }
                 // ^ d: picker Sources/Shop/Chessman.swift:2, Sources/Shop/Corvid.swift:2
    }
    switch bird {
    case .raven, .rook:
               // ^ d: Sources/Shop/Corvid.swift:2
        return true
    }
}
