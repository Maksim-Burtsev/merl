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
            // ^ d: Sources/Shop/Corvid.swift:2
    _ = advance(.rook)
              // ^ d: Sources/Shop/Chessman.swift:2
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

func hop(_ bird: Corvid) -> Int { 0 }

struct Hopper {
    func hop(_ piece: Chessman) -> Int { 1 }

    func land() -> Int {
        hop(.rook)
          // ^ d: picker Sources/Shop/Chessman.swift:2, Sources/Shop/Corvid.swift:2
    }
}

func preen(_ bird: Corvid) -> Int { 0 }

func flutter() -> Int {
    let preen = { (piece: Chessman) in 0 }
    return preen(.rook)
               // ^ d: picker Sources/Shop/Chessman.swift:2, Sources/Shop/Corvid.swift:2
}

func nest(depth: Int, piece: Chessman) -> Int { depth }

func alight() -> Int {
    nest(depth: 1, piece: .rook)
                         // ^ d: Sources/Shop/Chessman.swift:2
}
